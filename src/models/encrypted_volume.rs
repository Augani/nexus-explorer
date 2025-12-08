/*
 * Encrypted Volume Support
 * 
 * This module provides cross-platform support for detecting and managing encrypted volumes:
 * - Windows: BitLocker encrypted volumes via WMI Win32_EncryptableVolume
 * - Linux: LUKS encrypted volumes via cryptsetup/udisks2
 * 
 * Requirements: 21.1-21.8 (Encrypted Volume Support)
 */

use std::path::PathBuf;
use thiserror::Error;

/
#[derive(Debug, Error)]
pub enum EncryptedVolumeError {
    #[error("Volume not found: {0}")]
    VolumeNotFound(String),

    #[error("Volume is not encrypted")]
    NotEncrypted,

    #[error("Volume is already unlocked")]
    AlreadyUnlocked,

    #[error("Volume is already locked")]
    AlreadyLocked,

    #[error("Invalid password or recovery key")]
    InvalidCredentials,

    #[error("Unlock failed: {0}")]
    UnlockFailed(String),

    #[error("Lock failed: {0}")]
    LockFailed(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type EncryptedVolumeResult<T> = Result<T, EncryptedVolumeError>;

/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionType {
    BitLocker,
    Luks,
    FileVault,
    Unknown,
}

impl EncryptionType {
    pub fn display_name(&self) -> &'static str {
        match self {
            EncryptionType::BitLocker => "BitLocker",
            EncryptionType::Luks => "LUKS",
            EncryptionType::FileVault => "FileVault",
            EncryptionType::Unknown => "Unknown",
        }
    }
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionStatus {
    /
    Unlocked,
    /
    Locked,
    /
    Unknown,
}

impl ProtectionStatus {
    pub fn is_locked(&self) -> bool {
        matches!(self, ProtectionStatus::Locked)
    }

    pub fn is_unlocked(&self) -> bool {
        matches!(self, ProtectionStatus::Unlocked)
    }
}

/
#[derive(Debug, Clone)]
pub struct EncryptedVolumeInfo {
    /
    pub device_id: String,
    /
    pub mount_point: Option<PathBuf>,
    /
    pub encryption_type: EncryptionType,
    /
    pub protection_status: ProtectionStatus,
    /
    pub label: Option<String>,
    /
    pub size: u64,
    /
    pub encryption_percentage: Option<u8>,
}

impl EncryptedVolumeInfo {
    pub fn is_encrypted(&self) -> bool {
        !matches!(self.encryption_type, EncryptionType::Unknown)
    }

    pub fn is_unlocked(&self) -> bool {
        self.protection_status.is_unlocked()
    }

    pub fn is_locked(&self) -> bool {
        self.protection_status.is_locked()
    }
}

/
#[derive(Debug, Clone)]
pub enum UnlockCredential {
    Password(String),
    RecoveryKey(String),
}

/
pub struct EncryptedVolumeManager {
    #[cfg(target_os = "windows")]
    _windows_marker: std::marker::PhantomData<()>,
    #[cfg(target_os = "linux")]
    _linux_marker: std::marker::PhantomData<()>,
}

impl Default for EncryptedVolumeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptedVolumeManager {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            _windows_marker: std::marker::PhantomData,
            #[cfg(target_os = "linux")]
            _linux_marker: std::marker::PhantomData,
        }
    }

    /
    pub fn is_encrypted(&self, device_id: &str) -> bool {
        self.get_volume_info(device_id)
            .map(|info| info.is_encrypted())
            .unwrap_or(false)
    }

    /
    pub fn get_volume_info(&self, device_id: &str) -> EncryptedVolumeResult<EncryptedVolumeInfo> {
        #[cfg(target_os = "windows")]
        {
            get_bitlocker_info(device_id)
        }

        #[cfg(target_os = "linux")]
        {
            get_luks_info(device_id)
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(EncryptedVolumeError::PlatformNotSupported(
                "Encrypted volume detection not supported on this platform".to_string()
            ))
        }
    }

    /
    pub fn list_encrypted_volumes(&self) -> Vec<EncryptedVolumeInfo> {
        #[cfg(target_os = "windows")]
        {
            list_bitlocker_volumes()
        }

        #[cfg(target_os = "linux")]
        {
            list_luks_volumes()
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Vec::new()
        }
    }

    /
    pub fn unlock(&self, device_id: &str, credential: UnlockCredential) -> EncryptedVolumeResult<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            unlock_bitlocker(device_id, credential)
        }

        #[cfg(target_os = "linux")]
        {
            unlock_luks(device_id, credential)
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(EncryptedVolumeError::PlatformNotSupported(
                "Encrypted volume unlock not supported on this platform".to_string()
            ))
        }
    }

    /
    pub fn lock(&self, device_id: &str) -> EncryptedVolumeResult<()> {
        #[cfg(target_os = "windows")]
        {
            lock_bitlocker(device_id)
        }

        #[cfg(target_os = "linux")]
        {
            lock_luks(device_id)
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(EncryptedVolumeError::PlatformNotSupported(
                "Encrypted volume lock not supported on this platform".to_string()
            ))
        }
    }
}



#[cfg(target_os = "windows")]
fn get_bitlocker_info(drive_letter: &str) -> EncryptedVolumeResult<EncryptedVolumeInfo> {
    let drive = normalize_drive_letter(drive_letter);
    
    let script = format!(
        "$vol = Get-BitLockerVolume -MountPoint '{}:' -ErrorAction SilentlyContinue; \
         if ($vol) {{ \
             $status = $vol.ProtectionStatus; \
             $encType = $vol.EncryptionMethod; \
             $pct = $vol.EncryptionPercentage; \
             $label = $vol.VolumeLabel; \
             $size = (Get-Volume -DriveLetter '{}').Size; \
             \"$status|$encType|$pct|$label|$size\" \
         }} else {{ 'NOT_ENCRYPTED' }}",
        drive, drive
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(EncryptedVolumeError::Io)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result = stdout.trim();

    if result == "NOT_ENCRYPTED" || result.is_empty() {
        return Err(EncryptedVolumeError::NotEncrypted);
    }

    let parts: Vec<&str> = result.split('|').collect();
    if parts.len() < 5 {
        return Err(EncryptedVolumeError::NotEncrypted);
    }

    let protection_status = match parts[0] {
        "On" => ProtectionStatus::Locked,
        "Off" => ProtectionStatus::Unlocked,
        _ => ProtectionStatus::Unknown,
    };

    let encryption_percentage = parts[2].parse::<u8>().ok();
    let label = if parts[3].is_empty() { None } else { Some(parts[3].to_string()) };
    let size = parts[4].parse::<u64>().unwrap_or(0);

    Ok(EncryptedVolumeInfo {
        device_id: format!("{}:", drive),
        mount_point: Some(PathBuf::from(format!("{}:\\", drive))),
        encryption_type: EncryptionType::BitLocker,
        protection_status,
        label,
        size,
        encryption_percentage,
    })
}

#[cfg(target_os = "windows")]
fn list_bitlocker_volumes() -> Vec<EncryptedVolumeInfo> {
    let script = "Get-BitLockerVolume | ForEach-Object { \
        $size = (Get-Volume -DriveLetter $_.MountPoint.TrimEnd(':')).Size; \
        \"$($_.MountPoint)|$($_.ProtectionStatus)|$($_.EncryptionPercentage)|$($_.VolumeLabel)|$size\" \
    }";

    let output = match std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
    {
        Ok(out) => out,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut volumes = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.trim().split('|').collect();
        if parts.len() < 5 {
            continue;
        }

        let mount_point = parts[0].trim_end_matches(':');
        let protection_status = match parts[1] {
            "On" => ProtectionStatus::Locked,
            "Off" => ProtectionStatus::Unlocked,
            _ => ProtectionStatus::Unknown,
        };

        let encryption_percentage = parts[2].parse::<u8>().ok();
        let label = if parts[3].is_empty() { None } else { Some(parts[3].to_string()) };
        let size = parts[4].parse::<u64>().unwrap_or(0);

        volumes.push(EncryptedVolumeInfo {
            device_id: format!("{}:", mount_point),
            mount_point: Some(PathBuf::from(format!("{}:\\", mount_point))),
            encryption_type: EncryptionType::BitLocker,
            protection_status,
            label,
            size,
            encryption_percentage,
        });
    }

    volumes
}

#[cfg(target_os = "windows")]
fn unlock_bitlocker(drive_letter: &str, credential: UnlockCredential) -> EncryptedVolumeResult<PathBuf> {
    let drive = normalize_drive_letter(drive_letter);

    let script = match credential {
        UnlockCredential::Password(ref pwd) => {
            let escaped_pwd = pwd.replace("'", "''");
            format!(
                "$secPwd = ConvertTo-SecureString '{}' -AsPlainText -Force; \
                 Unlock-BitLocker -MountPoint '{}:' -Password $secPwd -ErrorAction Stop",
                escaped_pwd, drive
            )
        }
        UnlockCredential::RecoveryKey(ref key) => {
            let escaped_key = key.replace("'", "''");
            format!(
                "Unlock-BitLocker -MountPoint '{}:' -RecoveryPassword '{}' -ErrorAction Stop",
                drive, escaped_key
            )
        }
    };

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(EncryptedVolumeError::Io)?;

    if output.status.success() {
        Ok(PathBuf::from(format!("{}:\\", drive)))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("password") || stderr.contains("recovery") {
            Err(EncryptedVolumeError::InvalidCredentials)
        } else {
            Err(EncryptedVolumeError::UnlockFailed(stderr.to_string()))
        }
    }
}

#[cfg(target_os = "windows")]
fn lock_bitlocker(drive_letter: &str) -> EncryptedVolumeResult<()> {
    let drive = normalize_drive_letter(drive_letter);

    let script = format!(
        "Lock-BitLocker -MountPoint '{}:' -ForceDismount -ErrorAction Stop",
        drive
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(EncryptedVolumeError::Io)?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(EncryptedVolumeError::LockFailed(stderr.to_string()))
    }
}

#[cfg(target_os = "windows")]
fn normalize_drive_letter(input: &str) -> char {
    input.chars()
        .find(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('C')
}


#[cfg(target_os = "linux")]
fn get_luks_info(device_path: &str) -> EncryptedVolumeResult<EncryptedVolumeInfo> {
    let output = std::process::Command::new("cryptsetup")
        .args(["isLuks", device_path])
        .output()
        .map_err(EncryptedVolumeError::Io)?;

    if !output.status.success() {
        return Err(EncryptedVolumeError::NotEncrypted);
    }

    let status_output = std::process::Command::new("cryptsetup")
        .args(["status", device_path])
        .output()
        .map_err(EncryptedVolumeError::Io)?;

    let is_active = status_output.status.success();
    let protection_status = if is_active {
        ProtectionStatus::Unlocked
    } else {
        ProtectionStatus::Locked
    };

    let mount_point = if is_active {
        find_luks_mount_point(device_path)
    } else {
        None
    };

    let size = get_device_size(device_path).unwrap_or(0);

    Ok(EncryptedVolumeInfo {
        device_id: device_path.to_string(),
        mount_point,
        encryption_type: EncryptionType::Luks,
        protection_status,
        label: None,
        size,
        encryption_percentage: Some(100),
    })
}

#[cfg(target_os = "linux")]
fn list_luks_volumes() -> Vec<EncryptedVolumeInfo> {
    let output = match std::process::Command::new("lsblk")
        .args(["-o", "NAME,TYPE,FSTYPE,SIZE,MOUNTPOINT", "-J"])
        .output()
    {
        Ok(out) => out,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut volumes = Vec::new();

    for line in stdout.lines() {
        if line.contains("crypto_LUKS") {
            if let Some(device) = extract_device_from_lsblk_line(line) {
                if let Ok(info) = get_luks_info(&device) {
                    volumes.push(info);
                }
            }
        }
    }

    volumes
}

#[cfg(target_os = "linux")]
fn unlock_luks(device_path: &str, credential: UnlockCredential) -> EncryptedVolumeResult<PathBuf> {
    let password = match credential {
        UnlockCredential::Password(pwd) => pwd,
        UnlockCredential::RecoveryKey(key) => key,
    };

    let mapper_name = generate_mapper_name(device_path);

    let mut child = std::process::Command::new("cryptsetup")
        .args(["open", device_path, &mapper_name])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(EncryptedVolumeError::Io)?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(password.as_bytes());
        let _ = stdin.write_all(b"\n");
    }

    let output = child.wait_with_output().map_err(EncryptedVolumeError::Io)?;

    if output.status.success() {
        let mapper_path = PathBuf::from(format!("/dev/mapper/{}", mapper_name));
        
        let mount_point = PathBuf::from(format!("/mnt/{}", mapper_name));
        let _ = std::fs::create_dir_all(&mount_point);
        
        let mount_output = std::process::Command::new("mount")
            .args([mapper_path.to_str().unwrap_or(""), mount_point.to_str().unwrap_or("")])
            .output();

        if mount_output.map(|o| o.status.success()).unwrap_or(false) {
            Ok(mount_point)
        } else {
            Ok(mapper_path)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No key available") || stderr.contains("wrong") {
            Err(EncryptedVolumeError::InvalidCredentials)
        } else {
            Err(EncryptedVolumeError::UnlockFailed(stderr.to_string()))
        }
    }
}

#[cfg(target_os = "linux")]
fn lock_luks(device_path: &str) -> EncryptedVolumeResult<()> {
    let mapper_name = generate_mapper_name(device_path);
    let mapper_path = format!("/dev/mapper/{}", mapper_name);

    if let Some(mount_point) = find_mount_point(&mapper_path) {
        let _ = std::process::Command::new("umount")
            .arg(&mount_point)
            .output();
    }

    let output = std::process::Command::new("cryptsetup")
        .args(["close", &mapper_name])
        .output()
        .map_err(EncryptedVolumeError::Io)?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(EncryptedVolumeError::LockFailed(stderr.to_string()))
    }
}

#[cfg(target_os = "linux")]
fn find_luks_mount_point(device_path: &str) -> Option<PathBuf> {
    let mapper_name = generate_mapper_name(device_path);
    let mapper_path = format!("/dev/mapper/{}", mapper_name);
    find_mount_point(&mapper_path)
}

#[cfg(target_os = "linux")]
fn find_mount_point(device: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("findmnt")
        .args(["-n", "-o", "TARGET", device])
        .output()
        .ok()?;

    if output.status.success() {
        let mount = String::from_utf8_lossy(&output.stdout);
        let mount = mount.trim();
        if !mount.is_empty() {
            return Some(PathBuf::from(mount));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn generate_mapper_name(device_path: &str) -> String {
    let base = device_path
        .rsplit('/')
        .next()
        .unwrap_or("luks_volume");
    format!("luks_{}", base)
}

#[cfg(target_os = "linux")]
fn get_device_size(device_path: &str) -> Option<u64> {
    let output = std::process::Command::new("blockdev")
        .args(["--getsize64", device_path])
        .output()
        .ok()?;

    if output.status.success() {
        let size_str = String::from_utf8_lossy(&output.stdout);
        size_str.trim().parse().ok()
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn extract_device_from_lsblk_line(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if !parts.is_empty() {
        let name = parts[0].trim_start_matches(['├', '└', '─', '│', ' ']);
        return Some(format!("/dev/{}", name));
    }
    None
}


/
pub fn is_encrypted_volume_support_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", "Get-Command Get-BitLockerVolume -ErrorAction SilentlyContinue"])
            .output();
        output.map(|o| o.status.success()).unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("which")
            .arg("cryptsetup")
            .output();
        output.map(|o| o.status.success()).unwrap_or(false)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/
pub fn check_device_encrypted(device_id: &str) -> bool {
    let manager = EncryptedVolumeManager::new();
    manager.is_encrypted(device_id)
}
