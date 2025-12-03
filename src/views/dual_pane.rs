use std::path::PathBuf;

use gpui::{
    actions, div, prelude::*, px, App, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, Styled, Window,
};

use crate::models::{DualPane, DragPayload, FileEntry, PaneSide, theme_colors};
use crate::views::FileListView;

// Define actions for dual pane
actions!(dual_pane, [
    ToggleDualPane,
    SwitchPane,
    CopyToOther,
    MoveToOther,
]);

/// Pending operation from dual pane
#[derive(Debug, Clone)]
pub enum DualPaneAction {
    CopyFiles { sources: Vec<PathBuf>, destination: PathBuf },
    MoveFiles { sources: Vec<PathBuf>, destination: PathBuf },
    NavigateLeft(PathBuf),
    NavigateRight(PathBuf),
}

/// View component for dual pane file browsing
pub struct DualPaneView {
    dual_pane: DualPane,
    left_file_list: Entity<FileListView>,
    right_file_list: Entity<FileListView>,
    focus_handle: FocusHandle,
    pending_action: Option<DualPaneAction>,
    left_drop_hover: bool,
    right_drop_hover: bool,
}

impl DualPaneView {
    pub fn new(initial_path: PathBuf, cx: &mut Context<Self>) -> Self {
        let dual_pane = DualPane::new(initial_path.clone());
        
        // Create file lists for both panes
        let left_file_list = cx.new(|cx| FileListView::new(cx));
        let right_file_list = cx.new(|cx| FileListView::new(cx));
        
        Self {
            dual_pane,
            left_file_list,
            right_file_list,
            focus_handle: cx.focus_handle(),
            pending_action: None,
            left_drop_hover: false,
            right_drop_hover: false,
        }
    }

    pub fn with_paths(left_path: PathBuf, right_path: PathBuf, cx: &mut Context<Self>) -> Self {
        let dual_pane = DualPane::with_paths(left_path, right_path);
        
        let left_file_list = cx.new(|cx| FileListView::new(cx));
        let right_file_list = cx.new(|cx| FileListView::new(cx));
        
        Self {
            dual_pane,
            left_file_list,
            right_file_list,
            focus_handle: cx.focus_handle(),
            pending_action: None,
            left_drop_hover: false,
            right_drop_hover: false,
        }
    }

    /// Register key bindings for dual pane operations
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("cmd-shift-d", ToggleDualPane, Some("DualPane")),
            KeyBinding::new("tab", SwitchPane, Some("DualPane")),
            KeyBinding::new("f5", CopyToOther, Some("DualPane")),
            KeyBinding::new("f6", MoveToOther, Some("DualPane")),
        ]);
    }

    /// Returns whether dual pane mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.dual_pane.is_enabled()
    }

    /// Enables dual pane mode
    pub fn enable(&mut self, cx: &mut Context<Self>) {
        self.dual_pane.enable();
        cx.notify();
    }

    /// Disables dual pane mode
    pub fn disable(&mut self, cx: &mut Context<Self>) {
        self.dual_pane.disable();
        cx.notify();
    }

    /// Toggles dual pane mode
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.dual_pane.toggle();
        cx.notify();
    }

    /// Returns the active pane side
    pub fn active_side(&self) -> PaneSide {
        self.dual_pane.active_side()
    }

    /// Switches the active pane
    pub fn switch_active(&mut self, cx: &mut Context<Self>) {
        self.dual_pane.switch_active();
        cx.notify();
    }

    /// Sets the active pane
    pub fn set_active(&mut self, side: PaneSide, cx: &mut Context<Self>) {
        self.dual_pane.set_active(side);
        cx.notify();
    }

    /// Returns a reference to the underlying DualPane model
    pub fn inner(&self) -> &DualPane {
        &self.dual_pane
    }

    /// Returns a mutable reference to the underlying DualPane model
    pub fn inner_mut(&mut self) -> &mut DualPane {
        &mut self.dual_pane
    }

    /// Sets entries for the left pane
    pub fn set_left_entries(&mut self, entries: Vec<FileEntry>, cx: &mut Context<Self>) {
        self.dual_pane.left_pane_mut().set_entries(entries.clone());
        self.left_file_list.update(cx, |view, _| {
            view.inner_mut().set_entries(entries);
        });
        cx.notify();
    }

    /// Sets entries for the right pane
    pub fn set_right_entries(&mut self, entries: Vec<FileEntry>, cx: &mut Context<Self>) {
        self.dual_pane.right_pane_mut().set_entries(entries.clone());
        self.right_file_list.update(cx, |view, _| {
            view.inner_mut().set_entries(entries);
        });
        cx.notify();
    }

    /// Sets entries for a specific pane
    pub fn set_pane_entries(&mut self, side: PaneSide, entries: Vec<FileEntry>, cx: &mut Context<Self>) {
        match side {
            PaneSide::Left => self.set_left_entries(entries, cx),
            PaneSide::Right => self.set_right_entries(entries, cx),
        }
    }

    /// Navigates the left pane to a new path
    pub fn navigate_left(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.dual_pane.left_pane_mut().navigate_to(path.clone());
        self.pending_action = Some(DualPaneAction::NavigateLeft(path));
        cx.notify();
    }

    /// Navigates the right pane to a new path
    pub fn navigate_right(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.dual_pane.right_pane_mut().navigate_to(path.clone());
        self.pending_action = Some(DualPaneAction::NavigateRight(path));
        cx.notify();
    }

    /// Takes the pending action, if any
    pub fn take_pending_action(&mut self) -> Option<DualPaneAction> {
        self.pending_action.take()
    }

    /// Returns the left pane's current path
    pub fn left_path(&self) -> &PathBuf {
        &self.dual_pane.left_pane().path
    }

    /// Returns the right pane's current path
    pub fn right_path(&self) -> &PathBuf {
        &self.dual_pane.right_pane().path
    }

    /// Returns the active pane's current path
    pub fn active_path(&self) -> &PathBuf {
        &self.dual_pane.active_pane().path
    }

    // Action handlers
    fn handle_toggle(&mut self, _: &ToggleDualPane, _window: &mut Window, cx: &mut Context<Self>) {
        self.toggle(cx);
    }

    fn handle_switch_pane(&mut self, _: &SwitchPane, _window: &mut Window, cx: &mut Context<Self>) {
        self.switch_active(cx);
    }

    fn handle_copy_to_other(&mut self, _: &CopyToOther, _window: &mut Window, cx: &mut Context<Self>) {
        let sources = self.dual_pane.copy_to_other();
        if !sources.is_empty() {
            let destination = self.dual_pane.destination_path().clone();
            self.pending_action = Some(DualPaneAction::CopyFiles { sources, destination });
            cx.notify();
        }
    }

    fn handle_move_to_other(&mut self, _: &MoveToOther, _window: &mut Window, cx: &mut Context<Self>) {
        let sources = self.dual_pane.move_to_other();
        if !sources.is_empty() {
            let destination = self.dual_pane.destination_path().clone();
            self.pending_action = Some(DualPaneAction::MoveFiles { sources, destination });
            cx.notify();
        }
    }

    /// Handles a drop on the left pane
    pub fn handle_drop_left(&mut self, payload: DragPayload, cx: &mut Context<Self>) {
        self.left_drop_hover = false;
        let destination = self.dual_pane.left_pane().path.clone();
        self.pending_action = Some(DualPaneAction::CopyFiles {
            sources: payload.paths,
            destination,
        });
        cx.notify();
    }

    /// Handles a drop on the right pane
    pub fn handle_drop_right(&mut self, payload: DragPayload, cx: &mut Context<Self>) {
        self.right_drop_hover = false;
        let destination = self.dual_pane.right_pane().path.clone();
        self.pending_action = Some(DualPaneAction::CopyFiles {
            sources: payload.paths,
            destination,
        });
        cx.notify();
    }

    /// Sets the left pane drop hover state
    pub fn set_left_drop_hover(&mut self, hover: bool, cx: &mut Context<Self>) {
        self.left_drop_hover = hover;
        cx.notify();
    }

    /// Sets the right pane drop hover state
    pub fn set_right_drop_hover(&mut self, hover: bool, cx: &mut Context<Self>) {
        self.right_drop_hover = hover;
        cx.notify();
    }

    fn render_pane_header(&self, side: PaneSide, path: &PathBuf) -> impl IntoElement {
        let theme = theme_colors();
        let is_active = self.dual_pane.active_side() == side;
        
        let bg_color = if is_active {
            theme.bg_selected
        } else {
            theme.bg_secondary
        };
        
        let path_str = path.to_string_lossy().to_string();
        let display_path = if path_str.len() > 40 {
            format!("...{}", &path_str[path_str.len() - 37..])
        } else {
            path_str
        };

        div()
            .h(px(32.0))
            .px_3()
            .flex()
            .items_center()
            .bg(bg_color)
            .border_b_1()
            .border_color(theme.border_default)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(if is_active { theme.text_primary } else { theme.text_muted })
                    .truncate()
                    .child(display_path)
            )
    }
}

impl Focusable for DualPaneView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DualPaneView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let is_enabled = self.dual_pane.is_enabled();
        let active_side = self.dual_pane.active_side();
        
        let left_path = self.dual_pane.left_pane().path.clone();
        let right_path = self.dual_pane.right_pane().path.clone();

        div()
            .id("dual-pane")
            .key_context("DualPane")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_toggle))
            .on_action(cx.listener(Self::handle_switch_pane))
            .on_action(cx.listener(Self::handle_copy_to_other))
            .on_action(cx.listener(Self::handle_move_to_other))
            .size_full()
            .flex()
            .bg(theme.bg_void)
            .when(is_enabled, |this| {
                this.child(
                    // Left pane
                    div()
                        .id("left-pane")
                        .flex_1()
                        .flex()
                        .flex_col()
                        .border_r_1()
                        .border_color(theme.border_default)
                        .when(active_side == PaneSide::Left, |s| {
                            s.border_2().border_color(theme.accent_primary)
                        })
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|view, _, _, cx| {
                            view.set_active(PaneSide::Left, cx);
                        }))
                        .child(self.render_pane_header(PaneSide::Left, &left_path))
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .child(self.left_file_list.clone())
                        )
                )
                .child(
                    // Right pane
                    div()
                        .id("right-pane")
                        .flex_1()
                        .flex()
                        .flex_col()
                        .when(active_side == PaneSide::Right, |s| {
                            s.border_2().border_color(theme.accent_primary)
                        })
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|view, _, _, cx| {
                            view.set_active(PaneSide::Right, cx);
                        }))
                        .child(self.render_pane_header(PaneSide::Right, &right_path))
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .child(self.right_file_list.clone())
                        )
                )
            })
            .when(!is_enabled, |this| {
                // Single pane mode - just show left pane
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(self.render_pane_header(PaneSide::Left, &left_path))
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .child(self.left_file_list.clone())
                        )
                )
            })
    }
}
