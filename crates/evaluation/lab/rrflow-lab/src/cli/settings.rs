//! Showing and changing lab settings.

use super::*;

pub(super) fn show_settings(json: bool) -> Result<ExitCode> {
    let home = lab_home()?;
    let settings = load_settings(&home)?;
    if json {
        return print_json(&settings);
    }
    let keys = home.trusted_keys(&pinned_keys()?)?;
    let store = home.store()?;
    if store
        .status(&keys, now_unix_s()?)?
        .current_sequence
        .is_none()
    {
        return Err(NOTHING_INSTALLED.into());
    }
    let definitions = installed_definitions(&store, &keys)?;
    let waiting = waiting_for_approval(&definitions, &settings)?;
    println!("Systems");
    for (id, system) in &definitions.systems {
        let state = if !runs_somewhere(&definitions, id, system) {
            "n/a"
        } else if settings.off.contains(id) {
            "off"
        } else if waiting.contains(id) {
            "wait"
        } else {
            "on"
        };
        let tuned = settings
            .tuning
            .get(id)
            .map(|tuning| format!("  tuned: {}", tuning.describe().join(", ")))
            .unwrap_or_default();
        println!(
            "  {state:<5}{:<28}{}{tuned}",
            id.as_str(),
            system.binding.label
        );
    }
    println!("Tests");
    for (id, test) in &definitions.tests {
        let state = if settings.off.contains(id) {
            "off"
        } else {
            "on"
        };
        println!("  {state:<5}{:<28}{}", id.as_str(), test.test.title);
    }
    println!("on = runs, off = you turned it off, wait = needs your approval, n/a = not available");
    Ok(ExitCode::SUCCESS)
}

/// One system's name from the installed pack, or a message that lists what there is.
pub(super) fn known_system<'a>(
    definitions: &'a Definitions,
    name: &str,
) -> Result<(Id, &'a rrflow_lab_benchmarks::catalog::SystemDefinition)> {
    definitions
        .systems
        .iter()
        .find(|(id, _)| id.as_str() == name)
        .map(|(id, system)| (id.clone(), system))
        .ok_or_else(|| {
            let known: Vec<&str> = definitions.systems.keys().map(Id::as_str).collect();
            format!(
                "no system is named {name}. These are installed: {}",
                known.join(", ")
            )
        })
}

pub(super) fn change_settings(verb: &str, rest: &[String]) -> Result<ExitCode> {
    let home = lab_home()?;
    let keys = home.trusted_keys(&pinned_keys()?)?;
    let store = home.store()?;
    if store
        .status(&keys, now_unix_s()?)?
        .current_sequence
        .is_none()
    {
        return Err(NOTHING_INSTALLED.into());
    }
    let definitions = installed_definitions(&store, &keys)?;
    let mut settings = load_settings(&home)?;
    match (verb, rest) {
        ("on" | "off", [name]) => {
            let is_system = definitions.systems.keys().any(|id| id.as_str() == name);
            let is_test = definitions.tests.keys().any(|id| id.as_str() == name);
            if is_system == is_test {
                return Err(if is_system {
                    format!("{name} names both a system and a test")
                } else {
                    format!("no system or test is named {name}")
                });
            }
            let id = Id::try_from(name.clone())?;
            if verb == "off" {
                settings.off.insert(id);
                println!("{name} is off. It will still appear in reports, marked as turned off.");
            } else {
                settings.off.remove(&id);
                println!("{name} is on.");
            }
        }
        ("tune", [name, pairs @ ..]) if !pairs.is_empty() => {
            let (id, system) = known_system(&definitions, name)?;
            let mut tuning = settings.tuning.get(&id).cloned().unwrap_or_default();
            for pair in pairs {
                let (setting, value) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("{pair} needs the form NAME=VALUE"))?;
                tuning.set(setting, value)?;
            }
            settings.tuning.insert(id.clone(), tuning.clone());
            // Checked now, on every test it reaches, so a bad value is refused while it is typed.
            let mut check = settings.clone();
            check.off.remove(&id);
            check
                .approved_launches
                .insert(id.clone(), launch_digest(system)?);
            for test in build_plan(&definitions, &check)?.tests {
                if let Some(problem) = test.problem {
                    return Err(format!(
                        "That was not saved, because {} could not run with it: {problem}",
                        test.id.as_str()
                    ));
                }
            }
            println!(
                "{name} is tuned: {}. Its reports will say so.",
                tuning.describe().join(", ")
            );
        }
        ("approve", [all]) if all == "--all" => {
            let changed = approve_all(&mut settings, &definitions)?;
            for id in &changed {
                let summary = definitions.systems[id].launch.summary().unwrap_or_default();
                println!("Approved {}: {summary}", id.as_str());
            }
            if changed.is_empty() {
                println!("Everything was already approved.");
            }
        }
        ("approve", [name]) => {
            let (id, system) = known_system(&definitions, name)?;
            if needs_approval(system) {
                settings
                    .approved_launches
                    .insert(id, launch_digest(system)?);
                println!(
                    "Approved {name}: {}",
                    system.launch.summary().unwrap_or_default()
                );
            } else {
                println!("{name} starts nothing new, so it needs no approval.");
            }
        }
        ("reset", []) => {
            settings.off.clear();
            settings.tuning.clear();
            println!("Everything is on and at its published settings. Approvals are kept.");
        }
        ("reset", [name]) => {
            let id = Id::try_from(name.clone())?;
            settings.off.remove(&id);
            settings.tuning.remove(&id);
            println!("{name} is on and at its published settings.");
        }
        _ => return Err(USAGE.into()),
    }
    settings.save(&settings_path(&home))?;
    Ok(ExitCode::SUCCESS)
}
