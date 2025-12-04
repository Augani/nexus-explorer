/// Property-based tests for core types
/// **Feature: file-explorer-core**

use super::{CloudSyncStatus, FileEntry, FileType, IconKey, SortColumn, SortDirection, SortState};
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
                linux_permissions: None,
                sync_status: CloudSyncStatus::None,
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
        sort_state.directories_first = false;
        
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


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: ui-enhancements, Property 11: Sort Toggle Reversal**
    /// **Validates: Requirements 3.5**
    ///
    /// *For any* SortState with a given column, clicking the same column header twice
    /// SHALL reverse the sort direction (ascending becomes descending, descending becomes ascending).
    #[test]
    fn prop_sort_toggle_reversal(
        column in prop_oneof![
            Just(SortColumn::Name),
            Just(SortColumn::Date),
            Just(SortColumn::Type),
            Just(SortColumn::Size),
        ]
    ) {
        let mut sort_state = SortState::new();
        
        sort_state.toggle_column(column);
        let first_direction = sort_state.direction;
        
        // Second click on same column should reverse direction
        sort_state.toggle_column(column);
        let second_direction = sort_state.direction;
        
        prop_assert_ne!(
            first_direction, second_direction,
            "Clicking same column twice should reverse direction"
        );
        
        sort_state.toggle_column(column);
        let third_direction = sort_state.direction;
        
        prop_assert_eq!(
            first_direction, third_direction,
            "Clicking same column three times should return to original direction"
        );
    }
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: ui-enhancements, Property 12: Directories First Invariant**
    /// **Validates: Requirements 3.7**
    ///
    /// *For any* sorted list with directories_first enabled, all directory entries 
    /// SHALL appear before all file entries.
    #[test]
    fn prop_directories_first_invariant(entries in prop::collection::vec(arb_file_entry(), 0..50)) {
        let mut entries = entries;
        let mut sort_state = SortState::new();
        sort_state.directories_first = true;
        
        sort_state.sort_entries(&mut entries);
        
        // Find the first file (non-directory) index
        let first_file_idx = entries.iter().position(|e| !e.is_dir);
        
        // If there are files, all entries after the first file should also be files
        if let Some(first_file) = first_file_idx {
            for (i, entry) in entries.iter().enumerate().skip(first_file) {
                prop_assert!(
                    !entry.is_dir,
                    "Directory found at index {} after first file at index {}. Entry: {}",
                    i, first_file, entry.name
                );
            }
        }
        
        // All entries before the first file should be directories
        if let Some(first_file) = first_file_idx {
            for (i, entry) in entries.iter().enumerate().take(first_file) {
                prop_assert!(
                    entry.is_dir,
                    "File found at index {} before first file at index {}. Entry: {}",
                    i, first_file, entry.name
                );
            }
        }
    }
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: ui-enhancements, Property 13: Sort Stability on Update**
    /// **Validates: Requirements 3.8**
    ///
    /// *For any* SortState and list of entries, when new entries are added and sorted,
    /// the sort order SHALL be maintained consistently (same column and direction).
    #[test]
    fn prop_sort_stability_on_update(
        initial_count in 1usize..50,
        additional_count in 1usize..20,
        column in prop_oneof![
            Just(SortColumn::Name),
            Just(SortColumn::Date),
            Just(SortColumn::Type),
            Just(SortColumn::Size),
        ],
        ascending in proptest::bool::ANY,
    ) {
        use std::time::{Duration, UNIX_EPOCH};
        
        let mut entries: Vec<FileEntry> = (0..initial_count)
            .map(|i| {
                let name = format!("file_{:04}.txt", i);
                FileEntry::new(
                    name.clone(),
                    std::path::PathBuf::from(format!("/test/{}", name)),
                    false,
                    (i as u64 + 1) * 1000,
                    UNIX_EPOCH + Duration::from_secs(i as u64 * 86400),
                )
            })
            .collect();
        
        let mut sort_state = SortState::new();
        sort_state.column = column;
        sort_state.direction = if ascending { SortDirection::Ascending } else { SortDirection::Descending };
        sort_state.directories_first = false;
        
        sort_state.sort_entries(&mut entries);
        
        verify_sort_order(&entries, column, sort_state.direction)?;
        
        // Add new entries
        let new_entries: Vec<FileEntry> = (0..additional_count)
            .map(|i| {
                let name = format!("new_file_{:04}.txt", i);
                FileEntry::new(
                    name.clone(),
                    std::path::PathBuf::from(format!("/test/{}", name)),
                    false,
                    (i as u64 + 100) * 1000,
                    UNIX_EPOCH + Duration::from_secs((i as u64 + 100) * 86400),
                )
            })
            .collect();
        
        entries.extend(new_entries);
        
        // Re-sort with same sort state
        sort_state.sort_entries(&mut entries);
        
        verify_sort_order(&entries, column, sort_state.direction)?;
    }
}

fn verify_sort_order(
    entries: &[FileEntry],
    column: SortColumn,
    direction: SortDirection,
) -> std::result::Result<(), proptest::test_runner::TestCaseError> {
    use proptest::prop_assert;
    
    for window in entries.windows(2) {
        let a = &window[0];
        let b = &window[1];
        
        let ordering = match column {
            SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortColumn::Date => a.modified.cmp(&b.modified),
            SortColumn::Type => {
                let ext_a = a.name.rsplit('.').next().filter(|e| *e != &a.name).unwrap_or("").to_lowercase();
                let ext_b = b.name.rsplit('.').next().filter(|e| *e != &b.name).unwrap_or("").to_lowercase();
                ext_a.cmp(&ext_b)
            }
            SortColumn::Size => a.size.cmp(&b.size),
        };
        
        let is_valid = match direction {
            SortDirection::Ascending => ordering != std::cmp::Ordering::Greater,
            SortDirection::Descending => ordering != std::cmp::Ordering::Less,
        };
        
        prop_assert!(
            is_valid,
            "Sort order violated: '{}' vs '{}' with {:?} {:?}",
            a.name, b.name, column, direction
        );
    }
    
    Ok(())
}
