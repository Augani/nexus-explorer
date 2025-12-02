use std::path::Path;
use std::time::Duration;

use crate::models::{FsEvent, Result};

/// Default coalescing window for debouncing rapid events
pub const DEFAULT_COALESCE_WINDOW: Duration = Duration::from_millis(50);

/// File system change watcher trait.
/// 
/// Platform-specific implementations wrap native APIs (FSEvents, inotify, USN Journal)
/// to provide a unified interface for monitoring file system changes.
pub trait Watcher: Send {
    /// Start watching the specified path for changes.
    /// 
    /// The path can be a file or directory. For directories, changes to
    /// immediate children are monitored.
    fn watch(&mut self, path: &Path) -> Result<()>;

    /// Stop watching the specified path.
    fn unwatch(&mut self, path: &Path) -> Result<()>;

    /// Poll for pending file system events.
    /// 
    /// Returns a vector of coalesced events since the last poll.
    /// Events on the same path within the coalescing window are merged.
    fn poll_events(&mut self) -> Vec<FsEvent>;

    /// Set the coalescing window for event debouncing.
    /// 
    /// Events on the same path within this duration are merged to prevent
    /// update storms from rapid successive changes.
    fn set_coalesce_window(&mut self, window: Duration);

    /// Get the current coalescing window duration.
    fn coalesce_window(&self) -> Duration;
}

/// Platform-specific file system operations trait.
/// 
/// Provides factory methods for creating platform-appropriate watchers
/// and querying platform capabilities.
pub trait PlatformFs: Send + Sync {
    /// Create a new file system watcher appropriate for this platform.
    fn create_watcher(&self) -> Box<dyn Watcher>;

    /// Check if this platform supports MFT index acceleration (Windows NTFS only).
    fn supports_mft_index(&self) -> bool;

    /// Get the platform name for logging/debugging.
    fn platform_name(&self) -> &'static str;
}

/// Detect and return the appropriate platform implementation.
pub fn detect_platform() -> Box<dyn PlatformFs> {
    #[cfg(target_os = "macos")]
    {
        Box::new(super::MacOsPlatform)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(super::LinuxPlatform)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(super::WindowsPlatform)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        compile_error!("Unsupported platform")
    }
}
