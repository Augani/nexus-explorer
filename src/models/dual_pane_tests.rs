use crate::models::{DualPane, PaneSide, PaneState, FileEntry};
use std::path::PathBuf;
use std::time::SystemTime;
use proptest::prelude::*;

fn create_test_entry(name: &str, is_dir: bool) -> FileEntry {
    FileEntry::new(
        name.to_string(),
        PathBuf::from(format!("/test/{}", name)),
        is_dir,
        if is_dir { 0 } else { 1024 },
        SystemTime::now(),
    )
}

#[test]
fn test_pane_side_opposite() {
    assert_eq!(PaneSide::Left.opposite(), PaneSide::Right);
    assert_eq!(PaneSide::Right.opposite(), PaneSide::Left);
}

#[test]
fn test_pane_state_new() {
    let path = PathBuf::from("/home/user");
    let pane = PaneState::new(path.clone());
    
    assert_eq!(pane.path, path);
    assert!(pane.entries.is_empty());
    assert!(pane.selection.is_empty());
    assert_eq!(pane.scroll_offset, 0.0);
}

#[test]
fn test_pane_state_set_entries() {
    let mut pane = PaneState::new(PathBuf::from("/test"));
    pane.selection.push(0);
    
    let entries = vec![
        create_test_entry("file1.txt", false),
        create_test_entry("file2.txt", false),
    ];
    
    pane.set_entries(entries.clone());
    
    assert_eq!(pane.entries.len(), 2);
    assert!(pane.selection.is_empty());
}

#[test]
fn test_pane_state_navigate_to() {
    let mut pane = PaneState::new(PathBuf::from("/old"));
    pane.entries = vec![create_test_entry("file.txt", false)];
    pane.selection.push(0);
    pane.scroll_offset = 100.0;
    
    pane.navigate_to(PathBuf::from("/new"));
    
    assert_eq!(pane.path, PathBuf::from("/new"));
    assert!(pane.entries.is_empty());
    assert!(pane.selection.is_empty());
    assert_eq!(pane.scroll_offset, 0.0);
}

#[test]
fn test_pane_state_selection() {
    let mut pane = PaneState::new(PathBuf::from("/test"));
    pane.entries = vec![
        create_test_entry("file1.txt", false),
        create_test_entry("file2.txt", false),
        create_test_entry("file3.txt", false),
    ];
    
    pane.select(1);
    assert_eq!(pane.selection, vec![1]);
    assert!(pane.is_selected(1));
    assert!(!pane.is_selected(0));
    
    // Toggle selection
    pane.toggle_selection(2);
    assert_eq!(pane.selection, vec![1, 2]);
    
    pane.toggle_selection(1);
    assert_eq!(pane.selection, vec![2]);
    
    // Clear selection
    pane.clear_selection();
    assert!(pane.selection.is_empty());
}

#[test]
fn test_pane_state_selected_entries() {
    let mut pane = PaneState::new(PathBuf::from("/test"));
    let entries = vec![
        create_test_entry("file1.txt", false),
        create_test_entry("file2.txt", false),
        create_test_entry("file3.txt", false),
    ];
    pane.entries = entries;
    pane.selection = vec![0, 2];
    
    let selected = pane.selected_entries();
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].name, "file1.txt");
    assert_eq!(selected[1].name, "file3.txt");
}

#[test]
fn test_dual_pane_new() {
    let path = PathBuf::from("/home/user");
    let dual = DualPane::new(path.clone());
    
    assert_eq!(dual.left_pane().path, path);
    assert_eq!(dual.right_pane().path, path);
    assert_eq!(dual.active_side(), PaneSide::Left);
    assert!(!dual.is_enabled());
}

#[test]
fn test_dual_pane_with_paths() {
    let left = PathBuf::from("/left");
    let right = PathBuf::from("/right");
    let dual = DualPane::with_paths(left.clone(), right.clone());
    
    assert_eq!(dual.left_pane().path, left);
    assert_eq!(dual.right_pane().path, right);
}

#[test]
fn test_dual_pane_enable_disable() {
    let mut dual = DualPane::new(PathBuf::from("/"));
    
    assert!(!dual.is_enabled());
    
    dual.enable();
    assert!(dual.is_enabled());
    
    dual.disable();
    assert!(!dual.is_enabled());
    
    dual.toggle();
    assert!(dual.is_enabled());
    
    dual.toggle();
    assert!(!dual.is_enabled());
}

#[test]
fn test_dual_pane_switch_active() {
    let mut dual = DualPane::new(PathBuf::from("/"));
    
    assert_eq!(dual.active_side(), PaneSide::Left);
    
    dual.switch_active();
    assert_eq!(dual.active_side(), PaneSide::Right);
    
    dual.switch_active();
    assert_eq!(dual.active_side(), PaneSide::Left);
}

#[test]
fn test_dual_pane_set_active() {
    let mut dual = DualPane::new(PathBuf::from("/"));
    
    dual.set_active(PaneSide::Right);
    assert_eq!(dual.active_side(), PaneSide::Right);
    
    dual.set_active(PaneSide::Left);
    assert_eq!(dual.active_side(), PaneSide::Left);
}

#[test]
fn test_dual_pane_active_inactive_pane() {
    let mut dual = DualPane::with_paths(
        PathBuf::from("/left"),
        PathBuf::from("/right"),
    );
    
    // Left is active by default
    assert_eq!(dual.active_pane().path, PathBuf::from("/left"));
    assert_eq!(dual.inactive_pane().path, PathBuf::from("/right"));
    
    // Switch to right
    dual.switch_active();
    assert_eq!(dual.active_pane().path, PathBuf::from("/right"));
    assert_eq!(dual.inactive_pane().path, PathBuf::from("/left"));
}

#[test]
fn test_dual_pane_pane_access() {
    let mut dual = DualPane::with_paths(
        PathBuf::from("/left"),
        PathBuf::from("/right"),
    );
    
    assert_eq!(dual.pane(PaneSide::Left).path, PathBuf::from("/left"));
    assert_eq!(dual.pane(PaneSide::Right).path, PathBuf::from("/right"));
    
    dual.pane_mut(PaneSide::Left).navigate_to(PathBuf::from("/new-left"));
    assert_eq!(dual.left_pane().path, PathBuf::from("/new-left"));
}

#[test]
fn test_dual_pane_copy_move_operations() {
    let mut dual = DualPane::with_paths(
        PathBuf::from("/source"),
        PathBuf::from("/dest"),
    );
    
    // Add entries to left pane and select some
    dual.left_pane_mut().entries = vec![
        create_test_entry("file1.txt", false),
        create_test_entry("file2.txt", false),
    ];
    dual.left_pane_mut().selection = vec![0, 1];
    
    let copy_paths = dual.copy_to_other();
    assert_eq!(copy_paths.len(), 2);
    
    let move_paths = dual.move_to_other();
    assert_eq!(move_paths.len(), 2);
    
    assert_eq!(dual.destination_path(), &PathBuf::from("/dest"));
}

#[test]
fn test_dual_pane_sync_panes() {
    let mut dual = DualPane::with_paths(
        PathBuf::from("/active"),
        PathBuf::from("/inactive"),
    );
    
    dual.sync_panes();
    
    assert_eq!(dual.right_pane().path, PathBuf::from("/active"));
}

// Property-based tests

/// **Feature: ui-enhancements, Property 42: Dual Pane Independence**
/// **Validates: Requirements 22.2**
/// 
/// *For any* dual pane state, modifying one pane's path, entries, or selection
/// should not affect the other pane's state.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_dual_pane_independence(
        left_path in "[a-z]{1,10}",
        right_path in "[a-z]{1,10}",
        left_entries_count in 0usize..10,
        right_entries_count in 0usize..10,
        left_selection in proptest::collection::vec(0usize..10, 0..5),
        right_selection in proptest::collection::vec(0usize..10, 0..5),
    ) {
        let mut dual = DualPane::with_paths(
            PathBuf::from(format!("/{}", left_path)),
            PathBuf::from(format!("/{}", right_path)),
        );
        
        let left_entries: Vec<FileEntry> = (0..left_entries_count)
            .map(|i| create_test_entry(&format!("left_{}.txt", i), false))
            .collect();
        dual.left_pane_mut().set_entries(left_entries.clone());
        for &idx in &left_selection {
            if idx < left_entries_count {
                dual.left_pane_mut().toggle_selection(idx);
            }
        }
        
        let right_entries: Vec<FileEntry> = (0..right_entries_count)
            .map(|i| create_test_entry(&format!("right_{}.txt", i), false))
            .collect();
        dual.right_pane_mut().set_entries(right_entries.clone());
        for &idx in &right_selection {
            if idx < right_entries_count {
                dual.right_pane_mut().toggle_selection(idx);
            }
        }
        
        // Capture right pane state
        let right_path_before = dual.right_pane().path.clone();
        let right_entries_before = dual.right_pane().entries.len();
        let right_selection_before = dual.right_pane().selection.clone();
        
        // Modify left pane
        dual.left_pane_mut().navigate_to(PathBuf::from("/modified"));
        dual.left_pane_mut().set_entries(vec![create_test_entry("new.txt", false)]);
        dual.left_pane_mut().select(0);
        
        // Verify right pane is unchanged
        prop_assert_eq!(dual.right_pane().path.clone(), right_path_before);
        prop_assert_eq!(dual.right_pane().entries.len(), right_entries_before);
        prop_assert_eq!(&dual.right_pane().selection, &right_selection_before);
    }
}
