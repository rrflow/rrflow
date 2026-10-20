use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SensitiveReadUse {
    CausalLogPage,
    AlternateCausalLogPage,
    UnstampedCausalLogPage,
    SnapshotCausalLogPage,
    DataSnapshotReducer,
    GraphSnapshotReducer,
}

const SENSITIVE_READ_USES: [SensitiveReadUse; 6] = [
    SensitiveReadUse::CausalLogPage,
    SensitiveReadUse::AlternateCausalLogPage,
    SensitiveReadUse::UnstampedCausalLogPage,
    SensitiveReadUse::SnapshotCausalLogPage,
    SensitiveReadUse::DataSnapshotReducer,
    SensitiveReadUse::GraphSnapshotReducer,
];

impl SensitiveReadUse {
    fn needle(self) -> String {
        match self {
            Self::CausalLogPage => ["read", "_changes("].concat(),
            Self::AlternateCausalLogPage => ["runtime_read", "_changes("].concat(),
            Self::UnstampedCausalLogPage => ["changes", "_since("].concat(),
            Self::SnapshotCausalLogPage => ["snapshot", "_changes("].concat(),
            Self::DataSnapshotReducer => ["RuntimeDataSnapshot::from", "_changes("].concat(),
            Self::GraphSnapshotReducer => ["RuntimeGraphSnapshot::from", "_changes("].concat(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AllowedUse {
    path: &'static str,
    kind: SensitiveReadUse,
    count: usize,
    operation: &'static str,
}

const ALLOWED_USES: &[AllowedUse] = &[
    AllowedUse {
        path: "crates/operations/rrd-cluster/src/artifact_transfer.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "bounded cluster artifact-transfer replay",
    },
    AllowedUse {
        path: "crates/compute/rrd-query/tests/graph_traversal.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 2,
        operation: "exact breadth-first graph traversal oracle",
    },
    AllowedUse {
        path: "crates/compute/rrd-query/tests/query.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "exact rrflowQL result oracle",
    },
    AllowedUse {
        path: "crates/compute/rrd-query/tests/query.rs",
        kind: SensitiveReadUse::GraphSnapshotReducer,
        count: 1,
        operation: "exact rrflowQL graph reducer oracle",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/src/engine/rollback.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "explicit forward-rollback planning replay",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/src/engine/rollback.rs",
        kind: SensitiveReadUse::GraphSnapshotReducer,
        count: 2,
        operation: "explicit rollback current/target graph comparison",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/src/runtime/data_plane.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "embedding trace-only conflict recovery verification",
    },
    AllowedUse {
        path: "crates/compute/rrd-vector/tests/engine_differential.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "exact vector candidate oracle",
    },
    AllowedUse {
        path: "crates/kernel/rrd-core/src/runtime.rs",
        kind: SensitiveReadUse::DataSnapshotReducer,
        count: 1,
        operation: "kernel data reducer missing-table rejection self-test",
    },
    AllowedUse {
        path: "crates/kernel/rrd-core/src/runtime.rs",
        kind: SensitiveReadUse::GraphSnapshotReducer,
        count: 1,
        operation: "kernel graph reducer self-test",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/src/engine/diagnostic.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "explicit bounded diagnostic replay",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/src/engine/diagnostic.rs",
        kind: SensitiveReadUse::GraphSnapshotReducer,
        count: 2,
        operation: "explicit diagnostic graph comparison",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/src/access/semantic_commit.rs",
        kind: SensitiveReadUse::DataSnapshotReducer,
        count: 1,
        operation: "retirement validation over directly selected versions",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/src/repository/runtime.rs",
        kind: SensitiveReadUse::DataSnapshotReducer,
        count: 2,
        operation: "data snapshot and preview over directly selected versions",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/src/repository/runtime.rs",
        kind: SensitiveReadUse::GraphSnapshotReducer,
        count: 1,
        operation: "transaction graph preview over directly selected versions",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/direct_read_paths.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 1,
        operation: "exact all-model change-log oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/direct_read_paths.rs",
        kind: SensitiveReadUse::DataSnapshotReducer,
        count: 1,
        operation: "exact all-model data reducer oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/adjacency_reads.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 2,
        operation: "exact commit-path adjacency index oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/catalog_revision.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 3,
        operation: "stale and fresh catalogue-revision log API characterization",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/snapshot.rs",
        kind: SensitiveReadUse::CausalLogPage,
        count: 5,
        operation: "bounded authenticated change-page and stamp characterization",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/snapshot.rs",
        kind: SensitiveReadUse::GraphSnapshotReducer,
        count: 1,
        operation: "exact graph reducer oracle",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/src/engine/changefeed.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "public change-feed page at a caller-supplied cursor",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/src/backup.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "explicit backup object-reference scan to the backup watermark",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/src/archive/export.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "explicit logical-archive export paging",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/src/archive/restore.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 2,
        operation: "explicit archive-restore commit and watermark verification",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/tests/runtime_cluster_transfer_trace.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "cluster-transfer trace oracle over committed changes",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/tests/runtime_data_plane_trace.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 2,
        operation: "data-plane trace and embedding-rebase oracles",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/tests/runtime_operator_knowledge_trace.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 2,
        operation: "operator-knowledge trace and outbox-retry oracles",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/tests/runtime_query_trace.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "query trace oracle over committed changes",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/tests/runtime_trace.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 2,
        operation: "runtime trace-phase oracle",
    },
    AllowedUse {
        path: "crates/authority/rrd-engine/tests/runtime_vector_artifact_catalog.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "vector artifact publication oracle",
    },
    AllowedUse {
        path: "crates/operations/rrd-cluster/tests/openraft_process.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "OpenRaft process replication oracle",
    },
    AllowedUse {
        path: "crates/operations/rrd-cluster/tests/openraft_storage.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "OpenRaft storage transfer oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/durability.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "process-kill durability oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/logical_archive.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 2,
        operation: "archive export and restore log-coordinate oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/logical_archive_resilience.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 2,
        operation: "archive-resilience runtime equality oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/runtime.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 7,
        operation: "change-page, scope-filter, and reopen hash-chain characterization",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/semantic_commit_atomicity.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "semantic commit atomicity oracle",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/snapshot.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 3,
        operation: "live and mixed-scope change-page characterization",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/snapshot.rs",
        kind: SensitiveReadUse::SnapshotCausalLogPage,
        count: 12,
        operation: "snapshot-lease change-page characterization",
    },
    AllowedUse {
        path: "crates/persistence/rrd-store/tests/trace.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "store trace oracle over committed changes",
    },
    AllowedUse {
        path: "crates/transport/rrd-server/tests/http_process.rs",
        kind: SensitiveReadUse::UnstampedCausalLogPage,
        count: 1,
        operation: "public data-transaction replay oracle after restart",
    },
];

/// A normal-path log replay that C-04 has not removed. Each is tolerated only
/// while the POA&M record that names it is not closed, and removing the replay
/// fails this test until its entry is deleted with it.
#[derive(Debug, Clone, Copy)]
struct KnownDeficientUse {
    path: &'static str,
    kind: SensitiveReadUse,
    count: usize,
    operation: &'static str,
    poam: &'static str,
}

/// The POA&M ledger statuses of a deficiency that is still present.
const PRESENT_DEFICIENCY_STATUSES: [&str; 3] = ["Open", "Sequenced", "Verifying"];

const KNOWN_DEFICIENT_USES: &[KnownDeficientUse] = &[KnownDeficientUse {
    path: "crates/operations/rrd-maintenance/src/lib.rs",
    kind: SensitiveReadUse::UnstampedCausalLogPage,
    count: 1,
    operation: "maintenance-run state rebuilt from cursor 0; the crate has no in-tree caller",
    poam: "POAM-028",
}];

#[test]
fn causal_log_and_snapshot_reducer_uses_match_the_explicit_allowlist() {
    let root = workspace_root();
    let mut rust_sources = Vec::new();
    collect_rust_sources(&root.join("crates"), &mut rust_sources);
    rust_sources.sort();

    let mut actual = BTreeMap::new();
    for path in rust_sources {
        let relative = path
            .strip_prefix(&root)
            .expect("crate source must be workspace-relative")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let compact = source
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .collect::<String>();
        for kind in SENSITIVE_READ_USES {
            let count = count_identifier_bounded(&compact, &kind.needle());
            if count > 0 {
                actual.insert((relative.clone(), kind), count);
            }
        }
    }

    let mut expected = BTreeMap::new();
    for allowed in ALLOWED_USES {
        assert!(
            !allowed.operation.trim().is_empty() && allowed.count > 0,
            "sensitive read allowance must name a real operation and nonzero count: {allowed:?}"
        );
        assert!(
            expected
                .insert((allowed.path.to_owned(), allowed.kind), allowed.count)
                .is_none(),
            "sensitive read allowance duplicates path/kind: {allowed:?}"
        );
    }

    let ledger = fs::read_to_string(root.join("docs/poam/rrflow-1.0-alpha.md"))
        .expect("the POA&M ledger is readable");
    for deficient in KNOWN_DEFICIENT_USES {
        assert!(
            !deficient.operation.trim().is_empty() && deficient.count > 0,
            "known deficiency must name a real operation and nonzero count: {deficient:?}"
        );
        assert!(
            poam_status(&ledger, deficient.poam)
                .is_some_and(|status| PRESENT_DEFICIENCY_STATUSES.contains(&status.as_str())),
            "a known normal-path log replay needs a POA&M record that is not closed: {deficient:?}"
        );
        assert!(
            expected
                .insert((deficient.path.to_owned(), deficient.kind), deficient.count)
                .is_none(),
            "known deficiency duplicates an allowed path/kind: {deficient:?}"
        );
    }

    assert_eq!(
        actual, expected,
        "causal-log or kernel snapshot-reducer source closure changed; normal query, vector, retrieval, context, memory, and inference reads may not be allowlisted, and a known normal-path replay is listed only while its POA&M record is not closed"
    );
}

/// The ledger status of one POA&M row, read from its `| ID | Priority | Status |` cells.
fn poam_status(ledger: &str, id: &str) -> Option<String> {
    ledger.lines().find_map(|line| {
        let cells = line.split('|').map(str::trim).collect::<Vec<_>>();
        if cells.get(1) == Some(&id) {
            cells.get(3).map(|status| (*status).to_owned())
        } else {
            None
        }
    })
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|candidate| {
            candidate.join("Cargo.toml").is_file() && candidate.join("crates").is_dir()
        })
        .expect("rrd-engine must live below the workspace root")
        .to_path_buf()
}

fn collect_rust_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()));
    entries.sort_by_key(fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let kind = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

fn count_identifier_bounded(source: &str, needle: &str) -> usize {
    source
        .match_indices(needle)
        .filter(|(offset, _)| {
            *offset == 0
                || !source.as_bytes()[offset - 1].is_ascii_alphanumeric()
                    && source.as_bytes()[offset - 1] != b'_'
        })
        .count()
}
