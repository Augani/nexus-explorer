use gpui::{
    div, prelude::*, px, App, Context, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
};

pub struct Preview;

impl Preview {
    pub fn new() -> Self {
        Self
    }
}

pub struct PreviewView {
    preview: Preview,
    focus_handle: FocusHandle,
}

impl PreviewView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            preview: Preview::new(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for PreviewView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bg_dark = gpui::rgb(0x0d1117);
        let bg_header = gpui::rgb(0x161b22);
        let border_color = gpui::rgb(0x30363d);
        let text_gray = gpui::rgb(0x8b949e);
        let text_light = gpui::rgb(0xc9d1d9);

        div()
            .id("preview-content")
            .size_full()
            .bg(bg_dark)
            .flex()
            .flex_col()
            // Inspector Toolbar
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
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_gray)
                            .child("PREVIEW")
                    )
                    .child(
                        div().flex().gap_2().children(vec![
                            div().w(px(24.0)).h(px(24.0)).rounded_md().bg(gpui::rgb(0x21262d)).flex().items_center().justify_center().child("M"), // Maximize
                            div().w(px(24.0)).h(px(24.0)).rounded_md().bg(gpui::rgb(0x21262d)).flex().items_center().justify_center().child("X"), // Close
                        ])
                    )
            )
            // File Metadata Header
            .child(
                div()
                    .bg(bg_header)
                    .border_b_1()
                    .border_color(border_color)
                    .p_4()
                    .child(
                        div().flex().items_start().gap_3().mb_3().child(
                            div().p_2().bg(gpui::rgb(0x21262d)).rounded_lg().child(
                                div().w(px(32.0)).h(px(32.0)).bg(gpui::rgb(0x6e7681)) // Icon placeholder
                            )
                        ).child(
                            div().flex_1().min_w_0().child(
                                div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(text_light).truncate().child("package.json")
                            ).child(
                                div().text_xs().text_color(text_gray).mt_0p5().child("12 KB • JSON")
                            )
                        )
                    )
                    .child(
                        div().grid().gap_2().child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .py_1p5()
                                .px_3()
                                .bg(gpui::rgb(0x1f6feb)) // blue-600
                                .rounded_md()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(gpui::white())
                                .child("Explain Code")
                        )
                    )
            )
            // Content Preview
            .child(
                div()
                    .flex_1()
                    // .overflow_y(gpui::Overflow::Scroll)
                    .bg(bg_dark)
                    .p_4()
                    .child(
                        div().text_xs().font_family("Mono").text_color(text_gray).child(
                            r#"{
  "name": "nexus-explorer",
  "version": "0.1.0",
  "private": true,
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "lucide-react": "^0.263.1"
  }
}"#
                        )
                    )
            )
            // Bottom Info Bar
            .child(
                div()
                    .h(px(32.0))
                    .bg(bg_dark)
                    .border_t_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .text_xs()
                    .text_color(text_gray)
                    .child("UTF-8")
                    .child("2023-10-27")
                    .child("JSON")
            )
    }
}
