/*
 * Linux Device Monitor
 * 
 * Provides device detection and monitoring for Linux systems using:
 * - udev for device enumeration and hotplug events
 * - udisks2 D-Bus interface for mount/unmount operations
 * - /proc/mounts parsing for mounted filesystem information
 * 
 * Requirements: 1.7, 2.8, 26.6, 28.2, 28.3
 */

use super::device_monitor::{
    get_disk_space, Device, DeviceEvent, DeviceId, DeviceType, HealthStatus, SmartAttribute,
    SmartData, smart_attributes,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};


#[derive(Debug, Clone, Default)]
pub struct LinuxBlockDevice {
    pub device_node: String,
    pub device_type: String,
    pub id_fs_type: Option<String>,
    pub id_fs_label: Option<String>,
    pub id_fs_uuid: Option<String>,
    pub id_serial: Option<String>,
    pub id_model: Option<String>,
    pub id_vendor: Option<String>,
    pub id_bus: Option<String>,
    pub is_removable: bool,
    pub is_partition: bool,
    pub size_bytes: u64,
    pub mount_point: Option<PathBuf>,
}

impl LinuxBlockDevice {
    pub fn device_type(&self) -> DeviceType {
        if let Some(ref bus) = self.id_bus {
            if bus.eq_ignore_ascii_case("usb") {
                return DeviceType::UsbDrive;
            }
        }

        if let Some(ref fs_type) = self.id_fs_type {
            let fs = fs_type.to_lowercase();
            if fs == "iso9660" || fs == "udf" {
                return DeviceType::OpticalDrive;
            }
            if ["nfs", "nfs4", "cifs", "smbfs", "sshfs"].contains(&fs.as_str()) {
                return DeviceType::NetworkDrive;
            }
        }

        if self.device_node.contains("sr") || self.device_node.contains("cdrom") {
            return DeviceType::OpticalDrive;
        }

        if self.is_removable {
            return DeviceType::ExternalDrive;
        }

        DeviceType::InternalDrive
    }

    pub fn display_name(&self) -> String {
        if let Some(ref label) = self.id_fs_label {
            if !label.is_empty() {
                return label.clone();
            }
        }

        if let Some(ref model) = self.id_model {
            if !model.is_empty() {
                return model.replace("_", " ");
            }
        }

        if let Some(ref mount) = self.mount_point {
            if let Some(name) = mount.file_name() {
                return name.to_string_lossy().to_string();
            }
        }

        self.device_node
            .trim_start_matches("/dev/")
            .to_uppercase()
    }
}


#[derive(Debug, Clone, Default)]
pub struct UdevDeviceInfo {
    pub device_node: String,
    pub id_fs_type: Option<String>,
    pub id_fs_label: Option<String>,
    pub id_fs_uuid: Option<String>,
    pub id_serial: Option<String>,
    pub id_model: Option<String>,
    pub id_vendor: Option<String>,
    pub id_bus: Option<String>,
    pub is_removable: bool,
    pub size_bytes: u64,
}


pub struct UdevDeviceEnumerator {
    udev: Option<udev::Udev>,
}

impl Default for UdevDeviceEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl UdevDeviceEnumerator {
    pub fn new() -> Self {
        Self {
            udev: udev::Udev::new().ok(),
        }
    }


    pub fn get_device_info(&self, device_node: &str) -> Option<UdevDeviceInfo> {
        let _udev = self.udev.as_ref()?;
        
        let syspath = device_node_to_syspath(device_node)?;
        let device = udev::Device::from_syspath(std::path::Path::new(&syspath)).ok()?;
        
        let is_removable = device
            .property_value("ID_BUS")
            .map(|v| v.to_string_lossy().eq_ignore_ascii_case("usb"))
            .unwrap_or(false)
            || read_sysfs_removable(device_node);

        let size_bytes = device
            .attribute_value("size")
            .and_then(|v| v.to_string_lossy().parse::<u64>().ok())
            .map(|sectors| sectors * 512)
            .unwrap_or(0);

        Some(UdevDeviceInfo {
            device_node: device_node.to_string(),
            id_fs_type: device
                .property_value("ID_FS_TYPE")
                .map(|v| v.to_string_lossy().to_string()),
            id_fs_label: device
                .property_value("ID_FS_LABEL")
                .map(|v| v.to_string_lossy().to_string()),
            id_fs_uuid: device
                .property_value("ID_FS_UUID")
                .map(|v| v.to_string_lossy().to_string()),
            id_serial: device
                .property_value("ID_SERIAL")
                .map(|v| v.to_string_lossy().to_string()),
            id_model: device
                .property_value("ID_MODEL")
                .map(|v| v.to_string_lossy().to_string()),
            id_vendor: device
                .property_value("ID_VENDOR")
                .map(|v| v.to_string_lossy().to_string()),
            id_bus: device
                .property_value("ID_BUS")
                .map(|v| v.to_string_lossy().to_string()),
            is_removable,
            size_bytes,
        })
    }
}


fn device_node_to_syspath(device_node: &str) -> Option<String> {
    let dev_name = device_node.trim_start_matches("/dev/");
    let syspath = format!("/sys/class/block/{}", dev_name);
    if std::path::Path::new(&syspath).exists() {
        Some(syspath)
    } else {
        None
    }
}

fn find_mount_point_for_device(device: &str) -> Option<PathBuf> {
    let mounts = parse_proc_mounts();
    for mount in mounts {
        if mount.device == device || mount.device.ends_with(device.trim_start_matches("/dev/")) {
            return Some(mount.mount_point);
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: PathBuf,
    pub fs_type: String,
    pub options: String,
}


pub fn parse_proc_mounts() -> Vec<MountEntry> {
    let mut mounts = Vec::new();

    if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                mounts.push(MountEntry {
                    device: parts[0].to_string(),
                    mount_point: PathBuf::from(parts[1].replace("\\040", " ")),
                    fs_type: parts[2].to_string(),
                    options: parts[3].to_string(),
                });
            }
        }
    }

    mounts
}


pub fn is_device_removable(device_node: &str) -> bool {
    read_sysfs_removable(device_node)
}


pub fn detect_device_type(device_node: &str, fs_type: &str, mount_point: &str, is_removable: bool) -> DeviceType {
    if ["nfs", "nfs4", "cifs", "smbfs", "sshfs", "fuse.sshfs"].contains(&fs_type) {
        return DeviceType::NetworkDrive;
    }

    if device_node.contains("sr") || device_node.contains("cdrom") || fs_type == "iso9660" || fs_type == "udf" {
        return DeviceType::OpticalDrive;
    }

    if device_node.starts_with("/dev/sd") && is_removable {
        return DeviceType::UsbDrive;
    }

    if mount_point.starts_with("/media") || mount_point.starts_with("/mnt") {
        return DeviceType::ExternalDrive;
    }

    DeviceType::InternalDrive
}


pub fn get_device_name(mount_point: &PathBuf, device_node: &str, label: Option<&str>) -> String {
    if let Some(label) = label {
        if !label.is_empty() {
            return label.to_string();
        }
    }

    if let Ok(entries) = std::fs::read_dir("/dev/disk/by-label") {
        for entry in entries.flatten() {
            if let Ok(target) = std::fs::read_link(entry.path()) {
                let target_str = target.to_string_lossy();
                if device_node.ends_with(&*target_str)
                    || target_str.ends_with(device_node.trim_start_matches("/dev/"))
                {
                    if let Some(name) = entry.file_name().to_str() {
                        return name.replace("\\x20", " ");
                    }
                }
            }
        }
    }

    mount_point
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string()
}


pub struct LinuxDeviceMonitor {
    is_monitoring: Arc<AtomicBool>,
    stop_signal: Arc<AtomicBool>,
    monitor_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    known_devices: Arc<Mutex<HashMap<String, LinuxBlockDevice>>>,
}

impl Default for LinuxDeviceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxDeviceMonitor {
    pub fn new() -> Self {
        Self {
            is_monitoring: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            monitor_thread: Mutex::new(None),
            known_devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }


    pub fn enumerate_devices(&self) -> Vec<LinuxBlockDevice> {
        let mut devices = Vec::new();
        let mount_points = parse_mount_points();

        if let Ok(_udev) = udev::Udev::new() {
            if let Ok(mut enumerator) = udev::Enumerator::new() {
                let _ = enumerator.match_subsystem("block");

                if let Ok(device_list) = enumerator.scan_devices() {
                    for device in device_list {
                        if let Some(block_device) = self.parse_udev_device(&device, &mount_points) {
                            devices.push(block_device);
                        }
                    }
                }
            }
        }

        self.scan_user_mount_directories(&mut devices, &mount_points);

        if let Ok(mut known) = self.known_devices.lock() {
            known.clear();
            for device in &devices {
                known.insert(device.device_node.clone(), device.clone());
            }
        }

        devices
    }

    fn parse_udev_device(
        &self,
        device: &udev::Device,
        mount_points: &HashMap<String, PathBuf>,
    ) -> Option<LinuxBlockDevice> {
        let device_node = device.devnode()?.to_string_lossy().to_string();

        if device_node.contains("/loop")
            || device_node.contains("/ram")
            || (device_node.contains("/dm-") && !mount_points.contains_key(&device_node))
        {
            return None;
        }

        let devtype = device
            .property_value("DEVTYPE")
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_default();

        if devtype != "partition" && devtype != "disk" {
            return None;
        }

        if devtype == "disk" && !mount_points.contains_key(&device_node) {
            let has_partitions = device_node.ends_with(|c: char| c.is_ascii_digit())
                || std::fs::read_dir(format!("/sys/block/{}", device_node.trim_start_matches("/dev/")))
                    .map(|entries| {
                        entries.filter_map(|e| e.ok()).any(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with(device_node.trim_start_matches("/dev/"))
                        })
                    })
                    .unwrap_or(false);

            if has_partitions {
                return None;
            }
        }

        let id_fs_type = device
            .property_value("ID_FS_TYPE")
            .map(|v| v.to_string_lossy().to_string());

        let is_optical = device_node.contains("sr") || device_node.contains("cdrom");
        if id_fs_type.is_none() && !is_optical && !mount_points.contains_key(&device_node) {
            return None;
        }

        let is_removable = device
            .property_value("ID_BUS")
            .map(|v| v.to_string_lossy().eq_ignore_ascii_case("usb"))
            .unwrap_or(false)
            || read_sysfs_removable(&device_node);

        let size_bytes = device
            .attribute_value("size")
            .and_then(|v| v.to_string_lossy().parse::<u64>().ok())
            .map(|sectors| sectors * 512)
            .unwrap_or(0);

        let is_partition = devtype == "partition";
        Some(LinuxBlockDevice {
            device_node: device_node.clone(),
            device_type: devtype,
            id_fs_type,
            id_fs_label: device
                .property_value("ID_FS_LABEL")
                .map(|v| v.to_string_lossy().to_string()),
            id_fs_uuid: device
                .property_value("ID_FS_UUID")
                .map(|v| v.to_string_lossy().to_string()),
            id_serial: device
                .property_value("ID_SERIAL")
                .map(|v| v.to_string_lossy().to_string()),
            id_model: device
                .property_value("ID_MODEL")
                .map(|v| v.to_string_lossy().to_string()),
            id_vendor: device
                .property_value("ID_VENDOR")
                .map(|v| v.to_string_lossy().to_string()),
            id_bus: device
                .property_value("ID_BUS")
                .map(|v| v.to_string_lossy().to_string()),
            is_removable,
            is_partition,
            size_bytes,
            mount_point: mount_points.get(&device_node).cloned(),
        })
    }

    fn scan_user_mount_directories(
        &self,
        devices: &mut Vec<LinuxBlockDevice>,
        mount_points: &HashMap<String, PathBuf>,
    ) {
        let existing_mounts: std::collections::HashSet<_> = devices
            .iter()
            .filter_map(|d| d.mount_point.as_ref())
            .collect();

        for base_path in &["/media", "/mnt"] {
            let base = PathBuf::from(base_path);
            if !base.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if existing_mounts.contains(&path) {
                        continue;
                    }

                    if !is_mount_point(&path) {
                        continue;
                    }

                    let device_node = mount_points
                        .iter()
                        .find(|(_, mp)| *mp == &path)
                        .map(|(dev, _)| dev.clone())
                        .unwrap_or_else(|| format!("unknown:{}", path.display()));

                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    devices.push(LinuxBlockDevice {
                        device_node,
                        device_type: "partition".to_string(),
                        id_fs_label: Some(name),
                        mount_point: Some(path),
                        is_removable: true,
                        ..Default::default()
                    });
                }
            }
        }
    }
}


#[cfg(target_os = "linux")]
pub struct UdevMonitor {
    is_running: Arc<AtomicBool>,
    stop_signal: Arc<AtomicBool>,
}

#[cfg(target_os = "linux")]
impl UdevMonitor {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }


    pub fn start(&self, sender: flume::Sender<DeviceEvent>) -> Result<(), String> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let is_running = self.is_running.clone();
        let stop_signal = self.stop_signal.clone();
        
        stop_signal.store(false, Ordering::SeqCst);

        std::thread::spawn(move || {
            is_running.store(true, Ordering::SeqCst);
            
            let result = Self::monitor_loop(sender, stop_signal.clone());
            
            if let Err(e) = result {
                eprintln!("Udev monitor error: {}", e);
            }
            
            is_running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }


    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);
    }


    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }


    fn monitor_loop(sender: flume::Sender<DeviceEvent>, stop_signal: Arc<AtomicBool>) -> Result<(), String> {
        let _context = udev::Udev::new().map_err(|e| format!("Failed to create udev context: {}", e))?;
        
        let monitor = udev::MonitorBuilder::new()
            .map_err(|e| format!("Failed to create udev monitor: {}", e))?
            .match_subsystem("block")
            .map_err(|e| format!("Failed to match subsystem: {}", e))?
            .listen()
            .map_err(|e| format!("Failed to start listening: {}", e))?;

        use std::os::unix::io::AsRawFd;
        let fd = monitor.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let mut next_id = 1000u64;

        while !stop_signal.load(Ordering::SeqCst) {
            let mut poll_fds = [libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            }];

            let poll_result = unsafe {
                libc::poll(poll_fds.as_mut_ptr(), 1, 500)
            };

            if poll_result <= 0 {
                continue;
            }

            while let Some(event) = monitor.iter().next() {
                let action = event.action().and_then(|a| a.to_str().ok()).unwrap_or("");
                
                let Some(device_node) = event.devnode() else {
                    continue;
                };
                let device_node_str = device_node.to_string_lossy().to_string();

                let dev_type = event.devtype().and_then(|s| s.to_str().ok()).unwrap_or("");
                if dev_type == "disk" {
                    continue;
                }

                match action {
                    "add" => {
                        std::thread::sleep(std::time::Duration::from_millis(500));

                        if let Some(mount_point) = find_mount_point_for_device(&device_node_str) {
                            let udev_enum = UdevDeviceEnumerator::new();
                            let udev_info = udev_enum.get_device_info(&device_node_str);

                            let is_removable = udev_info
                                .as_ref()
                                .map(|i| i.is_removable)
                                .unwrap_or_else(|| is_device_removable(&device_node_str));

                            let fs_type = udev_info
                                .as_ref()
                                .and_then(|i| i.id_fs_type.as_deref())
                                .unwrap_or("");

                            let device_type = detect_device_type(
                                &device_node_str,
                                fs_type,
                                mount_point.to_str().unwrap_or(""),
                                is_removable,
                            );

                            let name = get_device_name(
                                &mount_point,
                                &device_node_str,
                                udev_info.as_ref().and_then(|i| i.id_fs_label.as_deref()),
                            );

                            let mut device = Device::new(
                                DeviceId::new(next_id),
                                name,
                                mount_point.clone(),
                                device_type,
                            )
                            .with_removable(is_removable);
                            next_id += 1;

                            if let Ok((total, free)) = get_disk_space(&mount_point) {
                                device = device.with_space(total, free);
                            }

                            let _ = sender.send(DeviceEvent::Connected(device));
                        }
                    }
                    "remove" => {
                        let id = DeviceId::new(hash_string(&device_node_str));
                        let _ = sender.send(DeviceEvent::Disconnected(id));
                    }
                    "change" => {
                        if let Some(mount_point) = find_mount_point_for_device(&device_node_str) {
                            let udev_enum = UdevDeviceEnumerator::new();
                            let udev_info = udev_enum.get_device_info(&device_node_str);

                            let is_removable = udev_info
                                .as_ref()
                                .map(|i| i.is_removable)
                                .unwrap_or_else(|| is_device_removable(&device_node_str));

                            let fs_type = udev_info
                                .as_ref()
                                .and_then(|i| i.id_fs_type.as_deref())
                                .unwrap_or("");

                            let device_type = detect_device_type(
                                &device_node_str,
                                fs_type,
                                mount_point.to_str().unwrap_or(""),
                                is_removable,
                            );

                            let name = get_device_name(
                                &mount_point,
                                &device_node_str,
                                udev_info.as_ref().and_then(|i| i.id_fs_label.as_deref()),
                            );

                            let mut device = Device::new(
                                DeviceId::new(hash_string(&device_node_str)),
                                name,
                                mount_point.clone(),
                                device_type,
                            )
                            .with_removable(is_removable);

                            if let Ok((total, free)) = get_disk_space(&mount_point) {
                                device = device.with_space(total, free);
                            }

                            let _ = sender.send(DeviceEvent::Updated(device));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Default for UdevMonitor {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(target_os = "linux")]
fn find_mount_point_for_device_linux(device_node: &str) -> Option<PathBuf> {
    let mounts = parse_proc_mounts();
    
    for mount in &mounts {
        if mount.device == device_node {
            return Some(mount.mount_point.clone());
        }
    }
    
    for prefix in &["/dev/disk/by-uuid/", "/dev/disk/by-label/"] {
        if let Ok(entries) = std::fs::read_dir(prefix) {
            for entry in entries.flatten() {
                if let Ok(target) = std::fs::read_link(entry.path()) {
                    let target_path = if target.is_absolute() {
                        target
                    } else {
                        PathBuf::from(prefix).join(&target)
                    };
                    
                    if let Ok(canonical) = std::fs::canonicalize(&target_path) {
                        if canonical.to_string_lossy() == device_node {
                            let symlink_path = entry.path().to_string_lossy().to_string();
                            for mount in &mounts {
                                if mount.device == symlink_path {
                                    return Some(mount.mount_point.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}


#[cfg(target_os = "linux")]
fn hash_string(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}


impl LinuxDeviceMonitor {
    pub fn start_monitoring(&self, sender: flume::Sender<DeviceEvent>) -> Result<(), String> {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.stop_signal.store(false, Ordering::SeqCst);
        self.is_monitoring.store(true, Ordering::SeqCst);

        let stop_signal = Arc::clone(&self.stop_signal);
        let known_devices = Arc::clone(&self.known_devices);
        let is_monitoring = Arc::clone(&self.is_monitoring);

        let handle = std::thread::spawn(move || {
            let udev = match udev::Udev::new() {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("Failed to create udev context: {}", e);
                    is_monitoring.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut monitor = match udev::MonitorBuilder::new(&udev) {
                Ok(builder) => match builder.match_subsystem("block") {
                    Ok(b) => match b.listen() {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("Failed to start udev monitor: {}", e);
                            is_monitoring.store(false, Ordering::SeqCst);
                            return;
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to match subsystem: {}", e);
                        is_monitoring.store(false, Ordering::SeqCst);
                        return;
                    }
                },
                Err(e) => {
                    eprintln!("Failed to create udev monitor: {}", e);
                    is_monitoring.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let poll_fd = match monitor.as_raw_fd() {
                fd if fd >= 0 => fd,
                _ => {
                    eprintln!("Invalid file descriptor for udev monitor");
                    is_monitoring.store(false, Ordering::SeqCst);
                    return;
                }
            };

            while !stop_signal.load(Ordering::SeqCst) {
                let mut poll_fds = [libc::pollfd {
                    fd: poll_fd,
                    events: libc::POLLIN,
                    revents: 0,
                }];

                let poll_result = unsafe { libc::poll(poll_fds.as_mut_ptr(), 1, 500) };

                if poll_result < 0 {
                    continue;
                }

                if poll_result == 0 || poll_fds[0].revents & libc::POLLIN == 0 {
                    continue;
                }

                if let Some(event) = monitor.iter().next() {
                    let action = event.action().map(|a| a.to_string_lossy().to_string());
                    let device_node = event
                        .devnode()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();

                    if device_node.is_empty() {
                        continue;
                    }

                    match action.as_deref() {
                        Some("add") => {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            
                            let mount_points = parse_mount_points();
                            if let Some(mount_point) = mount_points.get(&device_node) {
                                let device = create_device_from_udev_event(&event, mount_point);
                                
                                if let Ok(mut known) = known_devices.lock() {
                                    known.insert(device_node.clone(), LinuxBlockDevice {
                                        device_node: device_node.clone(),
                                        mount_point: Some(mount_point.clone()),
                                        ..Default::default()
                                    });
                                }
                                
                                let _ = sender.send(DeviceEvent::Connected(device));
                            }
                        }
                        Some("remove") => {
                            if let Ok(mut known) = known_devices.lock() {
                                if known.remove(&device_node).is_some() {
                                    let id = device_node_to_id(&device_node);
                                    let _ = sender.send(DeviceEvent::Disconnected(id));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            is_monitoring.store(false, Ordering::SeqCst);
        });

        if let Ok(mut thread) = self.monitor_thread.lock() {
            *thread = Some(handle);
        }

        Ok(())
    }

    pub fn stop_monitoring(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);

        if let Ok(mut thread) = self.monitor_thread.lock() {
            if let Some(handle) = thread.take() {
                let _ = handle.join();
            }
        }

        self.is_monitoring.store(false, Ordering::SeqCst);
    }
}

use std::os::unix::io::AsRawFd;

fn create_device_from_udev_event(event: &udev::Event, mount_point: &PathBuf) -> Device {
    let device_node = event
        .devnode()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let label = event
        .property_value("ID_FS_LABEL")
        .map(|v| v.to_string_lossy().to_string());

    let model = event
        .property_value("ID_MODEL")
        .map(|v| v.to_string_lossy().to_string());

    let name = label
        .or(model)
        .unwrap_or_else(|| {
            mount_point
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    let is_usb = event
        .property_value("ID_BUS")
        .map(|v| v.to_string_lossy().eq_ignore_ascii_case("usb"))
        .unwrap_or(false);

    let device_type = if is_usb {
        DeviceType::UsbDrive
    } else if device_node.contains("sr") || device_node.contains("cdrom") {
        DeviceType::OpticalDrive
    } else {
        DeviceType::ExternalDrive
    };

    let id = device_node_to_id(&device_node);

    let mut device = Device::new(id, name, mount_point.clone(), device_type).with_removable(true);

    if let Ok((total, free)) = get_disk_space(mount_point) {
        device = device.with_space(total, free);
    }

    device
}

fn device_node_to_id(device_node: &str) -> DeviceId {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    device_node.hash(&mut hasher);
    DeviceId::new(hasher.finish())
}

fn parse_mount_points() -> HashMap<String, PathBuf> {
    let mut mounts = HashMap::new();

    if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let device = parts[0].to_string();
                let mount_point = PathBuf::from(parts[1].replace("\\040", " "));
                mounts.insert(device, mount_point);
            }
        }
    }

    mounts
}

fn read_sysfs_removable(device_node: &str) -> bool {
    let base_device = device_node
        .trim_start_matches("/dev/")
        .trim_end_matches(char::is_numeric);

    let removable_path = format!("/sys/block/{}/removable", base_device);
    std::fs::read_to_string(&removable_path)
        .map(|content| content.trim() == "1")
        .unwrap_or(false)
}

fn is_mount_point(path: &PathBuf) -> bool {
    use std::os::unix::fs::MetadataExt;

    if let (Ok(path_meta), Some(parent)) = (std::fs::metadata(path), path.parent()) {
        if let Ok(parent_meta) = std::fs::metadata(parent) {
            return path_meta.dev() != parent_meta.dev();
        }
    }
    false
}


pub struct UDisks2Client {
    runtime: Option<tokio::runtime::Runtime>,
}

impl Default for UDisks2Client {
    fn default() -> Self {
        Self::new()
    }
}

impl UDisks2Client {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();

        Self { runtime }
    }


    pub fn unmount(&self, device_path: &str) -> Result<(), String> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or("Tokio runtime not available")?;

        let device_path = device_path.to_string();

        runtime.block_on(async {
            let connection = zbus::Connection::system()
                .await
                .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

            let object_path = device_to_dbus_path(&device_path);

            let proxy = zbus::Proxy::new(
                &connection,
                "org.freedesktop.UDisks2",
                object_path.as_str(),
                "org.freedesktop.UDisks2.Filesystem",
            )
            .await
            .map_err(|e| format!("Failed to create proxy: {}", e))?;

            let options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();

            proxy
                .call_method("Unmount", &(options,))
                .await
                .map_err(|e| format!("Unmount failed: {}", e))?;

            Ok(())
        })
    }


    pub fn power_off(&self, device_path: &str) -> Result<(), String> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or("Tokio runtime not available")?;

        let device_path = device_path.to_string();

        runtime.block_on(async {
            let connection = zbus::Connection::system()
                .await
                .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

            let base_device = device_path
                .trim_start_matches("/dev/")
                .trim_end_matches(char::is_numeric);
            let drive_path = format!("/org/freedesktop/UDisks2/drives/{}", base_device);

            let proxy = zbus::Proxy::new(
                &connection,
                "org.freedesktop.UDisks2",
                drive_path.as_str(),
                "org.freedesktop.UDisks2.Drive",
            )
            .await
            .map_err(|e| format!("Failed to create drive proxy: {}", e))?;

            let options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();

            proxy
                .call_method("PowerOff", &(options,))
                .await
                .map_err(|e| format!("PowerOff failed: {}", e))?;

            Ok(())
        })
    }


    pub fn mount(&self, device_path: &str) -> Result<PathBuf, String> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or("Tokio runtime not available")?;

        let device_path = device_path.to_string();

        runtime.block_on(async {
            let connection = zbus::Connection::system()
                .await
                .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

            let object_path = device_to_dbus_path(&device_path);

            let proxy = zbus::Proxy::new(
                &connection,
                "org.freedesktop.UDisks2",
                object_path.as_str(),
                "org.freedesktop.UDisks2.Filesystem",
            )
            .await
            .map_err(|e| format!("Failed to create proxy: {}", e))?;

            let options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();

            let reply: zbus::Message = proxy
                .call_method("Mount", &(options,))
                .await
                .map_err(|e| format!("Mount failed: {}", e))?;

            let mount_path: String = reply
                .body()
                .deserialize()
                .map_err(|e| format!("Failed to parse mount path: {}", e))?;

            Ok(PathBuf::from(mount_path))
        })
    }
}

fn device_to_dbus_path(device_path: &str) -> String {
    let device_name = device_path.trim_start_matches("/dev/");
    format!("/org/freedesktop/UDisks2/block_devices/{}", device_name)
}

impl super::device_monitor::DeviceMonitor {
    #[cfg(target_os = "linux")]
    pub fn enumerate_linux_devices(&mut self) {
        let monitor = LinuxDeviceMonitor::new();
        let block_devices = monitor.enumerate_devices();

        let root_path = PathBuf::from("/");
        if let Ok((total, free)) = get_disk_space(&root_path) {
            let root_device = Device::new(
                DeviceId::new(0),
                "Root".to_string(),
                root_path,
                DeviceType::InternalDrive,
            )
            .with_space(total, free)
            .with_removable(false);

            self.add_device(root_device);
        }

        for block_device in block_devices {
            if let Some(ref mount_point) = block_device.mount_point {
                if *mount_point == PathBuf::from("/") {
                    continue;
                }

                let id = device_node_to_id(&block_device.device_node);
                let device_type = block_device.device_type();
                let name = block_device.display_name();
                let mount_point = mount_point.clone();

                let mut device =
                    Device::new(id, name, mount_point.clone(), device_type)
                        .with_removable(block_device.is_removable);

                if let Ok((total, free)) = get_disk_space(&mount_point) {
                    device = device.with_space(total, free);
                }

                self.add_device(device);
            }
        }
    }

    #[cfg(target_os = "linux")]
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
        let client = UDisks2Client::new();

        if let Some(device_node) = find_device_node_for_mount(&path) {
            client
                .unmount(&device_node)
                .map_err(|e| super::device_monitor::DeviceError::EjectFailed(e))?;

            let _ = client.power_off(&device_node);
        } else {
            let output = std::process::Command::new("umount").arg(&path).output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(super::device_monitor::DeviceError::EjectFailed(
                    error.to_string(),
                ));
            }
        }

        self.remove_device(id);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn unmount(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self
            .get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;

        let path = device.path.clone();
        let client = UDisks2Client::new();

        if let Some(device_node) = find_device_node_for_mount(&path) {
            client
                .unmount(&device_node)
                .map_err(|e| super::device_monitor::DeviceError::UnmountFailed(e))?;
        } else {
            let output = std::process::Command::new("umount").arg(&path).output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(super::device_monitor::DeviceError::UnmountFailed(
                    error.to_string(),
                ));
            }
        }

        self.remove_device(id);
        Ok(())
    }
}

fn find_device_node_for_mount(mount_point: &PathBuf) -> Option<String> {
    let mounts = parse_mount_points();
    let mount_str = mount_point.to_string_lossy();

    for (device, path) in mounts {
        if path.to_string_lossy() == mount_str {
            return Some(device);
        }
    }
    None
}






#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Default)]
pub struct UDisks2DeviceProperties {
    pub device: String,
    pub id_label: Option<String>,
    pub id_type: Option<String>,
    pub id_uuid: Option<String>,
    pub size: u64,
    pub read_only: bool,
    pub removable: bool,
    pub ejectable: bool,
    pub mount_points: Vec<PathBuf>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
}


#[cfg(target_os = "linux")]
fn parse_udisksctl_mount_output(output: &str) -> Option<PathBuf> {
    for line in output.lines() {
        if line.starts_with("Mounted") && line.contains(" at ") {
            let parts: Vec<&str> = line.split(" at ").collect();
            if parts.len() >= 2 {
                let mount_point = parts[1].trim_end_matches('.');
                return Some(PathBuf::from(mount_point));
            }
        }
    }
    None
}


#[cfg(target_os = "linux")]
fn parse_udisksctl_info_output(output: &str) -> Option<UDisks2DeviceProperties> {
    let mut props = UDisks2DeviceProperties::default();
    let mut in_block_section = false;
    let mut in_filesystem_section = false;
    let mut in_drive_section = false;

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("org.freedesktop.UDisks2.Block:") {
            in_block_section = true;
            in_filesystem_section = false;
            in_drive_section = false;
            continue;
        } else if line.starts_with("org.freedesktop.UDisks2.Filesystem:") {
            in_block_section = false;
            in_filesystem_section = true;
            in_drive_section = false;
            continue;
        } else if line.starts_with("org.freedesktop.UDisks2.Drive:") {
            in_block_section = false;
            in_filesystem_section = false;
            in_drive_section = true;
            continue;
        } else if line.starts_with("org.freedesktop.UDisks2.") {
            in_block_section = false;
            in_filesystem_section = false;
            in_drive_section = false;
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('\'');

            match key {
                "Device" if in_block_section => {
                    props.device = value.to_string();
                }
                "IdLabel" => {
                    if !value.is_empty() {
                        props.id_label = Some(value.to_string());
                    }
                }
                "IdType" => {
                    if !value.is_empty() {
                        props.id_type = Some(value.to_string());
                    }
                }
                "IdUUID" => {
                    if !value.is_empty() {
                        props.id_uuid = Some(value.to_string());
                    }
                }
                "Size" => {
                    props.size = value.parse().unwrap_or(0);
                }
                "ReadOnly" => {
                    props.read_only = value == "true";
                }
                "Removable" => {
                    props.removable = value == "true";
                }
                "Ejectable" => {
                    props.ejectable = value == "true";
                }
                "MountPoints" if in_filesystem_section => {
                    let mount_str = value.trim_start_matches('[').trim_end_matches(']');
                    for mp in mount_str.split(',') {
                        let mp = mp.trim().trim_matches('\'').trim_matches('"');
                        if !mp.is_empty() && mp != "/" {
                            props.mount_points.push(PathBuf::from(mp));
                        }
                    }
                }
                "Vendor" if in_drive_section => {
                    if !value.is_empty() {
                        props.vendor = Some(value.to_string());
                    }
                }
                "Model" if in_drive_section => {
                    if !value.is_empty() {
                        props.model = Some(value.to_string());
                    }
                }
                "Serial" if in_drive_section => {
                    if !value.is_empty() {
                        props.serial = Some(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    if props.device.is_empty() {
        None
    } else {
        Some(props)
    }
}


#[cfg(target_os = "linux")]
fn get_parent_block_device(device: &str) -> String {
    let device = device.trim_start_matches("/dev/");
    
    if device.starts_with("nvme") {
        if let Some(pos) = device.rfind('p') {
            if device[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
                return format!("/dev/{}", &device[..pos]);
            }
        }
    }
    
    let base = device.trim_end_matches(char::is_numeric);
    format!("/dev/{}", base)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_block_device_display_name_with_label() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sdb1".to_string(),
            id_fs_label: Some("MyUSB".to_string()),
            ..Default::default()
        };
        assert_eq!(device.display_name(), "MyUSB");
    }

    #[test]
    fn test_linux_block_device_display_name_with_model() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sdb1".to_string(),
            id_model: Some("SanDisk_Ultra".to_string()),
            ..Default::default()
        };
        assert_eq!(device.display_name(), "SanDisk Ultra");
    }

    #[test]
    fn test_linux_block_device_display_name_fallback() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sdb1".to_string(),
            ..Default::default()
        };
        assert_eq!(device.display_name(), "SDB1");
    }

    #[test]
    fn test_linux_block_device_type_usb() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sdb1".to_string(),
            id_bus: Some("usb".to_string()),
            ..Default::default()
        };
        assert_eq!(device.device_type(), DeviceType::UsbDrive);
    }

    #[test]
    fn test_linux_block_device_type_optical() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sr0".to_string(),
            id_fs_type: Some("iso9660".to_string()),
            ..Default::default()
        };
        assert_eq!(device.device_type(), DeviceType::OpticalDrive);
    }

    #[test]
    fn test_linux_block_device_type_network() {
        let device = LinuxBlockDevice {
            device_node: "//server/share".to_string(),
            id_fs_type: Some("cifs".to_string()),
            ..Default::default()
        };
        assert_eq!(device.device_type(), DeviceType::NetworkDrive);
    }

    #[test]
    fn test_linux_block_device_type_removable() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sdc1".to_string(),
            is_removable: true,
            ..Default::default()
        };
        assert_eq!(device.device_type(), DeviceType::ExternalDrive);
    }

    #[test]
    fn test_linux_block_device_type_internal() {
        let device = LinuxBlockDevice {
            device_node: "/dev/sda1".to_string(),
            is_removable: false,
            ..Default::default()
        };
        assert_eq!(device.device_type(), DeviceType::InternalDrive);
    }

    #[test]
    fn test_detect_device_type_network_nfs() {
        let device_type = detect_device_type("server:/export", "nfs4", "/mnt/nfs", false);
        assert_eq!(device_type, DeviceType::NetworkDrive);
    }

    #[test]
    fn test_detect_device_type_network_cifs() {
        let device_type = detect_device_type("//server/share", "cifs", "/mnt/share", false);
        assert_eq!(device_type, DeviceType::NetworkDrive);
    }

    #[test]
    fn test_detect_device_type_optical_sr() {
        let device_type = detect_device_type("/dev/sr0", "iso9660", "/media/cdrom", false);
        assert_eq!(device_type, DeviceType::OpticalDrive);
    }

    #[test]
    fn test_detect_device_type_usb_removable() {
        let device_type = detect_device_type("/dev/sdb1", "vfat", "/media/usb", true);
        assert_eq!(device_type, DeviceType::UsbDrive);
    }

    #[test]
    fn test_detect_device_type_external_media() {
        let device_type = detect_device_type("/dev/sdc1", "ext4", "/media/user/disk", false);
        assert_eq!(device_type, DeviceType::ExternalDrive);
    }

    #[test]
    fn test_detect_device_type_external_mnt() {
        let device_type = detect_device_type("/dev/sdc1", "ext4", "/mnt/external", false);
        assert_eq!(device_type, DeviceType::ExternalDrive);
    }

    #[test]
    fn test_detect_device_type_internal() {
        let device_type = detect_device_type("/dev/sda1", "ext4", "/", false);
        assert_eq!(device_type, DeviceType::InternalDrive);
    }

    #[test]
    fn test_get_device_name_with_label() {
        let mount_point = PathBuf::from("/media/user/MyDisk");
        let name = get_device_name(&mount_point, "/dev/sdb1", Some("MyLabel"));
        assert_eq!(name, "MyLabel");
    }

    #[test]
    fn test_get_device_name_fallback_to_mount() {
        let mount_point = PathBuf::from("/media/user/MyDisk");
        let name = get_device_name(&mount_point, "/dev/sdb1", None);
        assert_eq!(name, "MyDisk");
    }

    #[test]
    fn test_get_device_name_empty_label() {
        let mount_point = PathBuf::from("/media/user/MyDisk");
        let name = get_device_name(&mount_point, "/dev/sdb1", Some(""));
        assert_eq!(name, "MyDisk");
    }

    #[test]
    fn test_mount_entry_parsing() {
        let mounts = parse_proc_mounts();
        for mount in &mounts {
            assert!(!mount.device.is_empty());
            assert!(!mount.fs_type.is_empty());
        }
    }

    #[test]
    fn test_udev_device_info_default() {
        let info = UdevDeviceInfo::default();
        assert!(info.device_node.is_empty());
        assert!(info.id_fs_type.is_none());
        assert!(!info.is_removable);
        assert_eq!(info.size_bytes, 0);
    }

    #[test]
    fn test_get_parent_block_device_regular() {
        assert_eq!(get_parent_block_device("/dev/sdb1"), "/dev/sdb");
        assert_eq!(get_parent_block_device("/dev/sdc2"), "/dev/sdc");
        assert_eq!(get_parent_block_device("sdb1"), "/dev/sdb");
    }

    #[test]
    fn test_get_parent_block_device_nvme() {
        assert_eq!(get_parent_block_device("/dev/nvme0n1p1"), "/dev/nvme0n1");
        assert_eq!(get_parent_block_device("/dev/nvme0n1p2"), "/dev/nvme0n1");
        assert_eq!(get_parent_block_device("nvme0n1p1"), "/dev/nvme0n1");
    }


pub struct SmartDataReader;

impl SmartDataReader {

    pub fn get_smart_data(device_node: &str) -> Option<SmartData> {
        Self::get_smart_data_smartctl(device_node)
            .or_else(|| Self::get_smart_data_sysfs(device_node))
    }


    fn get_smart_data_smartctl(device_node: &str) -> Option<SmartData> {
        use std::process::Command;

        let parent_device = get_parent_block_device(device_node);

        let output = Command::new("smartctl")
            .args(["-a", "-j", &parent_device])
            .output()
            .ok()?;

        if !output.status.success() && output.stdout.is_empty() {
            return Self::get_smart_data_smartctl_text(&parent_device);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        Self::parse_smartctl_json(&json_str)
    }


    fn parse_smartctl_json(json_str: &str) -> Option<SmartData> {
        use serde_json::Value;

        let json: Value = serde_json::from_str(json_str).ok()?;

        let mut data = SmartData::default();

        if let Some(smart_status) = json.get("smart_status") {
            if let Some(passed) = smart_status.get("passed").and_then(|v| v.as_bool()) {
                data.health_status = if passed {
                    HealthStatus::Good
                } else {
                    HealthStatus::Critical
                };
            }
        }

        if let Some(temp) = json.get("temperature") {
            if let Some(current) = temp.get("current").and_then(|v| v.as_u64()) {
                data.temperature_celsius = Some(current.min(255) as u8);
            }
        }

        if let Some(power_on) = json.get("power_on_time") {
            if let Some(hours) = power_on.get("hours").and_then(|v| v.as_u64()) {
                data.power_on_hours = Some(hours);
            }
        }

        if let Some(ata_attrs) = json.get("ata_smart_attributes") {
            if let Some(table) = ata_attrs.get("table").and_then(|v| v.as_array()) {
                for attr in table {
                    let id = attr.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                    if id == 0 {
                        continue;
                    }

                    let name = attr
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| SmartAttribute::get_standard_name(id))
                        .to_string();

                    let value = attr.get("value").and_then(|v| v.as_u64()).unwrap_or(0);
                    let worst = attr.get("worst").and_then(|v| v.as_u64()).unwrap_or(0);
                    let thresh = attr.get("thresh").and_then(|v| v.as_u64()).unwrap_or(0);

                    let raw_value = attr
                        .get("raw")
                        .and_then(|v| v.get("value"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v.to_string())
                        .unwrap_or_default();

                    let smart_attr = SmartAttribute::new(id, name, value, worst, thresh, raw_value.clone());
                    data.attributes.push(smart_attr);

                    match id {
                        smart_attributes::REALLOCATED_SECTORS_COUNT => {
                            data.reallocated_sectors = raw_value.parse().ok();
                        }
                        smart_attributes::CURRENT_PENDING_SECTOR_COUNT => {
                            data.pending_sectors = raw_value.parse().ok();
                        }
                        smart_attributes::TEMPERATURE_CELSIUS | smart_attributes::TEMPERATURE_CELSIUS_ALT => {
                            if data.temperature_celsius.is_none() {
                                data.temperature_celsius = raw_value.parse::<u64>().ok().map(|v| v.min(255) as u8);
                            }
                        }
                        smart_attributes::POWER_ON_HOURS => {
                            if data.power_on_hours.is_none() {
                                data.power_on_hours = raw_value.parse().ok();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if data.health_status == HealthStatus::Good {
            data.health_status = data.determine_health_status();
        }

        Some(data)
    }


    fn get_smart_data_smartctl_text(device_node: &str) -> Option<SmartData> {
        use std::process::Command;

        let output = Command::new("smartctl")
            .args(["-a", device_node])
            .output()
            .ok()?;

        let text = String::from_utf8_lossy(&output.stdout);
        Self::parse_smartctl_text(&text)
    }


    fn parse_smartctl_text(text: &str) -> Option<SmartData> {
        let mut data = SmartData::default();

        for line in text.lines() {
            if line.contains("SMART overall-health self-assessment test result:") {
                if line.contains("PASSED") {
                    data.health_status = HealthStatus::Good;
                } else if line.contains("FAILED") {
                    data.health_status = HealthStatus::Critical;
                }
            }
        }

        let mut in_attr_section = false;
        for line in text.lines() {
            if line.contains("ID#") && line.contains("ATTRIBUTE_NAME") {
                in_attr_section = true;
                continue;
            }

            if in_attr_section {
                if line.trim().is_empty() {
                    break;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    if let Ok(id) = parts[0].parse::<u8>() {
                        let name = parts[1].replace("_", " ");
                        let value = parts[3].parse::<u64>().unwrap_or(0);
                        let worst = parts[4].parse::<u64>().unwrap_or(0);
                        let thresh = parts[5].parse::<u64>().unwrap_or(0);
                        let raw_value = parts[9].to_string();

                        let attr = SmartAttribute::new(id, name, value, worst, thresh, raw_value.clone());
                        data.attributes.push(attr);

                        match id {
                            smart_attributes::REALLOCATED_SECTORS_COUNT => {
                                data.reallocated_sectors = raw_value.parse().ok();
                            }
                            smart_attributes::CURRENT_PENDING_SECTOR_COUNT => {
                                data.pending_sectors = raw_value.parse().ok();
                            }
                            smart_attributes::TEMPERATURE_CELSIUS | smart_attributes::TEMPERATURE_CELSIUS_ALT => {
                                if data.temperature_celsius.is_none() {
                                    data.temperature_celsius = raw_value.parse::<u64>().ok().map(|v| v.min(255) as u8);
                                }
                            }
                            smart_attributes::POWER_ON_HOURS => {
                                if data.power_on_hours.is_none() {
                                    data.power_on_hours = raw_value.parse().ok();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if data.health_status == HealthStatus::Good || data.health_status == HealthStatus::Unknown {
            data.health_status = data.determine_health_status();
        }

        if data.attributes.is_empty() && data.health_status == HealthStatus::Unknown {
            return None;
        }

        Some(data)
    }


    fn get_smart_data_sysfs(device_node: &str) -> Option<SmartData> {
        let parent_device = get_parent_block_device(device_node);
        let device_name = parent_device.trim_start_matches("/dev/");

        let sysfs_path = format!("/sys/block/{}/device", device_name);

        if !std::path::Path::new(&sysfs_path).exists() {
            return None;
        }

        let mut data = SmartData::default();

        if let Ok(state) = std::fs::read_to_string(format!("{}/state", sysfs_path)) {
            let state = state.trim();
            data.health_status = match state {
                "running" => HealthStatus::Good,
                "offline" | "blocked" => HealthStatus::Warning,
                _ => HealthStatus::Unknown,
            };
        }

        let hwmon_path = format!("{}/hwmon", sysfs_path);
        if let Ok(entries) = std::fs::read_dir(&hwmon_path) {
            for entry in entries.flatten() {
                let temp_path = entry.path().join("temp1_input");
                if let Ok(temp_str) = std::fs::read_to_string(&temp_path) {
                    if let Ok(temp_millicelsius) = temp_str.trim().parse::<u64>() {
                        data.temperature_celsius = Some((temp_millicelsius / 1000).min(255) as u8);
                        break;
                    }
                }
            }
        }

        if data.health_status == HealthStatus::Unknown && data.temperature_celsius.is_none() {
            return None;
        }

        Some(data)
    }
}

impl LinuxDeviceMonitor {

    pub fn get_smart_data(&self, device_node: &str) -> Option<SmartData> {
        SmartDataReader::get_smart_data(device_node)
    }
}

#[cfg(test)]
mod smart_tests {
    use super::*;

    #[test]
    fn test_parse_smartctl_json_healthy() {
        let json = r#"{
            "smart_status": {"passed": true},
            "temperature": {"current": 35},
            "power_on_time": {"hours": 1000},
            "ata_smart_attributes": {
                "table": [
                    {"id": 5, "name": "Reallocated_Sector_Ct", "value": 100, "worst": 100, "thresh": 10, "raw": {"value": 0}},
                    {"id": 9, "name": "Power_On_Hours", "value": 99, "worst": 99, "thresh": 0, "raw": {"value": 1000}},
                    {"id": 194, "name": "Temperature_Celsius", "value": 65, "worst": 50, "thresh": 0, "raw": {"value": 35}}
                ]
            }
        }"#;

        let result = SmartDataReader::parse_smartctl_json(json);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.health_status, HealthStatus::Good);
        assert_eq!(data.temperature_celsius, Some(35));
        assert_eq!(data.power_on_hours, Some(1000));
        assert_eq!(data.reallocated_sectors, Some(0));
    }

    #[test]
    fn test_parse_smartctl_json_failing() {
        let json = r#"{
            "smart_status": {"passed": false},
            "ata_smart_attributes": {
                "table": [
                    {"id": 5, "name": "Reallocated_Sector_Ct", "value": 1, "worst": 1, "thresh": 10, "raw": {"value": 500}}
                ]
            }
        }"#;

        let result = SmartDataReader::parse_smartctl_json(json);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.health_status, HealthStatus::Critical);
    }

    #[test]
    fn test_parse_smartctl_text() {
        let text = r#"
smartctl 7.2 2020-12-30 r5155 [x86_64-linux-5.10.0] (local build)
SMART overall-health self-assessment test result: PASSED

ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  5 Reallocated_Sector_Ct   0x0033   100   100   010    Pre-fail  Always       -       0
  9 Power_On_Hours          0x0032   099   099   000    Old_age   Always       -       1234
194 Temperature_Celsius     0x0022   065   050   000    Old_age   Always       -       35
"#;

        let result = SmartDataReader::parse_smartctl_text(text);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.health_status, HealthStatus::Good);
        assert_eq!(data.power_on_hours, Some(1234));
        assert_eq!(data.temperature_celsius, Some(35));
        assert_eq!(data.reallocated_sectors, Some(0));
    }
}

    #[test]
    fn test_get_parent_block_device_whole_disk() {
        assert_eq!(get_parent_block_device("/dev/sdb"), "/dev/sdb");
        assert_eq!(get_parent_block_device("/dev/nvme0n1"), "/dev/nvme0n1");
    }

    #[test]
    fn test_parse_udisksctl_mount_output() {
        let output = "Mounted /dev/sdb1 at /media/user/USBDRIVE.";
        let result = parse_udisksctl_mount_output(output);
        assert_eq!(result, Some(PathBuf::from("/media/user/USBDRIVE")));
    }

    #[test]
    fn test_parse_udisksctl_mount_output_no_match() {
        let output = "Error mounting device";
        let result = parse_udisksctl_mount_output(output);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_udisksctl_info_output() {
        let output = r#"
/org/freedesktop/UDisks2/block_devices/sdb1:
  org.freedesktop.UDisks2.Block:
    Device:                     /dev/sdb1
    IdLabel:                    MYUSB
    IdType:                     vfat
    IdUUID:                     1234-5678
    Size:                       16000000000
    ReadOnly:                   false
  org.freedesktop.UDisks2.Filesystem:
    MountPoints:                ['/media/user/MYUSB']
  org.freedesktop.UDisks2.Drive:
    Vendor:                     SanDisk
    Model:                      Ultra
    Serial:                     ABC123
    Removable:                  true
    Ejectable:                  true
"#;
        let result = parse_udisksctl_info_output(output);
        assert!(result.is_some());
        let props = result.unwrap();
        assert_eq!(props.device, "/dev/sdb1");
        assert_eq!(props.id_label, Some("MYUSB".to_string()));
        assert_eq!(props.id_type, Some("vfat".to_string()));
        assert_eq!(props.size, 16000000000);
        assert!(!props.read_only);
    }

    #[test]
    fn test_parse_udisksctl_info_output_empty() {
        let output = "";
        let result = parse_udisksctl_info_output(output);
        assert!(result.is_none());
    }

    #[test]
    fn test_linux_device_monitor_new() {
        let monitor = LinuxDeviceMonitor::new();
        assert!(!monitor.is_monitoring());
    }

    #[test]
    fn test_udisks2_device_properties_default() {
        let props = UDisks2DeviceProperties::default();
        assert!(props.device.is_empty());
        assert!(props.id_label.is_none());
        assert!(!props.removable);
        assert!(props.mount_points.is_empty());
    }
}
