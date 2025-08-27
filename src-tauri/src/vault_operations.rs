use std::fs;
use std::path::Path;
use crate::errors::{FileSystemError, FileSystemResult};
use crate::validation;
use crate::types::FileInfo;
use crate::performance::{time_operation, PerformanceTracker};
// File monitoring is now handled by the enhanced file_monitor module

/// Chunked scanning for very large vaults to avoid UI blocking
pub fn scan_vault_files_chunked_internal(
    vault_path: &str, 
    page: usize, 
    page_size: usize
) -> FileSystemResult<(Vec<FileInfo>, bool)> {
    time_operation!({
        let vault_path = Path::new(vault_path);
        
        // Validate vault path exists and is a directory
        validation::validate_path_exists(vault_path)?;
        validation::validate_is_directory(vault_path)?;

        // For chunked scanning, we need to scan everything first then paginate
        // In a real implementation, this could be optimized with a streaming approach
        let all_files = scan_vault_files_internal(&vault_path.to_string_lossy())?;
        
        let start_idx = page * page_size;
        let end_idx = ((page + 1) * page_size).min(all_files.len());
        
        let chunk = if start_idx < all_files.len() {
            all_files[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };
        
        let has_more = end_idx < all_files.len();
        
        Ok((chunk, has_more))
    }, &format!("scan_vault_files_chunked(page={}, size={})", page, page_size))
}

/// Internal scan vault files function using structured error handling
pub fn scan_vault_files_internal(vault_path: &str) -> FileSystemResult<Vec<FileInfo>> {
    time_operation!({
        let tracker = PerformanceTracker::start("scan_vault_files");
        let vault_path = Path::new(vault_path);
        
        // Validate vault path exists and is a directory
        validation::validate_path_exists(vault_path)?;
        validation::validate_is_directory(vault_path)?;
        
        tracker.checkpoint("validation_complete");

        // Use efficient iterator-based scanning with capacity pre-allocation
        let mut files = Vec::with_capacity(256); // Pre-allocate for typical vaults
        let mut directories = Vec::with_capacity(32); // Track directories to scan
        
        // Efficient non-recursive scanning using a work queue
        scan_directory_iterative(vault_path, &mut files, &mut directories)?;
        
        tracker.checkpoint("scanning_complete");
        
        // Efficient in-place sorting (directories first, then files alphabetically)
        sort_files_efficiently(&mut files);
        
        tracker.checkpoint("sorting_complete");
        let _duration = tracker.finish();
        
        Ok(files)
    }, "scan_vault_files_total")
}

/// Optimized iterative directory scanning to avoid stack overflow and improve performance
fn scan_directory_iterative(
    root_path: &Path, 
    files: &mut Vec<FileInfo>, 
    work_queue: &mut Vec<std::path::PathBuf>
) -> FileSystemResult<()> {
    work_queue.push(root_path.to_path_buf());
    
    while let Some(current_dir) = work_queue.pop() {
        if let Err(e) = scan_single_directory(&current_dir, files, work_queue) {
            // Log error but continue with other directories
            eprintln!("Warning: Error scanning directory {}: {}", current_dir.display(), e);
        }
    }
    
    Ok(())
}

/// Scan a single directory efficiently with early filtering and batch processing
fn scan_single_directory(
    dir: &Path, 
    files: &mut Vec<FileInfo>, 
    work_queue: &mut Vec<std::path::PathBuf>
) -> FileSystemResult<()> {
    let entries = fs::read_dir(dir)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied { 
                path: dir.to_string_lossy().to_string() 
            },
            _ => FileSystemError::IOError { 
                message: format!("Failed to read directory {}: {}", dir.display(), e) 
            },
        })?;

    // Process entries in batches for better memory locality
    let mut batch_dirs = Vec::with_capacity(16);
    let mut batch_files = Vec::with_capacity(64);
    
    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(_) => continue, // Skip problematic entries
        };

        let path = entry.path();
        
        // Fast path check for .md extension before metadata call
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "md" {
                    batch_files.push(entry);
                }
            }
            // Skip non-markdown files immediately
        } else if path.is_dir() {
            batch_dirs.push(entry);
        }
        // Skip other types (symlinks, etc.)
    }

    // Process directories batch
    for entry in batch_dirs {
        if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
            files.push(file_info);
        }
        // Add to work queue for processing
        work_queue.push(entry.path());
    }

    // Process markdown files batch
    for entry in batch_files {
        if let Ok(file_info) = FileInfo::from_dir_entry(&entry) {
            files.push(file_info);
        }
    }
    
    Ok(())
}

/// Efficient in-place sorting optimized for typical file structures
fn sort_files_efficiently(files: &mut [FileInfo]) {
    // Use unstable sort for better performance (stable order not needed here)
    files.sort_unstable_by(|a, b| {
        // Fast path: directories vs files
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,   // Directories first
            (false, true) => std::cmp::Ordering::Greater, // Files second
            _ => {
                // Both same type - compare by name (case-insensitive)
                // Use faster comparison without allocation
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        }
    });
}

/// Internal validate vault function to check if a path is a valid vault
pub fn validate_vault_internal(vault_path: &str) -> FileSystemResult<bool> {
    time_operation!({
        let vault_path = Path::new(vault_path);
        
        // Check basic validity
        if !vault_path.exists() {
            return Ok(false);
        }
        
        if !vault_path.is_dir() {
            return Ok(false);
        }
        
        // Try to read the directory to check accessibility
        match fs::read_dir(vault_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Directory exists but not accessible
        }
    }, &format!("validate_vault({})", vault_path))
}

/// Internal load vault function that validates and scans vault files
pub fn load_vault_internal(vault_path: &str) -> FileSystemResult<Vec<FileInfo>> {
    time_operation!({
        let tracker = PerformanceTracker::start("load_vault");
        let vault_path = Path::new(vault_path);
        
        // First validate the vault
        validation::validate_path_exists(vault_path)?;
        validation::validate_is_directory(vault_path)?;
        
        tracker.checkpoint("validation_complete");
        
        // Check if we can access the directory
        match fs::read_dir(vault_path) {
            Ok(_) => {
                tracker.checkpoint("access_verified");
                
                // If validation passes, scan the files
                let files = scan_vault_files_internal(&vault_path.to_string_lossy())?;
                
                tracker.checkpoint("scanning_complete");
                let _duration = tracker.finish();
                
                Ok(files)
            },
            Err(e) => match e.kind() {
                std::io::ErrorKind::PermissionDenied => Err(FileSystemError::PermissionDenied { 
                    path: vault_path.to_string_lossy().to_string() 
                }),
                _ => Err(FileSystemError::IOError { 
                    message: format!("Failed to load vault {}: {}", vault_path.display(), e) 
                }),
            }
        }
    }, &format!("load_vault({})", vault_path))
}

/// Internal watch vault function for file system change notifications
/// 
/// This function now integrates with the enhanced file monitoring system that
/// automatically connects to the indexing pipeline for real-time updates.
pub fn watch_vault_internal(vault_path: &str) -> FileSystemResult<()> {
    time_operation!({
        let vault_path = Path::new(vault_path);
        
        // Validate vault path exists and is a directory
        validation::validate_path_exists(vault_path)?;
        validation::validate_is_directory(vault_path)?;

        // Use the enhanced file monitoring system with indexing integration
        // This will be called asynchronously since Tauri commands can be async
        // For now, we create a basic runtime to handle the async call
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                return Err(FileSystemError::IOError {
                    message: format!("Failed to create async runtime: {}", e)
                });
            }
        };
        
        rt.block_on(async {
            // Import the file monitor here to avoid circular dependencies
            match crate::file_monitor::get_file_monitor().start_watching(vault_path.to_str().unwrap()).await {
                Ok(()) => {
                    log::info!("✅ Enhanced file monitoring started for vault: {:?}", vault_path);
                    Ok(())
                }
                Err(e) => {
                    Err(FileSystemError::IOError {
                        message: format!("Failed to start enhanced file monitoring: {}", e)
                    })
                }
            }
        })
    }, &format!("watch_vault({})", vault_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
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

    #[test]
    fn test_scan_vault_files_empty_directory() {
        let env = TestEnv::new();

        let result = scan_vault_files_internal(&env.get_path());
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

        let result = scan_vault_files_internal(&env.get_path());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 2); // Only .md files should be included
        
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        assert_eq!(md_files.len(), 2);
        
        // Check that files are sorted alphabetically
        assert!(md_files[0].name <= md_files[1].name);
    }

    #[test]
    fn test_scan_vault_files_nested_directories() {
        let env = TestEnv::new();
        
        // Create nested structure
        env.create_directory_structure(&["subdir/deep"]).unwrap();
        env.create_test_file("root.md", "# Root note").unwrap();
        env.create_test_file("subdir/sub.md", "# Sub note").unwrap();
        env.create_test_file("subdir/deep/deep.md", "# Deep note").unwrap();

        let result = scan_vault_files_internal(&env.get_path());
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
    }

    #[test]
    fn test_scan_vault_files_nonexistent_path() {
        let result = scan_vault_files_internal("nonexistent_directory");
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_vault_files_file_instead_of_directory() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", "content").unwrap();

        let result = scan_vault_files_internal(&test_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_vault_files_chunked() {
        let env = TestEnv::new();
        
        // Create test files
        for i in 0..10 {
            env.create_test_file(&format!("note_{:02}.md", i), &format!("# Note {}", i)).unwrap();
        }

        // Test first chunk
        let result = scan_vault_files_chunked_internal(&env.get_path(), 0, 5);
        assert!(result.is_ok());
        
        let (chunk, has_more) = result.unwrap();
        assert_eq!(chunk.len(), 5);
        assert!(has_more);

        // Test second chunk
        let result = scan_vault_files_chunked_internal(&env.get_path(), 1, 5);
        assert!(result.is_ok());
        
        let (chunk, has_more) = result.unwrap();
        assert_eq!(chunk.len(), 5);
        assert!(!has_more);
    }

    #[test]
    fn test_scan_vault_files_performance() {
        let env = TestEnv::new();
        
        // Create files to test performance (reduced number for test stability)
        for i in 0..50 {
            env.create_test_file(&format!("note_{:03}.md", i), &format!("# Note {}", i)).unwrap();
        }

        let start = std::time::Instant::now();
        let result = scan_vault_files_internal(&env.get_path());
        let duration = start.elapsed();

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.iter().filter(|f| !f.is_dir).count(), 50);
        
        // Performance target: <200ms for 50 files (more generous for test environment)
        assert!(duration.as_millis() < 200, "Scanning took too long: {:?}", duration);
    }

    #[test]
    #[ignore] // Expensive test - run with --ignored
    fn test_scan_vault_files_large_scale_performance() {
        let env = TestEnv::new();
        
        println!("Creating large scale test vault with 1000+ files...");
        
        // Create directory structure
        env.create_directory_structure(&[
            "docs", "projects", "archive", "notes", "ideas",
            "docs/api", "docs/guides", "projects/web", "projects/mobile",
            "archive/2023", "archive/2024", "notes/daily", "notes/weekly"
        ]).unwrap();
        
        // Create 1000 markdown files across the directory structure
        let dirs = ["docs", "projects", "archive", "notes", "ideas", 
                   "docs/api", "docs/guides", "projects/web", "projects/mobile",
                   "archive/2023", "archive/2024", "notes/daily", "notes/weekly"];
        
        for i in 0..1000 {
            let dir = dirs[i % dirs.len()];
            let file_path = format!("{}/note_{:04}.md", dir, i);
            let content = format!("# Note {}\n\nThis is note {} with content.\n\n## Details\n\nSome more content here to make it realistic.", i, i);
            env.create_test_file(&file_path, &content).unwrap();
        }
        
        // Add some non-markdown files (should be ignored)
        for i in 0..100 {
            let dir = dirs[i % dirs.len()];
            let file_path = format!("{}/document_{:03}.txt", dir, i);
            env.create_test_file(&file_path, &format!("Document {}", i)).unwrap();
        }
        
        println!("Testing scan performance with 1000+ files...");
        let start = std::time::Instant::now();
        let result = scan_vault_files_internal(&env.get_path());
        let duration = start.elapsed();

        assert!(result.is_ok());
        let files = result.unwrap();
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
        let directories: Vec<_> = files.iter().filter(|f| f.is_dir).collect();
        
        println!("Performance Results:");
        println!("  • Scan time: {:.2}ms", duration.as_secs_f64() * 1000.0);
        println!("  • Markdown files found: {}", md_files.len());
        println!("  • Directories found: {}", directories.len());
        println!("  • Files per second: {:.0}", files.len() as f64 / duration.as_secs_f64());
        
        assert_eq!(md_files.len(), 1000);
        assert_eq!(directories.len(), 13); // All the created directories
        
        // Performance target: <100ms for 1000 files (issue requirement)
        let target_ms = 100;
        assert!(duration.as_millis() < target_ms, 
               "Scanning 1000+ files took {:.2}ms, exceeds target of {}ms", 
               duration.as_secs_f64() * 1000.0, target_ms);
        
        println!("✅ Large scale performance test passed!");
    }

    #[test]
    fn test_scan_vault_files_mixed_file_types() {
        let env = TestEnv::new();
        
        // Create various file types
        env.create_test_file("note.md", "# Markdown note").unwrap();
        env.create_test_file("document.txt", "Text document").unwrap();
        env.create_test_file("script.js", "console.log('hello')").unwrap();
        env.create_test_file("data.json", "{}").unwrap();
        env.create_test_file("README", "No extension").unwrap();

        let result = scan_vault_files_internal(&env.get_path());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        let file_files: Vec<_> = files.iter().filter(|f| !f.is_dir).collect();
        
        // Only the .md file should be included
        assert_eq!(file_files.len(), 1);
        assert_eq!(file_files[0].name, "note.md");
    }

    #[test]
    fn test_vault_scanning_comprehensive() {
        let env = TestEnv::new();
        
        // Create complex directory structure
        let dirs = vec![
            "folder1",
            "folder1/subfolder",
            "folder2",
            "folder2/deep/nested"
        ];
        
        env.create_directory_structure(&dirs).unwrap();

        // Create various files
        let files = vec![
            ("root.md", "# Root"),
            ("folder1/note1.md", "# Note 1"),
            ("folder1/subfolder/note2.md", "# Note 2"),
            ("folder2/note3.md", "# Note 3"),
            ("folder2/deep/nested/note4.md", "# Note 4"),
            ("folder1/readme.txt", "Not markdown"), // Should be ignored
            ("folder2/config.json", "{}"), // Should be ignored
        ];

        for (path, content) in &files {
            env.create_test_file(path, content).unwrap();
        }

        // Scan vault
        let result = scan_vault_files_internal(&env.get_path());
        assert!(result.is_ok());
        
        let scanned_files = result.unwrap();
        
        // Count different types
        let directories: Vec<_> = scanned_files.iter().filter(|f| f.is_dir).collect();
        let md_files: Vec<_> = scanned_files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
        
        assert_eq!(directories.len(), 5, "Expected 5 directories (including intermediate 'deep' directory)");
        assert_eq!(md_files.len(), 5, "Expected 5 markdown files");

        // Verify all markdown files are found
        let expected_md_files = vec!["root.md", "note1.md", "note2.md", "note3.md", "note4.md"];
        for expected in &expected_md_files {
            assert!(md_files.iter().any(|f| f.name == *expected), 
                   "Missing file: {}", expected);
        }
    }

    #[test]
    fn test_watch_vault_success() {
        let env = TestEnv::new();
        
        // Create a simple vault structure
        env.create_test_file("test.md", "# Test").unwrap();
        
        // Test that watch can be initiated
        let result = watch_vault_internal(&env.get_path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_watch_vault_nonexistent_directory() {
        let result = watch_vault_internal("nonexistent_directory");
        assert!(result.is_err());
    }

    #[test] 
    fn test_watch_vault_file_instead_of_directory() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", "content").unwrap();

        let result = watch_vault_internal(&test_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_vault_success() {
        let env = TestEnv::new();
        
        // Create a valid vault structure
        env.create_test_file("note1.md", "# Note 1").unwrap();
        env.create_test_file("note2.md", "# Note 2").unwrap();
        
        let result = validate_vault_internal(&env.get_path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_validate_vault_nonexistent() {
        let result = validate_vault_internal("nonexistent_directory");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_validate_vault_file_instead_of_directory() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", "content").unwrap();

        let result = validate_vault_internal(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_load_vault_success() {
        let env = TestEnv::new();
        
        // Create a valid vault structure
        env.create_directory_structure(&["notes", "projects"]).unwrap();
        env.create_test_file("notes/daily.md", "# Daily Notes").unwrap();
        env.create_test_file("projects/work.md", "# Work Notes").unwrap();
        env.create_test_file("readme.txt", "Not markdown").unwrap(); // Should be ignored
        
        let result = load_vault_internal(&env.get_path());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        let md_files: Vec<_> = files.iter().filter(|f| !f.is_dir && f.name.ends_with(".md")).collect();
        let directories: Vec<_> = files.iter().filter(|f| f.is_dir).collect();
        
        assert_eq!(md_files.len(), 2);
        assert_eq!(directories.len(), 2);
    }

    #[test]
    fn test_load_vault_nonexistent() {
        let result = load_vault_internal("nonexistent_directory");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_vault_file_instead_of_directory() {
        let env = TestEnv::new();
        let test_file = env.get_test_file("test.md");
        env.create_test_file("test.md", "content").unwrap();

        let result = load_vault_internal(&test_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_vault_empty_directory() {
        let env = TestEnv::new();
        
        let result = load_vault_internal(&env.get_path());
        assert!(result.is_ok());
        
        let files = result.unwrap();
        assert_eq!(files.len(), 0);
    }
}