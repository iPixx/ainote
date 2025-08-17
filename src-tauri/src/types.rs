use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::errors::{FileSystemError, FileSystemResult};
use crate::metadata_cache;

/// Window state structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    /// Window width in pixels
    pub width: f64,
    /// Window height in pixels
    pub height: f64,
    /// Window X position
    pub x: Option<i32>,
    /// Window Y position
    pub y: Option<i32>,
    /// Whether window is maximized
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 800.0,
            x: None,
            y: None,
            maximized: false,
        }
    }
}

/// Layout state structure for column management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    /// File tree panel width in pixels
    pub file_tree_width: f64,
    /// AI panel width in pixels (when visible)
    pub ai_panel_width: f64,
    /// Whether file tree is visible/open
    pub file_tree_visible: bool,
    /// Whether AI panel is visible/open
    pub ai_panel_visible: bool,
    /// Editor/preview mode: "edit", "preview", "split"
    pub editor_mode: String,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            file_tree_width: 280.0,
            ai_panel_width: 350.0,
            file_tree_visible: true,
            ai_panel_visible: false, // Hidden in Phase 1
            editor_mode: "edit".to_string(),
        }
    }
}

/// Combined application state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    /// Window state
    pub window: WindowState,
    /// Layout state
    pub layout: LayoutState,
}

/// FileInfo struct representing file metadata for frontend communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Full file path as string
    pub path: String,
    /// File name only (without directory path)
    pub name: String,
    /// Last modified timestamp (Unix time in seconds)
    pub modified: u64,
    /// File size in bytes
    pub size: u64,
    /// Whether the item is a directory
    pub is_dir: bool,
}

impl FileInfo {
    /// Create FileInfo from std::fs::DirEntry
    pub fn from_dir_entry(entry: &std::fs::DirEntry) -> FileSystemResult<Self> {
        let path = entry.path();
        let path_str = Self::path_to_string(&path);
        
        let name = Self::extract_name(&path);
        
        // Use cached metadata when possible
        let metadata = metadata_cache::get_metadata(&path)
            .map_err(|_| FileSystemError::MetadataError { path: path_str.clone() })?;

        let modified = Self::extract_modified_time(&metadata, &path_str)?;
        let size = metadata.len();
        let is_dir = metadata.is_dir();

        Ok(FileInfo {
            path: path_str,
            name,
            modified,
            size,
            is_dir,
        })
    }

    /// Create FileInfo from Path
    pub fn from_path(path: &Path) -> FileSystemResult<Self> {
        let path_str = Self::path_to_string(path);
        let name = Self::extract_name(path);
        
        // Use cached metadata when possible
        let metadata = metadata_cache::get_metadata(path)
            .map_err(|_| FileSystemError::MetadataError { path: path_str.clone() })?;

        let modified = Self::extract_modified_time(&metadata, &path_str)?;
        let size = metadata.len();
        let is_dir = metadata.is_dir();

        Ok(FileInfo {
            path: path_str,
            name,
            modified,
            size,
            is_dir,
        })
    }

    /// Cross-platform path to string conversion (optimized)
    fn path_to_string(path: &Path) -> String {
        // Use into_owned only when necessary to avoid unnecessary allocations
        match path.to_string_lossy() {
            std::borrow::Cow::Borrowed(s) => s.to_string(),
            std::borrow::Cow::Owned(s) => s,
        }
    }

    /// Extract file/directory name from path (optimized)
    fn extract_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Extract modified time with proper error handling
    fn extract_modified_time(metadata: &std::fs::Metadata, path_str: &str) -> FileSystemResult<u64> {
        let modified_time = metadata
            .modified()
            .map_err(|_| FileSystemError::MetadataError { path: path_str.to_string() })?;
            
        modified_time
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| FileSystemError::MetadataError { path: path_str.to_string() })
            .map(|duration| duration.as_secs())
    }

    /// Compare by name (case-insensitive alphabetical)
    pub fn compare_by_name(&self, other: &Self) -> std::cmp::Ordering {
        self.name.to_lowercase().cmp(&other.name.to_lowercase())
    }

    /// Compare by modification time (newer first when used with sort)
    pub fn compare_by_modified(&self, other: &Self) -> std::cmp::Ordering {
        self.modified.cmp(&other.modified)
    }

    /// Compare by file size (larger first when used with sort)
    pub fn compare_by_size(&self, other: &Self) -> std::cmp::Ordering {
        self.size.cmp(&other.size)
    }

    /// Normalize path separators for cross-platform compatibility
    pub fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
    }
    
    /// Normalize Unicode string for cross-platform compatibility
    pub fn normalize_unicode(text: &str) -> String {
        // Basic Unicode normalization - keep all characters as-is for now
        // This helps with filename compatibility across different filesystems
        // In the future, we could add more sophisticated normalization
        text.to_string()
    }

    /// Get file extension if present
    pub fn get_extension(&self) -> Option<String> {
        Path::new(&self.path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Check if this is a markdown file
    pub fn is_markdown(&self) -> bool {
        self.get_extension()
            .map(|ext| ext == "md")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    /// Test utilities for better isolation and common operations
    struct TestEnv {
        #[allow(dead_code)] // Required for automatic cleanup
        temp_dir: TempDir,
        pub path: PathBuf,
    }

    impl TestEnv {
        /// Create a new isolated test environment with temporary directory
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temporary directory");
            let path = temp_dir.path().to_path_buf();
            
            TestEnv {
                temp_dir,
                path,
            }
        }

        /// Create a test file with content
        fn create_test_file(&self, name: &str, content: &str) -> std::io::Result<()> {
            let file_path = self.path.join(name);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file_path, content)
        }

        fn get_test_file(&self, name: &str) -> String {
            self.path.join(name).to_string_lossy().to_string()
        }
    }

    const TEST_CONTENT: &str = "# Test Content\n\nThis is test content.";

    #[test]
    fn test_window_state_default() {
        let window_state = WindowState::default();
        
        assert_eq!(window_state.width, 1200.0);
        assert_eq!(window_state.height, 800.0);
        assert_eq!(window_state.x, None);
        assert_eq!(window_state.y, None);
        assert_eq!(window_state.maximized, false);
    }

    #[test]
    fn test_layout_state_default() {
        let layout_state = LayoutState::default();
        
        assert_eq!(layout_state.file_tree_width, 280.0);
        assert_eq!(layout_state.ai_panel_width, 350.0);
        assert_eq!(layout_state.file_tree_visible, true);
        assert_eq!(layout_state.ai_panel_visible, false);
        assert_eq!(layout_state.editor_mode, "edit");
    }

    #[test]
    fn test_app_state_default() {
        let app_state = AppState::default();
        
        assert_eq!(app_state.window.width, 1200.0);
        assert_eq!(app_state.window.height, 800.0);
        assert_eq!(app_state.layout.file_tree_width, 280.0);
        assert_eq!(app_state.layout.ai_panel_width, 350.0);
        assert_eq!(app_state.layout.file_tree_visible, true);
        assert_eq!(app_state.layout.ai_panel_visible, false);
    }

    #[test]
    fn test_file_info_from_path() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert_eq!(file_info.path, test_file);
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);
        assert!(file_info.modified > 0);
    }

    #[test]
    fn test_file_info_from_dir_entry() {
        let env = TestEnv::new();
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let entries: Vec<_> = fs::read_dir(&env.path).unwrap().collect();
        let entry = entries.into_iter().find(|e| {
            e.as_ref().unwrap().file_name() == "test.md"
        }).unwrap().unwrap();

        let file_info = FileInfo::from_dir_entry(&entry).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);
    }

    #[test]
    fn test_file_info_comparison() {
        let file1 = FileInfo {
            path: "a.md".to_string(),
            name: "a.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let file2 = FileInfo {
            path: "b.md".to_string(),
            name: "b.md".to_string(),
            modified: 200,
            size: 100,
            is_dir: false,
        };

        assert_eq!(file1.compare_by_name(&file2), std::cmp::Ordering::Less);
        assert_eq!(file1.compare_by_modified(&file2), std::cmp::Ordering::Less);
        assert_eq!(file1.compare_by_size(&file2), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_fileinfo_path_utilities() {
        // Test path normalization
        assert_eq!(FileInfo::normalize_path("C:\\path\\to\\file"), "C:/path/to/file");
        assert_eq!(FileInfo::normalize_path("/unix/path/file"), "/unix/path/file");
        assert_eq!(FileInfo::normalize_path("mixed\\path/file"), "mixed/path/file");
    }

    #[test]
    fn test_fileinfo_extension_methods() {
        let md_file = FileInfo {
            path: "/path/to/file.md".to_string(),
            name: "file.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let txt_file = FileInfo {
            path: "/path/to/file.TXT".to_string(),
            name: "file.TXT".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let no_ext_file = FileInfo {
            path: "/path/to/README".to_string(),
            name: "README".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        // Test extension extraction (should be lowercase)
        assert_eq!(md_file.get_extension(), Some("md".to_string()));
        assert_eq!(txt_file.get_extension(), Some("txt".to_string()));
        assert_eq!(no_ext_file.get_extension(), None);

        // Test markdown detection
        assert!(md_file.is_markdown());
        assert!(!txt_file.is_markdown());
        assert!(!no_ext_file.is_markdown());
    }

    #[test]
    fn test_fileinfo_serialization() {
        let file_info = FileInfo {
            path: "/path/to/file.md".to_string(),
            name: "file.md".to_string(),
            modified: 1640995200,
            size: 1024,
            is_dir: false,
        };

        // Test serialization to JSON
        let json = serde_json::to_string(&file_info).unwrap();
        assert!(json.contains("\"path\":\"/path/to/file.md\""));
        assert!(json.contains("\"name\":\"file.md\""));
        assert!(json.contains("\"modified\":1640995200"));
        assert!(json.contains("\"size\":1024"));
        assert!(json.contains("\"is_dir\":false"));

        // Test deserialization from JSON
        let deserialized: FileInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, file_info.path);
        assert_eq!(deserialized.name, file_info.name);
        assert_eq!(deserialized.modified, file_info.modified);
        assert_eq!(deserialized.size, file_info.size);
        assert_eq!(deserialized.is_dir, file_info.is_dir);
    }

    #[test]
    fn test_unicode_normalization() {
        let unicode_text = "测试文档_émojis_🎉";
        let normalized = FileInfo::normalize_unicode(unicode_text);
        assert_eq!(normalized, unicode_text);
    }

    #[test]
    fn test_file_info_directory() {
        let env = TestEnv::new();
        let dir_path = env.path.join("test_dir");
        fs::create_dir(&dir_path).unwrap();

        let file_info = FileInfo::from_path(&dir_path).unwrap();
        assert!(file_info.is_dir);
        assert_eq!(file_info.name, "test_dir");
        assert!(!file_info.is_markdown());
    }

    #[test]
    fn test_case_insensitive_comparison() {
        let file1 = FileInfo {
            path: "AAA.md".to_string(),
            name: "AAA.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let file2 = FileInfo {
            path: "bbb.md".to_string(),
            name: "bbb.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        // Case-insensitive comparison: "AAA" < "bbb"
        assert_eq!(file1.compare_by_name(&file2), std::cmp::Ordering::Less);
    }
}