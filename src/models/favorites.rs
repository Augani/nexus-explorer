use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/
pub const MAX_FAVORITES: usize = 10;

/
#[derive(Debug, Error, PartialEq, Clone)]
pub enum FavoritesError {
    #[error("Maximum favorites limit ({0}) reached")]
    MaxReached(usize),

    #[error("Path already exists in favorites")]
    AlreadyExists,

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Favorite {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub is_valid: bool,
}

impl Favorite {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let is_valid = path.exists();

        Self {
            name,
            path,
            is_valid,
        }
    }

    pub fn with_name(path: PathBuf, name: String) -> Self {
        let is_valid = path.exists();
        Self {
            name,
            path,
            is_valid,
        }
    }

    /
    pub fn validate(&mut self) -> bool {
        self.is_valid = self.path.exists();
        self.is_valid
    }
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorites {
    items: Vec<Favorite>,
    #[serde(skip)]
    max_count: usize,
}

impl Default for Favorites {
    fn default() -> Self {
        Self::new()
    }
}

impl Favorites {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            max_count: MAX_FAVORITES,
        }
    }

    /
    pub fn add(&mut self, path: PathBuf) -> Result<(), FavoritesError> {
        if self.items.len() >= self.max_count {
            return Err(FavoritesError::MaxReached(self.max_count));
        }

        if self.items.iter().any(|f| f.path == path) {
            return Err(FavoritesError::AlreadyExists);
        }

        if !path.exists() {
            return Err(FavoritesError::InvalidPath(path.display().to_string()));
        }

        self.items.push(Favorite::new(path));
        Ok(())
    }

    /
    pub fn remove(&mut self, index: usize) -> Result<Favorite, FavoritesError> {
        if index >= self.items.len() {
            return Err(FavoritesError::IndexOutOfBounds(index));
        }
        Ok(self.items.remove(index))
    }

    /
    pub fn remove_by_path(&mut self, path: &PathBuf) -> Option<Favorite> {
        if let Some(index) = self.items.iter().position(|f| &f.path == path) {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /
    pub fn reorder(&mut self, from: usize, to: usize) -> Result<(), FavoritesError> {
        let len = self.items.len();
        if from >= len {
            return Err(FavoritesError::IndexOutOfBounds(from));
        }
        if to >= len {
            return Err(FavoritesError::IndexOutOfBounds(to));
        }

        if from == to {
            return Ok(());
        }

        let item = self.items.remove(from);
        self.items.insert(to, item);
        Ok(())
    }

    /
    pub fn items(&self) -> &[Favorite] {
        &self.items
    }

    /
    pub fn items_mut(&mut self) -> &mut Vec<Favorite> {
        &mut self.items
    }

    /
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.max_count
    }

    /
    pub fn validate_all(&mut self) -> Vec<usize> {
        let mut invalid_indices = Vec::new();
        for (i, favorite) in self.items.iter_mut().enumerate() {
            if !favorite.validate() {
                invalid_indices.push(i);
            }
        }
        invalid_indices
    }

    /
    fn config_path() -> Result<PathBuf, FavoritesError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| FavoritesError::Io("Could not find config directory".to_string()))?;

        let app_config = config_dir.join("nexus-explorer");
        Ok(app_config.join("favorites.json"))
    }

    /
    pub fn save(&self) -> Result<(), FavoritesError> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| FavoritesError::Io(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| FavoritesError::Serialization(e.to_string()))?;

        fs::write(&path, json).map_err(|e| FavoritesError::Io(e.to_string()))?;

        Ok(())
    }

    /
    pub fn load() -> Result<Self, FavoritesError> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let json = fs::read_to_string(&path).map_err(|e| FavoritesError::Io(e.to_string()))?;

        let mut favorites: Favorites = serde_json::from_str(&json)
            .map_err(|e| FavoritesError::Serialization(e.to_string()))?;

        favorites.max_count = MAX_FAVORITES;

        favorites.validate_all();

        Ok(favorites)
    }

    /
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.items.iter().any(|f| &f.path == path)
    }

    /
    pub fn get(&self, index: usize) -> Option<&Favorite> {
        self.items.get(index)
    }

    /
    pub fn find_index(&self, path: &PathBuf) -> Option<usize> {
        self.items.iter().position(|f| &f.path == path)
    }
}

#[cfg(test)]
#[path = "favorites_tests.rs"]
mod tests;
