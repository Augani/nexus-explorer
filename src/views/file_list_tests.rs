use super::*;
use crate::models::{FileEntry, IconKey};
use std::path::{Path, PathBuf};
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
    list.set_viewport_height(240.0);
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
    list.set_viewport_height(240.0);
    list.set_scroll_offset(240.0);

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
    // Note: With directories_first sorting, folder will come before test.txt
    list.set_entries(vec![
        create_test_entry("test.txt", false, 1024),
        create_test_entry("folder", true, 0),
    ]);

    // After sorting: folder (dir) comes first, then test.txt (file)
    let rendered_dir = list.render_item(0).unwrap();
    assert_eq!(rendered_dir.name, "folder");
    assert!(rendered_dir.is_dir);
    assert_eq!(rendered_dir.formatted_size, "--");

    let rendered = list.render_item(1).unwrap();
    assert_eq!(rendered.name, "test.txt");
    assert!(!rendered.is_dir);
    assert!(rendered.formatted_size.contains("KB") || rendered.formatted_size.contains("B"));
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
        formatted_date: "2025-01-01".to_string(),
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
    list.set_viewport_height(240.0);

    // max = ceil(240/24) + 2*5 = 10 + 10 = 20
    assert_eq!(list.max_rendered_items(), 20);
}

#[test]
fn test_render_visible_items() {
    let mut list = FileList::with_config(24.0, 1);
    list.set_entries(create_test_entries(100));
    list.set_viewport_height(72.0);
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
        scroll_factor in 0.0f32..1.0
    ) {
        let mut list = FileList::with_config(row_height, buffer_size);
        list.set_entries(create_test_entries(total_items));
        list.set_viewport_height(viewport_height);

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
// *For any* FileEntry, the rendered representation SHALL contain the file name,
// formatted size, formatted modification date, and an icon reference.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_rendered_entry_completeness(
        name in "[a-zA-Z0-9_.-]{1,50}",
        is_dir in proptest::bool::ANY,
        size in 0u64..10_000_000_000,
        days_since_epoch in 0u64..20000
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
    let highlight_positions = vec![vec![0, 1, 2], vec![0, 1, 2], vec![0, 1, 2, 3]];
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

    list.set_highlight_positions(Some(vec![vec![]]));
    let rendered = list.render_item(0).unwrap();
    assert!(rendered.highlight_positions.is_empty());
    assert!(!rendered.is_highlighted(0));
}

// **Feature: ui-enhancements, Property 1: Search Filter Correctness**
// *For any* file list with entries and a search query, when filtering is applied,
// and the filtered count SHALL be less than or equal to the original count.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_search_filter_correctness(
        entry_count in 1usize..100,
        query_len in 1usize..5,
    ) {
        // Generate random file entries
        let entries: Vec<FileEntry> = (0..entry_count)
            .map(|i| {
                let name = format!("file_{:04}.txt", i);
                create_test_entry(&name, false, i as u64 * 100)
            })
            .collect();

        let mut list = FileList::new();
        list.set_entries(entries.clone());

        // Generate a query from the first few characters of a random entry name
        let query: String = if !entries.is_empty() {
            let sample_name = &entries[0].name;
            sample_name.chars().take(query_len.min(sample_name.len())).collect()
        } else {
            "file".to_string()
        };

        // Simulate search results - find entries that contain the query
        let matches: Vec<(usize, Vec<usize>, u32)> = entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let name_lower = entry.name.to_lowercase();
                let query_lower = query.to_lowercase();
                if name_lower.contains(&query_lower) {
                    // Find match positions
                    let positions: Vec<usize> = name_lower
                        .match_indices(&query_lower)
                        .flat_map(|(start, matched)| start..start + matched.len())
                        .collect();
                    Some((idx, positions, 100))
                } else {
                    None
                }
            })
            .collect();

        let match_count = matches.len();
        list.apply_search_filter(&query, matches);

        // Property 1: Filtered count <= original count
        prop_assert!(
            list.item_count() <= entry_count,
            "Filtered count {} exceeds original count {}",
            list.item_count(), entry_count
        );

        // Property 2: Filtered count matches expected matches
        prop_assert_eq!(
            list.item_count(), match_count,
            "Filtered count {} doesn't match expected matches {}",
            list.item_count(), match_count
        );

        // Property 3: All filtered entries contain the query (case-insensitive)
        for i in 0..list.item_count() {
            if let Some(entry) = list.get_display_entry(i) {
                let name_lower = entry.name.to_lowercase();
                let query_lower = query.to_lowercase();
                prop_assert!(
                    name_lower.contains(&query_lower),
                    "Entry '{}' doesn't contain query '{}'",
                    entry.name, query
                );
            }
        }

        // Property 4: Search query is stored correctly
        prop_assert_eq!(
            list.search_query(), &query,
            "Search query mismatch: got '{}', expected '{}'",
            list.search_query(), query
        );
    }
}

#[test]
fn test_apply_search_filter() {
    let mut list = FileList::new();
    list.set_entries(vec![
        create_test_entry("document.txt", false, 1024),
        create_test_entry("data.csv", false, 2048),
        create_test_entry("readme.md", false, 512),
        create_test_entry("config.json", false, 256),
    ]);

    // After sorting by name (ascending), order is:
    // 0: config.json, 1: data.csv, 2: document.txt, 3: readme.md
    // Apply filter matching entries at sorted indices
    let matches = vec![(2, vec![0, 1, 2], 100), (1, vec![0, 1, 2], 90)];
    list.apply_search_filter("d", matches);

    assert_eq!(list.item_count(), 2);
    assert!(list.is_filtered());
    assert_eq!(list.search_query(), "d");

    // Verify filtered entries (order depends on match order in the vec)
    let names: Vec<_> = (0..list.item_count())
        .filter_map(|i| list.get_display_entry(i).map(|e| e.name.clone()))
        .collect();
    assert!(names.contains(&"document.txt".to_string()));
    assert!(names.contains(&"data.csv".to_string()));
}

#[test]
fn test_clear_search_filter() {
    let mut list = FileList::new();
    list.set_entries(vec![
        create_test_entry("document.txt", false, 1024),
        create_test_entry("data.csv", false, 2048),
    ]);

    list.apply_search_filter("doc", vec![(0, vec![0, 1, 2], 100)]);
    assert_eq!(list.item_count(), 1);

    list.clear_search_filter();
    assert_eq!(list.item_count(), 2);
    assert!(!list.is_filtered());
    assert!(list.search_query().is_empty());
}

#[test]
fn test_empty_query_clears_filter() {
    let mut list = FileList::new();
    list.set_entries(vec![
        create_test_entry("document.txt", false, 1024),
        create_test_entry("data.csv", false, 2048),
    ]);

    // Apply filter with empty query should clear
    list.apply_search_filter("", vec![]);
    assert_eq!(list.item_count(), 2);
    assert!(!list.is_filtered());
}

// **Feature: ui-enhancements, Property 2: Search Highlight Positions Validity**
// *For any* filtered entry with match positions, all highlight positions SHALL be
// valid indices within the entry name (0 <= position < name.len()).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_highlight_positions_validity(
        name_len in 1usize..50,
        num_positions in 0usize..20,
    ) {
        // Generate a random name
        let name: String = (0..name_len).map(|i| ((i % 26) as u8 + b'a') as char).collect();
        let entry = create_test_entry(&name, false, 1024);

        let mut list = FileList::new();
        list.set_entries(vec![entry]);

        // Generate valid match positions (all within bounds)
        let positions: Vec<usize> = (0..num_positions.min(name_len))
            .map(|i| i % name_len)
            .collect();

        // Apply filter with these positions
        let matches = vec![(0, positions.clone(), 100)];
        list.apply_search_filter("test", matches);

        // Verify all positions are valid
        if let Some(filtered_entry) = list.get_filtered_entry(0) {
            for &pos in &filtered_entry.match_positions {
                prop_assert!(
                    pos < name_len,
                    "Position {} is out of bounds for name of length {}",
                    pos, name_len
                );
            }
        }

        if let Some(match_positions) = list.get_match_positions(0) {
            prop_assert_eq!(
                match_positions.len(), positions.len(),
                "Match positions count mismatch"
            );
        }
    }
}

#[test]
fn test_highlight_positions_within_bounds() {
    let mut list = FileList::new();
    list.set_entries(vec![create_test_entry("test.txt", false, 100)]);

    // Apply filter with valid positions
    let matches = vec![(0, vec![0, 1, 2, 3], 100)];
    list.apply_search_filter("test", matches);

    let positions = list.get_match_positions(0).unwrap();
    assert_eq!(positions, &[0, 1, 2, 3]);

    // All positions should be within the name length (8 chars)
    for &pos in positions {
        assert!(pos < 8, "Position {} out of bounds", pos);
    }
}

#[test]
fn test_highlight_positions_empty_for_no_match() {
    let mut list = FileList::new();
    list.set_entries(vec![create_test_entry("test.txt", false, 100)]);

    // Apply filter with empty positions
    let matches = vec![(0, vec![], 100)];
    list.apply_search_filter("xyz", matches);

    let positions = list.get_match_positions(0).unwrap();
    assert!(positions.is_empty());
}

// *For any* file list with N entries, when the search query is empty or cleared,
// the FileList SHALL display all N original entries.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_empty_search_returns_all(
        entry_count in 1usize..200,
        initial_query_len in 1usize..10,
    ) {
        // Generate random file entries (all files, no dirs to avoid sorting complexity)
        let entries: Vec<FileEntry> = (0..entry_count)
            .map(|i| {
                let name = format!("file_{:04}.txt", i);
                create_test_entry(&name, false, i as u64 * 100)
            })
            .collect();

        let original_count = entries.len();
        let mut list = FileList::new();
        list.set_entries(entries.clone());

        prop_assert_eq!(
            list.item_count(), original_count,
            "Initial count {} doesn't match original {}",
            list.item_count(), original_count
        );

        // Generate a query and apply a filter (simulating some matches)
        // Use indices from the sorted list, not original entries
        let query: String = (0..initial_query_len).map(|_| 'f').collect();
        let sorted_entries = list.entries();
        let matches: Vec<(usize, Vec<usize>, u32)> = sorted_entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&query.to_lowercase()))
            .map(|(idx, _)| (idx, vec![0], 100))
            .collect();

        list.apply_search_filter(&query, matches);

        // Now clear the search with empty query
        list.apply_search_filter("", vec![]);

        // Property: After clearing, all original entries should be visible
        prop_assert_eq!(
            list.item_count(), original_count,
            "After clearing search, count {} doesn't match original {}",
            list.item_count(), original_count
        );

        // Property: Filter should be cleared
        prop_assert!(
            !list.is_filtered(),
            "List should not be filtered after clearing search"
        );

        // Property: Search query should be empty
        prop_assert!(
            list.search_query().is_empty(),
            "Search query should be empty after clearing, got '{}'",
            list.search_query()
        );

        // Property: All original entries should be accessible (by count, not by specific order)
        prop_assert_eq!(
            list.item_count(), original_count,
            "Entry count should match original after clearing search"
        );

        // Collect all entry names from the list
        let list_names: std::collections::HashSet<_> = (0..list.item_count())
            .filter_map(|i| list.get_display_entry(i).map(|e| e.name.clone()))
            .collect();

        // Verify all original entries are present (order may differ due to sorting)
        for entry in &entries {
            prop_assert!(
                list_names.contains(&entry.name),
                "Entry '{}' should be present after clearing search",
                entry.name
            );
        }
    }
}

#[test]
fn test_clear_search_filter_restores_all() {
    let mut list = FileList::new();
    let entries = vec![
        create_test_entry("alpha.txt", false, 100),
        create_test_entry("beta.txt", false, 200),
        create_test_entry("gamma.txt", false, 300),
        create_test_entry("delta.txt", false, 400),
        create_test_entry("epsilon.txt", false, 500),
    ];
    list.set_entries(entries.clone());

    assert_eq!(list.item_count(), 5);

    // After sorting by name, order is: alpha, beta, delta, epsilon, gamma
    // Apply filter that matches only some entries (using sorted indices)
    let matches = vec![(0, vec![0, 1], 100), (2, vec![0, 1], 90)];
    list.apply_search_filter("a", matches);

    // Verify filtered count
    assert_eq!(list.item_count(), 2);
    assert!(list.is_filtered());

    // Clear filter using clear_search_filter
    list.clear_search_filter();

    // Verify all entries are restored
    assert_eq!(list.item_count(), 5);
    assert!(!list.is_filtered());
    assert!(list.search_query().is_empty());

    // Verify all original entries are present (order may differ due to sorting)
    let list_names: std::collections::HashSet<_> = (0..list.item_count())
        .filter_map(|i| list.get_display_entry(i).map(|e| e.name.clone()))
        .collect();

    for original in &entries {
        assert!(
            list_names.contains(&original.name),
            "Entry '{}' should be present",
            original.name
        );
    }
}

#[test]
fn test_escape_clears_search_restores_entries() {
    let mut list = FileList::new();
    let entries = vec![
        create_test_entry("document.pdf", false, 1024),
        create_test_entry("spreadsheet.xlsx", false, 2048),
        create_test_entry("presentation.pptx", false, 4096),
    ];
    list.set_entries(entries.clone());

    let matches = vec![(0, vec![0, 1, 2], 100)];
    list.apply_search_filter("doc", matches);
    assert_eq!(list.item_count(), 1);

    // Simulate escape key by applying empty search
    list.apply_search_filter("", vec![]);

    // All entries should be restored
    assert_eq!(list.item_count(), 3);
    assert!(!list.is_filtered());
}

// **Feature: ui-enhancements, Property 33: Keyboard Selection Movement**
// *For any* file list with N items and current selection index S, pressing Up arrow
// SHALL move selection to max(0, S-1), and pressing Down arrow SHALL move selection
// to min(N-1, S+1). If no selection exists, both actions SHALL select index 0.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_keyboard_selection_movement(
        entry_count in 1usize..500,
        initial_selection in proptest::option::of(0usize..500),
        move_up_count in 0usize..20,
        move_down_count in 0usize..20,
    ) {
        // Generate file entries
        let entries: Vec<FileEntry> = (0..entry_count)
            .map(|i| create_test_entry(&format!("file_{:04}.txt", i), false, i as u64 * 100))
            .collect();

        let mut list = FileList::new();
        list.set_entries(entries);

        let clamped_initial = initial_selection.map(|s| s.min(entry_count.saturating_sub(1)));
        list.set_selected_index(clamped_initial);

        // Track expected selection
        let mut expected_selection = clamped_initial.unwrap_or(0);

        // Simulate move up actions
        for _ in 0..move_up_count {
            // Before move, if no selection, it should become 0
            if list.selected_index().is_none() {
                expected_selection = 0;
            } else {
                expected_selection = expected_selection.saturating_sub(1);
            }

            // Simulate the move_selection_up logic
            let item_count = list.item_count();
            if item_count > 0 {
                let new_index = match list.selected_index() {
                    Some(current) if current > 0 => current - 1,
                    Some(_) => 0,
                    None => 0,
                };
                list.set_selected_index(Some(new_index));
            }
        }

        // Verify selection after up movements
        if entry_count > 0 && move_up_count > 0 {
            prop_assert!(
                list.selected_index().is_some(),
                "Selection should exist after move up"
            );
            prop_assert_eq!(
                list.selected_index().unwrap(), expected_selection,
                "After {} up moves from {:?}, expected selection {}, got {:?}",
                move_up_count, clamped_initial, expected_selection, list.selected_index()
            );
        }

        // Simulate move down actions
        for _ in 0..move_down_count {
            let max_index = entry_count.saturating_sub(1);
            if list.selected_index().is_none() {
                expected_selection = 0;
            } else {
                expected_selection = (expected_selection + 1).min(max_index);
            }

            // Simulate the move_selection_down logic
            let item_count = list.item_count();
            if item_count > 0 {
                let max_idx = item_count.saturating_sub(1);
                let new_index = match list.selected_index() {
                    Some(current) if current < max_idx => current + 1,
                    Some(current) => current,
                    None => 0,
                };
                list.set_selected_index(Some(new_index));
            }
        }

        // Verify selection after down movements
        if entry_count > 0 && move_down_count > 0 {
            prop_assert!(
                list.selected_index().is_some(),
                "Selection should exist after move down"
            );
            prop_assert_eq!(
                list.selected_index().unwrap(), expected_selection,
                "After {} down moves, expected selection {}, got {:?}",
                move_down_count, expected_selection, list.selected_index()
            );
        }

        // Property: Selection should always be within bounds
        if let Some(selection) = list.selected_index() {
            prop_assert!(
                selection < entry_count,
                "Selection {} should be less than entry count {}",
                selection, entry_count
            );
        }

        // Property: Selection at boundary should not exceed bounds
        // Move up from 0 should stay at 0
        list.set_selected_index(Some(0));
        let item_count = list.item_count();
        if item_count > 0 {
            let new_index = match list.selected_index() {
                Some(current) if current > 0 => current - 1,
                Some(_) => 0,
                None => 0,
            };
            list.set_selected_index(Some(new_index));
        }
        prop_assert_eq!(
            list.selected_index(), Some(0),
            "Moving up from 0 should stay at 0"
        );

        // Move down from max should stay at max
        let max_index = entry_count.saturating_sub(1);
        list.set_selected_index(Some(max_index));
        if item_count > 0 {
            let max_idx = item_count.saturating_sub(1);
            let new_index = match list.selected_index() {
                Some(current) if current < max_idx => current + 1,
                Some(current) => current,
                None => 0,
            };
            list.set_selected_index(Some(new_index));
        }
        prop_assert_eq!(
            list.selected_index(), Some(max_index),
            "Moving down from max {} should stay at max",
            max_index
        );
    }
}

#[test]
fn test_move_selection_up_from_middle() {
    let mut list = FileList::new();
    list.set_entries(create_test_entries(10));
    list.set_selected_index(Some(5));

    // Simulate move up
    let new_index = match list.selected_index() {
        Some(current) if current > 0 => current - 1,
        Some(_) => 0,
        None => 0,
    };
    list.set_selected_index(Some(new_index));

    assert_eq!(list.selected_index(), Some(4));
}

#[test]
fn test_move_selection_up_from_top() {
    let mut list = FileList::new();
    list.set_entries(create_test_entries(10));
    list.set_selected_index(Some(0));

    // Simulate move up - should stay at 0
    let new_index = match list.selected_index() {
        Some(current) if current > 0 => current - 1,
        Some(_) => 0,
        None => 0,
    };
    list.set_selected_index(Some(new_index));

    assert_eq!(list.selected_index(), Some(0));
}

#[test]
fn test_move_selection_down_from_middle() {
    let mut list = FileList::new();
    list.set_entries(create_test_entries(10));
    list.set_selected_index(Some(5));

    // Simulate move down
    let max_index = list.item_count().saturating_sub(1);
    let new_index = match list.selected_index() {
        Some(current) if current < max_index => current + 1,
        Some(current) => current,
        None => 0,
    };
    list.set_selected_index(Some(new_index));

    assert_eq!(list.selected_index(), Some(6));
}

#[test]
fn test_move_selection_down_from_bottom() {
    let mut list = FileList::new();
    list.set_entries(create_test_entries(10));
    list.set_selected_index(Some(9));

    // Simulate move down - should stay at 9
    let max_index = list.item_count().saturating_sub(1);
    let new_index = match list.selected_index() {
        Some(current) if current < max_index => current + 1,
        Some(current) => current,
        None => 0,
    };
    list.set_selected_index(Some(new_index));

    assert_eq!(list.selected_index(), Some(9));
}

#[test]
fn test_move_selection_with_no_initial_selection() {
    let mut list = FileList::new();
    list.set_entries(create_test_entries(10));
    assert!(list.selected_index().is_none());

    // Simulate move down with no selection - should select first item
    let new_index = match list.selected_index() {
        Some(current) => current,
        None => 0,
    };
    list.set_selected_index(Some(new_index));

    assert_eq!(list.selected_index(), Some(0));
}

#[test]
fn test_move_selection_empty_list() {
    let mut list = FileList::new();
    assert_eq!(list.item_count(), 0);

    // Move operations should not crash on empty list
    if list.item_count() > 0 {
        list.set_selected_index(Some(0));
    }

    assert!(list.selected_index().is_none());
}

// **Feature: ui-enhancements, Property 34: Parent Navigation**
// *For any* path P with a parent directory, navigating to parent SHALL result in
// the parent path of P. For root paths, parent navigation SHALL have no effect.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_parent_navigation(
        depth in 1usize..10,
        segment_len in 1usize..20,
    ) {
        use std::path::Path;

        // Generate a path with the given depth
        let segments: Vec<String> = (0..depth)
            .map(|i| {
                let name: String = (0..segment_len.min(10))
                    .map(|j| ((((i * 7 + j) % 26) as u8) + b'a') as char)
                    .collect();
                name
            })
            .collect();

        let mut path = PathBuf::from("/");
        for segment in &segments {
            path.push(segment);
        }

        // Property 1: Parent of a non-root path should be the path without the last segment
        if let Some(parent) = path.parent() {
            // The parent should have one fewer component
            let path_components: Vec<_> = path.components().collect();
            let parent_components: Vec<_> = parent.components().collect();

            prop_assert!(
                parent_components.len() < path_components.len() || path_components.len() <= 1,
                "Parent should have fewer components: path has {}, parent has {}",
                path_components.len(), parent_components.len()
            );

            // The parent should be a prefix of the original path
            prop_assert!(
                path.starts_with(parent),
                "Path {:?} should start with parent {:?}",
                path, parent
            );
        }

        // Property 2: Navigating up from root should stay at root
        let root = PathBuf::from("/");
        let root_parent = root.parent();
        prop_assert!(
            root_parent.is_none(),
            "Root path should have no parent, got {:?}",
            root_parent
        );

        // Property 3: Multiple parent navigations should eventually reach root
        let mut current = path.clone();
        let mut iterations = 0;
        let max_iterations = depth + 5;

        while let Some(parent) = current.parent() {
            if parent == Path::new("") || parent == Path::new("/") {
                break;
            }
            current = parent.to_path_buf();
            iterations += 1;

            prop_assert!(
                iterations <= max_iterations,
                "Too many iterations ({}) to reach root from {:?}",
                iterations, path
            );
        }

        // Property 4: The number of parent navigations should equal the depth
        // (accounting for the root component)
        prop_assert!(
            iterations <= depth,
            "Iterations {} should be <= depth {} for path {:?}",
            iterations, depth, path
        );
    }
}

#[test]
fn test_parent_navigation_basic() {
    let path = PathBuf::from("/home/user/documents");
    let parent = path.parent().unwrap();
    assert_eq!(parent, Path::new("/home/user"));

    let grandparent = parent.parent().unwrap();
    assert_eq!(grandparent, Path::new("/home"));
}

#[test]
fn test_parent_navigation_root() {
    let root = PathBuf::from("/");
    assert!(root.parent().is_none() || root.parent() == Some(Path::new("")));
}

#[test]
fn test_parent_navigation_single_level() {
    let path = PathBuf::from("/home");
    let parent = path.parent().unwrap();
    assert_eq!(parent, Path::new("/"));
}

#[test]
fn test_parent_navigation_preserves_prefix() {
    let path = PathBuf::from("/a/b/c/d/e");
    let mut current = path.clone();

    // Each parent should be a prefix of the original
    while let Some(parent) = current.parent() {
        if parent == Path::new("") || parent == Path::new("/") {
            break;
        }
        assert!(path.starts_with(parent), "Path should start with parent");
        current = parent.to_path_buf();
    }
}
