/// Utilities for formatting values for display

/// Size unit constants
const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;
const TB: u64 = GB * 1024;

/// Format bytes to human-readable size string.
/// 
/// Returns a string with a numeric value between 0 and 1024 (exclusive for KB+)
/// followed by the appropriate unit (B, KB, MB, GB, TB).
/// 
/// # Examples
/// ```
/// use file_explorer::utils::format_size;
/// 
/// assert_eq!(format_size(0), "0 B");
/// assert_eq!(format_size(512), "512 B");
/// assert_eq!(format_size(1024), "1.0 KB");
/// assert_eq!(format_size(1536), "1.5 KB");
/// assert_eq!(format_size(1048576), "1.0 MB");
/// assert_eq!(format_size(1073741824), "1.0 GB");
/// assert_eq!(format_size(1099511627776), "1.0 TB");
/// ```
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

/// Format bytes to human-readable size string for file list display.
/// 
/// Returns "--" for directories, otherwise formats the size.
pub fn format_size_for_list(bytes: u64, is_dir: bool) -> String {
    if is_dir {
        "--".to_string()
    } else {
        format_size(bytes)
    }
}

/// Parse a formatted size string back to bytes.
/// 
/// This is useful for round-trip testing.
/// Returns None if the string cannot be parsed.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    
    // Try to split into number and unit
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

/// Check if disk space is critically low (less than 10% free)
pub fn is_space_critical(total: u64, free: u64) -> bool {
    if total == 0 {
        return false;
    }
    let usage_percent = ((total - free) as f64 / total as f64) * 100.0;
    usage_percent >= 90.0
}

/// Check if disk space is very low (less than 5% free)
pub fn is_space_very_low(total: u64, free: u64) -> bool {
    if total == 0 {
        return false;
    }
    let usage_percent = ((total - free) as f64 / total as f64) * 100.0;
    usage_percent >= 95.0
}

/// Calculate usage percentage (0.0 - 100.0)
pub fn usage_percentage(total: u64, free: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    ((total - free) as f64 / total as f64) * 100.0
}

/// Format disk space information for tooltip display.
/// 
/// Returns a multi-line string with total, used, and free space.
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

    /// **Feature: advanced-device-management, Property 6: Human-Readable Size Formatting**
    /// **Validates: Requirements 4.8**
    ///
    /// *For any* byte value B, the `format_size(B)` function SHALL return a string
    /// containing a numeric value and unit (B, KB, MB, GB, TB) where the numeric
    /// value is between 0 and 1024 (except for TB, the largest unit, which can exceed 1024).
    proptest! {
        #[test]
        fn prop_format_size_returns_valid_format(bytes in 0u64..u64::MAX) {
            let result = format_size(bytes);
            
            // Result should contain a space separating number and unit
            let parts: Vec<&str> = result.split_whitespace().collect();
            prop_assert_eq!(parts.len(), 2, "Result should have number and unit: {}", result);
            
            // First part should be a valid number
            let number: f64 = parts[0].parse()
                .expect(&format!("First part should be a number: {}", parts[0]));
            
            // Number should be non-negative
            prop_assert!(number >= 0.0, "Number should be non-negative: {}", number);
            
            // For units KB, MB, GB (not B or TB), number should be less than 1024
            // TB is the largest unit so it can exceed 1024
            // B can be 0-1023 (less than 1024)
            let unit = parts[1];
            if unit != "B" && unit != "TB" {
                prop_assert!(number < 1024.0, "Number should be < 1024 for {}: {}", unit, number);
            }
            
            // Unit should be one of the valid units
            let valid_units = ["B", "KB", "MB", "GB", "TB"];
            prop_assert!(valid_units.contains(&unit), "Invalid unit: {}", unit);
        }
        
        #[test]
        fn prop_format_size_unit_matches_magnitude(bytes in 0u64..u64::MAX) {
            let result = format_size(bytes);
            let parts: Vec<&str> = result.split_whitespace().collect();
            let unit = parts[1];
            
            // Verify the unit matches the expected magnitude
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
        // 10% free = 90% used = critical
        assert!(is_space_critical(100, 10));
        assert!(is_space_critical(1000, 100));
        
        // 11% free = 89% used = not critical
        assert!(!is_space_critical(100, 11));
        
        // Edge case: 0 total
        assert!(!is_space_critical(0, 0));
    }

    #[test]
    fn test_is_space_very_low() {
        // 5% free = 95% used = very low
        assert!(is_space_very_low(100, 5));
        assert!(is_space_very_low(1000, 50));
        
        // 6% free = 94% used = not very low
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
        
        // Invalid inputs
        assert_eq!(parse_size("invalid"), None);
        assert_eq!(parse_size("1.0 XB"), None);
        assert_eq!(parse_size(""), None);
    }

    // Edge case tests for Requirements 4.1, 4.2, 4.3
    
    #[test]
    fn test_format_size_edge_cases() {
        // Boundary between units
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        
        assert_eq!(format_size(1048575), "1024.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        
        assert_eq!(format_size(1073741823), "1024.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        
        assert_eq!(format_size(1099511627775), "1024.0 GB");
        assert_eq!(format_size(1099511627776), "1.0 TB");
        
        // Maximum u64 value
        let max_u64 = u64::MAX;
        let result = format_size(max_u64);
        assert!(result.contains("TB"), "Max u64 should be in TB: {}", result);
    }

    #[test]
    fn test_warning_thresholds_edge_cases() {
        // Exactly at 90% usage (10% free) - should be critical
        assert!(is_space_critical(1000, 100));
        
        // Just below 90% usage (10.1% free) - should not be critical
        assert!(!is_space_critical(1000, 101));
        
        // Exactly at 95% usage (5% free) - should be very low
        assert!(is_space_very_low(1000, 50));
        
        // Just below 95% usage (5.1% free) - should not be very low
        assert!(!is_space_very_low(1000, 51));
        
        // 100% usage (0% free) - should be both critical and very low
        assert!(is_space_critical(1000, 0));
        assert!(is_space_very_low(1000, 0));
        
        // 0% usage (100% free) - should be neither
        assert!(!is_space_critical(1000, 1000));
        assert!(!is_space_very_low(1000, 1000));
    }

    #[test]
    fn test_usage_percentage_edge_cases() {
        // Normal cases
        assert!((usage_percentage(1000, 500) - 50.0).abs() < 0.001);
        assert!((usage_percentage(1000, 250) - 75.0).abs() < 0.001);
        
        // Edge cases
        assert_eq!(usage_percentage(0, 0), 0.0); // Zero total
        assert_eq!(usage_percentage(100, 100), 0.0); // 0% used
        assert_eq!(usage_percentage(100, 0), 100.0); // 100% used
        
        // Large values
        let tb = 1099511627776u64;
        let half_tb = tb / 2;
        assert!((usage_percentage(tb, half_tb) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_format_space_tooltip_edge_cases() {
        // Zero space
        let tooltip = format_space_tooltip(0, 0);
        assert!(tooltip.contains("Total: 0 B"));
        assert!(tooltip.contains("Used: 0 B"));
        assert!(tooltip.contains("Free: 0 B"));
        
        // Full disk
        let tooltip = format_space_tooltip(1073741824, 0);
        assert!(tooltip.contains("Total: 1.0 GB"));
        assert!(tooltip.contains("Used: 1.0 GB"));
        assert!(tooltip.contains("100.0%"));
        assert!(tooltip.contains("Free: 0 B"));
        
        // Empty disk
        let tooltip = format_space_tooltip(1073741824, 1073741824);
        assert!(tooltip.contains("Total: 1.0 GB"));
        assert!(tooltip.contains("Used: 0 B"));
        assert!(tooltip.contains("0.0%"));
        assert!(tooltip.contains("Free: 1.0 GB"));
    }

    #[test]
    fn test_format_size_precision() {
        // Test that precision is consistent
        assert_eq!(format_size(1536), "1.5 KB"); // 1.5 KB
        assert_eq!(format_size(1587), "1.5 KB"); // Rounds to 1.5 KB
        assert_eq!(format_size(1638), "1.6 KB"); // Rounds to 1.6 KB
        
        // Test decimal precision for larger units
        assert_eq!(format_size(1610612736), "1.5 GB"); // 1.5 GB
        assert_eq!(format_size(1649267441664), "1.5 TB"); // 1.5 TB
    }
}
