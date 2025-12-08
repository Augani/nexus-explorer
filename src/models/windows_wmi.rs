//! Windows WMI-based device enumeration
#![cfg(target_os = "windows")]

use super::device_monitor::{get_disk_space, Device, DeviceId, DeviceType};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WmiDeviceInfo {
    pub device_id: String,
    pub volume_name: Option<String>,
    pub file_system: Option<String>,
    pub size: Option<u64>,
    pub free_space: Option<u64>,
    pub drive_type: u32,
    pub volume_serial_number: Option<String>,
    pub model: Option<String>,
    pub interface_type: Option<String>,
}

pub struct WmiDeviceEnumerator {
    _initialized: bool,
}

impl WmiDeviceEnumerator {
    pub fn new() -> Self {
        Self { _initialized: false }
    }

    pub fn enumerate_devices(&self) -> Vec<Device> {
        match self.enumerate_wmi_devices() {
            Ok(devices) => devices,
            Err(_) => self.enumerate_fallback(),
        }
    }

    pub fn get_device_info(&self, drive_letter: &str) -> Option<WmiDeviceInfo> {
        self.query_device_info(drive_letter).ok()
    }

    fn enumerate_wmi_devices(&self) -> Result<Vec<Device>, String> {
        use serde::Deserialize;
        use wmi::{COMLibrary, WMIConnection};

        #[derive(Deserialize, Debug)]
        #[serde(rename = "Win32_LogicalDisk")]
        #[serde(rename_all = "PascalCase")]
        struct Win32LogicalDisk {
            device_id: String,
            volume_name: Option<String>,
            #[allow(dead_code)]
            file_system: Option<String>,
            size: Option<u64>,
            free_space: Option<u64>,
            drive_type: u32,
            #[allow(dead_code)]
            volume_serial_number: Option<String>,
        }

        let com_con = COMLibrary::new()
            .map_err(|e| format!("COM init failed: {}", e))?;
        let wmi_con = WMIConnection::new(com_con)
            .map_err(|e| format!("WMI connect failed: {}", e))?;

        let results: Vec<Win32LogicalDisk> = wmi_con
            .raw_query("SELECT * FROM Win32_LogicalDisk")
            .map_err(|e| format!("WMI query failed: {}", e))?;

        let mut devices = Vec::new();
        for (idx, disk) in results.into_iter().enumerate() {
            let path = PathBuf::from(format!("{}\\", disk.device_id));
            if !path.exists() { continue; }

            let device_type = wmi_drive_type_to_device_type(disk.drive_type);
            let name = disk.volume_name
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("Local Disk ({})", disk.device_id));

            let is_removable = matches!(
                device_type,
                DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::OpticalDrive
            );

            let device = Device::new(DeviceId::new(idx as u64 + 1), name, path, device_type)
                .with_removable(is_removable)
                .with_space(disk.size.unwrap_or(0), disk.free_space.unwrap_or(0));

            devices.push(device);
        }
        Ok(devices)
    }

    fn query_device_info(&self, drive_letter: &str) -> Result<WmiDeviceInfo, String> {
        use serde::Deserialize;
        use wmi::{COMLibrary, WMIConnection};

        #[derive(Deserialize, Debug)]
        #[serde(rename = "Win32_LogicalDisk")]
        #[serde(rename_all = "PascalCase")]
        struct Win32LogicalDisk {
            device_id: String,
            volume_name: Option<String>,
            file_system: Option<String>,
            size: Option<u64>,
            free_space: Option<u64>,
            drive_type: u32,
            volume_serial_number: Option<String>,
        }

        let com_con = COMLibrary::new().map_err(|e| format!("{}", e))?;
        let wmi_con = WMIConnection::new(com_con).map_err(|e| format!("{}", e))?;

        let query = format!(
            "SELECT * FROM Win32_LogicalDisk WHERE DeviceID = '{}:'",
            drive_letter.trim_end_matches(':')
        );

        let results: Vec<Win32LogicalDisk> = wmi_con
            .raw_query(&query)
            .map_err(|e| format!("{}", e))?;

        results.into_iter().next()
            .map(|d| WmiDeviceInfo {
                device_id: d.device_id,
                volume_name: d.volume_name,
                file_system: d.file_system,
                size: d.size,
                free_space: d.free_space,
                drive_type: d.drive_type,
                volume_serial_number: d.volume_serial_number,
                model: None,
                interface_type: None,
            })
            .ok_or_else(|| "Not found".to_string())
    }

    fn enumerate_fallback(&self) -> Vec<Device> {
        let mut devices = Vec::new();
        for letter in b'A'..=b'Z' {
            let path = PathBuf::from(format!("{}:\\", letter as char));
            if !path.exists() { continue; }

            let device_type = detect_drive_type(&path);
            let name = get_volume_name(&path)
                .unwrap_or_else(|| format!("Local Disk ({}:)", letter as char));

            let is_removable = matches!(
                device_type,
                DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::OpticalDrive
            );

            let mut device = Device::new(DeviceId::new(letter as u64), name, path.clone(), device_type)
                .with_removable(is_removable);

            if let Ok((total, free)) = get_disk_space(&path) {
                device = device.with_space(total, free);
            }
            devices.push(device);
        }
        devices
    }
}

impl Default for WmiDeviceEnumerator {
    fn default() -> Self { Self::new() }
}

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;
const DRIVE_RAMDISK: u32 = 6;

fn wmi_drive_type_to_device_type(drive_type: u32) -> DeviceType {
    match drive_type {
        DRIVE_REMOVABLE => DeviceType::UsbDrive,
        DRIVE_FIXED => DeviceType::InternalDrive,
        DRIVE_REMOTE => DeviceType::NetworkDrive,
        DRIVE_CDROM => DeviceType::OpticalDrive,
        DRIVE_RAMDISK => DeviceType::DiskImage,
        _ => DeviceType::ExternalDrive,
    }
}

fn detect_drive_type(path: &PathBuf) -> DeviceType {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        let dt = windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr());
        wmi_drive_type_to_device_type(dt)
    }
}

fn get_volume_name(path: &PathBuf) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut vol: [u16; 261] = [0; 261];
    let mut fs: [u16; 261] = [0; 261];
    let mut sn: u32 = 0;
    let mut mcl: u32 = 0;
    let mut flags: u32 = 0;

    unsafe {
        if windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
            wide.as_ptr(), vol.as_mut_ptr(), 261, &mut sn, &mut mcl, &mut flags, fs.as_mut_ptr(), 261
        ) != 0 {
            let len = vol.iter().position(|&c| c == 0).unwrap_or(vol.len());
            let name = String::from_utf16_lossy(&vol[..len]);
            if !name.is_empty() { return Some(name); }
        }
    }
    None
}
