/
/
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
                is_shared: false,
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /
    /
    /
    /
    /
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

    /
    /
    /
    /
    /
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
            }
        }
    }

    /
    /
    /
    /
    /
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

    /
    /
    /
    /
    /
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

    /
    /
    /
    /
    /
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

    /
    /
    /
    /
    /
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

    /
    /
    /
    /
    /
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

    /
    /
    /
    /
    /
    #[test]
    fn prop_directories_first_invariant(entries in prop::collection::vec(arb_file_entry(), 0..50)) {
        let mut entries = entries;
        let mut sort_state = SortState::new();
        sort_state.directories_first = true;

        sort_state.sort_entries(&mut entries);

        let first_file_idx = entries.iter().position(|e| !e.is_dir);

        if let Some(first_file) = first_file_idx {
            for (i, entry) in entries.iter().enumerate().skip(first_file) {
                prop_assert!(
                    !entry.is_dir,
                    "Directory found at index {} after first file at index {}. Entry: {}",
                    i, first_file, entry.name
                );
            }
        }

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

    /
    /
    /
    /
    /
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

        /
        /
        /
        /
        /
        #[test]
        fn prop_symbolic_link_detection(
            file_name in "[a-zA-Z0-9]{1,20}",
            link_name in "[a-zA-Z0-9]{1,20}",
        ) {
            prop_assume!(file_name != link_name);

            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join(&file_name);
            let link_path = temp_dir.path().join(&link_name);

            File::create(&file_path).expect("Failed to create file");

            symlink(&file_path, &link_path).expect("Failed to create symlink");

            let entry = FileEntry::from_path(&link_path)
                .expect("Failed to create FileEntry from symlink path");

            prop_assert!(
                entry.is_symlink(),
                "is_symlink() should return true for symbolic link at {:?}",
                link_path
            );

            let target = entry.symlink_target();
            prop_assert!(
                target.is_some(),
                "symlink_target() should return Some for symbolic link at {:?}",
                link_path
            );

            let target_path = target.unwrap();
            prop_assert_eq!(
                target_path,
                file_path.as_path(),
                "symlink_target() should return the correct target path"
            );

            prop_assert!(
                !entry.is_broken_symlink(),
                "is_broken_symlink() should return false for valid symlink"
            );

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

        /
        /
        /
        /
        #[test]
        fn prop_broken_symbolic_link_detection(
            target_name in "[a-zA-Z0-9]{1,20}",
            link_name in "[a-zA-Z0-9]{1,20}",
        ) {
            prop_assume!(target_name != link_name);

            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let target_path = temp_dir.path().join(&target_name);
            let link_path = temp_dir.path().join(&link_name);

            symlink(&target_path, &link_path).expect("Failed to create symlink");

            prop_assert!(
                !target_path.exists(),
                "Target path should not exist for broken symlink test"
            );

            let entry = FileEntry::from_path(&link_path)
                .expect("Failed to create FileEntry from broken symlink path");

            prop_assert!(
                entry.is_symlink(),
                "is_symlink() should return true for broken symbolic link at {:?}",
                link_path
            );

            prop_assert!(
                entry.is_broken_symlink(),
                "is_broken_symlink() should return true for broken symlink at {:?}",
                link_path
            );

            let target = entry.symlink_target();
            prop_assert!(
                target.is_some(),
                "symlink_target() should return Some for broken symbolic link at {:?}",
                link_path
            );

            File::create(&target_path).expect("Failed to create target file");

            let fixed_entry = FileEntry::from_path(&link_path)
                .expect("Failed to create FileEntry from fixed symlink path");

            prop_assert!(
                !fixed_entry.is_broken_symlink(),
                "is_broken_symlink() should return false after target is created"
            );

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

        /
        /
        /
        /
        /
        /
        #[test]
        fn prop_symbolic_link_detection_regular_files(
            file_name in "[a-zA-Z0-9]{1,20}",
        ) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join(&file_name);

            File::create(&file_path).expect("Failed to create file");

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
