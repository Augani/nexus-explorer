use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Maximum number of favorites allowed
pub const MAX_FAVORITES: usize = 10;

/// Error types for favorites operations
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

/// A single favorite entry
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

    /// Validate that the path still exists
    pub fn validate(&mut self) -> bool {
        self.is_valid = self.path.exists();
        self.is_valid
    }
}

/// Manages user's favorite directories with persistence
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

    /// Add a new favorite from a path
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

    /// Remove a favorite by index
    pub fn remove(&mut self, index: usize) -> Result<Favorite, FavoritesError> {
        if index >= self.items.len() {
            return Err(FavoritesError::IndexOutOfBounds(index));
        }
        Ok(self.items.remove(index))
    }

    /// Remove a favorite by path
    pub fn remove_by_path(&mut self, path: &PathBuf) -> Option<Favorite> {
        if let Some(index) = self.items.iter().position(|f| &f.path == path) {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// Reorder favorites by moving item from one index to another
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

    /// Get all favorites
    pub fn items(&self) -> &[Favorite] {
        &self.items
    }

    /// Get mutable reference to all favorites
    pub fn items_mut(&mut self) -> &mut Vec<Favorite> {
        &mut self.items
    }

    /// Get the number of favorites
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if favorites is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if at maximum capacity
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.max_count
    }

    /// Validate all favorites and mark invalid ones
    pub fn validate_all(&mut self) -> Vec<usize> {
        let mut invalid_indices = Vec::new();
        for (i, favorite) in self.items.iter_mut().enumerate() {
            if !favorite.validate() {
                invalid_indices.push(i);
            }
        }
        invalid_indices
    }

    /// Get config file path
    fn config_path() -> Result<PathBuf, FavoritesError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| FavoritesError::Io("Could not find config directory".to_string()))?;

        let app_config = config_dir.join("nexus-explorer");
        Ok(app_config.join("favorites.json"))
    }

    /// Save favorites to JSON config file
    pub fn save(&self) -> Result<(), FavoritesError> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| FavoritesError::Io(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| FavoritesError::Serialization(e.to_string()))?;

        fs::write(&path, json).map_err(|e| FavoritesError::Io(e.to_string()))?;

        Ok(())
    }

    /// Load favorites from JSON config file
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

    /// Check if a path is already a favorite
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.items.iter().any(|f| &f.path == path)
    }

    /// Get a favorite by index
    pub fn get(&self, index: usize) -> Option<&Favorite> {
        self.items.get(index)
    }

    /// Find index of a favorite by path
    pub fn find_index(&self, path: &PathBuf) -> Option<usize> {
        self.items.iter().position(|f| &f.path == path)
    }
}

#[cfg(test)]
#[path = "favorites_tests.rs"]
mod tests;
