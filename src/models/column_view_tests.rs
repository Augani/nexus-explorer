use super::column_view::{Column, ColumnView};
use super::types::FileEntry;
use std::path::PathBuf;
use std::time::SystemTime;

fn create_test_entry(name: &str, is_dir: bool) -> FileEntry {
    FileEntry::new(
        name.to_string(),
        PathBuf::from(format!("/test/{}", name)),
        is_dir,
        if is_dir { 0 } else { 1024 },
        SystemTime::now(),
    )
}

fn create_test_entries() -> Vec<FileEntry> {
    vec![
        create_test_entry("folder1", true),
        create_test_entry("folder2", true),
        create_test_entry("file1.txt", false),
        create_test_entry("file2.txt", false),
    ]
}

#[test]
fn test_column_new() {
    let path = PathBuf::from("/test");
    let column = Column::new(path.clone());

    assert_eq!(column.path, path);
    assert!(column.entries.is_empty());
    assert_eq!(column.selected_index, None);
}

#[test]
fn test_column_with_entries() {
    let path = PathBuf::from("/test");
    let entries = create_test_entries();
    let column = Column::with_entries(path.clone(), entries.clone());

    assert_eq!(column.path, path);
    assert_eq!(column.entries.len(), 4);
    assert_eq!(column.selected_index, None);
}

#[test]
fn test_column_select() {
    let path = PathBuf::from("/test");
    let entries = create_test_entries();
    let mut column = Column::with_entries(path, entries);

    column.select(1);
    assert_eq!(column.selected_index, Some(1));

    // Selecting out of bounds should not change selection
    column.select(100);
    assert_eq!(column.selected_index, Some(1));
}

#[test]
fn test_column_selected_entry() {
    let path = PathBuf::from("/test");
    let entries = create_test_entries();
    let mut column = Column::with_entries(path, entries);

    assert!(column.selected_entry().is_none());

    column.select(0);
    let selected = column.selected_entry().unwrap();
    assert_eq!(selected.name, "folder1");
}

#[test]
fn test_column_view_new() {
    let root = PathBuf::from("/root");
    let view = ColumnView::new(root.clone());

    assert_eq!(view.column_count(), 1);
    assert_eq!(view.root_path(), &root);
    assert_eq!(view.scroll_offset(), 0.0);
    assert_eq!(view.column_width(), ColumnView::DEFAULT_COLUMN_WIDTH);
}

#[test]
fn test_column_view_with_column_width() {
    let root = PathBuf::from("/root");
    let view = ColumnView::with_column_width(root, 300.0);

    assert_eq!(view.column_width(), 300.0);
}

#[test]
fn test_column_view_width_clamping() {
    let root = PathBuf::from("/root");

    // Test minimum clamping
    let view = ColumnView::with_column_width(root.clone(), 50.0);
    assert_eq!(view.column_width(), ColumnView::MIN_COLUMN_WIDTH);

    // Test maximum clamping
    let view = ColumnView::with_column_width(root, 1000.0);
    assert_eq!(view.column_width(), ColumnView::MAX_COLUMN_WIDTH);
}

#[test]
fn test_column_view_select_directory() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    // Select a directory (index 0 is folder1)
    view.select(0, 0);

    // Should have 2 columns now (original + new for selected directory)
    assert_eq!(view.column_count(), 2);
    assert_eq!(view.columns()[0].selected_index, Some(0));
}

#[test]
fn test_column_view_select_file() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    // Select a file (index 2 is file1.txt)
    view.select(0, 2);

    // Should still have 1 column (no new column for files)
    assert_eq!(view.column_count(), 1);
    assert_eq!(view.columns()[0].selected_index, Some(2));
}

#[test]
fn test_column_view_visible_columns() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::with_column_width(root, 200.0);

    // Add multiple columns
    for i in 0..5 {
        let entries = vec![create_test_entry(&format!("folder{}", i), true)];
        if i == 0 {
            view.set_column_entries(0, entries);
        } else {
            view.select(i - 1, 0);
            view.set_column_entries(i, entries);
        }
    }

    // With 500px viewport and 200px columns, should see ~3 columns
    let visible = view.visible_columns(500.0);
    assert!(visible.end - visible.start <= 4);
}

#[test]
fn test_column_view_navigate_down() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    // Navigate down should select first item
    assert!(view.navigate_down());
    assert_eq!(view.columns()[0].selected_index, Some(0));

    // Navigate down again
    assert!(view.navigate_down());
    assert_eq!(view.columns()[0].selected_index, Some(1));
}

#[test]
fn test_column_view_navigate_up() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    // Select second item
    view.select(0, 1);

    assert!(view.navigate_up());
    assert_eq!(view.columns()[0].selected_index, Some(0));

    // Can't navigate up from first item
    assert!(!view.navigate_up());
}

#[test]
fn test_column_view_navigate_right() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    // Select a directory
    view.select(0, 0);

    // Add entries to the new column
    let child_entries = vec![
        create_test_entry("child1", false),
        create_test_entry("child2", false),
    ];
    view.set_column_entries(1, child_entries);

    // Navigate right should move to next column
    assert!(view.navigate_right());
    assert_eq!(view.columns()[1].selected_index, Some(0));
}

#[test]
fn test_column_view_navigate_left() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    // Select a directory and navigate into it
    view.select(0, 0);
    let child_entries = vec![create_test_entry("child1", false)];
    view.set_column_entries(1, child_entries);

    // Select in the second column using navigate_right
    view.navigate_right();

    // Navigate left should go back to parent column
    assert!(view.navigate_left());
    assert_eq!(view.columns()[1].selected_index, None);
}

#[test]
fn test_column_view_reset() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);
    view.select(0, 0);
    view.set_scroll_offset(100.0);

    view.reset();

    assert_eq!(view.column_count(), 1);
    assert_eq!(view.scroll_offset(), 0.0);
    assert_eq!(view.columns()[0].selected_index, None);
}

#[test]
fn test_column_view_set_root() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);
    view.select(0, 0);

    let new_root = PathBuf::from("/new_root");
    view.set_root(new_root.clone());

    assert_eq!(view.root_path(), &new_root);
    assert_eq!(view.column_count(), 1);
    assert_eq!(view.columns()[0].path, new_root);
}

#[test]
fn test_column_view_total_width() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::with_column_width(root, 200.0);

    assert_eq!(view.total_width(), 200.0);

    // Add more columns
    let entries = vec![create_test_entry("folder", true)];
    view.set_column_entries(0, entries);
    view.select(0, 0);

    assert_eq!(view.total_width(), 400.0);
}

#[test]
fn test_column_view_path_hierarchy() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root.clone());

    let entries = vec![create_test_entry("folder1", true)];
    view.set_column_entries(0, entries);
    view.select(0, 0);

    let hierarchy = view.path_hierarchy();
    assert_eq!(hierarchy.len(), 2);
    assert_eq!(hierarchy[0], &root);
}

// Property-based tests
use proptest::prelude::*;

/// Generates a valid file entry for testing
fn arb_file_entry() -> impl Strategy<Value = FileEntry> {
    ("[a-zA-Z0-9_]{1,20}", prop::bool::ANY).prop_map(|(name, is_dir)| {
        let ext = if is_dir { "" } else { ".txt" };
        let full_name = format!("{}{}", name, ext);
        FileEntry::new(
            full_name.clone(),
            PathBuf::from(format!("/test/{}", full_name)),
            is_dir,
            if is_dir { 0 } else { 1024 },
            SystemTime::now(),
        )
    })
}

/// Generates a vector of file entries with at least one directory
fn arb_entries_with_dir() -> impl Strategy<Value = Vec<FileEntry>> {
    prop::collection::vec(arb_file_entry(), 1..10)
        .prop_filter("must have at least one directory", |entries| {
            entries.iter().any(|e| e.is_dir)
        })
}

/// **Feature: ui-enhancements, Property 43: Column View Hierarchy**
/// **Validates: Requirements 23.1, 23.2**
///
/// *For any* column view with directories selected, when a directory is selected
/// in column N, a new column N+1 SHALL be created for that directory's contents,
/// and all columns after N SHALL be removed before adding the new column.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_column_view_hierarchy(
        entries in arb_entries_with_dir(),
        selection_idx in 0usize..10,
    ) {
        let root = PathBuf::from("/root");
        let mut view = ColumnView::new(root);

        view.set_column_entries(0, entries.clone());

        // Find a directory to select
        let dir_indices: Vec<usize> = entries.iter()
            .enumerate()
            .filter(|(_, e)| e.is_dir)
            .map(|(i, _)| i)
            .collect();

        if dir_indices.is_empty() {
            return Ok(());
        }

        let dir_idx = dir_indices[selection_idx % dir_indices.len()];

        // Select the directory
        view.select(0, dir_idx);

        prop_assert_eq!(view.column_count(), 2,
            "Selecting a directory should create a new column");

        // Property 2: The new column's path should be the selected directory's path
        let selected_entry = &entries[dir_idx];
        prop_assert_eq!(&view.columns()[1].path, &selected_entry.path,
            "New column path should match selected directory");

        // Property 3: The selection should be recorded in the first column
        prop_assert_eq!(view.columns()[0].selected_index, Some(dir_idx),
            "Selection should be recorded in the column");
    }

    #[test]
    fn prop_column_view_file_selection_no_new_column(
        entries in arb_entries_with_dir(),
    ) {
        let root = PathBuf::from("/root");
        let mut view = ColumnView::new(root);

        view.set_column_entries(0, entries.clone());

        // Find a file to select
        let file_indices: Vec<usize> = entries.iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .map(|(i, _)| i)
            .collect();

        if file_indices.is_empty() {
            return Ok(());
        }

        let file_idx = file_indices[0];

        // Select the file
        view.select(0, file_idx);

        prop_assert_eq!(view.column_count(), 1,
            "Selecting a file should not create a new column");

        // Property: The selection should still be recorded
        prop_assert_eq!(view.columns()[0].selected_index, Some(file_idx),
            "Selection should be recorded even for files");
    }

    #[test]
    fn prop_column_view_selection_truncates_right_columns(
        entries1 in arb_entries_with_dir(),
        entries2 in arb_entries_with_dir(),
    ) {
        let root = PathBuf::from("/root");
        let mut view = ColumnView::new(root);

        view.set_column_entries(0, entries1.clone());

        // Find directories in first column
        let dir_indices1: Vec<usize> = entries1.iter()
            .enumerate()
            .filter(|(_, e)| e.is_dir)
            .map(|(i, _)| i)
            .collect();

        if dir_indices1.is_empty() {
            return Ok(());
        }

        view.select(0, dir_indices1[0]);
        view.set_column_entries(1, entries2.clone());

        // Find directories in second column
        let dir_indices2: Vec<usize> = entries2.iter()
            .enumerate()
            .filter(|(_, e)| e.is_dir)
            .map(|(i, _)| i)
            .collect();

        if dir_indices2.is_empty() {
            return Ok(());
        }

        view.select(1, dir_indices2[0]);
        prop_assert_eq!(view.column_count(), 3, "Should have 3 columns");

        // Now select a different directory in column 1
        if dir_indices1.len() > 1 {
            view.select(0, dir_indices1[1]);

            // Property: Columns to the right should be truncated
            prop_assert_eq!(view.column_count(), 2,
                "Selecting in column 0 should truncate columns 2+");
        }
    }
}
