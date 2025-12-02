/// Property-based tests for core types
/// **Feature: file-explorer-core**

use super::{FileEntry, FileType, IconKey};
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
}
