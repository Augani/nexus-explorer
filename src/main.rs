mod app;
mod io;
mod models;
mod utils;
mod views;

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    px, size, Application, AssetSource, Bounds, Result, SharedString, WindowBounds, WindowOptions,
};
use tokio::runtime::Runtime;

use app::Workspace;
use models::{open_with, GlobalSettings, IconCache, IconKey, WindowManager};

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
    let assets_base = get_assets_base_path();
    let app = Application::new().with_assets(Assets { base: assets_base });

    app.run(|cx| {
        adabraka_ui::init(cx);
        adabraka_ui::set_icon_base_path("assets/icons");

        cx.set_global(GlobalSettings::default());

        let mut window_manager = WindowManager::new();

        let _tokio_runtime = spawn_tokio_runtime();

        open_with::init_app_registry();

        let _icon_cache = preload_default_icons();

        cx.on_window_closed(|cx| {
            if cx.has_global::<WindowManager>() {
                let _ = cx.global::<WindowManager>().save_state();
            }

            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let settings = GlobalSettings::load();
        let should_restore = settings.restore_windows_on_start();

        if should_restore {
            if let Some(saved_state) = WindowManager::load_state() {
                if !saved_state.windows.is_empty() {
                    window_manager.restore_state(saved_state, cx);
                    cx.set_global(window_manager);
                    return;
                }
            }
        }

        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Nexus Explorer".into()),
                appears_transparent: true,
                traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
            }),
            focus: true,
            show: true,
            kind: gpui::WindowKind::Normal,
            is_movable: true,
            app_id: Some("file-explorer".to_string()),
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };

        let path = home_dir.clone();
        let handle = cx
            .open_window(window_options, |_window, cx| Workspace::build(path, cx))
            .expect("Failed to open window");

        window_manager.register_window(handle, home_dir);
        cx.set_global(window_manager);
    });
}




fn get_assets_base_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(macos_dir) = exe_path.parent() {
                if macos_dir.ends_with("MacOS") {
                    if let Some(contents_dir) = macos_dir.parent() {
                        let resources_dir = contents_dir.join("Resources");
                        if resources_dir.exists() {
                            return resources_dir;
                        }
                    }
                }
            }
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let assets_dir = exe_dir.join("assets");
            if assets_dir.exists() {
                return exe_dir.to_path_buf();
            }
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}



fn spawn_tokio_runtime() -> Arc<Runtime> {
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    Arc::new(runtime)
}


fn preload_default_icons() -> IconCache {
    let mut cache = IconCache::new();

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

    for key in default_keys {
        cache.get_or_default(&key);
    }

    cache
}
