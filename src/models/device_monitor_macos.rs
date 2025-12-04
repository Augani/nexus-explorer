use super::device_monitor::{Device, DeviceId, DeviceMonitor, DeviceType, get_disk_space};
use std::path::PathBuf;

impl DeviceMonitor {
    /// Enumerate devices on macOS by scanning /Volumes
    #[cfg(target_os = "macos")]
    pub fn enumerate_macos_devices(&mut self) {
        // Add root volume
        let root_path = PathBuf::from("/");
        if let Ok((total, free)) = get_disk_space(&root_path) {
            let root_device = Device::new(
                DeviceId::new(0),
                "Macintosh HD".to_string(),
                root_path,
                DeviceType::InternalDrive,
            )
            .with_space(total, free)
            .with_removable(false);
            
            self.add_device(root_device);
        }

        // Scan /Volumes for mounted volumes
        let volumes_path = PathBuf::from("/Volumes");
        if let Ok(entries) = std::fs::read_dir(&volumes_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                // Skip the root volume symlink
                if path.file_name().map(|n| n == "Macintosh HD").unwrap_or(false) {
                    continue;
                }
                
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                
                let device_type = detect_macos_device_type(&path);
                let is_removable = matches!(
                    device_type,
                    DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::DiskImage
                );
                
                let mut device = Device::new(
                    DeviceId::new(0),
                    name,
                    path.clone(),
                    device_type,
                )
                .with_removable(is_removable);
                
                if let Ok((total, free)) = get_disk_space(&path) {
                    device = device.with_space(total, free);
                }
                
                device = device.with_read_only(is_volume_read_only(&path));
                
                self.add_device(device);
            }
        }
    }

    /// Eject a device on macOS
    #[cfg(target_os = "macos")]
    pub fn eject(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self.get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;
        
        if !device.is_removable {
            return Err(super::device_monitor::DeviceError::EjectFailed(
                "Device is not removable".to_string()
            ));
        }
        
        let path = device.path.clone();
        
        // Use diskutil to eject
        let output = std::process::Command::new("diskutil")
            .args(["eject", path.to_str().unwrap_or("")])
            .output()?;
        
        if output.status.success() {
            self.remove_device(id);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(super::device_monitor::DeviceError::EjectFailed(error.to_string()))
        }
    }

    /// Unmount a device on macOS
    #[cfg(target_os = "macos")]
    pub fn unmount(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self.get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;
        
        let path = device.path.clone();
        
        // Use diskutil to unmount
        let output = std::process::Command::new("diskutil")
            .args(["unmount", path.to_str().unwrap_or("")])
            .output()?;
        
        if output.status.success() {
            self.remove_device(id);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(super::device_monitor::DeviceError::UnmountFailed(error.to_string()))
        }
    }
}

/// Detect the type of device based on volume characteristics
#[cfg(target_os = "macos")]
fn detect_macos_device_type(path: &PathBuf) -> DeviceType {
    let path_str = path.to_string_lossy();
    
    if path_str.contains(".dmg") || path_str.contains("disk image") {
        return DeviceType::DiskImage;
    }
    
    if path_str.starts_with("/Volumes/") {
        if let Ok(output) = std::process::Command::new("mount").output() {
            let mount_info = String::from_utf8_lossy(&output.stdout);
            let path_escaped = path_str.replace(' ', "\\ ");
            
            for line in mount_info.lines() {
                if line.contains(&*path_str) || line.contains(&path_escaped) {
                    if line.contains("smbfs") || line.contains("nfs") || line.contains("afpfs") {
                        return DeviceType::NetworkDrive;
                    }
                    if line.contains("devfs") || line.contains("disk") {
                        if is_external_disk(path) {
                            return DeviceType::ExternalDrive;
                        }
                    }
                }
            }
        }
    }
    
    // Default to external drive for mounted volumes
    DeviceType::ExternalDrive
}

/// Check if a disk is external
#[cfg(target_os = "macos")]
fn is_external_disk(path: &PathBuf) -> bool {
    if let Ok(output) = std::process::Command::new("diskutil")
        .args(["info", path.to_str().unwrap_or("")])
        .output()
    {
        let info = String::from_utf8_lossy(&output.stdout);
        return info.contains("Removable Media:") && info.contains("Yes");
    }
    false
}

/// Check if a volume is read-only
#[cfg(target_os = "macos")]
fn is_volume_read_only(path: &PathBuf) -> bool {
    use std::os::unix::fs::MetadataExt;
    
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.mode();
        // If no write permission for owner, consider it read-only
        return (mode & 0o200) == 0;
    }
    false
}
