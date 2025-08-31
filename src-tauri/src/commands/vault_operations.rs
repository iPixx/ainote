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
use crate::commands::indexing_commands::{index_vault_notes, start_indexing_pipeline};
use crate::file_monitor::get_file_monitor;

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

/// Load a vault with automatic indexing and file monitoring (Phase 2C Integration)
///
/// This enhanced command performs comprehensive vault loading with automatic
/// AI indexing and real-time file monitoring activation. It integrates seamlessly
/// with the indexing pipeline to ensure the AI system has up-to-date embeddings
/// for all notes immediately after vault selection.
///
/// # Arguments
/// * `vault_path` - Absolute path to the vault directory
/// * `auto_index` - Whether to automatically start indexing (default: true)
/// * `auto_monitor` - Whether to start file monitoring (default: true)
///
/// # Returns
/// * `Ok(LoadVaultWithIndexingResult)` - Comprehensive result with files and indexing status
/// * `Err(String)` - Error message if vault loading or setup fails
///
/// # Operation Steps (Phase 2C)
/// 1. Validate vault directory structure
/// 2. Load and scan all vault files
/// 3. Initialize indexing pipeline
/// 4. Start automatic vault indexing in background
/// 5. Activate real-time file monitoring
/// 6. Return vault files with indexing status
///
/// # Performance Features
/// - Non-blocking indexing (runs in background)
/// - Immediate vault access while indexing proceeds
/// - Real-time progress tracking available via get_indexing_progress
/// - File monitoring for automatic re-indexing on changes
///
/// # Example Usage (from frontend)
/// ```javascript
/// const result = await invoke('load_vault_with_indexing', { 
///     vaultPath: '/path/to/vault',
///     autoIndex: true,
///     autoMonitor: true 
/// });
/// console.log(`Loaded ${result.files.length} files, indexing ${result.indexing_request_ids.length} files`);
/// console.log(`File monitoring: ${result.monitoring_active ? 'active' : 'inactive'}`);
/// 
/// // Monitor indexing progress
/// setInterval(async () => {
///     const progress = await invoke('get_indexing_progress');
///     console.log(`Indexing progress: ${progress.progress_percent}%`);
/// }, 1000);
/// ```
#[tauri::command]
pub async fn load_vault_with_indexing(
    vault_path: String,
    auto_index: Option<bool>,
    auto_monitor: Option<bool>
) -> Result<LoadVaultWithIndexingResult, String> {
    let vault_path_str = vault_path.clone();
    
    log::info!("🚀 Loading vault with automated indexing: {}", vault_path_str);
    
    // Step 1: Load vault files using existing logic
    let files = vault_operations::load_vault_internal(&vault_path).map_err(|e| {
        log::error!("❌ Failed to load vault: {}", e);
        format!("Vault loading failed: {}", e)
    })?;
    
    log::info!("✅ Loaded {} files from vault", files.len());
    
    let should_index = auto_index.unwrap_or(true);
    let should_monitor = auto_monitor.unwrap_or(true);
    
    let mut indexing_request_ids = Vec::new();
    let mut indexing_error = None;
    
    // Step 2: Initialize and start indexing pipeline if requested
    if should_index {
        log::info!("🔧 Initializing indexing pipeline for vault...");
        
        match start_indexing_pipeline(None).await {
            Ok(_) => {
                log::info!("✅ Indexing pipeline initialized successfully");
                
                // Step 3: Start automatic vault indexing
                match index_vault_notes(
                    vault_path_str.clone(),
                    Some("**/*.md".to_string()),
                    Some("UserTriggered".to_string())
                ).await {
                    Ok(request_ids) => {
                        indexing_request_ids = request_ids;
                        log::info!("✅ Started automatic vault indexing with {} requests", indexing_request_ids.len());
                    }
                    Err(e) => {
                        indexing_error = Some(format!("Failed to start vault indexing: {}", e));
                        log::warn!("⚠️ Vault indexing failed: {}", e);
                    }
                }
            }
            Err(e) => {
                indexing_error = Some(format!("Failed to initialize indexing pipeline: {}", e));
                log::warn!("⚠️ Indexing pipeline initialization failed: {}", e);
            }
        }
    } else {
        log::info!("ℹ️ Automatic indexing disabled for vault loading");
    }
    
    // Step 4: Start file monitoring if requested
    let mut monitoring_active = false;
    let mut monitoring_error = None;
    
    if should_monitor {
        log::info!("👁️ Starting file monitoring for vault...");
        
        let file_monitor = get_file_monitor();
        match file_monitor.start_watching(&vault_path_str).await {
            Ok(_) => {
                monitoring_active = true;
                log::info!("✅ File monitoring activated for vault");
            }
            Err(e) => {
                monitoring_error = Some(format!("Failed to start file monitoring: {}", e));
                log::warn!("⚠️ File monitoring failed: {}", e);
            }
        }
    } else {
        log::info!("ℹ️ File monitoring disabled for vault loading");
    }
    
    // Step 5: Return comprehensive result
    let indexing_active = !indexing_request_ids.is_empty();
    let result = LoadVaultWithIndexingResult {
        files,
        indexing_request_ids,
        indexing_active,
        indexing_error,
        monitoring_active,
        monitoring_error,
        vault_path: vault_path_str,
    };
    
    log::info!("🎉 Vault loading with indexing completed successfully");
    Ok(result)
}

/// Result structure for vault loading with indexing integration
#[derive(serde::Serialize)]
pub struct LoadVaultWithIndexingResult {
    /// List of all files found in the vault
    pub files: Vec<FileInfo>,
    /// Request IDs for indexing operations (for tracking progress)
    pub indexing_request_ids: Vec<u64>,
    /// Whether indexing is currently active
    pub indexing_active: bool,
    /// Error message if indexing failed (indexing failure doesn't prevent vault loading)
    pub indexing_error: Option<String>,
    /// Whether file monitoring is active
    pub monitoring_active: bool,
    /// Error message if file monitoring failed
    pub monitoring_error: Option<String>,
    /// Path to the loaded vault
    pub vault_path: String,
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