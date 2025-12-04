use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::path::Path;

use flume::{Receiver, Sender};
use lru::LruCache;

use super::IconKey;
use crate::utils::rgba_to_bgra_inplace;

/// Default maximum number of icons in the cache
const DEFAULT_MAX_ENTRIES: usize = 500;

/// Placeholder for GPU-rendered image data.
/// In a real GPUI application, this would be `gpui::RenderImage` or similar.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RenderImage {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self { width, height, data }
    }

    /// Creates a default placeholder icon (simple colored square)
    pub fn default_placeholder() -> Self {
        // Gray placeholder in BGRA format (16x16 pixels, 4 bytes per pixel)
        let pixel = [128u8, 128, 128, 255];
        let data: Vec<u8> = pixel.iter().cycle().take(16 * 16 * 4).copied().collect();
        Self {
            width: 16,
            height: 16,
            data,
        }
    }

    /// Creates a default folder icon
    pub fn default_folder() -> Self {
        // Folder-like color in BGRA format (16x16 pixels, 4 bytes per pixel)
        let pixel = [200u8, 180, 100, 255];
        let data: Vec<u8> = pixel.iter().cycle().take(16 * 16 * 4).copied().collect();
        Self {
            width: 16,
            height: 16,
            data,
        }
    }
}

/// GPU texture management with LRU eviction.
///
/// The IconCache manages icon textures for file entries, using LRU eviction
/// to bound memory usage. It provides immediate access to cached icons and
/// queues fetch requests for uncached icons.
pub struct IconCache {
    textures: HashMap<IconKey, RenderImage>,
    lru: LruCache<IconKey, ()>,
    pending: HashSet<IconKey>,
    max_entries: usize,
    default_icon: RenderImage,
    folder_icon: RenderImage,
}


impl IconCache {
    /// Creates a new IconCache with the default maximum entries.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_ENTRIES)
    }

    /// Creates a new IconCache with a custom maximum entries limit.
    pub fn with_capacity(max_entries: usize) -> Self {
        let capacity = NonZeroUsize::new(max_entries.max(1))
            .expect("max_entries must be at least 1");

        Self {
            textures: HashMap::new(),
            lru: LruCache::new(capacity),
            pending: HashSet::new(),
            max_entries: max_entries.max(1),
            default_icon: RenderImage::default_placeholder(),
            folder_icon: RenderImage::default_folder(),
        }
    }

    /// Returns the maximum number of entries allowed in the cache.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Returns the current number of cached icons.
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Gets an icon from the cache if it exists.
    /// Returns None if the icon is not cached.
    pub fn get_icon(&mut self, key: &IconKey) -> Option<&RenderImage> {
        if self.textures.contains_key(key) {
            self.lru.get(key);
            self.textures.get(key)
        } else {
            None
        }
    }

    /// Gets an icon from the cache, or returns the default placeholder.
    /// If the icon is not cached and not pending, it's added to the pending set.
    pub fn get_or_default(&mut self, key: &IconKey) -> &RenderImage {
        if self.textures.contains_key(key) {
            self.lru.get(key);
            self.textures.get(key).unwrap()
        } else {
            // Add to pending if not already there
            if !self.pending.contains(key) {
                self.pending.insert(key.clone());
            }
            match key {
                IconKey::Directory => &self.folder_icon,
                _ => &self.default_icon,
            }
        }
    }

    /// Checks if an icon is currently in the cache.
    pub fn contains(&self, key: &IconKey) -> bool {
        self.textures.contains_key(key)
    }

    /// Checks if an icon fetch is pending.
    pub fn is_pending(&self, key: &IconKey) -> bool {
        self.pending.contains(key)
    }

    /// Returns the set of pending icon keys.
    pub fn pending_keys(&self) -> &HashSet<IconKey> {
        &self.pending
    }

    /// Inserts an icon into the cache, evicting LRU entries if necessary.
    pub fn insert(&mut self, key: IconKey, image: RenderImage) {
        // Remove from pending
        self.pending.remove(&key);

        // Evict if at capacity
        while self.textures.len() >= self.max_entries {
            if let Some((evicted_key, _)) = self.lru.pop_lru() {
                self.textures.remove(&evicted_key);
            } else {
                break;
            }
        }

        // Insert new entry
        self.textures.insert(key.clone(), image);
        self.lru.put(key, ());
    }

    /// Removes an icon from the cache.
    pub fn remove(&mut self, key: &IconKey) -> Option<RenderImage> {
        self.lru.pop(key);
        self.pending.remove(key);
        self.textures.remove(key)
    }

    /// Clears all cached icons and pending requests.
    pub fn clear(&mut self) {
        self.textures.clear();
        self.lru.clear();
        self.pending.clear();
    }

    /// Removes a key from the pending set (e.g., after fetch completion or failure).
    pub fn remove_pending(&mut self, key: &IconKey) {
        self.pending.remove(key);
    }

    /// Returns a reference to the default placeholder icon.
    pub fn default_icon(&self) -> &RenderImage {
        &self.default_icon
    }

    /// Returns a reference to the folder icon.
    pub fn folder_icon(&self) -> &RenderImage {
        &self.folder_icon
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a completed icon fetch operation
#[derive(Debug, Clone)]
pub struct IconFetchResult {
    pub key: IconKey,
    pub image: Option<RenderImage>,
    pub error: Option<String>,
}

impl IconFetchResult {
    pub fn success(key: IconKey, image: RenderImage) -> Self {
        Self {
            key,
            image: Some(image),
            error: None,
        }
    }

    pub fn failure(key: IconKey, error: String) -> Self {
        Self {
            key,
            image: None,
            error: Some(error),
        }
    }

    pub fn is_success(&self) -> bool {
        self.image.is_some()
    }
}

/// Request to fetch an icon
#[derive(Debug, Clone)]
pub struct IconFetchRequest {
    pub key: IconKey,
    pub path: Option<std::path::PathBuf>,
}

/// Async icon fetch pipeline for loading icons on background threads.
/// 
/// This pipeline:
/// 1. Receives fetch requests for uncached icons
/// 2. Decodes images on background threads using the `image` crate
/// 3. Converts RGBA to BGRA format for GPU upload
/// 4. Sends completed results back to the main thread
pub struct IconFetchPipeline {
    request_tx: Sender<IconFetchRequest>,
    result_rx: Receiver<IconFetchResult>,
    _worker_handle: Option<std::thread::JoinHandle<()>>,
}

impl IconFetchPipeline {
    /// Creates a new icon fetch pipeline with a background worker thread.
    pub fn new() -> Self {
        let (request_tx, request_rx) = flume::unbounded::<IconFetchRequest>();
        let (result_tx, result_rx) = flume::unbounded::<IconFetchResult>();

        let worker_handle = std::thread::spawn(move || {
            Self::worker_loop(request_rx, result_tx);
        });

        Self {
            request_tx,
            result_rx,
            _worker_handle: Some(worker_handle),
        }
    }

    /// Queues a fetch request for an icon.
    pub fn request_icon(&self, key: IconKey, path: Option<std::path::PathBuf>) {
        let request = IconFetchRequest { key, path };
        let _ = self.request_tx.send(request);
    }

    /// Polls for completed fetch results without blocking.
    /// Returns all available results.
    pub fn poll_results(&self) -> Vec<IconFetchResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    /// Returns the receiver for fetch results (for async integration).
    pub fn result_receiver(&self) -> &Receiver<IconFetchResult> {
        &self.result_rx
    }

    fn worker_loop(request_rx: Receiver<IconFetchRequest>, result_tx: Sender<IconFetchResult>) {
        while let Ok(request) = request_rx.recv() {
            let result = Self::process_request(&request);
            if result_tx.send(result).is_err() {
                break;
            }
        }
    }

    fn process_request(request: &IconFetchRequest) -> IconFetchResult {
        match &request.path {
            Some(path) => Self::load_icon_from_path(&request.key, path),
            None => Self::load_default_for_key(&request.key),
        }
    }

    fn load_icon_from_path(key: &IconKey, path: &Path) -> IconFetchResult {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let mut data = rgba.into_raw();
                
                // Convert RGBA to BGRA for GPU
                rgba_to_bgra_inplace(&mut data);
                
                let image = RenderImage::new(width, height, data);
                IconFetchResult::success(key.clone(), image)
            }
            Err(e) => IconFetchResult::failure(key.clone(), e.to_string()),
        }
    }

    fn load_default_for_key(key: &IconKey) -> IconFetchResult {
        let image = match key {
            IconKey::Directory => RenderImage::default_folder(),
            _ => RenderImage::default_placeholder(),
        };
        IconFetchResult::success(key.clone(), image)
    }
}

impl Default for IconFetchPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl IconCache {
    /// Processes completed fetch results from the pipeline and updates the cache.
    /// Returns the number of icons successfully added to the cache.
    pub fn process_fetch_results(&mut self, results: Vec<IconFetchResult>) -> usize {
        let mut count = 0;
        for result in results {
            if let Some(image) = result.image {
                self.insert(result.key, image);
                count += 1;
            } else {
                // Remove from pending on failure
                self.remove_pending(&result.key);
            }
        }
        count
    }

    /// Queues fetch requests for all pending icons using the provided pipeline.
    pub fn queue_pending_fetches(&self, pipeline: &IconFetchPipeline) {
        for key in &self.pending {
            let path = match key {
                IconKey::Custom(p) => Some(p.clone()),
                _ => None,
            };
            pipeline.request_icon(key.clone(), path);
        }
    }
}

#[cfg(test)]
#[path = "icon_cache_tests.rs"]
mod tests;
