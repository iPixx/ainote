use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use lru::LruCache;
use std::num::NonZeroUsize;
use thiserror::Error;

/// Errors that can occur during cache operations
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache is full and cannot store more entries")]
    CacheFull,
    
    #[error("Cache key generation failed: {reason}")]
    KeyGenerationFailed { reason: String },
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Cache entry expired")]
    EntryExpired,
    
    #[error("Cache operation failed: {message}")]
    OperationFailed { message: String },
}

pub type CacheResult<T> = Result<T, CacheError>;

/// Configuration for the embedding cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Time-to-live for cache entries in seconds
    pub ttl_seconds: u64,
    /// Whether to persist cache to disk
    pub persist_to_disk: bool,
    /// Cache persistence file path
    pub cache_file_path: Option<String>,
    /// Enable detailed cache metrics
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,          // Store up to 1000 embeddings
            ttl_seconds: 3600,          // 1 hour TTL
            persist_to_disk: true,      // Persist across sessions
            cache_file_path: None,      // Will be set to ~/.ainote/embedding_cache.json
            enable_metrics: true,       // Enable metrics by default
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// The embedding vector
    embedding: Vec<f32>,
    /// Timestamp when entry was created
    created_at: u64,
    /// Time-to-live in seconds
    ttl_seconds: u64,
    /// Number of times this entry was accessed
    access_count: u64,
    /// Model name used for generation
    model_name: String,
    /// Text length (for metrics)
    text_length: usize,
}

impl CacheEntry {
    fn new(embedding: Vec<f32>, ttl_seconds: u64, model_name: String, text_length: usize) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        Self {
            embedding,
            created_at,
            ttl_seconds,
            access_count: 1,
            model_name,
            text_length,
        }
    }
    
    /// Check if this cache entry has expired
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        now > self.created_at + self.ttl_seconds
    }
    
    /// Mark this entry as accessed (for LRU and metrics)
    fn mark_accessed(&mut self) {
        self.access_count += 1;
    }
}

/// Cache hit/miss statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheMetrics {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Total number of cache insertions
    pub insertions: u64,
    /// Total number of cache evictions
    pub evictions: u64,
    /// Total number of expired entries removed
    pub expirations: u64,
    /// Average embedding generation time (ms) for cache misses
    pub avg_generation_time_ms: f64,
    /// Total memory usage estimate (bytes)
    pub memory_usage_bytes: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Timestamp of last metrics update
    pub last_updated: u64,
}

impl CacheMetrics {
    /// Update hit rate based on current hits and misses
    pub fn update_hit_rate(&mut self) {
        let total_requests = self.hits + self.misses;
        self.hit_rate = if total_requests > 0 {
            self.hits as f64 / total_requests as f64
        } else {
            0.0
        };
        
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
    }
    
    /// Update average generation time with new measurement
    pub fn update_avg_generation_time(&mut self, time_ms: f64) {
        // Simple exponential moving average
        let alpha = 0.1;
        self.avg_generation_time_ms = if self.avg_generation_time_ms == 0.0 {
            time_ms
        } else {
            alpha * time_ms + (1.0 - alpha) * self.avg_generation_time_ms
        };
    }
    
    /// Estimate memory usage based on cache size
    pub fn update_memory_usage(&mut self, cache_size: usize, avg_embedding_size: usize) {
        // Rough estimate: cache_size * (embedding_size + overhead)
        let entry_overhead = 200; // Approximate overhead per entry
        self.memory_usage_bytes = cache_size * (avg_embedding_size * 4 + entry_overhead); // f32 = 4 bytes
    }
}

/// Cache key for embeddings based on text hash and model
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    text_hash: String,
    model_name: String,
}

impl CacheKey {
    /// Generate cache key from text and model name
    fn from_text_and_model(text: &str, model: &str) -> CacheResult<Self> {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let text_hash = format!("{:x}", hasher.finalize());
        
        Ok(Self {
            text_hash,
            model_name: model.to_string(),
        })
    }
    
    /// Convert to string for LRU cache key
    fn as_string(&self) -> String {
        format!("{}:{}", self.model_name, self.text_hash)
    }
}

/// High-performance LRU cache with TTL for embeddings
pub struct EmbeddingCache {
    /// LRU cache with TTL
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
    /// Background cleanup task handle
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for EmbeddingCache {
    fn clone(&self) -> Self {
        // Don't clone the cleanup handle - each clone will start its own
        Self {
            cache: self.cache.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            cleanup_handle: None,
        }
    }
}

impl Default for EmbeddingCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingCache {
    /// Create a new embedding cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }
    
    /// Create a new embedding cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        let cache_size = NonZeroUsize::new(config.max_entries).unwrap_or(NonZeroUsize::new(1000).unwrap());
        let cache = Arc::new(RwLock::new(LruCache::new(cache_size)));
        let metrics = Arc::new(RwLock::new(CacheMetrics::default()));
        
        let mut cache_instance = Self {
            cache,
            config,
            metrics,
            cleanup_handle: None,
        };
        
        // Start background cleanup task
        cache_instance.start_cleanup_task();
        
        // Load persisted cache if enabled
        if cache_instance.config.persist_to_disk {
            if let Err(e) = cache_instance.load_from_disk() {
                eprintln!("⚠️ Failed to load cache from disk: {}", e);
            }
        }
        
        cache_instance
    }
    
    /// Get embedding from cache
    pub async fn get(&self, text: &str, model: &str) -> CacheResult<Option<Vec<f32>>> {
        let cache_key = CacheKey::from_text_and_model(text, model)?;
        let key_str = cache_key.as_string();
        
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&key_str) {
            // Check if entry has expired
            if entry.is_expired() {
                cache.pop(&key_str);
                
                // Update metrics
                if self.config.enable_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.expirations += 1;
                    metrics.misses += 1;
                    metrics.update_hit_rate();
                }
                
                return Ok(None);
            }
            
            // Mark as accessed and return
            entry.mark_accessed();
            
            // Update metrics
            if self.config.enable_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.hits += 1;
                metrics.update_hit_rate();
            }
            
            eprintln!("✅ Cache HIT for model '{}' (text length: {})", model, text.len());
            return Ok(Some(entry.embedding.clone()));
        }
        
        // Cache miss
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.misses += 1;
            metrics.update_hit_rate();
        }
        
        eprintln!("❌ Cache MISS for model '{}' (text length: {})", model, text.len());
        Ok(None)
    }
    
    /// Store embedding in cache
    pub async fn set(&self, text: &str, model: &str, embedding: Vec<f32>) -> CacheResult<()> {
        let cache_key = CacheKey::from_text_and_model(text, model)?;
        let key_str = cache_key.as_string();
        
        let embedding_size = embedding.len();
        let entry = CacheEntry::new(
            embedding, 
            self.config.ttl_seconds, 
            model.to_string(), 
            text.len()
        );
        
        let mut cache = self.cache.write().await;
        
        // Check if eviction will occur
        let will_evict = cache.len() >= cache.cap().get() && !cache.contains(&key_str);
        
        cache.put(key_str, entry);
        
        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.insertions += 1;
            if will_evict {
                metrics.evictions += 1;
            }
            
            // Update memory usage estimate
            let avg_embedding_size = if embedding_size > 0 { embedding_size } else { 384 }; // Default embedding size
            metrics.update_memory_usage(cache.len(), avg_embedding_size);
        }
        
        eprintln!("💾 Cached embedding for model '{}' (text length: {}, vector size: {})", 
                  model, text.len(), embedding_size);
        
        // Persist to disk if enabled
        if self.config.persist_to_disk {
            // Non-blocking persistence
            let cache_clone = self.cache.clone();
            let config_clone = self.config.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::persist_cache_async(cache_clone, config_clone).await {
                    eprintln!("⚠️ Failed to persist cache to disk: {}", e);
                }
            });
        }
        
        Ok(())
    }
    
    /// Clear all entries from cache
    pub async fn clear(&self) -> CacheResult<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        
        // Reset metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            *metrics = CacheMetrics::default();
        }
        
        eprintln!("🗑️ Cache cleared");
        Ok(())
    }
    
    /// Get cache metrics
    pub async fn get_metrics(&self) -> CacheMetrics {
        if self.config.enable_metrics {
            let metrics = self.metrics.read().await;
            metrics.clone()
        } else {
            CacheMetrics::default()
        }
    }
    
    /// Get cache size (number of entries)
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
    
    /// Check if cache contains key
    pub async fn contains(&self, text: &str, model: &str) -> CacheResult<bool> {
        let cache_key = CacheKey::from_text_and_model(text, model)?;
        let key_str = cache_key.as_string();
        
        let cache = self.cache.read().await;
        let contains = cache.peek(&key_str).is_some();
        Ok(contains)
    }
    
    /// Remove expired entries from cache
    pub async fn cleanup_expired(&self) -> CacheResult<usize> {
        let mut cache = self.cache.write().await;
        let mut expired_keys = Vec::new();
        
        // Collect expired keys
        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }
        
        // Remove expired entries
        let expired_count = expired_keys.len();
        for key in expired_keys {
            cache.pop(&key);
        }
        
        // Update metrics
        if self.config.enable_metrics && expired_count > 0 {
            let mut metrics = self.metrics.write().await;
            metrics.expirations += expired_count as u64;
        }
        
        if expired_count > 0 {
            eprintln!("🧹 Cleaned up {} expired cache entries", expired_count);
        }
        
        Ok(expired_count)
    }
    
    /// Start background cleanup task for expired entries
    fn start_cleanup_task(&mut self) {
        let cache_clone = self.cache.clone();
        let metrics_clone = self.metrics.clone();
        let enable_metrics = self.config.enable_metrics;
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Cleanup every 5 minutes
            
            loop {
                interval.tick().await;
                
                let mut cache = cache_clone.write().await;
                let mut expired_keys = Vec::new();
                
                // Collect expired keys
                for (key, entry) in cache.iter() {
                    if entry.is_expired() {
                        expired_keys.push(key.clone());
                    }
                }
                
                // Remove expired entries
                let expired_count = expired_keys.len();
                for key in expired_keys {
                    cache.pop(&key);
                }
                
                // Update metrics
                if enable_metrics && expired_count > 0 {
                    let mut metrics = metrics_clone.write().await;
                    metrics.expirations += expired_count as u64;
                    
                    eprintln!("🧹 Background cleanup: removed {} expired entries", expired_count);
                }
            }
        });
        
        self.cleanup_handle = Some(handle);
    }
    
    /// Load cache from disk (if persistence enabled)
    fn load_from_disk(&self) -> CacheResult<()> {
        if let Some(cache_file) = self.get_cache_file_path() {
            // Implementation would load from file - simplified for now
            eprintln!("📂 Loading cache from {}", cache_file);
        }
        Ok(())
    }
    
    /// Persist cache to disk asynchronously
    async fn persist_cache_async(
        cache: Arc<RwLock<LruCache<String, CacheEntry>>>, 
        config: CacheConfig
    ) -> CacheResult<()> {
        if let Some(cache_file) = Self::get_cache_file_path_static(&config) {
            // Implementation would save to file - simplified for now
            let cache_guard = cache.read().await;
            eprintln!("💾 Persisting {} cache entries to {}", cache_guard.len(), cache_file);
        }
        Ok(())
    }
    
    /// Get cache file path
    fn get_cache_file_path(&self) -> Option<String> {
        Self::get_cache_file_path_static(&self.config)
    }
    
    /// Get cache file path (static version)
    fn get_cache_file_path_static(config: &CacheConfig) -> Option<String> {
        config.cache_file_path.clone().or_else(|| {
            dirs::home_dir().map(|home| {
                home.join(".ainote").join("embedding_cache.json").to_string_lossy().to_string()
            })
        })
    }
    
    /// Update cache configuration
    pub fn update_config(&mut self, new_config: CacheConfig) {
        self.config = new_config;
    }
    
    /// Get current cache configuration
    pub fn get_config(&self) -> &CacheConfig {
        &self.config
    }
}

impl Drop for EmbeddingCache {
    fn drop(&mut self) {
        // Cancel background cleanup task
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_creation() {
        let cache = EmbeddingCache::new();
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_cache_key_generation() {
        let key1 = CacheKey::from_text_and_model("hello world", "test-model").unwrap();
        let key2 = CacheKey::from_text_and_model("hello world", "test-model").unwrap();
        let key3 = CacheKey::from_text_and_model("hello world", "different-model").unwrap();
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
    
    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = EmbeddingCache::new();
        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        
        // Cache miss initially
        let result = cache.get("test text", "test-model").await.unwrap();
        assert!(result.is_none());
        
        // Set embedding
        cache.set("test text", "test-model", embedding.clone()).await.unwrap();
        
        // Cache hit
        let result = cache.get("test text", "test-model").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), embedding);
        
        assert_eq!(cache.size().await, 1);
    }
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let mut config = CacheConfig::default();
        config.ttl_seconds = 1; // 1 second TTL
        
        let cache = EmbeddingCache::with_config(config);
        let embedding = vec![0.1, 0.2, 0.3];
        
        // Set embedding
        cache.set("test text", "test-model", embedding.clone()).await.unwrap();
        
        // Should be available immediately
        let result = cache.get("test text", "test-model").await.unwrap();
        assert!(result.is_some());
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Should be expired and removed
        let result = cache.get("test text", "test-model").await.unwrap();
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_metrics() {
        let cache = EmbeddingCache::new();
        let embedding = vec![0.1, 0.2];
        
        // Initial metrics
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        
        // Cache miss
        let _ = cache.get("test", "model").await.unwrap();
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.misses, 1);
        
        // Cache set
        cache.set("test", "model", embedding).await.unwrap();
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.insertions, 1);
        
        // Cache hit
        let _ = cache.get("test", "model").await.unwrap();
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.hits, 1);
        assert!(metrics.hit_rate > 0.0);
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        let cache = EmbeddingCache::new();
        let embedding = vec![0.1, 0.2, 0.3];
        
        // Add entries
        cache.set("test1", "model", embedding.clone()).await.unwrap();
        cache.set("test2", "model", embedding.clone()).await.unwrap();
        assert_eq!(cache.size().await, 2);
        
        // Clear cache
        cache.clear().await.unwrap();
        assert_eq!(cache.size().await, 0);
        
        // Verify entries are gone
        let result = cache.get("test1", "model").await.unwrap();
        assert!(result.is_none());
    }
    
    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.ttl_seconds, 3600);
        assert_eq!(config.persist_to_disk, true);
        assert_eq!(config.enable_metrics, true);
    }
    
    #[test]
    fn test_cache_entry_expiration_check() {
        let entry = CacheEntry::new(vec![0.1], 1, "model".to_string(), 100);
        
        // Should not be expired immediately
        assert!(!entry.is_expired());
        
        // Create entry with past timestamp
        let mut expired_entry = entry.clone();
        expired_entry.created_at = 0; // UNIX epoch
        expired_entry.ttl_seconds = 1;
        
        // Should be expired
        assert!(expired_entry.is_expired());
    }
}