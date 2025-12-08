use crate::models::{ShareConfig, ShareError, ShareInfo, SharePermission};
use std::path::PathBuf;

/// Actions for the share dialog
#[derive(Clone, PartialEq)]
pub enum ShareDialogAction {
    Share,
    Cancel,
    StopSharing,
    ShareNameChanged(String),
    DescriptionChanged(String),
    PermissionChanged(SharePermission),
    MaxUsersChanged(Option<u32>),
}

/// State for the share dialog
pub struct ShareDialog {
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
}

impl ShareDialog {
    pub fn new(path: PathBuf) -> Self {
        let default_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Share")
            .to_string();

        Self {
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

    /// Build the share config from current state
    pub fn build_config(&self) -> ShareConfig {
        let mut config = ShareConfig::new(self.share_name.clone(), self.path.clone())
            .with_description(self.description.clone())
            .with_permission(self.permission);

        if let Some(max) = self.max_users {
            config = config.with_max_users(max);
        }

        config
    }

    /// Validate the share name
    pub fn validate_share_name(&self) -> Result<(), String> {
        if self.share_name.is_empty() {
            return Err("Share name cannot be empty".to_string());
        }

        if self.share_name.len() > 80 {
            return Err("Share name is too long (max 80 characters)".to_string());
        }

        // Check for invalid characters
        let invalid_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
        for c in invalid_chars {
            if self.share_name.contains(c) {
                return Err(format!("Share name cannot contain '{}'", c));
            }
        }

        Ok(())
    }

    /// Validate all inputs
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

    /// Execute share action
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

    /// Execute stop sharing action
    pub fn execute_stop_sharing(&mut self) {
        if let Some(callback) = &self.on_stop_sharing {
            callback(self.path.clone());
        }
    }

    /// Handle cancel action
    pub fn handle_cancel(&self) {
        if let Some(callback) = &self.on_cancel {
            callback();
        }
    }

    /// Set share name
    pub fn set_share_name(&mut self, name: String) {
        self.share_name = name;
        self.error_message = None;
    }

    /// Set description
    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    /// Set permission level
    pub fn set_permission(&mut self, permission: SharePermission) {
        self.permission = permission;
    }

    /// Set max users
    pub fn set_max_users(&mut self, max_users: Option<u32>) {
        self.max_users = max_users;
    }

    /// Set share complete (called after share operation finishes)
    pub fn set_share_complete(&mut self, success: bool, error: Option<String>) {
        self.is_sharing = false;
        if success {
            self.is_already_shared = true;
        } else {
            self.error_message = error;
        }
    }

    /// Set share removed
    pub fn set_share_removed(&mut self) {
        self.is_already_shared = false;
        self.existing_share = None;
    }

    // Getters
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

    /// Get folder info summary
    pub fn folder_info_summary(&self) -> String {
        self.path.display().to_string()
    }

    /// Get available permission options
    pub fn available_permissions() -> Vec<SharePermission> {
        vec![
            SharePermission::ReadOnly,
            SharePermission::ReadWrite,
            SharePermission::Full,
        ]
    }

    /// Get platform-specific sharing info
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

    /// Check if sharing is supported on this platform
    pub fn is_sharing_supported() -> bool {
        cfg!(any(target_os = "windows", target_os = "linux", target_os = "macos"))
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

        // Valid name
        dialog.set_share_name("ValidShare".to_string());
        assert!(dialog.validate_share_name().is_ok());

        // Empty name
        dialog.set_share_name(String::new());
        assert!(dialog.validate_share_name().is_err());

        // Invalid characters
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
