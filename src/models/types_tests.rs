/// Property-based tests for core types
/// **Feature: file-explorer-core**

use super::{FileEntry, FileType, IconKey, SortColumn, SortDirection, SortState};
use proptest::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn arb_system_time() -> impl Strategy<Value = SystemTime> {
    (0u64..253402300799u64, 0u32..1_000_000_000u32).prop_map(|(secs, nanos)| {
        UNIX_EPOCH + Duration::new(secs, nanos)
    })
}

fn arb_file_type() -> impl Strategy<Value = FileType> {
    prop_oneof![
        Just(FileType::Directory),
        Just(FileType::RegularFile),
        Just(FileType::Symlink),
        Just(FileType::Unknown),
    ]
}

fn arb_icon_key() -> impl Strategy<Value = IconKey> {
    prop_oneof![
        Just(IconKey::Directory),
        Just(IconKey::GenericFile),
        "[a-z]{1,10}".prop_map(IconKey::Extension),
        "[a-z]+/[a-z]+".prop_map(IconKey::MimeType),
        "[a-zA-Z0-9_/]{1,50}".prop_map(|s| IconKey::Custom(PathBuf::from(s))),
    ]
}

fn arb_file_entry() -> impl Strategy<Value = FileEntry> {
    (
        "[a-zA-Z0-9_.-]{1,100}",
        "[a-zA-Z0-9_/.-]{1,200}",
        any::<bool>(),
        any::<u64>(),
        arb_system_time(),
        arb_file_type(),
        arb_icon_key(),
    )
        .prop_map(|(name, path_str, is_dir, size, modified, file_type, icon_key)| {
            FileEntry {
                name,
                path: PathBuf::from(path_str),
                is_dir,
                size,
                modified,
                file_type,
                icon_key,
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: file-explorer-core, Property 21: FileEntry Serialization Round-Trip**
    /// **Validates: Requirements 10.1, 10.4**
    ///
    /// *For any* valid FileEntry, serializing then deserializing SHALL produce
    /// an equivalent FileEntry with identical field values.
    #[test]
    fn prop_file_entry_serialization_round_trip(entry in arb_file_entry()) {
        let serialized = bincode::serialize(&entry)
            .expect("Serialization should succeed for valid FileEntry");

        let deserialized: FileEntry = bincode::deserialize(&serialized)
            .expect("Deserialization should succeed for valid serialized data");

        prop_assert_eq!(entry.name, deserialized.name);
        prop_assert_eq!(entry.path, deserialized.path);
        prop_assert_eq!(entry.is_dir, deserialized.is_dir);
        prop_assert_eq!(entry.size, deserialized.size);
        prop_assert_eq!(entry.modified, deserialized.modified);
        prop_assert_eq!(entry.file_type, deserialized.file_type);
        prop_assert_eq!(entry.icon_key, deserialized.icon_key);
    }

    /// **Feature: file-explorer-core, Property 22: Corrupted Data Rejection**
    /// **Validates: Requirements 10.2**
    ///
    /// *For any* byte sequence that is not a valid serialized format,
    /// deserialization SHALL return an error rather than producing an invalid or partial structure.
    #[test]
    fn prop_corrupted_data_rejection(random_bytes in prop::collection::vec(any::<u8>(), 0..1000)) {
        let result: std::result::Result<FileEntry, _> = bincode::deserialize(&random_bytes);

        match result {
            Ok(entry) => {
                let re_serialized = bincode::serialize(&entry)
                    .expect("Re-serialization should succeed");
                let re_deserialized: FileEntry = bincode::deserialize(&re_serialized)
                    .expect("Re-deserialization should succeed");
                prop_assert_eq!(entry, re_deserialized,
                    "If random bytes happen to deserialize, the result must be consistent");
            }
            Err(_) => {
                // Expected behavior: corrupted data should be rejected
            }
        }
    }

    /// **Feature: ui-enhancements, Property 7: Sort by Name Ordering**
    /// **Validates: Requirements 3.1**
    ///
    /// *For any* list of file entries sorted by name in ascending order,
    /// each entry's name (case-insensitive) SHALL be <= the next entry's name.
    #[test]
    fn prop_sort_by_name_ordering(entries in prop::collection::vec(arb_file_entry(), 0..50)) {
        let mut entries = entries;
        let mut sort_state = SortState::new();
        sort_state.column = SortColumn::Name;
        sort_state.direction = SortDirection::Ascending;
        sort_state.directories_first = false; // Test pure name sorting
        
        sort_state.sort_entries(&mut entries);
        
        for window in entries.windows(2) {
            let a = &window[0];
            let b = &window[1];
            prop_assert!(
                a.name.to_lowercase() <= b.name.to_lowercase(),
                "Name sort failed: '{}' should come before '{}'",
                a.name, b.name
            );
        }
    }

    /// **Feature: ui-enhancements, Property 8: Sort by Date Ordering**
    /// **Validates: Requirements 3.2**
    ///
    /// *For any* list of file entries sorted by date in descending order (newest first),
    /// each entry's modified time SHALL be >= the next entry's modified time.
    #[test]
    fn prop_sort_by_date_ordering(entries in prop::collection::vec(arb_file_entry(), 0..50)) {
        let mut entries = entries;
        let mut sort_state = SortState::new();
        sort_state.column = SortColumn::Date;
        sort_state.direction = SortDirection::Descending;
        sort_state.directories_first = false;
        
        sort_state.sort_entries(&mut entries);
        
        for window in entries.windows(2) {
            let a = &window[0];
            let b = &window[1];
            prop_assert!(
                a.modified >= b.modified,
                "Date sort failed: {:?} should come before {:?}",
                a.modified, b.modified
            );
        }
    }

    /// **Feature: ui-enhancements, Property 9: Sort by Type Ordering**
    /// **Validates: Requirements 3.3**
    ///
    /// *For any* list of file entries sorted by type (extension) in ascending order,
    /// each entry's extension (case-insensitive) SHALL be <= the next entry's extension.
    #[test]
    fn prop_sort_by_type_ordering(entries in prop::collection::vec(arb_file_entry(), 0..50)) {
        let mut entries = entries;
        let mut sort_state = SortState::new();
        sort_state.column = SortColumn::Type;
        sort_state.direction = SortDirection::Ascending;
        sort_state.directories_first = false;
        
        sort_state.sort_entries(&mut entries);
        
        fn get_ext(name: &str) -> String {
            name.rsplit('.').next()
                .filter(|ext| *ext != name)
                .unwrap_or("")
                .to_lowercase()
        }
        
        for window in entries.windows(2) {
            let a = &window[0];
            let b = &window[1];
            let ext_a = get_ext(&a.name);
            let ext_b = get_ext(&b.name);
            prop_assert!(
                ext_a <= ext_b,
                "Type sort failed: extension '{}' should come before '{}'",
                ext_a, ext_b
            );
        }
    }

    /// **Feature: ui-enhancements, Property 10: Sort by Size Ordering**
    /// **Validates: Requirements 3.4**
    ///
    /// *For any* list of file entries sorted by size in descending order (largest first),
    /// each entry's size SHALL be >= the next entry's size.
    #[test]
    fn prop_sort_by_size_ordering(entries in prop::collection::vec(arb_file_entry(), 0..50)) {
        let mut entries = entries;
        let mut sort_state = SortState::new();
        sort_state.column = SortColumn::Size;
        sort_state.direction = SortDirection::Descending;
        sort_state.directories_first = false;
        
        sort_state.sort_entries(&mut entries);
        
        for window in entries.windows(2) {
            let a = &window[0];
            let b = &window[1];
            prop_assert!(
                a.size >= b.size,
                "Size sort failed: {} should come before {}",
                a.size, b.size
            );
        }
    }
}
