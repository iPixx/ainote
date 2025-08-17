use std::fs;
use std::path::Path;
use crate::errors::{FileSystemError, FileSystemResult};

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

/// Maximum file size in bytes (10MB for markdown files)
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// Validate file size for content operations
pub fn validate_file_size(content: &str, file_path: &str) -> FileSystemResult<()> {
    let size = content.len() as u64;
    if size > MAX_FILE_SIZE {
        return Err(FileSystemError::FileTooLarge {
            path: file_path.to_string(),
            size,
            max_size: MAX_FILE_SIZE,
        });
    }
    Ok(())
}

/// Validate existing file size
pub fn validate_existing_file_size(path: &Path) -> FileSystemResult<()> {
    if let Ok(metadata) = path.metadata() {
        let size = metadata.len();
        if size > MAX_FILE_SIZE {
            return Err(FileSystemError::FileTooLarge {
                path: path.to_string_lossy().to_string(),
                size,
                max_size: MAX_FILE_SIZE,
            });
        }
    }
    Ok(())
}

/// Create a backup of an existing file before modifying it
pub fn create_backup(file_path: &Path) -> FileSystemResult<Option<String>> {
    if !file_path.exists() {
        return Ok(None);
    }
    
    // Create backup filename with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    let backup_path = file_path.with_extension(format!("md.backup.{}", timestamp));
    
    // Copy the original file to backup location
    fs::copy(file_path, &backup_path)
        .map_err(|e| FileSystemError::IOError {
            message: format!("Failed to create backup: {}", e)
        })?;
        
    Ok(Some(backup_path.to_string_lossy().to_string()))
}

/// Clean up old backup files (keep only the 5 most recent)
pub fn cleanup_old_backups(file_path: &Path) -> FileSystemResult<()> {
    let parent = match file_path.parent() {
        Some(p) => p,
        None => return Ok(()),
    };
    
    let file_stem = match file_path.file_stem() {
        Some(s) => s.to_string_lossy(),
        None => return Ok(()),
    };
    
    // Find all backup files for this file
    let entries = fs::read_dir(parent).map_err(|e| FileSystemError::IOError {
        message: format!("Failed to read directory for cleanup: {}", e)
    })?;
    
    let mut backups = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&format!("{}.md.backup.", file_stem)) {
            if let Ok(metadata) = entry.metadata() {
                backups.push((entry.path(), metadata.modified().unwrap_or(std::time::UNIX_EPOCH)));
            }
        }
    }
    
    // Sort by modification time (newest first)
    backups.sort_by(|a, b| b.1.cmp(&a.1));
    
    // Remove old backups (keep only 5 most recent)
    for (path, _) in backups.iter().skip(5) {
        let _ = fs::remove_file(path); // Ignore errors during cleanup
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct TestEnv {
        #[allow(dead_code)]
        temp_dir: TempDir,
        pub path: PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temporary directory");
            let path = temp_dir.path().to_path_buf();
            
            TestEnv {
                temp_dir,
                path,
            }
        }

        fn get_test_file(&self, name: &str) -> String {
            self.path.join(name).to_string_lossy().to_string()
        }

        fn create_test_file(&self, name: &str, content: &str) -> std::io::Result<()> {
            let file_path = self.path.join(name);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file_path, content)
        }

        fn get_path(&self) -> String {
            self.path.to_string_lossy().to_string()
        }
    }

    #[test]
    fn test_validate_markdown_extension() {
        let md_path = Path::new("test.md");
        let txt_path = Path::new("test.txt");
        let no_ext_path = Path::new("test");
        
        assert!(validate_markdown_extension(md_path).is_ok());
        assert!(validate_markdown_extension(txt_path).is_err());
        assert!(validate_markdown_extension(no_ext_path).is_err());
    }

    #[test]
    fn test_path_validation() {
        let env = TestEnv::new();
        let test_file_path = env.get_test_file("test.md");
        let test_dir_path = env.get_path();
        let non_existing_file = format!("{}/nonexistent.md", test_dir_path);
        
        env.create_test_file("test.md", "test").unwrap();
        let existing_path = Path::new(&test_file_path);
        let non_existing_path = Path::new(&non_existing_file);
        let test_dir = Path::new(&test_dir_path);
        
        assert!(validate_path_exists(existing_path).is_ok());
        assert!(validate_path_exists(non_existing_path).is_err());

        // Test file validation
        assert!(validate_is_file(existing_path).is_ok());
        assert!(validate_is_file(test_dir).is_err());

        // Test directory validation
        assert!(validate_is_directory(test_dir).is_ok());
        assert!(validate_is_directory(existing_path).is_err());

        // Test file not exists validation
        assert!(validate_file_not_exists(non_existing_path).is_ok());
        assert!(validate_file_not_exists(existing_path).is_err());
    }

    #[test]
    fn test_ensure_parent_directory() {
        let env = TestEnv::new();
        let nested_file = env.path.join("nested/deep/file.md");
        
        assert!(ensure_parent_directory(&nested_file).is_ok());
        assert!(nested_file.parent().unwrap().exists());
    }

    #[test]
    fn test_validate_file_size() {
        let small_content = "small content";
        let large_content = "x".repeat((MAX_FILE_SIZE as usize) + 1);
        
        assert!(validate_file_size(small_content, "test.md").is_ok());
        assert!(validate_file_size(&large_content, "test.md").is_err());
    }

    #[test]
    fn test_create_backup() {
        let env = TestEnv::new();
        let test_file = env.path.join("test.md");
        
        // Test with non-existent file
        let result = create_backup(&test_file).unwrap();
        assert!(result.is_none());
        
        // Test with existing file
        fs::write(&test_file, "content").unwrap();
        let result = create_backup(&test_file).unwrap();
        assert!(result.is_some());
        
        let backup_path = result.unwrap();
        assert!(Path::new(&backup_path).exists());
        assert!(backup_path.contains(".backup."));
    }

    #[test]
    fn test_cleanup_old_backups() {
        let env = TestEnv::new();
        let test_file = env.path.join("test.md");
        
        // Create main file
        fs::write(&test_file, "content").unwrap();
        
        // Create multiple backup files
        for i in 0..8 {
            let backup_file = env.path.join(format!("test.md.backup.{}", i));
            fs::write(&backup_file, "backup content").unwrap();
        }
        
        // Cleanup should succeed
        assert!(cleanup_old_backups(&test_file).is_ok());
        
        // Count remaining backup files
        let entries = fs::read_dir(&env.path).unwrap();
        let backup_count = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".backup."))
            .count();
        
        // Should keep only 5 most recent
        assert!(backup_count <= 5);
    }

    #[test]
    fn test_validate_existing_file_size() {
        let env = TestEnv::new();
        let test_file = env.path.join("test.md");
        
        // Create small file
        fs::write(&test_file, "small content").unwrap();
        assert!(validate_existing_file_size(&test_file).is_ok());
    }
}