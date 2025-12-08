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

#[cfg(test)]
#[path = "file_share_tests.rs"]
mod tests;
