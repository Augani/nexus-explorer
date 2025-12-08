use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;

/// Cloud sync status for files in cloud storage locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CloudSyncStatus {
    #[default]
    None,
    Synced,
    Syncing,
    Pending,
    Error,
    CloudOnly,
    LocalOnly,
}

impl CloudSyncStatus {
    pub fn icon_name(&self) -> Option<&'static str> {
        match self {
            CloudSyncStatus::None => None,
            CloudSyncStatus::Synced => Some("check"),
            CloudSyncStatus::Syncing => Some("refresh-cw"),
            CloudSyncStatus::Pending => Some("clock"),
            CloudSyncStatus::Error => Some("triangle-alert"),
            CloudSyncStatus::CloudOnly => Some("cloud"),
            CloudSyncStatus::LocalOnly => Some("hard-drive"),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CloudSyncStatus::None => "",
            CloudSyncStatus::Synced => "Synced",
            CloudSyncStatus::Syncing => "Syncing...",
            CloudSyncStatus::Pending => "Pending sync",
            CloudSyncStatus::Error => "Sync error",
            CloudSyncStatus::CloudOnly => "Available online only",
            CloudSyncStatus::LocalOnly => "Local only",
        }
    }

    pub fn color(&self) -> Option<u32> {
        match self {
            CloudSyncStatus::None => None,
            CloudSyncStatus::Synced => Some(0x3fb950),
            CloudSyncStatus::Syncing => Some(0x58a6ff),
            CloudSyncStatus::Pending => Some(0xd29922),
            CloudSyncStatus::Error => Some(0xf85149),
            CloudSyncStatus::CloudOnly => Some(0x8b949e),
            CloudSyncStatus::LocalOnly => Some(0x8b949e),
        }
    }
}

/// Single file or directory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    #[serde(with = "system_time_serde")]
    pub modified: SystemTime,
    pub file_type: FileType,
    pub icon_key: IconKey,
    /// Linux permissions (for WSL paths on Windows)
    #[serde(default)]
    pub linux_permissions: Option<LinuxFilePermissions>,
    /// Cloud sync status (for files in cloud storage locations)
    #[serde(default)]
    pub sync_status: CloudSyncStatus,
    /// Whether this entry is a symbolic link
    #[serde(default)]
    pub is_symlink: bool,
    /// Target path for symbolic links
    #[serde(default)]
    pub symlink_target: Option<PathBuf>,
    /// Whether the symlink target is broken (doesn't exist)
    #[serde(default)]
    pub is_broken_symlink: bool,
}

/// Linux file permissions for WSL integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinuxFilePermissions {
    /// Permission mode (e.g., 0o755)
    pub mode: u32,
    /// Owner username
    pub owner: String,
    /// Group name
    pub group: String,
}

impl LinuxFilePermissions {
    pub fn new(mode: u32, owner: String, group: String) -> Self {
        Self { mode, owner, group }
    }

    /// Format permissions as rwxrwxrwx string
    pub fn format_mode(&self) -> String {
        let mut result = String::with_capacity(9);

        // Owner permissions
        result.push(if self.mode & 0o400 != 0 { 'r' } else { '-' });
        result.push(if self.mode & 0o200 != 0 { 'w' } else { '-' });
        result.push(if self.mode & 0o100 != 0 { 'x' } else { '-' });

        // Group permissions
        result.push(if self.mode & 0o040 != 0 { 'r' } else { '-' });
        result.push(if self.mode & 0o020 != 0 { 'w' } else { '-' });
        result.push(if self.mode & 0o010 != 0 { 'x' } else { '-' });

        // Other permissions
        result.push(if self.mode & 0o004 != 0 { 'r' } else { '-' });
        result.push(if self.mode & 0o002 != 0 { 'w' } else { '-' });
        result.push(if self.mode & 0o001 != 0 { 'x' } else { '-' });

        result
    }

    /// Format as full permission string like "-rwxr-xr-x owner group"
    pub fn format_full(&self) -> String {
        let type_char = '-';
        format!(
            "{}{} {} {}",
            type_char,
            self.format_mode(),
            self.owner,
            self.group
        )
    }
}

/// Detected file type for icon selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileType {
    Directory,
    RegularFile,
    Symlink,
    Unknown,
}

/// Key for IconCache lookup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IconKey {
    Directory,
    GenericFile,
    Extension(String),
    MimeType(String),
    Custom(PathBuf),
}

/// Current loading state of the file system model
#[derive(Debug, Clone, PartialEq)]
pub enum LoadState {
    Idle,
    Loading { request_id: usize },
    Loaded { count: usize, duration: Duration },
    Error { message: String },
    Cached { stale: bool },
}

/// Cached directory state for LRU cache
#[derive(Debug, Clone)]
pub struct CachedDirectory {
    pub entries: Vec<FileEntry>,
    pub generation: usize,
    pub timestamp: Instant,
    pub mtime: SystemTime,
}

/// File system change events
#[derive(Debug, Clone, PartialEq)]
pub enum FsEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// File system errors with proper error handling
#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Platform error: {0}")]
    Platform(String),
}

/// Result type alias for file system operations
pub type Result<T> = std::result::Result<T, FileSystemError>;

mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        (duration.as_secs(), duration.subsec_nanos()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (secs, nanos): (u64, u32) = Deserialize::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::new(secs, nanos))
    }
}

impl FileEntry {
    pub fn new(name: String, path: PathBuf, is_dir: bool, size: u64, modified: SystemTime) -> Self {
        let file_type = if is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        let icon_key = if is_dir {
            IconKey::Directory
        } else {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| IconKey::Extension(ext.to_lowercase()))
                .unwrap_or(IconKey::GenericFile)
        };

        Self {
            name,
            path,
            is_dir,
            size,
            modified,
            file_type,
            icon_key,
            linux_permissions: None,
            sync_status: CloudSyncStatus::None,
            is_symlink: false,
            symlink_target: None,
            is_broken_symlink: false,
        }
    }

    /// Create a FileEntry with symlink information
    pub fn with_symlink_info(mut self, target: PathBuf, is_broken: bool) -> Self {
        self.is_symlink = true;
        self.symlink_target = Some(target);
        self.is_broken_symlink = is_broken;
        self.file_type = FileType::Symlink;
        self
    }

    /// Check if this entry is a symbolic link
    pub fn is_symlink(&self) -> bool {
        self.is_symlink
    }

    /// Get the symlink target path if this is a symbolic link
    pub fn symlink_target(&self) -> Option<&Path> {
        self.symlink_target.as_deref()
    }

    /// Check if this is a broken symbolic link (target doesn't exist)
    pub fn is_broken_symlink(&self) -> bool {
        self.is_broken_symlink
    }

    /// Create a FileEntry with Linux permissions (for WSL paths)
    pub fn with_linux_permissions(mut self, permissions: LinuxFilePermissions) -> Self {
        self.linux_permissions = Some(permissions);
        self
    }

    /// Set the cloud sync status for this entry
    pub fn with_sync_status(mut self, status: CloudSyncStatus) -> Self {
        self.sync_status = status;
        self
    }

    /// Update the sync status
    pub fn set_sync_status(&mut self, status: CloudSyncStatus) {
        self.sync_status = status;
    }

    /// Create a FileEntry from a path, detecting symlinks automatically
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let symlink_metadata = std::fs::symlink_metadata(path).ok()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_symlink = symlink_metadata.file_type().is_symlink();

        if is_symlink {
            let target = std::fs::read_link(path).ok();
            let target_exists = std::fs::metadata(path).is_ok();
            let is_broken = !target_exists;

            // For symlinks, get metadata of target if it exists, otherwise use symlink metadata
            let (is_dir, size, modified) = if target_exists {
                let target_meta = std::fs::metadata(path).ok()?;
                (
                    target_meta.is_dir(),
                    if target_meta.is_dir() { 0 } else { target_meta.len() },
                    target_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                )
            } else {
                (false, 0, symlink_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH))
            };

            let mut entry = Self::new(name, path.to_path_buf(), is_dir, size, modified);
            if let Some(target_path) = target {
                entry = entry.with_symlink_info(target_path, is_broken);
            } else {
                entry.is_symlink = true;
                entry.is_broken_symlink = true;
                entry.file_type = FileType::Symlink;
            }
            Some(entry)
        } else {
            let is_dir = symlink_metadata.is_dir();
            let size = if is_dir { 0 } else { symlink_metadata.len() };
            let modified = symlink_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Some(Self::new(name, path.to_path_buf(), is_dir, size, modified))
        }
    }
}

impl Default for LoadState {
    fn default() -> Self {
        Self::Idle
    }
}

impl CachedDirectory {
    pub fn new(entries: Vec<FileEntry>, generation: usize, mtime: SystemTime) -> Self {
        Self {
            entries,
            generation,
            timestamp: Instant::now(),
            mtime,
        }
    }

    pub fn is_stale(&self, current_mtime: SystemTime) -> bool {
        self.mtime != current_mtime
    }
}

/// Column to sort file entries by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SortColumn {
    #[default]
    Name,
    Date,
    Type,
    Size,
}

/// Sort direction (ascending or descending)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    pub fn is_ascending(&self) -> bool {
        matches!(self, SortDirection::Ascending)
    }
}

/// State for sorting file entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortState {
    pub column: SortColumn,
    pub direction: SortDirection,
    pub directories_first: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: SortColumn::Name,
            direction: SortDirection::Ascending,
            directories_first: true,
        }
    }
}

impl SortState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle sort column. If same column, reverse direction. If different column, use default direction.
    pub fn toggle_column(&mut self, column: SortColumn) {
        if self.column == column {
            self.direction = self.direction.toggle();
        } else {
            self.column = column;
            // Default directions: Name ascending, Date descending (newest first), Size descending (largest first), Type ascending
            self.direction = match column {
                SortColumn::Name => SortDirection::Ascending,
                SortColumn::Date => SortDirection::Descending,
                SortColumn::Type => SortDirection::Ascending,
                SortColumn::Size => SortDirection::Descending,
            };
        }
    }

    /// Sort entries in place according to current sort state
    pub fn sort_entries(&self, entries: &mut [FileEntry]) {
        if self.directories_first {
            // Partition into directories and files, sort each group
            let (mut dirs, mut files): (Vec<_>, Vec<_>) =
                entries.iter().cloned().partition(|e| e.is_dir);

            self.sort_by_column(&mut dirs);
            self.sort_by_column(&mut files);

            // Combine back: directories first, then files
            let combined: Vec<_> = dirs.into_iter().chain(files).collect();
            entries.clone_from_slice(&combined);
        } else {
            self.sort_by_column(entries);
        }
    }

    fn sort_by_column(&self, entries: &mut [FileEntry]) {
        let direction = self.direction;

        entries.sort_by(|a, b| {
            let ordering = match self.column {
                SortColumn::Name => compare_names(&a.name, &b.name),
                SortColumn::Date => a.modified.cmp(&b.modified),
                SortColumn::Type => compare_types(a, b),
                SortColumn::Size => a.size.cmp(&b.size),
            };

            if direction == SortDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }

    pub fn set_directories_first(&mut self, value: bool) {
        self.directories_first = value;
    }
}

/// Case-insensitive name comparison
fn compare_names(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

/// Compare by file extension (type)
fn compare_types(a: &FileEntry, b: &FileEntry) -> Ordering {
    let ext_a = get_extension(&a.name);
    let ext_b = get_extension(&b.name);
    ext_a.to_lowercase().cmp(&ext_b.to_lowercase())
}

/// Extract file extension for sorting
fn get_extension(name: &str) -> &str {
    name.rsplit('.')
        .next()
        .filter(|ext| *ext != name)
        .unwrap_or("")
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
