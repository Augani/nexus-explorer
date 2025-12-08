use crate::models::{CloudSyncStatus, FileEntry, FileType, IconKey};
use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Metadata for a trash entry including original location and deletion time
#[derive(Debug, Clone)]
pub struct TrashEntry {
    /// The name of the file/folder
    pub name: String,
    /// The original path before deletion
    pub original_path: PathBuf,
    /// When the item was deleted
    pub deletion_date: SystemTime,
    /// Size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// The internal ID used by the trash system for restoration
    pub trash_id: TrashId,
}

/// Platform-specific identifier for trash items
#[derive(Debug, Clone)]
pub enum TrashId {
    /// Path to the item in trash (macOS/Linux)
    Path(PathBuf),
    /// Windows Recycle Bin item identifier
    #[cfg(target_os = "windows")]
    Windows(String),
}

/// Error types for trash operations
#[derive(Debug, Clone)]
pub enum TrashError {
    /// Item not found in trash
    NotFound(String),
    /// Original location no longer exists
    OriginalLocationMissing(PathBuf),
    /// Permission denied
    PermissionDenied(String),
    /// IO error
    IoError(String),
    /// Platform-specific error
    PlatformError(String),
}

impl std::fmt::Display for TrashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrashError::NotFound(msg) => write!(f, "Item not found: {}", msg),
            TrashError::OriginalLocationMissing(path) => {
                write!(f, "Original location missing: {}", path.display())
            }
            TrashError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            TrashError::IoError(msg) => write!(f, "IO error: {}", msg),
            TrashError::PlatformError(msg) => write!(f, "Platform error: {}", msg),
        }
    }
}

impl std::error::Error for TrashError {}

/// Manages trash/recycle bin operations across platforms
pub struct TrashManager {
    /// Cached list of trash entries
    entries: Vec<TrashEntry>,
    /// Total size of all items in trash
    total_size: u64,
}

impl TrashManager {
    /// Create a new TrashManager
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            total_size: 0,
        }
    }

    /// Refresh the list of trash entries from the system
    pub fn refresh(&mut self) {
        self.entries = list_trash_entries();
        self.total_size = self.entries.iter().map(|e| e.size).sum();
    }

    /// Get all trash entries
    pub fn entries(&self) -> &[TrashEntry] {
        &self.entries
    }

    /// Get the total size of all items in trash
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// Get the number of items in trash
    pub fn item_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if trash is considered "large" (over 1GB)
    pub fn is_large(&self) -> bool {
        self.total_size > 1024 * 1024 * 1024
    }

    /// Restore an item from trash to its original location
    pub fn restore(&mut self, entry: &TrashEntry) -> Result<PathBuf, TrashError> {
        let result = restore_from_trash(entry)?;
        self.refresh();
        Ok(result)
    }

    /// Permanently delete a single item from trash
    pub fn delete_permanently(&mut self, entry: &TrashEntry) -> Result<(), TrashError> {
        delete_from_trash(entry)?;
        self.refresh();
        Ok(())
    }

    /// Empty the entire trash
    pub fn empty(&mut self) -> Result<(), TrashError> {
        empty_trash_internal()?;
        self.entries.clear();
        self.total_size = 0;
        Ok(())
    }

    /// Move a file to trash
    pub fn move_to_trash(&mut self, path: &PathBuf) -> Result<(), TrashError> {
        trash::delete(path).map_err(|e| TrashError::IoError(e.to_string()))?;
        self.refresh();
        Ok(())
    }
}

impl Default for TrashManager {
    fn default() -> Self {
        Self::new()
    }
}

/// List all entries in the system trash with metadata
pub fn list_trash_entries() -> Vec<TrashEntry> {
    #[cfg(target_os = "macos")]
    {
        list_trash_entries_macos()
    }

    #[cfg(target_os = "linux")]
    {
        list_trash_entries_linux()
    }

    #[cfg(target_os = "windows")]
    {
        list_trash_entries_windows()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn list_trash_entries_macos() -> Vec<TrashEntry> {
    
    let mut entries = Vec::new();
    let trash_path = get_trash_path();
    
    if let Ok(dir_entries) = std::fs::read_dir(&trash_path) {
        for entry in dir_entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            
            if name.is_empty() || name.starts_with('.') {
                continue;
            }
            
            let metadata = std::fs::metadata(&path).ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = if is_dir {
                calculate_dir_size(&path)
            } else {
                metadata.as_ref().map(|m| m.len()).unwrap_or(0)
            };
            
            let deletion_date = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::now());
            
            // On macOS, the original path is stored in .DS_Store or we use the trash path
            // For simplicity, we'll use the name as the original path indicator
            let original_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(&name);
            
            entries.push(TrashEntry {
                name,
                original_path,
                deletion_date,
                size,
                is_dir,
                trash_id: TrashId::Path(path),
            });
        }
    }
    
    entries.sort_by(|a, b| b.deletion_date.cmp(&a.deletion_date));
    entries
}

#[cfg(target_os = "linux")]
fn list_trash_entries_linux() -> Vec<TrashEntry> {
    use std::time::{Duration, UNIX_EPOCH};
    
    let mut entries = Vec::new();
    
    if let Ok(items) = trash::os_limited::list() {
        for item in items {
            let name = item.name.to_string_lossy().to_string();
            let original_path = item.original_parent.join(&item.name);
            
            let is_dir = std::fs::metadata(&item.id)
                .map(|m| m.is_dir())
                .unwrap_or(false);
            
            let size = if is_dir {
                calculate_dir_size(&item.id)
            } else {
                std::fs::metadata(&item.id)
                    .map(|m| m.len())
                    .unwrap_or(0)
            };
            
            let deletion_date = if item.time_deleted >= 0 {
                UNIX_EPOCH + Duration::from_secs(item.time_deleted as u64)
            } else {
                SystemTime::now()
            };
            
            entries.push(TrashEntry {
                name,
                original_path,
                deletion_date,
                size,
                is_dir,
                trash_id: TrashId::Path(item.id),
            });
        }
    }
    
    entries.sort_by(|a, b| b.deletion_date.cmp(&a.deletion_date));
    entries
}

#[cfg(target_os = "windows")]
fn list_trash_entries_windows() -> Vec<TrashEntry> {
    use std::time::{Duration, UNIX_EPOCH};
    
    let mut entries = Vec::new();
    
    if let Ok(items) = trash::os_limited::list() {
        for item in items {
            let name = item.name.to_string_lossy().to_string();
            let original_path = item.original_parent.join(&item.name);
            
            let is_dir = std::fs::metadata(&item.id)
                .map(|m| m.is_dir())
                .unwrap_or(false);
            
            let size = if is_dir {
                calculate_dir_size(&item.id)
            } else {
                std::fs::metadata(&item.id)
                    .map(|m| m.len())
                    .unwrap_or(0)
            };
            
            let deletion_date = if item.time_deleted >= 0 {
                UNIX_EPOCH + Duration::from_secs(item.time_deleted as u64)
            } else {
                SystemTime::now()
            };
            
            entries.push(TrashEntry {
                name: name.clone(),
                original_path,
                deletion_date,
                size,
                is_dir,
                trash_id: TrashId::Windows(item.id.to_string_lossy().to_string()),
            });
        }
    }
    
    entries.sort_by(|a, b| b.deletion_date.cmp(&a.deletion_date));
    entries
}

/// Calculate the total size of a directory recursively
pub fn calculate_dir_size(path: &PathBuf) -> u64 {
    let mut size = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                size += calculate_dir_size(&path);
            } else if let Ok(metadata) = std::fs::metadata(&path) {
                size += metadata.len();
            }
        }
    }
    size
}

/// Restore an item from trash to its original location
pub fn restore_from_trash(entry: &TrashEntry) -> Result<PathBuf, TrashError> {
    // Check if original location's parent exists
    if let Some(parent) = entry.original_path.parent() {
        if !parent.exists() {
            // Create the parent directory
            std::fs::create_dir_all(parent)
                .map_err(|e| TrashError::IoError(format!("Failed to create directory: {}", e)))?;
        }
    }
    
    // Check if something already exists at the original path
    if entry.original_path.exists() {
        return Err(TrashError::IoError(format!(
            "File already exists at: {}",
            entry.original_path.display()
        )));
    }
    
    match &entry.trash_id {
        TrashId::Path(trash_path) => {
            std::fs::rename(trash_path, &entry.original_path)
                .map_err(|e| TrashError::IoError(format!("Failed to restore: {}", e)))?;
        }
        #[cfg(target_os = "windows")]
        TrashId::Windows(_id) => {
            // On Windows, use the trash crate's restore functionality
            // The trash crate doesn't have a direct restore, so we need to handle it manually
            if let TrashId::Path(trash_path) = &entry.trash_id {
                std::fs::rename(trash_path, &entry.original_path)
                    .map_err(|e| TrashError::IoError(format!("Failed to restore: {}", e)))?;
            } else {
                return Err(TrashError::PlatformError(
                    "Windows restore not fully implemented".to_string(),
                ));
            }
        }
    }
    
    Ok(entry.original_path.clone())
}

/// Permanently delete an item from trash
fn delete_from_trash(entry: &TrashEntry) -> Result<(), TrashError> {
    match &entry.trash_id {
        TrashId::Path(trash_path) => {
            if entry.is_dir {
                std::fs::remove_dir_all(trash_path)
                    .map_err(|e| TrashError::IoError(format!("Failed to delete: {}", e)))?;
            } else {
                std::fs::remove_file(trash_path)
                    .map_err(|e| TrashError::IoError(format!("Failed to delete: {}", e)))?;
            }
        }
        #[cfg(target_os = "windows")]
        TrashId::Windows(id) => {
            // On Windows, the ID is the path in the recycle bin
            let path = PathBuf::from(id);
            if entry.is_dir {
                std::fs::remove_dir_all(&path)
                    .map_err(|e| TrashError::IoError(format!("Failed to delete: {}", e)))?;
            } else {
                std::fs::remove_file(&path)
                    .map_err(|e| TrashError::IoError(format!("Failed to delete: {}", e)))?;
            }
        }
    }
    Ok(())
}

/// Internal function to empty the trash
fn empty_trash_internal() -> Result<(), TrashError> {
    #[cfg(target_os = "macos")]
    {
        let trash_path = get_trash_path();
        if trash_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&trash_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                            .map_err(|e| TrashError::IoError(e.to_string()))?;
                    } else {
                        std::fs::remove_file(&path)
                            .map_err(|e| TrashError::IoError(e.to_string()))?;
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let trash_files = dirs::data_local_dir()
            .map(|d| d.join("Trash/files"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local/share/Trash/files"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            });

        let trash_info = dirs::data_local_dir()
            .map(|d| d.join("Trash/info"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local/share/Trash/info"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            });

        for trash_dir in [&trash_files, &trash_info] {
            if trash_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(trash_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir() {
                            let _ = std::fs::remove_dir_all(&path);
                        } else {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, we iterate through all items and delete them
        if let Ok(items) = trash::os_limited::list() {
            for item in items {
                if item.id.is_dir() {
                    let _ = std::fs::remove_dir_all(&item.id);
                } else {
                    let _ = std::fs::remove_file(&item.id);
                }
            }
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(TrashError::PlatformError(
            "Empty trash not implemented for this platform".to_string(),
        ))
    }
}

/// Get the list of items in the system trash (legacy function for compatibility)
pub fn list_trash_items() -> Vec<FileEntry> {
    #[cfg(target_os = "macos")]
    {
        list_trash_macos()
    }

    #[cfg(target_os = "linux")]
    {
        list_trash_linux()
    }

    #[cfg(target_os = "windows")]
    {
        list_trash_windows()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn list_trash_macos() -> Vec<FileEntry> {
    let mut entries = Vec::new();

    let script = r#"
        tell application "Finder"
            set trashItems to every item of trash
            set output to ""
            repeat with anItem in trashItems
                set itemPath to POSIX path of (anItem as alias)
                set output to output & itemPath & linefeed
            end repeat
            return output
        end tell
    "#;

    if let Ok(output) = Command::new("osascript").args(["-e", script]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.is_empty() {
                    continue;
                }
                let path = PathBuf::from(line.trim());
                if let Some(entry) = create_entry_from_path(&path) {
                    entries.push(entry);
                }
            }
        }
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

#[cfg(target_os = "linux")]
fn list_trash_linux() -> Vec<FileEntry> {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let mut entries = Vec::new();

    if let Ok(items) = trash::os_limited::list() {
        for item in items {
            let name = item.name.to_string_lossy().to_string();
            let path = item.original_parent.join(&item.name);
            let is_dir = std::fs::metadata(&item.id)
                .map(|m| m.is_dir())
                .unwrap_or(false);

            let file_type = if is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            };

            let icon_key = if is_dir {
                IconKey::Directory
            } else {
                PathBuf::from(&name)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IconKey::Extension(ext.to_lowercase()))
                    .unwrap_or(IconKey::GenericFile)
            };

            let modified = if item.time_deleted >= 0 {
                UNIX_EPOCH + Duration::from_secs(item.time_deleted as u64)
            } else {
                SystemTime::now()
            };

            entries.push(FileEntry {
                name,
                path,
                is_dir,
                size: 0,
                modified,
                file_type,
                icon_key,
                linux_permissions: None,
                sync_status: CloudSyncStatus::None,
                is_symlink: false,
                symlink_target: None,
                is_broken_symlink: false,
            });
        }
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

#[cfg(target_os = "windows")]
fn list_trash_windows() -> Vec<FileEntry> {
    use std::time::{Duration, UNIX_EPOCH};

    let mut entries = Vec::new();

    if let Ok(items) = trash::os_limited::list() {
        for item in items {
            let name = item.name.to_string_lossy().to_string();
            let path = item.original_parent.join(&item.name);
            let is_dir = std::fs::metadata(&item.id)
                .map(|m| m.is_dir())
                .unwrap_or(false);

            let file_type = if is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            };

            let icon_key = if is_dir {
                IconKey::Directory
            } else {
                PathBuf::from(&name)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IconKey::Extension(ext.to_lowercase()))
                    .unwrap_or(IconKey::GenericFile)
            };

            let modified = if item.time_deleted >= 0 {
                UNIX_EPOCH + Duration::from_secs(item.time_deleted as u64)
            } else {
                SystemTime::now()
            };

            entries.push(FileEntry {
                name,
                path,
                is_dir,
                size: 0,
                modified,
                file_type,
                icon_key,
                linux_permissions: None,
                sync_status: CloudSyncStatus::None,
                is_symlink: false,
                symlink_target: None,
                is_broken_symlink: false,
            });
        }
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

fn create_entry_from_path(path: &PathBuf) -> Option<FileEntry> {
    let name = path.file_name()?.to_str()?.to_string();
    let metadata = std::fs::metadata(path).ok();
    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH);

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

    Some(FileEntry {
        name,
        path: path.clone(),
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
    })
}

/// Get the trash path for the current platform
pub fn get_trash_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|h| h.join(".Trash"))
            .unwrap_or_else(|| PathBuf::from("/.Trash"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::data_local_dir()
            .map(|d| d.join("Trash/files"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local/share/Trash/files"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            })
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\$Recycle.Bin")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("/tmp")
    }
}

/// Check if a path is the trash folder
pub fn is_trash_path(path: &PathBuf) -> bool {
    let trash_path = get_trash_path();
    path == &trash_path
}

/// Empty the system trash
pub fn empty_trash() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::fs;

        let trash_path = get_trash_path();

        if !trash_path.exists() {
            return Ok(());
        }

        let entries: Vec<_> = fs::read_dir(&trash_path)
            .map_err(|e| format!("Failed to read trash: {}", e))?
            .filter_map(|e| e.ok())
            .collect();

        if entries.is_empty() {
            return Ok(());
        }

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
            } else {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
            }
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let trash_files = dirs::data_local_dir()
            .map(|d| d.join("Trash/files"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local/share/Trash/files"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            });

        let trash_info = dirs::data_local_dir()
            .map(|d| d.join("Trash/info"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local/share/Trash/info"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            });

        for trash_dir in [&trash_files, &trash_info] {
            if trash_dir.exists() {
                if let Ok(entries) = fs::read_dir(trash_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir() {
                            let _ = fs::remove_dir_all(&path);
                        } else {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Err("Empty trash not implemented for Windows - use system Recycle Bin".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err("Empty trash not implemented for this platform".to_string())
    }
}
