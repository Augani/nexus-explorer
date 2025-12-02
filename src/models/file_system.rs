use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use flume::Receiver;
use lru::LruCache;
use std::num::NonZeroUsize;

use super::{CachedDirectory, FileEntry, LoadState};
use crate::io::{
    create_batch_pipeline, traverse_directory_sorted, BatchConfig, SortKey, SortOrder,
    TraversalConfig,
};

/// Default cache capacity for directory states
const DEFAULT_CACHE_CAPACITY: usize = 100;

/// Central file system state and I/O coordination.
/// 
/// The FileSystem model manages directory navigation, caching, and async I/O coordination.
/// It uses generational ID tracking to prevent stale results from being displayed when
/// rapid navigation occurs.
pub struct FileSystem {
    current_path: PathBuf,
    entries: Vec<FileEntry>,
    state: LoadState,
    request_id: usize,
    cache: LruCache<PathBuf, CachedDirectory>,
}

impl FileSystem {
    /// Creates a new FileSystem model with the given initial path.
    pub fn new(initial_path: PathBuf) -> Self {
        let cache_capacity = NonZeroUsize::new(DEFAULT_CACHE_CAPACITY)
            .expect("DEFAULT_CACHE_CAPACITY must be non-zero");
        
        Self {
            current_path: initial_path,
            entries: Vec::new(),
            state: LoadState::Idle,
            request_id: 0,
            cache: LruCache::new(cache_capacity),
        }
    }

    /// Creates a new FileSystem model with a custom cache capacity.
    pub fn with_cache_capacity(initial_path: PathBuf, capacity: usize) -> Self {
        let cache_capacity = NonZeroUsize::new(capacity.max(1))
            .expect("capacity must be at least 1");
        
        Self {
            current_path: initial_path,
            entries: Vec::new(),
            state: LoadState::Idle,
            request_id: 0,
            cache: LruCache::new(cache_capacity),
        }
    }

    /// Returns a reference to the current file entries.
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Returns the current directory path.
    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    /// Returns the current loading state.
    pub fn state(&self) -> &LoadState {
        &self.state
    }

    /// Returns the current request ID for generational tracking.
    pub fn request_id(&self) -> usize {
        self.request_id
    }


    /// Initiates a navigation request to the specified path.
    /// 
    /// This increments the request_id for generational tracking and checks the cache.
    /// If cached data exists, it's immediately made available while revalidation
    /// can occur in the background.
    /// 
    /// Returns the new request_id for tracking async operations.
    pub fn begin_load(&mut self, path: PathBuf) -> usize {
        self.request_id = self.request_id.wrapping_add(1);
        self.current_path = path.clone();
        
        // Check cache for immediate display
        if let Some(cached) = self.cache.get(&path) {
            self.entries = cached.entries.clone();
            self.state = LoadState::Cached { stale: false };
        } else {
            self.entries.clear();
            self.state = LoadState::Loading { request_id: self.request_id };
        }
        
        self.request_id
    }

    /// Validates if the given request_id matches the current generation.
    /// 
    /// Used to prevent stale async results from being applied when a newer
    /// navigation request has superseded the original.
    pub fn is_valid_request(&self, request_id: usize) -> bool {
        self.request_id == request_id
    }

    /// Applies loaded entries if the request_id is still valid.
    /// 
    /// Returns true if the update was applied, false if it was discarded
    /// due to a stale request_id.
    pub fn complete_load(
        &mut self,
        request_id: usize,
        entries: Vec<FileEntry>,
        duration: std::time::Duration,
        mtime: std::time::SystemTime,
    ) -> bool {
        if !self.is_valid_request(request_id) {
            return false;
        }

        let count = entries.len();
        
        // Cache the directory state with current generation
        let cached = CachedDirectory::new(entries.clone(), request_id, mtime);
        self.cache.put(self.current_path.clone(), cached);
        
        self.entries = entries;
        self.state = LoadState::Loaded { count, duration };
        
        true
    }

    /// Sets an error state if the request_id is still valid.
    /// 
    /// Returns true if the error was applied, false if discarded.
    pub fn set_error(&mut self, request_id: usize, message: String) -> bool {
        if !self.is_valid_request(request_id) {
            return false;
        }
        
        self.state = LoadState::Error { message };
        true
    }

    /// Gets cached directory data if available.
    pub fn get_cached(&mut self, path: &Path) -> Option<&CachedDirectory> {
        self.cache.get(path)
    }

    /// Checks if a path exists in the cache.
    pub fn is_cached(&self, path: &Path) -> bool {
        self.cache.contains(path)
    }

    /// Returns the number of cached directories.
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// Clears all cached directory states.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Appends entries to the current list if the request_id is still valid.
    /// Used for incremental batch updates during traversal.
    /// 
    /// Returns true if the update was applied, false if discarded.
    pub fn append_entries(&mut self, request_id: usize, new_entries: Vec<FileEntry>) -> bool {
        if !self.is_valid_request(request_id) {
            return false;
        }
        
        self.entries.extend(new_entries);
        true
    }
}

/// Result of starting a directory load operation
pub struct LoadOperation {
    pub request_id: usize,
    pub batch_receiver: Receiver<Vec<FileEntry>>,
    pub traversal_handle: std::thread::JoinHandle<crate::models::Result<usize>>,
}

impl FileSystem {
    /// Initiates an asynchronous load of the specified directory path.
    /// 
    /// This method:
    /// 1. Increments the request_id for generational tracking
    /// 2. Checks cache for immediate display
    /// 3. Spawns a background traversal with batch aggregation
    /// 
    /// Returns a LoadOperation containing the request_id and receivers for
    /// processing batched results.
    pub fn load_path(
        &mut self,
        path: PathBuf,
        sort_key: SortKey,
        sort_order: SortOrder,
        include_hidden: bool,
    ) -> LoadOperation {
        let request_id = self.begin_load(path.clone());

        let config = TraversalConfig {
            sort_key,
            sort_order,
            include_hidden,
            max_depth: Some(1),
        };

        let batch_config = BatchConfig::default();
        let (entry_tx, batch_rx, _batch_handle) = create_batch_pipeline(batch_config);

        let traversal_handle = std::thread::spawn(move || {
            traverse_directory_sorted(&path, &config, entry_tx)
        });

        LoadOperation {
            request_id,
            batch_receiver: batch_rx,
            traversal_handle,
        }
    }

    /// Processes batched entries from a load operation.
    /// 
    /// This method should be called repeatedly to process incoming batches
    /// until the receiver is disconnected.
    /// 
    /// Returns the number of entries added, or None if the request was stale.
    pub fn process_batch(&mut self, request_id: usize, batch: Vec<FileEntry>) -> Option<usize> {
        if !self.is_valid_request(request_id) {
            return None;
        }

        let count = batch.len();
        self.entries.extend(batch);
        Some(count)
    }

    /// Completes a load operation, finalizing the state.
    /// 
    /// This should be called after all batches have been processed.
    pub fn finalize_load(&mut self, request_id: usize, duration: Duration) -> bool {
        if !self.is_valid_request(request_id) {
            return false;
        }

        let count = self.entries.len();
        
        // Get directory mtime for cache staleness detection
        let mtime = std::fs::metadata(&self.current_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // Cache the directory state
        let cached = CachedDirectory::new(self.entries.clone(), request_id, mtime);
        self.cache.put(self.current_path.clone(), cached);

        self.state = LoadState::Loaded { count, duration };
        true
    }
}

/// Synchronously loads a directory and processes all batches.
/// 
/// This is a convenience function for simple use cases where async
/// processing is not needed.
pub fn load_directory_sync(
    fs: &mut FileSystem,
    path: PathBuf,
    sort_key: SortKey,
    sort_order: SortOrder,
    include_hidden: bool,
) -> crate::models::Result<usize> {
    let start = Instant::now();
    let op = fs.load_path(path, sort_key, sort_order, include_hidden);
    let request_id = op.request_id;

    // Process all batches
    while let Ok(batch) = op.batch_receiver.recv() {
        fs.process_batch(request_id, batch);
    }

    // Wait for traversal to complete
    let result = op.traversal_handle.join()
        .map_err(|_| crate::models::FileSystemError::Platform("Traversal thread panicked".to_string()))?;

    let duration = start.elapsed();
    fs.finalize_load(request_id, duration);

    result
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new(PathBuf::from("/"))
    }
}

use super::FsEvent;

/// File event processing for the FileSystem model.
/// 
/// These methods handle real-time file system events from watchers,
/// updating the entries list to reflect changes without requiring
/// a full directory reload.
impl FileSystem {
    /// Processes a file system event and updates entries accordingly.
    /// 
    /// Returns true if the entries were modified, false otherwise.
    /// Events for paths outside the current directory are ignored.
    pub fn process_event(&mut self, event: FsEvent) -> bool {
        match event {
            FsEvent::Created(path) => self.handle_created(path),
            FsEvent::Modified(path) => self.handle_modified(path),
            FsEvent::Deleted(path) => self.handle_deleted(path),
            FsEvent::Renamed { from, to } => self.handle_renamed(from, to),
        }
    }

    /// Processes multiple file system events.
    /// 
    /// Returns the number of events that resulted in entry modifications.
    pub fn process_events(&mut self, events: Vec<FsEvent>) -> usize {
        events.into_iter()
            .filter(|event| self.process_event(event.clone()))
            .count()
    }

    fn is_in_current_directory(&self, path: &Path) -> bool {
        path.parent() == Some(&self.current_path)
    }

    fn handle_created(&mut self, path: PathBuf) -> bool {
        if !self.is_in_current_directory(&path) {
            return false;
        }

        if self.entries.iter().any(|e| e.path == path) {
            return false;
        }

        if let Some(entry) = Self::create_entry_from_path(&path) {
            let insert_pos = self.entries
                .binary_search_by(|e| e.name.cmp(&entry.name))
                .unwrap_or_else(|pos| pos);
            self.entries.insert(insert_pos, entry);
            self.invalidate_cache_for_current();
            true
        } else {
            false
        }
    }

    fn handle_modified(&mut self, path: PathBuf) -> bool {
        if !self.is_in_current_directory(&path) {
            return false;
        }

        if let Some(entry) = self.entries.iter_mut().find(|e| e.path == path) {
            if let Ok(metadata) = std::fs::metadata(&path) {
                entry.size = metadata.len();
                entry.modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                self.invalidate_cache_for_current();
                return true;
            }
        }
        false
    }

    fn handle_deleted(&mut self, path: PathBuf) -> bool {
        if !self.is_in_current_directory(&path) {
            return false;
        }

        let original_len = self.entries.len();
        self.entries.retain(|e| e.path != path);
        
        if self.entries.len() != original_len {
            self.invalidate_cache_for_current();
            true
        } else {
            false
        }
    }

    fn handle_renamed(&mut self, from: PathBuf, to: PathBuf) -> bool {
        let from_in_dir = self.is_in_current_directory(&from);
        let to_in_dir = self.is_in_current_directory(&to);

        match (from_in_dir, to_in_dir) {
            (true, true) => {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.path == from) {
                    entry.path = to.clone();
                    entry.name = to.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    self.entries.sort_by(|a, b| a.name.cmp(&b.name));
                    self.invalidate_cache_for_current();
                    true
                } else {
                    false
                }
            }
            (true, false) => self.handle_deleted(from),
            (false, true) => self.handle_created(to),
            (false, false) => false,
        }
    }

    fn create_entry_from_path(path: &Path) -> Option<FileEntry> {
        let metadata = std::fs::metadata(path).ok()?;
        let name = path.file_name()?.to_str()?.to_string();
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        Some(FileEntry::new(name, path.to_path_buf(), is_dir, size, modified))
    }

    fn invalidate_cache_for_current(&mut self) {
        self.cache.pop(&self.current_path);
    }

    /// Checks if a path exists in the current entries.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.entries.iter().any(|e| e.path == path)
    }
}

#[cfg(test)]
#[path = "file_system_tests.rs"]
mod tests;
