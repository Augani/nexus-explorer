use std::path::PathBuf;
use std::process::Command;

/// Check if this app is set as the default file browser
/// Note: This is a simplified check - full detection requires more complex APIs
pub fn is_default_file_browser() -> bool {
    // For now, we store this preference locally
    get_default_preference()
}

/// Set this app as the default file browser
pub fn set_as_default_file_browser() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"
            display dialog "To set Nexus as your default file browser:" & return & return & "1. Right-click any folder in Finder" & return & "2. Select 'Get Info'" & return & "3. Under 'Open with:', select Nexus" & return & "4. Click 'Change All...'" & return & return & "Alternatively, drag folders onto the Nexus icon in your Dock." buttons {"Open System Settings", "OK"} default button "OK"
            if button returned of result is "Open System Settings" then
                do shell script "open 'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles'"
            end if
        "#;

        let _ = Command::new("osascript").args(["-e", script]).output();

        // Save preference
        save_default_preference(true);

        Ok("Instructions shown. Follow the steps to set as default.".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/c", "start", "ms-settings:defaultapps"])
            .output();

        save_default_preference(true);
        Ok("Default Apps settings opened.".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

        let desktop_entry = format!(
            r#"[Desktop Entry]
Name=Nexus File Explorer
Exec={} %U
Type=Application
Categories=System;FileManager;
MimeType=inode/directory;
"#,
            exe_path.to_string_lossy()
        );

        if let Some(data_dir) = dirs::data_dir() {
            let desktop_path = data_dir.join("applications/nexus-file-explorer.desktop");
            let _ = std::fs::create_dir_all(desktop_path.parent().unwrap());
            let _ = std::fs::write(&desktop_path, desktop_entry);

            let _ = Command::new("xdg-mime")
                .args(["default", "nexus-file-explorer.desktop", "inode/directory"])
                .output();
        }

        save_default_preference(true);
        Ok("Set as default file manager.".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Not supported on this platform".to_string())
    }
}

/// Remove this app as the default file browser preference
pub fn restore_default_file_browser() -> Result<String, String> {
    save_default_preference(false);

    #[cfg(target_os = "macos")]
    {
        let script = r#"
            display dialog "To restore Finder as your default:" & return & return & "1. Right-click any folder" & return & "2. Select 'Get Info'" & return & "3. Under 'Open with:', select Finder" & return & "4. Click 'Change All...'" buttons {"OK"} default button "OK"
        "#;
        let _ = Command::new("osascript").args(["-e", script]).output();
    }

    Ok("Preference cleared.".to_string())
}

fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("nexus-explorer").join("default_browser.txt"))
}

fn get_default_preference() -> bool {
    if let Some(path) = get_config_path() {
        std::fs::read_to_string(path)
            .map(|s| s.trim() == "true")
            .unwrap_or(false)
    } else {
        false
    }
}

fn save_default_preference(is_default: bool) {
    if let Some(path) = get_config_path() {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(path, if is_default { "true" } else { "false" });
    }
}
