//! Win32 handles own both file identity and security inspection. No shell ACL
//! utility, inherited default ACL, or post-write permission repair is used.
#[cfg(test)]
use super::Path;

use super::{denied, File, Metadata};

use std::ffi::c_void;

use std::io;

use std::mem::{offset_of, size_of};

use std::os::windows::ffi::OsStrExt;

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    LocalFree, ERROR_INSUFFICIENT_BUFFER, GENERIC_READ, GENERIC_WRITE,
};

use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_FILE_OBJECT,
};

use windows_sys::Win32::Security::{
    EqualSid, GetAce, GetLengthSid, GetSecurityDescriptorControl, GetTokenInformation, IsValidAcl,
    IsValidSecurityDescriptor, IsValidSid, IsWellKnownSid, TokenOwner, TokenUser,
    WinLocalSystemSid, ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION,
    OWNER_SECURITY_INFORMATION, PSID, SE_DACL_PROTECTED, TOKEN_OWNER, TOKEN_QUERY, TOKEN_USER,
};

use windows_sys::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, GetFileType, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_TYPE_DISK, READ_CONTROL,
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

struct DefaultOwner {
    storage: Vec<usize>,
}

impl DefaultOwner {
    fn sid(&self) -> PSID {
        // SAFETY: query populated an aligned TOKEN_OWNER; storage never resizes.
        unsafe { (*self.storage.as_ptr().cast::<TOKEN_OWNER>()).Owner }
    }
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

    /// The owner Windows assigns to objects this token creates. For an
    /// administrator's token that is BUILTIN\Administrators, not the user, so
    /// a folder the same account made can carry a different owner SID than
    /// `sid()` and still be its own.
    fn default_owner(&self) -> io::Result<DefaultOwner> {
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
            GetTokenInformation(
                token.as_raw_handle(),
                TokenOwner,
                null_mut(),
                0,
                &mut length,
            )
        };
        if first != 0
            || io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
            || !(size_of::<TOKEN_OWNER>() as u32..=64 * 1024).contains(&length)
        {
            return Err(denied("cannot bound the current Windows default owner"));
        }
        let mut storage = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        // SAFETY: aligned storage has at least length writable bytes; the token
        // remains open and the output-length pointer is valid.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenOwner,
                storage.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let result = DefaultOwner { storage };
        // SAFETY: GetTokenInformation populated a complete TOKEN_OWNER and SID.
        if unsafe { IsValidSid(result.sid()) } == 0 {
            return Err(denied("invalid Windows default owner"));
        }
        Ok(result)
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

pub(super) fn open_relative(
    parent: &File,
    name: &std::ffi::OsStr,
    create: bool,
    directory: bool,
) -> io::Result<File> {
    open_relative_with_access(parent, name, create, directory, false, 0)
}

/// Make one existing directory, owned by the current user, private in place.
/// Its contents are not touched; the caller decides what moves below it.
pub(super) fn adopt_directory_relative(parent: &File, name: &std::ffi::OsStr) -> io::Result<File> {
    use windows_sys::Win32::Security::Authorization::SetSecurityInfo;
    use windows_sys::Win32::Security::{
        GetSecurityDescriptorDacl, PROTECTED_DACL_SECURITY_INFORMATION,
    };
    use windows_sys::Win32::Storage::FileSystem::{WRITE_DAC, WRITE_OWNER};
    // The owner holds WRITE_DAC implicitly; WRITE_OWNER is granted only by the
    // private DACL, so ownership is taken after that DACL is in place.
    let file = open_relative_with_access(parent, name, false, true, false, WRITE_DAC).map_err(
        |error| {
            io::Error::new(
                error.kind(),
                format!("cannot open the leftover folder for adoption: {error}"),
            )
        },
    )?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: file owns the live handle and information is writable.
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
        || unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(denied(
            "the existing .rrflow folder must be a real folder, not a link",
        ));
    }
    let user = CurrentUser::query()?;
    let mut owner = null_mut();
    let mut raw = null_mut();
    // SAFETY: the handle grants READ_CONTROL; raw owns the returned descriptor.
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut raw,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let _owner_descriptor = LocalAllocation(raw);
    let default_owner = user.default_owner()?;
    // SAFETY: owner points into the retained descriptor; all SIDs are live.
    // A folder this account created is owned either by the user or by the
    // token's default owner (BUILTIN\Administrators for an administrator),
    // which is what the GitHub Windows runner and any elevated shell produce.
    if owner.is_null()
        || unsafe { IsValidSid(owner) } == 0
        || (unsafe { EqualSid(owner, user.sid()) } == 0
            && unsafe { EqualSid(owner, default_owner.sid()) } == 0)
    {
        return Err(denied(
            "the existing .rrflow folder belongs to another account",
        ));
    }
    let text = user.text()?;
    let private = descriptor(&format!("D:P(A;OICI;FA;;;{text})(A;OICI;FA;;;SY)"))?;
    let mut present = 0;
    let mut dacl = null_mut();
    let mut defaulted = 0;
    // SAFETY: private owns the converted descriptor; outputs are live.
    if unsafe { GetSecurityDescriptorDacl(private.0, &mut present, &mut dacl, &mut defaulted) } == 0
        || present == 0
        || dacl.is_null()
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the handle grants WRITE_DAC and the DACL lives in `private`.
    let status = unsafe {
        SetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            dacl,
            null(),
        )
    };
    if status != 0 {
        let error = io::Error::from_raw_os_error(status as i32);
        return Err(io::Error::new(
            error.kind(),
            format!("cannot make the leftover folder private: {error}"),
        ));
    }
    // A fresh private folder is created with the user as its owner. A leftover
    // made under an elevated token or on a hosted runner is owned by
    // BUILTIN\Administrators instead; the private DACL just set grants the
    // user WRITE_OWNER, so ownership is taken through a second handle and the
    // adopted folder becomes indistinguishable from a fresh one.
    // SAFETY: owner points into the retained descriptor and user.sid() is live.
    if unsafe { EqualSid(owner, user.sid()) } == 0 {
        let owning = open_relative_with_access(parent, name, false, true, false, WRITE_OWNER)
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("cannot open the leftover folder to take ownership: {error}"),
                )
            })?;
        // SAFETY: the handle grants WRITE_OWNER and the SID lives in `user`.
        let status = unsafe {
            SetSecurityInfo(
                owning.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION,
                user.sid(),
                null_mut(),
                null_mut(),
                null(),
            )
        };
        if status != 0 {
            let error = io::Error::from_raw_os_error(status as i32);
            return Err(io::Error::new(
                error.kind(),
                format!("cannot take ownership of the leftover folder: {error}"),
            ));
        }
    }
    Ok(file)
}

pub(super) fn open_empty_relative(
    parent: &File,
    name: &std::ffi::OsStr,
    create: bool,
) -> io::Result<File> {
    // Empty lease creation needs no payload write access. Read sharing lets
    // contenders reach LockFileEx and lets read-only verification admit it.
    // Deletion and writes remain excluded for this handle's entire lifetime.
    open_relative_with_access(parent, name, create, false, true, 0)
}

fn open_relative_with_access(
    parent: &File,
    name: &std::ffi::OsStr,
    create: bool,
    directory: bool,
    empty_lease: bool,
    extra_access: u32,
) -> io::Result<File> {
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
        FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{RtlNtStatusToDosError, UNICODE_STRING};
    use windows_sys::Win32::Security::{SecurityIdentification, SECURITY_QUALITY_OF_SERVICE};
    use windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE;
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;
    let mut name: Vec<u16> = name.encode_wide().collect();
    if name.is_empty() || name.contains(&0) || name.len() > 32_767 {
        return Err(denied("invalid private leaf name"));
    }
    let length = (name.len() * 2) as u16;
    let mut string = UNICODE_STRING {
        Length: length,
        MaximumLength: length,
        Buffer: name.as_mut_ptr(),
    };
    let security = if create {
        let user = CurrentUser::query()?.text()?;
        let inheritance = if directory { "OICI" } else { "" };
        Some(descriptor(&format!(
            "O:{user}D:P(A;{inheritance};FA;;;{user})(A;{inheritance};FA;;;SY)"
        ))?)
    } else {
        None
    };
    let mut qos = SECURITY_QUALITY_OF_SERVICE {
        Length: size_of::<SECURITY_QUALITY_OF_SERVICE>() as u32,
        ImpersonationLevel: SecurityIdentification,
        ContextTrackingMode: 0,
        EffectiveOnly: true,
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle(),
        ObjectName: &mut string,
        Attributes: 0,
        SecurityDescriptor: security.as_ref().map_or(null_mut(), |value| value.0.cast()),
        SecurityQualityOfService: (&mut qos as *mut SECURITY_QUALITY_OF_SERVICE).cast(),
    };
    let mut status = IO_STATUS_BLOCK::default();
    let mut raw = null_mut();
    // One ordinary component relative to a retained directory. The descriptor
    // is applied atomically by FILE_CREATE, before the first secret byte.
    let result = unsafe {
        NtCreateFile(
            &mut raw,
            GENERIC_READ
                | READ_CONTROL
                | FILE_READ_ATTRIBUTES
                | SYNCHRONIZE
                | extra_access
                | if create && !empty_lease {
                    GENERIC_WRITE
                } else {
                    0
                },
            &attributes,
            &mut status,
            null(),
            if directory {
                FILE_ATTRIBUTE_DIRECTORY
            } else {
                FILE_ATTRIBUTE_NORMAL
            },
            if directory {
                FILE_SHARE_READ | FILE_SHARE_WRITE
            } else if create && !empty_lease {
                0
            } else {
                FILE_SHARE_READ
            },
            if create { FILE_CREATE } else { FILE_OPEN },
            FILE_OPEN_REPARSE_POINT
                | FILE_SYNCHRONOUS_IO_NONALERT
                | if directory {
                    FILE_DIRECTORY_FILE
                } else {
                    FILE_NON_DIRECTORY_FILE
                },
            null(),
            0,
        )
    };
    if result < 0 {
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(result) } as i32,
        ));
    }
    // SAFETY: a successful NtCreateFile transfers one owned file handle.
    Ok(unsafe { File::from_raw_handle(raw) })
}

#[cfg(test)]
pub(super) fn open_directory(path: &Path, create: bool) -> io::Result<File> {
    let path = rrd_store::filesystem::AnchoredPath::from_path(path)?;
    path.with_parent(|parent, name| open_relative(parent, name, create, true))
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

#[cfg(test)]
pub(super) fn open(path: &Path, create: bool) -> io::Result<File> {
    let path = rrd_store::filesystem::AnchoredPath::from_path(path)?;
    path.open_file_with(|parent, name| open_relative(parent, name, create, false))
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
mod tests;
