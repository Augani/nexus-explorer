use chrono::{DateTime, Local};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Preview of a single file rename operation
#[derive(Debug, Clone, PartialEq)]
pub struct RenamePreview {
    pub original: String,
    pub new_name: String,
    pub has_conflict: bool,
}

impl RenamePreview {
    pub fn new(original: String, new_name: String) -> Self {
        Self {
            original,
            new_name,
            has_conflict: false,
        }
    }
}

/// Token types that can appear in a rename pattern
#[derive(Debug, Clone, PartialEq)]
pub enum RenameToken {
    /// Literal text
    Text(String),
    /// Sequential counter {n} or {n:padding}
    Counter { start: usize, padding: usize },
    /// File modification date {date} or {date:format}
    Date { format: String },
    /// Original file extension {ext}
    Extension,
    /// Original file name without extension {name}
    OriginalName,
}

/// Error types for batch rename operations
#[derive(Debug, Clone, PartialEq)]
pub enum BatchRenameError {
    /// Rename would cause a naming conflict
    Conflict(Vec<usize>),
    /// Invalid pattern syntax
    InvalidPattern(String),
    /// File system error during rename
    FileSystemError(String),
    /// No files selected
    NoFiles,
}

/// Manages batch rename operations with pattern support and conflict detection
#[derive(Debug, Clone)]
pub struct BatchRename {
    files: Vec<PathBuf>,
    pattern: String,
    find_text: String,
    replace_text: String,
    use_find_replace: bool,
    use_regex: bool,
    case_insensitive: bool,
    preview: Vec<RenamePreview>,
    conflicts: Vec<usize>,
    counter_start: usize,
    counter_padding: usize,
    date_format: String,
}

impl Default for BatchRename {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl BatchRename {
    /// Create a new BatchRename instance with the given files
    pub fn new(files: Vec<PathBuf>) -> Self {
        let mut instance = Self {
            files,
            pattern: String::new(),
            find_text: String::new(),
            replace_text: String::new(),
            use_find_replace: false,
            use_regex: false,
            case_insensitive: false,
            preview: Vec::new(),
            conflicts: Vec::new(),
            counter_start: 1,
            counter_padding: 1,
            date_format: "%Y-%m-%d".to_string(),
        };
        instance.update_preview();
        instance
    }

    /// Get the list of files being renamed
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Set the rename pattern (e.g., "photo_{n}_{date}")
    pub fn set_pattern(&mut self, pattern: &str) {
        self.pattern = pattern.to_string();
        self.use_find_replace = false;
        self.update_preview();
    }

    /// Get the current pattern
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Set find/replace mode
    pub fn set_find_replace(&mut self, find: &str, replace: &str) {
        self.find_text = find.to_string();
        self.replace_text = replace.to_string();
        self.use_find_replace = true;
        self.update_preview();
    }

    /// Set find/replace with options
    pub fn set_find_replace_with_options(
        &mut self,
        find: &str,
        replace: &str,
        use_regex: bool,
        case_insensitive: bool,
    ) {
        self.find_text = find.to_string();
        self.replace_text = replace.to_string();
        self.use_find_replace = true;
        self.use_regex = use_regex;
        self.case_insensitive = case_insensitive;
        self.update_preview();
    }

    /// Enable or disable regex mode
    pub fn set_use_regex(&mut self, use_regex: bool) {
        self.use_regex = use_regex;
        self.update_preview();
    }

    /// Check if regex mode is enabled
    pub fn is_regex_mode(&self) -> bool {
        self.use_regex
    }

    /// Enable or disable case-insensitive matching
    pub fn set_case_insensitive(&mut self, case_insensitive: bool) {
        self.case_insensitive = case_insensitive;
        self.update_preview();
    }

    /// Check if case-insensitive mode is enabled
    pub fn is_case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /// Get find text
    pub fn find_text(&self) -> &str {
        &self.find_text
    }

    /// Get replace text
    pub fn replace_text(&self) -> &str {
        &self.replace_text
    }

    /// Check if using find/replace mode
    pub fn is_find_replace_mode(&self) -> bool {
        self.use_find_replace
    }

    /// Set counter start value
    pub fn set_counter_start(&mut self, start: usize) {
        self.counter_start = start;
        self.update_preview();
    }

    /// Set counter padding (number of digits)
    pub fn set_counter_padding(&mut self, padding: usize) {
        self.counter_padding = padding.max(1);
        self.update_preview();
    }

    /// Set date format string
    pub fn set_date_format(&mut self, format: &str) {
        self.date_format = format.to_string();
        self.update_preview();
    }

    /// Get the preview of all rename operations
    pub fn preview(&self) -> &[RenamePreview] {
        &self.preview
    }

    /// Check if there are any naming conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Get indices of files with conflicts
    pub fn conflicts(&self) -> &[usize] {
        &self.conflicts
    }

    /// Parse the pattern into tokens
    fn parse_pattern(&self) -> Vec<RenameToken> {
        let mut tokens = Vec::new();
        let mut chars = self.pattern.chars().peekable();
        let mut current_text = String::new();

        while let Some(c) = chars.next() {
            if c == '{' {
                // Save any accumulated text
                if !current_text.is_empty() {
                    tokens.push(RenameToken::Text(current_text.clone()));
                    current_text.clear();
                }

                // Parse token content
                let mut token_content = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c == '}' {
                        chars.next();
                        break;
                    }
                    token_content.push(chars.next().unwrap());
                }

                // Parse the token
                if let Some(token) = self.parse_token(&token_content) {
                    tokens.push(token);
                } else {
                    // Invalid token, treat as literal text
                    current_text.push('{');
                    current_text.push_str(&token_content);
                    current_text.push('}');
                }
            } else {
                current_text.push(c);
            }
        }

        // Add any remaining text
        if !current_text.is_empty() {
            tokens.push(RenameToken::Text(current_text));
        }

        tokens
    }

    /// Parse a single token from its content string
    fn parse_token(&self, content: &str) -> Option<RenameToken> {
        let parts: Vec<&str> = content.split(':').collect();
        let token_name = parts[0].trim().to_lowercase();

        match token_name.as_str() {
            "n" | "num" | "number" => {
                let padding = if parts.len() > 1 {
                    parts[1].parse().unwrap_or(self.counter_padding)
                } else {
                    self.counter_padding
                };
                Some(RenameToken::Counter {
                    start: self.counter_start,
                    padding,
                })
            }
            "date" => {
                let format = if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    self.date_format.clone()
                };
                Some(RenameToken::Date { format })
            }
            "ext" | "extension" => Some(RenameToken::Extension),
            "name" | "original" => Some(RenameToken::OriginalName),
            _ => None,
        }
    }

    /// Generate a new name for a file based on the pattern
    fn generate_name(&self, path: &Path, index: usize, tokens: &[RenameToken]) -> String {
        let file_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut result = String::new();

        for token in tokens {
            match token {
                RenameToken::Text(text) => {
                    result.push_str(text);
                }
                RenameToken::Counter { start, padding } => {
                    let num = start + index;
                    result.push_str(&format!("{:0width$}", num, width = *padding));
                }
                RenameToken::Date { format } => {
                    let date_str = self.get_file_date(path, format);
                    result.push_str(&date_str);
                }
                RenameToken::Extension => {
                    result.push_str(&extension);
                }
                RenameToken::OriginalName => {
                    result.push_str(&file_name);
                }
            }
        }

        // Add extension if not already included and original had one
        if !extension.is_empty() && !result.ends_with(&format!(".{}", extension)) {
            let has_ext_token = tokens.iter().any(|t| matches!(t, RenameToken::Extension));
            if !has_ext_token {
                result.push('.');
                result.push_str(&extension);
            }
        }

        result
    }

    /// Get the modification date of a file formatted according to the format string
    fn get_file_date(&self, path: &Path, format: &str) -> String {
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let datetime: DateTime<Local> = modified.into();
                return datetime.format(format).to_string();
            }
        }
        // Fallback to current date if file metadata unavailable
        Local::now().format(format).to_string()
    }

    /// Apply find/replace to a filename
    fn apply_find_replace(&self, original: &str) -> String {
        if self.find_text.is_empty() {
            return original.to_string();
        }

        if self.use_regex {
            // Build regex pattern with optional case-insensitivity
            let pattern = if self.case_insensitive {
                format!("(?i){}", &self.find_text)
            } else {
                self.find_text.clone()
            };

            match Regex::new(&pattern) {
                Ok(re) => re.replace_all(original, self.replace_text.as_str()).to_string(),
                Err(_) => original.to_string(), // Invalid regex, return original
            }
        } else if self.case_insensitive {
            // Case-insensitive literal replacement
            let lower_original = original.to_lowercase();
            let lower_find = self.find_text.to_lowercase();
            let mut result = String::new();
            let mut last_end = 0;

            for (start, _) in lower_original.match_indices(&lower_find) {
                result.push_str(&original[last_end..start]);
                result.push_str(&self.replace_text);
                last_end = start + self.find_text.len();
            }
            result.push_str(&original[last_end..]);
            result
        } else {
            // Simple literal replacement
            original.replace(&self.find_text, &self.replace_text)
        }
    }

    /// Update the preview based on current pattern/find-replace settings
    fn update_preview(&mut self) {
        self.preview.clear();
        self.conflicts.clear();

        if self.files.is_empty() {
            return;
        }

        let tokens = if !self.use_find_replace && !self.pattern.is_empty() {
            self.parse_pattern()
        } else {
            Vec::new()
        };

        // Generate new names
        for (index, path) in self.files.iter().enumerate() {
            let original = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let new_name = if self.use_find_replace {
                self.apply_find_replace(&original)
            } else if !tokens.is_empty() {
                self.generate_name(path, index, &tokens)
            } else {
                original.clone()
            };

            self.preview.push(RenamePreview::new(original, new_name));
        }

        // Detect conflicts
        self.detect_conflicts();
    }

    /// Detect naming conflicts in the preview
    fn detect_conflicts(&mut self) {
        self.conflicts.clear();

        for i in 0..self.preview.len() {
            for j in (i + 1)..self.preview.len() {
                if self.preview[i].new_name == self.preview[j].new_name {
                    if !self.conflicts.contains(&i) {
                        self.conflicts.push(i);
                    }
                    if !self.conflicts.contains(&j) {
                        self.conflicts.push(j);
                    }
                }
            }
        }

        if let Some(first_file) = self.files.first() {
            if let Some(parent) = first_file.parent() {
                for (index, preview) in self.preview.iter().enumerate() {
                    // Skip if already marked as conflict
                    if self.conflicts.contains(&index) {
                        continue;
                    }

                    let new_path = parent.join(&preview.new_name);
                    if new_path.exists() {
                        let is_our_file = self.files.iter().any(|f| f == &new_path);
                        if !is_our_file {
                            self.conflicts.push(index);
                        }
                    }
                }
            }
        }

        // Mark conflicts in preview
        for &index in &self.conflicts {
            if let Some(preview) = self.preview.get_mut(index) {
                preview.has_conflict = true;
            }
        }
    }

    /// Apply the rename operations to the file system
    pub fn apply(&self) -> Result<Vec<PathBuf>, BatchRenameError> {
        if self.files.is_empty() {
            return Err(BatchRenameError::NoFiles);
        }

        if self.has_conflicts() {
            return Err(BatchRenameError::Conflict(self.conflicts.clone()));
        }

        let mut renamed_paths = Vec::new();

        for (index, path) in self.files.iter().enumerate() {
            let preview = &self.preview[index];

            // Skip if name unchanged
            if preview.original == preview.new_name {
                renamed_paths.push(path.clone());
                continue;
            }

            let new_path = path
                .parent()
                .map(|p| p.join(&preview.new_name))
                .unwrap_or_else(|| PathBuf::from(&preview.new_name));

            std::fs::rename(path, &new_path).map_err(|e| {
                BatchRenameError::FileSystemError(format!(
                    "Failed to rename '{}' to '{}': {}",
                    path.display(),
                    new_path.display(),
                    e
                ))
            })?;

            renamed_paths.push(new_path);
        }

        Ok(renamed_paths)
    }

    /// Get the number of files that will be renamed (excluding unchanged)
    pub fn rename_count(&self) -> usize {
        self.preview
            .iter()
            .filter(|p| p.original != p.new_name)
            .count()
    }
}

#[cfg(test)]
#[path = "batch_rename_tests.rs"]
mod tests;
