//! Indexing System Module for Vector Database
//! 
//! This module provides efficient indexing mechanisms for fast retrieval 
//! of embedding entries. It implements various index types optimized for 
//! different query patterns and maintains index consistency with the storage layer.
//! 
//! ## Features
//! 
//! - **Multi-Index Support**: File path, model name, content hash indexes
//! - **Fast Lookups**: O(1) average case retrieval by various keys
//! - **Index Persistence**: Durable indexes that survive restarts
//! - **Consistency**: Automatic index updates with storage operations
//! - **Memory Efficient**: Optimized in-memory index structures
//! 
//! ## Index Types
//! 
//! - `FilePathIndex`: Maps file paths to embedding entry IDs
//! - `ModelNameIndex`: Maps model names to embedding entry IDs  
//! - `ContentHashIndex`: Maps content hashes to embedding entry IDs
//! - `ChunkIndex`: Maps chunk IDs within files to embedding entry IDs
//! - `TimestampIndex`: Maps time ranges to embedding entry IDs

use std::collections::{HashMap, BTreeMap};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::vector_db::types::{EmbeddingEntry, VectorDbError, VectorDbResult};
use crate::vector_db::storage::VectorStorage;

/// Comprehensive indexing system for fast embedding lookups
/// 
/// This struct maintains multiple indexes for different query patterns
/// and provides fast retrieval mechanisms for embedding entries.
pub struct IndexingSystem {
    /// File path to embedding IDs mapping
    file_path_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Model name to embedding IDs mapping
    model_name_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Content hash to embedding ID mapping (for deduplication)
    content_hash_index: Arc<RwLock<HashMap<String, String>>>,
    /// Chunk ID to embedding ID mapping (within file context)
    chunk_index: Arc<RwLock<HashMap<String, HashMap<String, String>>>>, // file_path -> (chunk_id -> entry_id)
    /// Timestamp range index for temporal queries
    timestamp_index: Arc<RwLock<BTreeMap<u64, Vec<String>>>>,
    /// Full entry ID to metadata mapping for quick lookups
    metadata_index: Arc<RwLock<HashMap<String, IndexMetadata>>>,
    /// Storage backend for data operations
    storage: Arc<VectorStorage>,
    /// Index persistence configuration
    persist_indexes: bool,
    /// Index file path
    index_file_path: String,
}

/// Lightweight metadata for index entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Entry ID
    pub entry_id: String,
    /// File path
    pub file_path: String,
    /// Chunk ID within file
    pub chunk_id: String,
    /// Model name used for embedding
    pub model_name: String,
    /// Content hash for deduplication
    pub content_hash: String,
    /// Entry creation timestamp
    pub created_at: u64,
    /// Entry last update timestamp
    pub updated_at: u64,
    /// Vector dimension count
    pub dimension: usize,
}

impl IndexMetadata {
    /// Create index metadata from an embedding entry
    pub fn from_entry(entry: &EmbeddingEntry) -> Self {
        Self {
            entry_id: entry.id.clone(),
            file_path: entry.metadata.file_path.clone(),
            chunk_id: entry.metadata.chunk_id.clone(),
            model_name: entry.metadata.model_name.clone(),
            content_hash: entry.metadata.text_hash.clone(),
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            dimension: entry.vector.len(),
        }
    }
}

/// Serializable index data for persistence
#[derive(Debug, Serialize, Deserialize)]
struct SerializableIndexData {
    /// File path index data
    file_path_index: HashMap<String, Vec<String>>,
    /// Model name index data
    model_name_index: HashMap<String, Vec<String>>,
    /// Content hash index data
    content_hash_index: HashMap<String, String>,
    /// Chunk index data
    chunk_index: HashMap<String, HashMap<String, String>>,
    /// Timestamp index data
    timestamp_index: BTreeMap<u64, Vec<String>>,
    /// Metadata index data
    metadata_index: HashMap<String, IndexMetadata>,
    /// Index creation timestamp
    created_at: u64,
    /// Index last update timestamp
    updated_at: u64,
}

impl IndexingSystem {
    /// Create a new indexing system
    /// 
    /// # Arguments
    /// 
    /// * `storage` - Storage backend for data operations
    /// * `persist_indexes` - Whether to persist indexes to disk
    /// * `index_file_path` - Path for index persistence file
    pub async fn new(
        storage: Arc<VectorStorage>,
        persist_indexes: bool,
        index_file_path: String,
    ) -> VectorDbResult<Self> {
        let indexing_system = Self {
            file_path_index: Arc::new(RwLock::new(HashMap::new())),
            model_name_index: Arc::new(RwLock::new(HashMap::new())),
            content_hash_index: Arc::new(RwLock::new(HashMap::new())),
            chunk_index: Arc::new(RwLock::new(HashMap::new())),
            timestamp_index: Arc::new(RwLock::new(BTreeMap::new())),
            metadata_index: Arc::new(RwLock::new(HashMap::new())),
            storage,
            persist_indexes,
            index_file_path,
        };

        // Try to load existing indexes if persistence is enabled
        if persist_indexes {
            if let Err(e) = indexing_system.load_indexes().await {
                eprintln!("⚠️ Failed to load existing indexes: {}, rebuilding...", e);
                indexing_system.rebuild_all_indexes().await?;
            }
        } else {
            // Build indexes from storage if not persisting
            indexing_system.rebuild_all_indexes().await?;
        }

        eprintln!("🔍 Indexing system initialized with {} entries", 
                  indexing_system.metadata_index.read().await.len());
        
        Ok(indexing_system)
    }

    /// Add an entry to all relevant indexes
    /// 
    /// This method updates all indexes when a new embedding entry is added
    /// to the storage system.
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The embedding entry to index
    pub async fn index_entry(&self, entry: &EmbeddingEntry) -> VectorDbResult<()> {
        let metadata = IndexMetadata::from_entry(entry);
        
        // Update file path index
        {
            let mut file_index = self.file_path_index.write().await;
            file_index
                .entry(entry.metadata.file_path.clone())
                .or_default()
                .push(entry.id.clone());
        }
        
        // Update model name index
        {
            let mut model_index = self.model_name_index.write().await;
            model_index
                .entry(entry.metadata.model_name.clone())
                .or_default()
                .push(entry.id.clone());
        }
        
        // Update content hash index (for deduplication)
        {
            let mut hash_index = self.content_hash_index.write().await;
            hash_index.insert(entry.metadata.text_hash.clone(), entry.id.clone());
        }
        
        // Update chunk index
        {
            let mut chunk_index = self.chunk_index.write().await;
            chunk_index
                .entry(entry.metadata.file_path.clone())
                .or_default()
                .insert(entry.metadata.chunk_id.clone(), entry.id.clone());
        }
        
        // Update timestamp index (group by hour for efficiency)
        {
            let hour_timestamp = (entry.created_at / 3600) * 3600; // Round to nearest hour
            let mut timestamp_index = self.timestamp_index.write().await;
            timestamp_index
                .entry(hour_timestamp)
                .or_default()
                .push(entry.id.clone());
        }
        
        // Update metadata index
        {
            let mut metadata_index = self.metadata_index.write().await;
            metadata_index.insert(entry.id.clone(), metadata);
        }
        
        // Persist indexes if enabled
        if self.persist_indexes {
            self.save_indexes().await?;
        }
        
        eprintln!("📇 Indexed entry: {} ({})", entry.id, entry.metadata.file_path);
        Ok(())
    }

    /// Remove an entry from all indexes
    /// 
    /// This method updates all indexes when an embedding entry is deleted
    /// from the storage system.
    /// 
    /// # Arguments
    /// 
    /// * `entry_id` - The ID of the entry to remove from indexes
    pub async fn remove_entry_from_indexes(&self, entry_id: &str) -> VectorDbResult<()> {
        // Get metadata before removal
        let metadata = {
            let metadata_index = self.metadata_index.read().await;
            metadata_index.get(entry_id).cloned()
        };
        
        if let Some(meta) = metadata {
            // Remove from file path index
            {
                let mut file_index = self.file_path_index.write().await;
                if let Some(file_entries) = file_index.get_mut(&meta.file_path) {
                    file_entries.retain(|id| id != entry_id);
                    if file_entries.is_empty() {
                        file_index.remove(&meta.file_path);
                    }
                }
            }
            
            // Remove from model name index
            {
                let mut model_index = self.model_name_index.write().await;
                if let Some(model_entries) = model_index.get_mut(&meta.model_name) {
                    model_entries.retain(|id| id != entry_id);
                    if model_entries.is_empty() {
                        model_index.remove(&meta.model_name);
                    }
                }
            }
            
            // Remove from content hash index
            {
                let mut hash_index = self.content_hash_index.write().await;
                hash_index.remove(&meta.content_hash);
            }
            
            // Remove from chunk index
            {
                let mut chunk_index = self.chunk_index.write().await;
                if let Some(file_chunks) = chunk_index.get_mut(&meta.file_path) {
                    file_chunks.remove(&meta.chunk_id);
                    if file_chunks.is_empty() {
                        chunk_index.remove(&meta.file_path);
                    }
                }
            }
            
            // Remove from timestamp index
            {
                let hour_timestamp = (meta.created_at / 3600) * 3600;
                let mut timestamp_index = self.timestamp_index.write().await;
                if let Some(hour_entries) = timestamp_index.get_mut(&hour_timestamp) {
                    hour_entries.retain(|id| id != entry_id);
                    if hour_entries.is_empty() {
                        timestamp_index.remove(&hour_timestamp);
                    }
                }
            }
        }
        
        // Remove from metadata index
        {
            let mut metadata_index = self.metadata_index.write().await;
            metadata_index.remove(entry_id);
        }
        
        // Persist indexes if enabled
        if self.persist_indexes {
            self.save_indexes().await?;
        }
        
        eprintln!("📇 Removed entry from indexes: {}", entry_id);
        Ok(())
    }

    /// Find embedding entries by file path
    /// 
    /// # Arguments
    /// 
    /// * `file_path` - The file path to search for
    /// 
    /// # Returns
    /// 
    /// Vector of entry IDs associated with the file path
    pub async fn find_by_file_path(&self, file_path: &str) -> Vec<String> {
        let file_index = self.file_path_index.read().await;
        file_index.get(file_path).cloned().unwrap_or_default()
    }

    /// Find embedding entries by model name
    /// 
    /// # Arguments
    /// 
    /// * `model_name` - The model name to search for
    /// 
    /// # Returns
    /// 
    /// Vector of entry IDs created with the specified model
    pub async fn find_by_model_name(&self, model_name: &str) -> Vec<String> {
        let model_index = self.model_name_index.read().await;
        model_index.get(model_name).cloned().unwrap_or_default()
    }

    /// Find embedding entry by content hash (for deduplication)
    /// 
    /// # Arguments
    /// 
    /// * `content_hash` - The content hash to search for
    /// 
    /// # Returns
    /// 
    /// Optional entry ID with matching content hash
    pub async fn find_by_content_hash(&self, content_hash: &str) -> Option<String> {
        let hash_index = self.content_hash_index.read().await;
        hash_index.get(content_hash).cloned()
    }

    /// Find embedding entry by chunk ID within a file
    /// 
    /// # Arguments
    /// 
    /// * `file_path` - The file path containing the chunk
    /// * `chunk_id` - The chunk ID to search for
    /// 
    /// # Returns
    /// 
    /// Optional entry ID for the specified chunk
    pub async fn find_by_chunk(&self, file_path: &str, chunk_id: &str) -> Option<String> {
        let chunk_index = self.chunk_index.read().await;
        chunk_index
            .get(file_path)?
            .get(chunk_id)
            .cloned()
    }

    /// Find embedding entries by timestamp range
    /// 
    /// # Arguments
    /// 
    /// * `start_timestamp` - Start of the time range (inclusive)
    /// * `end_timestamp` - End of the time range (inclusive)
    /// 
    /// # Returns
    /// 
    /// Vector of entry IDs created within the specified time range
    pub async fn find_by_timestamp_range(&self, start_timestamp: u64, end_timestamp: u64) -> Vec<String> {
        let timestamp_index = self.timestamp_index.read().await;
        let start_hour = (start_timestamp / 3600) * 3600;
        let end_hour = (end_timestamp / 3600) * 3600;
        
        let mut result = Vec::new();
        
        // Collect all entries from relevant hour buckets
        for entry_ids in timestamp_index.range(start_hour..=end_hour).map(|(_, v)| v) {
            result.extend_from_slice(entry_ids);
        }
        
        // Filter by exact timestamp range using metadata
        let metadata_index = self.metadata_index.read().await;
        result.retain(|entry_id| {
            if let Some(metadata) = metadata_index.get(entry_id) {
                metadata.created_at >= start_timestamp && metadata.created_at <= end_timestamp
            } else {
                false
            }
        });
        
        result
    }

    /// Get all indexed file paths
    /// 
    /// # Returns
    /// 
    /// Vector of all file paths that have associated embeddings
    pub async fn get_indexed_file_paths(&self) -> Vec<String> {
        let file_index = self.file_path_index.read().await;
        file_index.keys().cloned().collect()
    }

    /// Get all indexed model names
    /// 
    /// # Returns
    /// 
    /// Vector of all model names that have been used to create embeddings
    pub async fn get_indexed_model_names(&self) -> Vec<String> {
        let model_index = self.model_name_index.read().await;
        model_index.keys().cloned().collect()
    }

    /// Get metadata for a specific entry
    /// 
    /// # Arguments
    /// 
    /// * `entry_id` - The entry ID to get metadata for
    /// 
    /// # Returns
    /// 
    /// Optional index metadata for the entry
    pub async fn get_entry_metadata(&self, entry_id: &str) -> Option<IndexMetadata> {
        let metadata_index = self.metadata_index.read().await;
        metadata_index.get(entry_id).cloned()
    }

    /// Get index statistics
    /// 
    /// # Returns
    /// 
    /// Index statistics including entry counts and memory usage estimates
    pub async fn get_index_stats(&self) -> IndexStats {
        let file_index = self.file_path_index.read().await;
        let model_index = self.model_name_index.read().await;
        let hash_index = self.content_hash_index.read().await;
        let chunk_index = self.chunk_index.read().await;
        let timestamp_index = self.timestamp_index.read().await;
        let metadata_index = self.metadata_index.read().await;
        
        IndexStats {
            total_entries: metadata_index.len(),
            unique_file_paths: file_index.len(),
            unique_model_names: model_index.len(),
            unique_content_hashes: hash_index.len(),
            timestamp_buckets: timestamp_index.len(),
            memory_usage_estimate: Self::estimate_memory_usage(
                &file_index,
                &model_index,
                &hash_index,
                &chunk_index,
                &timestamp_index,
                &metadata_index,
            ),
        }
    }

    /// Rebuild all indexes from storage
    /// 
    /// This method scans all entries in storage and rebuilds all indexes.
    /// This is useful for recovery or initialization scenarios.
    pub async fn rebuild_all_indexes(&self) -> VectorDbResult<()> {
        eprintln!("🔄 Rebuilding all indexes from storage...");
        
        // Clear all existing indexes
        self.clear_all_indexes().await;
        
        // Get all entry IDs from storage
        let entry_ids = self.storage.list_entry_ids().await;
        
        if entry_ids.is_empty() {
            eprintln!("📇 No entries found in storage, indexes are empty");
            return Ok(());
        }
        
        // Retrieve all entries and rebuild indexes
        let entries = self.storage.retrieve_entries(&entry_ids).await?;
        
        for entry in entries {
            self.index_entry(&entry).await?;
        }
        
        eprintln!("✅ Rebuilt indexes for {} entries", entry_ids.len());
        Ok(())
    }

    /// Clear all indexes
    async fn clear_all_indexes(&self) {
        self.file_path_index.write().await.clear();
        self.model_name_index.write().await.clear();
        self.content_hash_index.write().await.clear();
        self.chunk_index.write().await.clear();
        self.timestamp_index.write().await.clear();
        self.metadata_index.write().await.clear();
    }

    /// Save indexes to disk (if persistence is enabled)
    async fn save_indexes(&self) -> VectorDbResult<()> {
        if !self.persist_indexes {
            return Ok(());
        }

        let index_data = SerializableIndexData {
            file_path_index: self.file_path_index.read().await.clone(),
            model_name_index: self.model_name_index.read().await.clone(),
            content_hash_index: self.content_hash_index.read().await.clone(),
            chunk_index: self.chunk_index.read().await.clone(),
            timestamp_index: self.timestamp_index.read().await.clone(),
            metadata_index: self.metadata_index.read().await.clone(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let serialized = serde_json::to_string_pretty(&index_data)?;
        fs::write(&self.index_file_path, serialized).map_err(|e| VectorDbError::Storage {
            message: format!("Failed to save indexes: {}", e),
        })?;

        eprintln!("💾 Saved indexes to {}", self.index_file_path);
        Ok(())
    }

    /// Load indexes from disk
    async fn load_indexes(&self) -> VectorDbResult<()> {
        if !Path::new(&self.index_file_path).exists() {
            return Err(VectorDbError::Storage {
                message: "Index file does not exist".to_string(),
            });
        }

        let content = fs::read_to_string(&self.index_file_path).map_err(|e| VectorDbError::Storage {
            message: format!("Failed to read index file: {}", e),
        })?;

        let index_data: SerializableIndexData = serde_json::from_str(&content)?;

        // Load data into indexes
        *self.file_path_index.write().await = index_data.file_path_index;
        *self.model_name_index.write().await = index_data.model_name_index;
        *self.content_hash_index.write().await = index_data.content_hash_index;
        *self.chunk_index.write().await = index_data.chunk_index;
        *self.timestamp_index.write().await = index_data.timestamp_index;
        *self.metadata_index.write().await = index_data.metadata_index;

        eprintln!("📂 Loaded indexes from {}", self.index_file_path);
        Ok(())
    }

    /// Estimate memory usage of indexes
    fn estimate_memory_usage(
        file_index: &HashMap<String, Vec<String>>,
        model_index: &HashMap<String, Vec<String>>,
        hash_index: &HashMap<String, String>,
        chunk_index: &HashMap<String, HashMap<String, String>>,
        timestamp_index: &BTreeMap<u64, Vec<String>>,
        metadata_index: &HashMap<String, IndexMetadata>,
    ) -> usize {
        let mut total = 0;

        // File path index
        total += file_index.keys().map(|k| k.len()).sum::<usize>();
        total += file_index.values().map(|v| v.iter().map(|s| s.len()).sum::<usize>()).sum::<usize>();

        // Model name index
        total += model_index.keys().map(|k| k.len()).sum::<usize>();
        total += model_index.values().map(|v| v.iter().map(|s| s.len()).sum::<usize>()).sum::<usize>();

        // Content hash index
        total += hash_index.keys().map(|k| k.len()).sum::<usize>();
        total += hash_index.values().map(|v| v.len()).sum::<usize>();

        // Chunk index
        for (file_path, chunks) in chunk_index {
            total += file_path.len();
            for (chunk_id, entry_id) in chunks {
                total += chunk_id.len() + entry_id.len();
            }
        }

        // Timestamp index  
        total += timestamp_index.len() * 8; // u64 keys
        total += timestamp_index.values().map(|v| v.iter().map(|s| s.len()).sum::<usize>()).sum::<usize>();

        // Metadata index
        total += metadata_index.keys().map(|k| k.len()).sum::<usize>();
        total += metadata_index.len() * std::mem::size_of::<IndexMetadata>();

        total
    }
}

/// Statistics about the indexing system
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Total number of indexed entries
    pub total_entries: usize,
    /// Number of unique file paths
    pub unique_file_paths: usize,
    /// Number of unique model names
    pub unique_model_names: usize,
    /// Number of unique content hashes
    pub unique_content_hashes: usize,
    /// Number of timestamp buckets
    pub timestamp_buckets: usize,
    /// Estimated memory usage in bytes
    pub memory_usage_estimate: usize,
}

impl IndexStats {
    /// Generate a human-readable summary of index statistics
    pub fn summary(&self) -> String {
        format!(
            "Index Stats: {} entries, {} files, {} models, {:.1} KB memory",
            self.total_entries,
            self.unique_file_paths,
            self.unique_model_names,
            self.memory_usage_estimate as f64 / 1024.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::vector_db::types::{VectorStorageConfig, CompressionAlgorithm};

    #[allow(dead_code)]
    fn create_test_config() -> VectorStorageConfig {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_string_lossy().to_string();
        std::mem::forget(temp_dir); // Keep temp dir alive for test
        
        VectorStorageConfig {
            storage_dir,
            enable_compression: false,
            compression_algorithm: CompressionAlgorithm::None,
            max_entries_per_file: 100,
            enable_checksums: false,
            auto_backup: false,
            max_backups: 0,
            enable_metrics: false,
        }
    }

    #[test]
    fn test_index_metadata_creation() {
        let entry = EmbeddingEntry::new(
            vec![0.1, 0.2, 0.3, 0.4, 0.5],
            "/test/file.md".to_string(),
            "chunk_1".to_string(),
            "Test text content",
            "test-model".to_string(),
        );

        let metadata = IndexMetadata::from_entry(&entry);
        
        assert_eq!(metadata.entry_id, entry.id);
        assert_eq!(metadata.file_path, "/test/file.md");
        assert_eq!(metadata.chunk_id, "chunk_1");
        assert_eq!(metadata.model_name, "test-model");
        assert_eq!(metadata.dimension, 5);
        assert!(!metadata.content_hash.is_empty());
    }

    #[test]
    fn test_index_stats_summary() {
        let stats = IndexStats {
            total_entries: 100,
            unique_file_paths: 25,
            unique_model_names: 3,
            unique_content_hashes: 95,
            timestamp_buckets: 24,
            memory_usage_estimate: 2048,
        };

        let summary = stats.summary();
        assert!(summary.contains("100 entries"));
        assert!(summary.contains("25 files"));
        assert!(summary.contains("3 models"));
        assert!(summary.contains("2.0 KB"));
    }

    // Note: Async integration tests for the indexing system will be implemented
    // in sub-issue #105 to avoid hanging issues during development. These unit
    // tests focus on data structure validation and basic functionality.
}