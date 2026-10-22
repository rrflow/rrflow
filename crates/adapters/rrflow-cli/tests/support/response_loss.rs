//! One bounded, test-owned relay. It withholds every response byte from the
//! client after forwarding exactly one authenticated commit to the real engine.
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const LIMIT: usize = 128 * 1024;

pub struct ResponseLoss {
    pub address: SocketAddr,
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

pub(super) fn bounded_io<T>(
    end: Instant,
    phase: &'static str,
    mut operation: impl FnMut(Duration) -> std::io::Result<T>,
) -> Result<T, String> {
    loop {
        let remaining = end
            .checked_duration_since(Instant::now())
            .ok_or_else(|| format!("{phase}: deadline"))?;
        match operation(remaining) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            // Error messages and wire data can contain secrets. Retain only the
            // closed OS error kind and the locally declared phase.
            result => return result.map_err(|error| format!("{phase}: {:?}", error.kind())),
        }
    }
}

fn read_part(
    stream: &mut TcpStream,
    bytes: &mut [u8],
    end: Instant,
    phase: &'static str,
) -> Result<usize, String> {
    bounded_io(end, phase, |remaining| {
        stream.set_read_timeout(Some(remaining))?;
        stream.read(bytes)
    })
}

pub(super) fn forward(
    mut client: TcpStream,
    target: SocketAddr,
    end: Instant,
) -> Result<(), String> {
    // accept() does not portably promise inherited or cleared O_NONBLOCK.
    // This worker uses blocking I/O with a remaining aggregate deadline.
    client
        .set_nonblocking(false)
        .map_err(|error| format!("client blocking mode: {:?}", error.kind()))?;
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 8192 {
            return Err("relay header limit".into());
        }
        let mut byte = [0];
        if read_part(&mut client, &mut byte, end, "client header read")? == 0 {
            return Err("relay incomplete header".into());
        }
        header.push(byte[0]);
    }
    let header = std::str::from_utf8(&header).map_err(|_| "relay header encoding")?;
    let mut lines = header.split("\r\n");
    let first = lines.next().ok_or("relay request line")?;
    if !first.starts_with("POST /v1/transactions/") || !first.ends_with("/commit HTTP/1.1") {
        return Err("relay requires one commit".into());
    }
    let mut outgoing = format!("{first}\r\n");
    let mut length = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or("relay header shape")?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("relay requires bounded content length".into());
        }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("relay duplicate content length".into());
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "relay content length")?,
            );
        }
        if !name.eq_ignore_ascii_case("connection") {
            outgoing.push_str(line);
            outgoing.push_str("\r\n");
        }
    }
    let length = length
        .filter(|length| *length <= LIMIT)
        .ok_or("relay request bound")?;
    outgoing.push_str("connection: close\r\n\r\n");
    let mut body = vec![0; length];
    let mut received = 0;
    while received < length {
        let count = read_part(&mut client, &mut body[received..], end, "client body read")?;
        if count == 0 {
            return Err("relay incomplete request".into());
        }
        received += count;
    }
    let mut server = bounded_io(end, "server connect", |remaining| {
        TcpStream::connect_timeout(&target, remaining)
    })?;
    for mut bytes in [outgoing.as_bytes(), body.as_slice()] {
        while !bytes.is_empty() {
            let count = bounded_io(end, "server write", |remaining| {
                server.set_write_timeout(Some(remaining))?;
                server.write(bytes)
            })?;
            if count == 0 {
                return Err("relay zero write".into());
            }
            bytes = &bytes[count..];
        }
    }
    // Wire bytes can contain credentials. They remain private, bounded memory
    // and are never printed or retained in a measurement artifact.
    let mut response = Vec::new();
    loop {
        let mut buffer = [0; 4096];
        let count = read_part(&mut server, &mut buffer, end, "server response read")?;
        if count == 0 {
            break;
        }
        if response.len() + count > LIMIT {
            return Err("relay response bound".into());
        }
        response.extend_from_slice(&buffer[..count]);
    }
    if !response.starts_with(b"HTTP/1.1 200 ") {
        return Err("relay did not intercept success".into());
    }
    // No write to client. EOF must be observed as an unknown transport result.
    drop(client);
    Ok(())
}

impl ResponseLoss {
    pub fn start(target: SocketAddr) -> Self {
        assert!(target.ip().is_loopback());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let stopped = cancel.clone();
        let worker = std::thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(60);
            while Instant::now() < end && !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((client, peer)) => {
                        if !peer.ip().is_loopback() {
                            return Err("relay peer".into());
                        }
                        return forward(client, target, end);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(format!("relay accept: {:?}", error.kind())),
                }
            }
            Err("relay canceled or expired".into())
        });
        Self {
            address,
            cancel,
            worker: Some(worker),
        }
    }

    pub fn finish(mut self) {
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("relay thread")
            .expect("bounded response loss");
    }
}

impl Drop for ResponseLoss {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
