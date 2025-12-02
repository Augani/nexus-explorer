use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

#[derive(Clone)]
pub struct SidebarItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
    pub children: Vec<SidebarItem>,
}

impl SidebarItem {
    pub fn new(name: String, path: PathBuf, is_dir: bool, depth: usize) -> Self {
        Self {
            name,
            path,
            is_dir,
            depth,
            is_expanded: false,
            children: Vec::new(),
        }
    }
}

pub struct Sidebar {
    favorites: Vec<(&'static str, PathBuf)>,
    workspace_root: Option<SidebarItem>,
    selected_path: Option<PathBuf>,
}

impl Sidebar {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let desktop = home.join("Desktop");
        let documents = home.join("Documents");
        let downloads = home.join("Downloads");

        Self {
            favorites: vec![
                ("Home", home),
                ("Desktop", desktop),
                ("Documents", documents),
                ("Downloads", downloads),
            ],
            workspace_root: None,
            selected_path: None,
        }
    }

    pub fn set_workspace_root(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Root")
            .to_string();
        self.workspace_root = Some(SidebarItem::new(name, path, true, 0));
    }

    pub fn set_selected_path(&mut self, path: PathBuf) {
        self.selected_path = Some(path);
    }
}

pub struct SidebarView {
    sidebar: Sidebar,
    focus_handle: FocusHandle,
    on_navigate: Option<Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>>,
}

impl SidebarView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            sidebar: Sidebar::new(),
            focus_handle: cx.focus_handle(),
            on_navigate: None,
        }
    }

    pub fn set_on_navigate<F>(&mut self, callback: F)
    where
        F: Fn(PathBuf, &mut Window, &mut App) + 'static,
    {
        self.on_navigate = Some(Box::new(callback));
    }

    pub fn set_workspace_root(&mut self, path: PathBuf) {
        self.sidebar.set_workspace_root(path);
    }

    fn handle_favorite_click(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.selected_path = Some(path.clone());
        // Navigation callback would be handled by parent Workspace
        cx.notify();
    }
}

impl Focusable for SidebarView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_dark = gpui::rgb(0x0d1117);
        let text_gray = gpui::rgb(0x8b949e);
        let text_light = gpui::rgb(0xe6edf3);
        let hover_bg = gpui::rgb(0x21262d);
        let selected_bg = gpui::rgb(0x1f3a5f);
        let label_color = gpui::rgb(0x6e7681);
        let icon_blue = gpui::rgb(0x54aeff);

        let selected_path = self.sidebar.selected_path.clone();
        let favorites = self.sidebar.favorites.clone();
        
        let favorite_icons = ["house", "monitor", "file-text", "cloud"];

        div()
            .id("sidebar-content")
            .size_full()
            .bg(bg_dark)
            .flex()
            .flex_col()
            .child(
                div()
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(label_color)
                            .mb_2()
                            .px_2()
                            .child("FAVORITES"),
                    )
                    .child(
                        div().flex().flex_col().gap_0p5().mb_6().children(
                            favorites.into_iter().enumerate().map(|(i, (label, path))| {
                                let is_selected = selected_path.as_ref() == Some(&path);
                                let path_clone = path.clone();
                                let icon_name = favorite_icons.get(i).unwrap_or(&"folder");

                                div()
                                    .id(SharedString::from(format!("fav-{}", i)))
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |s| {
                                        s.bg(selected_bg).text_color(text_light)
                                    })
                                    .when(!is_selected, |s| {
                                        s.text_color(text_gray)
                                            .hover(|h| h.bg(hover_bg).text_color(text_light))
                                    })
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                        view.handle_favorite_click(path_clone.clone(), window, cx);
                                    }))
                                    .child(
                                        svg()
                                            .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                            .size(px(14.0))
                                            .text_color(if is_selected { text_light } else { icon_blue }),
                                    )
                                    .child(label)
                            }),
                        ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(label_color)
                            .mb_2()
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("WORKSPACE")
                            .child(
                                svg()
                                    .path("assets/icons/chevron-down.svg")
                                    .size(px(12.0))
                                    .text_color(label_color),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .pb_4()
                    .child(self.render_workspace_tree(cx)),
            )
    }
}

impl SidebarView {
    fn render_workspace_tree(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let text_gray = gpui::rgb(0x8b949e);
        let hover_bg = gpui::rgb(0x21262d);
        let text_light = gpui::rgb(0xe6edf3);
        let folder_color = gpui::rgb(0x54aeff);

        if let Some(ref root) = self.sidebar.workspace_root {
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id("workspace-root")
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1p5()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(text_gray)
                        .hover(|s| s.bg(hover_bg).text_color(text_light))
                        .child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(14.0))
                                .text_color(gpui::rgb(0x6e7681)),
                        )
                        .child(
                            svg()
                                .path("assets/icons/folder.svg")
                                .size(px(14.0))
                                .text_color(folder_color),
                        )
                        .child(root.name.clone()),
                )
        } else {
            div()
                .px_3()
                .py_2()
                .text_sm()
                .text_color(text_gray)
                .flex()
                .items_center()
                .gap_2()
                .child(
                    svg()
                        .path("assets/icons/folder-x.svg")
                        .size(px(14.0))
                        .text_color(text_gray),
                )
                .child("No workspace open")
        }
    }
}
