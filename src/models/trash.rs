use crate::models::{CloudSyncStatus, FileEntry, FileType, IconKey};
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

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

    // Use AppleScript to get trash items via Finder
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
    let mut entries = Vec::new();

    if let Ok(items) = trash::os_limited::list() {
        for item in items {
            let name = item.name.clone();
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

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

#[cfg(target_os = "windows")]
fn list_trash_windows() -> Vec<FileEntry> {
    let mut entries = Vec::new();

    if let Ok(items) = trash::os_limited::list() {
        for item in items {
            let name = item.name.clone();
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
        let script = r#"
            tell application "Finder"
                empty trash
            end tell
        "#;
        
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;
        
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to empty trash: {}", stderr))
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, use the trash crate
        Err("Empty trash not implemented for this platform".to_string())
    }
}
