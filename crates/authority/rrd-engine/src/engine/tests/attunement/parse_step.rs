//! The Parse Step on an installed project: facts are committed with the job's
//! transition, read back through rrflowQL, survive a restart, and a batch limit
//! spreads the work over several Steps.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn policy(max_files: u64) -> rrd_contract::SourceParsePolicy {
    rrd_contract::SourceParsePolicy {
        format_version: 1,
        bindings: vec![rrd_contract::SourceParseBinding {
            extension: "rs".into(),
            language: CanonicalId::new("rust").unwrap(),
        }],
        max_files,
        max_file_bytes: 1_048_576,
        max_input_bytes: 4_194_304,
        max_nodes: 10_000,
        max_depth: 64,
        max_output_bytes: 1_048_576,
        max_commit_bytes: 262_144,
        max_elapsed_ms: 10_000,
        max_memory_bytes: 268_435_456,
    }
}

/// An installed project with two Rust files, its job stepped to Parse.
fn at_parse(max_files: u64) -> (Installed, RrdEngine, SessionLease, AttunementJob) {
    at_parse_with(policy(max_files), |root| {
        std::fs::write(root.join("alpha.rs"), b"fn alpha() {}\n").unwrap();
        std::fs::write(root.join("beta.rs"), b"fn beta() {\n    alpha();\n}\n").unwrap();
    })
}

/// An installed project written by `write`, its job stepped to Parse.
fn at_parse_with(
    policy: rrd_contract::SourceParsePolicy,
    write: impl FnOnce(&std::path::Path),
) -> (Installed, RrdEngine, SessionLease, AttunementJob) {
    at_parse_configured(policy, None, write)
}

/// As `at_parse_with`, installed with an engram configuration file.
fn at_parse_configured(
    policy: rrd_contract::SourceParsePolicy,
    configuration: Option<&std::path::Path>,
    write: impl FnOnce(&std::path::Path),
) -> (Installed, RrdEngine, SessionLease, AttunementJob) {
    let proposal = crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: [
            Phase::Connect,
            Phase::Inventory,
            Phase::Parse,
            Phase::Verify,
        ]
        .into_iter()
        .map(|phase| crate::AttunementProposalStep {
            phase,
            description: format!("{phase:?}"),
            estimated_items: 0,
            estimated_input_bytes: 0,
            parse_policy: (phase == Phase::Parse).then(|| policy.clone()),
        })
        .collect(),
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 30_000,
    };
    let (installed, engine, session) = Installed::with_configuration(
        InstallationTargetKind::ExistingProject,
        Some(&proposal),
        write,
        configuration,
    );
    let started = call(
        &engine,
        &session,
        &installed.start(60_000),
        "parse-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "parse-connect",
        BASE + 2_000,
    )
    .unwrap();
    let inventoried = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "parse-inventory",
        BASE + 3_000,
    )
    .unwrap();
    assert_eq!(inventoried.status.job.current_phase, Some(Phase::Parse));
    (installed, engine, session, inventoried.status.job)
}

/// Every `(name, role)` of the `source-symbol` records known now.
fn symbols(engine: &RrdEngine, session: &SessionLease, at: u64) -> BTreeSet<(String, String)> {
    let result = engine
        .execute_query(
            &session.session_id,
            &session.token,
            &rrd_contract::ExecuteQuery {
                scope: format!("instance:{}", engine.instance_id()),
                query: format!(
                    "FROM record:source-symbol AT VALID {at} KNOWN HEAD PROJECT name, role"
                ),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at,
            "request-symbol-query",
            "operation-symbol-query",
        )
        .unwrap();
    result
        .rows
        .iter()
        .map(|row| {
            let text = |key: &str| match row.values.get(key) {
                Some(rrd_contract::QueryValue::String(value)) => value.clone(),
                other => panic!("{key} is {other:?}"),
            };
            (text("name"), text("role"))
        })
        .collect()
}

fn expected() -> BTreeSet<(String, String)> {
    [
        ("alpha", "definition"),
        ("beta", "definition"),
        ("alpha", "reference"),
    ]
    .into_iter()
    .map(|(name, role)| (name.into(), role.into()))
    .collect()
}

#[test]
fn parse_commits_facts_with_its_checkpoint_and_they_survive_a_restart() {
    let (installed, engine, session, job) = at_parse(64);
    let parsed = call(&engine, &session, &step(&job), "parse-step", BASE + 4_000).unwrap();
    let job = parsed.status.job;
    let checkpoint = job.checkpoints.last().unwrap();
    assert_eq!(checkpoint.phase, Phase::Parse);
    assert_eq!(job.current_phase, Some(Phase::Verify));
    assert_eq!(job.state, State::Running);
    assert!(symbols(&engine, &session, BASE + 5_000).is_superset(&expected()));

    drop(engine);
    let engine = installed.reopen(BASE + 6_000);
    let session = installed.session(&engine, "parse-reopened", BASE + 6_100);
    assert!(symbols(&engine, &session, BASE + 6_200).is_superset(&expected()));
}

#[test]
fn a_file_limit_spreads_parse_over_steps_without_a_checkpoint_until_done() {
    let (_installed, engine, session, job) = at_parse(1);
    let first = call(&engine, &session, &step(&job), "parse-first", BASE + 4_000).unwrap();
    let job = first.status.job;
    assert_eq!(
        job.current_phase,
        Some(Phase::Parse),
        "one file is still waiting"
    );
    assert_eq!(job.checkpoints.last().unwrap().phase, Phase::Inventory);
    assert_eq!(symbols(&engine, &session, BASE + 4_500).len(), 1);

    let second = call(&engine, &session, &step(&job), "parse-second", BASE + 5_000).unwrap();
    let job = second.status.job;
    assert_eq!(job.checkpoints.last().unwrap().phase, Phase::Parse);
    assert_eq!(job.current_phase, Some(Phase::Verify));
    assert!(symbols(&engine, &session, BASE + 5_500).is_superset(&expected()));
}

/// Every `source-parse` state's `(path, status)` known now.
fn parse_states(engine: &RrdEngine, session: &SessionLease, at: u64) -> BTreeSet<(String, String)> {
    let result = engine
        .execute_query(
            &session.session_id,
            &session.token,
            &rrd_contract::ExecuteQuery {
                scope: format!("instance:{}", engine.instance_id()),
                query: format!(
                    "FROM record:source-parse AT VALID {at} KNOWN HEAD PROJECT path, status"
                ),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at,
            "request-state-query",
            "operation-state-query",
        )
        .unwrap();
    result
        .rows
        .iter()
        .map(|row| {
            let text = |key: &str| match row.values.get(key) {
                Some(rrd_contract::QueryValue::String(value)) => value.clone(),
                other => panic!("{key} is {other:?}"),
            };
            (text("path"), text("status"))
        })
        .collect()
}

/// The 2026-11-12 real run on this repository: the default policy gathered a
/// batch whose physical encoding passed the 16 MiB write-batch ceiling, the
/// store refused it, and every later Step retried the same batch. Parse now
/// measures the batch the store would write and publishes what fits, so a
/// project whose facts far exceed one batch finishes with every file parsed.
#[test]
fn facts_larger_than_one_physical_batch_are_published_across_steps() {
    const FILES: usize = 40;
    let mut policy = policy(1_000);
    policy.max_input_bytes = 64 * 1_048_576;
    policy.max_nodes = 1_000_000;
    policy.max_depth = 256;
    policy.max_output_bytes = 16 * 1_048_576;
    policy.max_commit_bytes = 16 * 1_048_576;
    policy.max_elapsed_ms = 60_000;
    let (_installed, engine, session, mut job) = at_parse_with(policy, |root| {
        let filler = "x".repeat(2_000);
        for file in 0..FILES {
            let mut text = String::new();
            for function in 0..150 {
                text.push_str(&format!(
                    "fn f_{file}_{function}() {{\n    let text = \"{filler}\";\n    g(text);\n}}\n"
                ));
            }
            std::fs::write(root.join(format!("big_{file:02}.rs")), text).unwrap();
        }
    });
    let mut steps = 0;
    while job.current_phase == Some(Phase::Parse) {
        steps += 1;
        assert!(steps <= 40, "Parse did not finish in 40 Steps");
        job = call(
            &engine,
            &session,
            &step(&job),
            &format!("parse-big-{steps}"),
            job.updated_at_unix_ms + 1,
        )
        .unwrap_or_else(|error| panic!("Parse Step {steps} was refused: {error:?}"))
        .status
        .job;
    }
    assert!(
        steps > 1,
        "the facts were meant to need more than one batch"
    );
    assert_eq!(job.checkpoints.last().unwrap().phase, Phase::Parse);
    let states = parse_states(&engine, &session, job.updated_at_unix_ms + 1);
    assert_eq!(states.len(), FILES);
    assert!(
        states.iter().all(|(_, status)| status == "parsed"),
        "{states:?}"
    );
}

/// Step a job at Parse until Parse has its checkpoint; returns the job and the Steps taken.
fn finish_parse(
    engine: &RrdEngine,
    session: &SessionLease,
    mut job: AttunementJob,
    label: &str,
) -> (AttunementJob, usize) {
    let mut steps = 0;
    while job.current_phase == Some(Phase::Parse) {
        steps += 1;
        assert!(steps <= 60, "{label}: Parse did not finish in 60 Steps");
        job = call(
            engine,
            session,
            &step(&job),
            &format!("{label}-parse-{steps}"),
            job.updated_at_unix_ms + 1,
        )
        .unwrap_or_else(|error| panic!("{label}: Parse Step {steps} was refused: {error:?}"))
        .status
        .job;
    }
    assert_eq!(job.checkpoints.last().unwrap().phase, Phase::Parse);
    (job, steps)
}

/// Replan a job against the project as it is now and step its successor to Parse.
fn replan_to_parse(
    engine: &RrdEngine,
    session: &SessionLease,
    done: &AttunementJob,
    label: &str,
) -> AttunementJob {
    let at = done.updated_at_unix_ms + 10;
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &done.id,
            done.revision,
            at,
            &format!("request-{label}-preview"),
            &format!("operation-{label}-preview"),
        )
        .unwrap();
    let successor = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context(&format!("{label}-accept")),
            at + 1,
        )
        .unwrap()
        .successor
        .job;
    let mut job = call(
        engine,
        session,
        &AttunementCommand::Start {
            job_id: successor.id.clone(),
            plan_sha256: successor.plan_sha256.clone(),
            expected_revision: successor.revision,
            lease_ms: 60_000,
        },
        &format!("{label}-start"),
        successor.updated_at_unix_ms + 1,
    )
    .unwrap()
    .status
    .job;
    for phase in ["connect", "inventory"] {
        job = call(
            engine,
            session,
            &step(&job),
            &format!("{label}-{phase}"),
            job.updated_at_unix_ms + 1,
        )
        .unwrap()
        .status
        .job;
    }
    assert_eq!(job.current_phase, Some(Phase::Parse));
    job
}

/// Files that leave a project are retired within the same commit bound that
/// publishing them obeyed. Before this, every departed file's retirement went
/// into one commit whatever its size: a large deletion broke the selected
/// policy's commit bound, and one larger than the engram's transaction
/// admission was refused on every Step, so the job could never finish.
#[test]
fn a_deletion_larger_than_one_commit_retires_across_steps() {
    const REMOVED: usize = 40;
    const ADMISSION: &str = "transaction_admission_bytes = 524288";
    let directory = tempfile::tempdir().unwrap();
    let configuration = directory.path().join("small-admission.toml");
    let default = include_str!("../../../../assets/install/default-engram-configuration-v3.toml");
    let small = default.replace("transaction_admission_bytes = 67108864", ADMISSION);
    assert!(
        small.contains(ADMISSION),
        "the default changed its admission line"
    );
    std::fs::write(&configuration, small).unwrap();

    let (installed, engine, session, job) =
        at_parse_configured(policy(1_000), Some(&configuration), |root| {
            std::fs::write(root.join("kept.rs"), b"fn kept() {}\n").unwrap();
            for file in 0..REMOVED {
                let mut text = String::new();
                for function in 0..100 {
                    text.push_str(&format!(
                        "fn removed_{file}_{function}() {{\n    kept();\n}}\n"
                    ));
                }
                std::fs::write(root.join(format!("removed_{file:02}.rs")), text).unwrap();
            }
        });
    let (first, _) = finish_parse(&engine, &session, job, "delete-first");
    assert_eq!(
        parse_states(&engine, &session, first.updated_at_unix_ms + 1).len(),
        REMOVED + 1
    );

    for file in 0..REMOVED {
        std::fs::remove_file(installed.root.path().join(format!("removed_{file:02}.rs"))).unwrap();
    }
    let successor = replan_to_parse(&engine, &session, &first, "delete-second");
    let (second, steps) = finish_parse(&engine, &session, successor, "delete-second");
    assert!(
        steps > 1,
        "the retirements were meant to need more than one commit"
    );

    let at = second.updated_at_unix_ms + 1;
    assert_eq!(
        parse_states(&engine, &session, at),
        BTreeSet::from([("kept.rs".to_owned(), "parsed".to_owned())]),
        "Parse finished before every departed file was retired"
    );
    let names: BTreeSet<String> = symbols(&engine, &session, at)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, BTreeSet::from(["kept".to_owned()]));
}

/// A replan parses only what changed: an edited file is parsed again, a
/// deleted file's facts and state are retired, and an unchanged file's state
/// is not written again.
#[test]
fn an_edit_reparses_only_the_edited_file_and_a_deletion_retires_its_facts() {
    let (installed, engine, session, job) = at_parse_with(policy(64), |root| {
        std::fs::write(root.join("kept.rs"), b"fn kept() {}\n").unwrap();
        std::fs::write(root.join("edited.rs"), b"fn before_edit() {}\n").unwrap();
        std::fs::write(root.join("removed.rs"), b"fn removed() {}\n").unwrap();
    });
    let (first, _) = finish_parse(&engine, &session, job, "edit-first");
    let scope = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    let before = engine.storage.runtime().read_stamp(&scope).unwrap();

    std::fs::write(
        installed.root.path().join("edited.rs"),
        b"fn after_edit() {}\n",
    )
    .unwrap();
    std::fs::remove_file(installed.root.path().join("removed.rs")).unwrap();
    let successor = replan_to_parse(&engine, &session, &first, "edit-second");
    let (second, _) = finish_parse(&engine, &session, successor, "edit-second");
    let at = second.updated_at_unix_ms + 1;

    let definitions: BTreeSet<String> = symbols(&engine, &session, at)
        .into_iter()
        .filter(|(_, role)| role == "definition")
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        definitions,
        BTreeSet::from(["after_edit".to_owned(), "kept".to_owned()]),
        "stale facts must be retired"
    );
    assert_eq!(
        parse_states(&engine, &session, at),
        BTreeSet::from([
            ("edited.rs".to_owned(), "parsed".to_owned()),
            ("kept.rs".to_owned(), "parsed".to_owned()),
        ])
    );

    // Every live `source-parse` state the second job wrote, from the change feed.
    let mut written = BTreeSet::new();
    let mut after_cursor = before.commit_cursor;
    loop {
        let page = engine
            .read_changefeed(
                &session.session_id,
                &session.token,
                &rrd_contract::ReadChangefeed {
                    scope: scope.as_str().to_owned(),
                    after_cursor,
                    limit: rrd_contract::MAX_CHANGEFEED_PAGE,
                },
                at,
                "request-parse-changes",
                "operation-parse-changes",
            )
            .unwrap();
        for change in &page.changes {
            after_cursor = after_cursor.max(change.cursor);
            if let rrd_contract::ChangeMutationSnapshot::Data {
                mutation:
                    rrd_contract::TransactionMutation::PutRecord {
                        reference,
                        valid_to: None,
                        properties,
                        ..
                    },
            } = &change.mutation
            {
                if reference.kind.as_str() == "source-parse" {
                    written.insert(format!("{:?}", properties.get("path")));
                }
            }
        }
        if !page.has_more {
            break;
        }
    }
    assert_eq!(
        written,
        BTreeSet::from([format!(
            "{:?}",
            Some(&rrd_contract::QueryValue::String("edited.rs".into()))
        )]),
        "only the edited file's state is written again"
    );
}

/// Rows and the selected access path of one rrflowQL query.
fn query(
    engine: &RrdEngine,
    session: &SessionLease,
    at: u64,
    text: &str,
) -> (String, Vec<BTreeMap<String, rrd_contract::QueryValue>>) {
    let result = engine
        .execute_query(
            &session.session_id,
            &session.token,
            &rrd_contract::ExecuteQuery {
                scope: format!("instance:{}", engine.instance_id()),
                query: text.into(),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at,
            &format!("request-query-{at}"),
            &format!("operation-query-{at}"),
        )
        .unwrap();
    let selected = result
        .plan
        .candidates
        .iter()
        .find(|candidate| candidate.selected)
        .map(|candidate| candidate.name.clone())
        .unwrap_or_default();
    (
        selected,
        result.rows.into_iter().map(|row| row.values).collect(),
    )
}

/// Parsed facts are read through the indexes Parse declares before it writes
/// them: a symbol by name, and code by the words of its identifiers.
#[test]
fn parsed_facts_are_read_through_their_commit_indexes() {
    let (installed, engine, session, job) = at_parse(64);
    call(&engine, &session, &step(&job), "parse-step", BASE + 4_000).unwrap();
    let check = |engine: &RrdEngine, session: &SessionLease, at: u64| {
        let (selected, rows) = query(
            engine,
            session,
            at,
            &format!("FROM record:source-symbol AT VALID {at} KNOWN HEAD WHERE name = \"alpha\" PROJECT path, role"),
        );
        assert_eq!(selected, "commit_index:source-symbol-name");
        let found = rows
            .iter()
            .map(|row| {
                let text = |key: &str| match row.get(key) {
                    Some(rrd_contract::QueryValue::String(value)) => value.clone(),
                    other => panic!("{key} is {other:?}"),
                };
                (text("path"), text("role"))
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            found,
            BTreeSet::from([
                ("alpha.rs".to_owned(), "definition".to_owned()),
                ("beta.rs".to_owned(), "reference".to_owned()),
            ])
        );
        let (selected, rows) = query(
            engine,
            session,
            at + 1,
            &format!(
                "FROM record:source-chunk AT VALID {} KNOWN HEAD WHERE text MATCH \"beta\" LIMIT 5",
                at + 1
            ),
        );
        assert_eq!(selected, "commit_index:source-chunk-text");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("path"),
            Some(&rrd_contract::QueryValue::String("beta.rs".into()))
        );
    };
    check(&engine, &session, BASE + 5_000);
    drop(engine);
    let engine = installed.reopen(BASE + 6_000);
    let session = installed.session(&engine, "parse-indexes-reopened", BASE + 6_100);
    check(&engine, &session, BASE + 6_200);
}

/// A question is answered from parsed code through the chunk postings, at one
/// read stamp, without reading every fact.
#[test]
fn context_answers_from_parsed_code_through_commit_postings() {
    let (_installed, engine, session, job) = at_parse(64);
    let parsed = call(&engine, &session, &step(&job), "parse-step", BASE + 4_000).unwrap();
    let at = parsed.status.job.updated_at_unix_ms + 1;
    let packet = engine
        .assemble_context(
            &session.session_id,
            &session.token,
            &rrd_contract::AssembleContext {
                scope: format!("instance:{}", engine.instance_id()),
                query: "where is beta defined".into(),
                valid_at: at,
                seeds: Vec::new(),
                max_graph_depth: 0,
                max_items: 5,
                max_output_bytes: 65_536,
                max_storage_keys: 100_000,
            },
            at,
            "request-context-parsed",
            "operation-context-parsed",
        )
        .unwrap();
    let lexical = packet
        .plan
        .stages
        .iter()
        .find(|stage| stage.kind == rrd_contract::ContextPlanStageKind::Lexical)
        .unwrap();
    assert_eq!(
        lexical.access_path,
        Some(rrd_contract::ContextAccessPath::CommitBm25),
        "{}",
        lexical.reason
    );
    let first = packet.items.first().expect("the question found code");
    assert_eq!(
        first.values.get("path"),
        Some(&rrd_contract::QueryValue::String("beta.rs".into())),
        "{:?}",
        packet.items
    );
    assert!(first
        .evidence
        .iter()
        .any(|evidence| evidence.source == "record:source-chunk:commit_bm25"));
}

/// Under a sealed follow policy the engine keeps its project attuned with no
/// client driving it: a fresh install is inventoried and parsed in one pass,
/// an unchanged project costs only a fingerprint, and an edit is relearned.
#[test]
fn a_followed_project_is_attuned_and_relearned_by_the_engine() {
    let directory = tempfile::tempdir().unwrap();
    let configuration = directory.path().join("follow.toml");
    std::fs::write(
        &configuration,
        format!(
            "{}\n[attunement_follow]\ninterval_ms = 1000\n",
            include_str!("../../../../assets/install/default-engram-configuration-v3.toml")
        ),
    )
    .unwrap();
    let (installed, engine, _session) = Installed::with_configuration(
        InstallationTargetKind::ExistingProject,
        None,
        |root| {
            std::fs::write(root.join("alpha.rs"), b"fn alpha() {}\n").unwrap();
            std::fs::write(root.join("beta.rs"), b"fn beta() {\n    alpha();\n}\n").unwrap();
        },
        Some(&configuration),
    );
    // The follower reads time from a clock that moves forward from the
    // fixture's base with real time, as the host clock does after an install.
    struct Advancing(std::time::Instant);
    impl Advancing {
        fn at(&self) -> u64 {
            BASE + 10_000 + self.0.elapsed().as_millis() as u64
        }
    }
    impl crate::engine::clock::ClockSource for Advancing {
        fn observe(
            &self,
        ) -> std::result::Result<
            crate::ClockObservation,
            crate::engine::clock::ClockObservationFailure,
        > {
            crate::engine::clock::FixedClock::at(self.at()).observe()
        }
    }
    let clock = Advancing(std::time::Instant::now());
    let now = || clock.at();
    let names = |engine: &RrdEngine, session: &SessionLease| {
        let at = now();
        symbols(engine, session, at)
            .into_iter()
            .filter(|(_, role)| role == "definition")
            .map(|(name, _)| name)
            .collect::<BTreeSet<_>>()
    };

    let first = engine.follow_attunement_with_clock(&clock, None).unwrap();
    assert!(
        matches!(
            first.outcome,
            crate::FollowOutcome::Attuned {
                state: State::Succeeded,
                ..
            }
        ),
        "{first:?}"
    );
    let session = installed.session(&engine, "follow-reader", now());
    assert_eq!(
        names(&engine, &session),
        BTreeSet::from(["alpha".to_owned(), "beta".to_owned()])
    );

    let parsed_by = |engine: &RrdEngine, session: &SessionLease| {
        let at = now();
        let (_, rows) = query(
            engine,
            session,
            at,
            &format!("FROM record:source-parse AT VALID {at} KNOWN HEAD PROJECT path, job_id"),
        );
        rows.into_iter()
            .map(|row| {
                let text = |key: &str| match row.get(key) {
                    Some(rrd_contract::QueryValue::String(value)) => value.clone(),
                    other => panic!("{key} is {other:?}"),
                };
                (text("path"), text("job_id"))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before_edit = parsed_by(&engine, &session);
    let unchanged = engine
        .follow_attunement_with_clock(&clock, first.fingerprint.as_deref())
        .unwrap();
    assert_eq!(unchanged.outcome, crate::FollowOutcome::Unchanged);

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(installed.root.path().join("alpha.rs"), b"fn gamma() {}\n").unwrap();
    let edited = engine
        .follow_attunement_with_clock(&clock, unchanged.fingerprint.as_deref())
        .unwrap();
    assert!(
        matches!(
            edited.outcome,
            crate::FollowOutcome::Attuned {
                state: State::Succeeded,
                ..
            }
        ),
        "{edited:?}"
    );
    assert_eq!(
        names(&engine, &session),
        BTreeSet::from(["beta".to_owned(), "gamma".to_owned()])
    );
    // Only the edited file was parsed again.
    let after_edit = parsed_by(&engine, &session);
    assert_eq!(after_edit["beta.rs"], before_edit["beta.rs"]);
    assert_ne!(after_edit["alpha.rs"], before_edit["alpha.rs"]);
}
