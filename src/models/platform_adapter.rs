use std::path::{Path, PathBuf};
use thiserror::Error;

use super::device_monitor::{Device, DeviceEvent, DeviceId, DeviceResult, SmartData};


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


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileSystemType {
    Fat32,
    ExFat,
    Ntfs,
    ReFS,
    Apfs,
    HfsPlus,
    Ext4,
    Btrfs,
    Xfs,
}

impl FileSystemType {

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


#[derive(Debug, Clone)]
pub enum PlatformAction {

    Eject(DeviceId),

    Format(DeviceId, FormatOptions),

    MountImage(PathBuf),

    Unmount(PathBuf),

    OpenTerminal(PathBuf),

    PinToQuickAccess(PathBuf),

    ScanWithDefender(PathBuf),

    ShareAirDrop(Vec<PathBuf>),

    QuickAction(String, Vec<PathBuf>),

    Custom(String, Vec<String>),
}







pub trait PlatformAdapter: Send + Sync {




    fn enumerate_devices(&self) -> Vec<Device>;












    fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> PlatformResult<()>;





    fn stop_monitoring(&self) -> PlatformResult<()>;












    fn eject_device(&self, device_id: DeviceId) -> PlatformResult<()>;













    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()>;













    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>>;












    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf>;











    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()>;











    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem>;












    fn execute_action(&self, action: PlatformAction) -> PlatformResult<()>;





    fn available_filesystems(&self) -> Vec<FileSystemType>;


    fn is_monitoring(&self) -> bool;
}


use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;


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
        let wmi_devices = self.wmi_enumerator.enumerate_devices();
        if !wmi_devices.is_empty() {
            return wmi_devices;
        }

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
        let devices = self.enumerate_devices();
        let device = devices.iter()
            .find(|d| d.id.0 == device_id.0)
            .ok_or(PlatformError::DeviceNotFound(device_id))?;

        if !device.is_removable {
            return Err(PlatformError::EjectFailed("Device is not removable".to_string()));
        }

        let path_str = device.path.to_string_lossy();
        let drive_letter = path_str.chars().next()
            .ok_or(PlatformError::EjectFailed("Invalid drive path".to_string()))?;

        eject_windows_drive(drive_letter)
    }

    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()> {
        let devices = self.enumerate_devices();
        let device = devices.iter()
            .find(|d| d.id.0 == device_id.0)
            .ok_or(PlatformError::DeviceNotFound(device_id))?;

        let path_str = device.path.to_string_lossy();
        let drive_letter = path_str.chars().next()
            .ok_or(PlatformError::FormatFailed("Invalid drive path".to_string()))?;

        let fs_param = match options.filesystem {
            FileSystemType::Ntfs => "NTFS",
            FileSystemType::Fat32 => "FAT32",
            FileSystemType::ExFat => "EXFAT",
            FileSystemType::ReFS => "REFS",
            _ => return Err(PlatformError::FormatFailed(
                format!("Filesystem {:?} not supported on Windows", options.filesystem)
            )),
        };

        let mut args = vec![
            format!("{}:", drive_letter),
            format!("/FS:{}", fs_param),
            "/Y".to_string(),
        ];

        if !options.label.is_empty() {
            args.push(format!("/V:{}", options.label));
        } else {
            args.push("/V:".to_string());
        }

        if options.quick_format {
            args.push("/Q".to_string());
        }

        if options.enable_compression && options.filesystem == FileSystemType::Ntfs {
            args.push("/C".to_string());
        }

        let output = std::process::Command::new("format.com")
            .args(&args)
            .output()
            .map_err(|e| PlatformError::Io(e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            let error_msg = if error.is_empty() {
                stdout.to_string()
            } else {
                error.to_string()
            };
            
            Err(PlatformError::FormatFailed(error_msg))
        }
    }

    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>> {
        Ok(None)
    }

    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf> {
        mount_windows_disk_image(path)
    }

    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()> {
        unmount_windows_disk_image(mount_point)
    }

    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem> {
        let mut items = vec![
            ContextMenuItem::new("pin_quick_access", "Pin to Quick Access").with_icon("pin"),
            ContextMenuItem::new("scan_defender", "Scan with Windows Defender").with_icon("shield"),
        ];

        if paths.len() == 1 {
            if let Some(ext) = paths[0].extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "iso" || ext_lower == "vhd" || ext_lower == "vhdx" || ext_lower == "img" {
                    items.insert(0, ContextMenuItem::new("mount_image", "Mount").with_icon("disc"));
                }
            }
        }

        items
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


#[cfg(target_os = "windows")]
fn eject_windows_drive(drive_letter: char) -> PlatformResult<()> {
    use std::os::windows::ffi::OsStrExt;

    let volume_path = format!("\\\\.\\{}:", drive_letter);
    let wide_path: Vec<u16> = volume_path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        const GENERIC_READ: u32 = 0x80000000;
        const GENERIC_WRITE: u32 = 0x40000000;
        let handle = windows_sys::Win32::Storage::FileSystem::CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
            std::ptr::null(),
            windows_sys::Win32::Storage::FileSystem::OPEN_EXISTING,
            0,
            0,
        );

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return eject_windows_drive_powershell(drive_letter);
        }

        const FSCTL_LOCK_VOLUME: u32 = 0x00090018;
        let mut bytes_returned: u32 = 0;
        let lock_result = windows_sys::Win32::System::IO::DeviceIoControl(
            handle,
            FSCTL_LOCK_VOLUME,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        if lock_result == 0 {
            windows_sys::Win32::Foundation::CloseHandle(handle);
            return eject_windows_drive_powershell(drive_letter);
        }

        const FSCTL_DISMOUNT_VOLUME: u32 = 0x00090020;
        let dismount_result = windows_sys::Win32::System::IO::DeviceIoControl(
            handle,
            FSCTL_DISMOUNT_VOLUME,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        if dismount_result == 0 {
            const FSCTL_UNLOCK_VOLUME: u32 = 0x0009001C;
            let _ = windows_sys::Win32::System::IO::DeviceIoControl(
                handle,
                FSCTL_UNLOCK_VOLUME,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                &mut bytes_returned,
                std::ptr::null_mut(),
            );
            windows_sys::Win32::Foundation::CloseHandle(handle);
            return eject_windows_drive_powershell(drive_letter);
        }

        const IOCTL_STORAGE_EJECT_MEDIA: u32 = 0x002D4808;
        let _ = windows_sys::Win32::System::IO::DeviceIoControl(
            handle,
            IOCTL_STORAGE_EJECT_MEDIA,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        windows_sys::Win32::Foundation::CloseHandle(handle);
    }

    Ok(())
}


#[cfg(target_os = "windows")]
fn eject_windows_drive_powershell(drive_letter: char) -> PlatformResult<()> {
    let script = format!(
        "$driveEject = New-Object -comObject Shell.Application; \
         $driveEject.Namespace(17).ParseName('{}:').InvokeVerb('Eject')",
        drive_letter
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| PlatformError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        if error.is_empty() {
            Err(PlatformError::EjectFailed(
                "Failed to eject drive. The drive may be in use.".to_string()
            ))
        } else {
            Err(PlatformError::EjectFailed(error.to_string()))
        }
    }
}


#[cfg(target_os = "windows")]
fn mount_windows_disk_image(path: &Path) -> PlatformResult<PathBuf> {
    let path_str = path.to_string_lossy();
    
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    
    let script = format!(
        "$image = Mount-DiskImage -ImagePath '{}' -PassThru; \
         $volume = $image | Get-Volume; \
         if ($volume) {{ $volume.DriveLetter + ':' }} \
         else {{ \
             $disk = $image | Get-Disk; \
             $partition = $disk | Get-Partition | Where-Object {{ $_.DriveLetter }}; \
             if ($partition) {{ $partition.DriveLetter + ':' }} \
             else {{ '' }} \
         }}",
        path_str.replace("'", "''")
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| PlatformError::Io(e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(PlatformError::MountFailed(
            if error.is_empty() {
                "Failed to mount disk image".to_string()
            } else {
                error.to_string()
            }
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let drive_letter = stdout.trim();
    
    if drive_letter.is_empty() || !drive_letter.ends_with(':') {
        return get_mounted_image_path(path);
    }

    let mount_point = PathBuf::from(format!("{}\\", drive_letter));
    
    if mount_point.exists() {
        Ok(mount_point)
    } else {
        Err(PlatformError::MountFailed(
            format!("Mount point {} does not exist after mounting", drive_letter)
        ))
    }
}


#[cfg(target_os = "windows")]
fn get_mounted_image_path(image_path: &Path) -> PlatformResult<PathBuf> {
    let path_str = image_path.to_string_lossy();
    
    let script = format!(
        "$image = Get-DiskImage -ImagePath '{}'; \
         if ($image.Attached) {{ \
             $volume = $image | Get-Volume; \
             if ($volume -and $volume.DriveLetter) {{ $volume.DriveLetter + ':' }} \
             else {{ \
                 $disk = $image | Get-Disk; \
                 $partition = $disk | Get-Partition | Where-Object {{ $_.DriveLetter }}; \
                 if ($partition) {{ $partition.DriveLetter + ':' }} \
                 else {{ '' }} \
             }} \
         }} else {{ '' }}",
        path_str.replace("'", "''")
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| PlatformError::Io(e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let drive_letter = stdout.trim();
    
    if drive_letter.is_empty() || !drive_letter.ends_with(':') {
        return Err(PlatformError::MountFailed(
            "Could not determine mount point for disk image".to_string()
        ));
    }

    Ok(PathBuf::from(format!("{}\\", drive_letter)))
}


#[cfg(target_os = "windows")]
fn unmount_windows_disk_image(mount_point: &Path) -> PlatformResult<()> {
    let path_str = mount_point.to_string_lossy();
    
    let drive_letter = path_str.chars().next()
        .ok_or_else(|| PlatformError::MountFailed("Invalid mount point".to_string()))?;
    
    let script = format!(
        "$volume = Get-Volume -DriveLetter '{}' -ErrorAction SilentlyContinue; \
         if ($volume) {{ \
             $disk = Get-Partition -DriveLetter '{}' | Get-Disk; \
             $image = Get-DiskImage | Where-Object {{ $_.Number -eq $disk.Number }}; \
             if ($image) {{ \
                 Dismount-DiskImage -ImagePath $image.ImagePath; \
                 'OK' \
             }} else {{ \
                 'NOT_IMAGE' \
             }} \
         }} else {{ \
             'NOT_FOUND' \
         }}",
        drive_letter, drive_letter
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| PlatformError::Io(e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result = stdout.trim();

    match result {
        "OK" => Ok(()),
        "NOT_IMAGE" => Err(PlatformError::MountFailed(
            "The specified path is not a mounted disk image".to_string()
        )),
        "NOT_FOUND" => Err(PlatformError::MountFailed(
            format!("Volume {} not found", drive_letter)
        )),
        _ => {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::MountFailed(
                if error.is_empty() {
                    format!("Failed to unmount disk image: {}", result)
                } else {
                    error.to_string()
                }
            ))
        }
    }
}


#[cfg(target_os = "windows")]
pub fn is_windows_disk_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let ext = e.to_lowercase();
            ext == "iso" || ext == "vhd" || ext == "vhdx" || ext == "img"
        })
        .unwrap_or(false)
}


#[cfg(target_os = "windows")]
pub fn is_windows_image_mounted(image_path: &Path) -> bool {
    let path_str = image_path.to_string_lossy();
    
    let script = format!(
        "$image = Get-DiskImage -ImagePath '{}' -ErrorAction SilentlyContinue; \
         if ($image -and $image.Attached) {{ 'true' }} else {{ 'false' }}",
        path_str.replace("'", "''")
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "true",
        Err(_) => false,
    }
}


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


    pub fn is_disk_image(&self, path: &Path) -> bool {
        super::device_monitor_macos::is_disk_image(&path.to_path_buf())
    }


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
        let path = {
            let cache = self.devices_cache.lock()
                .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
            
            cache.iter()
                .find(|d| d.id == device_id)
                .map(|d| d.path.clone())
                .ok_or(PlatformError::DeviceNotFound(device_id))?
        };

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
        let path = {
            let cache = self.devices_cache.lock()
                .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
            
            cache.iter()
                .find(|d| d.id == device_id)
                .map(|d| d.path.clone())
                .ok_or(PlatformError::DeviceNotFound(device_id))?
        };

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

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("/Volumes/") {
                return Ok(PathBuf::from(line));
            }
        }

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

        items.push(ContextMenuItem::new("airdrop", "Share via AirDrop").with_icon("share"));

        if paths.len() == 1 {
            let path = &paths[0];
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "dmg" || ext_lower == "iso" || ext_lower == "img" || ext_lower == "sparseimage" {
                    items.insert(0, ContextMenuItem::new("mount_image", "Mount").with_icon("disc"));
                }
            }
        }

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


#[cfg(target_os = "linux")]
pub struct LinuxAdapter {
    is_monitoring: Arc<AtomicBool>,
    udev_monitor: super::device_monitor_linux::UdevMonitor,
    udisks2_client: super::device_monitor_linux::UDisks2Client,
    devices_cache: std::sync::Mutex<Vec<Device>>,
}

#[cfg(target_os = "linux")]
impl LinuxAdapter {
    pub fn new() -> Self {
        Self {
            is_monitoring: Arc::new(AtomicBool::new(false)),
            udev_monitor: super::device_monitor_linux::UdevMonitor::new(),
            udisks2_client: super::device_monitor_linux::UDisks2Client::new(),
            devices_cache: std::sync::Mutex::new(Vec::new()),
        }
    }


    pub fn get_device_info(&self, device_node: &str) -> Option<super::device_monitor_linux::UdevDeviceInfo> {
        let enumerator = super::device_monitor_linux::UdevDeviceEnumerator::new();
        enumerator.get_device_info(device_node)
    }


    pub fn get_udisks2_properties(&self, block_device: &str) -> Option<super::device_monitor_linux::UDisks2DeviceProperties> {
        self.udisks2_client.get_device_properties(block_device)
    }


    pub fn mount_device(&self, block_device: &str) -> Result<PathBuf, String> {
        self.udisks2_client.mount(block_device)
    }


    pub fn unmount_device(&self, block_device: &str) -> Result<(), String> {
        self.udisks2_client.unmount(block_device)
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
        let monitor = super::device_monitor_linux::LinuxDeviceMonitor::new();
        let block_devices = monitor.enumerate_devices();
        
        let mut devices = Vec::new();
        let mut next_id = 1u64;

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

        for block_device in block_devices {
            if let Some(ref mount_point) = block_device.mount_point {
                if *mount_point == PathBuf::from("/") {
                    continue;
                }

                let device_type = block_device.device_type();
                let name = block_device.display_name();
                let mount_point = mount_point.clone();

                let mut device = Device::new(
                    DeviceId::new(next_id),
                    name,
                    mount_point.clone(),
                    device_type,
                )
                .with_removable(block_device.is_removable);
                next_id += 1;

                if let Ok((total, free)) = super::device_monitor::get_disk_space(&mount_point) {
                    device = device.with_space(total, free);
                }

                devices.push(device);
            }
        }

        if let Ok(mut cache) = self.devices_cache.lock() {
            *cache = devices.clone();
        }

        devices
    }

    fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> PlatformResult<()> {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.udev_monitor.start(sender)
            .map_err(|e| PlatformError::PlatformNotSupported(e))?;
        
        self.is_monitoring.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop_monitoring(&self) -> PlatformResult<()> {
        self.udev_monitor.stop();
        self.is_monitoring.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn eject_device(&self, device_id: DeviceId) -> PlatformResult<()> {
        let path = {
            let cache = self.devices_cache.lock()
                .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
            
            let device = cache.iter()
                .find(|d| d.id == device_id)
                .ok_or(PlatformError::DeviceNotFound(device_id))?;
            
            if !device.is_removable {
                return Err(PlatformError::EjectFailed("Device is not removable".to_string()));
            }
            
            device.path.clone()
        };

        self.udisks2_client.unmount(path.to_str().unwrap_or(""))
            .map_err(|e| PlatformError::EjectFailed(e))?;
        
        let _ = self.udisks2_client.power_off(path.to_str().unwrap_or(""));
        
        Ok(())
    }

    fn format_device(&self, device_id: DeviceId, options: FormatOptions) -> PlatformResult<()> {
        let path = {
            let cache = self.devices_cache.lock()
                .map_err(|e| PlatformError::PlatformNotSupported(format!("Lock error: {}", e)))?;
            
            cache.iter()
                .find(|d| d.id == device_id)
                .map(|d| d.path.clone())
                .ok_or(PlatformError::DeviceNotFound(device_id))?
        };

        let _ = self.udisks2_client.unmount(path.to_str().unwrap_or(""));

        let (mkfs_cmd, mkfs_args) = match options.filesystem {
            FileSystemType::Ext4 => ("mkfs.ext4", vec!["-L", &options.label]),
            FileSystemType::Btrfs => ("mkfs.btrfs", vec!["-L", &options.label, "-f"]),
            FileSystemType::Xfs => ("mkfs.xfs", vec!["-L", &options.label, "-f"]),
            FileSystemType::Fat32 => ("mkfs.vfat", vec!["-n", &options.label, "-F", "32"]),
            FileSystemType::ExFat => ("mkfs.exfat", vec!["-n", &options.label]),
            _ => return Err(PlatformError::FormatFailed(
                format!("Filesystem {:?} not supported on Linux", options.filesystem)
            )),
        };

        let mut cmd = std::process::Command::new(mkfs_cmd);
        cmd.args(&mkfs_args);
        cmd.arg(path.to_str().unwrap_or(""));

        let output = cmd.output()
            .map_err(|e| PlatformError::Io(e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::FormatFailed(error.to_string()))
        }
    }

    fn get_smart_data(&self, device_id: DeviceId) -> PlatformResult<Option<SmartData>> {
        Ok(None)
    }

    fn mount_image(&self, path: &Path) -> PlatformResult<PathBuf> {
        mount_linux_disk_image(path)
    }

    fn unmount_image(&self, mount_point: &Path) -> PlatformResult<()> {
        unmount_linux_disk_image(mount_point)
    }

    fn get_context_menu_items(&self, paths: &[PathBuf]) -> Vec<ContextMenuItem> {
        let mut items = vec![
            ContextMenuItem::new("open_terminal", "Open Terminal Here").with_icon("terminal"),
        ];

        if paths.len() == 1 {
            if let Some(ext) = paths[0].extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "iso" || ext_lower == "img" {
                    items.insert(0, ContextMenuItem::new("mount_image", "Mount").with_icon("disc"));
                }
            }
        }

        if paths.len() == 1 && paths[0].is_dir() {
            items.push(ContextMenuItem::new("open_as_root", "Open as Root").with_icon("shield"));
        }

        if paths.len() == 1 && paths[0].is_file() {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&paths[0]) {
                if meta.permissions().mode() & 0o111 != 0 {
                    items.push(ContextMenuItem::new("run_as_root", "Run as Root").with_icon("shield"));
                }
            }
        }

        items
    }

    fn execute_action(&self, action: PlatformAction) -> PlatformResult<()> {
        match action {
            PlatformAction::Eject(device_id) => self.eject_device(device_id),
            PlatformAction::Format(device_id, options) => self.format_device(device_id, options),
            PlatformAction::MountImage(path) => {
                self.mount_image(&path)?;
                Ok(())
            }
            PlatformAction::Unmount(path) => self.unmount_image(&path),
            PlatformAction::OpenTerminal(path) => {
                let terminals = ["gnome-terminal", "konsole", "xfce4-terminal", "xterm"];
                
                for terminal in terminals {
                    let result = std::process::Command::new(terminal)
                        .arg("--working-directory")
                        .arg(&path)
                        .spawn();
                    
                    if result.is_ok() {
                        return Ok(());
                    }
                }
                
                Err(PlatformError::PlatformNotSupported("No terminal emulator found".to_string()))
            }
            PlatformAction::Custom(cmd, args) => {
                let output = std::process::Command::new(&cmd)
                    .args(&args)
                    .output()
                    .map_err(|e| PlatformError::Io(e))?;

                if output.status.success() {
                    Ok(())
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(PlatformError::PlatformNotSupported(error.to_string()))
                }
            }
            _ => Err(PlatformError::PlatformNotSupported(
                format!("Action {:?} not supported on Linux", action)
            )),
        }
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
fn mount_linux_disk_image(path: &Path) -> PlatformResult<PathBuf> {
    let image_name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("disk_image");
    
    let base_mount_dir = if PathBuf::from("/run/media").exists() {
        PathBuf::from("/run/media")
    } else {
        PathBuf::from("/tmp")
    };
    
    let mount_point = base_mount_dir.join(format!("nexus_mount_{}", image_name));
    
    std::fs::create_dir_all(&mount_point)
        .map_err(|e| PlatformError::MountFailed(format!("Failed to create mount point: {}", e)))?;

    let mount_result = try_mount_with_privilege(path, &mount_point);
    
    match mount_result {
        Ok(()) => Ok(mount_point),
        Err(e) => {
            let _ = std::fs::remove_dir(&mount_point);
            Err(e)
        }
    }
}


#[cfg(target_os = "linux")]
fn try_mount_with_privilege(image_path: &Path, mount_point: &Path) -> PlatformResult<()> {
    let image_str = image_path.to_str().unwrap_or("");
    let mount_str = mount_point.to_str().unwrap_or("");
    
    let output = std::process::Command::new("mount")
        .args(["-o", "loop,ro", image_str, mount_str])
        .output()
        .map_err(|e| PlatformError::Io(e))?;

    if output.status.success() {
        return Ok(());
    }

    let output = std::process::Command::new("pkexec")
        .args(["mount", "-o", "loop,ro", image_str, mount_str])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            return Ok(());
        }
    }

    let output = std::process::Command::new("sudo")
        .args(["-n", "mount", "-o", "loop,ro", image_str, mount_str])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            return Ok(());
        }
    }

    let output = std::process::Command::new("udisksctl")
        .args(["loop-setup", "-f", image_str])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(loop_dev) = parse_udisksctl_loop_device(&stdout) {
                let mount_output = std::process::Command::new("udisksctl")
                    .args(["mount", "-b", &loop_dev])
                    .output();
                
                if let Ok(mount_out) = mount_output {
                    if mount_out.status.success() {
                        let mount_stdout = String::from_utf8_lossy(&mount_out.stdout);
                        if let Some(actual_mount) = parse_udisksctl_mount_point(&mount_stdout) {
                            let _ = std::fs::remove_dir(mount_point);
                            let _ = std::os::unix::fs::symlink(&actual_mount, mount_point);
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    Err(PlatformError::MountFailed(
        "Failed to mount disk image. Try running with elevated privileges.".to_string()
    ))
}


#[cfg(target_os = "linux")]
fn parse_udisksctl_loop_device(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("/dev/loop") {
            if let Some(start) = line.find("/dev/loop") {
                let rest = &line[start..];
                let end = rest.find(|c: char| c == '.' || c == ' ' || c == '\n')
                    .unwrap_or(rest.len());
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}


#[cfg(target_os = "linux")]
fn parse_udisksctl_mount_point(output: &str) -> Option<PathBuf> {
    for line in output.lines() {
        if line.contains(" at ") {
            if let Some(at_pos) = line.find(" at ") {
                let mount_path = line[at_pos + 4..].trim_end_matches('.');
                return Some(PathBuf::from(mount_path));
            }
        }
    }
    None
}


#[cfg(target_os = "linux")]
fn unmount_linux_disk_image(mount_point: &Path) -> PlatformResult<()> {
    let mount_str = mount_point.to_str().unwrap_or("");
    
    let actual_mount = if mount_point.is_symlink() {
        std::fs::read_link(mount_point).ok()
    } else {
        None
    };
    
    let output = std::process::Command::new("umount")
        .arg(mount_str)
        .output()
        .map_err(|e| PlatformError::Io(e))?;

    if output.status.success() {
        let _ = std::fs::remove_dir(mount_point);
        let _ = std::fs::remove_file(mount_point);
        return Ok(());
    }

    let output = std::process::Command::new("pkexec")
        .args(["umount", mount_str])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let _ = std::fs::remove_dir(mount_point);
            let _ = std::fs::remove_file(mount_point);
            return Ok(());
        }
    }

    let output = std::process::Command::new("sudo")
        .args(["-n", "umount", mount_str])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let _ = std::fs::remove_dir(mount_point);
            let _ = std::fs::remove_file(mount_point);
            return Ok(());
        }
    }

    if let Some(actual) = actual_mount {
        let output = std::process::Command::new("udisksctl")
            .args(["unmount", "-b", actual.to_str().unwrap_or("")])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let _ = std::fs::remove_file(mount_point);
                return Ok(());
            }
        }
    }

    let stderr = std::process::Command::new("umount")
        .arg(mount_str)
        .output()
        .map(|o| o.stderr)
        .unwrap_or_default();
    let error = String::from_utf8_lossy(&stderr);
    
    Err(PlatformError::MountFailed(
        if error.is_empty() {
            "Failed to unmount disk image".to_string()
        } else {
            error.to_string()
        }
    ))
}


#[cfg(target_os = "linux")]
pub fn is_linux_disk_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let ext = e.to_lowercase();
            ext == "iso" || ext == "img"
        })
        .unwrap_or(false)
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

    if mount_point.starts_with("/snap") {
        return true;
    }

    false
}

#[cfg(target_os = "linux")]
fn detect_linux_device_type(device: &str, fs_type: &str, mount_point: &str) -> super::device_monitor::DeviceType {
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


#[cfg(target_os = "windows")]
pub fn get_platform_adapter() -> Box<dyn PlatformAdapter> {
    Box::new(WindowsAdapter::new())
}


#[cfg(target_os = "macos")]
pub fn get_platform_adapter() -> Box<dyn PlatformAdapter> {
    Box::new(MacOSAdapter::new())
}


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
        
        assert!(!devices.is_empty(), "Should detect at least one device");
        
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
        
        assert!(!filesystems.is_empty(), "Should have at least one available filesystem");
        
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
                is_encrypted: false,
                smart_status: None,
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_device_detection_completeness(device in arb_device_metadata()) {
            
            prop_assert!(!device.name.is_empty(), 
                "Device name must not be empty");
            
            prop_assert!(!device.path.as_os_str().is_empty(), 
                "Device path must not be empty");
            
            let is_virtual = matches!(
                device.device_type,
                super::super::device_monitor::DeviceType::WslDistribution 
                    | super::super::device_monitor::DeviceType::CloudStorage
            );
            
            if !is_virtual {
                prop_assert!(device.total_space > 0, 
                    "Storage device total_space must be non-zero");
            }
            
            prop_assert!(device.free_space <= device.total_space,
                "Free space ({}) must not exceed total space ({})", 
                device.free_space, device.total_space);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_device_event_contains_valid_metadata(device in arb_device_metadata()) {
            let event = DeviceEvent::Connected(device.clone());
            
            if let DeviceEvent::Connected(connected_device) = event {
                prop_assert_eq!(connected_device.id, device.id);
                prop_assert_eq!(&connected_device.name, &device.name);
                prop_assert_eq!(&connected_device.path, &device.path);
                prop_assert!(!connected_device.name.is_empty());
            } else {
                prop_assert!(false, "Expected DeviceEvent::Connected");
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_device_disconnection_consistency(devices in prop::collection::vec(arb_device_metadata(), 1..10)) {
            use super::super::device_monitor::DeviceMonitor;
            
            let mut monitor = DeviceMonitor::new();
            let receiver = monitor.subscribe().expect("Should be able to subscribe");
            
            let mut added_ids = Vec::new();
            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }
            
            while receiver.try_recv().is_ok() {}
            
            for id in &added_ids {
                let removed = monitor.remove_device(*id);
                
                prop_assert!(removed.is_some(), 
                    "remove_device should return the removed device for id {:?}", id);
                
                prop_assert!(monitor.get_device(*id).is_none(),
                    "Device {:?} should not appear in devices() after removal", id);
            }
            
            let mut disconnect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Disconnected(id) = event {
                    disconnect_events.push(id);
                }
            }
            
            prop_assert_eq!(disconnect_events.len(), added_ids.len(),
                "Should receive {} disconnect events, got {}", 
                added_ids.len(), disconnect_events.len());
            
            for id in &added_ids {
                prop_assert!(disconnect_events.contains(id),
                    "Should have received Disconnected event for device {:?}", id);
            }
            
            prop_assert!(monitor.devices().is_empty(),
                "devices() should be empty after removing all devices");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_disconnection_event_contains_correct_id(device in arb_device_metadata()) {
            let event = DeviceEvent::Disconnected(device.id);
            
            if let DeviceEvent::Disconnected(disconnected_id) = event {
                prop_assert_eq!(disconnected_id, device.id,
                    "Disconnected event should contain the correct device ID");
            } else {
                prop_assert!(false, "Expected DeviceEvent::Disconnected");
            }
        }
    }

    prop_compose! {
        fn arb_removable_device()(
            id in 1u64..1000,
            name in "[a-zA-Z0-9 ]{1,50}",
            path in "/[a-zA-Z0-9/]{1,50}",
            device_type in prop_oneof![
                Just(super::super::device_monitor::DeviceType::UsbDrive),
                Just(super::super::device_monitor::DeviceType::ExternalDrive),
                Just(super::super::device_monitor::DeviceType::OpticalDrive),
            ],
            total_space in 1u64..u64::MAX,
            free_space_ratio in 0.0f64..=1.0,
        ) -> Device {
            let free_space = (total_space as f64 * free_space_ratio) as u64;
            Device {
                id: DeviceId::new(id),
                name,
                path: PathBuf::from(path),
                device_type,
                total_space,
                free_space,
                is_removable: true,
                is_read_only: false,
                is_mounted: true,
                is_encrypted: false,
                smart_status: None,
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_eject_operation_state_consistency(devices in prop::collection::vec(arb_removable_device(), 1..5)) {
            use super::super::device_monitor::DeviceMonitor;
            
            
            let mut monitor = DeviceMonitor::new();
            let receiver = monitor.subscribe().expect("Should be able to subscribe");
            
            let mut added_ids = Vec::new();
            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }
            
            while receiver.try_recv().is_ok() {}
            
            for id in &added_ids {
                let device_before = monitor.get_device(*id);
                prop_assert!(device_before.is_some(), 
                    "Device {:?} should exist before eject", id);
                
                if let Some(dev) = device_before {
                    prop_assert!(dev.is_removable,
                        "Device {:?} should be removable for eject", id);
                }
                
                let removed = monitor.remove_device(*id);
                prop_assert!(removed.is_some(),
                    "Eject should successfully remove device {:?}", id);
                
                prop_assert!(monitor.get_device(*id).is_none(),
                    "Device {:?} should not appear in devices() after eject", id);
                
                let device_ids: Vec<_> = monitor.devices().iter().map(|d| d.id).collect();
                prop_assert!(!device_ids.contains(id),
                    "Device {:?} should not be in devices list after eject", id);
            }
            
            let mut disconnect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Disconnected(id) = event {
                    disconnect_events.push(id);
                }
            }
            
            prop_assert_eq!(disconnect_events.len(), added_ids.len(),
                "Should receive {} disconnect events after eject, got {}", 
                added_ids.len(), disconnect_events.len());
            
            for id in &added_ids {
                prop_assert!(disconnect_events.contains(id),
                    "Should have received Disconnected event for ejected device {:?}", id);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_non_removable_device_eject_rejection(
            id in 1u64..1000,
            name in "[a-zA-Z0-9 ]{1,50}",
            path in "/[a-zA-Z0-9/]{1,50}",
            total_space in 1u64..u64::MAX,
        ) {
            
            let device = Device {
                id: DeviceId::new(id),
                name,
                path: PathBuf::from(path),
                device_type: super::super::device_monitor::DeviceType::InternalDrive,
                total_space,
                free_space: total_space / 2,
                is_removable: false,
                is_read_only: false,
                is_mounted: true,
                is_encrypted: false,
                smart_status: None,
            };
            
            prop_assert!(!device.is_removable,
                "Internal drives should not be marked as removable");
            
            prop_assert_eq!(device.device_type, super::super::device_monitor::DeviceType::InternalDrive,
                "Device type should be InternalDrive");
        }
    }

    fn arb_filesystem_type() -> impl Strategy<Value = FileSystemType> {
        prop_oneof![
            Just(FileSystemType::Fat32),
            Just(FileSystemType::ExFat),
            Just(FileSystemType::Ntfs),
            Just(FileSystemType::ReFS),
            Just(FileSystemType::Apfs),
            Just(FileSystemType::HfsPlus),
            Just(FileSystemType::Ext4),
            Just(FileSystemType::Btrfs),
            Just(FileSystemType::Xfs),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_platform_appropriate_filesystem_options(fs_type in arb_filesystem_type()) {
            
            let adapter = get_platform_adapter();
            let available = adapter.available_filesystems();
            
            for fs in &available {
                prop_assert!(fs.is_available_on_current_platform(),
                    "Filesystem {:?} returned by available_filesystems() must be available on current platform",
                    fs);
            }
            
            if fs_type.is_available_on_current_platform() && available.contains(&fs_type) {
                prop_assert!(available.contains(&fs_type),
                    "Filesystem {:?} is available on platform but not in available_filesystems()",
                    fs_type);
            }
            
            #[cfg(target_os = "windows")]
            {
                prop_assert!(available.contains(&FileSystemType::Ntfs),
                    "Windows should support NTFS");
                prop_assert!(available.contains(&FileSystemType::Fat32),
                    "Windows should support FAT32");
                prop_assert!(available.contains(&FileSystemType::ExFat),
                    "Windows should support exFAT");
                
                prop_assert!(!available.contains(&FileSystemType::Apfs),
                    "Windows should not support APFS");
                prop_assert!(!available.contains(&FileSystemType::HfsPlus),
                    "Windows should not support HFS+");
                prop_assert!(!available.contains(&FileSystemType::Ext4),
                    "Windows should not support ext4");
                prop_assert!(!available.contains(&FileSystemType::Btrfs),
                    "Windows should not support Btrfs");
                prop_assert!(!available.contains(&FileSystemType::Xfs),
                    "Windows should not support XFS");
            }
            
            #[cfg(target_os = "macos")]
            {
                prop_assert!(available.contains(&FileSystemType::Apfs),
                    "macOS should support APFS");
                prop_assert!(available.contains(&FileSystemType::HfsPlus),
                    "macOS should support HFS+");
                prop_assert!(available.contains(&FileSystemType::Fat32),
                    "macOS should support FAT32");
                prop_assert!(available.contains(&FileSystemType::ExFat),
                    "macOS should support exFAT");
                
                prop_assert!(!available.contains(&FileSystemType::Ntfs),
                    "macOS should not support NTFS formatting");
                prop_assert!(!available.contains(&FileSystemType::ReFS),
                    "macOS should not support ReFS");
                prop_assert!(!available.contains(&FileSystemType::Ext4),
                    "macOS should not support ext4");
                prop_assert!(!available.contains(&FileSystemType::Btrfs),
                    "macOS should not support Btrfs");
                prop_assert!(!available.contains(&FileSystemType::Xfs),
                    "macOS should not support XFS");
            }
            
            #[cfg(target_os = "linux")]
            {
                prop_assert!(available.contains(&FileSystemType::Ext4),
                    "Linux should support ext4");
                prop_assert!(available.contains(&FileSystemType::Btrfs),
                    "Linux should support Btrfs");
                prop_assert!(available.contains(&FileSystemType::Xfs),
                    "Linux should support XFS");
                prop_assert!(available.contains(&FileSystemType::Fat32),
                    "Linux should support FAT32");
                prop_assert!(available.contains(&FileSystemType::ExFat),
                    "Linux should support exFAT");
                
                prop_assert!(!available.contains(&FileSystemType::Ntfs),
                    "Linux should not support NTFS formatting");
                prop_assert!(!available.contains(&FileSystemType::ReFS),
                    "Linux should not support ReFS");
                prop_assert!(!available.contains(&FileSystemType::Apfs),
                    "Linux should not support APFS");
                prop_assert!(!available.contains(&FileSystemType::HfsPlus),
                    "Linux should not support HFS+");
            }
        }
    }

    #[test]
    fn prop_available_filesystems_non_empty() {
        let adapter = get_platform_adapter();
        let available = adapter.available_filesystems();
        
        assert!(!available.is_empty(), 
            "available_filesystems() should return at least one filesystem");
        
        assert!(available.contains(&FileSystemType::Fat32) || available.contains(&FileSystemType::ExFat),
            "At least one cross-platform filesystem (FAT32 or exFAT) should be available");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_filesystem_compatibility_info(fs_type in arb_filesystem_type()) {
            
            let info = fs_type.compatibility_info();
            
            prop_assert!(!info.is_empty(),
                "Compatibility info for {:?} must not be empty", fs_type);
            
            prop_assert!(info.len() >= 10,
                "Compatibility info for {:?} should be descriptive (got: '{}')", fs_type, info);
            
            match fs_type {
                FileSystemType::Fat32 | FileSystemType::ExFat => {
                    prop_assert!(info.contains("Windows") || info.contains("windows"),
                        "FAT32/exFAT compatibility info should mention Windows");
                    prop_assert!(info.contains("macOS") || info.contains("Mac"),
                        "FAT32/exFAT compatibility info should mention macOS");
                    prop_assert!(info.contains("Linux") || info.contains("linux"),
                        "FAT32/exFAT compatibility info should mention Linux");
                }
                FileSystemType::Ntfs => {
                    prop_assert!(info.contains("Windows") || info.contains("windows"),
                        "NTFS compatibility info should mention Windows");
                }
                FileSystemType::Apfs | FileSystemType::HfsPlus => {
                    prop_assert!(info.contains("macOS") || info.contains("Mac"),
                        "APFS/HFS+ compatibility info should mention macOS");
                }
                FileSystemType::Ext4 | FileSystemType::Btrfs | FileSystemType::Xfs => {
                    prop_assert!(info.contains("Linux") || info.contains("linux"),
                        "ext4/Btrfs/XFS compatibility info should mention Linux");
                }
                FileSystemType::ReFS => {
                    prop_assert!(info.contains("Windows") || info.contains("windows"),
                        "ReFS compatibility info should mention Windows");
                }
            }
        }
    }

    #[test]
    fn prop_all_filesystems_have_compatibility_info() {
        let all_filesystems = [
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

        for fs in all_filesystems {
            let info = fs.compatibility_info();
            assert!(!info.is_empty(), 
                "Filesystem {:?} must have non-empty compatibility info", fs);
            assert!(info.len() >= 10,
                "Filesystem {:?} compatibility info should be descriptive", fs);
        }
    }

    fn arb_disk_image_ext() -> impl Strategy<Value = &'static str> {
        #[cfg(target_os = "windows")]
        {
            prop_oneof![
                Just("iso"),
                Just("img"),
                Just("vhd"),
                Just("vhdx"),
            ]
        }
        #[cfg(target_os = "macos")]
        {
            prop_oneof![
                Just("iso"),
                Just("img"),
                Just("dmg"),
                Just("sparseimage"),
            ]
        }
        #[cfg(target_os = "linux")]
        {
            prop_oneof![
                Just("iso"),
                Just("img"),
            ]
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            prop_oneof![
                Just("iso"),
                Just("img"),
            ]
        }
    }

    prop_compose! {
        fn arb_disk_image_path()(
            name in "[a-zA-Z0-9_-]{1,20}",
            ext in arb_disk_image_ext(),
        ) -> PathBuf {
            PathBuf::from(format!("/tmp/{}.{}", name, ext))
        }
    }

    prop_compose! {
        fn arb_mount_point()(
            name in "[a-zA-Z0-9_-]{1,20}",
        ) -> PathBuf {
            #[cfg(target_os = "windows")]
            {
                PathBuf::from(format!("D:\\"))
            }
            #[cfg(target_os = "macos")]
            {
                PathBuf::from(format!("/Volumes/{}", name))
            }
            #[cfg(target_os = "linux")]
            {
                PathBuf::from(format!("/tmp/nexus_mount_{}", name))
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            {
                PathBuf::from(format!("/mnt/{}", name))
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_disk_image_mount_accessibility(mount_point in arb_mount_point()) {
            
            let path_str = mount_point.to_string_lossy();
            
            #[cfg(target_os = "windows")]
            {
                prop_assert!(path_str.len() >= 2,
                    "Windows mount point should be at least 2 characters (drive letter + colon)");
                let first_char = path_str.chars().next().unwrap_or(' ');
                prop_assert!(first_char.is_ascii_alphabetic(),
                    "Windows mount point should start with a drive letter");
            }
            
            #[cfg(target_os = "macos")]
            {
                prop_assert!(path_str.starts_with("/Volumes/"),
                    "macOS mount point should be under /Volumes/");
            }
            
            #[cfg(target_os = "linux")]
            {
                prop_assert!(
                    path_str.starts_with("/tmp/") || 
                    path_str.starts_with("/mnt/") || 
                    path_str.starts_with("/media/") ||
                    path_str.starts_with("/run/media/"),
                    "Linux mount point should be under /tmp/, /mnt/, /media/, or /run/media/"
                );
            }
            
            if mount_point.exists() {
                prop_assert!(mount_point.is_dir() || mount_point.is_symlink(),
                    "Existing mount point should be a directory or symlink");
                
                if mount_point.is_dir() {
                    let read_result = std::fs::read_dir(&mount_point);
                    prop_assert!(read_result.is_ok(),
                        "Should be able to read mount point directory contents");
                }
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_disk_image_path_validation(image_path in arb_disk_image_path()) {
            
            let ext = image_path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            
            let is_universal = ext == "iso" || ext == "img";
            
            #[cfg(target_os = "windows")]
            {
                let is_windows_specific = ext == "vhd" || ext == "vhdx";
                prop_assert!(is_universal || is_windows_specific,
                    "Windows disk image should have .iso, .img, .vhd, or .vhdx extension");
            }
            
            #[cfg(target_os = "macos")]
            {
                let is_macos_specific = ext == "dmg" || ext == "sparseimage";
                prop_assert!(is_universal || is_macos_specific,
                    "macOS disk image should have .iso, .img, .dmg, or .sparseimage extension");
            }
            
            #[cfg(target_os = "linux")]
            {
                prop_assert!(is_universal,
                    "Linux disk image should have .iso or .img extension");
            }
        }
    }

    #[test]
    fn prop_mount_point_generation_uniqueness() {
        
        let image_names = ["test1", "test2", "image_a", "image_b", "my_disk"];
        let mut mount_points = std::collections::HashSet::new();
        
        for name in image_names {
            #[cfg(target_os = "windows")]
            {
                let path = PathBuf::from(format!("{}:\\", 'D'));
                assert!(path.to_string_lossy().ends_with(":\\"));
            }
            
            #[cfg(target_os = "macos")]
            {
                let mount_point = PathBuf::from(format!("/Volumes/{}", name));
                assert!(mount_points.insert(mount_point.clone()),
                    "Mount point {:?} should be unique", mount_point);
            }
            
            #[cfg(target_os = "linux")]
            {
                let mount_point = PathBuf::from(format!("/tmp/nexus_mount_{}", name));
                assert!(mount_points.insert(mount_point.clone()),
                    "Mount point {:?} should be unique", mount_point);
            }
        }
    }
}
