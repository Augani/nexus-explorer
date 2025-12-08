use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::coalescer::EventCoalescer;
use super::watcher::{PlatformFs, Watcher, DEFAULT_COALESCE_WINDOW};
use crate::models::{FileSystemError, FsEvent, Result};

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileReferenceNumber(pub u64);

impl FileReferenceNumber {
    /
    pub fn record_number(&self) -> u64 {
        self.0 & 0x0000_FFFF_FFFF_FFFF
    }

    /
    pub fn sequence_number(&self) -> u16 {
        ((self.0 >> 48) & 0xFFFF) as u16
    }

    /
    pub fn new(record_number: u64, sequence_number: u16) -> Self {
        Self((record_number & 0x0000_FFFF_FFFF_FFFF) | ((sequence_number as u64) << 48))
    }

    /
    pub const ROOT: Self = Self(5);
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub parent: FileReferenceNumber,
    pub is_directory: bool,
    pub size: u64,
    pub created: u64,
    pub modified: u64,
    pub attributes: u32,
}

impl FileNode {
    pub fn new(
        name: String,
        parent: FileReferenceNumber,
        is_directory: bool,
        size: u64,
        created: u64,
        modified: u64,
        attributes: u32,
    ) -> Self {
        Self {
            name,
            parent,
            is_directory,
            size,
            created,
            modified,
            attributes,
        }
    }
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MftIndex {
    /
    pub files: HashMap<FileReferenceNumber, FileNode>,
    /
    pub usn_cursor: u64,
    /
    pub volume_path: PathBuf,
    /
    #[serde(skip)]
    path_cache: HashMap<FileReferenceNumber, PathBuf>,
}

impl MftIndex {
    /
    pub fn new(volume_path: PathBuf) -> Self {
        Self {
            files: HashMap::new(),
            usn_cursor: 0,
            volume_path,
            path_cache: HashMap::new(),
        }
    }

    /
    pub fn insert(&mut self, frn: FileReferenceNumber, node: FileNode) {
        self.path_cache.remove(&frn);
        self.files.insert(frn, node);
    }

    /
    pub fn remove(&mut self, frn: &FileReferenceNumber) -> Option<FileNode> {
        self.path_cache.remove(frn);
        self.files.remove(frn)
    }

    /
    pub fn get(&self, frn: &FileReferenceNumber) -> Option<&FileNode> {
        self.files.get(frn)
    }

    /
    pub fn contains(&self, frn: &FileReferenceNumber) -> bool {
        self.files.contains_key(frn)
    }

    /
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /
    /
    /
    /
    pub fn reconstruct_path(&mut self, frn: &FileReferenceNumber) -> Option<PathBuf> {
        if let Some(cached) = self.path_cache.get(frn) {
            return Some(cached.clone());
        }

        let path = self.build_path_uncached(frn)?;
        self.path_cache.insert(*frn, path.clone());
        Some(path)
    }

    /
    fn build_path_uncached(&self, frn: &FileReferenceNumber) -> Option<PathBuf> {
        let mut components: Vec<&str> = Vec::new();
        let mut current = *frn;
        let mut visited = std::collections::HashSet::new();

        loop {
            if !visited.insert(current) {
                return None;
            }

            let node = self.files.get(&current)?;

            if current == FileReferenceNumber::ROOT {
                break;
            }

            components.push(&node.name);
            current = node.parent;
        }

        components.reverse();
        let mut path = self.volume_path.clone();
        for component in components {
            path.push(component);
        }

        Some(path)
    }

    /
    pub fn clear_path_cache(&mut self) {
        self.path_cache.clear();
    }

    /
    pub fn set_usn_cursor(&mut self, cursor: u64) {
        self.usn_cursor = cursor;
    }

    /
    pub fn find_by_path(&mut self, path: &Path) -> Option<FileReferenceNumber> {
        let frns: Vec<FileReferenceNumber> = self.files.keys().copied().collect();

        for frn in frns {
            if let Some(reconstructed) = self.reconstruct_path(&frn) {
                if reconstructed == path {
                    return Some(frn);
                }
            }
        }
        None
    }

    /
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(FileSystemError::Serialization)
    }

    /
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(FileSystemError::Serialization)
    }

    /
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let data = self.serialize()?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::deserialize(&data)
    }
}

/
/
/
/
pub struct MftParser {
    volume_path: PathBuf,
}

impl MftParser {
    pub fn new(volume_path: PathBuf) -> Self {
        Self { volume_path }
    }

    /
    /
    /
    /
    #[cfg(target_os = "windows")]
    pub fn parse(&self) -> Result<MftIndex> {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::AsRawHandle;

        let volume_handle = OpenOptions::new()
            .read(true)
            .custom_flags(0x80000000)
            .open(&self.volume_path)
            .map_err(|e| FileSystemError::Platform(format!("Failed to open volume: {}", e)))?;

        let mut index = MftIndex::new(self.volume_path.clone());

        index.insert(
            FileReferenceNumber::ROOT,
            FileNode::new(
                String::new(),
                FileReferenceNumber::ROOT,
                true,
                0,
                0,
                0,
                0x10,
            ),
        );


        Ok(index)
    }

    /
    #[cfg(not(target_os = "windows"))]
    pub fn parse(&self) -> Result<MftIndex> {
        Err(FileSystemError::Platform(
            "MFT parsing is only supported on Windows".to_string(),
        ))
    }
}

/
#[derive(Debug, Clone)]
pub struct UsnRecord {
    pub frn: FileReferenceNumber,
    pub parent_frn: FileReferenceNumber,
    pub usn: u64,
    pub reason: UsnReason,
    pub file_name: String,
    pub file_attributes: u32,
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsnReason(pub u32);

impl UsnReason {
    pub const DATA_OVERWRITE: Self = Self(0x00000001);
    pub const DATA_EXTEND: Self = Self(0x00000002);
    pub const DATA_TRUNCATION: Self = Self(0x00000004);
    pub const NAMED_DATA_OVERWRITE: Self = Self(0x00000010);
    pub const NAMED_DATA_EXTEND: Self = Self(0x00000020);
    pub const NAMED_DATA_TRUNCATION: Self = Self(0x00000040);
    pub const FILE_CREATE: Self = Self(0x00000100);
    pub const FILE_DELETE: Self = Self(0x00000200);
    pub const EA_CHANGE: Self = Self(0x00000400);
    pub const SECURITY_CHANGE: Self = Self(0x00000800);
    pub const RENAME_OLD_NAME: Self = Self(0x00001000);
    pub const RENAME_NEW_NAME: Self = Self(0x00002000);
    pub const INDEXABLE_CHANGE: Self = Self(0x00004000);
    pub const BASIC_INFO_CHANGE: Self = Self(0x00008000);
    pub const HARD_LINK_CHANGE: Self = Self(0x00010000);
    pub const COMPRESSION_CHANGE: Self = Self(0x00020000);
    pub const ENCRYPTION_CHANGE: Self = Self(0x00040000);
    pub const OBJECT_ID_CHANGE: Self = Self(0x00080000);
    pub const REPARSE_POINT_CHANGE: Self = Self(0x00100000);
    pub const STREAM_CHANGE: Self = Self(0x00200000);
    pub const CLOSE: Self = Self(0x80000000);

    pub fn is_create(&self) -> bool {
        self.0 & Self::FILE_CREATE.0 != 0
    }

    pub fn is_delete(&self) -> bool {
        self.0 & Self::FILE_DELETE.0 != 0
    }

    pub fn is_rename(&self) -> bool {
        self.0 & (Self::RENAME_OLD_NAME.0 | Self::RENAME_NEW_NAME.0) != 0
    }

    pub fn is_modify(&self) -> bool {
        self.0 & (Self::DATA_OVERWRITE.0 | Self::DATA_EXTEND.0 | Self::DATA_TRUNCATION.0) != 0
    }
}

/
pub struct UsnJournalMonitor {
    volume_path: PathBuf,
    usn_cursor: u64,
    pending_records: Vec<UsnRecord>,
}

impl UsnJournalMonitor {
    pub fn new(volume_path: PathBuf) -> Self {
        Self {
            volume_path,
            usn_cursor: 0,
            pending_records: Vec::new(),
        }
    }

    /
    pub fn set_cursor(&mut self, cursor: u64) {
        self.usn_cursor = cursor;
    }

    /
    pub fn cursor(&self) -> u64 {
        self.usn_cursor
    }

    /
    #[cfg(target_os = "windows")]
    pub fn read_journal(&mut self) -> Result<Vec<UsnRecord>> {

        Ok(Vec::new())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn read_journal(&mut self) -> Result<Vec<UsnRecord>> {
        Err(FileSystemError::Platform(
            "USN Journal is only supported on Windows".to_string(),
        ))
    }

    /
    pub fn apply_to_index(&self, index: &mut MftIndex, records: &[UsnRecord]) {
        for record in records {
            if record.reason.is_create() {
                index.insert(
                    record.frn,
                    FileNode::new(
                        record.file_name.clone(),
                        record.parent_frn,
                        record.file_attributes & 0x10 != 0,
                        0,
                        0,
                        0,
                        record.file_attributes,
                    ),
                );
            } else if record.reason.is_delete() {
                index.remove(&record.frn);
            } else if record.reason.is_rename() {
                if let Some(node) = index.files.get_mut(&record.frn) {
                    node.name = record.file_name.clone();
                    node.parent = record.parent_frn;
                    index.path_cache.remove(&record.frn);
                }
            }
        }

        if let Some(last) = records.last() {
            index.set_usn_cursor(last.usn);
        }
    }

    /
    pub fn records_to_events(&self, index: &mut MftIndex, records: &[UsnRecord]) -> Vec<FsEvent> {
        let mut events = Vec::new();

        for record in records {
            if let Some(path) = index.reconstruct_path(&record.frn) {
                if record.reason.is_create() {
                    events.push(FsEvent::Created(path));
                } else if record.reason.is_delete() {
                    events.push(FsEvent::Deleted(path));
                } else if record.reason.is_modify() {
                    events.push(FsEvent::Modified(path));
                }
            }
        }

        events
    }
}

/
pub struct WindowsPlatform;

impl PlatformFs for WindowsPlatform {
    fn create_watcher(&self) -> Box<dyn Watcher> {
        Box::new(WindowsWatcher::new())
    }

    fn supports_mft_index(&self) -> bool {
        cfg!(target_os = "windows")
    }

    fn platform_name(&self) -> &'static str {
        "Windows"
    }
}

/
pub struct WindowsWatcher {
    coalesce_window: Duration,
    coalescer: EventCoalescer,
    mft_index: Option<MftIndex>,
    usn_monitor: Option<UsnJournalMonitor>,
    watched_paths: Vec<PathBuf>,
}

impl WindowsWatcher {
    pub fn new() -> Self {
        Self {
            coalesce_window: DEFAULT_COALESCE_WINDOW,
            coalescer: EventCoalescer::with_window(DEFAULT_COALESCE_WINDOW),
            mft_index: None,
            usn_monitor: None,
            watched_paths: Vec::new(),
        }
    }

    /
    pub fn init_mft_index(&mut self, volume_path: PathBuf) -> Result<()> {
        let parser = MftParser::new(volume_path.clone());
        let index = parser.parse()?;

        let mut monitor = UsnJournalMonitor::new(volume_path);
        monitor.set_cursor(index.usn_cursor);

        self.mft_index = Some(index);
        self.usn_monitor = Some(monitor);

        Ok(())
    }

    /
    pub fn mft_index(&self) -> Option<&MftIndex> {
        self.mft_index.as_ref()
    }

    /
    pub fn mft_index_mut(&mut self) -> Option<&mut MftIndex> {
        self.mft_index.as_mut()
    }
}

impl Default for WindowsWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Watcher for WindowsWatcher {
    fn watch(&mut self, path: &Path) -> Result<()> {
        if !self.watched_paths.contains(&path.to_path_buf()) {
            self.watched_paths.push(path.to_path_buf());
        }
        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.watched_paths.retain(|p| p != path);
        Ok(())
    }

    fn poll_events(&mut self) -> Vec<FsEvent> {
        let mut events = Vec::new();

        if let (Some(monitor), Some(index)) = (&mut self.usn_monitor, &mut self.mft_index) {
            if let Ok(records) = monitor.read_journal() {
                if !records.is_empty() {
                    monitor.apply_to_index(index, &records);

                    let new_events = monitor.records_to_events(index, &records);
                    self.coalescer.add_events(new_events);
                }
            }
        }

        events.extend(self.coalescer.poll_ready());
        events
    }

    fn set_coalesce_window(&mut self, window: Duration) {
        self.coalesce_window = window;
        self.coalescer.set_coalesce_window(window);
    }

    fn coalesce_window(&self) -> Duration {
        self.coalesce_window
    }
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
