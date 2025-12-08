use super::*;
use proptest::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;
use tempfile::TempDir;

/
fn file_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,20}\\.[a-z]{1,4}".prop_map(|s| s)
}

/
fn dir_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
}

/
fn create_test_file(dir: &std::path::Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

/
fn create_test_dir(parent: &std::path::Path, name: &str) -> PathBuf {
    let path = parent.join(name);
    fs::create_dir_all(&path).unwrap();
    path
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_trash_entry_metadata_non_empty(
        name in file_name_strategy(),
        size in 0u64..10000,
    ) {
        let original_path = PathBuf::from("/home/user/documents").join(&name);
        let deletion_date = SystemTime::now();
        
        let entry = TrashEntry {
            name: name.clone(),
            original_path: original_path.clone(),
            deletion_date,
            size,
            is_dir: false,
            trash_id: TrashId::Path(PathBuf::from("/trash").join(&name)),
        };
        
        
        prop_assert!(
            !entry.original_path.as_os_str().is_empty(),
            "Original path should not be empty"
        );
        
        prop_assert!(
            !entry.name.is_empty(),
            "Name should not be empty"
        );
        
        prop_assert!(
            entry.deletion_date != std::time::UNIX_EPOCH,
            "Deletion date should not be UNIX_EPOCH"
        );
        
        prop_assert!(
            entry.original_path.file_name().is_some(),
            "Original path should have a file name component"
        );
    }
    
    #[test]
    fn prop_trash_entry_directory_metadata(
        name in dir_name_strategy(),
        file_count in 1usize..5,
    ) {
        let original_path = PathBuf::from("/home/user/documents").join(&name);
        let deletion_date = SystemTime::now();
        
        let entry = TrashEntry {
            name: name.clone(),
            original_path: original_path.clone(),
            deletion_date,
            size: file_count as u64 * 1000,
            is_dir: true,
            trash_id: TrashId::Path(PathBuf::from("/trash").join(&name)),
        };
        
        prop_assert!(
            !entry.original_path.as_os_str().is_empty(),
            "Directory original path should not be empty"
        );
        
        prop_assert!(
            !entry.name.is_empty(),
            "Directory name should not be empty"
        );
        
        prop_assert!(
            entry.is_dir,
            "Entry should be marked as directory"
        );
        
        prop_assert!(
            entry.deletion_date != std::time::UNIX_EPOCH,
            "Directory deletion date should not be UNIX_EPOCH"
        );
    }
}

#[test]
fn test_trash_manager_new() {
    let manager = TrashManager::new();
    assert_eq!(manager.item_count(), 0);
    assert_eq!(manager.total_size(), 0);
    assert!(!manager.is_large());
}

#[test]
fn test_trash_entry_creation() {
    let entry = TrashEntry {
        name: "test.txt".to_string(),
        original_path: PathBuf::from("/home/user/test.txt"),
        deletion_date: SystemTime::now(),
        size: 1024,
        is_dir: false,
        trash_id: TrashId::Path(PathBuf::from("/trash/test.txt")),
    };
    
    assert_eq!(entry.name, "test.txt");
    assert_eq!(entry.original_path, PathBuf::from("/home/user/test.txt"));
    assert_eq!(entry.size, 1024);
    assert!(!entry.is_dir);
}

#[test]
fn test_trash_entry_directory() {
    let entry = TrashEntry {
        name: "my_folder".to_string(),
        original_path: PathBuf::from("/home/user/my_folder"),
        deletion_date: SystemTime::now(),
        size: 5000,
        is_dir: true,
        trash_id: TrashId::Path(PathBuf::from("/trash/my_folder")),
    };
    
    assert_eq!(entry.name, "my_folder");
    assert!(entry.is_dir);
    assert_eq!(entry.size, 5000);
}

#[test]
fn test_trash_manager_is_large() {
    let manager = TrashManager::new();
    
    assert!(!manager.is_large());
    
}

#[test]
fn test_trash_error_display() {
    let not_found = TrashError::NotFound("test.txt".to_string());
    assert!(not_found.to_string().contains("not found"));
    
    let missing = TrashError::OriginalLocationMissing(PathBuf::from("/home/user/deleted"));
    assert!(missing.to_string().contains("Original location missing"));
    
    let permission = TrashError::PermissionDenied("Access denied".to_string());
    assert!(permission.to_string().contains("Permission denied"));
    
    let io_error = TrashError::IoError("Read failed".to_string());
    assert!(io_error.to_string().contains("IO error"));
    
    let platform = TrashError::PlatformError("Not supported".to_string());
    assert!(platform.to_string().contains("Platform error"));
}

#[test]
fn test_get_trash_path() {
    let path = get_trash_path();
    
    assert!(!path.as_os_str().is_empty());
    
    #[cfg(target_os = "macos")]
    {
        assert!(path.to_string_lossy().contains(".Trash"));
    }
    
    #[cfg(target_os = "linux")]
    {
        assert!(path.to_string_lossy().contains("Trash"));
    }
    
    #[cfg(target_os = "windows")]
    {
        assert!(path.to_string_lossy().contains("Recycle"));
    }
}

#[test]
fn test_is_trash_path() {
    let trash_path = get_trash_path();
    assert!(is_trash_path(&trash_path));
    
    let non_trash = PathBuf::from("/home/user/documents");
    assert!(!is_trash_path(&non_trash));
}

#[test]
fn test_calculate_dir_size() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = create_test_dir(temp_dir.path(), "test_folder");
    
    create_test_file(&test_dir, "file1.txt", b"Hello");
    create_test_file(&test_dir, "file2.txt", b"World!");
    
    let size = calculate_dir_size(&test_dir);
    
    assert!(size >= 11, "Directory size should be at least 11 bytes, got {}", size);
}

#[test]
fn test_calculate_dir_size_nested() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = create_test_dir(temp_dir.path(), "parent");
    let sub_dir = create_test_dir(&test_dir, "child");
    
    create_test_file(&test_dir, "parent_file.txt", b"Parent content");
    create_test_file(&sub_dir, "child_file.txt", b"Child content");
    
    let size = calculate_dir_size(&test_dir);
    
    assert!(size >= 27, "Directory size should include nested files, got {}", size);
}

#[test]
fn test_calculate_dir_size_empty() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = create_test_dir(temp_dir.path(), "empty");
    
    let size = calculate_dir_size(&empty_dir);
    assert_eq!(size, 0, "Empty directory should have size 0");
}

#[test]
fn test_trash_id_path_variant() {
    let path = PathBuf::from("/trash/test.txt");
    let trash_id = TrashId::Path(path.clone());
    
    match trash_id {
        TrashId::Path(p) => assert_eq!(p, path),
        #[cfg(target_os = "windows")]
        TrashId::Windows(_) => panic!("Expected Path variant"),
    }
}

#[cfg(target_os = "windows")]
#[test]
fn test_trash_id_windows_variant() {
    let id = "S-1-5-21-123456789-987654321-111111111-1001\\$RECYCLE.BIN\\test.txt".to_string();
    let trash_id = TrashId::Windows(id.clone());
    
    match trash_id {
        TrashId::Windows(i) => assert_eq!(i, id),
        TrashId::Path(_) => panic!("Expected Windows variant"),
    }
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_trash_restore_location(
        file_name in file_name_strategy(),
        content_size in 10usize..1000,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = temp_dir.path().join("original");
        let trash_dir = temp_dir.path().join("trash");
        fs::create_dir_all(&original_dir).unwrap();
        fs::create_dir_all(&trash_dir).unwrap();
        
        let original_path = original_dir.join(&file_name);
        
        let content: Vec<u8> = vec![b'x'; content_size];
        let trash_path = trash_dir.join(&file_name);
        let mut file = File::create(&trash_path).unwrap();
        file.write_all(&content).unwrap();
        drop(file);
        
        let entry = TrashEntry {
            name: file_name.clone(),
            original_path: original_path.clone(),
            deletion_date: SystemTime::now(),
            size: content_size as u64,
            is_dir: false,
            trash_id: TrashId::Path(trash_path.clone()),
        };
        
        let result = restore_from_trash(&entry);
        
        prop_assert!(
            result.is_ok(),
            "Restore should succeed, got error: {:?}",
            result.err()
        );
        
        let restored_path = result.unwrap();
        
        prop_assert_eq!(
            &restored_path,
            &original_path,
            "Restored path should equal original path"
        );
        
        prop_assert!(
            restored_path.exists(),
            "File should exist at original path after restore"
        );
        
        prop_assert!(
            !trash_path.exists(),
            "File should no longer exist in trash after restore"
        );
        
        let restored_content = fs::read(&restored_path).unwrap();
        prop_assert_eq!(
            restored_content.len(),
            content_size,
            "Restored file should have same size"
        );
    }
    
    #[test]
    fn prop_trash_restore_creates_missing_parent(
        dir_name in dir_name_strategy(),
        file_name in file_name_strategy(),
        content_size in 10usize..500,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let trash_dir = temp_dir.path().join("trash");
        fs::create_dir_all(&trash_dir).unwrap();
        
        let missing_parent = temp_dir.path().join("missing_parent").join(&dir_name);
        let original_path = missing_parent.join(&file_name);
        
        prop_assert!(
            !missing_parent.exists(),
            "Parent directory should not exist initially"
        );
        
        let content: Vec<u8> = vec![b'y'; content_size];
        let trash_path = trash_dir.join(&file_name);
        let mut file = File::create(&trash_path).unwrap();
        file.write_all(&content).unwrap();
        drop(file);
        
        let entry = TrashEntry {
            name: file_name.clone(),
            original_path: original_path.clone(),
            deletion_date: SystemTime::now(),
            size: content_size as u64,
            is_dir: false,
            trash_id: TrashId::Path(trash_path.clone()),
        };
        
        let result = restore_from_trash(&entry);
        
        prop_assert!(
            result.is_ok(),
            "Restore should succeed even with missing parent, got error: {:?}",
            result.err()
        );
        
        prop_assert!(
            missing_parent.exists(),
            "Missing parent directory should be created"
        );
        
        prop_assert!(
            original_path.exists(),
            "File should exist at original path"
        );
    }
}

#[test]
fn test_restore_from_trash_basic() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = temp_dir.path().join("original");
    let trash_dir = temp_dir.path().join("trash");
    fs::create_dir_all(&original_dir).unwrap();
    fs::create_dir_all(&trash_dir).unwrap();
    
    let trash_path = trash_dir.join("test.txt");
    fs::write(&trash_path, b"Test content").unwrap();
    
    let original_path = original_dir.join("test.txt");
    
    let entry = TrashEntry {
        name: "test.txt".to_string(),
        original_path: original_path.clone(),
        deletion_date: SystemTime::now(),
        size: 12,
        is_dir: false,
        trash_id: TrashId::Path(trash_path.clone()),
    };
    
    let result = restore_from_trash(&entry);
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), original_path);
    assert!(original_path.exists());
    assert!(!trash_path.exists());
    assert_eq!(fs::read_to_string(&original_path).unwrap(), "Test content");
}

#[test]
fn test_restore_from_trash_creates_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let trash_dir = temp_dir.path().join("trash");
    fs::create_dir_all(&trash_dir).unwrap();
    
    let trash_path = trash_dir.join("test.txt");
    fs::write(&trash_path, b"Content").unwrap();
    
    let original_path = temp_dir.path().join("new_dir").join("subdir").join("test.txt");
    
    let entry = TrashEntry {
        name: "test.txt".to_string(),
        original_path: original_path.clone(),
        deletion_date: SystemTime::now(),
        size: 7,
        is_dir: false,
        trash_id: TrashId::Path(trash_path.clone()),
    };
    
    let result = restore_from_trash(&entry);
    
    assert!(result.is_ok());
    assert!(original_path.exists());
    assert!(original_path.parent().unwrap().exists());
}

#[test]
fn test_restore_from_trash_fails_if_exists() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = temp_dir.path().join("original");
    let trash_dir = temp_dir.path().join("trash");
    fs::create_dir_all(&original_dir).unwrap();
    fs::create_dir_all(&trash_dir).unwrap();
    
    let trash_path = trash_dir.join("test.txt");
    fs::write(&trash_path, b"Trash content").unwrap();
    
    let original_path = original_dir.join("test.txt");
    fs::write(&original_path, b"Existing content").unwrap();
    
    let entry = TrashEntry {
        name: "test.txt".to_string(),
        original_path: original_path.clone(),
        deletion_date: SystemTime::now(),
        size: 13,
        is_dir: false,
        trash_id: TrashId::Path(trash_path.clone()),
    };
    
    let result = restore_from_trash(&entry);
    
    assert!(result.is_err());
    match result {
        Err(TrashError::IoError(msg)) => {
            assert!(msg.contains("already exists"));
        }
        _ => panic!("Expected IoError with 'already exists' message"),
    }
    
    assert_eq!(fs::read_to_string(&original_path).unwrap(), "Existing content");
    assert!(trash_path.exists());
}

#[test]
fn test_restore_directory_from_trash() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = temp_dir.path().join("original");
    let trash_dir = temp_dir.path().join("trash");
    fs::create_dir_all(&original_dir).unwrap();
    fs::create_dir_all(&trash_dir).unwrap();
    
    let trash_folder = trash_dir.join("my_folder");
    fs::create_dir_all(&trash_folder).unwrap();
    fs::write(trash_folder.join("file1.txt"), b"File 1").unwrap();
    fs::write(trash_folder.join("file2.txt"), b"File 2").unwrap();
    
    let original_path = original_dir.join("my_folder");
    
    let entry = TrashEntry {
        name: "my_folder".to_string(),
        original_path: original_path.clone(),
        deletion_date: SystemTime::now(),
        size: 12,
        is_dir: true,
        trash_id: TrashId::Path(trash_folder.clone()),
    };
    
    let result = restore_from_trash(&entry);
    
    assert!(result.is_ok());
    assert!(original_path.exists());
    assert!(original_path.is_dir());
    assert!(original_path.join("file1.txt").exists());
    assert!(original_path.join("file2.txt").exists());
    assert!(!trash_folder.exists());
}
