use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};

use super::coalescer::EventCoalescer;
use super::watcher::{PlatformFs, Watcher};
use crate::models::{FileSystemError, FsEvent, Result};

pub struct LinuxPlatform;

impl PlatformFs for LinuxPlatform {
    fn create_watcher(&self) -> Box<dyn Watcher> {
        Box::new(LinuxWatcher::new())
    }

    fn supports_mft_index(&self) -> bool {
        false
    }

    fn platform_name(&self) -> &'static str {
        "Linux"
    }
}


pub struct LinuxWatcher {
    watcher: Option<RecommendedWatcher>,
    event_rx: Option<Receiver<notify::Result<Event>>>,
    coalescer: EventCoalescer,
    watched_paths: Vec<PathBuf>,
}

impl LinuxWatcher {
    pub fn new() -> Self {
        Self {
            watcher: None,
            event_rx: None,
            coalescer: EventCoalescer::new(),
            watched_paths: Vec::new(),
        }
    }

    fn ensure_watcher(&mut self) -> Result<()> {
        if self.watcher.is_some() {
            return Ok(());
        }

        let (tx, rx) = channel();
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| FileSystemError::Platform(format!("Failed to create watcher: {}", e)))?;

        self.watcher = Some(watcher);
        self.event_rx = Some(rx);
        Ok(())
    }

    fn convert_event(&self, event: &Event) -> Option<FsEvent> {
        let path = event.paths.first()?.clone();

        match &event.kind {
            EventKind::Create(_) => Some(FsEvent::Created(path)),
            EventKind::Modify(_) => Some(FsEvent::Modified(path)),
            EventKind::Remove(_) => Some(FsEvent::Deleted(path)),
            EventKind::Any => Some(FsEvent::Modified(path)),
            _ => None,
        }
    }

    fn drain_raw_events(&mut self) -> Vec<FsEvent> {
        let mut events = Vec::new();

        if let Some(rx) = &self.event_rx {
            loop {
                match rx.try_recv() {
                    Ok(Ok(event)) => {
                        if let Some(fs_event) = self.convert_event(&event) {
                            events.push(fs_event);
                        }
                    }
                    Ok(Err(_)) => continue,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.watcher = None;
                        self.event_rx = None;
                        break;
                    }
                }
            }
        }

        events
    }
}

impl Default for LinuxWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Watcher for LinuxWatcher {
    fn watch(&mut self, path: &Path) -> Result<()> {
        self.ensure_watcher()?;

        if let Some(watcher) = &mut self.watcher {
            watcher
                .watch(path, RecursiveMode::NonRecursive)
                .map_err(|e| FileSystemError::Platform(format!("Failed to watch path: {}", e)))?;
            self.watched_paths.push(path.to_path_buf());
        }

        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> Result<()> {
        if let Some(watcher) = &mut self.watcher {
            watcher
                .unwatch(path)
                .map_err(|e| FileSystemError::Platform(format!("Failed to unwatch path: {}", e)))?;
            self.watched_paths.retain(|p| p != path);
        }

        Ok(())
    }

    fn poll_events(&mut self) -> Vec<FsEvent> {
        let raw_events = self.drain_raw_events();
        self.coalescer.add_events(raw_events);
        self.coalescer.poll_ready()
    }

    fn set_coalesce_window(&mut self, window: Duration) {
        self.coalescer.set_coalesce_window(window);
    }

    fn coalesce_window(&self) -> Duration {
        self.coalescer.coalesce_window()
    }
}
