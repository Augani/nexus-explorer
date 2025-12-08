/

/
const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;
const TB: u64 = GB * 1024;

/
/
/
/
/
/
/
/
/
/
/
/
/
/
/
/
/
pub fn format_size(bytes: u64) -> String {
    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/
/
/
pub fn format_size_for_list(bytes: u64, is_dir: bool) -> String {
    if is_dir {
        "--".to_string()
    } else {
        format_size(bytes)
    }
}

/
/
/
/
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    
    let value: f64 = parts[0].parse().ok()?;
    let unit = parts[1].to_uppercase();
    
    let multiplier = match unit.as_str() {
        "B" => 1,
        "KB" => KB,
        "MB" => MB,
        "GB" => GB,
        "TB" => TB,
        _ => return None,
    };
    
    Some((value * multiplier as f64) as u64)
}

/
pub fn is_space_critical(total: u64, free: u64) -> bool {
    if total == 0 {
        return false;
    }
    let usage_percent = ((total - free) as f64 / total as f64) * 100.0;
    usage_percent >= 90.0
}

/
pub fn is_space_very_low(total: u64, free: u64) -> bool {
    if total == 0 {
        return false;
    }
    let usage_percent = ((total - free) as f64 / total as f64) * 100.0;
    usage_percent >= 95.0
}

/
pub fn usage_percentage(total: u64, free: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    ((total - free) as f64 / total as f64) * 100.0
}

/
/
/
pub fn format_space_tooltip(total: u64, free: u64) -> String {
    let used = total.saturating_sub(free);
    let percent = usage_percentage(total, free);
    
    format!(
        "Total: {}\nUsed: {} ({:.1}%)\nFree: {}",
        format_size(total),
        format_size(used),
        percent,
        format_size(free)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /
    /
    /
    /
    /
    /
    proptest! {
        #[test]
        fn prop_format_size_returns_valid_format(bytes in 0u64..u64::MAX) {
            let result = format_size(bytes);
            
            let parts: Vec<&str> = result.split_whitespace().collect();
            prop_assert_eq!(parts.len(), 2, "Result should have number and unit: {}", result);
            
            let number: f64 = parts[0].parse()
                .expect(&format!("First part should be a number: {}", parts[0]));
            
            prop_assert!(number >= 0.0, "Number should be non-negative: {}", number);
            
            let unit = parts[1];
            if unit != "B" && unit != "TB" {
                prop_assert!(number < 1024.0, "Number should be < 1024 for {}: {}", unit, number);
            }
            
            let valid_units = ["B", "KB", "MB", "GB", "TB"];
            prop_assert!(valid_units.contains(&unit), "Invalid unit: {}", unit);
        }
        
        #[test]
        fn prop_format_size_unit_matches_magnitude(bytes in 0u64..u64::MAX) {
            let result = format_size(bytes);
            let parts: Vec<&str> = result.split_whitespace().collect();
            let unit = parts[1];
            
            let expected_unit = if bytes >= TB {
                "TB"
            } else if bytes >= GB {
                "GB"
            } else if bytes >= MB {
                "MB"
            } else if bytes >= KB {
                "KB"
            } else {
                "B"
            };
            
            prop_assert_eq!(unit, expected_unit, 
                "Unit mismatch for {} bytes: got {}, expected {}", bytes, unit, expected_unit);
        }
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(10240), "10.0 KB");
        assert_eq!(format_size(1048575), "1024.0 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1572864), "1.5 MB");
        assert_eq!(format_size(104857600), "100.0 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(1610612736), "1.5 GB");
        assert_eq!(format_size(107374182400), "100.0 GB");
    }

    #[test]
    fn test_format_size_terabytes() {
        assert_eq!(format_size(1099511627776), "1.0 TB");
        assert_eq!(format_size(1649267441664), "1.5 TB");
        assert_eq!(format_size(10995116277760), "10.0 TB");
    }

    #[test]
    fn test_format_size_for_list() {
        assert_eq!(format_size_for_list(1024, false), "1.0 KB");
        assert_eq!(format_size_for_list(1024, true), "--");
        assert_eq!(format_size_for_list(0, true), "--");
    }

    #[test]
    fn test_is_space_critical() {
        assert!(is_space_critical(100, 10));
        assert!(is_space_critical(1000, 100));
        
        assert!(!is_space_critical(100, 11));
        
        assert!(!is_space_critical(0, 0));
    }

    #[test]
    fn test_is_space_very_low() {
        assert!(is_space_very_low(100, 5));
        assert!(is_space_very_low(1000, 50));
        
        assert!(!is_space_very_low(100, 6));
    }

    #[test]
    fn test_usage_percentage() {
        assert_eq!(usage_percentage(100, 50), 50.0);
        assert_eq!(usage_percentage(100, 0), 100.0);
        assert_eq!(usage_percentage(100, 100), 0.0);
        assert_eq!(usage_percentage(0, 0), 0.0);
    }

    #[test]
    fn test_format_space_tooltip() {
        let tooltip = format_space_tooltip(1073741824, 536870912);
        assert!(tooltip.contains("Total: 1.0 GB"));
        assert!(tooltip.contains("Used: 512.0 MB"));
        assert!(tooltip.contains("50.0%"));
        assert!(tooltip.contains("Free: 512.0 MB"));
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("0 B"), Some(0));
        assert_eq!(parse_size("512 B"), Some(512));
        assert_eq!(parse_size("1.0 KB"), Some(1024));
        assert_eq!(parse_size("1.5 KB"), Some(1536));
        assert_eq!(parse_size("1.0 MB"), Some(1048576));
        assert_eq!(parse_size("1.0 GB"), Some(1073741824));
        assert_eq!(parse_size("1.0 TB"), Some(1099511627776));
        
        assert_eq!(parse_size("invalid"), None);
        assert_eq!(parse_size("1.0 XB"), None);
        assert_eq!(parse_size(""), None);
    }

    
    #[test]
    fn test_format_size_edge_cases() {
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        
        assert_eq!(format_size(1048575), "1024.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        
        assert_eq!(format_size(1073741823), "1024.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        
        assert_eq!(format_size(1099511627775), "1024.0 GB");
        assert_eq!(format_size(1099511627776), "1.0 TB");
        
        let max_u64 = u64::MAX;
        let result = format_size(max_u64);
        assert!(result.contains("TB"), "Max u64 should be in TB: {}", result);
    }

    #[test]
    fn test_warning_thresholds_edge_cases() {
        assert!(is_space_critical(1000, 100));
        
        assert!(!is_space_critical(1000, 101));
        
        assert!(is_space_very_low(1000, 50));
        
        assert!(!is_space_very_low(1000, 51));
        
        assert!(is_space_critical(1000, 0));
        assert!(is_space_very_low(1000, 0));
        
        assert!(!is_space_critical(1000, 1000));
        assert!(!is_space_very_low(1000, 1000));
    }

    #[test]
    fn test_usage_percentage_edge_cases() {
        assert!((usage_percentage(1000, 500) - 50.0).abs() < 0.001);
        assert!((usage_percentage(1000, 250) - 75.0).abs() < 0.001);
        
        assert_eq!(usage_percentage(0, 0), 0.0);
        assert_eq!(usage_percentage(100, 100), 0.0);
        assert_eq!(usage_percentage(100, 0), 100.0);
        
        let tb = 1099511627776u64;
        let half_tb = tb / 2;
        assert!((usage_percentage(tb, half_tb) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_format_space_tooltip_edge_cases() {
        let tooltip = format_space_tooltip(0, 0);
        assert!(tooltip.contains("Total: 0 B"));
        assert!(tooltip.contains("Used: 0 B"));
        assert!(tooltip.contains("Free: 0 B"));
        
        let tooltip = format_space_tooltip(1073741824, 0);
        assert!(tooltip.contains("Total: 1.0 GB"));
        assert!(tooltip.contains("Used: 1.0 GB"));
        assert!(tooltip.contains("100.0%"));
        assert!(tooltip.contains("Free: 0 B"));
        
        let tooltip = format_space_tooltip(1073741824, 1073741824);
        assert!(tooltip.contains("Total: 1.0 GB"));
        assert!(tooltip.contains("Used: 0 B"));
        assert!(tooltip.contains("0.0%"));
        assert!(tooltip.contains("Free: 1.0 GB"));
    }

    #[test]
    fn test_format_size_precision() {
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1587), "1.5 KB");
        assert_eq!(format_size(1638), "1.6 KB");
        
        assert_eq!(format_size(1610612736), "1.5 GB");
        assert_eq!(format_size(1649267441664), "1.5 TB");
    }
}
