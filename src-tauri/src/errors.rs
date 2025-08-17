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
    
    #[error("File too large: {path} ({size} bytes, max {max_size} bytes)")]
    FileTooLarge { path: String, size: u64, max_size: u64 },
    
    #[error("File is locked: {path} (another operation in progress)")]
    FileLocked { path: String },
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
            FileSystemError::FileTooLarge { path, size, max_size } => {
                format!("The file '{}' is too large ({} bytes). Maximum allowed size is {} bytes ({}MB).", 
                    path, size, max_size, max_size / 1024 / 1024)
            }
            FileSystemError::FileLocked { path } => {
                format!("The file '{}' is currently being modified by another operation. Please try again in a moment.", path)
            }
        }
    }
}

/// Convert std::io::Error to FileSystemError with context
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

/// Helper trait to add context to IO errors
pub trait IOErrorContext<T> {
    fn with_path_context(self, path: &str, operation: &str) -> FileSystemResult<T>;
}

impl<T> IOErrorContext<T> for Result<T, std::io::Error> {
    fn with_path_context(self, path: &str, operation: &str) -> FileSystemResult<T> {
        self.map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::FileNotFound { 
                path: path.to_string() 
            },
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: path.to_string() 
            },
            std::io::ErrorKind::InvalidData => FileSystemError::EncodingError {
                path: path.to_string()
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to {} '{}': {}", operation, path, e) 
            },
        })
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

#[cfg(test)]
mod tests {
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
            FileSystemError::FileTooLarge { 
                path: "/test/huge.md".to_string(), 
                size: 15000000, 
                max_size: 10485760 
            },
            FileSystemError::FileLocked { path: "/test/locked.md".to_string() },
        ];

        for error in errors {
            let user_msg = error.user_message();
            assert!(!user_msg.is_empty());
            assert!(user_msg.len() > 10); // Should be descriptive
        }
    }

    #[test]
    fn test_error_conversion_to_string() {
        let error = FileSystemError::FileNotFound { path: "/test/file.md".to_string() };
        let error_string: String = error.into();
        assert!(error_string.contains("could not be found"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let fs_error: FileSystemError = io_error.into();
        
        match fs_error {
            FileSystemError::FileNotFound { .. } => (),
            _ => panic!("Wrong error type conversion"),
        }
    }

    #[test]
    fn test_error_with_path_context() {
        let result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied, 
            "Permission denied"
        ));
        
        let fs_result = result.with_path_context("/test/file.md", "read");
        assert!(fs_result.is_err());
        
        match fs_result.unwrap_err() {
            FileSystemError::PermissionDenied { path } => {
                assert_eq!(path, "/test/file.md");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_file_too_large_formatting() {
        let error = FileSystemError::FileTooLarge {
            path: "/test/huge.md".to_string(),
            size: 15728640, // 15MB
            max_size: 10485760, // 10MB
        };
        
        let message = error.user_message();
        assert!(message.contains("15728640 bytes"));
        assert!(message.contains("10485760 bytes"));
        assert!(message.contains("10MB"));
    }
}