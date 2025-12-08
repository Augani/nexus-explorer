use crate::models::{
    AclEntryType, FilePermissions, PermissionError, PermissionsManager,
    UnixPermissions, WindowsAcl,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::PathBuf;

/// Actions for the permissions dialog
#[derive(Clone, PartialEq)]
pub enum PermissionsDialogAction {
    Close,
    Apply,
    ApplyRecursive,
    OwnerReadChanged(bool),
    OwnerWriteChanged(bool),
    OwnerExecuteChanged(bool),
    GroupReadChanged(bool),
    GroupWriteChanged(bool),
    GroupExecuteChanged(bool),
    OthersReadChanged(bool),
    OthersWriteChanged(bool),
    OthersExecuteChanged(bool),
    SetuidChanged(bool),
    SetgidChanged(bool),
    StickyChanged(bool),
}

/// State for the permissions dialog
pub struct PermissionsDialog {
    path: PathBuf,
    original_permissions: Option<FilePermissions>,
    modified_permissions: Option<FilePermissions>,
    error_message: Option<String>,
    success_message: Option<String>,
    is_directory: bool,
    requires_elevation: bool,
    is_applying: bool,
    on_close: Option<Box<dyn Fn() + Send + Sync>>,
    on_apply: Option<Box<dyn Fn(PathBuf, FilePermissions) + Send + Sync>>,
}

impl PermissionsDialog {
    pub fn new(path: PathBuf) -> Self {
        let is_directory = path.is_dir();
        let requires_elevation = PermissionsManager::requires_elevation(&path);
        let original_permissions = PermissionsManager::read_permissions(&path).ok();
        let modified_permissions = original_permissions.clone();

        Self {
            path,
            original_permissions,
            modified_permissions,
            error_message: None,
            success_message: None,
            is_directory,
            requires_elevation,
            is_applying: false,
            on_close: None,
            on_apply: None,
        }
    }

    pub fn with_on_close<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_close = Some(Box::new(callback));
        self
    }

    pub fn with_on_apply<F>(mut self, callback: F) -> Self
    where
        F: Fn(PathBuf, FilePermissions) + Send + Sync + 'static,
    {
        self.on_apply = Some(Box::new(callback));
        self
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn is_directory(&self) -> bool {
        self.is_directory
    }

    pub fn requires_elevation(&self) -> bool {
        self.requires_elevation
    }

    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }

    pub fn success_message(&self) -> Option<&String> {
        self.success_message.as_ref()
    }

    pub fn is_applying(&self) -> bool {
        self.is_applying
    }

    /// Get the current permissions (modified or original)
    pub fn current_permissions(&self) -> Option<&FilePermissions> {
        self.modified_permissions.as_ref()
    }

    /// Check if permissions have been modified
    pub fn has_changes(&self) -> bool {
        match (&self.original_permissions, &self.modified_permissions) {
            (Some(FilePermissions::Unix(orig)), Some(FilePermissions::Unix(modified))) => {
                orig.to_mode() != modified.to_mode()
            }
            _ => false,
        }
    }

    /// Apply the current permissions
    pub fn apply(&mut self) -> Result<(), PermissionError> {
        if let Some(perms) = &self.modified_permissions {
            self.is_applying = true;
            self.error_message = None;
            self.success_message = None;

            match PermissionsManager::write_permissions(&self.path, perms) {
                Ok(()) => {
                    self.original_permissions = self.modified_permissions.clone();
                    self.success_message = Some("Permissions applied successfully".to_string());
                    self.is_applying = false;
                    
                    if let Some(callback) = &self.on_apply {
                        callback(self.path.clone(), perms.clone());
                    }
                    Ok(())
                }
                Err(e) => {
                    self.error_message = Some(e.to_string());
                    self.is_applying = false;
                    Err(e)
                }
            }
        } else {
            Err(PermissionError::PlatformNotSupported(
                "No permissions to apply".to_string(),
            ))
        }
    }

    /// Apply permissions recursively (for directories)
    #[cfg(unix)]
    pub fn apply_recursive(&mut self) -> Result<Vec<PathBuf>, PermissionError> {
        if !self.is_directory {
            return Err(PermissionError::PlatformNotSupported(
                "Recursive apply only works on directories".to_string(),
            ));
        }

        if let Some(FilePermissions::Unix(perms)) = &self.modified_permissions {
            self.is_applying = true;
            self.error_message = None;
            self.success_message = None;

            match PermissionsManager::apply_recursive(&self.path, perms, true) {
                Ok(failed) => {
                    self.is_applying = false;
                    if failed.is_empty() {
                        self.success_message =
                            Some("Permissions applied recursively".to_string());
                    } else {
                        self.error_message = Some(format!(
                            "Failed to apply permissions to {} files",
                            failed.len()
                        ));
                    }
                    Ok(failed)
                }
                Err(e) => {
                    self.is_applying = false;
                    self.error_message = Some(e.to_string());
                    Err(e)
                }
            }
        } else {
            Err(PermissionError::PlatformNotSupported(
                "Recursive apply only works with Unix permissions".to_string(),
            ))
        }
    }

    /// Close the dialog
    pub fn close(&self) {
        if let Some(callback) = &self.on_close {
            callback();
        }
    }

    /// Update Unix permission bit
    fn update_unix_bit<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut UnixPermissions),
    {
        if let Some(FilePermissions::Unix(ref mut perms)) = self.modified_permissions {
            updater(perms);
        }
    }

    /// Handle permission change actions
    pub fn handle_action(&mut self, action: PermissionsDialogAction) {
        match action {
            PermissionsDialogAction::Close => self.close(),
            PermissionsDialogAction::Apply => {
                let _ = self.apply();
            }
            PermissionsDialogAction::ApplyRecursive => {
                #[cfg(unix)]
                {
                    let _ = self.apply_recursive();
                }
            }
            PermissionsDialogAction::OwnerReadChanged(v) => {
                self.update_unix_bit(|p| p.owner.read = v);
            }
            PermissionsDialogAction::OwnerWriteChanged(v) => {
                self.update_unix_bit(|p| p.owner.write = v);
            }
            PermissionsDialogAction::OwnerExecuteChanged(v) => {
                self.update_unix_bit(|p| p.owner.execute = v);
            }
            PermissionsDialogAction::GroupReadChanged(v) => {
                self.update_unix_bit(|p| p.group.read = v);
            }
            PermissionsDialogAction::GroupWriteChanged(v) => {
                self.update_unix_bit(|p| p.group.write = v);
            }
            PermissionsDialogAction::GroupExecuteChanged(v) => {
                self.update_unix_bit(|p| p.group.execute = v);
            }
            PermissionsDialogAction::OthersReadChanged(v) => {
                self.update_unix_bit(|p| p.others.read = v);
            }
            PermissionsDialogAction::OthersWriteChanged(v) => {
                self.update_unix_bit(|p| p.others.write = v);
            }
            PermissionsDialogAction::OthersExecuteChanged(v) => {
                self.update_unix_bit(|p| p.others.execute = v);
            }
            PermissionsDialogAction::SetuidChanged(v) => {
                self.update_unix_bit(|p| p.special.setuid = v);
            }
            PermissionsDialogAction::SetgidChanged(v) => {
                self.update_unix_bit(|p| p.special.setgid = v);
            }
            PermissionsDialogAction::StickyChanged(v) => {
                self.update_unix_bit(|p| p.special.sticky = v);
            }
        }
    }

    /// Get Unix permissions if available
    pub fn unix_permissions(&self) -> Option<&UnixPermissions> {
        match &self.modified_permissions {
            Some(FilePermissions::Unix(perms)) => Some(perms),
            _ => None,
        }
    }

    /// Get Windows ACL if available
    pub fn windows_acl(&self) -> Option<&WindowsAcl> {
        match &self.modified_permissions {
            Some(FilePermissions::Windows(acl)) => Some(acl),
            _ => None,
        }
    }

    /// Get the octal mode string for Unix permissions
    pub fn octal_mode(&self) -> Option<String> {
        self.unix_permissions().map(|p| p.to_octal_string())
    }

    /// Get the symbolic mode string for Unix permissions
    pub fn symbolic_mode(&self) -> Option<String> {
        self.unix_permissions().map(|p| p.to_symbolic())
    }
}


/// Render helper functions for the permissions dialog
impl PermissionsDialog {
    /// Render a permission checkbox
    fn render_permission_checkbox(
        label: &str,
        checked: bool,
        on_change: impl Fn(bool) -> PermissionsDialogAction + 'static,
    ) -> impl IntoElement {
        let checkbox_bg = if checked {
            rgb(0x3B82F6) // Blue when checked
        } else {
            rgb(0x374151) // Gray when unchecked
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(px(3.0))
                    .bg(checkbox_bg)
                    .border_1()
                    .border_color(rgb(0x4B5563))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |el| {
                        el.child(
                            div()
                                .text_color(rgb(0xFFFFFF))
                                .text_size(px(12.0))
                                .child("✓"),
                        )
                    }),
            )
            .child(
                div()
                    .text_color(rgb(0xE5E7EB))
                    .text_size(px(13.0))
                    .child(label.to_string()),
            )
    }

    /// Render Unix permissions section
    fn render_unix_permissions(&self, perms: &UnixPermissions) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            // Owner section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_color(rgb(0x9CA3AF))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!(
                                "Owner{}",
                                perms
                                    .owner_name
                                    .as_ref()
                                    .map(|n| format!(" ({})", n))
                                    .unwrap_or_default()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(Self::render_permission_checkbox(
                                "Read",
                                perms.owner.read,
                                PermissionsDialogAction::OwnerReadChanged,
                            ))
                            .child(Self::render_permission_checkbox(
                                "Write",
                                perms.owner.write,
                                PermissionsDialogAction::OwnerWriteChanged,
                            ))
                            .child(Self::render_permission_checkbox(
                                "Execute",
                                perms.owner.execute,
                                PermissionsDialogAction::OwnerExecuteChanged,
                            )),
                    ),
            )
            // Group section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_color(rgb(0x9CA3AF))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!(
                                "Group{}",
                                perms
                                    .group_name
                                    .as_ref()
                                    .map(|n| format!(" ({})", n))
                                    .unwrap_or_default()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(Self::render_permission_checkbox(
                                "Read",
                                perms.group.read,
                                PermissionsDialogAction::GroupReadChanged,
                            ))
                            .child(Self::render_permission_checkbox(
                                "Write",
                                perms.group.write,
                                PermissionsDialogAction::GroupWriteChanged,
                            ))
                            .child(Self::render_permission_checkbox(
                                "Execute",
                                perms.group.execute,
                                PermissionsDialogAction::GroupExecuteChanged,
                            )),
                    ),
            )
            // Others section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_color(rgb(0x9CA3AF))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Others"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(Self::render_permission_checkbox(
                                "Read",
                                perms.others.read,
                                PermissionsDialogAction::OthersReadChanged,
                            ))
                            .child(Self::render_permission_checkbox(
                                "Write",
                                perms.others.write,
                                PermissionsDialogAction::OthersWriteChanged,
                            ))
                            .child(Self::render_permission_checkbox(
                                "Execute",
                                perms.others.execute,
                                PermissionsDialogAction::OthersExecuteChanged,
                            )),
                    ),
            )
            // Special bits section
            .when(perms.has_special_bits() || true, |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .mt_2()
                        .pt_2()
                        .border_t_1()
                        .border_color(rgb(0x374151))
                        .child(
                            div()
                                .text_color(rgb(0x9CA3AF))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Special Permissions"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_4()
                                .child(Self::render_permission_checkbox(
                                    "Set User ID",
                                    perms.special.setuid,
                                    PermissionsDialogAction::SetuidChanged,
                                ))
                                .child(Self::render_permission_checkbox(
                                    "Set Group ID",
                                    perms.special.setgid,
                                    PermissionsDialogAction::SetgidChanged,
                                ))
                                .child(Self::render_permission_checkbox(
                                    "Sticky Bit",
                                    perms.special.sticky,
                                    PermissionsDialogAction::StickyChanged,
                                )),
                        ),
                )
            })
            // Mode display
            .child(
                div()
                    .flex()
                    .gap_4()
                    .mt_2()
                    .pt_2()
                    .border_t_1()
                    .border_color(rgb(0x374151))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(rgb(0x9CA3AF))
                                    .text_size(px(12.0))
                                    .child("Octal:"),
                            )
                            .child(
                                div()
                                    .text_color(rgb(0xE5E7EB))
                                    .text_size(px(12.0))
                                    .font_family("monospace")
                                    .child(perms.to_octal_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(rgb(0x9CA3AF))
                                    .text_size(px(12.0))
                                    .child("Symbolic:"),
                            )
                            .child(
                                div()
                                    .text_color(rgb(0xE5E7EB))
                                    .text_size(px(12.0))
                                    .font_family("monospace")
                                    .child(perms.to_symbolic()),
                            ),
                    ),
            )
    }

    /// Render Windows ACL section
    fn render_windows_acl(&self, acl: &WindowsAcl) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            // Owner info
            .when_some(acl.owner.as_ref(), |el, owner| {
                el.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            div()
                                .text_color(rgb(0x9CA3AF))
                                .text_size(px(12.0))
                                .child("Owner:"),
                        )
                        .child(
                            div()
                                .text_color(rgb(0xE5E7EB))
                                .text_size(px(12.0))
                                .child(owner.clone()),
                        ),
                )
            })
            // ACL entries
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .mt_2()
                    .child(
                        div()
                            .text_color(rgb(0x9CA3AF))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Access Control Entries"),
                    )
                    .children(acl.entries.iter().map(|entry| {
                        let entry_type_color = match entry.entry_type {
                            AclEntryType::Allow => rgb(0x10B981), // Green
                            AclEntryType::Deny => rgb(0xEF4444),  // Red
                        };
                        let entry_type_text = match entry.entry_type {
                            AclEntryType::Allow => "Allow",
                            AclEntryType::Deny => "Deny",
                        };

                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .rounded(px(4.0))
                            .bg(rgb(0x1F2937))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(entry_type_color)
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(entry_type_text),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0xE5E7EB))
                                            .text_size(px(12.0))
                                            .child(entry.principal_name.clone()),
                                    )
                                    .when(entry.inherited, |el| {
                                        el.child(
                                            div()
                                                .text_color(rgb(0x6B7280))
                                                .text_size(px(10.0))
                                                .child("(inherited)"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_1()
                                    .children(entry.permissions.iter().map(|perm| {
                                        div()
                                            .px_2()
                                            .py(px(2.0))
                                            .rounded(px(3.0))
                                            .bg(rgb(0x374151))
                                            .text_color(rgb(0xD1D5DB))
                                            .text_size(px(10.0))
                                            .child(perm.display_name())
                                    })),
                            )
                    })),
            )
    }
}

/// View implementation for PermissionsDialog
pub struct PermissionsDialogView {
    dialog: PermissionsDialog,
    focus_handle: FocusHandle,
}

impl PermissionsDialogView {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            dialog: PermissionsDialog::new(path),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn dialog(&self) -> &PermissionsDialog {
        &self.dialog
    }

    pub fn dialog_mut(&mut self) -> &mut PermissionsDialog {
        &mut self.dialog
    }
}

impl Focusable for PermissionsDialogView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PermissionsDialogView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let path_display = self.dialog.path().display().to_string();
        let file_name = self
            .dialog
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_display.clone());

        div()
            .flex()
            .flex_col()
            .w(px(450.0))
            .max_h(px(600.0))
            .bg(rgb(0x111827))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(0x374151))
            .shadow_lg()
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0x374151))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_color(rgb(0xF9FAFB))
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Permissions"),
                            )
                            .child(
                                div()
                                    .text_color(rgb(0x9CA3AF))
                                    .text_size(px(12.0))
                                    .child(file_name),
                            ),
                    ),
            )
            // Content
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when_some(self.dialog.unix_permissions(), |el, perms| {
                        el.child(self.dialog.render_unix_permissions(perms))
                    })
                    .when_some(self.dialog.windows_acl(), |el, acl| {
                        el.child(self.dialog.render_windows_acl(acl))
                    })
                    .when(self.dialog.current_permissions().is_none(), |el| {
                        el.child(
                            div()
                                .p_4()
                                .text_color(rgb(0xEF4444))
                                .text_size(px(13.0))
                                .child("Unable to read permissions for this file."),
                        )
                    }),
            )
            // Error/Success messages
            .when_some(self.dialog.error_message(), |el, msg| {
                el.child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(rgb(0x7F1D1D))
                        .text_color(rgb(0xFCA5A5))
                        .text_size(px(12.0))
                        .child(msg.clone()),
                )
            })
            .when_some(self.dialog.success_message(), |el, msg| {
                el.child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(rgb(0x064E3B))
                        .text_color(rgb(0x6EE7B7))
                        .text_size(px(12.0))
                        .child(msg.clone()),
                )
            })
            // Elevation warning
            .when(self.dialog.requires_elevation(), |el| {
                el.child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(rgb(0x78350F))
                        .text_color(rgb(0xFCD34D))
                        .text_size(px(11.0))
                        .child("⚠ Modifying permissions may require administrator privileges"),
                )
            })
            // Footer with buttons
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(0x374151))
                    .when(self.dialog.is_directory(), |el| {
                        el.child(
                            div()
                                .px_3()
                                .py(px(6.0))
                                .rounded(px(4.0))
                                .bg(rgb(0x374151))
                                .text_color(rgb(0xE5E7EB))
                                .text_size(px(13.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x4B5563)))
                                .child("Apply to Contents"),
                        )
                    })
                    .child(
                        div()
                            .px_3()
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(rgb(0x374151))
                            .text_color(rgb(0xE5E7EB))
                            .text_size(px(13.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x4B5563)))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .px_3()
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(rgb(0x3B82F6))
                            .text_color(rgb(0xFFFFFF))
                            .text_size(px(13.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x2563EB)))
                            .when(!self.dialog.has_changes(), |el| {
                                el.opacity(0.5).cursor_default()
                            })
                            .child("Apply"),
                    ),
            )
    }
}
