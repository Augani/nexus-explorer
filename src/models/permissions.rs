use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid permission mode: {0}")]
    InvalidMode(u32),

    #[error("Elevation required: {0}")]
    ElevationRequired(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[cfg(target_os = "windows")]
    #[error("Windows ACL error: {0}")]
    WindowsAclError(String),
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermissionBits {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl PermissionBits {
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self { read, write, execute }
    }

    pub fn from_mode(mode: u8) -> Self {
        Self {
            read: (mode & 0b100) != 0,
            write: (mode & 0b010) != 0,
            execute: (mode & 0b001) != 0,
        }
    }

    pub fn to_mode(&self) -> u8 {
        let mut mode = 0u8;
        if self.read { mode |= 0b100; }
        if self.write { mode |= 0b010; }
        if self.execute { mode |= 0b001; }
        mode
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpecialBits {
    pub setuid: bool,
    pub setgid: bool,
    pub sticky: bool,
}

impl SpecialBits {
    pub fn new(setuid: bool, setgid: bool, sticky: bool) -> Self {
        Self { setuid, setgid, sticky }
    }

    pub fn from_mode(mode: u32) -> Self {
        Self {
            setuid: (mode & 0o4000) != 0,
            setgid: (mode & 0o2000) != 0,
            sticky: (mode & 0o1000) != 0,
        }
    }

    pub fn to_mode(&self) -> u32 {
        let mut mode = 0u32;
        if self.setuid { mode |= 0o4000; }
        if self.setgid { mode |= 0o2000; }
        if self.sticky { mode |= 0o1000; }
        mode
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnixPermissions {
    pub owner: PermissionBits,
    pub group: PermissionBits,
    pub others: PermissionBits,
    pub special: SpecialBits,
    pub owner_name: Option<String>,
    pub group_name: Option<String>,
    pub owner_id: Option<u32>,
    pub group_id: Option<u32>,
}

impl UnixPermissions {

    pub fn from_mode(mode: u32) -> Result<Self, PermissionError> {
        if mode > 0o7777 {
            return Err(PermissionError::InvalidMode(mode));
        }

        Ok(Self {
            owner: PermissionBits::from_mode(((mode >> 6) & 0o7) as u8),
            group: PermissionBits::from_mode(((mode >> 3) & 0o7) as u8),
            others: PermissionBits::from_mode((mode & 0o7) as u8),
            special: SpecialBits::from_mode(mode),
            owner_name: None,
            group_name: None,
            owner_id: None,
            group_id: None,
        })
    }


    pub fn to_mode(&self) -> u32 {
        let owner = (self.owner.to_mode() as u32) << 6;
        let group = (self.group.to_mode() as u32) << 3;
        let others = self.others.to_mode() as u32;
        let special = self.special.to_mode();
        special | owner | group | others
    }


    pub fn to_symbolic(&self) -> String {
        let format_bits = |bits: &PermissionBits, special: Option<char>, is_exec: bool| -> String {
            let r = if bits.read { 'r' } else { '-' };
            let w = if bits.write { 'w' } else { '-' };
            let x = match (bits.execute, special) {
                (true, Some(c)) => c.to_ascii_uppercase(),
                (false, Some(c)) => c,
                (true, None) => 'x',
                (false, None) => '-',
            };
            format!("{}{}{}", r, w, x)
        };

        let owner_special = if self.special.setuid { Some('s') } else { None };
        let group_special = if self.special.setgid { Some('s') } else { None };
        let other_special = if self.special.sticky { Some('t') } else { None };

        format!(
            "{}{}{}",
            format_bits(&self.owner, owner_special, true),
            format_bits(&self.group, group_special, true),
            format_bits(&self.others, other_special, true)
        )
    }


    pub fn to_octal_string(&self) -> String {
        format!("{:04o}", self.to_mode())
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsPermissionType {
    FullControl,
    Modify,
    ReadAndExecute,
    Read,
    Write,
    ListFolderContents,
    Special,
}

impl WindowsPermissionType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FullControl => "Full Control",
            Self::Modify => "Modify",
            Self::ReadAndExecute => "Read & Execute",
            Self::Read => "Read",
            Self::Write => "Write",
            Self::ListFolderContents => "List Folder Contents",
            Self::Special => "Special Permissions",
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclEntryType {
    Allow,
    Deny,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsAclEntry {
    pub principal_name: String,
    pub principal_sid: Option<String>,
    pub entry_type: AclEntryType,
    pub permissions: Vec<WindowsPermissionType>,
    pub inherited: bool,
}

impl WindowsAclEntry {
    pub fn new(
        principal_name: String,
        entry_type: AclEntryType,
        permissions: Vec<WindowsPermissionType>,
    ) -> Self {
        Self {
            principal_name,
            principal_sid: None,
            entry_type,
            permissions,
            inherited: false,
        }
    }
}


#[derive(Debug, Clone, Default)]
pub struct WindowsAcl {
    pub owner: Option<String>,
    pub owner_sid: Option<String>,
    pub group: Option<String>,
    pub group_sid: Option<String>,
    pub entries: Vec<WindowsAclEntry>,
    pub inheritance_enabled: bool,
}

impl WindowsAcl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entry(&mut self, entry: WindowsAclEntry) {
        self.entries.push(entry);
    }


    pub fn get_effective_permissions(&self, principal: &str) -> Vec<WindowsPermissionType> {
        let mut allowed: Vec<WindowsPermissionType> = Vec::new();
        let mut denied: Vec<WindowsPermissionType> = Vec::new();

        for entry in &self.entries {
            if entry.principal_name.eq_ignore_ascii_case(principal) {
                match entry.entry_type {
                    AclEntryType::Allow => allowed.extend(entry.permissions.iter().cloned()),
                    AclEntryType::Deny => denied.extend(entry.permissions.iter().cloned()),
                }
            }
        }

        allowed.retain(|p| !denied.contains(p));
        allowed
    }
}



#[derive(Debug, Clone)]
pub enum FilePermissions {
    Unix(UnixPermissions),
    Windows(WindowsAcl),
}

impl FilePermissions {

    pub fn is_readable(&self) -> bool {
        match self {
            Self::Unix(perms) => perms.owner.read || perms.group.read || perms.others.read,
            Self::Windows(acl) => {
                acl.entries.iter().any(|e| {
                    e.entry_type == AclEntryType::Allow
                        && (e.permissions.contains(&WindowsPermissionType::Read)
                            || e.permissions.contains(&WindowsPermissionType::FullControl))
                })
            }
        }
    }


    pub fn is_writable(&self) -> bool {
        match self {
            Self::Unix(perms) => perms.owner.write || perms.group.write || perms.others.write,
            Self::Windows(acl) => {
                acl.entries.iter().any(|e| {
                    e.entry_type == AclEntryType::Allow
                        && (e.permissions.contains(&WindowsPermissionType::Write)
                            || e.permissions.contains(&WindowsPermissionType::Modify)
                            || e.permissions.contains(&WindowsPermissionType::FullControl))
                })
            }
        }
    }


    pub fn is_executable(&self) -> bool {
        match self {
            Self::Unix(perms) => perms.owner.execute || perms.group.execute || perms.others.execute,
            Self::Windows(acl) => {
                acl.entries.iter().any(|e| {
                    e.entry_type == AclEntryType::Allow
                        && (e.permissions.contains(&WindowsPermissionType::ReadAndExecute)
                            || e.permissions.contains(&WindowsPermissionType::FullControl))
                })
            }
        }
    }
}


pub struct PermissionsManager;

impl PermissionsManager {

    pub fn read_permissions(path: &Path) -> Result<FilePermissions, PermissionError> {
        #[cfg(unix)]
        {
            Self::read_unix_permissions(path)
        }
        #[cfg(windows)]
        {
            Self::read_windows_permissions(path)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(PermissionError::PlatformNotSupported(
                "Permissions not supported on this platform".to_string(),
            ))
        }
    }


    pub fn write_permissions(path: &Path, permissions: &FilePermissions) -> Result<(), PermissionError> {
        match permissions {
            #[cfg(unix)]
            FilePermissions::Unix(perms) => Self::write_unix_permissions(path, perms),
            #[cfg(windows)]
            FilePermissions::Windows(_acl) => {
                Err(PermissionError::ElevationRequired(
                    "Modifying Windows ACLs requires administrator privileges".to_string(),
                ))
            }
            #[allow(unreachable_patterns)]
            _ => Err(PermissionError::PlatformNotSupported(
                "Cannot write permissions for this platform type".to_string(),
            )),
        }
    }

    #[cfg(unix)]
    fn read_unix_permissions(path: &Path) -> Result<FilePermissions, PermissionError> {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(path)?;
        let mode = metadata.permissions().mode();
        
        let mut perms = UnixPermissions::from_mode(mode & 0o7777)?;
        perms.owner_id = Some(metadata.uid());
        perms.group_id = Some(metadata.gid());

        #[cfg(target_os = "linux")]
        {
            perms.owner_name = get_user_name(metadata.uid());
            perms.group_name = get_group_name(metadata.gid());
        }
        #[cfg(target_os = "macos")]
        {
            perms.owner_name = get_user_name(metadata.uid());
            perms.group_name = get_group_name(metadata.gid());
        }

        Ok(FilePermissions::Unix(perms))
    }

    #[cfg(unix)]
    fn write_unix_permissions(path: &Path, perms: &UnixPermissions) -> Result<(), PermissionError> {
        use std::os::unix::fs::PermissionsExt;

        let mode = perms.to_mode();
        let permissions = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[cfg(windows)]
    fn read_windows_permissions(path: &Path) -> Result<FilePermissions, PermissionError> {
        let acl = super::permissions_windows::read_windows_acl(path)?;
        Ok(FilePermissions::Windows(acl))
    }


    pub fn requires_elevation(path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                unsafe {
                    let uid = libc::getuid();
                    metadata.uid() != uid
                }
            } else {
                true
            }
        }
        #[cfg(windows)]
        {
            let _ = path;
            true
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            true
        }
    }
}

#[cfg(unix)]
fn get_user_name(uid: u32) -> Option<String> {
    unsafe {
        let passwd = libc::getpwuid(uid);
        if passwd.is_null() {
            return None;
        }
        let name = std::ffi::CStr::from_ptr((*passwd).pw_name);
        name.to_str().ok().map(|s| s.to_string())
    }
}

#[cfg(unix)]
fn get_group_name(gid: u32) -> Option<String> {
    unsafe {
        let group = libc::getgrgid(gid);
        if group.is_null() {
            return None;
        }
        let name = std::ffi::CStr::from_ptr((*group).gr_name);
        name.to_str().ok().map(|s| s.to_string())
    }
}

impl PermissionsManager {

    #[cfg(unix)]
    pub fn apply_recursive(
        path: &Path,
        permissions: &UnixPermissions,
        include_directories: bool,
    ) -> Result<Vec<std::path::PathBuf>, PermissionError> {
        use std::os::unix::fs::PermissionsExt;

        let mut failed_paths = Vec::new();
        let mode = permissions.to_mode();
        let fs_permissions = std::fs::Permissions::from_mode(mode);

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();

                if entry_path.is_dir() {
                    if include_directories {
                        if std::fs::set_permissions(&entry_path, fs_permissions.clone()).is_err() {
                            failed_paths.push(entry_path.clone());
                        }
                    }
                    let sub_failed = Self::apply_recursive(&entry_path, permissions, include_directories)?;
                    failed_paths.extend(sub_failed);
                } else {
                    if std::fs::set_permissions(&entry_path, fs_permissions.clone()).is_err() {
                        failed_paths.push(entry_path);
                    }
                }
            }
        }

        Ok(failed_paths)
    }


    #[cfg(unix)]
    pub fn change_ownership(path: &Path, uid: Option<u32>, gid: Option<u32>) -> Result<(), PermissionError> {
        use std::os::unix::ffi::OsStrExt;

        let path_cstr = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| PermissionError::FileNotFound(path.display().to_string()))?;

        let uid = uid.map(|u| u as libc::uid_t).unwrap_or(u32::MAX as libc::uid_t);
        let gid = gid.map(|g| g as libc::gid_t).unwrap_or(u32::MAX as libc::gid_t);

        let result = unsafe { libc::chown(path_cstr.as_ptr(), uid, gid) };

        if result == 0 {
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                Err(PermissionError::ElevationRequired(
                    "Changing ownership requires root privileges".to_string(),
                ))
            } else {
                Err(PermissionError::IoError(err))
            }
        }
    }


    pub fn get_file_type_char(path: &Path) -> char {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => return '?',
        };

        if metadata.is_symlink() {
            'l'
        } else if metadata.is_dir() {
            'd'
        } else if metadata.is_file() {
            '-'
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                let ft = metadata.file_type();
                if ft.is_block_device() {
                    'b'
                } else if ft.is_char_device() {
                    'c'
                } else if ft.is_fifo() {
                    'p'
                } else if ft.is_socket() {
                    's'
                } else {
                    '?'
                }
            }
            #[cfg(not(unix))]
            {
                '?'
            }
        }
    }


    #[cfg(unix)]
    pub fn format_ls_style(path: &Path) -> Result<String, PermissionError> {
        let perms = Self::read_permissions(path)?;
        if let FilePermissions::Unix(unix_perms) = perms {
            let file_type = Self::get_file_type_char(path);
            Ok(format!("{}{}", file_type, unix_perms.to_symbolic()))
        } else {
            Err(PermissionError::PlatformNotSupported(
                "Expected Unix permissions".to_string(),
            ))
        }
    }
}

impl UnixPermissions {

    pub fn preset_file_default() -> Self {
        Self::from_mode(0o644).unwrap()
    }

    pub fn preset_file_executable() -> Self {
        Self::from_mode(0o755).unwrap()
    }

    pub fn preset_directory_default() -> Self {
        Self::from_mode(0o755).unwrap()
    }

    pub fn preset_private() -> Self {
        Self::from_mode(0o600).unwrap()
    }

    pub fn preset_private_executable() -> Self {
        Self::from_mode(0o700).unwrap()
    }


    pub fn has_special_bits(&self) -> bool {
        self.special.setuid || self.special.setgid || self.special.sticky
    }


    pub fn describe_special_bits(&self) -> Vec<&'static str> {
        let mut descriptions = Vec::new();
        if self.special.setuid {
            descriptions.push("Set User ID (setuid)");
        }
        if self.special.setgid {
            descriptions.push("Set Group ID (setgid)");
        }
        if self.special.sticky {
            descriptions.push("Sticky Bit");
        }
        descriptions
    }
}
