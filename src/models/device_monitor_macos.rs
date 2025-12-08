//! macOS device detection using DiskArbitration framework
//! 
//! This module provides device enumeration and monitoring for macOS using:
//! - DiskArbitration framework for disk events and metadata
//! - /Volumes directory scanning for mounted volumes
//! - diskutil command for additional device information

use super::device_monitor::{get_disk_space, Device, DeviceEvent, DeviceId, DeviceMonitor, DeviceType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// Note: core_foundation imports are available for future DiskArbitration integration
// Currently using diskutil command-line tool for device enumeration

/// Information about a macOS disk obtained from DiskArbitration
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub bsd_name: String,
    pub volume_name: Option<String>,
    pub volume_path: Option<PathBuf>,
    pub volume_uuid: Option<String>,
    pub media_name: Option<String>,
    pub media_size: u64,
    pub is_removable: bool,
    pub is_ejectable: bool,
    pub is_internal: bool,
    pub is_network: bool,
    pub is_whole_disk: bool,
    pub filesystem_type: Option<String>,
    pub bus_name: Option<String>,
}

impl Default for DiskInfo {
    fn default() -> Self {
        Self {
            bsd_name: String::new(),
            volume_name: None,
            volume_path: None,
            volume_uuid: None,
            media_name: None,
            media_size: 0,
            is_removable: false,
            is_ejectable: false,
            is_internal: true,
            is_network: false,
            is_whole_disk: false,
            filesystem_type: None,
            bus_name: None,
        }
    }
}

impl DiskInfo {
    /// Determine the device type based on disk properties
    pub fn device_type(&self) -> DeviceType {
        if self.is_network {
            return DeviceType::NetworkDrive;
        }

        // Check for disk images
        if let Some(ref fs) = self.filesystem_type {
            if fs.contains("hfs") || fs.contains("apfs") {
                if let Some(ref path) = self.volume_path {
                    let path_str = path.to_string_lossy().to_lowercase();
                    if path_str.contains(".dmg") || path_str.contains("disk image") {
                        return DeviceType::DiskImage;
                    }
                }
            }
        }

        // Check bus type for USB drives
        if let Some(ref bus) = self.bus_name {
            let bus_lower = bus.to_lowercase();
            if bus_lower.contains("usb") {
                return DeviceType::UsbDrive;
            }
        }

        // Check for optical drives
        if let Some(ref media) = self.media_name {
            let media_lower = media.to_lowercase();
            if media_lower.contains("cd") || media_lower.contains("dvd") || media_lower.contains("bd") {
                return DeviceType::OpticalDrive;
            }
        }

        if self.is_removable || self.is_ejectable {
            return DeviceType::ExternalDrive;
        }

        if self.is_internal {
            return DeviceType::InternalDrive;
        }

        DeviceType::ExternalDrive
    }
}

/// macOS disk monitor using DiskArbitration framework
#[cfg(target_os = "macos")]
pub struct MacOSDiskMonitor {
    is_monitoring: Arc<AtomicBool>,
    event_sender: Arc<Mutex<Option<flume::Sender<DeviceEvent>>>>,
    known_disks: Arc<Mutex<HashMap<String, DiskInfo>>>,
}

#[cfg(target_os = "macos")]
impl MacOSDiskMonitor {
    pub fn new() -> Self {
        Self {
            is_monitoring: Arc::new(AtomicBool::new(false)),
            event_sender: Arc::new(Mutex::new(None)),
            known_disks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Enumerate all mounted volumes using /Volumes directory and diskutil
    pub fn enumerate_volumes(&self) -> Vec<DiskInfo> {
        let mut disks = Vec::new();

        // Get root volume info
        if let Some(root_info) = self.get_volume_info(&PathBuf::from("/")) {
            disks.push(root_info);
        }

        // Scan /Volumes for mounted volumes
        let volumes_path = PathBuf::from("/Volumes");
        if let Ok(entries) = std::fs::read_dir(&volumes_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                // Skip symlinks to root
                if let Ok(target) = std::fs::read_link(&path) {
                    if target == PathBuf::from("/") {
                        continue;
                    }
                }

                // Skip "Macintosh HD" symlink
                if path.file_name().map(|n| n == "Macintosh HD").unwrap_or(false) {
                    if std::fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                        continue;
                    }
                }

                if let Some(info) = self.get_volume_info(&path) {
                    disks.push(info);
                }
            }
        }

        disks
    }

    /// Get detailed volume information using diskutil
    fn get_volume_info(&self, path: &PathBuf) -> Option<DiskInfo> {
        let path_str = path.to_str()?;
        
        let output = std::process::Command::new("diskutil")
            .args(["info", path_str])
            .output()
            .ok()?;

        if !output.status.success() {
            // Fall back to basic info for paths that diskutil doesn't recognize
            return self.get_basic_volume_info(path);
        }

        let info_str = String::from_utf8_lossy(&output.stdout);
        Some(self.parse_diskutil_output(&info_str, path))
    }

    /// Parse diskutil info output into DiskInfo
    fn parse_diskutil_output(&self, output: &str, path: &PathBuf) -> DiskInfo {
        let mut info = DiskInfo::default();
        info.volume_path = Some(path.clone());

        for line in output.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Device Identifier" => info.bsd_name = value.to_string(),
                    "Volume Name" => info.volume_name = Some(value.to_string()),
                    "Volume UUID" => info.volume_uuid = Some(value.to_string()),
                    "Disk Size" | "Total Size" => {
                        info.media_size = parse_size_string(value);
                    }
                    "Removable Media" => info.is_removable = value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("removable"),
                    "Ejectable" => info.is_ejectable = value.eq_ignore_ascii_case("yes"),
                    "Internal" => info.is_internal = value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("internal"),
                    "Protocol" => info.bus_name = Some(value.to_string()),
                    "File System Personality" | "Type (Bundle)" => {
                        info.filesystem_type = Some(value.to_string());
                    }
                    "Media Name" => info.media_name = Some(value.to_string()),
                    "Whole" => info.is_whole_disk = value.eq_ignore_ascii_case("yes"),
                    _ => {}
                }
            }
        }

        // Set volume name from path if not found
        if info.volume_name.is_none() {
            info.volume_name = path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
        }

        info
    }

    /// Get basic volume info when diskutil fails
    fn get_basic_volume_info(&self, path: &PathBuf) -> Option<DiskInfo> {
        if !path.exists() {
            return None;
        }

        let mut info = DiskInfo::default();
        info.volume_path = Some(path.clone());
        info.volume_name = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        // Check if it's a network mount
        if let Ok(output) = std::process::Command::new("mount").output() {
            let mount_info = String::from_utf8_lossy(&output.stdout);
            let path_str = path.to_string_lossy();
            
            for line in mount_info.lines() {
                if line.contains(&*path_str) {
                    if line.contains("smbfs") || line.contains("nfs") || line.contains("afpfs") || line.contains("webdav") {
                        info.is_network = true;
                    }
                    break;
                }
            }
        }

        // Get space info
        if let Ok((total, _free)) = get_disk_space(path) {
            info.media_size = total;
        }

        // Determine if removable based on path
        if path.starts_with("/Volumes") {
            info.is_removable = true;
            info.is_ejectable = true;
            info.is_internal = false;
        }

        Some(info)
    }

    /// Start monitoring for disk events
    pub fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> Result<(), String> {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Store the sender
        if let Ok(mut guard) = self.event_sender.lock() {
            *guard = Some(sender.clone());
        }

        // Initialize known disks
        let disks = self.enumerate_volumes();
        if let Ok(mut known) = self.known_disks.lock() {
            for disk in disks {
                if let Some(ref path) = disk.volume_path {
                    known.insert(path.to_string_lossy().to_string(), disk);
                }
            }
        }

        self.is_monitoring.store(true, Ordering::SeqCst);

        // Start a background thread to poll for changes
        let is_monitoring = self.is_monitoring.clone();
        let event_sender = self.event_sender.clone();
        let known_disks = self.known_disks.clone();

        std::thread::spawn(move || {
            Self::monitor_loop(is_monitoring, event_sender, known_disks);
        });

        Ok(())
    }

    /// Background monitoring loop that polls for disk changes
    fn monitor_loop(
        is_monitoring: Arc<AtomicBool>,
        event_sender: Arc<Mutex<Option<flume::Sender<DeviceEvent>>>>,
        known_disks: Arc<Mutex<HashMap<String, DiskInfo>>>,
    ) {
        let mut next_id = 1000u64; // Start IDs high to avoid conflicts

        while is_monitoring.load(Ordering::SeqCst) {
            // Sleep between polls
            std::thread::sleep(std::time::Duration::from_secs(2));

            if !is_monitoring.load(Ordering::SeqCst) {
                break;
            }

            // Get current volumes
            let volumes_path = PathBuf::from("/Volumes");
            let mut current_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

            if let Ok(entries) = std::fs::read_dir(&volumes_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    // Skip symlinks
                    if std::fs::symlink_metadata(&path)
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false) 
                    {
                        continue;
                    }

                    current_paths.insert(path.to_string_lossy().to_string());
                }
            }

            // Check for new volumes
            let known_paths: Vec<String> = {
                if let Ok(known) = known_disks.lock() {
                    known.keys().cloned().collect()
                } else {
                    continue;
                }
            };

            // Detect new volumes
            for path_str in &current_paths {
                if !known_paths.contains(path_str) && !path_str.contains("Macintosh HD") {
                    let path = PathBuf::from(path_str);
                    
                    // Get volume info
                    let info = Self::get_volume_info_static(&path);
                    
                    if let Some(disk_info) = info {
                        let device = Self::disk_info_to_device(&disk_info, DeviceId::new(next_id));
                        next_id += 1;

                        // Add to known disks
                        if let Ok(mut known) = known_disks.lock() {
                            known.insert(path_str.clone(), disk_info);
                        }

                        // Send connected event
                        if let Ok(guard) = event_sender.lock() {
                            if let Some(ref sender) = *guard {
                                let _ = sender.send(DeviceEvent::Connected(device));
                            }
                        }
                    }
                }
            }

            // Detect removed volumes
            let removed: Vec<String> = known_paths
                .iter()
                .filter(|p| !current_paths.contains(*p) && !p.as_str().eq("/"))
                .cloned()
                .collect();

            for path_str in removed {
                if let Ok(mut known) = known_disks.lock() {
                    if let Some(_disk_info) = known.remove(&path_str) {
                        // Send disconnected event
                        // Note: We use a hash of the path as the ID since we don't track IDs
                        let id = DeviceId::new(hash_path(&path_str));
                        
                        if let Ok(guard) = event_sender.lock() {
                            if let Some(ref sender) = *guard {
                                let _ = sender.send(DeviceEvent::Disconnected(id));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Static version of get_volume_info for use in monitor thread
    fn get_volume_info_static(path: &PathBuf) -> Option<DiskInfo> {
        let path_str = path.to_str()?;
        
        let output = std::process::Command::new("diskutil")
            .args(["info", path_str])
            .output()
            .ok()?;

        if !output.status.success() {
            return Self::get_basic_volume_info_static(path);
        }

        let info_str = String::from_utf8_lossy(&output.stdout);
        Some(Self::parse_diskutil_output_static(&info_str, path))
    }

    fn parse_diskutil_output_static(output: &str, path: &PathBuf) -> DiskInfo {
        let mut info = DiskInfo::default();
        info.volume_path = Some(path.clone());

        for line in output.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Device Identifier" => info.bsd_name = value.to_string(),
                    "Volume Name" => info.volume_name = Some(value.to_string()),
                    "Volume UUID" => info.volume_uuid = Some(value.to_string()),
                    "Disk Size" | "Total Size" => {
                        info.media_size = parse_size_string(value);
                    }
                    "Removable Media" => info.is_removable = value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("removable"),
                    "Ejectable" => info.is_ejectable = value.eq_ignore_ascii_case("yes"),
                    "Internal" => info.is_internal = value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("internal"),
                    "Protocol" => info.bus_name = Some(value.to_string()),
                    "File System Personality" | "Type (Bundle)" => {
                        info.filesystem_type = Some(value.to_string());
                    }
                    "Media Name" => info.media_name = Some(value.to_string()),
                    "Whole" => info.is_whole_disk = value.eq_ignore_ascii_case("yes"),
                    _ => {}
                }
            }
        }

        if info.volume_name.is_none() {
            info.volume_name = path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
        }

        info
    }

    fn get_basic_volume_info_static(path: &PathBuf) -> Option<DiskInfo> {
        if !path.exists() {
            return None;
        }

        let mut info = DiskInfo::default();
        info.volume_path = Some(path.clone());
        info.volume_name = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        if let Ok(output) = std::process::Command::new("mount").output() {
            let mount_info = String::from_utf8_lossy(&output.stdout);
            let path_str = path.to_string_lossy();
            
            for line in mount_info.lines() {
                if line.contains(&*path_str) {
                    if line.contains("smbfs") || line.contains("nfs") || line.contains("afpfs") || line.contains("webdav") {
                        info.is_network = true;
                    }
                    break;
                }
            }
        }

        if let Ok((total, _free)) = get_disk_space(path) {
            info.media_size = total;
        }

        if path.starts_with("/Volumes") {
            info.is_removable = true;
            info.is_ejectable = true;
            info.is_internal = false;
        }

        Some(info)
    }

    /// Convert DiskInfo to Device
    fn disk_info_to_device(info: &DiskInfo, id: DeviceId) -> Device {
        let name = info.volume_name.clone()
            .unwrap_or_else(|| "Unknown Volume".to_string());
        
        let path = info.volume_path.clone()
            .unwrap_or_else(|| PathBuf::from("/"));

        let device_type = info.device_type();
        let is_removable = info.is_removable || info.is_ejectable;

        let mut device = Device::new(id, name, path.clone(), device_type)
            .with_removable(is_removable);

        if let Ok((total, free)) = get_disk_space(&path) {
            device = device.with_space(total, free);
        } else if info.media_size > 0 {
            device = device.with_space(info.media_size, 0);
        }

        device
    }

    /// Stop monitoring for disk events
    pub fn stop_monitoring(&self) {
        self.is_monitoring.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.event_sender.lock() {
            *guard = None;
        }
    }

    /// Check if monitoring is active
    pub fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "macos")]
impl Default for MacOSDiskMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a size string like "500.1 GB (500107862016 Bytes)" into bytes
fn parse_size_string(s: &str) -> u64 {
    // Try to extract bytes from parentheses first
    if let Some(start) = s.find('(') {
        if let Some(end) = s.find(" Bytes") {
            let bytes_str = &s[start + 1..end];
            if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
                return bytes;
            }
        }
    }

    // Fall back to parsing the human-readable size
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(value) = parts[0].parse::<f64>() {
            let multiplier = match parts[1].to_uppercase().as_str() {
                "B" | "BYTES" => 1u64,
                "KB" => 1024,
                "MB" => 1024 * 1024,
                "GB" => 1024 * 1024 * 1024,
                "TB" => 1024 * 1024 * 1024 * 1024,
                _ => 1,
            };
            return (value * multiplier as f64) as u64;
        }
    }

    0
}

/// Hash a path string to create a stable device ID
fn hash_path(path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

impl DeviceMonitor {
    /// Enumerate devices on macOS by scanning /Volumes
    #[cfg(target_os = "macos")]
    pub fn enumerate_macos_devices(&mut self) {
        let monitor = MacOSDiskMonitor::new();
        let disk_infos = monitor.enumerate_volumes();

        for info in disk_infos {
            let name = info.volume_name.clone()
                .unwrap_or_else(|| "Unknown Volume".to_string());
            
            let path = info.volume_path.clone()
                .unwrap_or_else(|| PathBuf::from("/"));

            let device_type = info.device_type();
            let is_removable = info.is_removable || info.is_ejectable;

            let mut device = Device::new(DeviceId::new(0), name, path.clone(), device_type)
                .with_removable(is_removable);

            if let Ok((total, free)) = get_disk_space(&path) {
                device = device.with_space(total, free);
            }

            device = device.with_read_only(is_volume_read_only(&path));

            self.add_device(device);
        }
    }

    /// Eject a device on macOS
    #[cfg(target_os = "macos")]
    pub fn eject(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self
            .get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;

        if !device.is_removable {
            return Err(super::device_monitor::DeviceError::EjectFailed(
                "Device is not removable".to_string(),
            ));
        }

        let path = device.path.clone();

        let output = std::process::Command::new("diskutil")
            .args(["eject", path.to_str().unwrap_or("")])
            .output()?;

        if output.status.success() {
            self.remove_device(id);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(super::device_monitor::DeviceError::EjectFailed(
                error.to_string(),
            ))
        }
    }

    /// Unmount a device on macOS
    #[cfg(target_os = "macos")]
    pub fn unmount(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self
            .get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;

        let path = device.path.clone();

        let output = std::process::Command::new("diskutil")
            .args(["unmount", path.to_str().unwrap_or("")])
            .output()?;

        if output.status.success() {
            self.remove_device(id);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(super::device_monitor::DeviceError::UnmountFailed(
                error.to_string(),
            ))
        }
    }
}

/// Check if a volume is read-only
#[cfg(target_os = "macos")]
fn is_volume_read_only(path: &PathBuf) -> bool {
    use std::os::unix::fs::MetadataExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.mode();
        return (mode & 0o200) == 0;
    }
    false
}

/// Detect if a disk image is mounted at the given path
#[cfg(target_os = "macos")]
pub fn is_disk_image(path: &PathBuf) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    
    // Check path for common disk image indicators
    if path_str.contains(".dmg") || path_str.contains("disk image") {
        return true;
    }

    // Check mount info for disk image mounts
    if let Ok(output) = std::process::Command::new("hdiutil")
        .args(["info", "-plist"])
        .output()
    {
        let info = String::from_utf8_lossy(&output.stdout);
        if info.contains(&*path.to_string_lossy()) {
            return true;
        }
    }

    false
}

/// Get list of mounted disk images
#[cfg(target_os = "macos")]
pub fn get_mounted_disk_images() -> Vec<PathBuf> {
    let mut images = Vec::new();

    if let Ok(output) = std::process::Command::new("hdiutil")
        .args(["info"])
        .output()
    {
        let info = String::from_utf8_lossy(&output.stdout);
        let mut current_mount: Option<PathBuf> = None;

        for line in info.lines() {
            let line = line.trim();
            
            if line.starts_with("/dev/disk") {
                // New disk entry
                current_mount = None;
            } else if line.starts_with("/Volumes/") || line.starts_with("/private/") {
                current_mount = Some(PathBuf::from(line));
            }

            if let Some(ref mount) = current_mount {
                if !images.contains(mount) {
                    images.push(mount.clone());
                }
            }
        }
    }

    images
}
