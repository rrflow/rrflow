//! The lab home, trusted keys, the definition store and pack listings.

use super::*;

pub(super) fn now_unix_s() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|error| error.to_string())
}

pub(super) fn number(arguments: &Arguments, name: &str) -> Result<u64> {
    let text = arguments.required(name)?;
    text.parse()
        .map_err(|_| format!("--{name} must be a whole number, not {text}"))
}

pub(super) fn lab_home() -> Result<DefinitionHome> {
    DefinitionHome::resolve(
        std::env::var_os(HOME_VARIABLE).map(PathBuf::from),
        RRFLOW_PRODUCT,
        &|name| std::env::var(name).ok(),
    )
    .map_err(|error| format!("{error}: set {HOME_VARIABLE}"))
}

/// A named file if given. Otherwise the keys pinned in this build and this machine's own.
pub(super) fn trusted_keys(arguments: &Arguments) -> Result<TrustedKeys> {
    match arguments.values.get("trust") {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
            serde_json::from_slice(&bytes).map_err(|error| format!("{path}: {error}"))
        }
        None => lab_home()?.trusted_keys(&pinned_keys()?),
    }
}

pub(super) fn open_store(arguments: &Arguments) -> Result<PackStore> {
    match arguments.values.get("store") {
        Some(path) => PackStore::open(PathBuf::from(path), RRFLOW_PRODUCT),
        None => lab_home()?.store(),
    }
}

pub(super) fn settings_path(home: &DefinitionHome) -> PathBuf {
    home.root().join(SETTINGS_FILE)
}

pub(super) fn load_settings(home: &DefinitionHome) -> Result<LabSettings> {
    LabSettings::load(&settings_path(home))
}

/// What a person sees in place of "not found", wherever nothing is installed yet.
pub(super) const NOTHING_INSTALLED: &str = "No test definitions are installed yet.
In a checkout of this repository, run: rrflow-lab definitions publish
Otherwise, run: rrflow-lab definitions update --from FOLDER";

pub(super) fn installed_definitions(store: &PackStore, keys: &TrustedKeys) -> Result<Definitions> {
    Definitions::read(&store.current_files(keys)?)
}

pub(super) fn days_word(days: u64) -> String {
    if days == 1 {
        "1 day".to_owned()
    } else {
        format!("{days} days")
    }
}

pub(super) fn more_days(days: u64) -> String {
    if days == 1 {
        "1 more day".to_owned()
    } else {
        format!("{days} more days")
    }
}

pub(super) fn grouped(number: u64) -> String {
    let digits = number.to_string();
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

pub(super) fn listed(ids: &[Id]) -> String {
    ids.iter().map(Id::as_str).collect::<Vec<_>>().join(", ")
}

/// Why a pack's life is as it is, in one line.
pub(super) fn pack_line(status: &StoreStatus, now: u64) -> String {
    let sequence = status.current_sequence.unwrap_or(0);
    let published = status.published_at_unix_s.unwrap_or(now);
    let expires = status.expires_at_unix_s.unwrap_or(now);
    let age = now.saturating_sub(published) / DAY_SECONDS;
    let published = match age {
        0 => "today".to_owned(),
        _ => format!("{} ago", days_word(age)),
    };
    if status.expired {
        format!(
            "Test definitions: pack {sequence}, published {published}. It has expired. It still \
             works and results made with it say so, but get a newer one."
        )
    } else {
        format!(
            "Test definitions: pack {sequence}, published {published}, good for {}.",
            more_days(expires.saturating_sub(now) / DAY_SECONDS)
        )
    }
}

pub(super) fn approval_hint(waiting: &[Id]) -> Option<String> {
    (!waiting.is_empty()).then(|| {
        format!(
            "{} waiting for your approval: {}.\nSee what each would start: rrflow-lab plan\n\
             Then approve them: rrflow-lab settings approve --all",
            if waiting.len() == 1 {
                "1 system is".to_owned()
            } else {
                format!("{} systems are", waiting.len())
            },
            listed(waiting)
        )
    })
}

pub(super) fn print_changes(outcome: &InstallOutcome) {
    for (word, paths) in [
        ("Added", &outcome.added),
        ("Changed", &outcome.changed),
        ("Removed", &outcome.removed),
    ] {
        match paths.len() {
            0 => {}
            1..=12 => println!("{word}: {}", paths.join(", ")),
            count => println!("{word}: {count} files"),
        }
    }
    for line in &outcome.notable {
        println!("Look at this: {line}");
    }
}

/// The lab folder of the checkout this is run in.
pub(super) fn find_lab(arguments: &Arguments) -> Result<PathBuf> {
    if let Some(lab) = arguments.values.get("lab") {
        return Ok(PathBuf::from(lab));
    }
    let here = std::env::current_dir().map_err(|error| error.to_string())?;
    here.ancestors()
        .map(|folder| folder.join("eval").join("lab"))
        .find(|lab| lab.join("sources.json").is_file())
        .ok_or_else(|| {
            "no eval/lab was found from this folder. Run this inside a checkout, or name the \
             folder with --lab DIR"
                .to_owned()
        })
}
