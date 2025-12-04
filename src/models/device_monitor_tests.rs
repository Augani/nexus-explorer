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

// Property-based tests using proptest
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
            }
        }
    }

    // **Feature: ui-enhancements, Property 45: Device Detection Completeness**
    proptest! {
        #[test]
        fn prop_device_detection_completeness(devices in prop::collection::vec(arb_device(), 0..20)) {
            let mut monitor = DeviceMonitor::new();

            // Add all devices
            let mut added_ids = Vec::new();
            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }

            // Verify all devices are present
            prop_assert_eq!(monitor.devices().len(), devices.len());

            // Verify each device can be found by ID
            for id in &added_ids {
                prop_assert!(monitor.get_device(*id).is_some());
            }
        }
    }

    // **Feature: ui-enhancements, Property 46: Device Event Ordering**
    proptest! {
        #[test]
        fn prop_device_event_ordering(devices in prop::collection::vec(arb_device(), 1..10)) {
            let mut monitor = DeviceMonitor::new();
            let receiver = monitor.subscribe().unwrap();

            let mut added_ids = Vec::new();

            // Add devices and collect IDs
            for device in &devices {
                let id = monitor.add_device(device.clone());
                added_ids.push(id);
            }

            // Collect connect events
            let mut connect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Connected(_) = event {
                    connect_events.push(event);
                }
            }

            // Remove devices in order
            for id in &added_ids {
                monitor.remove_device(*id);
            }

            // Collect disconnect events
            let mut disconnect_events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let DeviceEvent::Disconnected(_) = event {
                    disconnect_events.push(event);
                }
            }

            // Verify event counts match
            prop_assert_eq!(connect_events.len(), devices.len());
            prop_assert_eq!(disconnect_events.len(), devices.len());

            // Verify disconnect events are in same order as added_ids
            for (i, event) in disconnect_events.iter().enumerate() {
                if let DeviceEvent::Disconnected(id) = event {
                    prop_assert_eq!(*id, added_ids[i]);
                }
            }
        }
    }

    // **Feature: ui-enhancements, Property 48: Device Space Accuracy**
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

            // Property: total_space >= free_space
            prop_assert!(device.total_space >= device.free_space);

            // Property: used_space = total_space - free_space
            prop_assert_eq!(device.used_space(), device.total_space.saturating_sub(device.free_space));

            // Property: usage_percentage is between 0 and 1
            let usage = device.usage_percentage();
            prop_assert!(usage >= 0.0 && usage <= 1.0);
        }
    }
}
