//! Real subprocess failure oracles for the lifecycle qualification harness.
#[path = "support/process.rs"]
mod process;
#[path = "support/response_loss.rs"]
mod response_loss;

use std::{
    io::Write,
    process::Command,
    time::{Duration, Instant},
};

fn fixture(mode: &str, deadline: Duration, limit: usize) -> process::Process {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "owned_child", "--ignored", "--nocapture"])
        .env("RRFLOW_CAPTURE_FIXTURE", mode);
    process::Process::start(&mut command, deadline, limit, None).unwrap()
}

#[test]
#[ignore = "explicit subprocess fixture; every mode is invoked by a parent test"]
fn owned_child() {
    match std::env::var("RRFLOW_CAPTURE_FIXTURE").unwrap().as_str() {
        "hang" => std::thread::sleep(Duration::from_secs(30)),
        "flood" => {
            let mut output = std::io::stdout().lock();
            loop {
                if output.write_all(&[b'x'; 8192]).is_err() {
                    break;
                }
            }
        }
        "exit" => {
            println!("bounded stdout");
            eprintln!("bounded stderr");
        }
        #[cfg(feature = "crash-test")]
        "partial" => {
            std::io::stdout().write_all(b"partial event").unwrap();
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }
        other => panic!("unknown fixture {other}"),
    }
}

#[cfg(feature = "crash-test")]
fn observed_fixture(
    mode: &str,
    capacity: usize,
) -> (
    process::Process,
    std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "owned_child", "--ignored", "--nocapture"])
        .env("RRFLOW_CAPTURE_FIXTURE", mode);
    let (send, receive) = std::sync::mpsc::sync_channel(capacity);
    let process = process::Process::start_observed(
        &mut command,
        Duration::from_secs(10),
        1024 * 1024,
        send,
        b"null",
    )
    .unwrap();
    (process, receive)
}

#[cfg(feature = "crash-test")]
#[test]
fn observed_events_preserve_complete_lines_and_both_streams() {
    let (mut child, receive) = observed_fixture("exit", 32);
    let output = child.finish().unwrap();
    assert!(output.status.success());
    let lines = receive.try_iter().map(Result::unwrap).collect::<Vec<_>>();
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|line| line.ends_with(b"\n")));
    assert_eq!(lines.concat(), output.stdout);
    assert!(String::from_utf8_lossy(&output.stderr).contains("bounded stderr"));
}

#[cfg(feature = "crash-test")]
#[test]
fn observed_queue_and_line_failures_reap_only_the_owned_child() {
    for (mode, capacity) in [("exit", 1), ("flood", 32), ("partial", 32)] {
        let (mut child, _receive) = observed_fixture(mode, capacity);
        let error = child.finish().unwrap_err();
        assert!(
            error.contains("output") || error.contains("diagnostic"),
            "{error}"
        );
        assert!(!child.running());
    }
}

#[cfg(feature = "crash-test")]
#[test]
fn diagnostic_input_is_bounded_before_any_child_can_spawn() {
    let (send, _receive) = std::sync::mpsc::sync_channel(1);
    let result = process::Process::start_observed(
        &mut Command::new("must-not-spawn"),
        Duration::from_secs(1),
        1024,
        send,
        &vec![b'x'; 4097],
    );
    match result {
        Err(error) => assert_eq!(error.to_string(), "diagnostic input exceeds bound"),
        Ok(_) => panic!("oversized input admitted"),
    }
}

#[test]
fn deadline_terminates_and_reaps_the_owned_child() {
    let start = Instant::now();
    let mut child = fixture("hang", Duration::from_secs(2), 4096);
    assert!(child.id() > 0);
    assert!(child.finish().unwrap_err().contains("deadline"));
    assert!(!child.running());
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn output_flood_fails_without_retaining_unbounded_bytes() {
    let start = Instant::now();
    let mut child = fixture("flood", Duration::from_secs(10), 1024);
    assert!(child.finish().unwrap_err().contains("output"));
    assert!(!child.running());
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn readiness_without_a_bounded_complete_line_is_rejected() {
    #[cfg(windows)]
    let mut command = {
        let shell = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/cmd.exe");
        let mut command = Command::new(shell);
        command.args(["/d", "/c", "echo oversized-readiness-announcement"]);
        command
    };
    #[cfg(unix)]
    let mut command = {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf 'oversized-readiness-announcement\\n'"]);
        command
    };
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let mut child =
        process::Process::start(&mut command, Duration::from_secs(10), 8, Some(send)).unwrap();
    // A fixed native helper emits a line longer than eight bytes. No partial
    // line may be accepted as the product's listening announcement.
    assert!(receive.recv_timeout(Duration::from_secs(5)).is_err());
    assert!(child.finish().unwrap_err().contains("announcement"));
    assert!(!child.running());
}

#[test]
fn failed_startup_retains_bounded_stderr_and_reaps_the_child() {
    let diagnostic = format!("discarded-prefix{}retained-startup-cause", "x".repeat(3000));
    #[cfg(windows)]
    let mut command = {
        let shell = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/cmd.exe");
        let mut command = Command::new(shell);
        command.args(["/d", "/c", &format!("echo {diagnostic} 1>&2 & exit /b 17")]);
        command
    };
    #[cfg(unix)]
    let mut command = {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", &format!("printf '{diagnostic}\\n' >&2; exit 17")]);
        command
    };
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let mut child =
        process::Process::start(&mut command, Duration::from_secs(10), 4096, Some(send)).unwrap();
    assert!(receive.recv_timeout(Duration::from_secs(5)).is_err());
    let failure = child.finish().unwrap_err();
    assert!(failure.contains("announcement"), "{failure}");
    assert!(failure.contains("retained-startup-cause"), "{failure}");
    assert!(!failure.contains("discarded-prefix"));
    assert!(failure.len() < 2300);
    assert!(!child.running());
}

#[test]
fn finite_output_preserves_both_streams_and_status() {
    let mut child = fixture("exit", Duration::from_secs(10), 4096);
    let output = child.finish().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("bounded stdout"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("bounded stderr"));
}

#[test]
fn concurrent_staging_preserves_executable_bytes() {
    let source = std::env::current_exe().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::thread::scope(|scope| {
        for index in 0..4 {
            let target = root
                .path()
                .join(format!("fixture-{index}{}", std::env::consts::EXE_SUFFIX));
            let source = &source;
            scope.spawn(move || {
                assert_eq!(
                    process::stage_executable(source, &target).unwrap(),
                    std::fs::metadata(source).unwrap().len()
                );
                let mut command = Command::new(target);
                command
                    .args(["--exact", "owned_child", "--ignored", "--nocapture"])
                    .env("RRFLOW_CAPTURE_FIXTURE", "exit");
                let mut child =
                    process::Process::start(&mut command, Duration::from_secs(10), 4096, None)
                        .unwrap();
                assert!(child.finish().unwrap().status.success());
            });
        }
    });
}

fn answer_relay(listener: std::net::TcpListener) {
    use std::io::Read;
    listener.set_nonblocking(true).unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    let mut server = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < end =>
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("bounded fixture accept: {:?}", error.kind()),
        }
    };
    server.set_nonblocking(false).unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    server
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let expected = b"POST /v1/transactions/fixture/commit HTTP/1.1\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}";
    let mut received = vec![0; expected.len()];
    server.read_exact(&mut received).unwrap();
    assert_eq!(received, expected);
    server
        .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}")
        .unwrap();
}

#[test]
fn relay_normalizes_nonblocking_accept_before_fragmented_request() {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    let front = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = TcpStream::connect(front.local_addr().unwrap()).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let (accepted, _) = front.accept().unwrap();
    // Explicitly model inherited O_NONBLOCK on every host, without relying on
    // which flags its accept implementation happens to inherit.
    accepted.set_nonblocking(true).unwrap();
    let backend = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = backend.local_addr().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        send.send(response_loss::forward(
            accepted,
            address,
            Instant::now() + Duration::from_secs(3),
        ))
        .unwrap();
    });
    for fragment in [
        b"POST /v1/".as_slice(),
        b"transactions/fixture/commit HTTP/1.1\r\ncontent-length: 2\r\n\r\n{",
    ] {
        assert!(
            matches!(
                receive.recv_timeout(Duration::from_millis(50)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ),
            "relay must wait for fragmented bytes"
        );
        client.write_all(fragment).unwrap();
    }
    assert!(matches!(
        receive.recv_timeout(Duration::from_millis(50)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    client.write_all(b"}").unwrap();
    answer_relay(backend);
    receive
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
        .unwrap();
    worker.join().unwrap();
    assert_eq!(
        client.read(&mut [0; 1]).unwrap(),
        0,
        "response bytes escaped"
    );
}

#[test]
fn relay_suppresses_response_and_idle_drop_reaps_accept() {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    let backend = TcpListener::bind("127.0.0.1:0").unwrap();
    let relay = response_loss::ResponseLoss::start(backend.local_addr().unwrap());
    let mut client = TcpStream::connect(relay.address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .write_all(b"POST /v1/transactions/fixture/commit HTTP/1.1\r\ncontent-length: 2\r\n\r\n{}")
        .unwrap();
    answer_relay(backend);
    relay.finish();
    assert_eq!(client.read(&mut [0; 1]).unwrap(), 0);
    let started = Instant::now();
    let unused = TcpListener::bind("127.0.0.1:0").unwrap();
    drop(response_loss::ResponseLoss::start(
        unused.local_addr().unwrap(),
    ));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn relay_interruption_keeps_the_deadline_and_redacts_error_messages() {
    use std::io::{Error, ErrorKind};
    let mut attempts = 0;
    let result =
        response_loss::bounded_io(Instant::now() + Duration::from_secs(1), "fixture", |_| {
            attempts += 1;
            if attempts == 1 {
                Err(Error::from(ErrorKind::Interrupted))
            } else {
                Ok(7)
            }
        });
    assert_eq!(result.unwrap(), 7);
    assert_eq!(attempts, 2);
    let end = Instant::now() + Duration::from_millis(20);
    let expired = response_loss::bounded_io::<()>(end, "fixture", |_| {
        std::thread::sleep(Duration::from_millis(1));
        Err(Error::from(ErrorKind::Interrupted))
    })
    .unwrap_err();
    assert_eq!(expired, "fixture: deadline");
    let denied =
        response_loss::bounded_io::<()>(Instant::now() + Duration::from_secs(1), "fixture", |_| {
            Err(Error::new(
                ErrorKind::PermissionDenied,
                "sensitive wire message",
            ))
        })
        .unwrap_err();
    assert_eq!(denied, "fixture: PermissionDenied");
}

#[test]
fn relay_stalled_header_expires_without_forwarding() {
    use std::net::{TcpListener, TcpStream};
    let front = TcpListener::bind("127.0.0.1:0").unwrap();
    let _client = TcpStream::connect(front.local_addr().unwrap()).unwrap();
    let (accepted, _) = front.accept().unwrap();
    let backend = TcpListener::bind("127.0.0.1:0").unwrap();
    backend.set_nonblocking(true).unwrap();
    let started = Instant::now();
    let error = response_loss::forward(
        accepted,
        backend.local_addr().unwrap(),
        started + Duration::from_millis(30),
    )
    .unwrap_err();
    assert!(error.starts_with("client header read: "));
    assert!(
        error.ends_with("TimedOut") || error.ends_with("WouldBlock") || error.ends_with("deadline")
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(
        backend.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn relay_denies_ambiguous_or_oversized_requests_before_connecting() {
    use std::net::{TcpListener, TcpStream};
    let prefix = "POST /v1/transactions/fixture/commit HTTP/1.1\r\n";
    let cases = [
        (
            format!("{prefix}content-length: 0\r\ncontent-length: 0\r\n\r\n"),
            "relay duplicate content length",
        ),
        (
            format!("{prefix}transfer-encoding: chunked\r\n\r\n"),
            "relay requires bounded content length",
        ),
        (
            format!("{prefix}content-length: 131073\r\n\r\n"),
            "relay request bound",
        ),
        ("x".repeat(8193), "relay header limit"),
    ];
    for (request, expected) in cases {
        let front = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(front.local_addr().unwrap()).unwrap();
        client
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let (accepted, _) = front.accept().unwrap();
        let backend = TcpListener::bind("127.0.0.1:0").unwrap();
        backend.set_nonblocking(true).unwrap();
        client.write_all(request.as_bytes()).unwrap();
        let error = response_loss::forward(
            accepted,
            backend.local_addr().unwrap(),
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap_err();
        assert_eq!(error, expected);
        assert_eq!(
            backend.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}
