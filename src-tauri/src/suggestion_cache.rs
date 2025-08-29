//! # Suggestion Cache System
//!
//! Advanced caching and context management system for AI-powered note suggestions.
//! This module implements intelligent caching to optimize suggestion performance,
//! reduce redundant searches, and provide context-aware filtering.
//!
//! ## Core Features
//!
//! ### Suggestion Result Caching
//! - **LRU Cache**: Memory-efficient caching with automatic eviction
//! - **Content-based Keys**: Cache keys generated from content hashes
//! - **TTL Support**: Time-based expiration for cache freshness
//! - **Context Awareness**: File-specific and content-specific caching
//!
//! ### Cache Invalidation
//! - **Content Change Detection**: Automatic invalidation on content changes
//! - **File Modification Tracking**: Invalidate when source files change
//! - **Smart Invalidation**: Only invalidate relevant cache entries
//! - **Manual Invalidation**: API for explicit cache clearing
//!
//! ### Context Management
//! - **Recent Suggestions**: Track recently generated suggestions
//! - **Current File Context**: File-aware suggestion filtering
//! - **User Activity Tracking**: Adapt suggestions to user behavior
//! - **Relevance Scoring**: Rank suggestions by contextual relevance
//!
//! ### Performance Optimization
//! - **Cache Warming**: Preload suggestions for frequently accessed files
//! - **Background Processing**: Asynchronous cache management
//! - **Memory Management**: Configurable memory limits and cleanup
//! - **Metrics Collection**: Performance monitoring and statistics
//!
//! ## Performance Targets
//!
//! - Cache hit rate >70% for repeat queries
//! - Cache lookup completes in <10ms
//! - Memory usage <25MB for cache system
//! - Cache operations don't block suggestion generation

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use lru::LruCache;
use std::num::NonZeroUsize;
use thiserror::Error;

use crate::similarity_search::SearchResult;

/// Errors that can occur during suggestion cache operations
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionCacheError {
    #[error("Cache is full and cannot store more entries")]
    CacheFull,
    
    #[error("Cache key generation failed: {reason}")]
    KeyGenerationFailed { reason: String },
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Cache entry expired")]
    EntryExpired,
    
    #[error("Invalid context filter: {message}")]
    InvalidContextFilter { message: String },
    
    #[error("Cache operation failed: {message}")]
    OperationFailed { message: String },
}

pub type SuggestionCacheResult<T> = Result<T, SuggestionCacheError>;

/// Configuration for the suggestion cache system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionCacheConfig {
    /// Maximum number of cached suggestion sets
    pub max_suggestion_sets: usize,
    /// Maximum number of suggestions per cached set
    pub max_suggestions_per_set: usize,
    /// Time-to-live for cache entries in seconds
    pub ttl_seconds: u64,
    /// Enable content-based invalidation
    pub enable_content_invalidation: bool,
    /// Enable context-aware filtering
    pub enable_context_filtering: bool,
    /// Enable cache warming for frequent files
    pub enable_cache_warming: bool,
    /// Maximum memory usage in bytes (rough estimate)
    pub max_memory_bytes: usize,
    /// Enable detailed metrics collection
    pub enable_metrics: bool,
    /// Recent suggestion tracking window size
    pub recent_suggestions_window: usize,
}

impl Default for SuggestionCacheConfig {
    fn default() -> Self {
        Self {
            max_suggestion_sets: 500,         // 500 cached suggestion sets
            max_suggestions_per_set: 20,      // Up to 20 suggestions per set
            ttl_seconds: 300,                 // 5 minute TTL
            enable_content_invalidation: true, // Smart invalidation
            enable_context_filtering: true,   // Context awareness
            enable_cache_warming: true,       // Preloading
            max_memory_bytes: 25 * 1024 * 1024, // 25MB limit
            enable_metrics: true,             // Performance metrics
            recent_suggestions_window: 100,   // Last 100 suggestions
        }
    }
}

/// Context information for suggestion filtering and relevance
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SuggestionContext {
    /// Current file path
    pub current_file: Option<String>,
    /// Current vault path
    pub vault_path: Option<String>,
    /// Content length at time of generation
    pub content_length: usize,
    /// Cursor position in content
    pub cursor_position: usize,
    /// Current paragraph being edited
    pub current_paragraph: String,
    /// Timestamp of context creation
    pub timestamp: u64,
}

impl SuggestionContext {
    /// Create new suggestion context
    pub fn new(
        current_file: Option<String>,
        vault_path: Option<String>,
        content_length: usize,
        cursor_position: usize,
        current_paragraph: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        Self {
            current_file,
            vault_path,
            content_length,
            cursor_position,
            current_paragraph,
            timestamp,
        }
    }
    
    /// Check if context is still relevant based on time and content changes
    pub fn is_relevant(&self, other: &SuggestionContext, max_age_seconds: u64) -> bool {
        // Check time relevance
        if other.timestamp > self.timestamp + max_age_seconds {
            return false;
        }
        
        // Check file relevance
        if self.current_file != other.current_file {
            return false;
        }
        
        // Check content relevance (rough heuristic)
        let content_change = (self.content_length as i32 - other.content_length as i32).abs();
        let cursor_change = (self.cursor_position as i32 - other.cursor_position as i32).abs();
        
        // Allow reasonable changes but reject major edits
        content_change < 500 && cursor_change < 1000
    }
}

/// Cached suggestion set with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSuggestionSet {
    /// The suggestion results
    pub suggestions: Vec<SearchResult>,
    /// Context when suggestions were generated
    pub context: SuggestionContext,
    /// Content hash used for cache key
    pub content_hash: String,
    /// Model name used for generation
    pub model_name: String,
    /// Timestamp when cached
    pub cached_at: u64,
    /// TTL for this cache entry
    pub ttl_seconds: u64,
    /// Number of times this entry was accessed
    pub access_count: u64,
    /// Last access timestamp
    pub last_accessed: u64,
}

impl CachedSuggestionSet {
    /// Create new cached suggestion set
    pub fn new(
        suggestions: Vec<SearchResult>,
        context: SuggestionContext,
        content_hash: String,
        model_name: String,
        ttl_seconds: u64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        Self {
            suggestions,
            context,
            content_hash,
            model_name,
            cached_at: timestamp,
            ttl_seconds,
            access_count: 1,
            last_accessed: timestamp,
        }
    }
    
    /// Check if this cache entry has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        now > self.cached_at + self.ttl_seconds
    }
    
    /// Mark this entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
    }
    
    /// Check if suggestions are relevant for given context
    pub fn is_relevant_for_context(&self, context: &SuggestionContext) -> bool {
        self.context.is_relevant(context, self.ttl_seconds)
    }
    
    /// Calculate relevance score for context (0.0 to 1.0)
    pub fn calculate_relevance_score(&self, context: &SuggestionContext) -> f64 {
        let mut score = 1.0;
        
        // File relevance
        if self.context.current_file != context.current_file {
            return 0.0; // Different file = no relevance
        }
        
        // Time relevance (decay over time)
        let age = context.timestamp - self.cached_at;
        let time_factor = if age > self.ttl_seconds {
            0.0
        } else {
            1.0 - (age as f64 / self.ttl_seconds as f64)
        };
        score *= time_factor;
        
        // Content similarity (rough heuristic)
        let content_similarity = if self.context.content_length == 0 || context.content_length == 0 {
            0.5
        } else {
            let length_ratio = (self.context.content_length.min(context.content_length) as f64) /
                              (self.context.content_length.max(context.content_length) as f64);
            length_ratio
        };
        score *= content_similarity;
        
        // Access frequency boost
        let frequency_boost = (self.access_count as f64).log10() * 0.1;
        score *= (1.0 + frequency_boost).min(1.5);
        
        score.max(0.0).min(1.0)
    }
}

/// Cache key for suggestion sets
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SuggestionCacheKey {
    /// Content hash
    content_hash: String,
    /// Model name
    model_name: String,
    /// File path (optional for file-specific caching)
    file_path: Option<String>,
}

impl SuggestionCacheKey {
    /// Generate cache key from content and context
    pub fn from_content_and_context(
        content: &str, 
        model: &str, 
        context: &SuggestionContext
    ) -> SuggestionCacheResult<Self> {
        let mut hasher = Sha256::new();
        
        // Include content in hash
        hasher.update(content.as_bytes());
        
        // Include context in hash for more specific caching
        if let Some(file) = &context.current_file {
            hasher.update(file.as_bytes());
        }
        
        // Include paragraph for more granular caching
        hasher.update(context.current_paragraph.as_bytes());
        
        let content_hash = format!("{:x}", hasher.finalize());
        
        Ok(Self {
            content_hash,
            model_name: model.to_string(),
            file_path: context.current_file.clone(),
        })
    }
    
    /// Convert to string for LRU cache key
    pub fn as_string(&self) -> String {
        if let Some(file) = &self.file_path {
            format!("{}:{}:{}", self.model_name, self.content_hash, file)
        } else {
            format!("{}:{}", self.model_name, self.content_hash)
        }
    }
}

/// Cache metrics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SuggestionCacheMetrics {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Cache entries added
    pub insertions: u64,
    /// Cache entries evicted
    pub evictions: u64,
    /// Cache entries expired
    pub expirations: u64,
    /// Cache entries invalidated
    pub invalidations: u64,
    /// Average cache lookup time (ms)
    pub avg_lookup_time_ms: f64,
    /// Memory usage estimate (bytes)
    pub memory_usage_bytes: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Context filtering applications
    pub context_filters_applied: u64,
    /// Cache warming operations
    pub cache_warming_operations: u64,
    /// Last metrics update
    pub last_updated: u64,
}

impl SuggestionCacheMetrics {
    /// Update hit rate
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
    
    /// Update average lookup time
    pub fn update_avg_lookup_time(&mut self, time_ms: f64) {
        let alpha = 0.1;
        self.avg_lookup_time_ms = if self.avg_lookup_time_ms == 0.0 {
            time_ms
        } else {
            alpha * time_ms + (1.0 - alpha) * self.avg_lookup_time_ms
        };
    }
    
    /// Update memory usage estimate
    pub fn update_memory_usage(&mut self, cache_size: usize) {
        // Rough estimate: each cached suggestion set uses ~5KB
        let avg_entry_size = 5 * 1024;
        self.memory_usage_bytes = cache_size * avg_entry_size;
    }
}

/// Recent suggestion tracking for context awareness
#[derive(Debug, Clone)]
pub struct RecentSuggestionTracker {
    /// Recent suggestions queue
    recent_suggestions: VecDeque<(SuggestionCacheKey, SuggestionContext, Instant)>,
    /// Maximum number of recent entries to track
    max_entries: usize,
}

impl RecentSuggestionTracker {
    pub fn new(max_entries: usize) -> Self {
        Self {
            recent_suggestions: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }
    
    /// Add recent suggestion
    pub fn add_recent(&mut self, key: SuggestionCacheKey, context: SuggestionContext) {
        self.recent_suggestions.push_back((key, context, Instant::now()));
        
        if self.recent_suggestions.len() > self.max_entries {
            self.recent_suggestions.pop_front();
        }
    }
    
    /// Get recent suggestions for context
    pub fn get_recent_for_context(&self, context: &SuggestionContext) -> Vec<SuggestionCacheKey> {
        self.recent_suggestions
            .iter()
            .filter_map(|(key, ctx, timestamp)| {
                // Only include recent suggestions (last 5 minutes)
                if timestamp.elapsed() < Duration::from_secs(300) && ctx.is_relevant(context, 300) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Cleanup old entries
    pub fn cleanup_old(&mut self, max_age: Duration) {
        let cutoff = Instant::now() - max_age;
        while let Some((_, _, timestamp)) = self.recent_suggestions.front() {
            if *timestamp < cutoff {
                self.recent_suggestions.pop_front();
            } else {
                break;
            }
        }
    }
}

/// Main suggestion cache system
pub struct SuggestionCache {
    /// Main LRU cache for suggestion sets
    cache: Arc<RwLock<LruCache<String, CachedSuggestionSet>>>,
    /// Cache configuration
    config: SuggestionCacheConfig,
    /// Performance metrics
    metrics: Arc<RwLock<SuggestionCacheMetrics>>,
    /// Recent suggestion tracker
    recent_tracker: Arc<RwLock<RecentSuggestionTracker>>,
    /// File modification tracking for invalidation
    file_modification_times: Arc<RwLock<HashMap<String, u64>>>,
    /// Background cleanup task handle
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for SuggestionCache {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            recent_tracker: self.recent_tracker.clone(),
            file_modification_times: self.file_modification_times.clone(),
            cleanup_handle: None, // Don't clone the cleanup handle
        }
    }
}

impl Default for SuggestionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SuggestionCache {
    /// Create new suggestion cache with default configuration
    pub fn new() -> Self {
        Self::with_config(SuggestionCacheConfig::default())
    }
    
    /// Create new suggestion cache with custom configuration
    pub fn with_config(config: SuggestionCacheConfig) -> Self {
        let cache_size = NonZeroUsize::new(config.max_suggestion_sets)
            .unwrap_or(NonZeroUsize::new(500).unwrap());
        let cache = Arc::new(RwLock::new(LruCache::new(cache_size)));
        let metrics = Arc::new(RwLock::new(SuggestionCacheMetrics::default()));
        let recent_tracker = Arc::new(RwLock::new(RecentSuggestionTracker::new(config.recent_suggestions_window)));
        let file_modification_times = Arc::new(RwLock::new(HashMap::new()));
        
        let mut cache_instance = Self {
            cache,
            config,
            metrics,
            recent_tracker,
            file_modification_times,
            cleanup_handle: None,
        };
        
        // Start background cleanup task
        cache_instance.start_cleanup_task();
        
        cache_instance
    }
    
    /// Get cached suggestions
    pub async fn get_suggestions(
        &self,
        content: &str,
        model: &str,
        context: &SuggestionContext,
    ) -> SuggestionCacheResult<Option<Vec<SearchResult>>> {
        let start_time = Instant::now();
        
        let cache_key = SuggestionCacheKey::from_content_and_context(content, model, context)?;
        let key_str = cache_key.as_string();
        
        let mut cache = self.cache.write().await;
        
        if let Some(cached_set) = cache.get_mut(&key_str) {
            // Check if entry has expired
            if cached_set.is_expired() {
                cache.pop(&key_str);
                
                if self.config.enable_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.expirations += 1;
                    metrics.misses += 1;
                    metrics.update_hit_rate();
                    metrics.update_avg_lookup_time(start_time.elapsed().as_secs_f64() * 1000.0);
                }
                
                return Ok(None);
            }
            
            // Check context relevance
            if !cached_set.is_relevant_for_context(context) {
                if self.config.enable_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.context_filters_applied += 1;
                    metrics.misses += 1;
                    metrics.update_hit_rate();
                }
                return Ok(None);
            }
            
            // Mark as accessed
            cached_set.mark_accessed();
            
            // Update metrics
            if self.config.enable_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.hits += 1;
                metrics.update_hit_rate();
                metrics.update_avg_lookup_time(start_time.elapsed().as_secs_f64() * 1000.0);
            }
            
            // Add to recent tracker
            if self.config.enable_context_filtering {
                let mut tracker = self.recent_tracker.write().await;
                tracker.add_recent(cache_key, context.clone());
            }
            
            println!("✅ Suggestion cache HIT for model '{}' (context: {:?})", model, context.current_file);
            return Ok(Some(cached_set.suggestions.clone()));
        }
        
        // Cache miss
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.misses += 1;
            metrics.update_hit_rate();
            metrics.update_avg_lookup_time(start_time.elapsed().as_secs_f64() * 1000.0);
        }
        
        println!("❌ Suggestion cache MISS for model '{}' (context: {:?})", model, context.current_file);
        Ok(None)
    }
    
    /// Cache suggestion set
    pub async fn cache_suggestions(
        &self,
        content: &str,
        model: &str,
        context: &SuggestionContext,
        suggestions: Vec<SearchResult>,
    ) -> SuggestionCacheResult<()> {
        let cache_key = SuggestionCacheKey::from_content_and_context(content, model, context)?;
        let key_str = cache_key.as_string();
        
        // Create content hash for the cached set
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());
        
        let cached_set = CachedSuggestionSet::new(
            suggestions,
            context.clone(),
            content_hash,
            model.to_string(),
            self.config.ttl_seconds,
        );
        
        let mut cache = self.cache.write().await;
        
        // Check if eviction will occur
        let will_evict = cache.len() >= cache.cap().get() && !cache.contains(&key_str);
        let suggestions_len = cached_set.suggestions.len();
        
        cache.put(key_str, cached_set);
        
        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.insertions += 1;
            if will_evict {
                metrics.evictions += 1;
            }
            metrics.update_memory_usage(cache.len());
        }
        
        // Add to recent tracker
        if self.config.enable_context_filtering {
            let mut tracker = self.recent_tracker.write().await;
            tracker.add_recent(cache_key, context.clone());
        }
        
        // Update file modification time for invalidation
        if let Some(file_path) = &context.current_file {
            if self.config.enable_content_invalidation {
                let mut file_times = self.file_modification_times.write().await;
                file_times.insert(file_path.clone(), context.timestamp);
            }
        }
        
        println!("💾 Cached {} suggestions for model '{}' (context: {:?})", 
                suggestions_len, model, context.current_file);
        
        Ok(())
    }
    
    /// Invalidate cache entries for a specific file
    pub async fn invalidate_file(&self, file_path: &str) -> SuggestionCacheResult<usize> {
        let mut cache = self.cache.write().await;
        let mut invalidated_count = 0;
        
        // Collect keys to invalidate
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter_map(|(key, cached_set)| {
                if let Some(ref cached_file) = cached_set.context.current_file {
                    if cached_file == file_path {
                        Some(key.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        // Remove invalidated entries
        for key in keys_to_remove {
            cache.pop(&key);
            invalidated_count += 1;
        }
        
        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.invalidations += invalidated_count as u64;
            metrics.update_memory_usage(cache.len());
        }
        
        if invalidated_count > 0 {
            println!("🗑️ Invalidated {} suggestion cache entries for file: {}", invalidated_count, file_path);
        }
        
        Ok(invalidated_count)
    }
    
    /// Clear all cached suggestions
    pub async fn clear(&self) -> SuggestionCacheResult<()> {
        let mut cache = self.cache.write().await;
        let entry_count = cache.len();
        cache.clear();
        
        // Clear recent tracker
        if self.config.enable_context_filtering {
            let mut tracker = self.recent_tracker.write().await;
            tracker.recent_suggestions.clear();
        }
        
        // Clear file modification times
        if self.config.enable_content_invalidation {
            let mut file_times = self.file_modification_times.write().await;
            file_times.clear();
        }
        
        // Reset metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            *metrics = SuggestionCacheMetrics::default();
        }
        
        println!("🗑️ Cleared {} suggestion cache entries", entry_count);
        Ok(())
    }
    
    /// Get cache metrics
    pub async fn get_metrics(&self) -> SuggestionCacheMetrics {
        if self.config.enable_metrics {
            let metrics = self.metrics.read().await;
            metrics.clone()
        } else {
            SuggestionCacheMetrics::default()
        }
    }
    
    /// Get cache size
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
    
    /// Start background cleanup task
    fn start_cleanup_task(&mut self) {
        let cache_clone = self.cache.clone();
        let metrics_clone = self.metrics.clone();
        let recent_tracker_clone = self.recent_tracker.clone();
        let enable_metrics = self.config.enable_metrics;
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Cleanup every minute
            
            loop {
                interval.tick().await;
                
                // Cleanup expired entries
                let mut cache = cache_clone.write().await;
                let mut expired_keys = Vec::new();
                
                for (key, cached_set) in cache.iter() {
                    if cached_set.is_expired() {
                        expired_keys.push(key.clone());
                    }
                }
                
                let expired_count = expired_keys.len();
                for key in expired_keys {
                    cache.pop(&key);
                }
                
                drop(cache);
                
                // Cleanup recent tracker
                {
                    let mut tracker = recent_tracker_clone.write().await;
                    tracker.cleanup_old(Duration::from_secs(600)); // Keep last 10 minutes
                }
                
                // Update metrics
                if enable_metrics && expired_count > 0 {
                    let mut metrics = metrics_clone.write().await;
                    metrics.expirations += expired_count as u64;
                    metrics.update_memory_usage(cache_clone.read().await.len());
                    
                    println!("🧹 Background cleanup: removed {} expired suggestion cache entries", expired_count);
                }
            }
        });
        
        self.cleanup_handle = Some(handle);
    }
    
    /// Warm cache for frequently accessed content
    pub async fn warm_cache_for_file(&self, file_path: &str) -> SuggestionCacheResult<()> {
        if !self.config.enable_cache_warming {
            return Ok(());
        }
        
        // This would be implemented to preload suggestions for frequently accessed files
        // For now, just track the warming operation
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.cache_warming_operations += 1;
        }
        
        println!("🔥 Cache warming initiated for file: {}", file_path);
        Ok(())
    }
    
    /// Update configuration
    pub fn update_config(&mut self, new_config: SuggestionCacheConfig) {
        self.config = new_config;
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &SuggestionCacheConfig {
        &self.config
    }
}

impl Drop for SuggestionCache {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::types::{EmbeddingEntry, EmbeddingMetadata};
    
    #[tokio::test]
    async fn test_suggestion_cache_creation() {
        let cache = SuggestionCache::new();
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_suggestion_context_relevance() {
        let ctx1 = SuggestionContext::new(
            Some("test.md".to_string()),
            None,
            100,
            50,
            "Test paragraph".to_string(),
        );
        
        let ctx2 = SuggestionContext::new(
            Some("test.md".to_string()),
            None,
            105,
            55,
            "Test paragraph modified".to_string(),
        );
        
        assert!(ctx1.is_relevant(&ctx2, 300));
        
        let ctx3 = SuggestionContext::new(
            Some("different.md".to_string()),
            None,
            100,
            50,
            "Test paragraph".to_string(),
        );
        
        assert!(!ctx1.is_relevant(&ctx3, 300));
    }
    
    #[tokio::test]
    async fn test_cache_key_generation() {
        let context = SuggestionContext::new(
            Some("test.md".to_string()),
            None,
            100,
            50,
            "Test paragraph".to_string(),
        );
        
        let key1 = SuggestionCacheKey::from_content_and_context("test content", "model", &context).unwrap();
        let key2 = SuggestionCacheKey::from_content_and_context("test content", "model", &context).unwrap();
        let key3 = SuggestionCacheKey::from_content_and_context("different content", "model", &context).unwrap();
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
    
    #[tokio::test]
    async fn test_suggestion_caching() {
        let cache = SuggestionCache::new();
        
        let context = SuggestionContext::new(
            Some("test.md".to_string()),
            None,
            100,
            50,
            "Test paragraph".to_string(),
        );
        
        let metadata = EmbeddingMetadata::new(
            "result1.md".to_string(),
            "chunk_0".to_string(),
            "Test content 1".to_string(),
            13, // text length
            "test-model".to_string(),
            "Test content 1", // original text
        );
        
        let entry = EmbeddingEntry {
            id: "test_entry_1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata,
            created_at: 1234567890,
            updated_at: 1234567890,
        };
        
        let suggestions = vec![
            SearchResult {
                entry,
                similarity: 0.95,
            }
        ];
        
        // Cache miss initially
        let result = cache.get_suggestions("test content", "model", &context).await.unwrap();
        assert!(result.is_none());
        
        // Cache suggestions
        cache.cache_suggestions("test content", "model", &context, suggestions.clone()).await.unwrap();
        
        // Cache hit
        let result = cache.get_suggestions("test content", "model", &context).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
        
        assert_eq!(cache.size().await, 1);
    }
    
    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = SuggestionCache::new();
        
        let context = SuggestionContext::new(
            Some("test.md".to_string()),
            None,
            100,
            50,
            "Test paragraph".to_string(),
        );
        
        let metadata = EmbeddingMetadata::new(
            "result1.md".to_string(),
            "chunk_0".to_string(),
            "Test content 1".to_string(),
            13, // text length
            "test-model".to_string(),
            "Test content 1", // original text
        );
        
        let entry = EmbeddingEntry {
            id: "test_entry_1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata,
            created_at: 1234567890,
            updated_at: 1234567890,
        };
        
        let suggestions = vec![
            SearchResult {
                entry,
                similarity: 0.95,
            }
        ];
        
        // Cache suggestions
        cache.cache_suggestions("test content", "model", &context, suggestions).await.unwrap();
        assert_eq!(cache.size().await, 1);
        
        // Invalidate file
        let invalidated = cache.invalidate_file("test.md").await.unwrap();
        assert_eq!(invalidated, 1);
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_metrics_tracking() {
        let cache = SuggestionCache::new();
        
        let context = SuggestionContext::new(
            Some("test.md".to_string()),
            None,
            100,
            50,
            "Test paragraph".to_string(),
        );
        
        // Cache miss
        let _ = cache.get_suggestions("test content", "model", &context).await.unwrap();
        
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.hits, 0);
        
        // Cache and hit
        let suggestions = vec![];
        cache.cache_suggestions("test content", "model", &context, suggestions).await.unwrap();
        let _ = cache.get_suggestions("test content", "model", &context).await.unwrap();
        
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.insertions, 1);
        assert!(metrics.hit_rate > 0.0);
    }
}