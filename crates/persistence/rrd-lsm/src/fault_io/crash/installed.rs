//! Private diagnostic-process control, absent from ordinary distributions.
use super::*;
use std::io::{self, Read};
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
}

struct Pending {
    schedule: Schedule,
    operation: Option<String>,
    admitted: Instant,
}

static CONFIGURED: AtomicBool = AtomicBool::new(false);
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Called exactly once by diagnostic `serve`, before opening its installed
/// estate. The caller supplies the locator-bound root, never the controller.
/// EOF delimits a <=4096-byte private pipe message. Null explicitly disarms.
pub fn configure_from_stdin(root: &Path) -> io::Result<()> {
    if CONFIGURED.swap(true, Ordering::AcqRel) {
        return Err(io::Error::other("diagnostic control already configured"));
    }
    let mut bytes = Vec::new();
    io::stdin().take(4097).read_to_end(&mut bytes)?;
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
        )
        || (control.point == Point::RecoveryRead) != control.operation_sha256.is_none()
        || control
            .operation_sha256
            .as_deref()
            .is_some_and(|s| !is_digest(s))
    {
        return Err(io::Error::other("inadmissible diagnostic schedule"));
    }
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
        }
    }
}

/// Native effects are observed only while the selected thread's scope is
/// armed. Root containment also rejects another database in that thread.
pub fn boundary(point: Point, path: &Path) {
    pause_if(|active| active.schedule.point == point && path.starts_with(&active.schedule.root));
}
