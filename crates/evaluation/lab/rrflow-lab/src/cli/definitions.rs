//! Publishing, updating and planning definition packs.

use super::*;

pub(super) fn publish_definitions(arguments: &Arguments) -> Result<ExitCode> {
    let files = lab_files(&find_lab(arguments)?)?;
    let home = lab_home()?;
    let pinned = pinned_keys()?;
    let days = match arguments.values.get("days") {
        Some(_) => number(arguments, "days")?,
        None => DEFAULT_PACK_DAYS,
    };
    let outcome = home.publish(
        &files,
        &RrflowContent,
        &pinned,
        &PublishOptions {
            days,
            now_unix_s: now_unix_s()?,
        },
    )?;
    if outcome.created_key {
        println!(
            "Created your signing key: {}\nBack this file up and keep it private. Anyone holding \
             it can publish tests your machines will trust.\n",
            home.signing_key_path().display()
        );
    }
    if outcome.kind == PublishKind::Unchanged {
        println!(
            "Nothing changed since pack {}, so nothing was published. It is good for {}.",
            outcome.sequence,
            more_days(outcome.days_left)
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!(
        "Published test definitions pack {}, good for {}.",
        outcome.sequence,
        days_word(outcome.days_left)
    );
    if let Some(install) = &outcome.install {
        print_changes(install);
    }
    match (outcome.patch_bytes, outcome.full_bytes) {
        (Some(patch), Some(full)) => println!(
            "Size: {} bytes as a patch, {} bytes in full.",
            grouped(patch),
            grouped(full)
        ),
        (None, Some(full)) => println!("Size: {} bytes.", grouped(full)),
        _ => {}
    }
    // Whoever publishes a launch has read it, on this machine only.
    let mut settings = load_settings(&home)?;
    approve_all(&mut settings, &Definitions::read(&files)?)?;
    settings.save(&settings_path(&home))?;
    let outbox = home.outbox();
    println!("Installed here, and what it starts is approved on this machine.");
    println!("To send it, copy this folder: {}", outbox.display());
    if !outcome.key_pinned {
        println!(
            "Other machines do not trust this key yet. On each one, run:\n  rrflow-lab definitions \
             trust --file {}",
            outbox
                .join(format!("{RRFLOW_PRODUCT}-trusted-keys.json"))
                .display()
        );
    }
    println!("Then, on each machine: rrflow-lab definitions update --from FOLDER");
    Ok(ExitCode::SUCCESS)
}

pub(super) fn update_definitions(arguments: &Arguments) -> Result<ExitCode> {
    let home = lab_home()?;
    let pinned = pinned_keys()?;
    let mut settings = load_settings(&home)?;
    let named = arguments.values.get("from").map(PathBuf::from);
    let folder = named
        .clone()
        .or_else(|| settings.update_from.as_ref().map(PathBuf::from))
        .unwrap_or_else(|| home.outbox());
    if named.is_none() && settings.update_from.is_none() && !folder.exists() {
        println!("No update folder is set yet. Name one with: rrflow-lab definitions update --from FOLDER");
        return Ok(ExitCode::SUCCESS);
    }
    let now = now_unix_s()?;
    let outcome = match home.update(&folder, &RrflowContent, &pinned, now) {
        Ok(outcome) => outcome,
        // A folder given just now is a mistake. A remembered one may simply be unreachable, and
        // the pack already here still works.
        Err(error) if named.is_some() => return Err(error),
        Err(error) => {
            println!("Could not check for updates: {error}");
            return Ok(ExitCode::SUCCESS);
        }
    };
    if let Some(from) = named {
        let text = from.to_string_lossy().into_owned();
        if settings.update_from.as_deref() != Some(text.as_str()) {
            settings.update_from = Some(text);
            settings.save(&settings_path(&home))?;
        }
    }
    let ignored_untrusted = outcome
        .ignored
        .iter()
        .any(|pack| pack.reason.contains("not trusted"));
    for pack in &outcome.ignored {
        println!("Did not use {}: {}", pack.file, pack.reason);
    }
    if ignored_untrusted {
        println!(
            "If you trust this publisher, run: rrflow-lab definitions trust --file {}",
            folder
                .join(format!("{RRFLOW_PRODUCT}-trusted-keys.json"))
                .display()
        );
    }
    match (&outcome.installed, outcome.current_sequence) {
        (Some(install), _) => {
            println!("Updated to test definitions pack {}.", install.sequence);
            print_changes(install);
            let keys = home.trusted_keys(&pinned)?;
            let definitions = installed_definitions(&home.store()?, &keys)?;
            if let Some(hint) = approval_hint(&waiting_for_approval(&definitions, &settings)?) {
                println!("{hint}");
            }
        }
        (None, Some(sequence)) => println!("Already up to date with pack {sequence}."),
        (None, None) => println!(
            "No test definitions could be installed from {}.",
            folder.display()
        ),
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn show_status(arguments: &Arguments) -> Result<ExitCode> {
    let store = open_store(arguments)?;
    let keys = trusted_keys(arguments)?;
    let now = now_unix_s()?;
    let status = store.status(&keys, now)?;
    let settings = if arguments.values.contains_key("store") {
        LabSettings::default()
    } else {
        load_settings(&lab_home()?)?
    };
    let definitions = status
        .current_sequence
        .map(|_| installed_definitions(&store, &keys))
        .transpose()?;
    let waiting = match &definitions {
        Some(definitions) => waiting_for_approval(definitions, &settings)?,
        None => Vec::new(),
    };
    if arguments.json() {
        let mut facts = serde_json::to_value(&status).map_err(|error| error.to_string())?;
        facts["systems"] = definitions.as_ref().map(|d| d.systems.len()).into();
        facts["tests"] = definitions.as_ref().map(|d| d.tests.len()).into();
        facts["waiting_for_approval"] =
            serde_json::to_value(&waiting).map_err(|error| error.to_string())?;
        facts["turned_off"] =
            serde_json::to_value(&settings.off).map_err(|error| error.to_string())?;
        facts["tuned"] = serde_json::to_value(settings.tuning.keys().collect::<Vec<_>>())
            .map_err(|error| error.to_string())?;
        facts["update_from"] = settings.update_from.clone().into();
        return print_json(&facts);
    }
    let Some(definitions) = definitions else {
        println!("{NOTHING_INSTALLED}");
        return Ok(ExitCode::SUCCESS);
    };
    println!("{}", pack_line(&status, now));
    println!(
        "{} systems and {} tests, checked against their signature.",
        definitions.systems.len(),
        definitions.tests.len()
    );
    if let Some(reason) = &status.reverted_because {
        println!(
            "Stepped back from pack {}: {reason}",
            status.highest_sequence
        );
    }
    if !settings.off.is_empty() {
        let off: Vec<Id> = settings.off.iter().cloned().collect();
        println!("Turned off: {}.", listed(&off));
    }
    for (id, tuning) in &settings.tuning {
        println!("Tuned: {} ({}).", id.as_str(), tuning.describe().join(", "));
    }
    if let Some(hint) = approval_hint(&waiting) {
        println!("{hint}");
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn trust_keys(arguments: &Arguments) -> Result<ExitCode> {
    let path = arguments.required("file")?;
    let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    let keys: TrustedKeys =
        serde_json::from_slice(&bytes).map_err(|error| format!("{path}: {error}"))?;
    let added = lab_home()?.add_trusted_keys(&keys)?;
    if added.is_empty() {
        println!("Those keys were already trusted here.");
    }
    for key in added {
        println!("Now trusting publisher key {key} on this machine.");
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn print_planned_test(test: &PlannedTest) {
    println!("{} ({})", test.title, test.id.as_str());
    if !test.on {
        println!(
            "  Turned off. Turn it on with: rrflow-lab settings on {}",
            test.id.as_str()
        );
        return;
    }
    println!(
        "  {} points, {} dimensions, {} queries, top {}, recall target {}",
        grouped(test.points),
        grouped(u64::from(test.dimensions)),
        grouped(u64::from(test.queries)),
        test.k,
        test.recall_target
    );
    if let Some(problem) = &test.problem {
        println!("  Cannot run as configured: {problem}");
        return;
    }
    let named = |state: SystemState| -> Vec<&str> {
        test.systems
            .iter()
            .filter(|system| system.state == state)
            .map(|system| system.id.as_str())
            .collect()
    };
    let will_run = named(SystemState::WillRun);
    if !will_run.is_empty() {
        println!("  Will run: {}", will_run.join(", "));
    }
    let unset = |name: &String| std::env::var_os(name).is_none();
    for name in test.needs.iter().filter(|name| unset(name)) {
        println!("  Before it runs, set {name} to the folder holding this test's data");
    }
    let mut missing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for system in &test.systems {
        if system.state == SystemState::WillRun {
            for name in system.needs.iter().filter(|name| unset(name)) {
                missing.entry(name).or_default().push(system.id.as_str());
            }
        }
    }
    for (name, systems) in missing {
        println!("  Before it runs, set {name} (for {})", systems.join(", "));
    }
    for system in test
        .systems
        .iter()
        .filter(|system| !system.tuned.is_empty())
    {
        println!(
            "  Tuned: {} ({})",
            system.id.as_str(),
            system.tuned.join(", ")
        );
    }
    let off = named(SystemState::TurnedOff);
    if !off.is_empty() {
        println!("  Turned off: {}", off.join(", "));
    }
    let held: Vec<_> = test
        .systems
        .iter()
        .filter(|system| system.state == SystemState::NeedsApproval)
        .collect();
    if !held.is_empty() {
        println!("  Waiting for approval:");
        for system in held {
            println!(
                "    {} would start {}",
                system.id.as_str(),
                system.starts.as_deref().unwrap_or("something")
            );
        }
        println!("    Approve one with: rrflow-lab settings approve NAME");
    }
    let gaps: Vec<_> = test
        .systems
        .iter()
        .filter(|system| system.state == SystemState::NotAvailable)
        .collect();
    if !gaps.is_empty() {
        println!("  Not available:");
        for system in gaps {
            println!(
                "    {}: {}",
                system.id.as_str(),
                system.reason.as_deref().unwrap_or("no reason given")
            );
        }
    }
}

/// Which pack a plan came from, and so which pack a run's report names.
pub(super) fn pack_identity(status: &StoreStatus) -> serde_json::Value {
    serde_json::json!({
        "sequence": status.current_sequence,
        "manifest_sha256": status.manifest_sha256,
        "stale": status.expired,
    })
}

pub(super) fn write_planned(
    directory: &Path,
    files: &BTreeMap<String, String>,
    plan: &LabPlan,
    status: &StoreStatus,
) -> Result<()> {
    if directory.exists() {
        return Err(format!("{} already exists", directory.display()));
    }
    let write = |relative: &str, text: &str| -> Result<()> {
        let path = directory.join(relative);
        let parent = path.parent().ok_or("a planned file has no parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        std::fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
    };
    for (relative, text) in files {
        write(relative, text)?;
    }
    let plan = serde_json::to_string_pretty(plan).map_err(|error| error.to_string())?;
    write("plan.json", &plan)?;
    write(
        "definitions.env",
        &format!(
            "sequence={}\nmanifest_sha256={}\nstale={}\n",
            status.current_sequence.unwrap_or(0),
            status.manifest_sha256.as_deref().unwrap_or_default(),
            status.expired
        ),
    )
}

pub(super) fn show_plan(arguments: &Arguments) -> Result<ExitCode> {
    let home = lab_home()?;
    let keys = home.trusted_keys(&pinned_keys()?)?;
    let store = home.store()?;
    let now = now_unix_s()?;
    let status = store.status(&keys, now)?;
    if status.current_sequence.is_none() {
        return Err(NOTHING_INSTALLED.into());
    }
    let definitions = installed_definitions(&store, &keys)?;
    let settings = load_settings(&home)?;
    let mut plan = build_plan(&definitions, &settings)?;
    if let Some(wanted) = arguments.values.get("test") {
        if !plan.tests.iter().any(|test| test.id.as_str() == wanted) {
            let known: Vec<&str> = plan.tests.iter().map(|test| test.id.as_str()).collect();
            return Err(format!(
                "no test is named {wanted}. These are installed: {}",
                known.join(", ")
            ));
        }
        if let Some(out) = arguments.values.get("out") {
            let files = planned_files(&definitions, &settings, &plan, wanted)?;
            plan.tests.retain(|test| test.id.as_str() == wanted);
            write_planned(Path::new(out), &files, &plan, &status)?;
        } else {
            plan.tests.retain(|test| test.id.as_str() == wanted);
        }
    } else if arguments.values.contains_key("out") {
        return Err("--out needs one test: name it with --test ID".into());
    }
    if arguments.json() {
        return print_json(&serde_json::json!({
            "definitions": pack_identity(&status),
            "tests": plan.tests,
        }));
    }
    println!("{}", pack_line(&status, now));
    for test in &plan.tests {
        println!();
        print_planned_test(test);
    }
    println!();
    println!("Nothing has started. To run a test on a Linux host with Docker:");
    println!("  eval/lab/run_all.sh TEST_ID RUN_FOLDER");
    Ok(ExitCode::SUCCESS)
}
