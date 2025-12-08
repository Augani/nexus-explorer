use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during file sharing operations
#[derive(Debug, Error)]
pub enum ShareError {
    #[error("Share creation failed: {0}")]
    CreationFailed(String),

    #[error("Share removal failed: {0}")]
    RemovalFailed(String),

    #[error("Share not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid share name: {0}")]
    InvalidShareName(String),

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("AirDrop unavailable: {0}")]
    AirDropUnavailable(String),

    #[error("Nearby Share unavailable: {0}")]
    NearbyShareUnavailable(String),

    #[error("Transfer failed: {0}")]
    TransferFailed(String),

    #[error("Transfer cancelled")]
    TransferCancelled,
}

/// Platform-specific sharing methods available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformShareMethod {
    /// macOS AirDrop
    AirDrop,
    /// Windows Nearby Share
    NearbyShare,
    /// Network SMB/Samba share
    NetworkShare,
    /// Copy to clipboard
    Clipboard,
}

impl PlatformShareMethod {
    pub fn display_name(&self) -> &'static str {
        match self {
            PlatformShareMethod::AirDrop => "AirDrop",
            PlatformShareMethod::NearbyShare => "Nearby Share",
            PlatformShareMethod::NetworkShare => "Network Share",
            PlatformShareMethod::Clipboard => "Copy Path",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PlatformShareMethod::AirDrop => "Share with nearby Apple devices",
            PlatformShareMethod::NearbyShare => "Share with nearby Windows devices",
            PlatformShareMethod::NetworkShare => "Share over local network",
            PlatformShareMethod::Clipboard => "Copy file path to clipboard",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            PlatformShareMethod::AirDrop => "airplay",
            PlatformShareMethod::NearbyShare => "share-2",
            PlatformShareMethod::NetworkShare => "cloud",
            PlatformShareMethod::Clipboard => "clipboard-paste",
        }
    }
}

/// Status of a platform share transfer
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformShareStatus {
    /// Checking availability
    Checking,
    /// Ready to share
    Ready,
    /// Waiting for recipient selection
    WaitingForRecipient,
    /// Transfer in progress
    InProgress { progress_percent: u8 },
    /// Transfer completed successfully
    Completed,
    /// Transfer failed
    Failed(String),
    /// Transfer was cancelled
    Cancelled,
    /// Feature not available
    Unavailable(String),
}

pub type ShareResult<T> = std::result::Result<T, ShareError>;

/// Permission level for network shares
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SharePermission {
    #[default]
    ReadOnly,
    ReadWrite,
    Full,
}

impl SharePermission {
    pub fn display_name(&self) -> &'static str {
        match self {
            SharePermission::ReadOnly => "Read Only",
            SharePermission::ReadWrite => "Read/Write",
            SharePermission::Full => "Full Control",
        }
    }
}

/// Configuration for creating a network share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareConfig {
    pub share_name: String,
    pub path: PathBuf,
    pub description: String,
    pub permission: SharePermission,
    pub max_users: Option<u32>,
    pub password: Option<String>,
    pub users: Vec<String>,
}

impl ShareConfig {
    pub fn new(share_name: String, path: PathBuf) -> Self {
        Self {
            share_name,
            path,
            description: String::new(),
            permission: SharePermission::ReadOnly,
            max_users: None,
            password: None,
            users: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_permission(mut self, permission: SharePermission) -> Self {
        self.permission = permission;
        self
    }

    pub fn with_max_users(mut self, max_users: u32) -> Self {
        self.max_users = Some(max_users);
        self
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    pub fn with_users(mut self, users: Vec<String>) -> Self {
        self.users = users;
        self
    }
}

/// Information about an active share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInfo {
    pub share_name: String,
    pub path: PathBuf,
    pub description: String,
    pub permission: SharePermission,
    pub current_users: u32,
    pub max_users: Option<u32>,
}

impl ShareInfo {
    pub fn new(share_name: String, path: PathBuf) -> Self {
        Self {
            share_name,
            path,
            description: String::new(),
            permission: SharePermission::ReadOnly,
            current_users: 0,
            max_users: None,
        }
    }
}

/// Manager for file sharing operations
pub struct ShareManager {
    shares: HashMap<PathBuf, ShareInfo>,
}

impl Default for ShareManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShareManager {
    pub fn new() -> Self {
        Self {
            shares: HashMap::new(),
        }
    }

    /// Check if a path is currently shared
    pub fn is_shared(&self, path: &PathBuf) -> bool {
        self.shares.contains_key(path)
    }

    /// Get share info for a path
    pub fn get_share(&self, path: &PathBuf) -> Option<&ShareInfo> {
        self.shares.get(path)
    }

    /// Get all active shares
    pub fn list_shares(&self) -> Vec<&ShareInfo> {
        self.shares.values().collect()
    }

    /// Create a new network share
    pub fn create_share(&mut self, config: ShareConfig) -> ShareResult<ShareInfo> {
        if !config.path.exists() {
            return Err(ShareError::PathNotFound(config.path));
        }

        if config.share_name.is_empty() {
            return Err(ShareError::InvalidShareName(
                "Share name cannot be empty".to_string(),
            ));
        }

        // Validate share name (no special characters)
        if config.share_name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
            return Err(ShareError::InvalidShareName(
                "Share name contains invalid characters".to_string(),
            ));
        }

        // Platform-specific share creation
        #[cfg(target_os = "windows")]
        {
            create_windows_share(&config)?;
        }

        #[cfg(target_os = "linux")]
        {
            create_linux_share(&config)?;
        }

        #[cfg(target_os = "macos")]
        {
            create_macos_share(&config)?;
        }

        let info = ShareInfo {
            share_name: config.share_name,
            path: config.path.clone(),
            description: config.description,
            permission: config.permission,
            current_users: 0,
            max_users: config.max_users,
        };

        self.shares.insert(config.path, info.clone());
        Ok(info)
    }

    /// Remove a network share
    pub fn remove_share(&mut self, path: &PathBuf) -> ShareResult<()> {
        let share = self
            .shares
            .get(path)
            .ok_or_else(|| ShareError::NotFound(path.to_string_lossy().to_string()))?;

        let share_name = share.share_name.clone();

        #[cfg(target_os = "windows")]
        {
            remove_windows_share(&share_name)?;
        }

        #[cfg(target_os = "linux")]
        {
            remove_linux_share(&share_name)?;
        }

        #[cfg(target_os = "macos")]
        {
            remove_macos_share(&share_name)?;
        }

        self.shares.remove(path);
        Ok(())
    }

    /// Refresh the list of shares from the system
    pub fn refresh_shares(&mut self) -> ShareResult<()> {
        self.shares.clear();

        #[cfg(target_os = "windows")]
        {
            let shares = enumerate_windows_shares()?;
            for share in shares {
                self.shares.insert(share.path.clone(), share);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let shares = enumerate_linux_shares()?;
            for share in shares {
                self.shares.insert(share.path.clone(), share);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let shares = enumerate_macos_shares()?;
            for share in shares {
                self.shares.insert(share.path.clone(), share);
            }
        }

        Ok(())
    }
}


// Windows-specific share implementation
#[cfg(target_os = "windows")]
fn create_windows_share(config: &ShareConfig) -> ShareResult<()> {
    use std::process::Command;

    let path_str = config.path.to_string_lossy();

    // Use net share command to create the share
    // net share <sharename>=<path> /GRANT:<user>,<permission> /REMARK:"<description>"
    let mut args = vec![
        "share".to_string(),
        format!("{}={}", config.share_name, path_str),
    ];

    // Add description if provided
    if !config.description.is_empty() {
        args.push(format!("/REMARK:{}", config.description));
    }

    // Add permission grants
    let permission_str = match config.permission {
        SharePermission::ReadOnly => "READ",
        SharePermission::ReadWrite => "CHANGE",
        SharePermission::Full => "FULL",
    };

    if config.users.is_empty() {
        // Grant to Everyone if no specific users
        args.push(format!("/GRANT:Everyone,{}", permission_str));
    } else {
        for user in &config.users {
            args.push(format!("/GRANT:{},{}", user, permission_str));
        }
    }

    // Add max users if specified
    if let Some(max) = config.max_users {
        args.push(format!("/USERS:{}", max));
    }

    let output = Command::new("net")
        .args(&args)
        .output()
        .map_err(|e| ShareError::Io(e))?;

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
        Err(ShareError::CreationFailed(error_msg))
    }
}

#[cfg(target_os = "windows")]
fn remove_windows_share(share_name: &str) -> ShareResult<()> {
    use std::process::Command;

    let output = Command::new("net")
        .args(["share", share_name, "/DELETE", "/YES"])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::RemovalFailed(error.to_string()))
    }
}

#[cfg(target_os = "windows")]
fn enumerate_windows_shares() -> ShareResult<Vec<ShareInfo>> {
    use std::process::Command;

    let output = Command::new("net")
        .args(["share"])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut shares = Vec::new();

    // Parse net share output
    // Format: Share name   Resource                        Remark
    for line in stdout.lines().skip(4) {
        // Skip header lines
        let line = line.trim();
        if line.is_empty() || line.starts_with("The command") {
            continue;
        }

        // Parse the line - columns are space-separated
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let share_name = parts[0].to_string();
            let path_str = parts[1];

            // Skip system shares (ending with $)
            if share_name.ends_with('$') {
                continue;
            }

            if let Ok(path) = PathBuf::from(path_str).canonicalize() {
                let description = if parts.len() > 2 {
                    parts[2..].join(" ")
                } else {
                    String::new()
                };

                shares.push(ShareInfo {
                    share_name,
                    path,
                    description,
                    permission: SharePermission::ReadOnly, // Default, actual permission requires more API calls
                    current_users: 0,
                    max_users: None,
                });
            }
        }
    }

    Ok(shares)
}

// Linux-specific share implementation using Samba
#[cfg(target_os = "linux")]
fn create_linux_share(config: &ShareConfig) -> ShareResult<()> {
    use std::process::Command;

    let path_str = config.path.to_string_lossy();

    // Try using net usershare first (doesn't require root)
    let guest_ok = if config.password.is_none() { "y" } else { "n" };

    let acl = match config.permission {
        SharePermission::ReadOnly => "Everyone:R",
        SharePermission::ReadWrite | SharePermission::Full => "Everyone:F",
    };

    let output = Command::new("net")
        .args([
            "usershare",
            "add",
            &config.share_name,
            &path_str,
            &config.description,
            acl,
            guest_ok,
        ])
        .output();

    match output {
        Ok(result) if result.status.success() => Ok(()),
        Ok(result) => {
            let error = String::from_utf8_lossy(&result.stderr);
            Err(ShareError::CreationFailed(error.to_string()))
        }
        Err(e) => {
            // net usershare not available, try alternative method
            Err(ShareError::CreationFailed(format!(
                "net usershare not available: {}. Install samba-common-bin package.",
                e
            )))
        }
    }
}

#[cfg(target_os = "linux")]
fn remove_linux_share(share_name: &str) -> ShareResult<()> {
    use std::process::Command;

    let output = Command::new("net")
        .args(["usershare", "delete", share_name])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::RemovalFailed(error.to_string()))
    }
}

#[cfg(target_os = "linux")]
fn enumerate_linux_shares() -> ShareResult<Vec<ShareInfo>> {
    use std::process::Command;

    let output = Command::new("net")
        .args(["usershare", "list"])
        .output();

    let share_names: Vec<String> = match output {
        Ok(result) if result.status.success() => {
            String::from_utf8_lossy(&result.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => return Ok(Vec::new()),
    };

    let mut shares = Vec::new();

    for share_name in share_names {
        // Get info for each share
        let info_output = Command::new("net")
            .args(["usershare", "info", &share_name])
            .output();

        if let Ok(result) = info_output {
            if result.status.success() {
                let info_str = String::from_utf8_lossy(&result.stdout);
                if let Some(share) = parse_linux_share_info(&share_name, &info_str) {
                    shares.push(share);
                }
            }
        }
    }

    Ok(shares)
}

#[cfg(target_os = "linux")]
fn parse_linux_share_info(share_name: &str, info: &str) -> Option<ShareInfo> {
    let mut path = None;
    let mut description = String::new();

    for line in info.lines() {
        let line = line.trim();
        if line.starts_with("path=") {
            path = Some(PathBuf::from(line.trim_start_matches("path=")));
        } else if line.starts_with("comment=") {
            description = line.trim_start_matches("comment=").to_string();
        }
    }

    path.map(|p| ShareInfo {
        share_name: share_name.to_string(),
        path: p,
        description,
        permission: SharePermission::ReadOnly,
        current_users: 0,
        max_users: None,
    })
}

// macOS-specific share implementation
#[cfg(target_os = "macos")]
fn create_macos_share(config: &ShareConfig) -> ShareResult<()> {
    use std::process::Command;

    // macOS uses sharing command or System Preferences
    // For programmatic access, we use the sharing command
    let path_str = config.path.to_string_lossy();

    // Enable SMB sharing for the folder
    let output = Command::new("sharing")
        .args([
            "-a",
            &path_str,
            "-n",
            &config.share_name,
            "-s",
            "001", // SMB sharing
        ])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::CreationFailed(error.to_string()))
    }
}

#[cfg(target_os = "macos")]
fn remove_macos_share(share_name: &str) -> ShareResult<()> {
    use std::process::Command;

    let output = Command::new("sharing")
        .args(["-r", share_name])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::RemovalFailed(error.to_string()))
    }
}

#[cfg(target_os = "macos")]
fn enumerate_macos_shares() -> ShareResult<Vec<ShareInfo>> {
    use std::process::Command;

    let output = Command::new("sharing")
        .args(["-l"])
        .output();

    let shares = match output {
        Ok(result) if result.status.success() => {
            parse_macos_shares(&String::from_utf8_lossy(&result.stdout))
        }
        _ => Vec::new(),
    };

    Ok(shares)
}

#[cfg(target_os = "macos")]
fn parse_macos_shares(output: &str) -> Vec<ShareInfo> {
    let mut shares = Vec::new();
    let mut current_name = None;
    let mut current_path = None;

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("name:") {
            if let (Some(name), Some(path)) = (current_name.take(), current_path.take()) {
                shares.push(ShareInfo {
                    share_name: name,
                    path,
                    description: String::new(),
                    permission: SharePermission::ReadOnly,
                    current_users: 0,
                    max_users: None,
                });
            }
            current_name = Some(line.trim_start_matches("name:").trim().to_string());
        } else if line.starts_with("path:") {
            current_path = Some(PathBuf::from(line.trim_start_matches("path:").trim()));
        }
    }

    // Don't forget the last share
    if let (Some(name), Some(path)) = (current_name, current_path) {
        shares.push(ShareInfo {
            share_name: name,
            path,
            description: String::new(),
            permission: SharePermission::ReadOnly,
            current_users: 0,
            max_users: None,
        });
    }

    shares
}

/// Get available platform-specific sharing methods for the current platform
pub fn get_available_share_methods() -> Vec<PlatformShareMethod> {
    let mut methods = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if is_airdrop_available() {
            methods.push(PlatformShareMethod::AirDrop);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if is_nearby_share_available() {
            methods.push(PlatformShareMethod::NearbyShare);
        }
    }

    // Network share is available on all platforms
    methods.push(PlatformShareMethod::NetworkShare);
    methods.push(PlatformShareMethod::Clipboard);

    methods
}

/// Check if a specific share method is available
pub fn is_share_method_available(method: PlatformShareMethod) -> bool {
    match method {
        PlatformShareMethod::AirDrop => {
            #[cfg(target_os = "macos")]
            {
                is_airdrop_available()
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        }
        PlatformShareMethod::NearbyShare => {
            #[cfg(target_os = "windows")]
            {
                is_nearby_share_available()
            }
            #[cfg(not(target_os = "windows"))]
            {
                false
            }
        }
        PlatformShareMethod::NetworkShare => true,
        PlatformShareMethod::Clipboard => true,
    }
}

// ============================================================================
// macOS AirDrop Implementation
// ============================================================================

#[cfg(target_os = "macos")]
pub fn is_airdrop_available() -> bool {
    use std::process::Command;

    // Check if AirDrop is available by checking Bluetooth and WiFi status
    // AirDrop requires both Bluetooth and WiFi to be enabled
    
    // Check if the system supports AirDrop (macOS 10.7+)
    let output = Command::new("system_profiler")
        .args(["SPBluetoothDataType", "-json"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            // Check if Bluetooth is available and powered on
            stdout.contains("\"controller_state\" : \"attrib_on\"") 
                || stdout.contains("state_on")
                || stdout.contains("\"bluetooth_power\" : \"on\"")
                || stdout.contains("\"controller_powerState\" : \"attrib_on\"")
                // Fallback: if we can query Bluetooth, assume it might be available
                || stdout.contains("SPBluetoothDataType")
        }
        _ => {
            // Fallback: check if AirDrop service exists
            std::path::Path::new("/System/Library/CoreServices/Finder.app").exists()
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_airdrop_available() -> bool {
    false
}

/// Share files via AirDrop (macOS only)
/// Opens Finder, selects the files, and triggers the Share menu
#[cfg(target_os = "macos")]
pub fn share_via_airdrop(paths: &[PathBuf]) -> ShareResult<()> {
    use std::process::Command;

    if paths.is_empty() {
        return Err(ShareError::CreationFailed("No files to share".to_string()));
    }

    for path in paths {
        if !path.exists() {
            return Err(ShareError::PathNotFound(path.clone()));
        }
    }

    // Build POSIX file references for AppleScript
    let file_refs: Vec<String> = paths
        .iter()
        .map(|p| {
            let escaped = p.display().to_string().replace("\\", "\\\\").replace("\"", "\\\"");
            format!("POSIX file \"{}\"", escaped)
        })
        .collect();
    let files_str = file_refs.join(", ");

    // Use Finder to select files and open the Share menu
    let script = format!(
        r#"
tell application "Finder"
    activate
    set theFiles to {{{}}}
    select theFiles
    delay 0.2
end tell

tell application "System Events"
    tell process "Finder"
        -- Click File menu then Share submenu
        click menu item "Share…" of menu 1 of menu bar item "File" of menu bar 1
    end tell
end tell
"#,
        files_str
    );

    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::AirDropUnavailable(format!(
            "Failed to open share menu: {}",
            error
        )))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn share_via_airdrop(_paths: &[PathBuf]) -> ShareResult<()> {
    Err(ShareError::PlatformNotSupported(
        "AirDrop is only available on macOS".to_string(),
    ))
}

/// Open AirDrop window (macOS only)
#[cfg(target_os = "macos")]
pub fn open_airdrop_window() -> ShareResult<()> {
    use std::process::Command;

    let output = Command::new("open")
        .args(["-a", "AirDrop"])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::AirDropUnavailable(error.to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn open_airdrop_window() -> ShareResult<()> {
    Err(ShareError::PlatformNotSupported(
        "AirDrop is only available on macOS".to_string(),
    ))
}

/// Open the native macOS share sheet for files (includes AirDrop, Messages, Mail, etc.)
#[cfg(target_os = "macos")]
pub fn open_macos_share_sheet(paths: &[PathBuf]) -> ShareResult<()> {
    // Just delegate to share_via_airdrop which now opens the full share menu
    share_via_airdrop(paths)
}

#[cfg(not(target_os = "macos"))]
pub fn open_macos_share_sheet(_paths: &[PathBuf]) -> ShareResult<()> {
    Err(ShareError::PlatformNotSupported(
        "macOS share sheet is only available on macOS".to_string(),
    ))
}

// ============================================================================
// Windows Nearby Share Implementation
// ============================================================================

#[cfg(target_os = "windows")]
pub fn is_nearby_share_available() -> bool {
    use std::process::Command;

    // Check Windows version (Nearby Share requires Windows 10 1803+)
    let output = Command::new("cmd")
        .args(["/C", "ver"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let version = String::from_utf8_lossy(&result.stdout);
            // Windows 10 build 17134 (1803) or later supports Nearby Share
            if let Some(build) = extract_windows_build(&version) {
                build >= 17134
            } else {
                // Assume available on modern Windows
                true
            }
        }
        _ => false,
    }
}

#[cfg(target_os = "windows")]
fn extract_windows_build(version_str: &str) -> Option<u32> {
    // Parse version string like "Microsoft Windows [Version 10.0.19041.1234]"
    let start = version_str.find('[')?;
    let end = version_str.find(']')?;
    let version_part = &version_str[start + 1..end];
    
    // Extract build number (third part after "Version X.Y.")
    let parts: Vec<&str> = version_part.split('.').collect();
    if parts.len() >= 3 {
        parts[2].split('.').next()?.parse().ok()
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_nearby_share_available() -> bool {
    false
}

/// Share files via Windows Nearby Share
/// Uses Windows.ApplicationModel.DataTransfer APIs via PowerShell
#[cfg(target_os = "windows")]
pub fn share_via_nearby_share(paths: &[PathBuf]) -> ShareResult<()> {
    use std::process::Command;

    if paths.is_empty() {
        return Err(ShareError::CreationFailed("No files to share".to_string()));
    }

    // Verify all paths exist
    for path in paths {
        if !path.exists() {
            return Err(ShareError::PathNotFound(path.clone()));
        }
    }

    // Build PowerShell script to invoke Windows Share UI
    // This uses the DataTransferManager to show the native share dialog
    let file_paths: Vec<String> = paths
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "\\\\"))
        .collect();
    let files_array = file_paths.join("\",\"");

    let script = format!(
        r#"
        Add-Type -AssemblyName System.Runtime.WindowsRuntime
        
        $files = @("{}")
        
        # Use explorer's share functionality as fallback
        foreach ($file in $files) {{
            Start-Process explorer.exe -ArgumentList "/select,`"$file`""
        }}
        
        # Open Share panel via keyboard shortcut simulation
        Add-Type -TypeDefinition @"
        using System;
        using System.Runtime.InteropServices;
        public class ShareHelper {{
            [DllImport("user32.dll")]
            public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
            
            public const byte VK_LWIN = 0x5B;
            public const byte VK_H = 0x48;
            public const uint KEYEVENTF_KEYUP = 0x0002;
            
            public static void OpenSharePanel() {{
                keybd_event(VK_LWIN, 0, 0, UIntPtr.Zero);
                keybd_event(VK_H, 0, 0, UIntPtr.Zero);
                keybd_event(VK_H, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
                keybd_event(VK_LWIN, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
            }}
        }}
"@
        
        Start-Sleep -Milliseconds 500
        [ShareHelper]::OpenSharePanel()
        "#,
        files_array
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::NearbyShareUnavailable(format!(
            "Failed to open Nearby Share: {}",
            if error.is_empty() {
                "Nearby Share may be disabled in Windows Settings"
            } else {
                &error
            }
        )))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn share_via_nearby_share(_paths: &[PathBuf]) -> ShareResult<()> {
    Err(ShareError::PlatformNotSupported(
        "Nearby Share is only available on Windows 10/11".to_string(),
    ))
}

/// Open Windows Nearby Share settings
#[cfg(target_os = "windows")]
pub fn open_nearby_share_settings() -> ShareResult<()> {
    use std::process::Command;

    let output = Command::new("cmd")
        .args(["/C", "start", "ms-settings:crossdevice"])
        .output()
        .map_err(|e| ShareError::Io(e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(ShareError::NearbyShareUnavailable(error.to_string()))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open_nearby_share_settings() -> ShareResult<()> {
    Err(ShareError::PlatformNotSupported(
        "Nearby Share settings are only available on Windows".to_string(),
    ))
}

// ============================================================================
// Generic Platform Share Function
// ============================================================================

/// Share files using the specified platform method
pub fn share_files(paths: &[PathBuf], method: PlatformShareMethod) -> ShareResult<()> {
    match method {
        PlatformShareMethod::AirDrop => share_via_airdrop(paths),
        PlatformShareMethod::NearbyShare => share_via_nearby_share(paths),
        PlatformShareMethod::NetworkShare => {
            // Network share requires the ShareManager and ShareConfig
            Err(ShareError::CreationFailed(
                "Use ShareManager.create_share() for network shares".to_string(),
            ))
        }
        PlatformShareMethod::Clipboard => {
            // Copy paths to clipboard
            let paths_str: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            let clipboard_text = paths_str.join("\n");
            copy_to_clipboard(&clipboard_text)
        }
    }
}

/// Copy text to system clipboard
fn copy_to_clipboard(text: &str) -> ShareResult<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::{Command, Stdio};
        use std::io::Write;

        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| ShareError::Io(e))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(text.as_bytes()).map_err(|e| ShareError::Io(e))?;
        }

        child.wait().map_err(|e| ShareError::Io(e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let script = format!(
            "Set-Clipboard -Value '{}'",
            text.replace('\'', "''")
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| ShareError::Io(e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(ShareError::CreationFailed(error.to_string()))
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::{Command, Stdio};
        use std::io::Write;

        // Try xclip first, then xsel
        let result = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes()).map_err(|e| ShareError::Io(e))?;
                }
                child.wait().map_err(|e| ShareError::Io(e))?;
                Ok(())
            }
            Err(_) => {
                // Try xsel as fallback
                let mut child = Command::new("xsel")
                    .args(["--clipboard", "--input"])
                    .stdin(Stdio::piped())
                    .spawn()
                    .map_err(|e| ShareError::Io(e))?;

                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes()).map_err(|e| ShareError::Io(e))?;
                }
                child.wait().map_err(|e| ShareError::Io(e))?;
                Ok(())
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(ShareError::PlatformNotSupported(
            "Clipboard not supported on this platform".to_string(),
        ))
    }
}

/// Get a user-friendly message about why a share method is unavailable
pub fn get_share_method_unavailable_reason(method: PlatformShareMethod) -> Option<String> {
    match method {
        PlatformShareMethod::AirDrop => {
            #[cfg(target_os = "macos")]
            {
                if !is_airdrop_available() {
                    Some("AirDrop requires Bluetooth and WiFi to be enabled. Check System Preferences > General > AirDrop & Handoff.".to_string())
                } else {
                    None
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                Some("AirDrop is only available on macOS".to_string())
            }
        }
        PlatformShareMethod::NearbyShare => {
            #[cfg(target_os = "windows")]
            {
                if !is_nearby_share_available() {
                    Some("Nearby Share requires Windows 10 version 1803 or later. Check Settings > System > Nearby sharing.".to_string())
                } else {
                    None
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Some("Nearby Share is only available on Windows 10/11".to_string())
            }
        }
        PlatformShareMethod::NetworkShare => None,
        PlatformShareMethod::Clipboard => None,
    }
}

#[cfg(test)]
#[path = "file_share_tests.rs"]
mod tests;
