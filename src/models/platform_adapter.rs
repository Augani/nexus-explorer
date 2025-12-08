use std::path::{Path, PathBuf};
use thiserror::Error;

use super::device_monitor::{Device, DeviceEvent, DeviceId, DeviceResult, SmartData};

/// Errors specific to platform adapter operations
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Device not found: {0:?}")]
    DeviceNotFound(DeviceId),

    #[error("Device busy: {0}")]
    DeviceBusy(String),

    #[error("Eject failed: {0}")]
    EjectFailed(String),

    #[error("Format failed: {0}")]
    FormatFailed(String),

    #[error("Mount failed: {0}")]
    MountFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PlatformResult<T> = std::result::Result<T, PlatformError>;

/// Supported file system types for formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileSystemType {
    // Cross-platform
    Fat32,
    ExFat,
    // Windows
    Ntfs,
    ReFS,
    // macOS
    Apfs,
    HfsPlus,
    // Linux
    Ext4,
    Btrfs,
    Xfs,
}

impl FileSystemType {
    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            FileSystemType::Fat32 => "FAT32",
            FileSystemType::ExFat => "exFAT",
            FileSystemType::Ntfs => "NTFS",
            FileSystemType::ReFS => "ReFS",
            FileSystemType::Apfs => "APFS",
            FileSystemType::HfsPlus => "HFS+",
            FileSystemType::Ext4 => "ext4",
            FileSystemType::Btrfs => "Btrfs",
            FileSystemType::Xfs => "XFS",
        }
    }


    /// Get compatibility information for this filesystem
    pub fn compatibility_info(&self) -> &'static str {
        match self {
            FileSystemType::Fat32 => "Compatible with Windows, macOS, Linux. Max file size: 4GB",
            FileSystemType::ExFat => "Compatible with Windows, macOS, Linux. No file size limit",
            FileSystemType::Ntfs => "Native Windows. Read-only on macOS, read-write on Linux with ntfs-3g",
            FileSystemType::ReFS => "Windows Server only. Not compatible with macOS or Linux",
            FileSystemType::Apfs => "Native macOS. Not compatible with Windows or Linux",
            FileSystemType::HfsPlus => "Legacy macOS. Read-only on Windows with drivers, limited Linux support",
            FileSystemType::Ext4 => "Native Linux. Not compatible with Windows or macOS without drivers",
            FileSystemType::Btrfs => "Linux only. Advanced features like snapshots and compression",
            FileSystemType::Xfs => "Linux only. High-performance for large files",
        }
    }

    /// Check if this filesystem is available on the current platform
    pub fn is_available_on_current_platform(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            matches!(
                self,
                FileSystemType::Fat32
                    | FileSystemType::ExFat
                    | FileSystemType::Ntfs
                    | FileSystemType::ReFS
            )
        }
        #[cfg(target_os = "macos")]
        {
            matches!(
                self,
                FileSystemType::Fat32
                    | FileSystemType::ExFat
                    | FileSystemType::Apfs
                    | FileSystemType::HfsPlus
            )
        }
        #[cfg(target_os = "linux")]
        {
            matches!(
                self,
                FileSystemType::Fat32
                    | FileSystemType::ExFat
                    | FileSystemType::Ext4
                    | FileSystemType::Btrfs
                    | FileSystemType::Xfs
            )
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            false
        }
    }
}

/// Options for formatting a device
#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub filesystem: FileSystemType,
    pub label: String,
    pub quick_format: bool,
    pub enable_compression: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            filesystem: FileSystemType::ExFat,
            label: String::new(),
            quick_format: true,
            enable_compression: false,
        }
    }
}

/// Context menu item for platform-specific actions
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub enabled: bool,
    pub separator_after: bool,
}

impl ContextMenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            enabled: true,
            separator_after: false,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn with_separator(mut self) -> Self {
        self.separator_after = true;
        self
    }
}

/// Platform-specific actions that can be executed
#[derive(Debug, Clone)]
pub enum PlatformAction {
    /// Eject a removable device
    Eject(DeviceId),
    /// Format a device with options
    Format(DeviceId, FormatOptions),
    /// Mount a disk image
    MountImage(PathBuf),
    /// Unmount a disk image or device
    Unmount(PathBuf),
    /// Open terminal at path (Linux)
    OpenTerminal(PathBuf),
    /// Pin to Quick Access (Windows)
    PinToQuickAccess(PathBuf),
    /// Scan with antivirus (Windows)
    ScanWithDefender(PathBuf),
    /// Share via AirDrop (macOS)
    ShareAirDrop(Vec<PathBuf>),
    /// Execute Quick Action (macOS)
    QuickAction(String, Vec<PathBuf>),
    /// Custom platform action
    Custom(String, Vec<String>),
}


/// Platform abstraction trait for device operations
/// 
/// This trait provides a unified interface for device management across
/// Windows, macOS, and Linux platforms. Each platform implements this
/// trait with platform-specific APIs.
pub trait PlatformAdapter: Send + Sync {
    /// Enumerate all connected storage devices
    /// 
    /// Returns a list of all currently connected and mounted storage devices.
    /// Each device includes metadata such as name, path, capacity, and type.
    fn enumerate_devices(&self) -> Vec<Device>;

    /// Start monitoring for device changes (connect/disconnect)
    /// 
    /// Begins listening for device hotplug events. When a device is connected
    /// or disconnected, a DeviceEvent is sent through the provided channel.
    /// 
    /// # Arguments
    /// * `sender` - Channel sender for device events
    /// 
    /// # Returns
    /// * `Ok(())` if monitoring started successfully
    /// * `Err(PlatformError)` if monitoring could not be started
    fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> PlatformResult<()>;

    /// Stop monitoring for device changes
    /// 
    /// Stops the device change listener. After calling this, no more
    /// DeviceEvents will be sent through the channel.
    fn stop_monitoring(&self) -> PlatformResult<()>;

    /// Safely eject a removable device
    /// 
    /// Flushes all pending writes and unmounts the device, making it
    /// safe for physical removal.
    /// 
    /// # Arguments
    /// * `device_id` - ID of the device to eject
    /// 
    /// # Returns
    /// * `Ok(())` if ejection was successful
    /// * `Err(PlatformError)` if ejection failed
    fn eject_device(&self, device_id: DeviceId) -> PlatformResult<()>;

    /// Format a device with the specified filesystem
    /// 
    /// Formats the device, erasing all data. This operation requires
    /// appropriate permissions and the device must not be in use.
    /// 
    /// # Arguments
    /// * `device_id` - ID of the device to format
    /// * `options` - Format options including filesystem type and label
    /// 
    /// # Returns
    /// * `Ok(())` if formatting was successful
    /// * `Err(PlatformError)` if formatting failed
    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()>;

    /// Get SMART health data for a device
    /// 
    /// Retrieves Self-Monitoring, Analysis and Reporting Technology data
    /// for storage devices that support it.
    /// 
    /// # Arguments
    /// * `device_id` - ID of the device to query
    /// 
    /// # Returns
    /// * `Ok(Some(SmartData))` if SMART data is available
    /// * `Ok(None)` if the device doesn't support SMART
    /// * `Err(PlatformError)` if query failed
    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>>;

    /// Mount a disk image file
    /// 
    /// Mounts an ISO, DMG, VHD, or other disk image file and returns
    /// the path where it was mounted.
    /// 
    /// # Arguments
    /// * `path` - Path to the disk image file
    /// 
    /// # Returns
    /// * `Ok(PathBuf)` - Mount point path
    /// * `Err(PlatformError)` if mounting failed
    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf>;

    /// Unmount a disk image
    /// 
    /// Unmounts a previously mounted disk image.
    /// 
    /// # Arguments
    /// * `mount_point` - Path where the image is mounted
    /// 
    /// # Returns
    /// * `Ok(())` if unmounting was successful
    /// * `Err(PlatformError)` if unmounting failed
    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()>;

    /// Get platform-specific context menu items for paths
    /// 
    /// Returns a list of context menu items that are available for
    /// the given paths on the current platform.
    /// 
    /// # Arguments
    /// * `paths` - Paths to get context menu items for
    /// 
    /// # Returns
    /// List of available context menu items
    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem>;

    /// Execute a platform-specific action
    /// 
    /// Executes a platform action such as ejecting a device, opening
    /// a terminal, or sharing via AirDrop.
    /// 
    /// # Arguments
    /// * `action` - The action to execute
    /// 
    /// # Returns
    /// * `Ok(())` if the action was successful
    /// * `Err(PlatformError)` if the action failed
    fn execute_action(&self, action: PlatformAction) -> PlatformResult<()>;

    /// Get available filesystem types for formatting on this platform
    /// 
    /// Returns a list of filesystem types that can be used for formatting
    /// devices on the current platform.
    fn available_filesystems(&self) -> Vec<FileSystemType>;

    /// Check if the platform adapter is currently monitoring
    fn is_monitoring(&self) -> bool;
}

// Platform-specific adapter implementations are defined below
// and in separate files for complex platform-specific logic

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Windows platform adapter using WMI for device enumeration
#[cfg(target_os = "windows")]
pub struct WindowsAdapter {
    is_monitoring: Arc<AtomicBool>,
    wmi_enumerator: super::windows_wmi::WmiDeviceEnumerator,
    notification_monitor: std::sync::Mutex<super::windows_device_notifications::WindowsDeviceNotificationMonitor>,
}

#[cfg(target_os = "windows")]
impl WindowsAdapter {
    pub fn new() -> Self {
        Self {
            is_monitoring: Arc::new(AtomicBool::new(false)),
            wmi_enumerator: super::windows_wmi::WmiDeviceEnumerator::new(),
            notification_monitor: std::sync::Mutex::new(
                super::windows_device_notifications::WindowsDeviceNotificationMonitor::new()
            ),
        }
    }

    /// Get extended device information for a drive letter using WMI
    pub fn get_device_info(&self, drive_letter: &str) -> Option<super::windows_wmi::WmiDeviceInfo> {
        self.wmi_enumerator.get_device_info(drive_letter)
    }
}

#[cfg(target_os = "windows")]
impl Default for WindowsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl PlatformAdapter for WindowsAdapter {
    fn enumerate_devices(&self) -> Vec<Device> {
        // Try WMI-based enumeration first for richer metadata
        let wmi_devices = self.wmi_enumerator.enumerate_devices();
        if !wmi_devices.is_empty() {
            return wmi_devices;
        }

        // Fall back to direct Win32 API enumeration
        let mut devices = Vec::new();
        
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive_path);

            if !path.exists() {
                continue;
            }

            let device_type = detect_windows_drive_type(&path);
            let name = get_windows_volume_name(&path)
                .unwrap_or_else(|| format_drive_name_by_type(letter as char, &device_type));

            let is_removable = matches!(
                device_type,
                super::device_monitor::DeviceType::UsbDrive 
                    | super::device_monitor::DeviceType::ExternalDrive 
                    | super::device_monitor::DeviceType::OpticalDrive
            );

            let mut device = Device::new(
                DeviceId::new(letter as u64),
                name,
                path.clone(),
                device_type,
            )
            .with_removable(is_removable);

            if let Ok((total, free)) = super::device_monitor::get_disk_space(&path) {
                device = device.with_space(total, free);
            }

            devices.push(device);
        }
        
        devices
    }

    fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> PlatformResult<()> {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut monitor = self.notification_monitor.lock()
            .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
        
        monitor.start(sender)
            .map_err(|e| PlatformError::PlatformNotSupported(e))?;
        
        self.is_monitoring.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop_monitoring(&self) -> PlatformResult<()> {
        if let Ok(mut monitor) = self.notification_monitor.lock() {
            monitor.stop();
        }
        self.is_monitoring.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn eject_device(&self, device_id: DeviceId) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Eject not yet implemented".to_string()))
    }

    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Format not yet implemented".to_string()))
    }

    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>> {
        Ok(None)
    }

    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf> {
        Err(PlatformError::PlatformNotSupported("Mount image not yet implemented".to_string()))
    }

    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Unmount image not yet implemented".to_string()))
    }

    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem> {
        vec![
            ContextMenuItem::new("pin_quick_access", "Pin to Quick Access").with_icon("pin"),
            ContextMenuItem::new("scan_defender", "Scan with Windows Defender").with_icon("shield"),
        ]
    }

    fn execute_action(&self, action: PlatformAction) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Action not yet implemented".to_string()))
    }

    fn available_filesystems(&self) -> Vec<FileSystemType> {
        vec![
            FileSystemType::Ntfs,
            FileSystemType::Fat32,
            FileSystemType::ExFat,
            FileSystemType::ReFS,
        ]
    }

    fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }
}

/// Format a drive name based on its type
#[cfg(target_os = "windows")]
fn format_drive_name_by_type(letter: char, device_type: &super::device_monitor::DeviceType) -> String {
    let type_name = match device_type {
        super::device_monitor::DeviceType::InternalDrive => "Local Disk",
        super::device_monitor::DeviceType::UsbDrive => "USB Drive",
        super::device_monitor::DeviceType::ExternalDrive => "External Drive",
        super::device_monitor::DeviceType::NetworkDrive => "Network Drive",
        super::device_monitor::DeviceType::OpticalDrive => "CD/DVD Drive",
        super::device_monitor::DeviceType::DiskImage => "RAM Disk",
        _ => "Drive",
    };
    format!("{} ({}:)", type_name, letter)
}

#[cfg(target_os = "windows")]
fn detect_windows_drive_type(path: &PathBuf) -> super::device_monitor::DeviceType {
    use std::os::windows::ffi::OsStrExt;

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;
    const DRIVE_RAMDISK: u32 = 6;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let drive_type = windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide_path.as_ptr());

        match drive_type {
            DRIVE_REMOVABLE => super::device_monitor::DeviceType::UsbDrive,
            DRIVE_FIXED => super::device_monitor::DeviceType::InternalDrive,
            DRIVE_REMOTE => super::device_monitor::DeviceType::NetworkDrive,
            DRIVE_CDROM => super::device_monitor::DeviceType::OpticalDrive,
            DRIVE_RAMDISK => super::device_monitor::DeviceType::DiskImage,
            _ => super::device_monitor::DeviceType::ExternalDrive,
        }
    }
}

#[cfg(target_os = "windows")]
fn get_windows_volume_name(path: &PathBuf) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut volume_name: [u16; 261] = [0; 261];
    let mut fs_name: [u16; 261] = [0; 261];
    let mut serial_number: u32 = 0;
    let mut max_component_length: u32 = 0;
    let mut fs_flags: u32 = 0;

    unsafe {
        if windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
            wide_path.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut serial_number,
            &mut max_component_length,
            &mut fs_flags,
            fs_name.as_mut_ptr(),
            fs_name.len() as u32,
        ) != 0
        {
            let len = volume_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(volume_name.len());
            let name = String::from_utf16_lossy(&volume_name[..len]);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// macOS platform adapter using DiskArbitration framework
#[cfg(target_os = "macos")]
pub struct MacOSAdapter {
    is_monitoring: Arc<AtomicBool>,
    disk_monitor: super::device_monitor_macos::MacOSDiskMonitor,
    devices_cache: std::sync::Mutex<Vec<Device>>,
}

#[cfg(target_os = "macos")]
impl MacOSAdapter {
    pub fn new() -> Self {
        Self {
            is_monitoring: Arc::new(AtomicBool::new(false)),
            disk_monitor: super::device_monitor_macos::MacOSDiskMonitor::new(),
            devices_cache: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Check if a path is a disk image mount
    pub fn is_disk_image(&self, path: &Path) -> bool {
        super::device_monitor_macos::is_disk_image(&path.to_path_buf())
    }

    /// Get all mounted disk images
    pub fn get_mounted_disk_images(&self) -> Vec<PathBuf> {
        super::device_monitor_macos::get_mounted_disk_images()
    }
}

#[cfg(target_os = "macos")]
impl Default for MacOSAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
impl PlatformAdapter for MacOSAdapter {
    fn enumerate_devices(&self) -> Vec<Device> {
        let disk_infos = self.disk_monitor.enumerate_volumes();
        let mut devices = Vec::new();
        let mut next_id = 1u64;

        for info in disk_infos {
            let name = info.volume_name.clone()
                .unwrap_or_else(|| "Unknown Volume".to_string());
            
            let path = info.volume_path.clone()
                .unwrap_or_else(|| PathBuf::from("/"));

            let device_type = info.device_type();
            let is_removable = info.is_removable || info.is_ejectable;

            let mut device = Device::new(
                DeviceId::new(next_id),
                name,
                path.clone(),
                device_type,
            )
            .with_removable(is_removable);
            next_id += 1;

            if let Ok((total, free)) = super::device_monitor::get_disk_space(&path) {
                device = device.with_space(total, free);
            } else if info.media_size > 0 {
                device = device.with_space(info.media_size, 0);
            }

            devices.push(device);
        }

        // Cache devices for later lookup
        if let Ok(mut cache) = self.devices_cache.lock() {
            *cache = devices.clone();
        }

        devices
    }

    fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> PlatformResult<()> {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.disk_monitor.start_monitoring(sender)
            .map_err(|e| PlatformError::PlatformNotSupported(e))?;
        
        self.is_monitoring.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop_monitoring(&self) -> PlatformResult<()> {
        self.disk_monitor.stop_monitoring();
        self.is_monitoring.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn eject_device(&self, device_id: DeviceId) -> PlatformResult<()> {
        // Find the device path from cache
        let path = {
            let cache = self.devices_cache.lock()
                .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
            
            cache.iter()
                .find(|d| d.id == device_id)
                .map(|d| d.path.clone())
                .ok_or(PlatformError::DeviceNotFound(device_id))?
        };

        // Use diskutil to eject
        let output = std::process::Command::new("diskutil")
            .args(["eject", path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| PlatformError::Io(e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::EjectFailed(error.to_string()))
        }
    }

    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()> {
        // Find the device path from cache
        let path = {
            let cache = self.devices_cache.lock()
                .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
            
            cache.iter()
                .find(|d| d.id == device_id)
                .map(|d| d.path.clone())
                .ok_or(PlatformError::DeviceNotFound(device_id))?
        };

        // Map filesystem type to diskutil format
        let fs_format = match options.filesystem {
            FileSystemType::Apfs => "APFS",
            FileSystemType::HfsPlus => "HFS+",
            FileSystemType::Fat32 => "FAT32",
            FileSystemType::ExFat => "ExFAT",
            _ => return Err(PlatformError::FormatFailed(
                format!("Filesystem {:?} not supported on macOS", options.filesystem)
            )),
        };

        let label = if options.label.is_empty() {
            "Untitled".to_string()
        } else {
            options.label
        };

        // Use diskutil to format
        let output = std::process::Command::new("diskutil")
            .args(["eraseDisk", fs_format, &label, path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| PlatformError::Io(e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::FormatFailed(error.to_string()))
        }
    }

    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>> {
        // SMART data on macOS requires smartmontools or system_profiler
        // For now, return None as it requires additional setup
        Ok(None)
    }

    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf> {
        let output = std::process::Command::new("hdiutil")
            .args(["attach", "-plist", path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| PlatformError::Io(e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::MountFailed(error.to_string()));
        }

        // Parse plist output to find mount point
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Simple parsing - look for mount-point in the output
        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("/Volumes/") {
                return Ok(PathBuf::from(line));
            }
        }

        // If we can't parse the plist, try to find the mount point by name
        let image_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("disk image");
        
        let mount_point = PathBuf::from("/Volumes").join(image_name);
        if mount_point.exists() {
            return Ok(mount_point);
        }

        Err(PlatformError::MountFailed("Could not determine mount point".to_string()))
    }

    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()> {
        let output = std::process::Command::new("hdiutil")
            .args(["detach", mount_point.to_str().unwrap_or("")])
            .output()
            .map_err(|e| PlatformError::Io(e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::MountFailed(error.to_string()))
        }
    }

    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem> {
        let mut items = vec![
            ContextMenuItem::new("quick_look", "Quick Look").with_icon("eye"),
        ];

        // Add AirDrop if available
        items.push(ContextMenuItem::new("airdrop", "Share via AirDrop").with_icon("share"));

        // Add Quick Actions for supported file types
        if paths.len() == 1 {
            let path = &paths[0];
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "pdf" => {
                        items.push(ContextMenuItem::new("create_pdf", "Create PDF").with_icon("file"));
                    }
                    "jpg" | "jpeg" | "png" | "heic" => {
                        items.push(ContextMenuItem::new("rotate_image", "Rotate Image").with_icon("rotate"));
                        items.push(ContextMenuItem::new("markup", "Markup").with_icon("pen"));
                    }
                    _ => {}
                }
            }
        }

        items
    }

    fn execute_action(&self, action: PlatformAction) -> PlatformResult<()> {
        match action {
            PlatformAction::Eject(device_id) => self.eject_device(device_id),
            PlatformAction::MountImage(path) => {
                self.mount_image(&path)?;
                Ok(())
            }
            PlatformAction::Unmount(path) => self.unmount_image(&path),
            PlatformAction::ShareAirDrop(paths) => {
                // Use NSWorkspace to share via AirDrop
                // This requires Objective-C bridging which is complex
                // For now, open the share sheet via AppleScript
                let paths_str: Vec<String> = paths.iter()
                    .filter_map(|p| p.to_str())
                    .map(|s| format!("POSIX file \"{}\"", s))
                    .collect();
                
                if paths_str.is_empty() {
                    return Err(PlatformError::PlatformNotSupported("No valid paths".to_string()));
                }

                let script = format!(
                    "tell application \"Finder\" to activate\n\
                     tell application \"System Events\"\n\
                         keystroke \"r\" using {{command down, shift down}}\n\
                     end tell"
                );

                let _ = std::process::Command::new("osascript")
                    .args(["-e", &script])
                    .output();

                Ok(())
            }
            PlatformAction::QuickAction(action_name, paths) => {
                // Execute Quick Action via Automator
                let paths_str: String = paths.iter()
                    .filter_map(|p| p.to_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let _ = std::process::Command::new("automator")
                    .args(["-i", &paths_str, &action_name])
                    .output();

                Ok(())
            }
            _ => Err(PlatformError::PlatformNotSupported(
                format!("Action {:?} not supported on macOS", action)
            )),
        }
    }

    fn available_filesystems(&self) -> Vec<FileSystemType> {
        vec![
            FileSystemType::Apfs,
            FileSystemType::HfsPlus,
            FileSystemType::Fat32,
            FileSystemType::ExFat,
        ]
    }

    fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }
}

/// Linux platform adapter
#[cfg(target_os = "linux")]
pub struct LinuxAdapter {
    is_monitoring: Arc<AtomicBool>,
    event_sender: Option<flume::Sender<DeviceEvent>>,
}

#[cfg(target_os = "linux")]
impl LinuxAdapter {
    pub fn new() -> Self {
        Self {
            is_monitoring: Arc::new(AtomicBool::new(false)),
            event_sender: None,
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for LinuxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl PlatformAdapter for LinuxAdapter {
    fn enumerate_devices(&self) -> Vec<Device> {
        let mut devices = Vec::new();
        let mut next_id = 1u64;
        
        // Add root filesystem
        let root_path = PathBuf::from("/");
        if let Ok((total, free)) = super::device_monitor::get_disk_space(&root_path) {
            let root_device = Device::new(
                DeviceId::new(next_id),
                "Root".to_string(),
                root_path,
                super::device_monitor::DeviceType::InternalDrive,
            )
            .with_space(total, free)
            .with_removable(false);
            next_id += 1;
            devices.push(root_device);
        }

        // Parse /proc/mounts to find mounted filesystems
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }

                let device_path = parts[0];
                let mount_point = parts[1];
                let fs_type = parts[2];

                // Skip virtual filesystems and system mounts
                if should_skip_linux_mount(device_path, mount_point, fs_type) {
                    continue;
                }

                let path = PathBuf::from(mount_point);
                let device_type = detect_linux_device_type(device_path, fs_type, mount_point);
                let name = get_linux_device_name(&path, device_path);

                let is_removable = is_linux_removable(device_path);

                let mut device = Device::new(
                    DeviceId::new(next_id),
                    name,
                    path.clone(),
                    device_type,
                )
                .with_removable(is_removable);
                next_id += 1;

                if let Ok((total, free)) = super::device_monitor::get_disk_space(&path) {
                    device = device.with_space(total, free);
                }

                devices.push(device);
            }
        }
        
        devices
    }

    fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> PlatformResult<()> {
        self.is_monitoring.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop_monitoring(&self) -> PlatformResult<()> {
        self.is_monitoring.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn eject_device(&self, device_id: DeviceId) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Eject not yet implemented".to_string()))
    }

    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Format not yet implemented".to_string()))
    }

    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>> {
        Ok(None)
    }

    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf> {
        Err(PlatformError::PlatformNotSupported("Mount image not yet implemented".to_string()))
    }

    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Unmount image not yet implemented".to_string()))
    }

    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem> {
        vec![
            ContextMenuItem::new("open_terminal", "Open Terminal Here").with_icon("terminal"),
            ContextMenuItem::new("run_as_root", "Run as Root").with_icon("shield"),
        ]
    }

    fn execute_action(&self, action: PlatformAction) -> PlatformResult<()> {
        Err(PlatformError::PlatformNotSupported("Action not yet implemented".to_string()))
    }

    fn available_filesystems(&self) -> Vec<FileSystemType> {
        vec![
            FileSystemType::Ext4,
            FileSystemType::Btrfs,
            FileSystemType::Xfs,
            FileSystemType::Fat32,
            FileSystemType::ExFat,
        ]
    }

    fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "linux")]
fn should_skip_linux_mount(device: &str, mount_point: &str, fs_type: &str) -> bool {
    let virtual_fs = [
        "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "securityfs",
        "cgroup", "cgroup2", "pstore", "debugfs", "hugetlbfs", "mqueue",
        "fusectl", "configfs", "binfmt_misc", "autofs", "efivarfs",
        "tracefs", "bpf", "overlay", "squashfs",
    ];

    if virtual_fs.contains(&fs_type) {
        return true;
    }

    // Skip snap mounts
    if mount_point.starts_with("/snap") {
        return true;
    }

    false
}

#[cfg(target_os = "linux")]
fn detect_linux_device_type(device: &str, fs_type: &str, mount_point: &str) -> super::device_monitor::DeviceType {
    // Network filesystems
    if ["nfs", "nfs4", "cifs", "smbfs", "sshfs", "fuse.sshfs"].contains(&fs_type) {
        return super::device_monitor::DeviceType::NetworkDrive;
    }

    if device.contains("sr") || device.contains("cdrom") || fs_type == "iso9660" {
        return super::device_monitor::DeviceType::OpticalDrive;
    }

    if device.starts_with("/dev/sd") && is_linux_removable(device) {
        return super::device_monitor::DeviceType::UsbDrive;
    }

    if mount_point.starts_with("/media") || mount_point.starts_with("/mnt") {
        return super::device_monitor::DeviceType::ExternalDrive;
    }

    super::device_monitor::DeviceType::InternalDrive
}

#[cfg(target_os = "linux")]
fn get_linux_device_name(path: &PathBuf, device: &str) -> String {
    if let Ok(entries) = std::fs::read_dir("/dev/disk/by-label") {
        for entry in entries.flatten() {
            if let Ok(target) = std::fs::read_link(entry.path()) {
                let target_str = target.to_string_lossy();
                if device.ends_with(&*target_str)
                    || target_str.ends_with(device.trim_start_matches("/dev/"))
                {
                    if let Some(name) = entry.file_name().to_str() {
                        return name.replace("\\x20", " ");
                    }
                }
            }
        }
    }

    // Fall back to mount point name
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

#[cfg(target_os = "linux")]
fn is_linux_removable(device: &str) -> bool {
    let base_device = device
        .trim_start_matches("/dev/")
        .trim_end_matches(char::is_numeric);

    let removable_path = format!("/sys/block/{}/removable", base_device);
    if let Ok(content) = std::fs::read_to_string(&removable_path) {
        return content.trim() == "1";
    }

    false
}

/// Get the appropriate platform adapter for the current OS
#[cfg(target_os = "windows")]
pub fn get_platform_adapter() -> Box<dyn PlatformAdapter> {
    Box::new(WindowsAdapter::new())
}

/// Get the appropriate platform adapter for the current OS
#[cfg(target_os = "macos")]
pub fn get_platform_adapter() -> Box<dyn PlatformAdapter> {
    Box::new(MacOSAdapter::new())
}

/// Get the appropriate platform adapter for the current OS
#[cfg(target_os = "linux")]
pub fn get_platform_adapter() -> Box<dyn PlatformAdapter> {
    Box::new(LinuxAdapter::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_type_display_names() {
        assert_eq!(FileSystemType::Fat32.display_name(), "FAT32");
        assert_eq!(FileSystemType::ExFat.display_name(), "exFAT");
        assert_eq!(FileSystemType::Ntfs.display_name(), "NTFS");
        assert_eq!(FileSystemType::Apfs.display_name(), "APFS");
        assert_eq!(FileSystemType::Ext4.display_name(), "ext4");
    }

    #[test]
    fn test_filesystem_compatibility_info_not_empty() {
        let filesystems = [
            FileSystemType::Fat32,
            FileSystemType::ExFat,
            FileSystemType::Ntfs,
            FileSystemType::ReFS,
            FileSystemType::Apfs,
            FileSystemType::HfsPlus,
            FileSystemType::Ext4,
            FileSystemType::Btrfs,
            FileSystemType::Xfs,
        ];

        for fs in filesystems {
            let info = fs.compatibility_info();
            assert!(!info.is_empty(), "Compatibility info for {:?} should not be empty", fs);
        }
    }

    #[test]
    fn test_format_options_default() {
        let options = FormatOptions::default();
        assert_eq!(options.filesystem, FileSystemType::ExFat);
        assert!(options.label.is_empty());
        assert!(options.quick_format);
        assert!(!options.enable_compression);
    }

    #[test]
    fn test_context_menu_item_builder() {
        let item = ContextMenuItem::new("eject", "Eject")
            .with_icon("eject-icon")
            .with_separator();

        assert_eq!(item.id, "eject");
        assert_eq!(item.label, "Eject");
        assert_eq!(item.icon, Some("eject-icon".to_string()));
        assert!(item.enabled);
        assert!(item.separator_after);
    }

    #[test]
    fn test_context_menu_item_disabled() {
        let item = ContextMenuItem::new("format", "Format").disabled();
        assert!(!item.enabled);
    }

    #[test]
    fn test_platform_adapter_enumerate_devices() {
        let adapter = get_platform_adapter();
        let devices = adapter.enumerate_devices();
        
        // Should have at least one device (root filesystem)
        assert!(!devices.is_empty(), "Should detect at least one device");
        
        // All devices should have valid metadata
        for device in &devices {
            assert!(!device.name.is_empty(), "Device name should not be empty");
            assert!(device.path.exists() || device.path.to_string_lossy().starts_with("\\\\"), 
                "Device path should exist or be a UNC path");
        }
    }

    #[test]
    fn test_platform_adapter_available_filesystems() {
        let adapter = get_platform_adapter();
        let filesystems = adapter.available_filesystems();
        
        // Should have at least one filesystem available
        assert!(!filesystems.is_empty(), "Should have at least one available filesystem");
        
        // All filesystems should be available on current platform
        for fs in &filesystems {
            assert!(fs.is_available_on_current_platform(), 
                "Filesystem {:?} should be available on current platform", fs);
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Generator for device types
    prop_compose! {
        fn arb_device_type()(variant in 0u8..8) -> super::super::device_monitor::DeviceType {
            match variant {
                0 => super::super::device_monitor::DeviceType::InternalDrive,
                1 => super::super::device_monitor::DeviceType::ExternalDrive,
                2 => super::super::device_monitor::DeviceType::UsbDrive,
                3 => super::super::device_monitor::DeviceType::NetworkDrive,
                4 => super::super::device_monitor::DeviceType::OpticalDrive,
                5 => super::super::device_monitor::DeviceType::DiskImage,
                6 => super::super::device_monitor::DeviceType::WslDistribution,
                _ => super::super::device_monitor::DeviceType::CloudStorage,
            }
        }
    }

    // Generator for valid device metadata
    prop_compose! {
        fn arb_device_metadata()(
            id in 1u64..1000,
            name in "[a-zA-Z0-9 ]{1,50}",
            path in "/[a-zA-Z0-9/]{1,50}",
            device_type in arb_device_type(),
            total_space in 1u64..u64::MAX,
            free_space_ratio in 0.0f64..=1.0,
            is_removable in any::<bool>(),
        ) -> Device {
            let free_space = (total_space as f64 * free_space_ratio) as u64;
            Device {
                id: DeviceId::new(id),
                name,
                path: PathBuf::from(path),
                device_type,
                total_space,
                free_space,
                is_removable,
                is_read_only: false,
                is_mounted: true,
            }
        }
    }

    // **Feature: advanced-device-management, Property 1: Device Detection Completeness**
    // **Validates: Requirements 1.1, 1.2, 1.8**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_device_detection_completeness(device in arb_device_metadata()) {
            // Property: For any device, it should have valid metadata
            // - Non-empty name
            // - Valid path (non-empty)
            // - Non-zero capacity for storage devices (except virtual devices)
            
            prop_assert!(!device.name.is_empty(), 
                "Device name must not be empty");
            
            prop_assert!(!device.path.as_os_str().is_empty(), 
                "Device path must not be empty");
            
            // For storage devices (not WSL or cloud), capacity should be non-zero
            let is_virtual = matches!(
                device.device_type,
                super::super::device_monitor::DeviceType::WslDistribution 
                    | super::super::device_monitor::DeviceType::CloudStorage
            );
            
            if !is_virtual {
                prop_assert!(device.total_space > 0, 
                    "Storage device total_space must be non-zero");
            }
            
            // Free space should not exceed total space
            prop_assert!(device.free_space <= device.total_space,
                "Free space ({}) must not exceed total space ({})", 
                device.free_space, device.total_space);
        }
    }

    // **Feature: advanced-device-management, Property 1: Device Detection Completeness (Event Emission)**
    // **Validates: Requirements 1.1, 1.2, 1.8**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_device_event_contains_valid_metadata(device in arb_device_metadata()) {
            // When a device is connected, the DeviceEvent::Connected should contain valid metadata
            let event = DeviceEvent::Connected(device.clone());
            
            if let DeviceEvent::Connected(connected_device) = event {
                // Verify the event contains the same device with valid metadata
                prop_assert_eq!(connected_device.id, device.id);
                prop_assert_eq!(&connected_device.name, &device.name);
                prop_assert_eq!(&connected_device.path, &device.path);
                prop_assert!(!connected_device.name.is_empty());
            } else {
                prop_assert!(false, "Expected DeviceEvent::Connected");
            }
        }
    }

    // **Feature: advanced-device-management, Property 2: Device Disconnection Consistency**
    // **Validates: Requirements 1.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_device_disconnection_consistency(devices in prop::collection::vec(arb_device_metadata(), 1..10)) {
            use super::super::device_monitor::DeviceMonitor;
            
            let mut monitor = DeviceMonitor::new();
            let receiver = monitor.subscribe().expect("Should be able to subscribe");
            
            // Add all devices
            let mut added_ids = Vec::new();
            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }
            
            // Drain connect events
            while receiver.try_recv().is_ok() {}
            
            // Remove each device and verify consistency
            for id in &added_ids {
                let removed = monitor.remove_device(*id);
                
                // Property: remove_device should return the removed device
                prop_assert!(removed.is_some(), 
                    "remove_device should return the removed device for id {:?}", id);
                
                // Property: Device should no longer appear in devices() list
                prop_assert!(monitor.get_device(*id).is_none(),
                    "Device {:?} should not appear in devices() after removal", id);
            }
            
            // Collect disconnect events
            let mut disconnect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Disconnected(id) = event {
                    disconnect_events.push(id);
                }
            }
            
            // Property: Should have received a Disconnected event for each removed device
            prop_assert_eq!(disconnect_events.len(), added_ids.len(),
                "Should receive {} disconnect events, got {}", 
                added_ids.len(), disconnect_events.len());
            
            // Property: All disconnect events should match the removed device IDs
            for id in &added_ids {
                prop_assert!(disconnect_events.contains(id),
                    "Should have received Disconnected event for device {:?}", id);
            }
            
            // Property: devices() should be empty after all removals
            prop_assert!(monitor.devices().is_empty(),
                "devices() should be empty after removing all devices");
        }
    }

    // **Feature: advanced-device-management, Property 2: Device Disconnection Event Correctness**
    // **Validates: Requirements 1.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_disconnection_event_contains_correct_id(device in arb_device_metadata()) {
            // When a device is disconnected, the DeviceEvent::Disconnected should contain the correct ID
            let event = DeviceEvent::Disconnected(device.id);
            
            if let DeviceEvent::Disconnected(disconnected_id) = event {
                prop_assert_eq!(disconnected_id, device.id,
                    "Disconnected event should contain the correct device ID");
            } else {
                prop_assert!(false, "Expected DeviceEvent::Disconnected");
            }
        }
    }
}
