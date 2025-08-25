//! # State Management Commands
//!
//! This module contains all Tauri commands related to application state persistence
//! and management. It handles window state, layout preferences, session data, and
//! vault preferences with robust validation and recovery mechanisms.
//!
//! ## Command Overview
//!
//! ### Core State Operations
//! - `load_app_state`: Load complete application state from disk
//! - `save_app_state`: Save complete application state to disk
//!
//! ### Window State Management
//! - `save_window_state`: Persist window dimensions, position, and maximized state
//!
//! ### Layout & UI State
//! - `save_layout_state`: Save panel visibility, sizes, and UI layout preferences
//! - `save_session_state`: Persist current session data and active file
//!
//! ### Vault Preferences
//! - `save_vault_preferences`: Save recently used vault paths
//! - `get_vault_preferences`: Retrieve vault history and preferences
//!
//! ## State Persistence
//!
//! ### Storage Location
//! Application state is stored in `~/.ainote/app_state.json` with the following structure:
//!
//! ```json
//! {
//!   "window": {
//!     "width": 1200.0,
//!     "height": 800.0,
//!     "x": 100,
//!     "y": 100,
//!     "maximized": false
//!   },
//!   "layout": {
//!     "file_tree_width": 280.0,
//!     "ai_panel_width": 350.0,
//!     "file_tree_visible": true,
//!     "ai_panel_visible": false
//!   },
//!   "session": {
//!     "current_vault": "/path/to/vault",
//!     "current_file": "/path/to/current/file.md",
//!     "recent_files": [...]
//!   }
//! }
//! ```
//!
//! ## Validation & Recovery
//!
//! ### Window State Validation
//! - Width: constrained between 800px and 2000px
//! - Height: constrained between 600px and 1400px
//! - Position: validated to ensure window remains on screen
//! - Maximized state: boolean validation
//!
//! ### Layout State Validation
//! - Panel widths: reasonable minimum and maximum constraints
//! - Visibility states: boolean validation
//! - UI layout: consistent state validation
//!
//! ### Error Recovery
//! - Invalid state files are backed up and reset to defaults
//! - Partial state corruption is handled gracefully
//! - Missing state files trigger creation of default state
//! - Atomic writes prevent state corruption during save operations
//!
//! ## Performance Considerations
//!
//! - State operations are lightweight and fast
//! - Atomic file operations prevent corruption
//! - Minimal memory footprint for state data
//! - Efficient JSON serialization/deserialization

use crate::state_management;
use crate::types::AppState;

/// Load the complete application state from persistent storage
///
/// This command loads all persisted application state including window
/// dimensions, layout preferences, session data, and vault preferences.
/// If no state file exists, it returns sensible defaults.
///
/// # Returns
/// * `Ok(AppState)` - Complete application state structure
/// * `Err(String)` - Error message if state cannot be loaded
///
/// # State Recovery
/// - Missing state file: Returns default state
/// - Corrupted state file: Backs up corrupt file and returns defaults
/// - Partial corruption: Loads valid portions and defaults for invalid sections
///
/// # Example Usage (from frontend)
/// ```javascript
/// const appState = await invoke('load_app_state');
/// console.log('Window size:', appState.window.width, 'x', appState.window.height);
/// console.log('Current vault:', appState.session.current_vault);
/// ```
#[tauri::command]
pub fn load_app_state() -> Result<AppState, String> {
    state_management::load_app_state_internal().map_err(|e| e.into())
}

/// Save the complete application state to persistent storage
///
/// This command performs an atomic save of all application state to ensure
/// data integrity. The save operation includes validation of all state values
/// before persistence.
///
/// # Arguments
/// * `state` - Complete AppState structure to persist
///
/// # Returns
/// * `Ok(())` - State successfully saved
/// * `Err(String)` - Error message if state cannot be saved
///
/// # Validation
/// - Window dimensions are validated and constrained
/// - Layout values are checked for reasonableness
/// - File paths are validated for existence and accessibility
/// - State structure is validated for completeness
///
/// # Example Usage (from frontend)
/// ```javascript
/// const appState = {
///     window: { width: 1440, height: 900, maximized: false },
///     layout: { file_tree_width: 300, ai_panel_visible: true },
///     session: { current_vault: '/path/to/vault' }
/// };
/// await invoke('save_app_state', { state: appState });
/// ```
#[tauri::command]
pub fn save_app_state(state: AppState) -> Result<(), String> {
    state_management::save_app_state_internal(&state).map_err(|e| e.into())
}

/// Save window state including dimensions, position, and maximized status
///
/// This command persists the current window state for restoration in future
/// sessions. It includes validation to ensure the window remains usable.
///
/// # Arguments
/// * `width` - Window width in logical pixels
/// * `height` - Window height in logical pixels  
/// * `x` - Window x position (optional, None if not positioned)
/// * `y` - Window y position (optional, None if not positioned)
/// * `maximized` - Whether window is maximized
///
/// # Returns
/// * `Ok(())` - Window state successfully saved
/// * `Err(String)` - Error message if state cannot be saved
///
/// # Validation Constraints
/// - **Width**: 800px ≤ width ≤ 2000px
/// - **Height**: 600px ≤ height ≤ 1400px
/// - **Position**: Constrained to keep window on screen
/// - **Maximized**: Boolean validation
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('save_window_state', {
///     width: 1200,
///     height: 800,
///     x: 100,
///     y: 50,
///     maximized: false
/// });
/// ```
#[tauri::command]
pub fn save_window_state(
    width: f64, 
    height: f64, 
    x: Option<i32>, 
    y: Option<i32>, 
    maximized: bool
) -> Result<(), String> {
    state_management::save_window_state_internal(width, height, x, y, maximized).map_err(|e| e.into())
}

/// Save layout state including panel dimensions and visibility
///
/// This command persists UI layout preferences including panel sizes,
/// visibility states, and other layout-related settings.
///
/// # Arguments
/// * `file_tree_width` - Width of the file tree panel
/// * `ai_panel_width` - Width of the AI assistance panel
/// * `file_tree_visible` - Whether file tree panel is visible
/// * `ai_panel_visible` - Whether AI panel is visible
/// * `editor_font_size` - Editor font size preference
/// * `theme` - UI theme preference ("light" or "dark")
///
/// # Returns
/// * `Ok(())` - Layout state successfully saved
/// * `Err(String)` - Error message if state cannot be saved
///
/// # Validation Constraints
/// - Panel widths: 200px ≤ width ≤ 600px
/// - Font size: 10px ≤ size ≤ 24px
/// - Theme: Must be "light" or "dark"
/// - Visibility: Boolean validation
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('save_layout_state', {
///     fileTreeWidth: 280,
///     aiPanelWidth: 350,
///     fileTreeVisible: true,
///     aiPanelVisible: false,
///     editorFontSize: 14,
///     theme: 'light'
/// });
/// ```
#[tauri::command]
pub fn save_layout_state(
    file_tree_width: f64,
    ai_panel_width: f64,
    file_tree_visible: bool,
    ai_panel_visible: bool,
    editor_mode: String
) -> Result<(), String> {
    state_management::save_layout_state_internal(
        file_tree_width,
        ai_panel_width, 
        file_tree_visible,
        ai_panel_visible,
        editor_mode
    ).map_err(|e| e.into())
}

/// Save session state including current vault, file, and recent items
///
/// This command persists the current session information to enable
/// restoration of the user's work context across application restarts.
///
/// # Arguments
/// * `current_vault` - Path to currently opened vault (optional)
/// * `current_file` - Path to currently active file (optional)
/// * `recent_files` - List of recently accessed file paths
/// * `recent_searches` - List of recent search queries
/// * `editor_state` - Current editor state (cursor position, selections, etc.)
///
/// # Returns
/// * `Ok(())` - Session state successfully saved
/// * `Err(String)` - Error message if state cannot be saved
///
/// # Session Data
/// - **Current Vault**: Active vault directory path
/// - **Current File**: Currently open/edited file path
/// - **Recent Files**: Up to 20 recently accessed files
/// - **Recent Searches**: Up to 10 recent search queries
/// - **Editor State**: Cursor position, selections, scroll position
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('save_session_state', {
///     currentVault: '/path/to/vault',
///     currentFile: '/path/to/current.md',
///     recentFiles: ['/path/to/file1.md', '/path/to/file2.md'],
///     recentSearches: ['search term 1', 'search term 2'],
///     editorState: { cursorLine: 10, cursorColumn: 5 }
/// });
/// ```
#[tauri::command]
pub fn save_session_state(
    current_vault: Option<String>,
    current_file: Option<String>,
    view_mode: String
) -> Result<(), String> {
    state_management::save_session_state_internal(
        current_vault,
        current_file,
        view_mode
    ).map_err(|e| e.into())
}

/// Save vault preferences including recently used vaults
///
/// This command persists the list of recently accessed vault directories
/// to enable quick vault switching and vault history features.
///
/// # Arguments
/// * `recent_vaults` - List of recently used vault directory paths
///
/// # Returns
/// * `Ok(())` - Vault preferences successfully saved
/// * `Err(String)` - Error message if preferences cannot be saved
///
/// # Vault History
/// - Maintains up to 10 recent vault paths
/// - Most recently used vaults appear first
/// - Automatically removes invalid/deleted vault paths
/// - Deduplicates vault paths in the list
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('save_vault_preferences', {
///     recentVaults: ['/path/to/vault1', '/path/to/vault2', '/path/to/vault3']
/// });
/// ```
#[tauri::command]
pub fn save_vault_preferences(recent_vaults: Vec<String>) -> Result<(), String> {
    state_management::save_vault_preferences_internal(recent_vaults).map_err(|e| e.into())
}

/// Get vault preferences including recently used vault paths
///
/// This command retrieves the saved list of recently accessed vault directories
/// for display in vault selection UI and quick access features.
///
/// # Returns
/// * `Ok(Vec<String>)` - List of recent vault directory paths
/// * `Err(String)` - Error message if preferences cannot be loaded
///
/// # Return Data
/// - Returns up to 10 recently used vault paths
/// - Paths are ordered by most recent first
/// - Only includes paths that still exist on the filesystem
/// - Empty list if no vaults have been used previously
///
/// # Example Usage (from frontend)
/// ```javascript
/// const recentVaults = await invoke('get_vault_preferences');
/// console.log('Recent vaults:', recentVaults);
/// // Display in vault selection dropdown
/// ```
#[tauri::command]
pub fn get_vault_preferences() -> Result<Vec<String>, String> {
    state_management::get_vault_preferences_internal().map_err(|e| e.into())
}