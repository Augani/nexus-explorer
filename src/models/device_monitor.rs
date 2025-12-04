use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// Unique identifier for a device
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub u64);

impl DeviceId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Type of storage device
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    InternalDrive,
    ExternalDrive,
    UsbDrive,
    NetworkDrive,
    OpticalDrive,
    DiskImage,
    WslDistribution,
    CloudStorage,
}

impl DeviceType {
    pub fn icon_name(&self) -> &'static str {
        match self {
            DeviceType::InternalDrive => "hard-drive",
            DeviceType::ExternalDrive => "hard-drive",
            DeviceType::UsbDrive => "usb",
            DeviceType::NetworkDrive => "cloud",
            DeviceType::OpticalDrive => "disc",
            DeviceType::DiskImage => "file-archive",
            DeviceType::WslDistribution => "terminal",
            DeviceType::CloudStorage => "cloud",
        }
    }
}

/// Represents a mounted storage device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub path: PathBuf,
    pub device_type: DeviceType,
    pub total_space: u64,
    pub free_space: u64,
    pub is_removable: bool,
    pub is_read_only: bool,
    pub is_mounted: bool,
}

impl Device {
    pub fn new(
        id: DeviceId,
        name: String,
        path: PathBuf,
        device_type: DeviceType,
    ) -> Self {
        Self {
            id,
            name,
            path,
            device_type,
            total_space: 0,
            free_space: 0,
            is_removable: false,
            is_read_only: false,
            is_mounted: true,
        }
    }

    pub fn with_space(mut self, total: u64, free: u64) -> Self {
        self.total_space = total;
        self.free_space = free;
        self
    }

    pub fn with_removable(mut self, removable: bool) -> Self {
        self.is_removable = removable;
        self
    }

    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.is_read_only = read_only;
        self
    }

    /// Calculate used space
    pub fn used_space(&self) -> u64 {
        self.total_space.saturating_sub(self.free_space)
    }

    /// Calculate usage percentage (0.0 - 1.0)
    pub fn usage_percentage(&self) -> f64 {
        if self.total_space == 0 {
            0.0
        } else {
            self.used_space() as f64 / self.total_space as f64
        }
    }
}


/// WSL distribution information (Windows only)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WslDistribution {
    pub name: String,
    pub path: PathBuf,
    pub is_running: bool,
    pub version: u8,
}

impl WslDistribution {
    pub fn new(name: String, path: PathBuf, version: u8) -> Self {
        Self {
            name,
            path,
            is_running: false,
            version,
        }
    }
}

/// Events emitted by the device monitor
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceEvent {
    Connected(Device),
    Disconnected(DeviceId),
    Updated(Device),
    WslStarted(String),
    WslStopped(String),
}

/// Errors that can occur during device operations
#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device not found: {0:?}")]
    NotFound(DeviceId),

    #[error("Eject failed: {0}")]
    EjectFailed(String),

    #[error("Unmount failed: {0}")]
    UnmountFailed(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type DeviceResult<T> = std::result::Result<T, DeviceError>;

/// Device monitor for tracking connected storage devices
pub struct DeviceMonitor {
    devices: Vec<Device>,
    wsl_distributions: Vec<WslDistribution>,
    next_id: u64,
    is_monitoring: Arc<AtomicBool>,
    event_sender: Option<flume::Sender<DeviceEvent>>,
    event_receiver: Option<flume::Receiver<DeviceEvent>>,
}

impl Default for DeviceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceMonitor {
    pub fn new() -> Self {
        let (sender, receiver) = flume::unbounded();
        Self {
            devices: Vec::new(),
            wsl_distributions: Vec::new(),
            next_id: 1,
            is_monitoring: Arc::new(AtomicBool::new(false)),
            event_sender: Some(sender),
            event_receiver: Some(receiver),
        }
    }

    /// Generate a new unique device ID
    fn next_device_id(&mut self) -> DeviceId {
        let id = DeviceId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Get all currently detected devices
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// Get all WSL distributions (Windows only)
    pub fn wsl_distributions(&self) -> &[WslDistribution] {
        &self.wsl_distributions
    }

    /// Find a device by ID
    pub fn get_device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Find a device by path
    pub fn get_device_by_path(&self, path: &PathBuf) -> Option<&Device> {
        self.devices.iter().find(|d| &d.path == path)
    }

    /// Subscribe to device events
    pub fn subscribe(&self) -> Option<flume::Receiver<DeviceEvent>> {
        self.event_receiver.clone()
    }

    /// Send a device event to subscribers
    fn send_event(&self, event: DeviceEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Add a device to the monitor
    pub fn add_device(&mut self, mut device: Device) -> DeviceId {
        device.id = self.next_device_id();
        let id = device.id;
        self.send_event(DeviceEvent::Connected(device.clone()));
        self.devices.push(device);
        id
    }

    /// Remove a device from the monitor
    pub fn remove_device(&mut self, id: DeviceId) -> Option<Device> {
        if let Some(pos) = self.devices.iter().position(|d| d.id == id) {
            let device = self.devices.remove(pos);
            self.send_event(DeviceEvent::Disconnected(id));
            Some(device)
        } else {
            None
        }
    }

    /// Update a device's information
    pub fn update_device(&mut self, device: Device) {
        if let Some(existing) = self.devices.iter_mut().find(|d| d.id == device.id) {
            *existing = device.clone();
            self.send_event(DeviceEvent::Updated(device));
        }
    }

    /// Check if monitoring is active
    pub fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }

    /// Start monitoring for device changes
    pub fn start_monitoring(&mut self) {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return;
        }
        self.is_monitoring.store(true, Ordering::SeqCst);
        
        self.enumerate_devices();
    }

    /// Stop monitoring for device changes
    pub fn stop_monitoring(&mut self) {
        self.is_monitoring.store(false, Ordering::SeqCst);
    }

    /// Enumerate all devices on the system
    pub fn enumerate_devices(&mut self) {
        self.devices.clear();
        
        #[cfg(target_os = "windows")]
        self.enumerate_windows_devices();
        
        #[cfg(target_os = "macos")]
        self.enumerate_macos_devices();
        
        #[cfg(target_os = "linux")]
        self.enumerate_linux_devices();
    }

    /// Refresh space information for all devices
    pub fn refresh_space_info(&mut self) {
        for device in &mut self.devices {
            if let Ok((total, free)) = get_disk_space(&device.path) {
                device.total_space = total;
                device.free_space = free;
            }
        }
    }
}


/// Get disk space for a path (cross-platform)
pub fn get_disk_space(path: &PathBuf) -> std::io::Result<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        
        let c_path = CString::new(path.as_os_str().as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                let total = stat.f_blocks as u64 * stat.f_frsize as u64;
                let free = stat.f_bavail as u64 * stat.f_frsize as u64;
                Ok((total, free))
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }
    
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        
        let wide_path: Vec<u16> = path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;
        
        unsafe {
            if windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide_path.as_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            ) != 0 {
                Ok((total_bytes, free_bytes_available))
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }
    
    #[cfg(not(any(unix, windows)))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Platform not supported",
        ))
    }
}


