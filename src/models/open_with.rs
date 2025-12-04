/// Application discovery for "Open With" functionality
/// 
/// Provides cross-platform support for finding applications that can open specific file types.
/// Uses a preloaded registry that's populated in the background at startup to avoid UI freezes.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

/// Represents an application that can open files
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppInfo {
    pub name: String,
    pub path: PathBuf,
    pub bundle_id: Option<String>,
    pub icon_path: Option<PathBuf>,
}

impl AppInfo {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            bundle_id: None,
            icon_path: None,
        }
    }

    pub fn with_bundle_id(mut self, bundle_id: String) -> Self {
        self.bundle_id = Some(bundle_id);
        self
    }
}

/// Global application registry that maps file extensions to applications
/// This is loaded once at startup in a background thread
pub struct AppRegistry {
    extension_to_apps: HashMap<String, Vec<AppInfo>>,
    all_apps: Vec<AppInfo>,
    is_loaded: AtomicBool,
}

impl AppRegistry {
    pub fn new() -> Self {
        Self {
            extension_to_apps: HashMap::new(),
            all_apps: Vec::new(),
            is_loaded: AtomicBool::new(false),
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.is_loaded.load(Ordering::SeqCst)
    }

    pub fn get_apps_for_extension(&self, extension: &str) -> Vec<AppInfo> {
        let ext = extension.to_lowercase();
        self.extension_to_apps
            .get(&ext)
            .cloned()
            .unwrap_or_default()
    }

    pub fn all_apps(&self) -> &[AppInfo] {
        &self.all_apps
    }
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    static ref APP_REGISTRY: Arc<RwLock<AppRegistry>> = Arc::new(RwLock::new(AppRegistry::new()));
    static ref LOADING_STARTED: AtomicBool = AtomicBool::new(false);
}

/// Initialize the app registry in a background thread
/// Call this once at application startup
pub fn init_app_registry() {
    if LOADING_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        let registry = load_app_registry();
        if let Ok(mut global_registry) = APP_REGISTRY.write() {
            *global_registry = registry;
        }
    });
}

/// Get the icon path for an application
/// On macOS, this extracts the .icns file path from the app bundle
pub fn get_app_icon_path(app_path: &Path) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        // Try to find the icon in the app bundle
        let resources = app_path.join("Contents/Resources");
        if resources.exists() {
            // First try to get icon name from Info.plist
            let info_plist = app_path.join("Contents/Info.plist");
            if info_plist.exists() {
                if let Ok(output) = Command::new("defaults")
                    .args(["read", info_plist.to_str().unwrap_or(""), "CFBundleIconFile"])
                    .output()
                {
                    let icon_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !icon_name.is_empty() {
                        let icon_file = if icon_name.ends_with(".icns") {
                            icon_name
                        } else {
                            format!("{}.icns", icon_name)
                        };
                        let icon_path = resources.join(&icon_file);
                        if icon_path.exists() {
                            return Some(icon_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
            
            // Fallback: look for common icon names
            for name in ["AppIcon.icns", "app.icns", "icon.icns"] {
                let icon_path = resources.join(name);
                if icon_path.exists() {
                    return Some(icon_path.to_string_lossy().to_string());
                }
            }
            
            // Last resort: find any .icns file
            if let Ok(entries) = std::fs::read_dir(&resources) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "icns").unwrap_or(false) {
                        return Some(path.to_string_lossy().to_string());
                    }
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app_path;
        None
    }
}

/// Get applications for a file (uses preloaded registry)
pub fn get_apps_for_file(file_path: &Path) -> Vec<AppInfo> {
    // Handle directories specially
    if file_path.is_dir() {
        if let Ok(registry) = APP_REGISTRY.read() {
            if registry.is_loaded() {
                return registry.get_apps_for_extension("__folder__");
            }
        }
        return Vec::new();
    }

    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if let Ok(registry) = APP_REGISTRY.read() {
        if registry.is_loaded() {
            return registry.get_apps_for_extension(&extension);
        }
    }

    // Fallback: return empty if not loaded yet (don't block UI)
    Vec::new()
}

/// Check if the registry is loaded
pub fn is_registry_loaded() -> bool {
    if let Ok(registry) = APP_REGISTRY.read() {
        registry.is_loaded()
    } else {
        false
    }
}

/// Open a file with a specific application
pub fn open_file_with_app(file_path: &Path, app: &AppInfo) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", app.path.to_str().unwrap_or("")])
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new(&app.path)
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        Command::new(&app.path)
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Platform not supported".to_string())
    }
}

/// Show the system's "Open With" dialog
pub fn show_open_with_dialog(file_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "Finder"
                activate
                set theFile to POSIX file "{}" as alias
                open theFile using (choose application with prompt "Choose an application to open this file:")
            end tell"#,
            file_path.display()
        );
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("Failed to show dialog: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32")
            .args(["shell32.dll,OpenAs_RunDLL", file_path.to_str().unwrap_or("")])
            .spawn()
            .map_err(|e| format!("Failed to show dialog: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Platform not supported".to_string())
    }
}

fn load_app_registry() -> AppRegistry {
    #[cfg(target_os = "macos")]
    {
        load_app_registry_macos()
    }

    #[cfg(target_os = "windows")]
    {
        load_app_registry_windows()
    }

    #[cfg(target_os = "linux")]
    {
        load_app_registry_linux()
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let mut registry = AppRegistry::new();
        registry.is_loaded.store(true, Ordering::SeqCst);
        registry
    }
}

#[cfg(target_os = "macos")]
fn load_app_registry_macos() -> AppRegistry {
    let mut registry = AppRegistry::new();
    let mut extension_to_apps: HashMap<String, Vec<AppInfo>> = HashMap::new();
    let mut all_apps: Vec<AppInfo> = Vec::new();
    let mut seen_apps: HashSet<PathBuf> = HashSet::new();

    // Scan application directories and read Info.plist for each app
    scan_applications_with_plist(&mut extension_to_apps, &mut all_apps, &mut seen_apps);

    // Sort apps by name for consistent display
    for apps in extension_to_apps.values_mut() {
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        apps.dedup_by(|a, b| a.path == b.path);
    }
    all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    registry.extension_to_apps = extension_to_apps;
    registry.all_apps = all_apps;
    registry.is_loaded.store(true, Ordering::SeqCst);
    registry
}

#[cfg(target_os = "macos")]
fn scan_applications_with_plist(
    extension_to_apps: &mut HashMap<String, Vec<AppInfo>>,
    all_apps: &mut Vec<AppInfo>,
    seen_apps: &mut HashSet<PathBuf>,
) {
    let app_dirs = [
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
        dirs::home_dir().map(|h| h.join("Applications")).unwrap_or_default(),
    ];

    // Comprehensive mappings for common applications
    // These are apps that can open various file types but may not declare them in Info.plist
    let editor_extensions: Vec<&str> = vec![
        "txt", "md", "markdown", "rs", "js", "jsx", "ts", "tsx", "py", "rb", "go", "java",
        "c", "cpp", "h", "hpp", "cs", "swift", "m", "mm", "json", "xml", "yaml", "yml",
        "toml", "ini", "cfg", "conf", "html", "htm", "css", "scss", "sass", "less",
        "sql", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", "makefile", "dockerfile",
        "gitignore", "gitattributes", "editorconfig", "env", "lock", "log", "csv",
        "vue", "svelte", "astro", "php", "pl", "pm", "lua", "r", "scala", "kt", "kts",
        "gradle", "groovy", "clj", "cljs", "ex", "exs", "erl", "hrl", "hs", "elm",
        "purs", "ml", "mli", "fs", "fsx", "fsi", "v", "sv", "vhd", "vhdl", "asm", "s",
    ];

    let ide_folder_apps: Vec<&str> = vec![
        "Visual Studio Code", "VSCodium", "Cursor", "Zed", "Sublime Text", "Atom",
        "WebStorm", "IntelliJ IDEA", "IntelliJ IDEA CE", "PyCharm", "PyCharm CE",
        "CLion", "GoLand", "RubyMine", "PhpStorm", "Rider", "DataGrip", "AppCode",
        "Android Studio", "Fleet", "Nova", "BBEdit", "TextMate", "CotEditor",
        "Xcode", "Eclipse", "NetBeans", "Brackets", "Komodo Edit", "Geany",
        "Kate", "Emacs", "MacVim", "Neovide",
    ];

    let common_mappings: HashMap<&str, Vec<&str>> = [
        // Text Editors & IDEs - can open most text files
        ("Visual Studio Code", editor_extensions.clone()),
        ("VSCodium", editor_extensions.clone()),
        ("Cursor", editor_extensions.clone()),
        ("Zed", editor_extensions.clone()),
        ("Sublime Text", editor_extensions.clone()),
        ("Atom", editor_extensions.clone()),
        ("TextMate", editor_extensions.clone()),
        ("BBEdit", editor_extensions.clone()),
        ("CotEditor", editor_extensions.clone()),
        ("Nova", editor_extensions.clone()),
        ("MacVim", editor_extensions.clone()),
        ("Neovide", editor_extensions.clone()),
        
        // JetBrains IDEs
        ("WebStorm", editor_extensions.clone()),
        ("IntelliJ IDEA", editor_extensions.clone()),
        ("IntelliJ IDEA CE", editor_extensions.clone()),
        ("PyCharm", editor_extensions.clone()),
        ("PyCharm CE", editor_extensions.clone()),
        ("CLion", editor_extensions.clone()),
        ("GoLand", editor_extensions.clone()),
        ("RubyMine", editor_extensions.clone()),
        ("PhpStorm", editor_extensions.clone()),
        ("Rider", editor_extensions.clone()),
        ("DataGrip", vec!["sql", "json", "csv"]),
        ("Fleet", editor_extensions.clone()),
        
        // Apple apps
        ("TextEdit", vec!["txt", "rtf", "rtfd", "html", "htm", "doc"]),
        ("Xcode", vec!["swift", "m", "mm", "h", "c", "cpp", "hpp", "metal", "xcodeproj", "xcworkspace", "playground", "storyboard", "xib", "plist"]),
        ("Preview", vec!["pdf", "png", "jpg", "jpeg", "gif", "tiff", "tif", "bmp", "heic", "heif", "webp", "ico", "icns", "psd", "ai", "eps"]),
        ("QuickTime Player", vec!["mp4", "mov", "m4v", "mp3", "m4a", "wav", "aiff", "aif", "aac", "3gp", "avi"]),
        ("Music", vec!["mp3", "m4a", "aac", "wav", "aiff", "flac", "alac"]),
        ("Photos", vec!["png", "jpg", "jpeg", "gif", "heic", "heif", "raw", "cr2", "nef", "arw"]),
        ("Safari", vec!["html", "htm", "url", "webloc", "webarchive"]),
        ("Numbers", vec!["csv", "xlsx", "xls", "numbers", "tsv"]),
        ("Pages", vec!["doc", "docx", "pages", "rtf", "txt"]),
        ("Keynote", vec!["ppt", "pptx", "key"]),
        ("Notes", vec!["txt", "md"]),
        ("Terminal", vec!["sh", "bash", "zsh", "command", "tool"]),
        ("Script Editor", vec!["scpt", "applescript", "scptd"]),
        
        // Browsers
        ("Firefox", vec!["html", "htm", "xhtml", "svg", "pdf", "json", "xml"]),
        ("Google Chrome", vec!["html", "htm", "xhtml", "svg", "pdf", "json", "xml"]),
        ("Brave Browser", vec!["html", "htm", "xhtml", "svg", "pdf", "json", "xml"]),
        ("Microsoft Edge", vec!["html", "htm", "xhtml", "svg", "pdf", "json", "xml"]),
        ("Arc", vec!["html", "htm", "xhtml", "svg", "pdf", "json", "xml"]),
        ("Opera", vec!["html", "htm", "xhtml", "svg", "pdf", "json", "xml"]),
        
        // Media players
        ("VLC", vec!["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma"]),
        ("IINA", vec!["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "mp3", "flac", "wav", "ogg"]),
        ("Infuse", vec!["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm"]),
        ("Spotify", vec!["mp3", "m4a", "ogg"]),
        
        // Image editors
        ("Pixelmator Pro", vec!["png", "jpg", "jpeg", "gif", "tiff", "psd", "heic", "webp", "svg", "pdf", "raw"]),
        ("Affinity Photo", vec!["png", "jpg", "jpeg", "gif", "tiff", "psd", "heic", "webp", "raw", "afphoto"]),
        ("Affinity Designer", vec!["svg", "ai", "eps", "pdf", "afdesign"]),
        ("Sketch", vec!["sketch", "svg", "pdf", "png", "jpg"]),
        ("Figma", vec!["fig", "svg", "png", "jpg", "pdf"]),
        ("GIMP", vec!["png", "jpg", "jpeg", "gif", "tiff", "psd", "xcf", "bmp"]),
        ("Adobe Photoshop", vec!["psd", "png", "jpg", "jpeg", "gif", "tiff", "bmp", "raw"]),
        ("Adobe Illustrator", vec!["ai", "svg", "eps", "pdf", "png"]),
        
        // Archive utilities
        ("Archive Utility", vec!["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "tgz", "tbz"]),
        ("The Unarchiver", vec!["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "tgz", "tbz", "lzma", "cab", "iso", "dmg"]),
        ("Keka", vec!["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "tgz", "tbz", "lzma"]),
        ("BetterZip", vec!["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "tgz", "tbz"]),
        
        // Development tools
        ("Android Studio", vec!["java", "kt", "kts", "xml", "gradle", "json"]),
        ("Docker", vec!["dockerfile", "yaml", "yml", "json"]),
        ("Postman", vec!["json", "xml"]),
        ("Insomnia", vec!["json", "xml", "yaml", "yml"]),
        
        // Database tools
        ("TablePlus", vec!["sql", "csv", "json"]),
        ("Sequel Pro", vec!["sql"]),
        ("DBeaver", vec!["sql", "csv", "json", "xml"]),
        
        // Other utilities
        ("Finder", vec![]),
        ("iTerm", vec!["sh", "bash", "zsh", "command"]),
        ("Warp", vec!["sh", "bash", "zsh", "command"]),
        ("Alacritty", vec!["sh", "bash", "zsh", "command"]),
        ("Kitty", vec!["sh", "bash", "zsh", "command"]),
    ].into_iter().collect();

    // Scan all app directories
    for app_dir in &app_dirs {
        if !app_dir.exists() {
            continue;
        }
        scan_app_directory(app_dir, extension_to_apps, all_apps, seen_apps, &common_mappings, &ide_folder_apps);
    }

    // Add folder support for IDEs
    for ide_name in &ide_folder_apps {
        if let Some(app) = all_apps.iter().find(|a| a.name == *ide_name) {
            extension_to_apps
                .entry("__folder__".to_string())
                .or_default()
                .push(app.clone());
        }
    }
}

#[cfg(target_os = "macos")]
fn scan_app_directory(
    dir: &Path,
    extension_to_apps: &mut HashMap<String, Vec<AppInfo>>,
    all_apps: &mut Vec<AppInfo>,
    seen_apps: &mut HashSet<PathBuf>,
    common_mappings: &HashMap<&str, Vec<&str>>,
    ide_folder_apps: &[&str],
) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };

    for entry in entries.flatten() {
        let path = entry.path();
        
        // Handle nested directories (like /Applications/Utilities)
        if path.is_dir() && path.extension().is_none() {
            scan_app_directory(&path, extension_to_apps, all_apps, seen_apps, common_mappings, ide_folder_apps);
            continue;
        }
        
        if !path.extension().map(|e| e == "app").unwrap_or(false) || seen_apps.contains(&path) {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let app_info = AppInfo::new(name.clone(), path.clone());
        
        // First, try to read extensions from Info.plist
        let plist_extensions = read_app_document_types(&path);
        
        // Add extensions from Info.plist
        for ext in &plist_extensions {
            extension_to_apps
                .entry(ext.to_lowercase())
                .or_default()
                .push(app_info.clone());
        }
        
        // Also add from common mappings (these may not be in Info.plist)
        if let Some(extensions) = common_mappings.get(name.as_str()) {
            for ext in extensions {
                let ext_lower = ext.to_lowercase();
                let apps = extension_to_apps.entry(ext_lower).or_default();
                if !apps.iter().any(|a| a.path == app_info.path) {
                    apps.push(app_info.clone());
                }
            }
        }

        all_apps.push(app_info);
        seen_apps.insert(path);
    }
}

#[cfg(target_os = "macos")]
fn read_app_document_types(app_path: &Path) -> Vec<String> {
    let mut extensions = Vec::new();
    let info_plist = app_path.join("Contents/Info.plist");
    
    if !info_plist.exists() {
        return extensions;
    }

    // Use plutil to convert plist to JSON for easier parsing
    let Ok(output) = Command::new("plutil")
        .args(["-convert", "json", "-o", "-", info_plist.to_str().unwrap_or("")])
        .output()
    else {
        return extensions;
    };

    let json_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse CFBundleDocumentTypes to get supported extensions
    // Look for patterns like "CFBundleTypeExtensions": ["ext1", "ext2"]
    if let Some(doc_types_start) = json_str.find("CFBundleDocumentTypes") {
        let remaining = &json_str[doc_types_start..];
        
        // Find all extension arrays
        let mut pos = 0;
        while let Some(ext_start) = remaining[pos..].find("CFBundleTypeExtensions") {
            let search_start = pos + ext_start;
            if let Some(arr_start) = remaining[search_start..].find('[') {
                let arr_begin = search_start + arr_start;
                if let Some(arr_end) = remaining[arr_begin..].find(']') {
                    let arr_content = &remaining[arr_begin + 1..arr_begin + arr_end];
                    // Extract extensions from the array
                    for part in arr_content.split(',') {
                        let ext = part.trim().trim_matches(|c| c == '"' || c == '\'' || c == ' ');
                        if !ext.is_empty() && ext.len() < 20 && !ext.contains(':') {
                            extensions.push(ext.to_lowercase());
                        }
                    }
                    pos = arr_begin + arr_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Also check UTExportedTypeDeclarations and UTImportedTypeDeclarations
    for uti_key in ["UTExportedTypeDeclarations", "UTImportedTypeDeclarations"] {
        if let Some(uti_start) = json_str.find(uti_key) {
            let remaining = &json_str[uti_start..];
            // Look for public.filename-extension patterns
            let mut pos = 0;
            while let Some(ext_tag_start) = remaining[pos..].find("public.filename-extension") {
                let search_start = pos + ext_tag_start;
                // Find the value after this key
                if let Some(colon) = remaining[search_start..].find(':') {
                    let value_start = search_start + colon + 1;
                    // Could be a string or array
                    let value_area = &remaining[value_start..value_start.saturating_add(200).min(remaining.len())];
                    
                    if let Some(quote_start) = value_area.find('"') {
                        if let Some(quote_end) = value_area[quote_start + 1..].find('"') {
                            let ext = &value_area[quote_start + 1..quote_start + 1 + quote_end];
                            if !ext.is_empty() && ext.len() < 20 {
                                extensions.push(ext.to_lowercase());
                            }
                        }
                    }
                    pos = value_start;
                } else {
                    break;
                }
            }
        }
    }

    extensions.sort();
    extensions.dedup();
    extensions
}

#[cfg(target_os = "windows")]
fn load_app_registry_windows() -> AppRegistry {
    let mut registry = AppRegistry::new();
    let mut extension_to_apps: HashMap<String, Vec<AppInfo>> = HashMap::new();
    let mut all_apps: Vec<AppInfo> = Vec::new();
    let mut seen_apps: HashSet<PathBuf> = HashSet::new();

    // Query Windows registry for file associations
    // HKEY_CLASSES_ROOT contains file extension associations
    if let Ok(output) = Command::new("reg")
        .args(["query", "HKEY_CLASSES_ROOT", "/s", "/f", "OpenWithProgids"])
        .output()
    {
        let result = String::from_utf8_lossy(&output.stdout);
        parse_windows_registry(&result, &mut extension_to_apps, &mut all_apps, &mut seen_apps);
    }

    // Scan common application directories
    let app_dirs = [
        PathBuf::from(r"C:\Program Files"),
        PathBuf::from(r"C:\Program Files (x86)"),
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default(),
    ];

    for app_dir in app_dirs {
        scan_windows_apps(&app_dir, &mut all_apps, &mut seen_apps);
    }

    // Sort apps
    for apps in extension_to_apps.values_mut() {
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        apps.dedup_by(|a, b| a.path == b.path);
    }
    all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    registry.extension_to_apps = extension_to_apps;
    registry.all_apps = all_apps;
    registry.is_loaded.store(true, Ordering::SeqCst);
    registry
}

#[cfg(target_os = "windows")]
fn parse_windows_registry(
    _output: &str,
    _extension_to_apps: &mut HashMap<String, Vec<AppInfo>>,
    _all_apps: &mut Vec<AppInfo>,
    _seen_apps: &mut HashSet<PathBuf>,
) {
    // Windows registry parsing is complex - for now we rely on scanning app directories
    // A full implementation would parse HKEY_CLASSES_ROOT\.ext\OpenWithProgids
}

#[cfg(target_os = "windows")]
fn scan_windows_apps(
    dir: &Path,
    all_apps: &mut Vec<AppInfo>,
    seen_apps: &mut HashSet<PathBuf>,
) {
    if !dir.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Look for .exe files in subdirectories
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let exe_path = sub_entry.path();
                        if exe_path.extension().map(|e| e == "exe").unwrap_or(false) 
                            && !seen_apps.contains(&exe_path) 
                        {
                            let name = exe_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown")
                                .to_string();

                            all_apps.push(AppInfo::new(name, exe_path.clone()));
                            seen_apps.insert(exe_path);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn load_app_registry_linux() -> AppRegistry {
    let mut registry = AppRegistry::new();
    let mut extension_to_apps: HashMap<String, Vec<AppInfo>> = HashMap::new();
    let mut all_apps: Vec<AppInfo> = Vec::new();
    let mut seen_apps: HashSet<PathBuf> = HashSet::new();

    // Parse .desktop files from standard locations
    let desktop_dirs = [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/applications"))
            .unwrap_or_default(),
    ];

    for dir in desktop_dirs {
        if dir.exists() {
            parse_desktop_files(&dir, &mut extension_to_apps, &mut all_apps, &mut seen_apps);
        }
    }

    // Sort apps
    for apps in extension_to_apps.values_mut() {
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        apps.dedup_by(|a, b| a.path == b.path);
    }
    all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    registry.extension_to_apps = extension_to_apps;
    registry.all_apps = all_apps;
    registry.is_loaded.store(true, Ordering::SeqCst);
    registry
}

#[cfg(target_os = "linux")]
fn parse_desktop_files(
    dir: &Path,
    extension_to_apps: &mut HashMap<String, Vec<AppInfo>>,
    all_apps: &mut Vec<AppInfo>,
    seen_apps: &mut HashSet<PathBuf>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    parse_single_desktop_file(&content, &path, extension_to_apps, all_apps, seen_apps);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_single_desktop_file(
    content: &str,
    desktop_path: &Path,
    extension_to_apps: &mut HashMap<String, Vec<AppInfo>>,
    all_apps: &mut Vec<AppInfo>,
    seen_apps: &mut HashSet<PathBuf>,
) {
    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut mime_types: Vec<String> = Vec::new();
    let mut no_display = false;

    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("Name=") && name.is_none() {
            name = trimmed.strip_prefix("Name=").map(|s| s.to_string());
        } else if trimmed.starts_with("Exec=") {
            exec = trimmed
                .strip_prefix("Exec=")
                .map(|s| s.split_whitespace().next().unwrap_or(s).to_string());
        } else if trimmed.starts_with("MimeType=") {
            if let Some(types) = trimmed.strip_prefix("MimeType=") {
                mime_types = types.split(';').map(|s| s.to_string()).collect();
            }
        } else if trimmed == "NoDisplay=true" {
            no_display = true;
        }
    }

    if no_display {
        return;
    }

    if let (Some(name), Some(exec)) = (name, exec) {
        let exec_path = PathBuf::from(&exec);
        
        if seen_apps.contains(&exec_path) {
            return;
        }

        let app_info = AppInfo::new(name, exec_path.clone());

        // Convert MIME types to extensions
        for mime in &mime_types {
            if let Some(ext) = mime_to_extension(mime) {
                extension_to_apps
                    .entry(ext)
                    .or_default()
                    .push(app_info.clone());
            }
        }

        all_apps.push(app_info);
        seen_apps.insert(exec_path);
    }
}

#[cfg(target_os = "linux")]
fn mime_to_extension(mime: &str) -> Option<String> {
    // Common MIME type to extension mappings
    let mapping: HashMap<&str, &str> = [
        ("text/plain", "txt"),
        ("text/html", "html"),
        ("text/css", "css"),
        ("text/javascript", "js"),
        ("text/markdown", "md"),
        ("application/json", "json"),
        ("application/pdf", "pdf"),
        ("application/zip", "zip"),
        ("application/x-tar", "tar"),
        ("application/gzip", "gz"),
        ("image/png", "png"),
        ("image/jpeg", "jpg"),
        ("image/gif", "gif"),
        ("image/svg+xml", "svg"),
        ("image/webp", "webp"),
        ("audio/mpeg", "mp3"),
        ("audio/wav", "wav"),
        ("audio/flac", "flac"),
        ("video/mp4", "mp4"),
        ("video/webm", "webm"),
        ("video/x-matroska", "mkv"),
    ]
    .into_iter()
    .collect();

    mapping.get(mime).map(|s| s.to_string())
}
