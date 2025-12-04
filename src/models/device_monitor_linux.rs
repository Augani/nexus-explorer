use super::device_monitor::{get_disk_space, Device, DeviceId, DeviceMonitor, DeviceType};
use std::path::PathBuf;

impl DeviceMonitor {
    /// Enumerate devices on Linux by scanning /media, /mnt, and /proc/mounts
    #[cfg(target_os = "linux")]
    pub fn enumerate_linux_devices(&mut self) {
        // Add root filesystem
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

        // Parse /proc/mounts to find all mounted filesystems
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }

                let device_path = parts[0];
                let mount_point = parts[1];
                let fs_type = parts[2];
                let options = parts[3];

                // Skip virtual filesystems and system mounts
                if should_skip_mount(device_path, mount_point, fs_type) {
                    continue;
                }

                let path = PathBuf::from(mount_point);
                let device_type = detect_linux_device_type(device_path, fs_type, mount_point);
                let name = get_linux_device_name(&path, device_path);

                let is_removable = is_linux_removable(device_path);
                let is_read_only = options.contains("ro");

                let mut device = Device::new(DeviceId::new(0), name, path.clone(), device_type)
                    .with_removable(is_removable)
                    .with_read_only(is_read_only);

                if let Ok((total, free)) = get_disk_space(&path) {
                    device = device.with_space(total, free);
                }

                self.add_device(device);
            }
        }

        // Also scan /media and /mnt for user-mounted devices
        for base_path in &["/media", "/mnt"] {
            self.scan_mount_directory(base_path);
        }
    }

    /// Scan a directory for mounted devices
    #[cfg(target_os = "linux")]
    fn scan_mount_directory(&mut self, base_path: &str) {
        let base = PathBuf::from(base_path);
        if !base.exists() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip if already added
                if self.get_device_by_path(&path).is_some() {
                    continue;
                }

                if !is_mount_point(&path) {
                    continue;
                }

                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let mut device = Device::new(
                    DeviceId::new(0),
                    name,
                    path.clone(),
                    DeviceType::ExternalDrive,
                )
                .with_removable(true);

                if let Ok((total, free)) = get_disk_space(&path) {
                    device = device.with_space(total, free);
                }

                self.add_device(device);
            }
        }
    }

    /// Eject a device on Linux
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

        // First unmount, then eject
        let output = std::process::Command::new("udisksctl")
            .args([
                "unmount",
                "-b",
                &find_block_device(&path).unwrap_or_default(),
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                // Try to power off the drive
                let _ = std::process::Command::new("udisksctl")
                    .args([
                        "power-off",
                        "-b",
                        &find_block_device(&path).unwrap_or_default(),
                    ])
                    .output();

                self.remove_device(id);
                Ok(())
            }
            Ok(out) => {
                let error = String::from_utf8_lossy(&out.stderr);
                Err(super::device_monitor::DeviceError::EjectFailed(
                    error.to_string(),
                ))
            }
            Err(e) => Err(super::device_monitor::DeviceError::EjectFailed(
                e.to_string(),
            )),
        }
    }

    /// Unmount a device on Linux
    #[cfg(target_os = "linux")]
    pub fn unmount(&mut self, id: DeviceId) -> super::device_monitor::DeviceResult<()> {
        let device = self
            .get_device(id)
            .ok_or(super::device_monitor::DeviceError::NotFound(id))?;

        let path = device.path.clone();

        // Use umount command
        let output = std::process::Command::new("umount").arg(&path).output()?;

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

/// Check if a mount should be skipped (virtual filesystems, etc.)
#[cfg(target_os = "linux")]
fn should_skip_mount(device: &str, mount_point: &str, fs_type: &str) -> bool {
    // Skip virtual filesystems
    let virtual_fs = [
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "tmpfs",
        "securityfs",
        "cgroup",
        "cgroup2",
        "pstore",
        "debugfs",
        "hugetlbfs",
        "mqueue",
        "fusectl",
        "configfs",
        "binfmt_misc",
        "autofs",
        "efivarfs",
        "tracefs",
        "bpf",
        "overlay",
        "squashfs",
    ];

    if virtual_fs.contains(&fs_type) {
        return true;
    }

    // Skip system mount points
    let system_mounts = [
        "/",
        "/boot",
        "/boot/efi",
        "/home",
        "/var",
        "/tmp",
        "/sys",
        "/proc",
        "/dev",
        "/run",
    ];

    if system_mounts.contains(&mount_point) && mount_point != "/" {
        return true;
    }

    // Skip snap mounts
    if mount_point.starts_with("/snap") {
        return true;
    }

    false
}

/// Detect the type of Linux device
#[cfg(target_os = "linux")]
fn detect_linux_device_type(device: &str, fs_type: &str, mount_point: &str) -> DeviceType {
    // Network filesystems
    if ["nfs", "nfs4", "cifs", "smbfs", "sshfs", "fuse.sshfs"].contains(&fs_type) {
        return DeviceType::NetworkDrive;
    }

    if device.contains("sr") || device.contains("cdrom") || fs_type == "iso9660" {
        return DeviceType::OpticalDrive;
    }

    if device.starts_with("/dev/sd") {
        if is_linux_removable(device) {
            return DeviceType::UsbDrive;
        }
    }

    if mount_point.starts_with("/media") || mount_point.starts_with("/mnt") {
        return DeviceType::ExternalDrive;
    }

    DeviceType::InternalDrive
}

/// Get a friendly name for a Linux device
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

/// Check if a Linux device is removable
#[cfg(target_os = "linux")]
fn is_linux_removable(device: &str) -> bool {
    // Extract the base device name (e.g., sda from /dev/sda1)
    let base_device = device
        .trim_start_matches("/dev/")
        .trim_end_matches(char::is_numeric);

    let removable_path = format!("/sys/block/{}/removable", base_device);
    if let Ok(content) = std::fs::read_to_string(&removable_path) {
        return content.trim() == "1";
    }

    false
}

/// Check if a path is a mount point
#[cfg(target_os = "linux")]
fn is_mount_point(path: &PathBuf) -> bool {
    use std::os::unix::fs::MetadataExt;

    if let (Ok(path_meta), Some(parent_meta)) = (
        std::fs::metadata(path),
        path.parent().and_then(|p| std::fs::metadata(p).ok()),
    ) {
        return path_meta.dev() != parent_meta.dev();
    }
    false
}

/// Find the block device for a mount point
#[cfg(target_os = "linux")]
fn find_block_device(mount_point: &PathBuf) -> Option<String> {
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        let mount_str = mount_point.to_string_lossy();
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == mount_str {
                return Some(parts[0].to_string());
            }
        }
    }
    None
}
