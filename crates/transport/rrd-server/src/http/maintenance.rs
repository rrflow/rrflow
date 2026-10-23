//! One joined worker driven by the existing server sweep. Offers while busy
//! are skipped; they never accumulate maintenance jobs behind foreground I/O.
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Default)]
struct State {
    pending: bool,
    running: bool,
    stopped: bool,
}

pub(super) struct MaintenanceWorker {
    shared: Arc<(Mutex<State>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl MaintenanceWorker {
    pub(super) fn start(mut step: impl FnMut() + Send + 'static) -> std::io::Result<Self> {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("rrd-maintenance".into())
            .spawn(move || {
                let (state, wake) = &*worker;
                loop {
                    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    while !guard.pending && !guard.stopped {
                        guard = wake.wait(guard).unwrap_or_else(|e| e.into_inner());
                    }
                    if guard.stopped {
                        break;
                    }
                    guard.pending = false;
                    guard.running = true;
                    drop(guard);
                    step();
                    state.lock().unwrap_or_else(|e| e.into_inner()).running = false;
                }
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub(super) fn offer(&self) -> super::Result<bool> {
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err(super::HttpError::Contract(
                "storage maintenance worker stopped".into(),
            ));
        }
        let (state, wake) = &*self.shared;
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        if state.stopped || state.running || state.pending {
            return Ok(false);
        }
        state.pending = true;
        wake.notify_one();
        Ok(true)
    }
}

impl Drop for MaintenanceWorker {
    fn drop(&mut self) {
        let (state, wake) = &*self.shared;
        {
            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
            state.stopped = true;
            state.pending = false;
            wake.notify_one();
        }
        // An admitted quantum uses installed cooperative I/O/maintenance
        // bounds. Cancellation never abandons a publisher in a detached task.
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                tracing::error!(target: "rrd_server::maintenance", "storage maintenance worker panicked");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn idle_maintenance_skips_busy_offers_and_joins_inflight_work() {
        let (started, began) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(0);
        let worker = MaintenanceWorker::start(move || {
            started.send(()).unwrap();
            released.recv().unwrap();
        })
        .unwrap();
        assert!(worker.offer().unwrap());
        began.recv_timeout(Duration::from_secs(5)).unwrap();
        for _ in 0..100 {
            assert!(!worker.offer().unwrap());
        }
        let (done, stopped) = mpsc::sync_channel(1);
        let shutdown = thread::spawn(move || {
            drop(worker);
            done.send(()).unwrap();
        });
        assert!(stopped.recv_timeout(Duration::from_millis(30)).is_err());
        release.send(()).unwrap();
        stopped.recv_timeout(Duration::from_secs(5)).unwrap();
        shutdown.join().unwrap();
        assert!(began.try_recv().is_err());
    }

    #[test]
    fn idle_maintenance_drop_wakes_an_idle_worker_and_panic_is_visible() {
        drop(MaintenanceWorker::start(|| {}).unwrap());
        let worker = MaintenanceWorker::start(|| panic!("injected maintenance panic")).unwrap();
        assert!(worker.offer().unwrap());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !worker.thread.as_ref().unwrap().is_finished() {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        assert!(worker.offer().is_err());
        drop(worker);
    }
}
