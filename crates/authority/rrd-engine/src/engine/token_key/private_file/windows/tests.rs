use super::super::{read_private_bounded, write_private_new};
use super::*;
use std::os::windows::fs::OpenOptionsExt;
use windows_sys::Win32::Security::Authorization::SetSecurityInfo;
use windows_sys::Win32::Security::{
    GetSecurityDescriptorDacl, PROTECTED_DACL_SECURITY_INFORMATION,
    UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows_sys::Win32::Storage::FileSystem::{READ_CONTROL, WRITE_DAC};

#[test]
#[ignore = "required native CI explicitly runs the privileged ownership fixture"]
fn foreign_owner_is_denied_without_repair() {
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, GetSecurityDescriptorOwner, LookupPrivilegeValueW,
        LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    };
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_BACKUP_SEMANTICS, WRITE_OWNER};
    assert_eq!(
        std::env::var("RRFLOW_PRIVATE_OWNER_FIXTURE").as_deref(),
        Ok("1")
    );
    let mut raw = null_mut();
    // SAFETY: current process pseudo handle and writable token output.
    assert_ne!(
        unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES,
                &mut raw,
            )
        },
        0
    );
    // SAFETY: successful OpenProcessToken transfers ownership.
    let token = unsafe { OwnedHandle::from_raw_handle(raw) };
    let name: Vec<u16> = "SeRestorePrivilege\0".encode_utf16().collect();
    let mut luid = Default::default();
    // SAFETY: live NUL-terminated name and writable LUID.
    assert_ne!(
        unsafe { LookupPrivilegeValueW(null(), name.as_ptr(), &mut luid) },
        0
    );
    let desired = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let mut previous = TOKEN_PRIVILEGES::default();
    let mut length = 0;
    // SAFETY: both privilege buffers have the declared one-entry size.
    unsafe { SetLastError(0) };
    assert_ne!(
        unsafe {
            AdjustTokenPrivileges(
                token.as_raw_handle(),
                0,
                &desired,
                size_of::<TOKEN_PRIVILEGES>() as u32,
                &mut previous,
                &mut length,
            )
        },
        0
    );
    // AdjustTokenPrivileges can succeed while reporting NOT_ALL_ASSIGNED.
    assert_eq!(
        unsafe { GetLastError() },
        0,
        "required restore privilege unavailable"
    );
    struct RestorePrivilege(OwnedHandle, TOKEN_PRIVILEGES);
    impl Drop for RestorePrivilege {
        fn drop(&mut self) {
            // SAFETY: restore exactly the captured state of the retained token.
            unsafe {
                AdjustTokenPrivileges(
                    self.0.as_raw_handle(),
                    0,
                    &self.1,
                    0,
                    null_mut(),
                    null_mut(),
                )
            };
        }
    }
    let _privilege = RestorePrivilege(token, previous);
    let user = CurrentUser::query().unwrap();
    let system = descriptor("O:SY").unwrap();
    let mut foreign = null_mut();
    let mut defaulted = 0;
    // SAFETY: system owns the descriptor and its returned SID throughout this test.
    assert_ne!(
        unsafe { GetSecurityDescriptorOwner(system.0, &mut foreign, &mut defaulted) },
        0
    );
    assert_eq!(unsafe { EqualSid(user.sid(), foreign) }, 0);
    let root = tempfile::tempdir().unwrap();
    let directory_path = root.path().join("private");
    let directory = open_directory(&directory_path, true).unwrap();
    let path = directory_path.join("secret");
    write_private_new(&path, b"unchanged ownership fixture").unwrap();
    let file = open(&path, false).unwrap();
    validate_directory(&directory).unwrap();
    validate(&file).unwrap();
    struct RestoreOwner<'a>(File, &'a CurrentUser);
    impl Drop for RestoreOwner<'_> {
        fn drop(&mut self) {
            // SAFETY: only the retained fixture handle, original SID and owner field.
            unsafe {
                SetSecurityInfo(
                    self.0.as_raw_handle(),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION,
                    self.1.sid(),
                    null_mut(),
                    null(),
                    null(),
                )
            };
        }
    }
    let mut owners = Vec::new();
    for path in [&directory_path, &path] {
        let handle = std::fs::OpenOptions::new()
            .access_mode(READ_CONTROL | WRITE_OWNER)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .unwrap();
        owners.push(RestoreOwner(handle, &user));
        // SAFETY: SeRestorePrivilege admits the fixture-only foreign owner;
        // DACL and all other security fields remain untouched.
        assert_eq!(
            unsafe {
                SetSecurityInfo(
                    owners.last().unwrap().0.as_raw_handle(),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION,
                    foreign,
                    null_mut(),
                    null(),
                    null(),
                )
            },
            0
        );
    }
    for _ in 0..2 {
        assert!(validate_directory(&directory).is_err());
        assert!(validate(&file).is_err());
        assert!(read_private_bounded(&path, 64).is_err());
        for object in [&directory, &file] {
            let mut owner = null_mut();
            let mut security = null_mut();
            // SAFETY: retained readable fixture handle and writable outputs.
            assert_eq!(
                unsafe {
                    GetSecurityInfo(
                        object.as_raw_handle(),
                        SE_FILE_OBJECT,
                        OWNER_SECURITY_INFORMATION,
                        &mut owner,
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        &mut security,
                    )
                },
                0
            );
            let _security = LocalAllocation(security);
            assert_ne!(
                unsafe { EqualSid(owner, foreign) },
                0,
                "denial must not repair ownership"
            );
        }
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"unchanged ownership fixture"
        );
    }
    drop(owners);
    validate_directory(&directory).unwrap();
    validate(&file).unwrap();
}

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
