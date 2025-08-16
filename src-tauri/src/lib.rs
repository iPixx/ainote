use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

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
    pub fn from_dir_entry(entry: &std::fs::DirEntry) -> Result<Self, String> {
        let path = entry.path();
        let path_str = Self::path_to_string(&path);
        
        let name = Self::extract_name(&path);
        
        let metadata = entry.metadata()
            .map_err(|e| format!("Failed to read metadata for {}: {}", path_str, e))?;

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
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let path_str = Self::path_to_string(path);
        let name = Self::extract_name(path);
        
        let metadata = path.metadata()
            .map_err(|e| format!("Failed to read metadata for {}: {}", path_str, e))?;

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

    /// Cross-platform path to string conversion
    fn path_to_string(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    /// Extract file/directory name from path
    fn extract_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }

    /// Extract modified time with proper error handling
    fn extract_modified_time(metadata: &fs::Metadata, path_str: &str) -> Result<u64, String> {
        metadata
            .modified()
            .map_err(|e| format!("Failed to read modified time for {}: {}", path_str, e))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Invalid modified time for {}: {}", path_str, e))
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

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn get_test_dir() -> String {
        format!("test_files_{}", std::process::id())
    }
    
    fn get_test_file() -> String {
        format!("{}/test.md", get_test_dir())
    }
    
    fn setup_test_dir() {
        let test_dir = get_test_dir();
        if Path::new(&test_dir).exists() {
            fs::remove_dir_all(&test_dir).ok();
        }
        fs::create_dir_all(&test_dir).unwrap();
    }

    fn cleanup_test_dir() {
        let test_dir = get_test_dir();
        if Path::new(&test_dir).exists() {
            fs::remove_dir_all(&test_dir).ok();
        }
    }

    #[test]
    fn test_fileinfo_struct_creation() {
        let file_info = FileInfo {
            path: "/path/to/file.md".to_string(),
            name: "file.md".to_string(),
            modified: 1640995200, // 2022-01-01 00:00:00 UTC
            size: 1024,
            is_dir: false,
        };

        assert_eq!(file_info.path, "/path/to/file.md");
        assert_eq!(file_info.name, "file.md");
        assert_eq!(file_info.modified, 1640995200);
        assert_eq!(file_info.size, 1024);
        assert!(!file_info.is_dir);
    }

    #[test]
    fn test_fileinfo_from_path() {
        setup_test_dir();
        let test_content = "# Test File\n\nThis is a test.";
        fs::write(&get_test_file(), test_content).unwrap();

        let test_file = get_test_file();
        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert!(file_info.path.contains("test.md"));
        assert!(!file_info.is_dir);
        assert_eq!(file_info.size, test_content.len() as u64);
        assert!(file_info.modified > 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_from_dir_entry() {
        setup_test_dir();
        let test_content = "# Test File\n\nThis is a test.";
        fs::write(&get_test_file(), test_content).unwrap();

        let entries: Vec<_> = fs::read_dir(&get_test_dir()).unwrap().collect();
        let entry = entries.into_iter()
            .find(|e| e.as_ref().unwrap().file_name() == "test.md")
            .unwrap().unwrap();

        let file_info = FileInfo::from_dir_entry(&entry).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert!(file_info.path.contains("test.md"));
        assert!(!file_info.is_dir);
        assert_eq!(file_info.size, test_content.len() as u64);
        assert!(file_info.modified > 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_directory() {
        setup_test_dir();
        let subdir = format!("{}/subdir", get_test_dir());
        fs::create_dir(&subdir).unwrap();

        let path = Path::new(&subdir);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.name, "subdir");
        assert!(file_info.path.contains("subdir"));
        assert!(file_info.is_dir);
        assert!(file_info.modified > 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_comparison_by_name() {
        let file_a = FileInfo {
            path: "a.md".to_string(),
            name: "a.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let file_b = FileInfo {
            path: "B.md".to_string(),
            name: "B.md".to_string(),
            modified: 200,
            size: 100,
            is_dir: false,
        };

        let file_z = FileInfo {
            path: "z.md".to_string(),
            name: "z.md".to_string(),
            modified: 300,
            size: 150,
            is_dir: false,
        };

        // Test case-insensitive comparison
        assert_eq!(file_a.compare_by_name(&file_b), std::cmp::Ordering::Less);
        assert_eq!(file_b.compare_by_name(&file_z), std::cmp::Ordering::Less);
        assert_eq!(file_z.compare_by_name(&file_a), std::cmp::Ordering::Greater);
        assert_eq!(file_a.compare_by_name(&file_a), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_fileinfo_comparison_by_modified() {
        let file_old = FileInfo {
            path: "old.md".to_string(),
            name: "old.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let file_new = FileInfo {
            path: "new.md".to_string(),
            name: "new.md".to_string(),
            modified: 200,
            size: 100,
            is_dir: false,
        };

        assert_eq!(file_old.compare_by_modified(&file_new), std::cmp::Ordering::Less);
        assert_eq!(file_new.compare_by_modified(&file_old), std::cmp::Ordering::Greater);
        assert_eq!(file_old.compare_by_modified(&file_old), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_fileinfo_comparison_by_size() {
        let file_small = FileInfo {
            path: "small.md".to_string(),
            name: "small.md".to_string(),
            modified: 100,
            size: 50,
            is_dir: false,
        };

        let file_large = FileInfo {
            path: "large.md".to_string(),
            name: "large.md".to_string(),
            modified: 200,
            size: 100,
            is_dir: false,
        };

        assert_eq!(file_small.compare_by_size(&file_large), std::cmp::Ordering::Less);
        assert_eq!(file_large.compare_by_size(&file_small), std::cmp::Ordering::Greater);
        assert_eq!(file_small.compare_by_size(&file_small), std::cmp::Ordering::Equal);
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
    fn test_fileinfo_special_characters_in_names() {
        setup_test_dir();
        
        // Test files with special characters
        let special_names = vec![
            "file with spaces.md",
            "file-with-dashes.md",
            "file_with_underscores.md",
            "file.with.dots.md",
            "file(with)parentheses.md",
        ];

        for name in special_names {
            let file_path = format!("{}/{}", get_test_dir(), name);
            fs::write(&file_path, "# Test").unwrap();
            
            let path = Path::new(&file_path);
            let file_info = FileInfo::from_path(path).unwrap();
            
            assert_eq!(file_info.name, name);
            assert!(file_info.path.contains(name));
        }

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_cross_platform_paths() {
        setup_test_dir();
        fs::write(&get_test_file(), "# Test").unwrap();

        let test_file = get_test_file();
        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        // Path should be properly formatted for the current platform
        #[cfg(windows)]
        assert!(file_info.path.contains("\\") || file_info.path.contains("/"));
        #[cfg(unix)]
        assert!(file_info.path.contains("/"));

        // Name should always be just the filename regardless of platform
        assert_eq!(file_info.name, "test.md");

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_large_file_size() {
        setup_test_dir();
        
        // Create a larger test file
        let large_content = "x".repeat(10000);
        fs::write(&get_test_file(), &large_content).unwrap();

        let test_file = get_test_file();
        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.size, large_content.len() as u64);
        assert_eq!(file_info.size, 10000);

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_empty_file() {
        setup_test_dir();
        
        // Create an empty file
        fs::write(&get_test_file(), "").unwrap();

        let test_file = get_test_file();
        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.size, 0);
        assert!(!file_info.is_dir);

        cleanup_test_dir();
    }

    #[test]
    fn test_fileinfo_error_handling() {
        // Test with non-existent file
        let result = FileInfo::from_path(Path::new("/non/existent/file.md"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read metadata"));
    }

    #[test]
    fn test_fileinfo_sorting_behavior() {
        let mut files = vec![
            FileInfo {
                path: "z.md".to_string(),
                name: "z.md".to_string(),
                modified: 100,
                size: 50,
                is_dir: false,
            },
            FileInfo {
                path: "a.md".to_string(),
                name: "a.md".to_string(),
                modified: 300,
                size: 200,
                is_dir: false,
            },
            FileInfo {
                path: "m.md".to_string(),
                name: "m.md".to_string(),
                modified: 200,
                size: 100,
                is_dir: false,
            },
        ];

        // Sort by name
        files.sort_by(|a, b| a.compare_by_name(b));
        assert_eq!(files[0].name, "a.md");
        assert_eq!(files[1].name, "m.md");
        assert_eq!(files[2].name, "z.md");

        // Sort by modified time
        files.sort_by(|a, b| a.compare_by_modified(b));
        assert_eq!(files[0].modified, 100);
        assert_eq!(files[1].modified, 200);
        assert_eq!(files[2].modified, 300);

        // Sort by size
        files.sort_by(|a, b| a.compare_by_size(b));
        assert_eq!(files[0].size, 50);
        assert_eq!(files[1].size, 100);
        assert_eq!(files[2].size, 200);
    }
}
