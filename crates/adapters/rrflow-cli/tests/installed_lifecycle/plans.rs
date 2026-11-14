//! Installed plans, explicit digests and source read choices.

use super::*;

pub(super) fn installed_plan_accepts_selected_attunement_and_rejects_tampering() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "selected project\n").unwrap();
    let before = inventory(&project);
    let schema = invoke(&["install", "attunement-schema"]);
    assert_success(&schema, "attunement schema");
    let schema: serde_json::Value = serde_json::from_slice(&schema.stdout).unwrap();
    assert_eq!(
        schema["properties"]["format_version"]["const"],
        rrd_engine::ATTUNEMENT_PROPOSAL_FORMAT_VERSION
    );
    assert_eq!(schema["properties"]["steps"]["minItems"], 1);
    assert_eq!(
        schema["properties"]["steps"]["maxItems"],
        rrd_contract::MAX_ATTUNEMENT_PHASES
    );
    assert_eq!(
        schema["$defs"]["AttunementProposalStep"]["properties"]["description"]["maxLength"],
        1024
    );
    assert_eq!(
        inventory(&project),
        before,
        "schema discovery wrote project state"
    );
    let identity = invoke(&["install", "identity"]);
    assert_success(&identity, "identity");
    let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
    let proposal_path = root.path().join("attunement.json");
    let step = |phase: &str, config: &str| {
        serde_json::json!({
            "phase": phase, "description": config,
            "estimated_items": 1, "estimated_input_bytes": 0
        })
    };
    let proposal = serde_json::json!({
        "format_version": 1, "steps": [step("connect", "first"), step("connect", "second"), step("verify", "verify")],
        "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 1000
    });
    std::fs::write(&proposal_path, serde_json::to_vec(&proposal).unwrap()).unwrap();
    let plan = invoke(&[
        "install",
        "plan",
        "--identity",
        identity["installation_identity"].as_str().unwrap(),
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--attunement-proposal",
        proposal_path.to_str().unwrap(),
    ]);
    assert_success(&plan, "selected install plan");
    assert_eq!(inventory(&project), before, "preview wrote project state");
    let preview: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(preview["attunement_proposal"], proposal);
    let phases = preview["attunement"]["phases"].as_array().unwrap();
    assert_eq!(phases.len(), 3);
    assert_eq!(phases[0]["phase"], "connect");
    assert_eq!(phases[1]["phase"], "connect");
    assert_eq!(phases[1]["sequence"], 2);
    assert_eq!(phases[2]["phase"], "verify");
    let digest = preview["installation"]["plan_sha256"].as_str().unwrap();
    let mut second = proposal.clone();
    second["steps"] = serde_json::json!([step("connect", "other"), step("verify", "verify")]);
    std::fs::write(&proposal_path, serde_json::to_vec(&second).unwrap()).unwrap();
    let another = invoke(&[
        "install",
        "plan",
        "--identity",
        identity["installation_identity"].as_str().unwrap(),
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--attunement-proposal",
        proposal_path.to_str().unwrap(),
    ]);
    assert_success(&another, "second selected plan");
    let another: serde_json::Value = serde_json::from_slice(&another.stdout).unwrap();
    assert_ne!(another["installation"]["plan_sha256"], digest);
    assert_eq!(another["attunement"]["phases"].as_array().unwrap().len(), 2);
    let mut reordered = proposal.clone();
    reordered["steps"] = serde_json::json!([
        step("inventory", "inventory"),
        step("connect", "connect"),
        step("verify", "verify")
    ]);
    std::fs::write(&proposal_path, serde_json::to_vec(&reordered).unwrap()).unwrap();
    let reordered_preview = invoke(&[
        "install",
        "plan",
        "--identity",
        identity["installation_identity"].as_str().unwrap(),
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--attunement-proposal",
        proposal_path.to_str().unwrap(),
    ]);
    assert_success(&reordered_preview, "reordered selected plan");
    let reordered_preview: serde_json::Value =
        serde_json::from_slice(&reordered_preview.stdout).unwrap();
    assert_eq!(
        reordered_preview["attunement"]["phases"][0]["phase"],
        "inventory"
    );
    assert_eq!(
        reordered_preview["attunement"]["phases"][1]["phase"],
        "connect"
    );
    assert_ne!(reordered_preview["installation"]["plan_sha256"], digest);
    assert_eq!(
        inventory(&project),
        before,
        "alternative previews wrote project state"
    );
    for invalid in [
        serde_json::json!({"format_version": 1, "steps": [step("unknown-effect", "x"), step("verify", "v")], "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 1}),
        serde_json::json!({"format_version": 1, "steps": [step("verify", "v"), step("connect", "x")], "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 1}),
        serde_json::json!({"format_version": 99, "steps": [step("verify", "v")], "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 1}),
        serde_json::json!({"format_version": 1, "steps": [step("connect", &"x".repeat(1025)), step("verify", "v")], "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 1}),
        serde_json::json!({"format_version": 1, "steps": vec![step("verify", "v"); rrd_contract::MAX_ATTUNEMENT_PHASES + 1], "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 1}),
    ] {
        std::fs::write(&proposal_path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let rejected = invoke(&[
            "install",
            "plan",
            "--identity",
            identity["installation_identity"].as_str().unwrap(),
            "--mode",
            "existing",
            "--project",
            project.to_str().unwrap(),
            "--attunement-proposal",
            proposal_path.to_str().unwrap(),
        ]);
        assert!(!rejected.status.success(), "invalid proposal was accepted");
        assert_eq!(
            inventory(&project),
            before,
            "invalid preview wrote project state"
        );
    }
    let path = root.path().join("preview.json");
    let mut tampered = preview.clone();
    tampered["attunement_proposal"]["steps"][0]["description"] = "changed".into();
    std::fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    let rejected = invoke(&[
        "install",
        "apply",
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--plan",
        path.to_str().unwrap(),
        "--expect",
        digest,
    ]);
    assert!(!rejected.status.success(), "changed proposal was accepted");
    assert_eq!(
        inventory(&project),
        before,
        "rejected proposal wrote project state"
    );
    for version in [None, Some(99)] {
        let mut invalid = preview.clone();
        if let Some(version) = version {
            invalid["preview_format_version"] = version.into();
        } else {
            invalid
                .as_object_mut()
                .unwrap()
                .remove("preview_format_version");
        }
        std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let rejected = invoke(&[
            "install",
            "apply",
            "--mode",
            "existing",
            "--project",
            project.to_str().unwrap(),
            "--plan",
            path.to_str().unwrap(),
            "--expect",
            digest,
        ]);
        assert!(
            !rejected.status.success(),
            "unknown preview version was accepted"
        );
        assert_eq!(
            inventory(&project),
            before,
            "invalid preview wrote project state"
        );
    }
    // Apply consumes the sealed preview rather than rereading the changed input file.
    std::fs::write(&proposal_path, b"no longer a proposal").unwrap();
    std::fs::write(&path, &plan.stdout).unwrap();
    let applied = invoke(&[
        "install",
        "apply",
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--plan",
        path.to_str().unwrap(),
        "--expect",
        digest,
    ]);
    assert_success(&applied, "apply selected plan");
    let verified = invoke(&["verify", "--project", project.to_str().unwrap()]);
    assert_success(&verified, "verify installed selected plan");
}

pub(super) fn install_requires_an_explicit_plan_digest_and_verify_is_read_only_by_shape() {
    let apply = invoke(&["install", "apply", "--project", "."]);
    assert!(!apply.status.success());
    let error = String::from_utf8(apply.stderr).unwrap();
    assert!(error.contains("--plan <PLAN>"));
    assert!(error.contains("--expect <EXPECT>"));

    let verify = invoke(&["verify", "--help"]);
    assert_success(&verify, "verify help");
    let help = String::from_utf8(verify.stdout).unwrap();
    assert!(help.contains("--level <LEVEL>"));
    assert!(help.contains("quick"));
    assert!(!help.contains("repair"));
    assert!(!help.contains("restore"));
}

pub(super) fn installed_engine_plans_applies_serves_discovers_authenticates_and_reopens() {
    installed_lifecycle("existing");
    // Parse runs only where a native supervisor contains its worker; on any
    // other platform the derived plan must not select it.
    installed_binary_replans_and_reads_receipt_after_forced_restart(cfg!(any(
        windows,
        target_os = "linux"
    )));
}

pub(super) fn fresh_installed_engine_survives_restart_and_restore() {
    installed_lifecycle("fresh");
}

pub(super) fn installed_source_read_choice_is_explicit_and_reopens() {
    let root = tempfile::tempdir().unwrap();
    for (name, opt_in) in [("default", false), ("source", true)] {
        let project = root.path().join(name);
        std::fs::create_dir(&project).unwrap();
        std::fs::write(project.join("source.txt"), "bounded source fixture\n").unwrap();
        std::fs::create_dir(project.join(".git")).unwrap();
        std::fs::write(
            project.join(".git/config"),
            b"[core]\nrepositoryFormatVersion=0\nignoreCase=false\n",
        )
        .unwrap();
        let project_arg = project.to_str().unwrap();
        let identity_output = invoke(&["install", "identity"]);
        assert_success(&identity_output, "allocate installation identity");
        let identity: serde_json::Value = serde_json::from_slice(&identity_output.stdout).unwrap();
        let identity = identity["installation_identity"].as_str().unwrap();
        let mut arguments = vec![
            "install",
            "plan",
            "--identity",
            identity,
            "--mode",
            "existing",
            "--project",
            project_arg,
        ];
        if opt_in {
            arguments.push("--allow-project-source-read");
        }
        let preview_output = invoke(&arguments);
        assert_success(&preview_output, "plan installation");
        let preview: serde_json::Value = serde_json::from_slice(&preview_output.stdout).unwrap();
        let grant_names: Vec<&str> = preview["installation"]["initial_grants"]
            .as_array()
            .unwrap()
            .iter()
            .map(|grant| grant.as_str().unwrap())
            .collect();
        assert_eq!(
            grant_names.contains(&"project_source_read"),
            opt_in,
            "the explicit choice must be the only source-read grant switch"
        );
        if opt_in {
            assert_eq!(preview["source_read_resource_scope"], "instance_prefix");
        } else {
            assert!(preview.get("source_read_resource_scope").is_none());
        }
        let plan_path = root.path().join(format!("{name}-plan.json"));
        std::fs::write(&plan_path, &preview_output.stdout).unwrap();
        let plan_arg = plan_path.to_str().unwrap();
        let expected = preview["installation"]["plan_sha256"].as_str().unwrap();
        let apply = invoke(&[
            "--json",
            "install",
            "apply",
            "--mode",
            "existing",
            "--project",
            project_arg,
            "--plan",
            plan_arg,
            "--expect",
            expected,
        ]);
        assert_success(&apply, "apply exact installation plan");
        let applied: serde_json::Value = serde_json::from_slice(&apply.stdout).unwrap();
        let reopened = invoke(&[
            "--json",
            "verify",
            "--project",
            project_arg,
            "--level",
            "quick",
        ]);
        assert_success(&reopened, "reopen and verify installed grant binding");
        let report: serde_json::Value = serde_json::from_slice(&reopened.stdout).unwrap();
        assert_eq!(report["status"], "passed");
        exercise_installed_source_worker(
            &project,
            opt_in,
            applied["attunement_job_id"].as_str().unwrap(),
        );
    }
}
