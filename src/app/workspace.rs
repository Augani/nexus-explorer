use std::path::PathBuf;
use std::time::Instant;

use gpui::{
    div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

use crate::io::{SortKey, SortOrder};
use crate::models::{FileSystem, IconCache, SearchEngine};
use crate::views::{FileList, FileListView, Preview, PreviewView, SearchInputView, Sidebar, SidebarView};

pub struct Workspace {
    file_system: Entity<FileSystem>,
    icon_cache: Entity<IconCache>,
    search_engine: Entity<SearchEngine>,
    file_list: Entity<FileListView>,
    sidebar: Entity<SidebarView>,
    search_input: Entity<SearchInputView>,
    preview: Option<Entity<PreviewView>>,
    focus_handle: FocusHandle,
    current_path: PathBuf,
    path_history: Vec<PathBuf>,
    is_terminal_open: bool,
    cached_entries: Vec<crate::models::FileEntry>,
}

impl Workspace {
    pub fn build(initial_path: PathBuf, cx: &mut App) -> Entity<Self> {
        // Register search input key bindings
        SearchInputView::register_key_bindings(cx);
        
        cx.new(|cx| {
            let mut file_system = FileSystem::new(initial_path.clone());

            let start = Instant::now();
            let op = file_system.load_path(
                initial_path.clone(),
                SortKey::Name,
                SortOrder::Ascending,
                false,
            );
            let request_id = op.request_id;

            while let Ok(batch) = op.batch_receiver.recv() {
                file_system.process_batch(request_id, batch);
            }

            let _ = op.traversal_handle.join();
            file_system.finalize_load(request_id, start.elapsed());

            let cached_entries = file_system.entries().to_vec();
            let mut file_list_inner = FileList::new();
            file_list_inner.set_entries(cached_entries.clone());
            file_list_inner.set_viewport_height(600.0);

            let file_system = cx.new(|_| file_system);
            let icon_cache = cx.new(|_| IconCache::new());
            
            // Create search engine and inject initial entries
            let search_engine_inner = SearchEngine::new();
            for entry in &cached_entries {
                search_engine_inner.inject(entry.path.clone());
            }
            let search_engine = cx.new(|_| search_engine_inner);

            let file_list = cx.new(|cx| FileListView::with_file_list(file_list_inner, cx));
            let sidebar = cx.new(|cx| {
                let mut sidebar_view = SidebarView::new(cx);
                sidebar_view.set_workspace_root(initial_path.clone());
                sidebar_view
            });
            
            let search_input = cx.new(|cx| {
                SearchInputView::new(cx).with_search_engine(search_engine.clone())
            });

            // Observe file list for navigation requests
            cx.observe(&file_list, |workspace: &mut Workspace, file_list, cx| {
                let nav_path = file_list.update(cx, |view, _| view.take_pending_navigation());
                if let Some(path) = nav_path {
                    workspace.navigate_to(path, cx);
                }
            })
            .detach();
            
            // Observe search input for query changes
            cx.observe(&search_input, |workspace: &mut Workspace, search_input, cx| {
                let query = search_input.read(cx).query().to_string();
                workspace.handle_search_query_change(&query, cx);
            })
            .detach();

            Self {
                file_system,
                icon_cache,
                search_engine,
                file_list,
                sidebar,
                search_input,
                preview: None,
                focus_handle: cx.focus_handle(),
                current_path: initial_path.clone(),
                path_history: vec![initial_path],
                is_terminal_open: true,
                cached_entries,
            }
        })
    }
    
    fn handle_search_query_change(&mut self, query: &str, cx: &mut Context<Self>) {
        if query.is_empty() {
            // Clear search - restore full file list
            self.file_list.update(cx, |view, _| {
                view.inner_mut().clear_search_filter();
            });
        } else {
            // Apply search filter
            let matches = self.search_engine.update(cx, |engine, _| {
                engine.set_pattern(query);
                let snapshot = engine.snapshot();
                snapshot.matches
            });
            
            // Convert matches to the format FileList expects
            let file_matches: Vec<(usize, Vec<usize>, u32)> = matches
                .iter()
                .filter_map(|m| {
                    // Find the index in cached_entries that matches this path
                    self.cached_entries
                        .iter()
                        .position(|e| e.path == m.path)
                        .map(|idx| (idx, m.positions.clone(), m.score))
                })
                .collect();
            
            self.file_list.update(cx, |view, _| {
                view.inner_mut().apply_search_filter(query, file_matches);
            });
        }
        cx.notify();
    }

    pub fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let start = Instant::now();

        self.file_system.update(cx, |fs, _| {
            let op = fs.load_path(path.clone(), SortKey::Name, SortOrder::Ascending, false);
            let request_id = op.request_id;

            while let Ok(batch) = op.batch_receiver.recv() {
                fs.process_batch(request_id, batch);
            }

            let _ = op.traversal_handle.join();
            fs.finalize_load(request_id, start.elapsed());
        });

        let entries = self.file_system.read(cx).entries().to_vec();
        self.cached_entries = entries.clone();
        
        // Clear search and update file list
        self.search_input.update(cx, |view, cx| {
            view.clear(cx);
        });
        
        self.file_list.update(cx, |view, _| {
            view.inner_mut().set_entries(entries.clone());
        });
        
        // Re-inject paths into search engine for new directory
        self.search_engine.update(cx, |engine, _| {
            engine.clear();
            for entry in &entries {
                engine.inject(entry.path.clone());
            }
        });

        self.path_history.push(path.clone());
        self.current_path = path;
        cx.notify();
    }

    pub fn navigate_back(&mut self, cx: &mut Context<Self>) {
        if self.path_history.len() > 1 {
            self.path_history.pop();
            if let Some(prev_path) = self.path_history.last().cloned() {
                let start = Instant::now();

                self.file_system.update(cx, |fs, _| {
                    let op =
                        fs.load_path(prev_path.clone(), SortKey::Name, SortOrder::Ascending, false);
                    let request_id = op.request_id;

                    while let Ok(batch) = op.batch_receiver.recv() {
                        fs.process_batch(request_id, batch);
                    }

                    let _ = op.traversal_handle.join();
                    fs.finalize_load(request_id, start.elapsed());
                });

                let entries = self.file_system.read(cx).entries().to_vec();
                self.cached_entries = entries.clone();
                
                // Clear search and update file list
                self.search_input.update(cx, |view, cx| {
                    view.clear(cx);
                });
                
                self.file_list.update(cx, |view, _| {
                    view.inner_mut().set_entries(entries.clone());
                });
                
                // Re-inject paths into search engine
                self.search_engine.update(cx, |engine, _| {
                    engine.clear();
                    for entry in &entries {
                        engine.inject(entry.path.clone());
                    }
                });

                self.current_path = prev_path;
                cx.notify();
            }
        }
    }

    pub fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf(), cx);
        }
    }

    pub fn toggle_terminal(&mut self, cx: &mut Context<Self>) {
        self.is_terminal_open = !self.is_terminal_open;
        cx.notify();
    }

    fn render_breadcrumbs(&self) -> impl IntoElement {
        let text_gray = gpui::rgb(0x8b949e);
        let text_light = gpui::rgb(0xc9d1d9);

        let mut parts: Vec<String> = Vec::new();
        let mut current = Some(self.current_path.as_path());

        while let Some(path) = current {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                parts.push(name.to_string());
            }
            current = path.parent();
            if parts.len() >= 4 {
                break;
            }
        }

        parts.reverse();

        div()
            .flex()
            .items_center()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .children(parts.into_iter().enumerate().map(|(i, part)| {
                div()
                    .flex()
                    .items_center()
                    .when(i > 0, |s| {
                        s.child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(14.0))
                                .text_color(text_gray)
                                .mx_1(),
                        )
                    })
                    .child(
                        div()
                            .text_color(text_light)
                            .cursor_pointer()
                            .hover(|s| s.text_color(gpui::rgb(0x58a6ff)))
                            .child(part)
                    )
            }))
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_dark = gpui::rgb(0x0d1117);
        let bg_darker = gpui::rgb(0x010409);
        let border_color = gpui::rgb(0x30363d);
        let text_gray = gpui::rgb(0x8b949e);
        let hover_bg = gpui::rgb(0x21262d);
        let blue_active = gpui::rgb(0x1f6feb);

        let is_terminal_open = self.is_terminal_open;
        let can_go_back = self.path_history.len() > 1;

        div()
            .id("workspace")
            .size_full()
            .flex()
            .flex_col()
            .bg(bg_dark)
            .text_color(gpui::rgb(0xc9d1d9))
            .font_family(".SystemUIFont")
            .child(
                div()
                    .h(px(40.0))
                    .bg(bg_darker)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div().flex().gap_1p5().mr_4().children(vec![
                                    div()
                                        .w(px(12.0))
                                        .h(px(12.0))
                                        .rounded_full()
                                        .bg(gpui::rgb(0xff5f56))
                                        .border_1()
                                        .border_color(gpui::rgb(0xe0443e)),
                                    div()
                                        .w(px(12.0))
                                        .h(px(12.0))
                                        .rounded_full()
                                        .bg(gpui::rgb(0xffbd2e))
                                        .border_1()
                                        .border_color(gpui::rgb(0xdea123)),
                                    div()
                                        .w(px(12.0))
                                        .h(px(12.0))
                                        .rounded_full()
                                        .bg(gpui::rgb(0x27c93f))
                                        .border_1()
                                        .border_color(gpui::rgb(0x1aab29)),
                                ]),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_gray)
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg()
                                            .path("assets/icons/hard-drive.svg")
                                            .size(px(12.0))
                                            .text_color(text_gray),
                                    )
                                    .child("Nexus Explorer"),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .w_1_3()
                            .max_w(px(450.0))
                            .child(self.search_input.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                svg()
                                    .path("assets/icons/sparkles.svg")
                                    .size(px(14.0))
                                    .text_color(text_gray)
                                    .cursor_pointer(),
                            )
                            .child(
                                svg()
                                    .path("assets/icons/monitor.svg")
                                    .size(px(14.0))
                                    .text_color(text_gray)
                                    .cursor_pointer(),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .w(px(256.0))
                            .bg(bg_dark)
                            .border_r_1()
                            .border_color(border_color)
                            .flex()
                            .flex_col()
                            .child(self.sidebar.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .bg(bg_darker)
                            .min_w_0()
                            .child(
                                div()
                                    .h(px(48.0))
                                    .bg(bg_dark)
                                    .border_b_1()
                                    .border_color(border_color)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_4()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .id("back-btn")
                                                    .p_1p5()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .when(can_go_back, |s| {
                                                        s.hover(|h| h.bg(hover_bg))
                                                    })
                                                    .when(!can_go_back, |s| s.opacity(0.3))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|view, _event, _window, cx| {
                                                            view.navigate_back(cx);
                                                        }),
                                                    )
                                                    .child(
                                                        svg()
                                                            .path("assets/icons/arrow-left.svg")
                                                            .size(px(16.0))
                                                            .text_color(text_gray),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .h(px(16.0))
                                                    .w(px(1.0))
                                                    .bg(gpui::rgb(0x30363d))
                                                    .mx_1(),
                                            )
                                            .child(self.render_breadcrumbs()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .id("terminal-btn")
                                                    .p_1p5()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .when(is_terminal_open, |s| {
                                                        s.bg(gpui::rgb(0x1f3a5f))
                                                    })
                                                    .when(!is_terminal_open, |s| {
                                                        s.hover(|h| h.bg(hover_bg))
                                                    })
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|view, _event, _window, cx| {
                                                            view.toggle_terminal(cx);
                                                        }),
                                                    )
                                                    .child(
                                                        svg()
                                                            .path("assets/icons/terminal.svg")
                                                            .size(px(16.0))
                                                            .text_color(if is_terminal_open { gpui::rgb(0x54aeff) } else { text_gray }),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .h(px(16.0))
                                                    .w(px(1.0))
                                                    .bg(gpui::rgb(0x30363d))
                                                    .mx_2(),
                                            )
                                            .child(
                                                div()
                                                    .id("copy-btn")
                                                    .p_1p5()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .hover(|h| h.bg(hover_bg))
                                                    .child(
                                                        svg()
                                                            .path("assets/icons/copy.svg")
                                                            .size(px(16.0))
                                                            .text_color(text_gray),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id("trash-btn")
                                                    .p_1p5()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .hover(|h| h.bg(hover_bg))
                                                    .child(
                                                        svg()
                                                            .path("assets/icons/trash-2.svg")
                                                            .size(px(16.0))
                                                            .text_color(text_gray),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .h(px(16.0))
                                                    .w(px(1.0))
                                                    .bg(gpui::rgb(0x30363d))
                                                    .mx_2(),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .bg(gpui::rgb(0x21262d))
                                                    .rounded_lg()
                                                    .p_0p5()
                                                    .child(
                                                        div()
                                                            .p_1()
                                                            .rounded_md()
                                                            .cursor_pointer()
                                                            .child(
                                                                svg()
                                                                    .path("assets/icons/grid-2x2.svg")
                                                                    .size(px(14.0))
                                                                    .text_color(text_gray),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .p_1()
                                                            .rounded_md()
                                                            .bg(gpui::rgb(0x30363d))
                                                            .cursor_pointer()
                                                            .child(
                                                                svg()
                                                                    .path("assets/icons/list.svg")
                                                                    .size(px(14.0))
                                                                    .text_color(gpui::white()),
                                                            ),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .bg(bg_darker)
                                    .overflow_hidden()
                                    .child(self.file_list.clone()),
                            )
                            .when(is_terminal_open, |this| {
                                this.child(
                                    div()
                                        .h(px(192.0))
                                        .bg(bg_dark)
                                        .border_t_1()
                                        .border_color(border_color)
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .h(px(32.0))
                                                .flex()
                                                .items_center()
                                                .px_4()
                                                .border_b_1()
                                                .border_color(border_color)
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_4()
                                                        .text_xs()
                                                        .font_family("Mono")
                                                        .child(
                                                            div()
                                                                .text_color(blue_active)
                                                                .border_b_2()
                                                                .border_color(blue_active)
                                                                .py_2()
                                                                .cursor_pointer()
                                                                .child("Terminal"),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_color(text_gray)
                                                                .py_2()
                                                                .cursor_pointer()
                                                                .hover(|s| s.text_color(gpui::rgb(0xc9d1d9)))
                                                                .child("Output"),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .id("close-terminal")
                                                        .text_color(text_gray)
                                                        .cursor_pointer()
                                                        .hover(|s| s.text_color(gpui::white()))
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(|view, _event, _window, cx| {
                                                                view.toggle_terminal(cx);
                                                            }),
                                                        )
                                                        .child("▼"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .p_3()
                                                .font_family("Mono")
                                                .text_xs()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .child(
                                                            div()
                                                                .text_color(gpui::rgb(0x3fb950))
                                                                .mr_2()
                                                                .child("➜"),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_color(blue_active)
                                                                .mr_2()
                                                                .child("~"),
                                                        )
                                                        .child(
                                                            div()
                                                                .w(px(8.0))
                                                                .h(px(16.0))
                                                                .bg(text_gray),
                                                        ),
                                                ),
                                        ),
                                )
                            }),
                    )
                    .children(self.preview.clone().map(|preview| {
                        div()
                            .w(px(320.0))
                            .bg(bg_dark)
                            .border_l_1()
                            .border_color(border_color)
                            .flex()
                            .flex_col()
                            .child(preview)
                    })),
            )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Grid,
    List,
}
