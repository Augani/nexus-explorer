use std::path::PathBuf;

use gpui::{prelude::*, px, Context, Window};
use serde::{Deserialize, Serialize};

/// Payload for drag-and-drop operations containing file paths
#[derive(Clone, Debug)]
pub struct DragPayload {
    pub paths: Vec<PathBuf>,
}

impl DragPayload {
    /// Creates a new drag payload with the given paths
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    /// Creates a drag payload for a single path
    pub fn single(path: PathBuf) -> Self {
        Self { paths: vec![path] }
    }
}

/// Data transferred during file drag operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDragData {
    /// Paths of files being dragged
    pub paths: Vec<PathBuf>,
    /// Source window identifier (for cross-window operations)
    pub source_window_id: Option<u64>,
    /// Current drag position
    pub position: (f32, f32),
}

impl FileDragData {
    /// Creates a new FileDragData for a single file
    pub fn single(path: PathBuf) -> Self {
        Self {
            paths: vec![path],
            source_window_id: None,
            position: (0.0, 0.0),
        }
    }

    /// Creates a new FileDragData for multiple files
    pub fn multiple(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            source_window_id: None,
            position: (0.0, 0.0),
        }
    }

    /// Sets the source window ID
    pub fn with_source_window(mut self, window_id: u64) -> Self {
        self.source_window_id = Some(window_id);
        self
    }

    /// Sets the drag position
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }

    /// Returns the number of files being dragged
    pub fn file_count(&self) -> usize {
        self.paths.len()
    }

    /// Returns whether this is a single file drag
    pub fn is_single(&self) -> bool {
        self.paths.len() == 1
    }

    /// Returns the first path (for single file operations)
    pub fn first_path(&self) -> Option<&PathBuf> {
        self.paths.first()
    }

    /// Returns whether all paths are directories
    pub fn all_directories(&self) -> bool {
        self.paths.iter().all(|p| p.is_dir())
    }

    /// Returns whether any path is a directory
    pub fn has_directories(&self) -> bool {
        self.paths.iter().any(|p| p.is_dir())
    }
}

impl Render for FileDragData {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::{div, Styled};
        
        let count = self.file_count();
        let label = if count == 1 {
            self.paths[0]
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("File")
                .to_string()
        } else {
            format!("{} items", count)
        };

        let pos_x = px(self.position.0);
        let pos_y = px(self.position.1);

        div()
            .absolute()
            .left(pos_x - px(60.0))
            .top(pos_y - px(20.0))
            .w(px(120.0))
            .h(px(40.0))
            .bg(gpui::rgba(0x1f6feb99))
            .rounded_md()
            .shadow_lg()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(gpui::white())
            .child(label)
    }
}

/// Represents a drop target for file operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTarget {
    /// Drop on a directory in the file list
    Directory(usize),
    /// Drop on the current directory (file list background)
    CurrentDirectory,
    /// Drop on a favorite in the sidebar
    Favorites,
    /// Drop on a specific favorite item
    FavoriteItem(usize),
}

/// Result of a drop operation
#[derive(Clone, Debug)]
pub enum DropResult {
    /// Files should be copied to the target
    Copy { sources: Vec<PathBuf>, target: PathBuf },
    /// Files should be moved to the target
    Move { sources: Vec<PathBuf>, target: PathBuf },
    /// A directory should be added to favorites
    AddToFavorites(PathBuf),
    /// Operation was cancelled or invalid
    Cancelled,
}

impl DropResult {
    /// Creates a copy operation
    pub fn copy(sources: Vec<PathBuf>, target: PathBuf) -> Self {
        Self::Copy { sources, target }
    }

    /// Creates a move operation
    pub fn move_files(sources: Vec<PathBuf>, target: PathBuf) -> Self {
        Self::Move { sources, target }
    }

    /// Creates an add to favorites operation
    pub fn add_favorite(path: PathBuf) -> Self {
        Self::AddToFavorites(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_drag_data_single() {
        let path = PathBuf::from("/home/user/file.txt");
        let data = FileDragData::single(path.clone());
        
        assert_eq!(data.file_count(), 1);
        assert!(data.is_single());
        assert_eq!(data.first_path(), Some(&path));
    }

    #[test]
    fn test_file_drag_data_multiple() {
        let paths = vec![
            PathBuf::from("/home/user/file1.txt"),
            PathBuf::from("/home/user/file2.txt"),
            PathBuf::from("/home/user/file3.txt"),
        ];
        let data = FileDragData::multiple(paths.clone());
        
        assert_eq!(data.file_count(), 3);
        assert!(!data.is_single());
        assert_eq!(data.first_path(), Some(&paths[0]));
    }

    #[test]
    fn test_file_drag_data_with_source_window() {
        let data = FileDragData::single(PathBuf::from("/test"))
            .with_source_window(42);
        
        assert_eq!(data.source_window_id, Some(42));
    }

    #[test]
    fn test_file_drag_data_with_position() {
        let data = FileDragData::single(PathBuf::from("/test"))
            .with_position(100.0, 200.0);
        
        assert_eq!(data.position, (100.0, 200.0));
    }

    #[test]
    fn test_drop_result_copy() {
        let sources = vec![PathBuf::from("/src/file.txt")];
        let target = PathBuf::from("/dst");
        let result = DropResult::copy(sources.clone(), target.clone());
        
        match result {
            DropResult::Copy { sources: s, target: t } => {
                assert_eq!(s, sources);
                assert_eq!(t, target);
            }
            _ => panic!("Expected Copy result"),
        }
    }

    #[test]
    fn test_drop_result_move() {
        let sources = vec![PathBuf::from("/src/file.txt")];
        let target = PathBuf::from("/dst");
        let result = DropResult::move_files(sources.clone(), target.clone());
        
        match result {
            DropResult::Move { sources: s, target: t } => {
                assert_eq!(s, sources);
                assert_eq!(t, target);
            }
            _ => panic!("Expected Move result"),
        }
    }

    #[test]
    fn test_drop_result_add_favorite() {
        let path = PathBuf::from("/home/user/projects");
        let result = DropResult::add_favorite(path.clone());
        
        match result {
            DropResult::AddToFavorites(p) => {
                assert_eq!(p, path);
            }
            _ => panic!("Expected AddToFavorites result"),
        }
    }
}
