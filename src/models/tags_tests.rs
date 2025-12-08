use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

#[test]
fn test_create_tag() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();

    assert!(manager.get_tag(id).is_some());
    assert_eq!(manager.get_tag(id).unwrap().name, "Work");
    assert_eq!(manager.get_tag(id).unwrap().color, TagColor::Blue);
}

#[test]
fn test_create_duplicate_tag_fails() {
    let mut manager = TagManager::empty();

    manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let result = manager.create_tag("work".to_string(), TagColor::Red);

    assert!(matches!(result, Err(TagError::DuplicateName(_))));
}

#[test]
fn test_delete_tag() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let path = PathBuf::from("/test/file.txt");
    manager.apply_tag(&path, id).unwrap();

    manager.delete_tag(id).unwrap();

    assert!(manager.get_tag(id).is_none());
    assert!(!manager.has_tag(&path, id));
}

#[test]
fn test_apply_tag() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Important".to_string(), TagColor::Red)
        .unwrap();
    let path = PathBuf::from("/test/file.txt");

    manager.apply_tag(&path, id).unwrap();

    assert!(manager.has_tag(&path, id));
    assert_eq!(manager.tags_for_file(&path).len(), 1);
}

#[test]
fn test_apply_nonexistent_tag_fails() {
    let mut manager = TagManager::empty();
    let path = PathBuf::from("/test/file.txt");

    let result = manager.apply_tag(&path, TagId::new(999));

    assert!(matches!(result, Err(TagError::TagNotFound(_))));
}

#[test]
fn test_remove_tag() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let path = PathBuf::from("/test/file.txt");

    manager.apply_tag(&path, id).unwrap();
    manager.remove_tag(&path, id).unwrap();

    assert!(!manager.has_tag(&path, id));
}

#[test]
fn test_multiple_tags_on_file() {
    let mut manager = TagManager::empty();

    let id1 = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let id2 = manager
        .create_tag("Important".to_string(), TagColor::Red)
        .unwrap();
    let path = PathBuf::from("/test/file.txt");

    manager.apply_tag(&path, id1).unwrap();
    manager.apply_tag(&path, id2).unwrap();

    assert!(manager.has_tag(&path, id1));
    assert!(manager.has_tag(&path, id2));
    assert_eq!(manager.tags_for_file(&path).len(), 2);
}

#[test]
fn test_files_with_tag() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let path1 = PathBuf::from("/test/file1.txt");
    let path2 = PathBuf::from("/test/file2.txt");
    let path3 = PathBuf::from("/test/file3.txt");

    manager.apply_tag(&path1, id).unwrap();
    manager.apply_tag(&path2, id).unwrap();

    let files = manager.files_with_tag(id);
    assert_eq!(files.len(), 2);
    assert!(files.contains(&&path1));
    assert!(files.contains(&&path2));
}

#[test]
fn test_get_tag_by_name() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();

    let tag = manager.get_tag_by_name("work").unwrap();
    assert_eq!(tag.id, id);

    assert!(manager.get_tag_by_name("nonexistent").is_none());
}

#[test]
fn test_rename_tag() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    manager.rename_tag(id, "Projects".to_string()).unwrap();

    assert_eq!(manager.get_tag(id).unwrap().name, "Projects");
}

#[test]
fn test_set_tag_color() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    manager.set_tag_color(id, TagColor::Green).unwrap();

    assert_eq!(manager.get_tag(id).unwrap().color, TagColor::Green);
}

#[test]
fn test_clear_file_tags() {
    let mut manager = TagManager::empty();

    let id1 = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let id2 = manager
        .create_tag("Important".to_string(), TagColor::Red)
        .unwrap();
    let path = PathBuf::from("/test/file.txt");

    manager.apply_tag(&path, id1).unwrap();
    manager.apply_tag(&path, id2).unwrap();
    manager.clear_file_tags(&path);

    assert!(!manager.has_tags(&path));
}

#[test]
fn test_update_file_path() {
    let mut manager = TagManager::empty();

    let id = manager
        .create_tag("Work".to_string(), TagColor::Blue)
        .unwrap();
    let old_path = PathBuf::from("/test/old.txt");
    let new_path = PathBuf::from("/test/new.txt");

    manager.apply_tag(&old_path, id).unwrap();
    manager.update_file_path(&old_path, &new_path);

    assert!(!manager.has_tag(&old_path, id));
    assert!(manager.has_tag(&new_path, id));
}

#[test]
fn test_default_manager_has_color_tags() {
    let manager = TagManager::new();

    assert_eq!(manager.tag_count(), 7);

    for color in TagColor::all() {
        assert!(manager.get_tag_by_name(color.display_name()).is_some());
    }
}

#[test]
fn test_tag_color_rgba() {
    for color in TagColor::all() {
        let (r, g, b, a) = color.to_rgba();
        assert!(r > 0 || g > 0 || b > 0, "Color should not be black");
        assert_eq!(a, 0xFF, "Alpha should be fully opaque");
    }
}

fn arb_tag_color() -> impl Strategy<Value = TagColor> {
    prop_oneof![
        Just(TagColor::Red),
        Just(TagColor::Orange),
        Just(TagColor::Yellow),
        Just(TagColor::Green),
        Just(TagColor::Blue),
        Just(TagColor::Purple),
        Just(TagColor::Gray),
    ]
}

fn arb_tag_name() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9 _-]{0,20}".prop_filter("non-empty", |s| !s.is_empty())
}

fn arb_file_path() -> impl Strategy<Value = PathBuf> {
    "[a-zA-Z0-9_-]{1,10}(\\.[a-z]{1,4})?".prop_map(|name| PathBuf::from(format!("/test/{}", name)))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /
    /
    /
    /
    /
    #[test]
    fn prop_tag_persistence(
        tag_name in arb_tag_name(),
        tag_color in arb_tag_color(),
        file_path in arb_file_path(),
    ) {
        let mut manager = TagManager::empty();
        let tag_id = manager.create_tag(tag_name.clone(), tag_color).unwrap();

        manager.apply_tag(&file_path, tag_id).unwrap();

        let json = serde_json::to_string(&manager).expect("Failed to serialize TagManager");

        let loaded: TagManager = serde_json::from_str(&json).expect("Failed to deserialize TagManager");

        let loaded_tag = loaded.get_tag(tag_id);
        prop_assert!(
            loaded_tag.is_some(),
            "Tag {:?} should exist after load",
            tag_id
        );

        let loaded_tag = loaded_tag.unwrap();
        prop_assert_eq!(
            &loaded_tag.name, &tag_name,
            "Tag name should be preserved"
        );
        prop_assert_eq!(
            loaded_tag.color, tag_color,
            "Tag color should be preserved"
        );

        prop_assert!(
            loaded.has_tag(&file_path, tag_id),
            "File {:?} should still have tag {:?} after load",
            file_path, tag_id
        );

        let file_tags = loaded.tags_for_file(&file_path);
        prop_assert!(
            file_tags.iter().any(|t| t.id == tag_id),
            "tags_for_file should include the applied tag"
        );
    }

    /
    /
    /
    /
    /
    #[test]
    fn prop_multiple_tags_persistence(
        tag_names in prop::collection::vec(arb_tag_name(), 1..5),
        file_paths in prop::collection::vec(arb_file_path(), 1..5),
    ) {
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        let unique_tag_names: Vec<String> = tag_names
            .into_iter()
            .filter(|name| seen_names.insert(name.to_lowercase()))
            .collect();

        prop_assume!(!unique_tag_names.is_empty());

        let mut manager = TagManager::empty();
        let mut tag_ids = Vec::new();

        for (i, name) in unique_tag_names.iter().enumerate() {
            let color = TagColor::all()[i % TagColor::all().len()];
            let id = manager.create_tag(name.clone(), color).unwrap();
            tag_ids.push(id);
        }

        for path in &file_paths {
            for &tag_id in &tag_ids {
                manager.apply_tag(path, tag_id).unwrap();
            }
        }

        let json = serde_json::to_string(&manager).expect("Failed to serialize");
        let loaded: TagManager = serde_json::from_str(&json).expect("Failed to deserialize");

        for &tag_id in &tag_ids {
            prop_assert!(
                loaded.get_tag(tag_id).is_some(),
                "Tag {:?} should exist after load",
                tag_id
            );
        }

        for path in &file_paths {
            for &tag_id in &tag_ids {
                prop_assert!(
                    loaded.has_tag(path, tag_id),
                    "File {:?} should have tag {:?} after load",
                    path, tag_id
                );
            }
        }
    }

    /
    /
    /
    /
    /
    #[test]
    fn prop_tag_operations_consistency(
        tag_name in arb_tag_name(),
        tag_color in arb_tag_color(),
        file_path in arb_file_path(),
    ) {
        let mut manager = TagManager::empty();

        let tag_id = manager.create_tag(tag_name.clone(), tag_color).unwrap();
        prop_assert_eq!(manager.tag_count(), 1);

        manager.apply_tag(&file_path, tag_id).unwrap();
        prop_assert!(manager.has_tag(&file_path, tag_id));
        prop_assert_eq!(manager.tagged_file_count(), 1);

        manager.remove_tag(&file_path, tag_id).unwrap();
        prop_assert!(!manager.has_tag(&file_path, tag_id));
        prop_assert_eq!(manager.tagged_file_count(), 0);

        manager.apply_tag(&file_path, tag_id).unwrap();
        prop_assert!(manager.has_tag(&file_path, tag_id));

        manager.delete_tag(tag_id).unwrap();
        prop_assert!(manager.get_tag(tag_id).is_none());
        prop_assert!(!manager.has_tag(&file_path, tag_id));
        prop_assert_eq!(manager.tag_count(), 0);
        prop_assert_eq!(manager.tagged_file_count(), 0);
    }
}
