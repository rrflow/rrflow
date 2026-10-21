//! Win32 handles own both file identity and security inspection. No shell ACL
//! utility, inherited default ACL, or post-write permission repair is used.
use super::{denied, File, Metadata, Path};
use std::ffi::c_void;
use std::io;
use std::mem::{offset_of, size_of};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    LocalFree, ERROR_INSUFFICIENT_BUFFER, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_FILE_OBJECT,
};
use windows_sys::Win32::Security::{
    EqualSid, GetAce, GetLengthSid, GetSecurityDescriptorControl, GetTokenInformation, IsValidAcl,
    IsValidSecurityDescriptor, IsValidSid, IsWellKnownSid, TokenUser, WinLocalSystemSid,
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
    PSID, SECURITY_ATTRIBUTES, SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, CreateFileW, GetFileInformationByHandle, GetFileType,
    BY_HANDLE_FILE_INFORMATION, CREATE_NEW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_DISK,
    OPEN_EXISTING, READ_CONTROL, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: only successful Win32 LocalAlloc-family results enter this owner.
        unsafe { LocalFree(self.0) };
    }
}

struct CurrentUser {
    // Pointer-sized elements give TOKEN_USER its required alignment. The SID
    // pointer returned by GetTokenInformation points into this retained buffer.
    storage: Vec<usize>,
}

impl CurrentUser {
    fn query() -> io::Result<Self> {
        let mut raw = null_mut();
        // SAFETY: a valid pseudo process handle and a writable output handle.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful OpenProcessToken transfers one owned real handle.
        let token = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut length = 0;
        // SAFETY: zero-length query requests only the required allocation size.
        let first = unsafe {
            GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut length)
        };
        if first != 0
            || io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
            || !(size_of::<TOKEN_USER>() as u32..=64 * 1024).contains(&length)
        {
            return Err(denied("cannot bound the current Windows user identity"));
        }
        let mut storage = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        // SAFETY: aligned storage has at least length writable bytes; the token
        // remains open and the output-length pointer is valid.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                storage.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let result = Self { storage };
        // SAFETY: GetTokenInformation populated a complete TOKEN_USER and SID.
        if unsafe { IsValidSid(result.sid()) } == 0 {
            return Err(denied("invalid Windows user identity"));
        }
        Ok(result)
    }

    fn sid(&self) -> PSID {
        // SAFETY: query populated an aligned TOKEN_USER; storage never resizes.
        unsafe { (*self.storage.as_ptr().cast::<TOKEN_USER>()).User.Sid }
    }

    fn text(&self) -> io::Result<String> {
        let mut raw = null_mut();
        // SAFETY: sid is valid while self is alive; raw receives an allocation.
        if unsafe { ConvertSidToStringSidW(self.sid(), &mut raw) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let allocation = LocalAllocation(raw.cast());
        let mut length = 0;
        // SAFETY: ConvertSidToStringSidW returns a NUL-terminated UTF-16 SID.
        while unsafe { *raw.add(length) } != 0 {
            length += 1;
        }
        // SAFETY: the preceding scan covers exactly the allocated string.
        let value = String::from_utf16(unsafe { std::slice::from_raw_parts(raw, length) })
            .map_err(|_| denied("invalid Windows user identity encoding"))?;
        drop(allocation);
        Ok(value)
    }
}

fn descriptor(sddl: &str) -> io::Result<LocalAllocation> {
    let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut raw = null_mut();
    // SAFETY: wide is NUL-terminated and live; raw receives a self-relative SD.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide.as_ptr(),
            SDDL_REVISION_1,
            &mut raw,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(LocalAllocation(raw))
}

pub(super) fn open_directory(path: &Path, create: bool) -> io::Result<File> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory path contains NUL",
        ));
    }
    wide.push(0);
    if create {
        let user = CurrentUser::query()?.text()?;
        // Descendants inherit only owner/System grants. Each of the four estate
        // directories gets its own protected descriptor rather than relying on
        // the project parent's ACL. Store-owned descendants retain their owner.
        let security = descriptor(&format!("O:{user}D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)"))?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.0,
            bInheritHandle: 0,
        };
        // SAFETY: path and descriptor remain live; creation never replaces an existing object.
        if unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: live terminated path; OPEN_REPARSE_POINT inspects the leaf itself.
    // These short-lived metadata handles do not claim ancestor exclusion.
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            READ_CONTROL | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS
                | FILE_FLAG_OPEN_REPARSE_POINT
                | SECURITY_SQOS_PRESENT
                | SECURITY_IDENTIFICATION,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful CreateFileW transfers one real handle to File.
    Ok(unsafe { File::from_raw_handle(raw) })
}

pub(super) fn validate_directory(file: &File) -> io::Result<()> {
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: file owns the live handle and information is writable.
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
        || unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(denied(
            "estate directory must be a native non-reparse directory",
        ));
    }
    validate_security(file, true)
}

pub(super) fn open(path: &Path, create: bool) -> io::Result<File> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "secret path contains NUL",
        ));
    }
    let wide: Vec<u16> = wide.into_iter().chain(Some(0)).collect();
    let security = if create {
        let user = CurrentUser::query()?.text()?;
        Some(descriptor(&format!(
            "O:{user}D:P(A;;FA;;;{user})(A;;FA;;;SY)"
        ))?)
    } else {
        None
    };
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security.as_ref().map_or(null_mut(), |value| value.0),
        bInheritHandle: 0,
    };
    // SAFETY: all pointers are live for this call. CREATE_NEW cannot truncate
    // an existing file; the explicit protected ACL applies before any write.
    // Read handles deny sharing for write/delete and open the reparse object
    // itself. SQOS prevents a device/pipe from impersonating this process.
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | if create { GENERIC_WRITE } else { 0 },
            if create { 0 } else { FILE_SHARE_READ },
            if create { &attributes } else { null() },
            if create { CREATE_NEW } else { OPEN_EXISTING },
            FILE_ATTRIBUTE_NORMAL
                | FILE_FLAG_OPEN_REPARSE_POINT
                | SECURITY_SQOS_PRESENT
                | SECURITY_IDENTIFICATION,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful CreateFileW transfers ownership of this one handle.
    Ok(unsafe { File::from_raw_handle(raw) })
}

pub(super) fn validate(file: &File) -> io::Result<Metadata> {
    let handle = file.as_raw_handle();
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: file owns a live handle, information is writable and correctly sized.
    if unsafe { GetFileType(handle) } != FILE_TYPE_DISK
        || unsafe { GetFileInformationByHandle(handle, &mut information) } == 0
    {
        return Err(denied("secret must be a native disk file"));
    }
    if information.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        || information.nNumberOfLinks != 1
    {
        return Err(denied(
            "secret must be a regular non-reparse file with exactly one link",
        ));
    }
    validate_security(file, false)?;
    file.metadata()
}

fn validate_security(file: &File, directory: bool) -> io::Result<()> {
    let handle = file.as_raw_handle();
    let user = CurrentUser::query()?;
    let mut owner = null_mut();
    let mut dacl: *mut ACL = null_mut();
    let mut raw = null_mut();
    // SAFETY: the handle grants READ_CONTROL; outputs point into raw,
    // a LocalFree-owned security descriptor retained through all ACL inspection.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut raw,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let security = LocalAllocation(raw);
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: GetSecurityInfo returned this descriptor and interior owner/DACL.
    let valid = unsafe {
        IsValidSecurityDescriptor(security.0) != 0
            && !owner.is_null()
            && IsValidSid(owner) != 0
            && EqualSid(owner, user.sid()) != 0
            && GetSecurityDescriptorControl(security.0, &mut control, &mut revision) != 0
            && control & SE_DACL_PROTECTED != 0
            && !dacl.is_null()
            && IsValidAcl(dacl) != 0
    };
    if !valid {
        return Err(denied(
            "private object requires the current owner and a protected non-null DACL",
        ));
    }
    // SAFETY: IsValidAcl validated the header and security keeps it alive.
    let count = unsafe { (*dacl).AceCount };
    if count == 0 || count > 2 {
        return Err(denied(
            "private DACL must grant only its owner and optional LocalSystem",
        ));
    }
    let mut owner_grant = false;
    for index in 0..u32::from(count) {
        let mut ace = null_mut();
        // SAFETY: dacl is valid, index is in its count, ace is an output pointer.
        if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: GetAce returns at least a valid ACE_HEADER in the live ACL.
        let header = unsafe { ace.cast::<ACE_HEADER>().read_unaligned() };
        let sid_offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        // ACCESS_ALLOWED_ACE_TYPE is the Win32 basic allow ACE (0). Object,
        // conditional, inherited and unknown ACE forms are not this contract.
        if header.AceType != 0
            || header.AceFlags != if directory { 3 } else { 0 }
            || usize::from(header.AceSize) < sid_offset + 8
        {
            return Err(denied("unsupported private DACL entry"));
        }
        // SAFETY: the checked ACE size contains a SID header at the fixed offset.
        let sid = unsafe { ace.cast::<u8>().add(sid_offset).cast() };
        // SAFETY: GetLengthSid reads that SID header; bounds are checked before
        // subsequent SID comparisons can consume the subauthority array.
        if unsafe { GetLengthSid(sid) } as usize > usize::from(header.AceSize) - sid_offset
            || unsafe { IsValidSid(sid) } == 0
        {
            return Err(denied("invalid private DACL identity"));
        }
        // SAFETY: both SIDs are valid and their owners remain alive.
        let is_owner = unsafe { EqualSid(sid, user.sid()) } != 0;
        if !is_owner && unsafe { IsWellKnownSid(sid, WinLocalSystemSid) } == 0 {
            return Err(denied("private DACL grants another principal access"));
        }
        owner_grant |= is_owner;
    }
    if !owner_grant {
        return Err(denied("private DACL lacks its owner grant"));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn make_directory_permissive_for_test(path: &Path) {
    let user = CurrentUser::query().unwrap().text().unwrap();
    tests::replace_dacl(
        path,
        &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;WD)"),
        true,
    );
}

#[cfg(test)]
pub(super) fn restore_private_directory_for_test(path: &Path) {
    let user = CurrentUser::query().unwrap().text().unwrap();
    tests::replace_dacl(
        path,
        &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)"),
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::super::{read_private_bounded, write_private_new};
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Security::Authorization::SetSecurityInfo;
    use windows_sys::Win32::Security::{
        GetSecurityDescriptorDacl, PROTECTED_DACL_SECURITY_INFORMATION,
        UNPROTECTED_DACL_SECURITY_INFORMATION,
    };
    use windows_sys::Win32::Storage::FileSystem::{READ_CONTROL, WRITE_DAC};

    pub(super) fn replace_dacl(path: &Path, text: &str, protected: bool) {
        let sd = descriptor(text).unwrap();
        let mut present = 0;
        let mut dacl = null_mut();
        let mut defaulted = 0;
        // SAFETY: sd owns the converted descriptor; all output pointers are live.
        assert_ne!(
            unsafe { GetSecurityDescriptorDacl(sd.0, &mut present, &mut dacl, &mut defaulted) },
            0
        );
        let file = std::fs::OpenOptions::new()
            .access_mode(READ_CONTROL | WRITE_DAC)
            .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .unwrap();
        let flag = if protected {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        // SAFETY: test-owned file grants WRITE_DAC; descriptor stays alive. Null
        // DACL deliberately simulates a permissive file and must be rejected.
        assert_eq!(
            unsafe {
                SetSecurityInfo(
                    file.as_raw_handle(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | flag,
                    null_mut(),
                    null_mut(),
                    dacl,
                    null(),
                )
            },
            0
        );
    }

    #[test]
    fn protected_creation_and_permissive_acl_denial_do_not_repair_files() {
        let root = tempfile::tempdir().unwrap();
        let user = CurrentUser::query().unwrap().text().unwrap();
        for (index, text, protected) in [
            (0, format!("D:P(A;;FA;;;{user})(A;;FR;;;WD)"), true),
            (1, format!("D:(A;;FA;;;{user})"), false),
            (2, "D:NO_ACCESS_CONTROL".into(), true),
        ] {
            let path = root.path().join(format!("secret-{index}"));
            write_private_new(&path, b"secret").unwrap();
            assert_eq!(read_private_bounded(&path, 6).unwrap(), b"secret");
            replace_dacl(&path, &text, protected);
            assert!(read_private_bounded(&path, 6).is_err());
            assert!(
                read_private_bounded(&path, 6).is_err(),
                "read must not silently repair ACLs"
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"secret");
        }
    }

    #[test]
    fn device_handles_are_not_secret_files() {
        assert!(read_private_bounded(Path::new("NUL"), 32).is_err());
    }

    #[test]
    fn directory_acl_denials_preserve_contents_and_do_not_repair() {
        let root = tempfile::tempdir().unwrap();
        let user = CurrentUser::query().unwrap().text().unwrap();
        for (index, text, protected) in [
            (0, format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;WD)"), true),
            (1, format!("D:(A;OICI;FA;;;{user})"), false),
            (2, "D:NO_ACCESS_CONTROL".into(), true),
            (3, format!("D:P(A;;FA;;;{user})"), true),
        ] {
            let path = root.path().join(format!("directory-{index}"));
            let directory = open_directory(&path, true).unwrap();
            validate_directory(&directory).unwrap();
            write_private_new(&path.join("retained"), b"unchanged").unwrap();
            replace_dacl(&path, &text, protected);
            assert!(validate_directory(&directory).is_err());
            assert!(validate_directory(&open_directory(&path, false).unwrap()).is_err());
            assert_eq!(std::fs::read(path.join("retained")).unwrap(), b"unchanged");
            restore_private_directory_for_test(&path);
            validate_directory(&directory).unwrap();
        }
    }
}
