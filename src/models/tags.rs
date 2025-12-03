use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Unique identifier for a tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagId(pub u64);

impl TagId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Predefined tag colors matching macOS Finder style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TagColor {
    #[default]
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Gray,
}

impl TagColor {
    /// Returns the RGBA color value for this tag color
    pub fn to_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            TagColor::Red => (0xE5, 0x3E, 0x3E, 0xFF),
            TagColor::Orange => (0xF5, 0x9E, 0x0B, 0xFF),
            TagColor::Yellow => (0xF5, 0xC5, 0x0B, 0xFF),
            TagColor::Green => (0x22, 0xC5, 0x5E, 0xFF),
            TagColor::Blue => (0x3B, 0x82, 0xF6, 0xFF),
            TagColor::Purple => (0x8B, 0x5C, 0xF6, 0xFF),
            TagColor::Gray => (0x6B, 0x72, 0x80, 0xFF),
        }
    }

    /// Returns all available tag colors
    pub fn all() -> &'static [TagColor] {
        &[
            TagColor::Red,
            TagColor::Orange,
            TagColor::Yellow,
            TagColor::Green,
            TagColor::Blue,
            TagColor::Purple,
            TagColor::Gray,
        ]
    }

    /// Returns the display name for this color
    pub fn display_name(&self) -> &'static str {
        match self {
            TagColor::Red => "Red",
            TagColor::Orange => "Orange",
            TagColor::Yellow => "Yellow",
            TagColor::Green => "Green",
            TagColor::Blue => "Blue",
            TagColor::Purple => "Purple",
            TagColor::Gray => "Gray",
        }
    }
}

/// A tag that can be applied to files
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: TagColor,
}

impl Tag {
    pub fn new(id: TagId, name: String, color: TagColor) -> Self {
        Self { id, name, color }
    }
}

/// Errors that can occur during tag operations
#[derive(Debug, Error)]
pub enum TagError {
    #[error("Tag not found: {0:?}")]
    TagNotFound(TagId),

    #[error("Tag with name '{0}' already exists")]
    DuplicateName(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Extended attributes not supported on this platform")]
    XattrNotSupported,
}

/// Result type for tag operations
pub type TagResult<T> = std::result::Result<T, TagError>;


/// Manages file tags - creating, deleting, and applying tags to files
/// 
/// Tags are stored in two ways:
/// 1. Extended attributes (xattr) on supported platforms for per-file storage
/// 2. A local JSON database as fallback or for platforms without xattr support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagManager {
    /// All defined tags
    tags: HashMap<TagId, Tag>,
    /// Mapping from file paths to their tag IDs
    file_tags: HashMap<PathBuf, HashSet<TagId>>,
    /// Next available tag ID
    next_id: u64,
}

impl Default for TagManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TagManager {
    /// Creates a new TagManager with default color tags
    pub fn new() -> Self {
        let mut manager = Self {
            tags: HashMap::new(),
            file_tags: HashMap::new(),
            next_id: 1,
        };
        
        // Create default color tags
        for color in TagColor::all() {
            let _ = manager.create_tag(color.display_name().to_string(), *color);
        }
        
        manager
    }

    /// Creates a new tag with the given name and color
    pub fn create_tag(&mut self, name: String, color: TagColor) -> TagResult<TagId> {
        // Check for duplicate names
        if self.tags.values().any(|t| t.name.eq_ignore_ascii_case(&name)) {
            return Err(TagError::DuplicateName(name));
        }

        let id = TagId::new(self.next_id);
        self.next_id += 1;

        let tag = Tag::new(id, name, color);
        self.tags.insert(id, tag);

        Ok(id)
    }

    /// Deletes a tag and removes it from all files
    pub fn delete_tag(&mut self, id: TagId) -> TagResult<()> {
        if !self.tags.contains_key(&id) {
            return Err(TagError::TagNotFound(id));
        }

        self.tags.remove(&id);

        // Remove this tag from all files
        for tags in self.file_tags.values_mut() {
            tags.remove(&id);
        }

        // Clean up empty entries
        self.file_tags.retain(|_, tags| !tags.is_empty());

        Ok(())
    }

    /// Applies a tag to a file
    pub fn apply_tag(&mut self, path: &Path, tag_id: TagId) -> TagResult<()> {
        if !self.tags.contains_key(&tag_id) {
            return Err(TagError::TagNotFound(tag_id));
        }

        self.file_tags
            .entry(path.to_path_buf())
            .or_default()
            .insert(tag_id);

        Ok(())
    }

    /// Removes a tag from a file
    pub fn remove_tag(&mut self, path: &Path, tag_id: TagId) -> TagResult<()> {
        if !self.tags.contains_key(&tag_id) {
            return Err(TagError::TagNotFound(tag_id));
        }

        if let Some(tags) = self.file_tags.get_mut(path) {
            tags.remove(&tag_id);
            if tags.is_empty() {
                self.file_tags.remove(path);
            }
        }

        Ok(())
    }

    /// Returns all tags applied to a file
    pub fn tags_for_file(&self, path: &Path) -> Vec<&Tag> {
        self.file_tags
            .get(path)
            .map(|tag_ids| {
                tag_ids
                    .iter()
                    .filter_map(|id| self.tags.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns all files that have a specific tag
    pub fn files_with_tag(&self, tag_id: TagId) -> Vec<&PathBuf> {
        self.file_tags
            .iter()
            .filter(|(_, tags)| tags.contains(&tag_id))
            .map(|(path, _)| path)
            .collect()
    }

    /// Returns all defined tags
    pub fn all_tags(&self) -> Vec<&Tag> {
        self.tags.values().collect()
    }

    /// Returns a tag by ID
    pub fn get_tag(&self, id: TagId) -> Option<&Tag> {
        self.tags.get(&id)
    }

    /// Returns a tag by name (case-insensitive)
    pub fn get_tag_by_name(&self, name: &str) -> Option<&Tag> {
        self.tags.values().find(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// Returns the number of defined tags
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Returns the number of files with tags
    pub fn tagged_file_count(&self) -> usize {
        self.file_tags.len()
    }

    /// Checks if a file has any tags
    pub fn has_tags(&self, path: &Path) -> bool {
        self.file_tags.get(path).map(|t| !t.is_empty()).unwrap_or(false)
    }

    /// Checks if a file has a specific tag
    pub fn has_tag(&self, path: &Path, tag_id: TagId) -> bool {
        self.file_tags
            .get(path)
            .map(|tags| tags.contains(&tag_id))
            .unwrap_or(false)
    }

    /// Renames a tag
    pub fn rename_tag(&mut self, id: TagId, new_name: String) -> TagResult<()> {
        // Check for duplicate names (excluding the tag being renamed)
        if self.tags.values().any(|t| t.id != id && t.name.eq_ignore_ascii_case(&new_name)) {
            return Err(TagError::DuplicateName(new_name));
        }

        if let Some(tag) = self.tags.get_mut(&id) {
            tag.name = new_name;
            Ok(())
        } else {
            Err(TagError::TagNotFound(id))
        }
    }

    /// Changes a tag's color
    pub fn set_tag_color(&mut self, id: TagId, color: TagColor) -> TagResult<()> {
        if let Some(tag) = self.tags.get_mut(&id) {
            tag.color = color;
            Ok(())
        } else {
            Err(TagError::TagNotFound(id))
        }
    }

    /// Clears all tags from a file
    pub fn clear_file_tags(&mut self, path: &Path) {
        self.file_tags.remove(path);
    }

    /// Updates file path when a file is renamed/moved
    pub fn update_file_path(&mut self, old_path: &Path, new_path: &Path) {
        if let Some(tags) = self.file_tags.remove(old_path) {
            self.file_tags.insert(new_path.to_path_buf(), tags);
        }
    }
}


/// Persistence format for tags
#[derive(Debug, Serialize, Deserialize)]
struct TagsConfig {
    version: u32,
    tags: Vec<TagEntry>,
    file_tags: Vec<FileTagEntry>,
    next_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TagEntry {
    id: u64,
    name: String,
    color: TagColor,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileTagEntry {
    path: String,
    tag_ids: Vec<u64>,
}

impl TagManager {
    /// Returns the config directory path for tags
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer")
            .join("tags.json")
    }

    /// Saves tags to the config file
    pub fn save(&self) -> TagResult<()> {
        let config_dir = Self::config_path().parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;

        let config = TagsConfig {
            version: 1,
            tags: self.tags.values().map(|t| TagEntry {
                id: t.id.0,
                name: t.name.clone(),
                color: t.color,
            }).collect(),
            file_tags: self.file_tags.iter().map(|(path, tags)| FileTagEntry {
                path: path.to_string_lossy().to_string(),
                tag_ids: tags.iter().map(|id| id.0).collect(),
            }).collect(),
            next_id: self.next_id,
        };

        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| TagError::Serialization(e.to_string()))?;

        std::fs::write(Self::config_path(), json)?;
        Ok(())
    }

    /// Loads tags from the config file
    pub fn load() -> TagResult<Self> {
        let config_path = Self::config_path();
        
        if !config_path.exists() {
            return Ok(Self::new());
        }

        let json = std::fs::read_to_string(&config_path)?;
        let config: TagsConfig = serde_json::from_str(&json)
            .map_err(|e| TagError::Serialization(e.to_string()))?;

        let mut manager = Self {
            tags: HashMap::new(),
            file_tags: HashMap::new(),
            next_id: config.next_id,
        };

        // Restore tags
        for entry in config.tags {
            let tag = Tag::new(TagId::new(entry.id), entry.name, entry.color);
            manager.tags.insert(tag.id, tag);
        }

        // Restore file tags
        for entry in config.file_tags {
            let path = PathBuf::from(entry.path);
            let tag_ids: HashSet<TagId> = entry.tag_ids.into_iter().map(TagId::new).collect();
            if !tag_ids.is_empty() {
                manager.file_tags.insert(path, tag_ids);
            }
        }

        Ok(manager)
    }

    /// Creates an empty TagManager without default tags (for testing)
    pub fn empty() -> Self {
        Self {
            tags: HashMap::new(),
            file_tags: HashMap::new(),
            next_id: 1,
        }
    }
}

/// Extended attribute storage for file tags
/// 
/// This module provides xattr-based storage for tags on supported platforms.
/// Falls back to the local database when xattr is not available.
pub mod xattr_storage {
    use super::*;
    
    /// The xattr name used to store tags
    const XATTR_NAME: &str = "user.nexus-explorer.tags";
    
    /// Error codes for "operation not supported" on different platforms
    #[cfg(target_os = "macos")]
    const ENOTSUP: i32 = 45;
    #[cfg(target_os = "linux")]
    const ENOTSUP: i32 = 95;
    #[cfg(target_os = "windows")]
    const ENOTSUP: i32 = 129; // Windows doesn't really use this
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    const ENOTSUP: i32 = 95;
    
    /// Checks if an error indicates xattr is not supported
    fn is_not_supported_error(e: &std::io::Error) -> bool {
        matches!(e.raw_os_error(), Some(code) if code == ENOTSUP || code == 95 || code == 45)
    }
    
    /// Checks if extended attributes are supported for the given path
    pub fn is_supported(path: &Path) -> bool {
        // Try to list xattrs - if it works, xattr is supported
        xattr::list(path).is_ok()
    }
    
    /// Reads tag IDs from a file's extended attributes
    pub fn read_tags(path: &Path) -> TagResult<Vec<u64>> {
        match xattr::get(path, XATTR_NAME) {
            Ok(Some(data)) => {
                // Parse comma-separated tag IDs
                let s = String::from_utf8_lossy(&data);
                let ids: Vec<u64> = s
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                Ok(ids)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => {
                if is_not_supported_error(&e) {
                    Err(TagError::XattrNotSupported)
                } else {
                    Err(TagError::Io(e))
                }
            }
        }
    }
    
    /// Writes tag IDs to a file's extended attributes
    pub fn write_tags(path: &Path, tag_ids: &[u64]) -> TagResult<()> {
        if tag_ids.is_empty() {
            // Remove the xattr if no tags
            match xattr::remove(path, XATTR_NAME) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => {
                    if is_not_supported_error(&e) {
                        Err(TagError::XattrNotSupported)
                    } else {
                        Err(TagError::Io(e))
                    }
                }
            }
        } else {
            // Write comma-separated tag IDs
            let data = tag_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            
            xattr::set(path, XATTR_NAME, data.as_bytes()).map_err(|e| {
                if is_not_supported_error(&e) {
                    TagError::XattrNotSupported
                } else {
                    TagError::Io(e)
                }
            })
        }
    }
    
    /// Removes all tags from a file's extended attributes
    pub fn clear_tags(path: &Path) -> TagResult<()> {
        write_tags(path, &[])
    }
}

impl TagManager {
    /// Applies a tag to a file and optionally stores it in xattr
    /// 
    /// This method first updates the in-memory state, then attempts to
    /// write to xattr. If xattr fails, the in-memory state is still valid.
    pub fn apply_tag_with_xattr(&mut self, path: &Path, tag_id: TagId) -> TagResult<()> {
        // First apply to in-memory state
        self.apply_tag(path, tag_id)?;
        
        // Try to write to xattr (best effort)
        let tag_ids: Vec<u64> = self.file_tags
            .get(path)
            .map(|ids| ids.iter().map(|id| id.0).collect())
            .unwrap_or_default();
        
        // Ignore xattr errors - fall back to database storage
        let _ = xattr_storage::write_tags(path, &tag_ids);
        
        Ok(())
    }
    
    /// Removes a tag from a file and updates xattr
    pub fn remove_tag_with_xattr(&mut self, path: &Path, tag_id: TagId) -> TagResult<()> {
        // First remove from in-memory state
        self.remove_tag(path, tag_id)?;
        
        // Try to update xattr (best effort)
        let tag_ids: Vec<u64> = self.file_tags
            .get(path)
            .map(|ids| ids.iter().map(|id| id.0).collect())
            .unwrap_or_default();
        
        let _ = xattr_storage::write_tags(path, &tag_ids);
        
        Ok(())
    }
    
    /// Syncs tags from xattr to in-memory state for a file
    /// 
    /// This is useful when loading a directory to pick up tags
    /// that were set by other applications or previous sessions.
    pub fn sync_from_xattr(&mut self, path: &Path) -> TagResult<()> {
        match xattr_storage::read_tags(path) {
            Ok(tag_ids) => {
                if tag_ids.is_empty() {
                    self.file_tags.remove(path);
                } else {
                    // Only include tag IDs that exist in our tag definitions
                    let valid_ids: HashSet<TagId> = tag_ids
                        .into_iter()
                        .map(TagId::new)
                        .filter(|id| self.tags.contains_key(id))
                        .collect();
                    
                    if valid_ids.is_empty() {
                        self.file_tags.remove(path);
                    } else {
                        self.file_tags.insert(path.to_path_buf(), valid_ids);
                    }
                }
                Ok(())
            }
            Err(TagError::XattrNotSupported) => {
                // xattr not supported, just use database
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    
    /// Syncs tags from in-memory state to xattr for a file
    pub fn sync_to_xattr(&self, path: &Path) -> TagResult<()> {
        let tag_ids: Vec<u64> = self.file_tags
            .get(path)
            .map(|ids| ids.iter().map(|id| id.0).collect())
            .unwrap_or_default();
        
        xattr_storage::write_tags(path, &tag_ids)
    }
    
    /// Checks if xattr storage is supported for the given path
    pub fn xattr_supported(path: &Path) -> bool {
        xattr_storage::is_supported(path)
    }
}

#[cfg(test)]
#[path = "tags_tests.rs"]
mod tests;
