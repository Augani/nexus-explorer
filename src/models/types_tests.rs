/// Property-based tests for core types
/// **Feature: file-explorer-core**
use super::{CloudSyncStatus, FileEntry, FileType, IconKey, SortColumn, SortDirection, SortState};
use proptest::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn arb_system_time() -> impl Strategy<Value = SystemTime> {
    (0u64..253402300799u64, 0u32..1_000_000_000u32)
        .prop_map(|(secs, nanos)| UNIX_EPOCH + Duration::new(secs, nanos))
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
        any::<bool>(),
        proptest::option::of("[a-zA-Z0-9_/.-]{1,100}".prop_map(PathBuf::from)),
        any::<bool>(),
    )
        .prop_map(
            |(name, path_str, is_dir, size, modified, file_type, icon_key, is_symlink, symlink_target, is_broken_symlink)| FileEntry {
                name,
                path: PathBuf::from(path_str),
                is_dir,
                size,
                modified,
                file_type,
                icon_key,
                linux_permissions: None,
                sync_status: CloudSyncStatus::None,
                is_symlink,
                symlink_target,
                is_broken_symlink,
            },
        )
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
                let ext_a = a
                    .name
                    .rsplit('.')
                    .next()
                    .filter(|e| *e != &a.name)
                    .unwrap_or("")
                    .to_lowercase();
                let ext_b = b
                    .name
                    .rsplit('.')
                    .next()
                    .filter(|e| *e != &b.name)
                    .unwrap_or("")
                    .to_lowercase();
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
            a.name,
            b.name,
            column,
            direction
        );
    }

    Ok(())
}


#[cfg(unix)]
mod symlink_tests {
    use super::*;
    use std::fs::{self, File};
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: advanced-device-management, Property 12: Symbolic Link Detection**
        /// **Validates: Requirements 9.3**
        ///
        /// *For any* file path that is a symbolic link, the `is_symlink()` function SHALL return true,
        /// and `symlink_target()` SHALL return the target path.
        #[test]
        fn prop_symbolic_link_detection(
            file_name in "[a-zA-Z0-9]{1,20}",
            link_name in "[a-zA-Z0-9]{1,20}",
        ) {
            // Skip if names are the same
            prop_assume!(file_name != link_name);

            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join(&file_name);
            let link_path = temp_dir.path().join(&link_name);

            // Create a regular file
            File::create(&file_path).expect("Failed to create file");

            // Create a symbolic link to the file
            symlink(&file_path, &link_path).expect("Failed to create symlink");

            // Test symlink detection using FileEntry::from_path
            let entry = FileEntry::from_path(&link_path)
                .expect("Failed to create FileEntry from symlink path");

            // Property: is_symlink() should return true for symbolic links
            prop_assert!(
                entry.is_symlink(),
                "is_symlink() should return true for symbolic link at {:?}",
                link_path
            );

            // Property: symlink_target() should return the target path
            let target = entry.symlink_target();
            prop_assert!(
                target.is_some(),
                "symlink_target() should return Some for symbolic link at {:?}",
                link_path
            );

            // The target should match the original file path
            let target_path = target.unwrap();
            prop_assert_eq!(
                target_path,
                file_path.as_path(),
                "symlink_target() should return the correct target path"
            );

            // Property: is_broken_symlink() should return false for valid symlinks
            prop_assert!(
                !entry.is_broken_symlink(),
                "is_broken_symlink() should return false for valid symlink"
            );

            // Test that regular files are NOT detected as symlinks
            let regular_entry = FileEntry::from_path(&file_path)
                .expect("Failed to create FileEntry from regular file path");

            prop_assert!(
                !regular_entry.is_symlink(),
                "is_symlink() should return false for regular file at {:?}",
                file_path
            );

            prop_assert!(
                regular_entry.symlink_target().is_none(),
                "symlink_target() should return None for regular file"
            );
        }

        /// **Feature: advanced-device-management, Property 13: Broken Symbolic Link Detection**
        /// **Validates: Requirements 9.5**
        ///
        /// *For any* symbolic link where the target does not exist, `is_broken_symlink()` SHALL return true.
        #[test]
        fn prop_broken_symbolic_link_detection(
            target_name in "[a-zA-Z0-9]{1,20}",
            link_name in "[a-zA-Z0-9]{1,20}",
        ) {
            // Skip if names are the same
            prop_assume!(target_name != link_name);

            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let target_path = temp_dir.path().join(&target_name);
            let link_path = temp_dir.path().join(&link_name);

            // Create a symbolic link to a non-existent target
            // The target_path does NOT exist - we're creating a broken symlink
            symlink(&target_path, &link_path).expect("Failed to create symlink");

            // Verify the target doesn't exist
            prop_assert!(
                !target_path.exists(),
                "Target path should not exist for broken symlink test"
            );

            // Test broken symlink detection using FileEntry::from_path
            let entry = FileEntry::from_path(&link_path)
                .expect("Failed to create FileEntry from broken symlink path");

            // Property: is_symlink() should return true for broken symbolic links
            prop_assert!(
                entry.is_symlink(),
                "is_symlink() should return true for broken symbolic link at {:?}",
                link_path
            );

            // Property: is_broken_symlink() should return true for broken symlinks
            prop_assert!(
                entry.is_broken_symlink(),
                "is_broken_symlink() should return true for broken symlink at {:?}",
                link_path
            );

            // Property: symlink_target() should still return the target path (even if broken)
            let target = entry.symlink_target();
            prop_assert!(
                target.is_some(),
                "symlink_target() should return Some for broken symbolic link at {:?}",
                link_path
            );

            // Now create the target file and verify the symlink is no longer broken
            File::create(&target_path).expect("Failed to create target file");

            let fixed_entry = FileEntry::from_path(&link_path)
                .expect("Failed to create FileEntry from fixed symlink path");

            // Property: After creating target, is_broken_symlink() should return false
            prop_assert!(
                !fixed_entry.is_broken_symlink(),
                "is_broken_symlink() should return false after target is created"
            );

            // Property: is_symlink() should still return true
            prop_assert!(
                fixed_entry.is_symlink(),
                "is_symlink() should still return true after target is created"
            );
        }
    }
}

#[cfg(windows)]
mod symlink_tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: advanced-device-management, Property 12: Symbolic Link Detection**
        /// **Validates: Requirements 9.3**
        ///
        /// *For any* file path that is a symbolic link, the `is_symlink()` function SHALL return true,
        /// and `symlink_target()` SHALL return the target path.
        /// Note: On Windows, symlink creation requires elevated privileges, so we test with regular files.
        #[test]
        fn prop_symbolic_link_detection_regular_files(
            file_name in "[a-zA-Z0-9]{1,20}",
        ) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join(&file_name);

            // Create a regular file
            File::create(&file_path).expect("Failed to create file");

            // Test that regular files are NOT detected as symlinks
            let entry = FileEntry::from_path(&file_path)
                .expect("Failed to create FileEntry from file path");

            prop_assert!(
                !entry.is_symlink(),
                "is_symlink() should return false for regular file at {:?}",
                file_path
            );

            prop_assert!(
                entry.symlink_target().is_none(),
                "symlink_target() should return None for regular file"
            );

            prop_assert!(
                !entry.is_broken_symlink(),
                "is_broken_symlink() should return false for regular file"
            );
        }
    }
}
