use std::fs;
use std::path::Path;

#[tauri::command]
fn read_file(file_path: String) -> Result<String, String> {
    let path = Path::new(&file_path);
    
    // Validate path exists and is a file
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }
    
    // Ensure it's a markdown file
    if let Some(extension) = path.extension() {
        if extension != "md" {
            return Err(format!("Only markdown (.md) files are supported: {}", file_path));
        }
    } else {
        return Err(format!("File must have .md extension: {}", file_path));
    }
    
    // Read file content with UTF-8 encoding
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Failed to read file {}: {}", file_path, e)),
    }
}

#[tauri::command]
fn write_file(file_path: String, content: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    
    // Validate path and extension
    if let Some(extension) = path.extension() {
        if extension != "md" {
            return Err(format!("Only markdown (.md) files are supported: {}", file_path));
        }
    } else {
        return Err(format!("File must have .md extension: {}", file_path));
    }
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Failed to create directory {}: {}", parent.display(), e));
            }
        }
    }
    
    // Write file content with UTF-8 encoding
    match fs::write(path, content) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to write file {}: {}", file_path, e)),
    }
}

#[tauri::command]
fn create_file(file_path: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    
    // Validate path and extension
    if let Some(extension) = path.extension() {
        if extension != "md" {
            return Err(format!("Only markdown (.md) files are supported: {}", file_path));
        }
    } else {
        return Err(format!("File must have .md extension: {}", file_path));
    }
    
    // Check if file already exists
    if path.exists() {
        return Err(format!("File already exists: {}", file_path));
    }
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Failed to create directory {}: {}", parent.display(), e));
            }
        }
    }
    
    // Get filename without extension for the title
    let title = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");
    
    // Create markdown template
    let template = format!("# {}\n\n", title);
    
    // Create file with template content
    match fs::write(path, template) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to create file {}: {}", file_path, e)),
    }
}

#[tauri::command]
fn delete_file(file_path: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    
    // Validate path exists and is a file
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }
    
    // Additional validation - ensure it's a markdown file
    if let Some(extension) = path.extension() {
        if extension != "md" {
            return Err(format!("Only markdown (.md) files can be deleted: {}", file_path));
        }
    } else {
        return Err(format!("File must have .md extension: {}", file_path));
    }
    
    // Delete the file
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to delete file {}: {}", file_path, e)),
    }
}

#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    let old = Path::new(&old_path);
    let new = Path::new(&new_path);
    
    // Validate old path exists and is a file
    if !old.exists() {
        return Err(format!("Source file not found: {}", old_path));
    }
    
    if !old.is_file() {
        return Err(format!("Source path is not a file: {}", old_path));
    }
    
    // Validate both paths have .md extension
    for (path, name) in [(old, "Source"), (new, "Destination")] {
        if let Some(extension) = path.extension() {
            if extension != "md" {
                return Err(format!("{} file must have .md extension: {}", name, path.display()));
            }
        } else {
            return Err(format!("{} file must have .md extension: {}", name, path.display()));
        }
    }
    
    // Check if destination already exists
    if new.exists() {
        return Err(format!("Destination file already exists: {}", new_path));
    }
    
    // Create destination directory if it doesn't exist
    if let Some(parent) = new.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Failed to create directory {}: {}", parent.display(), e));
            }
        }
    }
    
    // Rename the file
    match fs::rename(old, new) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to rename file from {} to {}: {}", old_path, new_path, e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            create_file,
            delete_file,
            rename_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    
    const TEST_DIR: &str = "test_files";
    const TEST_FILE: &str = "test_files/test.md";
    const TEST_FILE_2: &str = "test_files/test2.md";
    const TEST_CONTENT: &str = "# Test Content\n\nThis is test content.";
    
    fn setup_test_dir() {
        if Path::new(TEST_DIR).exists() {
            fs::remove_dir_all(TEST_DIR).ok();
        }
        fs::create_dir_all(TEST_DIR).unwrap();
    }
    
    fn cleanup_test_dir() {
        if Path::new(TEST_DIR).exists() {
            fs::remove_dir_all(TEST_DIR).ok();
        }
    }
    
    #[test]
    fn test_create_file_success() {
        setup_test_dir();
        
        let result = create_file(TEST_FILE.to_string());
        assert!(result.is_ok());
        assert!(Path::new(TEST_FILE).exists());
        
        let content = fs::read_to_string(TEST_FILE).unwrap();
        assert_eq!(content, "# test\n\n");
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_create_file_invalid_extension() {
        setup_test_dir();
        
        let result = create_file("test_files/test.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Only markdown (.md) files are supported"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_create_file_already_exists() {
        setup_test_dir();
        fs::write(TEST_FILE, "existing content").unwrap();
        
        let result = create_file(TEST_FILE.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File already exists"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_write_file_success() {
        setup_test_dir();
        
        let result = write_file(TEST_FILE.to_string(), TEST_CONTENT.to_string());
        assert!(result.is_ok());
        assert!(Path::new(TEST_FILE).exists());
        
        let content = fs::read_to_string(TEST_FILE).unwrap();
        assert_eq!(content, TEST_CONTENT);
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_write_file_invalid_extension() {
        setup_test_dir();
        
        let result = write_file("test_files/test.txt".to_string(), TEST_CONTENT.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Only markdown (.md) files are supported"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_read_file_success() {
        setup_test_dir();
        fs::write(TEST_FILE, TEST_CONTENT).unwrap();
        
        let result = read_file(TEST_FILE.to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_CONTENT);
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_read_file_not_found() {
        setup_test_dir();
        
        let result = read_file("test_files/nonexistent.md".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_read_file_invalid_extension() {
        setup_test_dir();
        fs::write("test_files/test.txt", "content").unwrap();
        
        let result = read_file("test_files/test.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Only markdown (.md) files are supported"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_read_file_is_directory() {
        setup_test_dir();
        fs::create_dir_all("test_files/subdir.md").unwrap();
        
        let result = read_file("test_files/subdir.md".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path is not a file"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_delete_file_success() {
        setup_test_dir();
        fs::write(TEST_FILE, TEST_CONTENT).unwrap();
        
        let result = delete_file(TEST_FILE.to_string());
        assert!(result.is_ok());
        assert!(!Path::new(TEST_FILE).exists());
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_delete_file_not_found() {
        setup_test_dir();
        
        let result = delete_file("test_files/nonexistent.md".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_delete_file_invalid_extension() {
        setup_test_dir();
        fs::write("test_files/test.txt", "content").unwrap();
        
        let result = delete_file("test_files/test.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Only markdown (.md) files can be deleted"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_rename_file_success() {
        setup_test_dir();
        fs::write(TEST_FILE, TEST_CONTENT).unwrap();
        
        let result = rename_file(TEST_FILE.to_string(), TEST_FILE_2.to_string());
        assert!(result.is_ok());
        assert!(!Path::new(TEST_FILE).exists());
        assert!(Path::new(TEST_FILE_2).exists());
        
        let content = fs::read_to_string(TEST_FILE_2).unwrap();
        assert_eq!(content, TEST_CONTENT);
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_rename_file_source_not_found() {
        setup_test_dir();
        
        let result = rename_file("test_files/nonexistent.md".to_string(), TEST_FILE_2.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Source file not found"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_rename_file_destination_exists() {
        setup_test_dir();
        fs::write(TEST_FILE, TEST_CONTENT).unwrap();
        fs::write(TEST_FILE_2, "other content").unwrap();
        
        let result = rename_file(TEST_FILE.to_string(), TEST_FILE_2.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Destination file already exists"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_rename_file_invalid_extension() {
        setup_test_dir();
        fs::write("test_files/test.txt", "content").unwrap();
        
        let result = rename_file("test_files/test.txt".to_string(), TEST_FILE_2.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Source file must have .md extension"));
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_utf8_encoding() {
        setup_test_dir();
        let utf8_content = "# UTF-8 Test\n\n✅ Checkmark\n🎉 Emoji\nÀccëntéd characters";
        
        let write_result = write_file(TEST_FILE.to_string(), utf8_content.to_string());
        assert!(write_result.is_ok());
        
        let read_result = read_file(TEST_FILE.to_string());
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), utf8_content);
        
        cleanup_test_dir();
    }
    
    #[test]
    fn test_nested_directory_creation() {
        setup_test_dir();
        let nested_file = "test_files/nested/deep/file.md";
        
        let result = create_file(nested_file.to_string());
        assert!(result.is_ok());
        assert!(Path::new(nested_file).exists());
        
        let content = fs::read_to_string(nested_file).unwrap();
        assert_eq!(content, "# file\n\n");
        
        cleanup_test_dir();
    }
}
