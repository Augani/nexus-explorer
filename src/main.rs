mod app;
mod io;
mod models;
mod utils;
mod views;

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{px, size, Application, AssetSource, Bounds, Result, SharedString, WindowBounds, WindowOptions};
use tokio::runtime::Runtime;

use app::Workspace;
use models::{GlobalSettings, IconCache, IconKey};

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        std::fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        std::fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

fn main() {
    let app = Application::new()
        .with_assets(Assets {
            base: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        });
    
    app.run(|cx| {
        // Initialize adabraka-ui components and set icon path
        adabraka_ui::init(cx);
        adabraka_ui::set_icon_base_path("assets/icons");
        
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
