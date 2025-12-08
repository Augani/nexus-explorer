use crate::models::{Device, DeviceId, FileSystemType, FormatOptions};
use gpui::*;


#[derive(Clone, PartialEq)]
pub enum FormatDialogAction {
    Format,
    Cancel,
    FilesystemChanged(FileSystemType),
    LabelChanged(String),
    QuickFormatChanged(bool),
    CompressionChanged(bool),
}


pub struct FormatDialog {
    device: Device,
    filesystem: FileSystemType,
    label: String,
    quick_format: bool,
    enable_compression: bool,
    available_filesystems: Vec<FileSystemType>,
    error_message: Option<String>,
    is_formatting: bool,
    show_confirmation: bool,
    on_format: Option<Box<dyn Fn(DeviceId, FormatOptions) + Send + Sync>>,
    on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

impl FormatDialog {
    pub fn new(device: Device, available_filesystems: Vec<FileSystemType>) -> Self {
        let default_fs = available_filesystems.first().copied().unwrap_or(FileSystemType::ExFat);
        
        Self {
            device,
            filesystem: default_fs,
            label: String::new(),
            quick_format: true,
            enable_compression: false,
            available_filesystems,
            error_message: None,
            is_formatting: false,
            show_confirmation: false,
            on_format: None,
            on_cancel: None,
        }
    }

    pub fn with_on_format<F>(mut self, callback: F) -> Self
    where
        F: Fn(DeviceId, FormatOptions) + Send + Sync + 'static,
    {
        self.on_format = Some(Box::new(callback));
        self
    }

    pub fn with_on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_cancel = Some(Box::new(callback));
        self
    }



    pub fn build_options(&self) -> FormatOptions {
        FormatOptions {
            filesystem: self.filesystem,
            label: self.label.clone(),
            quick_format: self.quick_format,
            enable_compression: self.enable_compression,
        }
    }


    pub fn validate_label(&self) -> Result<(), String> {
        let label = &self.label;
        
        let max_len = match self.filesystem {
            FileSystemType::Fat32 => 11,
            FileSystemType::ExFat => 11,
            FileSystemType::Ntfs => 32,
            FileSystemType::ReFS => 32,
            FileSystemType::Apfs => 255,
            FileSystemType::HfsPlus => 255,
            FileSystemType::Ext4 => 16,
            FileSystemType::Btrfs => 255,
            FileSystemType::Xfs => 12,
        };

        if label.len() > max_len {
            return Err(format!(
                "Label too long for {}. Maximum {} characters allowed.",
                self.filesystem.display_name(),
                max_len
            ));
        }

        match self.filesystem {
            FileSystemType::Fat32 | FileSystemType::ExFat => {
                let invalid_chars = ['*', '?', '<', '>', '|', '"', ':', '/', '\\'];
                for c in invalid_chars {
                    if label.contains(c) {
                        return Err(format!("Label cannot contain '{}'", c));
                    }
                }
            }
            FileSystemType::Ntfs | FileSystemType::ReFS => {
                let invalid_chars = ['*', '?', '<', '>', '|', '"', '/', '\\'];
                for c in invalid_chars {
                    if label.contains(c) {
                        return Err(format!("Label cannot contain '{}'", c));
                    }
                }
            }
            _ => {
                if label.contains('/') {
                    return Err("Label cannot contain '/'".to_string());
                }
            }
        }

        Ok(())
    }


    pub fn validate(&self) -> Result<(), String> {
        self.validate_label()
    }


    pub fn request_format(&mut self) {
        if let Err(msg) = self.validate() {
            self.error_message = Some(msg);
            return;
        }
        self.error_message = None;
        self.show_confirmation = true;
    }


    pub fn confirm_format(&mut self) {
        self.show_confirmation = false;
        self.is_formatting = true;
        self.error_message = None;

        let options = self.build_options();
        if let Some(callback) = &self.on_format {
            callback(self.device.id, options);
        }
    }


    pub fn cancel_confirmation(&mut self) {
        self.show_confirmation = false;
    }


    pub fn handle_cancel(&self) {
        if let Some(callback) = &self.on_cancel {
            callback();
        }
    }


    pub fn set_filesystem(&mut self, filesystem: FileSystemType) {
        self.filesystem = filesystem;
        self.error_message = None;
        
        if filesystem != FileSystemType::Ntfs {
            self.enable_compression = false;
        }
    }


    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.error_message = None;
    }


    pub fn set_quick_format(&mut self, quick: bool) {
        self.quick_format = quick;
    }


    pub fn set_compression(&mut self, enabled: bool) {
        if self.filesystem == FileSystemType::Ntfs {
            self.enable_compression = enabled;
        }
    }


    pub fn set_format_complete(&mut self, success: bool, error: Option<String>) {
        self.is_formatting = false;
        if !success {
            self.error_message = error;
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn filesystem(&self) -> FileSystemType {
        self.filesystem
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn quick_format(&self) -> bool {
        self.quick_format
    }

    pub fn compression_enabled(&self) -> bool {
        self.enable_compression
    }

    pub fn compression_available(&self) -> bool {
        self.filesystem == FileSystemType::Ntfs
    }

    pub fn available_filesystems(&self) -> &[FileSystemType] {
        &self.available_filesystems
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn is_formatting(&self) -> bool {
        self.is_formatting
    }

    pub fn show_confirmation(&self) -> bool {
        self.show_confirmation
    }


    pub fn compatibility_info(&self) -> &'static str {
        self.filesystem.compatibility_info()
    }


    pub fn device_info_summary(&self) -> String {
        let size = format_size(self.device.total_space);
        format!(
            "{} - {} ({})",
            self.device.name,
            size,
            self.device.path.display()
        )
    }


    pub fn format_warning(&self) -> String {
        format!(
            "WARNING: All data on \"{}\" will be permanently erased. This action cannot be undone.",
            self.device.name
        )
    }
}


fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

impl Default for FormatDialog {
    fn default() -> Self {
        let dummy_device = Device::new(
            DeviceId::new(0),
            "Unknown".to_string(),
            std::path::PathBuf::new(),
            crate::models::DeviceType::ExternalDrive,
        );
        Self::new(dummy_device, vec![FileSystemType::ExFat])
    }
}
