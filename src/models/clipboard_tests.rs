use super::*;
use proptest::prelude::*;
use std::path::PathBuf;


fn path_strategy() -> impl Strategy<Value = PathBuf> {
    "[a-z]{1,10}(/[a-z]{1,10}){0,3}\\.[a-z]{1,4}"
        .prop_map(|s| PathBuf::from(format!("/tmp/{}", s)))
}


fn paths_strategy(min: usize, max: usize) -> impl Strategy<Value = Vec<PathBuf>> {
    prop::collection::vec(path_strategy(), min..max)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_clipboard_copy_state(paths in paths_strategy(1, 20)) {
        let mut manager = ClipboardManager::new();
        
        manager.copy(paths.clone());
        
        let clipboard_paths = manager.paths();
        prop_assert!(clipboard_paths.is_some(), "Clipboard should have paths after copy");
        
        let clipboard_paths = clipboard_paths.unwrap();
        prop_assert_eq!(
            clipboard_paths.len(),
            paths.len(),
            "Clipboard should contain exactly {} paths, got {}",
            paths.len(),
            clipboard_paths.len()
        );
        
        for (i, path) in paths.iter().enumerate() {
            prop_assert_eq!(
                &clipboard_paths[i],
                path,
                "Path at index {} should match: expected {:?}, got {:?}",
                i,
                path,
                clipboard_paths[i]
            );
        }
        
        prop_assert!(
            manager.has_content(),
            "has_content() should return true after copy"
        );
        
        prop_assert!(
            manager.is_copy(),
            "Operation should be copy"
        );
        prop_assert!(
            !manager.is_cut(),
            "Operation should not be cut"
        );
    }
    
    #[test]
    fn prop_clipboard_cut_state(paths in paths_strategy(1, 20)) {
        let mut manager = ClipboardManager::new();
        
        manager.cut(paths.clone());
        
        let clipboard_paths = manager.paths();
        prop_assert!(clipboard_paths.is_some(), "Clipboard should have paths after cut");
        
        let clipboard_paths = clipboard_paths.unwrap();
        prop_assert_eq!(
            clipboard_paths.len(),
            paths.len(),
            "Clipboard should contain exactly {} paths, got {}",
            paths.len(),
            clipboard_paths.len()
        );
        
        prop_assert!(
            manager.has_content(),
            "has_content() should return true after cut"
        );
        
        prop_assert!(
            manager.is_cut(),
            "Operation should be cut"
        );
        prop_assert!(
            !manager.is_copy(),
            "Operation should not be copy"
        );
    }
    
    #[test]
    fn prop_clipboard_contains_path(paths in paths_strategy(1, 10)) {
        let mut manager = ClipboardManager::new();
        manager.copy(paths.clone());
        
        for path in &paths {
            prop_assert!(
                manager.contains_path(path),
                "Clipboard should contain path {:?}",
                path
            );
        }
        
        let non_existent = PathBuf::from("/non/existent/path/that/was/not/copied.txt");
        prop_assert!(
            !manager.contains_path(&non_existent),
            "Clipboard should not contain non-copied path"
        );
    }
    
    #[test]
    fn prop_clipboard_is_path_cut(paths in paths_strategy(1, 10)) {
        let mut manager = ClipboardManager::new();
        manager.cut(paths.clone());
        
        for path in &paths {
            prop_assert!(
                manager.is_path_cut(path),
                "Path {:?} should be marked as cut",
                path
            );
        }
        
        let non_existent = PathBuf::from("/non/existent/path.txt");
        prop_assert!(
            !manager.is_path_cut(&non_existent),
            "Non-cut path should not be marked as cut"
        );
    }
}

#[test]
fn test_clipboard_new() {
    let manager = ClipboardManager::new();
    assert!(!manager.has_content());
    assert!(manager.paths().is_none());
    assert!(!manager.is_cut());
    assert!(!manager.is_copy());
    assert_eq!(manager.item_count(), 0);
}

#[test]
fn test_clipboard_copy() {
    let mut manager = ClipboardManager::new();
    let paths = vec![
        PathBuf::from("/home/user/file1.txt"),
        PathBuf::from("/home/user/file2.txt"),
    ];
    
    manager.copy(paths.clone());
    
    assert!(manager.has_content());
    assert!(manager.is_copy());
    assert!(!manager.is_cut());
    assert_eq!(manager.item_count(), 2);
    assert_eq!(manager.paths().unwrap(), &paths);
}

#[test]
fn test_clipboard_cut() {
    let mut manager = ClipboardManager::new();
    let paths = vec![PathBuf::from("/home/user/file.txt")];
    
    manager.cut(paths.clone());
    
    assert!(manager.has_content());
    assert!(manager.is_cut());
    assert!(!manager.is_copy());
    assert_eq!(manager.item_count(), 1);
}

#[test]
fn test_clipboard_clear() {
    let mut manager = ClipboardManager::new();
    let paths = vec![PathBuf::from("/home/user/file.txt")];
    
    manager.copy(paths);
    assert!(manager.has_content());
    
    manager.clear();
    assert!(!manager.has_content());
    assert_eq!(manager.item_count(), 0);
    
    assert_eq!(manager.history().len(), 1);
}

#[test]
fn test_clipboard_history() {
    let mut manager = ClipboardManager::new();
    
    manager.copy(vec![PathBuf::from("/file1.txt")]);
    
    manager.copy(vec![PathBuf::from("/file2.txt")]);
    
    assert_eq!(manager.history().len(), 1);
    
    manager.copy(vec![PathBuf::from("/file3.txt")]);
    
    assert_eq!(manager.history().len(), 2);
}

#[test]
fn test_clipboard_contains_path() {
    let mut manager = ClipboardManager::new();
    let path1 = PathBuf::from("/home/user/file1.txt");
    let path2 = PathBuf::from("/home/user/file2.txt");
    let path3 = PathBuf::from("/home/user/file3.txt");
    
    manager.copy(vec![path1.clone(), path2.clone()]);
    
    assert!(manager.contains_path(&path1));
    assert!(manager.contains_path(&path2));
    assert!(!manager.contains_path(&path3));
}

#[test]
fn test_clipboard_is_path_cut() {
    let mut manager = ClipboardManager::new();
    let path1 = PathBuf::from("/home/user/file1.txt");
    let path2 = PathBuf::from("/home/user/file2.txt");
    
    manager.copy(vec![path1.clone()]);
    assert!(!manager.is_path_cut(&path1));
    
    manager.cut(vec![path2.clone()]);
    assert!(manager.is_path_cut(&path2));
    assert!(!manager.is_path_cut(&path1));
}

#[test]
fn test_paste_cancellation_token() {
    let token = PasteCancellationToken::new();
    assert!(!token.is_cancelled());
    
    token.cancel();
    assert!(token.is_cancelled());
}

#[test]
fn test_clipboard_paste_lifecycle() {
    let mut manager = ClipboardManager::new();
    let paths = vec![PathBuf::from("/home/user/file.txt")];
    
    manager.cut(paths);
    assert!(manager.has_content());
    
    let token = manager.start_paste();
    assert!(manager.is_paste_active());
    assert!(!token.is_cancelled());
    
    manager.complete_paste(true);
    assert!(!manager.is_paste_active());
    assert!(!manager.has_content());
}

#[test]
fn test_clipboard_paste_copy_preserves_content() {
    let mut manager = ClipboardManager::new();
    let paths = vec![PathBuf::from("/home/user/file.txt")];
    
    manager.copy(paths.clone());
    
    let _token = manager.start_paste();
    manager.complete_paste(false);
    
    assert!(manager.has_content());
    assert_eq!(manager.paths().unwrap(), &paths);
}

#[test]
fn test_paste_progress() {
    let progress = PasteProgress::new(10, 1000);
    
    assert_eq!(progress.total_files, 10);
    assert_eq!(progress.total_bytes, 1000);
    assert_eq!(progress.completed_files, 0);
    assert_eq!(progress.bytes_transferred, 0);
    assert_eq!(progress.percentage(), 0.0);
}

#[test]
fn test_paste_progress_percentage() {
    let mut progress = PasteProgress::new(10, 1000);
    
    progress.bytes_transferred = 500;
    assert_eq!(progress.percentage(), 50.0);
    
    progress.bytes_transferred = 1000;
    assert_eq!(progress.percentage(), 100.0);
}

#[test]
fn test_paste_progress_percentage_zero_bytes() {
    let mut progress = PasteProgress::new(10, 0);
    
    progress.completed_files = 5;
    assert_eq!(progress.percentage(), 50.0);
}

#[test]
fn test_paste_result() {
    let mut result = PasteResult::new();
    
    assert!(result.is_success());
    assert_eq!(result.total_processed(), 0);
    
    result.successful_files.push(PathBuf::from("/file1.txt"));
    result.skipped_files.push(PathBuf::from("/file2.txt"));
    
    assert!(result.is_success());
    assert_eq!(result.total_processed(), 2);
    
    result.failed_files.push((PathBuf::from("/file3.txt"), "Error".to_string()));
    
    assert!(!result.is_success());
    assert_eq!(result.total_processed(), 3);
}

#[test]
fn test_conflict_resolution_variants() {
    let _skip = ConflictResolution::Skip;
    let _replace = ConflictResolution::Replace;
    let _keep_both = ConflictResolution::KeepBoth;
    let _replace_newer = ConflictResolution::ReplaceIfNewer;
    let _replace_larger = ConflictResolution::ReplaceIfLarger;
}

#[test]
fn test_clipboard_operation_paths() {
    let paths = vec![
        PathBuf::from("/file1.txt"),
        PathBuf::from("/file2.txt"),
    ];
    
    let copy_op = ClipboardOperation::Copy { paths: paths.clone() };
    assert_eq!(copy_op.paths(), &paths);
    assert!(copy_op.is_copy());
    assert!(!copy_op.is_cut());
    
    let cut_op = ClipboardOperation::Cut { paths: paths.clone() };
    assert_eq!(cut_op.paths(), &paths);
    assert!(cut_op.is_cut());
    assert!(!cut_op.is_copy());
}


#[test]
fn test_paste_cancellation_token_clone() {
    let token = PasteCancellationToken::new();
    let token_clone = token.clone();
    
    assert!(!token.is_cancelled());
    assert!(!token_clone.is_cancelled());
    
    token.cancel();
    
    assert!(token.is_cancelled());
    assert!(token_clone.is_cancelled());
}

#[test]
fn test_clipboard_cancel_paste() {
    let mut manager = ClipboardManager::new();
    let paths = vec![PathBuf::from("/home/user/file.txt")];
    
    manager.copy(paths);
    let token = manager.start_paste();
    
    assert!(manager.is_paste_active());
    assert!(!token.is_cancelled());
    
    manager.cancel_paste();
    
    assert!(token.is_cancelled());
}

#[test]
fn test_paste_progress_update_variants() {
    let _started = PasteProgressUpdate::Started {
        total_files: 10,
        total_bytes: 1000,
    };
    
    let _file_started = PasteProgressUpdate::FileStarted {
        file: PathBuf::from("/test.txt"),
        file_size: 100,
    };
    
    let _bytes = PasteProgressUpdate::BytesTransferred { bytes: 50 };
    
    let _completed = PasteProgressUpdate::FileCompleted {
        file: PathBuf::from("/test.txt"),
    };
    
    let _skipped = PasteProgressUpdate::FileSkipped {
        file: PathBuf::from("/test.txt"),
        reason: "Already exists".to_string(),
    };
    
    let _failed = PasteProgressUpdate::FileFailed {
        file: PathBuf::from("/test.txt"),
        error: "Permission denied".to_string(),
    };
    
    let _conflict = PasteProgressUpdate::ConflictDetected {
        source: PathBuf::from("/src.txt"),
        destination: PathBuf::from("/dst.txt"),
    };
}


use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;


fn create_test_file(dir: &std::path::Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_paste_cancellation_cleanup(
        file_count in 1usize..5,
        file_sizes in prop::collection::vec(100usize..1000, 1..5),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        
        let actual_count = file_count.min(file_sizes.len());
        let mut source_files = Vec::new();
        
        for i in 0..actual_count {
            let name = format!("file_{}.txt", i);
            let content: Vec<u8> = vec![b'x'; file_sizes[i]];
            let path = create_test_file(&source_dir, &name, &content);
            source_files.push(path);
        }
        
        let token = PasteCancellationToken::new();
        token.cancel();
        
        let (tx, _rx) = flume::unbounded();
        let executor = PasteExecutor::new(token, tx);
        
        let result = executor.execute(
            &source_files,
            &dest_dir,
            false,
            |_, _| ConflictResolution::Replace,
        );
        
        let dest_entries: Vec<_> = fs::read_dir(&dest_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        
        for entry in &dest_entries {
            let dest_path = entry.path();
            if dest_path.is_file() {
                let file_name = dest_path.file_name().unwrap();
                let source_path = source_dir.join(file_name);
                
                if source_path.exists() {
                    let source_size = source_path.metadata().unwrap().len();
                    let dest_size = dest_path.metadata().unwrap().len();
                    
                    prop_assert!(
                        dest_size == source_size || dest_size == 0,
                        "File {:?} should be fully copied ({} bytes) or not present, but has {} bytes",
                        dest_path,
                        source_size,
                        dest_size
                    );
                }
            }
        }
        
        match result {
            Ok(paste_result) => {
                let total = paste_result.successful_files.len() 
                    + paste_result.skipped_files.len() 
                    + paste_result.failed_files.len();
                prop_assert!(
                    total <= actual_count,
                    "Total processed files should not exceed source count"
                );
            }
            Err(e) => {
                prop_assert!(
                    e.contains("cancelled") || e.contains("error"),
                    "Error should indicate cancellation: {}",
                    e
                );
            }
        }
    }
}

#[test]
fn test_paste_executor_basic() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    let source_file = create_test_file(&source_dir, "test.txt", b"Hello, World!");
    
    let token = PasteCancellationToken::new();
    let (tx, rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[source_file.clone()],
        &dest_dir,
        false,
        |_, _| ConflictResolution::Replace,
    );
    
    assert!(result.is_ok());
    let paste_result = result.unwrap();
    assert_eq!(paste_result.successful_files.len(), 1);
    assert!(paste_result.failed_files.is_empty());
    
    let dest_file = dest_dir.join("test.txt");
    assert!(dest_file.exists());
    assert_eq!(fs::read_to_string(&dest_file).unwrap(), "Hello, World!");
    
    let mut updates = Vec::new();
    while let Ok(update) = rx.try_recv() {
        updates.push(update);
    }
    assert!(!updates.is_empty());
}

#[test]
fn test_paste_executor_with_cancellation() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    let source_file = create_test_file(&source_dir, "test.txt", b"Hello, World!");
    
    let token = PasteCancellationToken::new();
    token.cancel();
    
    let (tx, _rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[source_file],
        &dest_dir,
        false,
        |_, _| ConflictResolution::Replace,
    );
    
    assert!(result.is_ok());
    let paste_result = result.unwrap();
    
    let dest_file = dest_dir.join("test.txt");
    assert!(!dest_file.exists() || paste_result.successful_files.is_empty());
}

#[test]
fn test_paste_executor_conflict_skip() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    let source_file = create_test_file(&source_dir, "test.txt", b"New content");
    
    create_test_file(&dest_dir, "test.txt", b"Existing content");
    
    let token = PasteCancellationToken::new();
    let (tx, _rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[source_file],
        &dest_dir,
        false,
        |_, _| ConflictResolution::Skip,
    );
    
    assert!(result.is_ok());
    let paste_result = result.unwrap();
    assert_eq!(paste_result.skipped_files.len(), 1);
    
    let dest_file = dest_dir.join("test.txt");
    assert_eq!(fs::read_to_string(&dest_file).unwrap(), "Existing content");
}

#[test]
fn test_paste_executor_conflict_replace() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    let source_file = create_test_file(&source_dir, "test.txt", b"New content");
    
    create_test_file(&dest_dir, "test.txt", b"Existing content");
    
    let token = PasteCancellationToken::new();
    let (tx, _rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[source_file],
        &dest_dir,
        false,
        |_, _| ConflictResolution::Replace,
    );
    
    assert!(result.is_ok());
    let paste_result = result.unwrap();
    assert_eq!(paste_result.successful_files.len(), 1);
    
    let dest_file = dest_dir.join("test.txt");
    assert_eq!(fs::read_to_string(&dest_file).unwrap(), "New content");
}

#[test]
fn test_paste_executor_conflict_keep_both() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    let source_file = create_test_file(&source_dir, "test.txt", b"New content");
    
    create_test_file(&dest_dir, "test.txt", b"Existing content");
    
    let token = PasteCancellationToken::new();
    let (tx, _rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[source_file],
        &dest_dir,
        false,
        |_, _| ConflictResolution::KeepBoth,
    );
    
    assert!(result.is_ok());
    let paste_result = result.unwrap();
    assert_eq!(paste_result.successful_files.len(), 1);
    
    let original_file = dest_dir.join("test.txt");
    assert!(original_file.exists());
    assert_eq!(fs::read_to_string(&original_file).unwrap(), "Existing content");
    
    let new_file = &paste_result.successful_files[0];
    assert!(new_file.exists());
    assert_eq!(fs::read_to_string(new_file).unwrap(), "New content");
}

#[test]
fn test_paste_executor_cut_operation() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    let source_file = create_test_file(&source_dir, "test.txt", b"Content to move");
    
    let token = PasteCancellationToken::new();
    let (tx, _rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[source_file.clone()],
        &dest_dir,
        true,
        |_, _| ConflictResolution::Replace,
    );
    
    assert!(result.is_ok());
    let paste_result = result.unwrap();
    assert_eq!(paste_result.successful_files.len(), 1);
    
    assert!(!source_file.exists(), "Source file should be deleted after cut");
    
    let dest_file = dest_dir.join("test.txt");
    assert!(dest_file.exists(), "Destination file should exist");
    assert_eq!(fs::read_to_string(&dest_file).unwrap(), "Content to move");
}

#[test]
fn test_paste_executor_directory() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");
    let subdir = source_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();
    
    create_test_file(&subdir, "file1.txt", b"Content 1");
    create_test_file(&subdir, "file2.txt", b"Content 2");
    
    let token = PasteCancellationToken::new();
    let (tx, _rx) = flume::unbounded();
    let executor = PasteExecutor::new(token, tx);
    
    let result = executor.execute(
        &[subdir.clone()],
        &dest_dir,
        false,
        |_, _| ConflictResolution::Replace,
    );
    
    assert!(result.is_ok());
    
    let dest_subdir = dest_dir.join("subdir");
    assert!(dest_subdir.exists());
    assert!(dest_subdir.join("file1.txt").exists());
    assert!(dest_subdir.join("file2.txt").exists());
}
