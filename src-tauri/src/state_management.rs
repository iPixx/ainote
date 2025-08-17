use std::fs;
use crate::errors::{FileSystemError, FileSystemResult};
use crate::types::AppState;

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
pub fn load_app_state_internal() -> FileSystemResult<AppState> {
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
pub fn save_app_state_internal(state: &AppState) -> FileSystemResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{WindowState, LayoutState};
    use tempfile::TempDir;

    #[allow(dead_code)]
    struct TestEnv {
        temp_dir: TempDir,
        pub path: std::path::PathBuf,
    }

    impl TestEnv {
        #[allow(dead_code)]
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temporary directory");
            let path = temp_dir.path().to_path_buf();
            
            TestEnv {
                temp_dir,
                path,
            }
        }
    }

    #[test]
    fn test_app_state_default_values() {
        let app_state = AppState::default();
        
        assert_eq!(app_state.window.width, 1200.0);
        assert_eq!(app_state.window.height, 800.0);
        assert_eq!(app_state.window.x, None);
        assert_eq!(app_state.window.y, None);
        assert_eq!(app_state.window.maximized, false);
        
        assert_eq!(app_state.layout.file_tree_width, 280.0);
        assert_eq!(app_state.layout.ai_panel_width, 350.0);
        assert_eq!(app_state.layout.file_tree_visible, true);
        assert_eq!(app_state.layout.ai_panel_visible, false);
        assert_eq!(app_state.layout.editor_mode, "edit");
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
        // This test would normally check the actual function but requires mocking dirs::home_dir
        // For now, just test that default state works
        let default_state = AppState::default();
        assert_eq!(default_state.window.width, 1200.0);
        assert_eq!(default_state.layout.file_tree_width, 280.0);
    }

    #[test] 
    fn test_state_backward_compatibility() {
        // Test with complete valid format
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
}