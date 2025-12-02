use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;

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
        let duration = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
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
    pub fn new(
        name: String,
        path: PathBuf,
        is_dir: bool,
        size: u64,
        modified: SystemTime,
    ) -> Self {
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
    name.rsplit('.').next().filter(|ext| *ext != name).unwrap_or("")
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
