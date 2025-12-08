//! Windows-specific ACL reading implementation
//! Uses Win32 Security APIs to read file ACLs

#[cfg(target_os = "windows")]
use std::path::Path;

#[cfg(target_os = "windows")]
use super::permissions::{
    AclEntryType, PermissionError, WindowsAcl, WindowsAclEntry, WindowsPermissionType,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE, PSID,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::{
    GetFileSecurityW, GetSecurityDescriptorDacl, GetSecurityDescriptorGroup,
    GetSecurityDescriptorOwner, LookupAccountSidW, ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE,
    ACL as WIN_ACL, DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
    OWNER_SECURITY_INFORMATION, SECURITY_DESCRIPTOR, SID_NAME_USE,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ALL_ACCESS, FILE_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_DATA,
    FILE_WRITE_DATA,
};

/
#[cfg(target_os = "windows")]
mod access_masks {
    pub const GENERIC_READ: u32 = 0x80000000;
    pub const GENERIC_WRITE: u32 = 0x40000000;
    pub const GENERIC_EXECUTE: u32 = 0x20000000;
    pub const GENERIC_ALL: u32 = 0x10000000;
    pub const DELETE: u32 = 0x00010000;
    pub const READ_CONTROL: u32 = 0x00020000;
    pub const WRITE_DAC: u32 = 0x00040000;
    pub const WRITE_OWNER: u32 = 0x00080000;
    pub const SYNCHRONIZE: u32 = 0x00100000;
    pub const FILE_READ_DATA: u32 = 0x0001;
    pub const FILE_WRITE_DATA: u32 = 0x0002;
    pub const FILE_APPEND_DATA: u32 = 0x0004;
    pub const FILE_READ_EA: u32 = 0x0008;
    pub const FILE_WRITE_EA: u32 = 0x0010;
    pub const FILE_EXECUTE: u32 = 0x0020;
    pub const FILE_DELETE_CHILD: u32 = 0x0040;
    pub const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    pub const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
}

/
#[cfg(target_os = "windows")]
pub fn read_windows_acl(path: &Path) -> Result<WindowsAcl, PermissionError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut acl = WindowsAcl::new();

    let mut needed_size: u32 = 0;
    let security_info =
        OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;

    unsafe {
        GetFileSecurityW(
            wide_path.as_ptr(),
            security_info,
            std::ptr::null_mut(),
            0,
            &mut needed_size,
        );
    }

    if needed_size == 0 {
        let metadata = std::fs::metadata(path)?;
        let readonly = metadata.permissions().readonly();

        let permissions = if readonly {
            vec![
                WindowsPermissionType::Read,
                WindowsPermissionType::ReadAndExecute,
            ]
        } else {
            vec![WindowsPermissionType::FullControl]
        };

        acl.add_entry(WindowsAclEntry::new(
            "Everyone".to_string(),
            AclEntryType::Allow,
            permissions,
        ));

        return Ok(acl);
    }

    let mut buffer: Vec<u8> = vec![0; needed_size as usize];
    let mut actual_size: u32 = 0;

    let success = unsafe {
        GetFileSecurityW(
            wide_path.as_ptr(),
            security_info,
            buffer.as_mut_ptr() as *mut _,
            needed_size,
            &mut actual_size,
        )
    };

    if success == 0 {
        let metadata = std::fs::metadata(path)?;
        let readonly = metadata.permissions().readonly();

        let permissions = if readonly {
            vec![
                WindowsPermissionType::Read,
                WindowsPermissionType::ReadAndExecute,
            ]
        } else {
            vec![WindowsPermissionType::FullControl]
        };

        acl.add_entry(WindowsAclEntry::new(
            "Everyone".to_string(),
            AclEntryType::Allow,
            permissions,
        ));

        return Ok(acl);
    }

    let sd_ptr = buffer.as_ptr() as *const SECURITY_DESCRIPTOR;

    let mut owner_sid: PSID = std::ptr::null_mut();
    let mut owner_defaulted: i32 = 0;

    unsafe {
        if GetSecurityDescriptorOwner(sd_ptr as *mut _, &mut owner_sid, &mut owner_defaulted) != 0 {
            if !owner_sid.is_null() {
                if let Some(name) = sid_to_name(owner_sid) {
                    acl.owner = Some(name);
                }
            }
        }
    }

    let mut group_sid: PSID = std::ptr::null_mut();
    let mut group_defaulted: i32 = 0;

    unsafe {
        if GetSecurityDescriptorGroup(sd_ptr as *mut _, &mut group_sid, &mut group_defaulted) != 0 {
            if !group_sid.is_null() {
                if let Some(name) = sid_to_name(group_sid) {
                    acl.group = Some(name);
                }
            }
        }
    }

    let mut dacl_present: i32 = 0;
    let mut dacl_ptr: *mut WIN_ACL = std::ptr::null_mut();
    let mut dacl_defaulted: i32 = 0;

    unsafe {
        if GetSecurityDescriptorDacl(
            sd_ptr as *mut _,
            &mut dacl_present,
            &mut dacl_ptr,
            &mut dacl_defaulted,
        ) != 0
        {
            if dacl_present != 0 && !dacl_ptr.is_null() {
                parse_dacl(dacl_ptr, &mut acl);
            }
        }
    }

    if acl.entries.is_empty() {
        let metadata = std::fs::metadata(path)?;
        let readonly = metadata.permissions().readonly();

        let permissions = if readonly {
            vec![
                WindowsPermissionType::Read,
                WindowsPermissionType::ReadAndExecute,
            ]
        } else {
            vec![WindowsPermissionType::FullControl]
        };

        acl.add_entry(WindowsAclEntry::new(
            "Everyone".to_string(),
            AclEntryType::Allow,
            permissions,
        ));
    }

    Ok(acl)
}

/
#[cfg(target_os = "windows")]
fn sid_to_name(sid: PSID) -> Option<String> {
    let mut name_size: u32 = 256;
    let mut domain_size: u32 = 256;
    let mut name_buffer: Vec<u16> = vec![0; name_size as usize];
    let mut domain_buffer: Vec<u16> = vec![0; domain_size as usize];
    let mut sid_type: SID_NAME_USE = 0;

    let success = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            sid,
            name_buffer.as_mut_ptr(),
            &mut name_size,
            domain_buffer.as_mut_ptr(),
            &mut domain_size,
            &mut sid_type,
        )
    };

    if success != 0 {
        let name = String::from_utf16_lossy(&name_buffer[..name_size as usize]);
        let domain = String::from_utf16_lossy(&domain_buffer[..domain_size as usize]);

        if domain.is_empty() {
            Some(name)
        } else {
            Some(format!("{}\\{}", domain, name))
        }
    } else {
        None
    }
}

/
#[cfg(target_os = "windows")]
fn parse_dacl(dacl: *mut WIN_ACL, acl: &mut WindowsAcl) {
    unsafe {
        let dacl_ref = &*dacl;
        let ace_count = dacl_ref.AceCount as usize;

        let mut ace_ptr = (dacl as *const u8).add(std::mem::size_of::<WIN_ACL>());

        for _ in 0..ace_count {
            let ace_header = ace_ptr as *const AceHeader;
            let ace_type = (*ace_header).ace_type;
            let ace_size = (*ace_header).ace_size as usize;

            match ace_type {
                0 => {
                    let ace = ace_ptr as *const ACCESS_ALLOWED_ACE;
                    let mask = (*ace).Mask;
                    let sid = &(*ace).SidStart as *const u32 as PSID;

                    if let Some(name) = sid_to_name(sid) {
                        let permissions = mask_to_permissions(mask);
                        if !permissions.is_empty() {
                            acl.add_entry(WindowsAclEntry {
                                principal_name: name,
                                principal_sid: None,
                                entry_type: AclEntryType::Allow,
                                permissions,
                                inherited: ((*ace_header).ace_flags & 0x10) != 0,
                            });
                        }
                    }
                }
                1 => {
                    let ace = ace_ptr as *const ACCESS_DENIED_ACE;
                    let mask = (*ace).Mask;
                    let sid = &(*ace).SidStart as *const u32 as PSID;

                    if let Some(name) = sid_to_name(sid) {
                        let permissions = mask_to_permissions(mask);
                        if !permissions.is_empty() {
                            acl.add_entry(WindowsAclEntry {
                                principal_name: name,
                                principal_sid: None,
                                entry_type: AclEntryType::Deny,
                                permissions,
                                inherited: ((*ace_header).ace_flags & 0x10) != 0,
                            });
                        }
                    }
                }
                _ => {
                }
            }

            ace_ptr = ace_ptr.add(ace_size);
        }
    }
}

/
#[cfg(target_os = "windows")]
#[repr(C)]
struct AceHeader {
    ace_type: u8,
    ace_flags: u8,
    ace_size: u16,
}

/
#[cfg(target_os = "windows")]
fn mask_to_permissions(mask: u32) -> Vec<WindowsPermissionType> {
    use access_masks::*;

    let mut permissions = Vec::new();

    if (mask & GENERIC_ALL) != 0 || mask == 0x1F01FF {
        permissions.push(WindowsPermissionType::FullControl);
        return permissions;
    }

    if (mask & (FILE_WRITE_DATA | FILE_APPEND_DATA | DELETE)) == (FILE_WRITE_DATA | FILE_APPEND_DATA | DELETE) {
        permissions.push(WindowsPermissionType::Modify);
    } else {
        if (mask & FILE_WRITE_DATA) != 0 || (mask & GENERIC_WRITE) != 0 {
            permissions.push(WindowsPermissionType::Write);
        }
    }

    if (mask & (FILE_READ_DATA | FILE_EXECUTE)) == (FILE_READ_DATA | FILE_EXECUTE)
        || (mask & GENERIC_EXECUTE) != 0
    {
        permissions.push(WindowsPermissionType::ReadAndExecute);
    } else if (mask & FILE_READ_DATA) != 0 || (mask & GENERIC_READ) != 0 {
        permissions.push(WindowsPermissionType::Read);
    }

    if permissions.is_empty() && mask != 0 {
        permissions.push(WindowsPermissionType::Special);
    }

    permissions
}

/
#[cfg(not(target_os = "windows"))]
pub fn read_windows_acl(_path: &std::path::Path) -> Result<super::permissions::WindowsAcl, super::permissions::PermissionError> {
    Err(super::permissions::PermissionError::PlatformNotSupported(
        "Windows ACL reading is only available on Windows".to_string(),
    ))
}
