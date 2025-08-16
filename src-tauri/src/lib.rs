use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Custom error types for file system operations
#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },
    
    #[error("Vault not selected or invalid")]
    VaultNotSelected,
    
    #[error("IO error: {message}")]
    IOError { message: String },
    
    #[error("Invalid file extension: {path} (only .md files are supported)")]
    InvalidExtension { path: String },
    
    #[error("File already exists: {path}")]
    FileAlreadyExists { path: String },
    
    #[error("Path is not a file: {path}")]
    NotAFile { path: String },
    
    #[error("Path is not a directory: {path}")]
    NotADirectory { path: String },
    
    #[error("Failed to read metadata for: {path}")]
    MetadataError { path: String },
    
    #[error("Failed to create directory: {path}")]
    DirectoryCreationError { path: String },
    
    #[error("UTF-8 encoding error in file: {path}")]
    EncodingError { path: String },
}

impl FileSystemError {
    /// Create a user-friendly error message for display in the frontend
    pub fn user_message(&self) -> String {
        match self {
            FileSystemError::FileNotFound { path } => {
                format!("The file '{}' could not be found. It may have been moved or deleted.", path)
            }
            FileSystemError::PermissionDenied { path } => {
                format!("Access denied to '{}'. Please check file permissions.", path)
            }
            FileSystemError::InvalidPath { path } => {
                format!("The path '{}' is not valid.", path)
            }
            FileSystemError::VaultNotSelected => {
                "Please select a vault folder first.".to_string()
            }
            FileSystemError::IOError { message } => {
                format!("File operation failed: {}", message)
            }
            FileSystemError::InvalidExtension { path } => {
                format!("The file '{}' is not a markdown file. Only .md files are supported.", path)
            }
            FileSystemError::FileAlreadyExists { path } => {
                format!("A file already exists at '{}'. Please choose a different name.", path)
            }
            FileSystemError::NotAFile { path } => {
                format!("'{}' is not a file.", path)
            }
            FileSystemError::NotADirectory { path } => {
                format!("'{}' is not a directory.", path)
            }
            FileSystemError::MetadataError { path } => {
                format!("Unable to read file information for '{}'.", path)
            }
            FileSystemError::DirectoryCreationError { path } => {
                format!("Failed to create directory '{}'.", path)
            }
            FileSystemError::EncodingError { path } => {
                format!("The file '{}' contains invalid text encoding.", path)
            }
        }
    }
}

/// Convert std::io::Error to FileSystemError
impl From<std::io::Error> for FileSystemError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::FileNotFound { 
                path: "unknown".to_string() 
            },
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: "unknown".to_string() 
            },
            _ => FileSystemError::IOError { 
                message: error.to_string() 
            },
        }
    }
}

/// Result type alias for our file system operations
pub type FileSystemResult<T> = Result<T, FileSystemError>;

/// Convert FileSystemError to String for Tauri commands
impl From<FileSystemError> for String {
    fn from(error: FileSystemError) -> Self {
        error.user_message()
    }
}

/// Helper functions for validation
pub mod validation {
    use super::*;
    
    /// Validate that a file path has a .md extension
    pub fn validate_markdown_extension(path: &Path) -> FileSystemResult<()> {
        match path.extension() {
            Some(ext) if ext == "md" => Ok(()),
            Some(_) => Err(FileSystemError::InvalidExtension { 
                path: path.to_string_lossy().to_string() 
            }),
            None => Err(FileSystemError::InvalidExtension { 
                path: path.to_string_lossy().to_string() 
            }),
        }
    }
    
    /// Validate that a path exists
    pub fn validate_path_exists(path: &Path) -> FileSystemResult<()> {
        if path.exists() {
            Ok(())
        } else {
            Err(FileSystemError::FileNotFound { 
                path: path.to_string_lossy().to_string() 
            })
        }
    }
    
    /// Validate that a path is a file
    pub fn validate_is_file(path: &Path) -> FileSystemResult<()> {
        if path.is_file() {
            Ok(())
        } else {
            Err(FileSystemError::NotAFile { 
                path: path.to_string_lossy().to_string() 
            })
        }
    }
    
    /// Validate that a path is a directory
    pub fn validate_is_directory(path: &Path) -> FileSystemResult<()> {
        if path.is_dir() {
            Ok(())
        } else {
            Err(FileSystemError::NotADirectory { 
                path: path.to_string_lossy().to_string() 
            })
        }
    }
    
    /// Validate that a file doesn't already exist
    pub fn validate_file_not_exists(path: &Path) -> FileSystemResult<()> {
        if !path.exists() {
            Ok(())
        } else {
            Err(FileSystemError::FileAlreadyExists { 
                path: path.to_string_lossy().to_string() 
            })
        }
    }
    
    /// Create parent directory if it doesn't exist
    pub fn ensure_parent_directory(path: &Path) -> FileSystemResult<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|_| FileSystemError::DirectoryCreationError { 
                        path: parent.to_string_lossy().to_string() 
                    })?;
            }
        }
        Ok(())
    }
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
        
        let metadata = entry.metadata()
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
        
        let metadata = path.metadata()
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
    fn extract_modified_time(metadata: &fs::Metadata, path_str: &str) -> FileSystemResult<u64> {
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

#[tauri::command]
fn read_file(file_path: String) -> Result<String, String> {
    read_file_internal(&file_path).map_err(|e| e.into())
}

/// Internal read file function using structured error handling
fn read_file_internal(file_path: &str) -> FileSystemResult<String> {
    let path = Path::new(file_path);

    // Validate path exists and is a file
    validation::validate_path_exists(path)?;
    validation::validate_is_file(path)?;
    validation::validate_markdown_extension(path)?;

    // Read file content with UTF-8 encoding
    fs::read_to_string(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::InvalidData => FileSystemError::EncodingError { 
                path: file_path.to_string() 
            },
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: file_path.to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to read file {}: {}", file_path, e) 
            },
        })
}

#[tauri::command]
fn write_file(file_path: String, content: String) -> Result<(), String> {
    write_file_internal(&file_path, &content).map_err(|e| e.into())
}

/// Internal write file function using structured error handling
fn write_file_internal(file_path: &str, content: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // Validate path and extension
    validation::validate_markdown_extension(path)?;

    // Create parent directory if it doesn't exist
    validation::ensure_parent_directory(path)?;

    // Write file content with UTF-8 encoding
    fs::write(path, content)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: file_path.to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to write file {}: {}", file_path, e) 
            },
        })
}

#[tauri::command]
fn create_file(file_path: String) -> Result<(), String> {
    create_file_internal(&file_path).map_err(|e| e.into())
}

/// Internal create file function using structured error handling
fn create_file_internal(file_path: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // Validate path and extension
    validation::validate_markdown_extension(path)?;

    // Check if file already exists
    validation::validate_file_not_exists(path)?;

    // Create parent directory if it doesn't exist
    validation::ensure_parent_directory(path)?;

    // Get filename without extension for the title
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");

    // Create markdown template
    let template = format!("# {}\n\n", title);

    // Create file with template content
    fs::write(path, template)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: file_path.to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to create file {}: {}", file_path, e) 
            },
        })
}

#[tauri::command]
fn delete_file(file_path: String) -> Result<(), String> {
    delete_file_internal(&file_path).map_err(|e| e.into())
}

/// Internal delete file function using structured error handling
fn delete_file_internal(file_path: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // Validate path exists and is a file
    validation::validate_path_exists(path)?;
    validation::validate_is_file(path)?;
    validation::validate_markdown_extension(path)?;

    // Delete the file
    fs::remove_file(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: file_path.to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to delete file {}: {}", file_path, e) 
            },
        })
}

#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    rename_file_internal(&old_path, &new_path).map_err(|e| e.into())
}

/// Internal rename file function using structured error handling
fn rename_file_internal(old_path: &str, new_path: &str) -> FileSystemResult<()> {
    let old = Path::new(old_path);
    let new = Path::new(new_path);

    // Validate old path exists and is a file
    validation::validate_path_exists(old)?;
    validation::validate_is_file(old)?;

    // Validate both paths have .md extension
    validation::validate_markdown_extension(old)?;
    validation::validate_markdown_extension(new)?;

    // Check if destination already exists
    validation::validate_file_not_exists(new)?;

    // Create destination directory if it doesn't exist
    validation::ensure_parent_directory(new)?;

    // Rename the file
    fs::rename(old, new)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: old_path.to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to rename file from {} to {}: {}", old_path, new_path, e) 
            },
        })
}

#[tauri::command]
async fn select_vault_folder() -> Result<Option<String>, String> {
    use rfd::AsyncFileDialog;
    
    let folder = AsyncFileDialog::new()
        .set_title("Select Vault Folder")
        .pick_folder()
        .await;

    match folder {
        Some(handle) => {
            let path = handle.path().to_string_lossy().to_string();
            Ok(Some(path))
        }
        None => Ok(None),
    }
}

#[tauri::command]
fn scan_vault_files(vault_path: String) -> Result<Vec<FileInfo>, String> {
    scan_vault_files_internal(&vault_path).map_err(|e| e.into())
}

/// Internal scan vault files function using structured error handling
fn scan_vault_files_internal(vault_path: &str) -> FileSystemResult<Vec<FileInfo>> {
    let vault_path = Path::new(vault_path);
    
    // Validate vault path exists and is a directory
    validation::validate_path_exists(vault_path)?;
    validation::validate_is_directory(vault_path)?;

    let mut files = Vec::new();
    
    // Recursive function to scan directories
    fn scan_directory(dir: &Path, files: &mut Vec<FileInfo>) -> FileSystemResult<()> {
        let entries = fs::read_dir(dir)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                    path: dir.to_string_lossy().to_string() 
                },
                _ => FileSystemError::IOError { 
                    message: format!("Failed to read directory {}: {}", dir.display(), e) 
                },
            })?;

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    // Log the error but continue with other entries
                    eprintln!("Warning: Failed to read directory entry in {}: {}", dir.display(), e);
                    continue;
                }
            };

            let path = entry.path();
            
            // Handle symbolic links gracefully
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    // Skip entries we can't read metadata for (broken symlinks, permission issues)
                    continue;
                }
            };

            if metadata.is_dir() {
                // Include directory in results
                if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
                    files.push(file_info);
                }
                
                // Recursively scan subdirectory
                if let Err(e) = scan_directory(&path, files) {
                    eprintln!("Warning: Error scanning subdirectory {}: {}", path.display(), e);
                }
            } else if metadata.is_file() {
                // Only include .md files
                if let Some(extension) = path.extension() {
                    if extension == "md" {
                        if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
                            files.push(file_info);
                        }
                    }
                }
            }
            // Skip other file types (symlinks, special files, etc.)
        }
        
        Ok(())
    }

    // Start recursive scanning
    scan_directory(vault_path, &mut files)?;
    
    // Sort files by name for consistent UI display (directories first, then files)
    files.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,   // Directories first
            (false, true) => std::cmp::Ordering::Greater, // Files second
            _ => a.compare_by_name(b),                    // Then alphabetical within each group
        }
    });
    
    Ok(files)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            create_file,
            delete_file,
            rename_file,
            select_vault_folder,
            scan_vault_files
        ])
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
    
    fn get_test_file_2() -> String {
        format!("{}/test2.md", get_test_dir())
    }
    
    const TEST_CONTENT: &str = "# Test Content\n\nThis is test content.";

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
    fn test_create_file_success() {
        setup_test_dir();

        let result = create_file(get_test_file());
        assert!(result.is_ok());
        assert!(Path::new(&get_test_file()).exists());

        let content = fs::read_to_string(&get_test_file()).unwrap();
        assert_eq!(content, "# test\n\n");

        cleanup_test_dir();
    }

    #[test]
    fn test_create_file_invalid_extension() {
        setup_test_dir();

        let result = create_file(format!("{}/test.txt", get_test_dir()));
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        println!("Actual error message: '{}'", error_msg);
        assert!(error_msg.contains("not a markdown file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_create_file_already_exists() {
        setup_test_dir();
        fs::write(&get_test_file(), "existing content").unwrap();

        let result = create_file(get_test_file());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("A file already exists at"));

        cleanup_test_dir();
    }

    #[test]
    fn test_write_file_success() {
        setup_test_dir();

        let result = write_file(get_test_file(), TEST_CONTENT.to_string());
        assert!(result.is_ok());
        assert!(Path::new(&get_test_file()).exists());

        let content = fs::read_to_string(&get_test_file()).unwrap();
        assert_eq!(content, TEST_CONTENT);

        cleanup_test_dir();
    }

    #[test]
    fn test_write_file_invalid_extension() {
        setup_test_dir();

        let result = write_file(format!("{}/test.txt", get_test_dir()), TEST_CONTENT.to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_success() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = read_file(get_test_file());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_CONTENT);

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_not_found() {
        setup_test_dir();

        let result = read_file(format!("{}/nonexistent.md", get_test_dir()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_invalid_extension() {
        setup_test_dir();
        fs::write(&format!("{}/test.txt", get_test_dir()), "content").unwrap();

        let result = read_file(format!("{}/test.txt", get_test_dir()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_is_directory() {
        setup_test_dir();
        fs::create_dir_all(&format!("{}/subdir.md", get_test_dir())).unwrap();

        let result = read_file(format!("{}/subdir.md", get_test_dir()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is not a file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_delete_file_success() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = delete_file(get_test_file());
        assert!(result.is_ok());
        assert!(!Path::new(&get_test_file()).exists());

        cleanup_test_dir();
    }

    #[test]
    fn test_delete_file_not_found() {
        setup_test_dir();

        let result = delete_file(format!("{}/nonexistent.md", get_test_dir()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));

        cleanup_test_dir();
    }

    #[test]
    fn test_delete_file_invalid_extension() {
        setup_test_dir();
        fs::write(&format!("{}/test.txt", get_test_dir()), "content").unwrap();

        let result = delete_file(format!("{}/test.txt", get_test_dir()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_success() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = rename_file(get_test_file(), get_test_file_2());
        assert!(result.is_ok());
        assert!(!Path::new(&get_test_file()).exists());
        assert!(Path::new(&get_test_file_2()).exists());

        let content = fs::read_to_string(&get_test_file_2()).unwrap();
        assert_eq!(content, TEST_CONTENT);

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_source_not_found() {
        setup_test_dir();

        let result = rename_file(
            format!("{}/nonexistent.md", get_test_dir()),
            get_test_file_2(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_destination_exists() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();
        fs::write(&get_test_file_2(), "other content").unwrap();

        let result = rename_file(get_test_file(), get_test_file_2());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("already exists"));

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_invalid_extension() {
        setup_test_dir();
        fs::write(&format!("{}/test.txt", get_test_dir()), "content").unwrap();

        let result = rename_file(format!("{}/test.txt", get_test_dir()), get_test_file_2());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_utf8_encoding() {
        setup_test_dir();
        let utf8_content = "# UTF-8 Test\n\n✅ Checkmark\n🎉 Emoji\nÀccëntéd characters";

        let write_result = write_file(get_test_file(), utf8_content.to_string());
        assert!(write_result.is_ok());

        let read_result = read_file(get_test_file());
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), utf8_content);

        cleanup_test_dir();
    }

    #[test]
    fn test_nested_directory_creation() {
        setup_test_dir();
        let nested_file = format!("{}/nested/deep/file.md", get_test_dir());

        let result = create_file(nested_file.clone());
        assert!(result.is_ok());
        assert!(Path::new(&nested_file).exists());

        let content = fs::read_to_string(&nested_file).unwrap();
        assert_eq!(content, "# file\n\n");

        cleanup_test_dir();
    }

    #[test]
    fn test_file_info_from_path() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let test_file = get_test_file();
        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert_eq!(file_info.path, get_test_file());
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);
        assert!(file_info.modified > 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_file_info_from_dir_entry() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let entries: Vec<_> = fs::read_dir(&get_test_dir()).unwrap().collect();
        let entry = entries.into_iter().find(|e| {
            e.as_ref().unwrap().file_name() == "test.md"
        }).unwrap().unwrap();

        let file_info = FileInfo::from_dir_entry(&entry).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);

        cleanup_test_dir();
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
    fn test_scan_vault_files_empty_directory() {
        setup_test_dir();

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_with_markdown_files() {
        setup_test_dir();
        
        // Create test files
        fs::write(format!("{}/note1.md", get_test_dir()), "# Note 1").unwrap();
        fs::write(format!("{}/note2.md", get_test_dir()), "# Note 2").unwrap();
        fs::write(format!("{}/readme.txt", get_test_dir()), "Not a markdown file").unwrap(); // Should be ignored

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 2); // Only .md files should be included
        
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        assert_eq!(md_files.len(), 2);
        
        // Check that files are sorted alphabetically
        assert!(md_files[0].name <= md_files[1].name);

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_nested_directories() {
        setup_test_dir();
        
        // Create nested structure
        fs::create_dir_all(format!("{}/subdir/deep", get_test_dir())).unwrap();
        fs::write(format!("{}/root.md", get_test_dir()), "# Root note").unwrap();
        fs::write(format!("{}/subdir/sub.md", get_test_dir()), "# Sub note").unwrap();
        fs::write(format!("{}/subdir/deep/deep.md", get_test_dir()), "# Deep note").unwrap();

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        
        // Should include directories and .md files
        let dirs: Vec<_> = files.iter().filter(|f| f.is_dir).collect();
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        
        assert_eq!(dirs.len(), 2); // subdir and deep
        assert_eq!(md_files.len(), 3); // root.md, sub.md, deep.md
        
        // Verify directories come first due to sorting
        let first_items: Vec<_> = files.iter().take(dirs.len()).collect();
        assert!(first_items.iter().all(|f| f.is_dir));

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_nonexistent_path() {
        let result = scan_vault_files("nonexistent_directory".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));
    }

    #[test]
    fn test_scan_vault_files_file_instead_of_directory() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = scan_vault_files(get_test_file());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is not a directory"));

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_performance_target() {
        setup_test_dir();
        
        // Create many files to test performance
        for i in 0..100 {
            fs::write(format!("{}/note_{:03}.md", get_test_dir(), i), format!("# Note {}", i)).unwrap();
        }

        let start = std::time::Instant::now();
        let result = scan_vault_files(get_test_dir());
        let duration = start.elapsed();

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.iter().filter(|f| !f.is_dir).count(), 100);
        
        // Performance target: <500ms for 1000+ files, so 100 files should be much faster
        assert!(duration.as_millis() < 100, "Scanning took too long: {:?}", duration);

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_mixed_file_types() {
        setup_test_dir();
        
        // Create various file types
        fs::write(format!("{}/note.md", get_test_dir()), "# Markdown note").unwrap();
        fs::write(format!("{}/document.txt", get_test_dir()), "Text document").unwrap();
        fs::write(format!("{}/script.js", get_test_dir()), "console.log('hello')").unwrap();
        fs::write(format!("{}/data.json", get_test_dir()), "{}").unwrap();
        fs::write(format!("{}/README", get_test_dir()), "No extension").unwrap();

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        let file_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        
        // Only the .md file should be included
        assert_eq!(file_files.len(), 1);
        assert_eq!(file_files[0].name, "note.md");

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_cross_platform_paths() {
        setup_test_dir();
        
        // Create a file and test that paths are handled properly
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        let file_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        assert_eq!(file_files.len(), 1);
        
        // Path should contain the correct separators for the platform
        let path = &file_files[0].path;
        assert!(path.contains("test.md"));
        
        // On Windows, path should use backslashes; on Unix, forward slashes
        #[cfg(windows)]
        assert!(path.contains("\\"));
        #[cfg(unix)]
        assert!(path.contains("/"));

        cleanup_test_dir();
    }

    // Error handling and integration tests
    mod error_handling_tests {
        use super::*;

        #[test]
        fn test_filesystem_error_user_messages() {
            let errors = vec![
                FileSystemError::FileNotFound { path: "/test/file.md".to_string() },
                FileSystemError::PermissionDenied { path: "/test/file.md".to_string() },
                FileSystemError::InvalidPath { path: "/invalid".to_string() },
                FileSystemError::VaultNotSelected,
                FileSystemError::IOError { message: "Test error".to_string() },
                FileSystemError::InvalidExtension { path: "/test/file.txt".to_string() },
                FileSystemError::FileAlreadyExists { path: "/test/file.md".to_string() },
                FileSystemError::NotAFile { path: "/test/dir".to_string() },
                FileSystemError::NotADirectory { path: "/test/file.md".to_string() },
                FileSystemError::MetadataError { path: "/test/file.md".to_string() },
                FileSystemError::DirectoryCreationError { path: "/test/dir".to_string() },
                FileSystemError::EncodingError { path: "/test/file.md".to_string() },
            ];

            for error in errors {
                let user_msg = error.user_message();
                assert!(!user_msg.is_empty());
                assert!(user_msg.len() > 10); // Should be descriptive
            }
        }

        #[test]
        fn test_validation_functions() {
            setup_test_dir();
            
            // Test markdown extension validation
            let md_path = Path::new("test.md");
            let txt_path = Path::new("test.txt");
            let no_ext_path = Path::new("test");
            
            assert!(validation::validate_markdown_extension(md_path).is_ok());
            assert!(validation::validate_markdown_extension(txt_path).is_err());
            assert!(validation::validate_markdown_extension(no_ext_path).is_err());

            // Test path existence validation
            let test_file_path = get_test_file();
            let test_dir_path = get_test_dir();
            let non_existing_file = format!("{}/nonexistent.md", test_dir_path);
            
            fs::write(&test_file_path, "test").unwrap();
            let existing_path = Path::new(&test_file_path);
            let non_existing_path = Path::new(&non_existing_file);
            let test_dir = Path::new(&test_dir_path);
            
            assert!(validation::validate_path_exists(existing_path).is_ok());
            assert!(validation::validate_path_exists(non_existing_path).is_err());

            // Test file validation
            assert!(validation::validate_is_file(existing_path).is_ok());
            assert!(validation::validate_is_file(test_dir).is_err());

            // Test directory validation
            assert!(validation::validate_is_directory(test_dir).is_ok());
            assert!(validation::validate_is_directory(existing_path).is_err());

            // Test file not exists validation
            assert!(validation::validate_file_not_exists(non_existing_path).is_ok());
            assert!(validation::validate_file_not_exists(existing_path).is_err());

            cleanup_test_dir();
        }

        #[test]
        fn test_error_conversion_to_string() {
            let error = FileSystemError::FileNotFound { path: "/test/file.md".to_string() };
            let error_string: String = error.into();
            assert!(error_string.contains("could not be found"));
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_complete_file_lifecycle() {
            setup_test_dir();
            let file_path = get_test_file();
            
            // 1. Create file
            let result = create_file(file_path.clone());
            assert!(result.is_ok(), "Failed to create file: {:?}", result);
            assert!(Path::new(&file_path).exists());

            // 2. Read file
            let content = read_file(file_path.clone()).unwrap();
            assert!(content.starts_with("# test"));

            // 3. Write to file
            let new_content = "# Updated Content\n\nThis is updated content.";
            let result = write_file(file_path.clone(), new_content.to_string());
            assert!(result.is_ok(), "Failed to write file: {:?}", result);

            // 4. Read updated content
            let updated_content = read_file(file_path.clone()).unwrap();
            assert_eq!(updated_content, new_content);

            // 5. Rename file
            let new_path = get_test_file_2();
            let result = rename_file(file_path.clone(), new_path.clone());
            assert!(result.is_ok(), "Failed to rename file: {:?}", result);
            assert!(!Path::new(&file_path).exists());
            assert!(Path::new(&new_path).exists());

            // 6. Delete file
            let result = delete_file(new_path.clone());
            assert!(result.is_ok(), "Failed to delete file: {:?}", result);
            assert!(!Path::new(&new_path).exists());

            cleanup_test_dir();
        }

        #[test]
        fn test_vault_scanning_comprehensive() {
            setup_test_dir();
            
            // Create complex directory structure
            let dirs = vec![
                "folder1",
                "folder1/subfolder",
                "folder2",
                "folder2/deep/nested"
            ];
            
            for dir in &dirs {
                fs::create_dir_all(format!("{}/{}", get_test_dir(), dir)).unwrap();
            }

            // Create various files
            let files = vec![
                ("root.md", "# Root"),
                ("folder1/note1.md", "# Note 1"),
                ("folder1/subfolder/note2.md", "# Note 2"),
                ("folder2/note3.md", "# Note 3"),
                ("folder2/deep/nested/note4.md", "# Note 4"),
                ("folder1/readme.txt", "Not markdown"), // Should be ignored
                ("folder2/config.json", "{}"), // Should be ignored
            ];

            for (path, content) in &files {
                fs::write(format!("{}/{}", get_test_dir(), path), content).unwrap();
            }

            // Scan vault
            let result = scan_vault_files(get_test_dir());
            assert!(result.is_ok(), "Failed to scan vault: {:?}", result);
            
            let scanned_files = result.unwrap();
            
            // Count different types
            let directories: Vec<_> = scanned_files.iter().filter(|f| f.is_dir).collect();
            let md_files: Vec<_> = scanned_files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
            
            assert_eq!(directories.len(), 5, "Expected 5 directories (including intermediate 'deep' directory)");
            assert_eq!(md_files.len(), 5, "Expected 5 markdown files");

            // Verify all markdown files are found
            let expected_md_files = vec!["root.md", "note1.md", "note2.md", "note3.md", "note4.md"];
            for expected in &expected_md_files {
                assert!(md_files.iter().any(|f| f.name == *expected), 
                       "Missing file: {}", expected);
            }

            cleanup_test_dir();
        }

        #[test]
        fn test_error_propagation_consistency() {
            setup_test_dir();
            
            let non_existent = format!("{}/nonexistent.md", get_test_dir());
            let invalid_ext = format!("{}/test.txt", get_test_dir());
            
            // Test that all commands handle missing files consistently
            assert!(read_file(non_existent.clone()).is_err());
            assert!(delete_file(non_existent.clone()).is_err());
            
            // Test that all commands handle invalid extensions consistently
            assert!(read_file(invalid_ext.clone()).is_err());
            assert!(write_file(invalid_ext.clone(), "content".to_string()).is_err());
            assert!(create_file(invalid_ext.clone()).is_err());
            assert!(delete_file(invalid_ext.clone()).is_err());

            cleanup_test_dir();
        }
    }

    mod performance_tests {
        use super::*;

        #[test]
        fn test_large_vault_performance() {
            setup_test_dir();
            
            // Create a larger test set (500 files in 10 directories)
            for dir_i in 0..10 {
                let dir_path = format!("{}/dir_{:02}", get_test_dir(), dir_i);
                fs::create_dir_all(&dir_path).unwrap();
                
                for file_i in 0..50 {
                    let file_path = format!("{}/note_{:03}.md", dir_path, file_i);
                    fs::write(&file_path, format!("# Note {} in Directory {}", file_i, dir_i)).unwrap();
                }
            }

            // Measure scanning performance
            let start = std::time::Instant::now();
            let result = scan_vault_files(get_test_dir());
            let scan_duration = start.elapsed();

            assert!(result.is_ok());
            let files = result.unwrap();
            
            // Should find 10 directories + 500 files = 510 total
            let file_count = files.iter().filter(|f| !f.is_dir).count();
            let dir_count = files.iter().filter(|f| f.is_dir).count();
            
            assert_eq!(file_count, 500, "Expected 500 markdown files");
            assert_eq!(dir_count, 10, "Expected 10 directories");

            // Performance target: Should complete in reasonable time
            assert!(scan_duration.as_millis() < 1000, 
                   "Scanning 500 files took too long: {:?}", scan_duration);

            // Test individual file operations performance
            let test_file = format!("{}/dir_01/note_001.md", get_test_dir());
            
            // Read performance
            let start = std::time::Instant::now();
            let content = read_file(test_file.clone()).unwrap();
            let read_duration = start.elapsed();
            assert!(read_duration.as_millis() < 50, "File read took too long: {:?}", read_duration);
            
            // Write performance
            let start = std::time::Instant::now();
            let result = write_file(test_file.clone(), content + "\n\nUpdated content");
            let write_duration = start.elapsed();
            assert!(result.is_ok());
            assert!(write_duration.as_millis() < 50, "File write took too long: {:?}", write_duration);

            cleanup_test_dir();
        }

        #[test] 
        fn test_memory_efficiency() {
            setup_test_dir();
            
            // Create files with larger content to test memory usage
            for i in 0..20 {
                let large_content = "# Large File\n\n".to_string() + 
                    &"This is a line of content that repeats many times to create a larger file.\n".repeat(100);
                let file_path = format!("{}/large_file_{:02}.md", get_test_dir(), i);
                fs::write(&file_path, large_content).unwrap();
            }

            // Test that scanning doesn't load all file contents into memory
            let result = scan_vault_files(get_test_dir());
            assert!(result.is_ok());
            
            let files = result.unwrap();
            let large_files: Vec<_> = files.iter().filter(|f| f.name.starts_with("large_file")).collect();
            assert_eq!(large_files.len(), 20);
            
            // Verify that FileInfo contains metadata but not content
            for file in &large_files {
                assert!(file.size > 1000); // Should be large
                assert!(file.modified > 0); // Should have valid timestamp
                // FileInfo should not contain actual file content
            }

            cleanup_test_dir();
        }
    }

    mod cross_platform_tests {
        use super::*;

        #[test]
        fn test_path_normalization() {
            let windows_path = r"C:\Users\Test\Documents\notes\file.md";
            let unix_path = "/home/test/Documents/notes/file.md";
            let mixed_path = r"C:\Users\Test/Documents\notes/file.md";

            assert_eq!(FileInfo::normalize_path(windows_path), "C:/Users/Test/Documents/notes/file.md");
            assert_eq!(FileInfo::normalize_path(unix_path), "/home/test/Documents/notes/file.md");
            assert_eq!(FileInfo::normalize_path(mixed_path), "C:/Users/Test/Documents/notes/file.md");
        }

        #[test]
        fn test_unicode_file_handling() {
            setup_test_dir();
            
            // Test with unicode filenames and content
            let unicode_filename = format!("{}/测试文档_émojis_🎉.md", get_test_dir());
            let unicode_content = "# Unicode Test 测试\n\n**Bold text** with émojis 🎉🚀\n\n中文内容测试";

            // Create file with unicode name and content
            let result = write_file(unicode_filename.clone(), unicode_content.to_string());
            assert!(result.is_ok(), "Failed to write unicode file: {:?}", result);

            // Read back the content
            let read_content = read_file(unicode_filename.clone()).unwrap();
            assert_eq!(read_content, unicode_content);

            // Test file info extraction with unicode
            let path = Path::new(&unicode_filename);
            let file_info = FileInfo::from_path(path).unwrap();
            assert!(file_info.name.contains("测试文档"));
            assert!(file_info.name.contains("🎉"));

            cleanup_test_dir();
        }

        #[test]
        fn test_special_characters_in_paths() {
            setup_test_dir();
            
            // Test with various special characters that are valid in filenames
            let special_files = vec![
                "file with spaces.md",
                "file-with-dashes.md",
                "file_with_underscores.md",
                "file.with.dots.md",
                "file(with)parentheses.md",
                "file[with]brackets.md",
            ];

            for filename in &special_files {
                let file_path = format!("{}/{}", get_test_dir(), filename);
                let content = format!("# {}\n\nContent for file with special characters.", filename);
                
                let result = write_file(file_path.clone(), content.clone());
                assert!(result.is_ok(), "Failed to write file with special chars: {}", filename);
                
                let read_content = read_file(file_path).unwrap();
                assert_eq!(read_content, content);
            }

            // Test scanning finds all files
            let scanned = scan_vault_files(get_test_dir()).unwrap();
            let md_files: Vec<_> = scanned.iter().filter(|f| !f.is_dir).collect();
            assert_eq!(md_files.len(), special_files.len());

            cleanup_test_dir();
        }
    }
}
