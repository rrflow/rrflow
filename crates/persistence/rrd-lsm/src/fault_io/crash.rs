//! Owned native process-death rendezvous. Unit fixtures use the cfg(test)
//! parent; explicit `crash-test` diagnostic dependencies compile this same
//! source through `crash_test`. Ordinary libraries and binaries exclude it.

#[cfg(test)]
use super::Kind;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Point {
    WalBeforeWrite,
    WalAppended,
    WalSynced,
    Visible,
    SegmentSynced,
    SuccessorWalSynced,
    CurrentBeforeRename,
    CurrentRenamed,
    ReclaimBeforeRemove,
    ReclaimRemoved,
    RecoveryRead,
    Acknowledged,
    #[cfg(not(test))]
    ReceiptPersisted,
    #[cfg(not(test))]
    SegmentBeforeCreate,
    #[cfg(not(test))]
    ManifestBeforeCreate,
    #[cfg(not(test))]
    CurrentBeforeCreate,
    Unreachable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Schedule {
    root: PathBuf,
    nonce: String,
    point: Point,
    id: u16,
    #[cfg(test)]
    owned: bool,
}

struct Armed {
    schedule: Schedule,
    #[cfg(test)]
    recovery_reads: usize,
    deadline: Instant,
}

thread_local! { static ARMED: RefCell<Option<Armed>> = const { RefCell::new(None) }; }

const PREFIX: &str = "C07C_NATIVE_PAUSE_V1=";
const CHILD_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Hit {
    nonce: String,
    pid: u32,
    point: Point,
    id: u16,
}

fn arm(schedule: Schedule) {
    assert!(schedule.root.is_absolute() && schedule.root.is_dir());
    assert_eq!(
        std::fs::canonicalize(&schedule.root).unwrap(),
        schedule.root
    );
    assert_eq!(schedule.nonce.len(), 64);
    assert!(schedule
        .nonce
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    assert!((1..=1024).contains(&schedule.id));
    ARMED.with_borrow_mut(|slot| {
        assert!(slot.is_none(), "native pause already armed");
        *slot = Some(Armed {
            schedule,
            #[cfg(test)]
            recovery_reads: 0,
            deadline: Instant::now() + CHILD_DEADLINE,
        });
    });
}

fn pause_if(matches: impl FnOnce(&mut Armed) -> bool) {
    let selected = ARMED.with_borrow_mut(|slot| {
        let active = slot.as_mut()?;
        assert!(
            Instant::now() < active.deadline,
            "native pause schedule expired"
        );
        if matches(active) {
            slot.take()
        } else {
            None
        }
    });
    let Some(active) = selected else {
        return;
    };
    let hit = Hit {
        nonce: active.schedule.nonce,
        pid: std::process::id(),
        point: active.schedule.point,
        id: active.schedule.id,
    };
    // Exactly one bounded identity receipt, without paths, values or secrets.
    let line = format!("{PREFIX}{}\n", serde_json::to_string(&hit).unwrap());
    assert!(line.len() < 512);
    std::io::stdout().write_all(line.as_bytes()).unwrap();
    std::io::stdout().flush().unwrap();
    #[cfg(not(test))]
    if installed::resume_private(&hit, active.deadline) {
        return;
    }
    while Instant::now() < active.deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("owned controller did not terminate paused child before deadline");
}

#[cfg(test)]
pub(crate) fn io(kind: Kind, path: &Path, after: bool) {
    pause_if(|active| {
        // Anonymous reads originate only from the WAL replay wrapper. The
        // third completed read is its first payload, before applying that frame.
        if active.schedule.point == Point::RecoveryRead
            && kind == Kind::Read
            && path.as_os_str().is_empty()
            && after
        {
            active.recovery_reads += 1;
            return active.recovery_reads == 3;
        }
        if !path.starts_with(&active.schedule.root) {
            return false;
        }
        let wal = path.extension().is_some_and(|ext| ext == "wal");
        let segment = path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "segments");
        let current = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".CURRENT."));
        match active.schedule.point {
            Point::WalBeforeWrite => kind == Kind::Write && wal && !after,
            Point::WalAppended => kind == Kind::SyncData && wal && !after,
            Point::WalSynced => kind == Kind::SyncData && wal && after,
            Point::SegmentSynced => kind == Kind::SyncAll && segment && after,
            Point::SuccessorWalSynced => kind == Kind::SyncAll && wal && after,
            Point::CurrentBeforeRename => kind == Kind::Rename && current && !after,
            Point::CurrentRenamed => kind == Kind::Rename && current && after,
            Point::ReclaimBeforeRemove => kind == Kind::Remove && !after,
            Point::ReclaimRemoved => kind == Kind::Remove && after,
            _ => false,
        }
    });
}

#[cfg(test)]
pub(crate) fn visible(root: &Path) {
    pause_if(|active| active.schedule.point == Point::Visible && active.schedule.root == root);
}

#[cfg(test)]
mod tests;

#[cfg(not(test))]
#[path = "crash/installed.rs"]
mod installed;
#[cfg(not(test))]
pub use installed::{
    before_io, boundary, configure_from_stdin, enter_domain, enter_recovery, observe_native_error,
    ScopeGuard,
};
