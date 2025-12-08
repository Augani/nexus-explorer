use crate::models::{
    ShareConfig, ShareInfo, SharePermission, 
    PlatformShareMethod, get_available_share_methods, is_share_method_available,
    get_share_method_unavailable_reason,
};
use std::path::PathBuf;

/
#[derive(Clone, PartialEq)]
pub enum ShareDialogAction {
    Share,
    Cancel,
    StopSharing,
    ShareNameChanged(String),
    DescriptionChanged(String),
    PermissionChanged(SharePermission),
    MaxUsersChanged(Option<u32>),
    /
    ShareViaPlatform(PlatformShareMethod),
    /
    SwitchTab(ShareDialogTab),
}

/
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ShareDialogTab {
    #[default]
    PlatformShare,
    NetworkShare,
}

/
pub struct ShareDialog {
    paths: Vec<PathBuf>,
    path: PathBuf,
    share_name: String,
    description: String,
    permission: SharePermission,
    max_users: Option<u32>,
    error_message: Option<String>,
    is_sharing: bool,
    is_already_shared: bool,
    existing_share: Option<ShareInfo>,
    on_share: Option<Box<dyn Fn(ShareConfig) + Send + Sync>>,
    on_stop_sharing: Option<Box<dyn Fn(PathBuf) + Send + Sync>>,
    on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
    on_platform_share: Option<Box<dyn Fn(Vec<PathBuf>, PlatformShareMethod) + Send + Sync>>,
    /
    current_tab: ShareDialogTab,
    /
    available_methods: Vec<PlatformShareMethod>,
    /
    platform_share_status: Option<String>,
}

impl ShareDialog {
    pub fn new(path: PathBuf) -> Self {
        let default_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Share")
            .to_string();

        let available_methods = get_available_share_methods();

        Self {
            paths: vec![path.clone()],
            path,
            share_name: default_name,
            description: String::new(),
            permission: SharePermission::ReadOnly,
            max_users: None,
            error_message: None,
            is_sharing: false,
            is_already_shared: false,
            existing_share: None,
            on_share: None,
            on_stop_sharing: None,
            on_cancel: None,
            on_platform_share: None,
            current_tab: ShareDialogTab::PlatformShare,
            available_methods,
            platform_share_status: None,
        }
    }

    /
    pub fn new_multi(paths: Vec<PathBuf>) -> Self {
        let default_name = if paths.len() == 1 {
            paths[0]
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Share")
                .to_string()
        } else {
            format!("{} items", paths.len())
        };

        let first_path = paths.first().cloned().unwrap_or_default();
        let available_methods = get_available_share_methods();

        Self {
            paths,
            path: first_path,
            share_name: default_name,
            description: String::new(),
            permission: SharePermission::ReadOnly,
            max_users: None,
            error_message: None,
            is_sharing: false,
            is_already_shared: false,
            existing_share: None,
            on_share: None,
            on_stop_sharing: None,
            on_cancel: None,
            on_platform_share: None,
            current_tab: ShareDialogTab::PlatformShare,
            available_methods,
            platform_share_status: None,
        }
    }

    pub fn with_existing_share(mut self, share: ShareInfo) -> Self {
        self.share_name = share.share_name.clone();
        self.description = share.description.clone();
        self.permission = share.permission;
        self.max_users = share.max_users;
        self.is_already_shared = true;
        self.existing_share = Some(share);
        self
    }

    pub fn with_on_share<F>(mut self, callback: F) -> Self
    where
        F: Fn(ShareConfig) + Send + Sync + 'static,
    {
        self.on_share = Some(Box::new(callback));
        self
    }

    pub fn with_on_stop_sharing<F>(mut self, callback: F) -> Self
    where
        F: Fn(PathBuf) + Send + Sync + 'static,
    {
        self.on_stop_sharing = Some(Box::new(callback));
        self
    }

    pub fn with_on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    pub fn with_on_platform_share<F>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<PathBuf>, PlatformShareMethod) + Send + Sync + 'static,
    {
        self.on_platform_share = Some(Box::new(callback));
        self
    }

    /
    pub fn build_config(&self) -> ShareConfig {
        let mut config = ShareConfig::new(self.share_name.clone(), self.path.clone())
            .with_description(self.description.clone())
            .with_permission(self.permission);

        if let Some(max) = self.max_users {
            config = config.with_max_users(max);
        }

        config
    }

    /
    pub fn validate_share_name(&self) -> Result<(), String> {
        if self.share_name.is_empty() {
            return Err("Share name cannot be empty".to_string());
        }

        if self.share_name.len() > 80 {
            return Err("Share name is too long (max 80 characters)".to_string());
        }

        let invalid_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
        for c in invalid_chars {
            if self.share_name.contains(c) {
                return Err(format!("Share name cannot contain '{}'", c));
            }
        }

        Ok(())
    }

    /
    pub fn validate(&self) -> Result<(), String> {
        self.validate_share_name()?;

        if !self.path.exists() {
            return Err("The folder no longer exists".to_string());
        }

        if !self.path.is_dir() {
            return Err("Only folders can be shared".to_string());
        }

        Ok(())
    }

    /
    pub fn execute_share(&mut self) {
        if let Err(msg) = self.validate() {
            self.error_message = Some(msg);
            return;
        }

        self.error_message = None;
        self.is_sharing = true;

        let config = self.build_config();
        if let Some(callback) = &self.on_share {
            callback(config);
        }
    }

    /
    pub fn execute_stop_sharing(&mut self) {
        if let Some(callback) = &self.on_stop_sharing {
            callback(self.path.clone());
        }
    }

    /
    pub fn handle_cancel(&self) {
        if let Some(callback) = &self.on_cancel {
            callback();
        }
    }

    /
    pub fn set_share_name(&mut self, name: String) {
        self.share_name = name;
        self.error_message = None;
    }

    /
    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    /
    pub fn set_permission(&mut self, permission: SharePermission) {
        self.permission = permission;
    }

    /
    pub fn set_max_users(&mut self, max_users: Option<u32>) {
        self.max_users = max_users;
    }

    /
    pub fn set_share_complete(&mut self, success: bool, error: Option<String>) {
        self.is_sharing = false;
        if success {
            self.is_already_shared = true;
        } else {
            self.error_message = error;
        }
    }

    /
    pub fn set_share_removed(&mut self) {
        self.is_already_shared = false;
        self.existing_share = None;
    }

    /
    pub fn execute_platform_share(&mut self, method: PlatformShareMethod) {
        if !is_share_method_available(method) {
            if let Some(reason) = get_share_method_unavailable_reason(method) {
                self.error_message = Some(reason);
            } else {
                self.error_message = Some(format!("{} is not available", method.display_name()));
            }
            return;
        }

        self.error_message = None;
        self.platform_share_status = Some(format!("Opening {}...", method.display_name()));

        if let Some(callback) = &self.on_platform_share {
            callback(self.paths.clone(), method);
        }
    }

    /
    pub fn set_platform_share_result(&mut self, success: bool, message: Option<String>) {
        if success {
            self.platform_share_status = message.or(Some("Share initiated".to_string()));
        } else {
            self.error_message = message;
            self.platform_share_status = None;
        }
    }

    /
    pub fn set_tab(&mut self, tab: ShareDialogTab) {
        self.current_tab = tab;
        self.error_message = None;
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn share_name(&self) -> &str {
        &self.share_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn permission(&self) -> SharePermission {
        self.permission
    }

    pub fn max_users(&self) -> Option<u32> {
        self.max_users
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn is_sharing(&self) -> bool {
        self.is_sharing
    }

    pub fn is_already_shared(&self) -> bool {
        self.is_already_shared
    }

    pub fn existing_share(&self) -> Option<&ShareInfo> {
        self.existing_share.as_ref()
    }

    pub fn current_tab(&self) -> ShareDialogTab {
        self.current_tab
    }

    pub fn available_methods(&self) -> &[PlatformShareMethod] {
        &self.available_methods
    }

    pub fn platform_share_status(&self) -> Option<&str> {
        self.platform_share_status.as_deref()
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /
    pub fn is_method_available(&self, method: PlatformShareMethod) -> bool {
        is_share_method_available(method)
    }

    /
    pub fn get_method_unavailable_reason(&self, method: PlatformShareMethod) -> Option<String> {
        get_share_method_unavailable_reason(method)
    }

    /
    pub fn folder_info_summary(&self) -> String {
        self.path.display().to_string()
    }

    /
    pub fn available_permissions() -> Vec<SharePermission> {
        vec![
            SharePermission::ReadOnly,
            SharePermission::ReadWrite,
            SharePermission::Full,
        ]
    }

    /
    pub fn platform_info(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "This folder will be shared using Windows SMB file sharing. \
             Other users on your network will be able to access it at \\\\COMPUTERNAME\\ShareName"
        }

        #[cfg(target_os = "linux")]
        {
            "This folder will be shared using Samba (SMB). \
             Make sure the samba-common-bin package is installed. \
             Other users on your network will be able to access it."
        }

        #[cfg(target_os = "macos")]
        {
            "This folder will be shared using macOS File Sharing. \
             You may need to enable File Sharing in System Preferences > Sharing."
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            "File sharing is not supported on this platform."
        }
    }

    /
    pub fn is_sharing_supported() -> bool {
        cfg!(any(target_os = "windows", target_os = "linux", target_os = "macos"))
    }

    /
    pub fn has_platform_share_options() -> bool {
        #[cfg(target_os = "macos")]
        {
            crate::models::is_airdrop_available()
        }
        #[cfg(target_os = "windows")]
        {
            crate::models::is_nearby_share_available()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }

    /
    pub fn primary_platform_method() -> Option<PlatformShareMethod> {
        #[cfg(target_os = "macos")]
        {
            Some(PlatformShareMethod::AirDrop)
        }
        #[cfg(target_os = "windows")]
        {
            Some(PlatformShareMethod::NearbyShare)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }
}

impl Default for ShareDialog {
    fn default() -> Self {
        Self::new(PathBuf::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_dialog_new() {
        let path = PathBuf::from("/home/user/Documents");
        let dialog = ShareDialog::new(path.clone());

        assert_eq!(dialog.path(), &path);
        assert_eq!(dialog.share_name(), "Documents");
        assert_eq!(dialog.permission(), SharePermission::ReadOnly);
        assert!(!dialog.is_already_shared());
    }

    #[test]
    fn test_share_dialog_with_existing_share() {
        let path = PathBuf::from("/home/user/Documents");
        let share = ShareInfo {
            share_name: "MyDocs".to_string(),
            path: path.clone(),
            description: "My documents".to_string(),
            permission: SharePermission::ReadWrite,
            current_users: 2,
            max_users: Some(10),
        };

        let dialog = ShareDialog::new(path).with_existing_share(share);

        assert_eq!(dialog.share_name(), "MyDocs");
        assert_eq!(dialog.description(), "My documents");
        assert_eq!(dialog.permission(), SharePermission::ReadWrite);
        assert_eq!(dialog.max_users(), Some(10));
        assert!(dialog.is_already_shared());
    }

    #[test]
    fn test_validate_share_name() {
        let mut dialog = ShareDialog::new(PathBuf::from("/tmp"));

        dialog.set_share_name("ValidShare".to_string());
        assert!(dialog.validate_share_name().is_ok());

        dialog.set_share_name(String::new());
        assert!(dialog.validate_share_name().is_err());

        dialog.set_share_name("Invalid/Share".to_string());
        assert!(dialog.validate_share_name().is_err());

        dialog.set_share_name("Invalid:Share".to_string());
        assert!(dialog.validate_share_name().is_err());
    }

    #[test]
    fn test_build_config() {
        let path = PathBuf::from("/tmp/test");
        let mut dialog = ShareDialog::new(path.clone());
        dialog.set_share_name("TestShare".to_string());
        dialog.set_description("Test description".to_string());
        dialog.set_permission(SharePermission::ReadWrite);
        dialog.set_max_users(Some(5));

        let config = dialog.build_config();

        assert_eq!(config.share_name, "TestShare");
        assert_eq!(config.path, path);
        assert_eq!(config.description, "Test description");
        assert_eq!(config.permission, SharePermission::ReadWrite);
        assert_eq!(config.max_users, Some(5));
    }

    #[test]
    fn test_available_permissions() {
        let permissions = ShareDialog::available_permissions();
        assert_eq!(permissions.len(), 3);
        assert!(permissions.contains(&SharePermission::ReadOnly));
        assert!(permissions.contains(&SharePermission::ReadWrite));
        assert!(permissions.contains(&SharePermission::Full));
    }
}
