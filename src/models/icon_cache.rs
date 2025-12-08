use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::path::Path;

use flume::{Receiver, Sender};
use lru::LruCache;

use super::IconKey;
use crate::utils::rgba_to_bgra_inplace;


const DEFAULT_MAX_ENTRIES: usize = 500;



#[derive(Debug, Clone, PartialEq)]
pub struct RenderImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RenderImage {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }


    pub fn default_placeholder() -> Self {
        let pixel = [128u8, 128, 128, 255];
        let data: Vec<u8> = pixel.iter().cycle().take(16 * 16 * 4).copied().collect();
        Self {
            width: 16,
            height: 16,
            data,
        }
    }


    pub fn default_folder() -> Self {
        let pixel = [200u8, 180, 100, 255];
        let data: Vec<u8> = pixel.iter().cycle().take(16 * 16 * 4).copied().collect();
        Self {
            width: 16,
            height: 16,
            data,
        }
    }
}






pub struct IconCache {
    textures: HashMap<IconKey, RenderImage>,
    lru: LruCache<IconKey, ()>,
    pending: HashSet<IconKey>,
    max_entries: usize,
    default_icon: RenderImage,
    folder_icon: RenderImage,
}

impl IconCache {

    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_ENTRIES)
    }


    pub fn with_capacity(max_entries: usize) -> Self {
        let capacity =
            NonZeroUsize::new(max_entries.max(1)).expect("max_entries must be at least 1");

        Self {
            textures: HashMap::new(),
            lru: LruCache::new(capacity),
            pending: HashSet::new(),
            max_entries: max_entries.max(1),
            default_icon: RenderImage::default_placeholder(),
            folder_icon: RenderImage::default_folder(),
        }
    }


    pub fn max_entries(&self) -> usize {
        self.max_entries
    }


    pub fn len(&self) -> usize {
        self.textures.len()
    }


    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }



    pub fn get_icon(&mut self, key: &IconKey) -> Option<&RenderImage> {
        if self.textures.contains_key(key) {
            self.lru.get(key);
            self.textures.get(key)
        } else {
            None
        }
    }



    pub fn get_or_default(&mut self, key: &IconKey) -> &RenderImage {
        if self.textures.contains_key(key) {
            self.lru.get(key);
            self.textures.get(key).unwrap()
        } else {
            if !self.pending.contains(key) {
                self.pending.insert(key.clone());
            }
            match key {
                IconKey::Directory => &self.folder_icon,
                _ => &self.default_icon,
            }
        }
    }


    pub fn contains(&self, key: &IconKey) -> bool {
        self.textures.contains_key(key)
    }


    pub fn is_pending(&self, key: &IconKey) -> bool {
        self.pending.contains(key)
    }


    pub fn pending_keys(&self) -> &HashSet<IconKey> {
        &self.pending
    }


    pub fn insert(&mut self, key: IconKey, image: RenderImage) {
        self.pending.remove(&key);

        while self.textures.len() >= self.max_entries {
            if let Some((evicted_key, _)) = self.lru.pop_lru() {
                self.textures.remove(&evicted_key);
            } else {
                break;
            }
        }

        self.textures.insert(key.clone(), image);
        self.lru.put(key, ());
    }


    pub fn remove(&mut self, key: &IconKey) -> Option<RenderImage> {
        self.lru.pop(key);
        self.pending.remove(key);
        self.textures.remove(key)
    }


    pub fn clear(&mut self) {
        self.textures.clear();
        self.lru.clear();
        self.pending.clear();
    }


    pub fn remove_pending(&mut self, key: &IconKey) {
        self.pending.remove(key);
    }


    pub fn default_icon(&self) -> &RenderImage {
        &self.default_icon
    }


    pub fn folder_icon(&self) -> &RenderImage {
        &self.folder_icon
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}


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


#[derive(Debug, Clone)]
pub struct IconFetchRequest {
    pub key: IconKey,
    pub path: Option<std::path::PathBuf>,
}








pub struct IconFetchPipeline {
    request_tx: Sender<IconFetchRequest>,
    result_rx: Receiver<IconFetchResult>,
    _worker_handle: Option<std::thread::JoinHandle<()>>,
}

impl IconFetchPipeline {

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


    pub fn request_icon(&self, key: IconKey, path: Option<std::path::PathBuf>) {
        let request = IconFetchRequest { key, path };
        let _ = self.request_tx.send(request);
    }



    pub fn poll_results(&self) -> Vec<IconFetchResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }


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


    pub fn process_fetch_results(&mut self, results: Vec<IconFetchResult>) -> usize {
        let mut count = 0;
        for result in results {
            if let Some(image) = result.image {
                self.insert(result.key, image);
                count += 1;
            } else {
                self.remove_pending(&result.key);
            }
        }
        count
    }


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
