//! Atomic File Operations for Vector Database
//! 
//! This module provides atomic write operations with file locking to ensure data integrity
//! during concurrent access. All write operations follow the write-to-temp-then-move pattern
//! to prevent corruption during system crashes or interruptions.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::time::sleep;

use crate::vector_db::types::{VectorDbError, VectorDbResult};

/// Errors specific to atomic file operations
#[derive(Error, Debug)]
pub enum AtomicError {
    #[error("Failed to acquire file lock after {timeout_ms}ms")]
    LockTimeout { timeout_ms: u64 },
    
    #[error("File lock already exists and appears stale: {lock_file}")]
    StaleLock { lock_file: String },
    
    #[error("Atomic operation interrupted: {reason}")]
    OperationInterrupted { reason: String },
    
    #[error("Temporary file creation failed: {path}")]
    TempFileCreation { path: String },
    
    #[error("File move operation failed: {from} -> {to}")]
    MoveOperation { from: String, to: String },
    
    #[error("IO error during atomic operation: {0}")]
    Io(#[from] io::Error),
}

impl From<AtomicError> for VectorDbError {
    fn from(error: AtomicError) -> Self {
        VectorDbError::Storage {
            message: format!("Atomic operation failed: {}", error),
        }
    }
}

/// Configuration for atomic file operations
#[derive(Debug, Clone)]
pub struct AtomicConfig {
    /// Maximum time to wait for file locks (milliseconds)
    pub lock_timeout_ms: u64,
    /// How often to check for lock availability (milliseconds) 
    pub lock_poll_interval_ms: u64,
    /// Consider locks stale after this time (seconds)
    pub lock_stale_timeout_secs: u64,
    /// Temporary file prefix
    pub temp_prefix: String,
    /// Temporary file suffix
    pub temp_suffix: String,
    /// Enable lock cleanup of stale locks
    pub cleanup_stale_locks: bool,
}

impl Default for AtomicConfig {
    fn default() -> Self {
        Self {
            lock_timeout_ms: 5000,           // 5 seconds max wait
            lock_poll_interval_ms: 50,       // Check every 50ms
            lock_stale_timeout_secs: 300,    // 5 minutes stale timeout
            temp_prefix: ".tmp_".to_string(),
            temp_suffix: ".tmp".to_string(),
            cleanup_stale_locks: true,
        }
    }
}

/// Atomic file writer that ensures safe, concurrent write operations
pub struct AtomicWriter {
    /// Target file path
    target_path: PathBuf,
    /// Temporary file path 
    temp_path: PathBuf,
    /// Lock file path
    lock_path: PathBuf,
    /// Configuration
    config: AtomicConfig,
    /// Lock acquisition start time
    lock_start: Option<Instant>,
}

impl AtomicWriter {
    /// Create a new atomic writer for the specified file
    pub fn new<P: AsRef<Path>>(target_path: P, config: AtomicConfig) -> Self {
        let target_path = target_path.as_ref().to_path_buf();
        let temp_path = Self::generate_temp_path(&target_path, &config);
        let lock_path = Self::generate_lock_path(&target_path);
        
        Self {
            target_path,
            temp_path,
            lock_path,
            config,
            lock_start: None,
        }
    }
    
    /// Create atomic writer with default configuration
    pub fn with_default_config<P: AsRef<Path>>(target_path: P) -> Self {
        Self::new(target_path, AtomicConfig::default())
    }
    
    /// Acquire exclusive lock for atomic write operation
    /// 
    /// This method blocks until a lock is acquired or timeout is reached.
    /// The lock is implemented using a lock file that contains the process ID
    /// and timestamp for stale lock detection.
    pub async fn acquire_lock(&mut self) -> VectorDbResult<()> {
        let start_time = Instant::now();
        self.lock_start = Some(start_time);
        
        loop {
            // Check if we've exceeded timeout
            if start_time.elapsed().as_millis() as u64 > self.config.lock_timeout_ms {
                return Err(AtomicError::LockTimeout {
                    timeout_ms: self.config.lock_timeout_ms
                }.into());
            }
            
            // Try to create lock file
            match self.try_create_lock().await {
                Ok(()) => {
                    eprintln!("🔒 Acquired atomic write lock: {}", self.lock_path.display());
                    return Ok(());
                }
                Err(AtomicError::StaleLock { .. }) if self.config.cleanup_stale_locks => {
                    // Try to clean up stale lock and retry
                    self.cleanup_stale_lock().await?;
                    continue;
                }
                Err(AtomicError::Io(e)) if e.kind() == io::ErrorKind::AlreadyExists => {
                    // Lock file exists, wait and retry
                    sleep(Duration::from_millis(self.config.lock_poll_interval_ms)).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    
    /// Write data atomically to the target file
    /// 
    /// This method:
    /// 1. Writes data to a temporary file
    /// 2. Syncs the temporary file to disk
    /// 3. Atomically moves the temporary file to the target location
    /// 4. Releases the lock
    pub async fn write_atomic(&self, data: &[u8]) -> VectorDbResult<()> {
        // Ensure we have a lock
        if !self.lock_path.exists() {
            return Err(VectorDbError::Storage {
                message: "Atomic write attempted without lock".to_string(),
            });
        }
        
        // Write to temporary file
        self.write_to_temp_file(data).await?;
        
        // Atomic move to final location
        self.move_temp_to_target().await?;
        
        eprintln!("📁 Atomic write completed: {} bytes to {}", 
                  data.len(), self.target_path.display());
        
        Ok(())
    }
    
    /// Release the acquired lock
    /// 
    /// This should always be called after write operations, even if they fail.
    /// It's safe to call multiple times.
    pub async fn release_lock(&self) -> VectorDbResult<()> {
        if self.lock_path.exists() {
            fs::remove_file(&self.lock_path).map_err(|e| VectorDbError::Storage {
                message: format!("Failed to release lock file: {}", e),
            })?;
            
            if let Some(start_time) = self.lock_start {
                let duration = start_time.elapsed();
                eprintln!("🔓 Released atomic write lock after {:.2}ms", 
                          duration.as_millis());
            }
        }
        
        // Clean up any leftover temporary files
        if self.temp_path.exists() {
            let _ = fs::remove_file(&self.temp_path);
        }
        
        Ok(())
    }
    
    /// Check if a lock exists for the target file
    pub fn has_lock(&self) -> bool {
        self.lock_path.exists()
    }
    
    /// Get the lock file age in seconds
    pub fn lock_age_secs(&self) -> Option<u64> {
        if let Ok(metadata) = fs::metadata(&self.lock_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = SystemTime::now().duration_since(modified) {
                    return Some(duration.as_secs());
                }
            }
        }
        None
    }
    
    /// Check if the current lock is stale based on configuration
    pub fn is_lock_stale(&self) -> bool {
        self.lock_age_secs()
            .map(|age| age > self.config.lock_stale_timeout_secs)
            .unwrap_or(false)
    }
    
    // Private helper methods
    
    /// Try to create a lock file
    async fn try_create_lock(&self) -> Result<(), AtomicError> {
        // Check for existing lock and if it's stale
        if self.lock_path.exists() {
            if self.is_lock_stale() {
                return Err(AtomicError::StaleLock {
                    lock_file: self.lock_path.to_string_lossy().to_string(),
                });
            } else {
                return Err(AtomicError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Lock file exists and is not stale"
                )));
            }
        }
        
        // Create lock file with process info
        let lock_content = self.create_lock_content();
        fs::write(&self.lock_path, lock_content)?;
        
        Ok(())
    }
    
    /// Create lock file content with process information
    fn create_lock_content(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let pid = std::process::id();
        
        format!("pid:{}\ntimestamp:{}\ntarget:{}\n", 
                pid, timestamp, self.target_path.display())
    }
    
    /// Clean up a stale lock file
    async fn cleanup_stale_lock(&self) -> Result<(), AtomicError> {
        if self.lock_path.exists() && self.is_lock_stale() {
            eprintln!("🧹 Cleaning up stale lock: {}", self.lock_path.display());
            fs::remove_file(&self.lock_path)?;
        }
        Ok(())
    }
    
    /// Write data to temporary file
    async fn write_to_temp_file(&self, data: &[u8]) -> VectorDbResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.temp_path.parent() {
            fs::create_dir_all(parent).map_err(|e| VectorDbError::Storage {
                message: format!("Failed to create temp directory: {}", e),
            })?;
        }
        
        // Write to temporary file
        let mut file = fs::File::create(&self.temp_path).map_err(|_e| {
            AtomicError::TempFileCreation {
                path: self.temp_path.to_string_lossy().to_string(),
            }
        })?;
        
        file.write_all(data).map_err(AtomicError::Io)?;
        file.sync_all().map_err(AtomicError::Io)?; // Ensure data is written to disk
        
        Ok(())
    }
    
    /// Atomically move temporary file to target location
    async fn move_temp_to_target(&self) -> VectorDbResult<()> {
        // Ensure target directory exists
        if let Some(parent) = self.target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| VectorDbError::Storage {
                message: format!("Failed to create target directory: {}", e),
            })?;
        }
        
        // Atomic move operation
        fs::rename(&self.temp_path, &self.target_path).map_err(|_e| {
            AtomicError::MoveOperation {
                from: self.temp_path.to_string_lossy().to_string(),
                to: self.target_path.to_string_lossy().to_string(),
            }
        })?;
        
        Ok(())
    }
    
    /// Generate temporary file path
    fn generate_temp_path(target_path: &Path, config: &AtomicConfig) -> PathBuf {
        let file_name = target_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        
        let temp_name = format!("{}{}{}", 
                               config.temp_prefix, 
                               file_name,
                               config.temp_suffix);
        
        target_path.with_file_name(temp_name)
    }
    
    /// Generate lock file path
    fn generate_lock_path(target_path: &Path) -> PathBuf {
        let lock_name = format!("{}.lock", 
                               target_path.file_name()
                                   .unwrap_or_default()
                                   .to_string_lossy());
        
        target_path.with_file_name(lock_name)
    }
}

/// RAII wrapper for atomic writer that ensures lock is always released
pub struct AtomicWriteGuard {
    writer: AtomicWriter,
    _locked: bool,
}

impl AtomicWriteGuard {
    /// Create a new atomic write guard and acquire lock
    pub async fn new<P: AsRef<Path>>(target_path: P) -> VectorDbResult<Self> {
        let mut writer = AtomicWriter::with_default_config(target_path);
        writer.acquire_lock().await?;
        
        Ok(Self {
            writer,
            _locked: true,
        })
    }
    
    /// Create guard with custom configuration
    pub async fn with_config<P: AsRef<Path>>(
        target_path: P, 
        config: AtomicConfig
    ) -> VectorDbResult<Self> {
        let mut writer = AtomicWriter::new(target_path, config);
        writer.acquire_lock().await?;
        
        Ok(Self {
            writer,
            _locked: true,
        })
    }
    
    /// Write data atomically
    pub async fn write(&self, data: &[u8]) -> VectorDbResult<()> {
        self.writer.write_atomic(data).await
    }
    
    /// Get reference to the underlying writer
    pub fn writer(&self) -> &AtomicWriter {
        &self.writer
    }
}

impl Drop for AtomicWriteGuard {
    fn drop(&mut self) {
        // Simple cleanup - just remove lock file directly
        // This avoids async runtime issues in Drop
        if self.writer.has_lock() {
            let _ = std::fs::remove_file(&self.writer.lock_path);
            if self.writer.temp_path.exists() {
                let _ = std::fs::remove_file(&self.writer.temp_path);
            }
        }
    }
}

/// Utility functions for atomic file operations
pub mod utils {
    use super::*;
    
    /// Perform a simple atomic write with default configuration
    pub async fn atomic_write<P: AsRef<Path>>(
        target_path: P, 
        data: &[u8]
    ) -> VectorDbResult<()> {
        let guard = AtomicWriteGuard::new(target_path).await?;
        guard.write(data).await?;
        // Lock is automatically released when guard drops
        Ok(())
    }
    
    /// Perform atomic write with custom timeout
    pub async fn atomic_write_with_timeout<P: AsRef<Path>>(
        target_path: P,
        data: &[u8],
        timeout_ms: u64,
    ) -> VectorDbResult<()> {
        let config = AtomicConfig {
            lock_timeout_ms: timeout_ms,
            ..AtomicConfig::default()
        };
        
        let guard = AtomicWriteGuard::with_config(target_path, config).await?;
        guard.write(data).await?;
        Ok(())
    }
    
    /// Check if a file has an active lock
    pub fn is_file_locked<P: AsRef<Path>>(file_path: P) -> bool {
        let writer = AtomicWriter::with_default_config(file_path);
        writer.has_lock()
    }
    
    /// Clean up any stale locks in a directory
    pub async fn cleanup_stale_locks_in_dir<P: AsRef<Path>>(
        dir_path: P
    ) -> VectorDbResult<usize> {
        let dir_path = dir_path.as_ref();
        let mut cleaned = 0;
        
        if !dir_path.exists() {
            return Ok(0);
        }
        
        let entries = fs::read_dir(dir_path).map_err(|e| VectorDbError::Storage {
            message: format!("Failed to read directory: {}", e),
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| VectorDbError::Storage {
                message: format!("Failed to read directory entry: {}", e),
            })?;
            
            let path = entry.path();
            if path.is_file() && 
               path.extension().and_then(|s| s.to_str()) == Some("lock") {
                
                let original_path = path.with_extension("");
                let writer = AtomicWriter::with_default_config(&original_path);
                if writer.is_lock_stale()
                    && fs::remove_file(&path).is_ok() {
                        cleaned += 1;
                        eprintln!("🧹 Cleaned stale lock: {}", path.display());
                    }
            }
        }
        
        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_atomic_config_default() {
        let config = AtomicConfig::default();
        assert_eq!(config.lock_timeout_ms, 5000);
        assert_eq!(config.lock_poll_interval_ms, 50);
        assert_eq!(config.lock_stale_timeout_secs, 300);
        assert_eq!(config.temp_prefix, ".tmp_");
        assert_eq!(config.temp_suffix, ".tmp");
        assert!(config.cleanup_stale_locks);
    }
    
    #[test]
    fn test_atomic_writer_paths() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_file.json");
        let config = AtomicConfig::default();
        
        let writer = AtomicWriter::new(&target_path, config);
        
        assert_eq!(writer.target_path, target_path);
        assert!(writer.temp_path.to_string_lossy().contains(".tmp_"));
        assert!(writer.temp_path.to_string_lossy().contains(".tmp"));
        assert!(writer.lock_path.to_string_lossy().contains(".lock"));
    }
    
    #[test]
    fn test_lock_content_format() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_file.json");
        let writer = AtomicWriter::with_default_config(&target_path);
        
        let content = writer.create_lock_content();
        assert!(content.contains("pid:"));
        assert!(content.contains("timestamp:"));
        assert!(content.contains("target:"));
        assert!(content.contains(&target_path.display().to_string()));
    }
    
    #[test] 
    fn test_path_generation() {
        let target_path = Path::new("/test/dir/file.json");
        let config = AtomicConfig::default();
        
        let temp_path = AtomicWriter::generate_temp_path(target_path, &config);
        let lock_path = AtomicWriter::generate_lock_path(target_path);
        
        assert!(temp_path.to_string_lossy().contains(".tmp_file.json.tmp"));
        assert!(lock_path.to_string_lossy().ends_with("file.json.lock"));
    }
    
    // Note: More comprehensive async integration tests will be in sub-issue #105
    // to avoid test hanging issues during development
}