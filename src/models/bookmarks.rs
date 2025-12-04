use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Maximum number of bookmarks allowed
pub const MAX_BOOKMARKS: usize = 50;

/// Maximum number of recent locations to track
pub const MAX_RECENT_LOCATIONS: usize = 20;

/// Unique identifier for a bookmark
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookmarkId(pub u64);

impl BookmarkId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Error types for bookmark operations
#[derive(Debug, Error, PartialEq, Clone)]
pub enum BookmarkError {
    #[error("Maximum bookmarks limit ({0}) reached")]
    MaxReached(usize),

    #[error("Bookmark already exists for this path")]
    AlreadyExists,

    #[error("Bookmark not found: {0}")]
    NotFound(u64),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Keyboard shortcut binding for a bookmark
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyBinding {
    /// Key code (e.g., "1", "2", "a", "b")
    pub key: String,
    /// Whether Cmd/Ctrl is required
    pub cmd: bool,
    /// Whether Shift is required
    pub shift: bool,
    /// Whether Alt/Option is required
    pub alt: bool,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            cmd: true,
            shift: false,
            alt: false,
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// Format the key binding for display
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.cmd {
            #[cfg(target_os = "macos")]
            parts.push("⌘");
            #[cfg(not(target_os = "macos"))]
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("⇧");
        }
        if self.alt {
            #[cfg(target_os = "macos")]
            parts.push("⌥");
            #[cfg(not(target_os = "macos"))]
            parts.push("Alt");
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

/// A single bookmark entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub id: BookmarkId,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub shortcut: Option<KeyBinding>,
    #[serde(default)]
    pub is_valid: bool,
}

impl Bookmark {
    pub fn new(id: BookmarkId, path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let is_valid = path.exists();

        Self {
            id,
            name,
            path,
            shortcut: None,
            is_valid,
        }
    }

    pub fn with_name(id: BookmarkId, path: PathBuf, name: String) -> Self {
        let is_valid = path.exists();
        Self {
            id,
            name,
            path,
            shortcut: None,
            is_valid,
        }
    }

    /// Validate that the path still exists
    pub fn validate(&mut self) -> bool {
        self.is_valid = self.path.exists();
        self.is_valid
    }
}

/// Manages user's bookmarks and recent locations with persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkManager {
    bookmarks: Vec<Bookmark>,
    recent_locations: VecDeque<PathBuf>,
    #[serde(skip)]
    next_id: u64,
    #[serde(skip)]
    max_bookmarks: usize,
    #[serde(skip)]
    max_recent: usize,
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BookmarkManager {
    pub fn new() -> Self {
        Self {
            bookmarks: Vec::new(),
            recent_locations: VecDeque::new(),
            next_id: 1,
            max_bookmarks: MAX_BOOKMARKS,
            max_recent: MAX_RECENT_LOCATIONS,
        }
    }

    /// Add a new bookmark from a path
    pub fn add(&mut self, path: PathBuf) -> Result<BookmarkId, BookmarkError> {
        if self.bookmarks.len() >= self.max_bookmarks {
            return Err(BookmarkError::MaxReached(self.max_bookmarks));
        }

        if self.bookmarks.iter().any(|b| b.path == path) {
            return Err(BookmarkError::AlreadyExists);
        }

        if !path.exists() {
            return Err(BookmarkError::InvalidPath(path.display().to_string()));
        }

        let id = BookmarkId::new(self.next_id);
        self.next_id += 1;
        self.bookmarks.push(Bookmark::new(id, path));
        Ok(id)
    }

    /// Add a bookmark with a custom name
    pub fn add_with_name(
        &mut self,
        path: PathBuf,
        name: String,
    ) -> Result<BookmarkId, BookmarkError> {
        if self.bookmarks.len() >= self.max_bookmarks {
            return Err(BookmarkError::MaxReached(self.max_bookmarks));
        }

        if self.bookmarks.iter().any(|b| b.path == path) {
            return Err(BookmarkError::AlreadyExists);
        }

        if !path.exists() {
            return Err(BookmarkError::InvalidPath(path.display().to_string()));
        }

        let id = BookmarkId::new(self.next_id);
        self.next_id += 1;
        self.bookmarks.push(Bookmark::with_name(id, path, name));
        Ok(id)
    }

    /// Remove a bookmark by ID
    pub fn remove(&mut self, id: BookmarkId) -> Result<Bookmark, BookmarkError> {
        if let Some(index) = self.bookmarks.iter().position(|b| b.id == id) {
            Ok(self.bookmarks.remove(index))
        } else {
            Err(BookmarkError::NotFound(id.0))
        }
    }

    /// Rename a bookmark
    pub fn rename(&mut self, id: BookmarkId, name: String) -> Result<(), BookmarkError> {
        if let Some(bookmark) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            bookmark.name = name;
            Ok(())
        } else {
            Err(BookmarkError::NotFound(id.0))
        }
    }

    /// Set a keyboard shortcut for a bookmark
    pub fn set_shortcut(
        &mut self,
        id: BookmarkId,
        shortcut: Option<KeyBinding>,
    ) -> Result<(), BookmarkError> {
        if let Some(ref new_shortcut) = shortcut {
            for bookmark in &mut self.bookmarks {
                if let Some(ref existing) = bookmark.shortcut {
                    if existing == new_shortcut && bookmark.id != id {
                        bookmark.shortcut = None;
                    }
                }
            }
        }

        if let Some(bookmark) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            bookmark.shortcut = shortcut;
            Ok(())
        } else {
            Err(BookmarkError::NotFound(id.0))
        }
    }

    /// Get all bookmarks
    pub fn bookmarks(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    /// Get a bookmark by ID
    pub fn get(&self, id: BookmarkId) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.id == id)
    }

    /// Get a bookmark by path
    pub fn get_by_path(&self, path: &PathBuf) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| &b.path == path)
    }

    /// Find a bookmark by keyboard shortcut
    pub fn find_by_shortcut(&self, shortcut: &KeyBinding) -> Option<&Bookmark> {
        self.bookmarks
            .iter()
            .find(|b| b.shortcut.as_ref().map_or(false, |s| s == shortcut))
    }

    /// Get the number of bookmarks
    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    /// Check if bookmarks is empty
    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    /// Check if at maximum capacity
    pub fn is_full(&self) -> bool {
        self.bookmarks.len() >= self.max_bookmarks
    }

    /// Validate all bookmarks and mark invalid ones
    pub fn validate_all(&mut self) -> Vec<BookmarkId> {
        let mut invalid_ids = Vec::new();
        for bookmark in &mut self.bookmarks {
            if !bookmark.validate() {
                invalid_ids.push(bookmark.id);
            }
        }
        invalid_ids
    }

    /// Get recent locations
    pub fn recent(&self) -> &VecDeque<PathBuf> {
        &self.recent_locations
    }

    /// Add a path to recent locations
    pub fn add_recent(&mut self, path: PathBuf) {
        // Remove if already exists (to move to front)
        self.recent_locations.retain(|p| p != &path);

        self.recent_locations.push_front(path);

        // Trim to max size
        while self.recent_locations.len() > self.max_recent {
            self.recent_locations.pop_back();
        }
    }

    /// Clear recent locations
    pub fn clear_recent(&mut self) {
        self.recent_locations.clear();
    }

    /// Check if a path is bookmarked
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.bookmarks.iter().any(|b| &b.path == path)
    }

    /// Get config file path
    fn config_path() -> Result<PathBuf, BookmarkError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| BookmarkError::Io("Could not find config directory".to_string()))?;

        let app_config = config_dir.join("nexus-explorer");
        Ok(app_config.join("bookmarks.json"))
    }

    /// Save bookmarks to JSON config file
    pub fn save(&self) -> Result<(), BookmarkError> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| BookmarkError::Io(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| BookmarkError::Serialization(e.to_string()))?;

        fs::write(&path, json).map_err(|e| BookmarkError::Io(e.to_string()))?;

        Ok(())
    }

    /// Load bookmarks from JSON config file
    pub fn load() -> Result<Self, BookmarkError> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let json = fs::read_to_string(&path).map_err(|e| BookmarkError::Io(e.to_string()))?;

        let mut manager: BookmarkManager =
            serde_json::from_str(&json).map_err(|e| BookmarkError::Serialization(e.to_string()))?;

        manager.max_bookmarks = MAX_BOOKMARKS;
        manager.max_recent = MAX_RECENT_LOCATIONS;

        // Calculate next_id from existing bookmarks
        manager.next_id = manager.bookmarks.iter().map(|b| b.id.0).max().unwrap_or(0) + 1;

        manager.validate_all();

        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path() -> PathBuf {
        env::temp_dir()
    }

    #[test]
    fn test_bookmark_manager_new() {
        let manager = BookmarkManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
        assert!(!manager.is_full());
    }

    #[test]
    fn test_add_bookmark() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let result = manager.add(path.clone());
        assert!(result.is_ok());
        assert_eq!(manager.len(), 1);
        assert!(manager.contains(&path));
    }

    #[test]
    fn test_add_duplicate_bookmark() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let _ = manager.add(path.clone());
        let result = manager.add(path);

        assert_eq!(result, Err(BookmarkError::AlreadyExists));
    }

    #[test]
    fn test_remove_bookmark() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let id = manager.add(path.clone()).unwrap();
        assert_eq!(manager.len(), 1);

        let removed = manager.remove(id);
        assert!(removed.is_ok());
        assert_eq!(manager.len(), 0);
        assert!(!manager.contains(&path));
    }

    #[test]
    fn test_rename_bookmark() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let id = manager.add(path).unwrap();
        let result = manager.rename(id, "My Bookmark".to_string());

        assert!(result.is_ok());
        assert_eq!(manager.get(id).unwrap().name, "My Bookmark");
    }

    #[test]
    fn test_set_shortcut() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let id = manager.add(path).unwrap();
        let shortcut = KeyBinding::new("1");

        let result = manager.set_shortcut(id, Some(shortcut.clone()));
        assert!(result.is_ok());

        let bookmark = manager.get(id).unwrap();
        assert_eq!(bookmark.shortcut, Some(shortcut));
    }

    #[test]
    fn test_find_by_shortcut() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let id = manager.add(path).unwrap();
        let shortcut = KeyBinding::new("1");
        manager.set_shortcut(id, Some(shortcut.clone())).unwrap();

        let found = manager.find_by_shortcut(&shortcut);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn test_recent_locations() {
        let mut manager = BookmarkManager::new();

        manager.add_recent(PathBuf::from("/path/1"));
        manager.add_recent(PathBuf::from("/path/2"));
        manager.add_recent(PathBuf::from("/path/3"));

        assert_eq!(manager.recent().len(), 3);
        // Most recent should be first
        assert_eq!(manager.recent()[0], PathBuf::from("/path/3"));
    }

    #[test]
    fn test_recent_locations_dedup() {
        let mut manager = BookmarkManager::new();

        manager.add_recent(PathBuf::from("/path/1"));
        manager.add_recent(PathBuf::from("/path/2"));
        manager.add_recent(PathBuf::from("/path/1"));

        assert_eq!(manager.recent().len(), 2);
        // /path/1 should now be first (most recent)
        assert_eq!(manager.recent()[0], PathBuf::from("/path/1"));
    }

    #[test]
    fn test_key_binding_display() {
        let shortcut = KeyBinding::new("1");
        let display = shortcut.display();
        #[cfg(target_os = "macos")]
        assert!(display.contains("⌘"));
        #[cfg(not(target_os = "macos"))]
        assert!(display.contains("Ctrl"));
    }

    #[test]
    fn test_serialization_round_trip() {
        let mut manager = BookmarkManager::new();
        let path = temp_path();

        let id = manager.add(path.clone()).unwrap();
        manager.rename(id, "Test Bookmark".to_string()).unwrap();
        manager
            .set_shortcut(id, Some(KeyBinding::new("1")))
            .unwrap();
        manager.add_recent(PathBuf::from("/recent/path"));

        let json = serde_json::to_string(&manager).unwrap();
        let loaded: BookmarkManager = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].name, "Test Bookmark");
        assert_eq!(loaded.recent_locations.len(), 1);
    }
}
