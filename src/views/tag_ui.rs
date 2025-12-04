use gpui::{div, prelude::*, px, Hsla, IntoElement, ParentElement, SharedString, Styled};

use crate::models::{Tag, TagColor, TagId, TagManager};
use std::path::Path;

/// Renders a single tag color dot
pub fn render_tag_dot(color: TagColor) -> impl IntoElement {
    let (r, g, b, _) = color.to_rgba();
    let hsla = Hsla::from(gpui::Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    });
    
    div()
        .size(px(8.0))
        .rounded_full()
        .bg(hsla)
        .flex_shrink_0()
}

/// Renders multiple tag dots for a file
pub fn render_tag_dots(tags: &[&Tag]) -> impl IntoElement {
    let mut container = div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .flex_shrink_0();
    
    // Show up to 3 dots, then show a count
    let max_visible = 3;
    let visible_tags = tags.iter().take(max_visible);
    
    for tag in visible_tags {
        container = container.child(render_tag_dot(tag.color));
    }
    
    if tags.len() > max_visible {
        let remaining = tags.len() - max_visible;
        container = container.child(
            div()
                .text_xs()
                .text_color(gpui::rgb(0x8b949e))
                .child(format!("+{}", remaining))
        );
    }
    
    container
}

/// Renders tag dots for a file path using the TagManager
pub fn render_file_tag_dots(tags: Vec<&Tag>) -> impl IntoElement {
    if tags.is_empty() {
        div().w(px(0.0)).flex().items_center()
    } else {
        let mut container = div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .flex_shrink_0();
        
        // Show up to 3 dots, then show a count
        let max_visible = 3;
        let visible_tags = tags.iter().take(max_visible);
        
        for tag in visible_tags {
            container = container.child(render_tag_dot(tag.color));
        }
        
        if tags.len() > max_visible {
            let remaining = tags.len() - max_visible;
            container = container.child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(0x8b949e))
                    .child(format!("+{}", remaining))
            );
        }
        
        container
    }
}

/// Renders a tag filter item for the sidebar
pub fn render_tag_filter_item(
    tag: &Tag,
    is_selected: bool,
    file_count: usize,
) -> impl IntoElement {
    let theme_bg_hover = gpui::rgb(0x161b22);
    let theme_bg_selected = gpui::rgb(0x1f3a5f);
    let theme_text_primary = gpui::rgb(0xc9d1d9);
    let theme_text_secondary = gpui::rgb(0x8b949e);
    
    div()
        .id(SharedString::from(format!("tag-filter-{}", tag.id.0)))
        .h(px(32.0))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .rounded_md()
        .when(is_selected, |s| s.bg(theme_bg_selected))
        .when(!is_selected, |s| s.hover(|h| h.bg(theme_bg_hover)))
        .child(render_tag_dot(tag.color))
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme_text_primary)
                .child(tag.name.clone())
        )
        .child(
            div()
                .text_xs()
                .text_color(theme_text_secondary)
                .child(format!("{}", file_count))
        )
}

/// Renders the tag context menu for applying/removing tags
pub fn render_tag_context_menu(
    tag_manager: &TagManager,
    file_path: &Path,
) -> impl IntoElement {
    let theme_bg = gpui::rgb(0x161b22);
    let theme_border = gpui::rgb(0x30363d);
    let theme_text = gpui::rgb(0xc9d1d9);
    let theme_hover = gpui::rgb(0x21262d);
    
    let file_tags = tag_manager.tags_for_file(file_path);
    let file_tag_ids: std::collections::HashSet<TagId> = file_tags.iter().map(|t| t.id).collect();
    
    let mut menu = div()
        .bg(theme_bg)
        .border_1()
        .border_color(theme_border)
        .rounded_md()
        .py_1()
        .min_w(px(160.0))
        .shadow_lg();
    
    menu = menu.child(
        div()
            .px_3()
            .py_1()
            .text_xs()
            .text_color(gpui::rgb(0x8b949e))
            .child("Tags")
    );
    
    for tag in tag_manager.all_tags() {
        let has_tag = file_tag_ids.contains(&tag.id);
        
        menu = menu.child(
            div()
                .id(SharedString::from(format!("tag-menu-{}", tag.id.0)))
                .px_3()
                .py_1()
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .hover(|s| s.bg(theme_hover))
                .child(render_tag_dot(tag.color))
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(theme_text)
                        .child(tag.name.clone())
                )
                .when(has_tag, |s| {
                    s.child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x3fb950))
                            .child("✓")
                    )
                })
        );
    }
    
    menu
}

/// Parses a search query for tag filters
/// Returns (remaining_query, tag_filters)
/// Supports syntax like "tag:red" or "tag:work"
pub fn parse_tag_query(query: &str) -> (String, Vec<String>) {
    let mut remaining_parts = Vec::new();
    let mut tag_filters = Vec::new();
    
    for part in query.split_whitespace() {
        if let Some(tag_name) = part.strip_prefix("tag:") {
            if !tag_name.is_empty() {
                tag_filters.push(tag_name.to_string());
            }
        } else {
            remaining_parts.push(part);
        }
    }
    
    (remaining_parts.join(" "), tag_filters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag_query_no_tags() {
        let (query, tags) = parse_tag_query("hello world");
        assert_eq!(query, "hello world");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_parse_tag_query_single_tag() {
        let (query, tags) = parse_tag_query("tag:red document");
        assert_eq!(query, "document");
        assert_eq!(tags, vec!["red"]);
    }

    #[test]
    fn test_parse_tag_query_multiple_tags() {
        let (query, tags) = parse_tag_query("tag:red tag:work important");
        assert_eq!(query, "important");
        assert_eq!(tags, vec!["red", "work"]);
    }

    #[test]
    fn test_parse_tag_query_only_tags() {
        let (query, tags) = parse_tag_query("tag:blue tag:green");
        assert_eq!(query, "");
        assert_eq!(tags, vec!["blue", "green"]);
    }

    #[test]
    fn test_parse_tag_query_empty_tag() {
        let (query, tags) = parse_tag_query("tag: document");
        assert_eq!(query, "document");
        assert!(tags.is_empty());
    }
}
