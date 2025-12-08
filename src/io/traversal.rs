use std::path::{Path, PathBuf};
use std::time::SystemTime;

use flume::Sender;
use jwalk::{WalkDir, WalkDirGeneric};
use serde::{Deserialize, Serialize};

use crate::models::{FileEntry, FileSystemError, Result};

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Date,
}

/
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

/
/
/
/
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
                if entry.depth() == 0 {
                    continue;
                }

                if let Some(file_entry) = dir_entry_to_file_entry(&entry) {
                    if !config.include_hidden && is_hidden(&file_entry.name) {
                        continue;
                    }

                    if sender.send(file_entry).is_err() {
                        break;
                    }
                    count += 1;
                }
            }
            Err(e) => {
                eprintln!("Traversal error: {}", e);
            }
        }
    }

    Ok(count)
}

/
fn build_walk_dir(path: &Path, config: &TraversalConfig) -> WalkDirGeneric<((), ())> {
    let mut walk_dir = WalkDir::new(path)
        .parallelism(jwalk::Parallelism::RayonNewPool(num_cpus()))
        .skip_hidden(!config.include_hidden);

    if let Some(depth) = config.max_depth {
        walk_dir = walk_dir.max_depth(depth);
    }

    walk_dir = walk_dir.sort(true);

    walk_dir
}

/
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/
fn dir_entry_to_file_entry(entry: &jwalk::DirEntry<((), ())>) -> Option<FileEntry> {
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().to_string();

    let symlink_metadata = std::fs::symlink_metadata(&path).ok()?;
    let is_symlink = symlink_metadata.file_type().is_symlink();

    if is_symlink {
        let target = std::fs::read_link(&path).ok();
        let target_exists = std::fs::metadata(&path).is_ok();
        let is_broken = !target_exists;

        let (is_dir, size, modified) = if target_exists {
            let target_meta = std::fs::metadata(&path).ok()?;
            (
                target_meta.is_dir(),
                if target_meta.is_dir() { 0 } else { target_meta.len() },
                target_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            )
        } else {
            (false, 0, symlink_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH))
        };

        let mut file_entry = FileEntry::new(name, path, is_dir, size, modified);
        if let Some(target_path) = target {
            file_entry = file_entry.with_symlink_info(target_path, is_broken);
        } else {
            file_entry.is_symlink = true;
            file_entry.is_broken_symlink = true;
            file_entry.file_type = crate::models::FileType::Symlink;
        }
        Some(file_entry)
    } else {
        let metadata = entry.metadata().ok()?;
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        Some(FileEntry::new(name, path, is_dir, size, modified))
    }
}

/
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/
pub fn sort_entries(entries: &mut [FileEntry], sort_key: SortKey, sort_order: SortOrder) {
    match (sort_key, sort_order) {
        (SortKey::Name, SortOrder::Ascending) => {
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });
        }
        (SortKey::Name, SortOrder::Descending) => {
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.name.to_lowercase().cmp(&a.name.to_lowercase()),
            });
        }
        (SortKey::Size, SortOrder::Ascending) => {
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.size.cmp(&b.size),
            });
        }
        (SortKey::Size, SortOrder::Descending) => {
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.size.cmp(&a.size),
            });
        }
        (SortKey::Date, SortOrder::Ascending) => {
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.modified.cmp(&b.modified),
            });
        }
        (SortKey::Date, SortOrder::Descending) => {
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.modified.cmp(&a.modified),
            });
        }
    }
}

/
/
/
/
/
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

    sort_entries(&mut entries, config.sort_key, config.sort_order);

    let count = entries.len();
    for entry in entries {
        if sender.send(entry).is_err() {
            break;
        }
    }

    Ok(count)
}

/
/
pub fn spawn_traversal(
    path: PathBuf,
    config: TraversalConfig,
) -> (
    flume::Receiver<FileEntry>,
    std::thread::JoinHandle<Result<usize>>,
) {
    let (sender, receiver) = flume::unbounded();

    let handle = std::thread::spawn(move || traverse_directory(&path, &config, sender));

    (receiver, handle)
}

/
/
pub fn spawn_sorted_traversal(
    path: PathBuf,
    config: TraversalConfig,
) -> (
    flume::Receiver<FileEntry>,
    std::thread::JoinHandle<Result<usize>>,
) {
    let (sender, receiver) = flume::unbounded();

    let handle = std::thread::spawn(move || traverse_directory_sorted(&path, &config, sender));

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

        File::create(temp_dir.path().join("file_a.txt")).unwrap();
        File::create(temp_dir.path().join("file_b.txt")).unwrap();
        File::create(temp_dir.path().join("file_c.txt")).unwrap();

        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

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
            FileEntry::new(
                "zebra.txt".to_string(),
                PathBuf::from("/zebra.txt"),
                false,
                100,
                SystemTime::UNIX_EPOCH,
            ),
            FileEntry::new(
                "alpha.txt".to_string(),
                PathBuf::from("/alpha.txt"),
                false,
                200,
                SystemTime::UNIX_EPOCH,
            ),
            FileEntry::new(
                "beta".to_string(),
                PathBuf::from("/beta"),
                true,
                0,
                SystemTime::UNIX_EPOCH,
            ),
        ];

        sort_entries(&mut entries, SortKey::Name, SortOrder::Ascending);

        assert_eq!(entries[0].name, "beta");
        assert_eq!(entries[1].name, "alpha.txt");
        assert_eq!(entries[2].name, "zebra.txt");
    }

    #[test]
    fn test_sort_entries_by_size() {
        let mut entries = vec![
            FileEntry::new(
                "small.txt".to_string(),
                PathBuf::from("/small.txt"),
                false,
                100,
                SystemTime::UNIX_EPOCH,
            ),
            FileEntry::new(
                "large.txt".to_string(),
                PathBuf::from("/large.txt"),
                false,
                1000,
                SystemTime::UNIX_EPOCH,
            ),
            FileEntry::new(
                "dir".to_string(),
                PathBuf::from("/dir"),
                true,
                0,
                SystemTime::UNIX_EPOCH,
            ),
        ];

        sort_entries(&mut entries, SortKey::Size, SortOrder::Ascending);

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
        assert_eq!(entries.len(), 4);

        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "subdir");

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

        assert!(entries[0].is_dir);

        assert_eq!(entries[1].name, "file_c.txt");
        assert_eq!(entries[2].name, "file_b.txt");
        assert_eq!(entries[3].name, "file_a.txt");
    }

    /
    fn is_sorted(entries: &[FileEntry], sort_key: SortKey, sort_order: SortOrder) -> bool {
        if entries.len() <= 1 {
            return true;
        }

        let mut seen_file = false;
        for entry in entries {
            if entry.is_dir && seen_file {
                return false;
            }
            if !entry.is_dir {
                seen_file = true;
            }
        }

        let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir).collect();
        let files: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();

        is_group_sorted(&dirs, sort_key, sort_order)
            && is_group_sorted(&files, sort_key, sort_order)
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
        /
        /
        /
        /
        /
        /
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

            let temp_dir = TempDir::new().unwrap();

            for i in 0..file_count {
                let file_path = temp_dir.path().join(format!("file_{:03}.txt", i));
                let mut file = File::create(&file_path).unwrap();
                use std::io::Write;
                let content = vec![b'x'; (i + 1) * 10];
                file.write_all(&content).unwrap();
            }

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

            prop_assert!(
                is_sorted(&entries, sort_key, sort_order),
                "Entries not sorted correctly for {:?} {:?}",
                sort_key, sort_order
            );
        }
    }
}
