/*
 * Tests for Encrypted Volume Support
 * 
 * Property-based tests for encrypted volume detection and unlock operations.
 * Requirements: 21.1-21.8
 */

use super::encrypted_volume::*;

#[test]
fn test_encryption_type_display_name() {
    assert_eq!(EncryptionType::BitLocker.display_name(), "BitLocker");
    assert_eq!(EncryptionType::Luks.display_name(), "LUKS");
    assert_eq!(EncryptionType::FileVault.display_name(), "FileVault");
    assert_eq!(EncryptionType::Unknown.display_name(), "Unknown");
}

#[test]
fn test_protection_status_methods() {
    assert!(ProtectionStatus::Locked.is_locked());
    assert!(!ProtectionStatus::Locked.is_unlocked());
    
    assert!(ProtectionStatus::Unlocked.is_unlocked());
    assert!(!ProtectionStatus::Unlocked.is_locked());
    
    assert!(!ProtectionStatus::Unknown.is_locked());
    assert!(!ProtectionStatus::Unknown.is_unlocked());
}

#[test]
fn test_encrypted_volume_info_is_encrypted() {
    let encrypted_info = EncryptedVolumeInfo {
        device_id: "C:".to_string(),
        mount_point: Some(std::path::PathBuf::from("C:\\")),
        encryption_type: EncryptionType::BitLocker,
        protection_status: ProtectionStatus::Unlocked,
        label: Some("System".to_string()),
        size: 500_000_000_000,
        encryption_percentage: Some(100),
    };
    assert!(encrypted_info.is_encrypted());

    let unencrypted_info = EncryptedVolumeInfo {
        device_id: "D:".to_string(),
        mount_point: Some(std::path::PathBuf::from("D:\\")),
        encryption_type: EncryptionType::Unknown,
        protection_status: ProtectionStatus::Unknown,
        label: None,
        size: 1_000_000_000_000,
        encryption_percentage: None,
    };
    assert!(!unencrypted_info.is_encrypted());
}

#[test]
fn test_encrypted_volume_info_lock_status() {
    let locked_info = EncryptedVolumeInfo {
        device_id: "/dev/sda1".to_string(),
        mount_point: None,
        encryption_type: EncryptionType::Luks,
        protection_status: ProtectionStatus::Locked,
        label: None,
        size: 256_000_000_000,
        encryption_percentage: Some(100),
    };
    assert!(locked_info.is_locked());
    assert!(!locked_info.is_unlocked());

    let unlocked_info = EncryptedVolumeInfo {
        device_id: "/dev/sda1".to_string(),
        mount_point: Some(std::path::PathBuf::from("/mnt/encrypted")),
        encryption_type: EncryptionType::Luks,
        protection_status: ProtectionStatus::Unlocked,
        label: Some("Data".to_string()),
        size: 256_000_000_000,
        encryption_percentage: Some(100),
    };
    assert!(unlocked_info.is_unlocked());
    assert!(!unlocked_info.is_locked());
}

#[test]
fn test_encrypted_volume_manager_creation() {
    let manager = EncryptedVolumeManager::new();
    let _default_manager = EncryptedVolumeManager::default();
    
    // Manager should be creatable without panicking
    assert!(!manager.is_encrypted("nonexistent_device"));
}

#[test]
fn test_unlock_credential_variants() {
    let password_cred = UnlockCredential::Password("test_password".to_string());
    let recovery_cred = UnlockCredential::RecoveryKey("123456-789012-345678-901234".to_string());
    
    match password_cred {
        UnlockCredential::Password(pwd) => assert_eq!(pwd, "test_password"),
        _ => panic!("Expected Password variant"),
    }
    
    match recovery_cred {
        UnlockCredential::RecoveryKey(key) => assert_eq!(key, "123456-789012-345678-901234"),
        _ => panic!("Expected RecoveryKey variant"),
    }
}

#[test]
fn test_encrypted_volume_error_display() {
    let err = EncryptedVolumeError::VolumeNotFound("C:".to_string());
    assert!(err.to_string().contains("C:"));
    
    let err = EncryptedVolumeError::NotEncrypted;
    assert!(err.to_string().contains("not encrypted"));
    
    let err = EncryptedVolumeError::InvalidCredentials;
    assert!(err.to_string().contains("Invalid"));
    
    let err = EncryptedVolumeError::UnlockFailed("Access denied".to_string());
    assert!(err.to_string().contains("Unlock failed"));
    
    let err = EncryptedVolumeError::LockFailed("Device busy".to_string());
    assert!(err.to_string().contains("Lock failed"));
}

#[cfg(target_os = "windows")]
mod windows_tests {
    use super::*;

    #[test]
    fn test_normalize_drive_letter() {
        assert_eq!(super::super::encrypted_volume::normalize_drive_letter("C:"), 'C');
        assert_eq!(super::super::encrypted_volume::normalize_drive_letter("c:"), 'C');
        assert_eq!(super::super::encrypted_volume::normalize_drive_letter("C:\\"), 'C');
        assert_eq!(super::super::encrypted_volume::normalize_drive_letter("d"), 'D');
    }
}

// **Feature: advanced-device-management, Property 21: Encrypted Volume Detection**
// **Validates: Requirements 21.1**
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_encrypted_volume_info_consistency(
            device_id in "[A-Z]:|/dev/sd[a-z][0-9]",
            is_bitlocker in any::<bool>(),
            is_locked in any::<bool>(),
            size in 1u64..10_000_000_000_000u64,
        ) {
            let encryption_type = if is_bitlocker {
                EncryptionType::BitLocker
            } else {
                EncryptionType::Luks
            };

            let protection_status = if is_locked {
                ProtectionStatus::Locked
            } else {
                ProtectionStatus::Unlocked
            };

            let info = EncryptedVolumeInfo {
                device_id: device_id.clone(),
                mount_point: if is_locked { None } else { Some(std::path::PathBuf::from("/mnt/test")) },
                encryption_type,
                protection_status,
                label: None,
                size,
                encryption_percentage: Some(100),
            };

            // Property: is_encrypted() returns true for BitLocker and LUKS
            prop_assert!(info.is_encrypted());

            // Property: is_locked() and is_unlocked() are mutually exclusive
            prop_assert!(info.is_locked() != info.is_unlocked() || 
                         matches!(info.protection_status, ProtectionStatus::Unknown));

            // Property: device_id is preserved
            prop_assert_eq!(info.device_id, device_id);
        }

        #[test]
        fn prop_unknown_encryption_not_encrypted(
            device_id in "[A-Z]:|/dev/sd[a-z][0-9]",
            size in 1u64..10_000_000_000_000u64,
        ) {
            let info = EncryptedVolumeInfo {
                device_id,
                mount_point: None,
                encryption_type: EncryptionType::Unknown,
                protection_status: ProtectionStatus::Unknown,
                label: None,
                size,
                encryption_percentage: None,
            };

            // Property: Unknown encryption type means not encrypted
            prop_assert!(!info.is_encrypted());
        }
    }
}
