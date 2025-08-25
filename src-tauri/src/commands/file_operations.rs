//! # File Operations Commands
//!
//! This module contains all Tauri commands related to file system operations.
//! It provides a clean interface for file CRUD operations, file preview,
//! auto-save functionality, and system integration features.
//!
//! ## Command Overview
//!
//! ### Core File Operations
//! - `read_file`: Read file contents as string
//! - `write_file`: Write content to file
//! - `create_file`: Create a new empty file
//! - `delete_file`: Delete an existing file
//! - `rename_file`: Rename/move a file
//!
//! ### Enhanced Operations
//! - `auto_save_file`: Auto-save with file locking
//! - `preview_file`: Read file with length limit for previews
//! - `reveal_in_finder`: Open file location in system file manager
//! - `get_file_info`: Get file metadata information
//! - `create_folder`: Create a new directory
//!
//! ## Error Handling
//!
//! All commands return `Result<T, String>` where errors are converted to
//! user-friendly string messages. The underlying implementation uses the
//! robust error handling from the `file_operations` module.
//!
//! ## Security Considerations
//!
//! - Path validation is performed in the underlying implementation
//! - File locking prevents concurrent access issues
//! - Directory traversal protection is built into path handling
//!
//! ## Performance Notes
//!
//! - Large file operations are optimized in the underlying implementation
//! - Auto-save uses file locking to prevent corruption
//! - Preview operations limit read length to avoid memory issues

use crate::file_operations;
use crate::types::FileInfo;

/// Read the complete contents of a file as a UTF-8 string
///
/// # Arguments
/// * `file_path` - Absolute path to the file to read
///
/// # Returns
/// * `Ok(String)` - File contents as UTF-8 string
/// * `Err(String)` - Error message if file cannot be read
///
/// # Example Usage (from frontend)
/// ```javascript
/// const content = await invoke('read_file', { filePath: '/path/to/file.md' });
/// ```
#[tauri::command]
pub fn read_file(file_path: String) -> Result<String, String> {
    file_operations::read_file_internal(&file_path).map_err(|e| e.into())
}

/// Write content to a file, creating it if it doesn't exist
///
/// # Arguments
/// * `file_path` - Absolute path to the file to write
/// * `content` - String content to write to the file
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if file cannot be written
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('write_file', { 
///     filePath: '/path/to/file.md', 
///     content: '# My Note\nContent here' 
/// });
/// ```
#[tauri::command]
pub fn write_file(file_path: String, content: String) -> Result<(), String> {
    file_operations::write_file_internal(&file_path, &content).map_err(|e| e.into())
}

/// Create a new empty file at the specified path
///
/// # Arguments
/// * `file_path` - Absolute path where the file should be created
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if file cannot be created
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('create_file', { filePath: '/path/to/new-note.md' });
/// ```
#[tauri::command]
pub fn create_file(file_path: String) -> Result<(), String> {
    file_operations::create_file_internal(&file_path).map_err(|e| e.into())
}

/// Delete an existing file from the filesystem
///
/// # Arguments
/// * `file_path` - Absolute path to the file to delete
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if file cannot be deleted
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('delete_file', { filePath: '/path/to/file.md' });
/// ```
#[tauri::command]
pub fn delete_file(file_path: String) -> Result<(), String> {
    file_operations::delete_file_internal(&file_path).map_err(|e| e.into())
}

/// Rename or move a file from one path to another
///
/// # Arguments
/// * `old_path` - Current absolute path of the file
/// * `new_path` - New absolute path for the file
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if file cannot be renamed
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('rename_file', { 
///     oldPath: '/path/to/old-name.md', 
///     newPath: '/path/to/new-name.md' 
/// });
/// ```
#[tauri::command]
pub fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    file_operations::rename_file_internal(&old_path, &new_path).map_err(|e| e.into())
}

/// Auto-save file content with file locking to prevent corruption
///
/// This command uses file locking mechanisms to ensure safe concurrent
/// access during auto-save operations.
///
/// # Arguments
/// * `file_path` - Absolute path to the file to auto-save
/// * `content` - String content to save to the file
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if auto-save fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('auto_save_file', { 
///     filePath: '/path/to/file.md', 
///     content: '# Updated content' 
/// });
/// ```
#[tauri::command]
pub fn auto_save_file(file_path: String, content: String) -> Result<(), String> {
    file_operations::auto_save_file_internal(&file_path, &content).map_err(|e| e.into())
}

/// Preview file content with optional length limitation
///
/// Useful for generating previews without loading large files entirely.
/// Default limit is 1000 characters if not specified.
///
/// # Arguments
/// * `file_path` - Absolute path to the file to preview
/// * `max_length` - Optional maximum number of characters to read
///
/// # Returns
/// * `Ok(String)` - File content (truncated if necessary)
/// * `Err(String)` - Error message if file cannot be previewed
///
/// # Example Usage (from frontend)
/// ```javascript
/// const preview = await invoke('preview_file', { 
///     filePath: '/path/to/file.md', 
///     maxLength: 500 
/// });
/// ```
#[tauri::command]
pub fn preview_file(file_path: String, max_length: Option<usize>) -> Result<String, String> {
    file_operations::preview_file_internal(&file_path, max_length.unwrap_or(1000)).map_err(|e| e.into())
}

/// Open the file's containing folder in the system file manager
///
/// This command will reveal the specified file in Finder (macOS), 
/// Explorer (Windows), or the default file manager (Linux).
///
/// # Arguments
/// * `file_path` - Absolute path to the file to reveal
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if file cannot be revealed
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('reveal_in_finder', { filePath: '/path/to/file.md' });
/// ```
#[tauri::command]
pub fn reveal_in_finder(file_path: String) -> Result<(), String> {
    file_operations::reveal_in_finder_internal(&file_path).map_err(|e| e.into())
}

/// Get detailed metadata information about a file
///
/// Returns structured information including file size, modification time,
/// file type, and other metadata.
///
/// # Arguments
/// * `file_path` - Absolute path to the file to inspect
///
/// # Returns
/// * `Ok(FileInfo)` - Structured file information
/// * `Err(String)` - Error message if file info cannot be retrieved
///
/// # Example Usage (from frontend)
/// ```javascript
/// const fileInfo = await invoke('get_file_info', { filePath: '/path/to/file.md' });
/// console.log(fileInfo.size, fileInfo.modified);
/// ```
#[tauri::command]
pub fn get_file_info(file_path: String) -> Result<FileInfo, String> {
    file_operations::get_file_info_internal(&file_path).map_err(|e| e.into())
}

/// Create a new directory at the specified path
///
/// Creates parent directories if they don't exist (similar to `mkdir -p`).
///
/// # Arguments
/// * `folder_path` - Absolute path where the directory should be created
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(String)` - Error message if directory cannot be created
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('create_folder', { folderPath: '/path/to/new-folder' });
/// ```
#[tauri::command]
pub fn create_folder(folder_path: String) -> Result<(), String> {
    file_operations::create_folder_internal(&folder_path).map_err(|e| e.into())
}