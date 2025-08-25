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
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod types;
pub mod storage;

use types::{EmbeddingEntry, StorageMetrics, VectorStorageConfig, VectorDbResult};
use storage::{VectorStorage, CompactionResult, IntegrityReport};

/// High-level vector database interface
/// 
/// This struct provides the main API for interacting with the vector storage system.
/// It manages the underlying storage, handles indexing, and provides convenient
/// methods for common operations.
pub struct VectorDatabase {
    /// Underlying storage engine
    storage: VectorStorage,
    /// Database configuration
    config: VectorStorageConfig,
    /// In-memory cache for frequently accessed entries
    cache: Arc<RwLock<HashMap<String, EmbeddingEntry>>>,
    /// Cache configuration
    cache_max_size: usize,
}

impl VectorDatabase {
    /// Create a new vector database with the given configuration
    pub async fn new(config: VectorStorageConfig) -> VectorDbResult<Self> {
        let storage = VectorStorage::new(config.clone())?;
        let cache_max_size = 100; // Cache up to 100 frequently accessed entries
        
        Ok(Self {
            storage,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_max_size,
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

// Re-export main types for convenience
pub use types::{
    EmbeddingMetadata,
    CompressionAlgorithm,
};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_database() -> VectorDatabase {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_string_lossy().to_string();
        
        // Don't drop temp_dir to keep it alive during tests
        std::mem::forget(temp_dir);
        
        VectorDatabase::with_default_config(storage_dir).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_database_creation() {
        let db = create_test_database().await;
        assert!(db.is_empty().await);
        assert_eq!(db.count_embeddings().await, 0);
    }
    
    #[tokio::test]
    async fn test_store_and_retrieve_embedding() {
        let db = create_test_database().await;
        
        let vector = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let text = "This is a test document for embedding";
        
        // Store embedding
        let entry_id = db.store_embedding(
            vector.clone(),
            "/test/document.md",
            "chunk_1",
            text,
            "test-model",
        ).await.unwrap();
        
        assert!(!entry_id.is_empty());
        assert!(!db.is_empty().await);
        assert_eq!(db.count_embeddings().await, 1);
        
        // Retrieve embedding
        let retrieved = db.retrieve_embedding(&entry_id).await.unwrap();
        assert!(retrieved.is_some());
        
        let entry = retrieved.unwrap();
        assert_eq!(entry.id, entry_id);
        assert_eq!(entry.vector, vector);
        assert_eq!(entry.metadata.file_path, "/test/document.md");
        assert_eq!(entry.metadata.model_name, "test-model");
    }
    
    #[tokio::test]
    async fn test_store_batch_embeddings() {
        let db = create_test_database().await;
        
        let entries = vec![
            EmbeddingEntry::new(
                vec![0.1, 0.2, 0.3],
                "/test/doc1.md".to_string(),
                "chunk_1".to_string(),
                "First document",
                "test-model".to_string(),
            ),
            EmbeddingEntry::new(
                vec![0.4, 0.5, 0.6],
                "/test/doc2.md".to_string(),
                "chunk_1".to_string(),
                "Second document",
                "test-model".to_string(),
            ),
        ];
        
        let expected_ids = entries.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
        
        // Store batch
        let stored_ids = db.store_embeddings_batch(entries).await.unwrap();
        assert_eq!(stored_ids, expected_ids);
        assert_eq!(db.count_embeddings().await, 2);
        
        // Retrieve batch
        let retrieved = db.retrieve_embeddings(&expected_ids).await.unwrap();
        assert_eq!(retrieved.len(), 2);
    }
    
    #[tokio::test]
    async fn test_update_embedding() {
        let db = create_test_database().await;
        
        // Store initial embedding
        let entry_id = db.store_embedding(
            vec![0.1, 0.2, 0.3],
            "/test/doc.md",
            "chunk_1",
            "Test document",
            "test-model",
        ).await.unwrap();
        
        // Update with new vector
        let new_vector = vec![0.4, 0.5, 0.6];
        let updated = db.update_embedding(&entry_id, new_vector.clone()).await.unwrap();
        assert!(updated);
        
        // Verify update
        let retrieved = db.retrieve_embedding(&entry_id).await.unwrap().unwrap();
        assert_eq!(retrieved.vector, new_vector);
    }
    
    #[tokio::test]
    async fn test_delete_embedding() {
        let db = create_test_database().await;
        
        // Store embedding
        let entry_id = db.store_embedding(
            vec![0.1, 0.2, 0.3],
            "/test/doc.md",
            "chunk_1",
            "Test document",
            "test-model",
        ).await.unwrap();
        
        assert_eq!(db.count_embeddings().await, 1);
        
        // Delete embedding
        let deleted = db.delete_embedding(&entry_id).await.unwrap();
        assert!(deleted);
        
        // Verify deletion
        let retrieved = db.retrieve_embedding(&entry_id).await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_find_embeddings_by_file() {
        let db = create_test_database().await;
        
        // Store embeddings for different files
        let file1_id = db.store_embedding(
            vec![0.1, 0.2, 0.3],
            "/test/file1.md",
            "chunk_1",
            "Content from file 1",
            "test-model",
        ).await.unwrap();
        
        let file2_id = db.store_embedding(
            vec![0.4, 0.5, 0.6],
            "/test/file2.md",
            "chunk_1",
            "Content from file 2",
            "test-model",
        ).await.unwrap();
        
        let file1_id2 = db.store_embedding(
            vec![0.7, 0.8, 0.9],
            "/test/file1.md",
            "chunk_2",
            "More content from file 1",
            "test-model",
        ).await.unwrap();
        
        // Find embeddings for file1
        let file1_embeddings = db.find_embeddings_by_file("/test/file1.md").await.unwrap();
        assert_eq!(file1_embeddings.len(), 2);
        
        let file1_ids: Vec<String> = file1_embeddings.iter().map(|e| e.id.clone()).collect();
        assert!(file1_ids.contains(&file1_id));
        assert!(file1_ids.contains(&file1_id2));
        assert!(!file1_ids.contains(&file2_id));
    }
    
    #[tokio::test]
    async fn test_delete_embeddings_by_file() {
        let db = create_test_database().await;
        
        // Store embeddings for different files
        db.store_embedding(
            vec![0.1, 0.2, 0.3],
            "/test/file1.md",
            "chunk_1",
            "Content 1",
            "test-model",
        ).await.unwrap();
        
        db.store_embedding(
            vec![0.4, 0.5, 0.6],
            "/test/file2.md",
            "chunk_1",
            "Content 2",
            "test-model",
        ).await.unwrap();
        
        db.store_embedding(
            vec![0.7, 0.8, 0.9],
            "/test/file1.md",
            "chunk_2",
            "Content 3",
            "test-model",
        ).await.unwrap();
        
        assert_eq!(db.count_embeddings().await, 3);
        
        // Delete all embeddings for file1
        let deleted_count = db.delete_embeddings_by_file("/test/file1.md").await.unwrap();
        assert_eq!(deleted_count, 2);
        assert_eq!(db.count_embeddings().await, 1);
        
        // Verify only file2 embedding remains
        let remaining = db.find_embeddings_by_file("/test/file2.md").await.unwrap();
        assert_eq!(remaining.len(), 1);
    }
    
    #[tokio::test]
    async fn test_database_metrics() {
        let db = create_test_database().await;
        
        // Store some embeddings
        for i in 0..5 {
            db.store_embedding(
                vec![0.1 * i as f32, 0.2 * i as f32, 0.3 * i as f32],
                format!("/test/file{}.md", i),
                "chunk_1",
                &format!("Content {}", i),
                "test-model",
            ).await.unwrap();
        }
        
        let metrics = db.get_metrics().await.unwrap();
        assert_eq!(metrics.total_embeddings(), 5);
        assert!(metrics.cache_utilization() > 0.0);
        
        let summary = metrics.summary();
        assert!(summary.contains("5 embeddings"));
    }
}