use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagId(pub u64);

impl TagId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/
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
    /
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

    /
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

    /
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

/
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

/
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

/
pub type TagResult<T> = std::result::Result<T, TagError>;

/
/
/
/
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagManager {
    /
    tags: HashMap<TagId, Tag>,
    /
    file_tags: HashMap<PathBuf, HashSet<TagId>>,
    /
    next_id: u64,
}

impl Default for TagManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TagManager {
    /
    pub fn new() -> Self {
        let mut manager = Self {
            tags: HashMap::new(),
            file_tags: HashMap::new(),
            next_id: 1,
        };

        for color in TagColor::all() {
            let _ = manager.create_tag(color.display_name().to_string(), *color);
        }

        manager
    }

    /
    pub fn create_tag(&mut self, name: String, color: TagColor) -> TagResult<TagId> {
        if self
            .tags
            .values()
            .any(|t| t.name.eq_ignore_ascii_case(&name))
        {
            return Err(TagError::DuplicateName(name));
        }

        let id = TagId::new(self.next_id);
        self.next_id += 1;

        let tag = Tag::new(id, name, color);
        self.tags.insert(id, tag);

        Ok(id)
    }

    /
    pub fn delete_tag(&mut self, id: TagId) -> TagResult<()> {
        if !self.tags.contains_key(&id) {
            return Err(TagError::TagNotFound(id));
        }

        self.tags.remove(&id);

        for tags in self.file_tags.values_mut() {
            tags.remove(&id);
        }

        self.file_tags.retain(|_, tags| !tags.is_empty());

        Ok(())
    }

    /
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

    /
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

    /
    pub fn tags_for_file(&self, path: &Path) -> Vec<&Tag> {
        self.file_tags
            .get(path)
            .map(|tag_ids| tag_ids.iter().filter_map(|id| self.tags.get(id)).collect())
            .unwrap_or_default()
    }

    /
    pub fn files_with_tag(&self, tag_id: TagId) -> Vec<&PathBuf> {
        self.file_tags
            .iter()
            .filter(|(_, tags)| tags.contains(&tag_id))
            .map(|(path, _)| path)
            .collect()
    }

    /
    pub fn all_tags(&self) -> Vec<&Tag> {
        self.tags.values().collect()
    }

    /
    pub fn get_tag(&self, id: TagId) -> Option<&Tag> {
        self.tags.get(&id)
    }

    /
    pub fn get_tag_by_name(&self, name: &str) -> Option<&Tag> {
        self.tags
            .values()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    /
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /
    pub fn tagged_file_count(&self) -> usize {
        self.file_tags.len()
    }

    /
    pub fn has_tags(&self, path: &Path) -> bool {
        self.file_tags
            .get(path)
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /
    pub fn has_tag(&self, path: &Path, tag_id: TagId) -> bool {
        self.file_tags
            .get(path)
            .map(|tags| tags.contains(&tag_id))
            .unwrap_or(false)
    }

    /
    pub fn rename_tag(&mut self, id: TagId, new_name: String) -> TagResult<()> {
        if self
            .tags
            .values()
            .any(|t| t.id != id && t.name.eq_ignore_ascii_case(&new_name))
        {
            return Err(TagError::DuplicateName(new_name));
        }

        if let Some(tag) = self.tags.get_mut(&id) {
            tag.name = new_name;
            Ok(())
        } else {
            Err(TagError::TagNotFound(id))
        }
    }

    /
    pub fn set_tag_color(&mut self, id: TagId, color: TagColor) -> TagResult<()> {
        if let Some(tag) = self.tags.get_mut(&id) {
            tag.color = color;
            Ok(())
        } else {
            Err(TagError::TagNotFound(id))
        }
    }

    /
    pub fn clear_file_tags(&mut self, path: &Path) {
        self.file_tags.remove(path);
    }

    /
    pub fn update_file_path(&mut self, old_path: &Path, new_path: &Path) {
        if let Some(tags) = self.file_tags.remove(old_path) {
            self.file_tags.insert(new_path.to_path_buf(), tags);
        }
    }
}

/
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
    /
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer")
            .join("tags.json")
    }

    /
    pub fn save(&self) -> TagResult<()> {
        let config_dir = Self::config_path().parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;

        let config = TagsConfig {
            version: 1,
            tags: self
                .tags
                .values()
                .map(|t| TagEntry {
                    id: t.id.0,
                    name: t.name.clone(),
                    color: t.color,
                })
                .collect(),
            file_tags: self
                .file_tags
                .iter()
                .map(|(path, tags)| FileTagEntry {
                    path: path.to_string_lossy().to_string(),
                    tag_ids: tags.iter().map(|id| id.0).collect(),
                })
                .collect(),
            next_id: self.next_id,
        };

        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| TagError::Serialization(e.to_string()))?;

        std::fs::write(Self::config_path(), json)?;
        Ok(())
    }

    /
    pub fn load() -> TagResult<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let json = std::fs::read_to_string(&config_path)?;
        let config: TagsConfig =
            serde_json::from_str(&json).map_err(|e| TagError::Serialization(e.to_string()))?;

        let mut manager = Self {
            tags: HashMap::new(),
            file_tags: HashMap::new(),
            next_id: config.next_id,
        };

        for entry in config.tags {
            let tag = Tag::new(TagId::new(entry.id), entry.name, entry.color);
            manager.tags.insert(tag.id, tag);
        }

        for entry in config.file_tags {
            let path = PathBuf::from(entry.path);
            let tag_ids: HashSet<TagId> = entry.tag_ids.into_iter().map(TagId::new).collect();
            if !tag_ids.is_empty() {
                manager.file_tags.insert(path, tag_ids);
            }
        }

        Ok(manager)
    }

    /
    pub fn empty() -> Self {
        Self {
            tags: HashMap::new(),
            file_tags: HashMap::new(),
            next_id: 1,
        }
    }
}

/
#[cfg(unix)]
pub mod xattr_storage {
    use super::*;

    const XATTR_NAME: &str = "user.nexus-explorer.tags";

    #[cfg(target_os = "macos")]
    const ENOTSUP: i32 = 45;
    #[cfg(target_os = "linux")]
    const ENOTSUP: i32 = 95;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    const ENOTSUP: i32 = 95;

    fn is_not_supported_error(e: &std::io::Error) -> bool {
        matches!(e.raw_os_error(), Some(code) if code == ENOTSUP || code == 95 || code == 45)
    }

    pub fn is_supported(path: &Path) -> bool {
        xattr::list(path).is_ok()
    }

    pub fn read_tags(path: &Path) -> TagResult<Vec<u64>> {
        match xattr::get(path, XATTR_NAME) {
            Ok(Some(data)) => {
                let s = String::from_utf8_lossy(&data);
                let ids: Vec<u64> = s.split(',').filter_map(|s| s.trim().parse().ok()).collect();
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

    pub fn write_tags(path: &Path, tag_ids: &[u64]) -> TagResult<()> {
        if tag_ids.is_empty() {
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

    pub fn clear_tags(path: &Path) -> TagResult<()> {
        write_tags(path, &[])
    }
}

/
#[cfg(not(unix))]
pub mod xattr_storage {
    use super::*;

    pub fn is_supported(_path: &Path) -> bool {
        false
    }

    pub fn read_tags(_path: &Path) -> TagResult<Vec<u64>> {
        Err(TagError::XattrNotSupported)
    }

    pub fn write_tags(_path: &Path, _tag_ids: &[u64]) -> TagResult<()> {
        Err(TagError::XattrNotSupported)
    }

    pub fn clear_tags(_path: &Path) -> TagResult<()> {
        Err(TagError::XattrNotSupported)
    }
}

impl TagManager {
    /
    /
    /
    /
    pub fn apply_tag_with_xattr(&mut self, path: &Path, tag_id: TagId) -> TagResult<()> {
        self.apply_tag(path, tag_id)?;

        let tag_ids: Vec<u64> = self
            .file_tags
            .get(path)
            .map(|ids| ids.iter().map(|id| id.0).collect())
            .unwrap_or_default();

        let _ = xattr_storage::write_tags(path, &tag_ids);

        Ok(())
    }

    /
    pub fn remove_tag_with_xattr(&mut self, path: &Path, tag_id: TagId) -> TagResult<()> {
        self.remove_tag(path, tag_id)?;

        let tag_ids: Vec<u64> = self
            .file_tags
            .get(path)
            .map(|ids| ids.iter().map(|id| id.0).collect())
            .unwrap_or_default();

        let _ = xattr_storage::write_tags(path, &tag_ids);

        Ok(())
    }

    /
    /
    /
    /
    pub fn sync_from_xattr(&mut self, path: &Path) -> TagResult<()> {
        match xattr_storage::read_tags(path) {
            Ok(tag_ids) => {
                if tag_ids.is_empty() {
                    self.file_tags.remove(path);
                } else {
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
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /
    pub fn sync_to_xattr(&self, path: &Path) -> TagResult<()> {
        let tag_ids: Vec<u64> = self
            .file_tags
            .get(path)
            .map(|ids| ids.iter().map(|id| id.0).collect())
            .unwrap_or_default();

        xattr_storage::write_tags(path, &tag_ids)
    }

    /
    pub fn xattr_supported(path: &Path) -> bool {
        xattr_storage::is_supported(path)
    }
}

#[cfg(test)]
#[path = "tags_tests.rs"]
mod tests;
