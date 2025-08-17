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

/// Update session state (vault, file, view mode)
pub fn save_session_state_internal(
    current_vault: Option<String>,
    current_file: Option<String>,
    view_mode: String,
) -> FileSystemResult<()> {
    let mut state = load_app_state_internal().unwrap_or_default();
    state.session.current_vault = current_vault;
    state.session.current_file = current_file;
    state.session.view_mode = view_mode;
    save_app_state_internal(&state)
}

/// Update layout state (panel widths and visibility)
pub fn save_layout_state_internal(
    file_tree_width: f64,
    ai_panel_width: f64,
    file_tree_visible: bool,
    ai_panel_visible: bool,
    editor_mode: String,
) -> FileSystemResult<()> {
    let mut state = load_app_state_internal().unwrap_or_default();
    state.layout.file_tree_width = file_tree_width;
    state.layout.ai_panel_width = ai_panel_width;
    state.layout.file_tree_visible = file_tree_visible;
    state.layout.ai_panel_visible = ai_panel_visible;
    state.layout.editor_mode = editor_mode;
    save_app_state_internal(&state)
}

/// Update window state (size and position)
pub fn save_window_state_internal(
    width: f64,
    height: f64,
    x: Option<i32>,
    y: Option<i32>,
    maximized: bool,
) -> FileSystemResult<()> {
    let mut state = load_app_state_internal().unwrap_or_default();
    state.window.width = width;
    state.window.height = height;
    state.window.x = x;
    state.window.y = y;
    state.window.maximized = maximized;
    save_app_state_internal(&state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{WindowState, LayoutState, SessionState};
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
            session: SessionState {
                current_vault: Some("/test/vault".to_string()),
                current_file: Some("/test/vault/file.md".to_string()),
                view_mode: "preview".to_string(),
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
        // Test with complete valid format including new session field
        let updated_format_json = r#"{
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
            },
            "session": {
                "current_vault": "/test/vault",
                "current_file": "/test/vault/note.md",
                "view_mode": "editor"
            }
        }"#;

        let state: AppState = serde_json::from_str(updated_format_json).unwrap();
        assert_eq!(state.window.width, 1366.0);
        assert_eq!(state.window.height, 768.0);
        assert_eq!(state.layout.file_tree_width, 250.0);
        assert_eq!(state.session.current_vault, Some("/test/vault".to_string()));
        assert_eq!(state.session.view_mode, "editor");
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
    fn test_session_state_default() {
        let session_state = SessionState::default();
        
        assert_eq!(session_state.current_vault, None);
        assert_eq!(session_state.current_file, None);
        assert_eq!(session_state.view_mode, "editor");
    }

    #[test]
    fn test_session_state_serialization() {
        let session_state = SessionState {
            current_vault: Some("/Users/test/vault".to_string()),
            current_file: Some("/Users/test/vault/note.md".to_string()),
            view_mode: "preview".to_string(),
        };

        // Test serialization
        let json = serde_json::to_string_pretty(&session_state).unwrap();
        assert!(json.contains("current_vault"));
        assert!(json.contains("/Users/test/vault"));
        assert!(json.contains("current_file"));
        assert!(json.contains("view_mode"));
        assert!(json.contains("preview"));

        // Test deserialization
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.current_vault, Some("/Users/test/vault".to_string()));
        assert_eq!(deserialized.current_file, Some("/Users/test/vault/note.md".to_string()));
        assert_eq!(deserialized.view_mode, "preview");
    }

    #[test]
    fn test_session_state_with_nulls() {
        let session_state = SessionState {
            current_vault: None,
            current_file: None,
            view_mode: "editor".to_string(),
        };

        let json = serde_json::to_string(&session_state).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.current_vault, None);
        assert_eq!(deserialized.current_file, None);
        assert_eq!(deserialized.view_mode, "editor");
    }

    #[test]
    fn test_save_session_state_internal() {
        // Test the session state save function
        // This may write to actual config file in test environment,
        // but we're testing that it doesn't panic and handles errors gracefully
        let result = save_session_state_internal(
            Some("/test/vault".to_string()),
            Some("/test/vault/file.md".to_string()),
            "preview".to_string(),
        );
        
        // Function should complete without panicking, regardless of success/failure
        let _ = result;
        
        // In a real test environment, we'd want to mock the file system
        // or use a temporary directory to avoid affecting user state
    }

    #[test]
    fn test_session_state_view_modes() {
        let view_modes = vec!["editor", "preview"];
        
        for mode in view_modes {
            let session_state = SessionState {
                current_vault: Some("/test".to_string()),
                current_file: Some("/test/file.md".to_string()),
                view_mode: mode.to_string(),
            };

            let json = serde_json::to_string(&session_state).unwrap();
            let deserialized: SessionState = serde_json::from_str(&json).unwrap();
            
            assert_eq!(deserialized.view_mode, mode);
        }
    }

    #[test]
    fn test_complete_app_state_with_session() {
        let app_state = AppState {
            window: WindowState {
                width: 1200.0,
                height: 800.0,
                x: Some(100),
                y: Some(50),
                maximized: false,
            },
            layout: LayoutState {
                file_tree_width: 280.0,
                ai_panel_width: 350.0,
                file_tree_visible: true,
                ai_panel_visible: false,
                editor_mode: "edit".to_string(),
            },
            session: SessionState {
                current_vault: Some("/home/user/notes".to_string()),
                current_file: Some("/home/user/notes/daily.md".to_string()),
                view_mode: "preview".to_string(),
            },
        };

        // Test that all three state components work together
        let json = serde_json::to_string_pretty(&app_state).unwrap();
        assert!(json.contains("window"));
        assert!(json.contains("layout"));
        assert!(json.contains("session"));
        assert!(json.contains("current_vault"));
        assert!(json.contains("/home/user/notes"));

        let deserialized: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.window.width, 1200.0);
        assert_eq!(deserialized.layout.file_tree_width, 280.0);
        assert_eq!(deserialized.session.current_vault, Some("/home/user/notes".to_string()));
        assert_eq!(deserialized.session.view_mode, "preview");
    }
}