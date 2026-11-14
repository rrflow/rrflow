//! Command-line switches, usage and argument parsing.

use super::*;

pub(super) const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const HOME_VARIABLE: &str = "RRFLOW_LAB_HOME";
pub(super) const SETTINGS_FILE: &str = "settings.json";
pub(super) const DEFAULT_PACK_DAYS: u64 = 90;
pub(super) const DAY_SECONDS: u64 = 86_400;
/// Flags that take no value.
pub(super) const SWITCHES: [&str; 3] = ["json", "all", "details"];
pub(super) const USAGE: &str = "usage:
  rrflow-lab validate --manifest FILE
  rrflow-lab resolve-all --tests DIR --systems DIR --out DIR
  rrflow-lab launch-info --system FILE
  rrflow-lab launch-args --system FILE --data DIR --lab DIR [--cpuset CORES] [--pod CONTAINER --volume NAME]
  rrflow-lab vdbbench-args --system FILE --data DIR --ef N
  rrflow-lab inputs --manifest FILE
  rrflow-lab run --manifest FILE --system ID --out DIR [--note KEY=VALUE]...
  rrflow-lab combine --manifest FILE --in DIR --out FILE
  rrflow-lab import --from vectordbbench --result FILE --harness-version VERSION --recall-target NUMBER --out FILE
  rrflow-lab sources --file FILE
  rrflow-lab schemas --out DIR
  rrflow-lab plan [--test ID] [--out NEW_DIR] [--json]
  rrflow-lab compare [--report FILE] [--details] [--json]
  rrflow-lab settings [--json]
  rrflow-lab settings on|off ID
  rrflow-lab settings tune SYSTEM NAME=VALUE...
  rrflow-lab settings approve SYSTEM | --all
  rrflow-lab settings reset [ID]
  rrflow-lab definitions publish [--lab DIR] [--days N]
  rrflow-lab definitions update [--from FOLDER]
  rrflow-lab definitions status [--json] [--trust FILE] [--store DIR]
  rrflow-lab definitions trust --file FILE
  rrflow-lab definitions keygen --private NEW_FILE --trust NEW_FILE
  rrflow-lab definitions build --lab DIR --key FILE --sequence N --days N --out FILE
  rrflow-lab definitions patch --lab DIR --key FILE --sequence N --days N --out FILE --base PACK [--trust FILE]
  rrflow-lab definitions verify --pack FILE [--trust FILE]
  rrflow-lab definitions install --pack FILE [--trust FILE] [--store DIR]
  rrflow-lab definitions revert --to N --reason TEXT [--trust FILE] [--store DIR]
  rrflow-lab tasks-validate --manifest FILE
  rrflow-lab tasks-order --manifest FILE
  rrflow-lab tasks-digest --manifest FILE
  rrflow-lab tasks-run --manifest FILE --task ID --arm ID --model ID --repetition N --checkout DIR --empty DIR --out DIR [--mcp-config FILE --chunks FILE]
  rrflow-lab tasks-audit --manifest FILE --in DIR [--out FILE]
  rrflow-lab tasks-combine --manifest FILE --in DIR --out FILE
  rrflow-lab tasks-import --checkout DIR --endpoint URL [--chunks-out FILE]
  rrflow-lab study-check --config FILE --catalog FILE --requirements FILE --sources FILE [--capture-proof FILE] [--out FILE]
  rrflow-lab study-plan --config FILE --catalog FILE --requirements FILE --sources FILE [--capture-proof FILE] --out FILE
  rrflow-lab report-validate --file FILE

Definitions, settings and any signing key live in this user's data folder. Set RRFLOW_LAB_HOME to use another.";

pub(super) struct Arguments {
    pub(super) values: BTreeMap<String, String>,
    pub(super) notes: BTreeMap<String, String>,
    pub(super) switches: BTreeSet<String>,
}

pub(super) fn parse(arguments: &[String]) -> Result<Arguments> {
    let mut values = BTreeMap::new();
    let mut notes = BTreeMap::new();
    let mut switches = BTreeSet::new();
    let mut iter = arguments.iter();
    while let Some(flag) = iter.next() {
        let name = flag
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {flag}"))?;
        if SWITCHES.contains(&name) {
            switches.insert(name.to_owned());
            continue;
        }
        let value = iter
            .next()
            .ok_or_else(|| format!("--{name} needs a value"))?;
        if name == "note" {
            let (key, note) = value.split_once('=').ok_or("--note needs KEY=VALUE")?;
            notes.insert(format!("note.{key}"), note.to_string());
        } else if values.insert(name.to_string(), value.clone()).is_some() {
            return Err(format!("--{name} was given twice"));
        }
    }
    Ok(Arguments {
        values,
        notes,
        switches,
    })
}

impl Arguments {
    pub(super) fn required(&self, name: &str) -> Result<&str> {
        self.values
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("--{name} is required"))
    }

    pub(super) fn json(&self) -> bool {
        self.switches.contains("json")
    }
}
