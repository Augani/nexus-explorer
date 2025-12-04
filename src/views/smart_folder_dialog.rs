use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

use crate::models::{
    theme_colors, DateFilter, SearchQuery, SizeFilter, SmartFolder, SmartFolderId,
};

/// Actions that can be triggered from the smart folder dialog
#[derive(Clone, Debug, PartialEq)]
pub enum SmartFolderDialogAction {
    Create {
        name: String,
        query: SearchQuery,
    },
    Update {
        id: SmartFolderId,
        query: SearchQuery,
    },
    Cancel,
}

/// State for the query builder form
#[derive(Clone, Default)]
pub struct QueryBuilderState {
    pub name: String,
    pub text_pattern: String,
    pub file_types: String,
    pub date_filter_type: DateFilterType,
    pub date_filter_value: u32,
    pub size_filter_type: SizeFilterType,
    pub size_filter_value: u64,
    pub include_hidden: bool,
    pub directories_only: bool,
    pub files_only: bool,
    pub recursive: bool,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum DateFilterType {
    #[default]
    None,
    LastDays,
    LastWeeks,
    LastMonths,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum SizeFilterType {
    #[default]
    None,
    SmallerThan,
    LargerThan,
}

impl QueryBuilderState {
    pub fn new() -> Self {
        Self {
            recursive: true,
            ..Default::default()
        }
    }

    pub fn from_smart_folder(folder: &SmartFolder) -> Self {
        let query = &folder.query;

        let (date_filter_type, date_filter_value) = match &query.date_filter {
            Some(DateFilter::LastDays(d)) => (DateFilterType::LastDays, *d),
            Some(DateFilter::LastWeeks(w)) => (DateFilterType::LastWeeks, *w),
            Some(DateFilter::LastMonths(m)) => (DateFilterType::LastMonths, *m),
            _ => (DateFilterType::None, 7),
        };

        let (size_filter_type, size_filter_value) = match &query.size_filter {
            Some(SizeFilter::SmallerThan(s)) => (SizeFilterType::SmallerThan, *s),
            Some(SizeFilter::LargerThan(s)) => (SizeFilterType::LargerThan, *s),
            _ => (SizeFilterType::None, 1024 * 1024),
        };

        Self {
            name: folder.name.clone(),
            text_pattern: query.text.clone().unwrap_or_default(),
            file_types: query.file_types.join(", "),
            date_filter_type,
            date_filter_value,
            size_filter_type,
            size_filter_value,
            include_hidden: query.include_hidden,
            directories_only: query.directories_only,
            files_only: query.files_only,
            recursive: query.recursive,
        }
    }

    pub fn to_search_query(&self) -> SearchQuery {
        let mut query = SearchQuery::new();

        if !self.text_pattern.is_empty() {
            query = query.text(self.text_pattern.clone());
        }

        if !self.file_types.is_empty() {
            let types: Vec<String> = self
                .file_types
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !types.is_empty() {
                query = query.file_types(types);
            }
        }

        match self.date_filter_type {
            DateFilterType::LastDays => {
                query = query.date_filter(DateFilter::LastDays(self.date_filter_value));
            }
            DateFilterType::LastWeeks => {
                query = query.date_filter(DateFilter::LastWeeks(self.date_filter_value));
            }
            DateFilterType::LastMonths => {
                query = query.date_filter(DateFilter::LastMonths(self.date_filter_value));
            }
            DateFilterType::None => {}
        }

        match self.size_filter_type {
            SizeFilterType::SmallerThan => {
                query = query.size_filter(SizeFilter::SmallerThan(self.size_filter_value));
            }
            SizeFilterType::LargerThan => {
                query = query.size_filter(SizeFilter::LargerThan(self.size_filter_value));
            }
            SizeFilterType::None => {}
        }

        query = query.include_hidden(self.include_hidden);
        query = query.recursive(self.recursive);
        query.directories_only = self.directories_only;
        query.files_only = self.files_only;

        query
    }
}

pub struct SmartFolderDialog {
    focus_handle: FocusHandle,
    state: QueryBuilderState,
    editing_id: Option<SmartFolderId>,
    pending_action: Option<SmartFolderDialogAction>,
}

impl SmartFolderDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: QueryBuilderState::new(),
            editing_id: None,
            pending_action: None,
        }
    }

    pub fn new_for_edit(folder: &SmartFolder, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: QueryBuilderState::from_smart_folder(folder),
            editing_id: Some(folder.id),
            pending_action: None,
        }
    }

    pub fn reset(&mut self) {
        self.state = QueryBuilderState::new();
        self.editing_id = None;
        self.pending_action = None;
    }

    pub fn set_editing(&mut self, folder: &SmartFolder) {
        self.state = QueryBuilderState::from_smart_folder(folder);
        self.editing_id = Some(folder.id);
        self.pending_action = None;
    }

    pub fn take_pending_action(&mut self) -> Option<SmartFolderDialogAction> {
        self.pending_action.take()
    }

    fn handle_save(&mut self, cx: &mut Context<Self>) {
        if self.state.name.is_empty() {
            return;
        }

        let query = self.state.to_search_query();

        if let Some(id) = self.editing_id {
            self.pending_action = Some(SmartFolderDialogAction::Update { id, query });
        } else {
            self.pending_action = Some(SmartFolderDialogAction::Create {
                name: self.state.name.clone(),
                query,
            });
        }
        cx.notify();
    }

    fn handle_cancel(&mut self, cx: &mut Context<Self>) {
        self.pending_action = Some(SmartFolderDialogAction::Cancel);
        cx.notify();
    }

    fn set_name(&mut self, name: String, cx: &mut Context<Self>) {
        self.state.name = name;
        cx.notify();
    }

    fn set_text_pattern(&mut self, pattern: String, cx: &mut Context<Self>) {
        self.state.text_pattern = pattern;
        cx.notify();
    }

    fn set_file_types(&mut self, types: String, cx: &mut Context<Self>) {
        self.state.file_types = types;
        cx.notify();
    }

    fn toggle_include_hidden(&mut self, cx: &mut Context<Self>) {
        self.state.include_hidden = !self.state.include_hidden;
        cx.notify();
    }

    fn toggle_recursive(&mut self, cx: &mut Context<Self>) {
        self.state.recursive = !self.state.recursive;
        cx.notify();
    }

    fn toggle_directories_only(&mut self, cx: &mut Context<Self>) {
        self.state.directories_only = !self.state.directories_only;
        if self.state.directories_only {
            self.state.files_only = false;
        }
        cx.notify();
    }

    fn toggle_files_only(&mut self, cx: &mut Context<Self>) {
        self.state.files_only = !self.state.files_only;
        if self.state.files_only {
            self.state.directories_only = false;
        }
        cx.notify();
    }

    fn cycle_date_filter(&mut self, cx: &mut Context<Self>) {
        self.state.date_filter_type = match self.state.date_filter_type {
            DateFilterType::None => DateFilterType::LastDays,
            DateFilterType::LastDays => DateFilterType::LastWeeks,
            DateFilterType::LastWeeks => DateFilterType::LastMonths,
            DateFilterType::LastMonths => DateFilterType::None,
        };
        cx.notify();
    }

    fn cycle_size_filter(&mut self, cx: &mut Context<Self>) {
        self.state.size_filter_type = match self.state.size_filter_type {
            SizeFilterType::None => SizeFilterType::SmallerThan,
            SizeFilterType::SmallerThan => SizeFilterType::LargerThan,
            SizeFilterType::LargerThan => SizeFilterType::None,
        };
        cx.notify();
    }
}

impl Focusable for SmartFolderDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SmartFolderDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_overlay = gpui::rgba(0x000000aa);
        let bg_dialog = theme.bg_secondary;
        let text_primary = theme.text_primary;
        let text_secondary = theme.text_secondary;
        let text_muted = theme.text_muted;
        let accent = theme.accent_primary;
        let border_color = theme.border_default;
        let hover_bg = theme.bg_hover;
        let input_bg = theme.bg_tertiary;

        let is_editing = self.editing_id.is_some();
        let title = if is_editing {
            "Edit Smart Folder"
        } else {
            "New Smart Folder"
        };
        let save_label = if is_editing { "Save" } else { "Create" };

        let name = self.state.name.clone();
        let text_pattern = self.state.text_pattern.clone();
        let file_types = self.state.file_types.clone();
        let include_hidden = self.state.include_hidden;
        let recursive = self.state.recursive;
        let directories_only = self.state.directories_only;
        let files_only = self.state.files_only;
        let date_filter_type = self.state.date_filter_type;
        let size_filter_type = self.state.size_filter_type;

        let date_filter_label = match date_filter_type {
            DateFilterType::None => "No date filter",
            DateFilterType::LastDays => "Modified in last N days",
            DateFilterType::LastWeeks => "Modified in last N weeks",
            DateFilterType::LastMonths => "Modified in last N months",
        };

        let size_filter_label = match size_filter_type {
            SizeFilterType::None => "No size filter",
            SizeFilterType::SmallerThan => "Smaller than",
            SizeFilterType::LargerThan => "Larger than",
        };

        div()
            .id("smart-folder-dialog-overlay")
            .absolute()
            .inset_0()
            .bg(bg_overlay)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _event, _window, cx| {
                    view.handle_cancel(cx);
                }),
            )
            .child(
                div()
                    .id("smart-folder-dialog")
                    .w(px(480.0))
                    .max_h(px(600.0))
                    .bg(bg_dialog)
                    .rounded_lg()
                    .border_1()
                    .border_color(border_color)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                        // Prevent click from propagating to overlay
                    })
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(border_color)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg()
                                            .path("assets/icons/sparkles.svg")
                                            .size(px(18.0))
                                            .text_color(accent),
                                    )
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(text_primary)
                                            .child(title),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-btn")
                                    .p_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|h| h.bg(hover_bg))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _event, _window, cx| {
                                            view.handle_cancel(cx);
                                        }),
                                    )
                                    .child(
                                        svg()
                                            .path("assets/icons/x.svg")
                                            .size(px(16.0))
                                            .text_color(text_secondary),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_primary)
                                            .child("Name"),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_2()
                                            .bg(input_bg)
                                            .rounded_md()
                                            .border_1()
                                            .border_color(border_color)
                                            .text_sm()
                                            .text_color(if name.is_empty() {
                                                text_muted
                                            } else {
                                                text_primary
                                            })
                                            .child(if name.is_empty() {
                                                "Enter smart folder name...".to_string()
                                            } else {
                                                name
                                            }),
                                    ),
                            )
                            // Text pattern field
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_primary)
                                            .child("Name contains"),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_2()
                                            .bg(input_bg)
                                            .rounded_md()
                                            .border_1()
                                            .border_color(border_color)
                                            .text_sm()
                                            .text_color(if text_pattern.is_empty() {
                                                text_muted
                                            } else {
                                                text_primary
                                            })
                                            .child(if text_pattern.is_empty() {
                                                "e.g., report, test, config".to_string()
                                            } else {
                                                text_pattern
                                            }),
                                    ),
                            )
                            // File types field
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_primary)
                                            .child("File types (comma separated)"),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_2()
                                            .bg(input_bg)
                                            .rounded_md()
                                            .border_1()
                                            .border_color(border_color)
                                            .text_sm()
                                            .text_color(if file_types.is_empty() {
                                                text_muted
                                            } else {
                                                text_primary
                                            })
                                            .child(if file_types.is_empty() {
                                                "e.g., rs, toml, md".to_string()
                                            } else {
                                                file_types
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_primary)
                                            .child("Date filter"),
                                    )
                                    .child(
                                        div()
                                            .id("date-filter-btn")
                                            .px_3()
                                            .py_2()
                                            .bg(input_bg)
                                            .rounded_md()
                                            .border_1()
                                            .border_color(border_color)
                                            .cursor_pointer()
                                            .hover(|h| h.bg(hover_bg))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|view, _event, _window, cx| {
                                                    view.cycle_date_filter(cx);
                                                }),
                                            )
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(text_primary)
                                                    .child(date_filter_label),
                                            )
                                            .child(
                                                svg()
                                                    .path("assets/icons/chevron-down.svg")
                                                    .size(px(14.0))
                                                    .text_color(text_secondary),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_primary)
                                            .child("Size filter"),
                                    )
                                    .child(
                                        div()
                                            .id("size-filter-btn")
                                            .px_3()
                                            .py_2()
                                            .bg(input_bg)
                                            .rounded_md()
                                            .border_1()
                                            .border_color(border_color)
                                            .cursor_pointer()
                                            .hover(|h| h.bg(hover_bg))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|view, _event, _window, cx| {
                                                    view.cycle_size_filter(cx);
                                                }),
                                            )
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(text_primary)
                                                    .child(size_filter_label),
                                            )
                                            .child(
                                                svg()
                                                    .path("assets/icons/chevron-down.svg")
                                                    .size(px(14.0))
                                                    .text_color(text_secondary),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(self.render_checkbox(
                                        "include-hidden",
                                        "Include hidden files",
                                        include_hidden,
                                        cx,
                                    ))
                                    .child(self.render_checkbox(
                                        "recursive",
                                        "Search recursively",
                                        recursive,
                                        cx,
                                    ))
                                    .child(self.render_checkbox(
                                        "dirs-only",
                                        "Directories only",
                                        directories_only,
                                        cx,
                                    ))
                                    .child(self.render_checkbox(
                                        "files-only",
                                        "Files only",
                                        files_only,
                                        cx,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(border_color)
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .id("cancel-btn")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(text_secondary)
                                    .hover(|h| h.bg(hover_bg))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _event, _window, cx| {
                                            view.handle_cancel(cx);
                                        }),
                                    )
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("save-btn")
                                    .px_4()
                                    .py_2()
                                    .bg(accent)
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(theme.text_inverse)
                                    .hover(|h| h.opacity(0.9))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _event, _window, cx| {
                                            view.handle_save(cx);
                                        }),
                                    )
                                    .child(save_label),
                            ),
                    ),
            )
    }
}

impl SmartFolderDialog {
    fn render_checkbox(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme_colors();
        let text_primary = theme.text_primary;
        let accent = theme.accent_primary;
        let border_color = theme.border_default;
        let hover_bg = theme.bg_hover;

        div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .hover(|h| h.bg(hover_bg))
            .px_2()
            .py_1()
            .rounded_md()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _event, _window, cx| match id {
                    "include-hidden" => view.toggle_include_hidden(cx),
                    "recursive" => view.toggle_recursive(cx),
                    "dirs-only" => view.toggle_directories_only(cx),
                    "files-only" => view.toggle_files_only(cx),
                    _ => {}
                }),
            )
            .child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if checked { accent } else { border_color })
                    .when(checked, |s| s.bg(accent))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |s| {
                        s.child(
                            svg()
                                .path("assets/icons/check.svg")
                                .size(px(12.0))
                                .text_color(theme.text_inverse),
                        )
                    }),
            )
            .child(div().text_sm().text_color(text_primary).child(label))
    }
}
