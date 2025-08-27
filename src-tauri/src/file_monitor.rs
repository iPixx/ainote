//! # File System Monitoring Integration
//!
//! This module provides comprehensive file system monitoring that integrates seamlessly
//! with the indexing pipeline for real-time updates. It handles file change detection,
//! debouncing, and automatic indexing of modified files.
//!
//! ## Features
//!
//! - **Real-time Monitoring**: Detects file changes immediately using native OS APIs
//! - **Debounced Processing**: Prevents excessive indexing during rapid file changes
//! - **Markdown Filtering**: Only processes markdown files (.md) for efficiency
//! - **Integration**: Seamlessly connects to the indexing pipeline for automatic updates
//! - **Error Handling**: Robust error recovery and logging for file system events
//! - **Performance**: Minimal overhead monitoring suitable for large vaults
//!
//! ## Architecture
//!
//! The file monitor runs in a background thread and uses the `notify` crate for
//! cross-platform file system event detection. Events are filtered, debounced,
//! and forwarded to the indexing pipeline for processing.
//!
//! ## Usage
//!
//! ```rust
//! use crate::file_monitor::FileMonitor;
//!
//! let monitor = FileMonitor::new();
//! monitor.start_watching("/path/to/vault").await?;
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;
use once_cell::sync::Lazy;

use crate::commands::indexing_commands::INDEXING_PIPELINE;

/// Global file monitor instance for managing vault file system changes
/// 
/// This monitor provides a singleton interface for file system monitoring
/// across the entire application. It maintains active watchers and handles
/// event processing in a centralized manner.
pub static FILE_MONITOR: Lazy<Arc<FileMonitor>> = 
    Lazy::new(|| Arc::new(FileMonitor::new()));

/// File change event with metadata for processing
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path to the changed file
    pub file_path: PathBuf,
    /// Type of change (create, modify, delete)
    pub event_kind: FileEventKind,
    /// Timestamp when the event was detected
    pub timestamp: Instant,
}

/// Types of file system events we monitor
#[derive(Debug, Clone, PartialEq)]
pub enum FileEventKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted  
    Deleted,
    /// File was renamed
    Renamed,
}

/// Configuration for file monitoring behavior
#[derive(Debug, Clone)]
pub struct FileMonitorConfig {
    /// Debounce time in milliseconds for file changes (default: 1000ms)
    pub debounce_ms: u64,
    /// Whether to monitor subdirectories recursively (default: true)
    pub recursive: bool,
    /// File extensions to monitor (default: ["md"])
    pub monitored_extensions: Vec<String>,
    /// Whether to automatically start indexing on file changes (default: true)
    pub auto_index: bool,
}

impl Default for FileMonitorConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 1000,
            recursive: true,
            monitored_extensions: vec!["md".to_string()],
            auto_index: true,
        }
    }
}

/// File system monitor for real-time vault change detection
pub struct FileMonitor {
    /// Configuration for monitoring behavior
    config: FileMonitorConfig,
    /// Active file watchers by vault path
    watchers: Arc<Mutex<HashMap<PathBuf, RecommendedWatcher>>>,
    /// Pending file changes for debouncing
    pending_changes: Arc<Mutex<HashMap<PathBuf, FileChangeEvent>>>,
}

impl FileMonitor {
    /// Create a new file monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(FileMonitorConfig::default())
    }
    
    /// Create a new file monitor with custom configuration
    pub fn with_config(config: FileMonitorConfig) -> Self {
        Self {
            config,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            pending_changes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Start monitoring a vault directory for file changes
    /// 
    /// This method sets up file system monitoring for the specified vault directory
    /// and begins processing change events. It automatically integrates with the
    /// indexing pipeline for real-time updates.
    /// 
    /// # Arguments
    /// * `vault_path` - Path to the vault directory to monitor
    /// 
    /// # Returns
    /// * `Ok(())` - Monitoring started successfully
    /// * `Err(String)` - Error message describing the failure
    /// 
    /// # Example
    /// ```rust
    /// let monitor = FileMonitor::new();
    /// monitor.start_watching("/path/to/vault").await?;
    /// ```
    pub async fn start_watching(&self, vault_path: &str) -> Result<(), String> {
        let vault_path_buf = PathBuf::from(vault_path);
        
        log::info!("👁️ Starting file system monitoring for vault: {:?}", vault_path_buf);
        
        // Validate vault path
        if !vault_path_buf.exists() {
            return Err(format!("Vault path does not exist: {:?}", vault_path_buf));
        }
        
        if !vault_path_buf.is_dir() {
            return Err(format!("Vault path is not a directory: {:?}", vault_path_buf));
        }
        
        // Check if already monitoring this vault
        {
            let watchers = self.watchers.lock().unwrap();
            if watchers.contains_key(&vault_path_buf) {
                log::info!("ℹ️ Already monitoring vault: {:?}", vault_path_buf);
                return Ok(());
            }
        }
        
        // Set up event channel
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let vault_path_for_closure = vault_path_buf.clone();
        let monitored_extensions = self.config.monitored_extensions.clone();
        
        // Create watcher with event handler
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            match result {
                Ok(event) => {
                    // Filter events by kind and file extension
                    let event_kind = match event.kind {
                        EventKind::Create(_) => Some(FileEventKind::Created),
                        EventKind::Modify(_) => Some(FileEventKind::Modified),
                        EventKind::Remove(_) => Some(FileEventKind::Deleted),
                        _ => None, // Ignore other event types
                    };
                    
                    if let Some(kind) = event_kind {
                        for path in event.paths {
                            // Only process markdown files
                            if let Some(extension) = path.extension() {
                                let ext_str = extension.to_string_lossy().to_lowercase();
                                if monitored_extensions.contains(&ext_str) {
                                    let change_event = FileChangeEvent {
                                        file_path: path,
                                        event_kind: kind.clone(),
                                        timestamp: Instant::now(),
                                    };
                                    
                                    if let Err(e) = event_tx.send(change_event) {
                                        log::error!("❌ Failed to send file change event: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("❌ File system watch error: {}", e);
                }
            }
        }).map_err(|e| format!("Failed to create file watcher: {}", e))?;
        
        // Start watching the vault directory
        let recursive_mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        
        watcher.watch(&vault_path_buf, recursive_mode)
            .map_err(|e| format!("Failed to start watching vault: {}", e))?;
        
        // Store the watcher
        {
            let mut watchers = self.watchers.lock().unwrap();
            watchers.insert(vault_path_buf.clone(), watcher);
        }
        
        // Start event processing task
        let pending_changes = Arc::clone(&self.pending_changes);
        let debounce_ms = self.config.debounce_ms;
        let auto_index = self.config.auto_index;
        
        tokio::spawn(async move {
            log::debug!("🔄 Started file change event processor for vault: {:?}", vault_path_for_closure);
            
            while let Some(change_event) = event_rx.recv().await {
                log::debug!("📁 File change detected: {:?} ({:?})", 
                           change_event.file_path, change_event.event_kind);
                
                // Add to pending changes for debouncing
                {
                    let mut pending = pending_changes.lock().unwrap();
                    pending.insert(change_event.file_path.clone(), change_event);
                }
                
                // Process changes after debounce period
                if auto_index {
                    let pending_clone = Arc::clone(&pending_changes);
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(debounce_ms)).await;
                        Self::process_debounced_changes(pending_clone).await;
                    });
                }
            }
            
            log::debug!("🛑 File change event processor stopped for vault: {:?}", vault_path_for_closure);
        });
        
        log::info!("✅ File system monitoring started successfully for vault: {:?}", vault_path_buf);
        Ok(())
    }
    
    /// Stop monitoring a vault directory
    /// 
    /// This method stops file system monitoring for the specified vault directory
    /// and cleans up associated resources.
    /// 
    /// # Arguments
    /// * `vault_path` - Path to the vault directory to stop monitoring
    /// 
    /// # Returns
    /// * `Ok(())` - Monitoring stopped successfully
    /// * `Err(String)` - Error message if stopping fails
    pub async fn stop_watching(&self, vault_path: &str) -> Result<(), String> {
        let vault_path_buf = PathBuf::from(vault_path);
        
        log::info!("⏹️ Stopping file system monitoring for vault: {:?}", vault_path_buf);
        
        let mut watchers = self.watchers.lock().unwrap();
        if watchers.remove(&vault_path_buf).is_some() {
            log::info!("✅ Stopped monitoring vault: {:?}", vault_path_buf);
        } else {
            log::info!("ℹ️ Vault was not being monitored: {:?}", vault_path_buf);
        }
        
        Ok(())
    }
    
    /// Get list of currently monitored vault paths
    pub fn get_monitored_vaults(&self) -> Vec<PathBuf> {
        let watchers = self.watchers.lock().unwrap();
        watchers.keys().cloned().collect()
    }
    
    /// Check if a vault is currently being monitored
    pub fn is_monitoring(&self, vault_path: &str) -> bool {
        let vault_path_buf = PathBuf::from(vault_path);
        let watchers = self.watchers.lock().unwrap();
        watchers.contains_key(&vault_path_buf)
    }
    
    /// Process debounced file changes by sending them to the indexing pipeline
    async fn process_debounced_changes(pending_changes: Arc<Mutex<HashMap<PathBuf, FileChangeEvent>>>) {
        // Extract pending changes
        let changes: Vec<FileChangeEvent> = {
            let mut pending = pending_changes.lock().unwrap();
            let changes: Vec<_> = pending.values().cloned().collect();
            pending.clear();
            changes
        };
        
        if changes.is_empty() {
            return;
        }
        
        log::debug!("🔄 Processing {} debounced file changes", changes.len());
        
        // Filter out deleted files and collect paths for indexing
        let mut files_to_index = Vec::new();
        for change in changes {
            match change.event_kind {
                FileEventKind::Created | FileEventKind::Modified | FileEventKind::Renamed => {
                    // Only index files that still exist
                    if change.file_path.exists() && change.file_path.is_file() {
                        files_to_index.push(change.file_path.to_string_lossy().to_string());
                        log::debug!("📋 Queuing file for indexing: {:?}", change.file_path);
                    }
                }
                FileEventKind::Deleted => {
                    // For deleted files, we might want to remove from vector database
                    // This is handled elsewhere in the system
                    log::debug!("🗑️ File deleted: {:?}", change.file_path);
                }
            }
        }
        
        // Send to indexing pipeline if there are files to process
        if !files_to_index.is_empty() {
            // Access the global indexing pipeline
            let pipeline_lock = INDEXING_PIPELINE.read().await;
            if let Some(pipeline) = pipeline_lock.as_ref() {
                if pipeline.is_running() {
                    // Convert string paths to PathBuf
                    let path_bufs: Vec<PathBuf> = files_to_index.into_iter()
                        .map(PathBuf::from)
                        .collect();
                    
                    match pipeline.index_files_debounced(path_bufs, Some(100)).await {
                        Ok(request_ids) => {
                            log::info!("✅ Queued {} files for real-time indexing", request_ids.len());
                        }
                        Err(e) => {
                            log::error!("❌ Failed to queue files for indexing: {}", e);
                        }
                    }
                } else {
                    log::warn!("⚠️ Indexing pipeline not running, skipping file change processing");
                }
            } else {
                log::warn!("⚠️ Indexing pipeline not initialized, skipping file change processing");
            }
        }
    }
}

impl Default for FileMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to get the global file monitor instance
pub fn get_file_monitor() -> Arc<FileMonitor> {
    Arc::clone(&FILE_MONITOR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_monitor_creation() {
        let monitor = FileMonitor::new();
        assert_eq!(monitor.config.debounce_ms, 1000);
        assert!(monitor.config.recursive);
        assert!(monitor.config.auto_index);
        assert_eq!(monitor.config.monitored_extensions, vec!["md"]);
    }

    #[test]
    fn test_file_monitor_custom_config() {
        let config = FileMonitorConfig {
            debounce_ms: 500,
            recursive: false,
            monitored_extensions: vec!["md".to_string(), "txt".to_string()],
            auto_index: false,
        };
        
        let monitor = FileMonitor::with_config(config.clone());
        assert_eq!(monitor.config.debounce_ms, 500);
        assert!(!monitor.config.recursive);
        assert!(!monitor.config.auto_index);
        assert_eq!(monitor.config.monitored_extensions.len(), 2);
    }

    #[tokio::test]
    async fn test_file_monitor_invalid_path() {
        let monitor = FileMonitor::new();
        let result = monitor.start_watching("/nonexistent/path").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_file_monitor_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().to_string_lossy().to_string();
        
        let monitor = FileMonitor::new();
        
        // Should not be monitoring initially
        assert!(!monitor.is_monitoring(&vault_path));
        
        // Start monitoring
        monitor.start_watching(&vault_path).await.unwrap();
        assert!(monitor.is_monitoring(&vault_path));
        
        // Should be in monitored vaults list
        let monitored = monitor.get_monitored_vaults();
        assert_eq!(monitored.len(), 1);
        assert_eq!(monitored[0], PathBuf::from(&vault_path));
        
        // Stop monitoring
        monitor.stop_watching(&vault_path).await.unwrap();
        assert!(!monitor.is_monitoring(&vault_path));
    }

    #[test]
    fn test_file_event_kind() {
        let event = FileChangeEvent {
            file_path: PathBuf::from("/test.md"),
            event_kind: FileEventKind::Created,
            timestamp: Instant::now(),
        };
        
        assert_eq!(event.event_kind, FileEventKind::Created);
        assert_eq!(event.file_path, PathBuf::from("/test.md"));
    }
}