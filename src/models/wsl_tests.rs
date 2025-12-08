use super::wsl::*;
use proptest::prelude::*;
use std::path::PathBuf;

/
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_windows_to_wsl_path_unc() {
        let windows_path = PathBuf::from("\\\\wsl$\\Ubuntu\\home\\user");
        let result = WslManager::windows_to_wsl_path(&windows_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/home/user");
    }

    #[test]
    fn test_windows_to_wsl_path_unc_localhost() {
        let windows_path = PathBuf::from("\\\\wsl.localhost\\Ubuntu\\home\\user");
        let result = WslManager::windows_to_wsl_path(&windows_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/home/user");
    }

    #[test]
    fn test_windows_to_wsl_path_drive() {
        let windows_path = PathBuf::from("C:\\Users\\test");
        let result = WslManager::windows_to_wsl_path(&windows_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/mnt/c/Users/test");
    }

    #[test]
    fn test_windows_to_wsl_path_drive_lowercase() {
        let windows_path = PathBuf::from("D:\\Projects");
        let result = WslManager::windows_to_wsl_path(&windows_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/mnt/d/Projects");
    }

    #[test]
    fn test_wsl_to_windows_path_mnt() {
        let result = WslManager::wsl_to_windows_path("Ubuntu", "/mnt/c/Users/test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("C:\\Users\\test"));
    }

    #[test]
    fn test_wsl_to_windows_path_linux() {
        let result = WslManager::wsl_to_windows_path("Ubuntu", "/home/user");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PathBuf::from("\\\\wsl$\\Ubuntu\\home\\user")
        );
    }

    #[test]
    fn test_is_wsl_path() {
        assert!(WslManager::is_wsl_path(&PathBuf::from("\\\\wsl$\\Ubuntu")));
        assert!(WslManager::is_wsl_path(&PathBuf::from(
            "\\\\wsl.localhost\\Debian"
        )));
        assert!(!WslManager::is_wsl_path(&PathBuf::from("C:\\Users")));
        assert!(!WslManager::is_wsl_path(&PathBuf::from("/home/user")));
    }

    #[test]
    fn test_extract_distro_name() {
        assert_eq!(
            WslManager::extract_distro_name(&PathBuf::from("\\\\wsl$\\Ubuntu\\home")),
            Some("Ubuntu".to_string())
        );
        assert_eq!(
            WslManager::extract_distro_name(&PathBuf::from("\\\\wsl.localhost\\Debian")),
            Some("Debian".to_string())
        );
        assert_eq!(
            WslManager::extract_distro_name(&PathBuf::from("C:\\Users")),
            None
        );
    }

    #[test]
    fn test_get_unc_path() {
        assert_eq!(
            WslManager::get_unc_path("Ubuntu"),
            PathBuf::from("\\\\wsl$\\Ubuntu")
        );
    }

    #[test]
    fn test_linux_permissions_format() {
        let perms = LinuxPermissions {
            mode: 0o755,
            owner: "root".to_string(),
            group: "root".to_string(),
        };
        assert_eq!(perms.format_mode(), "rwxr-xr-x");

        let perms2 = LinuxPermissions {
            mode: 0o644,
            owner: "user".to_string(),
            group: "users".to_string(),
        };
        assert_eq!(perms2.format_mode(), "rw-r--r--");

        let perms3 = LinuxPermissions {
            mode: 0o000,
            owner: "nobody".to_string(),
            group: "nogroup".to_string(),
        };
        assert_eq!(perms3.format_mode(), "---------");
    }

    #[test]
    fn test_wsl_state_from_str() {
        assert_eq!(WslState::from_str("Running"), WslState::Running);
        assert_eq!(WslState::from_str("running"), WslState::Running);
        assert_eq!(WslState::from_str("Stopped"), WslState::Stopped);
        assert_eq!(WslState::from_str("STOPPED"), WslState::Stopped);
        assert_eq!(WslState::from_str("Installing"), WslState::Installing);
        assert_eq!(WslState::from_str("Converting"), WslState::Converting);
        assert_eq!(WslState::from_str("Unknown"), WslState::Unknown);
        assert_eq!(WslState::from_str("garbage"), WslState::Unknown);
    }
}

/
/
/
#[cfg(test)]
mod property_tests {
    use super::*;

    /
    fn drive_letter_strategy() -> impl Strategy<Value = char> {
        prop_oneof![Just('C'), Just('D'), Just('E'), Just('F'),]
    }

    /
    fn path_component_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
    }

    /
    fn distro_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Ubuntu".to_string()),
            Just("Debian".to_string()),
            Just("kali-linux".to_string()),
            Just("openSUSE-Leap-15".to_string()),
        ]
    }

    proptest! {
        /
        /
        #[test]
        fn prop_windows_drive_path_roundtrip(
            drive in drive_letter_strategy(),
            components in prop::collection::vec(path_component_strategy(), 1..5)
        ) {
            let path_suffix = components.join("\\");
            let windows_path_str = format!("{}:\\{}", drive, path_suffix);
            let windows_path = PathBuf::from(&windows_path_str);

            let wsl_path = WslManager::windows_to_wsl_path(&windows_path);
            prop_assert!(wsl_path.is_ok(), "Failed to convert Windows path to WSL: {:?}", windows_path);
            let wsl_path = wsl_path.unwrap();

            let expected_prefix = format!("/mnt/{}", drive.to_ascii_lowercase());
            prop_assert!(
                wsl_path.starts_with(&expected_prefix),
                "WSL path should start with {}, got: {}",
                expected_prefix,
                wsl_path
            );

            let back_to_windows = WslManager::wsl_to_windows_path("Ubuntu", &wsl_path);
            prop_assert!(back_to_windows.is_ok(), "Failed to convert WSL path back to Windows: {}", wsl_path);
            let back_to_windows = back_to_windows.unwrap();

            prop_assert_eq!(
                windows_path.to_string_lossy().to_lowercase(),
                back_to_windows.to_string_lossy().to_lowercase(),
                "Round-trip failed: {} -> {} -> {}",
                windows_path.display(),
                wsl_path,
                back_to_windows.display()
            );
        }

        /
        #[test]
        fn prop_unc_path_distro_extraction(
            distro in distro_name_strategy(),
            components in prop::collection::vec(path_component_strategy(), 0..4)
        ) {
            let path_suffix = if components.is_empty() {
                String::new()
            } else {
                format!("\\{}", components.join("\\"))
            };

            let unc_path = PathBuf::from(format!("\\\\wsl$\\{}{}", distro, path_suffix));

            let extracted = WslManager::extract_distro_name(&unc_path);
            prop_assert!(extracted.is_some(), "Should extract distro name from: {:?}", unc_path);
            prop_assert_eq!(extracted.unwrap(), distro);
        }

        /
        #[test]
        fn prop_linux_path_to_unc(
            distro in distro_name_strategy(),
            components in prop::collection::vec(path_component_strategy(), 1..5)
        ) {
            let linux_path = format!("/{}", components.join("/"));

            let windows_path = WslManager::wsl_to_windows_path(&distro, &linux_path);
            prop_assert!(windows_path.is_ok(), "Failed to convert Linux path: {}", linux_path);
            let windows_path = windows_path.unwrap();

            prop_assert!(
                WslManager::is_wsl_path(&windows_path),
                "Result should be a WSL UNC path: {:?}",
                windows_path
            );

            let extracted = WslManager::extract_distro_name(&windows_path);
            prop_assert_eq!(extracted, Some(distro.clone()));
        }

        /
        #[test]
        fn prop_permission_format_length(mode in 0u32..0o777) {
            let perms = LinuxPermissions {
                mode,
                owner: "user".to_string(),
                group: "group".to_string(),
            };

            let formatted = perms.format_mode();

            prop_assert_eq!(formatted.len(), 9, "Permission string should be 9 chars: {}", formatted);

            prop_assert!(
                formatted.chars().all(|c| matches!(c, 'r' | 'w' | 'x' | '-')),
                "Invalid characters in permission string: {}",
                formatted
            );
        }
    }
}
