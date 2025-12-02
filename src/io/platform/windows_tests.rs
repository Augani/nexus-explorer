use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

/// Generate arbitrary FileReferenceNumber
fn arb_frn() -> impl Strategy<Value = FileReferenceNumber> {
    (0u64..0x0000_FFFF_FFFF_FFFFu64, 0u16..=u16::MAX)
        .prop_map(|(record, seq)| FileReferenceNumber::new(record, seq))
}

/// Generate arbitrary FileNode
fn arb_file_node() -> impl Strategy<Value = FileNode> {
    (
        "[a-zA-Z0-9_.-]{1,50}",  // name
        arb_frn(),               // parent
        any::<bool>(),           // is_directory
        any::<u64>(),            // size
        any::<u64>(),            // created
        any::<u64>(),            // modified
        any::<u32>(),            // attributes
    )
        .prop_map(|(name, parent, is_dir, size, created, modified, attrs)| {
            FileNode::new(name, parent, is_dir, size, created, modified, attrs)
        })
}

/// Generate a valid tree structure for MftIndex testing
/// Ensures unique FRNs by using sequential record numbers
fn arb_mft_tree(max_depth: usize, max_children: usize) -> impl Strategy<Value = Vec<(FileReferenceNumber, FileNode)>> {
    let root_frn = FileReferenceNumber::ROOT;
    
    prop::collection::vec(
        (
            "[a-zA-Z0-9_]{1,20}",  // name (simplified to avoid special chars)
            any::<bool>(),
        ),
        1..max_children * max_depth
    ).prop_map(move |items| {
        let mut result = Vec::new();
        let mut available_parents = vec![root_frn];
        
        // Use strictly sequential record numbers to ensure uniqueness
        for (i, (name, is_dir)) in items.into_iter().enumerate() {
            let frn = FileReferenceNumber::new(100 + i as u64, 1);
            let parent_idx = i % available_parents.len().max(1);
            let parent = available_parents[parent_idx];
            
            let node = FileNode::new(
                name,
                parent,
                is_dir,
                0,
                0,
                0,
                if is_dir { 0x10 } else { 0 },
            );
            
            if is_dir {
                available_parents.push(frn);
            }
            
            result.push((frn, node));
        }
        
        result
    })
}

#[test]
fn test_file_reference_number_components() {
    let frn = FileReferenceNumber::new(12345, 42);
    assert_eq!(frn.record_number(), 12345);
    assert_eq!(frn.sequence_number(), 42);
}

#[test]
fn test_mft_index_basic_operations() {
    let mut index = MftIndex::new(PathBuf::from("C:\\"));
    
    let frn = FileReferenceNumber::new(100, 1);
    let node = FileNode::new(
        "test.txt".to_string(),
        FileReferenceNumber::ROOT,
        false,
        1024,
        0,
        0,
        0,
    );
    
    assert!(index.is_empty());
    index.insert(frn, node.clone());
    assert_eq!(index.len(), 1);
    assert!(index.contains(&frn));
    
    let retrieved = index.get(&frn).unwrap();
    assert_eq!(retrieved.name, "test.txt");
    
    index.remove(&frn);
    assert!(!index.contains(&frn));
}

#[test]
fn test_path_reconstruction_simple() {
    let mut index = MftIndex::new(PathBuf::from("C:"));
    
    // Add root
    index.insert(
        FileReferenceNumber::ROOT,
        FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
    );
    
    // Add a file in root
    let file_frn = FileReferenceNumber::new(100, 1);
    index.insert(
        file_frn,
        FileNode::new("test.txt".to_string(), FileReferenceNumber::ROOT, false, 0, 0, 0, 0),
    );
    
    let path = index.reconstruct_path(&file_frn).unwrap();
    // Path should be volume + filename
    assert!(path.ends_with("test.txt"));
    assert!(path.starts_with("C:"));
}

#[test]
fn test_path_reconstruction_nested() {
    let mut index = MftIndex::new(PathBuf::from("C:"));
    
    // Add root
    index.insert(
        FileReferenceNumber::ROOT,
        FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
    );
    
    // Add folder
    let folder_frn = FileReferenceNumber::new(100, 1);
    index.insert(
        folder_frn,
        FileNode::new("Documents".to_string(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
    );
    
    // Add subfolder
    let subfolder_frn = FileReferenceNumber::new(101, 1);
    index.insert(
        subfolder_frn,
        FileNode::new("Projects".to_string(), folder_frn, true, 0, 0, 0, 0x10),
    );
    
    // Add file in subfolder
    let file_frn = FileReferenceNumber::new(102, 1);
    index.insert(
        file_frn,
        FileNode::new("readme.md".to_string(), subfolder_frn, false, 0, 0, 0, 0),
    );
    
    let path = index.reconstruct_path(&file_frn).unwrap();
    // Verify path components
    let components: Vec<_> = path.components().collect();
    assert!(components.len() >= 4); // C:, Documents, Projects, readme.md
    assert!(path.ends_with("readme.md"));
}

#[test]
fn test_usn_reason_flags() {
    let create = UsnReason::FILE_CREATE;
    assert!(create.is_create());
    assert!(!create.is_delete());
    
    let delete = UsnReason::FILE_DELETE;
    assert!(delete.is_delete());
    assert!(!delete.is_create());
    
    let modify = UsnReason::DATA_OVERWRITE;
    assert!(modify.is_modify());
    
    let rename = UsnReason(UsnReason::RENAME_OLD_NAME.0 | UsnReason::RENAME_NEW_NAME.0);
    assert!(rename.is_rename());
}

#[test]
fn test_usn_apply_create() {
    let mut index = MftIndex::new(PathBuf::from("C:\\"));
    
    // Add root
    index.insert(
        FileReferenceNumber::ROOT,
        FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
    );
    
    let monitor = UsnJournalMonitor::new(PathBuf::from("C:\\"));
    
    let record = UsnRecord {
        frn: FileReferenceNumber::new(200, 1),
        parent_frn: FileReferenceNumber::ROOT,
        usn: 1000,
        reason: UsnReason::FILE_CREATE,
        file_name: "newfile.txt".to_string(),
        file_attributes: 0,
    };
    
    monitor.apply_to_index(&mut index, &[record]);
    
    assert!(index.contains(&FileReferenceNumber::new(200, 1)));
    let node = index.get(&FileReferenceNumber::new(200, 1)).unwrap();
    assert_eq!(node.name, "newfile.txt");
}

#[test]
fn test_usn_apply_delete() {
    let mut index = MftIndex::new(PathBuf::from("C:\\"));
    
    let frn = FileReferenceNumber::new(200, 1);
    index.insert(
        frn,
        FileNode::new("oldfile.txt".to_string(), FileReferenceNumber::ROOT, false, 0, 0, 0, 0),
    );
    
    let monitor = UsnJournalMonitor::new(PathBuf::from("C:\\"));
    
    let record = UsnRecord {
        frn,
        parent_frn: FileReferenceNumber::ROOT,
        usn: 1000,
        reason: UsnReason::FILE_DELETE,
        file_name: "oldfile.txt".to_string(),
        file_attributes: 0,
    };
    
    monitor.apply_to_index(&mut index, &[record]);
    
    assert!(!index.contains(&frn));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // **Feature: file-explorer-core, Property 18: MFT Index Path Reconstruction**
    // **Validates: Requirements 7.1, 7.2**
    // 
    // For any FileReferenceNumber in the MFT index, path reconstruction SHALL produce
    // a valid absolute path by traversing parent references to the root.
    #[test]
    fn prop_mft_path_reconstruction(tree in arb_mft_tree(5, 10)) {
        let volume_root = PathBuf::from("C:");
        let mut index = MftIndex::new(volume_root.clone());
        
        // Add root first
        index.insert(
            FileReferenceNumber::ROOT,
            FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
        );
        
        // Add all nodes from the tree
        for (frn, node) in &tree {
            index.insert(*frn, node.clone());
        }
        
        // Verify all nodes can have their paths reconstructed
        for (frn, node) in &tree {
            let path = index.reconstruct_path(frn);
            
            // Path must be reconstructable
            prop_assert!(path.is_some(), "Failed to reconstruct path for {:?}", frn);
            
            let path = path.unwrap();
            
            // Path must start with volume root
            prop_assert!(
                path.starts_with(&volume_root), 
                "Path doesn't start with volume: {:?}", path
            );
            
            // Path must end with the file's name
            if !node.name.is_empty() {
                let file_name = path.file_name().and_then(|n| n.to_str());
                prop_assert!(
                    file_name == Some(&node.name as &str),
                    "Path doesn't end with file name: {:?} vs {:?}",
                    file_name,
                    node.name
                );
            }
        }
    }

    // **Feature: file-explorer-core, Property 19: MFT Index Event Consistency**
    // **Validates: Requirements 7.3**
    // 
    // For any USN create event for path P, the MFT index SHALL contain an entry for P
    // after processing. For any USN delete event for path P, the index SHALL NOT
    // contain P after processing.
    #[test]
    fn prop_mft_event_consistency(
        creates in prop::collection::vec(
            ("[a-zA-Z0-9_]{1,20}", 100u64..500u64),
            1..20
        ),
        deletes in prop::collection::vec(0usize..20, 0..10)
    ) {
        let mut index = MftIndex::new(PathBuf::from("C:"));
        
        // Add root
        index.insert(
            FileReferenceNumber::ROOT,
            FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
        );
        
        let monitor = UsnJournalMonitor::new(PathBuf::from("C:\\"));
        
        // Apply create events
        let mut created_frns = Vec::new();
        for (i, (name, record_num)) in creates.iter().enumerate() {
            let frn = FileReferenceNumber::new(*record_num + i as u64, 1);
            created_frns.push(frn);
            
            let record = UsnRecord {
                frn,
                parent_frn: FileReferenceNumber::ROOT,
                usn: i as u64 * 100,
                reason: UsnReason::FILE_CREATE,
                file_name: name.clone(),
                file_attributes: 0,
            };
            
            monitor.apply_to_index(&mut index, &[record]);
            
            // After create, entry must exist
            prop_assert!(
                index.contains(&frn),
                "Entry not found after create: {:?}",
                frn
            );
        }
        
        // Apply delete events for some created files
        for delete_idx in deletes {
            if delete_idx < created_frns.len() {
                let frn = created_frns[delete_idx];
                
                let record = UsnRecord {
                    frn,
                    parent_frn: FileReferenceNumber::ROOT,
                    usn: 10000 + delete_idx as u64,
                    reason: UsnReason::FILE_DELETE,
                    file_name: String::new(),
                    file_attributes: 0,
                };
                
                monitor.apply_to_index(&mut index, &[record]);
                
                // After delete, entry must not exist
                prop_assert!(
                    !index.contains(&frn),
                    "Entry still exists after delete: {:?}",
                    frn
                );
            }
        }
    }
}


#[test]
fn test_mft_serialization_basic() {
    let mut index = MftIndex::new(PathBuf::from("C:"));
    
    // Add root
    index.insert(
        FileReferenceNumber::ROOT,
        FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
    );
    
    // Add some files
    index.insert(
        FileReferenceNumber::new(100, 1),
        FileNode::new("test.txt".to_string(), FileReferenceNumber::ROOT, false, 1024, 100, 200, 0),
    );
    
    index.set_usn_cursor(12345);
    
    // Serialize and deserialize
    let serialized = index.serialize().expect("Serialization should succeed");
    let deserialized = MftIndex::deserialize(&serialized).expect("Deserialization should succeed");
    
    // Verify
    assert_eq!(deserialized.len(), index.len());
    assert_eq!(deserialized.usn_cursor, index.usn_cursor);
    assert_eq!(deserialized.volume_path, index.volume_path);
    assert!(deserialized.contains(&FileReferenceNumber::ROOT));
    assert!(deserialized.contains(&FileReferenceNumber::new(100, 1)));
}

#[test]
fn test_mft_corrupted_data_rejection() {
    // Random garbage data should fail to deserialize
    let garbage = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
    let result = MftIndex::deserialize(&garbage);
    assert!(result.is_err(), "Corrupted data should be rejected");
    
    // Truncated data should fail
    let mut index = MftIndex::new(PathBuf::from("C:"));
    index.insert(
        FileReferenceNumber::ROOT,
        FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
    );
    let serialized = index.serialize().unwrap();
    let truncated = &serialized[..serialized.len() / 2];
    let result = MftIndex::deserialize(truncated);
    assert!(result.is_err(), "Truncated data should be rejected");
}

/// Generate arbitrary MftIndex for property testing
fn arb_mft_index() -> impl Strategy<Value = MftIndex> {
    (
        "[A-Z]:",  // volume path
        0u64..u64::MAX,  // usn_cursor
        arb_mft_tree(3, 5),  // tree structure
    ).prop_map(|(volume, cursor, tree)| {
        let mut index = MftIndex::new(PathBuf::from(volume));
        index.set_usn_cursor(cursor);
        
        // Add root
        index.insert(
            FileReferenceNumber::ROOT,
            FileNode::new(String::new(), FileReferenceNumber::ROOT, true, 0, 0, 0, 0x10),
        );
        
        // Add tree nodes
        for (frn, node) in tree {
            index.insert(frn, node);
        }
        
        index
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // **Feature: file-explorer-core, Property 23: MFT Index Serialization Round-Trip**
    // **Validates: Requirements 10.3, 10.4**
    // 
    // For any valid MftIndex, serializing then deserializing SHALL produce an equivalent
    // index with identical file mappings.
    #[test]
    fn prop_mft_serialization_roundtrip(index in arb_mft_index()) {
        // Serialize
        let serialized = index.serialize();
        prop_assert!(serialized.is_ok(), "Serialization should succeed");
        let serialized = serialized.unwrap();
        
        // Deserialize
        let deserialized = MftIndex::deserialize(&serialized);
        prop_assert!(deserialized.is_ok(), "Deserialization should succeed");
        let deserialized = deserialized.unwrap();
        
        // Verify equivalence
        prop_assert_eq!(
            deserialized.len(),
            index.len(),
            "File count should match"
        );
        
        prop_assert_eq!(
            deserialized.usn_cursor,
            index.usn_cursor,
            "USN cursor should match"
        );
        
        prop_assert_eq!(
            &deserialized.volume_path,
            &index.volume_path,
            "Volume path should match"
        );
        
        // Verify all files are present with correct data
        for (frn, original_node) in &index.files {
            let deserialized_node = deserialized.get(frn);
            prop_assert!(
                deserialized_node.is_some(),
                "File {:?} should exist after round-trip",
                frn
            );
            
            let deserialized_node = deserialized_node.unwrap();
            prop_assert_eq!(
                &deserialized_node.name,
                &original_node.name,
                "File name should match for {:?}",
                frn
            );
            prop_assert_eq!(
                deserialized_node.parent.0,
                original_node.parent.0,
                "Parent should match for {:?}",
                frn
            );
            prop_assert_eq!(
                deserialized_node.is_directory,
                original_node.is_directory,
                "is_directory should match for {:?}",
                frn
            );
            prop_assert_eq!(
                deserialized_node.size,
                original_node.size,
                "Size should match for {:?}",
                frn
            );
            prop_assert_eq!(
                deserialized_node.attributes,
                original_node.attributes,
                "Attributes should match for {:?}",
                frn
            );
        }
    }
}
