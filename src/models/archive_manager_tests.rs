use super::*;
use proptest::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/
fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

/
fn create_test_structure(dir: &Path, files: &[(String, Vec<u8>)]) -> Vec<PathBuf> {
    files.iter().map(|(name, content)| {
        create_test_file(dir, name, content)
    }).collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_archive_listing_completeness(
        file_count in 1usize..10,
        file_sizes in prop::collection::vec(1usize..1000, 1..10),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        
        let actual_count = file_count.min(file_sizes.len());
        let mut files = Vec::new();
        let mut total_size: u64 = 0;
        
        for i in 0..actual_count {
            let name = format!("file_{}.txt", i);
            let content: Vec<u8> = vec![b'x'; file_sizes[i]];
            total_size += content.len() as u64;
            let path = create_test_file(&source_dir, &name, &content);
            files.push(path);
        }
        
        let archive_path = temp_dir.path().join("test.zip");
        let manager = ArchiveManager::new();
        
        let options = CompressOptions {
            format: ArchiveFormat::Zip,
            compression_level: 6,
            password: None,
        };
        
        manager.compress(&files, &archive_path, &options, |_| {}).unwrap();
        
        let entries = manager.list_contents(&archive_path).unwrap();
        
        for entry in &entries {
            prop_assert!(!entry.path.is_empty(), "Entry path should not be empty");
        }
        
        let listed_total: u64 = entries.iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum();
        
        prop_assert_eq!(
            listed_total, 
            total_size,
            "Total uncompressed size should match: listed={}, expected={}",
            listed_total,
            total_size
        );
        
        let file_entries: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();
        prop_assert_eq!(
            file_entries.len(),
            actual_count,
            "Number of file entries should match: got={}, expected={}",
            file_entries.len(),
            actual_count
        );
    }
}

#[test]
fn test_archive_listing_zip() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    
    let files = create_test_structure(&source_dir, &[
        ("file1.txt".to_string(), b"Hello World".to_vec()),
        ("file2.txt".to_string(), b"Test content".to_vec()),
        ("subdir/file3.txt".to_string(), b"Nested file".to_vec()),
    ]);
    
    let archive_path = temp_dir.path().join("test.zip");
    let manager = ArchiveManager::new();
    
    let options = CompressOptions::default();
    manager.compress(&[source_dir.clone()], &archive_path, &options, |_| {}).unwrap();
    
    let entries = manager.list_contents(&archive_path).unwrap();
    
    assert!(!entries.is_empty(), "Archive should have entries");
    
    for entry in &entries {
        assert!(!entry.path.is_empty(), "Entry path should not be empty");
    }
}

#[test]
fn test_archive_listing_tar_gz() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"Hello World");
    create_test_file(&source_dir, "file2.txt", b"Test content");
    
    let archive_path = temp_dir.path().join("test.tar.gz");
    let manager = ArchiveManager::new();
    
    let options = CompressOptions {
        format: ArchiveFormat::TarGz,
        compression_level: 6,
        password: None,
    };
    
    manager.compress(&[source_dir.clone()], &archive_path, &options, |_| {}).unwrap();
    
    let entries = manager.list_contents(&archive_path).unwrap();
    
    assert!(!entries.is_empty(), "Archive should have entries");
    
    for entry in &entries {
        assert!(!entry.path.is_empty(), "Entry path should not be empty");
    }
}

#[test]
fn test_get_total_uncompressed_size() {
    let entries = vec![
        ArchiveEntry {
            path: "file1.txt".to_string(),
            is_dir: false,
            size: 100,
            compressed_size: 50,
            modified: None,
            is_encrypted: false,
        },
        ArchiveEntry {
            path: "file2.txt".to_string(),
            is_dir: false,
            size: 200,
            compressed_size: 100,
            modified: None,
            is_encrypted: false,
        },
        ArchiveEntry {
            path: "subdir/".to_string(),
            is_dir: true,
            size: 0,
            compressed_size: 0,
            modified: None,
            is_encrypted: false,
        },
    ];
    
    let manager = ArchiveManager::new();
    let total = manager.get_total_uncompressed_size(&entries);
    
    assert_eq!(total, 300, "Total should be sum of file sizes, excluding directories");
}

#[test]
fn test_archive_format_detection() {
    assert_eq!(ArchiveFormat::from_extension(Path::new("test.zip")), Some(ArchiveFormat::Zip));
    assert_eq!(ArchiveFormat::from_extension(Path::new("test.tar.gz")), Some(ArchiveFormat::TarGz));
    assert_eq!(ArchiveFormat::from_extension(Path::new("test.tgz")), Some(ArchiveFormat::TarGz));
    assert_eq!(ArchiveFormat::from_extension(Path::new("test.7z")), Some(ArchiveFormat::SevenZip));
    assert_eq!(ArchiveFormat::from_extension(Path::new("test.txt")), None);
}

#[test]
fn test_is_archive() {
    let manager = ArchiveManager::new();
    
    assert!(manager.is_archive(Path::new("test.zip")));
    assert!(manager.is_archive(Path::new("test.tar.gz")));
    assert!(manager.is_archive(Path::new("test.7z")));
    assert!(!manager.is_archive(Path::new("test.txt")));
    assert!(!manager.is_archive(Path::new("test.pdf")));
}


#[test]
fn test_compression_with_progress() {
    use std::sync::{Arc, Mutex};
    
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"Hello World");
    create_test_file(&source_dir, "file2.txt", b"Test content with more data");
    create_test_file(&source_dir, "subdir/file3.txt", b"Nested file content");
    
    let archive_path = temp_dir.path().join("test.zip");
    let manager = ArchiveManager::new();
    
    let options = CompressOptions::default();
    
    let progress_updates = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = progress_updates.clone();
    
    manager.compress(
        &[source_dir.clone()],
        &archive_path,
        &options,
        move |progress| {
            progress_clone.lock().unwrap().push(progress.clone());
        }
    ).unwrap();
    
    let updates = progress_updates.lock().unwrap();
    
    assert!(!updates.is_empty(), "Should have received progress updates");
    
    let final_progress = updates.last().unwrap();
    assert!(
        final_progress.percentage >= 99.0,
        "Final progress should be ~100%, got {}",
        final_progress.percentage
    );
    
    assert!(archive_path.exists(), "Archive should exist");
}

#[test]
fn test_compression_tar_gz_with_progress() {
    use std::sync::{Arc, Mutex};
    
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"Hello World");
    create_test_file(&source_dir, "file2.txt", b"Test content");
    
    let archive_path = temp_dir.path().join("test.tar.gz");
    let manager = ArchiveManager::new();
    
    let options = CompressOptions {
        format: ArchiveFormat::TarGz,
        compression_level: 6,
        password: None,
    };
    
    let progress_updates = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = progress_updates.clone();
    
    manager.compress(
        &[source_dir.clone()],
        &archive_path,
        &options,
        move |progress| {
            progress_clone.lock().unwrap().push(progress.clone());
        }
    ).unwrap();
    
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty(), "Should have received progress updates");
    assert!(archive_path.exists(), "Archive should exist");
}


#[test]
fn test_extraction_with_progress() {
    use std::sync::{Arc, Mutex};
    
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&source_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"Hello World");
    create_test_file(&source_dir, "file2.txt", b"Test content with more data");
    
    let archive_path = temp_dir.path().join("test.zip");
    let manager = ArchiveManager::new();
    
    manager.compress(
        &[source_dir.clone()],
        &archive_path,
        &CompressOptions::default(),
        |_| {}
    ).unwrap();
    
    let progress_updates = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = progress_updates.clone();
    
    let extract_options = ExtractOptions {
        destination: extract_dir.clone(),
        password: None,
        overwrite: OverwriteMode::Replace,
    };
    
    manager.extract(
        &archive_path,
        &extract_options,
        move |progress| {
            progress_clone.lock().unwrap().push(progress.clone());
        }
    ).unwrap();
    
    let updates = progress_updates.lock().unwrap();
    
    assert!(!updates.is_empty(), "Should have received progress updates");
    
    let final_progress = updates.last().unwrap();
    assert!(
        final_progress.percentage >= 99.0,
        "Final progress should be ~100%, got {}",
        final_progress.percentage
    );
    
    assert!(extract_dir.exists(), "Extract directory should exist");
}

#[test]
fn test_extraction_tar_gz_with_progress() {
    use std::sync::{Arc, Mutex};
    
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&source_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"Hello World");
    create_test_file(&source_dir, "file2.txt", b"Test content");
    
    let archive_path = temp_dir.path().join("test.tar.gz");
    let manager = ArchiveManager::new();
    
    let options = CompressOptions {
        format: ArchiveFormat::TarGz,
        compression_level: 6,
        password: None,
    };
    
    manager.compress(&[source_dir.clone()], &archive_path, &options, |_| {}).unwrap();
    
    let progress_updates = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = progress_updates.clone();
    
    let extract_options = ExtractOptions {
        destination: extract_dir.clone(),
        password: None,
        overwrite: OverwriteMode::Replace,
    };
    
    manager.extract(
        &archive_path,
        &extract_options,
        move |progress| {
            progress_clone.lock().unwrap().push(progress.clone());
        }
    ).unwrap();
    
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty(), "Should have received progress updates");
    assert!(extract_dir.exists(), "Extract directory should exist");
}

#[test]
fn test_extraction_overwrite_skip() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&extract_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"Original content");
    
    let archive_path = temp_dir.path().join("test.zip");
    let manager = ArchiveManager::new();
    
    manager.compress(&[source_dir.clone()], &archive_path, &CompressOptions::default(), |_| {}).unwrap();
    
    let existing_file = extract_dir.join("source/file1.txt");
    fs::create_dir_all(existing_file.parent().unwrap()).unwrap();
    fs::write(&existing_file, b"Existing content").unwrap();
    
    let extract_options = ExtractOptions {
        destination: extract_dir.clone(),
        password: None,
        overwrite: OverwriteMode::Skip,
    };
    
    manager.extract(&archive_path, &extract_options, |_| {}).unwrap();
    
    let content = fs::read_to_string(&existing_file).unwrap();
    assert_eq!(content, "Existing content", "File should not be overwritten in Skip mode");
}

#[test]
fn test_extraction_overwrite_replace() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&extract_dir).unwrap();
    
    create_test_file(&source_dir, "file1.txt", b"New content");
    
    let archive_path = temp_dir.path().join("test.zip");
    let manager = ArchiveManager::new();
    
    manager.compress(&[source_dir.clone()], &archive_path, &CompressOptions::default(), |_| {}).unwrap();
    
    let existing_file = extract_dir.join("source/file1.txt");
    fs::create_dir_all(existing_file.parent().unwrap()).unwrap();
    fs::write(&existing_file, b"Old content").unwrap();
    
    let extract_options = ExtractOptions {
        destination: extract_dir.clone(),
        password: None,
        overwrite: OverwriteMode::Replace,
    };
    
    manager.extract(&archive_path, &extract_options, |_| {}).unwrap();
    
    let content = fs::read_to_string(&existing_file).unwrap();
    assert_eq!(content, "New content", "File should be overwritten in Replace mode");
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_archive_extraction_progress(
        file_count in 1usize..8,
        file_sizes in prop::collection::vec(10usize..500, 1..8),
    ) {
        use std::sync::{Arc, Mutex};
        
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");
        fs::create_dir_all(&source_dir).unwrap();
        
        let actual_count = file_count.min(file_sizes.len());
        let mut files = Vec::new();
        
        for i in 0..actual_count {
            let name = format!("file_{}.txt", i);
            let content: Vec<u8> = vec![b'a' + (i as u8 % 26); file_sizes[i]];
            let path = create_test_file(&source_dir, &name, &content);
            files.push(path);
        }
        
        let archive_path = temp_dir.path().join("test.zip");
        let manager = ArchiveManager::new();
        
        manager.compress(
            &files,
            &archive_path,
            &CompressOptions::default(),
            |_| {}
        ).unwrap();
        
        let entries = manager.list_contents(&archive_path).unwrap();
        let entry_paths: std::collections::HashSet<_> = entries.iter()
            .map(|e| e.path.clone())
            .collect();
        
        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = progress_updates.clone();
        
        let extract_options = ExtractOptions {
            destination: extract_dir.clone(),
            password: None,
            overwrite: OverwriteMode::Replace,
        };
        
        manager.extract(
            &archive_path,
            &extract_options,
            move |progress| {
                progress_clone.lock().unwrap().push(progress.clone());
            }
        ).unwrap();
        
        let updates = progress_updates.lock().unwrap();
        
        let mut prev_percentage = -1.0f64;
        for (i, update) in updates.iter().enumerate() {
            prop_assert!(
                update.percentage >= prev_percentage,
                "Progress should increase monotonically: update {} has {}% but previous was {}%",
                i,
                update.percentage,
                prev_percentage
            );
            prev_percentage = update.percentage;
        }
        
        if let Some(final_update) = updates.last() {
            prop_assert!(
                final_update.percentage >= 99.0,
                "Final progress should be ~100%, got {}%",
                final_update.percentage
            );
        }
        
        for update in updates.iter() {
            if !update.current_file.is_empty() && update.current_file != "Complete" {
                let normalized_current = update.current_file.replace('\\', "/");
                let is_valid = entry_paths.iter().any(|entry| {
                    let normalized_entry = entry.replace('\\', "/");
                    normalized_entry.contains(&normalized_current) || 
                    normalized_current.contains(&normalized_entry) ||
                    normalized_entry.ends_with(&normalized_current)
                });
                
                if !is_valid && !update.current_file.contains("file_") {
                    prop_assert!(
                        is_valid,
                        "Current file '{}' should be a valid archive entry. Valid entries: {:?}",
                        update.current_file,
                        entry_paths
                    );
                }
            }
        }
    }
}
