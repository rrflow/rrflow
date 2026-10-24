//! Private diagnostic-process control, absent from ordinary distributions.
use super::*;
use std::io::{self, BufRead, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    version: u8,
    nonce: String,
    id: u16,
    point: Point,
    operation_sha256: Option<String>,
    #[serde(default)]
    resume: bool,
}

struct Pending {
    schedule: Schedule,
    operation: Option<String>,
    admitted: Instant,
}

static CONFIGURED: AtomicBool = AtomicBool::new(false);
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
static RESUMABLE: AtomicBool = AtomicBool::new(false);

struct Observation {
    schedule: Schedule,
    emitted: bool,
    wal_payload_bytes: Option<usize>,
}
thread_local! { static OBSERVATION: RefCell<Option<Observation>> = const { RefCell::new(None) }; }

/// Called exactly once by diagnostic `serve`, before opening its installed
/// estate. The caller supplies the locator-bound root, never the controller.
/// Newline or EOF delimits a <=4096-byte private pipe message. Null disarms.
pub fn configure_from_stdin(root: &Path) -> io::Result<()> {
    if CONFIGURED.swap(true, Ordering::AcqRel) {
        return Err(io::Error::other("diagnostic control already configured"));
    }
    let mut bytes = Vec::new();
    io::stdin()
        .lock()
        .take(4097)
        .read_until(b'\n', &mut bytes)?;
    if bytes.len() > 4096 {
        return Err(io::Error::other("diagnostic control exceeds bound"));
    }
    let control: Option<Control> = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::other("invalid diagnostic control"))?;
    let Some(control) = control else {
        return Ok(());
    };
    let is_digest = |s: &str| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    };
    if control.version != 1
        || !is_digest(&control.nonce)
        || !(1..=1024).contains(&control.id)
        || !matches!(
            control.point,
            Point::WalBeforeWrite
                | Point::WalAppended
                | Point::WalSynced
                | Point::Visible
                | Point::ReceiptPersisted
                | Point::RecoveryRead
                | Point::RecoveryManifest
                | Point::SegmentSynced
                | Point::SuccessorWalSynced
                | Point::CurrentRenamed
                | Point::ReclaimBeforeRemove
                | Point::ReclaimRemoved
                | Point::SegmentBeforeCreate
                | Point::ManifestBeforeCreate
                | Point::CurrentBeforeCreate
                | Point::CurrentBeforeRename
        )
        || matches!(control.point, Point::RecoveryRead | Point::RecoveryManifest)
            != control.operation_sha256.is_none()
        || control
            .operation_sha256
            .as_deref()
            .is_some_and(|s| !is_digest(s))
    {
        return Err(io::Error::other("inadmissible diagnostic schedule"));
    }
    if control.resume && matches!(control.point, Point::RecoveryRead | Point::RecoveryManifest) {
        return Err(io::Error::other(
            "capacity resume requires a domain effect boundary",
        ));
    }
    RESUMABLE.store(control.resume, Ordering::Release);
    let schedule = Schedule {
        root: std::fs::canonicalize(root)?,
        nonce: control.nonce,
        point: control.point,
        id: control.id,
    };
    *PENDING.lock().unwrap() = Some(Pending {
        schedule,
        operation: control.operation_sha256,
        admitted: Instant::now(),
    });
    Ok(())
}

/// Scoped to the authorized domain commit after its intent has been saved.
/// A different digest leaves the one-use schedule pending, never armed.
pub fn enter_domain(operation: &str) -> ScopeGuard {
    enter_selected(|p| p.operation.as_deref() == Some(operation))
}

/// Installed preflight is already complete. The native recovery window begins
/// here, excluding executable hashing and read-only preflight from its clock.
pub fn enter_recovery(root: &Path) -> ScopeGuard {
    enter_selected(|p| p.operation.is_none() && p.schedule.root == root)
}

fn enter_selected(matches: impl FnOnce(&Pending) -> bool) -> ScopeGuard {
    let mut pending = PENDING.lock().unwrap();
    if !pending.as_ref().is_some_and(matches) {
        return ScopeGuard(false);
    }
    let selected = pending.take().unwrap();
    assert!(
        selected.admitted.elapsed() < Duration::from_secs(180),
        "diagnostic admission expired"
    );
    drop(pending);
    if RESUMABLE.load(Ordering::Acquire) {
        OBSERVATION.with_borrow_mut(|slot| {
            *slot = Some(Observation {
                schedule: selected.schedule.clone(),
                emitted: false,
                wal_payload_bytes: None,
            })
        });
    }
    arm(selected.schedule);
    ScopeGuard(true)
}

pub struct ScopeGuard(bool);
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if self.0 {
            ARMED.with_borrow_mut(|slot| {
                slot.take();
            });
            OBSERVATION.with_borrow_mut(|slot| {
                slot.take();
            });
        }
    }
}

/// Native effects are observed only while the selected thread's scope is
/// armed. Root containment also rejects another database in that thread.
pub fn boundary(point: Point, path: &Path) {
    pause_if(|active| active.schedule.point == point && path.starts_with(&active.schedule.root));
}

/// Record the selected physical append's size without changing selection,
/// admission or I/O. Values and keys never enter the private pause receipt.
pub fn wal_before_write(path: &Path, payload_bytes: usize) {
    OBSERVATION.with_borrow_mut(|slot| {
        if let Some(active) = slot.as_mut() {
            if active.schedule.point == Point::WalBeforeWrite
                && path.starts_with(&active.schedule.root)
            {
                active.wal_payload_bytes = Some(payload_bytes);
            }
        }
    });
    boundary(Point::WalBeforeWrite, path);
}

pub(super) fn selected_wal_payload_bytes() -> Option<usize> {
    OBSERVATION.with_borrow(|slot| slot.as_ref().and_then(|active| active.wal_payload_bytes))
}

/// Continue only through the already-private controller pipe with the exact
/// one-use identity. No filesystem flag, network listener or default hook.
pub(super) fn resume_private(hit: &Hit, deadline: Instant) -> bool {
    if !RESUMABLE.swap(false, Ordering::AcqRel) {
        return false;
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Resume {
        version: u8,
        nonce: String,
        id: u16,
    }
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = (|| -> io::Result<Resume> {
            let mut bytes = Vec::new();
            io::stdin().lock().take(513).read_until(b'\n', &mut bytes)?;
            if bytes.len() > 512 {
                return Err(io::Error::other("capacity resume exceeds bound"));
            }
            serde_json::from_slice(&bytes).map_err(|_| io::Error::other("invalid capacity resume"))
        })();
        let _ = send.send(result);
    });
    let resume = receive
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("bounded private capacity resume")
        .expect("valid private capacity resume");
    assert!(
        resume.version == 1 && resume.nonce == hit.nonce && resume.id == hit.id,
        "capacity resume identity mismatch"
    );
    true
}

pub fn before_io(operation: &'static str, path: &Path) {
    io_boundary(operation, path, false);
}

/// A successful native effect, before any following directory barrier. This
/// observes process-death ordering; it does not simulate a failed native call.
pub fn after_io(operation: &'static str, path: &Path) {
    io_boundary(operation, path, true);
}

fn io_boundary(operation: &'static str, path: &Path, after: bool) {
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let collectible = matches!(parent, Some("segments" | "manifests" | "wal"))
        && matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("seg" | "json" | "wal")
        );
    let point = match (operation, after) {
        ("create", false) if parent == Some("segments") => Some(Point::SegmentBeforeCreate),
        ("create", false) if parent == Some("manifests") => Some(Point::ManifestBeforeCreate),
        ("create", false) if name.starts_with(".CURRENT.") => Some(Point::CurrentBeforeCreate),
        ("rename", false) if name.starts_with(".CURRENT.") => Some(Point::CurrentBeforeRename),
        ("rename", true) if name.starts_with(".CURRENT.") => Some(Point::CurrentRenamed),
        ("sync_all", true) if parent == Some("segments") => Some(Point::SegmentSynced),
        ("sync_all", true) if parent == Some("wal") => Some(Point::SuccessorWalSynced),
        ("remove", false) if collectible => Some(Point::ReclaimBeforeRemove),
        ("remove", true) if collectible => Some(Point::ReclaimRemoved),
        _ => None,
    };
    if let Some(point) = point {
        boundary(point, path);
    }
}

/// Observe an actual native call result. This never injects or substitutes I/O.
pub fn observe_native_error<T>(operation: &'static str, path: &Path, result: &io::Result<T>) {
    let Err(error) = result else {
        return;
    };
    let Some(code) = error.raw_os_error() else {
        return;
    };
    OBSERVATION.with_borrow_mut(|slot| {
        let Some(active) = slot.as_mut() else {
            return;
        };
        if active.emitted || !path.starts_with(&active.schedule.root) {
            return;
        }
        active.emitted = true;
        let parent = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str());
        let role = match parent {
            Some("wal") => "wal",
            Some("segments") => "segment",
            Some("manifests") => "manifest",
            _ => "publication",
        };
        let receipt = serde_json::json!({"version":1,"nonce":active.schedule.nonce,
            "id":active.schedule.id,"pid":std::process::id(),"point":active.schedule.point,
            "operation":operation,"role":role,"native_code":code});
        let line = format!("C07F_NATIVE_IO_V1={receipt}\n");
        assert!(line.len() < 1024);
        std::io::stdout()
            .write_all(line.as_bytes())
            .expect("native capacity receipt");
        std::io::stdout()
            .flush()
            .expect("native capacity receipt flush");
    });
}
