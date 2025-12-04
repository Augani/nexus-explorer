/// Property-based tests for FileSystem model
/// **Feature: file-explorer-core**

use super::*;
use crate::models::{CloudSyncStatus, FileEntry, FileType, IconKey, LoadState};
use proptest::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn arb_system_time() -> impl Strategy<Value = SystemTime> {
    (0u64..253402300799u64, 0u32..1_000_000_000u32).prop_map(|(secs, nanos)| {
        UNIX_EPOCH + Duration::new(secs, nanos)
    })
}

fn arb_file_entry() -> impl Strategy<Value = FileEntry> {
    (
        "[a-zA-Z0-9_.-]{1,50}",
        "[a-zA-Z0-9_/.-]{1,100}",
        any::<bool>(),
        any::<u64>(),
        arb_system_time(),
    )
        .prop_map(|(name, path_str, is_dir, size, modified)| {
            FileEntry {
                name: name.clone(),
                path: PathBuf::from(&path_str),
                is_dir,
                size,
                modified,
                file_type: if is_dir { FileType::Directory } else { FileType::RegularFile },
                icon_key: if is_dir { IconKey::Directory } else { IconKey::GenericFile },
                linux_permissions: None,
                sync_status: CloudSyncStatus::None,
            }
        })
}

fn arb_file_entries(max_count: usize) -> impl Strategy<Value = Vec<FileEntry>> {
    prop::collection::vec(arb_file_entry(), 0..max_count)
}

fn arb_path() -> impl Strategy<Value = PathBuf> {
    "[a-zA-Z0-9_/.-]{1,100}".prop_map(PathBuf::from)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: file-explorer-core, Property 1: Loading State Consistency**
    /// **Validates: Requirements 1.2**
    ///
    /// *For any* FileSystem model transitioning to a loading state, the `state` field
    /// SHALL be `LoadState::Loading` with a valid `request_id` until the operation
    /// completes or is superseded.
    #[test]
    fn prop_loading_state_consistency(
        path in arb_path(),
        entries in arb_file_entries(20),
    ) {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let request_id = fs.begin_load(path.clone());
        
        match fs.state() {
            LoadState::Loading { request_id: state_id } => {
                prop_assert_eq!(*state_id, request_id,
                    "Loading state request_id {} should match begin_load return value {}",
                    state_id, request_id);
            }
            LoadState::Cached { .. } => {
                // This is valid if the path was previously cached
                // In this case, we should verify the path is in cache
                prop_assert!(fs.is_cached(&path), 
                    "If state is Cached, path should be in cache");
            }
            other => {
                prop_assert!(false, 
                    "After begin_load, state should be Loading or Cached, got {:?}", other);
            }
        }
        
        let mtime = SystemTime::now();
        let applied = fs.complete_load(request_id, entries.clone(), Duration::from_millis(50), mtime);
        prop_assert!(applied, "complete_load with valid request_id should succeed");
        
        match fs.state() {
            LoadState::Loaded { count, .. } => {
                prop_assert_eq!(*count, entries.len(),
                    "Loaded state count {} should match entries length {}",
                    count, entries.len());
            }
            other => {
                prop_assert!(false, 
                    "After complete_load, state should be Loaded, got {:?}", other);
            }
        }
    }

    /// **Feature: file-explorer-core, Property 1: Loading State Superseded**
    /// **Validates: Requirements 1.2**
    ///
    /// When a new navigation request supersedes a loading operation, the state
    /// should transition to the new Loading state with the new request_id.
    #[test]
    fn prop_loading_state_superseded(
        path1 in arb_path(),
        path2 in arb_path(),
    ) {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let id1 = fs.begin_load(path1.clone());
        
        match fs.state() {
            LoadState::Loading { request_id } => {
                prop_assert_eq!(*request_id, id1);
            }
            LoadState::Cached { .. } => {
                // Valid if path1 was cached
            }
            other => {
                prop_assert!(false, "Unexpected state: {:?}", other);
            }
        }
        
        let id2 = fs.begin_load(path2.clone());
        prop_assert!(id2 > id1, "Second request_id should be greater than first");
        
        match fs.state() {
            LoadState::Loading { request_id } => {
                prop_assert_eq!(*request_id, id2,
                    "After superseding, state should have new request_id {}, got {}",
                    id2, request_id);
            }
            LoadState::Cached { .. } => {
                // Valid if path2 was cached
            }
            other => {
                prop_assert!(false, "Unexpected state after supersede: {:?}", other);
            }
        }
        
        // First request_id should now be invalid
        prop_assert!(!fs.is_valid_request(id1), 
            "Old request_id should be invalid after supersede");
        prop_assert!(fs.is_valid_request(id2),
            "New request_id should be valid");
    }

    /// **Feature: file-explorer-core, Property 4: Generational ID Monotonicity and Validation**
    /// **Validates: Requirements 1.5, 8.1, 8.2, 8.3**
    ///
    /// *For any* sequence of N navigation requests, each request SHALL receive a strictly
    /// increasing request_id, and only results matching the current request_id SHALL be
    /// applied to the model state.
    #[test]
    fn prop_generational_id_monotonicity(
        paths in prop::collection::vec(arb_path(), 1..20)
    ) {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        let mut prev_id = 0usize;

        for path in &paths {
            let new_id = fs.begin_load(path.clone());
            
            // Each request_id must be strictly greater than the previous
            prop_assert!(new_id > prev_id, 
                "Request ID {} should be greater than previous {}", new_id, prev_id);
            
            prop_assert_eq!(fs.request_id(), new_id);
            
            prev_id = new_id;
        }
    }

    /// **Feature: file-explorer-core, Property 4: Generational ID Validation**
    /// **Validates: Requirements 1.5, 8.1, 8.2, 8.3**
    ///
    /// Only results matching the current request_id SHALL be applied to the model state.
    /// Stale results (with old request_ids) must be discarded.
    #[test]
    fn prop_stale_request_discarded(
        path1 in arb_path(),
        path2 in arb_path(),
        entries1 in arb_file_entries(10),
        entries2 in arb_file_entries(10),
    ) {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let id1 = fs.begin_load(path1.clone());
        
        let id2 = fs.begin_load(path2.clone());
        
        let stale_applied = fs.complete_load(
            id1,
            entries1.clone(),
            Duration::from_millis(100),
            SystemTime::now(),
        );
        prop_assert!(!stale_applied, "Stale request should be rejected");
        
        match fs.state() {
            LoadState::Loading { request_id } => {
                prop_assert_eq!(*request_id, id2);
            }
            LoadState::Cached { .. } => {
                // This is also valid if path2 was cached
            }
            other => {
                prop_assert!(false, "Unexpected state after stale rejection: {:?}", other);
            }
        }
        
        let valid_applied = fs.complete_load(
            id2,
            entries2.clone(),
            Duration::from_millis(50),
            SystemTime::now(),
        );
        prop_assert!(valid_applied, "Valid request should be applied");
        
        prop_assert_eq!(fs.entries().len(), entries2.len());
    }


    /// **Feature: file-explorer-core, Property 3: Cache Hit Returns Cached Data**
    /// **Validates: Requirements 1.4**
    ///
    /// *For any* path that exists in the LRU cache, calling begin_load SHALL immediately
    /// make cached entries available via entries() before any async operation completes.
    #[test]
    fn prop_cache_hit_returns_cached_data(
        path in arb_path(),
        entries in arb_file_entries(20),
    ) {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        let mtime = SystemTime::now();
        
        let id1 = fs.begin_load(path.clone());
        fs.complete_load(id1, entries.clone(), Duration::from_millis(100), mtime);
        
        let _ = fs.begin_load(PathBuf::from("/other"));
        
        // Navigate back - should hit cache
        let _ = fs.begin_load(path.clone());
        
        // Entries should be immediately available from cache
        prop_assert_eq!(fs.entries().len(), entries.len(), 
            "Cache hit should immediately provide entries");
        
        // State should indicate cached
        match fs.state() {
            LoadState::Cached { .. } => {}
            other => {
                prop_assert!(false, "Expected Cached state, got {:?}", other);
            }
        }
    }

    /// **Feature: file-explorer-core, Property 20: Cache Generation Stored**
    /// **Validates: Requirements 8.4**
    ///
    /// *For any* CachedDirectory, the generation field SHALL equal the request_id
    /// that was active when the cache entry was created.
    #[test]
    fn prop_cache_generation_stored(
        path in arb_path(),
        entries in arb_file_entries(10),
    ) {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        let mtime = SystemTime::now();
        
        let request_id = fs.begin_load(path.clone());
        
        fs.complete_load(request_id, entries.clone(), Duration::from_millis(50), mtime);
        
        // Retrieve cached entry and verify generation matches request_id
        let cached = fs.get_cached(&path)
            .expect("Path should be cached after complete_load");
        
        prop_assert_eq!(cached.generation, request_id,
            "Cached generation {} should equal request_id {}", 
            cached.generation, request_id);
    }
}

use crate::models::FsEvent;
use tempfile::TempDir;
use std::fs::File;

fn setup_temp_dir_with_files(filenames: &[&str]) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().to_path_buf();
    
    for filename in filenames {
        File::create(dir_path.join(filename)).unwrap();
    }
    
    (temp_dir, dir_path)
}

fn create_fs_with_entries(dir_path: &Path, filenames: &[&str]) -> FileSystem {
    let mut fs = FileSystem::new(dir_path.to_path_buf());
    let mtime = SystemTime::now();
    
    let entries: Vec<FileEntry> = filenames.iter()
        .map(|name| {
            let path = dir_path.join(name);
            FileEntry::new(
                name.to_string(),
                path,
                false,
                100,
                mtime,
            )
        })
        .collect();
    
    let id = fs.begin_load(dir_path.to_path_buf());
    fs.complete_load(id, entries, Duration::from_millis(10), mtime);
    fs
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_created_event_adds_entry(
        filename in "[a-zA-Z0-9]{1,20}\\.(txt|rs|md)"
    ) {
        let (_temp_dir, dir_path) = setup_temp_dir_with_files(&[]);
        let mut fs = create_fs_with_entries(&dir_path, &[]);
        
        let new_file_path = dir_path.join(&filename);
        File::create(&new_file_path).unwrap();
        
        let event = FsEvent::Created(new_file_path.clone());
        let modified = fs.process_event(event);
        
        prop_assert!(modified, "Created event should modify entries");
        prop_assert!(
            fs.contains_path(&new_file_path),
            "Entries should contain the created file path"
        );
    }
    
    #[test]
    fn prop_deleted_event_removes_entry(
        filename in "[a-zA-Z0-9]{1,20}\\.(txt|rs|md)"
    ) {
        let (_temp_dir, dir_path) = setup_temp_dir_with_files(&[]);
        
        let file_path = dir_path.join(&filename);
        File::create(&file_path).unwrap();
        
        let mut fs = create_fs_with_entries(&dir_path, &[&filename]);
        
        prop_assert!(
            fs.contains_path(&file_path),
            "Entry should exist before deletion"
        );
        
        std::fs::remove_file(&file_path).unwrap();
        
        let event = FsEvent::Deleted(file_path.clone());
        let modified = fs.process_event(event);
        
        prop_assert!(modified, "Deleted event should modify entries");
        prop_assert!(
            !fs.contains_path(&file_path),
            "Entries should NOT contain the deleted file path"
        );
    }
    
    #[test]
    fn prop_events_outside_current_dir_ignored(
        filename in "[a-zA-Z0-9]{1,20}\\.txt"
    ) {
        let (_temp_dir, dir_path) = setup_temp_dir_with_files(&[]);
        let mut fs = create_fs_with_entries(&dir_path, &[]);
        
        let other_dir = PathBuf::from("/some/other/directory");
        let other_file = other_dir.join(&filename);
        
        let event = FsEvent::Created(other_file.clone());
        let modified = fs.process_event(event);
        
        prop_assert!(!modified, "Events outside current directory should be ignored");
        prop_assert!(
            !fs.contains_path(&other_file),
            "Entry from other directory should not be added"
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_new_filesystem_has_idle_state() {
        let fs = FileSystem::new(PathBuf::from("/home"));
        assert!(matches!(fs.state(), LoadState::Idle));
        assert_eq!(fs.current_path(), Path::new("/home"));
        assert!(fs.entries().is_empty());
        assert_eq!(fs.request_id(), 0);
    }

    #[test]
    fn test_begin_load_increments_request_id() {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let id1 = fs.begin_load(PathBuf::from("/path1"));
        assert_eq!(id1, 1);
        
        let id2 = fs.begin_load(PathBuf::from("/path2"));
        assert_eq!(id2, 2);
        
        let id3 = fs.begin_load(PathBuf::from("/path3"));
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_is_valid_request() {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let id1 = fs.begin_load(PathBuf::from("/path1"));
        assert!(fs.is_valid_request(id1));
        
        let id2 = fs.begin_load(PathBuf::from("/path2"));
        assert!(!fs.is_valid_request(id1));
        assert!(fs.is_valid_request(id2));
    }

    #[test]
    fn test_complete_load_caches_entries() {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        let path = PathBuf::from("/test");
        let mtime = SystemTime::now();
        
        let id = fs.begin_load(path.clone());
        
        let entries = vec![
            FileEntry::new("file1.txt".into(), path.join("file1.txt"), false, 100, mtime),
        ];
        
        fs.complete_load(id, entries.clone(), Duration::from_millis(10), mtime);
        
        assert!(fs.is_cached(&path));
        assert_eq!(fs.entries().len(), 1);
    }

    #[test]
    fn test_cache_capacity_limit() {
        let mut fs = FileSystem::with_cache_capacity(PathBuf::from("/"), 3);
        let mtime = SystemTime::now();
        
        // Fill cache beyond capacity
        for i in 0..5 {
            let path = PathBuf::from(format!("/path{}", i));
            let id = fs.begin_load(path.clone());
            fs.complete_load(id, vec![], Duration::from_millis(10), mtime);
        }
        
        // Cache should be bounded to capacity
        assert!(fs.cache_len() <= 3);
    }

    #[test]
    fn test_load_directory_sync() {
        use crate::io::{SortKey, SortOrder};
        use crate::models::load_directory_sync;
        use std::fs::File;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        
        File::create(temp_dir.path().join("file_a.txt")).unwrap();
        File::create(temp_dir.path().join("file_b.txt")).unwrap();
        File::create(temp_dir.path().join("file_c.txt")).unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let result = load_directory_sync(
            &mut fs,
            temp_dir.path().to_path_buf(),
            SortKey::Name,
            SortOrder::Ascending,
            false,
        );

        assert!(result.is_ok());
        assert_eq!(fs.entries().len(), 4);
        
        // Verify sorted order (directories first)
        assert!(fs.entries()[0].is_dir);
        assert_eq!(fs.entries()[0].name, "subdir");
        
        match fs.state() {
            LoadState::Loaded { count, .. } => {
                assert_eq!(*count, 4);
            }
            other => panic!("Expected Loaded state, got {:?}", other),
        }
    }

    #[test]
    fn test_load_path_request_id_validation() {
        use crate::io::{SortKey, SortOrder};
        use std::fs::File;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("file.txt")).unwrap();

        let mut fs = FileSystem::new(PathBuf::from("/"));
        
        let op = fs.load_path(
            temp_dir.path().to_path_buf(),
            SortKey::Name,
            SortOrder::Ascending,
            false,
        );
        let request_id = op.request_id;

        let _op2 = fs.load_path(
            PathBuf::from("/other"),
            SortKey::Name,
            SortOrder::Ascending,
            false,
        );

        while let Ok(batch) = op.batch_receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            let result = fs.process_batch(request_id, batch);
            assert!(result.is_none(), "Stale batch should be rejected");
        }
    }

    #[test]
    fn test_append_entries() {
        let mut fs = FileSystem::new(PathBuf::from("/"));
        let mtime = SystemTime::now();
        
        let id = fs.begin_load(PathBuf::from("/test"));
        
        let entries1 = vec![
            FileEntry::new("file1.txt".into(), PathBuf::from("/test/file1.txt"), false, 100, mtime),
        ];
        let entries2 = vec![
            FileEntry::new("file2.txt".into(), PathBuf::from("/test/file2.txt"), false, 200, mtime),
        ];
        
        // Append first batch
        assert!(fs.append_entries(id, entries1));
        assert_eq!(fs.entries().len(), 1);
        
        // Append second batch
        assert!(fs.append_entries(id, entries2));
        assert_eq!(fs.entries().len(), 2);
        
        // Try to append with stale ID
        let _ = fs.begin_load(PathBuf::from("/other"));
        let entries3 = vec![
            FileEntry::new("file3.txt".into(), PathBuf::from("/test/file3.txt"), false, 300, mtime),
        ];
        assert!(!fs.append_entries(id, entries3));
    }
}
