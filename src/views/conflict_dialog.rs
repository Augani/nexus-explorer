use crate::models::ConflictResolution;
use gpui::{
    div, prelude::*, px, rgb, Context, FontWeight, IntoElement, MouseButton, Render, Window,
};
use std::path::PathBuf;

/
#[derive(Clone, PartialEq)]
pub enum ConflictDialogAction {
    Skip,
    Replace,
    KeepBoth,
    Cancel,
    ApplyToAll(bool),
}

/
#[derive(Clone, Debug)]
pub struct ConflictInfo {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub source_size: u64,
    pub dest_size: u64,
    pub source_modified: Option<std::time::SystemTime>,
    pub dest_modified: Option<std::time::SystemTime>,
}

impl ConflictInfo {
    pub fn new(source: PathBuf, destination: PathBuf) -> Self {
        let source_meta = source.metadata().ok();
        let dest_meta = destination.metadata().ok();
        
        Self {
            source: source.clone(),
            destination: destination.clone(),
            source_size: source_meta.as_ref().map(|m| m.len()).unwrap_or(0),
            dest_size: dest_meta.as_ref().map(|m| m.len()).unwrap_or(0),
            source_modified: source_meta.and_then(|m| m.modified().ok()),
            dest_modified: dest_meta.and_then(|m| m.modified().ok()),
        }
    }

    pub fn source_name(&self) -> &str {
        self.source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
    }

    pub fn dest_folder(&self) -> &str {
        self.destination
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("Unknown")
    }
}

/
pub struct ConflictDialog {
    conflict: ConflictInfo,
    apply_to_all: bool,
    remaining_conflicts: usize,
    on_resolve: Option<Box<dyn Fn(ConflictResolution, bool) + Send + Sync>>,
    on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

impl ConflictDialog {
    pub fn new(conflict: ConflictInfo, remaining_conflicts: usize) -> Self {
        Self {
            conflict,
            apply_to_all: false,
            remaining_conflicts,
            on_resolve: None,
            on_cancel: None,
        }
    }

    pub fn with_on_resolve<F>(mut self, callback: F) -> Self
    where
        F: Fn(ConflictResolution, bool) + Send + Sync + 'static,
    {
        self.on_resolve = Some(Box::new(callback));
        self
    }

    pub fn with_on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    pub fn set_apply_to_all(&mut self, value: bool) {
        self.apply_to_all = value;
    }

    pub fn resolve(&self, resolution: ConflictResolution) {
        if let Some(callback) = &self.on_resolve {
            callback(resolution, self.apply_to_all);
        }
    }

    pub fn cancel(&self) {
        if let Some(callback) = &self.on_cancel {
            callback();
        }
    }

    fn format_size(&self, bytes: u64) -> String {
        crate::utils::format_size(bytes)
    }

    fn format_time(&self, time: Option<std::time::SystemTime>) -> String {
        match time {
            Some(t) => {
                if let Ok(duration) = t.duration_since(std::time::UNIX_EPOCH) {
                    let secs = duration.as_secs() as i64;
                    let datetime = chrono::DateTime::from_timestamp(secs, 0);
                    datetime
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                } else {
                    "Unknown".to_string()
                }
            }
            None => "Unknown".to_string(),
        }
    }
}

impl Render for ConflictDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file_name = self.conflict.source_name().to_string();
        let dest_folder = self.conflict.dest_folder().to_string();
        let source_size = self.format_size(self.conflict.source_size);
        let dest_size = self.format_size(self.conflict.dest_size);
        let source_modified = self.format_time(self.conflict.source_modified);
        let dest_modified = self.format_time(self.conflict.dest_modified);
        let remaining = self.remaining_conflicts;
        let apply_to_all = self.apply_to_all;

        div()
            .flex()
            .flex_col()
            .w(px(450.0))
            .bg(rgb(0x1e1e1e))
            .rounded_lg()
            .border_1()
            .border_color(rgb(0x3c3c3c))
            .shadow_lg()
            .p_4()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .child("File Conflict"),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xcccccc))
                    .child(format!(
                        "\"{}\" already exists in \"{}\"",
                        file_name, dest_folder
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_3()
                            .bg(rgb(0x252526))
                            .rounded_md()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0x888888))
                                    .child("Source"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(format!("Size: {}", source_size)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(format!("Modified: {}", source_modified)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_3()
                            .bg(rgb(0x252526))
                            .rounded_md()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0x888888))
                                    .child("Existing"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(format!("Size: {}", dest_size)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(format!("Modified: {}", dest_modified)),
                            ),
                    ),
            )
            .child(
                div()
                    .id("apply-to-all-checkbox")
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, _cx| {
                        this.apply_to_all = !this.apply_to_all;
                    }))
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x555555))
                            .bg(if apply_to_all {
                                rgb(0x0078d4)
                            } else {
                                rgb(0x2d2d2d)
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(if apply_to_all {
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .child("✓")
                            } else {
                                div()
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xcccccc))
                            .child(format!(
                                "Apply to all {} remaining conflicts",
                                remaining
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        div()
                            .id("skip-button")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x3c3c3c))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x4c4c4c)))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, _cx| {
                                this.resolve(ConflictResolution::Skip);
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xffffff))
                                    .child("Skip"),
                            ),
                    )
                    .child(
                        div()
                            .id("keep-both-button")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x3c3c3c))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x4c4c4c)))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, _cx| {
                                this.resolve(ConflictResolution::KeepBoth);
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xffffff))
                                    .child("Keep Both"),
                            ),
                    )
                    .child(
                        div()
                            .id("replace-button")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x0078d4))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x1084d8)))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, _cx| {
                                this.resolve(ConflictResolution::Replace);
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xffffff))
                                    .child("Replace"),
                            ),
                    )
                    .child(
                        div()
                            .id("cancel-button")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x5a1d1d))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x6a2d2d)))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, _cx| {
                                this.cancel();
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xffffff))
                                    .child("Cancel"),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_info_new() {
        let source = PathBuf::from("/tmp/source/test.txt");
        let dest = PathBuf::from("/tmp/dest/test.txt");
        
        let info = ConflictInfo::new(source.clone(), dest.clone());
        
        assert_eq!(info.source, source);
        assert_eq!(info.destination, dest);
    }

    #[test]
    fn test_conflict_info_source_name() {
        let info = ConflictInfo::new(
            PathBuf::from("/tmp/source/myfile.txt"),
            PathBuf::from("/tmp/dest/myfile.txt"),
        );
        
        assert_eq!(info.source_name(), "myfile.txt");
    }

    #[test]
    fn test_conflict_info_dest_folder() {
        let info = ConflictInfo::new(
            PathBuf::from("/tmp/source/myfile.txt"),
            PathBuf::from("/tmp/dest/subfolder/myfile.txt"),
        );
        
        assert_eq!(info.dest_folder(), "/tmp/dest/subfolder");
    }

    #[test]
    fn test_conflict_dialog_new() {
        let info = ConflictInfo::new(
            PathBuf::from("/tmp/source/test.txt"),
            PathBuf::from("/tmp/dest/test.txt"),
        );
        
        let dialog = ConflictDialog::new(info, 5);
        
        assert!(!dialog.apply_to_all);
        assert_eq!(dialog.remaining_conflicts, 5);
    }

    #[test]
    fn test_conflict_dialog_set_apply_to_all() {
        let info = ConflictInfo::new(
            PathBuf::from("/tmp/source/test.txt"),
            PathBuf::from("/tmp/dest/test.txt"),
        );
        
        let mut dialog = ConflictDialog::new(info, 5);
        
        assert!(!dialog.apply_to_all);
        dialog.set_apply_to_all(true);
        assert!(dialog.apply_to_all);
    }
}
