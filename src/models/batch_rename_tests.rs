use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

// Unit tests for BatchRename

#[test]
fn test_new_batch_rename() {
    let files = vec![
        PathBuf::from("/test/file1.txt"),
        PathBuf::from("/test/file2.txt"),
    ];
    let batch = BatchRename::new(files.clone());

    assert_eq!(batch.files().len(), 2);
    assert!(!batch.has_conflicts());
}

#[test]
fn test_empty_batch_rename() {
    let batch = BatchRename::new(Vec::new());
    assert!(batch.files().is_empty());
    assert!(batch.preview().is_empty());
}

#[test]
fn test_pattern_with_counter() {
    let files = vec![
        PathBuf::from("/test/a.txt"),
        PathBuf::from("/test/b.txt"),
        PathBuf::from("/test/c.txt"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("photo_{n}");

    let preview = batch.preview();
    assert_eq!(preview.len(), 3);
    assert_eq!(preview[0].new_name, "photo_1.txt");
    assert_eq!(preview[1].new_name, "photo_2.txt");
    assert_eq!(preview[2].new_name, "photo_3.txt");
}

#[test]
fn test_pattern_with_padded_counter() {
    let files = vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_counter_padding(3);
    batch.set_pattern("file_{n}");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "file_001.txt");
    assert_eq!(preview[1].new_name, "file_002.txt");
}

#[test]
fn test_pattern_with_original_name() {
    let files = vec![
        PathBuf::from("/test/document.txt"),
        PathBuf::from("/test/report.txt"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("{name}_backup");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "document_backup.txt");
    assert_eq!(preview[1].new_name, "report_backup.txt");
}

#[test]
fn test_pattern_with_extension() {
    let files = vec![PathBuf::from("/test/file.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("renamed.{ext}");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "renamed.txt");
}

#[test]
fn test_find_replace() {
    let files = vec![
        PathBuf::from("/test/old_file.txt"),
        PathBuf::from("/test/old_document.txt"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_find_replace("old", "new");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "new_file.txt");
    assert_eq!(preview[1].new_name, "new_document.txt");
}

#[test]
fn test_find_replace_no_match() {
    let files = vec![PathBuf::from("/test/file.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_find_replace("xyz", "abc");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "file.txt");
}

#[test]
fn test_conflict_detection_duplicate_names() {
    let files = vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("same_name");

    assert!(batch.has_conflicts());
    assert_eq!(batch.conflicts().len(), 2);
}

#[test]
fn test_no_conflict_unique_names() {
    let files = vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("file_{n}");

    assert!(!batch.has_conflicts());
}

#[test]
fn test_counter_start_value() {
    let files = vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_counter_start(10);
    batch.set_pattern("item_{n}");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "item_10.txt");
    assert_eq!(preview[1].new_name, "item_11.txt");
}

#[test]
fn test_rename_count() {
    let files = vec![
        PathBuf::from("/test/file1.txt"),
        PathBuf::from("/test/file2.txt"),
    ];
    let mut batch = BatchRename::new(files);

    assert_eq!(batch.rename_count(), 0);

    batch.set_pattern("new_{n}");
    assert_eq!(batch.rename_count(), 2);
}

#[test]
fn test_apply_returns_error_on_empty() {
    let batch = BatchRename::new(Vec::new());
    let result = batch.apply();
    assert!(matches!(result, Err(BatchRenameError::NoFiles)));
}

#[test]
fn test_apply_returns_error_on_conflict() {
    let files = vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("same");

    let result = batch.apply();
    assert!(matches!(result, Err(BatchRenameError::Conflict(_))));
}

// Property-Based Tests

/// **Feature: ui-enhancements, Property 39: Batch Rename Preview Accuracy**
/// **Validates: Requirements 19.2**
///
/// *For any* set of files and any valid pattern, the preview SHALL accurately reflect
/// what the renamed files will be called, with the same number of previews as input files.
proptest! {
    #[test]
    fn prop_preview_count_matches_file_count(
        file_count in 1usize..20,
        pattern_type in 0u8..3,
    ) {
        // Generate file paths
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("/test/file_{}.txt", i)))
            .collect();

        let mut batch = BatchRename::new(files.clone());

        // Apply different pattern types
        match pattern_type {
            0 => batch.set_pattern("renamed_{n}"),
            1 => batch.set_pattern("{name}_copy"),
            _ => batch.set_find_replace("file", "document"),
        }

        // Property: preview count must equal file count
        prop_assert_eq!(batch.preview().len(), file_count);

        // Property: each preview must have non-empty original and new_name
        for preview in batch.preview() {
            prop_assert!(!preview.original.is_empty());
            prop_assert!(!preview.new_name.is_empty());
        }
    }

    #[test]
    fn prop_preview_original_matches_filename(
        file_names in proptest::collection::vec("[a-z]{1,10}\\.[a-z]{2,4}", 1..10),
    ) {
        let files: Vec<PathBuf> = file_names.iter()
            .map(|name| PathBuf::from(format!("/test/{}", name)))
            .collect();

        let batch = BatchRename::new(files.clone());

        // Property: each preview's original must match the corresponding file's name
        for (i, preview) in batch.preview().iter().enumerate() {
            let expected_name = files[i].file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            prop_assert_eq!(&preview.original, &expected_name);
        }
    }
}

// Additional tests for rename patterns (Requirements 19.3, 19.4, 19.5, 19.6)

#[test]
fn test_pattern_date_token() {
    let files = vec![PathBuf::from("/test/photo.jpg")];
    let mut batch = BatchRename::new(files);
    batch.set_date_format("%Y%m%d");
    batch.set_pattern("img_{date}");

    let preview = batch.preview();
    // The date should be in YYYYMMDD format
    assert!(preview[0].new_name.starts_with("img_"));
    assert!(preview[0].new_name.ends_with(".jpg"));
    // Date portion should be 8 digits
    let name_without_ext = preview[0].new_name.strip_suffix(".jpg").unwrap();
    let date_part = name_without_ext.strip_prefix("img_").unwrap();
    assert_eq!(date_part.len(), 8);
    assert!(date_part.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_pattern_combined_tokens() {
    let files = vec![
        PathBuf::from("/test/document.pdf"),
        PathBuf::from("/test/report.pdf"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_counter_padding(2);
    batch.set_pattern("{name}_{n}");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "document_01.pdf");
    assert_eq!(preview[1].new_name, "report_02.pdf");
}

#[test]
fn test_pattern_extension_explicit() {
    let files = vec![PathBuf::from("/test/file.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("newfile.{ext}");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "newfile.txt");
}

#[test]
fn test_find_replace_multiple_occurrences() {
    let files = vec![PathBuf::from("/test/old_old_file.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_find_replace("old", "new");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "new_new_file.txt");
}

#[test]
fn test_find_replace_case_sensitive() {
    let files = vec![PathBuf::from("/test/OldFile.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_find_replace("old", "new");

    let preview = batch.preview();
    // Should not match because case is different
    assert_eq!(preview[0].new_name, "OldFile.txt");
}

#[test]
fn test_find_replace_case_insensitive() {
    let files = vec![PathBuf::from("/test/OldFile.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_find_replace_with_options("old", "new", false, true);

    let preview = batch.preview();
    // Should match because case-insensitive is enabled
    assert_eq!(preview[0].new_name, "newFile.txt");
}

#[test]
fn test_find_replace_regex() {
    let files = vec![
        PathBuf::from("/test/file001.txt"),
        PathBuf::from("/test/file002.txt"),
        PathBuf::from("/test/file123.txt"),
    ];
    let mut batch = BatchRename::new(files);
    // Replace digits with underscore
    batch.set_find_replace_with_options(r"\d+", "NUM", true, false);

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "fileNUM.txt");
    assert_eq!(preview[1].new_name, "fileNUM.txt");
    assert_eq!(preview[2].new_name, "fileNUM.txt");
}

#[test]
fn test_find_replace_regex_case_insensitive() {
    let files = vec![PathBuf::from("/test/MyDocument.txt")];
    let mut batch = BatchRename::new(files);
    // Case-insensitive regex
    batch.set_find_replace_with_options("mydoc", "YourDoc", true, true);

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "YourDocument.txt");
}

#[test]
fn test_find_replace_invalid_regex() {
    let files = vec![PathBuf::from("/test/file.txt")];
    let mut batch = BatchRename::new(files);
    // Invalid regex pattern - should not crash, just return original
    batch.set_find_replace_with_options("[invalid", "replacement", true, false);

    let preview = batch.preview();
    // Should return original name when regex is invalid
    assert_eq!(preview[0].new_name, "file.txt");
}

#[test]
fn test_pattern_preserves_extension_automatically() {
    let files = vec![PathBuf::from("/test/image.png")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("photo_{n}");

    let preview = batch.preview();
    // Extension should be preserved even without {ext} token
    assert_eq!(preview[0].new_name, "photo_1.png");
}

#[test]
fn test_file_without_extension() {
    let files = vec![PathBuf::from("/test/Makefile")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("build_{n}");

    let preview = batch.preview();
    // No extension to preserve
    assert_eq!(preview[0].new_name, "build_1");
}

#[test]
fn test_pattern_with_literal_braces() {
    let files = vec![PathBuf::from("/test/file.txt")];
    let mut batch = BatchRename::new(files);
    // Invalid token should be treated as literal text
    batch.set_pattern("test_{invalid}");

    let preview = batch.preview();
    assert_eq!(preview[0].new_name, "test_{invalid}.txt");
}

/// **Feature: ui-enhancements, Property 40: Batch Rename Sequential Numbers**
/// **Validates: Requirements 19.3**
///
/// *For any* set of files with a pattern containing {n}, the generated names SHALL
/// contain sequential numbers starting from the configured start value.
proptest! {
    #[test]
    fn prop_sequential_numbers_are_consecutive(
        file_count in 2usize..20,
        start_value in 0usize..100,
        padding in 1usize..5,
    ) {
        // Generate file paths
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("/test/file_{}.txt", i)))
            .collect();

        let mut batch = BatchRename::new(files);
        batch.set_counter_start(start_value);
        batch.set_counter_padding(padding);
        batch.set_pattern("item_{n}");

        let preview = batch.preview();

        // Extract numbers from the generated names
        let mut numbers: Vec<usize> = Vec::new();
        for p in preview.iter() {
            // Parse the number from "item_XXX.txt"
            let name_without_ext = p.new_name.strip_suffix(".txt").unwrap_or(&p.new_name);
            let num_str = name_without_ext.strip_prefix("item_").unwrap_or("");
            if let Ok(num) = num_str.parse::<usize>() {
                numbers.push(num);
            }
        }

        // Property: numbers should be consecutive starting from start_value
        prop_assert_eq!(numbers.len(), file_count);
        for (i, &num) in numbers.iter().enumerate() {
            prop_assert_eq!(num, start_value + i,
                "Expected {} at index {}, got {}", start_value + i, i, num);
        }
    }

    #[test]
    fn prop_sequential_numbers_padding(
        file_count in 1usize..10,
        padding in 1usize..6,
    ) {
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("/test/f{}.txt", i)))
            .collect();

        let mut batch = BatchRename::new(files);
        batch.set_counter_padding(padding);
        batch.set_pattern("num_{n}");

        let preview = batch.preview();

        // Property: each number should be padded to at least 'padding' digits
        for p in preview.iter() {
            let name_without_ext = p.new_name.strip_suffix(".txt").unwrap_or(&p.new_name);
            let num_str = name_without_ext.strip_prefix("num_").unwrap_or("");

            // The number string should be at least 'padding' characters long
            prop_assert!(num_str.len() >= padding,
                "Number '{}' should be at least {} digits, but is {} digits",
                num_str, padding, num_str.len());

            // Leading characters should be zeros if padded
            if num_str.len() > 1 {
                let leading_zeros = num_str.chars().take_while(|&c| c == '0').count();
                let actual_num: usize = num_str.parse().unwrap_or(0);
                let expected_padding = padding.saturating_sub(actual_num.to_string().len());
                prop_assert!(leading_zeros >= expected_padding.min(num_str.len() - 1));
            }
        }
    }
}

// Additional tests for conflict detection (Requirement 19.8)

#[test]
fn test_conflict_marks_preview_items() {
    let files = vec![
        PathBuf::from("/test/a.txt"),
        PathBuf::from("/test/b.txt"),
        PathBuf::from("/test/c.txt"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("same_name");

    // All items should be marked as conflicts
    for preview in batch.preview() {
        assert!(
            preview.has_conflict,
            "Preview '{}' should be marked as conflict",
            preview.new_name
        );
    }
}

#[test]
fn test_partial_conflict() {
    let files = vec![
        PathBuf::from("/test/a.txt"),
        PathBuf::from("/test/b.txt"),
        PathBuf::from("/test/c.txt"),
    ];
    let mut batch = BatchRename::new(files);
    // This pattern will cause a and b to have the same name, but c will be different
    batch.set_find_replace("a", "x");
    batch.set_find_replace("b", "x");

    // Only the first file should be renamed (a.txt -> x.txt)
    // b.txt stays as b.txt, c.txt stays as c.txt
    // No conflicts expected with this simple find/replace
    assert!(!batch.has_conflicts());
}

#[test]
fn test_no_conflict_when_names_differ() {
    let files = vec![
        PathBuf::from("/test/file1.txt"),
        PathBuf::from("/test/file2.txt"),
        PathBuf::from("/test/file3.txt"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("doc_{n}");

    assert!(!batch.has_conflicts());
    for preview in batch.preview() {
        assert!(!preview.has_conflict);
    }
}

#[test]
fn test_conflict_indices_correct() {
    let files = vec![
        PathBuf::from("/test/a.txt"),
        PathBuf::from("/test/b.txt"),
        PathBuf::from("/test/c.txt"),
    ];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("conflict");

    let conflicts = batch.conflicts();
    // All three should be in conflict
    assert_eq!(conflicts.len(), 3);
    assert!(conflicts.contains(&0));
    assert!(conflicts.contains(&1));
    assert!(conflicts.contains(&2));
}

#[test]
fn test_apply_blocked_by_conflict() {
    let files = vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")];
    let mut batch = BatchRename::new(files);
    batch.set_pattern("same");

    let result = batch.apply();
    match result {
        Err(BatchRenameError::Conflict(indices)) => {
            assert_eq!(indices.len(), 2);
        }
        _ => panic!("Expected Conflict error"),
    }
}

/// **Feature: advanced-device-management, Property 19: Batch Rename Pattern Expansion**
/// **Validates: Requirements 15.2**
///
/// *For any* batch rename pattern containing {n}, the expanded names SHALL contain
/// sequential numbers starting from 1, with no duplicates.
proptest! {
    #[test]
    fn prop_batch_rename_pattern_expansion_sequential_no_duplicates(
        file_count in 2usize..50,
    ) {
        // Generate file paths
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("/test/file_{}.txt", i)))
            .collect();

        let mut batch = BatchRename::new(files);
        batch.set_pattern("renamed_{n}");

        let preview = batch.preview();

        // Property 1: Preview count matches file count
        prop_assert_eq!(preview.len(), file_count);

        // Extract numbers from the generated names
        let mut numbers: Vec<usize> = Vec::new();
        for p in preview.iter() {
            let name_without_ext = p.new_name.strip_suffix(".txt").unwrap_or(&p.new_name);
            let num_str = name_without_ext.strip_prefix("renamed_").unwrap_or("");
            if let Ok(num) = num_str.parse::<usize>() {
                numbers.push(num);
            }
        }

        // Property 2: All numbers were successfully extracted
        prop_assert_eq!(numbers.len(), file_count,
            "Expected {} numbers, got {}", file_count, numbers.len());

        // Property 3: Numbers start from 1 (default counter start)
        prop_assert_eq!(numbers[0], 1,
            "First number should be 1, got {}", numbers[0]);

        // Property 4: Numbers are sequential (each is previous + 1)
        for i in 1..numbers.len() {
            prop_assert_eq!(numbers[i], numbers[i-1] + 1,
                "Numbers should be sequential: {} should follow {}", numbers[i], numbers[i-1]);
        }

        // Property 5: No duplicates (all numbers are unique)
        let mut sorted_numbers = numbers.clone();
        sorted_numbers.sort();
        sorted_numbers.dedup();
        prop_assert_eq!(sorted_numbers.len(), file_count,
            "All numbers should be unique, found duplicates");

        // Property 6: No conflicts should exist with sequential counter pattern
        prop_assert!(!batch.has_conflicts(),
            "Sequential counter pattern should never produce conflicts");
    }
}

/// Property test for conflict detection
proptest! {
    #[test]
    fn prop_unique_counter_names_no_conflict(
        file_count in 1usize..20,
        start in 1usize..100,
    ) {
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("/test/file_{}.txt", i)))
            .collect();

        let mut batch = BatchRename::new(files);
        batch.set_counter_start(start);
        batch.set_pattern("item_{n}");

        // Property: using counter pattern should never produce conflicts
        prop_assert!(!batch.has_conflicts(),
            "Counter pattern should produce unique names");
    }

    #[test]
    fn prop_static_pattern_causes_conflict_for_multiple_files(
        file_count in 2usize..10,
    ) {
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("/test/file_{}.txt", i)))
            .collect();

        let mut batch = BatchRename::new(files);
        batch.set_pattern("static_name");

        // Property: static pattern with multiple files should always cause conflicts
        prop_assert!(batch.has_conflicts(),
            "Static pattern with {} files should cause conflicts", file_count);
        prop_assert_eq!(batch.conflicts().len(), file_count,
            "All {} files should be in conflict", file_count);
    }
}
