//! Windows device change notifications using message-based monitoring
#![cfg(target_os = "windows")]

use super::device_monitor::{Device, DeviceEvent, DeviceId, DeviceType, get_disk_space};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct WindowsDeviceNotificationMonitor {
    is_running: Arc<AtomicBool>,
    monitor_thread: Option<thread::JoinHandle<()>>,
}

impl WindowsDeviceNotificationMonitor {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            monitor_thread: None,
        }
    }

    pub fn start(&mut self, sender: flume::Sender<DeviceEvent>) -> Result<(), String> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_running.store(true, Ordering::SeqCst);
        let is_running = self.is_running.clone();

        let handle = thread::spawn(move || {
            let mut known_drives = get_current_drives();
            
            while is_running.load(Ordering::SeqCst) {
                thread::sleep(std::time::Duration::from_secs(2));
                
                let current_drives = get_current_drives();
                
                // Check for new drives
                for drive in &current_drives {
                    if !known_drives.contains(drive) {
                        if let Some(device) = create_device_for_drive(*drive) {
                            let _ = sender.send(DeviceEvent::Connected(device));
                        }
                    }
                }
                
                // Check for removed drives
                for drive in &known_drives {
                    if !current_drives.contains(drive) {
                        let _ = sender.send(DeviceEvent::Disconnected(DeviceId::new(*drive as u64)));
                    }
                }
                
                known_drives = current_drives;
            }
        });

        self.monitor_thread = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.monitor_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Default for WindowsDeviceNotificationMonitor {
    fn default() -> Self { Self::new() }
}

impl Drop for WindowsDeviceNotificationMonitor {
    fn drop(&mut self) { self.stop(); }
}

fn get_current_drives() -> Vec<char> {
    let mut drives = Vec::new();
    unsafe {
        let mask = windows_sys::Win32::Storage::FileSystem::GetLogicalDrives();
        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                drives.push((b'A' + i) as char);
            }
        }
    }
    drives
}

fn create_device_for_drive(letter: char) -> Option<Device> {
    let path = PathBuf::from(format!("{}:\\", letter));
    if !path.exists() { return None; }

    let device_type = detect_drive_type(&path);
    let name = get_volume_name(&path)
        .unwrap_or_else(|| format!("Local Disk ({}:)", letter));

    let is_removable = matches!(
        device_type,
        DeviceType::UsbDrive | DeviceType::ExternalDrive | DeviceType::OpticalDrive
    );

    let mut device = Device::new(DeviceId::new(letter as u64), name, path.clone(), device_type)
        .with_removable(is_removable);

    if let Ok((total, free)) = get_disk_space(&path) {
        device = device.with_space(total, free);
    }

    Some(device)
}

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;
const DRIVE_RAMDISK: u32 = 6;

fn detect_drive_type(path: &PathBuf) -> DeviceType {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        match windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr()) {
            DRIVE_REMOVABLE => DeviceType::UsbDrive,
            DRIVE_FIXED => DeviceType::InternalDrive,
            DRIVE_REMOTE => DeviceType::NetworkDrive,
            DRIVE_CDROM => DeviceType::OpticalDrive,
            DRIVE_RAMDISK => DeviceType::DiskImage,
            _ => DeviceType::ExternalDrive,
        }
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
