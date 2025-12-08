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

    let view = ColumnView::with_column_width(root.clone(), 50.0);
    assert_eq!(view.column_width(), ColumnView::MIN_COLUMN_WIDTH);

    let view = ColumnView::with_column_width(root, 1000.0);
    assert_eq!(view.column_width(), ColumnView::MAX_COLUMN_WIDTH);
}

#[test]
fn test_column_view_select_directory() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    view.select(0, 0);

    assert_eq!(view.column_count(), 2);
    assert_eq!(view.columns()[0].selected_index, Some(0));
}

#[test]
fn test_column_view_select_file() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    view.select(0, 2);

    assert_eq!(view.column_count(), 1);
    assert_eq!(view.columns()[0].selected_index, Some(2));
}

#[test]
fn test_column_view_visible_columns() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::with_column_width(root, 200.0);

    for i in 0..5 {
        let entries = vec![create_test_entry(&format!("folder{}", i), true)];
        if i == 0 {
            view.set_column_entries(0, entries);
        } else {
            view.select(i - 1, 0);
            view.set_column_entries(i, entries);
        }
    }

    let visible = view.visible_columns(500.0);
    assert!(visible.end - visible.start <= 4);
}

#[test]
fn test_column_view_navigate_down() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    assert!(view.navigate_down());
    assert_eq!(view.columns()[0].selected_index, Some(0));

    assert!(view.navigate_down());
    assert_eq!(view.columns()[0].selected_index, Some(1));
}

#[test]
fn test_column_view_navigate_up() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    view.select(0, 1);

    assert!(view.navigate_up());
    assert_eq!(view.columns()[0].selected_index, Some(0));

    assert!(!view.navigate_up());
}

#[test]
fn test_column_view_navigate_right() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    view.select(0, 0);

    let child_entries = vec![
        create_test_entry("child1", false),
        create_test_entry("child2", false),
    ];
    view.set_column_entries(1, child_entries);

    assert!(view.navigate_right());
    assert_eq!(view.columns()[1].selected_index, Some(0));
}

#[test]
fn test_column_view_navigate_left() {
    let root = PathBuf::from("/root");
    let mut view = ColumnView::new(root);

    let entries = create_test_entries();
    view.set_column_entries(0, entries);

    view.select(0, 0);
    let child_entries = vec![create_test_entry("child1", false)];
    view.set_column_entries(1, child_entries);

    view.navigate_right();

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

use proptest::prelude::*;


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


fn arb_entries_with_dir() -> impl Strategy<Value = Vec<FileEntry>> {
    prop::collection::vec(arb_file_entry(), 1..10)
        .prop_filter("must have at least one directory", |entries| {
            entries.iter().any(|e| e.is_dir)
        })
}







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

        let dir_indices: Vec<usize> = entries.iter()
            .enumerate()
            .filter(|(_, e)| e.is_dir)
            .map(|(i, _)| i)
            .collect();

        if dir_indices.is_empty() {
            return Ok(());
        }

        let dir_idx = dir_indices[selection_idx % dir_indices.len()];

        view.select(0, dir_idx);

        prop_assert_eq!(view.column_count(), 2,
            "Selecting a directory should create a new column");

        let selected_entry = &entries[dir_idx];
        prop_assert_eq!(&view.columns()[1].path, &selected_entry.path,
            "New column path should match selected directory");

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

        let file_indices: Vec<usize> = entries.iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .map(|(i, _)| i)
            .collect();

        if file_indices.is_empty() {
            return Ok(());
        }

        let file_idx = file_indices[0];

        view.select(0, file_idx);

        prop_assert_eq!(view.column_count(), 1,
            "Selecting a file should not create a new column");

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

        if dir_indices1.len() > 1 {
            view.select(0, dir_indices1[1]);

            prop_assert_eq!(view.column_count(), 2,
                "Selecting in column 0 should truncate columns 2+");
        }
    }
}
