use std::path::PathBuf;
use std::time::Instant;

use gpui::{
    div, prelude::*, px, App, AppContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};

use crate::io::{SortKey, SortOrder};
use crate::models::{FileSystem, IconCache, SearchEngine};
use crate::views::{FileList, FileListView, Preview, PreviewView, Sidebar, SidebarView};

/// Root View managing layout panes for the file explorer.
/// 
/// The Workspace is the top-level container that:
/// - Holds Entity handles for FileSystem, IconCache, and SearchEngine
/// - Manages layout panes (Sidebar, FileList, Preview)
/// - Coordinates between views and models
pub struct Workspace {
    file_system: Entity<FileSystem>,
    icon_cache: Entity<IconCache>,
    search_engine: Entity<SearchEngine>,
    file_list: Entity<FileListView>,
    sidebar: Entity<SidebarView>,
    preview: Option<Entity<PreviewView>>,
    focus_handle: FocusHandle,
}



impl Workspace {
    /// Creates a new Workspace with the given initial path.
    pub fn build(initial_path: PathBuf, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut file_system = FileSystem::new(initial_path.clone());
            
            // Load the initial directory synchronously for now
            let start = Instant::now();
            let op = file_system.load_path(
                initial_path,
                SortKey::Name,
                SortOrder::Ascending,
                false,
            );
            let request_id = op.request_id;
            
            // Process all batches
            while let Ok(batch) = op.batch_receiver.recv() {
                file_system.process_batch(request_id, batch);
            }
            
            // Wait for traversal and finalize
            let _ = op.traversal_handle.join();
            file_system.finalize_load(request_id, start.elapsed());
            
            // Create FileList with entries from FileSystem
            let mut file_list_inner = FileList::new();
            file_list_inner.set_entries(file_system.entries().to_vec());
            file_list_inner.set_viewport_height(800.0); // Default viewport
            
            let file_system = cx.new(|_| file_system);
            let icon_cache = cx.new(|_| IconCache::new());
            let search_engine = cx.new(|_| SearchEngine::new());

            let file_list = cx.new(|cx| FileListView::with_file_list(file_list_inner, cx));
            let sidebar = cx.new(|cx| SidebarView::new(cx));

            Self {
                file_system,
                icon_cache,
                search_engine,
                file_list,
                sidebar,
                preview: None,
                focus_handle: cx.focus_handle(),
            }
        })
    }

    /// Returns a reference to the FileSystem entity.
    pub fn file_system(&self) -> &Entity<FileSystem> {
        &self.file_system
    }

    /// Returns a reference to the IconCache entity.
    pub fn icon_cache(&self) -> &Entity<IconCache> {
        &self.icon_cache
    }

    /// Returns a reference to the SearchEngine entity.
    pub fn search_engine(&self) -> &Entity<SearchEngine> {
        &self.search_engine
    }

    /// Returns a reference to the FileList view entity.
    pub fn file_list(&self) -> &Entity<FileListView> {
        &self.file_list
    }

    /// Returns a reference to the Sidebar view entity.
    pub fn sidebar(&self) -> &Entity<SidebarView> {
        &self.sidebar
    }

    /// Returns a reference to the Preview view entity if it exists.
    pub fn preview(&self) -> Option<&Entity<PreviewView>> {
        self.preview.as_ref()
    }

    /// Shows the preview pane.
    pub fn show_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview.is_none() {
            self.preview = Some(cx.new(|cx| PreviewView::new(cx)));
        }
    }

    /// Hides the preview pane.
    pub fn hide_preview(&mut self) {
        self.preview = None;
    }

    /// Toggles the preview pane visibility.
    pub fn toggle_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview.is_some() {
            self.hide_preview();
        } else {
            self.show_preview(cx);
        }
    }

    /// Returns the current view mode (Grid or List).
    pub fn view_mode(&self) -> ViewMode {
        // TODO: Store this in a proper state
        ViewMode::List
    }

    /// Toggles the terminal visibility.
    pub fn toggle_terminal(&mut self, _cx: &mut Context<Self>) {
        // TODO: Implement terminal toggle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Grid,
    List,
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Colors from MockApp.tsx
        let bg_dark = gpui::rgb(0x0d1117);
        let bg_darker = gpui::rgb(0x010409);
        let border_color = gpui::rgb(0x30363d); // gray-800 approx

        div()
            .id("workspace")
            .size_full()
            .flex()
            .flex_col()
            .bg(bg_dark)
            .text_color(gpui::rgb(0xc9d1d9)) // text-gray-300 approx
            .font_family("Inter")
            // Title Bar
            .child(
                div()
                    .h(px(40.0)) // h-10
                    .bg(bg_darker)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div().flex().items_center().gap_3().child(
                            // Traffic Lights
                            div().flex().gap_1p5().mr_4().children(vec![
                                div().w(px(12.0)).h(px(12.0)).rounded_full().bg(gpui::rgb(0xff5f56)).border_1().border_color(gpui::rgb(0xe0443e)),
                                div().w(px(12.0)).h(px(12.0)).rounded_full().bg(gpui::rgb(0xffbd2e)).border_1().border_color(gpui::rgb(0xdea123)),
                                div().w(px(12.0)).h(px(12.0)).rounded_full().bg(gpui::rgb(0x27c93f)).border_1().border_color(gpui::rgb(0x1aab29)),
                            ])
                        ).child(
                            // App Title
                            div().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(gpui::rgb(0x8b949e)).flex().items_center().child("Nexus Explorer")
                        )
                    )
                    // Universal Search
                    .child(
                        div().relative().w_1_3().max_w(px(450.0)).child(
                            div()
                                .w_full()
                                .bg(gpui::rgb(0x161b22))
                                .text_xs()
                                .rounded_md()
                                .border_1()
                                .border_color(gpui::rgb(0x30363d))
                                .py_1p5()
                                .pl_9()
                                .pr_3()
                                .text_color(gpui::rgb(0x8b949e))
                                .child("Search files, commands, and more...")
                        )
                    )
                    // Right Icons
                    .child(
                        div().flex().items_center().gap_3().text_color(gpui::rgb(0x8b949e)).child("Icons") // Placeholders
                    )
            )
            // Main Content Area
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    // COLUMN 1: SIDEBAR
                    .child(
                        div()
                            .w(px(256.0)) // w-64
                            .bg(bg_dark)
                            .border_r_1()
                            .border_color(border_color)
                            .flex()
                            .flex_col()
                            .child(self.sidebar.clone())
                    )
                    // COLUMN 2: BROWSER (Main Content)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .bg(bg_darker)
                            .min_w_0()
                            // Toolbar
                            .child(
                                div()
                                    .h(px(48.0)) // h-12
                                    .bg(bg_dark)
                                    .border_b_1()
                                    .border_color(border_color)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_4()
                                    .child(
                                        div().flex().items_center().gap_2().child("Breadcrumbs")
                                    )
                                    .child(
                                        div().flex().items_center().gap_1().child("Actions")
                                    )
                            )
                            // File Grid/List View
                            .child(
                                div()
                                    .flex_1()
                                    // .overflow_y_scroll()
                                    .bg(bg_darker)
                                    .child(self.file_list.clone())
                            )
                            // Collapsible Terminal Panel (Placeholder)
                            .child(
                                div()
                                    .h(px(192.0)) // h-48
                                    .bg(bg_dark)
                                    .border_t_1()
                                    .border_color(border_color)
                                    .child("Terminal")
                            )
                    )
                    // COLUMN 3: INSPECTOR (Preview Pane)
                    .children(self.preview.clone().map(|preview| {
                        div()
                            .w(px(320.0)) // w-80
                            .bg(bg_dark)
                            .border_l_1()
                            .border_color(border_color)
                            .flex()
                            .flex_col()
                            .child(preview)
                    }))
            )
    }
}


