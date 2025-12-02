use super::*;
use crate::models::{FileEntry, IconKey};
use std::path::PathBuf;
use std::time::SystemTime;

fn create_test_entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
    FileEntry::new(
        name.to_string(),
        PathBuf::from(format!("/test/{}", name)),
        is_dir,
        size,
        SystemTime::now(),
    )
}

fn create_test_entries(count: usize) -> Vec<FileEntry> {
    (0..count)
        .map(|i| create_test_entry(&format!("file_{}", i), false, i as u64 * 1000))
        .collect()
}

#[test]
fn test_file_list_new() {
    let list = FileList::new();
    assert_eq!(list.item_count(), 0);
    assert_eq!(list.row_height(), DEFAULT_ROW_HEIGHT);
    assert_eq!(list.buffer_size(), DEFAULT_BUFFER_SIZE);
}

#[test]
fn test_file_list_with_config() {
    let list = FileList::with_config(32.0, 10);
    assert_eq!(list.row_height(), 32.0);
    assert_eq!(list.buffer_size(), 10);
}

#[test]
fn test_set_entries() {
    let mut list = FileList::new();
    let entries = create_test_entries(100);
    list.set_entries(entries);
    assert_eq!(list.item_count(), 100);
}

#[test]
fn test_visible_range_empty_list() {
    let list = FileList::new();
    let range = list.calculate_visible_range();
    assert_eq!(range.start, 0);
    assert_eq!(range.end, 0);
}


#[test]
fn test_visible_range_calculation() {
    let mut list = FileList::with_config(24.0, 2);
    list.set_entries(create_test_entries(100));
    list.set_viewport_height(240.0); // 10 visible rows
    list.set_scroll_offset(0.0);
    
    let range = list.calculate_visible_range();
    // start = 0 - buffer(2) = 0 (clamped)
    // end = 0 + 10 + buffer(2) = 12
    assert_eq!(range.start, 0);
    assert_eq!(range.end, 12);
}

#[test]
fn test_visible_range_with_scroll() {
    let mut list = FileList::with_config(24.0, 2);
    list.set_entries(create_test_entries(100));
    list.set_viewport_height(240.0); // 10 visible rows
    list.set_scroll_offset(240.0); // Scrolled down 10 rows
    
    let range = list.calculate_visible_range();
    // start_raw = 240/24 = 10, start = 10 - 2 = 8
    // end = 10 + 10 + 2 = 22
    assert_eq!(range.start, 8);
    assert_eq!(range.end, 22);
}

#[test]
fn test_visible_range_clamped_to_total() {
    let mut list = FileList::with_config(24.0, 2);
    list.set_entries(create_test_entries(15));
    list.set_viewport_height(240.0);
    list.set_scroll_offset(240.0);
    
    let range = list.calculate_visible_range();
    // end should be clamped to 15
    assert!(range.end <= 15);
}

#[test]
fn test_render_item() {
    let mut list = FileList::new();
    list.set_entries(vec![
        create_test_entry("test.txt", false, 1024),
        create_test_entry("folder", true, 0),
    ]);
    
    let rendered = list.render_item(0).unwrap();
    assert_eq!(rendered.name, "test.txt");
    assert!(!rendered.is_dir);
    assert!(rendered.formatted_size.contains("KB") || rendered.formatted_size.contains("B"));
    
    let rendered_dir = list.render_item(1).unwrap();
    assert_eq!(rendered_dir.name, "folder");
    assert!(rendered_dir.is_dir);
    assert_eq!(rendered_dir.formatted_size, "--");
}

#[test]
fn test_render_item_out_of_bounds() {
    let list = FileList::new();
    assert!(list.render_item(0).is_none());
}

#[test]
fn test_format_size() {
    assert_eq!(format_size(0, false), "0 B");
    assert_eq!(format_size(512, false), "512 B");
    assert_eq!(format_size(1024, false), "1.0 KB");
    assert_eq!(format_size(1536, false), "1.5 KB");
    assert_eq!(format_size(1048576, false), "1.0 MB");
    assert_eq!(format_size(1073741824, false), "1.0 GB");
    assert_eq!(format_size(1099511627776, false), "1.0 TB");
    assert_eq!(format_size(1000, true), "--");
}

#[test]
fn test_format_date() {
    let date = format_date(SystemTime::UNIX_EPOCH);
    assert_eq!(date, "1970-01-01");
}

#[test]
fn test_visible_range_methods() {
    let range = VisibleRange { start: 5, end: 15 };
    assert_eq!(range.len(), 10);
    assert!(!range.is_empty());
    assert!(range.contains(5));
    assert!(range.contains(14));
    assert!(!range.contains(15));
    assert!(!range.contains(4));
}

#[test]
fn test_rendered_entry_highlighting() {
    let entry = RenderedEntry {
        name: "test.txt".to_string(),
        formatted_size: "1 KB".to_string(),
        formatted_date: "2024-01-01".to_string(),
        icon_key: IconKey::GenericFile,
        is_dir: false,
        highlight_positions: vec![0, 2, 4],
    };
    
    assert!(entry.is_highlighted(0));
    assert!(!entry.is_highlighted(1));
    assert!(entry.is_highlighted(2));
    
    let highlights = entry.name_with_highlights();
    assert_eq!(highlights[0], ('t', true));
    assert_eq!(highlights[1], ('e', false));
    assert_eq!(highlights[2], ('s', true));
}

#[test]
fn test_max_rendered_items() {
    let mut list = FileList::with_config(24.0, 5);
    list.set_viewport_height(240.0); // 10 visible rows
    
    // max = ceil(240/24) + 2*5 = 10 + 10 = 20
    assert_eq!(list.max_rendered_items(), 20);
}

#[test]
fn test_render_visible_items() {
    let mut list = FileList::with_config(24.0, 1);
    list.set_entries(create_test_entries(100));
    list.set_viewport_height(72.0); // 3 visible rows
    list.set_scroll_offset(0.0);
    
    let visible = list.render_visible_items();
    // start = 0, end = 0 + 3 + 1 = 4
    assert_eq!(visible.len(), 4);
    assert_eq!(visible[0].0, 0);
    assert_eq!(visible[3].0, 3);
}


// Property-based tests using proptest
use proptest::prelude::*;

// **Feature: file-explorer-core, Property 5: Virtualization Bounds**
// **Validates: Requirements 2.1**
// 
// *For any* file list with N total items, viewport height H, and row height R, 
// the number of rendered items SHALL be at most `ceil(H / R) + buffer_size`, regardless of N.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_virtualization_bounds(
        total_items in 0usize..100000,
        viewport_height in 1.0f32..2000.0,
        row_height in 1.0f32..100.0,
        buffer_size in 0usize..20,
        scroll_offset in 0.0f32..10000.0
    ) {
        let mut list = FileList::with_config(row_height, buffer_size);
        list.set_entries(create_test_entries(total_items));
        list.set_viewport_height(viewport_height);
        list.set_scroll_offset(scroll_offset);
        
        let range = list.calculate_visible_range();
        let rendered_count = range.len();
        
        // Maximum rendered items should be bounded by viewport + buffer
        // regardless of total items
        let visible_rows = (viewport_height / row_height).ceil() as usize;
        let max_allowed = visible_rows + (buffer_size * 2);
        
        prop_assert!(
            rendered_count <= max_allowed,
            "Rendered {} items but max allowed is {} (visible_rows={}, buffer={})",
            rendered_count, max_allowed, visible_rows, buffer_size
        );
        
        // Also verify we don't exceed total items
        prop_assert!(
            rendered_count <= total_items,
            "Rendered {} items but only {} total items exist",
            rendered_count, total_items
        );
    }
}

// **Feature: file-explorer-core, Property 7: Visible Range Calculation**
// **Validates: Requirements 2.4**
// 
// *For any* viewport height H, row height R, scroll offset S, and total items N, 
// the calculated visible range `[start, end)` SHALL satisfy: 
// start = floor(S / R) - buffer (clamped to 0), 
// end = min(start_raw + ceil(H / R) + buffer, N).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_visible_range_calculation(
        total_items in 1usize..10000,
        viewport_height in 1.0f32..2000.0,
        row_height in 1.0f32..100.0,
        buffer_size in 0usize..20,
        scroll_factor in 0.0f32..1.0  // Fraction of total scrollable area
    ) {
        let mut list = FileList::with_config(row_height, buffer_size);
        list.set_entries(create_test_entries(total_items));
        list.set_viewport_height(viewport_height);
        
        // Calculate max scroll offset
        let total_height = total_items as f32 * row_height;
        let max_scroll = (total_height - viewport_height).max(0.0);
        let scroll_offset = scroll_factor * max_scroll;
        list.set_scroll_offset(scroll_offset);
        
        let range = list.calculate_visible_range();
        
        // Verify start calculation
        let start_raw = (scroll_offset / row_height).floor() as usize;
        let expected_start = start_raw.saturating_sub(buffer_size);
        prop_assert_eq!(
            range.start, expected_start,
            "Start mismatch: got {}, expected {} (scroll={}, row_height={}, buffer={})",
            range.start, expected_start, scroll_offset, row_height, buffer_size
        );
        
        // Verify end is clamped to total items
        prop_assert!(
            range.end <= total_items,
            "End {} exceeds total items {}",
            range.end, total_items
        );
        
        // Verify range is valid (start <= end)
        prop_assert!(
            range.start <= range.end,
            "Invalid range: start {} > end {}",
            range.start, range.end
        );
    }
}


// **Feature: file-explorer-core, Property 6: Rendered Entry Completeness**
// **Validates: Requirements 2.3**
// 
// *For any* FileEntry, the rendered representation SHALL contain the file name, 
// formatted size, formatted modification date, and an icon reference.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn prop_rendered_entry_completeness(
        name in "[a-zA-Z0-9_.-]{1,50}",
        is_dir in proptest::bool::ANY,
        size in 0u64..10_000_000_000,
        days_since_epoch in 0u64..20000  // ~55 years from epoch
    ) {
        use std::time::{Duration, UNIX_EPOCH};
        
        let modified = UNIX_EPOCH + Duration::from_secs(days_since_epoch * 86400);
        let entry = FileEntry::new(
            name.clone(),
            PathBuf::from(format!("/test/{}", name)),
            is_dir,
            size,
            modified,
        );
        
        let mut list = FileList::new();
        list.set_entries(vec![entry]);
        
        let rendered = list.render_item(0).expect("Should render valid entry");
        
        // Verify name is present and matches
        prop_assert_eq!(
            &rendered.name, &name,
            "Name mismatch: got '{}', expected '{}'",
            rendered.name, name
        );
        
        // Verify formatted_size is non-empty
        prop_assert!(
            !rendered.formatted_size.is_empty(),
            "Formatted size should not be empty"
        );
        
        // Verify directories show "--" for size
        if is_dir {
            prop_assert_eq!(
                &rendered.formatted_size, "--",
                "Directory size should be '--', got '{}'",
                &rendered.formatted_size
            );
        }
        
        // Verify formatted_date is non-empty and has valid format
        prop_assert!(
            !rendered.formatted_date.is_empty(),
            "Formatted date should not be empty"
        );
        
        // Date should be in YYYY-MM-DD format or "Unknown"
        let is_valid_date = rendered.formatted_date == "Unknown" 
            || (rendered.formatted_date.len() == 10 
                && rendered.formatted_date.chars().nth(4) == Some('-')
                && rendered.formatted_date.chars().nth(7) == Some('-'));
        prop_assert!(
            is_valid_date,
            "Date format invalid: '{}'",
            rendered.formatted_date
        );
        
        // Verify icon_key is appropriate for entry type
        match (is_dir, &rendered.icon_key) {
            (true, IconKey::Directory) => {},
            (false, IconKey::GenericFile) | (false, IconKey::Extension(_)) => {},
            _ => prop_assert!(
                false,
                "Icon key {:?} inappropriate for is_dir={}",
                rendered.icon_key, is_dir
            ),
        }
        
        // Verify is_dir flag matches
        prop_assert_eq!(
            rendered.is_dir, is_dir,
            "is_dir mismatch: got {}, expected {}",
            rendered.is_dir, is_dir
        );
    }
}


#[test]
fn test_search_result_highlighting_integration() {
    let mut list = FileList::new();
    list.set_entries(vec![
        create_test_entry("document.txt", false, 1024),
        create_test_entry("data.csv", false, 2048),
        create_test_entry("readme.md", false, 512),
    ]);
    
    // Simulate search results with match positions
    // "doc" matches positions 0,1,2 in "document.txt"
    // "dat" matches positions 0,1,2 in "data.csv"
    // "read" matches positions 0,1,2,3 in "readme.md"
    let highlight_positions = vec![
        vec![0, 1, 2],      // document.txt
        vec![0, 1, 2],      // data.csv
        vec![0, 1, 2, 3],   // readme.md
    ];
    list.set_highlight_positions(Some(highlight_positions));
    
    // Verify first entry highlighting
    let rendered = list.render_item(0).unwrap();
    assert_eq!(rendered.highlight_positions, vec![0, 1, 2]);
    assert!(rendered.is_highlighted(0));
    assert!(rendered.is_highlighted(1));
    assert!(rendered.is_highlighted(2));
    assert!(!rendered.is_highlighted(3));
    
    // Verify third entry highlighting
    let rendered = list.render_item(2).unwrap();
    assert_eq!(rendered.highlight_positions, vec![0, 1, 2, 3]);
    
    // Verify name_with_highlights returns correct flags
    let highlights = rendered.name_with_highlights();
    assert_eq!(highlights[0], ('r', true));
    assert_eq!(highlights[1], ('e', true));
    assert_eq!(highlights[2], ('a', true));
    assert_eq!(highlights[3], ('d', true));
    assert_eq!(highlights[4], ('m', false));
}

#[test]
fn test_search_result_highlighting_cleared() {
    let mut list = FileList::new();
    list.set_entries(vec![create_test_entry("test.txt", false, 100)]);
    
    // Set highlights
    list.set_highlight_positions(Some(vec![vec![0, 1]]));
    let rendered = list.render_item(0).unwrap();
    assert_eq!(rendered.highlight_positions, vec![0, 1]);
    
    // Clear highlights
    list.set_highlight_positions(None);
    let rendered = list.render_item(0).unwrap();
    assert!(rendered.highlight_positions.is_empty());
}

#[test]
fn test_search_result_highlighting_empty_positions() {
    let mut list = FileList::new();
    list.set_entries(vec![create_test_entry("test.txt", false, 100)]);
    
    // Set empty highlight positions (no matches)
    list.set_highlight_positions(Some(vec![vec![]]));
    let rendered = list.render_item(0).unwrap();
    assert!(rendered.highlight_positions.is_empty());
    assert!(!rendered.is_highlighted(0));
}
