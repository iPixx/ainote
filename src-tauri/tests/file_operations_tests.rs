//! File Operations Tests
//! 
//! Comprehensive tests for file CRUD operations, vault scanning, and file lifecycle management.
//! These tests were extracted from the main lib.rs to improve code organization and maintainability.

use ainote_lib::commands::*;
use ainote_lib::types::{FileInfo, AppState, WindowState, LayoutState, SessionState, VaultPreferences};
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

// Test constants
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
fn test_read_file_success() {
    let env = TestEnv::new();
    let test_file = env.get_test_file("test.md");
    env.create_test_file("test.md", TEST_CONTENT).unwrap();

    let result = read_file(test_file);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), TEST_CONTENT);
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
fn test_app_state_default_values() {
    let app_state = AppState::default();
    
    assert_eq!(app_state.window.width, 1200.0);
    assert_eq!(app_state.window.height, 800.0);
    assert_eq!(app_state.layout.file_tree_width, 280.0);
    assert_eq!(app_state.layout.ai_panel_width, 350.0);
    assert_eq!(app_state.layout.file_tree_visible, true);
    assert_eq!(app_state.layout.ai_panel_visible, false);
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
fn test_complete_file_lifecycle() {
    let env = TestEnv::new();
    let file_path = env.get_test_file("lifecycle_test.md");
    
    // 1. Create file
    let result = create_file(file_path.clone());
    assert!(result.is_ok(), "Failed to create file: {:?}", result);
    assert!(Path::new(&file_path).exists());

    // 2. Read file
    let content = read_file(file_path.clone()).unwrap();
    assert!(content.starts_with("# lifecycle_test"));

    // 3. Write to file
    let new_content = "# Updated Content\n\nThis is updated content.";
    let result = write_file(file_path.clone(), new_content.to_string());
    assert!(result.is_ok(), "Failed to write file: {:?}", result);

    // 4. Read updated content
    let updated_content = read_file(file_path.clone()).unwrap();
    assert_eq!(updated_content, new_content);

    // 5. Test auto-save
    let auto_save_content = "# Auto-saved Content\n\nThis is auto-saved.";
    let result = auto_save_file(file_path.clone(), auto_save_content.to_string());
    assert!(result.is_ok(), "Failed to auto-save file: {:?}", result);

    // 6. Read auto-saved content
    let auto_saved_content = read_file(file_path.clone()).unwrap();
    assert_eq!(auto_saved_content, auto_save_content);

    // 7. Rename file
    let new_file_path = env.get_test_file("renamed_lifecycle_test.md");
    let result = rename_file(file_path.clone(), new_file_path.clone());
    assert!(result.is_ok(), "Failed to rename file: {:?}", result);
    assert!(!Path::new(&file_path).exists());
    assert!(Path::new(&new_file_path).exists());

    // 8. Delete file
    let result = delete_file(new_file_path.clone());
    assert!(result.is_ok(), "Failed to delete file: {:?}", result);
    assert!(!Path::new(&new_file_path).exists());
}

#[test]
fn test_vault_and_state_integration() {
    let env = TestEnv::new();
    
    // Create test vault structure
    env.create_directory_structure(&["notes", "archive", "projects/ai"]).unwrap();
    env.create_test_file("notes/daily.md", "# Daily Notes").unwrap();
    env.create_test_file("notes/ideas.md", "# Ideas").unwrap();
    env.create_test_file("archive/old.md", "# Old Notes").unwrap();
    env.create_test_file("projects/ai/research.md", "# AI Research").unwrap();
    env.create_test_file("README.txt", "Not markdown").unwrap(); // Should be ignored

    // Test vault scanning
    let result = scan_vault_files(env.get_path());
    assert!(result.is_ok());
    
    let files = result.unwrap();
    let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
    let directories: Vec<_> = files.iter().filter(|f| f.is_dir).collect();
    
    assert_eq!(md_files.len(), 4);
    assert_eq!(directories.len(), 4); // notes, archive, projects, ai

    // Test chunked scanning
    let result = scan_vault_files_chunked(env.get_path(), 0, 3);
    assert!(result.is_ok());
    let (chunk, has_more) = result.unwrap();
    assert_eq!(chunk.len(), 3);
    assert!(has_more);

    // Test app state operations
    let mut app_state = AppState::default();
    app_state.window.width = 1440.0;
    app_state.layout.file_tree_width = 320.0;
    
    // Note: We can't fully test state management without mocking dirs::home_dir
    // But we can test serialization
    let json = serde_json::to_string(&app_state).unwrap();
    let deserialized: AppState = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.window.width, 1440.0);
    assert_eq!(deserialized.layout.file_tree_width, 320.0);
}

#[test]
fn test_error_handling_integration() {
    let env = TestEnv::new();
    
    // Test various error conditions
    let nonexistent_file = env.get_test_file("nonexistent.md");
    let invalid_file = env.get_test_file("test.txt");
    
    // Test read errors
    let result = read_file(nonexistent_file.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("could not be found"));

    // Test invalid extension errors
    let result = create_file(invalid_file.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not a markdown file"));

    // Test file already exists error
    let test_file = env.get_test_file("existing.md");
    env.create_test_file("existing.md", "content").unwrap();
    let result = create_file(test_file.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));

    // Test delete non-existent file
    let result = delete_file(nonexistent_file);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("could not be found"));

    // Test rename errors
    let result = rename_file(
        env.get_test_file("nonexistent.md"),
        env.get_test_file("target.md")
    );
    assert!(result.is_err());

    // Test directory operations on vault scanning
    let result = scan_vault_files("nonexistent_directory".to_string());
    assert!(result.is_err());
}

#[test]
fn test_preview_functionality() {
    let env = TestEnv::new();
    let test_file = env.get_test_file("preview.md");
    
    // Test small file preview
    let small_content = "# Small File\n\nThis is a small file.";
    env.create_test_file("preview.md", small_content).unwrap();
    
    let result = preview_file(test_file.clone(), Some(1000));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), small_content);

    // Test large file preview
    let large_content = "# Large File\n\n".to_string() + &"A".repeat(2000);
    env.create_test_file("preview.md", &large_content).unwrap();
    
    let result = preview_file(test_file, Some(100));
    assert!(result.is_ok());
    let preview = result.unwrap();
    assert!(preview.len() < large_content.len());
    assert!(preview.contains("File preview truncated"));
}

#[test]
fn test_concurrent_file_operations() {
    use std::sync::Arc;
    use std::thread;
    
    let env = Arc::new(TestEnv::new());
    
    // Test concurrent file creation
    let handles: Vec<_> = (0..5).map(|i| {
        let env_clone = Arc::clone(&env);
        thread::spawn(move || {
            let file_path = env_clone.get_test_file(&format!("concurrent_{}.md", i));
            create_file(file_path)
        })
    }).collect();

    // All should succeed since they're different files
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok(), "Failed to create concurrent file: {:?}", result);
    }

    // Test concurrent access to same file (should use file locking)
    let shared_file = env.get_test_file("shared.md");
    let _ = create_file(shared_file.clone());
    
    let handles: Vec<_> = (0..3).map(|i| {
        let file_path = shared_file.clone();
        thread::spawn(move || {
            // Add small delay to reduce contention
            thread::sleep(std::time::Duration::from_millis(i * 10));
            write_file(file_path, format!("Content from thread {}", i))
        })
    }).collect();

    // Count successful operations (some may fail due to locking, which is expected)
    let mut successful = 0;
    for handle in handles {
        let result = handle.join().unwrap();
        if result.is_ok() {
            successful += 1;
        }
    }
    
    // At least one should succeed
    assert!(successful >= 1, "No concurrent operations succeeded");
}

#[test]
fn test_new_vault_commands() {
    let env = TestEnv::new();
    
    // Create test vault structure
    env.create_directory_structure(&["notes", "projects"]).unwrap();
    env.create_test_file("notes/daily.md", "# Daily Notes").unwrap();
    env.create_test_file("projects/work.md", "# Work Notes").unwrap();
    
    let vault_path = env.get_path();
    
    // Test validate_vault command
    let validate_result = validate_vault(vault_path.clone());
    assert!(validate_result.is_ok());
    assert_eq!(validate_result.unwrap(), true);
    
    // Test validate_vault with nonexistent path
    let invalid_validate = validate_vault("nonexistent_path".to_string());
    assert!(invalid_validate.is_ok());
    assert_eq!(invalid_validate.unwrap(), false);
    
    // Test load_vault command
    let load_result = load_vault(vault_path);
    assert!(load_result.is_ok());
    
    let files = load_result.unwrap();
    let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
    let directories: Vec<_> = files.iter().filter(|f| f.is_dir).collect();
    
    assert_eq!(md_files.len(), 2);
    assert_eq!(directories.len(), 2);
    
    // Test load_vault with nonexistent path
    let invalid_load = load_vault("nonexistent_path".to_string());
    assert!(invalid_load.is_err());
}

#[test]
fn test_comprehensive_tauri_command_integration() {
    let env = TestEnv::new();
    
    // Test complete integration flow through Tauri commands
    
    // 1. Test vault selection and scanning
    let vault_path = env.get_path();
    
    // Create a complete vault structure
    env.create_directory_structure(&["docs", "projects", "archive"]).unwrap();
    env.create_test_file("docs/readme.md", "# Documentation\n\nMain docs").unwrap();
    env.create_test_file("projects/ai-notes.md", "# AI Notes\n\nProject notes").unwrap();
    env.create_test_file("archive/old-notes.md", "# Old Notes\n\nArchived content").unwrap();
    
    // Test vault scanning command
    let files_result = scan_vault_files(vault_path.clone());
    assert!(files_result.is_ok());
    let files = files_result.unwrap();
    
    let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
    let dirs: Vec<_> = files.iter().filter(|f| f.is_dir).collect();
    
    assert_eq!(md_files.len(), 3);
    assert_eq!(dirs.len(), 3);
    
    // 2. Test chunked scanning command
    let chunk_result = scan_vault_files_chunked(vault_path, 0, 2);
    assert!(chunk_result.is_ok());
    let (chunk, has_more) = chunk_result.unwrap();
    assert_eq!(chunk.len(), 2);
    assert!(has_more);
    
    // 3. Test file operations integration
    let test_file = env.get_test_file("integration-test.md");
    
    // Create file command
    let create_result = create_file(test_file.clone());
    assert!(create_result.is_ok());
    
    // Read file command
    let read_result = read_file(test_file.clone());
    assert!(read_result.is_ok());
    let initial_content = read_result.unwrap();
    assert!(initial_content.contains("# integration-test"));
    
    // Write file command
    let new_content = "# Integration Test\n\nThis is a comprehensive test of all commands.";
    let write_result = write_file(test_file.clone(), new_content.to_string());
    assert!(write_result.is_ok());
    
    // Auto-save command
    let auto_save_content = "# Integration Test\n\nAuto-saved content.";
    let auto_save_result = auto_save_file(test_file.clone(), auto_save_content.to_string());
    assert!(auto_save_result.is_ok());
    
    // Preview command
    let preview_result = preview_file(test_file.clone(), Some(100));
    assert!(preview_result.is_ok());
    let preview_content = preview_result.unwrap();
    assert!(preview_content.contains("Integration Test"));
    
    // Rename file command
    let renamed_file = env.get_test_file("renamed-integration-test.md");
    let rename_result = rename_file(test_file.clone(), renamed_file.clone());
    assert!(rename_result.is_ok());
    
    // Verify rename worked
    assert!(!Path::new(&test_file).exists());
    assert!(Path::new(&renamed_file).exists());
    
    // Delete file command
    let delete_result = delete_file(renamed_file.clone());
    assert!(delete_result.is_ok());
    assert!(!Path::new(&renamed_file).exists());
    
    // 4. Test state management commands
    
    // Test app state loading (should return default)
    let _load_state_result = load_app_state();
    // This may fail due to home directory access, which is expected in test environment
    
    // Test app state saving
    let test_state = AppState {
        window: WindowState {
            width: 1440.0,
            height: 900.0,
            x: Some(100),
            y: Some(50),
            maximized: false,
        },
        layout: LayoutState {
            file_tree_width: 320.0,
            ai_panel_width: 400.0,
            file_tree_visible: true,
            ai_panel_visible: false,
            editor_mode: "split".to_string(),
        },
        session: SessionState::default(),
        vault_preferences: VaultPreferences::default(),
    };
    
    let _save_state_result = save_app_state(test_state.clone());
    // May fail due to home directory access, which is expected
    
    // Note: Avoid testing individual Tauri commands that write to real state file
    // These are tested separately in state_management.rs with proper isolation
    
    // These may fail in test environment due to home directory access
    // The important thing is they don't panic and handle errors gracefully
}