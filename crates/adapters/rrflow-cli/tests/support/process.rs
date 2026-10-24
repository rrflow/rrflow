//! Bounded, test-owned process capture. This is qualification tooling only.

use std::io::{self, Read, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

// A concurrent fork can inherit another thread's writable executable descriptor
// until exec (rust-lang/rust#114554). Cover copies AND every spawn, not test runs.
static SPAWN_ADMISSION: Mutex<()> = Mutex::new(());

pub fn stage_executable(source: &std::path::Path, target: &std::path::Path) -> io::Result<u64> {
    let _admission = SPAWN_ADMISSION.lock().unwrap();
    std::fs::copy(source, target)
}

pub fn spawn(command: &mut Command) -> io::Result<Child> {
    let _admission = SPAWN_ADMISSION.lock().unwrap();
    command.spawn()
}

type Capture = (mpsc::Receiver<io::Result<Vec<u8>>>, JoinHandle<()>);

pub struct Process {
    child: Arc<Mutex<Child>>,
    cancel: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<&'static str>>>,
    monitor: Option<JoinHandle<()>>,
    capture: Vec<Capture>,
    control_input: Option<std::process::ChildStdin>,
}

impl Process {
    pub fn start(
        command: &mut Command,
        deadline: Duration,
        limit: usize,
        announcement: Option<mpsc::SyncSender<io::Result<Vec<u8>>>>,
    ) -> io::Result<Self> {
        Self::start_inner(command, deadline, limit, announcement, None, None, false)
    }

    #[cfg(feature = "crash-test")]
    #[allow(dead_code)] // Shared by integration targets; only the crash corpus uses it.
    pub fn start_observed(
        command: &mut Command,
        deadline: Duration,
        limit: usize,
        lines: mpsc::SyncSender<io::Result<Vec<u8>>>,
        input: &[u8],
    ) -> io::Result<Self> {
        Self::start_inner(
            command,
            deadline,
            limit,
            None,
            Some(lines),
            Some(input),
            false,
        )
    }

    #[cfg(feature = "crash-test")]
    #[allow(dead_code)]
    pub fn start_controlled(
        command: &mut Command,
        deadline: Duration,
        limit: usize,
        lines: mpsc::SyncSender<io::Result<Vec<u8>>>,
        input: &[u8],
    ) -> io::Result<Self> {
        if input.contains(&b'\n') || input.len() >= 4096 {
            return Err(io::Error::other(
                "private capacity control must be one bounded line",
            ));
        }
        Self::start_inner(
            command,
            deadline,
            limit,
            None,
            Some(lines),
            Some(input),
            true,
        )
    }

    #[cfg(feature = "crash-test")]
    #[allow(dead_code)]
    pub fn resume_controlled(&mut self, input: &[u8]) -> io::Result<()> {
        if input.contains(&b'\n') || input.len() >= 512 {
            return Err(io::Error::other(
                "private capacity resume must be one bounded line",
            ));
        }
        let mut pipe = self
            .control_input
            .take()
            .ok_or_else(|| io::Error::other("capacity control already consumed"))?;
        pipe.write_all(input)?;
        pipe.write_all(b"\n")?;
        pipe.flush()
    }

    fn start_inner(
        command: &mut Command,
        deadline: Duration,
        limit: usize,
        announcement: Option<mpsc::SyncSender<io::Result<Vec<u8>>>>,
        lines: Option<mpsc::SyncSender<io::Result<Vec<u8>>>>,
        input: Option<&[u8]>,
        retain_input: bool,
    ) -> io::Result<Self> {
        if let Some(input) = input {
            if input.len() > 4096 {
                return Err(io::Error::other("diagnostic input exceeds bound"));
            }
            command.stdin(Stdio::piped());
        }
        let mut child = spawn(command.stdout(Stdio::piped()).stderr(Stdio::piped()))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let child = Arc::new(Mutex::new(child));
        let cancel = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(Mutex::new(None));
        let mut process = Self {
            child: child.clone(),
            cancel: cancel.clone(),
            failure: failure.clone(),
            monitor: None,
            capture: Vec::new(),
            control_input: None,
        };
        process.capture(stdout, limit, announcement, lines);
        process.capture(stderr, limit, None, None);
        process.monitor = Some(thread::spawn(move || {
            let end = Instant::now() + deadline;
            loop {
                let mut child = child.lock().unwrap();
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Err(_) => {
                        *failure.lock().unwrap() = Some("child status unavailable");
                        cancel.store(true, Ordering::Release);
                    }
                    Ok(None) => {}
                }
                if Instant::now() >= end {
                    *failure.lock().unwrap() = Some("child exceeded its deadline");
                    cancel.store(true, Ordering::Release);
                }
                if cancel.load(Ordering::Acquire) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                drop(child);
                thread::sleep(Duration::from_millis(10));
            }
        }));
        if let Some(input) = input {
            // The writer sends one bounded control message; resumable callers
            // retain the private pipe for one subsequent message. The
            // process guard and monitor already own cleanup on any error.
            let mut pipe = stdin.expect("private diagnostic stdin");
            pipe.write_all(input)?;
            if retain_input {
                pipe.write_all(b"\n")?;
                pipe.flush()?;
                process.control_input = Some(pipe);
            }
        }
        Ok(process)
    }

    fn capture(
        &mut self,
        mut stream: impl Read + Send + 'static,
        limit: usize,
        announcement: Option<mpsc::SyncSender<io::Result<Vec<u8>>>>,
        lines: Option<mpsc::SyncSender<io::Result<Vec<u8>>>>,
    ) {
        let (send, receive) = mpsc::sync_channel(1);
        let cancel = self.cancel.clone();
        let failure = self.failure.clone();
        let reader = thread::spawn(move || {
            let result = (|| {
                let mut bytes = Vec::new();
                if let Some(lines) = lines {
                    let mut line = Vec::new();
                    let mut byte = [0];
                    while stream.read(&mut byte)? != 0 {
                        bytes.push(byte[0]);
                        line.push(byte[0]);
                        if bytes.len() > limit || line.len() > 65_536 {
                            return Err(io::Error::other(
                                "bounded diagnostic line capture exceeded",
                            ));
                        }
                        if byte[0] == b'\n' {
                            lines.try_send(Ok(std::mem::take(&mut line))).map_err(|_| {
                                io::Error::other("diagnostic event queue unavailable")
                            })?;
                        }
                    }
                    if !line.is_empty() {
                        return Err(io::Error::other("incomplete diagnostic event"));
                    }
                    return Ok(bytes);
                }
                if let Some(announcement) = announcement {
                    // No unbounded read_line allocation before readiness.
                    let mut byte = [0];
                    while bytes.len() < limit.min(65_536) {
                        if stream.read(&mut byte)? == 0 {
                            break;
                        }
                        bytes.push(byte[0]);
                        if byte[0] == b'\n' {
                            break;
                        }
                    }
                    if !bytes.ends_with(b"\n") {
                        return Err(io::Error::other(
                            "missing or oversized readiness announcement",
                        ));
                    }
                    let _ = announcement.send(Ok(bytes.clone()));
                }
                let remaining = limit.saturating_sub(bytes.len()) + 1;
                stream.take(remaining as u64).read_to_end(&mut bytes)?;
                if bytes.len() > limit {
                    return Err(io::Error::other("child exceeded its output limit"));
                }
                Ok(bytes)
            })();
            if result.is_err() {
                *failure.lock().unwrap() =
                    Some("child output capture failed or exceeded its limit");
                cancel.store(true, Ordering::Release);
            }
            let _ = send.send(result);
        });
        self.capture.push((receive, reader));
    }

    pub fn id(&self) -> u32 {
        self.child.lock().unwrap().id()
    }

    pub fn terminate(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    pub fn running(&self) -> bool {
        self.child
            .lock()
            .unwrap()
            .try_wait()
            .expect("read child status")
            .is_none()
    }

    pub fn finish(&mut self) -> Result<Output, String> {
        self.control_input.take();
        if let Some(monitor) = self.monitor.take() {
            monitor.join().map_err(|_| "child monitor panicked")?;
        }
        let mut streams = Vec::new();
        for (receive, reader) in self.capture.drain(..) {
            // A leaked descendant pipe cannot hang cleanup or allocate forever.
            let bytes = receive
                .recv_timeout(Duration::from_secs(2))
                .map_err(|_| "child output did not close within its deadline")?
                .map_err(|error| format!("child output capture failed: {error}"))?;
            reader.join().map_err(|_| "child capture panicked")?;
            streams.push(bytes);
        }
        if let Some(failure) = *self.failure.lock().unwrap() {
            return Err(failure.into());
        }
        if streams.len() != 2 {
            return Err("child output already consumed".into());
        }
        Ok(Output {
            status: self
                .child
                .lock()
                .unwrap()
                .wait()
                .map_err(|_| "reap child")?,
            stderr: streams.pop().unwrap(),
            stdout: streams.pop().unwrap(),
        })
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.control_input.take();
        self.terminate();
        if let Some(monitor) = self.monitor.take() {
            let _ = monitor.join();
        }
        for (receive, reader) in self.capture.drain(..) {
            if receive.recv_timeout(Duration::from_secs(2)).is_ok() {
                let _ = reader.join();
            }
        }
    }
}
