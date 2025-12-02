mod app;
mod io;
mod models;
mod utils;
mod views;

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{px, size, Application, Bounds, WindowBounds, WindowOptions};
use tokio::runtime::Runtime;

use app::Workspace;
use models::{GlobalSettings, IconCache, IconKey};

fn main() {
    let app = Application::new();
    
    app.run(|cx| {
        // Register GlobalSettings as GPUI global state
        cx.set_global(GlobalSettings::default());
        
        // Spawn Tokio runtime on a dedicated thread for I/O operations
        let _tokio_runtime = spawn_tokio_runtime();
        
        // Pre-load default icons into IconCache
        let _icon_cache = preload_default_icons();
        
        // Detect user's home directory
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        
        // Create the main window
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: gpui::WindowKind::Normal,
            is_movable: true,
            app_id: Some("file-explorer".to_string()),
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };
        
        cx.open_window(window_options, |_window, cx| {
            Workspace::build(home_dir, cx)
        })
        .expect("Failed to open window");
    });
}

/// Spawns the Tokio runtime on a dedicated thread.
/// Returns an Arc to the runtime for shared access.
fn spawn_tokio_runtime() -> Arc<Runtime> {
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    Arc::new(runtime)
}

/// Pre-loads default icons into an IconCache.
fn preload_default_icons() -> IconCache {
    let mut cache = IconCache::new();
    
    // Pre-load common icons that will be used frequently
    let default_keys = [
        IconKey::Directory,
        IconKey::GenericFile,
        IconKey::Extension("txt".to_string()),
        IconKey::Extension("rs".to_string()),
        IconKey::Extension("md".to_string()),
        IconKey::Extension("json".to_string()),
        IconKey::Extension("toml".to_string()),
        IconKey::Extension("yaml".to_string()),
        IconKey::Extension("yml".to_string()),
        IconKey::Extension("js".to_string()),
        IconKey::Extension("ts".to_string()),
        IconKey::Extension("py".to_string()),
        IconKey::Extension("html".to_string()),
        IconKey::Extension("css".to_string()),
        IconKey::Extension("png".to_string()),
        IconKey::Extension("jpg".to_string()),
        IconKey::Extension("gif".to_string()),
        IconKey::Extension("svg".to_string()),
        IconKey::Extension("pdf".to_string()),
        IconKey::Extension("zip".to_string()),
    ];
    
    // Request icons to be loaded (they'll use defaults initially)
    for key in default_keys {
        cache.get_or_default(&key);
    }
    
    cache
}
