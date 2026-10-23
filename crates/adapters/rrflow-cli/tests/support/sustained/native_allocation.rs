//! Read-only kernel verification of launcher-owned memory admission. An
//! environment string alone is never evidence that an engine belongs to it.
#[cfg(any(windows, target_os = "linux"))]
use rrflow_eval::storage::NativeAllocationKind;
use rrflow_eval::storage::{AllocationEnforcement, NativeAllocationReceipt, StorageCampaign};
use std::io::Read;

pub(crate) fn receipt(
    campaign: &StorageCampaign,
) -> Result<Option<NativeAllocationReceipt>, String> {
    let raw = match std::env::var("RRFLOW_SUSTAINED_ALLOCATION_RECEIPT") {
        Ok(raw) => raw,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(_) => return Err("native allocation receipt is not UTF-8".into()),
    };
    if raw.len() > 4096 {
        return Err("native allocation receipt exceeds admission".into());
    }
    let receipt: NativeAllocationReceipt = rrflow_eval::storage::decode(raw.as_bytes())?;
    receipt.validate(campaign)?;
    use sha2::{Digest as _, Sha256};
    let mut file = std::fs::File::open(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let admitted = file.metadata().map_err(|e| e.to_string())?.len();
    if admitted > 2 * 1024 * 1024 * 1024 {
        return Err("harness digest exceeds two-GiB input bound".into());
    }
    let mut read = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        read += count as u64;
        if read > admitted {
            return Err("harness changed during identity read".into());
        }
        hash.update(&buffer[..count]);
    }
    if read != admitted || file.metadata().map_err(|e| e.to_string())?.len() != admitted {
        return Err("harness extent changed during identity read".into());
    }
    let actual = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != receipt.harness_sha256.as_str() {
        return Err("allocation receipt names another harness".into());
    }
    Ok(Some(receipt))
}

pub(crate) fn enforcement(
    pid: u32,
    receipt: Option<&NativeAllocationReceipt>,
) -> Result<AllocationEnforcement, String> {
    let Some(receipt) = receipt else {
        return Ok(AllocationEnforcement::ObservedOnly);
    };
    if !verify(pid, receipt) {
        return Err("owned engine native allocation membership or limits changed".into());
    }
    receipt.enforcement()
}

#[cfg(target_os = "linux")]
fn verify(pid: u32, receipt: &NativeAllocationReceipt) -> bool {
    use std::path::{Path, PathBuf};
    fn read(path: impl AsRef<Path>) -> Option<String> {
        let mut raw = String::new();
        std::fs::File::open(path)
            .ok()?
            .take(65537)
            .read_to_string(&mut raw)
            .ok()?;
        (raw.len() <= 65536).then_some(raw)
    }
    let valid = (|| {
        if receipt.kind != NativeAllocationKind::CgroupV2 {
            return None;
        }
        let group = PathBuf::from(std::env::var_os("RRFLOW_SUSTAINED_CGROUP")?);
        if group.canonicalize().ok()? != group
            || group.file_name()?.to_str()? != format!("rrflow-capacity-{}", receipt.nonce.as_str())
        {
            return None;
        }
        let relative = group.strip_prefix("/sys/fs/cgroup").ok()?.to_str()?;
        if relative.is_empty() {
            return None;
        }
        let membership = format!("0::/{relative}");
        for process in [pid, std::process::id()] {
            if !read(format!("/proc/{process}/cgroup"))?
                .lines()
                .any(|line| line == membership)
            {
                return None;
            }
        }
        if read(group.join("memory.max"))?.trim() != receipt.engine_bytes.get().to_string()
            || read(group.join("memory.swap.max"))?.trim() != "0"
            || read(group.join("memory.oom.group"))?.trim() != "1"
            || read(group.join("pids.max"))?.trim() != "512"
            || read(group.join("cpu.max"))?.trim()
                != format!("{} 100000", receipt.vcpus.get() * 100000)
        {
            return None;
        }
        Some(())
    })();
    valid.is_some()
}

#[cfg(windows)]
fn verify(pid: u32, receipt: &NativeAllocationReceipt) -> bool {
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
        ptr::null_mut,
    };
    #[repr(C)]
    struct Basic {
        process_time: i64,
        job_time: i64,
        flags: u32,
        minimum_working_set: usize,
        maximum_working_set: usize,
        active_processes: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    struct Limits {
        basic: Basic,
        io: [u64; 6],
        process_memory: usize,
        job_memory: usize,
        peak_process_memory: usize,
        peak_job_memory: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenJobObjectW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn IsProcessInJob(process: *mut c_void, job: *mut c_void, result: *mut i32) -> i32;
        fn QueryInformationJobObject(
            job: *mut c_void,
            class: u32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetActiveProcessorCount(group: u16) -> u32;
    }
    struct Handle(*mut c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }
    if receipt.kind != NativeAllocationKind::WindowsJob {
        return false;
    }
    let name = format!("Local\\RRFlowCapacity-{}", receipt.nonce.as_str())
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        let job = Handle(OpenJobObjectW(0x0004, 0, name.as_ptr()));
        if job.0.is_null() {
            return false;
        }
        for id in [pid, std::process::id()] {
            let process = Handle(OpenProcess(0x0400, 0, id));
            let mut member = 0;
            if process.0.is_null()
                || IsProcessInJob(process.0, job.0, &mut member) == 0
                || member == 0
            {
                return false;
            }
        }
        let mut limits: Limits = zeroed();
        if QueryInformationJobObject(
            job.0,
            9,
            &mut limits as *mut _ as *mut c_void,
            size_of::<Limits>() as u32,
            null_mut(),
        ) == 0
        {
            return false;
        }
        let mut cpu = [0u32; 2];
        if u64::from(GetActiveProcessorCount(0xffff)) != receipt.host_logical_cpus.get()
            || QueryInformationJobObject(
                job.0,
                15,
                cpu.as_mut_ptr().cast(),
                size_of::<[u32; 2]>() as u32,
                null_mut(),
            ) == 0
            || cpu[0] != (0x1 | 0x4)
            || u64::from(cpu[1]) != 10000 * receipt.vcpus.get() / receipt.host_logical_cpus.get()
        {
            return false;
        }
        limits.basic.flags & (0x2000 | 0x100 | 0x200) == (0x2000 | 0x100 | 0x200)
            && limits.basic.flags & (0x800 | 0x1000) == 0
            && limits.process_memory as u64 == receipt.engine_bytes.get()
            && limits.job_memory as u64 == receipt.engine_bytes.get()
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn verify(_pid: u32, _receipt: &NativeAllocationReceipt) -> bool {
    false
}
