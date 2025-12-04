use super::*;

#[test]
fn test_new_cache_is_empty() {
    let cache = IconCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_insert_and_get() {
    let mut cache = IconCache::new();
    let key = IconKey::Extension("txt".to_string());
    let image = RenderImage::new(32, 32, vec![0; 32 * 32 * 4]);

    cache.insert(key.clone(), image.clone());

    assert!(cache.contains(&key));
    assert_eq!(cache.len(), 1);

    let retrieved = cache.get_icon(&key);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), &image);
}

#[test]
fn test_get_or_default_returns_default_for_missing() {
    let mut cache = IconCache::new();
    let key = IconKey::Extension("pdf".to_string());
    let expected_default = cache.default_icon().clone();

    let result = cache.get_or_default(&key);

    assert_eq!(result, &expected_default);
    // Should add to pending
    assert!(cache.is_pending(&key));
}

#[test]
fn test_get_or_default_returns_folder_icon_for_directory() {
    let mut cache = IconCache::new();
    let key = IconKey::Directory;
    let expected_folder = cache.folder_icon().clone();

    let result = cache.get_or_default(&key);

    assert_eq!(result, &expected_folder);
    assert!(cache.is_pending(&key));
}

#[test]
fn test_insert_removes_from_pending() {
    let mut cache = IconCache::new();
    let key = IconKey::Extension("rs".to_string());

    // First access adds to pending
    let _ = cache.get_or_default(&key);
    assert!(cache.is_pending(&key));

    // Insert removes from pending
    cache.insert(key.clone(), RenderImage::default_placeholder());
    assert!(!cache.is_pending(&key));
}

#[test]
fn test_remove() {
    let mut cache = IconCache::new();
    let key = IconKey::GenericFile;
    let image = RenderImage::default_placeholder();

    cache.insert(key.clone(), image.clone());
    assert!(cache.contains(&key));

    let removed = cache.remove(&key);
    assert!(removed.is_some());
    assert!(!cache.contains(&key));
}

#[test]
fn test_clear() {
    let mut cache = IconCache::new();

    cache.insert(IconKey::Directory, RenderImage::default_folder());
    cache.insert(IconKey::GenericFile, RenderImage::default_placeholder());
    let _ = cache.get_or_default(&IconKey::Extension("md".to_string()));

    assert_eq!(cache.len(), 2);
    assert!(!cache.pending_keys().is_empty());

    cache.clear();

    assert!(cache.is_empty());
    assert!(cache.pending_keys().is_empty());
}

#[test]
fn test_max_entries() {
    let cache = IconCache::with_capacity(100);
    assert_eq!(cache.max_entries(), 100);
}

#[test]
fn test_capacity_minimum_is_one() {
    let cache = IconCache::with_capacity(0);
    assert_eq!(cache.max_entries(), 1);
}

// Property-based tests
use proptest::prelude::*;
use std::path::PathBuf;

fn arb_icon_key() -> impl Strategy<Value = IconKey> {
    prop_oneof![
        Just(IconKey::Directory),
        Just(IconKey::GenericFile),
        "[a-z]{1,10}".prop_map(IconKey::Extension),
        "[a-z]+/[a-z]+".prop_map(IconKey::MimeType),
        "[a-zA-Z0-9_/]{1,50}".prop_map(|s| IconKey::Custom(PathBuf::from(s))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: file-explorer-core, Property 9: Icon Cache Miss Returns Default**
    /// **Validates: Requirements 4.1**
    ///
    /// *For any* IconKey not present in the cache, `get_or_default` SHALL return
    /// the default placeholder icon AND add the key to the pending fetch set.
    #[test]
    fn prop_icon_cache_miss_returns_default(key in arb_icon_key()) {
        let mut cache = IconCache::new();

        // Ensure key is not in cache
        prop_assert!(!cache.contains(&key));

        let expected_default = match &key {
            IconKey::Directory => cache.folder_icon().clone(),
            _ => cache.default_icon().clone(),
        };

        let result = cache.get_or_default(&key);

        prop_assert_eq!(result, &expected_default,
            "Cache miss should return default icon for key {:?}", key);

        prop_assert!(cache.is_pending(&key),
            "Cache miss should add key {:?} to pending set", key);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: file-explorer-core, Property 12: LRU Eviction Bounds Cache Size**
    /// **Validates: Requirements 4.4**
    ///
    /// *For any* sequence of icon insertions, when cache size exceeds `max_entries`,
    /// the least recently accessed icons SHALL be evicted until size is within bounds.
    #[test]
    fn prop_lru_eviction_bounds_cache_size(
        max_entries in 1usize..50,
        num_insertions in 1usize..200
    ) {
        let mut cache = IconCache::with_capacity(max_entries);

        // Insert more items than capacity
        for i in 0..num_insertions {
            let key = IconKey::Extension(format!("ext{}", i));
            let image = RenderImage::new(16, 16, vec![i as u8; 16 * 16 * 4]);
            cache.insert(key, image);

            // Cache size should never exceed max_entries
            prop_assert!(
                cache.len() <= max_entries,
                "Cache size {} exceeded max_entries {} after {} insertions",
                cache.len(), max_entries, i + 1
            );
        }

        // Final size should be at most max_entries
        prop_assert!(
            cache.len() <= max_entries,
            "Final cache size {} exceeded max_entries {}",
            cache.len(), max_entries
        );
    }
}

#[test]
fn test_icon_fetch_result_success() {
    let key = IconKey::Extension("png".to_string());
    let image = RenderImage::default_placeholder();
    let result = IconFetchResult::success(key.clone(), image.clone());

    assert!(result.is_success());
    assert_eq!(result.key, key);
    assert!(result.image.is_some());
    assert!(result.error.is_none());
}

#[test]
fn test_icon_fetch_result_failure() {
    let key = IconKey::Extension("png".to_string());
    let result = IconFetchResult::failure(key.clone(), "File not found".to_string());

    assert!(!result.is_success());
    assert_eq!(result.key, key);
    assert!(result.image.is_none());
    assert!(result.error.is_some());
}

#[test]
fn test_process_fetch_results() {
    let mut cache = IconCache::new();
    let key = IconKey::Extension("txt".to_string());

    // Add to pending first
    let _ = cache.get_or_default(&key);
    assert!(cache.is_pending(&key));

    let results = vec![IconFetchResult::success(
        key.clone(),
        RenderImage::default_placeholder(),
    )];
    let count = cache.process_fetch_results(results);

    assert_eq!(count, 1);
    assert!(cache.contains(&key));
    assert!(!cache.is_pending(&key));
}

#[test]
fn test_process_fetch_results_failure_removes_pending() {
    let mut cache = IconCache::new();
    let key = IconKey::Extension("txt".to_string());

    // Add to pending first
    let _ = cache.get_or_default(&key);
    assert!(cache.is_pending(&key));

    let results = vec![IconFetchResult::failure(key.clone(), "Error".to_string())];
    let count = cache.process_fetch_results(results);

    assert_eq!(count, 0);
    assert!(!cache.contains(&key));
    assert!(!cache.is_pending(&key));
}

#[test]
fn test_icon_fetch_pipeline_default_icons() {
    let pipeline = IconFetchPipeline::new();

    // Request default icons (no path)
    pipeline.request_icon(IconKey::Directory, None);
    pipeline.request_icon(IconKey::GenericFile, None);

    std::thread::sleep(std::time::Duration::from_millis(50));

    let results = pipeline.poll_results();
    assert_eq!(results.len(), 2);

    for result in results {
        assert!(result.is_success());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: file-explorer-core, Property 11: Icon Fetch Completion Updates Cache**
    /// **Validates: Requirements 4.3**
    ///
    /// *For any* completed icon fetch for key K, the cache SHALL contain the decoded
    /// texture for K AND K SHALL be removed from the pending set.
    #[test]
    fn prop_icon_fetch_completion_updates_cache(key in arb_icon_key()) {
        let mut cache = IconCache::new();

        // First, trigger a cache miss to add to pending
        let _ = cache.get_or_default(&key);
        prop_assert!(cache.is_pending(&key), "Key should be in pending set after cache miss");

        // Simulate a successful fetch completion
        let image = RenderImage::new(32, 32, vec![0u8; 32 * 32 * 4]);
        let result = IconFetchResult::success(key.clone(), image.clone());

        let count = cache.process_fetch_results(vec![result]);

        // Verify: cache should contain the icon
        prop_assert_eq!(count, 1, "Should have processed 1 result");
        prop_assert!(cache.contains(&key), "Cache should contain key after fetch completion");

        // Verify: key should be removed from pending
        prop_assert!(!cache.is_pending(&key), "Key should be removed from pending after fetch completion");

        // Verify: we can retrieve the icon
        let retrieved = cache.get_icon(&key);
        prop_assert!(retrieved.is_some(), "Should be able to retrieve the icon");
        prop_assert_eq!(retrieved.unwrap(), &image, "Retrieved icon should match inserted icon");
    }
}
