//! # Application Setup and Window Management
//!
//! This module handles application initialization, window setup, and event handling.
//! It manages window state persistence, validation, and event handling for the main
//! application window.
//!
//! ## Functionality
//!
//! ### Window State Management
//! - Loading and applying saved window state (size, position, maximization)
//! - Validating window dimensions and position to ensure they're reasonable
//! - Saving window state changes in response to user interactions
//!
//! ### Event Handling
//! - Window resize events with automatic state saving
//! - Window move events with automatic state saving  
//! - Window close events with final state preservation
//!
//! ## Architecture
//!
//! The setup follows these principles:
//! - **Validation**: All window dimensions and positions are validated for sanity
//! - **Persistence**: Window state is automatically saved on changes
//! - **Resilience**: Handles cases where saved state is invalid or unavailable
//! - **Performance**: Efficient event handling without blocking UI operations

use tauri::{WebviewWindow, LogicalSize, LogicalPosition, WindowEvent};
use crate::state_management::{load_app_state_internal, save_window_state_internal};

/// Sets up the application window with saved state restoration
///
/// This function handles the complete window initialization process:
/// 1. Loads saved window state from persistent storage
/// 2. Validates and constrains dimensions to reasonable bounds
/// 3. Applies the validated state to the window
/// 4. Sets up event handlers for automatic state persistence
///
/// # Arguments
/// 
/// * `window` - The main application webview window to configure
///
/// # Window State Validation
///
/// - **Width**: Constrained between 800px and 2000px
/// - **Height**: Constrained between 600px and 1400px
/// - **Position X**: Constrained between -100px and 1500px
/// - **Position Y**: Constrained between -100px and 1000px
///
/// These constraints ensure the window remains usable and visible on typical displays.
///
/// # Error Handling
///
/// If saved state cannot be loaded or is invalid, the function gracefully falls back
/// to default window behavior. Individual state operations (size, position, maximize)
/// are attempted independently so partial failures don't affect other operations.
pub fn setup_window_state(window: &WebviewWindow) {
    // Load and apply saved window state
    if let Ok(app_state) = load_app_state_internal() {
        let window_state = &app_state.window;
        
        // Validate and constrain window size to reasonable bounds
        let validated_width = window_state.width.clamp(800.0, 2000.0);
        let validated_height = window_state.height.clamp(600.0, 1400.0);
        
        // Apply saved window size with validation
        let _ = window.set_size(LogicalSize::new(
            validated_width,
            validated_height,
        ));
        
        // Apply saved window position if available
        if let (Some(x), Some(y)) = (window_state.x, window_state.y) {
            // Validate position to ensure window is on screen
            let validated_x = x.clamp(-100, 1500);
            let validated_y = y.clamp(-100, 1000);
            let _ = window.set_position(LogicalPosition::new(validated_x, validated_y));
        }
        
        // Apply maximized state
        if window_state.maximized {
            let _ = window.maximize();
        }
    }
}

/// Sets up event handlers for automatic window state persistence
///
/// This function establishes event listeners that automatically save window state
/// when the user interacts with the window. This ensures state is preserved
/// between application sessions without requiring manual save operations.
///
/// # Events Handled
///
/// - **CloseRequested**: Saves final window state and exits application
/// - **Resized**: Saves new window dimensions
/// - **Moved**: Saves new window position
///
/// # Arguments
///
/// * `window` - The webview window to attach event handlers to
///
/// # Implementation Notes
///
/// The event handlers use a cloned window reference to avoid borrowing issues
/// in the async event closure context.
pub fn setup_window_events(window: &WebviewWindow) {
    let window_clone = window.clone();
    window.on_window_event(move |event| {
        match event {
            WindowEvent::CloseRequested { .. } => {
                save_current_window_state(&window_clone);
                std::process::exit(0);
            }
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                save_current_window_state(&window_clone);
            }
            _ => {}
        }
    });
}

/// Helper function to save current window state to persistent storage
///
/// This function extracts the current window state (size, position, maximized status)
/// and saves it for restoration in future application sessions. It handles scale
/// factor conversion to ensure consistent logical coordinates.
///
/// # Arguments
///
/// * `window` - The window whose state should be saved
///
/// # Implementation Details
///
/// - Converts physical coordinates to logical coordinates using scale factor
/// - Handles potential errors gracefully by ignoring failed operations
/// - Uses logical coordinates for consistent cross-platform behavior
///
/// # Error Handling
///
/// Individual operations may fail (e.g., if window is in an invalid state), but
/// the function continues attempting other operations. The `save_window_state`
/// function call result is also ignored to prevent error propagation to UI thread.
fn save_current_window_state(window: &WebviewWindow) {
    if let (Ok(size), Ok(position)) = (window.inner_size(), window.outer_position()) {
        let is_maximized = window.is_maximized().unwrap_or(false);
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let logical_size = size.to_logical::<f64>(scale_factor);
        let logical_position = position.to_logical::<i32>(scale_factor);
        let _ = save_window_state_internal(
            logical_size.width,
            logical_size.height,
            Some(logical_position.x),
            Some(logical_position.y),
            is_maximized
        );
    }
}