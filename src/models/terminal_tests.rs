/// **Feature: ui-enhancements, Property 21: Terminal Virtualization**
/// **Validates: Requirements 6.8**
/// 
/// *For any* terminal state with scrollback content exceeding the visible area,
/// the visible_lines() iterator SHALL return exactly `rows` lines, and the
/// visible line range SHALL be correctly bounded within the total line buffer.

use proptest::prelude::*;
use crate::models::terminal::{TerminalState, DEFAULT_COLS, DEFAULT_ROWS};

/// Strategy for generating terminal dimensions
fn terminal_dimensions() -> impl Strategy<Value = (usize, usize)> {
    // cols: 20-120, rows: 5-50
    (20usize..=120, 5usize..=50)
}

/// Strategy for generating number of lines to write (can exceed visible rows)
fn line_count_strategy() -> impl Strategy<Value = usize> {
    1usize..200
}

/// Strategy for generating scroll offset
fn scroll_offset_strategy(max: usize) -> impl Strategy<Value = usize> {
    0..=max
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 21: Terminal Virtualization
    /// For any terminal with content exceeding visible area, visible_lines() returns exactly `rows` lines
    #[test]
    fn prop_visible_lines_count_equals_rows(
        (cols, rows) in terminal_dimensions(),
        line_count in line_count_strategy(),
    ) {
        let mut state = TerminalState::new(cols, rows);
        
        // Write lines to the terminal (may exceed visible area)
        for i in 0..line_count {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        let visible: Vec<_> = state.visible_lines().collect();
        prop_assert_eq!(
            visible.len(), 
            rows,
            "visible_lines() should return exactly {} lines, got {}",
            rows,
            visible.len()
        );
    }

    /// Property 21b: Scroll offset bounds
    /// For any scroll offset, the visible range stays within total lines
    #[test]
    fn prop_scroll_offset_bounds_visible_range(
        (cols, rows) in terminal_dimensions(),
        line_count in line_count_strategy(),
    ) {
        let mut state = TerminalState::new(cols, rows);
        
        for i in 0..line_count {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        let total_lines = state.total_lines();
        let max_scroll = state.max_scroll_offset();
        
        // Test various scroll positions
        for scroll in [0, max_scroll / 2, max_scroll] {
            if scroll <= max_scroll {
                state.scroll_to_bottom();
                state.scroll_viewport_up(scroll);
                
                prop_assert!(
                    state.scroll_offset() <= max_scroll,
                    "scroll_offset {} should be <= max_scroll_offset {}",
                    state.scroll_offset(),
                    max_scroll
                );
                
                // Verify visible lines are valid
                let visible: Vec<_> = state.visible_lines().collect();
                prop_assert_eq!(visible.len(), rows);
            }
        }
    }

    /// Property 21c: Scrollback lines calculation
    /// scrollback_lines() = total_lines - rows (when total > rows)
    #[test]
    fn prop_scrollback_lines_calculation(
        (cols, rows) in terminal_dimensions(),
        line_count in line_count_strategy(),
    ) {
        let mut state = TerminalState::new(cols, rows);
        
        for i in 0..line_count {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        let total = state.total_lines();
        let scrollback = state.scrollback_lines();
        
        if total > rows {
            prop_assert_eq!(
                scrollback, 
                total - rows,
                "scrollback_lines should be total_lines - rows"
            );
        } else {
            prop_assert_eq!(
                scrollback, 
                0,
                "scrollback_lines should be 0 when total <= rows"
            );
        }
    }

    /// Property 21d: Scroll to bottom resets offset
    /// After scroll_to_bottom(), scroll_offset should be 0
    #[test]
    fn prop_scroll_to_bottom_resets_offset(
        (cols, rows) in terminal_dimensions(),
        line_count in line_count_strategy(),
    ) {
        let mut state = TerminalState::new(cols, rows);
        
        for i in 0..line_count {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        // Scroll up some amount
        state.scroll_viewport_up(10);
        
        // Scroll to bottom
        state.scroll_to_bottom();
        
        prop_assert_eq!(
            state.scroll_offset(), 
            0,
            "scroll_to_bottom should set scroll_offset to 0"
        );
        prop_assert!(
            state.is_at_bottom(),
            "is_at_bottom should return true after scroll_to_bottom"
        );
    }

    /// Property 21e: Scroll to top sets max offset
    /// After scroll_to_top(), scroll_offset should equal max_scroll_offset
    #[test]
    fn prop_scroll_to_top_sets_max_offset(
        (cols, rows) in terminal_dimensions(),
        line_count in 50usize..200,
    ) {
        let mut state = TerminalState::new(cols, rows);
        
        for i in 0..line_count {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        let max_scroll = state.max_scroll_offset();
        
        // Only test if we have scrollback
        if max_scroll > 0 {
            state.scroll_to_top();
            
            prop_assert_eq!(
                state.scroll_offset(), 
                max_scroll,
                "scroll_to_top should set scroll_offset to max_scroll_offset"
            );
        }
    }

    /// Property 21f: Visible lines content consistency
    /// The visible lines should contain valid content from the line buffer
    #[test]
    fn prop_visible_lines_content_valid(
        (cols, rows) in terminal_dimensions(),
        line_count in 10usize..100,
    ) {
        let mut state = TerminalState::new(cols, rows);
        
        // Write numbered lines
        for i in 0..line_count {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        let visible: Vec<_> = state.visible_lines().collect();
        
        // Each visible line should have valid cells
        for line in visible {
            prop_assert!(
                line.len() == cols,
                "Each line should have {} cells, got {}",
                cols,
                line.len()
            );
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_virtualization_with_scrollback() {
        let mut state = TerminalState::new(80, 5);
        
        // Write 20 lines (15 in scrollback)
        for i in 0..20 {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        // Should have scrollback
        assert!(state.scrollback_lines() > 0);
        assert_eq!(state.total_lines(), 21);
        
        // Visible lines should be exactly 5
        let visible: Vec<_> = state.visible_lines().collect();
        assert_eq!(visible.len(), 5);
    }

    #[test]
    fn test_scroll_viewport_up_down() {
        let mut state = TerminalState::new(80, 5);
        
        for i in 0..20 {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        assert!(state.is_at_bottom());
        assert_eq!(state.scroll_offset(), 0);
        
        state.scroll_viewport_up(5);
        assert_eq!(state.scroll_offset(), 5);
        assert!(!state.is_at_bottom());
        
        state.scroll_viewport_down(3);
        assert_eq!(state.scroll_offset(), 2);
        
        // Scroll to bottom
        state.scroll_to_bottom();
        assert!(state.is_at_bottom());
    }

    #[test]
    fn test_max_scroll_offset_clamping() {
        let mut state = TerminalState::new(80, 5);
        
        // Write 10 lines (5 in scrollback)
        for i in 0..10 {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        
        let max = state.max_scroll_offset();
        
        // Try to scroll beyond max
        state.scroll_viewport_up(100);
        
        // Should be clamped to max
        assert_eq!(state.scroll_offset(), max);
    }
}
