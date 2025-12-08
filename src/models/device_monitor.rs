use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub u64);

impl DeviceId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}


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
    pub is_encrypted: bool,
    pub smart_status: Option<HealthStatus>,
}

impl Device {
    pub fn new(id: DeviceId, name: String, path: PathBuf, device_type: DeviceType) -> Self {
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
            is_encrypted: false,
            smart_status: None,
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

    pub fn with_encrypted(mut self, encrypted: bool) -> Self {
        self.is_encrypted = encrypted;
        self
    }

    pub fn with_smart_status(mut self, status: HealthStatus) -> Self {
        self.smart_status = Some(status);
        self
    }


    pub fn used_space(&self) -> u64 {
        self.total_space.saturating_sub(self.free_space)
    }


    pub fn usage_percentage(&self) -> f64 {
        if self.total_space == 0 {
            0.0
        } else {
            self.used_space() as f64 / self.total_space as f64
        }
    }


    pub fn has_health_warning(&self) -> bool {
        matches!(
            self.smart_status,
            Some(HealthStatus::Warning) | Some(HealthStatus::Critical)
        )
    }
}


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


#[derive(Debug, Clone, PartialEq)]
pub enum DeviceEvent {
    Connected(Device),
    Disconnected(DeviceId),
    Updated(Device),
    WslStarted(String),
    WslStopped(String),
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Good,
    Warning,
    Critical,
    Unknown,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl HealthStatus {

    pub fn icon_name(&self) -> &'static str {
        match self {
            HealthStatus::Good => "check",
            HealthStatus::Warning => "triangle-alert",
            HealthStatus::Critical => "triangle-alert",
            HealthStatus::Unknown => "circle-question-mark",
        }
    }


    pub fn color(&self) -> u32 {
        match self {
            HealthStatus::Good => 0x3fb950,
            HealthStatus::Warning => 0xd29922,
            HealthStatus::Critical => 0xf85149,
            HealthStatus::Unknown => 0x8b949e,
        }
    }


    pub fn description(&self) -> &'static str {
        match self {
            HealthStatus::Good => "Good",
            HealthStatus::Warning => "Warning",
            HealthStatus::Critical => "Critical",
            HealthStatus::Unknown => "Unknown",
        }
    }


    pub fn requires_attention(&self) -> bool {
        matches!(self, HealthStatus::Warning | HealthStatus::Critical)
    }
}


pub mod smart_attributes {
    pub const RAW_READ_ERROR_RATE: u8 = 1;
    pub const THROUGHPUT_PERFORMANCE: u8 = 2;
    pub const SPIN_UP_TIME: u8 = 3;
    pub const START_STOP_COUNT: u8 = 4;
    pub const REALLOCATED_SECTORS_COUNT: u8 = 5;
    pub const SEEK_ERROR_RATE: u8 = 7;
    pub const POWER_ON_HOURS: u8 = 9;
    pub const SPIN_RETRY_COUNT: u8 = 10;
    pub const POWER_CYCLE_COUNT: u8 = 12;
    pub const SOFT_READ_ERROR_RATE: u8 = 13;
    pub const CURRENT_PENDING_SECTOR_COUNT: u8 = 197;
    pub const OFFLINE_UNCORRECTABLE: u8 = 198;
    pub const UDMA_CRC_ERROR_COUNT: u8 = 199;
    pub const TEMPERATURE_CELSIUS: u8 = 194;
    pub const TEMPERATURE_CELSIUS_ALT: u8 = 190;
    pub const WEAR_LEVELING_COUNT: u8 = 177;
    pub const USED_RESERVED_BLOCK_COUNT: u8 = 180;
    pub const PROGRAM_FAIL_COUNT: u8 = 181;
    pub const ERASE_FAIL_COUNT: u8 = 182;
    pub const TOTAL_LBAS_WRITTEN: u8 = 241;
    pub const TOTAL_LBAS_READ: u8 = 242;
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmartAttribute {
    pub id: u8,
    pub name: String,
    pub value: u64,
    pub worst: u64,
    pub threshold: u64,
    pub raw_value: String,
}

impl SmartAttribute {
    pub fn new(id: u8, name: String, value: u64, worst: u64, threshold: u64, raw_value: String) -> Self {
        Self {
            id,
            name,
            value,
            worst,
            threshold,
            raw_value,
        }
    }


    pub fn is_failing(&self) -> bool {
        self.value > 0 && self.threshold > 0 && self.value <= self.threshold
    }


    pub fn is_warning(&self) -> bool {
        if self.threshold == 0 {
            return false;
        }
        let warning_threshold = self.threshold.saturating_add(10);
        self.value > self.threshold && self.value <= warning_threshold
    }


    pub fn get_standard_name(id: u8) -> &'static str {
        match id {
            smart_attributes::RAW_READ_ERROR_RATE => "Raw Read Error Rate",
            smart_attributes::THROUGHPUT_PERFORMANCE => "Throughput Performance",
            smart_attributes::SPIN_UP_TIME => "Spin Up Time",
            smart_attributes::START_STOP_COUNT => "Start/Stop Count",
            smart_attributes::REALLOCATED_SECTORS_COUNT => "Reallocated Sectors Count",
            smart_attributes::SEEK_ERROR_RATE => "Seek Error Rate",
            smart_attributes::POWER_ON_HOURS => "Power-On Hours",
            smart_attributes::SPIN_RETRY_COUNT => "Spin Retry Count",
            smart_attributes::POWER_CYCLE_COUNT => "Power Cycle Count",
            smart_attributes::SOFT_READ_ERROR_RATE => "Soft Read Error Rate",
            smart_attributes::CURRENT_PENDING_SECTOR_COUNT => "Current Pending Sector Count",
            smart_attributes::OFFLINE_UNCORRECTABLE => "Offline Uncorrectable",
            smart_attributes::UDMA_CRC_ERROR_COUNT => "UDMA CRC Error Count",
            smart_attributes::TEMPERATURE_CELSIUS | smart_attributes::TEMPERATURE_CELSIUS_ALT => "Temperature",
            smart_attributes::WEAR_LEVELING_COUNT => "Wear Leveling Count",
            smart_attributes::USED_RESERVED_BLOCK_COUNT => "Used Reserved Block Count",
            smart_attributes::PROGRAM_FAIL_COUNT => "Program Fail Count",
            smart_attributes::ERASE_FAIL_COUNT => "Erase Fail Count",
            smart_attributes::TOTAL_LBAS_WRITTEN => "Total LBAs Written",
            smart_attributes::TOTAL_LBAS_READ => "Total LBAs Read",
            _ => "Unknown Attribute",
        }
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmartData {
    pub health_status: HealthStatus,
    pub temperature_celsius: Option<u8>,
    pub power_on_hours: Option<u64>,
    pub reallocated_sectors: Option<u64>,
    pub pending_sectors: Option<u64>,
    pub attributes: Vec<SmartAttribute>,
}

impl Default for SmartData {
    fn default() -> Self {
        Self {
            health_status: HealthStatus::Unknown,
            temperature_celsius: None,
            power_on_hours: None,
            reallocated_sectors: None,
            pending_sectors: None,
            attributes: Vec::new(),
        }
    }
}

impl SmartData {

    pub fn from_attributes(attributes: Vec<SmartAttribute>) -> Self {
        let mut data = SmartData {
            attributes: attributes.clone(),
            ..Default::default()
        };

        for attr in &attributes {
            match attr.id {
                smart_attributes::TEMPERATURE_CELSIUS | smart_attributes::TEMPERATURE_CELSIUS_ALT => {
                    if let Ok(temp) = attr.raw_value.parse::<u64>() {
                        data.temperature_celsius = Some(temp.min(255) as u8);
                    }
                }
                smart_attributes::POWER_ON_HOURS => {
                    if let Ok(hours) = attr.raw_value.parse::<u64>() {
                        data.power_on_hours = Some(hours);
                    }
                }
                smart_attributes::REALLOCATED_SECTORS_COUNT => {
                    if let Ok(sectors) = attr.raw_value.parse::<u64>() {
                        data.reallocated_sectors = Some(sectors);
                    }
                }
                smart_attributes::CURRENT_PENDING_SECTOR_COUNT => {
                    if let Ok(sectors) = attr.raw_value.parse::<u64>() {
                        data.pending_sectors = Some(sectors);
                    }
                }
                _ => {}
            }
        }

        data.health_status = data.determine_health_status();
        data
    }


    pub fn determine_health_status(&self) -> HealthStatus {
        if let Some(reallocated) = self.reallocated_sectors {
            if reallocated > 100 {
                return HealthStatus::Critical;
            }
        }

        if let Some(pending) = self.pending_sectors {
            if pending > 10 {
                return HealthStatus::Critical;
            }
        }

        for attr in &self.attributes {
            if attr.is_failing() {
                return HealthStatus::Critical;
            }
        }

        if let Some(reallocated) = self.reallocated_sectors {
            if reallocated > 0 {
                return HealthStatus::Warning;
            }
        }

        if let Some(pending) = self.pending_sectors {
            if pending > 0 {
                return HealthStatus::Warning;
            }
        }

        for attr in &self.attributes {
            if attr.is_warning() {
                return HealthStatus::Warning;
            }
        }

        if let Some(temp) = self.temperature_celsius {
            if temp > 60 {
                return HealthStatus::Critical;
            }
            if temp > 50 {
                return HealthStatus::Warning;
            }
        }

        HealthStatus::Good
    }


    pub fn health_summary(&self) -> String {
        match self.health_status {
            HealthStatus::Good => "Drive is healthy".to_string(),
            HealthStatus::Warning => {
                let mut issues = Vec::new();
                if let Some(reallocated) = self.reallocated_sectors {
                    if reallocated > 0 {
                        issues.push(format!("{} reallocated sectors", reallocated));
                    }
                }
                if let Some(pending) = self.pending_sectors {
                    if pending > 0 {
                        issues.push(format!("{} pending sectors", pending));
                    }
                }
                if let Some(temp) = self.temperature_celsius {
                    if temp > 50 {
                        issues.push(format!("High temperature ({}°C)", temp));
                    }
                }
                if issues.is_empty() {
                    "Drive health warning".to_string()
                } else {
                    format!("Warning: {}", issues.join(", "))
                }
            }
            HealthStatus::Critical => "Drive health critical - backup data immediately!".to_string(),
            HealthStatus::Unknown => "Health data unavailable".to_string(),
        }
    }


    pub fn get_attribute(&self, id: u8) -> Option<&SmartAttribute> {
        self.attributes.iter().find(|a| a.id == id)
    }
}


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


    fn next_device_id(&mut self) -> DeviceId {
        let id = DeviceId::new(self.next_id);
        self.next_id += 1;
        id
    }


    pub fn devices(&self) -> &[Device] {
        &self.devices
    }


    pub fn wsl_distributions(&self) -> &[WslDistribution] {
        &self.wsl_distributions
    }


    pub fn wsl_distributions_mut(&mut self) -> &mut Vec<WslDistribution> {
        &mut self.wsl_distributions
    }


    pub fn get_device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }


    pub fn get_device_by_path(&self, path: &PathBuf) -> Option<&Device> {
        self.devices.iter().find(|d| &d.path == path)
    }


    pub fn subscribe(&self) -> Option<flume::Receiver<DeviceEvent>> {
        self.event_receiver.clone()
    }


    fn send_event(&self, event: DeviceEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }


    pub fn add_device(&mut self, mut device: Device) -> DeviceId {
        device.id = self.next_device_id();
        let id = device.id;
        self.send_event(DeviceEvent::Connected(device.clone()));
        self.devices.push(device);
        id
    }


    pub fn remove_device(&mut self, id: DeviceId) -> Option<Device> {
        if let Some(pos) = self.devices.iter().position(|d| d.id == id) {
            let device = self.devices.remove(pos);
            self.send_event(DeviceEvent::Disconnected(id));
            Some(device)
        } else {
            None
        }
    }


    pub fn update_device(&mut self, device: Device) {
        if let Some(existing) = self.devices.iter_mut().find(|d| d.id == device.id) {
            *existing = device.clone();
            self.send_event(DeviceEvent::Updated(device));
        }
    }


    pub fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }


    pub fn start_monitoring(&mut self) {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return;
        }
        self.is_monitoring.store(true, Ordering::SeqCst);

        self.enumerate_devices();
    }


    pub fn stop_monitoring(&mut self) {
        self.is_monitoring.store(false, Ordering::SeqCst);
    }


    pub fn enumerate_devices(&mut self) {
        self.devices.clear();

        #[cfg(target_os = "windows")]
        self.enumerate_windows_devices();

        #[cfg(target_os = "macos")]
        self.enumerate_macos_devices();

        #[cfg(target_os = "linux")]
        self.enumerate_linux_devices();
    }


    pub fn refresh_space_info(&mut self) {
        for device in &mut self.devices {
            if let Ok((total, free)) = get_disk_space(&device.path) {
                device.total_space = total;
                device.free_space = free;
            }
        }
    }
}


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

        let wide_path: Vec<u16> = path
            .as_os_str()
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
            ) != 0
            {
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
