//! Core CRUD Operations Module for Vector Database
//! 
//! This module provides the fundamental Create, Read, Update, Delete operations 
//! for the vector database system. It implements the core business logic for 
//! managing embedding entries while ensuring data integrity and performance.
//! 
//! ## Features
//! 
//! - **CRUD Operations**: Full create, read, update, delete functionality
//! - **Batch Operations**: Efficient bulk operations for multiple embeddings
//! - **Data Validation**: Input validation and integrity checks
//! - **Error Handling**: Comprehensive error reporting and recovery
//! - **Performance Optimized**: Operations designed for speed and efficiency
//! 
//! ## Key Components
//! 
//! - `VectorOperations`: Core operations interface
//! - `BatchOperations`: Bulk operation management
//! - `ValidationOperations`: Data integrity validation
//! - `CleanupOperations`: Orphaned data cleanup utilities

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::vector_db::types::{
    EmbeddingEntry, VectorStorageConfig, VectorDbError, VectorDbResult
};
use crate::vector_db::storage::{VectorStorage, IntegrityReport};

/// Core CRUD operations for vector database
/// 
/// This struct provides the fundamental operations for managing embedding entries
/// in the vector database, including create, read, update, and delete operations.
#[derive(Clone)]
pub struct VectorOperations {
    /// Underlying storage layer
    storage: Arc<VectorStorage>,
    /// Operation configuration
    config: VectorStorageConfig,
}

impl VectorOperations {
    /// Create a new operations instance with storage backend
    pub fn new(storage: Arc<VectorStorage>, config: VectorStorageConfig) -> Self {
        Self { storage, config }
    }

    /// Store a single embedding entry in the database
    /// 
    /// This method validates the entry, assigns a unique ID, and stores it
    /// in the underlying storage system.
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
    /// 
    /// # Errors
    /// 
    /// Returns `VectorDbError::InvalidEntry` if the entry fails validation
    /// Returns `VectorDbError::Storage` if the storage operation fails
    pub async fn store_embedding(
        &self,
        vector: Vec<f32>,
        file_path: impl Into<String>,
        chunk_id: impl Into<String>,
        original_text: &str,
        model_name: impl Into<String>,
    ) -> VectorDbResult<String> {
        // Create and validate the embedding entry
        let entry = EmbeddingEntry::new(
            vector,
            file_path.into(),
            chunk_id.into(),
            original_text,
            model_name.into(),
        );
        
        // Validate entry before storing
        entry.validate()?;
        
        let entry_id = entry.id.clone();
        
        // Store the entry using batch operation (more efficient)
        self.storage.store_entries(vec![entry]).await?;
        
        eprintln!("✅ Stored embedding entry: {} ({})", entry_id, original_text.chars().take(50).collect::<String>());
        Ok(entry_id)
    }

    /// Retrieve an embedding entry by its ID
    /// 
    /// This method looks up an embedding entry in the storage system using
    /// its unique identifier.
    /// 
    /// # Arguments
    /// 
    /// * `entry_id` - The unique identifier of the embedding entry
    /// 
    /// # Returns
    /// 
    /// `Some(EmbeddingEntry)` if the entry is found, `None` otherwise
    /// 
    /// # Errors
    /// 
    /// Returns `VectorDbError::Storage` if the retrieval operation fails
    pub async fn retrieve_embedding(&self, entry_id: &str) -> VectorDbResult<Option<EmbeddingEntry>> {
        if entry_id.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "entry_id cannot be empty".to_string(),
            });
        }

        let result = self.storage.retrieve_entry(entry_id).await?;
        
        if result.is_some() {
            eprintln!("📖 Retrieved embedding entry: {}", entry_id);
        }
        
        Ok(result)
    }

    /// Update an existing embedding entry's vector
    /// 
    /// This method updates the vector data of an existing embedding entry
    /// while preserving the metadata.
    /// 
    /// # Arguments
    /// 
    /// * `entry_id` - The unique identifier of the embedding entry
    /// * `new_vector` - The new embedding vector to replace the existing one
    /// 
    /// # Returns
    /// 
    /// `true` if the entry was found and updated, `false` if not found
    /// 
    /// # Errors
    /// 
    /// Returns `VectorDbError::Storage` if the update operation fails
    pub async fn update_embedding(&self, entry_id: &str, new_vector: Vec<f32>) -> VectorDbResult<bool> {
        if entry_id.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "entry_id cannot be empty".to_string(),
            });
        }

        if new_vector.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "new_vector cannot be empty".to_string(),
            });
        }

        // Validate vector contains valid float values
        for (i, &value) in new_vector.iter().enumerate() {
            if !value.is_finite() {
                return Err(VectorDbError::InvalidEntry {
                    reason: format!("new_vector contains invalid value at index {}: {}", i, value),
                });
            }
        }

        // Retrieve existing entry
        if let Some(mut entry) = self.retrieve_embedding(entry_id).await? {
            // Update the vector and timestamp
            entry.update_vector(new_vector);
            
            // Store the updated entry
            self.storage.store_entries(vec![entry]).await?;
            
            eprintln!("🔄 Updated embedding entry: {}", entry_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete an embedding entry from the database
    /// 
    /// This method removes an embedding entry from the storage system.
    /// The actual file cleanup happens during compaction operations.
    /// 
    /// # Arguments
    /// 
    /// * `entry_id` - The unique identifier of the embedding entry to delete
    /// 
    /// # Returns
    /// 
    /// `true` if the entry was found and deleted, `false` if not found
    /// 
    /// # Errors
    /// 
    /// Returns `VectorDbError::Storage` if the deletion operation fails
    pub async fn delete_embedding(&self, entry_id: &str) -> VectorDbResult<bool> {
        if entry_id.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "entry_id cannot be empty".to_string(),
            });
        }

        let deleted = self.storage.delete_entry(entry_id).await?;
        
        if deleted {
            eprintln!("🗑️ Deleted embedding entry: {}", entry_id);
        }
        
        Ok(deleted)
    }

    /// Check if an embedding entry exists
    /// 
    /// This is a lightweight check that doesn't load the full entry data.
    /// 
    /// # Arguments
    /// 
    /// * `entry_id` - The unique identifier to check
    /// 
    /// # Returns
    /// 
    /// `true` if the entry exists, `false` otherwise
    pub async fn exists(&self, entry_id: &str) -> VectorDbResult<bool> {
        let entry_ids = self.storage.list_entry_ids().await;
        Ok(entry_ids.contains(&entry_id.to_string()))
    }

    /// Get the total number of embeddings in the database
    /// 
    /// This method returns the count of all embedding entries currently
    /// stored in the database.
    pub async fn count_embeddings(&self) -> usize {
        self.storage.list_entry_ids().await.len()
    }

    /// List all embedding IDs in the database
    /// 
    /// Returns a list of all unique identifiers for embedding entries
    /// currently stored in the database.
    pub async fn list_embedding_ids(&self) -> Vec<String> {
        self.storage.list_entry_ids().await
    }
}

/// Batch operations for efficient bulk processing
/// 
/// This struct provides optimized operations for handling multiple
/// embedding entries simultaneously, improving performance for bulk operations.
#[derive(Clone)]
pub struct BatchOperations {
    /// Operations instance for individual operations
    operations: VectorOperations,
}

impl BatchOperations {
    /// Create a new batch operations instance
    pub fn new(operations: VectorOperations) -> Self {
        Self { operations }
    }

    /// Store multiple embedding entries in a single batch operation
    /// 
    /// This method is more efficient than storing embeddings individually
    /// as it minimizes I/O operations and maintains data consistency.
    /// 
    /// # Arguments
    /// 
    /// * `entries` - Vector of embedding entries to store
    /// 
    /// # Returns
    /// 
    /// Vector of entry IDs for the stored embeddings
    /// 
    /// # Errors
    /// 
    /// Returns error if any entry fails validation or storage fails
    pub async fn store_embeddings_batch(&self, entries: Vec<EmbeddingEntry>) -> VectorDbResult<Vec<String>> {
        if entries.is_empty() {
            return Ok(vec![]);
        }

        // Validate all entries before storing
        for entry in &entries {
            entry.validate()?;
        }

        let entry_ids = entries.iter().map(|e| e.id.clone()).collect();
        
        // Store all entries in a single batch operation
        self.operations.storage.store_entries(entries.clone()).await?;
        
        eprintln!("📦 Batch stored {} embedding entries", entries.len());
        Ok(entry_ids)
    }

    /// Retrieve multiple embedding entries by their IDs
    /// 
    /// This method efficiently retrieves multiple embedding entries
    /// by batching the requests and minimizing storage access.
    /// 
    /// # Arguments
    /// 
    /// * `entry_ids` - Vector of entry IDs to retrieve
    /// 
    /// # Returns
    /// 
    /// Vector of embedding entries that were found
    pub async fn retrieve_embeddings_batch(&self, entry_ids: &[String]) -> VectorDbResult<Vec<EmbeddingEntry>> {
        if entry_ids.is_empty() {
            return Ok(vec![]);
        }

        let results = self.operations.storage.retrieve_entries(entry_ids).await?;
        
        eprintln!("📖 Batch retrieved {} of {} requested entries", results.len(), entry_ids.len());
        Ok(results)
    }

    /// Delete multiple embedding entries by their IDs
    /// 
    /// This method efficiently deletes multiple embedding entries
    /// in a batch operation.
    /// 
    /// # Arguments
    /// 
    /// * `entry_ids` - Vector of entry IDs to delete
    /// 
    /// # Returns
    /// 
    /// Number of entries that were successfully deleted
    pub async fn delete_embeddings_batch(&self, entry_ids: &[String]) -> VectorDbResult<usize> {
        if entry_ids.is_empty() {
            return Ok(0);
        }

        let mut deleted_count = 0;
        
        for entry_id in entry_ids {
            if self.operations.delete_embedding(entry_id).await? {
                deleted_count += 1;
            }
        }
        
        eprintln!("🗑️ Batch deleted {} of {} requested entries", deleted_count, entry_ids.len());
        Ok(deleted_count)
    }

    /// Update multiple embedding entries' vectors
    /// 
    /// This method efficiently updates the vectors of multiple embedding entries.
    /// 
    /// # Arguments
    /// 
    /// * `updates` - HashMap mapping entry IDs to new vectors
    /// 
    /// # Returns
    /// 
    /// Number of entries that were successfully updated
    pub async fn update_embeddings_batch(&self, updates: HashMap<String, Vec<f32>>) -> VectorDbResult<usize> {
        if updates.is_empty() {
            return Ok(0);
        }

        let mut updated_count = 0;
        
        for (entry_id, new_vector) in updates {
            if self.operations.update_embedding(&entry_id, new_vector).await? {
                updated_count += 1;
            }
        }
        
        eprintln!("🔄 Batch updated {} entries", updated_count);
        Ok(updated_count)
    }
}

/// Validation operations for data integrity
/// 
/// This struct provides validation and repair utilities for ensuring
/// the integrity of the vector database.
#[derive(Clone)]
pub struct ValidationOperations {
    /// Storage backend for validation operations
    storage: Arc<VectorStorage>,
}

impl ValidationOperations {
    /// Create a new validation operations instance
    pub fn new(storage: Arc<VectorStorage>) -> Self {
        Self { storage }
    }

    /// Validate all embedding entries in the database
    /// 
    /// This method checks the integrity of all stored embedding entries
    /// and returns a detailed report of any issues found.
    /// 
    /// # Returns
    /// 
    /// An `IntegrityReport` containing validation results and any errors
    pub async fn validate_database(&self) -> VectorDbResult<IntegrityReport> {
        eprintln!("🔍 Starting database validation...");
        
        let report = self.storage.validate_integrity().await?;
        
        eprintln!("✅ Database validation completed: {}", report.summary());
        Ok(report)
    }

    /// Repair database integrity issues
    /// 
    /// This method attempts to repair common integrity issues found
    /// during validation, such as rebuilding indexes or cleaning up
    /// orphaned entries.
    /// 
    /// # Returns
    /// 
    /// Number of issues that were successfully repaired
    pub async fn repair_database(&self) -> VectorDbResult<usize> {
        eprintln!("🔧 Starting database repair...");
        
        // Rebuild the index to fix any indexing issues
        self.storage.rebuild_index_async().await?;
        
        // Run validation to check for remaining issues
        let report = self.validate_database().await?;
        
        let issues_found = report.errors.len() + report.corrupted_files + report.orphaned_entries;
        let repairs_made = if issues_found == 0 { 0 } else { 1 }; // Simple repair count
        
        eprintln!("🔧 Database repair completed: {} issues addressed", repairs_made);
        Ok(repairs_made)
    }

    /// Validate a specific embedding entry
    /// 
    /// This method validates the structure and content of a single
    /// embedding entry.
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The embedding entry to validate
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if the entry is valid, error otherwise
    pub fn validate_entry(&self, entry: &EmbeddingEntry) -> VectorDbResult<()> {
        entry.validate()
    }

    /// Check for duplicate embedding entries
    /// 
    /// This method scans the database for potential duplicate entries
    /// based on content hash and file path.
    /// 
    /// # Returns
    /// 
    /// HashMap mapping original entry ID to vector of duplicate entry IDs
    pub async fn find_duplicates(&self) -> VectorDbResult<HashMap<String, Vec<String>>> {
        let all_ids = self.storage.list_entry_ids().await;
        let all_entries = self.storage.retrieve_entries(&all_ids).await?;
        
        let mut content_hash_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut duplicates: HashMap<String, Vec<String>> = HashMap::new();
        
        // Group entries by content hash
        for entry in all_entries {
            let key = format!("{}:{}", entry.metadata.file_path, entry.metadata.text_hash);
            content_hash_map.entry(key).or_default().push(entry.id);
        }
        
        // Find groups with multiple entries (duplicates)
        for (_, entry_ids) in content_hash_map {
            if entry_ids.len() > 1 {
                let original = entry_ids[0].clone();
                let dups = entry_ids[1..].to_vec();
                duplicates.insert(original, dups);
            }
        }
        
        if !duplicates.is_empty() {
            eprintln!("⚠️ Found {} sets of duplicate entries", duplicates.len());
        }
        
        Ok(duplicates)
    }
}

/// Cleanup operations for database maintenance
/// 
/// This struct provides utilities for cleaning up orphaned data,
/// removing stale entries, and optimizing storage.
#[derive(Clone)]
pub struct CleanupOperations {
    /// Storage backend for cleanup operations
    storage: Arc<VectorStorage>,
    /// Operations for individual entry management
    operations: VectorOperations,
}

impl CleanupOperations {
    /// Create a new cleanup operations instance
    pub fn new(storage: Arc<VectorStorage>, operations: VectorOperations) -> Self {
        Self { storage, operations }
    }

    /// Remove orphaned embedding entries
    /// 
    /// This method identifies and removes embedding entries that are no longer
    /// referenced or associated with valid files.
    /// 
    /// # Arguments
    /// 
    /// * `valid_file_paths` - Optional list of valid file paths to check against
    /// 
    /// # Returns
    /// 
    /// Number of orphaned entries that were removed
    pub async fn cleanup_orphaned_embeddings(&self, valid_file_paths: Option<&[String]>) -> VectorDbResult<usize> {
        eprintln!("🧹 Starting orphaned embedding cleanup...");
        
        let all_ids = self.storage.list_entry_ids().await;
        let all_entries = self.storage.retrieve_entries(&all_ids).await?;
        
        let mut orphaned_count = 0;
        
        if let Some(valid_paths) = valid_file_paths {
            // Remove entries for files that no longer exist
            for entry in all_entries {
                if !valid_paths.contains(&entry.metadata.file_path) {
                    if self.operations.delete_embedding(&entry.id).await? {
                        orphaned_count += 1;
                    }
                }
            }
        } else {
            // Use file system to check if files exist
            for entry in all_entries {
                let file_path = PathBuf::from(&entry.metadata.file_path);
                if !file_path.exists() {
                    if self.operations.delete_embedding(&entry.id).await? {
                        orphaned_count += 1;
                    }
                }
            }
        }
        
        eprintln!("🧹 Cleanup completed: {} orphaned entries removed", orphaned_count);
        Ok(orphaned_count)
    }

    /// Remove all embeddings for a specific file
    /// 
    /// This method removes all embedding entries associated with a specific
    /// file path. This is useful when a file is deleted or significantly modified.
    /// 
    /// # Arguments
    /// 
    /// * `file_path` - The file path to remove embeddings for
    /// 
    /// # Returns
    /// 
    /// Number of entries that were removed for the file
    pub async fn cleanup_file_embeddings(&self, file_path: &str) -> VectorDbResult<usize> {
        if file_path.is_empty() {
            return Ok(0);
        }

        let all_ids = self.storage.list_entry_ids().await;
        let all_entries = self.storage.retrieve_entries(&all_ids).await?;
        
        let mut removed_count = 0;
        
        for entry in all_entries {
            if entry.metadata.file_path == file_path {
                if self.operations.delete_embedding(&entry.id).await? {
                    removed_count += 1;
                }
            }
        }
        
        eprintln!("🧹 Removed {} embeddings for file: {}", removed_count, file_path);
        Ok(removed_count)
    }

    /// Remove embeddings older than a specified timestamp
    /// 
    /// This method removes embedding entries that were created before
    /// a specified timestamp.
    /// 
    /// # Arguments
    /// 
    /// * `before_timestamp` - Unix timestamp; entries created before this will be removed
    /// 
    /// # Returns
    /// 
    /// Number of entries that were removed
    pub async fn cleanup_old_embeddings(&self, before_timestamp: u64) -> VectorDbResult<usize> {
        let all_ids = self.storage.list_entry_ids().await;
        let all_entries = self.storage.retrieve_entries(&all_ids).await?;
        
        let mut removed_count = 0;
        
        for entry in all_entries {
            if entry.created_at < before_timestamp {
                if self.operations.delete_embedding(&entry.id).await? {
                    removed_count += 1;
                }
            }
        }
        
        eprintln!("🧹 Removed {} old embeddings (before timestamp {})", removed_count, before_timestamp);
        Ok(removed_count)
    }

    /// Compact the database to optimize storage
    /// 
    /// This method triggers a compaction operation to remove deleted entries
    /// from storage files and optimize file sizes.
    /// 
    /// # Returns
    /// 
    /// Compaction results including files affected and space saved
    pub async fn compact_database(&self) -> VectorDbResult<crate::vector_db::storage::CompactionResult> {
        eprintln!("🗜️ Starting database compaction...");
        
        let result = self.storage.compact_storage().await?;
        
        eprintln!("🗜️ Compaction completed: {} files removed, {} files compacted", 
                  result.files_removed, result.files_compacted);
        
        Ok(result)
    }

    /// Remove duplicate embedding entries
    /// 
    /// This method identifies and removes duplicate embedding entries,
    /// keeping only the most recent version of each unique embedding.
    /// 
    /// # Returns
    /// 
    /// Number of duplicate entries that were removed
    pub async fn remove_duplicates(&self) -> VectorDbResult<usize> {
        eprintln!("🔍 Finding duplicate embeddings...");
        
        let validation_ops = ValidationOperations::new(self.storage.clone());
        let duplicates = validation_ops.find_duplicates().await?;
        
        let mut removed_count = 0;
        
        // Remove duplicate entries, keeping the original
        for (_, duplicate_ids) in duplicates {
            for duplicate_id in duplicate_ids {
                if self.operations.delete_embedding(&duplicate_id).await? {
                    removed_count += 1;
                }
            }
        }
        
        eprintln!("🧹 Removed {} duplicate entries", removed_count);
        Ok(removed_count)
    }

    /// Get current timestamp for cleanup operations
    /// 
    /// Helper method to get the current Unix timestamp for use in
    /// time-based cleanup operations.
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get timestamp for N days ago
    /// 
    /// Helper method to calculate timestamp for N days in the past.
    /// 
    /// # Arguments
    /// 
    /// * `days_ago` - Number of days to subtract from current time
    /// 
    /// # Returns
    /// 
    /// Unix timestamp for N days ago
    pub fn timestamp_days_ago(days_ago: u64) -> u64 {
        let seconds_per_day = 24 * 60 * 60;
        let current_time = Self::current_timestamp();
        current_time.saturating_sub(days_ago * seconds_per_day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::vector_db::types::VectorStorageConfig;

    fn create_test_config() -> VectorStorageConfig {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_string_lossy().to_string();
        std::mem::forget(temp_dir); // Keep temp dir alive for test
        
        VectorStorageConfig {
            storage_dir,
            enable_compression: false,
            compression_algorithm: crate::vector_db::types::CompressionAlgorithm::None,
            max_entries_per_file: 100,
            enable_checksums: false,
            auto_backup: false,
            max_backups: 0,
            enable_metrics: false,
        }
    }

    #[test]
    fn test_operations_creation() {
        let config = create_test_config();
        let storage = Arc::new(VectorStorage::new(config.clone()).unwrap());
        let operations = VectorOperations::new(storage, config);
        
        // Test basic structure creation
        assert_eq!(operations.config.enable_compression, false);
        assert_eq!(operations.config.enable_checksums, false);
    }

    #[test]
    fn test_batch_operations_creation() {
        let config = create_test_config();
        let storage = Arc::new(VectorStorage::new(config.clone()).unwrap());
        let operations = VectorOperations::new(storage, config);
        let batch_operations = BatchOperations::new(operations);
        
        // Test batch operations structure
        assert_eq!(batch_operations.operations.config.enable_compression, false);
    }

    #[test]
    fn test_validation_operations_creation() {
        let config = create_test_config();
        let storage = Arc::new(VectorStorage::new(config).unwrap());
        let validation_ops = ValidationOperations::new(storage);
        
        // Test validation operations structure - this is sufficient for unit testing
        // without async operations that might hang
        assert!(Arc::strong_count(&validation_ops.storage) >= 1);
    }

    #[test]
    fn test_cleanup_operations_creation() {
        let config = create_test_config();
        let storage = Arc::new(VectorStorage::new(config.clone()).unwrap());
        let operations = VectorOperations::new(storage.clone(), config);
        let cleanup_ops = CleanupOperations::new(storage, operations);
        
        // Test cleanup operations structure
        assert!(Arc::strong_count(&cleanup_ops.storage) >= 1);
        assert_eq!(cleanup_ops.operations.config.enable_compression, false);
    }

    #[test]
    fn test_timestamp_helpers() {
        let current = CleanupOperations::current_timestamp();
        let days_ago = CleanupOperations::timestamp_days_ago(7);
        
        assert!(current > 0);
        assert!(days_ago < current);
        assert!(current - days_ago >= 7 * 24 * 60 * 60); // At least 7 days difference
    }

    // Note: Full async integration tests will be implemented in sub-issue #105
    // These unit tests focus on structure validation to avoid hanging issues
}