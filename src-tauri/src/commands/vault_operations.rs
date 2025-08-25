//! # Vault Operations Commands
//!
//! This module contains all Tauri commands related to vault management and operations.
//! It provides functionality for vault selection, validation, scanning, watching, and
//! comprehensive vault lifecycle management.
//!
//! ## Command Overview
//!
//! ### Vault Selection & Validation
//! - `select_vault_folder`: Interactive folder selection dialog
//! - `select_vault`: Alias for folder selection (API compatibility)  
//! - `validate_vault`: Check if folder is a valid vault
//!
//! ### Vault Content Management
//! - `load_vault`: Load vault with validation and file scanning
//! - `scan_vault_files`: Comprehensive scan of all vault files
//! - `scan_vault_files_chunked`: Paginated scanning for large vaults
//! - `watch_vault`: Set up filesystem watching for changes
//!
//! ## Cross-Platform Support
//!
//! - **macOS**: Uses native file dialogs via `rfd` crate
//! - **Windows**: Uses Windows file dialogs
//! - **Linux**: Uses system-appropriate file dialogs
//!
//! ## Performance Optimizations
//!
//! - **Chunked Loading**: Large vaults are processed in chunks to prevent UI blocking
//! - **Async Operations**: File dialogs and I/O operations are non-blocking
//! - **Efficient Scanning**: Only processes markdown and relevant files
//! - **Caching**: File information is cached to improve subsequent operations
//!
//! ## Vault Structure
//!
//! A valid vault is expected to:
//! - Be a readable directory
//! - Contain `.md` files (directly or in subdirectories)
//! - Have appropriate permissions for read/write operations
//! - Not be a system or protected directory
//!
//! ## Error Handling
//!
//! All commands return `Result<T, String>` with user-friendly error messages.
//! Common error scenarios include:
//! - Directory access permissions
//! - Invalid vault structure  
//! - File system I/O errors
//! - User cancellation of dialogs

use crate::vault_operations;
use crate::types::FileInfo;

/// Open an interactive folder selection dialog to choose a vault directory
///
/// This command presents a native file dialog allowing users to select
/// a directory to use as their note vault. It's implemented asynchronously
/// to prevent blocking the UI thread.
///
/// # Returns
/// * `Ok(Some(String))` - Selected directory path
/// * `Ok(None)` - User cancelled the dialog
/// * `Err(String)` - Error occurred during dialog operation
///
/// # Cross-Platform Behavior
/// - **macOS**: Uses native Finder dialog
/// - **Windows**: Uses Windows Explorer dialog  
/// - **Linux**: Uses system file manager dialog
///
/// # Example Usage (from frontend)
/// ```javascript
/// const vaultPath = await invoke('select_vault_folder');
/// if (vaultPath) {
///     console.log('Selected vault:', vaultPath);
/// } else {
///     console.log('User cancelled selection');
/// }
/// ```
#[tauri::command]
pub async fn select_vault_folder() -> Result<Option<String>, String> {
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

/// Select a vault directory (alias for select_vault_folder)
///
/// This command provides API compatibility by offering an alternative
/// name for the vault folder selection functionality.
///
/// # Returns
/// * `Ok(Some(String))` - Selected directory path
/// * `Ok(None)` - User cancelled the dialog
/// * `Err(String)` - Error occurred during dialog operation
///
/// # Example Usage (from frontend)
/// ```javascript
/// const vaultPath = await invoke('select_vault');
/// ```
#[tauri::command]
pub async fn select_vault() -> Result<Option<String>, String> {
    // Alias for select_vault_folder to match the API requirements
    select_vault_folder().await
}

/// Validate whether a directory is a suitable vault
///
/// Performs comprehensive validation to ensure the directory can serve
/// as a note vault, checking permissions, structure, and accessibility.
///
/// # Arguments
/// * `vault_path` - Absolute path to the directory to validate
///
/// # Returns
/// * `Ok(true)` - Directory is a valid vault
/// * `Ok(false)` - Directory is not suitable as a vault
/// * `Err(String)` - Error occurred during validation
///
/// # Validation Criteria
/// - Directory exists and is readable
/// - Has appropriate write permissions
/// - Is not a system or protected directory
/// - Contains or can contain markdown files
///
/// # Example Usage (from frontend)
/// ```javascript
/// const isValid = await invoke('validate_vault', { vaultPath: '/path/to/vault' });
/// if (isValid) {
///     console.log('Valid vault directory');
/// }
/// ```
#[tauri::command]
pub fn validate_vault(vault_path: String) -> Result<bool, String> {
    vault_operations::validate_vault_internal(&vault_path).map_err(|e| e.into())
}

/// Load a vault with comprehensive validation and file scanning
///
/// This command performs a complete vault loading operation including
/// validation, initial file scanning, and preparation for use. It's the
/// primary command for initializing vault access.
///
/// # Arguments
/// * `vault_path` - Absolute path to the vault directory
///
/// # Returns
/// * `Ok(Vec<FileInfo>)` - List of all files found in the vault
/// * `Err(String)` - Error message if vault cannot be loaded
///
/// # Operation Steps
/// 1. Validate vault directory structure
/// 2. Check permissions and accessibility
/// 3. Scan for all relevant files
/// 4. Generate file metadata
/// 5. Return structured file information
///
/// # Example Usage (from frontend)
/// ```javascript
/// const files = await invoke('load_vault', { vaultPath: '/path/to/vault' });
/// console.log(`Loaded ${files.length} files from vault`);
/// ```
#[tauri::command]
pub fn load_vault(vault_path: String) -> Result<Vec<FileInfo>, String> {
    // Enhanced vault loading with validation
    vault_operations::load_vault_internal(&vault_path).map_err(|e| e.into())
}

/// Scan all files in a vault directory
///
/// Performs a comprehensive scan of the vault directory and all subdirectories,
/// identifying all relevant files and generating metadata. This is typically
/// used for vault refresh operations or initial loading.
///
/// # Arguments
/// * `vault_path` - Absolute path to the vault directory
///
/// # Returns
/// * `Ok(Vec<FileInfo>)` - List of all files found with metadata
/// * `Err(String)` - Error message if scanning fails
///
/// # Performance Notes
/// - For large vaults (>1000 files), consider using `scan_vault_files_chunked`
/// - Results include full file metadata (size, modification time, etc.)
/// - Only includes supported file types (primarily `.md` files)
///
/// # Example Usage (from frontend)
/// ```javascript
/// const allFiles = await invoke('scan_vault_files', { vaultPath: '/path/to/vault' });
/// ```
#[tauri::command]
pub fn scan_vault_files(vault_path: String) -> Result<Vec<FileInfo>, String> {
    vault_operations::scan_vault_files_internal(&vault_path).map_err(|e| e.into())
}

/// Scan vault files with pagination for large vaults
///
/// This command provides chunked/paginated scanning for large vaults to prevent
/// UI blocking and memory issues. It returns a page of results and indicates
/// whether more pages are available.
///
/// # Arguments
/// * `vault_path` - Absolute path to the vault directory
/// * `page` - Page number to retrieve (0-based)
/// * `page_size` - Number of files per page
///
/// # Returns
/// * `Ok((Vec<FileInfo>, bool))` - Tuple of (files, has_more_pages)
/// * `Err(String)` - Error message if scanning fails
///
/// # Pagination Details
/// - Pages are 0-based (first page is page 0)
/// - Returns `has_more_pages` boolean to indicate if pagination should continue
/// - Consistent ordering across pagination calls
/// - Efficient for large vaults with thousands of files
///
/// # Example Usage (from frontend)
/// ```javascript
/// let page = 0;
/// let allFiles = [];
/// let hasMore = true;
/// 
/// while (hasMore) {
///     const [pageFiles, hasMorePages] = await invoke('scan_vault_files_chunked', {
///         vaultPath: '/path/to/vault',
///         page: page,
///         pageSize: 100
///     });
///     allFiles.push(...pageFiles);
///     hasMore = hasMorePages;
///     page++;
/// }
/// ```
#[tauri::command]
pub fn scan_vault_files_chunked(
    vault_path: String, 
    page: usize, 
    page_size: usize
) -> Result<(Vec<FileInfo>, bool), String> {
    vault_operations::scan_vault_files_chunked_internal(&vault_path, page, page_size).map_err(|e| e.into())
}

/// Set up filesystem watching for vault changes
///
/// Establishes filesystem watching on the vault directory to detect changes
/// such as file creation, modification, deletion, and renaming. This enables
/// real-time updates in the application when vault contents change externally.
///
/// # Arguments
/// * `vault_path` - Absolute path to the vault directory to watch
///
/// # Returns
/// * `Ok(())` - Watching successfully established
/// * `Err(String)` - Error message if watching cannot be set up
///
/// # Watching Behavior
/// - Monitors the entire vault directory recursively
/// - Detects file creation, modification, deletion, and moves
/// - Provides real-time notifications for vault synchronization
/// - Automatically handles subdirectory changes
/// - Optimized to avoid excessive notifications
///
/// # Performance Considerations
/// - Uses platform-native filesystem watching mechanisms
/// - Minimal CPU overhead when vault is idle
/// - Efficient handling of bulk changes
/// - Automatic cleanup when vault is closed
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('watch_vault', { vaultPath: '/path/to/vault' });
/// console.log('Vault watching enabled');
/// ```
#[tauri::command]
pub fn watch_vault(vault_path: String) -> Result<(), String> {
    vault_operations::watch_vault_internal(&vault_path).map_err(|e| e.into())
}