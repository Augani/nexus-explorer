use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use gpui::{
    actions, div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

use crate::io::{SortKey, SortOrder};
use crate::models::{FileSystem, GlobalSettings, GridConfig, IconCache, SearchEngine, ThemeId, ViewMode, WindowManager, theme_colors, current_theme};
use crate::views::{FileList, FileListView, GridView, GridViewComponent, PreviewView, SearchInputView, SidebarView, StatusBarView, StatusBarAction, ThemePickerView, ToolAction, TerminalView, QuickLookView, ToastManager, ContextMenuAction};
use adabraka_ui::components::input::{Input, InputState, InputEvent};

// Define global keyboard shortcut actions
actions!(workspace, [
    NewTab,
    CloseTab,
    ToggleTerminal,
    FocusSearch,
    NewWindow,
    QuickLookToggle,
]);

/// Recursively copy a directory (standalone function for background thread)
fn copy_dir_recursive_async(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive_async(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Dialog state for creating new files/folders
#[derive(Clone)]
pub enum DialogState {
    None,
    NewFile { name: String },
    NewFolder { name: String },
    Rename { path: PathBuf, name: String },
}

/// Clipboard operation type
#[derive(Clone, Debug, PartialEq)]
pub enum ClipboardOperation {
    Copy(PathBuf),
    Cut(PathBuf),
}

pub struct Workspace {
    file_system: Entity<FileSystem>,
    icon_cache: Entity<IconCache>,
    search_engine: Entity<SearchEngine>,
    file_list: Entity<FileListView>,
    grid_view: Entity<GridViewComponent>,
    sidebar: Entity<SidebarView>,
    search_input: Entity<SearchInputView>,
    preview: Option<Entity<PreviewView>>,
    theme_picker: Entity<ThemePickerView>,
    status_bar: Entity<StatusBarView>,
    terminal: Entity<TerminalView>,
    quick_look: Entity<QuickLookView>,
    toast_manager: Entity<ToastManager>,
    focus_handle: FocusHandle,
    current_path: PathBuf,
    path_history: Vec<PathBuf>,
    is_terminal_open: bool,
    terminal_height: f32,
    is_resizing_terminal: bool,
    preview_width: f32,
    is_resizing_preview: bool,
    cached_entries: Vec<crate::models::FileEntry>,
    view_mode: ViewMode,
    dialog_state: DialogState,
    show_hidden_files: bool,
    current_theme_id: ThemeId,
    clipboard: Option<ClipboardOperation>,
    /// Copy/Move mode - shows destination pane
    copy_move_mode: bool,
    /// Destination file list for copy/move operations
    dest_file_list: Entity<FileListView>,
    /// Current path in destination pane
    dest_path: PathBuf,
    /// Cached entries for destination pane
    dest_entries: Vec<crate::models::FileEntry>,
    /// Input state for new file/folder dialog
    dialog_input: Option<Entity<InputState>>,
    /// Flag to focus dialog input on next render
    should_focus_dialog_input: bool,
}

impl Workspace {
    /// Returns the current directory path for this workspace
    pub fn current_path(&self) -> &PathBuf {
        &self.current_path
    }

    /// Opens a new window with the specified path
    pub fn open_new_window(path: PathBuf, cx: &mut App) {
        if cx.has_global::<WindowManager>() {
            cx.update_global::<WindowManager, _>(|manager, cx| {
                manager.open_window(path, cx);
            });
        }
    }

    /// Opens a new window with the current directory
    pub fn open_new_window_here(&self, cx: &mut App) {
        Self::open_new_window(self.current_path.clone(), cx);
    }
}

impl Workspace {
    /// Register global keyboard shortcuts for the workspace
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("cmd-t", NewTab, Some("Workspace")),
            KeyBinding::new("cmd-w", CloseTab, Some("Workspace")),
            KeyBinding::new("cmd-`", ToggleTerminal, Some("Workspace")),
            KeyBinding::new("cmd-f", FocusSearch, Some("Workspace")),
            KeyBinding::new("cmd-n", NewWindow, Some("Workspace")),
            KeyBinding::new("space", QuickLookToggle, Some("FileList")),
        ]);
    }

    pub fn build(initial_path: PathBuf, cx: &mut App) -> Entity<Self> {
        // Register key bindings for all views
        SearchInputView::register_key_bindings(cx);
        FileListView::register_key_bindings(cx);
        Self::register_key_bindings(cx);
        
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
            
            let search_engine_inner = SearchEngine::new();
            for entry in &cached_entries {
                search_engine_inner.inject(entry.path.clone());
            }
            let search_engine = cx.new(|_| search_engine_inner);

            let file_list = cx.new(|cx| FileListView::with_file_list(file_list_inner, cx));
            
            let mut grid_view_inner = GridView::with_config(GridConfig::default());
            grid_view_inner.set_entries(cached_entries.clone());
            let grid_view = cx.new(|cx| GridViewComponent::with_grid_view(grid_view_inner, cx));
            
            let sidebar = cx.new(|cx| {
                let mut sidebar_view = SidebarView::new(cx);
                sidebar_view.set_workspace_root(initial_path.clone());
                sidebar_view
            });
            
            let search_input = cx.new(|cx| {
                SearchInputView::new(cx).with_search_engine(search_engine.clone())
            });

            let settings = GlobalSettings::load();
            let view_mode = settings.view_mode;
            let show_hidden_files = settings.show_hidden_files;
            let current_theme_id = settings.theme_id;
            
            crate::models::set_current_theme(current_theme_id);

            let theme_picker = cx.new(|cx| {
                ThemePickerView::new(cx).with_selected_theme(current_theme_id)
            });

            let status_bar = cx.new(|cx| {
                let mut status_bar_view = StatusBarView::new(cx);
                status_bar_view.update_from_entries(&cached_entries, None, cx);
                status_bar_view.set_current_directory(&initial_path, cx);
                status_bar_view.set_view_mode(view_mode, cx);
                status_bar_view
            });

            let terminal = cx.new(|cx| {
                TerminalView::new(cx).with_working_directory(initial_path.clone())
            });

            let quick_look = cx.new(|cx| QuickLookView::new(cx));

            let toast_manager = cx.new(|cx| ToastManager::new(cx));

            // Observe file list for navigation requests and selection changes
            let sidebar_for_file_list = sidebar.clone();
            let status_bar_for_file_list = status_bar.clone();
            cx.observe(&file_list, move |workspace: &mut Workspace, file_list, cx| {
                let wants_parent = file_list.update(cx, |view, _| view.take_pending_parent_navigation());
                if wants_parent {
                    workspace.navigate_up(cx);
                }
                
                let nav_path = file_list.update(cx, |view, _| view.take_pending_navigation());
                if let Some(path) = nav_path {
                    workspace.navigate_to(path, cx);
                }
                
                let context_action = file_list.update(cx, |view, _| view.take_pending_context_action());
                if let Some(action) = context_action {
                    workspace.handle_context_menu_action(action, cx);
                }
                
                let selected_index = file_list.read(cx).inner().selected_index();
                let selection_count = if selected_index.is_some() { 1 } else { 0 };
                sidebar_for_file_list.update(cx, |view, _| {
                    view.set_selected_file_count(selection_count);
                });
                
                status_bar_for_file_list.update(cx, |view, cx| {
                    view.update_from_entries(&workspace.cached_entries, selected_index, cx);
                });
                
                workspace.update_preview_for_selection(cx);
            })
            .detach();
            
            // Observe grid view for navigation requests and selection changes
            let sidebar_for_grid = sidebar.clone();
            let status_bar_for_grid = status_bar.clone();
            cx.observe(&grid_view, move |workspace: &mut Workspace, grid_view, cx| {
                let nav_path = grid_view.update(cx, |view, _| view.take_pending_navigation());
                if let Some(path) = nav_path {
                    workspace.navigate_to(path, cx);
                }
                
                let context_action = grid_view.update(cx, |view, _| view.take_pending_context_action());
                if let Some(action) = context_action {
                    workspace.handle_context_menu_action(action, cx);
                }
                
                let selected_index = grid_view.read(cx).inner().selected_index();
                let selection_count = if selected_index.is_some() { 1 } else { 0 };
                sidebar_for_grid.update(cx, |view, _| {
                    view.set_selected_file_count(selection_count);
                });
                
                status_bar_for_grid.update(cx, |view, cx| {
                    view.update_from_entries(&workspace.cached_entries, selected_index, cx);
                });
                
                workspace.update_preview_for_selection(cx);
            })
            .detach();
            
            // Observe search input for query changes
            cx.observe(&search_input, |workspace: &mut Workspace, search_input, cx| {
                let query = search_input.read(cx).query().to_string();
                workspace.handle_search_query_change(&query, cx);
            })
            .detach();
            
            // Observe sidebar for tool actions
            cx.observe(&sidebar, |workspace: &mut Workspace, sidebar, cx| {
                let action = sidebar.update(cx, |view, _| view.take_pending_action());
                if let Some(action) = action {
                    workspace.handle_tool_action(action, cx);
                }
                
                let nav_path = sidebar.update(cx, |view, _| view.take_pending_navigation());
                if let Some(path) = nav_path {
                    workspace.navigate_to(path, cx);
                }
            })
            .detach();

            // Observe status bar for actions
            cx.observe(&status_bar, |workspace: &mut Workspace, status_bar, cx| {
                let action = status_bar.update(cx, |view, _| view.take_pending_action());
                if let Some(action) = action {
                    match action {
                        StatusBarAction::ToggleTerminal => workspace.toggle_terminal(cx),
                        StatusBarAction::ToggleViewMode => workspace.toggle_view_mode(cx),
                    }
                }
            })
            .detach();

            // Observe theme picker for theme changes
            cx.observe(&theme_picker, |workspace: &mut Workspace, theme_picker, cx| {
                let selected = theme_picker.read(cx).selected_theme();
                if workspace.current_theme_id != selected {
                    workspace.set_theme(selected, cx);
                }
            })
            .detach();

            Self {
                file_system,
                icon_cache,
                search_engine,
                file_list,
                grid_view,
                sidebar,
                search_input,
                preview: None,
                theme_picker,
                status_bar,
                terminal,
                quick_look,
                toast_manager,
                focus_handle: cx.focus_handle(),
                current_path: initial_path.clone(),
                path_history: vec![initial_path.clone()],
                is_terminal_open: false,
                terminal_height: 300.0,
                is_resizing_terminal: false,
                preview_width: 320.0,
                is_resizing_preview: false,
                cached_entries,
                view_mode,
                dialog_state: DialogState::None,
                show_hidden_files,
                current_theme_id,
                clipboard: None,
                copy_move_mode: false,
                dest_file_list: cx.new(|cx| FileListView::with_file_list(FileList::new(), cx)),
                dest_path: initial_path,
                dest_entries: Vec::new(),
                dialog_input: None,
                should_focus_dialog_input: false,
            }
        })
    }
    
    fn open_dialog(&mut self, is_file: bool, cx: &mut Context<Self>) {
        let input_state = cx.new(|cx| InputState::new(cx));
        
        cx.subscribe(&input_state, |workspace: &mut Workspace, _input, event: &InputEvent, cx| {
            match event {
                InputEvent::Enter => {
                    workspace.submit_dialog(cx);
                }
                _ => {}
            }
        }).detach();
        
        self.dialog_input = Some(input_state);
        self.dialog_state = if is_file {
            DialogState::NewFile { name: String::new() }
        } else {
            DialogState::NewFolder { name: String::new() }
        };
        self.should_focus_dialog_input = true;
        cx.notify();
    }
    
    fn submit_dialog(&mut self, cx: &mut Context<Self>) {
        let name = if let Some(input) = &self.dialog_input {
            input.read(cx).content.to_string()
        } else {
            return;
        };
        
        if name.is_empty() {
            return;
        }
        
        match &self.dialog_state {
            DialogState::NewFile { .. } => {
                self.create_new_file(&name, cx);
            }
            DialogState::NewFolder { .. } => {
                self.create_new_folder(&name, cx);
            }
            DialogState::Rename { .. } => {
                self.submit_rename(cx);
                return;
            }
            DialogState::None => {}
        }
        
        self.dialog_input = None;
    }
    
    fn handle_tool_action(&mut self, action: ToolAction, cx: &mut Context<Self>) {
        match action {
            ToolAction::NewFile => {
                self.open_dialog(true, cx);
            }
            ToolAction::NewFolder => {
                self.open_dialog(false, cx);
            }
            ToolAction::Refresh => {
                self.refresh_current_directory(cx);
            }
            ToolAction::OpenTerminalHere => {
                self.is_terminal_open = true;
                cx.notify();
            }
            ToolAction::ToggleHiddenFiles => {
                let show_hidden = self.sidebar.read(cx).show_hidden_files();
                self.show_hidden_files = show_hidden;
                self.refresh_current_directory(cx);
            }
            ToolAction::CopyPath => {
            }
            ToolAction::Copy => {
                if let Some(entry) = self.get_selected_entry(cx) {
                    let name = entry.path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("item")
                        .to_string();
                    self.clipboard = Some(ClipboardOperation::Copy(entry.path.clone()));
                    self.copy_move_mode = true;
                    self.dest_path = self.current_path.clone();
                    self.load_destination_entries(cx);
                    self.toast_manager.update(cx, |toast, cx| {
                        toast.show_info(format!("Select destination for: {}", name), cx);
                    });
                    cx.notify();
                }
            }
            ToolAction::Move => {
                if let Some(entry) = self.get_selected_entry(cx) {
                    let name = entry.path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("item")
                        .to_string();
                    self.clipboard = Some(ClipboardOperation::Cut(entry.path.clone()));
                    self.copy_move_mode = true;
                    self.dest_path = self.current_path.clone();
                    self.load_destination_entries(cx);
                    self.toast_manager.update(cx, |toast, cx| {
                        toast.show_info(format!("Select destination for: {}", name), cx);
                    });
                    cx.notify();
                }
            }
            ToolAction::Paste => {
                self.paste_from_clipboard(cx);
            }
            ToolAction::Delete => {
                if let Some(entry) = self.get_selected_entry(cx) {
                    let name = entry.path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("item")
                        .to_string();
                    let path = entry.path.clone();
                    let is_dir = entry.is_dir;
                    
                    let result = if is_dir {
                        fs::remove_dir_all(&path)
                    } else {
                        fs::remove_file(&path)
                    };
                    
                    match result {
                        Ok(()) => {
                            self.file_list.update(cx, |view, _| {
                                view.inner_mut().set_selected_index(None);
                            });
                            self.preview = None;
                            self.toast_manager.update(cx, |toast, cx| {
                                toast.show_success(format!("Deleted: {}", name), cx);
                            });
                            self.refresh_current_directory(cx);
                        }
                        Err(e) => {
                            self.toast_manager.update(cx, |toast, cx| {
                                toast.show_error(format!("Failed to delete: {}", e), cx);
                            });
                        }
                    }
                }
            }
            ToolAction::SetAsDefault => {
            }
        }
    }
    
    fn handle_context_menu_action(&mut self, action: ContextMenuAction, cx: &mut Context<Self>) {
        match action {
            ContextMenuAction::Open(path) => {
                if path.is_dir() {
                    self.navigate_to(path, cx);
                } else {
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(&path).spawn();
                    }
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("cmd").args(["/C", "start", "", path.to_str().unwrap_or("")]).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                    }
                }
            }
            ContextMenuAction::OpenWith(_path) => {
                // This is now handled by the submenu - kept for backwards compatibility
            }
            ContextMenuAction::OpenWithApp { file_path, app_path, app_name } => {
                let app_info = crate::models::AppInfo::new(app_name.clone(), app_path);
                match crate::models::open_file_with_app(&file_path, &app_info) {
                    Ok(()) => {
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_success(format!("Opening with {}", app_name), cx);
                        });
                    }
                    Err(e) => {
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed to open: {}", e), cx);
                        });
                    }
                }
            }
            ContextMenuAction::OpenWithOther(path) => {
                match crate::models::show_open_with_dialog(&path) {
                    Ok(()) => {}
                    Err(e) => {
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed to show dialog: {}", e), cx);
                        });
                    }
                }
            }
            ContextMenuAction::OpenInNewWindow(path) => {
                Self::open_new_window(path, cx);
            }
            ContextMenuAction::GetInfo(path) => {
                #[cfg(target_os = "macos")]
                {
                    let script = format!(
                        "tell application \"Finder\" to open information window of (POSIX file \"{}\" as alias)",
                        path.display()
                    );
                    let _ = std::process::Command::new("osascript").args(["-e", &script]).spawn();
                }
            }
            ContextMenuAction::Rename(path) => {
                self.start_rename(path, cx);
            }
            ContextMenuAction::Copy(path) => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("item").to_string();
                self.clipboard = Some(ClipboardOperation::Copy(path));
                self.sidebar.update(cx, |view, _| view.set_has_clipboard(true));
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_info(format!("Copied: {}", name), cx);
                });
            }
            ContextMenuAction::Cut(path) => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("item").to_string();
                self.clipboard = Some(ClipboardOperation::Cut(path));
                self.sidebar.update(cx, |view, _| view.set_has_clipboard(true));
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_info(format!("Cut: {}", name), cx);
                });
            }
            ContextMenuAction::Paste => {
                self.paste_from_clipboard(cx);
            }
            ContextMenuAction::Duplicate(path) => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                let parent = path.parent().unwrap_or(&self.current_path);
                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
                
                let new_name = if extension.is_empty() {
                    format!("{} copy", stem)
                } else {
                    format!("{} copy.{}", stem, extension)
                };
                let new_path = parent.join(&new_name);
                
                let result = if path.is_dir() {
                    copy_dir_recursive_async(&path, &new_path)
                } else {
                    fs::copy(&path, &new_path).map(|_| ())
                };
                
                match result {
                    Ok(()) => {
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_success(format!("Duplicated: {}", new_name), cx);
                        });
                        self.refresh_current_directory(cx);
                    }
                    Err(e) => {
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed to duplicate: {}", e), cx);
                        });
                    }
                }
            }
            ContextMenuAction::MoveToTrash(path) => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("item").to_string();
                match trash::delete(&path) {
                    Ok(()) => {
                        self.file_list.update(cx, |view, _| view.inner_mut().set_selected_index(None));
                        self.preview = None;
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_success(format!("Moved to Trash: {}", name), cx);
                        });
                        self.refresh_current_directory(cx);
                    }
                    Err(e) => {
                        self.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed to trash: {}", e), cx);
                        });
                    }
                }
            }
            ContextMenuAction::Compress(path) => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("archive");
                let parent = path.parent().unwrap_or(&self.current_path);
                let archive_name = format!("{}.zip", name);
                let archive_path = parent.join(&archive_name);
                
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_info(format!("Compressing: {}...", name), cx);
                });
                
                let path_clone = path.clone();
                let archive_path_clone = archive_path.clone();
                let name_clone = name.to_string();
                
                cx.spawn(async move |this, cx| {
                    let result = std::thread::spawn(move || {
                        #[cfg(target_os = "macos")]
                        {
                            std::process::Command::new("ditto")
                                .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
                                .arg(&path_clone)
                                .arg(&archive_path_clone)
                                .output()
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            std::process::Command::new("zip")
                                .args(["-r"])
                                .arg(&archive_path_clone)
                                .arg(&path_clone)
                                .output()
                        }
                    }).join();
                    
                    let _ = this.update(cx, |workspace, cx| {
                        match result {
                            Ok(Ok(output)) if output.status.success() => {
                                workspace.toast_manager.update(cx, |toast, cx| {
                                    toast.show_success(format!("Created: {}.zip", name_clone), cx);
                                });
                                workspace.refresh_current_directory(cx);
                            }
                            _ => {
                                workspace.toast_manager.update(cx, |toast, cx| {
                                    toast.show_error("Failed to compress".to_string(), cx);
                                });
                            }
                        }
                    });
                }).detach();
            }
            ContextMenuAction::Share(path) => {
                #[cfg(target_os = "macos")]
                {
                    let script = format!(
                        "tell application \"Finder\" to activate\n\
                         tell application \"System Events\"\n\
                         keystroke \"i\" using {{command down, option down}}\n\
                         end tell"
                    );
                    let _ = std::process::Command::new("open").args(["-R"]).arg(&path).spawn();
                }
            }
            ContextMenuAction::CopyPath(path) => {
                let path_str = path.to_string_lossy().to_string();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(path_str.clone()));
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_success("Path copied to clipboard".to_string(), cx);
                });
            }
            ContextMenuAction::ShowInFinder(path) => {
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open").args(["-R"]).arg(&path).spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("explorer").args(["/select,"]).arg(&path).spawn();
                }
                #[cfg(target_os = "linux")]
                {
                    if let Some(parent) = path.parent() {
                        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                    }
                }
            }
            ContextMenuAction::QuickLook(path) => {
                if !path.is_dir() {
                    let entries = self.cached_entries.clone();
                    let index = entries.iter().position(|e| e.path == path).unwrap_or(0);
                    self.quick_look.update(cx, |view, _| {
                        view.toggle(path, entries, index);
                    });
                    cx.notify();
                }
            }
            ContextMenuAction::AddToFavorites(path) => {
                self.sidebar.update(cx, |view, _| {
                    let _ = view.add_favorite(path.clone());
                });
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("item");
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_success(format!("Added to Favorites: {}", name), cx);
                });
            }
            ContextMenuAction::NewFolder => {
                self.open_dialog(false, cx);
            }
            ContextMenuAction::NewFile => {
                self.open_dialog(true, cx);
            }
        }
    }
    
    fn start_rename(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        self.dialog_state = DialogState::Rename { path, name: name.clone() };
        
        let input_state = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.content = name.clone().into();
            state.select_on_focus = true;
            state
        });
        
        cx.subscribe(&input_state, |workspace: &mut Workspace, _input, event: &InputEvent, cx| {
            match event {
                InputEvent::Enter => {
                    workspace.submit_rename(cx);
                }
                _ => {}
            }
        }).detach();
        
        self.dialog_input = Some(input_state);
        self.should_focus_dialog_input = true;
        cx.notify();
    }
    
    fn submit_rename(&mut self, cx: &mut Context<Self>) {
        let (old_path, new_name) = match &self.dialog_state {
            DialogState::Rename { path, .. } => {
                let new_name = if let Some(input) = &self.dialog_input {
                    input.read(cx).content.to_string()
                } else {
                    return;
                };
                (path.clone(), new_name)
            }
            _ => return,
        };
        
        if new_name.is_empty() {
            return;
        }
        
        let new_path = old_path.parent().unwrap_or(&self.current_path).join(&new_name);
        
        match fs::rename(&old_path, &new_path) {
            Ok(()) => {
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_success(format!("Renamed to: {}", new_name), cx);
                });
                self.refresh_current_directory(cx);
            }
            Err(e) => {
                self.toast_manager.update(cx, |toast, cx| {
                    toast.show_error(format!("Failed to rename: {}", e), cx);
                });
            }
        }
        
        self.dialog_state = DialogState::None;
        self.dialog_input = None;
        cx.notify();
    }
    
    fn refresh_current_directory(&mut self, cx: &mut Context<Self>) {
        let path = self.current_path.clone();
        self.navigate_to(path, cx);
    }

    fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(clipboard_op) = self.clipboard.take() else {
            return;
        };

        let (source_path, is_move) = match clipboard_op {
            ClipboardOperation::Copy(path) => (path, false),
            ClipboardOperation::Cut(path) => (path, true),
        };

        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let dest_path = self.current_path.join(&file_name);

        // Don't paste into the same location
        if source_path.parent() == Some(&self.current_path) && !is_move {
            self.clipboard = Some(ClipboardOperation::Copy(source_path));
            return;
        }

        // Show "in progress" toast
        let action = if is_move { "Moving" } else { "Copying" };
        self.toast_manager.update(cx, |toast, cx| {
            toast.show_info(format!("{}: {}...", action, file_name), cx);
        });

        // Clear clipboard UI immediately
        self.sidebar.update(cx, |view, _| {
            view.set_has_clipboard(false);
        });

        // Run file operation in background
        let source = source_path.clone();
        let dest = dest_path.clone();
        let name = file_name.clone();
        
        cx.spawn(async move |this, cx| {
            // Perform the copy/move in background
            let result = std::thread::spawn(move || {
                if source.is_dir() {
                    copy_dir_recursive_async(&source, &dest)
                } else {
                    fs::copy(&source, &dest).map(|_| ())
                }
            }).join().unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::Other, "Thread panic")));

            let _ = this.update(cx, |workspace, cx| {
                match result {
                    Ok(()) => {
                        if is_move {
                            // Delete source after successful copy
                            let _ = if source_path.is_dir() {
                                fs::remove_dir_all(&source_path)
                            } else {
                                fs::remove_file(&source_path)
                            };
                            workspace.toast_manager.update(cx, |toast, cx| {
                                toast.show_success(format!("Moved: {}", name), cx);
                            });
                        } else {
                            workspace.toast_manager.update(cx, |toast, cx| {
                                toast.show_success(format!("Copied: {}", name), cx);
                            });
                        }
                        workspace.refresh_current_directory(cx);
                    }
                    Err(e) => {
                        workspace.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed: {}", e), cx);
                        });
                        // Restore clipboard on failure
                        if is_move {
                            workspace.clipboard = Some(ClipboardOperation::Cut(source_path.clone()));
                        } else {
                            workspace.clipboard = Some(ClipboardOperation::Copy(source_path.clone()));
                        }
                        workspace.sidebar.update(cx, |view, _| {
                            view.set_has_clipboard(true);
                        });
                    }
                }
            });
        }).detach();
    }

    fn load_destination_entries(&mut self, cx: &mut Context<Self>) {
        let path = self.dest_path.clone();
        // Always show all files in destination pane for copy/move operations
        let show_hidden = true;
        
        cx.spawn(async move |this, cx| {
            let entries = std::thread::spawn(move || {
                let mut entries = Vec::new();
                if let Ok(read_dir) = fs::read_dir(&path) {
                    for entry in read_dir.flatten() {
                        let entry_path = entry.path();
                        let name = entry_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        
                        if !show_hidden && name.starts_with('.') {
                            continue;
                        }
                        
                        let is_dir = entry_path.is_dir();
                        let metadata = entry_path.metadata().ok();
                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified = metadata.as_ref()
                            .and_then(|m| m.modified().ok())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        
                        let file_type = if is_dir {
                            crate::models::FileType::Directory
                        } else {
                            crate::models::FileType::RegularFile
                        };
                        
                        let icon_key = if is_dir {
                            crate::models::IconKey::Directory
                        } else {
                            entry_path.extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| crate::models::IconKey::Extension(ext.to_lowercase()))
                                .unwrap_or(crate::models::IconKey::GenericFile)
                        };
                        
                        entries.push(crate::models::FileEntry {
                            name,
                            path: entry_path,
                            is_dir,
                            size,
                            modified,
                            file_type,
                            icon_key,
                            linux_permissions: None,
                            sync_status: crate::models::CloudSyncStatus::None,
                        });
                    }
                }
                entries.sort_by(|a, b| {
                    match (a.is_dir, b.is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    }
                });
                entries
            }).join().unwrap_or_default();
            
            let _ = this.update(cx, |workspace, cx| {
                workspace.dest_entries = entries;
                cx.notify();
            });
        }).detach();
    }

    fn navigate_dest_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.dest_path = path;
        self.load_destination_entries(cx);
    }

    fn paste_to_destination(&mut self, cx: &mut Context<Self>) {
        let Some(clipboard_op) = self.clipboard.take() else {
            return;
        };

        let (source_path, is_move) = match clipboard_op {
            ClipboardOperation::Copy(path) => (path, false),
            ClipboardOperation::Cut(path) => (path, true),
        };

        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let dest_path = self.dest_path.join(&file_name);

        // Exit copy/move mode
        self.copy_move_mode = false;

        // Show "in progress" toast
        let action = if is_move { "Moving" } else { "Copying" };
        self.toast_manager.update(cx, |toast, cx| {
            toast.show_info(format!("{}: {}...", action, file_name), cx);
        });

        cx.notify();

        let source = source_path.clone();
        let dest = dest_path.clone();
        let name = file_name.clone();
        
        cx.spawn(async move |this, cx| {
            let result = std::thread::spawn(move || {
                if source.is_dir() {
                    copy_dir_recursive_async(&source, &dest)
                } else {
                    fs::copy(&source, &dest).map(|_| ())
                }
            }).join().unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::Other, "Thread panic")));

            let _ = this.update(cx, |workspace, cx| {
                match result {
                    Ok(()) => {
                        if is_move {
                            let _ = if source_path.is_dir() {
                                fs::remove_dir_all(&source_path)
                            } else {
                                fs::remove_file(&source_path)
                            };
                            workspace.toast_manager.update(cx, |toast, cx| {
                                toast.show_success(format!("Moved: {}", name), cx);
                            });
                        } else {
                            workspace.toast_manager.update(cx, |toast, cx| {
                                toast.show_success(format!("Copied: {}", name), cx);
                            });
                        }
                        workspace.refresh_current_directory(cx);
                    }
                    Err(e) => {
                        workspace.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed: {}", e), cx);
                        });
                    }
                }
            });
        }).detach();
    }

    fn cancel_copy_move_mode(&mut self, cx: &mut Context<Self>) {
        self.copy_move_mode = false;
        self.clipboard = None;
        cx.notify();
    }

    fn empty_trash(&mut self, cx: &mut Context<Self>) {
        self.toast_manager.update(cx, |toast, cx| {
            toast.show_info("Emptying trash...".to_string(), cx);
        });
        
        cx.spawn(async move |this, cx| {
            let result = std::thread::spawn(|| {
                crate::models::empty_trash()
            }).join().unwrap_or_else(|_| Err("Thread panic".to_string()));
            
            let _ = this.update(cx, |workspace, cx| {
                match result {
                    Ok(()) => {
                        workspace.toast_manager.update(cx, |toast, cx| {
                            toast.show_success("Trash emptied".to_string(), cx);
                        });
                        
                        if crate::models::is_trash_path(&workspace.current_path) {
                            workspace.file_list.update(cx, |list, cx| {
                                list.inner_mut().set_entries(Vec::new());
                                cx.notify();
                            });
                            workspace.grid_view.update(cx, |grid, cx| {
                                grid.inner_mut().set_entries(Vec::new());
                                cx.notify();
                            });
                            workspace.status_bar.update(cx, |status, cx| {
                                status.update_from_entries(&[], None, cx);
                            });
                        }
                        
                        workspace.refresh_current_directory(cx);
                    }
                    Err(e) => {
                        workspace.toast_manager.update(cx, |toast, cx| {
                            toast.show_error(format!("Failed: {}", e), cx);
                        });
                    }
                }
            });
        }).detach();
    }
    
    fn create_new_file(&mut self, name: &str, cx: &mut Context<Self>) {
        if name.is_empty() {
            return;
        }
        
        let file_path = self.current_path.join(name);
        if let Err(e) = fs::File::create(&file_path) {
            eprintln!("Failed to create file: {}", e);
            return;
        }
        
        self.dialog_state = DialogState::None;
        self.refresh_current_directory(cx);
    }
    
    fn create_new_folder(&mut self, name: &str, cx: &mut Context<Self>) {
        if name.is_empty() {
            return;
        }
        
        let folder_path = self.current_path.join(name);
        if let Err(e) = fs::create_dir(&folder_path) {
            eprintln!("Failed to create folder: {}", e);
            return;
        }
        
        self.dialog_state = DialogState::None;
        self.refresh_current_directory(cx);
    }
    
    fn cancel_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog_state = DialogState::None;
        self.dialog_input = None;
        self.should_focus_dialog_input = false;
        cx.notify();
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
        let show_hidden = self.show_hidden_files;

        self.file_system.update(cx, |fs, _| {
            let op = fs.load_path(path.clone(), SortKey::Name, SortOrder::Ascending, show_hidden);
            let request_id = op.request_id;

            while let Ok(batch) = op.batch_receiver.recv() {
                fs.process_batch(request_id, batch);
            }

            let _ = op.traversal_handle.join();
            fs.finalize_load(request_id, start.elapsed());
        });

        let cloud_manager = self.sidebar.read(cx).sidebar().cloud_manager().clone();
        self.file_system.update(cx, |fs, _| {
            fs.update_sync_status(&cloud_manager);
        });

        let entries = self.file_system.read(cx).entries().to_vec();
        self.cached_entries = entries.clone();
        
        self.search_input.update(cx, |view, cx| {
            view.clear(cx);
        });
        
        self.file_list.update(cx, |view, _| {
            view.inner_mut().set_entries(entries.clone());
        });
        
        self.grid_view.update(cx, |view, _| {
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
        self.current_path = path.clone();
        
        self.sidebar.update(cx, |view, _| {
            view.set_current_directory(path.clone());
        });
        
        self.status_bar.update(cx, |view, cx| {
            view.update_from_entries(&entries, None, cx);
            view.set_current_directory(&path, cx);
        });
        
        // Sync terminal working directory if terminal is open
        if self.is_terminal_open {
            let terminal_path = path.clone();
            self.terminal.update(cx, |terminal, _| {
                terminal.set_working_directory(terminal_path);
            });
        }
        
        cx.notify();
    }

    pub fn navigate_back(&mut self, cx: &mut Context<Self>) {
        if self.path_history.len() > 1 {
            self.path_history.pop();
            if let Some(prev_path) = self.path_history.last().cloned() {
                let start = Instant::now();
                let show_hidden = self.show_hidden_files;

                self.file_system.update(cx, |fs, _| {
                    let op =
                        fs.load_path(prev_path.clone(), SortKey::Name, SortOrder::Ascending, show_hidden);
                    let request_id = op.request_id;

                    while let Ok(batch) = op.batch_receiver.recv() {
                        fs.process_batch(request_id, batch);
                    }

                    let _ = op.traversal_handle.join();
                    fs.finalize_load(request_id, start.elapsed());
                });

                let entries = self.file_system.read(cx).entries().to_vec();
                self.cached_entries = entries.clone();
                
                self.search_input.update(cx, |view, cx| {
                    view.clear(cx);
                });
                
                self.file_list.update(cx, |view, _| {
                    view.inner_mut().set_entries(entries.clone());
                });
                
                self.grid_view.update(cx, |view, _| {
                    view.inner_mut().set_entries(entries.clone());
                });
                
                // Re-inject paths into search engine
                self.search_engine.update(cx, |engine, _| {
                    engine.clear();
                    for entry in &entries {
                        engine.inject(entry.path.clone());
                    }
                });

                self.current_path = prev_path.clone();
                
                self.sidebar.update(cx, |view, _| {
                    view.set_current_directory(prev_path.clone());
                });
                
                self.status_bar.update(cx, |view, cx| {
                    view.update_from_entries(&entries, None, cx);
                    view.set_current_directory(&prev_path, cx);
                });
                
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
        
        // Start or stop the terminal based on visibility
        if self.is_terminal_open {
            // Sync working directory and start terminal
            let current_path = self.current_path.clone();
            self.terminal.update(cx, |terminal, _| {
                terminal.set_working_directory(current_path);
                terminal.set_visible(true);
                if !terminal.is_running() {
                    let _ = terminal.start();
                }
            });
        } else {
            // Hide terminal (but keep it running for quick toggle)
            self.terminal.update(cx, |terminal, _| {
                terminal.set_visible(false);
            });
        }
        
        self.status_bar.update(cx, |view, cx| {
            view.set_terminal_open(self.is_terminal_open, cx);
        });
        cx.notify();
    }

    pub fn toggle_theme_picker(&mut self, cx: &mut Context<Self>) {
        self.theme_picker.update(cx, |picker, cx| {
            picker.toggle(cx);
        });
        cx.notify();
    }

    pub fn set_theme(&mut self, theme_id: ThemeId, cx: &mut Context<Self>) {
        self.current_theme_id = theme_id;
        crate::models::set_current_theme(theme_id);
        
        let mut settings = GlobalSettings::load();
        settings.theme_id = theme_id;
        let _ = settings.save();
        cx.notify();
    }

    pub fn toggle_view_mode(&mut self, cx: &mut Context<Self>) {
        // Preserve selection when switching views
        let selected_index = match self.view_mode {
            ViewMode::List | ViewMode::Details => {
                self.file_list.read(cx).inner().selected_index()
            }
            ViewMode::Grid => {
                self.grid_view.read(cx).inner().selected_index()
            }
        };

        // Toggle view mode
        self.view_mode = match self.view_mode {
            ViewMode::List | ViewMode::Details => ViewMode::Grid,
            ViewMode::Grid => ViewMode::List,
        };

        // Apply selection to new view
        match self.view_mode {
            ViewMode::List | ViewMode::Details => {
                self.file_list.update(cx, |view, _| {
                    view.inner_mut().set_selected_index(selected_index);
                });
            }
            ViewMode::Grid => {
                self.grid_view.update(cx, |view, _| {
                    view.inner_mut().set_selected_index(selected_index);
                });
            }
        }

        self.save_settings();

        self.status_bar.update(cx, |view, cx| {
            view.set_view_mode(self.view_mode, cx);
        });

        cx.notify();
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        if self.view_mode != mode {
            self.view_mode = mode;
            self.save_settings();
            self.status_bar.update(cx, |view, cx| {
                view.set_view_mode(mode, cx);
            });
            cx.notify();
        }
    }

    /// Get the currently selected file entry
    fn get_selected_entry(&self, cx: &mut Context<Self>) -> Option<crate::models::FileEntry> {
        match self.view_mode {
            ViewMode::List | ViewMode::Details => {
                let file_list = self.file_list.read(cx);
                let idx = file_list.inner().selected_index();
                idx.and_then(|i| file_list.inner().get_display_entry(i).cloned())
            }
            ViewMode::Grid => {
                let grid_view = self.grid_view.read(cx);
                let idx = grid_view.inner().selected_index();
                idx.and_then(|i| self.cached_entries.get(i).cloned())
            }
        }
    }

    /// Update preview panel based on current selection
    fn update_preview_for_selection(&mut self, cx: &mut Context<Self>) {
        let selected_entry = self.get_selected_entry(cx);

        match selected_entry {
            Some(entry) if !entry.is_dir => {
                // Show preview for files
                if self.preview.is_none() {
                    self.preview = Some(cx.new(|cx| PreviewView::new(cx)));
                }
                if let Some(ref preview) = self.preview {
                    preview.update(cx, |view, _| {
                        view.load_file(&entry.path);
                    });
                }
                cx.notify();
            }
            _ => {
                // Hide preview for directories or no selection
                if self.preview.is_some() {
                    self.preview = None;
                    cx.notify();
                }
            }
        }
    }

    fn save_settings(&self) {
        let mut settings = GlobalSettings::load();
        settings.view_mode = self.view_mode;
        settings.show_hidden_files = self.show_hidden_files;
        let _ = settings.save();
    }

    
    /// Handle Cmd+T - New Tab (opens new window for now)
    fn handle_new_tab(&mut self, _: &NewTab, _window: &mut Window, cx: &mut Context<Self>) {
        // For now, open a new window since full tab support isn't implemented
        let path = self.current_path.clone();
        cx.defer(move |cx| {
            if cx.has_global::<WindowManager>() {
                cx.update_global::<WindowManager, _>(|manager, cx| {
                    manager.open_window(path, cx);
                });
            }
        });
    }

    /// Handle Cmd+W - Close Tab/Window
    fn handle_close_tab(&mut self, _: &CloseTab, window: &mut Window, _cx: &mut Context<Self>) {
        // Close the current window
        window.remove_window();
    }

    /// Handle Cmd+` - Toggle Terminal
    fn handle_toggle_terminal(&mut self, _: &ToggleTerminal, _window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_terminal(cx);
    }

    /// Handle Cmd+F - Focus Search
    fn handle_focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |view, _cx| {
            view.focus(window);
        });
    }

    /// Handle Cmd+N - New Window
    fn handle_new_window(&mut self, _: &NewWindow, _window: &mut Window, cx: &mut Context<Self>) {
        let path = self.current_path.clone();
        cx.defer(move |cx| {
            if cx.has_global::<WindowManager>() {
                cx.update_global::<WindowManager, _>(|manager, cx| {
                    manager.open_window(path, cx);
                });
            }
        });
    }

    /// Handle Space - Toggle Quick Look
    fn handle_quick_look_toggle(&mut self, _: &QuickLookToggle, _window: &mut Window, cx: &mut Context<Self>) {
        let selected_entry = match self.view_mode {
            ViewMode::List | ViewMode::Details => {
                let idx = self.file_list.read(cx).inner().selected_index();
                idx.and_then(|i| self.cached_entries.get(i).cloned())
            }
            ViewMode::Grid => {
                let idx = self.grid_view.read(cx).inner().selected_index();
                idx.and_then(|i| self.cached_entries.get(i).cloned())
            }
        };

        if let Some(entry) = selected_entry {
            if !entry.is_dir {
                let entries = self.cached_entries.clone();
                let index = self.cached_entries.iter().position(|e| e.path == entry.path).unwrap_or(0);
                self.quick_look.update(cx, |view, _| {
                    view.toggle(entry.path, entries, index);
                });
                cx.notify();
            }
        }
    }

    fn render_breadcrumbs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let text_light = theme.text_primary;
        let accent_primary = theme.accent_primary;
        let accent_secondary = theme.accent_secondary;
        let hover_bg = theme.bg_hover;

        let mut parts: Vec<(String, PathBuf)> = Vec::new();
        let mut current = Some(self.current_path.as_path());

        while let Some(path) = current {
            let name = if path.parent().is_none() {
                "/".to_string()
            } else {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()
            };
            if !name.is_empty() {
                parts.push((name, path.to_path_buf()));
            }
            current = path.parent();
            if parts.len() >= 5 {
                break;
            }
        }

        parts.reverse();

        div()
            .flex()
            .items_center()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .children(parts.into_iter().enumerate().map(|(i, (name, path))| {
                div()
                    .flex()
                    .items_center()
                    .when(i > 0, |s| {
                        s.child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(14.0))
                                .text_color(accent_secondary)
                                .mx_1(),
                        )
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("breadcrumb-{}", i)))
                            .px(px(crate::models::toolbar::BREADCRUMB_PADDING))
                            .py_0p5()
                            .rounded_sm()
                            .text_color(text_light)
                            .cursor_pointer()
                            .hover(|s| s.text_color(accent_primary).bg(hover_bg))
                            .on_mouse_down(MouseButton::Left, {
                                let nav_path = path.clone();
                                cx.listener(move |view, _, _, cx| {
                                    view.navigate_to(nav_path.clone(), cx);
                                })
                            })
                            .child(name)
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.should_focus_dialog_input {
            if let Some(input) = &self.dialog_input {
                window.focus(&input.read(cx).focus_handle(cx));
            }
            self.should_focus_dialog_input = false;
        }
        
        let theme = theme_colors();
        let bg_dark = theme.bg_secondary;
        let bg_darker = theme.bg_void;
        let border_color = theme.border_default;
        let text_gray = theme.text_muted;
        let hover_bg = theme.bg_hover;
        let blue_active = theme.accent_primary;

        let is_terminal_open = self.is_terminal_open;
        let can_go_back = self.path_history.len() > 1;

        let current = current_theme();
        let content_bg = current.content_background();
        
        div()
            .id("workspace")
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_new_tab))
            .on_action(cx.listener(Self::handle_close_tab))
            .on_action(cx.listener(Self::handle_toggle_terminal))
            .on_action(cx.listener(Self::handle_focus_search))
            .on_action(cx.listener(Self::handle_new_window))
            .on_action(cx.listener(Self::handle_quick_look_toggle))
            .on_mouse_up(MouseButton::Left, cx.listener(|view, _, _, cx| {
                if view.is_resizing_terminal {
                    view.is_resizing_terminal = false;
                    cx.notify();
                }
                if view.is_resizing_preview {
                    view.is_resizing_preview = false;
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|view, event: &gpui::MouseMoveEvent, window, cx| {
                if view.is_resizing_terminal {
                    let bounds = window.bounds();
                    let mouse_y = event.position.y;
                    let window_height = bounds.size.height;
                    let new_height = f32::from(window_height) - f32::from(mouse_y) - 30.0;
                    view.terminal_height = new_height.clamp(150.0, 600.0);
                    cx.notify();
                }
                if view.is_resizing_preview {
                    let bounds = window.bounds();
                    let mouse_x = event.position.x;
                    let window_width = bounds.size.width;
                    // Preview is on the right, so width = window_width - mouse_x
                    let new_width = f32::from(window_width) - f32::from(mouse_x);
                    view.preview_width = new_width.clamp(200.0, 600.0);
                    cx.notify();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            // Apply layered background with gradient effect
            .bg(content_bg.base_color)
            .text_color(theme.text_primary)
            .font_family(".SystemUIFont")
            // Titlebar - draggable for window movement and double-click to zoom
            .child(
                div()
                    .id("titlebar")
                    .h(px(52.0))
                    .bg(bg_darker)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_5()
                    .py_2()
                    .border_b_1()
                    .border_color(border_color)
                    .on_click(|event, window, _cx| {
                        if event.click_count() == 2 {
                            window.titlebar_double_click();
                        }
                    })
                    // Left side - leave space for traffic lights on macOS
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .pl(px(70.0))
                            .child(
                                svg()
                                    .path("assets/icons/hard-drive.svg")
                                    .size(px(16.0))
                                    .text_color(theme.accent_primary),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child("Nexus Explorer"),
                            ),
                    )
                    // Center - search input
                    .child(
                        div()
                            .relative()
                            .w_1_3()
                            .max_w(px(500.0))
                            .child(self.search_input.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .id("theme-picker-btn")
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .hover(|h| h.bg(hover_bg))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _event, _window, cx| {
                                            view.toggle_theme_picker(cx);
                                        }),
                                    )
                                    .child(
                                        svg()
                                            .path("assets/icons/sparkles.svg")
                                            .size(px(14.0))
                                            .text_color(theme.accent_primary),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.text_secondary)
                                            .child("Themes"),
                                    ),
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
                            .w(px(crate::models::sidebar::WIDTH))
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
                            // Toolbar with RPG styling - 52px height, themed dividers
                            .child(
                                div()
                                    .h(px(crate::models::toolbar::HEIGHT))
                                    .bg(bg_dark)
                                    .border_b_1()
                                    .border_color(border_color)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(crate::models::toolbar::PADDING_X))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(crate::models::toolbar::BUTTON_GAP))
                                            .child(
                                                // Back button with 36px size
                                                div()
                                                    .id("back-btn")
                                                    .size(px(crate::models::toolbar::BUTTON_SIZE))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
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
                                                            .size(px(18.0))
                                                            .text_color(text_gray),
                                                    ),
                                            )
                                            // Themed divider between toolbar sections
                                            .child(
                                                div()
                                                    .h(px(20.0))
                                                    .w(px(1.0))
                                                    .bg(theme.border_subtle)
                                                    .mx(px(crate::models::toolbar::BUTTON_GAP)),
                                            )
                                            .child(self.render_breadcrumbs(cx)),
                                    )
                                    // Right side toolbar buttons with RPG styling
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(crate::models::toolbar::BUTTON_GAP))
                                            .child(
                                                // Terminal toggle button - 36px
                                                div()
                                                    .id("terminal-btn")
                                                    .size(px(crate::models::toolbar::BUTTON_SIZE))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .when(is_terminal_open, |s| {
                                                        s.bg(theme.bg_selected)
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
                                                            .size(px(18.0))
                                                            .text_color(if is_terminal_open { theme.accent_primary } else { text_gray }),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .h(px(20.0))
                                                    .w(px(1.0))
                                                    .bg(theme.border_subtle)
                                                    .mx(px(crate::models::toolbar::BUTTON_GAP)),
                                            )
                                            .child(
                                                // Copy button - 36px
                                                div()
                                                    .id("copy-btn")
                                                    .size(px(crate::models::toolbar::BUTTON_SIZE))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .hover(|h| h.bg(hover_bg))
                                                    .child(
                                                        svg()
                                                            .path("assets/icons/copy.svg")
                                                            .size(px(18.0))
                                                            .text_color(text_gray),
                                                    ),
                                            )
                                            .child(
                                                // Trash button - 36px
                                                div()
                                                    .id("trash-btn")
                                                    .size(px(crate::models::toolbar::BUTTON_SIZE))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
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
                                            // Empty Trash button - only shown when in Trash folder
                                            .when(crate::models::is_trash_path(&self.current_path), |toolbar| {
                                                toolbar.child(
                                                    div()
                                                        .id("empty-trash-btn")
                                                        .px_3()
                                                        .py(px(6.0))
                                                        .bg(gpui::rgb(0xda3633))
                                                        .text_color(gpui::rgb(0xffffff))
                                                        .rounded_md()
                                                        .text_xs()
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .cursor_pointer()
                                                        .hover(|h| h.bg(gpui::rgb(0xb62324)))
                                                        .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                            view.empty_trash(cx);
                                                        }))
                                                        .child("Empty Trash")
                                                )
                                            })
                                            .child(
                                                div()
                                                    .h(px(16.0))
                                                    .w(px(1.0))
                                                    .bg(theme.border_subtle)
                                                    .mx_2(),
                                            )
                                            .child({
                                                let is_grid = matches!(self.view_mode, ViewMode::Grid);
                                                div()
                                                    .flex()
                                                    .bg(theme.bg_tertiary)
                                                    .rounded_lg()
                                                    .p_0p5()
                                                    .child(
                                                        div()
                                                            .id("grid-view-btn")
                                                            .p_1()
                                                            .rounded_md()
                                                            .cursor_pointer()
                                                            .when(is_grid, |s| s.bg(theme.bg_hover))
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(|view, _event, _window, cx| {
                                                                    view.set_view_mode(ViewMode::Grid, cx);
                                                                }),
                                                            )
                                                            .child(
                                                                svg()
                                                                    .path("assets/icons/grid-2x2.svg")
                                                                    .size(px(14.0))
                                                                    .text_color(if is_grid { theme.text_primary } else { text_gray }),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .id("list-view-btn")
                                                            .p_1()
                                                            .rounded_md()
                                                            .cursor_pointer()
                                                            .when(!is_grid, |s| s.bg(theme.bg_hover))
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(|view, _event, _window, cx| {
                                                                    view.set_view_mode(ViewMode::List, cx);
                                                                }),
                                                            )
                                                            .child(
                                                                svg()
                                                                    .path("assets/icons/list.svg")
                                                                    .size(px(14.0))
                                                                    .text_color(if !is_grid { theme.text_primary } else { text_gray }),
                                                            ),
                                                    )
                                            }),
                                    ),
                            )
                            .child({
                                let is_grid = matches!(self.view_mode, ViewMode::Grid);
                                div()
                                    .flex_1()
                                    .bg(bg_darker)
                                    .overflow_hidden()
                                    .min_h(px(100.0))
                                    .when(self.copy_move_mode, |d| d.opacity(0.5))
                                    .when(is_grid, |this| this.child(self.grid_view.clone()))
                                    .when(!is_grid, |this| this.child(self.file_list.clone()))
                            })
                            .when(is_terminal_open, |this| {
                                let terminal_height = self.terminal_height;
                                let handle_color = theme.border_default;
                                this
                                    .child(
                                        div()
                                            .id("terminal-resize-handle")
                                            .w_full()
                                            .h(px(6.0))
                                            .cursor_row_resize()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(bg_dark)
                                            .border_t_1()
                                            .border_color(handle_color)
                                            .hover(|h| h.bg(theme.bg_hover))
                                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                view.is_resizing_terminal = true;
                                                cx.notify();
                                            }))
                                            .child(
                                                div()
                                                    .w(px(40.0))
                                                    .h(px(3.0))
                                                    .rounded_full()
                                                    .bg(handle_color),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .h(px(terminal_height))
                                            .min_h(px(150.0))
                                            .max_h(px(600.0))
                                            .child(self.terminal.clone()),
                                    )
                            }),
                    )
                    .when(self.copy_move_mode, |this| {
                        this.child(self.render_destination_pane(cx))
                    })
                    .when(!self.copy_move_mode, |this| {
                        this.children(self.preview.clone().map(|preview| {
                            let preview_width = self.preview_width;
                            let handle_color = border_color;
                            div()
                                .flex()
                                .h_full()
                                .child(
                                    div()
                                        .id("preview-resize-handle")
                                        .w(px(6.0))
                                        .h_full()
                                        .cursor_col_resize()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(bg_dark)
                                        .border_l_1()
                                        .border_color(handle_color)
                                        .hover(|h| h.bg(theme.bg_hover))
                                        .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                            view.is_resizing_preview = true;
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .w(px(3.0))
                                                .h(px(40.0))
                                                .rounded_full()
                                                .bg(handle_color),
                                        ),
                                )
                                // Preview content
                                .child(
                                    div()
                                        .w(px(preview_width))
                                        .min_w(px(200.0))
                                        .max_w(px(600.0))
                                        .h_full()
                                        .bg(bg_dark)
                                        .flex()
                                        .flex_col()
                                        .child(preview),
                                )
                        }))
                    }),
            )
            // Status bar at the bottom
            .child(self.status_bar.clone())
            .when(!matches!(self.dialog_state, DialogState::None), |this| {
                this.child(self.render_dialog_overlay(cx))
            })
            // Theme picker overlay
            .child(self.theme_picker.clone())
            // Quick Look overlay
            .child(self.quick_look.clone())
            // Toast notifications
            .child(self.toast_manager.clone())
    }
}

impl Workspace {
    fn render_destination_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_dark = theme.bg_secondary;
        let bg_darker = theme.bg_primary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;
        let hover_bg = theme.bg_hover;
        let folder_color = theme.accent_primary;
        
        div()
            .flex()
            .flex_1()
            .h_full()
            .border_l_1()
            .border_color(border_color)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(bg_darker)
                    .flex()
                    .flex_col()
                    // Toolbar matching main pane
                    .child(
                        div()
                            .h(px(crate::models::toolbar::HEIGHT))
                            .bg(bg_dark)
                            .border_b_1()
                            .border_color(border_color)
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(crate::models::toolbar::PADDING_X))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(crate::models::toolbar::BUTTON_GAP))
                                    .child(
                                        div()
                                            .id("dest-up-btn")
                                            .size(px(crate::models::toolbar::BUTTON_SIZE))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .hover(|h| h.bg(hover_bg))
                                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                if let Some(parent) = view.dest_path.parent() {
                                                    let parent_path = parent.to_path_buf();
                                                    view.navigate_dest_to(parent_path, cx);
                                                }
                                            }))
                                            .child(
                                                svg()
                                                    .path("assets/icons/arrow-up.svg")
                                                    .size(px(18.0))
                                                    .text_color(text_muted),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .h(px(20.0))
                                            .w(px(1.0))
                                            .bg(theme.border_subtle)
                                            .mx(px(crate::models::toolbar::BUTTON_GAP)),
                                    )
                                    // Breadcrumb navigation
                                    .child(self.render_dest_breadcrumbs(cx))
                            )
                            // Right side - action buttons
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .id("paste-here-btn")
                                            .px_3()
                                            .py(px(6.0))
                                            .bg(theme.accent_primary)
                                            .text_color(theme.bg_primary)
                                            .rounded_md()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .cursor_pointer()
                                            .hover(|h| h.bg(theme.accent_secondary))
                                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                view.paste_to_destination(cx);
                                            }))
                                            .child("PASTE HERE"),
                                    )
                                    .child(
                                        div()
                                            .id("cancel-copy-btn")
                                            .px_3()
                                            .py(px(6.0))
                                            .text_color(text_muted)
                                            .rounded_md()
                                            .text_xs()
                                            .cursor_pointer()
                                            .hover(|h| h.bg(theme.bg_hover).text_color(text_primary))
                                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                view.cancel_copy_move_mode(cx);
                                            }))
                                            .child("CANCEL"),
                                    )
                            )
                    )
                    // File list with scroll
                    .child(
                        div()
                            .id("dest-file-list")
                            .flex_1()
                            .bg(bg_darker)
                            .overflow_y_scroll()
                            .children(self.dest_entries.iter().map(|entry| {
                                let entry_path = entry.path.clone();
                                let is_dir = entry.is_dir;
                                let name = entry.name.clone();
                                
                                div()
                                    .id(SharedString::from(format!("dest-{}", entry.name)))
                                    .h(px(40.0))
                                    .w_full()
                                    .px(px(16.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .cursor_pointer()
                                    .border_b_1()
                                    .border_color(theme.border_subtle)
                                    .hover(|h| h.bg(hover_bg))
                                    .when(is_dir, |d| {
                                        d.on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                                            view.navigate_dest_to(entry_path.clone(), cx);
                                        }))
                                    })
                                    .child(
                                        svg()
                                            .path(if is_dir { "assets/icons/folder.svg" } else { "assets/icons/file.svg" })
                                            .size(px(20.0))
                                            .text_color(if is_dir { folder_color } else { text_muted }),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(text_primary)
                                            .child(name),
                                    )
                            }))
                    )
            )
    }

    fn render_dest_breadcrumbs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let text_muted = theme.text_muted;
        let text_primary = theme.text_primary;
        
        let mut parts: Vec<(String, PathBuf)> = Vec::new();
        let mut current = Some(self.dest_path.as_path());
        
        while let Some(p) = current {
            let name = if p.parent().is_none() {
                "/".to_string()
            } else {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()
            };
            if !name.is_empty() {
                parts.push((name, p.to_path_buf()));
            }
            current = p.parent();
        }
        parts.reverse();
        
        div()
            .flex()
            .items_center()
            .gap_1()
            .overflow_x_hidden()
            .children(parts.into_iter().enumerate().map(|(i, (name, path))| {
                let is_last = i == 0;
                div()
                    .flex()
                    .items_center()
                    .when(i > 0, |d| {
                        d.child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(12.0))
                                .text_color(text_muted)
                                .mx_1()
                        )
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("dest-crumb-{}", i)))
                            .px(px(crate::models::toolbar::BREADCRUMB_PADDING))
                            .py_0p5()
                            .rounded_sm()
                            .cursor_pointer()
                            .text_sm()
                            .text_color(text_muted)
                            .hover(|h| h.bg(theme.bg_hover).text_color(text_primary))
                            .on_mouse_down(MouseButton::Left, {
                                let nav_path = path.clone();
                                cx.listener(move |view, _, _, cx| {
                                    view.navigate_dest_to(nav_path.clone(), cx);
                                })
                            })
                            .child(name)
                    )
            }))
    }
}

impl Workspace {
    fn render_dialog_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let overlay_bg = gpui::rgba(0x00000099);
        let dialog_bg = theme.bg_secondary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;
        let accent = theme.accent_primary;
        let hover_bg = theme.bg_hover;
        
        let (title, placeholder, icon_path, button_text) = match &self.dialog_state {
            DialogState::NewFile { .. } => ("New File", "Enter file name...", "assets/icons/file-plus.svg", "Create"),
            DialogState::NewFolder { .. } => ("New Folder", "Enter folder name...", "assets/icons/folder-plus.svg", "Create"),
            DialogState::Rename { .. } => ("Rename", "Enter new name...", "assets/icons/pen.svg", "Rename"),
            DialogState::None => ("", "", "", ""),
        };
        
        let is_rename = matches!(self.dialog_state, DialogState::Rename { .. });
        
        let input_element: Option<Input> = self.dialog_input.as_ref().map(|input_state| {
            Input::new(input_state).placeholder(placeholder)
        });
        
        div()
            .id("dialog-overlay")
            .absolute()
            .inset_0()
            .bg(overlay_bg)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                view.cancel_dialog(cx);
            }))
            .child(
                div()
                    .id("dialog-content")
                    .occlude()
                    .w(px(400.0))
                    .bg(dialog_bg)
                    .rounded_lg()
                    .border_1()
                    .border_color(border_color)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        div()
                            .p_4()
                            .border_b_1()
                            .border_color(border_color)
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                svg()
                                    .path(SharedString::from(icon_path))
                                    .size(px(20.0))
                                    .text_color(accent),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(text_primary)
                                    .child(title)
                            )
                    )
                    .child(
                        div()
                            .p_4()
                            .child(
                                div()
                                    .w_full()
                                    .children(input_element)
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child(if is_rename { 
                                        "Press Enter to rename, Escape to cancel" 
                                    } else { 
                                        "Press Enter to create, Escape to cancel" 
                                    })
                            )
                    )
                    .child(
                        div()
                            .p_4()
                            .border_t_1()
                            .border_color(border_color)
                            .flex()
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
                                    .text_color(text_muted)
                                    .hover(|h| h.bg(hover_bg).text_color(text_primary))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                        view.cancel_dialog(cx);
                                    }))
                                    .child("Cancel")
                            )
                            .child(
                                div()
                                    .id("submit-btn")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .bg(accent)
                                    .text_color(theme.text_inverse)
                                    .hover(|h| h.opacity(0.9))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, _window, cx| {
                                        if is_rename {
                                            view.submit_rename(cx);
                                        } else {
                                            view.submit_dialog(cx);
                                        }
                                    }))
                                    .child(button_text)
                            )
                    )
            )
    }
}

// Remove duplicate closing brace


