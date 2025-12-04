use crate::models::{CloudSyncStatus, FileEntry, FileType, IconKey};
use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Get the list of items in the system trash
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
