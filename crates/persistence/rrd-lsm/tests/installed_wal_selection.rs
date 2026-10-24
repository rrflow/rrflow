#![cfg(feature = "crash-test")]

use rrd_lsm::{crash_test, recover, Durability, WalBatch, WalWriter};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver};
use std::time::{Duration, Instant};

const LARGE: usize = 512 * 1024;
const PREFIX: &str = "C07C_NATIVE_PAUSE_V1=";

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn(root: &std::path::Path, control: Value) -> (OwnedChild, Receiver<String>) {
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "wal_selection_child", "--nocapture"])
            .env("RRFLOW_WAL_SELECTION_CHILD", root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    writeln!(child.0.stdin.as_mut().unwrap(), "{control}").unwrap();
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = sync_channel(16);
    std::thread::spawn(move || {
        // The owned test child emits only a few bounded status lines.
        for line in BufReader::new(stdout.take(8192)).lines() {
            if send.send(line.unwrap()).is_err() {
                break;
            }
        }
    });
    (child, receive)
}

fn status(lines: &Receiver<String>) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let line = lines
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if line.starts_with(PREFIX) || line.starts_with("selection:") {
            return line;
        }
    }
}

fn finish(child: &mut OwnedChild) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "bounded child completion");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn control() -> Value {
    json!({"version":1,"nonce":"a".repeat(64),"id":1,"point":"wal_before_write",
        "operation_sha256":"b".repeat(64),"resume":true})
}

#[test]
fn first_write_payload_is_observed_without_changing_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (mut child, lines) = spawn(root.path(), control());
    let line = status(&lines);
    let hit: Value = serde_json::from_str(line.strip_prefix(PREFIX).expect("WAL pause")).unwrap();
    assert_eq!(hit["pid"], child.0.id());
    assert_eq!(hit["wal_payload_bytes"], 1);
    writeln!(
        child.0.stdin.as_mut().unwrap(),
        "{}",
        json!({"version":1,"nonce":hit["nonce"],"id":hit["id"]})
    )
    .unwrap();
    assert_eq!(status(&lines), "selection:small-committed");
    assert_eq!(status(&lines), "selection:large-committed");
    finish(&mut child);
    let recovered = recover(&root.path().join("active.wal")).unwrap();
    assert_eq!(recovered.recovered_through, 2);
    assert_eq!(recovered.batches.len(), 2);
    assert_eq!(recovered.batches[0].payload, b"x");
    assert_eq!(recovered.batches[1].payload, vec![0x5a; LARGE]);
}

#[test]
fn wal_selection_child() {
    let Some(root) = std::env::var_os("RRFLOW_WAL_SELECTION_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    if crash_test::configure_from_stdin(&root).is_err() {
        println!("selection:denied");
        return;
    }
    let mut writer = WalWriter::create(&root.join("active.wal")).unwrap();
    let _scope = crash_test::enter_domain(&"b".repeat(64));
    writer
        .append(
            &WalBatch {
                first_sequence: 1,
                last_sequence: 1,
                payload: b"x",
            },
            Durability::Authoritative,
        )
        .unwrap();
    println!("selection:small-committed");
    writer
        .append(
            &WalBatch {
                first_sequence: 2,
                last_sequence: 2,
                payload: &vec![0x5a; LARGE],
            },
            Durability::Authoritative,
        )
        .unwrap();
    println!("selection:large-committed");
}
