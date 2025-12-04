mod coalescer;
mod linux;
mod macos;
mod watcher;
mod windows;

pub use coalescer::EventCoalescer;
pub use linux::*;
pub use macos::*;
pub use watcher::*;
pub use windows::*;
