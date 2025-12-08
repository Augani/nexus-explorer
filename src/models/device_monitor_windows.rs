//! Windows-specific device detection and monitoring implementation
//! 
//! This module provides Windows device detection using:
//! - WMI (Windows Management Instrumentation) for device enumeration
//! - GetDriveTypeW for drive type detection
//! - GetVolumeInformationW for volume metadata

use super::device_monitor::{
    get_disk_space, Device, DeviceId, DeviceMonitor, DeviceType, WslDistribution,
};
use std::path::PathBuf;

/
const DRIVE_UNKNOWN: u32 = 0;
const DRIVE_NO_ROOT_DIR: u32 = 1;
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;
const DRIVE_RAMDISK: u32 = 6;

/
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct WmiLogicalDisk {
    pub device_id: String,
    pub volume_name: Option<String>,
    pub file_system: Option<String>,
    pub size: Option<u64>,
    pub free_space: Option<u64>,
    pub drive_type: u32,
    pub volume_serial_number: Option<String>,
}

/
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct WmiDiskDrive {
    pub device_id: String,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub size: Option<u64>,
    pub media_type: Option<String>,
    pub interface_type: Option<String>,
}

impl DeviceMonitor {
    /
    #[cfg(target_os = "windows")]
    pub fn enumerate_windows_devices(&mut self) {
        if let Ok(wmi_devices) = enumerate_wmi_logical_disks() {
            for wmi_disk in wmi_devices {
                if let Some(device) = wmi_disk_to_device(&wmi_disk) {
                    self.add_device(device);
                }
            }
        } else {
            self.enumerate_windows_drives_basic();
        }

        self.enumerate_wsl_distributions();
    }

    /
    #[cfg(target_os = "windows")]
    fn enumerate_windows_drives_basic(&mut self) {
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive_path);

            if !path.exists() {
                continue;
            }

            let device_type = detect_windows_drive_type(&path);
            let name = get_windows_volume_name(&path)
                .unwrap_or_else(|| format!("Local Disk ({}:)", letter as char));

            let is_removable = matches!(
                device_type,
                DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::OpticalDrive
            );

            let mut device = Device::new(DeviceId::new(0), name, path.clone(), device_type)
                .with_removable(is_removable);

            if let Ok((total, free)) = get_disk_space(&path) {
                device = device.with_space(total, free);
            }

            device = device.with_read_only(is_drive_read_only(&path));

            self.add_device(device);
        }
    }

    /
    #[cfg(target_os = "windows")]
    fn enumerate_wsl_distributions(&mut self) {
        self.wsl_distributions_mut().clear();

        if let Ok(output) = std::process::Command::new("wsl")
            .args(["--list", "--verbose"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                for line in stdout.lines().skip(1) {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let line = line.trim_start_matches('*').trim();

                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let name = parts[0].to_string();
                        let is_running = parts[1].eq_ignore_ascii_case("Running");
                        let version = parts[2].parse().unwrap_or(2);

                        let wsl_path = PathBuf::from(format!("\\\\wsl$\\{}", name));

                        let distro = WslDistribution {
                            name: name.clone(),
                            path: wsl_path.clone(),
                            is_running,
                            version,
                        };

                        self.wsl_distributions_mut().push(distro);

                        if is_running {
                            let device = Device::new(
                                DeviceId::new(0),
                                format!("WSL: {}", name),
                                wsl_path,
                                DeviceType::WslDistribution,
                            )
                            .with_removable(false);

                            self.add_device(device);
                        }
                    }
                }
            }
        }
    }

    /
    #[cfg(target_os = "windows")]
    pub fn eject(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self
            .get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;

        if !device.is_removable {
            return Err(super::device_monitor::DeviceError::EjectFailed(
                "Device is not removable".to_string(),
            ));
        }

        let drive_letter = device.path.to_string_lossy();
        let drive_letter = drive_letter.trim_end_matches('\\').trim_end_matches(':');

        let script = format!(
            "$driveEject = New-Object -comObject Shell.Application; \
             $driveEject.Namespace(17).ParseName('{}:').InvokeVerb('Eject')",
            drive_letter
        );

        let output = std::process::Command::new("powershell")
            .args(["-Command", &script])
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

    /
    #[cfg(target_os = "windows")]
    pub fn unmount(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        self.eject(id)
    }
}

/
#[cfg(target_os = "windows")]
pub fn enumerate_wmi_logical_disks() -> Result<Vec<WmiLogicalDisk>, String> {
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

    let com_con = COMLibrary::new().map_err(|e| format!("Failed to initialize COM: {}", e))?;
    let wmi_con = WMIConnection::new(com_con)
        .map_err(|e| format!("Failed to connect to WMI: {}", e))?;

    let results: Vec<Win32LogicalDisk> = wmi_con
        .raw_query("SELECT DeviceID, VolumeName, FileSystem, Size, FreeSpace, DriveType, VolumeSerialNumber FROM Win32_LogicalDisk")
        .map_err(|e| format!("WMI query failed: {}", e))?;

    Ok(results
        .into_iter()
        .map(|disk| WmiLogicalDisk {
            device_id: disk.device_id,
            volume_name: disk.volume_name,
            file_system: disk.file_system,
            size: disk.size,
            free_space: disk.free_space,
            drive_type: disk.drive_type,
            volume_serial_number: disk.volume_serial_number,
        })
        .collect())
}

/
#[cfg(target_os = "windows")]
pub fn enumerate_wmi_disk_drives() -> Result<Vec<WmiDiskDrive>, String> {
    use serde::Deserialize;
    use wmi::{COMLibrary, WMIConnection};

    #[derive(Deserialize, Debug)]
    #[serde(rename = "Win32_DiskDrive")]
    #[serde(rename_all = "PascalCase")]
    struct Win32DiskDrive {
        device_id: String,
        model: Option<String>,
        serial_number: Option<String>,
        size: Option<u64>,
        media_type: Option<String>,
        interface_type: Option<String>,
    }

    let com_con = COMLibrary::new().map_err(|e| format!("Failed to initialize COM: {}", e))?;
    let wmi_con = WMIConnection::new(com_con)
        .map_err(|e| format!("Failed to connect to WMI: {}", e))?;

    let results: Vec<Win32DiskDrive> = wmi_con
        .raw_query("SELECT DeviceID, Model, SerialNumber, Size, MediaType, InterfaceType FROM Win32_DiskDrive")
        .map_err(|e| format!("WMI query failed: {}", e))?;

    Ok(results
        .into_iter()
        .map(|disk| WmiDiskDrive {
            device_id: disk.device_id,
            model: disk.model,
            serial_number: disk.serial_number,
            size: disk.size,
            media_type: disk.media_type,
            interface_type: disk.interface_type,
        })
        .collect())
}

/
#[cfg(target_os = "windows")]
fn wmi_disk_to_device(wmi_disk: &WmiLogicalDisk) -> Option<Device> {
    let path = PathBuf::from(format!("{}\\", wmi_disk.device_id));
    
    if !path.exists() {
        return None;
    }

    let device_type = wmi_drive_type_to_device_type(wmi_disk.drive_type);
    
    let name = wmi_disk
        .volume_name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Local Disk ({})", wmi_disk.device_id));

    let is_removable = matches!(
        device_type,
        DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::OpticalDrive
    );

    let total_space = wmi_disk.size.unwrap_or(0);
    let free_space = wmi_disk.free_space.unwrap_or(0);

    let device = Device::new(DeviceId::new(0), name, path.clone(), device_type)
        .with_removable(is_removable)
        .with_space(total_space, free_space)
        .with_read_only(is_drive_read_only(&path));

    Some(device)
}

/
#[cfg(target_os = "windows")]
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

/
#[cfg(target_os = "windows")]
pub fn detect_windows_drive_type(path: &PathBuf) -> DeviceType {
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let drive_type = windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide_path.as_ptr());

        match drive_type {
            DRIVE_REMOVABLE => DeviceType::UsbDrive,
            DRIVE_FIXED => DeviceType::InternalDrive,
            DRIVE_REMOTE => DeviceType::NetworkDrive,
            DRIVE_CDROM => DeviceType::OpticalDrive,
            DRIVE_RAMDISK => DeviceType::DiskImage,
            _ => DeviceType::ExternalDrive,
        }
    }
}

/
#[cfg(target_os = "windows")]
pub fn get_windows_volume_name(path: &PathBuf) -> Option<String> {
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

/
#[cfg(target_os = "windows")]
pub fn get_windows_filesystem_type(path: &PathBuf) -> Option<String> {
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
            let len = fs_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(fs_name.len());
            let name = String::from_utf16_lossy(&fs_name[..len]);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/
#[cfg(target_os = "windows")]
pub fn is_drive_read_only(path: &PathBuf) -> bool {
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let attrs = windows_sys::Win32::Storage::FileSystem::GetFileAttributesW(wide_path.as_ptr());
        if attrs != windows_sys::Win32::Storage::FileSystem::INVALID_FILE_ATTRIBUTES {
            return (attrs & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_READONLY) != 0;
        }
    }
    false
}

/
#[cfg(target_os = "windows")]
pub fn get_available_drive_letters() -> Vec<char> {
    let mut drives = Vec::new();
    
    unsafe {
        let bitmask = windows_sys::Win32::Storage::FileSystem::GetLogicalDrives();
        for i in 0..26 {
            if (bitmask & (1 << i)) != 0 {
                drives.push((b'A' + i) as char);
            }
        }
    }
    
    drives
}

/
#[cfg(target_os = "windows")]
pub fn get_drive_info(drive_letter: char) -> Option<DriveInfo> {
    let path = PathBuf::from(format!("{}:\\", drive_letter));
    
    if !path.exists() {
        return None;
    }

    let drive_type = detect_windows_drive_type(&path);
    let volume_name = get_windows_volume_name(&path);
    let filesystem = get_windows_filesystem_type(&path);
    let (total_space, free_space) = get_disk_space(&path).ok()?;
    let is_read_only = is_drive_read_only(&path);

    Some(DriveInfo {
        drive_letter,
        path,
        drive_type,
        volume_name,
        filesystem,
        total_space,
        free_space,
        is_read_only,
    })
}

/
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub drive_letter: char,
    pub path: PathBuf,
    pub drive_type: DeviceType,
    pub volume_name: Option<String>,
    pub filesystem: Option<String>,
    pub total_space: u64,
    pub free_space: u64,
    pub is_read_only: bool,
}

#[cfg(target_os = "windows")]
impl DriveInfo {
    /
    pub fn is_removable(&self) -> bool {
        matches!(
            self.drive_type,
            DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::OpticalDrive
        )
    }

    /
    pub fn display_name(&self) -> String {
        self.volume_name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("Local Disk ({}:)", self.drive_letter))
    }
}
