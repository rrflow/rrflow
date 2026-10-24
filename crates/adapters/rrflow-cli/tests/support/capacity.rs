//! Real native capacity corpus. Only the owned-image runner supplies admission.
use super::*;
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Volume {
    version: u16,
    root: PathBuf,
    nonce: String,
    filesystem: String,
    max_bytes: u64,
}

impl Volume {
    fn admitted() -> Self {
        let descriptor = std::env::var_os("RRFLOW_CAPACITY_DESCRIPTOR").expect(
            "run through scripts/ci/run_native_capacity.py; arbitrary volume roots are forbidden",
        );
        let mut bytes = Vec::new();
        File::open(descriptor)
            .unwrap()
            .take(4097)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 4096);
        let mut volume: Self = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(volume.version, 1);
        assert_eq!(volume.nonce.len(), 64);
        assert!(volume.nonce.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!((128 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&volume.max_bytes));
        assert!(volume.root.is_absolute());
        assert!(!std::fs::symlink_metadata(&volume.root)
            .unwrap()
            .file_type()
            .is_symlink());
        volume.root = std::fs::canonicalize(&volume.root).unwrap();
        assert_ne!(
            volume.root,
            std::fs::canonicalize(std::env::temp_dir()).unwrap()
        );
        volume.verify_native_mount();
        assert!(matches!(
            volume.filesystem.as_str(),
            "ext4" | "apfs" | "ntfs"
        ));
        let marker = volume.root.join("RRFLOW-CAPACITY-OWNER");
        assert!(std::fs::symlink_metadata(&marker).unwrap().is_file());
        let mut owner = Vec::new();
        File::open(marker)
            .unwrap()
            .take(65)
            .read_to_end(&mut owner)
            .unwrap();
        assert_eq!(owner, volume.nonce.as_bytes());
        volume
    }

    #[cfg(not(target_os = "macos"))]
    fn verify_native_mount(&self) {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let matching: Vec<_> = disks
            .iter()
            .filter(|disk| {
                std::fs::canonicalize(disk.mount_point()).ok().as_ref() == Some(&self.root)
            })
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "fixture must be a separately enumerated native volume"
        );
        let disk = matching[0];
        assert_eq!(
            disk.file_system().to_str().unwrap().to_ascii_lowercase(),
            self.filesystem
        );
        assert!(disk.total_space() > 32 * 1024 * 1024 && disk.total_space() <= self.max_bytes);
    }

    #[cfg(target_os = "macos")]
    fn verify_native_mount(&self) {
        use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
        // sysinfo intentionally hides non-browsable APFS volumes. Query the
        // kernel mount itself; visibility in Finder is not volume admission.
        let path = std::ffi::CString::new(self.root.as_os_str().as_bytes()).unwrap();
        let mut native = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: NUL-terminated path and writable, correctly sized output.
        let result = unsafe { libc::statfs(path.as_ptr(), native.as_mut_ptr()) };
        assert_eq!(
            result,
            0,
            "statfs failed: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: successful statfs initialized every output field.
        let native = unsafe { native.assume_init() };
        let text = |field: &[libc::c_char]| {
            let end = field
                .iter()
                .position(|c| *c == 0)
                .expect("terminated statfs field");
            field[..end].iter().map(|c| *c as u8).collect::<Vec<_>>()
        };
        let mount = text(&native.f_mntonname);
        assert_eq!(
            std::fs::canonicalize(std::ffi::OsStr::from_bytes(&mount)).unwrap(),
            self.root,
            "fixture must be an exact native mount root"
        );
        assert_eq!(text(&native.f_fstypename), self.filesystem.as_bytes());
        assert_eq!(self.filesystem, "apfs");
        assert_ne!(
            std::fs::metadata(&self.root).unwrap().dev(),
            std::fs::metadata(self.root.parent().unwrap())
                .unwrap()
                .dev(),
            "fixture must be a separate filesystem"
        );
        let total = native
            .f_blocks
            .checked_mul(u64::from(native.f_bsize))
            .expect("capacity overflow");
        assert!(total > 32 * 1024 * 1024 && total <= self.max_bytes);
    }
}

struct Filler {
    file: Option<File>,
    path: PathBuf,
    bytes: u64,
    native_code: i32,
    metadata_paths: Vec<PathBuf>,
    metadata_directories: Vec<PathBuf>,
    metadata_native_code: Option<i32>,
    free_inodes: Option<u64>,
}

// Filler files only need OS allocation, not survival of a power failure.
// Rust's sync_all AND sync_data request F_FULLFSYNC on Apple targets: using
// either for every empty filler file flushes the device's hardware cache.
// Keep that stronger barrier on authoritative engine files, never here.
fn sync_filler_allocation(file: &File) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        sync_filler_host(file)
    }
    #[cfg(not(target_os = "macos"))]
    {
        file.sync_data()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn sync_filler_host(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the borrowed File owns this live descriptor for the whole call.
    // No retry/fallback can turn a native allocation or I/O error into success.
    if unsafe { libc::fsync(file.as_raw_fd()) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

impl Filler {
    fn fill(volume: &Volume, metadata_directory: Option<&std::path::Path>) -> Self {
        let directory = metadata_directory.map(|path| {
            assert!(!std::fs::symlink_metadata(path)
                .unwrap()
                .file_type()
                .is_symlink());
            let path = std::fs::canonicalize(path).unwrap();
            assert!(
                path.starts_with(&volume.root) && path != volume.root,
                "metadata fill must remain inside the owned estate volume"
            );
            path
        });
        let path = volume.root.join(format!("{}.filler", volume.nonce));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut filler = Self {
            file: None,
            path,
            bytes: 0,
            native_code: 0,
            metadata_paths: Vec::new(),
            metadata_directories: Vec::new(),
            metadata_native_code: None,
            free_inodes: None,
        };
        // Nonzero data prevents sparse/zero-block optimizations. Allocation and
        // sync failures are actual OS results, never injected StorageFull.
        let buffer = [0xa5u8; 64 * 1024];
        let started = Instant::now();
        // A full NTFS directory index does not prove that free MFT records are
        // exhausted: a small CURRENT file may still be resident. Reserve a
        // fixed set of empty, fixture-owned directory indexes before filling
        // data, then consume their spare records without touching engine files.
        #[cfg(windows)]
        if directory.is_some() {
            for index in 0..16 {
                let path = volume.root.join(format!("{}-m{index:02}", volume.nonce));
                std::fs::create_dir(&path).unwrap();
                filler.metadata_directories.push(path);
            }
        }
        // Ext4 directory blocks may have filename-dependent spare slots. Exhaust
        // inodes while data space remains, then fill data through the already
        // open filler. A free-inode observation of zero proves create denial
        // independent of the selected publication filename.
        #[cfg(target_os = "linux")]
        if let Some(directory) = &directory {
            filler.fill_metadata(volume, directory, started);
            use std::os::fd::AsRawFd;
            let handle = File::open(directory).unwrap();
            let mut state = std::mem::MaybeUninit::<libc::statvfs>::uninit();
            // SAFETY: valid owned directory fd and writable statvfs storage.
            assert_eq!(
                unsafe { libc::fstatvfs(handle.as_raw_fd(), state.as_mut_ptr()) },
                0
            );
            // SAFETY: successful fstatvfs initialized the value.
            let state = unsafe { state.assume_init() };
            assert_eq!(
                state.f_ffree, 0,
                "metadata error did not exhaust ext4 inodes"
            );
            filler.free_inodes = Some(state.f_ffree);
        }
        let mut write_size = buffer.len();
        let error = loop {
            assert!(
                started.elapsed() < Duration::from_secs(25),
                "bounded owned-volume fill"
            );
            assert!(
                filler.bytes < volume.max_bytes,
                "owned filesystem did not enforce its capacity"
            );
            match file.write(&buffer[..write_size]) {
                Ok(0) => panic!("zero-length filler write"),
                Ok(n) => filler.bytes += n as u64,
                Err(error) => {
                    let code = error.raw_os_error().expect("native capacity code");
                    assert!(capacity_code(code), "unexpected fill error: {error}");
                    if write_size == 1 {
                        break error;
                    }
                    // An all-or-nothing large write may leave smaller free
                    // allocations. Consume those too before declaring full.
                    write_size = (write_size / 16).max(1);
                    continue;
                }
            }
            // Force delayed allocation while the error still belongs to this
            // owned volume. A sync failure is retained as native evidence.
            if filler.bytes.is_multiple_of(4 * 1024 * 1024) {
                if let Err(error) = sync_filler_allocation(&file) {
                    break error;
                }
            }
        };
        filler.native_code = error.raw_os_error().expect("native capacity code");
        assert!(
            capacity_code(filler.native_code),
            "filler failed for another reason: {error}"
        );
        filler.file = Some(file);
        // Resident metadata on APFS/NTFS is exhausted after data allocation.
        if !cfg!(target_os = "linux") {
            if let Some(directory) = &directory {
                filler.fill_metadata(volume, directory, started);
            }
        }
        // Each empty index can consume otherwise reusable MFT records. A
        // failure in one directory must not end the global metadata fill.
        for directory in filler.metadata_directories.clone() {
            filler.fill_metadata(volume, &directory, started);
        }
        filler
    }
    fn fill_metadata(&mut self, volume: &Volume, directory: &std::path::Path, started: Instant) {
        for index in 0..8192u32 {
            assert!(self.metadata_paths.len() < 8192, "metadata entry bound");
            assert!(
                started.elapsed() < Duration::from_secs(25),
                "bounded metadata fill: files={} data_bytes={} elapsed_ms={}",
                self.metadata_paths.len(),
                self.bytes,
                started.elapsed().as_millis()
            );
            let path = directory.join(format!("c{}-{index:04x}", &volume.nonce[..8]));
            let created = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .and_then(|file| {
                    // Register before sync so an allocation failure still cleans
                    // up this exact owned entry. Drop ignores close errors.
                    self.metadata_paths.push(path);
                    if cfg!(target_os = "macos") {
                        sync_filler_allocation(&file)?;
                    }
                    drop(file);
                    Ok(())
                });
            match created {
                Ok(()) => {}
                Err(error) => {
                    let code = error.raw_os_error().expect("native metadata capacity code");
                    assert!(
                        capacity_code(code),
                        "metadata fill failed for another reason: {error}"
                    );
                    self.metadata_native_code = Some(code);
                    break;
                }
            }
        }
        assert!(
            self.metadata_native_code.is_some(),
            "metadata file count bound exhausted"
        );
    }
}

#[test]
fn filler_allocation_sync_preserves_regular_file_contents() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("allocation");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let payload = [0xa5; 8192];
    file.write_all(&payload).unwrap();
    sync_filler_allocation(&file).unwrap();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    sync_filler_host(&file).unwrap();
    drop(file);
    assert_eq!(std::fs::read(path).unwrap(), payload);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn filler_host_sync_preserves_native_noncapacity_errors() {
    // A live socket descriptor is valid but cannot synchronize file
    // allocation. Exercise the actual syscall, not an injected capacity error.
    let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor: std::os::fd::OwnedFd = socket.into();
    let file = File::from(descriptor);
    let error = sync_filler_host(&file).unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    assert!(!capacity_code(error.raw_os_error().unwrap()));
}

impl Drop for Filler {
    fn drop(&mut self) {
        self.file.take();
        for path in &self.metadata_paths {
            if let Err(error) = std::fs::remove_file(path) {
                if !std::thread::panicking() {
                    panic!("owned metadata filler cleanup failed: {error}");
                }
            }
        }
        for path in &self.metadata_directories {
            if let Err(error) = std::fs::remove_dir(path) {
                if !std::thread::panicking() {
                    panic!("owned empty metadata directory cleanup failed: {error}");
                }
            }
        }
        // Exact create_new-owned file only; never recurse or remove estate data.
        if let Err(error) = std::fs::remove_file(&self.path) {
            if !std::thread::panicking() {
                panic!("owned filler cleanup failed: {error}");
            }
        }
    }
}
fn capacity_code(code: i32) -> bool {
    if cfg!(windows) {
        matches!(code, 39 | 112)
    } else if cfg!(target_os = "macos") {
        matches!(code, 28 | 69)
    } else {
        matches!(code, 28 | 122)
    }
}

fn resume_after_fill(
    mut child: Child,
    control: Value,
    volume: Volume,
    metadata_directory: Option<PathBuf>,
) -> (Child, Filler, Value) {
    let line = child.line();
    let hit: Value = serde_json::from_str(
        std::str::from_utf8(&line)
            .unwrap()
            .trim()
            .strip_prefix(PREFIX)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hit["pid"], child.child.id());
    for key in ["nonce", "id", "point"] {
        assert_eq!(hit[key], control[key]);
    }
    println!("native_capacity_pause={hit}");
    let filler = Filler::fill(&volume, metadata_directory.as_deref());
    println!(
        "native_capacity_fill={}",
        json!({"point":control["point"],
        "bytes":filler.bytes,"native_code":filler.native_code,
        "metadata_files":filler.metadata_paths.len(),
        "metadata_native_code":filler.metadata_native_code,"free_inodes":filler.free_inodes})
    );
    child
        .child
        .resume_controlled(
            &serde_json::to_vec(&json!({"version":1,
        "nonce":control["nonce"],"id":control["id"]}))
            .unwrap(),
        )
        .unwrap();
    let line = match child.lines.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(line)) => line,
        failure => {
            child.child.terminate();
            let output = child.child.finish().expect("reap failed capacity engine");
            let tail = |bytes: &[u8]| {
                String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(2048)..]).into_owned()
            };
            panic!("selected native capacity event missing: {failure:?}; exit={}; stdout_tail={}; stderr_tail={}",
                output.status, tail(&output.stdout), tail(&output.stderr));
        }
    };
    let error: Value = serde_json::from_str(
        std::str::from_utf8(&line)
            .unwrap()
            .trim()
            .strip_prefix("C07F_NATIVE_IO_V1=")
            .expect("selected native I/O must actually fail"),
    )
    .unwrap();
    for key in ["pid", "nonce", "id", "point"] {
        assert_eq!(error[key], hit[key]);
    }
    assert!(
        capacity_code(error["native_code"].as_i64().unwrap() as i32),
        "wrong native error: {error}"
    );
    let expected_role = match control["point"].as_str().unwrap() {
        "wal_before_write" => "wal",
        "segment_before_create" => "segment",
        "manifest_before_create" => "manifest",
        _ => "publication",
    };
    assert_eq!(error["role"], expected_role);
    (child, filler, json!({"hit":hit,"error":error}))
}

async fn read_values(
    client: &RrdClient,
    session: &rrd_client::Session,
    at: u64,
) -> BTreeMap<String, String> {
    // Enumerate the complete identity set, then fetch bounded object windows at
    // that exact read cursor. Native capacity payloads exceed a single public
    // response; raising the contract's response cap would hide the boundary.
    let query = |text| ExecuteQuery {
        scope: format!("instance:{}", client.instance_id()),
        query: text,
        parameters: BTreeMap::new(),
        budget: QueryBudget {
            max_storage_keys: 100_000,
            max_rows: 1024,
            max_output_bytes: 512 * 1024,
            max_batch_rows: 64,
            max_memory_bytes: 64 * 1024 * 1024,
            max_spill_bytes: 64 * 1024 * 1024,
            max_elapsed_ms: 30_000,
        },
    };
    let index = client
        .execute_query(
            session,
            query(format!(
                "FROM claim:crash AT VALID {at} KNOWN HEAD PROJECT subject EXPLAIN CONTRACT"
            )),
            RequestOptions::read("capacity-index", "capacity-index").unwrap(),
        )
        .await
        .unwrap();
    assert!(!index.execution.truncated);
    let cursor = index.known_at_cursor;
    let mut subjects = index
        .rows
        .iter()
        .map(|row| {
            let QueryValue::String(subject) = &row.values["subject"] else {
                panic!("non-string subject")
            };
            subject.clone()
        })
        .collect::<Vec<_>>();
    subjects.sort();
    subjects.dedup();
    assert_eq!(subjects.len(), index.rows.len(), "duplicate claim subject");
    let mut values = BTreeMap::new();
    for window in subjects.chunks(64) {
        let first = serde_json::to_string(window.first().unwrap()).unwrap();
        let last = serde_json::to_string(window.last().unwrap()).unwrap();
        let result = client.execute_query(session, query(format!(
            "FROM claim:crash AT VALID {at} KNOWN {cursor} WHERE subject >= {first} AND subject <= {last} PROJECT subject, object EXPLAIN CONTRACT")),
            RequestOptions::read("capacity-window","capacity-window").unwrap()).await.unwrap();
        assert!(!result.execution.truncated);
        assert_eq!(result.known_at_cursor, cursor);
        assert_eq!(result.rows.len(), window.len());
        for row in result.rows {
            let QueryValue::String(value) = &row.values["object"] else {
                panic!("non-string claim")
            };
            assert!(values.insert(row.identity, value.clone()).is_none());
        }
    }
    assert_eq!(cursor, values.len() as u64 + 2);
    values
}

fn requested_values(request: &CommitTransaction) -> BTreeMap<String, String> {
    request
        .mutations
        .iter()
        .map(|m| {
            let TransactionMutation::AssertClaim {
                subject,
                predicate,
                object,
                ..
            } = m
            else {
                panic!("claim corpus")
            };
            (format!("claim:{subject}:{predicate}"), object.clone())
        })
        .collect()
}

async fn begin(
    client: &RrdClient,
    session: &rrd_client::Session,
    name: &str,
) -> rrd_contract::TransactionLease {
    client
        .begin_transaction(
            session,
            BeginTransaction {
                scope: CanonicalId::new("claims").unwrap(),
                timeout_ms: 600_000,
            },
            options(&format!("{name}-begin")),
        )
        .await
        .unwrap()
}

fn wal_bytes(estate: &Estate) -> u64 {
    let locator = RrdEngine::read_project_locator(&estate.project).unwrap();
    let root = estate.project.join(locator.storage_root).join("wal");
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .max()
        .unwrap()
}

fn capacity_case(point: &str, mode: &str) {
    let volume = Volume::admitted();
    let estate = Estate::install_at(mode, Some(&volume.root), point != "wal_before_write");
    let locator = RrdEngine::read_project_locator(&estate.project).unwrap();
    let native_root = estate.project.join(locator.storage_root);
    let metadata_directory = match point {
        "wal_before_write" => None,
        "segment_before_create" => Some(native_root.join("segments")),
        "manifest_before_create" => Some(native_root.join("manifests")),
        "current_before_create" => Some(native_root),
        _ => panic!("unsupported native capacity point"),
    };
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let mut ledger = AcknowledgementLedger::default();
    let clock = Instant::now();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let (child, client) = estate.serve(&Value::Null);
        let session = client.create_session(credential.principal_id.clone(),credential.credential(),
            CreateSession {limits:SessionLimits {idle_timeout_ms:600_000,absolute_timeout_ms:3_600_000,max_open_transactions:2}},
            options("capacity-session")).await.unwrap();
        let mut values = BTreeMap::new();
        let mut acknowledged = Vec::new();
        // Consume the native WAL's initial preallocation with real acknowledged
        // operations. No direct file growth or sparse-file simulation is allowed.
        for index in 0..256 {
            let name = format!("prior-{index}");
            let mut request = request(&name, estate.at);
            for mutation in &mut request.mutations {
                if let TransactionMutation::AssertClaim {object,..} = mutation { *object = format!("{name}:").repeat(400); object.truncate(4096); }
            }
            request.operation_sha256 = transaction_operation_sha256(&request.mutations);
            let expected = requested_values(&request);
            let lease = begin(&client,&session,&name).await;
            let started = clock.elapsed().as_nanos() as u64;
            let receipt = client.commit_transaction(&session,&lease.transaction_id,request.clone(),options(&name)).await.unwrap();
            record(&mut ledger, &name, &request, &expected, &receipt, started, &clock);
            values.extend(expected);
            acknowledged.push((name,request,lease,receipt_hash(&receipt)));
            if point != "wal_before_write" || wal_bytes(&estate) > 2 * 1024 * 1024 { break; }
            assert!(index < 255,"failed to consume initial WAL allocation within corpus bound");
        }
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let mutations = (0..180).map(|index| TransactionMutation::AssertClaim {
            subject:CanonicalId::new(format!("crash-uncertain-{index}")).unwrap(),
            predicate:CanonicalId::new("crash").unwrap(), object:format!("{index:04}").repeat(1024),
            valid_from:estate.at,tx_time:estate.at,producer:CanonicalId::new("independent-oracle").unwrap(),confidence:None,
        }).collect::<Vec<_>>();
        let target = CommitTransaction {operation_sha256:transaction_operation_sha256(&mutations),mutations};
        assert!(serde_json::to_vec(&target).unwrap().len() < 1024 * 1024);
        let target_values = requested_values(&target);
        ledger.expected.push(ExpectedOperation {logical_id:"uncertain".to_owned().try_into().unwrap(),
            request_sha256:digest("c07f.request",&target).unwrap(),effect_sha256:digest("c07f.effect",&target_values).unwrap(),
            resolution:ExpectedResolution::Present});
        let mut control = estate.control(point,1,Some(&target.operation_sha256));
        control["resume"] = json!(true);
        let (child,client) = estate.serve(&control);
        let lease = begin(&client,&session,"uncertain").await;
        let started = clock.elapsed().as_nanos() as u64;
        let controller = tokio::task::spawn_blocking(move ||resume_after_fill(child,control,volume,metadata_directory));
        let result = client.commit_transaction(&session,&lease.transaction_id,target.clone(),options("uncertain")).await;
        let result_class = match &result {
            Ok(_) => "acknowledged".to_owned(),
            Err(rrd_client::Error::Api { error, .. }) => format!("api-{:?}",error.code),
            Err(rrd_client::Error::Timeout) => "client-timeout".to_owned(),
            Err(_) => "client-failure".to_owned(),
        };
        println!("native_capacity_client={}",json!({"point":point,"mode":mode,"class":result_class}));
        let (child,filler,native) = controller.await.unwrap();
        assert!(result.is_err(),"capacity operation unexpectedly acknowledged");
        child.kill();
        let fill = json!({"bytes":filler.bytes,"native_code":filler.native_code,
            "metadata_files":filler.metadata_paths.len(),"metadata_native_code":filler.metadata_native_code,"free_inodes":filler.free_inodes});
        drop(filler);
        ledger.attempts.push(Attempt {attempt_id:"capacity-uncertain".to_owned().try_into().unwrap(),
            logical_id:"uncertain".to_owned().try_into().unwrap(),request_sha256:digest("c07f.request",&target).unwrap(),
            started_ns:started,finished_ns:clock.elapsed().as_nanos() as u64,
            outcome:AttemptOutcome::Uncertain {reason:"native-capacity-error".to_owned().try_into().unwrap()}});
        // Ordinary verify cannot silently repair. Only a positively classified
        // tail yields a plan; any other failure remains fatal on serve/re-read.
        let mut command = estate.command();
        command.args(["repair","plan","--project"]).arg(&estate.project);
        let output = process::Process::start(&mut command,Duration::from_secs(120),8*LIMIT,None).unwrap().finish().unwrap();
        let repair_receipt = if output.status.success() {
            let plan:Value = serde_json::from_slice(&output.stdout).unwrap();
            let path = estate.temp.path().join("repair.json");
            std::fs::write(&path,&output.stdout).unwrap();
            Some(estate.invoke(&["repair","apply","--project",estate.project.to_str().unwrap(),
                "--plan",path.to_str().unwrap(),"--expect",plan["plan_sha256"].as_str().unwrap()]))
        } else { None };
        let (child,client) = estate.serve(&Value::Null);
        let after = read_values(&client,&session,estate.at).await;
        let mut with_target = values.clone(); with_target.extend(target_values.clone());
        assert!(after == values || after == with_target,"prior ACK lost or non-atomic target after capacity recovery");
        // Resolve the unknown attempt via the original request identity, then
        // prove receipt stability for every prior acknowledged logical call.
        let retry_started = clock.elapsed().as_nanos() as u64;
        let receipt = client.commit_transaction(&session,&lease.transaction_id,target.clone(),options("uncertain")).await.unwrap();
        record(&mut ledger,"uncertain",&target,&target_values,&receipt,retry_started,&clock);
        values = with_target;
        for (name,request,lease,hash) in acknowledged {
            let receipt = client.commit_transaction(&session,&lease.transaction_id,request,options(&name)).await.unwrap();
            assert!(receipt.idempotent_replay); assert_eq!(receipt_hash(&receipt),hash);
        }
        let last = request("after",estate.at);
        let lease = begin(&client,&session,"after").await;
        let started = clock.elapsed().as_nanos() as u64;
        let receipt = client.commit_transaction(&session,&lease.transaction_id,last.clone(),options("after")).await.unwrap();
        record(&mut ledger,"after",&last,&expected("after"),&receipt,started,&clock);
        values.extend(expected("after"));
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let (child,client) = estate.serve(&Value::Null);
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let report = ledger.reconcile(1024, 1_800_000_000_000).unwrap();
        assert!(report.violations.is_empty());
        println!("\nnative_capacity_receipt={}",json!({"version":1,"mode":mode,"point":point,
            "build":estate.build,"native":native,"filler":fill,"repair":repair_receipt,
            "expected_sha256":digest("c07f.values",&values).unwrap(),"ledger":ledger,"report":report}));
    });
}

fn record(
    ledger: &mut AcknowledgementLedger,
    name: &str,
    request: &CommitTransaction,
    values: &BTreeMap<String, String>,
    receipt: &CommitReceipt,
    started: u64,
    clock: &Instant,
) {
    let logical_id = name.to_owned().try_into().unwrap();
    let request_sha256 = digest("c07f.request", request).unwrap();
    let effect_sha256 = digest("c07f.effect", values).unwrap();
    if !ledger
        .expected
        .iter()
        .any(|item| item.logical_id == logical_id)
    {
        ledger.expected.push(ExpectedOperation {
            logical_id,
            request_sha256: request_sha256.clone(),
            effect_sha256: effect_sha256.clone(),
            resolution: ExpectedResolution::Present,
        });
    }
    ledger.attempts.push(Attempt {
        attempt_id: format!("ack-{name}").try_into().unwrap(),
        logical_id: name.to_owned().try_into().unwrap(),
        request_sha256: request_sha256.clone(),
        started_ns: started,
        finished_ns: clock.elapsed().as_nanos() as u64,
        outcome: AttemptOutcome::Acknowledged {
            receipt_sha256: receipt_hash(receipt),
        },
    });
    ledger.readback.push(Readback::Present {
        logical_id: name.to_owned().try_into().unwrap(),
        request_sha256,
        effect_sha256,
        receipt_sha256: receipt_hash(receipt),
        applications: 1.try_into().unwrap(),
    });
}

#[test]
#[ignore = "requires runner-owned ext4/APFS/NTFS image; never fill a developer volume"]
fn installed_native_capacity_recovers_acknowledged_history() {
    let point = std::env::var("RRFLOW_CAPACITY_POINT").unwrap();
    assert!([
        "wal_before_write",
        "segment_before_create",
        "manifest_before_create",
        "current_before_create"
    ]
    .contains(&point.as_str()));
    let mode = std::env::var("RRFLOW_CAPACITY_MODE").unwrap();
    assert!(matches!(mode.as_str(), "fresh" | "existing"));
    capacity_case(&point, &mode);
}

/// Characterize the actual installed domain's first WAL append without filling
/// a filesystem. Native exhaustion remains the separate owned-image oracle.
#[test]
fn installed_capacity_wal_payload_is_observed_before_domain_commit() {
    let estate = Estate::install("existing");
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mutations = (0..180)
                .map(|index| TransactionMutation::AssertClaim {
                    subject: CanonicalId::new(format!("crash-uncertain-{index}")).unwrap(),
                    predicate: CanonicalId::new("crash").unwrap(),
                    object: format!("{index:04}").repeat(1024),
                    valid_from: estate.at,
                    tx_time: estate.at,
                    producer: CanonicalId::new("independent-oracle").unwrap(),
                    confidence: None,
                })
                .collect::<Vec<_>>();
            let target = CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            };
            let mut control = estate.control("wal_before_write", 1, Some(&target.operation_sha256));
            control["resume"] = json!(true);
            let (mut child, client) = estate.serve(&control);
            let session = client
                .create_session(
                    credential.principal_id.clone(),
                    credential.credential(),
                    CreateSession {
                        limits: SessionLimits {
                            idle_timeout_ms: 600_000,
                            absolute_timeout_ms: 3_600_000,
                            max_open_transactions: 2,
                        },
                    },
                    options("capacity-session"),
                )
                .await
                .unwrap();
            let lease = begin(&client, &session, "selection").await;
            let controller = tokio::task::spawn_blocking(move || {
                let line = child.line();
                let hit: Value = serde_json::from_str(
                    std::str::from_utf8(&line)
                        .unwrap()
                        .trim()
                        .strip_prefix(PREFIX)
                        .unwrap(),
                )
                .unwrap();
                for key in ["nonce", "id", "point"] {
                    assert_eq!(hit[key], control[key]);
                }
                assert_eq!(hit["pid"], child.child.id());
                println!("installed_capacity_wal_selection={hit}");
                child
                    .child
                    .resume_controlled(
                        &serde_json::to_vec(
                            &json!({"version":1,"nonce":hit["nonce"],"id":hit["id"]}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                (child, hit)
            });
            let receipt = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    target.clone(),
                    options("selection"),
                )
                .await
                .unwrap();
            let (child, hit) = controller.await.unwrap();
            assert!(hit["wal_payload_bytes"].as_u64().unwrap() > 0);
            let replay = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    target.clone(),
                    options("selection"),
                )
                .await
                .unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(receipt_hash(&receipt), receipt_hash(&replay));
            assert_eq!(
                read_values(&client, &session, estate.at).await,
                requested_values(&target)
            );
            child.kill();
            let (child, client) = estate.serve(&Value::Null);
            assert_eq!(
                read_values(&client, &session, estate.at).await,
                requested_values(&target)
            );
            child.kill();
        });
}
