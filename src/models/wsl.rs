/// WSL (Windows Subsystem for Linux) Integration
///
/// This module provides functionality for detecting, browsing, and interacting
/// with WSL distributions on Windows systems.
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during WSL operations
#[derive(Debug, Error)]
pub enum WslError {
    #[error("WSL is not installed")]
    NotInstalled,

    #[error("Distribution not found: {0}")]
    DistributionNotFound(String),

    #[error("Path translation failed: {0}")]
    PathTranslationFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type WslResult<T> = std::result::Result<T, WslError>;

/// WSL distribution information
#[derive(Debug, Clone, PartialEq)]
pub struct WslDistro {
    pub name: String,
    pub is_default: bool,
    pub is_running: bool,
    pub version: u8,
    pub state: WslState,
}

/// State of a WSL distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslState {
    Running,
    Stopped,
    Installing,
    Converting,
    Unknown,
}

impl WslState {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" => WslState::Running,
            "stopped" => WslState::Stopped,
            "installing" => WslState::Installing,
            "converting" => WslState::Converting,
            _ => WslState::Unknown,
        }
    }
}

/// WSL manager for detecting and interacting with WSL distributions
pub struct WslManager {
    distributions: Vec<WslDistro>,
    is_available: bool,
}

impl Default for WslManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WslManager {
    pub fn new() -> Self {
        let mut manager = Self {
            distributions: Vec::new(),
            is_available: false,
        };
        manager.refresh();
        manager
    }

    /// Check if WSL is installed and available
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Get all detected WSL distributions
    pub fn distributions(&self) -> &[WslDistro] {
        &self.distributions
    }

    /// Get a distribution by name
    pub fn get_distribution(&self, name: &str) -> Option<&WslDistro> {
        self.distributions
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case(name))
    }

    /// Get the default distribution
    pub fn default_distribution(&self) -> Option<&WslDistro> {
        self.distributions.iter().find(|d| d.is_default)
    }

    /// Refresh the list of WSL distributions
    pub fn refresh(&mut self) {
        self.distributions.clear();
        self.is_available = false;

        #[cfg(target_os = "windows")]
        {
            self.detect_wsl_windows();
        }
    }

    /// Detect WSL distributions on Windows
    #[cfg(target_os = "windows")]
    fn detect_wsl_windows(&mut self) {
        let status_check = std::process::Command::new("wsl")
            .args(["--status"])
            .output();

        if status_check.is_err() {
            return;
        }

        let output = match std::process::Command::new("wsl")
            .args(["--list", "--verbose"])
            .output()
        {
            Ok(o) => o,
            Err(_) => return,
        };

        if !output.status.success() {
            return;
        }

        self.is_available = true;

        let stdout = decode_wsl_output(&output.stdout);

        self.parse_wsl_list_output(&stdout);
    }

    /// Parse the output of `wsl --list --verbose`
    fn parse_wsl_list_output(&mut self, output: &str) {
        // Skip the header line and any empty lines
        for line in output.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse line format: "* Ubuntu    Running    2" or "  Debian    Stopped    1"
            let is_default = line.starts_with('*');
            let line = line.trim_start_matches('*').trim();

            // Split by whitespace, handling multiple spaces
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let state = WslState::from_str(parts[1]);
                let version = parts[2].parse().unwrap_or(2);

                let distro = WslDistro {
                    name,
                    is_default,
                    is_running: state == WslState::Running,
                    version,
                    state,
                };

                self.distributions.push(distro);
            }
        }
    }

    /// Get the UNC path for a WSL distribution
    pub fn get_unc_path(distro_name: &str) -> PathBuf {
        PathBuf::from(format!("\\\\wsl$\\{}", distro_name))
    }

    /// Get the UNC path using wsl.localhost (newer format)
    pub fn get_unc_path_localhost(distro_name: &str) -> PathBuf {
        PathBuf::from(format!("\\\\wsl.localhost\\{}", distro_name))
    }

    /// Translate a Windows path to a WSL path
    pub fn windows_to_wsl_path(windows_path: &Path) -> WslResult<String> {
        let path_str = windows_path.to_string_lossy();

        let wsl_prefix = "\\\\wsl$\\";
        let wsl_localhost_prefix = "\\\\wsl.localhost\\";

        if path_str.starts_with(wsl_prefix) || path_str.starts_with(wsl_localhost_prefix) {
            let without_prefix = if path_str.starts_with(wsl_prefix) {
                &path_str[wsl_prefix.len()..]
            } else {
                &path_str[wsl_localhost_prefix.len()..]
            };

            // Find the distribution name (first path component)
            if let Some(slash_pos) = without_prefix.find('\\') {
                let linux_path = &without_prefix[slash_pos..];
                // Convert backslashes to forward slashes
                return Ok(linux_path.replace('\\', "/"));
            } else {
                // Just the distribution root
                return Ok("/".to_string());
            }
        }

        if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
            if let Some(drive_letter) = path_str.chars().next() {
                let rest = &path_str[2..].replace('\\', "/");
                return Ok(format!(
                    "/mnt/{}{}",
                    drive_letter.to_ascii_lowercase(),
                    rest
                ));
            }
        }

        Err(WslError::PathTranslationFailed(format!(
            "Cannot translate path: {}",
            path_str
        )))
    }

    /// Translate a WSL path to a Windows path for a specific distribution
    pub fn wsl_to_windows_path(distro_name: &str, wsl_path: &str) -> WslResult<PathBuf> {
        if wsl_path.starts_with("/mnt/") && wsl_path.len() >= 6 {
            if let Some(drive_letter) = wsl_path.chars().nth(5) {
                let rest = &wsl_path[6..];
                let windows_path = format!(
                    "{}:{}",
                    drive_letter.to_ascii_uppercase(),
                    rest.replace('/', "\\")
                );
                return Ok(PathBuf::from(windows_path));
            }
        }

        // Build the path manually to avoid PathBuf::join issues with UNC paths
        let linux_path = wsl_path.replace('/', "\\");
        let linux_path = linux_path.trim_start_matches('\\');

        let full_path = format!("\\\\wsl$\\{}\\{}", distro_name, linux_path);
        Ok(PathBuf::from(full_path))
    }

    /// Check if a path is a WSL path
    pub fn is_wsl_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.starts_with("\\\\wsl$\\") || path_str.starts_with("\\\\wsl.localhost\\")
    }

    /// Extract the distribution name from a WSL path
    pub fn extract_distro_name(path: &Path) -> Option<String> {
        let path_str = path.to_string_lossy();

        let wsl_prefix = "\\\\wsl$\\";
        let wsl_localhost_prefix = "\\\\wsl.localhost\\";

        let without_prefix = if path_str.starts_with(wsl_prefix) {
            Some(&path_str[wsl_prefix.len()..])
        } else if path_str.starts_with(wsl_localhost_prefix) {
            Some(&path_str[wsl_localhost_prefix.len()..])
        } else {
            None
        }?;

        let end = without_prefix.find('\\').unwrap_or(without_prefix.len());
        Some(without_prefix[..end].to_string())
    }

    /// Start a WSL distribution
    #[cfg(target_os = "windows")]
    pub fn start_distribution(&self, name: &str) -> WslResult<()> {
        let output = std::process::Command::new("wsl")
            .args(["-d", name, "--", "echo", "started"])
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(WslError::CommandFailed(error.to_string()))
        }
    }

    /// Terminate a WSL distribution
    #[cfg(target_os = "windows")]
    pub fn terminate_distribution(&self, name: &str) -> WslResult<()> {
        let output = std::process::Command::new("wsl")
            .args(["--terminate", name])
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(WslError::CommandFailed(error.to_string()))
        }
    }

    /// Execute a command in a WSL distribution
    #[cfg(target_os = "windows")]
    pub fn execute_command(&self, distro: &str, command: &str) -> WslResult<String> {
        let output = std::process::Command::new("wsl")
            .args(["-d", distro, "--", "sh", "-c", command])
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(WslError::CommandFailed(error.to_string()))
        }
    }

    /// Get Linux file permissions for a path in WSL
    #[cfg(target_os = "windows")]
    pub fn get_linux_permissions(
        &self,
        distro: &str,
        linux_path: &str,
    ) -> WslResult<LinuxPermissions> {
        let command = format!("stat -c '%a %U %G' '{}'", linux_path);
        let output = self.execute_command(distro, &command)?;

        let parts: Vec<&str> = output.trim().split_whitespace().collect();
        if parts.len() >= 3 {
            Ok(LinuxPermissions {
                mode: u32::from_str_radix(parts[0], 8).unwrap_or(0),
                owner: parts[1].to_string(),
                group: parts[2].to_string(),
            })
        } else {
            Err(WslError::CommandFailed(
                "Failed to parse permissions".to_string(),
            ))
        }
    }

    /// Open a terminal in a WSL distribution at the specified path
    /// This opens Windows Terminal (wt.exe) or falls back to cmd with wsl
    #[cfg(target_os = "windows")]
    pub fn open_terminal_here(distro: &str, linux_path: &str) -> WslResult<()> {
        // Try Windows Terminal first (preferred)
        let wt_result = std::process::Command::new("wt")
            .args(["-d", linux_path, "wsl", "-d", distro])
            .spawn();

        if wt_result.is_ok() {
            return Ok(());
        }

        // Fall back to cmd with wsl
        let cmd_result = std::process::Command::new("cmd")
            .args(["/c", "start", "wsl", "-d", distro, "--cd", linux_path])
            .spawn();

        match cmd_result {
            Ok(_) => Ok(()),
            Err(e) => Err(WslError::CommandFailed(format!(
                "Failed to open terminal: {}",
                e
            ))),
        }
    }

    /// Open a terminal in a WSL distribution from a Windows UNC path
    #[cfg(target_os = "windows")]
    pub fn open_terminal_at_path(path: &Path) -> WslResult<()> {
        if !Self::is_wsl_path(path) {
            return Err(WslError::PathTranslationFailed(
                "Not a WSL path".to_string(),
            ));
        }

        let distro = Self::extract_distro_name(path).ok_or_else(|| {
            WslError::PathTranslationFailed("Could not extract distribution name".to_string())
        })?;

        let linux_path = Self::windows_to_wsl_path(path)?;

        Self::open_terminal_here(&distro, &linux_path)
    }
}

/// Linux file permissions
#[derive(Debug, Clone, PartialEq)]
pub struct LinuxPermissions {
    pub mode: u32,
    pub owner: String,
    pub group: String,
}

impl LinuxPermissions {
    /// Format permissions as rwxrwxrwx string
    pub fn format_mode(&self) -> String {
        let mut result = String::with_capacity(9);

        // Owner permissions
        result.push(if self.mode & 0o400 != 0 { 'r' } else { '-' });
        result.push(if self.mode & 0o200 != 0 { 'w' } else { '-' });
        result.push(if self.mode & 0o100 != 0 { 'x' } else { '-' });

        // Group permissions
        result.push(if self.mode & 0o040 != 0 { 'r' } else { '-' });
        result.push(if self.mode & 0o020 != 0 { 'w' } else { '-' });
        result.push(if self.mode & 0o010 != 0 { 'x' } else { '-' });

        // Other permissions
        result.push(if self.mode & 0o004 != 0 { 'r' } else { '-' });
        result.push(if self.mode & 0o002 != 0 { 'w' } else { '-' });
        result.push(if self.mode & 0o001 != 0 { 'x' } else { '-' });

        result
    }

    /// Format as full permission string like "-rwxr-xr-x owner group"
    pub fn format_full(&self) -> String {
        format!("-{} {} {}", self.format_mode(), self.owner, self.group)
    }
}

/// Decode WSL command output (handles UTF-16LE on Windows)
#[cfg(target_os = "windows")]
fn decode_wsl_output(bytes: &[u8]) -> String {
    // WSL outputs UTF-16LE with BOM on Windows
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16LE with BOM
        let u16_iter = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
        char::decode_utf16(u16_iter)
            .filter_map(|r| r.ok())
            .collect()
    } else if bytes.iter().step_by(2).skip(1).all(|&b| b == 0) && bytes.len() > 1 {
        // UTF-16LE without BOM (every other byte is 0)
        let u16_iter = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
        char::decode_utf16(u16_iter)
            .filter_map(|r| r.ok())
            .collect()
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

#[cfg(not(target_os = "windows"))]
fn decode_wsl_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}
