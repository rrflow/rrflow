//! Handle-bound available-space admission. Observation is racy; all
//! subsequent native write/sync errors retain their existing failure semantics.
use std::{fs::File, io};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Space {
    pub available: u64,
    pub unit: u64,
}

impl Space {
    fn admit(self, reserve: u64, bytes: u64, new_file: bool) -> crate::Result<()> {
        if self.unit == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "filesystem allocation unit unavailable",
            )
            .into());
        }
        let units = bytes
            .div_ceil(self.unit)
            .checked_add(if new_file { 4 } else { 1 });
        let required = units
            .and_then(|n| n.checked_mul(self.unit))
            .and_then(|n| n.checked_add(reserve))
            .ok_or_else(|| crate::Error::InvalidConfiguration("disk admission overflow".into()))?;
        if self.available < required {
            return Err(crate::Error::DiskAdmission {
                available: self.available,
                required,
                reserve,
            });
        }
        Ok(())
    }
}

pub(crate) fn admit(file: &File, reserve: u64, bytes: u64, new_file: bool) -> crate::Result<()> {
    // Unconfigured raw file utilities have no independent storage policy.
    // Database create/open always bind the positive DatabaseOptions reserve.
    if reserve == 0 {
        return Ok(());
    }
    available(file)?.admit(reserve, bytes, new_file)
}

#[cfg(unix)]
fn available(file: &File) -> io::Result<Space> {
    use std::os::fd::AsRawFd;
    let mut info: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatvfs(file.as_raw_fd(), &mut info) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let unit = u64::try_from(u128::from(info.f_frsize)).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "native allocation unit overflow",
        )
    })?;
    let available = u128::from(info.f_bavail)
        .checked_mul(u128::from(unit))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "native available space overflow",
            )
        })?;
    Ok(Space { available, unit })
}

#[cfg(windows)]
fn available(file: &File) -> io::Result<Space> {
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
        os::windows::io::AsRawHandle,
    };
    #[repr(C)]
    struct Status {
        status_or_pointer: usize,
        information: usize,
    }
    #[repr(C, align(8))]
    struct FullSize {
        total: i64,
        caller_available: i64,
        actual_available: i64,
        sectors: u32,
        bytes: u32,
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryVolumeInformationFile(
            handle: *mut c_void,
            status: *mut Status,
            info: *mut c_void,
            length: u32,
            class: u32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }
    let mut info: FullSize = unsafe { zeroed() };
    let mut status: Status = unsafe { zeroed() };
    // FileFsFullSizeInformation = 7, queried on an existing synchronous
    // directory/file handle; repository filesystem opens do not use overlapped I/O.
    let result = unsafe {
        NtQueryVolumeInformationFile(
            file.as_raw_handle(),
            &mut status,
            (&mut info as *mut FullSize).cast(),
            size_of::<FullSize>() as u32,
            7,
        )
    };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(result) } as i32,
        ));
    }
    if status.information != size_of::<FullSize>() || info.caller_available < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "incomplete native volume space",
        ));
    }
    let unit = u64::from(info.sectors) * u64::from(info.bytes);
    let available = (info.caller_available as u64)
        .checked_mul(unit)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "native available space overflow",
            )
        })?;
    Ok(Space { available, unit })
}

#[cfg(not(any(unix, windows)))]
fn available(_file: &File) -> io::Result<Space> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native disk admission unavailable",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserve_accounts_for_rounding_temporary_bytes_and_metadata() {
        let space = Space {
            available: 4096 * 10,
            unit: 4096,
        };
        space.admit(4096 * 4, 4097, true).unwrap();
        assert!(matches!(
            space.admit(4096 * 4 + 1, 4097, true),
            Err(crate::Error::DiskAdmission { .. })
        ));
        assert!(space.admit(u64::MAX, 1, false).is_err());
        assert!(Space {
            available: 100,
            unit: 0
        }
        .admit(1, 1, false)
        .is_err());
    }
    #[test]
    fn retained_handle_observes_native_capacity_without_allocating_disk() {
        let directory = tempfile::tempdir().unwrap();
        let file = tempfile::tempfile_in(directory.path()).unwrap();
        let space = available(&file).unwrap();
        assert!(space.unit > 0 && space.available > 0);
        assert!(matches!(
            space.admit(space.available, 1, false),
            Err(crate::Error::DiskAdmission { .. })
        ));
    }
}
