use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

/// **Feature: file-explorer-core, Property 13: Search Pattern Update**
/// **Validates: Requirements 5.1**
/// 
/// *For any* pattern string P, after calling `set_pattern(P)`, the SearchEngine's 
/// internal pattern state SHALL equal P.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_search_pattern_update(pattern in "[a-zA-Z0-9_\\-\\.]{0,50}") {
        let mut engine = SearchEngine::new();
        
        engine.set_pattern(&pattern);
        
        prop_assert_eq!(engine.pattern(), pattern.as_str());
        prop_assert_eq!(engine.is_active(), !pattern.is_empty());
    }
}

/// **Feature: file-explorer-core, Property 15: Injected Items Searchable**
/// **Validates: Requirements 5.5**
/// 
/// *For any* path P injected into the SearchEngine, a subsequent search with a pattern 
/// matching P's filename SHALL include P in the results.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_injected_items_searchable(
        filename in "[a-zA-Z][a-zA-Z0-9]{2,10}",
        dir_parts in prop::collection::vec("[a-zA-Z][a-zA-Z0-9]{1,5}", 1..3)
    ) {
        let mut engine = SearchEngine::new();
        
        let mut path = PathBuf::from("/");
        for part in &dir_parts {
            path.push(part);
        }
        path.push(&filename);
        
        engine.inject(path.clone());
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Search for the exact filename
        engine.set_pattern(&filename);
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        let snapshot = engine.snapshot();
        
        // The injected path should be found in results
        let found = snapshot.matches.iter().any(|m| m.path == path);
        prop_assert!(found, "Injected path {:?} not found in search results for pattern '{}'", path, filename);
    }
}

/// **Feature: file-explorer-core, Property 14: Match Positions Validity**
/// **Validates: Requirements 5.2**
/// 
/// *For any* MatchedItem with positions array, each position SHALL be a valid index 
/// into the matched file name string (0 <= pos < name.len()).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn property_match_positions_validity(
        filename in "[a-zA-Z][a-zA-Z0-9]{3,15}",
    ) {
        let mut engine = SearchEngine::new();
        
        let path = PathBuf::from(format!("/test/{}", filename));
        engine.inject(path.clone());
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Search for a substring of the filename (first 3 chars)
        let search_pattern = &filename[0..3.min(filename.len())];
        engine.set_pattern(search_pattern);
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        let snapshot = engine.snapshot();
        
        // For each matched item, verify all positions are valid indices
        for matched_item in &snapshot.matches {
            let path_str = matched_item.path.to_string_lossy();
            let path_len = path_str.len();
            
            for &pos in &matched_item.positions {
                prop_assert!(
                    pos < path_len,
                    "Position {} is out of bounds for path '{}' (len={})",
                    pos, path_str, path_len
                );
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_search_engine_new() {
        let engine = SearchEngine::new();
        assert_eq!(engine.pattern(), "");
        assert!(!engine.is_active());
    }

    #[test]
    fn test_set_pattern_empty() {
        let mut engine = SearchEngine::new();
        engine.set_pattern("");
        assert_eq!(engine.pattern(), "");
        assert!(!engine.is_active());
    }

    #[test]
    fn test_set_pattern_non_empty() {
        let mut engine = SearchEngine::new();
        engine.set_pattern("test");
        assert_eq!(engine.pattern(), "test");
        assert!(engine.is_active());
    }

    #[test]
    fn test_clear_resets_state() {
        let mut engine = SearchEngine::new();
        engine.set_pattern("test");
        engine.inject(PathBuf::from("/test/file.txt"));
        
        engine.clear();
        
        assert_eq!(engine.pattern(), "");
        assert!(!engine.is_active());
    }

    #[test]
    fn test_snapshot_empty_engine() {
        let mut engine = SearchEngine::new();
        let snapshot = engine.snapshot();
        
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.len(), 0);
        assert_eq!(snapshot.pattern, "");
    }

    #[test]
    fn test_inject_and_search() {
        let mut engine = SearchEngine::new();
        
        engine.inject(PathBuf::from("/home/user/documents/report.txt"));
        engine.inject(PathBuf::from("/home/user/downloads/image.png"));
        engine.inject(PathBuf::from("/home/user/documents/notes.txt"));
        
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        engine.set_pattern("report");
        
        // Give nucleo time to match
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let snapshot = engine.snapshot();
        
        // Should find the report.txt file
        assert!(!snapshot.is_empty(), "Expected to find matches for 'report'");
    }

    #[test]
    fn test_matched_item_is_match_position() {
        let item = MatchedItem {
            path: PathBuf::from("/test/file.txt"),
            score: 100,
            positions: vec![0, 2, 5],
        };
        
        assert!(item.is_match_position(0));
        assert!(!item.is_match_position(1));
        assert!(item.is_match_position(2));
        assert!(!item.is_match_position(3));
        assert!(item.is_match_position(5));
    }
}
