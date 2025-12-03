use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{px, size, App, Bounds, Global, Pixels, WindowBounds, WindowHandle, WindowOptions};
use serde::{Deserialize, Serialize};

use crate::app::Workspace;

static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for application windows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppWindowId(u64);

impl AppWindowId {
    pub fn new() -> Self {
        Self(NEXT_WINDOW_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for AppWindowId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the state of a single window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub id: AppWindowId,
    pub path: PathBuf,
    pub bounds: Option<WindowBoundsState>,
    pub is_active: bool,
}

/// Serializable window bounds for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBoundsState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl WindowBoundsState {
    pub fn from_bounds(bounds: &Bounds<Pixels>) -> Self {
        Self {
            x: bounds.origin.x.into(),
            y: bounds.origin.y.into(),
            width: bounds.size.width.into(),
            height: bounds.size.height.into(),
        }
    }

    pub fn to_bounds(&self) -> Bounds<Pixels> {
        Bounds {
            origin: gpui::Point {
                x: px(self.x),
                y: px(self.y),
            },
            size: gpui::Size {
                width: px(self.width),
                height: px(self.height),
            },
        }
    }
}


/// Manages multiple application windows
pub struct WindowManager {
    /// Map of window IDs to their GPUI window handles
    windows: HashMap<AppWindowId, WindowHandle<Workspace>>,
    /// Map of window IDs to their state
    window_states: HashMap<AppWindowId, WindowState>,
    /// Currently active window
    active_window: Option<AppWindowId>,
    /// Default window size
    default_width: f32,
    default_height: f32,
    /// Offset for cascading new windows
    cascade_offset: f32,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_states: HashMap::new(),
            active_window: None,
            default_width: 1200.0,
            default_height: 800.0,
            cascade_offset: 30.0,
        }
    }

    /// Returns the number of open windows
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Returns whether there are any open windows
    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    /// Returns the active window ID
    pub fn active_window(&self) -> Option<AppWindowId> {
        self.active_window
    }

    /// Sets the active window
    pub fn set_active(&mut self, id: AppWindowId) {
        if self.windows.contains_key(&id) {
            // Update previous active window state
            if let Some(prev_id) = self.active_window {
                if let Some(state) = self.window_states.get_mut(&prev_id) {
                    state.is_active = false;
                }
            }
            // Set new active window
            self.active_window = Some(id);
            if let Some(state) = self.window_states.get_mut(&id) {
                state.is_active = true;
            }
        }
    }

    /// Returns all window IDs
    pub fn window_ids(&self) -> Vec<AppWindowId> {
        self.windows.keys().copied().collect()
    }

    /// Returns the window handle for a given ID
    pub fn get_window(&self, id: AppWindowId) -> Option<&WindowHandle<Workspace>> {
        self.windows.get(&id)
    }

    /// Returns the window state for a given ID
    pub fn get_window_state(&self, id: AppWindowId) -> Option<&WindowState> {
        self.window_states.get(&id)
    }

    /// Opens a new window with the given path
    pub fn open_window(&mut self, path: PathBuf, cx: &mut App) -> Option<AppWindowId> {
        let id = AppWindowId::new();
        
        // Calculate cascaded position for new window
        let offset = self.windows.len() as f32 * self.cascade_offset;
        let bounds = Bounds::centered(
            None,
            size(px(self.default_width), px(self.default_height)),
            cx,
        );
        
        // Offset the bounds for cascading effect
        let cascaded_bounds = Bounds {
            origin: gpui::Point {
                x: bounds.origin.x + px(offset),
                y: bounds.origin.y + px(offset),
            },
            size: bounds.size,
        };
        
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(cascaded_bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: gpui::WindowKind::Normal,
            is_movable: true,
            app_id: Some("file-explorer".to_string()),
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };
        
        let path_clone = path.clone();
        match cx.open_window(window_options, |_window, cx| {
            Workspace::build(path_clone, cx)
        }) {
            Ok(handle) => {
                // Store window state
                let state = WindowState {
                    id,
                    path,
                    bounds: Some(WindowBoundsState::from_bounds(&cascaded_bounds)),
                    is_active: true,
                };
                
                self.windows.insert(id, handle);
                self.window_states.insert(id, state);
                self.set_active(id);
                
                Some(id)
            }
            Err(e) => {
                eprintln!("Failed to open window: {}", e);
                None
            }
        }
    }

    /// Closes a window by ID
    pub fn close_window(&mut self, id: AppWindowId, cx: &mut App) {
        if let Some(handle) = self.windows.remove(&id) {
            self.window_states.remove(&id);
            
            // Update active window if we closed the active one
            if self.active_window == Some(id) {
                self.active_window = self.windows.keys().next().copied();
                if let Some(new_active) = self.active_window {
                    if let Some(state) = self.window_states.get_mut(&new_active) {
                        state.is_active = true;
                    }
                }
            }
            
            // Note: GPUI windows are closed when their handle is dropped
            // The handle is removed from our map, which will drop it
            drop(handle);
        }
    }

    /// Registers an existing window (used for the initial window)
    pub fn register_window(&mut self, handle: WindowHandle<Workspace>, path: PathBuf) -> AppWindowId {
        let id = AppWindowId::new();
        
        let state = WindowState {
            id,
            path,
            bounds: None,
            is_active: true,
        };
        
        self.windows.insert(id, handle);
        self.window_states.insert(id, state);
        self.active_window = Some(id);
        
        id
    }

    /// Updates the stored bounds for a window
    pub fn update_window_bounds(&mut self, id: AppWindowId, bounds: Bounds<gpui::Pixels>) {
        if let Some(state) = self.window_states.get_mut(&id) {
            state.bounds = Some(WindowBoundsState::from_bounds(&bounds));
        }
    }

    /// Updates the stored path for a window
    pub fn update_window_path(&mut self, id: AppWindowId, path: PathBuf) {
        if let Some(state) = self.window_states.get_mut(&id) {
            state.path = path;
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for WindowManager {}


/// Persisted window manager state for save/restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowManagerState {
    pub windows: Vec<WindowState>,
    pub active_window_index: Option<usize>,
}

impl WindowManager {
    /// Saves the current window state to disk
    pub fn save_state(&self) -> std::io::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer");
        
        std::fs::create_dir_all(&config_dir)?;
        
        let state = WindowManagerState {
            windows: self.window_states.values().cloned().collect(),
            active_window_index: self.active_window.and_then(|id| {
                self.window_states.values()
                    .position(|s| s.id == id)
            }),
        };
        
        let config_path = config_dir.join("windows.json");
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        std::fs::write(config_path, json)
    }

    /// Loads window state from disk
    pub fn load_state() -> Option<WindowManagerState> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer")
            .join("windows.json");
        
        if config_path.exists() {
            if let Ok(json) = std::fs::read_to_string(&config_path) {
                if let Ok(state) = serde_json::from_str::<WindowManagerState>(&json) {
                    return Some(state);
                }
            }
        }
        
        None
    }

    /// Restores windows from saved state
    pub fn restore_state(&mut self, state: WindowManagerState, cx: &mut App) {
        for window_state in state.windows {
            // Only restore if the path still exists
            if window_state.path.exists() {
                let id = AppWindowId::new();
                
                // Use saved bounds or default
                let bounds = window_state.bounds
                    .as_ref()
                    .map(|b| b.to_bounds())
                    .unwrap_or_else(|| {
                        Bounds::centered(
                            None,
                            size(px(self.default_width), px(self.default_height)),
                            cx,
                        )
                    });
                
                let window_options = WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: None,
                    focus: false, // Don't focus restored windows initially
                    show: true,
                    kind: gpui::WindowKind::Normal,
                    is_movable: true,
                    app_id: Some("file-explorer".to_string()),
                    window_background: gpui::WindowBackgroundAppearance::Opaque,
                    ..Default::default()
                };
                
                let path = window_state.path.clone();
                if let Ok(handle) = cx.open_window(window_options, |_window, cx| {
                    Workspace::build(path.clone(), cx)
                }) {
                    let new_state = WindowState {
                        id,
                        path,
                        bounds: window_state.bounds,
                        is_active: false,
                    };
                    
                    self.windows.insert(id, handle);
                    self.window_states.insert(id, new_state);
                }
            }
        }
        
        // Set active window
        if let Some(active_idx) = state.active_window_index {
            if let Some(id) = self.window_ids().get(active_idx).copied() {
                self.set_active(id);
            }
        } else if let Some(id) = self.window_ids().first().copied() {
            self.set_active(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_window_id_unique() {
        let id1 = AppWindowId::new();
        let id2 = AppWindowId::new();
        let id3 = AppWindowId::new();
        
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_window_bounds_state_roundtrip() {
        let bounds = Bounds {
            origin: gpui::Point {
                x: px(100.0),
                y: px(200.0),
            },
            size: gpui::Size {
                width: px(800.0),
                height: px(600.0),
            },
        };
        
        let state = WindowBoundsState::from_bounds(&bounds);
        let restored = state.to_bounds();
        
        // Compare using Into<f32> conversion
        let orig_x: f32 = bounds.origin.x.into();
        let orig_y: f32 = bounds.origin.y.into();
        let orig_w: f32 = bounds.size.width.into();
        let orig_h: f32 = bounds.size.height.into();
        
        let rest_x: f32 = restored.origin.x.into();
        let rest_y: f32 = restored.origin.y.into();
        let rest_w: f32 = restored.size.width.into();
        let rest_h: f32 = restored.size.height.into();
        
        assert_eq!(orig_x, rest_x);
        assert_eq!(orig_y, rest_y);
        assert_eq!(orig_w, rest_w);
        assert_eq!(orig_h, rest_h);
    }

    #[test]
    fn test_window_manager_new() {
        let manager = WindowManager::new();
        
        assert_eq!(manager.window_count(), 0);
        assert!(!manager.has_windows());
        assert!(manager.active_window().is_none());
    }

    #[test]
    fn test_window_state_serialization() {
        let state = WindowState {
            id: AppWindowId(42),
            path: PathBuf::from("/home/user"),
            bounds: Some(WindowBoundsState {
                x: 100.0,
                y: 200.0,
                width: 800.0,
                height: 600.0,
            }),
            is_active: true,
        };
        
        let json = serde_json::to_string(&state).expect("Failed to serialize");
        let restored: WindowState = serde_json::from_str(&json).expect("Failed to deserialize");
        
        assert_eq!(state.id, restored.id);
        assert_eq!(state.path, restored.path);
        assert_eq!(state.is_active, restored.is_active);
        
        let bounds = state.bounds.unwrap();
        let restored_bounds = restored.bounds.unwrap();
        assert_eq!(bounds.x, restored_bounds.x);
        assert_eq!(bounds.y, restored_bounds.y);
        assert_eq!(bounds.width, restored_bounds.width);
        assert_eq!(bounds.height, restored_bounds.height);
    }

    #[test]
    fn test_window_manager_state_serialization() {
        let state = WindowManagerState {
            windows: vec![
                WindowState {
                    id: AppWindowId(1),
                    path: PathBuf::from("/home/user"),
                    bounds: None,
                    is_active: true,
                },
                WindowState {
                    id: AppWindowId(2),
                    path: PathBuf::from("/tmp"),
                    bounds: Some(WindowBoundsState {
                        x: 50.0,
                        y: 50.0,
                        width: 1000.0,
                        height: 700.0,
                    }),
                    is_active: false,
                },
            ],
            active_window_index: Some(0),
        };
        
        let json = serde_json::to_string_pretty(&state).expect("Failed to serialize");
        let restored: WindowManagerState = serde_json::from_str(&json).expect("Failed to deserialize");
        
        assert_eq!(state.windows.len(), restored.windows.len());
        assert_eq!(state.active_window_index, restored.active_window_index);
    }
}
