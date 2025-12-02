mod coalescer;
mod linux;
mod macos;
mod windows;
mod watcher;

pub use coalescer::EventCoalescer;
pub use linux::*;
pub use macos::*;
pub use windows::*;
pub use watcher::*;
