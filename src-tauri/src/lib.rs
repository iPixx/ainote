use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub modified: u64,
    pub size: u64,
    pub is_dir: bool,
}

impl FileInfo {
    pub fn from_dir_entry(entry: &std::fs::DirEntry) -> Result<Self, String> {
        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();
        
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let metadata = entry.metadata()
            .map_err(|e| format!("Failed to read metadata for {}: {}", path_str, e))?;

        let modified = metadata
            .modified()
            .map_err(|e| format!("Failed to read modified time for {}: {}", path_str, e))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Invalid modified time for {}: {}", path_str, e))?
            .as_secs();

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

    pub fn from_path(path: &Path) -> Result<Self, String> {
        let path_str = path.to_string_lossy().to_string();
        
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let metadata = path.metadata()
            .map_err(|e| format!("Failed to read metadata for {}: {}", path_str, e))?;

        let modified = metadata
            .modified()
            .map_err(|e| format!("Failed to read modified time for {}: {}", path_str, e))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Invalid modified time for {}: {}", path_str, e))?
            .as_secs();

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

    pub fn compare_by_name(&self, other: &Self) -> std::cmp::Ordering {
        self.name.to_lowercase().cmp(&other.name.to_lowercase())
    }

    pub fn compare_by_modified(&self, other: &Self) -> std::cmp::Ordering {
        self.modified.cmp(&other.modified)
    }

    pub fn compare_by_size(&self, other: &Self) -> std::cmp::Ordering {
        self.size.cmp(&other.size)
    }
}

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
            return Err(format!(
                "Only markdown (.md) files are supported: {}",
                file_path
            ));
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
            return Err(format!(
                "Only markdown (.md) files are supported: {}",
                file_path
            ));
        }
    } else {
        return Err(format!("File must have .md extension: {}", file_path));
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
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
            return Err(format!(
                "Only markdown (.md) files are supported: {}",
                file_path
            ));
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
                return Err(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
            }
        }
    }

    // Get filename without extension for the title
    let title = path
        .file_stem()
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
            return Err(format!(
                "Only markdown (.md) files can be deleted: {}",
                file_path
            ));
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
                return Err(format!(
                    "{} file must have .md extension: {}",
                    name,
                    path.display()
                ));
            }
        } else {
            return Err(format!(
                "{} file must have .md extension: {}",
                name,
                path.display()
            ));
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
                return Err(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
            }
        }
    }

    // Rename the file
    match fs::rename(old, new) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!(
            "Failed to rename file from {} to {}: {}",
            old_path, new_path, e
        )),
    }
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
    let vault_path = Path::new(&vault_path);
    
    // Validate vault path exists and is a directory
    if !vault_path.exists() {
        return Err(format!("Vault path does not exist: {}", vault_path.display()));
    }
    
    if !vault_path.is_dir() {
        return Err(format!("Vault path is not a directory: {}", vault_path.display()));
    }

    let mut files = Vec::new();
    
    // Recursive function to scan directories
    fn scan_directory(dir: &Path, files: &mut Vec<FileInfo>) -> Result<(), String> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                // Log the error but continue with other directories
                eprintln!("Warning: Failed to read directory {}: {}", dir.display(), e);
                return Ok(());
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    // Log the error but continue with other entries
                    eprintln!("Warning: Failed to read directory entry in {}: {}", dir.display(), e);
                    continue;
                }
            };

            let path = entry.path();
            
            // Handle symbolic links gracefully
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    // Skip entries we can't read metadata for (broken symlinks, permission issues)
                    continue;
                }
            };

            if metadata.is_dir() {
                // Include directory in results
                if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
                    files.push(file_info);
                }
                
                // Recursively scan subdirectory
                if let Err(e) = scan_directory(&path, files) {
                    eprintln!("Warning: Error scanning subdirectory {}: {}", path.display(), e);
                }
            } else if metadata.is_file() {
                // Only include .md files
                if let Some(extension) = path.extension() {
                    if extension == "md" {
                        if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
                            files.push(file_info);
                        }
                    }
                }
            }
            // Skip other file types (symlinks, special files, etc.)
        }
        
        Ok(())
    }

    // Start recursive scanning
    scan_directory(vault_path, &mut files)?;
    
    // Sort files by name for consistent UI display (directories first, then files)
    files.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,   // Directories first
            (false, true) => std::cmp::Ordering::Greater, // Files second
            _ => a.compare_by_name(b),                    // Then alphabetical within each group
        }
    });
    
    Ok(files)
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
            rename_file,
            select_vault_folder,
            scan_vault_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn get_test_dir() -> String {
        format!("test_files_{}", std::process::id())
    }
    
    fn get_test_file() -> String {
        format!("{}/test.md", get_test_dir())
    }
    
    fn get_test_file_2() -> String {
        format!("{}/test2.md", get_test_dir())
    }
    
    const TEST_CONTENT: &str = "# Test Content\n\nThis is test content.";

    fn setup_test_dir() {
        let test_dir = get_test_dir();
        if Path::new(&test_dir).exists() {
            fs::remove_dir_all(&test_dir).ok();
        }
        fs::create_dir_all(&test_dir).unwrap();
    }

    fn cleanup_test_dir() {
        let test_dir = get_test_dir();
        if Path::new(&test_dir).exists() {
            fs::remove_dir_all(&test_dir).ok();
        }
    }

    #[test]
    fn test_create_file_success() {
        setup_test_dir();

        let result = create_file(get_test_file());
        assert!(result.is_ok());
        assert!(Path::new(&get_test_file()).exists());

        let content = fs::read_to_string(&get_test_file()).unwrap();
        assert_eq!(content, "# test\n\n");

        cleanup_test_dir();
    }

    #[test]
    fn test_create_file_invalid_extension() {
        setup_test_dir();

        let result = create_file(format!("{}/test.txt", get_test_dir()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Only markdown (.md) files are supported"));

        cleanup_test_dir();
    }

    #[test]
    fn test_create_file_already_exists() {
        setup_test_dir();
        fs::write(&get_test_file(), "existing content").unwrap();

        let result = create_file(get_test_file());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File already exists"));

        cleanup_test_dir();
    }

    #[test]
    fn test_write_file_success() {
        setup_test_dir();

        let result = write_file(get_test_file(), TEST_CONTENT.to_string());
        assert!(result.is_ok());
        assert!(Path::new(&get_test_file()).exists());

        let content = fs::read_to_string(&get_test_file()).unwrap();
        assert_eq!(content, TEST_CONTENT);

        cleanup_test_dir();
    }

    #[test]
    fn test_write_file_invalid_extension() {
        setup_test_dir();

        let result = write_file(format!("{}/test.txt", get_test_dir()), TEST_CONTENT.to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Only markdown (.md) files are supported"));

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_success() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = read_file(get_test_file());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_CONTENT);

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_not_found() {
        setup_test_dir();

        let result = read_file(format!("{}/nonexistent.md", get_test_dir()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_invalid_extension() {
        setup_test_dir();
        fs::write(&format!("{}/test.txt", get_test_dir()), "content").unwrap();

        let result = read_file(format!("{}/test.txt", get_test_dir()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Only markdown (.md) files are supported"));

        cleanup_test_dir();
    }

    #[test]
    fn test_read_file_is_directory() {
        setup_test_dir();
        fs::create_dir_all(&format!("{}/subdir.md", get_test_dir())).unwrap();

        let result = read_file(format!("{}/subdir.md", get_test_dir()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path is not a file"));

        cleanup_test_dir();
    }

    #[test]
    fn test_delete_file_success() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = delete_file(get_test_file());
        assert!(result.is_ok());
        assert!(!Path::new(&get_test_file()).exists());

        cleanup_test_dir();
    }

    #[test]
    fn test_delete_file_not_found() {
        setup_test_dir();

        let result = delete_file(format!("{}/nonexistent.md", get_test_dir()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));

        cleanup_test_dir();
    }

    #[test]
    fn test_delete_file_invalid_extension() {
        setup_test_dir();
        fs::write(&format!("{}/test.txt", get_test_dir()), "content").unwrap();

        let result = delete_file(format!("{}/test.txt", get_test_dir()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Only markdown (.md) files can be deleted"));

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_success() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = rename_file(get_test_file(), get_test_file_2());
        assert!(result.is_ok());
        assert!(!Path::new(&get_test_file()).exists());
        assert!(Path::new(&get_test_file_2()).exists());

        let content = fs::read_to_string(&get_test_file_2()).unwrap();
        assert_eq!(content, TEST_CONTENT);

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_source_not_found() {
        setup_test_dir();

        let result = rename_file(
            format!("{}/nonexistent.md", get_test_dir()),
            get_test_file_2(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Source file not found"));

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_destination_exists() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();
        fs::write(&get_test_file_2(), "other content").unwrap();

        let result = rename_file(get_test_file(), get_test_file_2());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Destination file already exists"));

        cleanup_test_dir();
    }

    #[test]
    fn test_rename_file_invalid_extension() {
        setup_test_dir();
        fs::write(&format!("{}/test.txt", get_test_dir()), "content").unwrap();

        let result = rename_file(format!("{}/test.txt", get_test_dir()), get_test_file_2());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Source file must have .md extension"));

        cleanup_test_dir();
    }

    #[test]
    fn test_utf8_encoding() {
        setup_test_dir();
        let utf8_content = "# UTF-8 Test\n\n✅ Checkmark\n🎉 Emoji\nÀccëntéd characters";

        let write_result = write_file(get_test_file(), utf8_content.to_string());
        assert!(write_result.is_ok());

        let read_result = read_file(get_test_file());
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), utf8_content);

        cleanup_test_dir();
    }

    #[test]
    fn test_nested_directory_creation() {
        setup_test_dir();
        let nested_file = format!("{}/nested/deep/file.md", get_test_dir());

        let result = create_file(nested_file.clone());
        assert!(result.is_ok());
        assert!(Path::new(&nested_file).exists());

        let content = fs::read_to_string(&nested_file).unwrap();
        assert_eq!(content, "# file\n\n");

        cleanup_test_dir();
    }

    #[test]
    fn test_file_info_from_path() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let test_file = get_test_file();
        let path = Path::new(&test_file);
        let file_info = FileInfo::from_path(path).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert_eq!(file_info.path, get_test_file());
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);
        assert!(file_info.modified > 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_file_info_from_dir_entry() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let entries: Vec<_> = fs::read_dir(&get_test_dir()).unwrap().collect();
        let entry = entries.into_iter().find(|e| {
            e.as_ref().unwrap().file_name() == "test.md"
        }).unwrap().unwrap();

        let file_info = FileInfo::from_dir_entry(&entry).unwrap();

        assert_eq!(file_info.name, "test.md");
        assert!(!file_info.is_dir);
        assert!(file_info.size > 0);

        cleanup_test_dir();
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
    fn test_scan_vault_files_empty_directory() {
        setup_test_dir();

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 0);

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_with_markdown_files() {
        setup_test_dir();
        
        // Create test files
        fs::write(format!("{}/note1.md", get_test_dir()), "# Note 1").unwrap();
        fs::write(format!("{}/note2.md", get_test_dir()), "# Note 2").unwrap();
        fs::write(format!("{}/readme.txt", get_test_dir()), "Not a markdown file").unwrap(); // Should be ignored

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 2); // Only .md files should be included
        
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        assert_eq!(md_files.len(), 2);
        
        // Check that files are sorted alphabetically
        assert!(md_files[0].name <= md_files[1].name);

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_nested_directories() {
        setup_test_dir();
        
        // Create nested structure
        fs::create_dir_all(format!("{}/subdir/deep", get_test_dir())).unwrap();
        fs::write(format!("{}/root.md", get_test_dir()), "# Root note").unwrap();
        fs::write(format!("{}/subdir/sub.md", get_test_dir()), "# Sub note").unwrap();
        fs::write(format!("{}/subdir/deep/deep.md", get_test_dir()), "# Deep note").unwrap();

        let result = scan_vault_files(get_test_dir());
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

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_nonexistent_path() {
        let result = scan_vault_files("nonexistent_directory".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Vault path does not exist"));
    }

    #[test]
    fn test_scan_vault_files_file_instead_of_directory() {
        setup_test_dir();
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = scan_vault_files(get_test_file());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Vault path is not a directory"));

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_performance_target() {
        setup_test_dir();
        
        // Create many files to test performance
        for i in 0..100 {
            fs::write(format!("{}/note_{:03}.md", get_test_dir(), i), format!("# Note {}", i)).unwrap();
        }

        let start = std::time::Instant::now();
        let result = scan_vault_files(get_test_dir());
        let duration = start.elapsed();

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.iter().filter(|f| !f.is_dir).count(), 100);
        
        // Performance target: <500ms for 1000+ files, so 100 files should be much faster
        assert!(duration.as_millis() < 100, "Scanning took too long: {:?}", duration);

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_mixed_file_types() {
        setup_test_dir();
        
        // Create various file types
        fs::write(format!("{}/note.md", get_test_dir()), "# Markdown note").unwrap();
        fs::write(format!("{}/document.txt", get_test_dir()), "Text document").unwrap();
        fs::write(format!("{}/script.js", get_test_dir()), "console.log('hello')").unwrap();
        fs::write(format!("{}/data.json", get_test_dir()), "{}").unwrap();
        fs::write(format!("{}/README", get_test_dir()), "No extension").unwrap();

        let result = scan_vault_files(get_test_dir());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        let file_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        
        // Only the .md file should be included
        assert_eq!(file_files.len(), 1);
        assert_eq!(file_files[0].name, "note.md");

        cleanup_test_dir();
    }

    #[test]
    fn test_scan_vault_files_cross_platform_paths() {
        setup_test_dir();
        
        // Create a file and test that paths are handled properly
        fs::write(&get_test_file(), TEST_CONTENT).unwrap();

        let result = scan_vault_files(get_test_dir());
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

        cleanup_test_dir();
    }
}
