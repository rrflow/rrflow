//! Native read-only sampling. Address-space mappings, committed private bytes,
//! and resident bytes are distinct quantities; unavailable data is explicit.
use rrflow_eval::storage::NativeMetric;
#[path = "native_allocation.rs"]
mod allocation;
pub(super) use allocation::{enforcement, receipt};

fn missing(reason: &str) -> NativeMetric {
    NativeMetric::Unavailable {
        reason: reason.to_owned().try_into().unwrap(),
    }
}

#[cfg(target_os = "linux")]
pub(super) fn memory(pid: u32) -> (NativeMetric, NativeMetric) {
    use std::io::Read;
    fn read(path: &str) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .ok()?
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        (bytes.len() <= 4 * 1024 * 1024).then_some(bytes)
    }
    let private = (|| {
        let data = read(&format!("/proc/{pid}/smaps_rollup"))?;
        let text = std::str::from_utf8(&data).ok()?;
        let mut sum = 0u64;
        let mut seen = 0;
        for line in text.lines() {
            let mut fields = line.split_ascii_whitespace();
            if matches!(
                fields.next(),
                Some("Private_Clean:" | "Private_Dirty:" | "Private_Hugetlb:")
            ) {
                let bytes = fields.next()?.parse::<u64>().ok()?.checked_mul(1024)?;
                if fields.next() != Some("kB") {
                    return None;
                }
                sum = sum.checked_add(bytes)?;
                seen += 1;
            }
        }
        (seen >= 2).then_some(sum)
    })();
    let mapped = (|| {
        let data = read(&format!("/proc/{pid}/maps"))?;
        let mut sum = 0u64;
        for line in data.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
            let mut fields = line
                .split(|b| b.is_ascii_whitespace())
                .filter(|f| !f.is_empty());
            let range = std::str::from_utf8(fields.next()?).ok()?;
            fields.next()?;
            fields.next()?;
            fields.next()?;
            let inode = std::str::from_utf8(fields.next()?)
                .ok()?
                .parse::<u64>()
                .ok()?;
            if inode != 0 {
                let (first, last) = range.split_once('-')?;
                let length = u64::from_str_radix(last, 16)
                    .ok()?
                    .checked_sub(u64::from_str_radix(first, 16).ok()?)?;
                sum = sum.checked_add(length)?;
            }
        }
        Some(sum)
    })();
    (
        private.map_or_else(
            || missing("smaps-private-unavailable"),
            |value| NativeMetric::Observed { value },
        ),
        mapped.map_or_else(
            || missing("file-mappings-unavailable"),
            |value| NativeMetric::Observed { value },
        ),
    )
}

#[cfg(windows)]
pub(super) fn memory(pid: u32) -> (NativeMetric, NativeMetric) {
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
    };
    #[repr(C)]
    struct Counters {
        cb: u32,
        faults: u32,
        sizes: [usize; 9],
    }
    #[repr(C)]
    struct Region {
        base: usize,
        allocation: usize,
        allocation_protect: u32,
        partition: u16,
        size: usize,
        state: u32,
        protect: u32,
        kind: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetLastError() -> u32;
        fn VirtualQueryEx(
            process: *mut c_void,
            address: *const c_void,
            buffer: *mut Region,
            length: usize,
        ) -> usize;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(process: *mut c_void, counters: *mut Counters, size: u32) -> i32;
    }
    struct Owned(*mut c_void);
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    // Only the harness-owned PID is read. No process write capability is opened.
    let handle = unsafe { OpenProcess(0x410, 0, pid) };
    if handle.is_null() {
        return (
            missing("native-process-unavailable"),
            missing("native-process-unavailable"),
        );
    }
    let process = Owned(handle);
    let mut counters: Counters = unsafe { zeroed() };
    counters.cb = size_of::<Counters>() as u32;
    let private =
        if unsafe { GetProcessMemoryInfo(process.0, &mut counters, size_of::<Counters>() as u32) }
            != 0
        {
            NativeMetric::Observed {
                value: counters.sizes[8] as u64,
            }
        } else {
            missing("private-commit-unavailable")
        };
    let mut address = 0usize;
    let mut mapped = 0u64;
    let mut ended = false;
    for _ in 0..262_144 {
        let mut region: Region = unsafe { zeroed() };
        let bytes = unsafe {
            VirtualQueryEx(
                process.0,
                address as *const c_void,
                &mut region,
                size_of::<Region>(),
            )
        };
        if bytes == 0 {
            ended = address > 0 && unsafe { GetLastError() } == 87;
            break;
        }
        if bytes != size_of::<Region>() {
            break;
        }
        if matches!(region.kind, 0x40000 | 0x1000000) && region.state == 0x1000 {
            let Some(next) = mapped.checked_add(region.size as u64) else {
                break;
            };
            mapped = next;
        }
        let Some(next) = region
            .base
            .checked_add(region.size)
            .filter(|next| *next > address)
        else {
            break;
        };
        address = next;
    }
    (
        private,
        if ended {
            NativeMetric::Observed { value: mapped }
        } else {
            missing("mapping-walk-incomplete")
        },
    )
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(super) fn memory(_pid: u32) -> (NativeMetric, NativeMetric) {
    (
        missing("native-private-sampler-unavailable"),
        missing("native-mapped-sampler-unavailable"),
    )
}
