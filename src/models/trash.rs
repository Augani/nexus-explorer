use std::path::PathBuf;
use std::process::Command;
use crate::models::{FileEntry, FileType, IconKey, CloudSyncStatus};
use std::time::SystemTime;

/// Get the list of items in the system trash
pub fn list_trash_items() -> Vec<FileEntry> {
    let mut entries = Vec::new();
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, use mdfind to query Spotlight for trash items
        if let Ok(output) = Command::new("mdfind")
            .args(["-onlyin", &get_trash_path().to_string_lossy(), "kMDItemFSName == '*'"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let path = PathBuf::from(line);
                    if let Some(entry) = create_entry_from_path(&path) {
                        entries.push(entry);
                    }
                }
            }
        }
        
        // Fallback: try direct listing (may fail without Full Disk Access)
        if entries.is_empty() {
            entries = list_directory_entries(&get_trash_path());
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // On Linux, use freedesktop trash via the trash crate
        if let Ok(items) = trash::os_limited::list() {
            for item in items {
                let name = item.name.clone();
                let path = item.original_parent.join(&item.name);
                let is_dir = std::fs::metadata(&item.id).map(|m| m.is_dir()).unwrap_or(false);
                
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
                
                entries.push(FileEntry {
                    name,
                    path,
                    is_dir,
                    size: 0,
                    modified: item.time_deleted.into(),
                    file_type,
                    icon_key,
                    linux_permissions: None,
                    sync_status: CloudSyncStatus::None,
                });
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, use os_limited from trash crate
        if let Ok(items) = trash::os_limited::list() {
            for item in items {
                let name = item.name.clone();
                let path = item.original_parent.join(&item.name);
                let is_dir = std::fs::metadata(&item.id).map(|m| m.is_dir()).unwrap_or(false);
                
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
                
                entries.push(FileEntry {
                    name,
                    path,
                    is_dir,
                    size: 0,
                    modified: item.time_deleted.into(),
                    file_type,
                    icon_key,
                    linux_permissions: None,
                    sync_status: CloudSyncStatus::None,
                });
            }
        }
    }
    
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    
    entries
}

fn create_entry_from_path(path: &PathBuf) -> Option<FileEntry> {
    let name = path.file_name()?.to_str()?.to_string();
    let metadata = std::fs::metadata(path).ok()?;
    let is_dir = metadata.is_dir();
    let size = metadata.len();
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    
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
    })
}

fn list_directory_entries(path: &PathBuf) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            if let Some(file_entry) = create_entry_from_path(&entry.path()) {
                entries.push(file_entry);
            }
        }
    }
    entries
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
