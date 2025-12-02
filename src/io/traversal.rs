use std::path::{Path, PathBuf};
use std::time::SystemTime;

use flume::Sender;
use jwalk::{WalkDir, WalkDirGeneric};
use serde::{Deserialize, Serialize};

use crate::models::{FileEntry, FileSystemError, Result};

/// Sort order for directory traversal results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

/// Sort key for directory traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Date,
}

/// Configuration for directory traversal
#[derive(Debug, Clone)]
pub struct TraversalConfig {
    pub sort_key: SortKey,
    pub sort_order: SortOrder,
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            sort_key: SortKey::Name,
            sort_order: SortOrder::Ascending,
            include_hidden: false,
            max_depth: Some(1),
        }
    }
}

/// Traverses a directory using jwalk and streams results through a flume channel.
/// 
/// This function is designed to be called via `spawn_blocking` on a Tokio thread pool
/// to avoid blocking the UI thread.
pub fn traverse_directory(
    path: &Path,
    config: &TraversalConfig,
    sender: Sender<FileEntry>,
) -> Result<usize> {
    if !path.exists() {
        return Err(FileSystemError::PathNotFound(path.to_path_buf()));
    }

    if !path.is_dir() {
        return Err(FileSystemError::Io(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "Path is not a directory",
        )));
    }

    let walk_dir = build_walk_dir(path, config);
    let mut count = 0;

    for entry_result in walk_dir {
        match entry_result {
            Ok(entry) => {
                // Skip the root directory itself
                if entry.depth() == 0 {
                    continue;
                }

                if let Some(file_entry) = dir_entry_to_file_entry(&entry) {
                    // Filter hidden files if configured
                    if !config.include_hidden && is_hidden(&file_entry.name) {
                        continue;
                    }

                    if sender.send(file_entry).is_err() {
                        // Receiver dropped, stop traversal
                        break;
                    }
                    count += 1;
                }
            }
            Err(e) => {
                // Log error but continue traversal
                eprintln!("Traversal error: {}", e);
            }
        }
    }

    Ok(count)
}

/// Builds a jwalk WalkDir with the specified configuration.
fn build_walk_dir(path: &Path, config: &TraversalConfig) -> WalkDirGeneric<((), ())> {
    let mut walk_dir = WalkDir::new(path)
        .parallelism(jwalk::Parallelism::RayonNewPool(num_cpus()))
        .skip_hidden(!config.include_hidden);

    if let Some(depth) = config.max_depth {
        walk_dir = walk_dir.max_depth(depth);
    }

    // Configure sorting based on sort key
    walk_dir = walk_dir.sort(true);

    walk_dir
}

/// Returns the number of CPUs available for parallel traversal.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Converts a jwalk DirEntry to our FileEntry type.
fn dir_entry_to_file_entry(entry: &jwalk::DirEntry<((), ())>) -> Option<FileEntry> {
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().to_string();
    
    let metadata = entry.metadata().ok()?;
    let is_dir = metadata.is_dir();
    let size = if is_dir { 0 } else { metadata.len() };
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

    Some(FileEntry::new(name, path, is_dir, size, modified))
}

/// Checks if a file name indicates a hidden file.
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Sorts a vector of FileEntry according to the specified configuration.
pub fn sort_entries(entries: &mut [FileEntry], sort_key: SortKey, sort_order: SortOrder) {
    match (sort_key, sort_order) {
        (SortKey::Name, SortOrder::Ascending) => {
            entries.sort_by(|a, b| {
                // Directories first, then by name (case-insensitive)
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });
        }
        (SortKey::Name, SortOrder::Descending) => {
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.name.to_lowercase().cmp(&a.name.to_lowercase()),
                }
            });
        }
        (SortKey::Size, SortOrder::Ascending) => {
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.size.cmp(&b.size),
                }
            });
        }
        (SortKey::Size, SortOrder::Descending) => {
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.size.cmp(&a.size),
                }
            });
        }
        (SortKey::Date, SortOrder::Ascending) => {
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.modified.cmp(&b.modified),
                }
            });
        }
        (SortKey::Date, SortOrder::Descending) => {
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.modified.cmp(&a.modified),
                }
            });
        }
    }
}

/// Traverses a directory and returns sorted results.
/// 
/// This function collects all entries, sorts them according to the config,
/// and then streams them through the channel. This ensures results are
/// delivered in sorted order.
pub fn traverse_directory_sorted(
    path: &Path,
    config: &TraversalConfig,
    sender: Sender<FileEntry>,
) -> Result<usize> {
    if !path.exists() {
        return Err(FileSystemError::PathNotFound(path.to_path_buf()));
    }

    if !path.is_dir() {
        return Err(FileSystemError::Io(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "Path is not a directory",
        )));
    }

    let walk_dir = build_walk_dir(path, config);
    let mut entries = Vec::new();

    for entry_result in walk_dir {
        match entry_result {
            Ok(entry) => {
                if entry.depth() == 0 {
                    continue;
                }

                if let Some(file_entry) = dir_entry_to_file_entry(&entry) {
                    if !config.include_hidden && is_hidden(&file_entry.name) {
                        continue;
                    }
                    entries.push(file_entry);
                }
            }
            Err(e) => {
                eprintln!("Traversal error: {}", e);
            }
        }
    }

    // Sort entries according to configuration
    sort_entries(&mut entries, config.sort_key, config.sort_order);

    let count = entries.len();
    for entry in entries {
        if sender.send(entry).is_err() {
            break;
        }
    }

    Ok(count)
}

/// Spawns a directory traversal task on a blocking thread pool.
/// Returns a receiver for streaming FileEntry results.
pub fn spawn_traversal(
    path: PathBuf,
    config: TraversalConfig,
) -> (flume::Receiver<FileEntry>, std::thread::JoinHandle<Result<usize>>) {
    let (sender, receiver) = flume::unbounded();
    
    let handle = std::thread::spawn(move || {
        traverse_directory(&path, &config, sender)
    });

    (receiver, handle)
}

/// Spawns a sorted directory traversal task on a blocking thread pool.
/// Results are collected, sorted, and then streamed.
pub fn spawn_sorted_traversal(
    path: PathBuf,
    config: TraversalConfig,
) -> (flume::Receiver<FileEntry>, std::thread::JoinHandle<Result<usize>>) {
    let (sender, receiver) = flume::unbounded();
    
    let handle = std::thread::spawn(move || {
        traverse_directory_sorted(&path, &config, sender)
    });

    (receiver, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    fn create_test_directory() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        
        // Create some test files
        File::create(temp_dir.path().join("file_a.txt")).unwrap();
        File::create(temp_dir.path().join("file_b.txt")).unwrap();
        File::create(temp_dir.path().join("file_c.txt")).unwrap();
        
        // Create a subdirectory
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        
        // Create a hidden file
        File::create(temp_dir.path().join(".hidden")).unwrap();
        
        temp_dir
    }

    #[test]
    fn test_traverse_directory_basic() {
        let temp_dir = create_test_directory();
        let (sender, receiver) = flume::unbounded();
        let config = TraversalConfig::default();
        
        let result = traverse_directory(temp_dir.path(), &config, sender);
        assert!(result.is_ok());
        
        let entries: Vec<_> = receiver.iter().collect();
        // Should have 4 entries: 3 files + 1 subdir (hidden file excluded by default)
        assert_eq!(entries.len(), 4);
    }

    #[test]
    fn test_traverse_directory_with_hidden() {
        let temp_dir = create_test_directory();
        let (sender, receiver) = flume::unbounded();
        let config = TraversalConfig {
            include_hidden: true,
            ..Default::default()
        };
        
        let result = traverse_directory(temp_dir.path(), &config, sender);
        assert!(result.is_ok());
        
        let entries: Vec<_> = receiver.iter().collect();
        // Should have 5 entries: 3 files + 1 subdir + 1 hidden file
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn test_traverse_nonexistent_path() {
        let (sender, _receiver) = flume::unbounded();
        let config = TraversalConfig::default();
        
        let result = traverse_directory(Path::new("/nonexistent/path"), &config, sender);
        assert!(matches!(result, Err(FileSystemError::PathNotFound(_))));
    }

    #[test]
    fn test_sort_entries_by_name() {
        let mut entries = vec![
            FileEntry::new("zebra.txt".to_string(), PathBuf::from("/zebra.txt"), false, 100, SystemTime::UNIX_EPOCH),
            FileEntry::new("alpha.txt".to_string(), PathBuf::from("/alpha.txt"), false, 200, SystemTime::UNIX_EPOCH),
            FileEntry::new("beta".to_string(), PathBuf::from("/beta"), true, 0, SystemTime::UNIX_EPOCH),
        ];
        
        sort_entries(&mut entries, SortKey::Name, SortOrder::Ascending);
        
        // Directory should be first, then alphabetical
        assert_eq!(entries[0].name, "beta");
        assert_eq!(entries[1].name, "alpha.txt");
        assert_eq!(entries[2].name, "zebra.txt");
    }

    #[test]
    fn test_sort_entries_by_size() {
        let mut entries = vec![
            FileEntry::new("small.txt".to_string(), PathBuf::from("/small.txt"), false, 100, SystemTime::UNIX_EPOCH),
            FileEntry::new("large.txt".to_string(), PathBuf::from("/large.txt"), false, 1000, SystemTime::UNIX_EPOCH),
            FileEntry::new("dir".to_string(), PathBuf::from("/dir"), true, 0, SystemTime::UNIX_EPOCH),
        ];
        
        sort_entries(&mut entries, SortKey::Size, SortOrder::Ascending);
        
        // Directory first, then by size ascending
        assert_eq!(entries[0].name, "dir");
        assert_eq!(entries[1].name, "small.txt");
        assert_eq!(entries[2].name, "large.txt");
    }

    #[test]
    fn test_traverse_directory_sorted() {
        let temp_dir = create_test_directory();
        let (sender, receiver) = flume::unbounded();
        let config = TraversalConfig {
            sort_key: SortKey::Name,
            sort_order: SortOrder::Ascending,
            ..Default::default()
        };
        
        let result = traverse_directory_sorted(temp_dir.path(), &config, sender);
        assert!(result.is_ok());
        
        let entries: Vec<_> = receiver.iter().collect();
        // Should have 4 entries: 1 subdir + 3 files (hidden excluded)
        assert_eq!(entries.len(), 4);
        
        // Directory should be first
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "subdir");
        
        // Files should be sorted alphabetically
        assert_eq!(entries[1].name, "file_a.txt");
        assert_eq!(entries[2].name, "file_b.txt");
        assert_eq!(entries[3].name, "file_c.txt");
    }

    #[test]
    fn test_traverse_directory_sorted_descending() {
        let temp_dir = create_test_directory();
        let (sender, receiver) = flume::unbounded();
        let config = TraversalConfig {
            sort_key: SortKey::Name,
            sort_order: SortOrder::Descending,
            ..Default::default()
        };
        
        let result = traverse_directory_sorted(temp_dir.path(), &config, sender);
        assert!(result.is_ok());
        
        let entries: Vec<_> = receiver.iter().collect();
        
        // Directory should still be first (directories always first)
        assert!(entries[0].is_dir);
        
        // Files should be sorted in reverse alphabetical order
        assert_eq!(entries[1].name, "file_c.txt");
        assert_eq!(entries[2].name, "file_b.txt");
        assert_eq!(entries[3].name, "file_a.txt");
    }

    /// Helper to check if entries are sorted correctly
    fn is_sorted(entries: &[FileEntry], sort_key: SortKey, sort_order: SortOrder) -> bool {
        if entries.len() <= 1 {
            return true;
        }

        // First check: directories should come before files
        let mut seen_file = false;
        for entry in entries {
            if entry.is_dir && seen_file {
                return false; // Directory after file
            }
            if !entry.is_dir {
                seen_file = true;
            }
        }

        // Check sorting within directories and files separately
        let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir).collect();
        let files: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();

        is_group_sorted(&dirs, sort_key, sort_order) && is_group_sorted(&files, sort_key, sort_order)
    }

    fn is_group_sorted(entries: &[&FileEntry], sort_key: SortKey, sort_order: SortOrder) -> bool {
        for window in entries.windows(2) {
            let a = window[0];
            let b = window[1];
            
            let cmp = match sort_key {
                SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortKey::Size => a.size.cmp(&b.size),
                SortKey::Date => a.modified.cmp(&b.modified),
            };

            let valid = match sort_order {
                SortOrder::Ascending => cmp != std::cmp::Ordering::Greater,
                SortOrder::Descending => cmp != std::cmp::Ordering::Less,
            };

            if !valid {
                return false;
            }
        }
        true
    }

    proptest! {
        /// **Feature: file-explorer-core, Property 8: Traversal Results Sorted**
        /// **Validates: Requirements 3.2**
        /// 
        /// For any directory traversal result, the delivered entries SHALL be sorted
        /// by the configured sort key (name, size, or date) in the configured order
        /// (ascending or descending).
        #[test]
        fn prop_traversal_results_sorted(
            file_count in 0usize..20,
            sort_key_idx in 0usize..3,
            sort_order_idx in 0usize..2
        ) {
            let sort_key = match sort_key_idx {
                0 => SortKey::Name,
                1 => SortKey::Size,
                _ => SortKey::Date,
            };
            let sort_order = match sort_order_idx {
                0 => SortOrder::Ascending,
                _ => SortOrder::Descending,
            };

            // Create a temp directory with random files
            let temp_dir = TempDir::new().unwrap();
            
            // Create files with varying sizes
            for i in 0..file_count {
                let file_path = temp_dir.path().join(format!("file_{:03}.txt", i));
                let mut file = File::create(&file_path).unwrap();
                // Write varying amounts of data to create different sizes
                use std::io::Write;
                let content = vec![b'x'; (i + 1) * 10];
                file.write_all(&content).unwrap();
            }

            // Create some directories
            let dir_count = file_count / 3;
            for i in 0..dir_count {
                fs::create_dir(temp_dir.path().join(format!("dir_{:03}", i))).unwrap();
            }

            let config = TraversalConfig {
                sort_key,
                sort_order,
                include_hidden: false,
                max_depth: Some(1),
            };

            let (sender, receiver) = flume::unbounded();
            let result = traverse_directory_sorted(temp_dir.path(), &config, sender);
            prop_assert!(result.is_ok());

            let entries: Vec<_> = receiver.iter().collect();
            
            // Verify entries are sorted correctly
            prop_assert!(
                is_sorted(&entries, sort_key, sort_order),
                "Entries not sorted correctly for {:?} {:?}",
                sort_key, sort_order
            );
        }
    }
}
