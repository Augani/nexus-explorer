use super::device_monitor::{Device, DeviceId, DeviceMonitor, DeviceType, WslDistribution, get_disk_space};
use std::path::PathBuf;

impl DeviceMonitor {
    /// Enumerate devices on Windows by scanning drive letters
    #[cfg(target_os = "windows")]
    pub fn enumerate_windows_devices(&mut self) {
        // Enumerate all drive letters A-Z
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
            
            device = device.with_read_only(is_drive_read_only(&path));
            
            self.add_device(device);
        }
        
        // Enumerate WSL distributions
        self.enumerate_wsl_distributions();
    }

    /// Enumerate WSL distributions
    #[cfg(target_os = "windows")]
    fn enumerate_wsl_distributions(&mut self) {
        self.wsl_distributions.clear();
        
        // Try to list WSL distributions using wsl.exe
        if let Ok(output) = std::process::Command::new("wsl")
            .args(["--list", "--verbose"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                
                // Parse the output (skip header line)
                for line in stdout.lines().skip(1) {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    // Parse: "* Ubuntu    Running    2" or "  Debian    Stopped    1"
                    let is_default = line.starts_with('*');
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
                        
                        self.wsl_distributions.push(distro);
                        
                        // Also add as a device if running
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

    /// Eject a device on Windows
    #[cfg(target_os = "windows")]
    pub fn eject(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self.get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;
        
        if !device.is_removable {
            return Err(super::device_monitor::DeviceError::EjectFailed(
                "Device is not removable".to_string()
            ));
        }
        
        // For Windows, we'd use DeviceIoControl with IOCTL_STORAGE_EJECT_MEDIA
        // This is a simplified implementation using PowerShell
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
            Err(super::device_monitor::DeviceError::EjectFailed(error.to_string()))
        }
    }

    /// Unmount a device on Windows (same as eject for most cases)
    #[cfg(target_os = "windows")]
    pub fn unmount(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        self.eject(id)
    }
}

/// Detect the type of Windows drive
#[cfg(target_os = "windows")]
fn detect_windows_drive_type(path: &PathBuf) -> DeviceType {
    use std::os::windows::ffi::OsStrExt;
    
    let wide_path: Vec<u16> = path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    unsafe {
        let drive_type = windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide_path.as_ptr());
        
        match drive_type {
            windows_sys::Win32::Storage::FileSystem::DRIVE_REMOVABLE => DeviceType::UsbDrive,
            windows_sys::Win32::Storage::FileSystem::DRIVE_FIXED => DeviceType::InternalDrive,
            windows_sys::Win32::Storage::FileSystem::DRIVE_REMOTE => DeviceType::NetworkDrive,
            windows_sys::Win32::Storage::FileSystem::DRIVE_CDROM => DeviceType::OpticalDrive,
            windows_sys::Win32::Storage::FileSystem::DRIVE_RAMDISK => DeviceType::DiskImage,
            _ => DeviceType::ExternalDrive,
        }
    }
}

/// Get the volume name for a Windows drive
#[cfg(target_os = "windows")]
fn get_windows_volume_name(path: &PathBuf) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    
    let wide_path: Vec<u16> = path.as_os_str()
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
        ) != 0 {
            let len = volume_name.iter().position(|&c| c == 0).unwrap_or(volume_name.len());
            let name = String::from_utf16_lossy(&volume_name[..len]);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Check if a drive is read-only on Windows
#[cfg(target_os = "windows")]
fn is_drive_read_only(path: &PathBuf) -> bool {
    use std::os::windows::ffi::OsStrExt;
    
    let wide_path: Vec<u16> = path.as_os_str()
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
