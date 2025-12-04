use std::path::PathBuf;

use gpui::{
    anchored, div, prelude::*, px, svg, App, Context, Corner, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    Render, SharedString, Styled, Window,
};

use super::file_list::{get_file_icon, get_file_icon_color, ContextMenuAction};
use crate::models::{theme_colors, FileEntry, GridConfig};

pub struct GridView {
    entries: Vec<FileEntry>,
    config: GridConfig,
    selected_index: Option<usize>,
    viewport_width: f32,
}

pub struct GridViewComponent {
    grid_view: GridView,
    focus_handle: FocusHandle,
    pending_navigation: Option<PathBuf>,
    context_menu_position: Option<Point<Pixels>>,
    context_menu_index: Option<usize>,
    pending_context_action: Option<ContextMenuAction>,
    show_open_with_submenu: bool,
}

impl GridView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            config: GridConfig::default(),
            selected_index: None,
            viewport_width: 800.0,
        }
    }

    pub fn with_config(config: GridConfig) -> Self {
        Self {
            entries: Vec::new(),
            config,
            selected_index: None,
            viewport_width: 800.0,
        }
    }

    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.selected_index = None;
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn item_count(&self) -> usize {
        self.entries.len()
    }

    pub fn config(&self) -> &GridConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: GridConfig) {
        self.config = config;
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn set_viewport_width(&mut self, width: f32) {
        self.viewport_width = width;
    }

    pub fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    pub fn columns(&self) -> usize {
        self.config.columns_for_width(self.viewport_width)
    }

    pub fn rows(&self) -> usize {
        self.config
            .rows_for_items(self.entries.len(), self.viewport_width)
    }

    pub fn content_height(&self) -> f32 {
        self.config
            .content_height(self.entries.len(), self.viewport_width)
    }
}

impl Default for GridView {
    fn default() -> Self {
        Self::new()
    }
}

impl GridViewComponent {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            grid_view: GridView::new(),
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_index: None,
            pending_context_action: None,
            show_open_with_submenu: false,
        }
    }

    pub fn with_grid_view(grid_view: GridView, cx: &mut Context<Self>) -> Self {
        Self {
            grid_view,
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_index: None,
            pending_context_action: None,
            show_open_with_submenu: false,
        }
    }

    pub fn inner(&self) -> &GridView {
        &self.grid_view
    }

    pub fn inner_mut(&mut self) -> &mut GridView {
        &mut self.grid_view
    }

    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu_position = None;
        self.context_menu_index = None;
        self.show_open_with_submenu = false;
    }

    pub fn take_pending_context_action(&mut self) -> Option<ContextMenuAction> {
        self.pending_context_action.take()
    }

    pub fn select_item(&mut self, index: usize, cx: &mut Context<Self>) {
        self.grid_view.selected_index = Some(index);
        cx.notify();
    }

    pub fn open_item(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(entry) = self.grid_view.entries.get(index) {
            if entry.is_dir {
                self.pending_navigation = Some(entry.path.clone());
                cx.notify();
            }
        }
    }
}

impl Focusable for GridViewComponent {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GridViewComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total_items = self.grid_view.item_count();
        let config = self.grid_view.config;
        let selected_index = self.grid_view.selected_index;
        let context_menu_pos = self.context_menu_position;
        let _context_menu_idx = self.context_menu_index;

        let theme = theme_colors();
        let bg_darker = theme.bg_void;
        let bg_dark = theme.bg_secondary;
        let border_color = theme.border_default;
        let border_subtle = theme.border_subtle;
        let text_gray = theme.text_muted;
        let text_light = theme.text_primary;
        let hover_bg = theme.bg_hover;
        let selected_bg = theme.bg_selected;
        let folder_color = theme.accent_primary;
        let folder_open_color = theme.accent_secondary;
        let menu_bg = theme.bg_tertiary;

        div()
            .id("grid-view")
            .size_full()
            .bg(bg_darker)
            .flex()
            .flex_col()
            .relative()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _event, _window, cx| {
                    view.close_context_menu();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .flex()
                    .h(px(36.0))
                    .bg(bg_dark)
                    .border_b_1()
                    .border_color(border_color)
                    .px_4()
                    .items_center()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_gray)
                    .child(format!("{} items", total_items)),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_4()
                    .when(total_items == 0, |this| {
                        this.flex().items_center().justify_center().child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .text_color(text_gray)
                                .child(
                                    svg()
                                        .path("assets/icons/folder-open.svg")
                                        .size(px(48.0))
                                        .text_color(border_subtle)
                                        .mb_4(),
                                )
                                .child("Folder is empty"),
                        )
                    })
                    .when(total_items > 0, |this| {
                        let entity = cx.entity().clone();
                        let entries = self.grid_view.entries.clone();

                        this.child(div().flex().flex_wrap().gap(px(config.gap)).children(
                            entries.iter().enumerate().map(|(ix, entry)| {
                                let is_selected = selected_index == Some(ix);
                                let is_dir = entry.is_dir;
                                let name = entry.name.clone();
                                let icon_name = get_file_icon(&name, is_dir);
                                let icon_color = if is_dir {
                                    if is_selected {
                                        folder_open_color
                                    } else {
                                        folder_color
                                    }
                                } else {
                                    get_file_icon_color(&name)
                                };
                                let entry_path = entry.path.clone();
                                let entity = entity.clone();
                                let entity_for_ctx = entity.clone();

                                div()
                                    .id(SharedString::from(format!("grid-item-{}", ix)))
                                    .w(px(config.item_width))
                                    .h(px(config.item_height))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .p_2()
                                    .rounded_lg()
                                    .cursor_pointer()
                                    .when(is_selected, |s| s.bg(selected_bg))
                                    .when(!is_selected, |s| s.hover(|h| h.bg(hover_bg)))
                                    .on_click({
                                        let entry_path = entry_path.clone();
                                        let entity = entity.clone();
                                        move |event, _window, cx| {
                                            entity.update(cx, |view, cx| {
                                                view.close_context_menu();
                                                if event.click_count() == 2 && is_dir {
                                                    view.pending_navigation =
                                                        Some(entry_path.clone());
                                                } else {
                                                    view.grid_view.selected_index = Some(ix);
                                                }
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_mouse_down(MouseButton::Right, {
                                        let entity = entity_for_ctx.clone();
                                        move |event: &MouseDownEvent, _window, cx| {
                                            entity.update(cx, |view, cx| {
                                                view.grid_view.selected_index = Some(ix);
                                                view.context_menu_position = Some(event.position);
                                                view.context_menu_index = Some(ix);
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .child(
                                        svg()
                                            .path(SharedString::from(format!(
                                                "assets/icons/{}.svg",
                                                icon_name
                                            )))
                                            .size(px(config.icon_size))
                                            .text_color(icon_color),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .text_center()
                                            .text_xs()
                                            .text_color(if is_selected {
                                                theme.text_primary
                                            } else {
                                                text_light
                                            })
                                            .truncate()
                                            .child(truncate_name(&name, 14)),
                                    )
                            }),
                        ))
                    }),
            )
            .child(
                div()
                    .h(px(28.0))
                    .bg(bg_dark)
                    .border_t_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .text_xs()
                    .text_color(text_gray)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                svg()
                                    .path("assets/icons/grid-2x2.svg")
                                    .size(px(12.0))
                                    .text_color(text_gray),
                            )
                            .child(format!("{} items", total_items)),
                    )
                    .child(div().flex().items_center().gap_3().child("Grid View")),
            )
            .when_some(context_menu_pos, |this, pos| {
                let entity = cx.entity().clone();
                let selected_entry = self
                    .context_menu_index
                    .and_then(|idx| self.grid_view.entries.get(idx).cloned());
                let is_dir = selected_entry.as_ref().map(|e| e.is_dir).unwrap_or(false);

                this.child(
                    anchored()
                        .snap_to_window_with_margin(px(8.0))
                        .anchor(Corner::TopLeft)
                        .position(pos)
                        .child(
                            div()
                                .id("grid-view-context-menu")
                                .occlude()
                                .w(px(220.0))
                                .bg(menu_bg)
                                .border_1()
                                .border_color(border_color)
                                .rounded_lg()
                                .shadow_lg()
                                .py_1()
                                .on_mouse_down_out(cx.listener(|view, _, _, cx| {
                                    view.close_context_menu();
                                    cx.notify();
                                }))
                                .child(render_context_menu_item(
                                    "folder-open",
                                    "Open",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::Open(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_grid_open_with_submenu(
                                    selected_entry.clone(),
                                    self.show_open_with_submenu,
                                    text_light,
                                    hover_bg,
                                    menu_bg,
                                    border_color,
                                    entity.clone(),
                                ))
                                .when(is_dir, |this| {
                                    let entity = entity.clone();
                                    let entity2 = entity.clone();
                                    let entry = selected_entry.clone();
                                    let entry2 = selected_entry.clone();
                                    this.child(render_context_menu_item(
                                        "app-window",
                                        "Open in New Window",
                                        text_light,
                                        hover_bg,
                                        {
                                            move |_window, cx| {
                                                if let Some(ref e) = entry {
                                                    entity.update(cx, |view, cx| {
                                                        view.pending_context_action = Some(
                                                            ContextMenuAction::OpenInNewWindow(
                                                                e.path.clone(),
                                                            ),
                                                        );
                                                        view.close_context_menu();
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(
                                        render_context_menu_item(
                                            "folder-plus",
                                            "Open in New Tab",
                                            text_light,
                                            hover_bg,
                                            {
                                                move |_window, cx| {
                                                    if let Some(ref e) = entry2 {
                                                        entity2.update(cx, |view, cx| {
                                                            view.pending_context_action = Some(
                                                                ContextMenuAction::OpenInNewTab(
                                                                    e.path.clone(),
                                                                ),
                                                            );
                                                            view.close_context_menu();
                                                            cx.notify();
                                                        });
                                                    }
                                                }
                                            },
                                        ),
                                    )
                                })
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item(
                                    "eye",
                                    "Quick Look",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action =
                                                        Some(ContextMenuAction::QuickLook(
                                                            e.path.clone(),
                                                        ));
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "info",
                                    "Get Info",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::GetInfo(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item(
                                    "pen",
                                    "Rename",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::Rename(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "copy",
                                    "Copy",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::Copy(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "scissors",
                                    "Cut",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::Cut(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "clipboard-paste",
                                    "Paste",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        move |_window, cx| {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action =
                                                    Some(ContextMenuAction::Paste);
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "files",
                                    "Duplicate",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action =
                                                        Some(ContextMenuAction::Duplicate(
                                                            e.path.clone(),
                                                        ));
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item(
                                    "archive",
                                    "Compress",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::Compress(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "share-2",
                                    "Share...",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::Share(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item(
                                    "link",
                                    "Copy Path",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(
                                                        ContextMenuAction::CopyPath(e.path.clone()),
                                                    );
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "folder-search",
                                    "Show in Finder",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action =
                                                        Some(ContextMenuAction::ShowInFinder(
                                                            e.path.clone(),
                                                        ));
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_item(
                                    "star",
                                    "Add to Favorites",
                                    text_light,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action =
                                                        Some(ContextMenuAction::AddToFavorites(
                                                            e.path.clone(),
                                                        ));
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                ))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item(
                                    "trash-2",
                                    "Move to Trash",
                                    theme.error,
                                    hover_bg,
                                    {
                                        let entity = entity.clone();
                                        let entry = selected_entry.clone();
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action =
                                                        Some(ContextMenuAction::MoveToTrash(
                                                            e.path.clone(),
                                                        ));
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    },
                                )),
                        ),
                )
            })
    }
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len.saturating_sub(3)])
    }
}

fn render_context_menu_item<F>(
    icon_name: &'static str,
    label: &'static str,
    text_color: gpui::Rgba,
    hover_bg: gpui::Rgba,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    div()
        .id(SharedString::from(format!("grid-ctx-{}", label)))
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_1p5()
        .mx_1()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .text_color(text_color)
        .hover(|s| s.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            on_click(window, cx);
        })
        .child(
            svg()
                .path(SharedString::from(format!(
                    "assets/icons/{}.svg",
                    icon_name
                )))
                .size(px(14.0))
                .text_color(text_color),
        )
        .child(label)
}

fn render_context_menu_divider(color: gpui::Rgba) -> impl IntoElement {
    div().h(px(1.0)).mx_2().my_1().bg(color)
}

fn render_grid_open_with_submenu(
    selected_entry: Option<FileEntry>,
    show_submenu: bool,
    text_color: gpui::Rgba,
    hover_bg: gpui::Rgba,
    _menu_bg: gpui::Rgba,
    border_color: gpui::Rgba,
    entity: gpui::Entity<GridViewComponent>,
) -> impl IntoElement {
    let apps = selected_entry
        .as_ref()
        .map(|e| crate::models::get_apps_for_file(&e.path))
        .unwrap_or_default();

    let has_apps = !apps.is_empty();
    let entry_for_other = selected_entry.clone();
    let entity_for_toggle = entity.clone();

    div()
        .id("grid-open-with-menu-wrapper")
        .flex()
        .flex_col()
        .child(
            div()
                .id("grid-open-with-trigger")
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .py_1p5()
                .mx_1()
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(text_color)
                .hover(|s| s.bg(hover_bg))
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    entity_for_toggle.update(cx, |view, cx| {
                        view.show_open_with_submenu = !view.show_open_with_submenu;
                        cx.notify();
                    });
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            svg()
                                .path("assets/icons/external-link.svg")
                                .size(px(14.0))
                                .text_color(text_color),
                        )
                        .child("Open With"),
                )
                .child(
                    svg()
                        .path(if show_submenu {
                            "assets/icons/chevron-down.svg"
                        } else {
                            "assets/icons/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(text_color),
                ),
        )
        .when(show_submenu, move |this| {
            this.child(
                div()
                    .id("grid-open-with-inline-list")
                    .flex()
                    .flex_col()
                    .pl_4()
                    .border_l_1()
                    .border_color(border_color)
                    .ml_4()
                    .when(has_apps, |submenu| {
                        let mut submenu = submenu;
                        for app in apps.iter().take(10) {
                            let app_name = app.name.clone();
                            let app_path = app.path.clone();
                            let file_path = selected_entry.as_ref().map(|e| e.path.clone());
                            let entity = entity.clone();

                            submenu = submenu.child(
                                div()
                                    .id(SharedString::from(format!("grid-app-{}", app_name)))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(text_color)
                                    .hover(|s| s.bg(hover_bg))
                                    .on_mouse_down(MouseButton::Left, {
                                        let app_name = app_name.clone();
                                        let app_path = app_path.clone();
                                        move |_event, _window, cx| {
                                            if let Some(ref fp) = file_path {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action =
                                                        Some(ContextMenuAction::OpenWithApp {
                                                            file_path: fp.clone(),
                                                            app_path: app_path.clone(),
                                                            app_name: app_name.clone(),
                                                        });
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                    .child(
                                        svg()
                                            .path("assets/icons/app-window.svg")
                                            .size(px(14.0))
                                            .text_color(text_color),
                                    )
                                    .child(app_name),
                            );
                        }
                        submenu
                    })
                    .child({
                        let entity = entity.clone();
                        div()
                            .id("grid-open-with-other")
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .text_sm()
                            .text_color(text_color)
                            .hover(|s| s.bg(hover_bg))
                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                if let Some(ref e) = entry_for_other {
                                    entity.update(cx, |view, cx| {
                                        view.pending_context_action =
                                            Some(ContextMenuAction::OpenWithOther(e.path.clone()));
                                        view.close_context_menu();
                                        cx.notify();
                                    });
                                }
                            })
                            .child(
                                svg()
                                    .path("assets/icons/more-horizontal.svg")
                                    .size(px(14.0))
                                    .text_color(text_color),
                            )
                            .child("Other...")
                    }),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_entry(name: &str, is_dir: bool) -> FileEntry {
        FileEntry::new(
            name.to_string(),
            PathBuf::from(format!("/test/{}", name)),
            is_dir,
            1024,
            SystemTime::now(),
        )
    }

    #[test]
    fn test_grid_view_new() {
        let grid = GridView::new();
        assert_eq!(grid.item_count(), 0);
        assert!(grid.selected_index().is_none());
    }

    #[test]
    fn test_grid_view_set_entries() {
        let mut grid = GridView::new();
        let entries = vec![
            create_test_entry("file1.txt", false),
            create_test_entry("folder1", true),
        ];

        grid.set_entries(entries);
        assert_eq!(grid.item_count(), 2);
    }

    #[test]
    fn test_grid_view_selection() {
        let mut grid = GridView::new();
        let entries = vec![
            create_test_entry("file1.txt", false),
            create_test_entry("file2.txt", false),
        ];

        grid.set_entries(entries);
        assert!(grid.selected_index().is_none());

        grid.set_selected_index(Some(1));
        assert_eq!(grid.selected_index(), Some(1));

        grid.set_selected_index(None);
        assert!(grid.selected_index().is_none());
    }

    #[test]
    fn test_grid_view_columns_and_rows() {
        let mut grid = GridView::new();
        let entries: Vec<FileEntry> = (0..10)
            .map(|i| create_test_entry(&format!("file{}.txt", i), false))
            .collect();

        grid.set_entries(entries);
        grid.set_viewport_width(400.0);

        let columns = grid.columns();
        let rows = grid.rows();

        assert!(columns >= grid.config().min_columns);
        assert!(rows * columns >= 10);
    }

    #[test]
    fn test_truncate_name() {
        assert_eq!(truncate_name("short.txt", 14), "short.txt");
        assert_eq!(
            truncate_name("very_long_filename.txt", 14),
            "very_long_f..."
        );
        assert_eq!(truncate_name("a", 14), "a");
    }
}
