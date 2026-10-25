//! Bounded unit-test-only fault scheduling. No feature, environment variable,
//! network operation or production configuration can enable this module.

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) mod crash;
mod model;
mod tests;
use model::Device;

const MAX_EVENTS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Open,
    Read,
    Write,
    SyncData,
    SyncAll,
    DirectorySync,
    Rename,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    Before,
    After,
    Short(usize),
    Zero,
    Interrupted,
}

#[derive(Clone, Debug)]
struct Event {
    kind: Kind,
    path: PathBuf,
}

struct State {
    device: Device,
    events: Vec<Event>,
    faults: Vec<(usize, Fault)>,
    fired: bool,
    fired_count: usize,
}

thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

struct Session;

impl Session {
    fn start(root: &Path, fault: Option<(usize, Fault)>) -> Self {
        Self::schedule(root, &fault.into_iter().collect::<Vec<_>>())
    }

    fn schedule(root: &Path, faults: &[(usize, Fault)]) -> Self {
        assert!(faults.len() <= 2);
        assert!(faults.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let device = Device::seed(root);
        STATE.with_borrow_mut(|slot| {
            assert!(slot.is_none(), "nested I/O fault session");
            *slot = Some(State {
                device,
                events: Vec::new(),
                faults: faults.to_vec(),
                fired: false,
                fired_count: 0,
            });
        });
        Self
    }

    fn finish(self) -> State {
        STATE.with_borrow_mut(|slot| slot.take().expect("active fault session"))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        STATE.with_borrow_mut(|slot| {
            slot.take();
        });
    }
}

pub(crate) struct Ticket {
    kind: Kind,
    path: PathBuf,
    action: Option<Fault>,
    active: bool,
}

fn injected() -> io::Error {
    io::Error::other("C-07b deterministic I/O fault")
}

impl Ticket {
    pub(crate) fn begin(kind: Kind, path: &Path) -> io::Result<Self> {
        crash::io(kind, path, false);
        STATE.with_borrow_mut(|slot| {
            let Some(state) = slot else {
                return Ok(Self {
                    kind,
                    path: path.to_owned(),
                    action: None,
                    active: false,
                });
            };
            assert!(
                state.events.len() < MAX_EVENTS,
                "fault history exceeds event bound"
            );
            let action = state
                .faults
                .iter()
                .find(|(at, _)| *at == state.events.len())
                .map(|(_, fault)| *fault);
            state.events.push(Event {
                kind,
                path: path.to_owned(),
            });
            if action.is_some() {
                state.fired = true;
                state.fired_count += 1;
            }
            if action == Some(Fault::Before) {
                return Err(injected());
            }
            Ok(Self {
                kind,
                path: path.to_owned(),
                action,
                active: true,
            })
        })
    }

    pub(crate) fn write_limit(&self) -> io::Result<Option<usize>> {
        match self.action {
            Some(Fault::Short(count)) => Ok(Some(count)),
            Some(Fault::Zero) => Ok(Some(0)),
            Some(Fault::Interrupted) => Err(io::ErrorKind::Interrupted.into()),
            _ => Ok(None),
        }
    }

    fn effect(self, update: impl FnOnce(&mut Device, &Path)) -> io::Result<()> {
        if self.active {
            STATE.with_borrow_mut(|slot| update(&mut slot.as_mut().unwrap().device, &self.path));
        }
        self.done()
    }

    pub(crate) fn done(self) -> io::Result<()> {
        crash::io(self.kind, &self.path, true);
        if self.action == Some(Fault::After) {
            Err(injected())
        } else {
            Ok(())
        }
    }
    pub(crate) fn opened(self) -> io::Result<()> {
        self.effect(Device::opened)
    }
    pub(crate) fn written(self, offset: u64, bytes: &[u8]) -> io::Result<()> {
        self.effect(|device, path| device.write(path, offset, bytes))
    }
    pub(crate) fn synced(self) -> io::Result<()> {
        self.effect(Device::sync_file)
    }
    pub(crate) fn directory_synced(self) -> io::Result<()> {
        self.effect(Device::sync_directory)
    }
    pub(crate) fn renamed(self, target: &Path) -> io::Result<()> {
        self.effect(|device, source| device.rename(source, target))
    }
    pub(crate) fn removed(self) -> io::Result<()> {
        self.effect(Device::remove)
    }
}
