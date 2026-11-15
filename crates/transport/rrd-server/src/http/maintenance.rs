//! One joined worker driven by native and configured cold-history sweeps.
//! Offers coalesce to at most one pending quantum of each kind.
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MaintenanceWork {
    Native,
    ColdHistory,
}

#[derive(Default)]
struct State {
    native_pending: bool,
    cold_pending: bool,
    last_started: Option<MaintenanceWork>,
    stopped: bool,
}

impl State {
    fn take_next(&mut self) -> Option<MaintenanceWork> {
        let work = match (self.native_pending, self.cold_pending, self.last_started) {
            (false, false, _) => return None,
            (true, true, Some(MaintenanceWork::Native)) => MaintenanceWork::ColdHistory,
            (true, _, _) => MaintenanceWork::Native,
            (false, true, _) => MaintenanceWork::ColdHistory,
        };
        match work {
            MaintenanceWork::Native => self.native_pending = false,
            MaintenanceWork::ColdHistory => self.cold_pending = false,
        }
        self.last_started = Some(work);
        Some(work)
    }
}

pub(super) struct MaintenanceWorker {
    shared: Arc<(Mutex<State>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl MaintenanceWorker {
    pub(super) fn start(
        mut step: impl FnMut(MaintenanceWork) + Send + 'static,
    ) -> std::io::Result<Self> {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("rrd-maintenance".into())
            .spawn(move || {
                let (state, wake) = &*worker;
                loop {
                    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    while !guard.native_pending && !guard.cold_pending && !guard.stopped {
                        guard = wake.wait(guard).unwrap_or_else(|e| e.into_inner());
                    }
                    if guard.stopped {
                        break;
                    }
                    let work = guard.take_next().expect("pending maintenance work");
                    drop(guard);
                    step(work);
                }
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub(super) fn offer(&self, work: MaintenanceWork) -> super::Result<bool> {
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err(super::HttpError::Contract(
                "storage maintenance worker stopped".into(),
            ));
        }
        let (state, wake) = &*self.shared;
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        if state.stopped {
            return Ok(false);
        }
        let pending = match work {
            MaintenanceWork::Native => &mut state.native_pending,
            MaintenanceWork::ColdHistory => &mut state.cold_pending,
        };
        if *pending {
            return Ok(false);
        }
        *pending = true;
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
            state.native_pending = false;
            state.cold_pending = false;
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
    fn idle_maintenance_coalesces_offers_and_joins_inflight_work() {
        let (started, began) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(0);
        let worker = MaintenanceWorker::start(move |work| {
            started.send(work).unwrap();
            released.recv().unwrap();
        })
        .unwrap();
        assert!(worker.offer(MaintenanceWork::ColdHistory).unwrap());
        assert_eq!(
            began.recv_timeout(Duration::from_secs(5)).unwrap(),
            MaintenanceWork::ColdHistory
        );
        assert!(worker.offer(MaintenanceWork::Native).unwrap());
        for _ in 0..100 {
            assert!(!worker.offer(MaintenanceWork::Native).unwrap());
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
    fn pending_kinds_alternate_and_duplicate_offers_stay_bounded() {
        let (started, began) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(0);
        let worker = MaintenanceWorker::start(move |work| {
            started.send(work).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
        })
        .unwrap();
        assert!(worker.offer(MaintenanceWork::Native).unwrap());
        assert_eq!(
            began.recv_timeout(Duration::from_secs(5)).unwrap(),
            MaintenanceWork::Native
        );
        assert!(worker.offer(MaintenanceWork::Native).unwrap());
        assert!(worker.offer(MaintenanceWork::ColdHistory).unwrap());
        for _ in 0..100 {
            assert!(!worker.offer(MaintenanceWork::Native).unwrap());
            assert!(!worker.offer(MaintenanceWork::ColdHistory).unwrap());
        }
        release.send(()).unwrap();
        assert_eq!(
            began.recv_timeout(Duration::from_secs(5)).unwrap(),
            MaintenanceWork::ColdHistory
        );
        assert!(worker.offer(MaintenanceWork::ColdHistory).unwrap());
        release.send(()).unwrap();
        assert_eq!(
            began.recv_timeout(Duration::from_secs(5)).unwrap(),
            MaintenanceWork::Native
        );
        release.send(()).unwrap();
        assert_eq!(
            began.recv_timeout(Duration::from_secs(5)).unwrap(),
            MaintenanceWork::ColdHistory
        );
        release.send(()).unwrap();
        drop(worker);
        assert!(began.try_recv().is_err());
    }

    #[test]
    fn idle_maintenance_drop_wakes_an_idle_worker_and_panic_is_visible() {
        drop(MaintenanceWorker::start(|_| {}).unwrap());
        let worker = MaintenanceWorker::start(|_| panic!("injected maintenance panic")).unwrap();
        assert!(worker.offer(MaintenanceWork::Native).unwrap());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !worker.thread.as_ref().unwrap().is_finished() {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        assert!(worker.offer(MaintenanceWork::Native).is_err());
        drop(worker);
    }
}
