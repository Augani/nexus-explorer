use super::permissions::*;
use proptest::prelude::*;

/
/
/
/
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_unix_permission_round_trip(mode in 0u32..=0o7777u32) {
        let perms = UnixPermissions::from_mode(mode).expect("Valid mode should parse");
        
        let round_tripped_mode = perms.to_mode();
        
        prop_assert_eq!(mode, round_tripped_mode, 
            "Mode {:04o} round-tripped to {:04o}", mode, round_tripped_mode);
    }

    #[test]
    fn test_permission_bits_round_trip(mode in 0u8..=7u8) {
        let bits = PermissionBits::from_mode(mode);
        let round_tripped = bits.to_mode();
        prop_assert_eq!(mode, round_tripped);
    }

    #[test]
    fn test_special_bits_round_trip(
        setuid in proptest::bool::ANY,
        setgid in proptest::bool::ANY,
        sticky in proptest::bool::ANY
    ) {
        let special = SpecialBits::new(setuid, setgid, sticky);
        let mode = special.to_mode();
        let round_tripped = SpecialBits::from_mode(mode);
        
        prop_assert_eq!(special.setuid, round_tripped.setuid);
        prop_assert_eq!(special.setgid, round_tripped.setgid);
        prop_assert_eq!(special.sticky, round_tripped.sticky);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_common_permission_modes() {
        let test_cases = [
            (0o644, "rw-r--r--"),
            (0o755, "rwxr-xr-x"),
            (0o777, "rwxrwxrwx"),
            (0o600, "rw-------"),
            (0o700, "rwx------"),
            (0o000, "---------"),
        ];

        for (mode, expected_symbolic) in test_cases {
            let perms = UnixPermissions::from_mode(mode).unwrap();
            assert_eq!(perms.to_mode(), mode, "Mode {:04o} failed round-trip", mode);
            assert_eq!(perms.to_symbolic(), expected_symbolic, "Mode {:04o} symbolic mismatch", mode);
        }
    }

    #[test]
    fn test_special_bits_symbolic() {
        let perms = UnixPermissions::from_mode(0o4755).unwrap();
        assert!(perms.to_symbolic().starts_with("rwS") || perms.to_symbolic().starts_with("rws"));
        
        let perms = UnixPermissions::from_mode(0o2755).unwrap();
        let symbolic = perms.to_symbolic();
        assert!(symbolic.contains('S') || symbolic.contains('s'));
        
        let perms = UnixPermissions::from_mode(0o1755).unwrap();
        let symbolic = perms.to_symbolic();
        assert!(symbolic.ends_with('T') || symbolic.ends_with('t'));
    }

    #[test]
    fn test_invalid_mode_rejected() {
        assert!(UnixPermissions::from_mode(0o10000).is_err());
        assert!(UnixPermissions::from_mode(0o77777).is_err());
    }

    #[test]
    fn test_octal_string_format() {
        let perms = UnixPermissions::from_mode(0o755).unwrap();
        assert_eq!(perms.to_octal_string(), "0755");

        let perms = UnixPermissions::from_mode(0o4755).unwrap();
        assert_eq!(perms.to_octal_string(), "4755");
    }

    #[test]
    fn test_preset_permissions() {
        assert_eq!(UnixPermissions::preset_file_default().to_mode(), 0o644);
        assert_eq!(UnixPermissions::preset_file_executable().to_mode(), 0o755);
        assert_eq!(UnixPermissions::preset_directory_default().to_mode(), 0o755);
        assert_eq!(UnixPermissions::preset_private().to_mode(), 0o600);
        assert_eq!(UnixPermissions::preset_private_executable().to_mode(), 0o700);
    }

    #[test]
    fn test_has_special_bits() {
        let normal = UnixPermissions::from_mode(0o755).unwrap();
        assert!(!normal.has_special_bits());

        let setuid = UnixPermissions::from_mode(0o4755).unwrap();
        assert!(setuid.has_special_bits());

        let setgid = UnixPermissions::from_mode(0o2755).unwrap();
        assert!(setgid.has_special_bits());

        let sticky = UnixPermissions::from_mode(0o1755).unwrap();
        assert!(sticky.has_special_bits());
    }

    #[test]
    fn test_windows_acl_effective_permissions() {
        let mut acl = WindowsAcl::new();
        
        acl.add_entry(WindowsAclEntry::new(
            "User1".to_string(),
            AclEntryType::Allow,
            vec![WindowsPermissionType::Read, WindowsPermissionType::Write],
        ));
        
        acl.add_entry(WindowsAclEntry::new(
            "User1".to_string(),
            AclEntryType::Deny,
            vec![WindowsPermissionType::Write],
        ));

        let effective = acl.get_effective_permissions("User1");
        
        assert!(effective.contains(&WindowsPermissionType::Read));
        assert!(!effective.contains(&WindowsPermissionType::Write));
    }

    #[test]
    fn test_file_permissions_checks() {
        let unix_perms = UnixPermissions::from_mode(0o644).unwrap();
        let file_perms = FilePermissions::Unix(unix_perms);
        
        assert!(file_perms.is_readable());
        assert!(file_perms.is_writable());
        assert!(!file_perms.is_executable());

        let exec_perms = UnixPermissions::from_mode(0o755).unwrap();
        let file_perms = FilePermissions::Unix(exec_perms);
        
        assert!(file_perms.is_readable());
        assert!(file_perms.is_writable());
        assert!(file_perms.is_executable());
    }
}
