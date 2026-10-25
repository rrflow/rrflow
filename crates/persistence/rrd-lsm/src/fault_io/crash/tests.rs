use super::*;
use crate::{Database, Durability, Mutation, WriteBatch};
use ring::rand::{SecureRandom, SystemRandom};
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;

const CHILD_TEST: &str = "fault_io::crash::tests::owned_child";
const OUTPUT_BYTES: u64 = 8192;

fn pair(id: u16) -> WriteBatch {
    WriteBatch::new(
        (0..2)
            .map(|column| Mutation::Put {
                key: [id.to_be_bytes().as_slice(), &[column]].concat(),
                value: vec![column, 42, (id % 251) as u8],
            })
            .collect(),
    )
    .unwrap()
}

fn assert_history(db: &Database, ids: std::ops::RangeInclusive<u16>) {
    for id in ids {
        for column in 0..2 {
            assert_eq!(
                db.get(
                    &[id.to_be_bytes().as_slice(), &[column]].concat(),
                    db.snapshot()
                )
                .unwrap(),
                Some(vec![column, 42, (id % 251) as u8]),
                "lost or partial acknowledged pair {id}"
            );
        }
    }
}

fn fixture(existing: bool) -> (tempfile::TempDir, PathBuf) {
    // Use the test artifact's filesystem, not an assumed /tmp block device.
    let executable = std::env::current_exe().unwrap();
    let temp = tempfile::Builder::new()
        .prefix("c07c-native-")
        .tempdir_in(executable.parent().unwrap())
        .unwrap();
    let root = temp.path().join("database");
    let mut db = Database::create(&root).unwrap();
    db.write(&pair(1), Durability::Authoritative).unwrap();
    if existing {
        db.flush_memtable(1).unwrap();
        db.write(&pair(2), Durability::Authoritative).unwrap();
        db.flush_memtable(2).unwrap();
    }
    drop(db);
    (temp, std::fs::canonicalize(root).unwrap())
}

fn schedule(root: &Path, point: Point, id: u16, owned: bool) -> Schedule {
    let mut nonce = [0; 32];
    SystemRandom::new().fill(&mut nonce).unwrap();
    Schedule {
        root: root.to_owned(),
        nonce: nonce.iter().map(|b| format!("{b:02x}")).collect(),
        point,
        id,
        owned,
    }
}

struct OwnedChild {
    child: Child,
    stdout: Option<JoinHandle<Vec<u8>>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
}

impl OwnedChild {
    fn start(schedule: &Schedule) -> (Self, mpsc::Receiver<Result<Hit, String>>) {
        let bytes = serde_json::to_vec(schedule).unwrap();
        assert!(bytes.len() <= 4096);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", CHILD_TEST, "--ignored", "--nocapture"])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let mut owned = Self {
            child: command.spawn().unwrap(),
            stdout: None,
            stderr: None,
        };
        let stdout = owned.child.stdout.take().unwrap();
        let stderr = owned.child.stderr.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout.take(OUTPUT_BYTES + 1));
            let mut all = Vec::new();
            let mut sent = false;
            loop {
                let mut line = Vec::new();
                let n = reader.read_until(b'\n', &mut line).unwrap();
                if n == 0 {
                    break;
                }
                if let Some(body) = line.strip_prefix(PREFIX.as_bytes()) {
                    assert!(!sent, "duplicate native pause receipt");
                    sent = true;
                    let parsed = serde_json::from_slice(body)
                        .map_err(|_| "malformed native pause receipt".to_owned());
                    let _ = send.send(parsed);
                }
                all.extend_from_slice(&line);
            }
            if !sent {
                let _ = send.send(Err("native boundary was not reached".into()));
            }
            assert!(all.len() as u64 <= OUTPUT_BYTES, "child stdout budget");
            all
        });
        let errors = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr
                .take(OUTPUT_BYTES + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            assert!(bytes.len() as u64 <= OUTPUT_BYTES, "child stderr budget");
            bytes
        });
        owned.stdout = Some(reader);
        owned.stderr = Some(errors);
        owned.child.stdin.take().unwrap().write_all(&bytes).unwrap();
        (owned, receive)
    }

    fn kill_and_reap(&mut self) {
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "child exited before owned kill"
        );
        self.child.kill().unwrap();
        assert!(!self.child.wait().unwrap().success());
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Join even on failed admission: no orphan processes or pipe readers.
        for reader in [&mut self.stdout, &mut self.stderr] {
            if let Some(reader) = reader.take() {
                let _ = reader.join();
            }
        }
    }
}

fn validate_hit(schedule: &Schedule, pid: u32, hit: &Hit) -> Result<(), &'static str> {
    if hit.nonce != schedule.nonce
        || hit.pid != pid
        || hit.point != schedule.point
        || hit.id != schedule.id
    {
        return Err("native pause identity mismatch");
    }
    Ok(())
}

#[derive(Serialize)]
struct KillReceipt {
    hit: Hit,
    hit_ns: u64,
    reaped_ns: u64,
}

fn kill_at(schedule: &Schedule) -> KillReceipt {
    let started = Instant::now();
    let (mut child, receive) = OwnedChild::start(schedule);
    let hit = receive
        .recv_timeout(Duration::from_secs(20))
        .expect("bounded native pause deadline")
        .expect("native boundary must be reached");
    validate_hit(schedule, child.child.id(), &hit).unwrap();
    let hit_ns = started.elapsed().as_nanos().try_into().unwrap();
    child.kill_and_reap();
    let reaped_ns = started.elapsed().as_nanos().try_into().unwrap();
    // A failure in either capture worker must fail evidence, not be swallowed.
    for reader in [&mut child.stdout, &mut child.stderr] {
        reader
            .take()
            .unwrap()
            .join()
            .expect("bounded child capture");
    }
    KillReceipt {
        hit,
        hit_ns,
        reaped_ns,
    }
}

fn executable_sha256() -> String {
    let mut file = std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    hash.finish()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
#[ignore = "owned child fixture, invoked explicitly by native crash tests"]
fn owned_child() {
    let mut bytes = Vec::new();
    std::io::stdin().take(4097).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= 4096);
    let plan: Schedule = serde_json::from_slice(&bytes).unwrap();
    if plan.point == Point::RecoveryRead {
        arm(plan.clone());
    }
    let mut db = Database::open(&plan.root).unwrap();
    match plan.point {
        Point::RecoveryRead => panic!("recovery boundary not reached"),
        Point::SegmentSynced
        | Point::SuccessorWalSynced
        | Point::CurrentBeforeRename
        | Point::CurrentRenamed => {
            // No new semantic operation: every input was acknowledged before
            // the flush starts. Publication cannot lose an earlier ACK.
            db.write(&pair(plan.id), Durability::Authoritative).unwrap();
            arm(plan);
            db.flush_memtable(10).unwrap();
        }
        Point::ReclaimBeforeRemove | Point::ReclaimRemoved => {
            db.write(&pair(plan.id), Durability::Authoritative).unwrap();
            db.flush_memtable(10).unwrap();
            arm(plan);
            db.garbage_collect().unwrap();
        }
        Point::Acknowledged => {
            let receipt = db.write(&pair(plan.id), Durability::Authoritative).unwrap();
            assert!(receipt.durable);
            assert_eq!(receipt.last_sequence - receipt.first_sequence, 1);
            arm(plan);
            pause_if(|_| true);
        }
        _ => {
            arm(plan.clone());
            if plan.owned {
                db.write_owned(pair(plan.id), Durability::Authoritative)
                    .unwrap();
            } else {
                db.write(&pair(plan.id), Durability::Authoritative).unwrap();
            }
        }
    }
    panic!("native boundary not reached");
}

#[test]
fn unreached_and_mismatched_native_schedules_cannot_claim_a_kill() {
    let (_temp, root) = fixture(false);
    let plan = schedule(&root, Point::Unreachable, 2, false);
    let (mut child, receive) = OwnedChild::start(&plan);
    assert!(receive
        .recv_timeout(Duration::from_secs(20))
        .unwrap()
        .is_err());
    assert!(!child.child.wait().unwrap().success());
    let hit = Hit {
        nonce: plan.nonce.clone(),
        pid: 1,
        point: plan.point,
        id: plan.id,
    };
    assert!(validate_hit(&plan, 2, &hit).is_err());
    let other = schedule(&root, Point::Unreachable, 2, false);
    assert!(validate_hit(&other, 1, &hit).is_err());
    let mut other = plan.clone();
    other.point = Point::WalSynced;
    assert!(validate_hit(&other, 1, &hit).is_err());
    other = plan.clone();
    other.id += 1;
    assert!(validate_hit(&other, 1, &hit).is_err());
    assert!(
        serde_json::from_str::<Hit>(r#"{"nonce":"x","pid":1,"point":"unknown","id":2}"#).is_err()
    );
}

#[test]
fn native_pre_wal_kill_preserves_ack_and_denies_unwritten_pair() {
    let (_temp, root) = fixture(false);
    kill_at(&schedule(&root, Point::WalBeforeWrite, 2, false));
    let db = Database::open(&root).unwrap();
    assert_history(&db, 1..=1);
    assert_eq!(db.snapshot().sequence, 2);
    assert!(db.get(&[0, 2, 0], db.snapshot()).unwrap().is_none());
    assert!(db.get(&[0, 2, 1], db.snapshot()).unwrap().is_none());
}

#[test]
fn native_boundaries_and_compound_reopen_preserve_atomic_history() {
    let mut hits = Vec::new();
    for existing in [false, true] {
        for owned in [false, true] {
            for point in [
                Point::WalBeforeWrite,
                Point::WalAppended,
                Point::WalSynced,
                Point::Visible,
                Point::SegmentSynced,
                Point::SuccessorWalSynced,
                Point::CurrentBeforeRename,
                Point::CurrentRenamed,
                Point::ReclaimBeforeRemove,
                Point::ReclaimRemoved,
            ] {
                let (_temp, root) = fixture(existing);
                let id = if existing { 3 } else { 2 };
                hits.push(kill_at(&schedule(&root, point, id, owned)));
                let last = if point == Point::WalBeforeWrite {
                    id - 1
                } else {
                    id
                };
                // Ensure recovery has a WAL payload even after published flush;
                // readback is checked before adding this separately ACKed pair.
                let mut db = Database::open(&root).unwrap();
                assert_history(&db, 1..=last);
                assert_eq!(db.snapshot().sequence, u64::from(last) * 2);
                db.write(&pair(last + 1), Durability::Authoritative)
                    .unwrap();
                drop(db);
                for _ in 0..2 {
                    hits.push(kill_at(&schedule(
                        &root,
                        Point::RecoveryRead,
                        last + 1,
                        owned,
                    )));
                }
                let db = Database::open(&root).unwrap();
                assert_history(&db, 1..=last + 1);
                assert_eq!(db.snapshot().sequence, u64::from(last + 1) * 2);
            }
        }
    }
    assert_eq!(hits.len(), 120);
    println!(
        "native_crash_boundary_receipt={}",
        serde_json::json!({"instrumentation":"rrd-lsm-unit-test-only", "executable_sha256":executable_sha256(), "native_os":std::env::consts::OS,"boundary_cases":40,"compound_recovery_kills":80,"hits":hits})
    );
}

#[test]
fn one_hundred_acknowledged_native_kill_reopen_cycles_preserve_exact_sequence() {
    for existing in [false, true] {
        let (_temp, root) = fixture(existing);
        let first = if existing { 3 } else { 2 };
        let mut hits = Vec::new();
        for id in first..first + 100 {
            hits.push(kill_at(&schedule(&root, Point::Acknowledged, id, true)));
            let db = Database::open(&root).unwrap();
            assert_history(&db, 1..=id);
            assert_eq!(db.snapshot().sequence, u64::from(id) * 2);
        }
        assert_eq!(hits.len(), 100);
        println!(
            "native_crash_cycle_receipt={}",
            serde_json::json!({"instrumentation":"rrd-lsm-unit-test-only", "executable_sha256":executable_sha256(), "native_os":std::env::consts::OS,"existing":existing,"cycles":100,"pairs":first + 99,"last_sequence":(first + 99) * 2,"hits":hits})
        );
    }
}
