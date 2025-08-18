use std::fs;
use std::path::Path;
use crate::errors::{FileSystemError, FileSystemResult, IOErrorContext};
use crate::validation;
use crate::file_locks::FileLockGuard;
use crate::performance::time_operation;
use crate::types::FileInfo;

/// Internal read file function using structured error handling
pub fn read_file_internal(file_path: &str) -> FileSystemResult<String> {
    time_operation!({
        let path = Path::new(file_path);

        // Validate path exists and is a file
        validation::validate_path_exists(path)?;
        validation::validate_is_file(path)?;
        validation::validate_markdown_extension(path)?;
        validation::validate_existing_file_size(path)?;

        // Read file content with UTF-8 encoding
        fs::read_to_string(path)
            .with_path_context(file_path, "read")
    }, &format!("read_file({})", file_path))
}

/// Internal preview file function for large files
pub fn preview_file_internal(file_path: &str, max_length: usize) -> FileSystemResult<String> {
    let path = Path::new(file_path);

    // Validate path exists and is a file
    validation::validate_path_exists(path)?;
    validation::validate_is_file(path)?;
    validation::validate_markdown_extension(path)?;

    // Read file content with UTF-8 encoding
    let full_content = fs::read_to_string(path)
        .with_path_context(file_path, "preview")?;

    // Return preview (truncated if necessary)
    if full_content.len() <= max_length {
        Ok(full_content)
    } else {
        // Find a good break point (end of line or word)
        let truncated = &full_content[..max_length];
        if let Some(last_newline) = truncated.rfind('\n') {
            Ok(format!("{}...\n\n[File preview truncated - full file is {} characters]", 
                &truncated[..last_newline], full_content.len()))
        } else if let Some(last_space) = truncated.rfind(' ') {
            Ok(format!("{}...\n\n[File preview truncated - full file is {} characters]", 
                &truncated[..last_space], full_content.len()))
        } else {
            Ok(format!("{}...\n\n[File preview truncated - full file is {} characters]", 
                truncated, full_content.len()))
        }
    }
}

/// Internal auto-save file function (optimized for frequent saves)
pub fn auto_save_file_internal(file_path: &str, content: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // For auto-save, we don't need file locking as aggressively since it's the same user
    // But we still validate and ensure basic safety
    
    // Validate path and extension
    validation::validate_markdown_extension(path)?;
    validation::validate_file_size(content, file_path)?;

    // Create parent directory if it doesn't exist
    validation::ensure_parent_directory(path)?;

    // Only create backup every 10th auto-save to avoid too many backup files
    let should_backup = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() % 10 == 0;
    
    if should_backup {
        let _backup_path = validation::create_backup(path)?;
    }

    // Write file content with UTF-8 encoding
    fs::write(path, content)
        .with_path_context(file_path, "auto-save")
}

/// Internal write file function using structured error handling
pub fn write_file_internal(file_path: &str, content: &str) -> FileSystemResult<()> {
    time_operation!({
        let path = Path::new(file_path);

        // Acquire file lock to prevent concurrent access
        let _lock = FileLockGuard::acquire(file_path)?;

        // Validate path and extension
        validation::validate_markdown_extension(path)?;
        validation::validate_file_size(content, file_path)?;

        // Create parent directory if it doesn't exist
        validation::ensure_parent_directory(path)?;

        // Create backup if file exists
        let _backup_path = validation::create_backup(path)?;

        // Write file content with UTF-8 encoding
        let write_result = fs::write(path, content)
            .with_path_context(file_path, "write");

        // If write was successful, clean up old backups
        if write_result.is_ok() {
            let _ = validation::cleanup_old_backups(path); // Don't fail on cleanup errors
        }

        write_result
    }, &format!("write_file({}, {} bytes)", file_path, content.len()))
}

/// Internal create file function using structured error handling
pub fn create_file_internal(file_path: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // Acquire file lock to prevent concurrent access
    let _lock = FileLockGuard::acquire(file_path)?;

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

/// Internal delete file function using structured error handling
pub fn delete_file_internal(file_path: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // Acquire file lock to prevent concurrent access
    let _lock = FileLockGuard::acquire(file_path)?;

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

/// Internal rename file function using structured error handling
pub fn rename_file_internal(old_path: &str, new_path: &str) -> FileSystemResult<()> {
    let old = Path::new(old_path);
    let new = Path::new(new_path);

    // Acquire locks for both source and destination files
    let _old_lock = FileLockGuard::acquire(old_path)?;
    let _new_lock = FileLockGuard::acquire(new_path)?;

    // Validate old path exists
    validation::validate_path_exists(old)?;

    // Check if it's a file or directory and validate accordingly
    let is_directory = old.is_dir();
    if !is_directory {
        // For files, validate it's a file and has .md extension
        validation::validate_is_file(old)?;
        validation::validate_markdown_extension(old)?;
        validation::validate_markdown_extension(new)?;
    }
    // For directories, no extension validation needed

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

/// Internal get file info function for retrieving metadata of a specific file
pub fn get_file_info_internal(file_path: &str) -> FileSystemResult<FileInfo> {
    time_operation!({
        let path = Path::new(file_path);

        // Validate path exists
        validation::validate_path_exists(path)?;

        // Create FileInfo from path
        FileInfo::from_path(path)
    }, &format!("get_file_info({})", file_path))
}

/// Internal create folder function using structured error handling
pub fn create_folder_internal(folder_path: &str) -> FileSystemResult<()> {
    time_operation!({
        let path = Path::new(folder_path);

        // Check if folder already exists
        if path.exists() {
            return Err(FileSystemError::FileAlreadyExists { 
                path: folder_path.to_string() 
            });
        }

        // Create parent directories if they don't exist
        validation::ensure_parent_directory(path)?;

        // Create the directory
        fs::create_dir_all(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                    path: folder_path.to_string() 
                },
                _ => FileSystemError::IOError { 
                    message: format!("Failed to create folder {}: {}", folder_path, e) 
                },
            })
    }, &format!("create_folder({})", folder_path))
}

/// Reveal file in system file manager (Finder on macOS, Explorer on Windows, file manager on Linux)
pub fn reveal_in_finder_internal(file_path: &str) -> FileSystemResult<()> {
    let path = Path::new(file_path);

    // Validate path exists
    validation::validate_path_exists(path)?;

    // Use the opener plugin to reveal the file
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .args(["-R", file_path])
            .spawn()
            .map_err(|e| FileSystemError::IOError {
                message: format!("Failed to reveal file in Finder: {} ({})", file_path, e),
            })?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer")
            .args(["/select,", file_path])
            .spawn()
            .map_err(|e| FileSystemError::IOError {
                message: format!("Failed to reveal file in Explorer: {} ({})", file_path, e),
            })?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        // Try different file managers commonly available on Linux
        let file_managers = ["nautilus", "dolphin", "thunar", "nemo", "pcmanfm"];
        
        for &manager in &file_managers {
            if Command::new("which")
                .arg(manager)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
            {
                Command::new(manager)
                    .arg(file_path)
                    .spawn()
                    .map_err(|e| FileSystemError::IOError {
                        message: format!("Failed to reveal file in {}: {} ({})", manager, file_path, e),
                    })?;
                return Ok(());
            }
        }
        
        // Fallback: just open the parent directory
        if let Some(parent) = path.parent() {
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| FileSystemError::IOError {
                    message: format!("Failed to open parent directory: {} ({})", file_path, e),
                })?;
        }
    }

    Ok(())
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

        /// Get path to a test file in the test directory
        fn get_test_file(&self, name: &str) -> String {
            self.path.join(name).to_string_lossy().to_string()
        }

        /// Create a test file with content
        fn create_test_file(&self, name: &str, content: &str) -> std::io::Result<()> {
            let file_path = self.path.join(name);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file_path, content)
        }

        /// Create a test directory structure
        fn create_directory_structure(&self, dirs: &[&str]) -> std::io::Result<()> {
            for dir in dirs {
                let dir_path = self.path.join(dir);
                fs::create_dir_all(dir_path)?;
            }
            Ok(())
        }

        /// Get the temp directory path as string
        fn get_path(&self) -> String {
            self.path.to_string_lossy().to_string()
        }
    }

    const TEST_CONTENT: &str = "# Test Content\n\nThis is test content.";
    const UTF8_CONTENT: &str = "# UTF-8 Test\n\n✅ Checkmark\n🎉 Emoji\nÀccëntéd characters";

    #[test]
    fn test_create_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let result = create_file_internal(&test_file);
        assert!(result.is_ok());
        assert!(Path::new(&test_file).exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "# test\n\n");
    }

    #[test]
    fn test_create_file_invalid_extension() {
        let env = TestEnv::new();
        let invalid_file = env.get_test_file("test.txt");

        let result = create_file_internal(&invalid_file);
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("InvalidExtension"));
    }

    #[test]
    fn test_create_file_already_exists() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        
        // Create the file first
        env.create_test_file("test.md", "existing content").unwrap();

        let result = create_file_internal(&test_file);
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("FileAlreadyExists"));
    }

    #[test]
    fn test_write_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let result = write_file_internal(&test_file, TEST_CONTENT);
        assert!(result.is_ok());
        assert!(Path::new(&test_file).exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, TEST_CONTENT);
    }

    #[test]
    fn test_write_file_invalid_extension() {
        let env = TestEnv::new();
        let invalid_file = env.get_test_file("test.txt");

        let result = write_file_internal(&invalid_file, TEST_CONTENT);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = read_file_internal(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_CONTENT);
    }

    #[test]
    fn test_read_file_not_found() {
        let env = TestEnv::new();

        let result = read_file_internal(&format!("{}/nonexistent.md", env.get_path()));
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_invalid_extension() {
        let env = TestEnv::new();
        env.create_test_file("test.txt", "content").unwrap();

        let result = read_file_internal(&env.get_test_file("test.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_is_directory() {
        let env = TestEnv::new();
        env.create_directory_structure(&["subdir.md"]).unwrap();

        let result = read_file_internal(&env.get_test_file("subdir.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = delete_file_internal(&test_file);
        assert!(result.is_ok());
        assert!(!Path::new(&test_file).exists());
    }

    #[test]
    fn test_delete_file_not_found() {
        let env = TestEnv::new();

        let result = delete_file_internal(&format!("{}/nonexistent.md", env.get_path()));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_file_invalid_extension() {
        let env = TestEnv::new();
        env.create_test_file("test.txt", "content").unwrap();

        let result = delete_file_internal(&env.get_test_file("test.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        let test_file_2 = env.get_test_file("test2.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = rename_file_internal(&test_file, &test_file_2);
        assert!(result.is_ok());
        assert!(!Path::new(&test_file).exists());
        assert!(Path::new(&test_file_2).exists());

        let content = fs::read_to_string(&test_file_2).unwrap();
        assert_eq!(content, TEST_CONTENT);
    }

    #[test]
    fn test_rename_file_source_not_found() {
        let env = TestEnv::new();

        let result = rename_file_internal(
            &format!("{}/nonexistent.md", env.get_path()),
            &env.get_test_file("test2.md"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_file_destination_exists() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        let test_file_2 = env.get_test_file("test2.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();
        env.create_test_file("test2.md", "other content").unwrap();

        let result = rename_file_internal(&test_file, &test_file_2);
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_file_invalid_extension() {
        let env = TestEnv::new();
        env.create_test_file("test.txt", "content").unwrap();

        let result = rename_file_internal(&env.get_test_file("test.txt"), &env.get_test_file("test2.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_directory_success() {
        let env = TestEnv::new();
        let old_dir = env.path.join("old_folder");
        let new_dir = env.path.join("new_folder");
        
        // Create directory with a file inside
        fs::create_dir(&old_dir).unwrap();
        let file_inside = old_dir.join("file.md");
        fs::write(&file_inside, TEST_CONTENT).unwrap();
        
        let old_dir_str = old_dir.to_string_lossy().to_string();
        let new_dir_str = new_dir.to_string_lossy().to_string();
        
        let result = rename_file_internal(&old_dir_str, &new_dir_str);
        assert!(result.is_ok());
        assert!(!old_dir.exists());
        assert!(new_dir.exists());
        
        // Check that file inside was moved too
        let moved_file = new_dir.join("file.md");
        assert!(moved_file.exists());
        let content = fs::read_to_string(&moved_file).unwrap();
        assert_eq!(content, TEST_CONTENT);
    }

    #[test]
    fn test_utf8_encoding() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let write_result = write_file_internal(&test_file, UTF8_CONTENT);
        assert!(write_result.is_ok());

        let read_result = read_file_internal(&test_file);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), UTF8_CONTENT);
    }

    #[test]
    fn test_nested_directory_creation() {
        let env = TestEnv::new();
        let nested_file = env.get_test_file("nested/deep/file.md");

        let result = create_file_internal(&nested_file);
        assert!(result.is_ok());
        assert!(Path::new(&nested_file).exists());

        let content = fs::read_to_string(&nested_file).unwrap();
        assert_eq!(content, "# file\n\n");
    }

    #[test]
    fn test_preview_file_small() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = preview_file_internal(&test_file, 1000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_CONTENT);
    }

    #[test]
    fn test_preview_file_large() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        let large_content = "A".repeat(2000);
        env.create_test_file("test.md", &large_content).unwrap();

        let result = preview_file_internal(&test_file, 100);
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert!(preview.len() < large_content.len());
        assert!(preview.contains("File preview truncated"));
    }

    #[test]
    fn test_auto_save_file() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let result = auto_save_file_internal(&test_file, TEST_CONTENT);
        assert!(result.is_ok());
        assert!(Path::new(&test_file).exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, TEST_CONTENT);
    }

    #[test]
    fn test_get_file_info_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = get_file_info_internal(&test_file);
        assert!(result.is_ok());
        
        let file_info = result.unwrap();
        assert_eq!(file_info.name, "test.md");
        assert_eq!(file_info.path, test_file);
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);
        assert!(file_info.modified > 0);
        assert!(file_info.is_markdown());
    }

    #[test]
    fn test_get_file_info_directory() {
        let env = TestEnv::new();
        env.create_directory_structure(&["test_dir"]).unwrap();
        let dir_path = env.get_test_file("test_dir");

        let result = get_file_info_internal(&dir_path);
        assert!(result.is_ok());
        
        let file_info = result.unwrap();
        assert_eq!(file_info.name, "test_dir");
        assert!(file_info.is_dir);
        assert!(!file_info.is_markdown());
    }

    #[test]
    fn test_get_file_info_not_found() {
        let env = TestEnv::new();
        let nonexistent_file = env.get_test_file("nonexistent.md");

        let result = get_file_info_internal(&nonexistent_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_folder_success() {
        let env = TestEnv::new();
        let folder_path = env.get_test_file("new_folder");

        let result = create_folder_internal(&folder_path);
        assert!(result.is_ok());
        assert!(Path::new(&folder_path).exists());
        assert!(Path::new(&folder_path).is_dir());
    }

    #[test]
    fn test_create_folder_nested() {
        let env = TestEnv::new();
        let nested_folder = env.get_test_file("level1/level2/level3");

        let result = create_folder_internal(&nested_folder);
        assert!(result.is_ok());
        assert!(Path::new(&nested_folder).exists());
        assert!(Path::new(&nested_folder).is_dir());
    }

    #[test]
    fn test_create_folder_already_exists() {
        let env = TestEnv::new();
        let folder_path = env.get_test_file("existing_folder");
        
        // Create the folder first
        fs::create_dir(&folder_path).unwrap();

        let result = create_folder_internal(&folder_path);
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("FileAlreadyExists"));
    }
}