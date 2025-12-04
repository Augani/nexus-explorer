/// Application discovery for "Open With" functionality
/// 
/// Provides cross-platform support for finding applications that can open specific file types
/// based on file extension and system-registered handlers.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;

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

/// Get applications that can open a file based on its extension
pub fn get_apps_for_file(file_path: &Path) -> Vec<AppInfo> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    #[cfg(target_os = "macos")]
    {
        get_apps_for_extension_macos(&extension)
    }
    
    #[cfg(target_os = "windows")]
    {
        get_apps_for_extension_windows(&extension)
    }
    
    #[cfg(target_os = "linux")]
    {
        get_apps_for_extension_linux(&extension)
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Vec::new()
    }
}

/// Open a file with a specific application
pub fn open_file_with_app(file_path: &Path, app: &AppInfo) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .args(["-a", app.path.to_str().unwrap_or("")])
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    {
        let status = Command::new(&app.path)
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    {
        let status = Command::new(&app.path)
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
        // Try various Linux file managers' open-with dialogs
        if Command::new("nautilus")
            .args(["--select", file_path.to_str().unwrap_or("")])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
        
        // Fallback to xdg-open
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

#[cfg(target_os = "macos")]
fn get_apps_for_extension_macos(extension: &str) -> Vec<AppInfo> {
    let mut apps = Vec::new();
    
    // Use Launch Services to get apps for this UTI
    let output = Command::new("mdfind")
        .args(["kMDItemContentTypeTree == 'public.app' && kMDItemKind == 'Application'"])
        .output();
    
    // Get the default app for this extension first
    if let Some(default_app) = get_default_app_macos(extension) {
        apps.push(default_app);
    }
    
    // Get recommended apps based on extension
    let recommended = get_recommended_apps_for_extension(extension);
    for app_name in recommended {
        if let Some(app) = find_app_by_name_macos(&app_name) {
            if !apps.iter().any(|a| a.path == app.path) {
                apps.push(app);
            }
        }
    }
    
    // Add common apps that might be installed
    let common_apps = get_common_apps_macos();
    for app in common_apps {
        if !apps.iter().any(|a| a.path == app.path) && app.path.exists() {
            apps.push(app);
        }
    }
    
    apps
}

#[cfg(target_os = "macos")]
fn get_default_app_macos(extension: &str) -> Option<AppInfo> {
    // Use duti or Launch Services to get default app
    let output = Command::new("sh")
        .args(["-c", &format!(
            "defaults read com.apple.LaunchServices/com.apple.launchservices.secure LSHandlers 2>/dev/null | grep -A2 'LSHandlerContentType.*{}' | grep LSHandlerRoleAll | head -1 | sed 's/.*= \"\\(.*\\)\";/\\1/'",
            extension
        )])
        .output()
        .ok()?;
    
    let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if bundle_id.is_empty() {
        return None;
    }
    
    // Convert bundle ID to app path
    let path_output = Command::new("mdfind")
        .args([&format!("kMDItemCFBundleIdentifier == '{}'", bundle_id)])
        .output()
        .ok()?;
    
    let path_str = String::from_utf8_lossy(&path_output.stdout);
    let path = path_str.lines().next()?;
    
    let name = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();
    
    Some(AppInfo::new(name, PathBuf::from(path)).with_bundle_id(bundle_id))
}

#[cfg(target_os = "macos")]
fn find_app_by_name_macos(name: &str) -> Option<AppInfo> {
    let paths = [
        format!("/Applications/{}.app", name),
        format!("/System/Applications/{}.app", name),
        format!("/Applications/Utilities/{}.app", name),
    ];
    
    for path_str in &paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Some(AppInfo::new(name.to_string(), path));
        }
    }
    
    // Try mdfind as fallback
    let output = Command::new("mdfind")
        .args([&format!("kMDItemDisplayName == '{}'", name)])
        .output()
        .ok()?;
    
    let path_str = String::from_utf8_lossy(&output.stdout);
    let path = path_str.lines().find(|p| p.ends_with(".app"))?;
    
    Some(AppInfo::new(name.to_string(), PathBuf::from(path)))
}

#[cfg(target_os = "macos")]
fn get_common_apps_macos() -> Vec<AppInfo> {
    vec![
        AppInfo::new("TextEdit".to_string(), PathBuf::from("/System/Applications/TextEdit.app")),
        AppInfo::new("Preview".to_string(), PathBuf::from("/System/Applications/Preview.app")),
        AppInfo::new("QuickTime Player".to_string(), PathBuf::from("/System/Applications/QuickTime Player.app")),
        AppInfo::new("Safari".to_string(), PathBuf::from("/Applications/Safari.app")),
        AppInfo::new("Finder".to_string(), PathBuf::from("/System/Library/CoreServices/Finder.app")),
    ]
}

fn get_recommended_apps_for_extension(extension: &str) -> Vec<&'static str> {
    match extension {
        // Text/Code files
        "txt" | "md" | "markdown" => vec!["TextEdit", "Visual Studio Code", "Sublime Text", "BBEdit", "Nova"],
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "swift" => 
            vec!["Visual Studio Code", "Xcode", "Sublime Text", "Nova", "TextEdit"],
        "html" | "htm" | "css" | "scss" | "sass" | "less" => 
            vec!["Visual Studio Code", "Safari", "Google Chrome", "Firefox", "Sublime Text"],
        "json" | "yaml" | "yml" | "toml" | "xml" => 
            vec!["Visual Studio Code", "TextEdit", "Sublime Text", "Nova"],
        
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "tiff" | "bmp" | "heic" => 
            vec!["Preview", "Photos", "Pixelmator Pro", "Affinity Photo", "Adobe Photoshop"],
        "svg" => vec!["Safari", "Affinity Designer", "Sketch", "Figma", "Adobe Illustrator"],
        "psd" => vec!["Adobe Photoshop", "Affinity Photo", "Pixelmator Pro", "Preview"],
        "ai" => vec!["Adobe Illustrator", "Affinity Designer", "Sketch"],
        
        // Documents
        "pdf" => vec!["Preview", "Adobe Acrobat Reader", "Safari", "Google Chrome"],
        "doc" | "docx" => vec!["Microsoft Word", "Pages", "LibreOffice Writer", "Google Docs"],
        "xls" | "xlsx" => vec!["Microsoft Excel", "Numbers", "LibreOffice Calc"],
        "ppt" | "pptx" => vec!["Microsoft PowerPoint", "Keynote", "LibreOffice Impress"],
        "pages" => vec!["Pages"],
        "numbers" => vec!["Numbers"],
        "key" => vec!["Keynote"],
        
        // Audio
        "mp3" | "wav" | "aac" | "flac" | "m4a" | "ogg" => 
            vec!["Music", "QuickTime Player", "VLC", "Audacity"],
        
        // Video
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v" => 
            vec!["QuickTime Player", "VLC", "IINA", "Infuse"],
        
        // Archives
        "zip" | "tar" | "gz" | "rar" | "7z" => 
            vec!["Archive Utility", "The Unarchiver", "Keka", "BetterZip"],
        
        // Executables/Scripts
        "sh" | "bash" | "zsh" => vec!["Terminal", "iTerm", "Visual Studio Code"],
        "app" => vec!["Finder"],
        
        // Default
        _ => vec!["TextEdit", "Preview"],
    }
}

#[cfg(target_os = "windows")]
fn get_apps_for_extension_windows(extension: &str) -> Vec<AppInfo> {
    let mut apps = Vec::new();
    
    // Query registry for associated programs
    let output = Command::new("cmd")
        .args(["/c", &format!("assoc .{}", extension)])
        .output();
    
    // Add recommended apps based on extension
    let recommended = get_recommended_apps_for_extension_windows(extension);
    for (name, path) in recommended {
        let app_path = PathBuf::from(path);
        if app_path.exists() {
            apps.push(AppInfo::new(name.to_string(), app_path));
        }
    }
    
    apps
}

#[cfg(target_os = "windows")]
fn get_recommended_apps_for_extension_windows(extension: &str) -> Vec<(&'static str, &'static str)> {
    match extension {
        "txt" | "md" => vec![
            ("Notepad", "C:\\Windows\\System32\\notepad.exe"),
            ("Visual Studio Code", "C:\\Program Files\\Microsoft VS Code\\Code.exe"),
        ],
        "pdf" => vec![
            ("Microsoft Edge", "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"),
            ("Adobe Acrobat", "C:\\Program Files\\Adobe\\Acrobat DC\\Acrobat\\Acrobat.exe"),
        ],
        "png" | "jpg" | "jpeg" | "gif" => vec![
            ("Photos", "C:\\Program Files\\WindowsApps\\Microsoft.Windows.Photos_*\\Microsoft.Photos.exe"),
            ("Paint", "C:\\Windows\\System32\\mspaint.exe"),
        ],
        _ => vec![
            ("Notepad", "C:\\Windows\\System32\\notepad.exe"),
        ],
    }
}

#[cfg(target_os = "linux")]
fn get_apps_for_extension_linux(extension: &str) -> Vec<AppInfo> {
    let mut apps = Vec::new();
    
    // Get MIME type for extension
    let mime_output = Command::new("xdg-mime")
        .args(["query", "filetype", &format!("dummy.{}", extension)])
        .output();
    
    if let Ok(output) = mime_output {
        let mime_type = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // Get default app for MIME type
        if let Ok(default_output) = Command::new("xdg-mime")
            .args(["query", "default", &mime_type])
            .output()
        {
            let desktop_file = String::from_utf8_lossy(&default_output.stdout).trim().to_string();
            if !desktop_file.is_empty() {
                if let Some(app) = parse_desktop_file_linux(&desktop_file) {
                    apps.push(app);
                }
            }
        }
    }
    
    // Add common Linux apps
    let common = vec![
        ("gedit", "/usr/bin/gedit"),
        ("nautilus", "/usr/bin/nautilus"),
        ("eog", "/usr/bin/eog"),
        ("vlc", "/usr/bin/vlc"),
        ("code", "/usr/bin/code"),
    ];
    
    for (name, path) in common {
        let app_path = PathBuf::from(path);
        if app_path.exists() && !apps.iter().any(|a| a.path == app_path) {
            apps.push(AppInfo::new(name.to_string(), app_path));
        }
    }
    
    apps
}

#[cfg(target_os = "linux")]
fn parse_desktop_file_linux(desktop_file: &str) -> Option<AppInfo> {
    let paths = [
        format!("/usr/share/applications/{}", desktop_file),
        format!("{}/.local/share/applications/{}", std::env::var("HOME").unwrap_or_default(), desktop_file),
    ];
    
    for path_str in &paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut name = None;
                let mut exec = None;
                
                for line in content.lines() {
                    if line.starts_with("Name=") {
                        name = Some(line.trim_start_matches("Name=").to_string());
                    } else if line.starts_with("Exec=") {
                        let exec_line = line.trim_start_matches("Exec=");
                        exec = Some(exec_line.split_whitespace().next().unwrap_or("").to_string());
                    }
                }
                
                if let (Some(n), Some(e)) = (name, exec) {
                    return Some(AppInfo::new(n, PathBuf::from(e)));
                }
            }
        }
    }
    
    None
}
