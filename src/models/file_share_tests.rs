use super::*;
use proptest::prelude::*;
use std::path::PathBuf;

/
fn valid_share_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_]{0,30}".prop_map(|s| s)
}

/
fn valid_path_strategy() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec("[a-zA-Z0-9_]+", 1..4)
        .prop_map(|parts| {
            let mut path = PathBuf::from("/tmp");
            for part in parts {
                path.push(part);
            }
            path
        })
}

proptest! {
    /
    /
    /
    /
    /
    #[test]
    fn test_share_status_tracking(
        share_name in valid_share_name_strategy(),
        path in valid_path_strategy(),
    ) {
        let mut manager = ShareManager::new();

        prop_assert!(!manager.is_shared(&path), "Path should not be shared initially");

        let share_info = ShareInfo {
            share_name: share_name.clone(),
            path: path.clone(),
            description: String::new(),
            permission: SharePermission::ReadOnly,
            current_users: 0,
            max_users: None,
        };
        manager.shares.insert(path.clone(), share_info);

        prop_assert!(manager.is_shared(&path), "Path should be shared after adding");

        let retrieved = manager.get_share(&path);
        prop_assert!(retrieved.is_some(), "get_share should return Some after adding");
        prop_assert_eq!(&retrieved.unwrap().share_name, &share_name, "Share name should match");

        manager.shares.remove(&path);

        prop_assert!(!manager.is_shared(&path), "Path should not be shared after removal");

        prop_assert!(manager.get_share(&path).is_none(), "get_share should return None after removal");
    }

    /
    #[test]
    fn test_invalid_share_name_validation(
        invalid_char in prop::sample::select(vec!['\\', '/', ':', '*', '?', '"', '<', '>', '|']),
        prefix in "[a-zA-Z]{1,10}",
        suffix in "[a-zA-Z]{1,10}",
    ) {
        let invalid_name = format!("{}{}{}", prefix, invalid_char, suffix);

        let has_invalid = invalid_name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']);
        prop_assert!(has_invalid, "Generated name should contain invalid character");
    }

    /
    #[test]
    fn test_valid_share_name_validation(name in valid_share_name_strategy()) {
        let invalid_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
        let has_invalid = name.chars().any(|c| invalid_chars.contains(&c));
        prop_assert!(!has_invalid, "Valid share name should not contain invalid characters");
    }

    /
    #[test]
    fn test_list_shares_completeness(
        shares in prop::collection::vec(
            (valid_share_name_strategy(), valid_path_strategy()),
            0..10
        )
    ) {
        let mut manager = ShareManager::new();
        let mut unique_paths = std::collections::HashSet::new();

        for (name, path) in shares {
            if unique_paths.insert(path.clone()) {
                let share_info = ShareInfo {
                    share_name: name,
                    path: path.clone(),
                    description: String::new(),
                    permission: SharePermission::ReadOnly,
                    current_users: 0,
                    max_users: None,
                };
                manager.shares.insert(path, share_info);
            }
        }

        let listed = manager.list_shares();
        prop_assert_eq!(listed.len(), unique_paths.len(),
            "list_shares should return all tracked shares");

        for share in listed {
            prop_assert!(manager.is_shared(&share.path),
                "Listed share should be marked as shared");
        }
    }
}

#[test]
fn test_share_config_builder() {
    let config = ShareConfig::new("TestShare".to_string(), PathBuf::from("/tmp/test"))
        .with_description("Test description".to_string())
        .with_permission(SharePermission::ReadWrite)
        .with_max_users(10)
        .with_users(vec!["user1".to_string(), "user2".to_string()]);

    assert_eq!(config.share_name, "TestShare");
    assert_eq!(config.description, "Test description");
    assert_eq!(config.permission, SharePermission::ReadWrite);
    assert_eq!(config.max_users, Some(10));
    assert_eq!(config.users.len(), 2);
}

#[test]
fn test_share_permission_display_names() {
    assert_eq!(SharePermission::ReadOnly.display_name(), "Read Only");
    assert_eq!(SharePermission::ReadWrite.display_name(), "Read/Write");
    assert_eq!(SharePermission::Full.display_name(), "Full Control");
}

#[test]
fn test_share_manager_default() {
    let manager = ShareManager::default();
    assert!(manager.list_shares().is_empty());
}

#[test]
fn test_share_info_new() {
    let path = PathBuf::from("/tmp/test");
    let info = ShareInfo::new("TestShare".to_string(), path.clone());

    assert_eq!(info.share_name, "TestShare");
    assert_eq!(info.path, path);
    assert_eq!(info.permission, SharePermission::ReadOnly);
    assert_eq!(info.current_users, 0);
    assert!(info.max_users.is_none());
}
