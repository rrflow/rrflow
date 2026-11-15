//! Evidence-derived attunement plans are a pure, verifiable function of the
//! evidence: unimplemented capabilities are never selected, preconditions
//! gate every optional capability, and a derived preview validates only
//! against the plan the evidence produces.

use rrd_contract::{
    attunement_capability_configuration_sha256, attunement_plan_sha256,
    attunement_replan_preview_sha256, attunement_replan_seed, derive_attunement_phases,
    derive_attunement_plan, AttunementEvidence, AttunementPhase, AttunementPhasePlan,
    AttunementPlan, AttunementReplanPreview, CanonicalId, InstalledEngramIdentity,
    ATTUNEMENT_CAPABILITIES, ATTUNEMENT_CONTRACT_VERSION, ATTUNEMENT_EVIDENCE_FORMAT_VERSION,
};

use AttunementPhase as P;

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn evidence(
    languages: &[(&str, u64)],
    grammars: &[&str],
    embedding: bool,
    available: &[P],
) -> AttunementEvidence {
    let files = languages.iter().map(|(_, n)| n).sum::<u64>() + 1;
    let mut evidence = AttunementEvidence {
        format_version: ATTUNEMENT_EVIDENCE_FORMAT_VERSION,
        inventory_sha256: "1".repeat(64),
        entries: files + 2,
        files,
        content_bytes: 4_096 * files,
        languages: languages.iter().map(|(l, n)| (id(l), *n)).collect(),
        bound_grammars: grammars.iter().map(|g| id(g)).collect(),
        embedding_binding: embedding,
        available: available.iter().copied().collect(),
        evidence_sha256: String::new(),
    };
    evidence.evidence_sha256 = evidence.digest().unwrap();
    evidence.validate().unwrap();
    evidence
}

const ALL: [P; 11] = [
    P::Connect,
    P::Inventory,
    P::Parse,
    P::Normalize,
    P::EntityLink,
    P::LexicalIndex,
    P::Embed,
    P::VectorIndex,
    P::Graph,
    P::Ground,
    P::Verify,
];

fn predecessor() -> AttunementPlan {
    let phases = [P::Connect, P::Inventory]
        .into_iter()
        .enumerate()
        .map(|(index, phase)| AttunementPhasePlan {
            phase,
            sequence: (index + 1) as u16,
            description: format!("{phase:?}"),
            configuration_sha256: "a".repeat(64),
            estimated_items: 1,
            estimated_input_bytes: 0,
            parse_policy: None,
        })
        .collect();
    let mut plan = AttunementPlan {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        id: id("attune-predecessor"),
        target: InstalledEngramIdentity {
            project_id: id("project"),
            engram_id: id("engram"),
            instance_id: id("instance"),
        },
        source_sha256: "2".repeat(64),
        phases,
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 30_000,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = attunement_plan_sha256(&plan).unwrap();
    plan.validate().unwrap();
    plan
}

#[test]
fn catalogue_is_in_canonical_order_and_verify_is_last() {
    let phases: Vec<P> = ATTUNEMENT_CAPABILITIES.iter().map(|c| c.phase).collect();
    assert_eq!(phases, ALL.to_vec());
    for capability in &ATTUNEMENT_CAPABILITIES {
        for required in capability.requires {
            assert!(
                ALL.iter().position(|p| p == required)
                    < ALL.iter().position(|p| *p == capability.phase),
                "{:?} requires a later capability",
                capability.phase
            );
        }
    }
}

#[test]
fn only_executable_capabilities_are_ever_selected() {
    // Today's engine: Connect and Inventory only. A repository full of Rust
    // still derives a two-step plan; nothing unavailable is selected.
    let today = evidence(
        &[("rust", 900), ("markdown", 300)],
        &["rust"],
        true,
        &[P::Connect, P::Inventory],
    );
    assert_eq!(
        derive_attunement_phases(&today),
        vec![P::Connect, P::Inventory]
    );
}

#[test]
fn preconditions_gate_every_optional_capability() {
    let empty = evidence(&[], &[], false, &ALL);
    assert_eq!(
        derive_attunement_phases(&empty),
        vec![
            P::Connect,
            P::Inventory,
            P::LexicalIndex,
            P::Ground,
            P::Verify
        ],
        "one file and no grammar selects lexical access only"
    );
    let rust = evidence(&[("rust", 10)], &["rust"], false, &ALL);
    assert_eq!(
        derive_attunement_phases(&rust),
        vec![
            P::Connect,
            P::Inventory,
            P::Parse,
            P::Normalize,
            P::EntityLink,
            P::LexicalIndex,
            P::Graph,
            P::Ground,
            P::Verify
        ],
        "a bound grammar selects parsing, linking and the graph, never embeddings"
    );
    let bound = evidence(&[("rust", 10)], &["rust"], true, &ALL);
    assert_eq!(
        derive_attunement_phases(&bound),
        vec![
            P::Connect,
            P::Inventory,
            P::Parse,
            P::Normalize,
            P::EntityLink,
            P::LexicalIndex,
            P::Embed,
            P::VectorIndex,
            P::Graph,
            P::Ground,
            P::Verify
        ],
        "an applied embedding binding adds Embed and VectorIndex"
    );
    let unparsed = evidence(&[("markdown", 10)], &[], true, &ALL);
    assert!(
        !derive_attunement_phases(&unparsed).contains(&P::Embed),
        "embeddings require parsed chunks, so no grammar means no Embed"
    );
}

#[test]
fn derived_plan_binds_the_evidence_and_carries_a_default_parse_policy() {
    let rust = evidence(
        &[("rust", 10), ("python", 2)],
        &["rust", "python"],
        false,
        &ALL,
    );
    let plan = derive_attunement_plan(&predecessor(), &rust, id("attune-derived"), "3".repeat(64))
        .unwrap();
    assert_eq!(plan.id.as_str(), "attune-derived");
    assert_eq!(plan.source_sha256, "3".repeat(64));
    let parse = plan.phases.iter().find(|s| s.phase == P::Parse).unwrap();
    assert_eq!(parse.estimated_items, 12);
    assert_eq!(parse.estimated_input_bytes, rust.content_bytes);
    assert_eq!(
        parse.configuration_sha256,
        attunement_capability_configuration_sha256(P::Parse, 1, &rust.evidence_sha256).unwrap()
    );
    let policy = parse
        .parse_policy
        .as_ref()
        .expect("default policy for bound grammars");
    let extensions: Vec<&str> = policy
        .bindings
        .iter()
        .map(|b| b.extension.as_str())
        .collect();
    assert_eq!(extensions, vec!["py", "pyi", "rs"]);
    for (index, step) in plan.phases.iter().enumerate() {
        assert_eq!(usize::from(step.sequence), index + 1);
        assert!(step.parse_policy.is_none() || step.phase == P::Parse);
    }
    // Different evidence, same selection, different digests: the chain cannot be reused.
    let other = evidence(
        &[("rust", 11), ("python", 2)],
        &["rust", "python"],
        false,
        &ALL,
    );
    let plan2 =
        derive_attunement_plan(&predecessor(), &other, id("attune-derived"), "3".repeat(64))
            .unwrap();
    assert_ne!(
        plan.phases[0].configuration_sha256,
        plan2.phases[0].configuration_sha256
    );
}

#[test]
fn evidence_rejects_inconsistent_observations() {
    let mut bad = evidence(&[("rust", 10)], &["rust"], false, &ALL);
    bad.bound_grammars.insert(id("go"));
    bad.evidence_sha256 = bad.digest().unwrap();
    assert!(
        bad.validate().is_err(),
        "a grammar bound to an undetected language"
    );
    let mut tampered = evidence(&[("rust", 10)], &["rust"], false, &ALL);
    tampered.files += 1;
    assert!(tampered.validate().is_err(), "digest no longer matches");
    let mut unavailable = evidence(
        &[("rust", 10)],
        &["rust"],
        false,
        &[P::Connect, P::Inventory],
    );
    unavailable.available.remove(&P::Inventory);
    unavailable.evidence_sha256 = unavailable.digest().unwrap();
    assert!(
        unavailable.validate().is_err(),
        "inventory must be available"
    );
}

#[test]
fn derived_preview_validates_only_against_the_plan_its_evidence_produces() {
    let old = predecessor();
    let job = id("job-predecessor");
    let rust = evidence(
        &[("rust", 10)],
        &["rust"],
        false,
        &[P::Connect, P::Inventory, P::Parse],
    );
    let source = "4".repeat(64);
    let seed = attunement_replan_seed(&job, 3, &old.plan_sha256, 1, &source).unwrap();
    let plan_id = id(&format!("attune-{}", &seed[..24]));
    let plan = derive_attunement_plan(&old, &rust, plan_id, source).unwrap();
    assert_eq!(
        plan.phases.iter().map(|s| s.phase).collect::<Vec<_>>(),
        vec![P::Connect, P::Inventory, P::Parse]
    );
    let preview = AttunementReplanPreview {
        predecessor_job_id: job.clone(),
        predecessor_revision: 3,
        generation: 1,
        proposal_sha256: attunement_replan_preview_sha256(&job, 3, 1, &plan).unwrap(),
        plan: plan.clone(),
        evidence: Some(rust.clone()),
    };
    preview.validate_for(&job, 3, &old).unwrap();

    // A plan that claims more than the evidence derives is refused.
    let mut forged = preview.clone();
    forged.plan.phases.push(AttunementPhasePlan {
        phase: P::Verify,
        sequence: 4,
        description: "forged verify".into(),
        configuration_sha256: "b".repeat(64),
        estimated_items: 1,
        estimated_input_bytes: 0,
        parse_policy: None,
    });
    forged.plan.plan_sha256 = attunement_plan_sha256(&forged.plan).unwrap();
    forged.proposal_sha256 = attunement_replan_preview_sha256(&job, 3, 1, &forged.plan).unwrap();
    assert!(forged.validate_for(&job, 3, &old).is_err());

    // The same plan without its evidence is not a plain rebind of the predecessor.
    let mut unexplained = preview.clone();
    unexplained.evidence = None;
    assert!(unexplained.validate_for(&job, 3, &old).is_err());

    // Evidence that does not match the plan it accompanies is refused.
    let mut mismatched = preview;
    let mut stale = rust;
    stale.files += 5;
    stale.evidence_sha256 = stale.digest().unwrap();
    mismatched.evidence = Some(stale);
    assert!(mismatched.validate_for(&job, 3, &old).is_err());
}
