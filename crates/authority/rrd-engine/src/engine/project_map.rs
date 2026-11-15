//! A plain-language map of an installed project, for its owner and the AI
//! tools they use with RRFlow. RRFlow regenerates it from the same bounded,
//! `.gitignore`-aware walk as the installation inventory each time the project
//! is installed or served. It is a replaceable projection with no authority;
//! only the notes section belongs to the owner and is carried forward intact.

use super::estate_layout::EstateFilesystem;
use super::installation::{project_entries, ProjectEntry};
use super::*;
use rrd_store::filesystem::{AnchoredPath, OpenOptions};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;

pub const PROJECT_MAP_RELATIVE_PATH: &str = ".rrflow/project-map.md";
pub(super) const PROJECT_MAP_PENDING: &str = ".rrflow/.project-map.pending";
const NOTES_START: &str = "<!-- rrflow:notes:start -->";
const NOTES_END: &str = "<!-- rrflow:notes:end -->";
const DEFAULT_NOTES: &str = "Write anything that helps: what this project is for, how you like to work,\nwhere to begin. RRFlow keeps this section exactly as you leave it.";
const MAX_EXISTING_MAP_BYTES: u64 = 1024 * 1024;
const MAX_FOLDER_ROWS: usize = 40;

/// What the latest refresh found, for callers that report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMapSummary {
    pub path: std::path::PathBuf,
    pub files: u64,
    pub folders: u64,
    pub bytes: u64,
    pub changed: bool,
}

impl RrdEngine {
    /// Regenerate `.rrflow/project-map.md` for an installed project, keeping
    /// the owner's notes. Writes only when the content changed.
    pub fn refresh_project_map(project: &Path) -> Result<ProjectMapSummary> {
        let locator = Self::read_project_locator(project)?;
        let (profile, _) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
        let root = AnchoredPath::open_root(project).map_err(storage_error)?;
        let engram = EstateFilesystem::admit(root, false)?;
        // The installed profile's inventory policy and the project's own Git
        // ignore rules decide what is part of the project, as for installation.
        let entries = project_entries(engram.project(), &profile)?;
        let census = Census::collect(&entries);
        let name = engram
            .project()
            .diagnostic_path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project")
            .to_owned();
        let target = engram.managed(PROJECT_MAP_RELATIVE_PATH)?;
        let existing = read_existing(&target)?;
        let notes = existing
            .as_deref()
            .and_then(owner_notes)
            .unwrap_or(DEFAULT_NOTES)
            .to_owned();
        let rendered = census.render(&name, &notes);
        let changed = existing.as_deref() != Some(rendered.as_str());
        if changed {
            let pending = engram.managed(PROJECT_MAP_PENDING)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&pending)
                .map_err(storage_error)?;
            file.write_all(rendered.as_bytes()).map_err(storage_error)?;
            file.sync_all().map_err(storage_error)?;
            drop(file);
            pending.rename_to(&target).map_err(storage_error)?;
        }
        Ok(ProjectMapSummary {
            path: project.join(PROJECT_MAP_RELATIVE_PATH),
            files: census.files,
            folders: census.folders,
            bytes: census.bytes,
            changed,
        })
    }
}

fn storage_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Storage(error.to_string())
}

fn read_existing(path: &AnchoredPath) -> Result<Option<String>> {
    match path.metadata() {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_EXISTING_MAP_BYTES => {
            let bytes = path.read().map_err(storage_error)?;
            Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
        }
        // An oversized or replaced map is regenerated with default notes.
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(storage_error(error)),
    }
}

/// The owner's text between the notes markers, trimmed of surrounding blank
/// lines. `None` when the markers are missing, so a damaged map is recovered.
fn owner_notes(map: &str) -> Option<&str> {
    let start = map.find(NOTES_START)? + NOTES_START.len();
    let end = start + map[start..].find(NOTES_END)?;
    let notes = map[start..end].trim_matches(['\r', '\n']);
    (!notes.trim().is_empty()).then_some(notes)
}

#[derive(Default)]
struct Totals {
    files: u64,
    bytes: u64,
    languages: BTreeMap<&'static str, u64>,
}

impl Totals {
    fn add(&mut self, language: Option<&'static str>, bytes: u64) {
        self.files += 1;
        self.bytes += bytes;
        if let Some(language) = language {
            *self.languages.entry(language).or_default() += bytes;
        }
    }

    fn mostly(&self) -> &'static str {
        self.languages
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then(right.0.cmp(left.0)))
            .map_or("", |(language, _)| language)
    }
}

#[derive(Default)]
struct Census {
    files: u64,
    folders: u64,
    bytes: u64,
    all: Totals,
    top_level_files: Totals,
    folder_totals: BTreeMap<String, Totals>,
    root_files: Vec<String>,
    workflows: bool,
}

impl Census {
    fn collect(entries: &[ProjectEntry]) -> Self {
        let mut census = Self::default();
        for entry in entries {
            let relative = entry.relative.clone();
            let (first, rest) = match relative.split_once('/') {
                Some((first, rest)) => (first.to_owned(), Some(rest)),
                None => (relative.clone(), None),
            };
            if entry.directory {
                census.folders += 1;
                if rest.is_none() {
                    census.folder_totals.entry(first).or_default();
                }
                if relative == ".github/workflows" {
                    census.workflows = true;
                }
                continue;
            }
            let bytes = entry.bytes;
            let language = language(&relative);
            census.files += 1;
            census.bytes += bytes;
            census.all.add(language, bytes);
            match rest {
                Some(_) => census
                    .folder_totals
                    .entry(first)
                    .or_default()
                    .add(language, bytes),
                None => {
                    census.top_level_files.add(language, bytes);
                    census.root_files.push(relative);
                }
            }
        }
        census.root_files.sort();
        census
    }

    fn render(&self, name: &str, notes: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Project map: {name}\n");
        out.push_str(
            "RRFlow keeps this map current each time the project is installed or opened.\n\
             It describes what is here so you, and the AI tools you use with RRFlow,\n\
             can find your way around. RRFlow rewrites everything except\n\
             \"Your notes\", which stays exactly as you leave it.\n\n",
        );

        out.push_str("## At a glance\n\n");
        let kinds = self.kinds();
        if !kinds.is_empty() {
            let _ = writeln!(out, "- **Kind:** {}", kinds.join(", "));
        }
        let _ = writeln!(
            out,
            "- **Size:** {} files in {} folders, {}. Files your `.gitignore` excludes are not counted.",
            count(self.files),
            count(self.folders),
            size(self.bytes)
        );
        let languages = self.main_languages();
        if !languages.is_empty() {
            let _ = writeln!(out, "- **Main languages:** {}", languages.join(", "));
        }
        out.push('\n');

        let starts = self.start_here();
        if !starts.is_empty() {
            out.push_str("## Start here\n\n");
            for (file, meaning) in starts {
                let _ = writeln!(out, "- `{file}`: {meaning}");
            }
            out.push('\n');
        }

        out.push_str("## Folders\n\n");
        if self.folder_totals.is_empty() && self.top_level_files.files == 0 {
            out.push_str("The project is empty so far.\n\n");
        } else {
            out.push_str("| Folder | Files | Size | Mostly |\n|---|---:|---:|---|\n");
            let mut folders: Vec<_> = self.folder_totals.iter().collect();
            folders
                .sort_by(|left, right| right.1.files.cmp(&left.1.files).then(left.0.cmp(right.0)));
            for (folder, totals) in folders.iter().take(MAX_FOLDER_ROWS) {
                let _ = writeln!(
                    out,
                    "| `{folder}/` | {} | {} | {} |",
                    count(totals.files),
                    size(totals.bytes),
                    totals.mostly()
                );
            }
            if self.top_level_files.files > 0 {
                let _ = writeln!(
                    out,
                    "| (top level) | {} | {} | {} |",
                    count(self.top_level_files.files),
                    size(self.top_level_files.bytes),
                    self.top_level_files.mostly()
                );
            }
            if folders.len() > MAX_FOLDER_ROWS {
                let _ = writeln!(
                    out,
                    "\nand {} smaller folders.",
                    count((folders.len() - MAX_FOLDER_ROWS) as u64)
                );
            }
            out.push('\n');
        }

        out.push_str("## Your notes\n\n");
        let _ = writeln!(out, "{NOTES_START}\n{notes}\n{NOTES_END}");
        out
    }

    fn has(&self, file: &str) -> bool {
        self.root_files
            .binary_search_by(|name| name.as_str().cmp(file))
            .is_ok()
    }

    fn has_suffix(&self, suffix: &str) -> bool {
        self.root_files.iter().any(|name| name.ends_with(suffix))
    }

    fn kinds(&self) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        for (present, kind) in [
            (self.has("Cargo.toml"), "Rust (Cargo)"),
            (
                self.has("package.json") && self.has("pnpm-workspace.yaml"),
                "JavaScript/TypeScript (pnpm workspace)",
            ),
            (
                self.has("package.json") && !self.has("pnpm-workspace.yaml"),
                "JavaScript/TypeScript (npm)",
            ),
            (
                self.has("pyproject.toml") || self.has("requirements.txt") || self.has("setup.py"),
                "Python",
            ),
            (self.has("go.mod"), "Go"),
            (self.has("pom.xml"), "Java (Maven)"),
            (
                self.has("build.gradle") || self.has("build.gradle.kts"),
                "JVM (Gradle)",
            ),
            (
                self.has_suffix(".sln") || self.has_suffix(".csproj"),
                ".NET",
            ),
            (self.has("Gemfile"), "Ruby"),
            (self.has("composer.json"), "PHP"),
            (
                self.has("Dockerfile")
                    || self.has("docker-compose.yml")
                    || self.has("compose.yaml"),
                "Containers (Docker)",
            ),
            (self.workflows, "GitHub Actions CI"),
        ] {
            if present {
                kinds.push(kind);
            }
        }
        kinds
    }

    fn main_languages(&self) -> Vec<String> {
        let total: u64 = self.all.languages.values().sum();
        if total == 0 {
            return Vec::new();
        }
        let mut languages: Vec<_> = self.all.languages.iter().collect();
        languages.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        languages
            .into_iter()
            .take(5)
            .filter_map(|(language, bytes)| {
                let percent = bytes * 100 / total;
                (percent > 0).then(|| format!("{language} ({percent}%)"))
            })
            .collect()
    }

    fn start_here(&self) -> Vec<(&str, &'static str)> {
        let known: [(&str, &'static str); 22] = [
            ("README.md", "the project's own introduction"),
            ("README", "the project's own introduction"),
            ("AGENTS.md", "instructions for AI coding agents"),
            ("CLAUDE.md", "instructions for Claude"),
            ("CONTRIBUTING.md", "how to contribute"),
            ("LICENSE", "the license"),
            ("LICENSE.md", "the license"),
            ("Cargo.toml", "Rust build manifest"),
            ("package.json", "Node.js package: dependencies and scripts"),
            ("pnpm-workspace.yaml", "pnpm workspace layout"),
            ("pyproject.toml", "Python project settings"),
            ("requirements.txt", "Python dependencies"),
            ("go.mod", "Go module definition"),
            ("pom.xml", "Maven build"),
            ("build.gradle", "Gradle build"),
            ("build.gradle.kts", "Gradle build"),
            ("Gemfile", "Ruby dependencies"),
            ("composer.json", "PHP dependencies"),
            ("Makefile", "common build commands"),
            ("Dockerfile", "container image recipe"),
            ("docker-compose.yml", "local services"),
            ("compose.yaml", "local services"),
        ];
        known
            .into_iter()
            .filter(|(file, _)| self.has(file))
            .collect()
    }
}

fn language(relative: &str) -> Option<&'static str> {
    super::source_language::source_language(relative).map(|language| language.display)
}

fn count(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn size(bytes: u64) -> String {
    const UNITS: [&str; 3] = ["KB", "MB", "GB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_survive_and_a_damaged_map_is_recovered() {
        let map = format!("# x\n\n{NOTES_START}\nKeep this.\n\nAnd this.\n{NOTES_END}\n");
        assert_eq!(owner_notes(&map), Some("Keep this.\n\nAnd this."));
        assert_eq!(owner_notes("# x without markers"), None);
        assert_eq!(
            owner_notes(&format!("{NOTES_START}\n  \n{NOTES_END}")),
            None
        );
    }

    #[test]
    fn counts_and_sizes_read_naturally() {
        assert_eq!(count(0), "0");
        assert_eq!(count(1_849), "1,849");
        assert_eq!(count(1_234_567), "1,234,567");
        assert_eq!(size(512), "512 B");
        assert_eq!(size(1_536), "1.5 KB");
        assert_eq!(size(40 * 1024 * 1024), "40.0 MB");
    }
}
