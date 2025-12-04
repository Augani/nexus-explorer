use super::*;
use proptest::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn create_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

fn create_test_file(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = File::create(&path).expect("Failed to create test file");
    file.write_all(content).expect("Failed to write test file");
    path
}

fn create_test_subdir(dir: &TempDir, name: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::create_dir(&path).expect("Failed to create test subdir");
    path
}

// ============================================================================
// Property 22: Preview Metadata Completeness
// For any valid file path, the preview metadata SHALL contain name, size, type, date, and permissions
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn property_22_preview_metadata_completeness(
        file_name in "[a-zA-Z][a-zA-Z0-9_]{0,10}\\.(txt|rs|json|md)",
        content in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let temp_dir = create_test_dir();
        let file_path = create_test_file(&temp_dir, &file_name, &content);

        let metadata = FileMetadata::from_path(&file_path);

        // Property: Metadata must be present for valid files
        prop_assert!(metadata.is_some(), "Metadata should be present for valid file");

        let meta = metadata.unwrap();

        // Property: Name must not be empty
        prop_assert!(!meta.name.is_empty(), "Name should not be empty");

        // Property: Name should match the file name
        prop_assert_eq!(&meta.name, &file_name, "Name should match file name");

        // Property: Size should match content length
        prop_assert_eq!(meta.size, content.len() as u64, "Size should match content length");

        // Property: File type should not be empty
        prop_assert!(!meta.file_type.is_empty(), "File type should not be empty");

        // Property: is_dir should be false for files
        prop_assert!(!meta.is_dir, "is_dir should be false for files");

        prop_assert!(meta.has_all_fields(), "has_all_fields should return true");
    }
}

// ============================================================================
// Property 23: Preview Text Line Numbers
// For any text file, the preview SHALL display line numbers matching the actual line count
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn property_23_preview_text_line_numbers(
        lines in prop::collection::vec("[a-zA-Z0-9 ]{0,50}", 1..100)
    ) {
        let temp_dir = create_test_dir();
        let content = lines.join("\n");
        let file_path = create_test_file(&temp_dir, "test.txt", content.as_bytes());

        let mut preview = Preview::new();
        preview.load_file(&file_path);

        // Property: Content should be Text type
        match preview.content() {
            PreviewContent::Text { line_count, .. } => {
                // Property: Line count should match actual lines
                let expected_lines = content.lines().count();
                prop_assert_eq!(
                    *line_count, expected_lines,
                    "Line count {} should match actual lines {}",
                    line_count, expected_lines
                );
            }
            other => {
                prop_assert!(false, "Expected Text content, got {:?}", other);
            }
        }
    }
}

// ============================================================================
// Property 24: Preview Image Dimensions
// For any image file, the preview SHALL display format information
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn property_24_preview_image_format(
        ext in prop::sample::select(vec!["png", "jpg", "jpeg", "gif", "bmp", "webp"])
    ) {
        let temp_dir = create_test_dir();
        let file_name = format!("test.{}", ext);
        let file_path = create_test_file(&temp_dir, &file_name, b"fake image data");

        let mut preview = Preview::new();
        preview.load_file(&file_path);

        // Property: Content should be Image type for image extensions
        match preview.content() {
            PreviewContent::Image { format, .. } => {
                // Property: Format should match extension (uppercase)
                prop_assert_eq!(
                    format.to_lowercase(), ext.to_lowercase(),
                    "Format should match extension"
                );
            }
            other => {
                prop_assert!(false, "Expected Image content for .{}, got {:?}", ext, other);
            }
        }
    }
}

// ============================================================================
// Property 25: Preview Hex Dump Size
// For any binary file, the hex dump SHALL show at most 256 bytes
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn property_25_preview_hex_dump_size(
        content in prop::collection::vec(any::<u8>(), 1..1000)
    ) {
        let temp_dir = create_test_dir();
        // Use .bin extension to ensure binary treatment
        let file_path = create_test_file(&temp_dir, "test.bin", &content);

        let mut preview = Preview::new();
        preview.load_file(&file_path);

        // Property: Content should be HexDump type for binary files
        match preview.content() {
            PreviewContent::HexDump { bytes, total_size } => {
                // Property: Bytes should be at most 256
                prop_assert!(
                    bytes.len() <= 256,
                    "Hex dump should show at most 256 bytes, got {}",
                    bytes.len()
                );

                // Property: Bytes should be min(256, content.len())
                let expected_len = content.len().min(256);
                prop_assert_eq!(
                    bytes.len(), expected_len,
                    "Hex dump should show {} bytes, got {}",
                    expected_len, bytes.len()
                );

                // Property: Total size should match actual file size
                prop_assert_eq!(
                    *total_size, content.len() as u64,
                    "Total size should match file size"
                );

                // Property: Bytes should match the first N bytes of content
                prop_assert_eq!(
                    bytes.as_slice(), &content[..expected_len],
                    "Hex dump bytes should match file content"
                );
            }
            other => {
                prop_assert!(false, "Expected HexDump content for .bin, got {:?}", other);
            }
        }
    }
}

// ============================================================================
// Property 26: Preview Directory Statistics
// For any directory, the preview SHALL show correct item count, total size, and subdirectory count
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn property_26_preview_directory_statistics(
        file_count in 0usize..10,
        subdir_count in 0usize..5,
        file_sizes in prop::collection::vec(0u64..1000, 0..10)
    ) {
        let temp_dir = create_test_dir();

        let actual_file_count = file_count.min(file_sizes.len());
        let mut total_size = 0u64;
        for i in 0..actual_file_count {
            let size = file_sizes.get(i).copied().unwrap_or(0);
            let content = vec![0u8; size as usize];
            create_test_file(&temp_dir, &format!("file{}.txt", i), &content);
            total_size += size;
        }

        for i in 0..subdir_count {
            create_test_subdir(&temp_dir, &format!("subdir{}", i));
        }

        let mut preview = Preview::new();
        preview.load_file(temp_dir.path());

        // Property: Content should be Directory type
        match preview.content() {
            PreviewContent::Directory {
                item_count,
                total_size: reported_size,
                subdir_count: reported_subdirs,
                file_count: reported_files,
            } => {
                let expected_item_count = actual_file_count + subdir_count;

                // Property: Item count should match total items
                prop_assert_eq!(
                    *item_count, expected_item_count,
                    "Item count should be {}, got {}",
                    expected_item_count, item_count
                );

                // Property: Subdir count should match
                prop_assert_eq!(
                    *reported_subdirs, subdir_count,
                    "Subdir count should be {}, got {}",
                    subdir_count, reported_subdirs
                );

                // Property: File count should match
                prop_assert_eq!(
                    *reported_files, actual_file_count,
                    "File count should be {}, got {}",
                    actual_file_count, reported_files
                );

                // Property: Total size should match sum of file sizes
                prop_assert_eq!(
                    *reported_size, total_size,
                    "Total size should be {}, got {}",
                    total_size, reported_size
                );
            }
            other => {
                prop_assert!(false, "Expected Directory content, got {:?}", other);
            }
        }
    }
}

// ============================================================================
// ============================================================================

#[test]
fn test_format_size() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(1536), "1.50 KB");
    assert_eq!(format_size(1048576), "1.00 MB");
    assert_eq!(format_size(1073741824), "1.00 GB");
}

#[test]
fn test_format_hex_dump() {
    let bytes = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f];
    let lines = format_hex_dump(&bytes);

    assert_eq!(lines.len(), 1);
    let (offset, hex, ascii) = &lines[0];
    assert_eq!(offset, "00000000");
    assert!(hex.starts_with("48 65 6C 6C 6F"));
    assert!(ascii.starts_with("Hello"));
}

#[test]
fn test_format_hex_dump_multiple_lines() {
    let bytes: Vec<u8> = (0..32).collect();
    let lines = format_hex_dump(&bytes);

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].0, "00000000");
    assert_eq!(lines[1].0, "00000010");
}

#[test]
fn test_preview_clear() {
    let mut preview = Preview::new();
    let temp_dir = create_test_dir();
    let file_path = create_test_file(&temp_dir, "test.txt", b"content");

    preview.load_file(&file_path);
    assert!(preview.metadata().is_some());

    preview.clear();
    assert!(preview.metadata().is_none());
    assert!(matches!(preview.content(), PreviewContent::None));
}

#[test]
fn test_file_metadata_has_all_fields() {
    let meta = FileMetadata {
        name: "test.txt".to_string(),
        size: 100,
        file_type: "TXT".to_string(),
        modified: Some(SystemTime::now()),
        permissions: "rw-r--r--".to_string(),
        is_dir: false,
    };

    assert!(meta.has_all_fields());

    let empty_meta = FileMetadata::default();
    assert!(!empty_meta.has_all_fields());
}
