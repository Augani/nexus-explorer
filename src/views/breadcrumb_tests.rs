use super::*;
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
    
    // Each segment index should return a valid path
    for i in 0..breadcrumb.segment_count() {
        assert!(breadcrumb.path_for_segment(i).is_some());
    }
    
    // Out of bounds should return None
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
    
    // Setting max_visible to 0 or 1 should clamp to 2
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

// Property-based tests will be added in separate tasks
