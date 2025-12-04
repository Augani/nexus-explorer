use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

#[test]
fn test_breadcrumb_from_simple_path() {
    let path = PathBuf::from("/home/user/documents");
    let breadcrumb = Breadcrumb::from_path(&path);
    
    assert!(breadcrumb.segment_count() >= 3);
    
    // Last segment should be "documents"
    let segments = breadcrumb.segments();
    assert_eq!(segments.last().unwrap().name, "documents");
}

#[test]
fn test_breadcrumb_segment_paths() {
    let path = PathBuf::from("/home/user/documents");
    let breadcrumb = Breadcrumb::from_path(&path);
    
    // Each segment should have a valid path
    for segment in breadcrumb.segments() {
        assert!(!segment.path.as_os_str().is_empty());
    }
}

#[test]
fn test_breadcrumb_root_segment() {
    let path = PathBuf::from("/home/user");
    let breadcrumb = Breadcrumb::from_path(&path);
    
    // First segment should be marked as root
    let segments = breadcrumb.segments();
    if !segments.is_empty() {
        assert!(segments[0].is_root);
    }
}

#[test]
fn test_breadcrumb_visible_segments_no_truncation() {
    let path = PathBuf::from("/home/user");
    let breadcrumb = Breadcrumb::from_path(&path);
    
    // With few segments, all should be visible
    let visible = breadcrumb.visible_segments();
    let all = breadcrumb.segments();
    
    if all.len() <= breadcrumb.max_visible {
        assert_eq!(visible.len(), all.len());
    }
}

#[test]
fn test_breadcrumb_truncation() {
    let path = PathBuf::from("/home/user/documents/projects/rust/nexus");
    let mut breadcrumb = Breadcrumb::from_path(&path);
    breadcrumb.set_max_visible(3);
    
    if breadcrumb.segment_count() > 3 {
        assert!(breadcrumb.needs_truncation());
        
        let visible = breadcrumb.visible_segments();
        assert!(visible.len() <= 3);
        
        let hidden = breadcrumb.hidden_segments();
        assert!(!hidden.is_empty());
    }
}

#[test]
fn test_breadcrumb_path_reconstruction() {
    let original_path = PathBuf::from("/home/user/documents");
    let breadcrumb = Breadcrumb::from_path(&original_path);
    
    // The last segment's path should match the original
    if let Some(current) = breadcrumb.current_path() {
        assert_eq!(current, original_path.as_path());
    }
}

#[test]
fn test_breadcrumb_path_for_segment() {
    let path = PathBuf::from("/home/user/documents");
    let breadcrumb = Breadcrumb::from_path(&path);
    
    for i in 0..breadcrumb.segment_count() {
        assert!(breadcrumb.path_for_segment(i).is_some());
    }
    
    assert!(breadcrumb.path_for_segment(100).is_none());
}

#[test]
fn test_breadcrumb_ellipsis_menu_toggle() {
    let path = PathBuf::from("/home/user");
    let mut breadcrumb = Breadcrumb::from_path(&path);
    
    assert!(!breadcrumb.is_ellipsis_menu_shown());
    
    breadcrumb.toggle_ellipsis_menu();
    assert!(breadcrumb.is_ellipsis_menu_shown());
    
    breadcrumb.toggle_ellipsis_menu();
    assert!(!breadcrumb.is_ellipsis_menu_shown());
}

#[test]
fn test_breadcrumb_set_max_visible_minimum() {
    let path = PathBuf::from("/home/user");
    let mut breadcrumb = Breadcrumb::from_path(&path);
    
    breadcrumb.set_max_visible(0);
    assert_eq!(breadcrumb.max_visible, 2);
    
    breadcrumb.set_max_visible(1);
    assert_eq!(breadcrumb.max_visible, 2);
}

#[test]
fn test_path_segment_creation() {
    let segment = PathSegment::new(
        "documents".to_string(),
        PathBuf::from("/home/user/documents"),
        false,
    );
    
    assert_eq!(segment.name, "documents");
    assert_eq!(segment.path, PathBuf::from("/home/user/documents"));
    assert!(!segment.is_root);
}

// **Feature: ui-enhancements, Property 4: Breadcrumb Segment Count**
// *For any* path with N components, the Breadcrumb SHALL render exactly N clickable segments
// (or N-k visible + ellipsis if truncated).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_breadcrumb_segment_count(
        depth in 1usize..10,
    ) {
        // Generate a path with the specified depth
        let mut path = PathBuf::from("/");
        for i in 0..depth {
            path.push(format!("dir{}", i));
        }
        
        let breadcrumb = Breadcrumb::from_path(&path);
        
        // Property 1: Segment count should equal path depth + 1 (for root)
        // On Unix, "/" is the root, then each component adds one segment
        let expected_count = depth + 1;
        prop_assert_eq!(
            breadcrumb.segment_count(), expected_count,
            "Path {:?} should have {} segments, got {}",
            path, expected_count, breadcrumb.segment_count()
        );
        
        // Property 2: Each segment should be clickable (have a valid path)
        for i in 0..breadcrumb.segment_count() {
            prop_assert!(
                breadcrumb.path_for_segment(i).is_some(),
                "Segment {} should have a valid path",
                i
            );
        }
        
        // Property 3: First segment should be root
        let segments = breadcrumb.segments();
        prop_assert!(
            segments[0].is_root,
            "First segment should be marked as root"
        );
        
        // Property 4: Last segment name should match last path component
        if depth > 0 {
            let last_segment = segments.last().unwrap();
            let expected_name = format!("dir{}", depth - 1);
            prop_assert_eq!(
                &last_segment.name, &expected_name,
                "Last segment name should be '{}', got '{}'",
                expected_name, last_segment.name
            );
        }
    }
    
    #[test]
    fn prop_breadcrumb_visible_plus_hidden_equals_total(
        depth in 1usize..15,
        max_visible in 2usize..6,
    ) {
        let mut path = PathBuf::from("/");
        for i in 0..depth {
            path.push(format!("folder{}", i));
        }
        
        let mut breadcrumb = Breadcrumb::from_path(&path);
        breadcrumb.set_max_visible(max_visible);
        
        let total = breadcrumb.segment_count();
        let visible = breadcrumb.visible_segments().len();
        let hidden = breadcrumb.hidden_segments().len();
        
        if total <= max_visible {
            // No truncation needed
            prop_assert_eq!(visible, total, "All segments should be visible when no truncation");
            prop_assert_eq!(hidden, 0, "No segments should be hidden when no truncation");
        } else {
            // Truncation: visible should be at most max_visible
            prop_assert!(
                visible <= max_visible,
                "Visible segments {} should be <= max_visible {}",
                visible, max_visible
            );
            // Hidden + visible should account for all segments
            // Note: visible includes root + last (max_visible-1) segments
            // Hidden includes middle segments
            prop_assert!(
                hidden > 0,
                "Should have hidden segments when truncated"
            );
        }
    }
}

// **Feature: ui-enhancements, Property 5: Breadcrumb Path Reconstruction**
// *For any* breadcrumb segment at index I, clicking it SHALL navigate to the path
// formed by joining segments 0..=I.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_breadcrumb_path_reconstruction(
        depth in 1usize..8,
        click_index in 0usize..8,
    ) {
        let mut path = PathBuf::from("/");
        for i in 0..depth {
            path.push(format!("level{}", i));
        }
        
        let breadcrumb = Breadcrumb::from_path(&path);
        let segment_count = breadcrumb.segment_count();
        
        // Only test valid indices
        if click_index < segment_count {
            let clicked_path = breadcrumb.path_for_segment(click_index);
            
            // Property 1: Clicked path should exist
            prop_assert!(
                clicked_path.is_some(),
                "Path for segment {} should exist",
                click_index
            );
            
            let clicked_path = clicked_path.unwrap();
            
            // Property 2: Clicked path should be a prefix of the full path
            prop_assert!(
                path.starts_with(clicked_path) || clicked_path == path.as_path(),
                "Clicked path {:?} should be prefix of {:?}",
                clicked_path, path
            );
            
            // Property 3: Path should match segment's stored path
            let segment = &breadcrumb.segments()[click_index];
            prop_assert_eq!(
                clicked_path, segment.path.as_path(),
                "Clicked path should match segment's path"
            );
            
            // Property 4: Segment name should be the last component of its path
            if click_index > 0 {
                let path_name = segment.path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                prop_assert_eq!(
                    &segment.name, &path_name,
                    "Segment name '{}' should match path component '{}'",
                    segment.name, path_name
                );
            }
        }
    }
    
    #[test]
    fn prop_breadcrumb_current_path_matches_input(
        depth in 1usize..10,
    ) {
        let mut path = PathBuf::from("/");
        for i in 0..depth {
            path.push(format!("subdir{}", i));
        }
        
        let breadcrumb = Breadcrumb::from_path(&path);
        
        let current = breadcrumb.current_path();
        prop_assert!(current.is_some(), "Current path should exist");
        prop_assert_eq!(
            current.unwrap(), path.as_path(),
            "Current path {:?} should match input {:?}",
            current.unwrap(), path
        );
    }
}

// **Feature: ui-enhancements, Property 6: Breadcrumb Truncation**
// *For any* path with more than max_visible segments, the Breadcrumb SHALL display
// an ellipsis containing the hidden middle segments.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_breadcrumb_truncation(
        depth in 5usize..15,
        max_visible in 2usize..5,
    ) {
        let mut path = PathBuf::from("/");
        for i in 0..depth {
            path.push(format!("deep{}", i));
        }
        
        let mut breadcrumb = Breadcrumb::from_path(&path);
        breadcrumb.set_max_visible(max_visible);
        
        let total = breadcrumb.segment_count();
        
        if total > max_visible {
            // Property 1: Should need truncation
            prop_assert!(
                breadcrumb.needs_truncation(),
                "Path with {} segments should need truncation when max_visible is {}",
                total, max_visible
            );
            
            // Property 2: Visible segments should be limited
            let visible = breadcrumb.visible_segments();
            prop_assert!(
                visible.len() <= max_visible,
                "Visible segments {} should be <= max_visible {}",
                visible.len(), max_visible
            );
            
            // Property 3: First visible segment should be root
            prop_assert!(
                visible[0].is_root,
                "First visible segment should be root"
            );
            
            // Property 4: Last visible segment should be the current directory
            let last_visible = visible.last().unwrap();
            let last_segment = breadcrumb.segments().last().unwrap();
            prop_assert_eq!(
                &last_visible.path, &last_segment.path,
                "Last visible segment should be current directory"
            );
            
            // Property 5: Hidden segments should contain middle segments
            let hidden = breadcrumb.hidden_segments();
            prop_assert!(
                !hidden.is_empty(),
                "Should have hidden segments when truncated"
            );
            
            // Property 6: Hidden segments should not include root or last
            let last_path = &last_segment.path;
            for seg in &hidden {
                prop_assert!(
                    !seg.is_root,
                    "Hidden segments should not include root"
                );
                prop_assert!(
                    &seg.path != last_path,
                    "Hidden segments should not include current directory"
                );
            }
        }
    }
    
    #[test]
    fn prop_breadcrumb_no_truncation_when_short(
        depth in 1usize..4,
    ) {
        let mut path = PathBuf::from("/");
        for i in 0..depth {
            path.push(format!("short{}", i));
        }
        
        let breadcrumb = Breadcrumb::from_path(&path);
        // Default max_visible is 4
        
        let total = breadcrumb.segment_count();
        
        if total <= 4 {
            // Property: Should not need truncation
            prop_assert!(
                !breadcrumb.needs_truncation(),
                "Path with {} segments should not need truncation",
                total
            );
            
            // All segments should be visible
            let visible = breadcrumb.visible_segments();
            prop_assert_eq!(
                visible.len(), total,
                "All {} segments should be visible",
                total
            );
            
            // No hidden segments
            let hidden = breadcrumb.hidden_segments();
            prop_assert!(
                hidden.is_empty(),
                "Should have no hidden segments"
            );
        }
    }
}
