use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

#[test]
fn test_favorite_new() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let fav = Favorite::new(path.clone());

    assert_eq!(fav.path, path);
    assert!(fav.is_valid);
    assert!(!fav.name.is_empty());
}

#[test]
fn test_favorite_with_name() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let fav = Favorite::with_name(path.clone(), "Custom Name".to_string());

    assert_eq!(fav.name, "Custom Name");
    assert_eq!(fav.path, path);
    assert!(fav.is_valid);
}

#[test]
fn test_favorite_validate_existing() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut fav = Favorite::new(path);
    assert!(fav.validate());
    assert!(fav.is_valid);
}

#[test]
fn test_favorite_validate_nonexistent() {
    let mut fav = Favorite {
        name: "Test".to_string(),
        path: PathBuf::from("/nonexistent/path/that/does/not/exist"),
        is_valid: true,
    };

    assert!(!fav.validate());
    assert!(!fav.is_valid);
}

#[test]
fn test_favorites_new() {
    let favs = Favorites::new();

    assert!(favs.is_empty());
    assert_eq!(favs.len(), 0);
    assert!(!favs.is_full());
}

#[test]
fn test_favorites_add() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    let result = favs.add(path.clone());

    assert!(result.is_ok());
    assert_eq!(favs.len(), 1);
    assert!(favs.contains(&path));
}

#[test]
fn test_favorites_add_duplicate() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    let result = favs.add(path);
    assert!(matches!(result, Err(FavoritesError::AlreadyExists)));
}

#[test]
fn test_favorites_add_invalid_path() {
    let mut favs = Favorites::new();
    let result = favs.add(PathBuf::from("/nonexistent/path"));

    assert!(matches!(result, Err(FavoritesError::InvalidPath(_))));
}

#[test]
fn test_favorites_add_max_reached() {
    let temp = create_temp_dir();
    let mut favs = Favorites::new();

    // Add MAX_FAVORITES items
    for i in 0..MAX_FAVORITES {
        let subdir = temp.path().join(format!("dir{}", i));
        std::fs::create_dir(&subdir).unwrap();
        favs.add(subdir).unwrap();
    }

    // Try to add one more
    let extra = temp.path().join("extra");
    std::fs::create_dir(&extra).unwrap();
    let result = favs.add(extra);

    assert!(matches!(result, Err(FavoritesError::MaxReached(_))));
}

#[test]
fn test_favorites_remove() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    let removed = favs.remove(0);
    assert!(removed.is_ok());
    assert_eq!(removed.unwrap().path, path);
    assert!(favs.is_empty());
}

#[test]
fn test_favorites_remove_out_of_bounds() {
    let mut favs = Favorites::new();
    let result = favs.remove(0);

    assert!(matches!(result, Err(FavoritesError::IndexOutOfBounds(0))));
}

#[test]
fn test_favorites_remove_by_path() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    let removed = favs.remove_by_path(&path);
    assert!(removed.is_some());
    assert!(favs.is_empty());
}

#[test]
fn test_favorites_remove_by_path_not_found() {
    let mut favs = Favorites::new();
    let result = favs.remove_by_path(&PathBuf::from("/not/found"));

    assert!(result.is_none());
}

#[test]
fn test_favorites_reorder() {
    let temp = create_temp_dir();
    let mut favs = Favorites::new();

    let paths: Vec<PathBuf> = (0..3)
        .map(|i| {
            let p = temp.path().join(format!("dir{}", i));
            std::fs::create_dir(&p).unwrap();
            p
        })
        .collect();

    for p in &paths {
        favs.add(p.clone()).unwrap();
    }

    // Reorder: move first to last
    favs.reorder(0, 2).unwrap();

    assert_eq!(favs.items()[0].path, paths[1]);
    assert_eq!(favs.items()[1].path, paths[2]);
    assert_eq!(favs.items()[2].path, paths[0]);
}

#[test]
fn test_favorites_reorder_same_index() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    let result = favs.reorder(0, 0);
    assert!(result.is_ok());
    assert_eq!(favs.items()[0].path, path);
}

#[test]
fn test_favorites_reorder_out_of_bounds() {
    let mut favs = Favorites::new();

    let result = favs.reorder(0, 1);
    assert!(matches!(result, Err(FavoritesError::IndexOutOfBounds(_))));
}

#[test]
fn test_favorites_validate_all() {
    let temp = create_temp_dir();
    let valid_path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(valid_path).unwrap();

    // Manually add an invalid favorite
    favs.items_mut().push(Favorite {
        name: "Invalid".to_string(),
        path: PathBuf::from("/nonexistent/path"),
        is_valid: true,
    });

    let invalid = favs.validate_all();

    assert_eq!(invalid.len(), 1);
    assert_eq!(invalid[0], 1);
    assert!(favs.items()[0].is_valid);
    assert!(!favs.items()[1].is_valid);
}

#[test]
fn test_favorites_contains() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    assert!(favs.contains(&path));
    assert!(!favs.contains(&PathBuf::from("/other/path")));
}

#[test]
fn test_favorites_get() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    assert!(favs.get(0).is_some());
    assert_eq!(favs.get(0).unwrap().path, path);
    assert!(favs.get(1).is_none());
}

#[test]
fn test_favorites_find_index() {
    let temp = create_temp_dir();
    let path = temp.path().to_path_buf();

    let mut favs = Favorites::new();
    favs.add(path.clone()).unwrap();

    assert_eq!(favs.find_index(&path), Some(0));
    assert_eq!(favs.find_index(&PathBuf::from("/other")), None);
}

#[test]
fn test_favorites_is_full() {
    let temp = create_temp_dir();
    let mut favs = Favorites::new();

    for i in 0..MAX_FAVORITES {
        let subdir = temp.path().join(format!("dir{}", i));
        std::fs::create_dir(&subdir).unwrap();
        favs.add(subdir).unwrap();
    }

    assert!(favs.is_full());
}

// Property-based tests using proptest
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    /// Property 14: Favorites Add Persistence
    /// For any valid path added to favorites, saving and loading should preserve the favorite
    #[test]
    fn property_favorites_add_persistence() {
        let temp = TempDir::new().unwrap();
        let test_dir = temp.path().join("test_persistence");
        std::fs::create_dir(&test_dir).unwrap();

        let mut favs = Favorites::new();
        favs.add(test_dir.clone()).unwrap();

        // Save to a custom location for testing
        let config_path = temp.path().join("favorites_test.json");
        let json = serde_json::to_string_pretty(&favs).unwrap();
        std::fs::write(&config_path, &json).unwrap();

        let loaded_json = std::fs::read_to_string(&config_path).unwrap();
        let mut loaded: Favorites = serde_json::from_str(&loaded_json).unwrap();
        loaded.validate_all();

        // Verify the favorite was persisted
        assert_eq!(loaded.len(), favs.len());
        assert!(loaded.contains(&test_dir));
        assert_eq!(loaded.items()[0].name, favs.items()[0].name);
    }

    proptest! {
        /// Property 15: Favorites Reorder Preservation
        /// For any valid reorder operation, all original items should still be present
        #[test]
        fn property_favorites_reorder_preservation(
            num_items in 2usize..=5,
            from in 0usize..5,
            to in 0usize..5
        ) {
            let temp = TempDir::new().unwrap();
            let mut favs = Favorites::new();

            let mut paths = Vec::new();
            for i in 0..num_items {
                let p = temp.path().join(format!("dir{}", i));
                std::fs::create_dir(&p).unwrap();
                paths.push(p.clone());
                favs.add(p).unwrap();
            }

            // Only reorder if indices are valid
            if from < num_items && to < num_items {
                favs.reorder(from, to).unwrap();

                // All original paths should still be present
                for path in &paths {
                    prop_assert!(favs.contains(path));
                }

                // Length should be unchanged
                prop_assert_eq!(favs.len(), num_items);
            }
        }

        /// Property 16: Favorites Invalid Path Detection
        /// For any path that doesn't exist, validate should mark it as invalid
        #[test]
        fn property_favorites_invalid_path_detection(
            path_suffix in "[a-z]{5,10}"
        ) {
            let invalid_path = PathBuf::from(format!("/nonexistent/path/{}", path_suffix));

            let mut fav = Favorite {
                name: "Test".to_string(),
                path: invalid_path,
                is_valid: true,
            };

            let result = fav.validate();

            prop_assert!(!result);
            prop_assert!(!fav.is_valid);
        }
    }
}
