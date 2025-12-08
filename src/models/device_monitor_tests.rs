use super::device_monitor::*;
use std::path::PathBuf;

#[test]
fn test_device_creation() {
    let device = Device::new(
        DeviceId::new(1),
        "Test Drive".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    );

    assert_eq!(device.id, DeviceId::new(1));
    assert_eq!(device.name, "Test Drive");
    assert_eq!(device.path, PathBuf::from("/test"));
    assert_eq!(device.device_type, DeviceType::InternalDrive);
    assert!(device.is_mounted);
}

#[test]
fn test_device_with_space() {
    let device = Device::new(
        DeviceId::new(1),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::UsbDrive,
    )
    .with_space(1000, 400);

    assert_eq!(device.total_space, 1000);
    assert_eq!(device.free_space, 400);
    assert_eq!(device.used_space(), 600);
}

#[test]
fn test_device_usage_percentage() {
    let device = Device::new(
        DeviceId::new(1),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::UsbDrive,
    )
    .with_space(1000, 250);

    let usage = device.usage_percentage();
    assert!((usage - 0.75).abs() < 0.001);
}

#[test]
fn test_device_usage_percentage_zero_total() {
    let device = Device::new(
        DeviceId::new(1),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::UsbDrive,
    )
    .with_space(0, 0);

    assert_eq!(device.usage_percentage(), 0.0);
}

#[test]
fn test_device_monitor_creation() {
    let monitor = DeviceMonitor::new();
    assert!(monitor.devices().is_empty());
    assert!(monitor.wsl_distributions().is_empty());
    assert!(!monitor.is_monitoring());
}

#[test]
fn test_device_monitor_add_device() {
    let mut monitor = DeviceMonitor::new();

    let device = Device::new(
        DeviceId::new(0),
        "Test Drive".to_string(),
        PathBuf::from("/test"),
        DeviceType::ExternalDrive,
    );

    let id = monitor.add_device(device);

    assert_eq!(monitor.devices().len(), 1);
    assert!(monitor.get_device(id).is_some());
}

#[test]
fn test_device_monitor_remove_device() {
    let mut monitor = DeviceMonitor::new();

    let device = Device::new(
        DeviceId::new(0),
        "Test Drive".to_string(),
        PathBuf::from("/test"),
        DeviceType::ExternalDrive,
    );

    let id = monitor.add_device(device);
    assert_eq!(monitor.devices().len(), 1);

    let removed = monitor.remove_device(id);
    assert!(removed.is_some());
    assert!(monitor.devices().is_empty());
}

#[test]
fn test_device_monitor_get_by_path() {
    let mut monitor = DeviceMonitor::new();

    let path = PathBuf::from("/test/path");
    let device = Device::new(
        DeviceId::new(0),
        "Test Drive".to_string(),
        path.clone(),
        DeviceType::ExternalDrive,
    );

    monitor.add_device(device);

    let found = monitor.get_device_by_path(&path);
    assert!(found.is_some());
    assert_eq!(found.unwrap().path, path);
}

#[test]
fn test_device_monitor_subscribe() {
    let monitor = DeviceMonitor::new();
    let receiver = monitor.subscribe();
    assert!(receiver.is_some());
}

#[test]
fn test_device_type_icon_names() {
    assert_eq!(DeviceType::InternalDrive.icon_name(), "hard-drive");
    assert_eq!(DeviceType::UsbDrive.icon_name(), "usb");
    assert_eq!(DeviceType::NetworkDrive.icon_name(), "cloud");
    assert_eq!(DeviceType::OpticalDrive.icon_name(), "disc");
    assert_eq!(DeviceType::WslDistribution.icon_name(), "terminal");
}

#[test]
fn test_wsl_distribution_creation() {
    let distro = WslDistribution::new("Ubuntu".to_string(), PathBuf::from("\\\\wsl$\\Ubuntu"), 2);

    assert_eq!(distro.name, "Ubuntu");
    assert_eq!(distro.version, 2);
    assert!(!distro.is_running);
}

#[test]
fn test_device_monitor_multiple_devices() {
    let mut monitor = DeviceMonitor::new();

    let device1 = Device::new(
        DeviceId::new(0),
        "Drive 1".to_string(),
        PathBuf::from("/drive1"),
        DeviceType::InternalDrive,
    );

    let device2 = Device::new(
        DeviceId::new(0),
        "Drive 2".to_string(),
        PathBuf::from("/drive2"),
        DeviceType::ExternalDrive,
    );

    let id1 = monitor.add_device(device1);
    let id2 = monitor.add_device(device2);

    assert_eq!(monitor.devices().len(), 2);
    assert_ne!(id1, id2);
}

#[test]
fn test_device_monitor_update_device() {
    let mut monitor = DeviceMonitor::new();

    let device = Device::new(
        DeviceId::new(0),
        "Test Drive".to_string(),
        PathBuf::from("/test"),
        DeviceType::ExternalDrive,
    )
    .with_space(1000, 500);

    let id = monitor.add_device(device);

    let mut updated = monitor.get_device(id).unwrap().clone();
    updated.free_space = 300;
    monitor.update_device(updated);

    let device = monitor.get_device(id).unwrap();
    assert_eq!(device.free_space, 300);
}

#[test]
fn test_device_removable_flag() {
    let device = Device::new(
        DeviceId::new(1),
        "USB".to_string(),
        PathBuf::from("/usb"),
        DeviceType::UsbDrive,
    )
    .with_removable(true);

    assert!(device.is_removable);
}

#[test]
fn test_device_read_only_flag() {
    let device = Device::new(
        DeviceId::new(1),
        "CD".to_string(),
        PathBuf::from("/cdrom"),
        DeviceType::OpticalDrive,
    )
    .with_read_only(true);

    assert!(device.is_read_only);
}

#[cfg(target_os = "macos")]
mod macos_tests {
    use super::*;
    use crate::models::device_monitor_macos::*;

    #[test]
    fn test_macos_disk_monitor_creation() {
        let monitor = MacOSDiskMonitor::new();
        assert!(!monitor.is_monitoring());
    }

    #[test]
    fn test_macos_enumerate_volumes() {
        let monitor = MacOSDiskMonitor::new();
        let volumes = monitor.enumerate_volumes();
        
        assert!(!volumes.is_empty(), "Should detect at least one volume");
        
        let has_root = volumes.iter().any(|v| {
            v.volume_path.as_ref().map(|p| p.as_path() == std::path::Path::new("/")).unwrap_or(false)
        });
        assert!(has_root, "Root volume should be detected");
    }

    #[test]
    fn test_macos_volume_has_valid_metadata() {
        let monitor = MacOSDiskMonitor::new();
        let volumes = monitor.enumerate_volumes();
        
        for volume in &volumes {
            assert!(volume.volume_name.is_some() || volume.volume_path.is_some(),
                "Volume should have either a name or path");
            
            if let Some(ref path) = volume.volume_path {
                assert!(path.exists() || path.to_string_lossy().starts_with("/"),
                    "Volume path should exist: {:?}", path);
            }
        }
    }

    #[test]
    fn test_macos_root_volume_properties() {
        let monitor = MacOSDiskMonitor::new();
        let volumes = monitor.enumerate_volumes();
        
        let root = volumes.iter().find(|v| {
            v.volume_path.as_ref().map(|p| p.as_path() == std::path::Path::new("/")).unwrap_or(false)
        });
        
        assert!(root.is_some(), "Root volume should be found");
        let root = root.unwrap();
        
        assert!(root.is_internal, "Root volume should be internal");
        assert!(!root.is_removable, "Root volume should not be removable");
        assert!(!root.is_ejectable, "Root volume should not be ejectable");
    }

    #[test]
    fn test_macos_disk_info_device_type_internal() {
        let info = DiskInfo {
            is_internal: true,
            is_removable: false,
            is_ejectable: false,
            is_network: false,
            ..Default::default()
        };
        
        assert_eq!(info.device_type(), DeviceType::InternalDrive);
    }

    #[test]
    fn test_macos_disk_info_device_type_network() {
        let info = DiskInfo {
            is_network: true,
            ..Default::default()
        };
        
        assert_eq!(info.device_type(), DeviceType::NetworkDrive);
    }

    #[test]
    fn test_macos_disk_info_device_type_usb() {
        let info = DiskInfo {
            bus_name: Some("USB".to_string()),
            is_removable: true,
            ..Default::default()
        };
        
        assert_eq!(info.device_type(), DeviceType::UsbDrive);
    }

    #[test]
    fn test_macos_disk_info_device_type_external() {
        let info = DiskInfo {
            is_removable: true,
            is_ejectable: true,
            is_internal: false,
            ..Default::default()
        };
        
        assert_eq!(info.device_type(), DeviceType::ExternalDrive);
    }

    #[test]
    fn test_macos_disk_info_device_type_optical() {
        let info = DiskInfo {
            media_name: Some("DVD Drive".to_string()),
            ..Default::default()
        };
        
        assert_eq!(info.device_type(), DeviceType::OpticalDrive);
    }

    #[test]
    fn test_macos_disk_info_default() {
        let info = DiskInfo::default();
        
        assert!(info.bsd_name.is_empty());
        assert!(info.volume_name.is_none());
        assert!(info.volume_path.is_none());
        assert_eq!(info.media_size, 0);
        assert!(!info.is_removable);
        assert!(!info.is_ejectable);
        assert!(info.is_internal);
        assert!(!info.is_network);
    }

    #[test]
    fn test_macos_is_disk_image_false_for_regular_path() {
        let path = PathBuf::from("/Volumes/TestDrive");
        assert!(!is_disk_image(&path));
    }

    #[test]
    fn test_macos_device_monitor_enumerate() {
        let mut monitor = DeviceMonitor::new();
        monitor.enumerate_macos_devices();
        
        assert!(!monitor.devices().is_empty(), "Should detect at least one device");
        
        for device in monitor.devices() {
            assert!(!device.name.is_empty(), "Device name should not be empty");
            assert!(device.path.exists() || device.path.to_string_lossy().starts_with("/"),
                "Device path should exist: {:?}", device.path);
        }
    }

    #[test]
    fn test_macos_volumes_path_exists() {
        let volumes_path = PathBuf::from("/Volumes");
        assert!(volumes_path.exists(), "/Volumes directory should exist on macOS");
    }

    #[test]
    fn test_macos_platform_adapter_enumerate() {
        use crate::models::platform_adapter::{get_platform_adapter, PlatformAdapter};
        
        let adapter = get_platform_adapter();
        let devices = adapter.enumerate_devices();
        
        assert!(!devices.is_empty(), "Should detect at least one device");
        
        let has_root = devices.iter().any(|d| d.path == PathBuf::from("/"));
        assert!(has_root, "Root volume should be detected");
    }

    #[test]
    fn test_macos_platform_adapter_available_filesystems() {
        use crate::models::platform_adapter::{get_platform_adapter, PlatformAdapter, FileSystemType};
        
        let adapter = get_platform_adapter();
        let filesystems = adapter.available_filesystems();
        
        assert!(filesystems.contains(&FileSystemType::Apfs), "APFS should be available");
        assert!(filesystems.contains(&FileSystemType::HfsPlus), "HFS+ should be available");
        assert!(filesystems.contains(&FileSystemType::ExFat), "exFAT should be available");
    }
}

#[test]
fn test_health_status_default() {
    assert_eq!(HealthStatus::default(), HealthStatus::Unknown);
}

#[test]
fn test_health_status_icon_names() {
    assert_eq!(HealthStatus::Good.icon_name(), "check");
    assert_eq!(HealthStatus::Warning.icon_name(), "triangle-alert");
    assert_eq!(HealthStatus::Critical.icon_name(), "triangle-alert");
    assert_eq!(HealthStatus::Unknown.icon_name(), "circle-question-mark");
}

#[test]
fn test_health_status_colors() {
    assert_eq!(HealthStatus::Good.color(), 0x3fb950);
    assert_eq!(HealthStatus::Warning.color(), 0xd29922);
    assert_eq!(HealthStatus::Critical.color(), 0xf85149);
    assert_eq!(HealthStatus::Unknown.color(), 0x8b949e);
}

#[test]
fn test_health_status_requires_attention() {
    assert!(!HealthStatus::Good.requires_attention());
    assert!(HealthStatus::Warning.requires_attention());
    assert!(HealthStatus::Critical.requires_attention());
    assert!(!HealthStatus::Unknown.requires_attention());
}

#[test]
fn test_smart_attribute_creation() {
    let attr = SmartAttribute::new(
        5,
        "Reallocated Sectors Count".to_string(),
        100,
        100,
        10,
        "0".to_string(),
    );

    assert_eq!(attr.id, 5);
    assert_eq!(attr.name, "Reallocated Sectors Count");
    assert_eq!(attr.value, 100);
    assert_eq!(attr.worst, 100);
    assert_eq!(attr.threshold, 10);
    assert_eq!(attr.raw_value, "0");
}

#[test]
fn test_smart_attribute_is_failing() {
    let failing = SmartAttribute::new(5, "Test".to_string(), 5, 5, 10, "100".to_string());
    assert!(failing.is_failing());

    let at_threshold = SmartAttribute::new(5, "Test".to_string(), 10, 10, 10, "100".to_string());
    assert!(at_threshold.is_failing());

    let healthy = SmartAttribute::new(5, "Test".to_string(), 100, 100, 10, "0".to_string());
    assert!(!healthy.is_failing());

    let no_threshold = SmartAttribute::new(5, "Test".to_string(), 100, 100, 0, "0".to_string());
    assert!(!no_threshold.is_failing());
}

#[test]
fn test_smart_attribute_is_warning() {
    let warning = SmartAttribute::new(5, "Test".to_string(), 15, 15, 10, "50".to_string());
    assert!(warning.is_warning());

    let healthy = SmartAttribute::new(5, "Test".to_string(), 100, 100, 10, "0".to_string());
    assert!(!healthy.is_warning());

    let failing = SmartAttribute::new(5, "Test".to_string(), 10, 10, 10, "100".to_string());
    assert!(!failing.is_warning());
}

#[test]
fn test_smart_attribute_standard_names() {
    assert_eq!(SmartAttribute::get_standard_name(5), "Reallocated Sectors Count");
    assert_eq!(SmartAttribute::get_standard_name(9), "Power-On Hours");
    assert_eq!(SmartAttribute::get_standard_name(194), "Temperature");
    assert_eq!(SmartAttribute::get_standard_name(197), "Current Pending Sector Count");
    assert_eq!(SmartAttribute::get_standard_name(255), "Unknown Attribute");
}

#[test]
fn test_smart_data_default() {
    let data = SmartData::default();
    assert_eq!(data.health_status, HealthStatus::Unknown);
    assert!(data.temperature_celsius.is_none());
    assert!(data.power_on_hours.is_none());
    assert!(data.reallocated_sectors.is_none());
    assert!(data.pending_sectors.is_none());
    assert!(data.attributes.is_empty());
}

#[test]
fn test_smart_data_from_attributes_healthy() {
    let attributes = vec![
        SmartAttribute::new(5, "Reallocated Sectors Count".to_string(), 100, 100, 10, "0".to_string()),
        SmartAttribute::new(9, "Power-On Hours".to_string(), 99, 99, 0, "1234".to_string()),
        SmartAttribute::new(194, "Temperature".to_string(), 65, 50, 0, "35".to_string()),
        SmartAttribute::new(197, "Current Pending Sector Count".to_string(), 100, 100, 0, "0".to_string()),
    ];

    let data = SmartData::from_attributes(attributes);

    assert_eq!(data.health_status, HealthStatus::Good);
    assert_eq!(data.temperature_celsius, Some(35));
    assert_eq!(data.power_on_hours, Some(1234));
    assert_eq!(data.reallocated_sectors, Some(0));
    assert_eq!(data.pending_sectors, Some(0));
}

#[test]
fn test_smart_data_from_attributes_warning() {
    let attributes = vec![
        SmartAttribute::new(5, "Reallocated Sectors Count".to_string(), 100, 100, 10, "5".to_string()),
        SmartAttribute::new(197, "Current Pending Sector Count".to_string(), 100, 100, 0, "0".to_string()),
    ];

    let data = SmartData::from_attributes(attributes);

    assert_eq!(data.health_status, HealthStatus::Warning);
    assert_eq!(data.reallocated_sectors, Some(5));
}

#[test]
fn test_smart_data_from_attributes_critical() {
    let attributes = vec![
        SmartAttribute::new(5, "Reallocated Sectors Count".to_string(), 100, 100, 10, "150".to_string()),
        SmartAttribute::new(197, "Current Pending Sector Count".to_string(), 100, 100, 0, "20".to_string()),
    ];

    let data = SmartData::from_attributes(attributes);

    assert_eq!(data.health_status, HealthStatus::Critical);
}

#[test]
fn test_smart_data_determine_health_status_temperature() {
    let mut data = SmartData::default();
    data.temperature_celsius = Some(55);
    assert_eq!(data.determine_health_status(), HealthStatus::Warning);

    data.temperature_celsius = Some(65);
    assert_eq!(data.determine_health_status(), HealthStatus::Critical);

    data.temperature_celsius = Some(35);
    assert_eq!(data.determine_health_status(), HealthStatus::Good);
}

#[test]
fn test_smart_data_health_summary() {
    let mut data = SmartData::default();
    data.health_status = HealthStatus::Good;
    assert_eq!(data.health_summary(), "Drive is healthy");

    data.health_status = HealthStatus::Critical;
    assert_eq!(data.health_summary(), "Drive health critical - backup data immediately!");

    data.health_status = HealthStatus::Unknown;
    assert_eq!(data.health_summary(), "Health data unavailable");

    data.health_status = HealthStatus::Warning;
    data.reallocated_sectors = Some(5);
    assert!(data.health_summary().contains("reallocated sectors"));
}

#[test]
fn test_smart_data_get_attribute() {
    let attributes = vec![
        SmartAttribute::new(5, "Reallocated Sectors Count".to_string(), 100, 100, 10, "0".to_string()),
        SmartAttribute::new(9, "Power-On Hours".to_string(), 99, 99, 0, "1234".to_string()),
    ];

    let data = SmartData::from_attributes(attributes);

    let attr = data.get_attribute(5);
    assert!(attr.is_some());
    assert_eq!(attr.unwrap().id, 5);

    let missing = data.get_attribute(194);
    assert!(missing.is_none());
}

#[test]
fn test_device_with_smart_status() {
    let device = Device::new(
        DeviceId::new(1),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    )
    .with_smart_status(HealthStatus::Warning);

    assert_eq!(device.smart_status, Some(HealthStatus::Warning));
    assert!(device.has_health_warning());
}

#[test]
fn test_device_has_health_warning() {
    let good = Device::new(
        DeviceId::new(1),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    )
    .with_smart_status(HealthStatus::Good);
    assert!(!good.has_health_warning());

    let warning = Device::new(
        DeviceId::new(2),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    )
    .with_smart_status(HealthStatus::Warning);
    assert!(warning.has_health_warning());

    let critical = Device::new(
        DeviceId::new(3),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    )
    .with_smart_status(HealthStatus::Critical);
    assert!(critical.has_health_warning());

    let no_status = Device::new(
        DeviceId::new(4),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    );
    assert!(!no_status.has_health_warning());
}

#[test]
fn test_device_with_encrypted() {
    let device = Device::new(
        DeviceId::new(1),
        "Test".to_string(),
        PathBuf::from("/test"),
        DeviceType::InternalDrive,
    )
    .with_encrypted(true);

    assert!(device.is_encrypted);
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_device_type()(variant in 0u8..8) -> DeviceType {
            match variant {
                0 => DeviceType::InternalDrive,
                1 => DeviceType::ExternalDrive,
                2 => DeviceType::UsbDrive,
                3 => DeviceType::NetworkDrive,
                4 => DeviceType::OpticalDrive,
                5 => DeviceType::DiskImage,
                6 => DeviceType::WslDistribution,
                _ => DeviceType::CloudStorage,
            }
        }
    }

    prop_compose! {
        fn arb_device()(
            id in 1u64..1000,
            name in "[a-zA-Z0-9 ]{1,50}",
            path in "[a-zA-Z0-9/]{1,100}",
            device_type in arb_device_type(),
            total_space in 0u64..u64::MAX,
            free_space_ratio in 0.0f64..=1.0,
            is_removable in any::<bool>(),
            is_read_only in any::<bool>(),
        ) -> Device {
            let free_space = (total_space as f64 * free_space_ratio) as u64;
            Device {
                id: DeviceId::new(id),
                name,
                path: PathBuf::from(path),
                device_type,
                total_space,
                free_space,
                is_removable,
                is_read_only,
                is_mounted: true,
                is_encrypted: false,
                smart_status: None,
            }
        }
    }

    proptest! {
        #[test]
        fn prop_device_detection_completeness(devices in prop::collection::vec(arb_device(), 0..20)) {
            let mut monitor = DeviceMonitor::new();

            let mut added_ids = Vec::new();
            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }

            prop_assert_eq!(monitor.devices().len(), devices.len());

            for id in &added_ids {
                prop_assert!(monitor.get_device(*id).is_some());
            }
        }
    }

    proptest! {
        #[test]
        fn prop_device_event_ordering(devices in prop::collection::vec(arb_device(), 1..10)) {
            let mut monitor = DeviceMonitor::new();
            let receiver = monitor.subscribe().unwrap();

            let mut added_ids = Vec::new();

            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }

            let mut connect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Connected(_) = event {
                    connect_events.push(event);
                }
            }

            for id in &added_ids {
                monitor.remove_device(*id);
            }

            let mut disconnect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Disconnected(_) = event {
                    disconnect_events.push(event);
                }
            }

            prop_assert_eq!(connect_events.len(), devices.len());
            prop_assert_eq!(disconnect_events.len(), devices.len());

            for (i, event) in disconnect_events.iter().enumerate() {
                if let DeviceEvent::Disconnected(id) = event {
                    prop_assert_eq!(*id, added_ids[i]);
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_device_space_accuracy(
            total in 0u64..u64::MAX,
            free_ratio in 0.0f64..=1.0,
        ) {
            let free = (total as f64 * free_ratio) as u64;

            let device = Device::new(
                DeviceId::new(1),
                "Test".to_string(),
                PathBuf::from("/test"),
                DeviceType::InternalDrive,
            )
            .with_space(total, free);

            prop_assert!(device.total_space >= device.free_space);

            prop_assert_eq!(device.used_space(), device.total_space.saturating_sub(device.free_space));

            let usage = device.usage_percentage();
            prop_assert!(usage >= 0.0 && usage <= 1.0);
        }
    }
}
