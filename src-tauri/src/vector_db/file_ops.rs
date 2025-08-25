//! File Operations for Vector Database
//! 
//! This module provides comprehensive file system operations for the vector database,
//! including initialization, cleanup, backup/recovery, and safe concurrent access.
//! All operations are designed to maintain data integrity and handle failure scenarios.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as async_fs;
use serde::{Deserialize, Serialize};

use crate::vector_db::types::{
    VectorStorageConfig, VectorDbError, VectorDbResult,
};
use crate::vector_db::atomic::utils as atomic_utils;

/// File system operations manager for vector database
pub struct FileOperations {
    /// Storage configuration
    config: VectorStorageConfig,
    /// Base storage directory
    storage_dir: PathBuf,
    /// Backup directory
    backup_dir: PathBuf,
    /// Temporary directory for operations
    temp_dir: PathBuf,
}

/// Database initialization status
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializationStatus {
    /// Whether the database is properly initialized
    pub is_initialized: bool,
    /// Storage directory path
    pub storage_dir: String,
    /// Number of existing storage files
    pub existing_files: usize,
    /// Estimated total entries (from file headers)
    pub estimated_entries: usize,
    /// Any initialization warnings or issues
    pub warnings: Vec<String>,
    /// Initialization timestamp
    pub initialized_at: u64,
}

/// Cleanup operation result
#[derive(Debug, Default)]
pub struct CleanupResult {
    /// Number of temporary files removed
    pub temp_files_removed: usize,
    /// Number of orphaned lock files removed
    pub lock_files_removed: usize,
    /// Number of old backup files removed
    pub backup_files_removed: usize,
    /// Total disk space reclaimed (bytes)
    pub bytes_reclaimed: usize,
    /// Cleanup operation warnings
    pub warnings: Vec<String>,
}

/// Backup operation result
#[derive(Debug)]
pub struct BackupResult {
    /// Path to created backup
    pub backup_path: PathBuf,
    /// Number of files backed up
    pub files_backed_up: usize,
    /// Total backup size in bytes
    pub backup_size: usize,
    /// Backup creation timestamp
    pub created_at: u64,
    /// Backup checksum (if enabled)
    pub checksum: Option<String>,
}

/// Recovery operation result
#[derive(Debug)]
pub struct RecoveryResult {
    /// Whether recovery was successful
    pub success: bool,
    /// Number of files recovered
    pub files_recovered: usize,
    /// Recovery method used
    pub recovery_method: String,
    /// Recovery warnings
    pub warnings: Vec<String>,
}

impl FileOperations {
    /// Create a new file operations manager
    pub fn new(config: VectorStorageConfig) -> VectorDbResult<Self> {
        let storage_dir = PathBuf::from(&config.storage_dir);
        let backup_dir = storage_dir.join("backups");
        let temp_dir = storage_dir.join("temp");
        
        Ok(Self {
            config,
            storage_dir,
            backup_dir,
            temp_dir,
        })
    }
    
    /// Initialize the database file structure
    /// 
    /// This method creates necessary directories, validates existing files,
    /// and performs any required migrations or cleanup.
    pub async fn initialize_database(&self) -> VectorDbResult<InitializationStatus> {
        let mut status = InitializationStatus {
            is_initialized: false,
            storage_dir: self.storage_dir.to_string_lossy().to_string(),
            existing_files: 0,
            estimated_entries: 0,
            warnings: Vec::new(),
            initialized_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        // Create base directories
        self.create_directories().await?;
        
        // Scan existing files
        let (file_count, entry_estimate) = self.scan_existing_files().await?;
        status.existing_files = file_count;
        status.estimated_entries = entry_estimate;
        
        // Clean up any stale locks or temporary files
        let cleanup_result = self.cleanup_stale_files().await?;
        if !cleanup_result.warnings.is_empty() {
            status.warnings.extend(cleanup_result.warnings);
        }
        
        // Validate storage integrity
        if let Err(e) = self.validate_storage_structure().await {
            status.warnings.push(format!("Storage validation warning: {}", e));
        }
        
        // Create initialization marker file
        self.create_initialization_marker(&status).await?;
        
        status.is_initialized = true;
        
        eprintln!("✅ Vector database initialized: {} existing files, ~{} entries", 
                  status.existing_files, status.estimated_entries);
        
        Ok(status)
    }
    
    /// Clean up temporary files, stale locks, and old backups
    pub async fn cleanup_stale_files(&self) -> VectorDbResult<CleanupResult> {
        let mut result = CleanupResult::default();
        
        // Clean up temporary files
        result.temp_files_removed = self.cleanup_temp_files().await?;
        
        // Clean up stale lock files
        result.lock_files_removed = atomic_utils::cleanup_stale_locks_in_dir(&self.storage_dir).await?;
        
        // Clean up old backup files (if enabled)
        if self.config.auto_backup && self.config.max_backups > 0 {
            let (removed, bytes_reclaimed) = self.cleanup_old_backups().await?;
            result.backup_files_removed = removed;
            result.bytes_reclaimed += bytes_reclaimed;
        }
        
        eprintln!("🧹 Cleanup completed: {} temp files, {} locks, {} backups removed", 
                  result.temp_files_removed,
                  result.lock_files_removed, 
                  result.backup_files_removed);
        
        Ok(result)
    }
    
    /// Create a full backup of the vector database
    pub async fn create_backup(&self) -> VectorDbResult<BackupResult> {
        if !self.config.auto_backup {
            return Err(VectorDbError::Storage {
                message: "Backup is disabled in configuration".to_string(),
            });
        }
        
        // Create backup directory
        if !self.backup_dir.exists() {
            async_fs::create_dir_all(&self.backup_dir).await.map_err(|e| {
                VectorDbError::Storage {
                    message: format!("Failed to create backup directory: {}", e),
                }
            })?;
        }
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let backup_name = format!("vector_db_backup_{}.tar", timestamp);
        let _backup_path = self.backup_dir.join(&backup_name);
        
        // For now, implement a simple file copying backup
        // TODO: Implement proper tar archive creation
        let backup_dir_path = self.backup_dir.join(format!("backup_{}", timestamp));
        async_fs::create_dir_all(&backup_dir_path).await.map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to create backup subdirectory: {}", e),
            }
        })?;
        
        let mut files_backed_up = 0;
        let mut total_size = 0;
        
        // Copy storage files to backup directory
        if self.storage_dir.exists() {
            let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
                VectorDbError::Storage {
                    message: format!("Failed to read storage directory: {}", e),
                }
            })?;
            
            for entry in entries {
                let entry = entry.map_err(|e| VectorDbError::Storage {
                    message: format!("Failed to read directory entry: {}", e),
                })?;
                
                let path = entry.path();
                if path.is_file() && self.is_storage_file(&path) {
                    let file_name = path.file_name().unwrap_or_default();
                    let backup_file_path = backup_dir_path.join(file_name);
                    
                    async_fs::copy(&path, &backup_file_path).await.map_err(|e| {
                        VectorDbError::Storage {
                            message: format!("Failed to copy file to backup: {}", e),
                        }
                    })?;
                    
                    if let Ok(metadata) = fs::metadata(&path) {
                        total_size += metadata.len() as usize;
                    }
                    
                    files_backed_up += 1;
                }
            }
        }
        
        // Create backup metadata
        let backup_result = BackupResult {
            backup_path: backup_dir_path,
            files_backed_up,
            backup_size: total_size,
            created_at: timestamp,
            checksum: None, // TODO: Implement checksum generation
        };
        
        eprintln!("💾 Backup created: {} files, {} bytes at {}", 
                  backup_result.files_backed_up,
                  backup_result.backup_size,
                  backup_result.backup_path.display());
        
        Ok(backup_result)
    }
    
    /// Recover from backup or attempt automatic recovery
    pub async fn recover_from_backup(&self, backup_path: Option<PathBuf>) -> VectorDbResult<RecoveryResult> {
        let mut result = RecoveryResult {
            success: false,
            files_recovered: 0,
            recovery_method: "none".to_string(),
            warnings: Vec::new(),
        };
        
        // Attempt recovery from specified backup
        if let Some(backup_path) = backup_path {
            if backup_path.exists() {
                result = self.recover_from_backup_directory(&backup_path).await?;
                result.recovery_method = "explicit_backup".to_string();
            } else {
                result.warnings.push(format!("Specified backup path does not exist: {}", 
                                           backup_path.display()));
            }
        }
        
        // If no explicit backup or recovery failed, try automatic recovery
        if !result.success {
            result = self.attempt_automatic_recovery().await?;
            result.recovery_method = "automatic".to_string();
        }
        
        eprintln!("🔧 Recovery attempted: {} (success: {}, {} files recovered)", 
                  result.recovery_method, result.success, result.files_recovered);
        
        Ok(result)
    }
    
    /// Get comprehensive file system metrics
    pub async fn get_file_metrics(&self) -> VectorDbResult<FileSystemMetrics> {
        let mut metrics = FileSystemMetrics::default();
        
        // Storage directory metrics
        if self.storage_dir.exists() {
            let (files, total_size) = self.calculate_directory_metrics(&self.storage_dir).await?;
            metrics.storage_files = files;
            metrics.storage_size_bytes = total_size;
        }
        
        // Backup directory metrics
        if self.backup_dir.exists() {
            let (files, total_size) = self.calculate_directory_metrics(&self.backup_dir).await?;
            metrics.backup_files = files;
            metrics.backup_size_bytes = total_size;
        }
        
        // Temporary directory metrics
        if self.temp_dir.exists() {
            let (files, total_size) = self.calculate_directory_metrics(&self.temp_dir).await?;
            metrics.temp_files = files;
            metrics.temp_size_bytes = total_size;
        }
        
        // Lock file count
        metrics.active_locks = self.count_lock_files().await?;
        
        metrics.total_size_bytes = metrics.storage_size_bytes + 
                                  metrics.backup_size_bytes + 
                                  metrics.temp_size_bytes;
        
        Ok(metrics)
    }
    
    /// Safely write data to a storage file with atomic operations
    pub async fn write_storage_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        data: &[u8],
    ) -> VectorDbResult<()> {
        let file_path = file_path.as_ref();
        
        // Ensure storage directory exists
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                async_fs::create_dir_all(parent).await.map_err(|e| {
                    VectorDbError::Storage {
                        message: format!("Failed to create storage directory: {}", e),
                    }
                })?;
            }
        }
        
        // Use atomic write operations
        atomic_utils::atomic_write(file_path, data).await?;
        
        // Create backup if enabled
        if self.config.auto_backup {
            let _ = self.create_file_backup(file_path).await;
        }
        
        Ok(())
    }
    
    /// Safely read data from a storage file
    pub async fn read_storage_file<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> VectorDbResult<Vec<u8>> {
        let file_path = file_path.as_ref();
        
        if !file_path.exists() {
            return Err(VectorDbError::Storage {
                message: format!("Storage file does not exist: {}", file_path.display()),
            });
        }
        
        // Check if file is locked
        if atomic_utils::is_file_locked(file_path) {
            return Err(VectorDbError::Storage {
                message: format!("Storage file is locked: {}", file_path.display()),
            });
        }
        
        // Read file contents
        async_fs::read(file_path).await.map_err(|e| VectorDbError::Storage {
            message: format!("Failed to read storage file: {}", e),
        })
    }
    
    /// Get storage configuration
    pub fn get_config(&self) -> &VectorStorageConfig {
        &self.config
    }
    
    /// Update storage configuration
    pub fn update_config(&mut self, new_config: VectorStorageConfig) {
        self.config = new_config;
        // Update derived paths if storage directory changed
        self.storage_dir = PathBuf::from(&self.config.storage_dir);
        self.backup_dir = self.storage_dir.join("backups");
        self.temp_dir = self.storage_dir.join("temp");
    }
    
    // Private helper methods
    
    /// Create necessary directories
    async fn create_directories(&self) -> VectorDbResult<()> {
        for dir in &[&self.storage_dir, &self.backup_dir, &self.temp_dir] {
            if !dir.exists() {
                async_fs::create_dir_all(dir).await.map_err(|e| {
                    VectorDbError::Storage {
                        message: format!("Failed to create directory {}: {}", dir.display(), e),
                    }
                })?;
            }
        }
        Ok(())
    }
    
    /// Scan existing storage files and estimate entry count
    async fn scan_existing_files(&self) -> VectorDbResult<(usize, usize)> {
        if !self.storage_dir.exists() {
            return Ok((0, 0));
        }
        
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to read storage directory: {}", e),
            }
        })?;
        
        let mut file_count = 0;
        let mut estimated_entries = 0;
        
        for entry in entries {
            let entry = entry.map_err(|e| VectorDbError::Storage {
                message: format!("Failed to read directory entry: {}", e),
            })?;
            
            let path = entry.path();
            if path.is_file() && self.is_storage_file(&path) {
                file_count += 1;
                
                // Try to read header to get entry count estimate
                if let Ok(entry_count) = self.read_file_entry_count(&path).await {
                    estimated_entries += entry_count;
                }
            }
        }
        
        Ok((file_count, estimated_entries))
    }
    
    /// Read entry count from storage file header
    async fn read_file_entry_count(&self, file_path: &Path) -> VectorDbResult<usize> {
        // This is a simplified version - in practice would need to parse the full file
        // For now, estimate based on file size
        if let Ok(metadata) = fs::metadata(file_path) {
            // Rough estimate: 1KB per entry on average
            let estimated_entries = (metadata.len() / 1024).max(1) as usize;
            Ok(estimated_entries)
        } else {
            Ok(0)
        }
    }
    
    /// Check if path is a storage file
    fn is_storage_file(&self, path: &Path) -> bool {
        if let Some(file_name) = path.file_name() {
            let file_str = file_name.to_string_lossy();
            file_str.starts_with("vector_") && 
            (file_str.ends_with(".json") || 
             file_str.ends_with(".json.gz") ||
             file_str.ends_with(".json.lz4"))
        } else {
            false
        }
    }
    
    /// Validate basic storage structure
    async fn validate_storage_structure(&self) -> VectorDbResult<()> {
        // Check that storage directory is readable/writable
        let test_file = self.storage_dir.join(".write_test");
        async_fs::write(&test_file, b"test").await.map_err(|e| {
            VectorDbError::Storage {
                message: format!("Storage directory is not writable: {}", e),
            }
        })?;
        
        let _ = async_fs::remove_file(&test_file).await;
        Ok(())
    }
    
    /// Create initialization marker file
    async fn create_initialization_marker(&self, status: &InitializationStatus) -> VectorDbResult<()> {
        let marker_path = self.storage_dir.join(".ainote_initialized");
        let marker_content = serde_json::to_string_pretty(status)
            .map_err(VectorDbError::Serialization)?;
        
        async_fs::write(&marker_path, marker_content).await.map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to create initialization marker: {}", e),
            }
        })?;
        
        Ok(())
    }
    
    /// Clean up temporary files
    async fn cleanup_temp_files(&self) -> VectorDbResult<usize> {
        if !self.temp_dir.exists() {
            return Ok(0);
        }
        
        let entries = fs::read_dir(&self.temp_dir).map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to read temp directory: {}", e),
            }
        })?;
        
        let mut removed = 0;
        for entry in entries {
            let entry = entry.map_err(|e| VectorDbError::Storage {
                message: format!("Failed to read temp entry: {}", e),
            })?;
            
            let path = entry.path();
            if path.is_file()
                && async_fs::remove_file(&path).await.is_ok() {
                    removed += 1;
                }
        }
        
        Ok(removed)
    }
    
    /// Clean up old backup files
    async fn cleanup_old_backups(&self) -> VectorDbResult<(usize, usize)> {
        if !self.backup_dir.exists() {
            return Ok((0, 0));
        }
        
        let entries = fs::read_dir(&self.backup_dir).map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to read backup directory: {}", e),
            }
        })?;
        
        let mut backup_files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .collect();
        
        // Sort by creation time (newest first)
        backup_files.sort_by(|a, b| {
            let time_a = a.metadata().and_then(|m| m.created()).unwrap_or(SystemTime::UNIX_EPOCH);
            let time_b = b.metadata().and_then(|m| m.created()).unwrap_or(SystemTime::UNIX_EPOCH);
            time_b.cmp(&time_a)
        });
        
        let mut removed = 0;
        let mut bytes_reclaimed = 0;
        
        // Remove excess backups beyond max_backups limit
        for backup in backup_files.iter().skip(self.config.max_backups) {
            let path = backup.path();
            
            // Calculate directory size before removal
            if let Ok((_, size)) = self.calculate_directory_metrics(&path).await {
                bytes_reclaimed += size;
            }
            
            if async_fs::remove_dir_all(&path).await.is_ok() {
                removed += 1;
            }
        }
        
        Ok((removed, bytes_reclaimed))
    }
    
    /// Recover from backup directory
    async fn recover_from_backup_directory(&self, backup_path: &Path) -> VectorDbResult<RecoveryResult> {
        let mut result = RecoveryResult {
            success: false,
            files_recovered: 0,
            recovery_method: "backup_directory".to_string(),
            warnings: Vec::new(),
        };
        
        if !backup_path.exists() || !backup_path.is_dir() {
            result.warnings.push(format!("Backup path is not a valid directory: {}", 
                                        backup_path.display()));
            return Ok(result);
        }
        
        // Copy backup files to storage directory
        let entries = fs::read_dir(backup_path).map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to read backup directory: {}", e),
            }
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| VectorDbError::Storage {
                message: format!("Failed to read backup entry: {}", e),
            })?;
            
            let path = entry.path();
            if path.is_file() && self.is_storage_file(&path) {
                let file_name = path.file_name().unwrap_or_default();
                let target_path = self.storage_dir.join(file_name);
                
                if async_fs::copy(&path, &target_path).await.is_ok() {
                    result.files_recovered += 1;
                } else {
                    result.warnings.push(format!("Failed to recover file: {}", 
                                                file_name.to_string_lossy()));
                }
            }
        }
        
        result.success = result.files_recovered > 0;
        Ok(result)
    }
    
    /// Attempt automatic recovery from various sources
    async fn attempt_automatic_recovery(&self) -> VectorDbResult<RecoveryResult> {
        let mut result = RecoveryResult {
            success: false,
            files_recovered: 0,
            recovery_method: "automatic".to_string(),
            warnings: Vec::new(),
        };
        
        // Try to recover from most recent backup
        if self.backup_dir.exists() {
            let entries = fs::read_dir(&self.backup_dir).map_err(|e| {
                VectorDbError::Storage {
                    message: format!("Failed to read backup directory: {}", e),
                }
            })?;
            
            let mut backup_dirs: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .collect();
            
            // Sort by creation time (newest first)
            backup_dirs.sort_by(|a, b| {
                let time_a = a.metadata().and_then(|m| m.created()).unwrap_or(SystemTime::UNIX_EPOCH);
                let time_b = b.metadata().and_then(|m| m.created()).unwrap_or(SystemTime::UNIX_EPOCH);
                time_b.cmp(&time_a)
            });
            
            if let Some(latest_backup) = backup_dirs.first() {
                result = self.recover_from_backup_directory(&latest_backup.path()).await?;
                if result.success {
                    result.recovery_method = "latest_backup".to_string();
                    return Ok(result);
                }
            }
        }
        
        result.warnings.push("No valid backups found for automatic recovery".to_string());
        Ok(result)
    }
    
    /// Calculate directory metrics (file count and total size)
    async fn calculate_directory_metrics(&self, dir_path: &Path) -> VectorDbResult<(usize, usize)> {
        if !dir_path.exists() {
            return Ok((0, 0));
        }
        
        let entries = fs::read_dir(dir_path).map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to read directory: {}", e),
            }
        })?;
        
        let mut file_count = 0;
        let mut total_size = 0;
        
        for entry in entries {
            let entry = entry.map_err(|e| VectorDbError::Storage {
                message: format!("Failed to read directory entry: {}", e),
            })?;
            
            let path = entry.path();
            if path.is_file() {
                file_count += 1;
                if let Ok(metadata) = fs::metadata(&path) {
                    total_size += metadata.len() as usize;
                }
            }
        }
        
        Ok((file_count, total_size))
    }
    
    /// Count active lock files
    async fn count_lock_files(&self) -> VectorDbResult<usize> {
        if !self.storage_dir.exists() {
            return Ok(0);
        }
        
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            VectorDbError::Storage {
                message: format!("Failed to read storage directory: {}", e),
            }
        })?;
        
        let count = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().is_file() && 
                entry.path().extension().and_then(|s| s.to_str()) == Some("lock")
            })
            .count();
        
        Ok(count)
    }
    
    /// Create backup of specific file
    async fn create_file_backup(&self, file_path: &Path) -> VectorDbResult<()> {
        if let Some(file_name) = file_path.file_name() {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let backup_name = format!("{}_{}.backup", 
                                    timestamp, 
                                    file_name.to_string_lossy());
            let backup_path = self.backup_dir.join(backup_name);
            
            async_fs::copy(file_path, &backup_path).await.map_err(|e| {
                VectorDbError::Storage {
                    message: format!("Failed to create file backup: {}", e),
                }
            })?;
        }
        
        Ok(())
    }
}

/// File system metrics for the vector database
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileSystemMetrics {
    /// Number of storage files
    pub storage_files: usize,
    /// Total storage file size in bytes
    pub storage_size_bytes: usize,
    /// Number of backup files
    pub backup_files: usize,
    /// Total backup size in bytes
    pub backup_size_bytes: usize,
    /// Number of temporary files
    pub temp_files: usize,
    /// Total temporary file size in bytes
    pub temp_size_bytes: usize,
    /// Number of active file locks
    pub active_locks: usize,
    /// Total size across all files
    pub total_size_bytes: usize,
}

impl FileSystemMetrics {
    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "FileSystem: {} storage files ({:.1} MB), {} backups ({:.1} MB), {} temp files, {} active locks",
            self.storage_files,
            self.storage_size_bytes as f64 / (1024.0 * 1024.0),
            self.backup_files,
            self.backup_size_bytes as f64 / (1024.0 * 1024.0),
            self.temp_files,
            self.active_locks
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_config(storage_dir: &str) -> VectorStorageConfig {
        use crate::vector_db::types::CompressionAlgorithm;
        
        VectorStorageConfig {
            storage_dir: storage_dir.to_string(),
            enable_compression: false,
            compression_algorithm: CompressionAlgorithm::None,
            max_entries_per_file: 100,
            enable_checksums: false,
            auto_backup: true,
            max_backups: 3,
            enable_metrics: false,
        }
    }
    
    #[test]
    fn test_file_operations_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir.path().to_string_lossy());
        
        let file_ops = FileOperations::new(config).unwrap();
        assert!(file_ops.storage_dir.to_string_lossy().contains(temp_dir.path().to_str().unwrap()));
        assert!(file_ops.backup_dir.to_string_lossy().contains("backups"));
        assert!(file_ops.temp_dir.to_string_lossy().contains("temp"));
    }
    
    #[test]
    fn test_is_storage_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir.path().to_string_lossy());
        let file_ops = FileOperations::new(config).unwrap();
        
        assert!(file_ops.is_storage_file(Path::new("vector_123.json")));
        assert!(file_ops.is_storage_file(Path::new("vector_456.json.gz")));
        assert!(file_ops.is_storage_file(Path::new("vector_789.json.lz4")));
        assert!(!file_ops.is_storage_file(Path::new("other_file.json")));
        assert!(!file_ops.is_storage_file(Path::new("vector_file.txt")));
    }
    
    #[test]
    fn test_initialization_status() {
        let status = InitializationStatus {
            is_initialized: true,
            storage_dir: "/test/storage".to_string(),
            existing_files: 5,
            estimated_entries: 100,
            warnings: vec!["Test warning".to_string()],
            initialized_at: 1234567890,
        };
        
        assert!(status.is_initialized);
        assert_eq!(status.existing_files, 5);
        assert_eq!(status.estimated_entries, 100);
        assert_eq!(status.warnings.len(), 1);
    }
    
    #[test]
    fn test_cleanup_result() {
        let mut result = CleanupResult::default();
        assert_eq!(result.temp_files_removed, 0);
        assert_eq!(result.lock_files_removed, 0);
        assert_eq!(result.backup_files_removed, 0);
        assert_eq!(result.bytes_reclaimed, 0);
        assert!(result.warnings.is_empty());
        
        result.warnings.push("Test warning".to_string());
        assert_eq!(result.warnings.len(), 1);
    }
    
    #[test]
    fn test_file_system_metrics() {
        let metrics = FileSystemMetrics {
            storage_files: 10,
            storage_size_bytes: 1024 * 1024, // 1 MB
            backup_files: 3,
            backup_size_bytes: 512 * 1024,   // 0.5 MB
            temp_files: 2,
            temp_size_bytes: 1024,
            active_locks: 1,
            total_size_bytes: 1024 * 1024 + 512 * 1024 + 1024,
        };
        
        let summary = metrics.summary();
        assert!(summary.contains("10 storage files"));
        assert!(summary.contains("3 backups"));
        assert!(summary.contains("1 active locks"));
    }
    
    // Note: Full async integration tests will be in sub-issue #105
    // to avoid hanging issues during development
}