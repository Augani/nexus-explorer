use gpui::{
    div, prelude::*, px, App, Context, FocusHandle, Focusable, IntoElement, Render, Window,
};

pub struct Sidebar;

impl Sidebar {
    pub fn new() -> Self {
        Self
    }
}

pub struct SidebarView {
    sidebar: Sidebar,
    focus_handle: FocusHandle,
}

impl SidebarView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            sidebar: Sidebar::new(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for SidebarView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bg_dark = gpui::rgb(0x0d1117);
        let text_gray = gpui::rgb(0x8b949e); // gray-400 approx
        let hover_bg = gpui::rgb(0x21262d); // gray-800 approx

        div()
            .id("sidebar-content")
            .size_full()
            .bg(bg_dark)
            .flex()
            .flex_col()
            .child(
                div().p_3().child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(gpui::rgb(0x6e7681)) // gray-500
                        .mb_2()
                        .px_2()
                        .child("FAVORITES")
                ).child(
                    div().flex().flex_col().gap_0p5().mb_6().children(
                        vec![
                            ("Home", "home"),
                            ("Desktop", "monitor"),
                            ("Documents", "file-text"),
                            ("iCloud", "cloud"),
                        ].into_iter().map(|(label, _icon)| {
                            div()
                                .flex()
                                .items_center()
                                .px_2()
                                .py_1p5()
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .text_color(text_gray)
                                .hover(|style| style.bg(hover_bg).text_color(gpui::rgb(0xe6edf3)))
                                .child(
                                    div().mr_3().w(px(14.0)).h(px(14.0)).bg(gpui::rgb(0x6e7681)) // Placeholder icon
                                )
                                .child(label)
                        })
                    )
                ).child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(gpui::rgb(0x6e7681))
                        .mb_2()
                        .px_2()
                        .child("WORKSPACE")
                )
            )
            .child(
                div()
                    .flex_1()
                    // .overflow_y_scroll()
                    .pb_4()
                    .child(
                        // Placeholder for file tree
                        div().px_2().py_1().text_sm().text_color(text_gray).child("File Tree Placeholder")
                    )
            )
    }
}
