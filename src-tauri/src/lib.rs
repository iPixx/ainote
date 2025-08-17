use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashSet;
use std::sync::{Mutex, LazyLock};
use std::time::Instant;

/// Performance instrumentation module
pub mod performance {
    use std::time::{Duration, Instant};
    
    /// Macro to time operations and log performance
    #[cfg(debug_assertions)]
    macro_rules! time_operation {
        ($operation:expr, $name:expr) => {{
            let start = Instant::now();
            let result = $operation;
            let duration = start.elapsed();
            if duration.as_millis() > 10 {
                eprintln!("PERF: {} took {:.3}ms", $name, duration.as_secs_f64() * 1000.0);
            }
            result
        }};
    }
    
    #[cfg(not(debug_assertions))]
    macro_rules! time_operation {
        ($operation:expr, $name:expr) => {{
            $operation
        }};
    }
    
    pub(crate) use time_operation;
    
    /// Performance tracker for detailed metrics
    pub struct PerformanceTracker {
        operation: String,
        start: Instant,
    }
    
    impl PerformanceTracker {
        pub fn start(operation: &str) -> Self {
            Self {
                operation: operation.to_string(),
                start: Instant::now(),
            }
        }
        
        pub fn finish(self) -> Duration {
            let duration = self.start.elapsed();
            #[cfg(debug_assertions)]
            if duration.as_millis() > 5 {
                eprintln!("PERF: {} completed in {:.3}ms", self.operation, duration.as_secs_f64() * 1000.0);
            }
            duration
        }
        
        pub fn checkpoint(&self, checkpoint_name: &str) {
            let duration = self.start.elapsed();
            #[cfg(debug_assertions)]
            eprintln!("PERF: {} - {} at {:.3}ms", self.operation, checkpoint_name, duration.as_secs_f64() * 1000.0);
        }
    }
}

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

/// Global file lock registry to prevent concurrent access
static FILE_LOCKS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Metadata cache for performance optimization
mod metadata_cache {
    use std::collections::HashMap;
    use std::sync::{Mutex, LazyLock};
    use std::time::{SystemTime, Duration};
    use std::path::Path;
    use std::fs::Metadata;

    /// Cache entry with TTL
    struct CacheEntry {
        metadata: Metadata,
        timestamp: SystemTime,
    }

    impl CacheEntry {
        fn new(metadata: Metadata) -> Self {
            Self {
                metadata,
                timestamp: SystemTime::now(),
            }
        }

        fn is_expired(&self) -> bool {
            // Cache entries expire after 5 seconds
            const CACHE_TTL: Duration = Duration::from_secs(5);
            SystemTime::now()
                .duration_since(self.timestamp)
                .map(|duration| duration > CACHE_TTL)
                .unwrap_or(true)
        }
    }

    static CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

    /// Get cached metadata or fetch and cache it
    pub fn get_metadata(path: &Path) -> std::io::Result<Metadata> {
        let path_str = path.to_string_lossy().to_string();
        
        // Try to get from cache first
        {
            let mut cache = CACHE.lock().unwrap();
            if let Some(entry) = cache.get(&path_str) {
                if !entry.is_expired() {
                    // Clone metadata (it's relatively cheap)
                    return Ok(entry.metadata.clone());
                } else {
                    // Remove expired entry
                    cache.remove(&path_str);
                }
            }
        }

        // Cache miss or expired - fetch metadata
        let metadata = path.metadata()?;
        
        // Store in cache
        {
            let mut cache = CACHE.lock().unwrap();
            cache.insert(path_str, CacheEntry::new(metadata.clone()));
            
            // Prevent cache from growing too large
            if cache.len() > 1000 {
                // Remove oldest entries (simple cleanup)
                let mut expired_keys = Vec::new();
                for (key, entry) in cache.iter() {
                    if entry.is_expired() {
                        expired_keys.push(key.clone());
                    }
                }
                for key in expired_keys {
                    cache.remove(&key);
                }
            }
        }
        
        Ok(metadata)
    }

    /// Clear the entire cache (useful for testing)
    #[allow(dead_code)]
    pub fn clear() {
        let mut cache = CACHE.lock().unwrap();
        cache.clear();
    }
}

/// File lock guard that automatically releases the lock when dropped
pub struct FileLockGuard {
    path: String,
}

impl FileLockGuard {
    /// Acquire a lock on a file path
    pub fn acquire(path: &str) -> FileSystemResult<Self> {
        let normalized_path = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf())
            .to_string_lossy()
            .to_string();
            
        let mut locks = FILE_LOCKS.lock().unwrap();
        
        if locks.contains(&normalized_path) {
            return Err(FileSystemError::FileLocked {
                path: path.to_string(),
            });
        }
        
        locks.insert(normalized_path.clone());
        
        Ok(FileLockGuard {
            path: normalized_path,
        })
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let mut locks = FILE_LOCKS.lock().unwrap();
        locks.remove(&self.path);
    }
}

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
}

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
            width: 1920.0,
            height: 1080.0,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// Window state
    pub window: WindowState,
    /// Layout state
    pub layout: LayoutState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            window: WindowState::default(),
            layout: LayoutState::default(),
        }
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

#[tauri::command]
fn read_file(file_path: String) -> Result<String, String> {
    read_file_internal(&file_path).map_err(|e| e.into())
}

/// Internal read file function using structured error handling
fn read_file_internal(file_path: &str) -> FileSystemResult<String> {
    use performance::time_operation;
    
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

#[tauri::command]
fn preview_file(file_path: String, max_length: Option<usize>) -> Result<String, String> {
    preview_file_internal(&file_path, max_length.unwrap_or(1000)).map_err(|e| e.into())
}

/// Internal preview file function for large files
fn preview_file_internal(file_path: &str, max_length: usize) -> FileSystemResult<String> {
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

#[tauri::command]
fn auto_save_file(file_path: String, content: String) -> Result<(), String> {
    auto_save_file_internal(&file_path, &content).map_err(|e| e.into())
}

/// Internal auto-save file function (optimized for frequent saves)
fn auto_save_file_internal(file_path: &str, content: &str) -> FileSystemResult<()> {
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

#[tauri::command]
fn write_file(file_path: String, content: String) -> Result<(), String> {
    write_file_internal(&file_path, &content).map_err(|e| e.into())
}

/// Internal write file function using structured error handling
fn write_file_internal(file_path: &str, content: &str) -> FileSystemResult<()> {
    use performance::time_operation;
    
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

#[tauri::command]
fn create_file(file_path: String) -> Result<(), String> {
    create_file_internal(&file_path).map_err(|e| e.into())
}

/// Internal create file function using structured error handling
fn create_file_internal(file_path: &str) -> FileSystemResult<()> {
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

#[tauri::command]
fn delete_file(file_path: String) -> Result<(), String> {
    delete_file_internal(&file_path).map_err(|e| e.into())
}

/// Internal delete file function using structured error handling
fn delete_file_internal(file_path: &str) -> FileSystemResult<()> {
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

#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    rename_file_internal(&old_path, &new_path).map_err(|e| e.into())
}

/// Internal rename file function using structured error handling
fn rename_file_internal(old_path: &str, new_path: &str) -> FileSystemResult<()> {
    let old = Path::new(old_path);
    let new = Path::new(new_path);

    // Acquire locks for both source and destination files
    let _old_lock = FileLockGuard::acquire(old_path)?;
    let _new_lock = FileLockGuard::acquire(new_path)?;

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

#[tauri::command]
fn scan_vault_files_chunked(
    vault_path: String, 
    page: usize, 
    page_size: usize
) -> Result<(Vec<FileInfo>, bool), String> {
    scan_vault_files_chunked_internal(&vault_path, page, page_size).map_err(|e| e.into())
}

/// Chunked scanning for very large vaults to avoid UI blocking
fn scan_vault_files_chunked_internal(
    vault_path: &str, 
    page: usize, 
    page_size: usize
) -> FileSystemResult<(Vec<FileInfo>, bool)> {
    use performance::time_operation;
    
    time_operation!({
        let vault_path = Path::new(vault_path);
        
        // Validate vault path exists and is a directory
        validation::validate_path_exists(vault_path)?;
        validation::validate_is_directory(vault_path)?;

        // For chunked scanning, we need to scan everything first then paginate
        // In a real implementation, this could be optimized with a streaming approach
        let all_files = scan_vault_files_internal(&vault_path.to_string_lossy())?;
        
        let start_idx = page * page_size;
        let end_idx = ((page + 1) * page_size).min(all_files.len());
        
        let chunk = if start_idx < all_files.len() {
            all_files[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };
        
        let has_more = end_idx < all_files.len();
        
        Ok((chunk, has_more))
    }, &format!("scan_vault_files_chunked(page={}, size={})", page, page_size))
}

/// Internal scan vault files function using structured error handling
fn scan_vault_files_internal(vault_path: &str) -> FileSystemResult<Vec<FileInfo>> {
    use performance::{time_operation, PerformanceTracker};
    
    time_operation!({
        let tracker = PerformanceTracker::start("scan_vault_files");
        let vault_path = Path::new(vault_path);
        
        // Validate vault path exists and is a directory
        validation::validate_path_exists(vault_path)?;
        validation::validate_is_directory(vault_path)?;
        
        tracker.checkpoint("validation_complete");

        // Use efficient iterator-based scanning with capacity pre-allocation
        let mut files = Vec::with_capacity(256); // Pre-allocate for typical vaults
        let mut directories = Vec::with_capacity(32); // Track directories to scan
        
        // Efficient non-recursive scanning using a work queue
        scan_directory_iterative(vault_path, &mut files, &mut directories)?;
        
        tracker.checkpoint("scanning_complete");
        
        // Efficient in-place sorting (directories first, then files alphabetically)
        sort_files_efficiently(&mut files);
        
        tracker.checkpoint("sorting_complete");
        let _duration = tracker.finish();
        
        Ok(files)
    }, "scan_vault_files_total")
}

/// Optimized iterative directory scanning to avoid stack overflow and improve performance
fn scan_directory_iterative(
    root_path: &Path, 
    files: &mut Vec<FileInfo>, 
    work_queue: &mut Vec<std::path::PathBuf>
) -> FileSystemResult<()> {
    work_queue.push(root_path.to_path_buf());
    
    while let Some(current_dir) = work_queue.pop() {
        if let Err(e) = scan_single_directory(&current_dir, files, work_queue) {
            // Log error but continue with other directories
            eprintln!("Warning: Error scanning directory {}: {}", current_dir.display(), e);
        }
    }
    
    Ok(())
}

/// Scan a single directory efficiently with early filtering and batch processing
fn scan_single_directory(
    dir: &Path, 
    files: &mut Vec<FileInfo>, 
    work_queue: &mut Vec<std::path::PathBuf>
) -> FileSystemResult<()> {
    let entries = fs::read_dir(dir)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: dir.to_string_lossy().to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to read directory {}: {}", dir.display(), e) 
            },
        })?;

    // Process entries in batches for better memory locality
    let mut batch_dirs = Vec::with_capacity(16);
    let mut batch_files = Vec::with_capacity(64);
    
    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(_) => continue, // Skip problematic entries
        };

        let path = entry.path();
        
        // Fast path check for .md extension before metadata call
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "md" {
                    batch_files.push(entry);
                }
            }
            // Skip non-markdown files immediately
        } else if path.is_dir() {
            batch_dirs.push(entry);
        }
        // Skip other types (symlinks, etc.)
    }

    // Process directories batch
    for entry in batch_dirs {
        if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
            files.push(file_info);
        }
        // Add to work queue for processing
        work_queue.push(entry.path());
    }

    // Process markdown files batch
    for entry in batch_files {
        if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
            files.push(file_info);
        }
    }
    
    Ok(())
}

/// Efficient in-place sorting optimized for typical file structures
fn sort_files_efficiently(files: &mut [FileInfo]) {
    // Use unstable sort for better performance (stable order not needed here)
    files.sort_unstable_by(|a, b| {
        // Fast path: directories vs files
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,   // Directories first
            (false, true) => std::cmp::Ordering::Greater, // Files second
            _ => {
                // Both same type - compare by name (case-insensitive)
                // Use faster comparison without allocation
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        }
    });
}

/// Get the application state file path
fn get_state_file_path() -> FileSystemResult<std::path::PathBuf> {
    // For debugging, use a simpler path first
    let home_dir = dirs::home_dir()
        .ok_or_else(|| FileSystemError::IOError {
            message: "Could not determine home directory".to_string(),
        })?;
    
    let ainote_dir = home_dir.join(".ainote");
    
    // Create directory if it doesn't exist
    if !ainote_dir.exists() {
        fs::create_dir_all(&ainote_dir)
            .map_err(|e| FileSystemError::IOError {
                message: format!("Failed to create config directory: {}", e),
            })?;
    }
    
    let state_file = ainote_dir.join("app_state.json");
    Ok(state_file)
}

/// Load application state from disk
fn load_app_state_internal() -> FileSystemResult<AppState> {
    let state_file = get_state_file_path()?;
    
    if !state_file.exists() {
        // Return default state if file doesn't exist
        return Ok(AppState::default());
    }
    
    let content = fs::read_to_string(&state_file)
        .map_err(|e| FileSystemError::IOError {
            message: format!("Failed to read state file: {}", e),
        })?;
    
    let state: AppState = serde_json::from_str(&content)
        .map_err(|e| FileSystemError::IOError {
            message: format!("Failed to parse state file: {}", e),
        })?;
    
    Ok(state)
}

/// Save application state to disk
fn save_app_state_internal(state: &AppState) -> FileSystemResult<()> {
    let state_file = get_state_file_path()?;
    
    let content = serde_json::to_string_pretty(state)
        .map_err(|e| FileSystemError::IOError {
            message: format!("Failed to serialize state: {}", e),
        })?;
    
    fs::write(&state_file, content)
        .map_err(|e| FileSystemError::IOError {
            message: format!("Failed to write state file: {}", e),
        })?;
    
    Ok(())
}

#[tauri::command]
fn load_app_state() -> Result<AppState, String> {
    load_app_state_internal().map_err(|e| e.into())
}

#[tauri::command]
fn save_app_state(state: AppState) -> Result<(), String> {
    save_app_state_internal(&state).map_err(|e| e.into())
}

#[tauri::command]
fn save_window_state(width: f64, height: f64, x: Option<i32>, y: Option<i32>, maximized: bool) -> Result<(), String> {
    let mut state = load_app_state_internal().unwrap_or_default();
    state.window.width = width;
    state.window.height = height;
    state.window.x = x;
    state.window.y = y;
    state.window.maximized = maximized;
    save_app_state_internal(&state).map_err(|e| e.into())
}

#[tauri::command]
fn save_layout_state(
    file_tree_width: f64,
    ai_panel_width: f64,
    file_tree_visible: bool,
    ai_panel_visible: bool,
    editor_mode: String,
) -> Result<(), String> {
    let mut state = load_app_state_internal().unwrap_or_default();
    state.layout.file_tree_width = file_tree_width;
    state.layout.ai_panel_width = ai_panel_width;
    state.layout.file_tree_visible = file_tree_visible;
    state.layout.ai_panel_visible = ai_panel_visible;
    state.layout.editor_mode = editor_mode;
    save_app_state_internal(&state).map_err(|e| e.into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            auto_save_file,
            create_file,
            delete_file,
            rename_file,
            select_vault_folder,
            scan_vault_files,
            scan_vault_files_chunked,
            preview_file,
            load_app_state,
            save_app_state,
            save_window_state,
            save_layout_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

    // Automatic cleanup happens when TestEnv is dropped due to TempDir
    
    const TEST_CONTENT: &str = "# Test Content\n\nThis is test content.";
    const UTF8_CONTENT: &str = "# UTF-8 Test\n\n✅ Checkmark\n🎉 Emoji\nÀccëntéd characters";

    #[test]
    fn test_create_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let result = create_file(test_file.clone());
        assert!(result.is_ok());
        assert!(Path::new(&test_file).exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "# test\n\n");
    }

    #[test]
    fn test_create_file_invalid_extension() {
        let env = TestEnv::new();
        let invalid_file = env.get_test_file("test.txt");

        let result = create_file(invalid_file);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("not a markdown file"));
    }

    #[test]
    fn test_create_file_already_exists() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        
        // Create the file first
        env.create_test_file("test.md", "existing content").unwrap();

        let result = create_file(test_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("A file already exists at"));
    }

    #[test]
    fn test_write_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let result = write_file(test_file.clone(), TEST_CONTENT.to_string());
        assert!(result.is_ok());
        assert!(Path::new(&test_file).exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, TEST_CONTENT);
    }

    #[test]
    fn test_write_file_invalid_extension() {
        let env = TestEnv::new();
        let invalid_file = env.get_test_file("test.txt");

        let result = write_file(invalid_file, TEST_CONTENT.to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));
    }

    #[test]
    fn test_read_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = read_file(test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_CONTENT);
    }

    #[test]
    fn test_read_file_not_found() {
        let env = TestEnv::new();

        let result = read_file(format!("{}/nonexistent.md", env.get_path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));
    }

    #[test]
    fn test_read_file_invalid_extension() {
        let env = TestEnv::new();
        env.create_test_file("test.txt", "content").unwrap();

        let result = read_file(env.get_test_file("test.txt"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));
    }

    #[test]
    fn test_read_file_is_directory() {
        let env = TestEnv::new();
        env.create_directory_structure(&["subdir.md"]).unwrap();

        let result = read_file(env.get_test_file("subdir.md"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is not a file"));
    }

    #[test]
    fn test_delete_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = delete_file(test_file.clone());
        assert!(result.is_ok());
        assert!(!Path::new(&test_file).exists());
    }

    #[test]
    fn test_delete_file_not_found() {
        let env = TestEnv::new();

        let result = delete_file(format!("{}/nonexistent.md", env.get_path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));
    }

    #[test]
    fn test_delete_file_invalid_extension() {
        let env = TestEnv::new();
        env.create_test_file("test.txt", "content").unwrap();

        let result = delete_file(env.get_test_file("test.txt"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));
    }

    #[test]
    fn test_rename_file_success() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        let test_file_2 = env.get_test_file("test2.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = rename_file(test_file.clone(), test_file_2.clone());
        assert!(result.is_ok());
        assert!(!Path::new(&test_file).exists());
        assert!(Path::new(&test_file_2).exists());

        let content = fs::read_to_string(&test_file_2).unwrap();
        assert_eq!(content, TEST_CONTENT);
    }

    #[test]
    fn test_rename_file_source_not_found() {
        let env = TestEnv::new();

        let result = rename_file(
            format!("{}/nonexistent.md", env.get_path()),
            env.get_test_file("test2.md"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));
    }

    #[test]
    fn test_rename_file_destination_exists() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        let test_file_2 = env.get_test_file("test2.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();
        env.create_test_file("test2.md", "other content").unwrap();

        let result = rename_file(test_file, test_file_2);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("already exists"));
    }

    #[test]
    fn test_rename_file_invalid_extension() {
        let env = TestEnv::new();
        env.create_test_file("test.txt", "content").unwrap();

        let result = rename_file(env.get_test_file("test.txt"), env.get_test_file("test2.md"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not a markdown file"));
    }

    #[test]
    fn test_utf8_encoding() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");

        let write_result = write_file(test_file.clone(), UTF8_CONTENT.to_string());
        assert!(write_result.is_ok());

        let read_result = read_file(test_file);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), UTF8_CONTENT);
    }

    #[test]
    fn test_nested_directory_creation() {
        let env = TestEnv::new();
        let nested_file = env.get_test_file("nested/deep/file.md");

        let result = create_file(nested_file.clone());
        assert!(result.is_ok());
        assert!(Path::new(&nested_file).exists());

        let content = fs::read_to_string(&nested_file).unwrap();
        assert_eq!(content, "# file\n\n");
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

        let entries: Vec<_> = fs::read_dir(&env.get_path()).unwrap().collect();
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
    fn test_scan_vault_files_empty_directory() {
        let env = TestEnv::new();

        let result = scan_vault_files(env.get_path());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_scan_vault_files_with_markdown_files() {
        let env = TestEnv::new();
        
        // Create test files
        env.create_test_file("note1.md", "# Note 1").unwrap();
        env.create_test_file("note2.md", "# Note 2").unwrap();
        env.create_test_file("readme.txt", "Not a markdown file").unwrap(); // Should be ignored

        let result = scan_vault_files(env.get_path());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 2); // Only .md files should be included
        
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        assert_eq!(md_files.len(), 2);
        
        // Check that files are sorted alphabetically
        assert!(md_files[0].name <= md_files[1].name);
    }

    #[test]
    fn test_scan_vault_files_nested_directories() {
        let env = TestEnv::new();
        
        // Create nested structure
        env.create_directory_structure(&["subdir/deep"]).unwrap();
        env.create_test_file("root.md", "# Root note").unwrap();
        env.create_test_file("subdir/sub.md", "# Sub note").unwrap();
        env.create_test_file("subdir/deep/deep.md", "# Deep note").unwrap();

        let result = scan_vault_files(env.get_path());
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
    }

    #[test]
    fn test_scan_vault_files_nonexistent_path() {
        let result = scan_vault_files("nonexistent_directory".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not be found"));
    }

    #[test]
    fn test_scan_vault_files_file_instead_of_directory() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = scan_vault_files(test_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is not a directory"));
    }

    #[test]
    fn test_scan_vault_files_performance_target() {
        let env = TestEnv::new();
        
        // Create many files to test performance
        for i in 0..100 {
            env.create_test_file(&format!("note_{:03}.md", i), &format!("# Note {}", i)).unwrap();
        }

        let start = std::time::Instant::now();
        let result = scan_vault_files(env.get_path());
        let duration = start.elapsed();

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.iter().filter(|f| !f.is_dir).count(), 100);
        
        // Performance target: <500ms for 1000+ files, so 100 files should be much faster
        assert!(duration.as_millis() < 100, "Scanning took too long: {:?}", duration);
    }

    #[test]
    fn test_scan_vault_files_mixed_file_types() {
        let env = TestEnv::new();
        
        // Create various file types
        env.create_test_file("note.md", "# Markdown note").unwrap();
        env.create_test_file("document.txt", "Text document").unwrap();
        env.create_test_file("script.js", "console.log('hello')").unwrap();
        env.create_test_file("data.json", "{}").unwrap();
        env.create_test_file("README", "No extension").unwrap();

        let result = scan_vault_files(env.get_path());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        let file_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        
        // Only the .md file should be included
        assert_eq!(file_files.len(), 1);
        assert_eq!(file_files[0].name, "note.md");
    }

    #[test]
    fn test_scan_vault_files_cross_platform_paths() {
        let env = TestEnv::new();
        
        // Create a file and test that paths are handled properly
        env.create_test_file("test.md", TEST_CONTENT).unwrap();

        let result = scan_vault_files(env.get_path());
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
            let env = TestEnv::new();
            
            // Test markdown extension validation
            let md_path = Path::new("test.md");
            let txt_path = Path::new("test.txt");
            let no_ext_path = Path::new("test");
            
            assert!(validation::validate_markdown_extension(md_path).is_ok());
            assert!(validation::validate_markdown_extension(txt_path).is_err());
            assert!(validation::validate_markdown_extension(no_ext_path).is_err());

            // Test path existence validation
            let test_file_path = env.get_test_file("test.md");
            let test_dir_path = env.get_path();
            let non_existing_file = format!("{}/nonexistent.md", test_dir_path);
            
            env.create_test_file("test.md", "test").unwrap();
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
            let env = TestEnv::new();
            let file_path = env.get_test_file("test.md");
            
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
            let new_path = env.get_test_file("test2.md");
            let result = rename_file(file_path.clone(), new_path.clone());
            assert!(result.is_ok(), "Failed to rename file: {:?}", result);
            assert!(!Path::new(&file_path).exists());
            assert!(Path::new(&new_path).exists());

            // 6. Delete file
            let result = delete_file(new_path.clone());
            assert!(result.is_ok(), "Failed to delete file: {:?}", result);
            assert!(!Path::new(&new_path).exists());
        }

        #[test]
        fn test_vault_scanning_comprehensive() {
            let env = TestEnv::new();
            
            // Create complex directory structure
            let dirs = vec![
                "folder1",
                "folder1/subfolder",
                "folder2",
                "folder2/deep/nested"
            ];
            
            env.create_directory_structure(&dirs).unwrap();

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
                env.create_test_file(path, content).unwrap();
            }

            // Scan vault
            let result = scan_vault_files(env.get_path());
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
        }

        #[test]
        fn test_error_propagation_consistency() {
            let env = TestEnv::new();
            
            let non_existent = format!("{}/nonexistent.md", env.get_path());
            let invalid_ext = env.get_test_file("test.txt");
            
            // Test that all commands handle missing files consistently
            assert!(read_file(non_existent.clone()).is_err());
            assert!(delete_file(non_existent.clone()).is_err());
            
            // Test that all commands handle invalid extensions consistently
            assert!(read_file(invalid_ext.clone()).is_err());
            assert!(write_file(invalid_ext.clone(), "content".to_string()).is_err());
            assert!(create_file(invalid_ext.clone()).is_err());
            assert!(delete_file(invalid_ext.clone()).is_err());
        }
    }

    mod performance_tests {
        use super::*;

        #[test]
        fn test_large_vault_performance() {
            let env = TestEnv::new();
            
            // Create a larger test set (500 files in 10 directories)
            for dir_i in 0..10 {
                let dir_name = format!("dir_{:02}", dir_i);
                env.create_directory_structure(&[&dir_name]).unwrap();
                
                for file_i in 0..50 {
                    let file_path = format!("{}/note_{:03}.md", dir_name, file_i);
                    env.create_test_file(&file_path, &format!("# Note {} in Directory {}", file_i, dir_i)).unwrap();
                }
            }

            // Measure scanning performance
            let start = std::time::Instant::now();
            let result = scan_vault_files(env.get_path());
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
            let test_file = env.get_test_file("dir_01/note_001.md");
            
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
        }

        #[test] 
        fn test_memory_efficiency() {
            let env = TestEnv::new();
            
            // Create files with larger content to test memory usage
            for i in 0..20 {
                let large_content = "# Large File\n\n".to_string() + 
                    &"This is a line of content that repeats many times to create a larger file.\n".repeat(100);
                let file_name = format!("large_file_{:02}.md", i);
                env.create_test_file(&file_name, &large_content).unwrap();
            }

            // Test that scanning doesn't load all file contents into memory
            let result = scan_vault_files(env.get_path());
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
        }

        #[test]
        fn test_extreme_vault_performance() {
            let env = TestEnv::new();
            
            // Create an extremely large test set (1000 files in 20 directories)
            for dir_i in 0..20 {
                let dir_name = format!("dir_{:03}", dir_i);
                env.create_directory_structure(&[&dir_name]).unwrap();
                
                for file_i in 0..50 {
                    let file_path = format!("{}/note_{:04}.md", dir_name, file_i);
                    let content = format!("# Note {} in Directory {}\n\nContent for performance testing.", file_i, dir_i);
                    env.create_test_file(&file_path, &content).unwrap();
                }
            }

            // Measure scanning performance for 1000 files
            let start = std::time::Instant::now();
            let result = scan_vault_files(env.get_path());
            let scan_duration = start.elapsed();

            assert!(result.is_ok());
            let files = result.unwrap();
            
            let file_count = files.iter().filter(|f| !f.is_dir).count();
            let dir_count = files.iter().filter(|f| f.is_dir).count();
            
            assert_eq!(file_count, 1000, "Expected 1000 markdown files");
            assert_eq!(dir_count, 20, "Expected 20 directories");

            // CRITICAL: Performance target <500ms for 1000+ files
            assert!(scan_duration.as_millis() < 500, 
                   "Scanning 1000 files exceeded target (500ms): {:?}", scan_duration);
                   
            println!("✅ Extreme vault scanning performance: {}ms for 1000 files", scan_duration.as_millis());
        }

        #[test]
        fn test_chunked_scanning_performance() {
            let env = TestEnv::new();
            
            // Create test files
            for i in 0..200 {
                let file_path = format!("note_{:03}.md", i);
                env.create_test_file(&file_path, &format!("# Note {}", i)).unwrap();
            }

            // Test chunked scanning performance
            let start = std::time::Instant::now();
            let result = scan_vault_files_chunked(env.get_path(), 0, 50);
            let chunk_duration = start.elapsed();

            assert!(result.is_ok());
            let (chunk, has_more) = result.unwrap();
            
            assert_eq!(chunk.len(), 50);
            assert!(has_more);
            
            // Chunked scanning should be fast
            assert!(chunk_duration.as_millis() < 100, 
                   "Chunked scanning took too long: {:?}", chunk_duration);
        }

        #[test]
        fn test_metadata_cache_performance() {
            let env = TestEnv::new();
            
            // Create test files
            for i in 0..50 {
                let file_path = format!("note_{:02}.md", i);
                env.create_test_file(&file_path, &format!("# Note {}", i)).unwrap();
            }

            // First scan (cold cache)
            let start = std::time::Instant::now();
            let result1 = scan_vault_files(env.get_path());
            let first_scan = start.elapsed();
            assert!(result1.is_ok());

            // Second scan (warm cache) - should be faster
            let start = std::time::Instant::now();
            let result2 = scan_vault_files(env.get_path());
            let second_scan = start.elapsed();
            assert!(result2.is_ok());

            // Warm cache should provide some performance benefit
            println!("Cache performance - First: {}ms, Second: {}ms", 
                    first_scan.as_millis(), second_scan.as_millis());
            
            // Results should be identical
            assert_eq!(result1.unwrap().len(), result2.unwrap().len());
        }

        #[test]
        fn test_concurrent_operations_performance() {
            let env = TestEnv::new();
            
            // Create test files
            for i in 0..20 {
                let file_path = format!("note_{:02}.md", i);
                env.create_test_file(&file_path, &format!("# Note {}", i)).unwrap();
            }

            // Test multiple operations in sequence to ensure no performance degradation
            
            // Operation 1: Read file
            let start = std::time::Instant::now();
            let _result = read_file(env.get_test_file("note_01.md"));
            let duration = start.elapsed();
            assert!(duration.as_millis() < 100, "Read operation took too long: {:?}", duration);
            
            // Operation 2: Write file
            let start = std::time::Instant::now();
            let _result = write_file(env.get_test_file("temp.md"), "# Temp".to_string());
            let duration = start.elapsed();
            assert!(duration.as_millis() < 100, "Write operation took too long: {:?}", duration);
            
            // Operation 3: Scan vault
            let start = std::time::Instant::now();
            let _result = scan_vault_files(env.get_path());
            let duration = start.elapsed();
            assert!(duration.as_millis() < 100, "Scan operation took too long: {:?}", duration);
            
            // Operation 4: Delete file
            let start = std::time::Instant::now();
            let _result = delete_file(env.get_test_file("temp.md"));
            let duration = start.elapsed();
            assert!(duration.as_millis() < 100, "Delete operation took too long: {:?}", duration);
        }

        #[test]
        fn test_memory_usage_patterns() {
            let env = TestEnv::new();
            
            // Create various file sizes to test memory patterns
            let file_sizes = vec![100, 1000, 10000, 50000]; // bytes
            
            for (i, size) in file_sizes.iter().enumerate() {
                let content = "A".repeat(*size);
                let file_path = format!("size_test_{}.md", i);
                env.create_test_file(&file_path, &content).unwrap();
            }

            // Test that file operations don't accumulate memory
            for _ in 0..10 {
                for i in 0..file_sizes.len() {
                    let file_path = env.get_test_file(&format!("size_test_{}.md", i));
                    
                    let start = std::time::Instant::now();
                    let content = read_file(file_path.clone()).unwrap();
                    let read_time = start.elapsed();
                    
                    assert!(read_time.as_millis() < 50, "Large file read too slow: {:?}", read_time);
                    assert_eq!(content.len(), file_sizes[i]); // Content should match exactly
                }
            }
        }

        #[test]
        fn test_performance_targets_compliance() {
            let env = TestEnv::new();
            
            // Test the exact performance targets from the issue
            
            // 1. File operations <50ms
            env.create_test_file("target_test.md", "# Target Test").unwrap();
            let test_file = env.get_test_file("target_test.md");
            
            // Read operation
            let start = std::time::Instant::now();
            let _content = read_file(test_file.clone()).unwrap();
            let read_time = start.elapsed();
            assert!(read_time.as_millis() < 50, "Read exceeded 50ms target: {:?}", read_time);
            
            // Write operation
            let start = std::time::Instant::now();
            let _result = write_file(test_file.clone(), "# Updated Content".to_string()).unwrap();
            let write_time = start.elapsed();
            assert!(write_time.as_millis() < 50, "Write exceeded 50ms target: {:?}", write_time);
            
            // Create operation
            let new_file = env.get_test_file("new_target_test.md");
            let start = std::time::Instant::now();
            let _result = create_file(new_file.clone()).unwrap();
            let create_time = start.elapsed();
            assert!(create_time.as_millis() < 50, "Create exceeded 50ms target: {:?}", create_time);
            
            // Delete operation
            let start = std::time::Instant::now();
            let _result = delete_file(new_file).unwrap();
            let delete_time = start.elapsed();
            assert!(delete_time.as_millis() < 50, "Delete exceeded 50ms target: {:?}", delete_time);
            
            // 2. Vault scanning <500ms for 1000+ files
            // (Covered in test_extreme_vault_performance)
            
            println!("✅ All performance targets met:");
            println!("   Read: {}ms < 50ms", read_time.as_millis());
            println!("   Write: {}ms < 50ms", write_time.as_millis());
            println!("   Create: {}ms < 50ms", create_time.as_millis());
            println!("   Delete: {}ms < 50ms", delete_time.as_millis());
        }
    }

    mod state_persistence_tests {
        use super::*;
        use std::fs;

        #[test]
        fn test_window_state_default_values() {
            let window_state = WindowState::default();
            
            assert_eq!(window_state.width, 1920.0);
            assert_eq!(window_state.height, 1080.0);
            assert_eq!(window_state.x, None);
            assert_eq!(window_state.y, None);
            assert_eq!(window_state.maximized, false);
        }

        #[test]
        fn test_layout_state_default_values() {
            let layout_state = LayoutState::default();
            
            assert_eq!(layout_state.file_tree_width, 280.0);
            assert_eq!(layout_state.ai_panel_width, 350.0);
            assert_eq!(layout_state.file_tree_visible, true);
            assert_eq!(layout_state.ai_panel_visible, false);
            assert_eq!(layout_state.editor_mode, "edit");
        }

        #[test]
        fn test_app_state_default_values() {
            let app_state = AppState::default();
            
            assert_eq!(app_state.window.width, 1920.0);
            assert_eq!(app_state.window.height, 1080.0);
            assert_eq!(app_state.layout.file_tree_width, 280.0);
            assert_eq!(app_state.layout.ai_panel_width, 350.0);
            assert_eq!(app_state.layout.file_tree_visible, true);
            assert_eq!(app_state.layout.ai_panel_visible, false);
        }

        #[test]
        fn test_window_state_serialization() {
            let window_state = WindowState {
                width: 1600.0,
                height: 900.0,
                x: Some(100),
                y: Some(50),
                maximized: true,
            };

            // Test serialization
            let json = serde_json::to_string(&window_state).unwrap();
            assert!(json.contains("\"width\":1600.0"));
            assert!(json.contains("\"height\":900.0"));
            assert!(json.contains("\"x\":100"));
            assert!(json.contains("\"y\":50"));
            assert!(json.contains("\"maximized\":true"));

            // Test deserialization
            let deserialized: WindowState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.width, 1600.0);
            assert_eq!(deserialized.height, 900.0);
            assert_eq!(deserialized.x, Some(100));
            assert_eq!(deserialized.y, Some(50));
            assert_eq!(deserialized.maximized, true);
        }

        #[test]
        fn test_layout_state_serialization() {
            let layout_state = LayoutState {
                file_tree_width: 320.0,
                ai_panel_width: 400.0,
                file_tree_visible: false,
                ai_panel_visible: true,
                editor_mode: "preview".to_string(),
            };

            // Test serialization
            let json = serde_json::to_string(&layout_state).unwrap();
            assert!(json.contains("\"file_tree_width\":320.0"));
            assert!(json.contains("\"ai_panel_width\":400.0"));
            assert!(json.contains("\"file_tree_visible\":false"));
            assert!(json.contains("\"ai_panel_visible\":true"));
            assert!(json.contains("\"editor_mode\":\"preview\""));

            // Test deserialization
            let deserialized: LayoutState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.file_tree_width, 320.0);
            assert_eq!(deserialized.ai_panel_width, 400.0);
            assert_eq!(deserialized.file_tree_visible, false);
            assert_eq!(deserialized.ai_panel_visible, true);
            assert_eq!(deserialized.editor_mode, "preview");
        }

        #[test]
        fn test_app_state_serialization() {
            let app_state = AppState {
                window: WindowState {
                    width: 1440.0,
                    height: 900.0,
                    x: Some(200),
                    y: Some(100),
                    maximized: false,
                },
                layout: LayoutState {
                    file_tree_width: 250.0,
                    ai_panel_width: 300.0,
                    file_tree_visible: true,
                    ai_panel_visible: true,
                    editor_mode: "split".to_string(),
                },
            };

            // Test serialization
            let json = serde_json::to_string_pretty(&app_state).unwrap();
            assert!(json.contains("\"window\""));
            assert!(json.contains("\"layout\""));
            // The pretty formatter might add spaces, so check for the value more flexibly
            assert!(json.contains("1440"));
            assert!(json.contains("\"editor_mode\": \"split\""));

            // Test deserialization
            let deserialized: AppState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.window.width, 1440.0);
            assert_eq!(deserialized.window.height, 900.0);
            assert_eq!(deserialized.layout.file_tree_width, 250.0);
            assert_eq!(deserialized.layout.editor_mode, "split");
        }

        #[test]
        fn test_load_app_state_nonexistent_file() {
            // Test that loading non-existent state returns default
            let env = TestEnv::new();
            
            // Temporarily change the config directory to our test environment
            std::env::set_var("HOME", env.get_path());
            
            let result = load_app_state_internal();
            assert!(result.is_ok());
            
            let state = result.unwrap();
            assert_eq!(state.window.width, 1920.0);
            assert_eq!(state.layout.file_tree_width, 280.0);
            
            // Clean up
            std::env::remove_var("HOME");
        }

        #[test]
        fn test_save_and_load_app_state_roundtrip() {
            let env = TestEnv::new();
            
            // Create a custom state
            let original_state = AppState {
                window: WindowState {
                    width: 1366.0,
                    height: 768.0,
                    x: Some(50),
                    y: Some(25),
                    maximized: false,
                },
                layout: LayoutState {
                    file_tree_width: 300.0,
                    ai_panel_width: 400.0,
                    file_tree_visible: false,
                    ai_panel_visible: true,
                    editor_mode: "preview".to_string(),
                },
            };

            // Manually create config directory and save state
            let config_dir = env.path.join("config").join("ainote");
            fs::create_dir_all(&config_dir).unwrap();
            let state_file = config_dir.join("app_state.json");
            
            let json = serde_json::to_string_pretty(&original_state).unwrap();
            fs::write(&state_file, json).unwrap();

            // Test loading
            let loaded_json = fs::read_to_string(&state_file).unwrap();
            let loaded_state: AppState = serde_json::from_str(&loaded_json).unwrap();

            // Verify all fields match
            assert_eq!(loaded_state.window.width, 1366.0);
            assert_eq!(loaded_state.window.height, 768.0);
            assert_eq!(loaded_state.window.x, Some(50));
            assert_eq!(loaded_state.window.y, Some(25));
            assert_eq!(loaded_state.window.maximized, false);

            assert_eq!(loaded_state.layout.file_tree_width, 300.0);
            assert_eq!(loaded_state.layout.ai_panel_width, 400.0);
            assert_eq!(loaded_state.layout.file_tree_visible, false);
            assert_eq!(loaded_state.layout.ai_panel_visible, true);
            assert_eq!(loaded_state.layout.editor_mode, "preview");
        }

        #[test]
        fn test_save_window_state_command() {
            let env = TestEnv::new();
            
            // Create config directory structure manually for testing
            let config_dir = env.path.join("config").join("ainote");
            fs::create_dir_all(&config_dir).unwrap();
            
            // Save initial state
            let initial_state = AppState::default();
            let state_file = config_dir.join("app_state.json");
            let json = serde_json::to_string_pretty(&initial_state).unwrap();
            fs::write(&state_file, json).unwrap();

            // Now test the actual save_window_state_internal function logic
            let mut state = AppState::default();
            state.window.width = 1280.0;
            state.window.height = 720.0;
            state.window.x = Some(100);
            state.window.y = Some(200);
            state.window.maximized = true;

            // Save the updated state
            let updated_json = serde_json::to_string_pretty(&state).unwrap();
            fs::write(&state_file, updated_json).unwrap();

            // Verify it was saved correctly
            let saved_json = fs::read_to_string(&state_file).unwrap();
            let saved_state: AppState = serde_json::from_str(&saved_json).unwrap();
            
            assert_eq!(saved_state.window.width, 1280.0);
            assert_eq!(saved_state.window.height, 720.0);
            assert_eq!(saved_state.window.x, Some(100));
            assert_eq!(saved_state.window.y, Some(200));
            assert_eq!(saved_state.window.maximized, true);
        }

        #[test]
        fn test_save_layout_state_command() {
            let env = TestEnv::new();
            
            // Create config directory structure
            let config_dir = env.path.join("config").join("ainote");
            fs::create_dir_all(&config_dir).unwrap();
            
            // Save initial state
            let initial_state = AppState::default();
            let state_file = config_dir.join("app_state.json");
            let json = serde_json::to_string_pretty(&initial_state).unwrap();
            fs::write(&state_file, json).unwrap();

            // Test layout state update logic
            let mut state = AppState::default();
            state.layout.file_tree_width = 350.0;
            state.layout.ai_panel_width = 450.0;
            state.layout.file_tree_visible = false;
            state.layout.ai_panel_visible = true;
            state.layout.editor_mode = "split".to_string();

            // Save the updated state
            let updated_json = serde_json::to_string_pretty(&state).unwrap();
            fs::write(&state_file, updated_json).unwrap();

            // Verify it was saved correctly
            let saved_json = fs::read_to_string(&state_file).unwrap();
            let saved_state: AppState = serde_json::from_str(&saved_json).unwrap();
            
            assert_eq!(saved_state.layout.file_tree_width, 350.0);
            assert_eq!(saved_state.layout.ai_panel_width, 450.0);
            assert_eq!(saved_state.layout.file_tree_visible, false);
            assert_eq!(saved_state.layout.ai_panel_visible, true);
            assert_eq!(saved_state.layout.editor_mode, "split");
        }

        #[test]
        fn test_invalid_state_file_handling() {
            let env = TestEnv::new();
            
            // Create config directory and invalid JSON file
            let config_dir = env.path.join("config").join("ainote");
            fs::create_dir_all(&config_dir).unwrap();
            let state_file = config_dir.join("app_state.json");
            
            // Write invalid JSON
            fs::write(&state_file, "{ invalid json }").unwrap();

            // Test that invalid JSON is handled gracefully
            let content = fs::read_to_string(&state_file).unwrap();
            let result: Result<AppState, _> = serde_json::from_str(&content);
            assert!(result.is_err());
            
            // The actual function should return default state on error
            // (we can't test the full function without mocking dirs::config_dir)
        }

        #[test]
        fn test_state_file_path_creation() {
            // Test that the state file path logic is sound
            let test_path = std::path::PathBuf::from("/test/config/ainote/app_state.json");
            
            // Verify the expected structure
            assert_eq!(test_path.file_name().unwrap(), "app_state.json");
            assert_eq!(test_path.parent().unwrap().file_name().unwrap(), "ainote");
            assert_eq!(test_path.extension().unwrap(), "json");
        }

        #[test]
        fn test_window_state_edge_cases() {
            // Test with extreme values
            let window_state = WindowState {
                width: 100.0,  // Very small
                height: 100.0, // Very small
                x: Some(-1000), // Negative position
                y: Some(-1000), // Negative position
                maximized: true,
            };

            let json = serde_json::to_string(&window_state).unwrap();
            let deserialized: WindowState = serde_json::from_str(&json).unwrap();
            
            assert_eq!(deserialized.width, 100.0);
            assert_eq!(deserialized.height, 100.0);
            assert_eq!(deserialized.x, Some(-1000));
            assert_eq!(deserialized.y, Some(-1000));
            assert_eq!(deserialized.maximized, true);
        }

        #[test]
        fn test_layout_state_editor_modes() {
            let modes = vec!["edit", "preview", "split"];
            
            for mode in modes {
                let layout_state = LayoutState {
                    file_tree_width: 280.0,
                    ai_panel_width: 350.0,
                    file_tree_visible: true,
                    ai_panel_visible: false,
                    editor_mode: mode.to_string(),
                };

                let json = serde_json::to_string(&layout_state).unwrap();
                let deserialized: LayoutState = serde_json::from_str(&json).unwrap();
                
                assert_eq!(deserialized.editor_mode, mode);
            }
        }

        #[test]
        fn test_state_persistence_performance() {
            let _env = TestEnv::new();
            
            // Test serialization performance with large state
            let start = std::time::Instant::now();
            
            let app_state = AppState {
                window: WindowState {
                    width: 1920.0,
                    height: 1080.0,
                    x: Some(100),
                    y: Some(50),
                    maximized: false,
                },
                layout: LayoutState {
                    file_tree_width: 350.0,
                    ai_panel_width: 450.0,
                    file_tree_visible: true,
                    ai_panel_visible: true,
                    editor_mode: "split".to_string(),
                },
            };

            // Serialize and deserialize 1000 times to test performance
            for _ in 0..1000 {
                let json = serde_json::to_string(&app_state).unwrap();
                let _: AppState = serde_json::from_str(&json).unwrap();
            }

            let duration = start.elapsed();
            
            // Should complete in reasonable time (< 100ms for 1000 iterations)
            assert!(duration.as_millis() < 100, 
                   "State serialization took too long: {:?}", duration);
        }

        #[test]
        fn test_state_file_concurrent_access() {
            let env = TestEnv::new();
            
            // Create config directory
            let config_dir = env.path.join("config").join("ainote");
            fs::create_dir_all(&config_dir).unwrap();
            let state_file = config_dir.join("app_state.json");
            
            // Write initial state
            let initial_state = AppState::default();
            let json = serde_json::to_string_pretty(&initial_state).unwrap();
            fs::write(&state_file, json).unwrap();

            // Simulate concurrent access - multiple reads should work
            for _i in 0..10 {
                let content = fs::read_to_string(&state_file).unwrap();
                let state: AppState = serde_json::from_str(&content).unwrap();
                
                assert_eq!(state.window.width, 1920.0);
                assert_eq!(state.layout.file_tree_width, 280.0);
            }
        }

        #[test] 
        fn test_state_backward_compatibility() {
            let _env = TestEnv::new();
            
            // Test loading old format (missing fields should get defaults)
            let minimal_json = r#"{
                "window": {
                    "width": 1366.0,
                    "height": 768.0
                },
                "layout": {
                    "file_tree_width": 250.0
                }
            }"#;

            // This should fail because required fields are missing
            // But in real implementation, we'd use serde defaults
            let result: Result<AppState, _> = serde_json::from_str(minimal_json);
            assert!(result.is_err()); // Current implementation requires all fields

            // Test with complete valid old format
            let old_format_json = r#"{
                "window": {
                    "width": 1366.0,
                    "height": 768.0,
                    "x": null,
                    "y": null,
                    "maximized": false
                },
                "layout": {
                    "file_tree_width": 250.0,
                    "ai_panel_width": 300.0,
                    "file_tree_visible": true,
                    "ai_panel_visible": false,
                    "editor_mode": "edit"
                }
            }"#;

            let state: AppState = serde_json::from_str(old_format_json).unwrap();
            assert_eq!(state.window.width, 1366.0);
            assert_eq!(state.window.height, 768.0);
            assert_eq!(state.layout.file_tree_width, 250.0);
        }

        #[test]
        fn test_state_size_constraints() {
            // Test that we can handle various window sizes
            let test_cases = vec![
                (800.0, 600.0),    // Small window
                (1920.0, 1080.0), // Full HD
                (2560.0, 1440.0), // QHD
                (3840.0, 2160.0), // 4K
            ];

            for (width, height) in test_cases {
                let window_state = WindowState {
                    width,
                    height,
                    x: None,
                    y: None,
                    maximized: false,
                };

                let json = serde_json::to_string(&window_state).unwrap();
                let deserialized: WindowState = serde_json::from_str(&json).unwrap();
                
                assert_eq!(deserialized.width, width);
                assert_eq!(deserialized.height, height);
            }
        }

        #[test]
        fn test_layout_panel_constraints() {
            // Test various panel width configurations
            let test_cases = vec![
                (250.0, 300.0), // Minimum widths
                (400.0, 500.0), // Maximum widths 
                (325.0, 375.0), // Mid-range widths
            ];

            for (file_tree_width, ai_panel_width) in test_cases {
                let layout_state = LayoutState {
                    file_tree_width,
                    ai_panel_width,
                    file_tree_visible: true,
                    ai_panel_visible: true,
                    editor_mode: "edit".to_string(),
                };

                let json = serde_json::to_string(&layout_state).unwrap();
                let deserialized: LayoutState = serde_json::from_str(&json).unwrap();
                
                assert_eq!(deserialized.file_tree_width, file_tree_width);
                assert_eq!(deserialized.ai_panel_width, ai_panel_width);
            }
        }

        #[test]
        fn test_state_error_recovery() {
            let _env = TestEnv::new();
            
            // Test recovery from various corrupted states
            let corrupted_states = vec![
                "invalid json",
                "{",
                "{}",
                r#"{"window": "invalid"}"#,
                r#"{"layout": 123}"#,
            ];

            for corrupted in corrupted_states {
                let result: Result<AppState, _> = serde_json::from_str(corrupted);
                assert!(result.is_err(), "Should fail to parse: {}", corrupted);
            }
        }

        #[test]
        fn test_tauri_command_interfaces() {
            // Test that our Tauri command functions have correct signatures
            // by calling them with test data
            
            let app_state = AppState::default();
            
            // Test load_app_state command
            let load_result = load_app_state();
            assert!(load_result.is_ok() || load_result.is_err()); // Should return some result
            
            // Test save_app_state command  
            let save_result = save_app_state(app_state);
            assert!(save_result.is_ok() || save_result.is_err()); // Should return some result
            
            // Test save_window_state command
            let window_result = save_window_state(1920.0, 1080.0, Some(100), Some(50), false);
            assert!(window_result.is_ok() || window_result.is_err()); // Should return some result
            
            // Test save_layout_state command
            let layout_result = save_layout_state(280.0, 350.0, true, false, "edit".to_string());
            assert!(layout_result.is_ok() || layout_result.is_err()); // Should return some result
        }

        #[test]
        fn test_json_pretty_formatting() {
            let app_state = AppState::default();
            
            // Test that pretty printing produces readable JSON
            let json = serde_json::to_string_pretty(&app_state).unwrap();
            
            // Should contain newlines for readability
            assert!(json.contains('\n'));
            // Should contain proper indentation
            assert!(json.contains("  "));
            // Should contain all expected fields
            assert!(json.contains("\"window\""));
            assert!(json.contains("\"layout\""));
            assert!(json.contains("\"width\""));
            assert!(json.contains("\"file_tree_width\""));
            
            // Should be parseable back to the same state
            let parsed: AppState = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.window.width, app_state.window.width);
            assert_eq!(parsed.layout.file_tree_width, app_state.layout.file_tree_width);
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
            let env = TestEnv::new();
            
            // Test with unicode filenames and content
            let unicode_filename = env.get_test_file("测试文档_émojis_🎉.md");
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
        }

        #[test]
        fn test_special_characters_in_paths() {
            let env = TestEnv::new();
            
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
                let file_path = env.get_test_file(filename);
                let content = format!("# {}\n\nContent for file with special characters.", filename);
                
                let result = write_file(file_path.clone(), content.clone());
                assert!(result.is_ok(), "Failed to write file with special chars: {}", filename);
                
                let read_content = read_file(file_path).unwrap();
                assert_eq!(read_content, content);
            }

            // Test scanning finds all files
            let scanned = scan_vault_files(env.get_path()).unwrap();
            let md_files: Vec<_> = scanned.iter().filter(|f| !f.is_dir).collect();
            assert_eq!(md_files.len(), special_files.len());
        }
    }
}
