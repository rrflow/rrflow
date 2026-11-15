use rrd_contract::*;

#[test]
fn installed_jobs_have_a_public_read_only_discovery_operation() {
    let catalogue = endpoint_catalogue();
    let descriptor = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "attunement-job-list")
        .expect("an installed client must discover jobs without deriving engine IDs");
    assert_eq!(descriptor.method, HttpMethod::Post);
    assert_eq!(
        descriptor.authentication,
        EndpointAuthentication::SessionBearer
    );
    assert!(!descriptor.mutation);
    assert_eq!(descriptor.path, "/v1/attunement/jobs/list");
    assert_eq!(descriptor.request_type, "ListAttunementJobs");
    assert_eq!(descriptor.response_type, "AttunementJobPage");
}

// Protocol correctness fixture only; these values are not model or training evidence.
fn fixture() -> (ListAttunementJobs, AttunementJobPage) {
    let request = ListAttunementJobs {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        after_job_id: None,
        limit: 2,
    };
    let summary = |generation, predecessor_job_id| AttunementJobSummary {
        job_id: CanonicalId::new(format!("job-{generation}")).unwrap(),
        predecessor_job_id,
        generation,
        plan_id: CanonicalId::new(format!("plan-{generation}")).unwrap(),
        plan_sha256: "a".repeat(64),
        source_sha256: "b".repeat(64),
        revision: 1,
        state: AttunementJobState::Pending,
        checkpoint_count: 0,
        next_phase: Some(AttunementPhase::Connect),
        created_at_unix_ms: 10,
        updated_at_unix_ms: 10,
    };
    let first = summary(0, None);
    let second = summary(1, Some(first.job_id.clone()));
    let page = AttunementJobPage {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        target: InstalledEngramIdentity {
            project_id: CanonicalId::new("project").unwrap(),
            engram_id: CanonicalId::new("engram").unwrap(),
            instance_id: CanonicalId::new("instance").unwrap(),
        },
        control_sequence: 1,
        observed_at_unix_ms: 20,
        after_job_id: None,
        next_after_job_id: Some(second.job_id.clone()),
        jobs: vec![first, second],
        source_verification: AttunementSourceVerification::NotChecked,
    };
    (request, page)
}

#[test]
fn validates_initial_continuation_and_exhausted_pages() {
    let (request, page) = fixture();
    page.validate_for(&request, &page.target.instance_id)
        .unwrap();
    let continued = ListAttunementJobs {
        after_job_id: Some(page.jobs[0].job_id.clone()),
        ..request
    };
    let mut next = page.clone();
    next.after_job_id = continued.after_job_id.clone();
    next.jobs.remove(0);
    next.next_after_job_id = None;
    next.validate_for(&continued, &page.target.instance_id)
        .unwrap();
    next.jobs.clear();
    next.validate_for(&continued, &page.target.instance_id)
        .unwrap();
}

#[test]
fn rejects_response_identity_lineage_time_and_cursor_mismatches() {
    let (request, page) = fixture();
    assert!(page
        .validate_for(&request, &CanonicalId::new("foreign-instance").unwrap())
        .is_err());
    let changes: [fn(&mut AttunementJobPage); 18] = [
        |p| p.contract_version += 1,
        |p| p.control_sequence = 0,
        |p| p.after_job_id = Some(p.jobs[0].job_id.clone()),
        |p| p.jobs.clear(),
        |p| p.jobs.push(p.jobs[1].clone()),
        |p| p.jobs[1].job_id = p.jobs[0].job_id.clone(),
        |p| p.jobs[1].predecessor_job_id = None,
        |p| p.jobs[0].generation = 1,
        |p| p.jobs[1].generation = 3,
        |p| p.jobs[0].revision = 0,
        |p| p.jobs[0].checkpoint_count = u16::MAX,
        |p| p.jobs[0].created_at_unix_ms = 0,
        |p| p.jobs[0].updated_at_unix_ms = 9,
        |p| p.jobs[0].updated_at_unix_ms = 21,
        |p| p.jobs[0].source_sha256 = "invalid".into(),
        |p| p.jobs[0].plan_sha256 = "invalid".into(),
        |p| p.next_after_job_id = Some(p.jobs[0].job_id.clone()),
        |p| {
            p.jobs.remove(0);
        },
    ];
    for (index, change) in changes.into_iter().enumerate() {
        let mut invalid = page.clone();
        change(&mut invalid);
        assert!(
            invalid
                .validate_for(&request, &page.target.instance_id)
                .is_err(),
            "mutation {index}"
        );
    }
}

#[test]
fn rejects_unbounded_requests_and_unknown_wire_fields() {
    let (request, page) = fixture();
    for limit in [0, MAX_ATTUNEMENT_JOB_PAGE + 1, u16::MAX] {
        assert!(ListAttunementJobs {
            limit,
            ..request.clone()
        }
        .validate()
        .is_err());
    }
    assert!(ListAttunementJobs {
        contract_version: ATTUNEMENT_CONTRACT_VERSION + 1,
        ..request.clone()
    }
    .validate()
    .is_err());
    let mut wire = serde_json::to_value(&request).unwrap();
    wire["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ListAttunementJobs>(wire).is_err());
    let mut wire = serde_json::to_value(&page).unwrap();
    wire["source_verification"] = serde_json::json!("verified");
    assert!(serde_json::from_value::<AttunementJobPage>(wire).is_err());
}

#[test]
fn maximum_page_has_bounded_encoding_and_projected_limits() {
    let (mut request, mut page) = fixture();
    request.limit = MAX_ATTUNEMENT_JOB_PAGE;
    let template = page.jobs[0].clone();
    page.jobs.clear();
    let mut previous = None;
    for generation in 0..u64::from(MAX_ATTUNEMENT_JOB_PAGE) {
        let job_id = CanonicalId::new(format!("{generation:0128}")).unwrap();
        let plan_id = CanonicalId::new(format!("p{generation:0127}")).unwrap();
        page.jobs.push(AttunementJobSummary {
            job_id: job_id.clone(),
            plan_id,
            generation,
            predecessor_job_id: previous,
            ..template.clone()
        });
        previous = Some(job_id);
    }
    page.next_after_job_id = previous;
    page.validate_for(&request, &page.target.instance_id)
        .unwrap();
    assert!(serde_json::to_vec(&page).unwrap().len() <= MAX_ATTUNEMENT_JOB_PAGE_BYTES);
    let request_schema = serde_json::to_value(schemars::schema_for!(ListAttunementJobs)).unwrap();
    assert_eq!(request_schema["properties"]["limit"]["minimum"], 1);
    assert_eq!(
        request_schema["properties"]["limit"]["maximum"],
        MAX_ATTUNEMENT_JOB_PAGE
    );
    let page_schema = serde_json::to_value(schemars::schema_for!(AttunementJobPage)).unwrap();
    assert_eq!(
        page_schema["properties"]["jobs"]["maxItems"],
        MAX_ATTUNEMENT_JOB_PAGE
    );
}
