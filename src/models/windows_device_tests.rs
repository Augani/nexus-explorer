//! Unit tests for Windows device detection
//! 
//! These tests verify drive enumeration and device type detection on Windows.

#![cfg(target_os = "windows")]

use super::device_monitor::{Device, DeviceId, DeviceMonitor, DeviceType};
use super::device_monitor_windows::*;
use std::path::PathBuf;

#[test]
fn test_get_available_drive_letters() {
    let drives = get_available_drive_letters();
    
    // Should have at least one drive (typically C:)
    assert!(!drives.is_empty(), "Should detect at least one drive letter");
    
    // All drive letters should be uppercase A-Z
    for letter in &drives {
        assert!(letter.is_ascii_uppercase(), "Drive letter should be uppercase");
        assert!((*letter as u8) >= b'A' && (*letter as u8) <= b'Z', 
            "Drive letter should be A-Z");
    }
    
    // C: drive should typically exist on Windows
    assert!(drives.contains(&'C'), "C: drive should typically exist");
}

#[test]
fn test_detect_windows_drive_type_c_drive() {
    let path = PathBuf::from("C:\\");
    if path.exists() {
        let drive_type = detect_windows_drive_type(&path);
        
        // C: is typically an internal/fixed drive
        assert_eq!(drive_type, DeviceType::InternalDrive, 
            "C: drive should be detected as InternalDrive");
    }
}

#[test]
fn test_get_windows_volume_name() {
    let path = PathBuf::from("C:\\");
    if path.exists() {
        // Volume name may or may not be set, but function should not panic
        let _name = get_windows_volume_name(&path);
    }
}

#[test]
fn test_get_windows_filesystem_type() {
    let path = PathBuf::from("C:\\");
    if path.exists() {
        let fs_type = get_windows_filesystem_type(&path);
        
        // C: drive typically has NTFS filesystem
        if let Some(fs) = fs_type {
            assert!(!fs.is_empty(), "Filesystem type should not be empty");
            // Common Windows filesystems
            let valid_fs = ["NTFS", "FAT32", "exFAT", "FAT", "ReFS"];
            assert!(valid_fs.iter().any(|&v| fs.contains(v)), 
                "Filesystem should be a known Windows type, got: {}", fs);
        }
    }
}

#[test]
fn test_is_drive_read_only() {
    let path = PathBuf::from("C:\\");
    if path.exists() {
        let read_only = is_drive_read_only(&path);
        
        // C: drive is typically not read-only
        assert!(!read_only, "C: drive should not be read-only");
    }
}

#[test]
fn test_get_drive_info() {
    let info = get_drive_info('C');
    
    assert!(info.is_some(), "Should get info for C: drive");
    
    if let Some(drive_info) = info {
        assert_eq!(drive_info.drive_letter, 'C');
        assert_eq!(drive_info.path, PathBuf::from("C:\\"));
        assert_eq!(drive_info.drive_type, DeviceType::InternalDrive);
        assert!(drive_info.total_space > 0, "Total space should be > 0");
        assert!(drive_info.free_space <= drive_info.total_space, 
            "Free space should not exceed total space");
    }
}

#[test]
fn test_get_drive_info_nonexistent() {
    // Try to get info for a drive letter that likely doesn't exist
    // Z: is often unused
    let info = get_drive_info('Z');
    
    // This may or may not exist depending on the system
    // Just verify it doesn't panic
    if info.is_none() {
        // Expected for non-existent drive
    }
}

#[test]
fn test_drive_info_is_removable() {
    if let Some(info) = get_drive_info('C') {
        // C: drive should not be removable
        assert!(!info.is_removable(), "C: drive should not be removable");
    }
}

#[test]
fn test_drive_info_display_name() {
    if let Some(info) = get_drive_info('C') {
        let name = info.display_name();
        assert!(!name.is_empty(), "Display name should not be empty");
    }
}

#[test]
fn test_device_monitor_enumerate_windows() {
    let mut monitor = DeviceMonitor::new();
    monitor.enumerate_devices();
    
    let devices = monitor.devices();
    
    // Should have at least one device
    assert!(!devices.is_empty(), "Should detect at least one device");
    
    // Verify device properties
    for device in devices {
        assert!(!device.name.is_empty(), "Device name should not be empty");
        assert!(device.path.exists() || device.path.to_string_lossy().starts_with("\\\\"),
            "Device path should exist or be a UNC path");
    }
}

#[test]
fn test_device_type_detection_consistency() {
    // Get drives via both methods and verify consistency
    let drive_letters = get_available_drive_letters();
    
    for letter in drive_letters {
        let path = PathBuf::from(format!("{}:\\", letter));
        if !path.exists() {
            continue;
        }
        
        let drive_type = detect_windows_drive_type(&path);
        
        // Verify drive type is valid
        match drive_type {
            DeviceType::InternalDrive |
            DeviceType::ExternalDrive |
            DeviceType::UsbDrive |
            DeviceType::NetworkDrive |
            DeviceType::OpticalDrive |
            DeviceType::DiskImage => {
                // Valid types
            }
            _ => {
                panic!("Unexpected drive type for {}: {:?}", letter, drive_type);
            }
        }
    }
}

#[test]
fn test_wmi_logical_disk_struct() {
    // Test the WmiLogicalDisk struct creation
    let disk = WmiLogicalDisk {
        device_id: "C:".to_string(),
        volume_name: Some("Windows".to_string()),
        file_system: Some("NTFS".to_string()),
        size: Some(500_000_000_000),
        free_space: Some(100_000_000_000),
        drive_type: 3, // DRIVE_FIXED
        volume_serial_number: Some("1234ABCD".to_string()),
    };
    
    assert_eq!(disk.device_id, "C:");
    assert_eq!(disk.volume_name, Some("Windows".to_string()));
    assert_eq!(disk.drive_type, 3);
}

#[test]
fn test_wmi_disk_drive_struct() {
    // Test the WmiDiskDrive struct creation
    let disk = WmiDiskDrive {
        device_id: "\\\\.\\PHYSICALDRIVE0".to_string(),
        model: Some("Samsung SSD 970 EVO".to_string()),
        serial_number: Some("S123456789".to_string()),
        size: Some(1_000_000_000_000),
        media_type: Some("Fixed hard disk media".to_string()),
        interface_type: Some("NVMe".to_string()),
    };
    
    assert_eq!(disk.device_id, "\\\\.\\PHYSICALDRIVE0");
    assert!(disk.model.is_some());
    assert!(disk.size.is_some());
}

// Property-based tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: advanced-device-management, Property: Drive Letter Enumeration Completeness**
    // **Validates: Requirements 1.1, 1.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        
        #[test]
        fn prop_drive_enumeration_returns_valid_letters(_seed in 0u32..100) {
            let drives = get_available_drive_letters();
            
            // All returned letters should be valid
            for letter in &drives {
                prop_assert!(letter.is_ascii_uppercase());
                prop_assert!((*letter as u8) >= b'A');
                prop_assert!((*letter as u8) <= b'Z');
            }
            
            // No duplicates
            let mut sorted = drives.clone();
            sorted.sort();
            sorted.dedup();
            prop_assert_eq!(sorted.len(), drives.len(), "Should have no duplicate drive letters");
        }
    }

    // **Feature: advanced-device-management, Property: Drive Type Detection Consistency**
    // **Validates: Requirements 1.1, 1.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        
        #[test]
        fn prop_drive_type_detection_is_deterministic(_seed in 0u32..100) {
            let drives = get_available_drive_letters();
            
            for letter in drives {
                let path = PathBuf::from(format!("{}:\\", letter));
                if !path.exists() {
                    continue;
                }
                
                // Call twice and verify same result
                let type1 = detect_windows_drive_type(&path);
                let type2 = detect_windows_drive_type(&path);
                
                prop_assert_eq!(type1, type2, 
                    "Drive type detection should be deterministic for {}", letter);
            }
        }
    }
}
