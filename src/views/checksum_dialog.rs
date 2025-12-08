use crate::models::{
    HashAlgorithm, HashComparisonResult, HashProgress, HashResult,
    calculate_file_hash, compare_hashes, detect_algorithm,
};
use std::path::PathBuf;


#[derive(Clone, PartialEq)]
pub enum ChecksumDialogAction {
    Calculate,
    Cancel,
    AlgorithmChanged(HashAlgorithm),
    ComparisonHashChanged(String),
    CopyHash,
    Close,
}


pub struct ChecksumDialog {
    file_path: PathBuf,
    file_name: String,
    file_size: u64,
    selected_algorithm: HashAlgorithm,
    calculated_hash: Option<HashResult>,
    comparison_hash: String,
    comparison_result: Option<HashComparisonResult>,
    is_calculating: bool,
    progress: Option<HashProgress>,
    error_message: Option<String>,
    on_copy: Option<Box<dyn Fn(String) + Send + Sync>>,
    on_close: Option<Box<dyn Fn() + Send + Sync>>,
}

impl ChecksumDialog {
    pub fn new(file_path: PathBuf) -> Self {
        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        let file_size = std::fs::metadata(&file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Self {
            file_path,
            file_name,
            file_size,
            selected_algorithm: HashAlgorithm::Sha256,
            calculated_hash: None,
            comparison_hash: String::new(),
            comparison_result: None,
            is_calculating: false,
            progress: None,
            error_message: None,
            on_copy: None,
            on_close: None,
        }
    }

    pub fn with_on_copy<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_copy = Some(Box::new(callback));
        self
    }

    pub fn with_on_close<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_close = Some(Box::new(callback));
        self
    }


    pub fn set_algorithm(&mut self, algorithm: HashAlgorithm) {
        self.selected_algorithm = algorithm;
        self.calculated_hash = None;
        self.comparison_result = None;
        self.error_message = None;
    }


    pub fn set_comparison_hash(&mut self, hash: String) {
        self.comparison_hash = hash;
        if let Some(detected) = detect_algorithm(&self.comparison_hash) {
            self.selected_algorithm = detected;
        }
        self.update_comparison();
    }


    pub fn start_calculation(&mut self) {
        self.is_calculating = true;
        self.error_message = None;
        self.progress = Some(HashProgress::new(0, self.file_size));
    }


    pub fn update_progress(&mut self, progress: HashProgress) {
        self.progress = Some(progress);
    }


    pub fn set_result(&mut self, result: Result<HashResult, String>) {
        self.is_calculating = false;
        self.progress = None;
        
        match result {
            Ok(hash_result) => {
                self.calculated_hash = Some(hash_result);
                self.error_message = None;
                self.update_comparison();
            }
            Err(error) => {
                self.error_message = Some(error);
            }
        }
    }


    pub fn calculate_sync(&mut self) {
        self.start_calculation();
        
        match calculate_file_hash(&self.file_path, self.selected_algorithm) {
            Ok(hash) => {
                self.set_result(Ok(HashResult::new(self.selected_algorithm, hash)));
            }
            Err(e) => {
                self.set_result(Err(e.to_string()));
            }
        }
    }


    fn update_comparison(&mut self) {
        if let Some(ref hash_result) = self.calculated_hash {
            if !self.comparison_hash.is_empty() {
                self.comparison_result = Some(compare_hashes(
                    &hash_result.hash,
                    &self.comparison_hash,
                ));
            } else {
                self.comparison_result = None;
            }
        }
    }


    pub fn copy_hash(&self) {
        if let Some(ref hash_result) = self.calculated_hash {
            if let Some(ref callback) = self.on_copy {
                callback(hash_result.hash.clone());
            }
        }
    }


    pub fn handle_close(&self) {
        if let Some(ref callback) = self.on_close {
            callback();
        }
    }

    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn file_size_formatted(&self) -> String {
        format_size(self.file_size)
    }

    pub fn selected_algorithm(&self) -> HashAlgorithm {
        self.selected_algorithm
    }

    pub fn calculated_hash(&self) -> Option<&HashResult> {
        self.calculated_hash.as_ref()
    }

    pub fn hash_string(&self) -> Option<&str> {
        self.calculated_hash.as_ref().map(|r| r.hash.as_str())
    }

    pub fn comparison_hash(&self) -> &str {
        &self.comparison_hash
    }

    pub fn comparison_result(&self) -> Option<HashComparisonResult> {
        self.comparison_result
    }

    pub fn is_calculating(&self) -> bool {
        self.is_calculating
    }

    pub fn progress(&self) -> Option<&HashProgress> {
        self.progress.as_ref()
    }

    pub fn progress_percentage(&self) -> f64 {
        self.progress.as_ref().map(|p| p.percentage).unwrap_or(0.0)
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn available_algorithms(&self) -> &'static [HashAlgorithm] {
        HashAlgorithm::all()
    }

    pub fn has_hash(&self) -> bool {
        self.calculated_hash.is_some()
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

impl Default for ChecksumDialog {
    fn default() -> Self {
        Self::new(PathBuf::new())
    }
}
