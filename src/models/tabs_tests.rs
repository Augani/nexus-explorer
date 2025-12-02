use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

#[test]
fn test_tab_creation() {
    let path = PathBuf::from("/home/user/documents");
    let tab = Tab::new(TabId::new(0), path.clone());
    
    assert_eq!(tab.id, TabId::new(0));
    assert_eq!(tab.path, path);
    assert_eq!(tab.title, "documents");
    assert!(!tab.needs_refresh);
    assert_eq!(tab.scroll_position, 0.0);
    assert_eq!(tab.selection, None);
}

#[test]
fn test_tab_title_from_root() {
    let path = PathBuf::from("/");
    let tab = Tab::new(TabId::new(0), path.clone());
    
    // Root path should use the path string as title
    assert_eq!(tab.title, "/");
}

#[test]
fn test_tab_set_path() {
    let mut tab = Tab::new(TabId::new(0), PathBuf::from("/home/user"));
    tab.set_path(PathBuf::from("/home/user/downloads"));
    
    assert_eq!(tab.path, PathBuf::from("/home/user/downloads"));
    assert_eq!(tab.title, "downloads");
}

#[test]
fn test_tab_needs_refresh() {
    let mut tab = Tab::new(TabId::new(0), PathBuf::from("/home"));
    
    assert!(!tab.needs_refresh);
    tab.mark_needs_refresh();
    assert!(tab.needs_refresh);
    tab.clear_needs_refresh();
    assert!(!tab.needs_refresh);
}

#[test]
fn test_tab_state_creation() {
    let path = PathBuf::from("/home/user");
    let state = TabState::new(path.clone());
    
    assert_eq!(state.tab_count(), 1);
    assert_eq!(state.active_index(), 0);
    assert_eq!(state.active_tab().path, path);
}

#[test]
fn test_tab_state_open_tab() {
    let mut state = TabState::new(PathBuf::from("/home"));
    let initial_count = state.tab_count();
    
    let new_id = state.open_tab(PathBuf::from("/home/user"));
    
    assert_eq!(state.tab_count(), initial_count + 1);
    assert_eq!(state.active_tab().id, new_id);
    assert_eq!(state.active_tab().path, PathBuf::from("/home/user"));
}

#[test]
fn test_tab_state_close_tab() {
    let mut state = TabState::new(PathBuf::from("/home"));
    let id1 = state.active_tab_id();
    let id2 = state.open_tab(PathBuf::from("/home/user"));
    
    assert_eq!(state.tab_count(), 2);
    
    // Close the second tab
    assert!(state.close_tab(id2));
    assert_eq!(state.tab_count(), 1);
    assert_eq!(state.active_tab().id, id1);
}

#[test]
fn test_tab_state_close_last_tab_opens_home() {
    let mut state = TabState::new(PathBuf::from("/tmp"));
    let id = state.active_tab_id();
    
    // Closing the last tab should open home directory instead
    assert!(state.close_tab(id));
    assert_eq!(state.tab_count(), 1);
    
    // The tab should now point to home directory
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    assert_eq!(state.active_tab().path, home);
}

#[test]
fn test_tab_state_switch_to() {
    let mut state = TabState::new(PathBuf::from("/home"));
    let id1 = state.active_tab_id();
    let id2 = state.open_tab(PathBuf::from("/tmp"));
    
    assert_eq!(state.active_tab().id, id2);
    
    assert!(state.switch_to(id1));
    assert_eq!(state.active_tab().id, id1);
    
    // Switch to non-existent tab should fail
    assert!(!state.switch_to(TabId::new(999)));
}

#[test]
fn test_tab_state_switch_to_index() {
    let mut state = TabState::new(PathBuf::from("/home"));
    state.open_tab(PathBuf::from("/tmp"));
    state.open_tab(PathBuf::from("/var"));
    
    assert!(state.switch_to_index(0));
    assert_eq!(state.active_index(), 0);
    
    assert!(state.switch_to_index(2));
    assert_eq!(state.active_index(), 2);
    
    // Invalid index should fail
    assert!(!state.switch_to_index(10));
}

#[test]
fn test_tab_state_next_prev_tab() {
    let mut state = TabState::new(PathBuf::from("/home"));
    state.open_tab(PathBuf::from("/tmp"));
    state.open_tab(PathBuf::from("/var"));
    
    state.switch_to_index(0);
    
    state.next_tab();
    assert_eq!(state.active_index(), 1);
    
    state.next_tab();
    assert_eq!(state.active_index(), 2);
    
    // Should wrap around
    state.next_tab();
    assert_eq!(state.active_index(), 0);
    
    // Previous should also wrap
    state.prev_tab();
    assert_eq!(state.active_index(), 2);
}

#[test]
fn test_tab_state_update_active_path() {
    let mut state = TabState::new(PathBuf::from("/home"));
    state.update_active_path(PathBuf::from("/home/user/documents"));
    
    assert_eq!(state.active_tab().path, PathBuf::from("/home/user/documents"));
    assert_eq!(state.active_tab().title, "documents");
}

#[test]
fn test_tab_state_close_active_tab() {
    let mut state = TabState::new(PathBuf::from("/home"));
    state.open_tab(PathBuf::from("/tmp"));
    
    assert_eq!(state.tab_count(), 2);
    assert!(state.close_active_tab());
    assert_eq!(state.tab_count(), 1);
}

#[test]
fn test_tab_state_get_tab() {
    let mut state = TabState::new(PathBuf::from("/home"));
    let id = state.open_tab(PathBuf::from("/tmp"));
    
    let tab = state.get_tab(id);
    assert!(tab.is_some());
    assert_eq!(tab.unwrap().path, PathBuf::from("/tmp"));
    
    // Non-existent tab
    assert!(state.get_tab(TabId::new(999)).is_none());
}

fn arb_path() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec("[a-z]{1,10}", 1..5)
        .prop_map(|parts| {
            let mut path = PathBuf::from("/");
            for part in parts {
                path.push(part);
            }
            path
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: ui-enhancements, Property 28: Tab Open Increases Count**
    /// **Validates: Requirements 9.2**
    ///
    /// *For any* TabState and valid path, opening a new tab SHALL increase
    /// the tab count by exactly one.
    #[test]
    fn prop_tab_open_increases_count(
        initial_path in arb_path(),
        new_paths in prop::collection::vec(arb_path(), 1..10),
    ) {
        let mut state = TabState::new(initial_path);
        
        for path in new_paths {
            let count_before = state.tab_count();
            let _id = state.open_tab(path);
            let count_after = state.tab_count();
            
            prop_assert_eq!(
                count_after, count_before + 1,
                "Opening a tab should increase count by 1"
            );
        }
    }

    /// **Feature: ui-enhancements, Property 29: Tab Close Decreases Count**
    /// **Validates: Requirements 9.4**
    ///
    /// *For any* TabState with more than one tab, closing a tab SHALL decrease
    /// the tab count by exactly one.
    #[test]
    fn prop_tab_close_decreases_count(
        initial_path in arb_path(),
        extra_paths in prop::collection::vec(arb_path(), 1..5),
    ) {
        let mut state = TabState::new(initial_path);
        
        // Open additional tabs
        let mut ids: Vec<TabId> = vec![state.active_tab_id()];
        for path in extra_paths {
            ids.push(state.open_tab(path));
        }
        
        // Close tabs (except the last one, which has special behavior)
        while state.tab_count() > 1 {
            let count_before = state.tab_count();
            let id_to_close = state.tabs()[0].id;
            
            let closed = state.close_tab(id_to_close);
            prop_assert!(closed, "Tab should be closeable");
            
            let count_after = state.tab_count();
            prop_assert_eq!(
                count_after, count_before - 1,
                "Closing a tab should decrease count by 1"
            );
        }
    }

    /// **Feature: ui-enhancements, Property 30: Tab Title Matches Directory**
    /// **Validates: Requirements 9.5**
    ///
    /// *For any* tab, the title SHALL match the directory name of the tab's path.
    #[test]
    fn prop_tab_title_matches_directory(
        path in arb_path(),
    ) {
        let tab = Tab::new(TabId::new(0), path.clone());
        
        let expected_title = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        
        prop_assert_eq!(
            tab.title, expected_title,
            "Tab title should match directory name"
        );
    }

    /// Property: Active tab is always valid
    /// *For any* sequence of tab operations, the active tab index SHALL always
    /// point to a valid tab.
    #[test]
    fn prop_active_tab_always_valid(
        initial_path in arb_path(),
        operations in prop::collection::vec(
            prop_oneof![
                arb_path().prop_map(|p| TabOp::Open(p)),
                Just(TabOp::CloseActive),
                Just(TabOp::Next),
                Just(TabOp::Prev),
                (0usize..10).prop_map(|i| TabOp::SwitchTo(i)),
            ],
            0..20
        ),
    ) {
        let mut state = TabState::new(initial_path);
        
        for op in operations {
            match op {
                TabOp::Open(path) => { state.open_tab(path); }
                TabOp::CloseActive => { state.close_active_tab(); }
                TabOp::Next => { state.next_tab(); }
                TabOp::Prev => { state.prev_tab(); }
                TabOp::SwitchTo(idx) => { state.switch_to_index(idx); }
            }
            
            // Active index should always be valid
            prop_assert!(
                state.active_index() < state.tab_count(),
                "Active index {} should be < tab count {}",
                state.active_index(), state.tab_count()
            );
            
            // Should always have at least one tab
            prop_assert!(
                state.tab_count() >= 1,
                "Should always have at least one tab"
            );
        }
    }

    /// Property: Tab IDs are unique
    /// *For any* TabState, all tab IDs SHALL be unique.
    #[test]
    fn prop_tab_ids_unique(
        initial_path in arb_path(),
        paths in prop::collection::vec(arb_path(), 0..10),
    ) {
        let mut state = TabState::new(initial_path);
        
        for path in paths {
            state.open_tab(path);
        }
        
        let ids: Vec<TabId> = state.tabs().iter().map(|t| t.id).collect();
        let unique_ids: std::collections::HashSet<TabId> = ids.iter().cloned().collect();
        
        prop_assert_eq!(
            ids.len(), unique_ids.len(),
            "All tab IDs should be unique"
        );
    }
}

#[derive(Debug, Clone)]
enum TabOp {
    Open(PathBuf),
    CloseActive,
    Next,
    Prev,
    SwitchTo(usize),
}
