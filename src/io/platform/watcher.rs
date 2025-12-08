use std::path::Path;
use std::time::Duration;

use crate::models::{FsEvent, Result};

pub const DEFAULT_COALESCE_WINDOW: Duration = Duration::from_millis(50);

pub trait Watcher: Send {
    fn watch(&mut self, path: &Path) -> Result<()>;

    fn unwatch(&mut self, path: &Path) -> Result<()>;

    fn poll_events(&mut self) -> Vec<FsEvent>;

    fn set_coalesce_window(&mut self, window: Duration);

    fn coalesce_window(&self) -> Duration;
}

pub trait PlatformFs: Send + Sync {
    fn create_watcher(&self) -> Box<dyn Watcher>;

    fn supports_mft_index(&self) -> bool;

    fn platform_name(&self) -> &'static str;
}

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
