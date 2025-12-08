use crate::models::{
    CloudSyncStatus, DateFilter, FileEntry, FileType, IconKey, SearchQuery, SizeFilter,
    SmartFolder, SmartFolderId, SmartFolderManager, TagId,
};
use proptest::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

fn create_test_entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
    let file_type = if is_dir {
        FileType::Directory
    } else {
        FileType::RegularFile
    };
    let icon_key = if is_dir {
        IconKey::Directory
    } else {
        IconKey::GenericFile
    };
    FileEntry {
        name: name.to_string(),
        path: PathBuf::from(format!("/test/{}", name)),
        is_dir,
        size,
        modified: SystemTime::now(),
        file_type,
        icon_key,
        linux_permissions: None,
        sync_status: CloudSyncStatus::None,
        is_symlink: false,
        symlink_target: None,
        is_broken_symlink: false,
        is_shared: false,
    }
}

fn create_entry_with_extension(name: &str, ext: &str, size: u64) -> FileEntry {
    let full_name = format!("{}.{}", name, ext);
    let file_type = FileType::RegularFile;
    let icon_key = IconKey::Extension(ext.to_string());
    FileEntry {
        name: full_name.clone(),
        path: PathBuf::from(format!("/test/{}", full_name)),
        is_dir: false,
        size,
        modified: SystemTime::now(),
        file_type,
        icon_key,
        linux_permissions: None,
        sync_status: CloudSyncStatus::None,
        is_symlink: false,
        symlink_target: None,
        is_broken_symlink: false,
        is_shared: false,
    }
}

// Strategy for generating valid file names
fn file_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_]{0,15}".prop_map(|s| s)
}

// Strategy for generating file extensions
fn extension_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("rs".to_string()),
        Just("txt".to_string()),
        Just("md".to_string()),
        Just("json".to_string()),
        Just("toml".to_string()),
        Just("py".to_string()),
        Just("js".to_string()),
    ]
}

// Strategy for generating file entries
fn file_entry_strategy() -> impl Strategy<Value = FileEntry> {
    (
        file_name_strategy(),
        extension_strategy(),
        prop::bool::ANY,
        0u64..10000,
    )
        .prop_map(|(name, ext, is_dir, size)| {
            if is_dir {
                create_test_entry(&name, true, 0)
            } else {
                create_entry_with_extension(&name, &ext, size)
            }
        })
}

// Strategy for generating search queries
fn search_query_strategy() -> impl Strategy<Value = SearchQuery> {
    (
        prop::option::of(file_name_strategy()),
        prop::collection::vec(extension_strategy(), 0..3),
        prop::option::of(prop_oneof![
            (1u32..30).prop_map(DateFilter::LastDays),
            (1u32..12).prop_map(DateFilter::LastWeeks),
            (1u32..12).prop_map(DateFilter::LastMonths),
        ]),
        prop::option::of(prop_oneof![
            (1u64..10000).prop_map(SizeFilter::SmallerThan),
            (1u64..10000).prop_map(SizeFilter::LargerThan),
            Just(SizeFilter::Empty),
            Just(SizeFilter::NonEmpty),
        ]),
        prop::bool::ANY,
        prop::bool::ANY,
        prop::bool::ANY,
    )
        .prop_map(
            |(
                text,
                file_types,
                date_filter,
                size_filter,
                include_hidden,
                dirs_only,
                files_only,
            )| {
                SearchQuery {
                    text,
                    file_types,
                    date_filter,
                    size_filter,
                    tags: Vec::new(),
                    locations: Vec::new(),
                    recursive: true,
                    include_hidden,
                    directories_only: dirs_only && !files_only,
                    files_only: files_only && !dirs_only,
                }
            },
        )
}

/// **Feature: ui-enhancements, Property 44: Smart Folder Query Consistency**
/// **Validates: Requirements 24.3**
///
/// *For any* saved smart folder, opening it SHALL return the same results
/// as running the query manually against the same set of files.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_smart_folder_query_consistency(
        folder_name in "[a-zA-Z][a-zA-Z0-9 ]{0,20}",
        query in search_query_strategy(),
        entries in prop::collection::vec(file_entry_strategy(), 0..20),
    ) {
        let mut manager = SmartFolderManager::new();

        let id = manager.create(folder_name, query.clone()).unwrap();

        // Execute the query through the smart folder
        let smart_folder_results = manager
            .execute(id, &entries, |_| HashSet::new())
            .unwrap();

        // Execute the same query manually
        let manual_results = manager.execute_query(&query, &entries, |_| HashSet::new());

        // Results should be identical
        prop_assert_eq!(
            smart_folder_results.len(),
            manual_results.len(),
            "Smart folder and manual query should return same number of results"
        );

        for (sf_entry, manual_entry) in smart_folder_results.iter().zip(manual_results.iter()) {
            prop_assert_eq!(
                &sf_entry.path,
                &manual_entry.path,
                "Results should be in the same order with same paths"
            );
        }
    }

    #[test]
    fn prop_query_filter_correctness(
        query in search_query_strategy(),
        entries in prop::collection::vec(file_entry_strategy(), 1..30),
    ) {
        let manager = SmartFolderManager::new();
        let results = manager.execute_query(&query, &entries, |_| HashSet::new());

        // Every result should match the query
        for entry in &results {
            let matches = query.matches(entry, &HashSet::new());
            prop_assert!(
                matches,
                "Entry {:?} was returned but doesn't match query {:?}",
                entry.name,
                query.text
            );
        }

        // Every entry that matches should be in results
        for entry in &entries {
            let matches = query.matches(entry, &HashSet::new());
            let in_results = results.iter().any(|r| r.path == entry.path);
            prop_assert_eq!(
                matches,
                in_results,
                "Entry {:?} match status ({}) doesn't match presence in results ({})",
                entry.name,
                matches,
                in_results
            );
        }
    }

    #[test]
    fn prop_text_filter_case_insensitive(
        base_name in file_name_strategy(),
        entries in prop::collection::vec(file_entry_strategy(), 1..20),
    ) {
        let manager = SmartFolderManager::new();

        let lower_query = SearchQuery::with_text(base_name.to_lowercase());
        let upper_query = SearchQuery::with_text(base_name.to_uppercase());

        let lower_results = manager.execute_query(&lower_query, &entries, |_| HashSet::new());
        let upper_results = manager.execute_query(&upper_query, &entries, |_| HashSet::new());

        prop_assert_eq!(
            lower_results.len(),
            upper_results.len(),
            "Case-insensitive search should return same number of results"
        );
    }

    #[test]
    fn prop_empty_query_returns_all_visible(
        entries in prop::collection::vec(file_entry_strategy(), 0..20),
    ) {
        let manager = SmartFolderManager::new();

        let query = SearchQuery {
            include_hidden: true,
            ..Default::default()
        };

        let results = manager.execute_query(&query, &entries, |_| HashSet::new());

        prop_assert_eq!(
            results.len(),
            entries.len(),
            "Empty query with include_hidden should return all entries"
        );
    }

    #[test]
    fn prop_directories_only_filter(
        entries in prop::collection::vec(file_entry_strategy(), 1..20),
    ) {
        let manager = SmartFolderManager::new();

        let query = SearchQuery {
            directories_only: true,
            include_hidden: true,
            ..Default::default()
        };

        let results = manager.execute_query(&query, &entries, |_| HashSet::new());

        // All results should be directories
        for entry in &results {
            prop_assert!(
                entry.is_dir,
                "directories_only filter returned non-directory: {}",
                entry.name
            );
        }

        // Count should match
        let expected_count = entries.iter().filter(|e| e.is_dir).count();
        prop_assert_eq!(results.len(), expected_count);
    }

    #[test]
    fn prop_files_only_filter(
        entries in prop::collection::vec(file_entry_strategy(), 1..20),
    ) {
        let manager = SmartFolderManager::new();

        let query = SearchQuery {
            files_only: true,
            include_hidden: true,
            ..Default::default()
        };

        let results = manager.execute_query(&query, &entries, |_| HashSet::new());

        // All results should be files
        for entry in &results {
            prop_assert!(
                !entry.is_dir,
                "files_only filter returned directory: {}",
                entry.name
            );
        }

        // Count should match
        let expected_count = entries.iter().filter(|e| !e.is_dir).count();
        prop_assert_eq!(results.len(), expected_count);
    }

    #[test]
    fn prop_size_filter_correctness(
        threshold in 100u64..5000,
        entries in prop::collection::vec(file_entry_strategy(), 1..20),
    ) {
        let manager = SmartFolderManager::new();

        let query = SearchQuery {
            size_filter: Some(SizeFilter::LargerThan(threshold)),
            include_hidden: true,
            ..Default::default()
        };

        let results = manager.execute_query(&query, &entries, |_| HashSet::new());

        // All file results should have size > threshold
        for entry in &results {
            if !entry.is_dir {
                prop_assert!(
                    entry.size > threshold,
                    "Size filter returned file with size {} <= threshold {}",
                    entry.size,
                    threshold
                );
            }
        }
    }

    #[test]
    fn prop_file_type_filter_correctness(
        extensions in prop::collection::vec(extension_strategy(), 1..3),
        entries in prop::collection::vec(file_entry_strategy(), 1..20),
    ) {
        let manager = SmartFolderManager::new();

        let query = SearchQuery {
            file_types: extensions.clone(),
            include_hidden: true,
            ..Default::default()
        };

        let results = manager.execute_query(&query, &entries, |_| HashSet::new());

        // All file results should have one of the specified extensions
        for entry in &results {
            if !entry.is_dir {
                let ext = entry
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let matches_ext = extensions.iter().any(|e| e.to_lowercase() == ext);
                prop_assert!(
                    matches_ext,
                    "File type filter returned file with extension '{}' not in {:?}",
                    ext,
                    extensions
                );
            }
        }
    }
}

#[test]
fn test_smart_folder_persistence_round_trip() {
    let mut manager = SmartFolderManager::new();

    let query = SearchQuery::with_text("test")
        .file_types(vec!["rs".to_string()])
        .size_filter(SizeFilter::LargerThan(100));

    let id = manager
        .create("Test Folder".to_string(), query.clone())
        .unwrap();

    // Serialize and deserialize
    let json = serde_json::to_string(&manager).unwrap();
    let loaded: SmartFolderManager = serde_json::from_str(&json).unwrap();

    // Verify the folder was preserved
    let folder = loaded.get(id).unwrap();
    assert_eq!(folder.name, "Test Folder");
    assert_eq!(folder.query.text, Some("test".to_string()));
    assert_eq!(folder.query.file_types, vec!["rs".to_string()]);
}

#[test]
fn test_query_with_tags() {
    let manager = SmartFolderManager::new();
    let tag1 = TagId::new(1);
    let tag2 = TagId::new(2);

    let query = SearchQuery::new().tags(vec![tag1]).include_hidden(true);

    let entries = vec![
        create_test_entry("file1.txt", false, 100),
        create_test_entry("file2.txt", false, 100),
    ];

    // File 1 has tag1, file 2 has tag2
    let results = manager.execute_query(&query, &entries, |path| {
        let mut tags = HashSet::new();
        if path.to_string_lossy().contains("file1") {
            tags.insert(tag1);
        } else {
            tags.insert(tag2);
        }
        tags
    });

    assert_eq!(results.len(), 1);
    assert!(results[0].name.contains("file1"));
}
