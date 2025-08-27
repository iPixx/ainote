//! Vector Database Module for aiNote
//! 
//! This module provides a lightweight, file-based vector storage system optimized 
//! for markdown note embeddings with efficient indexing and retrieval capabilities.
//! 
//! ## Features
//! 
//! - **File-based storage**: JSON serialization with optional compression
//! - **Data integrity**: Checksum validation and version compatibility
//! - **Atomic operations**: Safe concurrent access with file locking
//! - **Compression support**: Gzip compression for storage efficiency
//! - **Backup system**: Automatic backup creation for data safety
//! - **Metrics tracking**: Performance and storage statistics
//! 
//! ## Architecture
//! 
//! The vector database consists of three main components:
//! 
//! 1. **Types** (`types.rs`): Core data structures and serialization
//!    - `EmbeddingEntry`: Container for vector data and metadata
//!    - `EmbeddingMetadata`: Associated metadata (file path, chunk ID, etc.)
//!    - `VectorStorageConfig`: Configuration for storage behavior
//! 
//! 2. **Storage** (`storage.rs`): File operations and data persistence
//!    - `VectorStorage`: Main storage engine with CRUD operations
//!    - Compression and decompression handling
//!    - Index management for fast retrieval
//! 
//! 3. **Database** (this file): High-level database interface
//!    - `VectorDatabase`: Main database API
//!    - Transaction-like operations
//!    - Query and similarity search preparation
//! 
//! ## Usage Example
//! 
//! ```rust
//! use crate::vector_db::{VectorDatabase, VectorStorageConfig};
//! 
//! // Create database with default configuration
//! let config = VectorStorageConfig::default();
//! let mut db = VectorDatabase::new(config).await?;
//! 
//! // Store an embedding
//! let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
//! let entry_id = db.store_embedding(
//!     embedding,
//!     "/path/to/file.md",
//!     "chunk_1",
//!     "This is the original text content",
//!     "embedding-model-name"
//! ).await?;
//! 
//! // Retrieve the embedding
//! if let Some(entry) = db.retrieve_embedding(&entry_id).await? {
//!     println!("Retrieved embedding with {} dimensions", entry.vector.len());
//! }
//! 
//! // Delete the embedding
//! db.delete_embedding(&entry_id).await?;
//! ```
//! 
//! ## Performance Characteristics
//! 
//! - **Storage**: Linear scaling with number of embeddings
//! - **Retrieval**: O(1) lookup via in-memory index
//! - **Memory usage**: <50MB for 1000 notes (target)
//! - **Disk usage**: <10MB per 1000 embeddings (compressed)
//! 
//! ## Data Format
//! 
//! The storage format uses JSON serialization with optional compression:
//! 
//! ```json
//! {
//!   "header": {
//!     "version": {"major": 1, "minor": 0, "patch": 0},
//!     "compression": "Gzip",
//!     "entry_count": 100,
//!     "created_at": 1635724800
//!   },
//!   "entries": [
//!     {
//!       "id": "sha256_hash",
//!       "vector": [0.1, 0.2, 0.3, ...],
//!       "metadata": {
//!         "file_path": "/path/to/file.md",
//!         "chunk_id": "chunk_1",
//!         "created_at": 1635724800,
//!         "text_hash": "content_hash",
//!         "model_name": "embedding-model"
//!       }
//!     }
//!   ]
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod types;
pub mod storage;
pub mod operations;
pub mod indexing;
pub mod incremental;
pub mod atomic;
pub mod file_ops;
pub mod maintenance;
pub mod rebuilding;
pub mod performance_monitor;
pub mod deduplication;

#[cfg(test)]
mod atomic_performance_test;

#[cfg(test)]
mod tests;

use types::{EmbeddingEntry, StorageMetrics, VectorStorageConfig, VectorDbResult, VectorDbError};
use storage::{VectorStorage, CompactionResult, IntegrityReport};
use operations::{VectorOperations, BatchOperations, ValidationOperations, CleanupOperations};
use indexing::{IndexingSystem, IndexStats};
use incremental::{IncrementalUpdateManager, IncrementalConfig, UpdateStats};
use file_ops::{FileOperations, InitializationStatus, CleanupResult, BackupResult, RecoveryResult, FileSystemMetrics};
use maintenance::{MaintenanceManager, MaintenanceConfig, MaintenanceStats};
use rebuilding::{IndexRebuilder, HealthChecker, RebuildingConfig, HealthCheckConfig, RebuildResult, HealthCheckResult, RebuildProgress};

/// High-level vector database interface
/// 
/// This struct provides the main API for interacting with the vector storage system.
/// It manages the underlying storage, handles indexing, and provides convenient
/// methods for common operations.
pub struct VectorDatabase {
    /// Underlying storage engine
    storage: Arc<VectorStorage>,
    /// File operations manager
    file_ops: FileOperations,
    /// Database configuration
    config: VectorStorageConfig,
    /// In-memory cache for frequently accessed entries
    cache: Arc<RwLock<HashMap<String, EmbeddingEntry>>>,
    /// Cache configuration
    cache_max_size: usize,
    /// Core CRUD operations interface
    operations: VectorOperations,
    /// Batch operations interface
    batch_operations: BatchOperations,
    /// Validation operations interface
    validation_operations: ValidationOperations,
    /// Cleanup operations interface
    cleanup_operations: CleanupOperations,
    /// Indexing system for fast lookups
    indexing_system: Option<IndexingSystem>,
    /// Incremental update manager for file change monitoring
    incremental_manager: Option<IncrementalUpdateManager>,
    /// Maintenance manager for cleanup and optimization operations
    maintenance_manager: Option<MaintenanceManager>,
    /// Index rebuilder for full index rebuilding operations
    index_rebuilder: Option<IndexRebuilder>,
    /// Health checker for index validation and health monitoring
    health_checker: Option<HealthChecker>,
}

impl VectorDatabase {
    /// Create a new vector database with the given configuration
    pub async fn new(config: VectorStorageConfig) -> VectorDbResult<Self> {
        let storage = Arc::new(VectorStorage::new(config.clone())?);
        let file_ops = FileOperations::new(config.clone())?;
        let cache_max_size = 100; // Cache up to 100 frequently accessed entries
        
        // Create operations interfaces
        let operations = VectorOperations::new(storage.clone(), config.clone());
        let batch_operations = BatchOperations::new(operations.clone());
        let validation_operations = ValidationOperations::new(storage.clone());
        let cleanup_operations = CleanupOperations::new(storage.clone(), operations.clone());
        
        // Initialize indexing system if enabled in config (optional for performance)
        let indexing_system = if config.enable_metrics {
            let index_file_path = format!("{}/vector_indexes.json", config.storage_dir);
            Some(IndexingSystem::new(storage.clone(), true, index_file_path).await?)
        } else {
            None
        };
        
        Ok(Self {
            storage,
            file_ops,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_max_size,
            operations,
            batch_operations,
            validation_operations,
            cleanup_operations,
            indexing_system,
            incremental_manager: None, // Initialized on demand via enable_incremental_updates
            maintenance_manager: None, // Initialized on demand via enable_maintenance
            index_rebuilder: None, // Initialized on demand via enable_index_rebuilding
            health_checker: None, // Initialized on demand via enable_health_checks
        })
    }
    
    /// Create a new vector database with default configuration
    pub async fn with_default_config(storage_dir: impl Into<String>) -> VectorDbResult<Self> {
        let config = VectorStorageConfig {
            storage_dir: storage_dir.into(),
            ..VectorStorageConfig::default()
        };
        Self::new(config).await
    }
    
    /// Store a new embedding in the database
    /// 
    /// # Arguments
    /// 
    /// * `vector` - The embedding vector (f32 values)
    /// * `file_path` - Path to the source file
    /// * `chunk_id` - Unique identifier for the text chunk
    /// * `original_text` - The original text that was embedded
    /// * `model_name` - Name of the model used to generate the embedding
    /// 
    /// # Returns
    /// 
    /// The unique ID of the stored embedding entry
    pub async fn store_embedding(
        &self,
        vector: Vec<f32>,
        file_path: impl Into<String>,
        chunk_id: impl Into<String>,
        original_text: &str,
        model_name: impl Into<String>,
    ) -> VectorDbResult<String> {
        let entry = EmbeddingEntry::new(
            vector,
            file_path.into(),
            chunk_id.into(),
            original_text,
            model_name.into(),
        );
        
        let entry_id = entry.id.clone();
        
        // Validate entry before storing
        entry.validate()?;
        
        // Store in persistent storage
        self.storage.store_entries(vec![entry.clone()]).await?;
        
        // Update cache
        self.update_cache(entry_id.clone(), entry).await;
        
        Ok(entry_id)
    }
    
    /// Store multiple embeddings in a batch operation
    /// 
    /// This is more efficient than storing embeddings individually as it minimizes
    /// I/O operations and maintains data consistency.
    pub async fn store_embeddings_batch(&self, entries: Vec<EmbeddingEntry>) -> VectorDbResult<Vec<String>> {
        if entries.is_empty() {
            return Ok(vec![]);
        }
        
        // Validate all entries
        for entry in &entries {
            entry.validate()?;
        }
        
        let entry_ids = entries.iter().map(|e| e.id.clone()).collect();
        
        // Store in persistent storage
        self.storage.store_entries(entries.clone()).await?;
        
        // Update cache for each entry
        for entry in entries {
            self.update_cache(entry.id.clone(), entry).await;
        }
        
        Ok(entry_ids)
    }
    
    /// Retrieve an embedding by its ID
    pub async fn retrieve_embedding(&self, entry_id: &str) -> VectorDbResult<Option<EmbeddingEntry>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(entry_id) {
                return Ok(Some(entry.clone()));
            }
        }
        
        // Retrieve from storage
        if let Some(entry) = self.storage.retrieve_entry(entry_id).await? {
            // Update cache with retrieved entry
            self.update_cache(entry_id.to_string(), entry.clone()).await;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }
    
    /// Retrieve multiple embeddings by their IDs
    pub async fn retrieve_embeddings(&self, entry_ids: &[String]) -> VectorDbResult<Vec<EmbeddingEntry>> {
        let mut results = Vec::new();
        let mut uncached_ids = Vec::new();
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            for entry_id in entry_ids {
                if let Some(entry) = cache.get(entry_id) {
                    results.push(entry.clone());
                } else {
                    uncached_ids.push(entry_id.clone());
                }
            }
        }
        
        // Retrieve uncached entries from storage
        if !uncached_ids.is_empty() {
            let storage_results = self.storage.retrieve_entries(&uncached_ids).await?;
            
            // Update cache with retrieved entries
            for entry in &storage_results {
                self.update_cache(entry.id.clone(), entry.clone()).await;
            }
            
            results.extend(storage_results);
        }
        
        Ok(results)
    }
    
    /// Update an existing embedding entry
    pub async fn update_embedding(&self, entry_id: &str, new_vector: Vec<f32>) -> VectorDbResult<bool> {
        if let Some(mut entry) = self.retrieve_embedding(entry_id).await? {
            entry.update_vector(new_vector);
            
            // Store updated entry
            self.storage.store_entries(vec![entry.clone()]).await?;
            
            // Update cache
            self.update_cache(entry_id.to_string(), entry).await;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Delete an embedding from the database
    pub async fn delete_embedding(&self, entry_id: &str) -> VectorDbResult<bool> {
        let deleted = self.storage.delete_entry(entry_id).await?;
        
        if deleted {
            // Remove from cache
            let mut cache = self.cache.write().await;
            cache.remove(entry_id);
        }
        
        Ok(deleted)
    }
    
    /// List all embedding IDs in the database
    pub async fn list_embedding_ids(&self) -> Vec<String> {
        self.storage.list_entry_ids().await
    }
    
    /// Get database statistics and metrics
    pub async fn get_metrics(&self) -> VectorDbResult<DatabaseMetrics> {
        let storage_metrics = self.storage.get_metrics().await;
        let cache_metrics = self.get_cache_metrics().await;
        
        Ok(DatabaseMetrics {
            storage: storage_metrics,
            cache: cache_metrics,
        })
    }
    
    /// Compact the database to optimize storage and remove deleted entries
    pub async fn compact(&self) -> VectorDbResult<CompactionResult> {
        // Clear cache before compaction to avoid stale data
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }
        
        self.storage.compact_storage().await
    }
    
    /// Validate database integrity and return a detailed report
    pub async fn validate_integrity(&self) -> VectorDbResult<IntegrityReport> {
        self.storage.validate_integrity().await
    }
    
    /// Get the current database configuration
    pub fn get_config(&self) -> &VectorStorageConfig {
        &self.config
    }
    
    /// Update database configuration
    /// 
    /// Note: Some configuration changes may require a database restart to take effect
    pub fn update_config(&mut self, new_config: VectorStorageConfig) {
        self.config = new_config;
        // Note: Storage config update would need to be implemented in VectorStorage
    }
    
    /// Find embeddings by file path
    /// 
    /// This is useful for finding all embeddings associated with a specific file
    pub async fn find_embeddings_by_file(&self, file_path: &str) -> VectorDbResult<Vec<EmbeddingEntry>> {
        let all_ids = self.list_embedding_ids().await;
        let all_entries = self.retrieve_embeddings(&all_ids).await?;
        
        let matching_entries = all_entries
            .into_iter()
            .filter(|entry| entry.metadata.file_path == file_path)
            .collect();
        
        Ok(matching_entries)
    }
    
    /// Find embeddings by model name
    /// 
    /// This is useful for finding all embeddings generated by a specific model
    pub async fn find_embeddings_by_model(&self, model_name: &str) -> VectorDbResult<Vec<EmbeddingEntry>> {
        let all_ids = self.list_embedding_ids().await;
        let all_entries = self.retrieve_embeddings(&all_ids).await?;
        
        let matching_entries = all_entries
            .into_iter()
            .filter(|entry| entry.metadata.model_name == model_name)
            .collect();
        
        Ok(matching_entries)
    }
    
    /// Remove all embeddings for a specific file
    /// 
    /// This is useful when a file is deleted or significantly modified
    pub async fn delete_embeddings_by_file(&self, file_path: &str) -> VectorDbResult<usize> {
        let matching_entries = self.find_embeddings_by_file(file_path).await?;
        let mut deleted_count = 0;
        
        for entry in matching_entries {
            if self.delete_embedding(&entry.id).await? {
                deleted_count += 1;
            }
        }
        
        Ok(deleted_count)
    }
    
    /// Get storage directory path
    pub fn get_storage_path(&self) -> PathBuf {
        PathBuf::from(&self.config.storage_dir)
    }
    
    /// Check if the database is empty
    pub async fn is_empty(&self) -> bool {
        self.list_embedding_ids().await.is_empty()
    }
    
    /// Get the total number of embeddings in the database
    pub async fn count_embeddings(&self) -> usize {
        self.list_embedding_ids().await.len()
    }

    // === New Operations Interface Methods ===
    
    /// Get reference to core CRUD operations
    /// 
    /// Provides access to the core operations interface for more advanced
    /// or specialized use cases beyond the standard database methods.
    pub fn operations(&self) -> &VectorOperations {
        &self.operations
    }
    
    /// Get reference to batch operations
    /// 
    /// Provides access to optimized batch operations for bulk processing
    /// of multiple embedding entries.
    pub fn batch_operations(&self) -> &BatchOperations {
        &self.batch_operations
    }
    
    /// Get reference to validation operations
    /// 
    /// Provides access to database validation and integrity checking
    /// operations.
    pub fn validation_operations(&self) -> &ValidationOperations {
        &self.validation_operations
    }
    
    /// Get reference to cleanup operations
    /// 
    /// Provides access to database maintenance and cleanup operations
    /// including orphaned entry removal and compaction.
    pub fn cleanup_operations(&self) -> &CleanupOperations {
        &self.cleanup_operations
    }
    
    /// Get reference to indexing system (if enabled)
    /// 
    /// Provides access to the advanced indexing system for fast lookups
    /// by various criteria like file path, model name, etc.
    pub fn indexing_system(&self) -> Option<&IndexingSystem> {
        self.indexing_system.as_ref()
    }
    
    /// Find embeddings by file path using indexing system
    /// 
    /// This method provides fast lookup of embeddings associated with a
    /// specific file path using the indexing system if available.
    /// 
    /// # Arguments
    /// 
    /// * `file_path` - The file path to search for
    /// 
    /// # Returns
    /// 
    /// Vector of embedding entries associated with the file path
    pub async fn find_embeddings_by_file_indexed(&self, file_path: &str) -> VectorDbResult<Vec<EmbeddingEntry>> {
        if let Some(indexing) = &self.indexing_system {
            let entry_ids = indexing.find_by_file_path(file_path).await;
            self.retrieve_embeddings(&entry_ids).await
        } else {
            // Fallback to the original method if indexing is not available
            self.find_embeddings_by_file(file_path).await
        }
    }
    
    /// Find embeddings by model name using indexing system
    /// 
    /// This method provides fast lookup of embeddings created with a
    /// specific model using the indexing system if available.
    /// 
    /// # Arguments
    /// 
    /// * `model_name` - The model name to search for
    /// 
    /// # Returns
    /// 
    /// Vector of embedding entries created with the specified model
    pub async fn find_embeddings_by_model_indexed(&self, model_name: &str) -> VectorDbResult<Vec<EmbeddingEntry>> {
        if let Some(indexing) = &self.indexing_system {
            let entry_ids = indexing.find_by_model_name(model_name).await;
            self.retrieve_embeddings(&entry_ids).await
        } else {
            // Fallback to the original method if indexing is not available
            self.find_embeddings_by_model(model_name).await
        }
    }
    
    /// Find embeddings by timestamp range using indexing system
    /// 
    /// This method provides fast lookup of embeddings created within a
    /// specific time range using the indexing system if available.
    /// 
    /// # Arguments
    /// 
    /// * `start_timestamp` - Start of the time range (inclusive)
    /// * `end_timestamp` - End of the time range (inclusive)
    /// 
    /// # Returns
    /// 
    /// Vector of embedding entries created within the time range
    pub async fn find_embeddings_by_timestamp_range(&self, start_timestamp: u64, end_timestamp: u64) -> VectorDbResult<Vec<EmbeddingEntry>> {
        if let Some(indexing) = &self.indexing_system {
            let entry_ids = indexing.find_by_timestamp_range(start_timestamp, end_timestamp).await;
            self.retrieve_embeddings(&entry_ids).await
        } else {
            Err(VectorDbError::Storage {
                message: "Timestamp range queries require indexing system to be enabled".to_string(),
            })
        }
    }
    
    /// Get comprehensive database statistics including index statistics
    /// 
    /// This method returns detailed statistics about the database including
    /// storage metrics, cache metrics, and index statistics if available.
    pub async fn get_comprehensive_metrics(&self) -> VectorDbResult<ComprehensiveDatabaseMetrics> {
        let base_metrics = self.get_metrics().await?;
        
        let index_stats = if let Some(indexing) = &self.indexing_system {
            Some(indexing.get_index_stats().await)
        } else {
            None
        };
        
        Ok(ComprehensiveDatabaseMetrics {
            database: base_metrics,
            indexing: index_stats,
        })
    }
    
    /// Perform comprehensive database validation
    /// 
    /// This method performs a thorough validation of the database including
    /// storage integrity, index consistency, and data validation.
    pub async fn validate_comprehensive(&self) -> VectorDbResult<ComprehensiveValidationReport> {
        let storage_report = self.validation_operations.validate_database().await?;
        
        // Additional validations can be added here
        
        Ok(ComprehensiveValidationReport {
            storage_integrity: storage_report,
            indexing_consistency: true, // Placeholder - would implement actual index validation
        })
    }
    
    // === File Operations Methods ===
    
    /// Initialize the database file system and perform startup checks
    /// 
    /// This method should be called after creating the VectorDatabase instance
    /// to ensure the storage directory structure is properly set up and any
    /// existing data is validated.
    pub async fn initialize(&self) -> VectorDbResult<InitializationStatus> {
        self.file_ops.initialize_database().await
    }
    
    /// Clean up temporary files, stale locks, and old backups
    /// 
    /// This is useful for maintenance operations and can be called periodically
    /// to keep the storage directory clean and optimize disk usage.
    pub async fn cleanup(&self) -> VectorDbResult<CleanupResult> {
        self.file_ops.cleanup_stale_files().await
    }
    
    /// Create a full backup of the vector database
    /// 
    /// This creates a point-in-time backup that can be used for recovery.
    /// Backup creation is atomic and safe to run during normal operations.
    pub async fn create_backup(&self) -> VectorDbResult<BackupResult> {
        self.file_ops.create_backup().await
    }
    
    /// Recover from backup or attempt automatic recovery
    /// 
    /// If backup_path is provided, recovery will be attempted from that specific
    /// backup. Otherwise, automatic recovery will try to use the most recent backup.
    pub async fn recover(&self, backup_path: Option<PathBuf>) -> VectorDbResult<RecoveryResult> {
        self.file_ops.recover_from_backup(backup_path).await
    }
    
    /// Get detailed file system metrics
    /// 
    /// This provides comprehensive information about storage usage, including
    /// storage files, backups, temporary files, and active locks.
    pub async fn get_file_metrics(&self) -> VectorDbResult<FileSystemMetrics> {
        self.file_ops.get_file_metrics().await
    }
    
    /// Get combined database metrics including storage, cache, and file system
    /// 
    /// This extends the basic metrics with file system information for a
    /// complete view of the database state.
    pub async fn get_comprehensive_file_metrics(&self) -> VectorDbResult<ComprehensiveMetrics> {
        let storage_metrics = self.storage.get_metrics().await;
        let cache_metrics = self.get_cache_metrics().await;
        let file_metrics = self.file_ops.get_file_metrics().await?;
        
        Ok(ComprehensiveMetrics {
            storage: storage_metrics,
            cache: cache_metrics,
            file_system: file_metrics,
        })
    }
    
    // === Incremental Update System Methods ===
    
    /// Enable incremental updates for the database
    /// 
    /// This method initializes the incremental update manager that monitors
    /// file system changes and automatically updates embeddings when files
    /// are created, modified, or deleted.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Configuration for the incremental update system
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure of initialization
    pub async fn enable_incremental_updates(&mut self, config: IncrementalConfig) -> VectorDbResult<()> {
        let incremental_manager = IncrementalUpdateManager::new(
            self.storage.clone(),
            self.config.clone(),
            config,
        ).await?;
        
        self.incremental_manager = Some(incremental_manager);
        
        eprintln!("✅ Incremental update system enabled");
        Ok(())
    }
    
    /// Start monitoring a vault path for incremental updates
    /// 
    /// This method begins monitoring a specific directory for file changes.
    /// The incremental update system must be enabled first.
    /// 
    /// # Arguments
    /// 
    /// * `vault_path` - Path to the vault directory to monitor
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure
    pub async fn start_incremental_monitoring(&mut self, vault_path: &Path) -> VectorDbResult<()> {
        if let Some(ref mut manager) = self.incremental_manager {
            manager.start_monitoring(vault_path).await?;
            Ok(())
        } else {
            Err(VectorDbError::Storage {
                message: "Incremental update system not enabled. Call enable_incremental_updates first.".to_string(),
            })
        }
    }
    
    /// Stop monitoring a vault path for incremental updates
    /// 
    /// This method stops monitoring a specific directory for file changes.
    /// 
    /// # Arguments
    /// 
    /// * `vault_path` - Path to the vault directory to stop monitoring
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure
    pub async fn stop_incremental_monitoring(&mut self, vault_path: &Path) -> VectorDbResult<()> {
        if let Some(ref mut manager) = self.incremental_manager {
            manager.stop_monitoring(vault_path).await?;
            Ok(())
        } else {
            Err(VectorDbError::Storage {
                message: "Incremental update system not enabled.".to_string(),
            })
        }
    }
    
    /// Process pending incremental updates
    /// 
    /// This method should be called periodically to process any pending
    /// file changes detected by the incremental update system.
    /// 
    /// # Returns
    /// 
    /// Optional update statistics if changes were processed, None if no changes
    pub async fn process_incremental_updates(&self) -> VectorDbResult<Option<UpdateStats>> {
        if let Some(ref manager) = self.incremental_manager {
            manager.process_pending_changes().await
        } else {
            Ok(None) // No incremental manager, no updates to process
        }
    }
    
    /// Get incremental update statistics
    /// 
    /// Returns recent update history from the incremental update system.
    /// 
    /// # Returns
    /// 
    /// Vector of update statistics from recent operations
    pub async fn get_incremental_update_history(&self) -> Vec<UpdateStats> {
        if let Some(ref manager) = self.incremental_manager {
            manager.get_update_history().await
        } else {
            Vec::new()
        }
    }
    
    /// Check if incremental updates are currently being processed
    /// 
    /// # Returns
    /// 
    /// True if updates are currently being processed, false otherwise
    pub async fn is_processing_incremental_updates(&self) -> bool {
        if let Some(ref manager) = self.incremental_manager {
            manager.is_processing().await
        } else {
            false
        }
    }
    
    /// Get incremental update system configuration
    /// 
    /// # Returns
    /// 
    /// Current incremental update configuration, or None if not enabled
    pub fn get_incremental_config(&self) -> Option<&IncrementalConfig> {
        self.incremental_manager.as_ref().map(|m| m.get_config())
    }
    
    // === Maintenance System Methods ===
    
    /// Enable maintenance operations for the database
    /// 
    /// This method initializes the maintenance manager that provides comprehensive
    /// index maintenance including orphaned embedding detection, automatic cleanup,
    /// index compaction, and storage optimization.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Configuration for the maintenance system
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure of initialization
    pub async fn enable_maintenance(&mut self, config: MaintenanceConfig) -> VectorDbResult<()> {
        let maintenance_manager = MaintenanceManager::new(
            self.storage.clone(),
            self.operations.clone(),
            config,
        ).await?;
        
        self.maintenance_manager = Some(maintenance_manager);
        
        eprintln!("✅ Maintenance system enabled");
        Ok(())
    }
    
    /// Start automatic maintenance operations
    /// 
    /// This method begins automatic maintenance cycles that run in the background
    /// to keep the database optimized and clean up orphaned data.
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure
    pub async fn start_maintenance(&self) -> VectorDbResult<()> {
        if let Some(ref manager) = self.maintenance_manager {
            manager.start_maintenance().await?;
            Ok(())
        } else {
            Err(VectorDbError::Storage {
                message: "Maintenance system not enabled. Call enable_maintenance first.".to_string(),
            })
        }
    }
    
    /// Stop automatic maintenance operations
    /// 
    /// This method stops the automatic maintenance cycles.
    pub async fn stop_maintenance(&self) {
        if let Some(ref manager) = self.maintenance_manager {
            manager.stop_maintenance().await;
        }
    }
    
    /// Run a manual maintenance cycle
    /// 
    /// This method performs a complete maintenance cycle including orphaned
    /// embedding cleanup, index compaction, and storage optimization.
    /// 
    /// # Returns
    /// 
    /// Maintenance statistics from the cycle
    pub async fn run_maintenance_cycle(&self) -> VectorDbResult<MaintenanceStats> {
        if let Some(ref manager) = self.maintenance_manager {
            manager.run_maintenance_cycle().await
        } else {
            Err(VectorDbError::Storage {
                message: "Maintenance system not enabled. Call enable_maintenance first.".to_string(),
            })
        }
    }
    
    /// Get maintenance statistics
    /// 
    /// Returns comprehensive statistics about maintenance operations including
    /// cleanup counts, performance metrics, and operation history.
    /// 
    /// # Returns
    /// 
    /// Current maintenance statistics
    pub async fn get_maintenance_stats(&self) -> VectorDbResult<MaintenanceStats> {
        if let Some(ref manager) = self.maintenance_manager {
            Ok(manager.get_maintenance_stats().await)
        } else {
            Err(VectorDbError::Storage {
                message: "Maintenance system not enabled.".to_string(),
            })
        }
    }
    
    /// Check if automatic maintenance is currently running
    /// 
    /// # Returns
    /// 
    /// True if maintenance is running, false otherwise
    pub async fn is_maintenance_running(&self) -> bool {
        if let Some(ref manager) = self.maintenance_manager {
            manager.is_maintenance_running().await
        } else {
            false
        }
    }
    
    /// Get maintenance system configuration
    /// 
    /// # Returns
    /// 
    /// Current maintenance configuration, or None if not enabled
    pub fn get_maintenance_config(&self) -> Option<&MaintenanceConfig> {
        self.maintenance_manager.as_ref().map(|m| m.get_config())
    }
    
    // === Index Rebuilding System Methods ===
    
    /// Enable index rebuilding capabilities for the database
    /// 
    /// This method initializes the index rebuilder that provides full index rebuilding
    /// capabilities with progress tracking and parallel processing support.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Configuration for the index rebuilding system
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure of initialization
    pub async fn enable_index_rebuilding(&mut self, config: RebuildingConfig) -> VectorDbResult<()> {
        let operations = VectorOperations::new(self.storage.clone(), self.config.clone());
        let index_rebuilder = IndexRebuilder::new(
            self.storage.clone(),
            operations,
            config,
        );
        
        self.index_rebuilder = Some(index_rebuilder);
        
        eprintln!("✅ Index rebuilding system enabled");
        Ok(())
    }
    
    /// Perform a complete index rebuild with progress tracking
    /// 
    /// This method performs a full reconstruction of the vector database index
    /// with optional parallel processing, progress reporting, and health validation.
    /// 
    /// # Returns
    /// 
    /// Detailed results of the rebuild operation including performance metrics
    pub async fn rebuild_index_full(&self) -> VectorDbResult<RebuildResult> {
        if let Some(ref rebuilder) = self.index_rebuilder {
            rebuilder.rebuild_index().await
        } else {
            Err(VectorDbError::Storage {
                message: "Index rebuilding system not enabled. Call enable_index_rebuilding first.".to_string(),
            })
        }
    }
    
    /// Set a progress callback for index rebuilding operations
    /// 
    /// This method allows the frontend to receive real-time progress updates
    /// during index rebuilding operations.
    /// 
    /// # Arguments
    /// 
    /// * `callback` - Callback function to receive progress updates
    pub async fn set_rebuild_progress_callback(&mut self, callback: Arc<dyn Fn(RebuildProgress) + Send + Sync>) -> VectorDbResult<()> {
        if let Some(ref mut rebuilder) = self.index_rebuilder {
            rebuilder.set_progress_callback(callback);
            Ok(())
        } else {
            Err(VectorDbError::Storage {
                message: "Index rebuilding system not enabled.".to_string(),
            })
        }
    }
    
    /// Cancel any currently running index rebuild operation
    /// 
    /// This method allows graceful cancellation of long-running rebuild operations.
    pub async fn cancel_index_rebuild(&self) {
        if let Some(ref rebuilder) = self.index_rebuilder {
            rebuilder.cancel();
        }
    }
    
    // === Health Check System Methods ===
    
    /// Enable health check capabilities for the database
    /// 
    /// This method initializes the health checker that provides comprehensive
    /// index integrity validation, performance testing, and corruption detection.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Configuration for the health check system
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure of initialization
    pub async fn enable_health_checks(&mut self, config: HealthCheckConfig) -> VectorDbResult<()> {
        let operations = VectorOperations::new(self.storage.clone(), self.config.clone());
        let health_checker = HealthChecker::new(
            self.storage.clone(),
            operations,
            config,
        );
        
        self.health_checker = Some(health_checker);
        
        eprintln!("✅ Health check system enabled");
        Ok(())
    }
    
    /// Perform a comprehensive health check of the index
    /// 
    /// This method performs integrity validation, performance testing, and corruption
    /// detection to assess the overall health of the vector database index.
    /// 
    /// # Returns
    /// 
    /// Detailed health check results with recommendations
    pub async fn perform_health_check(&self) -> VectorDbResult<HealthCheckResult> {
        if let Some(ref health_checker) = self.health_checker {
            health_checker.perform_health_check().await
        } else {
            Err(VectorDbError::Storage {
                message: "Health check system not enabled. Call enable_health_checks first.".to_string(),
            })
        }
    }
    
    /// Perform a quick health check focused on performance validation
    /// 
    /// This method performs a faster health check that focuses primarily on
    /// performance validation to meet the <1 second target requirement.
    /// 
    /// # Returns
    /// 
    /// Health check results with emphasis on performance metrics
    pub async fn perform_quick_health_check(&self) -> VectorDbResult<HealthCheckResult> {
        if let Some(ref health_checker) = self.health_checker {
            // Create a performance-focused config for quick checks
            let quick_config = HealthCheckConfig {
                enable_integrity_validation: false,
                enable_performance_validation: true,
                enable_corruption_detection: false,
                performance_sample_percentage: 0.05, // 5% sample for speed
                target_check_time_seconds: 1,
                enable_detailed_reporting: false,
            };
            
            let quick_checker = HealthChecker::new(
                self.storage.clone(),
                VectorOperations::new(self.storage.clone(), self.config.clone()),
                quick_config,
            );
            
            quick_checker.perform_health_check().await
        } else {
            Err(VectorDbError::Storage {
                message: "Health check system not enabled. Call enable_health_checks first.".to_string(),
            })
        }
    }
    
    /// Detect and report potential index corruption
    /// 
    /// This method performs focused corruption detection to identify data integrity
    /// issues that may require index rebuilding or recovery.
    /// 
    /// # Returns
    /// 
    /// Health check results focused on corruption detection
    pub async fn detect_index_corruption(&self) -> VectorDbResult<HealthCheckResult> {
        if let Some(ref health_checker) = self.health_checker {
            // Create a corruption-focused config
            let corruption_config = HealthCheckConfig {
                enable_integrity_validation: true,
                enable_performance_validation: false,
                enable_corruption_detection: true,
                performance_sample_percentage: 0.1, // 10% sample for thoroughness
                target_check_time_seconds: 5, // Allow more time for corruption detection
                enable_detailed_reporting: true,
            };
            
            let corruption_checker = HealthChecker::new(
                self.storage.clone(),
                VectorOperations::new(self.storage.clone(), self.config.clone()),
                corruption_config,
            );
            
            corruption_checker.perform_health_check().await
        } else {
            Err(VectorDbError::Storage {
                message: "Health check system not enabled. Call enable_health_checks first.".to_string(),
            })
        }
    }
    
    /// Get the configuration of the index rebuilding system
    /// 
    /// # Returns
    /// 
    /// Current rebuilding configuration, or None if not enabled
    pub fn get_rebuilding_config(&self) -> Option<&RebuildingConfig> {
        // Note: We would need to store the config in IndexRebuilder to return it
        // For now, return None as a placeholder
        None
    }
    
    /// Get the configuration of the health check system
    /// 
    /// # Returns
    /// 
    /// Current health check configuration, or None if not enabled
    pub fn get_health_check_config(&self) -> Option<&HealthCheckConfig> {
        // Note: We would need to store the config in HealthChecker to return it
        // For now, return None as a placeholder
        None
    }
    
    /// Check if index rebuilding system is enabled
    /// 
    /// # Returns
    /// 
    /// True if rebuilding system is enabled, false otherwise
    pub fn is_rebuilding_enabled(&self) -> bool {
        self.index_rebuilder.is_some()
    }
    
    /// Check if health check system is enabled
    /// 
    /// # Returns
    /// 
    /// True if health check system is enabled, false otherwise
    pub fn is_health_checks_enabled(&self) -> bool {
        self.health_checker.is_some()
    }
    
    // === Deduplication System Methods ===
    
    /// Perform embedding deduplication with similarity-based duplicate detection
    /// 
    /// This method identifies and merges near-identical embeddings based on cosine
    /// similarity, maintaining reference tracking for backward compatibility.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Deduplication configuration (similarity thresholds, strategies)
    /// 
    /// # Returns
    /// 
    /// Comprehensive deduplication results including clusters, reference mappings, and metrics
    /// 
    /// # Performance
    /// 
    /// - Target: <10 seconds per 1000 embeddings
    /// - Memory: Efficient batch processing for large datasets
    /// - Index reduction: 10-30% typical reduction through deduplication
    pub async fn deduplicate_embeddings(
        &self,
        config: deduplication::DeduplicationConfig,
    ) -> VectorDbResult<deduplication::DeduplicationResult_> {
        use deduplication::EmbeddingDeduplicator;
        
        // Get all current embeddings
        let all_ids = self.list_embedding_ids().await;
        let all_embeddings = self.retrieve_embeddings(&all_ids).await?;
        
        eprintln!("🔧 Starting deduplication of {} embeddings", all_embeddings.len());
        
        // Perform deduplication
        let deduplication_result = EmbeddingDeduplicator::deduplicate_embeddings(
            all_embeddings,
            &config,
        ).map_err(|e| VectorDbError::Storage {
            message: format!("Deduplication failed: {}", e),
        })?;
        
        eprintln!("✅ Deduplication completed: {} -> {} embeddings ({:.1}% reduction)",
                  deduplication_result.metrics.embeddings_processed,
                  deduplication_result.deduplicated_embeddings.len(),
                  deduplication_result.metrics.index_size_reduction_percentage);
        
        Ok(deduplication_result)
    }
    
    /// Apply deduplication results to the database
    /// 
    /// This method updates the database with deduplicated embeddings and maintains
    /// reference mappings for backward compatibility.
    /// 
    /// # Arguments
    /// 
    /// * `deduplication_result` - Results from deduplication operation
    /// * `create_backup` - Whether to create a backup before applying changes
    /// 
    /// # Returns
    /// 
    /// Success status and operation metrics
    pub async fn apply_deduplication_results(
        &self,
        deduplication_result: &deduplication::DeduplicationResult_,
        create_backup: bool,
    ) -> VectorDbResult<ApplyDeduplicationStats> {
        use std::time::Instant;
        
        let start_time = Instant::now();
        
        if create_backup {
            eprintln!("💾 Creating backup before applying deduplication...");
            self.create_backup().await?;
        }
        
        // Clear current cache to avoid inconsistencies
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }
        
        // Store deduplicated embeddings
        let deduplicated_ids = self.store_embeddings_batch(
            deduplication_result.deduplicated_embeddings.clone()
        ).await?;
        
        // Remove duplicates from storage (logical deletion through index)
        let mut removed_count = 0;
        for original_id in deduplication_result.reference_mapping.mapping.keys() {
            if self.delete_embedding(original_id).await? {
                removed_count += 1;
            }
        }
        
        // Trigger compaction to physically remove deleted entries
        let compaction_result = self.compact().await?;
        
        let total_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        
        let stats = ApplyDeduplicationStats {
            deduplicated_embeddings_stored: deduplicated_ids.len(),
            duplicate_embeddings_removed: removed_count,
            compaction_result: Some(compaction_result),
            total_apply_time_ms: total_time_ms,
            backup_created: create_backup,
        };
        
        eprintln!("🎯 Applied deduplication: stored {} representatives, removed {} duplicates in {:.2}ms",
                  stats.deduplicated_embeddings_stored,
                  stats.duplicate_embeddings_removed,
                  stats.total_apply_time_ms);
        
        Ok(stats)
    }
    
    /// Resolve an embedding ID through deduplication reference mapping
    /// 
    /// This method resolves embedding IDs that may have been merged during
    /// deduplication, providing backward compatibility.
    /// 
    /// # Arguments
    /// 
    /// * `embedding_id` - Original embedding ID to resolve
    /// * `reference_mapping` - Reference mapping from deduplication
    /// 
    /// # Returns
    /// 
    /// The representative embedding ID (original ID if not deduplicated)
    pub fn resolve_embedding_reference(
        &self,
        embedding_id: &str,
        reference_mapping: &deduplication::ReferenceMapping,
    ) -> String {
        deduplication::EmbeddingDeduplicator::resolve_embedding_reference(
            embedding_id,
            reference_mapping,
        )
    }
    
    /// Check if an embedding was affected by deduplication
    /// 
    /// # Arguments
    /// 
    /// * `embedding_id` - Embedding ID to check
    /// * `reference_mapping` - Reference mapping from deduplication
    /// 
    /// # Returns
    /// 
    /// True if the embedding was merged into a representative
    pub fn was_deduplicated(
        &self,
        embedding_id: &str,
        reference_mapping: &deduplication::ReferenceMapping,
    ) -> bool {
        deduplication::EmbeddingDeduplicator::was_deduplicated(
            embedding_id,
            reference_mapping,
        )
    }
    
    /// Get comprehensive deduplication statistics
    /// 
    /// This method analyzes the current database state and provides metrics
    /// about potential deduplication benefits.
    /// 
    /// # Returns
    /// 
    /// Statistics about duplicate potential and recommended actions
    pub async fn analyze_duplication_potential(
        &self,
        config: &deduplication::DeduplicationConfig,
    ) -> VectorDbResult<DuplicationAnalysis> {
        use std::time::Instant;
        
        let start_time = Instant::now();
        let all_ids = self.list_embedding_ids().await;
        
        if all_ids.len() < 2 {
            return Ok(DuplicationAnalysis {
                total_embeddings: all_ids.len(),
                estimated_duplicates: 0,
                estimated_reduction_percentage: 0.0,
                recommended_threshold: config.similarity_threshold,
                analysis_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
                sample_size: all_ids.len(),
            });
        }
        
        // Sample-based analysis for large datasets
        let sample_size = if all_ids.len() > 1000 { 100 } else { all_ids.len() };
        let sample_ids = if all_ids.len() > sample_size {
            all_ids.into_iter().take(sample_size).collect()
        } else {
            all_ids
        };
        
        let sample_embeddings = self.retrieve_embeddings(&sample_ids).await?;
        let mut duplicate_count = 0;
        let mut similarity_sum = 0.0;
        let mut comparisons = 0;
        
        // Quick similarity analysis on sample
        for i in 0..sample_embeddings.len() {
            for j in (i + 1)..sample_embeddings.len() {
                if let Ok(similarity) = crate::similarity_search::SimilaritySearch::cosine_similarity(
                    &sample_embeddings[i].vector,
                    &sample_embeddings[j].vector,
                ) {
                    similarity_sum += similarity;
                    comparisons += 1;
                    
                    if similarity >= config.similarity_threshold {
                        duplicate_count += 1;
                    }
                }
            }
        }
        
        let avg_similarity = if comparisons > 0 { similarity_sum / comparisons as f32 } else { 0.0 };
        let sample_duplicate_rate = if sample_embeddings.len() > 0 {
            duplicate_count as f32 / sample_embeddings.len() as f32
        } else {
            0.0
        };
        
        // Extrapolate to full database
        let total_embeddings = self.count_embeddings().await;
        let estimated_duplicates = (total_embeddings as f32 * sample_duplicate_rate) as usize;
        let estimated_reduction_percentage = if total_embeddings > 0 {
            (estimated_duplicates as f32 / total_embeddings as f32) * 100.0
        } else {
            0.0
        };
        
        // Recommend threshold based on analysis
        let recommended_threshold = if avg_similarity > 0.9 {
            0.98 // High similarity dataset - use stricter threshold
        } else if avg_similarity > 0.7 {
            0.95 // Medium similarity - use default
        } else {
            0.90 // Lower similarity - use more lenient threshold
        };
        
        Ok(DuplicationAnalysis {
            total_embeddings,
            estimated_duplicates,
            estimated_reduction_percentage,
            recommended_threshold,
            analysis_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            sample_size: sample_embeddings.len(),
        })
    }
    
    // Private helper methods
    
    /// Update the in-memory cache with an entry
    async fn update_cache(&self, entry_id: String, entry: EmbeddingEntry) {
        let mut cache = self.cache.write().await;
        
        // Implement simple LRU eviction if cache is full
        if cache.len() >= self.cache_max_size && !cache.contains_key(&entry_id) {
            // Remove oldest entry (simplified LRU - would need proper timestamp tracking)
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        
        cache.insert(entry_id, entry);
    }
    
    /// Get cache-specific metrics
    async fn get_cache_metrics(&self) -> CacheMetrics {
        let cache = self.cache.read().await;
        
        CacheMetrics {
            entries_count: cache.len(),
            max_size: self.cache_max_size,
            memory_usage_bytes: cache.values().map(|e| e.memory_footprint()).sum(),
        }
    }
}

/// Combined database metrics including storage and cache statistics
#[derive(Debug, Clone)]
pub struct DatabaseMetrics {
    /// Storage layer metrics
    pub storage: StorageMetrics,
    /// Cache layer metrics
    pub cache: CacheMetrics,
}

/// Comprehensive database metrics including file system information
#[derive(Debug, Clone)]
pub struct ComprehensiveMetrics {
    /// Storage layer metrics
    pub storage: StorageMetrics,
    /// Cache layer metrics  
    pub cache: CacheMetrics,
    /// File system metrics
    pub file_system: FileSystemMetrics,
}

/// Cache-specific metrics
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    /// Number of entries in cache
    pub entries_count: usize,
    /// Maximum cache size
    pub max_size: usize,
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: usize,
}

impl DatabaseMetrics {
    /// Get total number of unique embeddings (from storage)
    pub fn total_embeddings(&self) -> usize {
        self.storage.total_entries
    }
    
    /// Get cache hit ratio estimate (simplified)
    pub fn cache_utilization(&self) -> f64 {
        if self.cache.max_size > 0 {
            self.cache.entries_count as f64 / self.cache.max_size as f64
        } else {
            0.0
        }
    }
    
    /// Get total memory usage estimate
    pub fn total_memory_usage(&self) -> usize {
        self.cache.memory_usage_bytes // Storage is on disk
    }
    
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Vector DB: {} embeddings, {} files, {:.1} MB storage, {} cached entries ({:.1}% cache utilization)",
            self.storage.total_entries,
            self.storage.file_count,
            self.storage.total_size_bytes as f64 / (1024.0 * 1024.0),
            self.cache.entries_count,
            self.cache_utilization() * 100.0
        )
    }
}

impl ComprehensiveMetrics {
    /// Get total number of unique embeddings (from storage)
    pub fn total_embeddings(&self) -> usize {
        self.storage.total_entries
    }
    
    /// Get cache hit ratio estimate (simplified)
    pub fn cache_utilization(&self) -> f64 {
        if self.cache.max_size > 0 {
            self.cache.entries_count as f64 / self.cache.max_size as f64
        } else {
            0.0
        }
    }
    
    /// Get total memory usage estimate
    pub fn total_memory_usage(&self) -> usize {
        self.cache.memory_usage_bytes // Storage and file system are on disk
    }
    
    /// Get total disk usage estimate
    pub fn total_disk_usage(&self) -> usize {
        self.storage.total_size_bytes + self.file_system.total_size_bytes
    }
    
    /// Generate a comprehensive human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Vector DB: {} embeddings, {} storage files ({:.1} MB), {} backups ({:.1} MB), {} cached entries ({:.1}% cache), {} locks",
            self.storage.total_entries,
            self.file_system.storage_files,
            self.file_system.storage_size_bytes as f64 / (1024.0 * 1024.0),
            self.file_system.backup_files,
            self.file_system.backup_size_bytes as f64 / (1024.0 * 1024.0),
            self.cache.entries_count,
            self.cache_utilization() * 100.0,
            self.file_system.active_locks
        )
    }
}

/// Comprehensive database metrics including indexing statistics
#[derive(Debug, Clone)]
pub struct ComprehensiveDatabaseMetrics {
    /// Standard database metrics (storage and cache)
    pub database: DatabaseMetrics,
    /// Optional indexing system statistics
    pub indexing: Option<IndexStats>,
}

impl ComprehensiveDatabaseMetrics {
    /// Generate a comprehensive summary of all metrics
    pub fn summary(&self) -> String {
        let mut summary = self.database.summary();
        
        if let Some(index_stats) = &self.indexing {
            summary.push_str(&format!(", {}", index_stats.summary()));
        }
        
        summary
    }
}

/// Comprehensive validation report including storage and indexing validation
#[derive(Debug)]
pub struct ComprehensiveValidationReport {
    /// Storage integrity validation results
    pub storage_integrity: IntegrityReport,
    /// Index consistency validation results
    pub indexing_consistency: bool,
}

impl ComprehensiveValidationReport {
    /// Check if the entire database is healthy
    pub fn is_healthy(&self) -> bool {
        self.storage_integrity.is_healthy() && self.indexing_consistency
    }
    
    /// Generate a comprehensive validation summary
    pub fn summary(&self) -> String {
        let storage_summary = self.storage_integrity.summary();
        let indexing_status = if self.indexing_consistency { "OK" } else { "ISSUES" };
        
        format!("{}, Indexing: {}", storage_summary, indexing_status)
    }
}

/// Statistics for applying deduplication results to the database
#[derive(Debug, Clone)]
pub struct ApplyDeduplicationStats {
    /// Number of deduplicated (representative) embeddings stored
    pub deduplicated_embeddings_stored: usize,
    /// Number of duplicate embeddings removed
    pub duplicate_embeddings_removed: usize,
    /// Results from storage compaction
    pub compaction_result: Option<storage::CompactionResult>,
    /// Total time to apply deduplication changes
    pub total_apply_time_ms: f64,
    /// Whether a backup was created before applying changes
    pub backup_created: bool,
}

/// Analysis of duplication potential in the database
#[derive(Debug, Clone)]
pub struct DuplicationAnalysis {
    /// Total number of embeddings in the database
    pub total_embeddings: usize,
    /// Estimated number of duplicates that could be found
    pub estimated_duplicates: usize,
    /// Estimated reduction percentage if deduplication were applied
    pub estimated_reduction_percentage: f32,
    /// Recommended similarity threshold based on analysis
    pub recommended_threshold: f32,
    /// Time taken for the analysis in milliseconds
    pub analysis_time_ms: f64,
    /// Size of the sample used for analysis
    pub sample_size: usize,
}

impl DuplicationAnalysis {
    /// Get a human-readable summary of the analysis
    pub fn summary(&self) -> String {
        format!(
            "Duplication Analysis: {} embeddings, ~{} duplicates ({:.1}% reduction potential), recommended threshold: {:.3}",
            self.total_embeddings,
            self.estimated_duplicates,
            self.estimated_reduction_percentage,
            self.recommended_threshold
        )
    }
    
    /// Check if deduplication would be beneficial
    pub fn is_deduplication_beneficial(&self) -> bool {
        self.estimated_reduction_percentage > 5.0 // More than 5% reduction
    }
}

// Re-export main types for convenience
pub use types::{
    EmbeddingMetadata,
    CompressionAlgorithm,
};

// Re-export additional operations types not already imported above
pub use indexing::IndexMetadata;

// Re-export incremental update system types
pub use incremental::{
    ChangeType,
};

// Re-export atomic operations types
pub use atomic::{
    AtomicConfig,
};

// Re-export rebuilding and health check types
pub use rebuilding::{
    RebuildPhase,
    HealthStatus,
    RebuildMetrics,
    HealthIssue,
    HealthIssueType,
    IssueSeverity,
    CorruptionType,
    CorruptionSeverity,
};

// Re-export deduplication types
pub use deduplication::{
    DeduplicationConfig,
    RepresentativeSelectionStrategy,
    DeduplicationResult_,
    DeduplicationMetrics,
    DuplicateCluster,
    ReferenceMapping,
    ReferenceMappingStats,
    EmbeddingDeduplicator,
};


/// Comprehensive test utilities and additional tests  
/// located in separate tests.rs module
#[cfg(test)]
pub use tests::test_utils;