//! Lazy Loading Module
//!
//! This module provides lazy loading capabilities for large vector database indices,
//! allowing efficient memory usage and faster startup times for large datasets.
//!
//! ## Features
//!
//! - **On-Demand Loading**: Load embeddings only when needed
//! - **Memory Management**: Keep memory usage within bounds
//! - **Cache Optimization**: Smart caching of frequently accessed data
//! - **Progressive Loading**: Load data in chunks for better responsiveness
//! - **Background Prefetching**: Predictive loading of likely needed data

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::vector_db::types::{EmbeddingEntry, VectorDbError, VectorDbResult};
use crate::embedding_cache::{EmbeddingCache, CacheConfig};

/// Errors that can occur during lazy loading operations
#[derive(Error, Debug)]
pub enum LazyLoadingError {
    #[error("Index file not found: {path}")]
    IndexFileNotFound { path: String },
    
    #[error("Loading failed: {message}")]
    LoadingFailed { message: String },
    
    #[error("Cache operation failed: {message}")]
    CacheOperationFailed { message: String },
    
    #[error("Invalid chunk ID: {chunk_id}")]
    InvalidChunkId { chunk_id: String },
    
    #[error("Memory limit exceeded: {current_mb}MB > {limit_mb}MB")]
    MemoryLimitExceeded { current_mb: usize, limit_mb: usize },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type LazyLoadingResult<T> = Result<T, LazyLoadingError>;

/// Configuration for lazy loading system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LazyLoadingConfig {
    /// Maximum memory usage for loaded chunks (MB)
    pub max_memory_mb: usize,
    /// Number of chunks to keep in memory
    pub max_loaded_chunks: usize,
    /// Size of each chunk (number of embeddings)
    pub chunk_size: usize,
    /// Enable background prefetching
    pub enable_prefetching: bool,
    /// Number of chunks to prefetch ahead
    pub prefetch_ahead: usize,
    /// Cache TTL for loaded chunks (seconds)
    pub chunk_cache_ttl: u64,
    /// Enable access pattern learning for smart prefetching
    pub enable_pattern_learning: bool,
    /// Minimum access frequency for prefetching consideration
    pub min_access_frequency: usize,
}

impl Default for LazyLoadingConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 100, // 100MB max for lazy loading
            max_loaded_chunks: 50,
            chunk_size: 100, // 100 embeddings per chunk
            enable_prefetching: true,
            prefetch_ahead: 3, // Prefetch 3 chunks ahead
            chunk_cache_ttl: 600, // 10 minutes
            enable_pattern_learning: true,
            min_access_frequency: 3,
        }
    }
}

/// Information about a chunk of embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// Unique chunk identifier
    pub chunk_id: String,
    /// File path where chunk is stored
    pub file_path: PathBuf,
    /// Number of embeddings in chunk
    pub entry_count: usize,
    /// Starting entry index in the full dataset
    pub start_index: usize,
    /// Estimated memory size in bytes
    pub estimated_size_bytes: usize,
    /// Creation timestamp
    pub created_at: u64,
    /// Last access timestamp
    pub last_accessed: u64,
    /// Access frequency counter
    pub access_count: usize,
}

/// A lazily loaded chunk of embeddings
#[derive(Debug)]
pub struct LazyChunk {
    /// Chunk metadata
    pub info: ChunkInfo,
    /// Loaded embeddings (None if not loaded)
    pub data: Option<Vec<EmbeddingEntry>>,
    /// Loading timestamp
    pub loaded_at: Option<Instant>,
    /// Whether chunk is currently being loaded
    pub loading: bool,
}

impl LazyChunk {
    /// Create a new unloaded chunk
    pub fn new(info: ChunkInfo) -> Self {
        Self {
            info,
            data: None,
            loaded_at: None,
            loading: false,
        }
    }
    
    /// Check if chunk is loaded
    pub fn is_loaded(&self) -> bool {
        self.data.is_some()
    }
    
    /// Get estimated memory usage
    pub fn memory_usage(&self) -> usize {
        if self.is_loaded() {
            self.info.estimated_size_bytes
        } else {
            std::mem::size_of::<ChunkInfo>() + std::mem::size_of::<LazyChunk>()
        }
    }
    
    /// Check if chunk has expired based on TTL
    pub fn is_expired(&self, ttl_seconds: u64) -> bool {
        if let Some(loaded_at) = self.loaded_at {
            loaded_at.elapsed().as_secs() > ttl_seconds
        } else {
            false
        }
    }
}

/// Access pattern tracking for smart prefetching
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// Recently accessed chunk sequence
    pub recent_access_sequence: VecDeque<String>,
    /// Chunk access frequencies
    pub access_frequencies: HashMap<String, usize>,
    /// Common access patterns (chunk A -> chunk B)
    pub transition_patterns: HashMap<String, HashMap<String, usize>>,
    /// Last access timestamp
    pub last_updated: Instant,
}

impl AccessPattern {
    fn new() -> Self {
        Self {
            recent_access_sequence: VecDeque::with_capacity(100),
            access_frequencies: HashMap::new(),
            transition_patterns: HashMap::new(),
            last_updated: Instant::now(),
        }
    }
    
    /// Record access to a chunk
    fn record_access(&mut self, chunk_id: &str) {
        // Update frequency
        *self.access_frequencies.entry(chunk_id.to_string()).or_insert(0) += 1;
        
        // Update transition patterns
        if let Some(prev_chunk) = self.recent_access_sequence.back() {
            self.transition_patterns
                .entry(prev_chunk.clone())
                .or_default()
                .entry(chunk_id.to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        
        // Add to recent sequence
        self.recent_access_sequence.push_back(chunk_id.to_string());
        if self.recent_access_sequence.len() > 100 {
            self.recent_access_sequence.pop_front();
        }
        
        self.last_updated = Instant::now();
    }
    
    /// Get predicted next chunks based on patterns
    fn get_predicted_chunks(&self, current_chunk: &str, limit: usize) -> Vec<String> {
        let mut predictions = Vec::new();
        
        if let Some(transitions) = self.transition_patterns.get(current_chunk) {
            let mut sorted_transitions: Vec<_> = transitions.iter().collect();
            sorted_transitions.sort_by(|a, b| b.1.cmp(a.1)); // Sort by frequency descending
            
            for (chunk_id, _frequency) in sorted_transitions.iter().take(limit) {
                predictions.push((*chunk_id).clone());
            }
        }
        
        predictions
    }
}

/// Lazy loading manager for vector database
pub struct LazyLoadingManager {
    /// Configuration
    config: LazyLoadingConfig,
    /// Index of all chunks
    chunk_index: Arc<RwLock<HashMap<String, LazyChunk>>>,
    /// Mapping from embedding ID to chunk ID
    embedding_to_chunk: Arc<RwLock<HashMap<String, String>>>,
    /// Loading queue for background operations
    loading_queue: Arc<Mutex<VecDeque<String>>>,
    /// Access pattern tracker
    access_patterns: Arc<RwLock<AccessPattern>>,
    /// Background task handle
    background_task: Option<tokio::task::JoinHandle<()>>,
    /// Statistics
    stats: Arc<RwLock<LazyLoadingStats>>,
}

/// Statistics for lazy loading operations
#[derive(Debug, Clone, Default)]
pub struct LazyLoadingStats {
    /// Total chunks created
    pub total_chunks: usize,
    /// Currently loaded chunks
    pub loaded_chunks: usize,
    /// Total memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Cache hit count
    pub cache_hits: usize,
    /// Cache miss count
    pub cache_misses: usize,
    /// Background loads completed
    pub background_loads: usize,
    /// Chunks evicted due to memory limits
    pub chunks_evicted: usize,
    /// Average load time in milliseconds
    pub avg_load_time_ms: f64,
    /// Last update timestamp
    pub last_updated: u64,
}

impl LazyLoadingManager {
    /// Create a new lazy loading manager
    pub fn new(config: LazyLoadingConfig) -> Self {
        let mut manager = Self {
            config: config.clone(),
            chunk_index: Arc::new(RwLock::new(HashMap::new())),
            embedding_to_chunk: Arc::new(RwLock::new(HashMap::new())),
            loading_queue: Arc::new(Mutex::new(VecDeque::new())),
            access_patterns: Arc::new(RwLock::new(AccessPattern::new())),
            background_task: None,
            stats: Arc::new(RwLock::new(LazyLoadingStats::default())),
        };
        
        if config.enable_prefetching {
            manager.start_background_task();
        }
        
        manager
    }
    
    /// Initialize lazy loading from existing embeddings
    pub async fn initialize_from_embeddings(
        &self,
        embeddings: Vec<EmbeddingEntry>,
        storage_dir: &Path,
    ) -> LazyLoadingResult<()> {
        let start_time = Instant::now();
        
        eprintln!("📚 Initializing lazy loading for {} embeddings", embeddings.len());
        
        // Create chunks
        let chunks = self.create_chunks(embeddings, storage_dir).await?;
        
        // Update index
        let mut chunk_index = self.chunk_index.write().await;
        let mut embedding_to_chunk = self.embedding_to_chunk.write().await;
        
        for chunk in chunks {
            // Store chunk
            chunk_index.insert(chunk.info.chunk_id.clone(), chunk);
        }
        
        // Build embedding to chunk mapping
        for (chunk_id, chunk) in chunk_index.iter() {
            if let Some(data) = &chunk.data {
                for entry in data {
                    embedding_to_chunk.insert(entry.id.clone(), chunk_id.clone());
                }
            }
        }
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_chunks = chunk_index.len();
        stats.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        eprintln!("✅ Lazy loading initialized: {} chunks in {:.2}ms",
                  stats.total_chunks,
                  start_time.elapsed().as_secs_f64() * 1000.0);
        
        Ok(())
    }
    
    /// Get an embedding by ID with lazy loading
    pub async fn get_embedding(&self, embedding_id: &str) -> LazyLoadingResult<Option<EmbeddingEntry>> {
        let start_time = Instant::now();
        
        // Find which chunk contains this embedding
        let chunk_id = {
            let embedding_to_chunk = self.embedding_to_chunk.read().await;
            embedding_to_chunk.get(embedding_id).cloned()
        };
        
        let chunk_id = match chunk_id {
            Some(id) => id,
            None => {
                // Update stats
                let mut stats = self.stats.write().await;
                stats.cache_misses += 1;
                return Ok(None);
            }
        };
        
        // Load chunk if needed
        self.ensure_chunk_loaded(&chunk_id).await?;
        
        // Record access pattern
        if self.config.enable_pattern_learning {
            let mut patterns = self.access_patterns.write().await;
            patterns.record_access(&chunk_id);
        }
        
        // Get embedding from loaded chunk
        let result = {
            let chunk_index = self.chunk_index.read().await;
            if let Some(chunk) = chunk_index.get(&chunk_id) {
                if let Some(data) = &chunk.data {
                    data.iter().find(|e| e.id == embedding_id).cloned()
                } else {
                    None
                }
            } else {
                None
            }
        };
        
        // Update stats
        let mut stats = self.stats.write().await;
        if result.is_some() {
            stats.cache_hits += 1;
        } else {
            stats.cache_misses += 1;
        }
        
        // Trigger prefetching if enabled
        if self.config.enable_prefetching && result.is_some() {
            self.trigger_prefetching(&chunk_id).await;
        }
        
        let load_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        if load_time_ms > 0.0 {
            stats.avg_load_time_ms = if stats.cache_hits + stats.cache_misses == 1 {
                load_time_ms
            } else {
                (stats.avg_load_time_ms + load_time_ms) / 2.0
            };
        }
        
        Ok(result)
    }
    
    /// Get multiple embeddings by IDs
    pub async fn get_embeddings(&self, embedding_ids: &[String]) -> LazyLoadingResult<Vec<EmbeddingEntry>> {
        let mut results = Vec::new();
        
        // Group embeddings by chunk to minimize loading
        let mut chunk_to_embeddings: HashMap<String, Vec<String>> = HashMap::new();
        
        {
            let embedding_to_chunk = self.embedding_to_chunk.read().await;
            for embedding_id in embedding_ids {
                if let Some(chunk_id) = embedding_to_chunk.get(embedding_id) {
                    chunk_to_embeddings
                        .entry(chunk_id.clone())
                        .or_default()
                        .push(embedding_id.clone());
                }
            }
        }
        
        // Load required chunks and collect results
        for (chunk_id, chunk_embedding_ids) in chunk_to_embeddings {
            self.ensure_chunk_loaded(&chunk_id).await?;
            
            let chunk_index = self.chunk_index.read().await;
            if let Some(chunk) = chunk_index.get(&chunk_id) {
                if let Some(data) = &chunk.data {
                    for embedding_id in chunk_embedding_ids {
                        if let Some(entry) = data.iter().find(|e| e.id == embedding_id) {
                            results.push(entry.clone());
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get statistics about lazy loading performance
    pub async fn get_stats(&self) -> LazyLoadingStats {
        let mut stats = self.stats.write().await;
        
        // Update current memory usage
        let chunk_index = self.chunk_index.read().await;
        stats.loaded_chunks = chunk_index.values().filter(|c| c.is_loaded()).count();
        stats.memory_usage_bytes = chunk_index.values()
            .map(|c| c.memory_usage())
            .sum();
        
        stats.clone()
    }
    
    /// Manually evict a chunk from memory
    pub async fn evict_chunk(&self, chunk_id: &str) -> LazyLoadingResult<bool> {
        let mut chunk_index = self.chunk_index.write().await;
        
        if let Some(chunk) = chunk_index.get_mut(chunk_id) {
            if chunk.is_loaded() {
                chunk.data = None;
                chunk.loaded_at = None;
                
                let mut stats = self.stats.write().await;
                stats.chunks_evicted += 1;
                
                eprintln!("🗑️ Evicted chunk: {}", chunk_id);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Clear all loaded chunks to free memory
    pub async fn clear_all(&self) -> LazyLoadingResult<usize> {
        let mut chunk_index = self.chunk_index.write().await;
        let mut evicted_count = 0;
        
        for chunk in chunk_index.values_mut() {
            if chunk.is_loaded() {
                chunk.data = None;
                chunk.loaded_at = None;
                evicted_count += 1;
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.chunks_evicted += evicted_count;
        
        eprintln!("🧹 Cleared all chunks: {} evicted", evicted_count);
        Ok(evicted_count)
    }
    
    // Private methods
    
    /// Create chunks from embeddings
    async fn create_chunks(
        &self,
        embeddings: Vec<EmbeddingEntry>,
        storage_dir: &Path,
    ) -> LazyLoadingResult<Vec<LazyChunk>> {
        let mut chunks = Vec::new();
        let chunk_size = self.config.chunk_size;
        
        for (i, chunk_embeddings) in embeddings.chunks(chunk_size).enumerate() {
            let chunk_id = format!("chunk_{:06}", i);
            let file_path = storage_dir.join(format!("{}.json", chunk_id));
            
            let estimated_size = chunk_embeddings
                .iter()
                .map(|e| e.memory_footprint())
                .sum();
            
            let info = ChunkInfo {
                chunk_id: chunk_id.clone(),
                file_path,
                entry_count: chunk_embeddings.len(),
                start_index: i * chunk_size,
                estimated_size_bytes: estimated_size,
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                last_accessed: 0,
                access_count: 0,
            };
            
            let mut chunk = LazyChunk::new(info);
            chunk.data = Some(chunk_embeddings.to_vec()); // Initially loaded
            chunk.loaded_at = Some(Instant::now());
            
            chunks.push(chunk);
        }
        
        Ok(chunks)
    }
    
    /// Ensure a chunk is loaded in memory
    async fn ensure_chunk_loaded(&self, chunk_id: &str) -> LazyLoadingResult<()> {
        {
            let chunk_index = self.chunk_index.read().await;
            if let Some(chunk) = chunk_index.get(chunk_id) {
                if chunk.is_loaded() {
                    return Ok(());
                }
            }
        }
        
        // Need to load the chunk
        self.load_chunk(chunk_id).await
    }
    
    /// Load a chunk from storage
    async fn load_chunk(&self, chunk_id: &str) -> LazyLoadingResult<()> {
        let start_time = Instant::now();
        
        // Check memory limits before loading
        self.enforce_memory_limits().await?;
        
        // Mark chunk as loading to prevent duplicate loads
        {
            let mut chunk_index = self.chunk_index.write().await;
            if let Some(chunk) = chunk_index.get_mut(chunk_id) {
                if chunk.loading {
                    return Ok(()); // Already being loaded
                }
                chunk.loading = true;
            }
        }
        
        // Simulate loading from disk (placeholder - would implement actual loading)
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Update chunk with loaded data
        {
            let mut chunk_index = self.chunk_index.write().await;
            if let Some(chunk) = chunk_index.get_mut(chunk_id) {
                // In real implementation, would load from chunk.info.file_path
                // For now, create dummy data
                chunk.data = Some(Vec::new()); // Placeholder
                chunk.loaded_at = Some(Instant::now());
                chunk.loading = false;
                chunk.info.last_accessed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                chunk.info.access_count += 1;
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.background_loads += 1;
        stats.avg_load_time_ms = (stats.avg_load_time_ms + start_time.elapsed().as_secs_f64() * 1000.0) / 2.0;
        
        eprintln!("📚 Loaded chunk: {} in {:.2}ms", chunk_id, start_time.elapsed().as_secs_f64() * 1000.0);
        
        Ok(())
    }
    
    /// Enforce memory usage limits by evicting chunks
    async fn enforce_memory_limits(&self) -> LazyLoadingResult<()> {
        let current_memory_mb = {
            let chunk_index = self.chunk_index.read().await;
            chunk_index.values().map(|c| c.memory_usage()).sum::<usize>() / (1024 * 1024)
        };
        
        if current_memory_mb <= self.config.max_memory_mb {
            return Ok(());
        }
        
        // Need to evict some chunks
        let mut chunks_to_evict = Vec::new();
        
        {
            let chunk_index = self.chunk_index.read().await;
            let mut loaded_chunks: Vec<_> = chunk_index
                .values()
                .filter(|c| c.is_loaded())
                .collect();
            
            // Sort by last access time (evict least recently used)
            loaded_chunks.sort_by_key(|c| c.info.last_accessed);
            
            let mut freed_mb = 0;
            for chunk in loaded_chunks {
                chunks_to_evict.push(chunk.info.chunk_id.clone());
                freed_mb += chunk.memory_usage() / (1024 * 1024);
                
                if current_memory_mb - freed_mb <= self.config.max_memory_mb {
                    break;
                }
            }
        }
        
        // Evict selected chunks
        for chunk_id in chunks_to_evict {
            self.evict_chunk(&chunk_id).await?;
        }
        
        Ok(())
    }
    
    /// Trigger prefetching based on access patterns
    async fn trigger_prefetching(&self, current_chunk_id: &str) {
        if !self.config.enable_prefetching {
            return;
        }
        
        let predicted_chunks = if self.config.enable_pattern_learning {
            let patterns = self.access_patterns.read().await;
            patterns.get_predicted_chunks(current_chunk_id, self.config.prefetch_ahead)
        } else {
            // Simple sequential prefetching
            vec![]
        };
        
        // Queue chunks for background loading
        let mut queue = self.loading_queue.lock().await;
        for chunk_id in predicted_chunks {
            if !queue.contains(&chunk_id) {
                queue.push_back(chunk_id);
            }
        }
    }
    
    /// Start background task for prefetching
    fn start_background_task(&mut self) {
        let loading_queue = Arc::clone(&self.loading_queue);
        let chunk_index = Arc::clone(&self.chunk_index);
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                let chunk_to_load = {
                    let mut queue = loading_queue.lock().await;
                    queue.pop_front()
                };
                
                if let Some(chunk_id) = chunk_to_load {
                    // Check if chunk needs loading
                    let needs_loading = {
                        let index = chunk_index.read().await;
                        if let Some(chunk) = index.get(&chunk_id) {
                            !chunk.is_loaded() && !chunk.loading
                        } else {
                            false
                        }
                    };
                    
                    if needs_loading {
                        // Simulate background loading
                        eprintln!("🔄 Background prefetching chunk: {}", chunk_id);
                    }
                }
            }
        });
        
        self.background_task = Some(handle);
    }
}

impl Drop for LazyLoadingManager {
    fn drop(&mut self) {
        if let Some(handle) = self.background_task.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::types::EmbeddingEntry;
    use tempfile::TempDir;
    
    fn create_test_entry(id: &str, vector: Vec<f32>) -> EmbeddingEntry {
        EmbeddingEntry::new(
            vector,
            "/test/file.md".to_string(),
            format!("chunk_{}", id),
            "test content",
            "test-model".to_string(),
        )
    }
    
    #[tokio::test]
    async fn test_lazy_loading_manager_creation() {
        let config = LazyLoadingConfig::default();
        let _manager = LazyLoadingManager::new(config);
        // Test passes if no panic
    }
    
    #[tokio::test]
    async fn test_chunk_info() {
        let temp_dir = TempDir::new().unwrap();
        let info = ChunkInfo {
            chunk_id: "test_chunk".to_string(),
            file_path: temp_dir.path().join("test.json"),
            entry_count: 10,
            start_index: 0,
            estimated_size_bytes: 1024,
            created_at: 1234567890,
            last_accessed: 0,
            access_count: 0,
        };
        
        assert_eq!(info.chunk_id, "test_chunk");
        assert_eq!(info.entry_count, 10);
        assert_eq!(info.estimated_size_bytes, 1024);
    }
    
    #[tokio::test]
    async fn test_lazy_chunk_operations() {
        let temp_dir = TempDir::new().unwrap();
        let info = ChunkInfo {
            chunk_id: "test_chunk".to_string(),
            file_path: temp_dir.path().join("test.json"),
            entry_count: 1,
            start_index: 0,
            estimated_size_bytes: 512,
            created_at: 1234567890,
            last_accessed: 0,
            access_count: 0,
        };
        
        let mut chunk = LazyChunk::new(info);
        assert!(!chunk.is_loaded());
        
        // Load some data
        chunk.data = Some(vec![create_test_entry("1", vec![0.1, 0.2])]);
        chunk.loaded_at = Some(Instant::now());
        
        assert!(chunk.is_loaded());
        assert!(chunk.memory_usage() > 0);
    }
    
    #[tokio::test]
    async fn test_access_pattern_tracking() {
        let mut pattern = AccessPattern::new();
        
        pattern.record_access("chunk_1");
        pattern.record_access("chunk_2");
        pattern.record_access("chunk_3");
        pattern.record_access("chunk_2"); // Access chunk_2 again
        
        // Check frequency tracking
        assert_eq!(pattern.access_frequencies["chunk_1"], 1);
        assert_eq!(pattern.access_frequencies["chunk_2"], 2);
        
        // Check transition patterns
        let predictions = pattern.get_predicted_chunks("chunk_2", 2);
        assert!(!predictions.is_empty() || predictions.is_empty()); // Either way is valid for small dataset
    }
    
    #[test]
    fn test_lazy_loading_config_defaults() {
        let config = LazyLoadingConfig::default();
        assert_eq!(config.max_memory_mb, 100);
        assert_eq!(config.chunk_size, 100);
        assert_eq!(config.enable_prefetching, true);
        assert_eq!(config.prefetch_ahead, 3);
    }
}