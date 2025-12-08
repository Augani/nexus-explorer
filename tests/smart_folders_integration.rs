use file_explorer::models::{
    DateFilter, FileEntry, FileType, IconKey, SearchQuery, SizeFilter,
    SmartFolderManager, TagId, CloudSyncStatus,
};
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

    let json = serde_json::to_string(&manager).unwrap();
    let loaded: SmartFolderManager = serde_json::from_str(&json).unwrap();

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


#[test]
fn test_empty_query_returns_all_visible() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_test_entry("file1.txt", false, 100),
        create_test_entry("file2.txt", false, 200),
        create_test_entry("folder1", true, 0),
    ];

    let query = SearchQuery {
        include_hidden: true,
        ..Default::default()
    };

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), entries.len());
}

#[test]
fn test_directories_only_filter() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_test_entry("file1.txt", false, 100),
        create_test_entry("file2.txt", false, 200),
        create_test_entry("folder1", true, 0),
        create_test_entry("folder2", true, 0),
    ];

    let query = SearchQuery {
        directories_only: true,
        include_hidden: true,
        ..Default::default()
    };

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), 2);
    for entry in &results {
        assert!(entry.is_dir);
    }
}

#[test]
fn test_files_only_filter() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_test_entry("file1.txt", false, 100),
        create_test_entry("file2.txt", false, 200),
        create_test_entry("folder1", true, 0),
    ];

    let query = SearchQuery {
        files_only: true,
        include_hidden: true,
        ..Default::default()
    };

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), 2);
    for entry in &results {
        assert!(!entry.is_dir);
    }
}

#[test]
fn test_size_filter_larger_than() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_test_entry("small.txt", false, 50),
        create_test_entry("medium.txt", false, 150),
        create_test_entry("large.txt", false, 500),
    ];

    let query = SearchQuery {
        size_filter: Some(SizeFilter::LargerThan(100)),
        include_hidden: true,
        ..Default::default()
    };

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), 2);
    for entry in &results {
        assert!(entry.size > 100);
    }
}

#[test]
fn test_size_filter_smaller_than() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_test_entry("small.txt", false, 50),
        create_test_entry("medium.txt", false, 150),
        create_test_entry("large.txt", false, 500),
    ];

    let query = SearchQuery {
        size_filter: Some(SizeFilter::SmallerThan(200)),
        include_hidden: true,
        ..Default::default()
    };

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), 2);
    for entry in &results {
        assert!(entry.size < 200);
    }
}

#[test]
fn test_file_type_filter() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_entry_with_extension("code", "rs", 100),
        create_entry_with_extension("readme", "md", 200),
        create_entry_with_extension("config", "toml", 50),
        create_entry_with_extension("script", "rs", 150),
    ];

    let query = SearchQuery {
        file_types: vec!["rs".to_string()],
        include_hidden: true,
        ..Default::default()
    };

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), 2);
    for entry in &results {
        assert!(entry.name.ends_with(".rs"));
    }
}

#[test]
fn test_text_search_case_insensitive() {
    let manager = SmartFolderManager::new();
    
    let entries = vec![
        create_test_entry("README.md", false, 100),
        create_test_entry("readme.txt", false, 200),
        create_test_entry("other.txt", false, 50),
    ];

    let query = SearchQuery::with_text("readme").include_hidden(true);

    let results = manager.execute_query(&query, &entries, |_| HashSet::new());
    assert_eq!(results.len(), 2);
}

#[test]
fn test_smart_folder_create_and_execute() {
    let mut manager = SmartFolderManager::new();
    
    let query = SearchQuery::with_text("test").include_hidden(true);
    let id = manager.create("Test Folder".to_string(), query).unwrap();
    
    let entries = vec![
        create_test_entry("test_file.txt", false, 100),
        create_test_entry("other.txt", false, 200),
        create_test_entry("test_folder", true, 0),
    ];

    let results = manager.execute(id, &entries, |_| HashSet::new()).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_smart_folder_update() {
    let mut manager = SmartFolderManager::new();
    
    let query = SearchQuery::with_text("old").include_hidden(true);
    let id = manager.create("Test Folder".to_string(), query).unwrap();
    
    let new_query = SearchQuery::with_text("new").include_hidden(true);
    manager.update(id, new_query).unwrap();
    
    let folder = manager.get(id).unwrap();
    assert_eq!(folder.query.text, Some("new".to_string()));
}

#[test]
fn test_smart_folder_delete() {
    let mut manager = SmartFolderManager::new();
    
    let query = SearchQuery::with_text("test").include_hidden(true);
    let id = manager.create("Test Folder".to_string(), query).unwrap();
    
    assert!(manager.get(id).is_some());
    manager.delete(id);
    assert!(manager.get(id).is_none());
}
