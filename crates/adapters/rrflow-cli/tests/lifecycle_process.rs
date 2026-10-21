//! Real subprocess failure oracles for the lifecycle qualification harness.
#[path = "support/process.rs"]
mod process;

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
        other => panic!("unknown fixture {other}"),
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
