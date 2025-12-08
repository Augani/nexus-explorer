use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

use crate::models::{CloudSyncStatus, FileEntry, TagId};

/// Unique identifier for a smart folder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SmartFolderId(pub u64);

impl SmartFolderId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Errors that can occur during smart folder operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SmartFolderError {
    #[error("Smart folder not found: {0}")]
    NotFound(u64),

    #[error("Smart folder with name '{0}' already exists")]
    DuplicateName(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Result type for smart folder operations
pub type SmartFolderResult<T> = std::result::Result<T, SmartFolderError>;

/// Date range filter for search queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateFilter {
    /// Files modified within the last N days
    LastDays(u32),
    /// Files modified within the last N weeks
    LastWeeks(u32),
    /// Files modified within the last N months
    LastMonths(u32),
    /// Files modified between two dates
    Between(u64, u64),
    /// Files modified before a date
    Before(u64),
    /// Files modified after a date
    After(u64),
}

impl DateFilter {
    /// Checks if a file's modification time matches this filter
    pub fn matches(&self, modified: SystemTime) -> bool {
        let file_secs = modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        match self {
            DateFilter::LastDays(days) => {
                let threshold = now_secs.saturating_sub(*days as u64 * 24 * 60 * 60);
                file_secs >= threshold
            }
            DateFilter::LastWeeks(weeks) => {
                let threshold = now_secs.saturating_sub(*weeks as u64 * 7 * 24 * 60 * 60);
                file_secs >= threshold
            }
            DateFilter::LastMonths(months) => {
                let threshold = now_secs.saturating_sub(*months as u64 * 30 * 24 * 60 * 60);
                file_secs >= threshold
            }
            DateFilter::Between(start, end) => file_secs >= *start && file_secs <= *end,
            DateFilter::Before(timestamp) => file_secs < *timestamp,
            DateFilter::After(timestamp) => file_secs > *timestamp,
        }
    }
}

/// Size range filter for search queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeFilter {
    /// Files smaller than N bytes
    SmallerThan(u64),
    /// Files larger than N bytes
    LargerThan(u64),
    /// Files between min and max bytes
    Between(u64, u64),
    /// Empty files (0 bytes)
    Empty,
    /// Non-empty files
    NonEmpty,
}

impl SizeFilter {
    /// Checks if a file's size matches this filter
    pub fn matches(&self, size: u64) -> bool {
        match self {
            SizeFilter::SmallerThan(max) => size < *max,
            SizeFilter::LargerThan(min) => size > *min,
            SizeFilter::Between(min, max) => size >= *min && size <= *max,
            SizeFilter::Empty => size == 0,
            SizeFilter::NonEmpty => size > 0,
        }
    }
}

/// A search query that defines what files a smart folder contains
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Text pattern to match against file names (fuzzy match)
    #[serde(default)]
    pub text: Option<String>,

    /// File extensions to include (e.g., ["rs", "toml"])
    #[serde(default)]
    pub file_types: Vec<String>,

    /// Date range filter
    #[serde(default)]
    pub date_filter: Option<DateFilter>,

    /// Size range filter
    #[serde(default)]
    pub size_filter: Option<SizeFilter>,

    /// Tags that files must have (any of these)
    #[serde(default)]
    pub tags: Vec<TagId>,

    /// Directories to search in
    #[serde(default)]
    pub locations: Vec<PathBuf>,

    /// Whether to search recursively in subdirectories
    #[serde(default = "default_recursive")]
    pub recursive: bool,

    /// Whether to include hidden files
    #[serde(default)]
    pub include_hidden: bool,

    /// Only include directories (not files)
    #[serde(default)]
    pub directories_only: bool,

    /// Only include files (not directories)
    #[serde(default)]
    pub files_only: bool,
}

fn default_recursive() -> bool {
    true
}

impl SearchQuery {
    /// Creates a new empty search query
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a query that searches for text in file names
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// Adds a text pattern to match
    pub fn text(mut self, pattern: impl Into<String>) -> Self {
        self.text = Some(pattern.into());
        self
    }

    /// Adds file type filters
    pub fn file_types(mut self, types: Vec<String>) -> Self {
        self.file_types = types;
        self
    }

    /// Adds a date filter
    pub fn date_filter(mut self, filter: DateFilter) -> Self {
        self.date_filter = Some(filter);
        self
    }

    /// Adds a size filter
    pub fn size_filter(mut self, filter: SizeFilter) -> Self {
        self.size_filter = Some(filter);
        self
    }

    /// Adds tag filters
    pub fn tags(mut self, tags: Vec<TagId>) -> Self {
        self.tags = tags;
        self
    }

    /// Adds search locations
    pub fn locations(mut self, locations: Vec<PathBuf>) -> Self {
        self.locations = locations;
        self
    }

    /// Sets recursive search
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Sets whether to include hidden files
    pub fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Checks if a file entry matches this query
    pub fn matches(&self, entry: &FileEntry, file_tags: &HashSet<TagId>) -> bool {
        if let Some(ref pattern) = self.text {
            let name_lower = entry.name.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            if !name_lower.contains(&pattern_lower) {
                return false;
            }
        }

        if !self.file_types.is_empty() {
            let ext = entry
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !self.file_types.iter().any(|t| t.to_lowercase() == ext) {
                return false;
            }
        }

        if let Some(ref date_filter) = self.date_filter {
            if !date_filter.matches(entry.modified) {
                return false;
            }
        }

        if let Some(ref size_filter) = self.size_filter {
            if !entry.is_dir && !size_filter.matches(entry.size) {
                return false;
            }
        }

        if !self.tags.is_empty() {
            let has_matching_tag = self.tags.iter().any(|tag| file_tags.contains(tag));
            if !has_matching_tag {
                return false;
            }
        }

        if !self.include_hidden && entry.name.starts_with('.') {
            return false;
        }

        if self.directories_only && !entry.is_dir {
            return false;
        }

        if self.files_only && entry.is_dir {
            return false;
        }

        true
    }

    /// Returns true if this query has any filters set
    pub fn has_filters(&self) -> bool {
        self.text.is_some()
            || !self.file_types.is_empty()
            || self.date_filter.is_some()
            || self.size_filter.is_some()
            || !self.tags.is_empty()
            || self.directories_only
            || self.files_only
    }

    /// Returns a human-readable description of this query
    pub fn description(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref text) = self.text {
            parts.push(format!("name contains \"{}\"", text));
        }

        if !self.file_types.is_empty() {
            parts.push(format!("type: {}", self.file_types.join(", ")));
        }

        if let Some(ref date) = self.date_filter {
            let desc = match date {
                DateFilter::LastDays(d) => format!("modified in last {} days", d),
                DateFilter::LastWeeks(w) => format!("modified in last {} weeks", w),
                DateFilter::LastMonths(m) => format!("modified in last {} months", m),
                DateFilter::Between(_, _) => "modified between dates".to_string(),
                DateFilter::Before(_) => "modified before date".to_string(),
                DateFilter::After(_) => "modified after date".to_string(),
            };
            parts.push(desc);
        }

        if let Some(ref size) = self.size_filter {
            let desc = match size {
                SizeFilter::SmallerThan(s) => format!("smaller than {} bytes", s),
                SizeFilter::LargerThan(s) => format!("larger than {} bytes", s),
                SizeFilter::Between(min, max) => format!("size {} - {} bytes", min, max),
                SizeFilter::Empty => "empty files".to_string(),
                SizeFilter::NonEmpty => "non-empty files".to_string(),
            };
            parts.push(desc);
        }

        if !self.tags.is_empty() {
            parts.push(format!("{} tags", self.tags.len()));
        }

        if parts.is_empty() {
            "All files".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// A smart folder that dynamically shows files matching a query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmartFolder {
    pub id: SmartFolderId,
    pub name: String,
    pub query: SearchQuery,
    /// Icon identifier for display
    #[serde(default)]
    pub icon: String,
    /// When the smart folder was created
    #[serde(default)]
    pub created: u64,
    /// When the smart folder was last modified
    #[serde(default)]
    pub modified: u64,
}

impl SmartFolder {
    pub fn new(id: SmartFolderId, name: String, query: SearchQuery) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            id,
            name,
            query,
            icon: "search".to_string(),
            created: now,
            modified: now,
        }
    }

    /// Updates the query and modification time
    pub fn update_query(&mut self, query: SearchQuery) {
        self.query = query;
        self.modified = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
}

/// Manages smart folders - creating, editing, deleting, and executing queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderManager {
    folders: Vec<SmartFolder>,
    next_id: u64,
}

impl Default for SmartFolderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartFolderManager {
    /// Creates a new empty SmartFolderManager
    pub fn new() -> Self {
        Self {
            folders: Vec::new(),
            next_id: 1,
        }
    }

    /// Creates a new smart folder with the given name and query
    pub fn create(&mut self, name: String, query: SearchQuery) -> SmartFolderResult<SmartFolderId> {
        if self
            .folders
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case(&name))
        {
            return Err(SmartFolderError::DuplicateName(name));
        }

        let id = SmartFolderId::new(self.next_id);
        self.next_id += 1;

        let folder = SmartFolder::new(id, name, query);
        self.folders.push(folder);

        Ok(id)
    }

    /// Deletes a smart folder by ID
    pub fn delete(&mut self, id: SmartFolderId) -> SmartFolderResult<SmartFolder> {
        if let Some(index) = self.folders.iter().position(|f| f.id == id) {
            Ok(self.folders.remove(index))
        } else {
            Err(SmartFolderError::NotFound(id.0))
        }
    }

    /// Updates a smart folder's query
    pub fn update(&mut self, id: SmartFolderId, query: SearchQuery) -> SmartFolderResult<()> {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.id == id) {
            folder.update_query(query);
            Ok(())
        } else {
            Err(SmartFolderError::NotFound(id.0))
        }
    }

    /// Renames a smart folder
    pub fn rename(&mut self, id: SmartFolderId, new_name: String) -> SmartFolderResult<()> {
        if self
            .folders
            .iter()
            .any(|f| f.id != id && f.name.eq_ignore_ascii_case(&new_name))
        {
            return Err(SmartFolderError::DuplicateName(new_name));
        }

        if let Some(folder) = self.folders.iter_mut().find(|f| f.id == id) {
            folder.name = new_name;
            folder.modified = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Ok(())
        } else {
            Err(SmartFolderError::NotFound(id.0))
        }
    }

    /// Executes a smart folder's query against a list of file entries
    ///
    /// This is the core method that filters files based on the smart folder's query.
    /// The `file_tags` function provides tag information for each file.
    pub fn execute<F>(
        &self,
        id: SmartFolderId,
        entries: &[FileEntry],
        file_tags: F,
    ) -> SmartFolderResult<Vec<FileEntry>>
    where
        F: Fn(&Path) -> HashSet<TagId>,
    {
        let folder = self.get(id).ok_or(SmartFolderError::NotFound(id.0))?;
        Ok(self.execute_query(&folder.query, entries, file_tags))
    }

    /// Executes a query directly against a list of file entries
    pub fn execute_query<F>(
        &self,
        query: &SearchQuery,
        entries: &[FileEntry],
        file_tags: F,
    ) -> Vec<FileEntry>
    where
        F: Fn(&Path) -> HashSet<TagId>,
    {
        entries
            .iter()
            .filter(|entry| {
                let tags = file_tags(&entry.path);
                query.matches(entry, &tags)
            })
            .cloned()
            .collect()
    }

    /// Gets a smart folder by ID
    pub fn get(&self, id: SmartFolderId) -> Option<&SmartFolder> {
        self.folders.iter().find(|f| f.id == id)
    }

    /// Gets a mutable reference to a smart folder by ID
    pub fn get_mut(&mut self, id: SmartFolderId) -> Option<&mut SmartFolder> {
        self.folders.iter_mut().find(|f| f.id == id)
    }

    /// Gets a smart folder by name (case-insensitive)
    pub fn get_by_name(&self, name: &str) -> Option<&SmartFolder> {
        self.folders
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// Returns all smart folders
    pub fn folders(&self) -> &[SmartFolder] {
        &self.folders
    }

    /// Returns the number of smart folders
    pub fn len(&self) -> usize {
        self.folders.len()
    }

    /// Returns true if there are no smart folders
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }

    /// Returns the config file path
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer")
            .join("smart_folders.json")
    }

    /// Saves smart folders to the config file
    pub fn save(&self) -> SmartFolderResult<()> {
        let config_path = Self::config_path();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SmartFolderError::Io(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SmartFolderError::Serialization(e.to_string()))?;

        std::fs::write(&config_path, json).map_err(|e| SmartFolderError::Io(e.to_string()))?;

        Ok(())
    }

    /// Loads smart folders from the config file
    pub fn load() -> SmartFolderResult<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let json = std::fs::read_to_string(&config_path)
            .map_err(|e| SmartFolderError::Io(e.to_string()))?;

        let mut manager: SmartFolderManager = serde_json::from_str(&json)
            .map_err(|e| SmartFolderError::Serialization(e.to_string()))?;

        // Calculate next_id from existing folders
        manager.next_id = manager.folders.iter().map(|f| f.id.0).max().unwrap_or(0) + 1;

        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileType, IconKey};

    fn create_test_entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
        let file_type = if is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };
        let icon_key = if is_dir {
            IconKey::Directory
        } else {
            IconKey::GenericFile
        };
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(format!("/test/{}", name)),
            is_dir,
            size,
            modified: SystemTime::now(),
            file_type,
            icon_key,
            linux_permissions: None,
            sync_status: CloudSyncStatus::None,
            is_symlink: false,
            symlink_target: None,
            is_broken_symlink: false,
        }
    }

    #[test]
    fn test_smart_folder_manager_new() {
        let manager = SmartFolderManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_create_smart_folder() {
        let mut manager = SmartFolderManager::new();
        let query = SearchQuery::with_text("test");

        let result = manager.create("My Search".to_string(), query);
        assert!(result.is_ok());
        assert_eq!(manager.len(), 1);

        let folder = manager.get(result.unwrap()).unwrap();
        assert_eq!(folder.name, "My Search");
    }

    #[test]
    fn test_create_duplicate_name() {
        let mut manager = SmartFolderManager::new();
        let query = SearchQuery::new();

        manager.create("Test".to_string(), query.clone()).unwrap();
        let result = manager.create("test".to_string(), query);

        assert!(matches!(result, Err(SmartFolderError::DuplicateName(_))));
    }

    #[test]
    fn test_delete_smart_folder() {
        let mut manager = SmartFolderManager::new();
        let query = SearchQuery::new();

        let id = manager.create("Test".to_string(), query).unwrap();
        assert_eq!(manager.len(), 1);

        let deleted = manager.delete(id);
        assert!(deleted.is_ok());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_delete_not_found() {
        let mut manager = SmartFolderManager::new();
        let result = manager.delete(SmartFolderId::new(999));
        assert!(matches!(result, Err(SmartFolderError::NotFound(999))));
    }

    #[test]
    fn test_update_query() {
        let mut manager = SmartFolderManager::new();
        let query1 = SearchQuery::with_text("old");
        let query2 = SearchQuery::with_text("new");

        let id = manager.create("Test".to_string(), query1).unwrap();
        manager.update(id, query2).unwrap();

        let folder = manager.get(id).unwrap();
        assert_eq!(folder.query.text, Some("new".to_string()));
    }

    #[test]
    fn test_rename_smart_folder() {
        let mut manager = SmartFolderManager::new();
        let query = SearchQuery::new();

        let id = manager.create("Old Name".to_string(), query).unwrap();
        manager.rename(id, "New Name".to_string()).unwrap();

        let folder = manager.get(id).unwrap();
        assert_eq!(folder.name, "New Name");
    }

    #[test]
    fn test_query_text_match() {
        let query = SearchQuery::with_text("test");
        let entry = create_test_entry("test_file.rs", false, 100);
        let empty_tags = HashSet::new();

        assert!(query.matches(&entry, &empty_tags));
    }

    #[test]
    fn test_query_text_no_match() {
        let query = SearchQuery::with_text("xyz");
        let entry = create_test_entry("test_file.rs", false, 100);
        let empty_tags = HashSet::new();

        assert!(!query.matches(&entry, &empty_tags));
    }

    #[test]
    fn test_query_file_types() {
        let query = SearchQuery::new().file_types(vec!["rs".to_string(), "toml".to_string()]);

        let rs_file = create_test_entry("main.rs", false, 100);
        let txt_file = create_test_entry("readme.txt", false, 100);
        let empty_tags = HashSet::new();

        assert!(query.matches(&rs_file, &empty_tags));
        assert!(!query.matches(&txt_file, &empty_tags));
    }

    #[test]
    fn test_query_size_filter() {
        let query = SearchQuery::new().size_filter(SizeFilter::LargerThan(50));

        let large_file = create_test_entry("large.txt", false, 100);
        let small_file = create_test_entry("small.txt", false, 10);
        let empty_tags = HashSet::new();

        assert!(query.matches(&large_file, &empty_tags));
        assert!(!query.matches(&small_file, &empty_tags));
    }

    #[test]
    fn test_query_hidden_files() {
        let query_no_hidden = SearchQuery::new().include_hidden(false);
        let query_with_hidden = SearchQuery::new().include_hidden(true);

        let hidden_file = create_test_entry(".hidden", false, 100);
        let normal_file = create_test_entry("normal.txt", false, 100);
        let empty_tags = HashSet::new();

        assert!(!query_no_hidden.matches(&hidden_file, &empty_tags));
        assert!(query_no_hidden.matches(&normal_file, &empty_tags));
        assert!(query_with_hidden.matches(&hidden_file, &empty_tags));
    }

    #[test]
    fn test_query_directories_only() {
        let query = SearchQuery {
            directories_only: true,
            ..Default::default()
        };

        let dir = create_test_entry("folder", true, 0);
        let file = create_test_entry("file.txt", false, 100);
        let empty_tags = HashSet::new();

        assert!(query.matches(&dir, &empty_tags));
        assert!(!query.matches(&file, &empty_tags));
    }

    #[test]
    fn test_query_files_only() {
        let query = SearchQuery {
            files_only: true,
            ..Default::default()
        };

        let dir = create_test_entry("folder", true, 0);
        let file = create_test_entry("file.txt", false, 100);
        let empty_tags = HashSet::new();

        assert!(!query.matches(&dir, &empty_tags));
        assert!(query.matches(&file, &empty_tags));
    }

    #[test]
    fn test_query_with_tags() {
        let tag1 = TagId::new(1);
        let tag2 = TagId::new(2);

        let query = SearchQuery::new().tags(vec![tag1]);
        let entry = create_test_entry("file.txt", false, 100);

        let mut tags_with_match = HashSet::new();
        tags_with_match.insert(tag1);

        let mut tags_without_match = HashSet::new();
        tags_without_match.insert(tag2);

        let empty_tags = HashSet::new();

        assert!(query.matches(&entry, &tags_with_match));
        assert!(!query.matches(&entry, &tags_without_match));
        assert!(!query.matches(&entry, &empty_tags));
    }

    #[test]
    fn test_execute_query() {
        let mut manager = SmartFolderManager::new();
        let query = SearchQuery::with_text("test");
        let id = manager.create("Test Search".to_string(), query).unwrap();

        let entries = vec![
            create_test_entry("test_file.rs", false, 100),
            create_test_entry("other.rs", false, 100),
            create_test_entry("test_dir", true, 0),
        ];

        let results = manager.execute(id, &entries, |_| HashSet::new()).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|e| e.name == "test_file.rs"));
        assert!(results.iter().any(|e| e.name == "test_dir"));
    }

    #[test]
    fn test_date_filter_last_days() {
        let filter = DateFilter::LastDays(7);
        let recent = SystemTime::now();

        assert!(filter.matches(recent));
    }

    #[test]
    fn test_size_filter_between() {
        let filter = SizeFilter::Between(50, 150);

        assert!(filter.matches(100));
        assert!(!filter.matches(10));
        assert!(!filter.matches(200));
    }

    #[test]
    fn test_query_description() {
        let query = SearchQuery::with_text("test")
            .file_types(vec!["rs".to_string()])
            .size_filter(SizeFilter::LargerThan(1000));

        let desc = query.description();
        assert!(desc.contains("test"));
        assert!(desc.contains("rs"));
        assert!(desc.contains("larger than"));
    }

    #[test]
    fn test_serialization_round_trip() {
        let mut manager = SmartFolderManager::new();
        let query = SearchQuery::with_text("test")
            .file_types(vec!["rs".to_string()])
            .date_filter(DateFilter::LastDays(7));

        manager.create("Test Search".to_string(), query).unwrap();

        let json = serde_json::to_string(&manager).unwrap();
        let loaded: SmartFolderManager = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.folders()[0].name, "Test Search");
        assert_eq!(loaded.folders()[0].query.text, Some("test".to_string()));
    }
}
