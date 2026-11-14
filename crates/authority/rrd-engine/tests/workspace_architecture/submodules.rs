//! Declared, pinned submodules and case-insensitive path uniqueness.

use super::*;

#[derive(Debug)]
pub(super) struct SubmoduleDeclaration {
    pub(super) name: String,
    pub(super) path: Option<String>,
    pub(super) url: Option<String>,
}

/// Reads `.gitmodules` through Git itself so quoting and section names follow
/// Git's own configuration grammar.
pub(super) fn declared_submodules(root: &Path) -> Vec<SubmoduleDeclaration> {
    if !root.join(".gitmodules").is_file() {
        return Vec::new();
    }
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "config",
            "--file",
            ".gitmodules",
            "--null",
            "--get-regexp",
            r"^submodule\..*\.(path|url)$",
        ])
        .output()
        .expect("git config must start");
    assert!(
        output.status.success(),
        ".gitmodules must parse and declare at least one path or url: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut declarations = BTreeMap::<String, SubmoduleDeclaration>::new();
    for record in output.stdout.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let record = std::str::from_utf8(record).expect(".gitmodules must be valid UTF-8");
        let (key, value) = record
            .split_once('\n')
            .expect("git config --null records separate key and value");
        let (name, field) = key
            .strip_prefix("submodule.")
            .and_then(|rest| rest.rsplit_once('.'))
            .expect("git config returned a key outside the submodule section");
        let declaration =
            declarations
                .entry(name.to_owned())
                .or_insert_with(|| SubmoduleDeclaration {
                    name: name.to_owned(),
                    path: None,
                    url: None,
                });
        match field {
            "path" => declaration.path = Some(value.to_owned()),
            _ => declaration.url = Some(value.to_owned()),
        }
    }
    declarations.into_values().collect()
}

/// A submodule is a pinned source input only when a clean clone can reproduce
/// it: declared once, at a tracked gitlink, fetched over credential-free HTTPS.
pub(super) fn submodule_violations(
    gitlinks: &BTreeSet<PathBuf>,
    declarations: &[SubmoduleDeclaration],
) -> Vec<String> {
    let mut violations = Vec::new();
    let mut declared_paths = BTreeSet::new();
    for declaration in declarations {
        let name = &declaration.name;
        let Some(path) = declaration.path.as_deref() else {
            violations.push(format!("submodule {name} declares no path"));
            continue;
        };
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            violations.push(format!(
                "submodule {name} path {path} must be a normalized repository-relative path"
            ));
        }
        if !declared_paths.insert(relative.to_path_buf()) {
            violations.push(format!("submodule path {path} is declared more than once"));
        }
        if !gitlinks.contains(relative) {
            violations.push(format!(
                "submodule {name} declares {path}, but no gitlink pins a revision there"
            ));
        }
        match declaration.url.as_deref() {
            None => violations.push(format!("submodule {name} declares no url")),
            Some(url) => {
                let authority = url
                    .strip_prefix("https://")
                    .map(|rest| rest.split('/').next().unwrap_or_default());
                match authority {
                    None => violations.push(format!(
                        "submodule {name} url {url} must use https:// so a clean clone and CI can fetch it"
                    )),
                    Some(authority) if authority.contains('@') => violations.push(format!(
                        "submodule {name} url must not embed credentials"
                    )),
                    Some("") => {
                        violations.push(format!("submodule {name} url {url} names no host"))
                    }
                    Some(_) => {}
                }
            }
        }
    }
    for gitlink in gitlinks {
        if !declared_paths.contains(gitlink) {
            violations.push(format!(
                "gitlink {} is not declared in .gitmodules",
                gitlink.display()
            ));
        }
    }
    violations
}

#[test]
pub(super) fn submodule_policy_accepts_only_declared_pinned_https_submodules() {
    let declaration = |name: &str, path: Option<&str>, url: Option<&str>| SubmoduleDeclaration {
        name: name.to_owned(),
        path: path.map(str::to_owned),
        url: url.map(str::to_owned),
    };
    let gitlinks = |paths: &[&str]| {
        paths
            .iter()
            .map(PathBuf::from)
            .collect::<BTreeSet<PathBuf>>()
    };

    assert!(
        submodule_violations(
            &gitlinks(&["clients/connectome"]),
            &[declaration(
                "connectome",
                Some("clients/connectome"),
                Some("https://github.com/rrflow/connectome-development.git"),
            )],
        )
        .is_empty(),
        "a declared, gitlink-pinned HTTPS submodule is a permitted source input"
    );
    assert!(submodule_violations(&gitlinks(&[]), &[]).is_empty());

    let rejected = [
        (
            gitlinks(&["clients/connectome"]),
            vec![],
            "is not declared in .gitmodules",
        ),
        (
            gitlinks(&[]),
            vec![declaration(
                "connectome",
                Some("clients/connectome"),
                Some("https://github.com/rrflow/connectome-development.git"),
            )],
            "no gitlink pins a revision",
        ),
        (
            gitlinks(&["clients/connectome"]),
            vec![declaration(
                "connectome",
                Some("clients/connectome"),
                Some("git@github.com:rrflow/connectome-development.git"),
            )],
            "must use https://",
        ),
        (
            gitlinks(&["clients/connectome"]),
            vec![declaration(
                "connectome",
                Some("clients/connectome"),
                Some("../connectome"),
            )],
            "must use https://",
        ),
        (
            gitlinks(&["clients/connectome"]),
            vec![declaration(
                "connectome",
                Some("clients/connectome"),
                Some("https://user:token@github.com/rrflow/connectome-development.git"),
            )],
            "must not embed credentials",
        ),
        (
            gitlinks(&["clients/connectome"]),
            vec![declaration("connectome", Some("clients/connectome"), None)],
            "declares no url",
        ),
        (
            gitlinks(&[]),
            vec![declaration(
                "escape",
                Some("../outside"),
                Some("https://github.com/rrflow/outside.git"),
            )],
            "normalized repository-relative path",
        ),
        (
            gitlinks(&["clients/connectome"]),
            vec![
                declaration(
                    "a",
                    Some("clients/connectome"),
                    Some("https://github.com/rrflow/connectome-development.git"),
                ),
                declaration(
                    "b",
                    Some("clients/connectome"),
                    Some("https://github.com/rrflow/connectome-development.git"),
                ),
            ],
            "declared more than once",
        ),
    ];
    for (links, declarations, expected) in rejected {
        let violations = submodule_violations(&links, &declarations);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(expected)),
            "expected a violation containing {expected:?}, observed {violations:#?}"
        );
    }
}

#[test]
pub(super) fn tracked_paths_are_case_insensitively_unique() {
    let metadata = workspace_metadata();
    let mut folded_paths = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    for entry in git_index_entries(&metadata.root) {
        folded_paths
            .entry(entry.path.to_string_lossy().to_lowercase())
            .or_default()
            .insert(entry.path);
    }
    let collisions = folded_paths
        .into_iter()
        .filter_map(|(folded, paths)| (paths.len() > 1).then_some((folded, paths)))
        .collect::<Vec<_>>();
    assert!(
        collisions.is_empty(),
        "tracked paths collide on case-insensitive filesystems: {collisions:#?}"
    );

    let mut folded_packages = BTreeMap::<String, BTreeSet<String>>::new();
    for name in metadata.packages.keys() {
        folded_packages
            .entry(name.to_lowercase())
            .or_default()
            .insert(name.clone());
    }
    let package_collisions = folded_packages
        .into_iter()
        .filter_map(|(folded, names)| (names.len() > 1).then_some((folded, names)))
        .collect::<Vec<_>>();
    assert!(
        package_collisions.is_empty(),
        "workspace package names collide case-insensitively: {package_collisions:#?}"
    );
}
