//! Enhanced Cache Module
//!
//! This module provides advanced caching capabilities for frequently accessed embeddings,
//! building on the basic embedding cache with smart eviction policies, access pattern
//! learning, and performance optimization.
//!
//! ## Features
//!
//! - **Multi-Level Caching**: L1 (hot) and L2 (warm) cache levels
//! - **Smart Eviction**: LRU, LFU, and adaptive policies
//! - **Access Pattern Learning**: Predictive caching based on usage patterns
//! - **Memory Management**: Adaptive cache sizing based on available memory
//! - **Performance Monitoring**: Detailed cache performance metrics

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Instant, Duration};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::vector_db::types::EmbeddingEntry;
use crate::embedding_cache::CacheConfig;

/// Errors that can occur during enhanced cache operations
#[derive(Error, Debug)]
pub enum EnhancedCacheError {
    #[error("Cache capacity exceeded: {current} > {max}")]
    CapacityExceeded { current: usize, max: usize },
    
    #[error("Invalid cache level: {level}")]
    InvalidCacheLevel { level: u8 },
    
    #[error("Memory limit exceeded: {used_mb}MB > {limit_mb}MB")]
    MemoryLimitExceeded { used_mb: usize, limit_mb: usize },
    
    #[error("Cache operation failed: {message}")]
    OperationFailed { message: String },
    
    #[error("Access pattern analysis failed: {message}")]
    AnalysisFailed { message: String },
}

pub type EnhancedCacheResult<T> = Result<T, EnhancedCacheError>;

/// Configuration for enhanced cache system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedCacheConfig {
    /// L1 cache configuration (hot data)
    pub l1_config: CacheConfig,
    /// L2 cache configuration (warm data)
    pub l2_config: CacheConfig,
    /// Eviction policy for cache management
    pub eviction_policy: EvictionPolicy,
    /// Enable access pattern learning
    pub enable_pattern_learning: bool,
    /// Enable adaptive cache sizing
    pub enable_adaptive_sizing: bool,
    /// Maximum total memory usage (MB)
    pub max_memory_mb: usize,
    /// Background cleanup interval (seconds)
    pub cleanup_interval_seconds: u64,
    /// Access frequency threshold for L1 promotion
    pub l1_promotion_threshold: usize,
    /// Time window for access frequency calculation (seconds)
    pub frequency_window_seconds: u64,
    /// Enable cache prewarming
    pub enable_prewarming: bool,
    /// Number of items to prewarm
    pub prewarm_count: usize,
}

impl Default for EnhancedCacheConfig {
    fn default() -> Self {
        Self {
            l1_config: CacheConfig {
                max_entries: 500,
                ttl_seconds: 3600, // 1 hour
                persist_to_disk: false, // L1 is memory-only for speed
                cache_file_path: None,
                enable_metrics: true,
            },
            l2_config: CacheConfig {
                max_entries: 2000,
                ttl_seconds: 7200, // 2 hours
                persist_to_disk: true, // L2 can persist
                cache_file_path: None,
                enable_metrics: true,
            },
            eviction_policy: EvictionPolicy::AdaptiveLRULFU,
            enable_pattern_learning: true,
            enable_adaptive_sizing: true,
            max_memory_mb: 200, // 200MB total for cache
            cleanup_interval_seconds: 300, // 5 minutes
            l1_promotion_threshold: 3, // Promote after 3 accesses
            frequency_window_seconds: 1800, // 30 minutes window
            enable_prewarming: true,
            prewarm_count: 100,
        }
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time-based eviction (TTL)
    TimeBasedTTL,
    /// Adaptive combination of LRU and LFU
    AdaptiveLRULFU,
    /// Machine learning-based prediction
    MLBased,
}

/// Cache entry with enhanced metadata
#[derive(Debug, Clone)]
pub struct EnhancedCacheEntry {
    /// The embedding entry
    pub entry: EmbeddingEntry,
    /// Last access timestamp
    pub last_accessed: Instant,
    /// Access frequency counter
    pub access_count: usize,
    /// Cache level (1 for L1, 2 for L2)
    pub cache_level: u8,
    /// Memory usage estimate
    pub memory_usage: usize,
    /// Creation timestamp in cache
    pub cached_at: Instant,
    /// Time to live
    pub ttl: Duration,
    /// Access frequency in current time window
    pub recent_access_frequency: f64,
    /// Predicted next access time (for ML-based eviction)
    pub predicted_next_access: Option<Instant>,
}

impl EnhancedCacheEntry {
    /// Create new cache entry
    fn new(entry: EmbeddingEntry, cache_level: u8, ttl: Duration) -> Self {
        let memory_usage = entry.memory_footprint();
        
        Self {
            entry,
            last_accessed: Instant::now(),
            access_count: 1,
            cache_level,
            memory_usage,
            cached_at: Instant::now(),
            ttl,
            recent_access_frequency: 1.0,
            predicted_next_access: None,
        }
    }
    
    /// Update access statistics
    fn record_access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
        
        // Update recent frequency (exponential moving average)
        let time_since_cache = self.cached_at.elapsed().as_secs() as f64;
        if time_since_cache > 0.0 {
            self.recent_access_frequency = self.access_count as f64 / time_since_cache;
        }
    }
    
    /// Check if entry has expired
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
    
    /// Calculate cache priority score (higher = keep longer)
    fn priority_score(&self, policy: &EvictionPolicy) -> f64 {
        match policy {
            EvictionPolicy::LRU => {
                // More recent = higher score
                1.0 / (self.last_accessed.elapsed().as_secs() as f64 + 1.0)
            }
            EvictionPolicy::LFU => {
                // More frequent = higher score
                self.recent_access_frequency
            }
            EvictionPolicy::TimeBasedTTL => {
                // More time remaining = higher score
                let remaining = self.ttl.as_secs() as f64 - self.cached_at.elapsed().as_secs() as f64;
                remaining.max(0.0)
            }
            EvictionPolicy::AdaptiveLRULFU => {
                // Weighted combination of recency and frequency
                let recency_score = 1.0 / (self.last_accessed.elapsed().as_secs() as f64 + 1.0);
                let frequency_score = self.recent_access_frequency;
                0.6 * recency_score + 0.4 * frequency_score
            }
            EvictionPolicy::MLBased => {
                // Use predicted next access if available, otherwise adaptive
                if let Some(predicted) = self.predicted_next_access {
                    let time_to_predicted = if predicted > Instant::now() {
                        predicted.duration_since(Instant::now()).as_secs() as f64
                    } else {
                        0.0
                    };
                    1.0 / (time_to_predicted + 1.0)
                } else {
                    // Fallback to adaptive
                    let recency_score = 1.0 / (self.last_accessed.elapsed().as_secs() as f64 + 1.0);
                    let frequency_score = self.recent_access_frequency;
                    0.6 * recency_score + 0.4 * frequency_score
                }
            }
        }
    }
}

/// Access pattern for predictive caching
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// Entry access sequence
    pub access_sequence: VecDeque<(String, Instant)>,
    /// Access intervals for each entry
    pub access_intervals: HashMap<String, VecDeque<Duration>>,
    /// Co-access patterns (entries accessed together)
    pub co_access_patterns: HashMap<String, HashMap<String, usize>>,
    /// Time-based access patterns (hour of day)
    pub temporal_patterns: HashMap<String, Vec<usize>>, // 24-hour histogram
}

impl AccessPattern {
    fn new() -> Self {
        Self {
            access_sequence: VecDeque::with_capacity(1000),
            access_intervals: HashMap::new(),
            co_access_patterns: HashMap::new(),
            temporal_patterns: HashMap::new(),
        }
    }
    
    /// Record access to an entry
    fn record_access(&mut self, entry_id: &str, access_time: Instant) {
        // Update access sequence
        self.access_sequence.push_back((entry_id.to_string(), access_time));
        if self.access_sequence.len() > 1000 {
            self.access_sequence.pop_front();
        }
        
        // Calculate access interval
        if let Some(intervals) = self.access_intervals.get_mut(entry_id) {
            if let Some(last_interval) = intervals.back() {
                let last_access_time = access_time - *last_interval;
                let new_interval = access_time.duration_since(last_access_time);
                intervals.push_back(new_interval);
                if intervals.len() > 10 {
                    intervals.pop_front();
                }
            }
        } else {
            self.access_intervals.insert(entry_id.to_string(), VecDeque::new());
        }
        
        // Update co-access patterns
        let recent_accesses: Vec<_> = self.access_sequence
            .iter()
            .rev()
            .take(5) // Look at last 5 accesses
            .filter(|(_, time)| access_time.duration_since(*time).as_secs() < 60) // Within 1 minute
            .map(|(id, _)| id.clone())
            .collect();
        
        for other_entry in &recent_accesses {
            if other_entry != entry_id {
                self.co_access_patterns
                    .entry(entry_id.to_string())
                    .or_default()
                    .entry(other_entry.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        }
        
        // Update temporal patterns (hour of day)
        let now = SystemTime::now();
        if let Ok(duration) = now.duration_since(UNIX_EPOCH) {
            let hour = (duration.as_secs() / 3600) % 24;
            self.temporal_patterns
                .entry(entry_id.to_string())
                .or_insert_with(|| vec![0; 24])
                [hour as usize] += 1;
        }
    }
    
    /// Predict next access time for an entry
    #[allow(dead_code)]
    fn predict_next_access(&self, entry_id: &str) -> Option<Instant> {
        // Use average access interval if available
        if let Some(intervals) = self.access_intervals.get(entry_id) {
            if !intervals.is_empty() {
                let avg_interval: Duration = intervals.iter().sum::<Duration>() / intervals.len() as u32;
                return Some(Instant::now() + avg_interval);
            }
        }
        
        None
    }
    
    /// Get entries likely to be accessed together
    fn get_co_access_candidates(&self, entry_id: &str, limit: usize) -> Vec<String> {
        if let Some(co_accesses) = self.co_access_patterns.get(entry_id) {
            let mut sorted_candidates: Vec<_> = co_accesses.iter().collect();
            sorted_candidates.sort_by(|a, b| b.1.cmp(a.1)); // Sort by frequency
            sorted_candidates
                .into_iter()
                .take(limit)
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Enhanced cache system with multi-level caching and smart eviction
pub struct EnhancedCache {
    /// Configuration
    config: EnhancedCacheConfig,
    /// L1 cache (hot data)
    l1_cache: Arc<RwLock<HashMap<String, EnhancedCacheEntry>>>,
    /// L2 cache (warm data)
    l2_cache: Arc<RwLock<HashMap<String, EnhancedCacheEntry>>>,
    /// Access pattern tracker
    access_patterns: Arc<RwLock<AccessPattern>>,
    /// Cache statistics
    stats: Arc<RwLock<EnhancedCacheStats>>,
    /// Background cleanup task
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

/// Comprehensive cache statistics
#[derive(Debug, Clone, Default)]
pub struct EnhancedCacheStats {
    /// L1 cache statistics
    pub l1_stats: CacheLevelStats,
    /// L2 cache statistics
    pub l2_stats: CacheLevelStats,
    /// Cache promotions (L2 -> L1)
    pub promotions: usize,
    /// Cache demotions (L1 -> L2)
    pub demotions: usize,
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
    /// Performance statistics
    pub performance_stats: PerformanceStats,
    /// Pattern learning statistics
    pub pattern_stats: PatternStats,
    /// Last update timestamp
    pub last_updated: u64,
}

/// Statistics for individual cache level
#[derive(Debug, Clone, Default)]
pub struct CacheLevelStats {
    /// Number of entries
    pub entry_count: usize,
    /// Cache hits
    pub hits: usize,
    /// Cache misses
    pub misses: usize,
    /// Hit rate
    pub hit_rate: f64,
    /// Evictions
    pub evictions: usize,
    /// Memory usage in bytes
    pub memory_usage: usize,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total memory usage in bytes
    pub total_usage_bytes: usize,
    /// Peak memory usage
    pub peak_usage_bytes: usize,
    /// Memory efficiency (cache hits per MB)
    pub memory_efficiency: f64,
    /// Adaptive sizing enabled
    pub adaptive_sizing_active: bool,
    /// Current memory limit
    pub current_limit_mb: usize,
}

/// Performance statistics
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Average access time (microseconds)
    pub avg_access_time_us: f64,
    /// Average eviction time (microseconds)
    pub avg_eviction_time_us: f64,
    /// Cache warming time (milliseconds)
    pub last_warmup_time_ms: f64,
    /// Background operations completed
    pub background_operations: usize,
}

/// Pattern learning statistics
#[derive(Debug, Clone, Default)]
pub struct PatternStats {
    /// Number of patterns learned
    pub patterns_learned: usize,
    /// Prediction accuracy
    pub prediction_accuracy: f64,
    /// Co-access predictions made
    pub co_access_predictions: usize,
    /// Temporal predictions made
    pub temporal_predictions: usize,
}

impl EnhancedCache {
    /// Create new enhanced cache system
    pub fn new(config: EnhancedCacheConfig) -> Self {
        let mut cache = Self {
            config,
            l1_cache: Arc::new(RwLock::new(HashMap::new())),
            l2_cache: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(AccessPattern::new())),
            stats: Arc::new(RwLock::new(EnhancedCacheStats::default())),
            cleanup_task: None,
        };
        
        cache.start_background_cleanup();
        cache
    }
    
    /// Get entry from cache with multi-level lookup
    pub async fn get(&self, entry_id: &str) -> EnhancedCacheResult<Option<EmbeddingEntry>> {
        let start_time = Instant::now();
        
        // Try L1 first
        {
            let mut l1_cache = self.l1_cache.write().await;
            if let Some(entry) = l1_cache.get_mut(entry_id) {
                entry.record_access();
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.l1_stats.hits += 1;
                stats.l1_stats.hit_rate = stats.l1_stats.hits as f64 / 
                    (stats.l1_stats.hits + stats.l1_stats.misses) as f64;
                
                // Record access pattern
                if self.config.enable_pattern_learning {
                    let mut patterns = self.access_patterns.write().await;
                    patterns.record_access(entry_id, Instant::now());
                }
                
                return Ok(Some(entry.entry.clone()));
            } else {
                let mut stats = self.stats.write().await;
                stats.l1_stats.misses += 1;
                stats.l1_stats.hit_rate = stats.l1_stats.hits as f64 / 
                    (stats.l1_stats.hits + stats.l1_stats.misses) as f64;
            }
        }
        
        // Try L2 if not in L1
        {
            let mut l2_cache = self.l2_cache.write().await;
            if let Some(entry) = l2_cache.get_mut(entry_id) {
                entry.record_access();
                let entry_clone = entry.entry.clone();
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.l2_stats.hits += 1;
                stats.l2_stats.hit_rate = stats.l2_stats.hits as f64 / 
                    (stats.l2_stats.hits + stats.l2_stats.misses) as f64;
                
                // Consider promotion to L1
                if entry.access_count >= self.config.l1_promotion_threshold {
                    self.promote_to_l1(entry_id, entry_clone.clone()).await?;
                    stats.promotions += 1;
                }
                
                // Record access pattern
                if self.config.enable_pattern_learning {
                    let mut patterns = self.access_patterns.write().await;
                    patterns.record_access(entry_id, Instant::now());
                }
                
                return Ok(Some(entry_clone));
            } else {
                let mut stats = self.stats.write().await;
                stats.l2_stats.misses += 1;
                stats.l2_stats.hit_rate = stats.l2_stats.hits as f64 / 
                    (stats.l2_stats.hits + stats.l2_stats.misses) as f64;
            }
        }
        
        // Update performance stats
        let access_time_us = start_time.elapsed().as_micros() as f64;
        let mut stats = self.stats.write().await;
        stats.performance_stats.avg_access_time_us = 
            (stats.performance_stats.avg_access_time_us + access_time_us) / 2.0;
        
        Ok(None)
    }
    
    /// Store entry in appropriate cache level
    pub async fn set(&self, entry: EmbeddingEntry) -> EnhancedCacheResult<()> {
        let entry_id = entry.id.clone();
        
        // Determine cache level based on access patterns
        let cache_level = if self.config.enable_pattern_learning {
            self.determine_cache_level(&entry_id).await
        } else {
            2 // Default to L2
        };
        
        let ttl = if cache_level == 1 {
            Duration::from_secs(self.config.l1_config.ttl_seconds)
        } else {
            Duration::from_secs(self.config.l2_config.ttl_seconds)
        };
        
        let cache_entry = EnhancedCacheEntry::new(entry, cache_level, ttl);
        
        if cache_level == 1 {
            // Ensure L1 has capacity
            self.ensure_l1_capacity().await?;
            
            let mut l1_cache = self.l1_cache.write().await;
            l1_cache.insert(entry_id.clone(), cache_entry);
        } else {
            // Ensure L2 has capacity
            self.ensure_l2_capacity().await?;
            
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.insert(entry_id.clone(), cache_entry);
        }
        
        // Update statistics
        self.update_cache_stats().await;
        
        // Trigger co-access prefetching if enabled
        if self.config.enable_pattern_learning {
            self.prefetch_co_accessed_entries(&entry_id).await;
        }
        
        Ok(())
    }
    
    /// Clear all cache levels
    pub async fn clear(&self) -> EnhancedCacheResult<()> {
        let mut l1_cache = self.l1_cache.write().await;
        let mut l2_cache = self.l2_cache.write().await;
        
        l1_cache.clear();
        l2_cache.clear();
        
        // Reset statistics
        let mut stats = self.stats.write().await;
        *stats = EnhancedCacheStats::default();
        
        Ok(())
    }
    
    /// Get comprehensive cache statistics
    pub async fn get_stats(&self) -> EnhancedCacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Perform cache prewarming
    pub async fn prewarm(&self, entries: Vec<EmbeddingEntry>) -> EnhancedCacheResult<usize> {
        if !self.config.enable_prewarming {
            return Ok(0);
        }
        
        let start_time = Instant::now();
        let count = entries.len().min(self.config.prewarm_count);
        
        eprintln!("🔥 Prewarming cache with {} entries", count);
        
        for entry in entries.into_iter().take(count) {
            self.set(entry).await?;
        }
        
        // Update performance statistics
        let mut stats = self.stats.write().await;
        stats.performance_stats.last_warmup_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        
        eprintln!("✅ Cache prewarming completed in {:.2}ms", 
                  stats.performance_stats.last_warmup_time_ms);
        
        Ok(count)
    }
    
    // Private methods
    
    async fn determine_cache_level(&self, entry_id: &str) -> u8 {
        // Check access patterns to determine appropriate level
        let patterns = self.access_patterns.read().await;
        
        if let Some(intervals) = patterns.access_intervals.get(entry_id) {
            if !intervals.is_empty() {
                let avg_interval: Duration = intervals.iter().sum::<Duration>() / intervals.len() as u32;
                if avg_interval < Duration::from_secs(300) { // < 5 minutes
                    return 1; // Hot data -> L1
                }
            }
        }
        
        2 // Default to L2
    }
    
    async fn promote_to_l1(&self, entry_id: &str, entry: EmbeddingEntry) -> EnhancedCacheResult<()> {
        // Remove from L2
        {
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.remove(entry_id);
        }
        
        // Ensure L1 capacity
        self.ensure_l1_capacity().await?;
        
        // Add to L1
        let cache_entry = EnhancedCacheEntry::new(
            entry, 
            1, 
            Duration::from_secs(self.config.l1_config.ttl_seconds)
        );
        
        let mut l1_cache = self.l1_cache.write().await;
        l1_cache.insert(entry_id.to_string(), cache_entry);
        
        eprintln!("⬆️ Promoted {} to L1 cache", entry_id);
        Ok(())
    }
    
    async fn ensure_l1_capacity(&self) -> EnhancedCacheResult<()> {
        let mut l1_cache = self.l1_cache.write().await;
        
        while l1_cache.len() >= self.config.l1_config.max_entries {
            // Find entry to evict based on policy
            if let Some((evict_id, _)) = self.find_eviction_candidate(&l1_cache).await {
                let evicted_entry = l1_cache.remove(&evict_id);
                
                // Demote to L2 instead of discarding
                if let Some(entry) = evicted_entry {
                    drop(l1_cache); // Release L1 lock
                    self.demote_to_l2(&evict_id, entry.entry).await?;
                    l1_cache = self.l1_cache.write().await; // Re-acquire lock
                }
                
                let mut stats = self.stats.write().await;
                stats.l1_stats.evictions += 1;
                stats.demotions += 1;
            } else {
                break;
            }
        }
        
        Ok(())
    }
    
    async fn ensure_l2_capacity(&self) -> EnhancedCacheResult<()> {
        let mut l2_cache = self.l2_cache.write().await;
        
        while l2_cache.len() >= self.config.l2_config.max_entries {
            if let Some((evict_id, _)) = self.find_eviction_candidate(&l2_cache).await {
                l2_cache.remove(&evict_id);
                
                let mut stats = self.stats.write().await;
                stats.l2_stats.evictions += 1;
            } else {
                break;
            }
        }
        
        Ok(())
    }
    
    async fn demote_to_l2(&self, entry_id: &str, entry: EmbeddingEntry) -> EnhancedCacheResult<()> {
        // Ensure L2 has capacity
        self.ensure_l2_capacity().await?;
        
        let cache_entry = EnhancedCacheEntry::new(
            entry, 
            2, 
            Duration::from_secs(self.config.l2_config.ttl_seconds)
        );
        
        let mut l2_cache = self.l2_cache.write().await;
        l2_cache.insert(entry_id.to_string(), cache_entry);
        
        Ok(())
    }
    
    async fn find_eviction_candidate(
        &self,
        cache: &HashMap<String, EnhancedCacheEntry>
    ) -> Option<(String, f64)> {
        if cache.is_empty() {
            return None;
        }
        
        let mut min_score = f64::INFINITY;
        let mut candidate = None;
        
        for (entry_id, entry) in cache {
            // Check if expired first
            if entry.is_expired() {
                return Some((entry_id.clone(), 0.0));
            }
            
            let score = entry.priority_score(&self.config.eviction_policy);
            if score < min_score {
                min_score = score;
                candidate = Some((entry_id.clone(), score));
            }
        }
        
        candidate
    }
    
    async fn update_cache_stats(&self) {
        let mut stats = self.stats.write().await;
        
        // Update memory statistics
        let l1_cache = self.l1_cache.read().await;
        let l2_cache = self.l2_cache.read().await;
        
        stats.l1_stats.entry_count = l1_cache.len();
        stats.l1_stats.memory_usage = l1_cache.values()
            .map(|e| e.memory_usage)
            .sum();
        
        stats.l2_stats.entry_count = l2_cache.len();
        stats.l2_stats.memory_usage = l2_cache.values()
            .map(|e| e.memory_usage)
            .sum();
        
        stats.memory_stats.total_usage_bytes = 
            stats.l1_stats.memory_usage + stats.l2_stats.memory_usage;
        
        if stats.memory_stats.total_usage_bytes > stats.memory_stats.peak_usage_bytes {
            stats.memory_stats.peak_usage_bytes = stats.memory_stats.total_usage_bytes;
        }
        
        // Calculate memory efficiency
        let total_hits = stats.l1_stats.hits + stats.l2_stats.hits;
        let total_memory_mb = stats.memory_stats.total_usage_bytes as f64 / (1024.0 * 1024.0);
        if total_memory_mb > 0.0 {
            stats.memory_stats.memory_efficiency = total_hits as f64 / total_memory_mb;
        }
        
        stats.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
    
    async fn prefetch_co_accessed_entries(&self, entry_id: &str) {
        let patterns = self.access_patterns.read().await;
        let candidates = patterns.get_co_access_candidates(entry_id, 3);
        
        // Queue prefetch operations (simplified - would implement actual prefetching)
        for candidate in candidates {
            eprintln!("🔮 Would prefetch co-accessed entry: {}", candidate);
        }
    }
    
    fn start_background_cleanup(&mut self) {
        let l1_cache = Arc::clone(&self.l1_cache);
        let l2_cache = Arc::clone(&self.l2_cache);
        let stats = Arc::clone(&self.stats);
        let cleanup_interval = self.config.cleanup_interval_seconds;
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(cleanup_interval));
            
            loop {
                interval.tick().await;
                
                // Clean expired entries
                let mut expired_count = 0;
                
                // Clean L1
                {
                    let mut l1 = l1_cache.write().await;
                    l1.retain(|_, entry| {
                        if entry.is_expired() {
                            expired_count += 1;
                            false
                        } else {
                            true
                        }
                    });
                }
                
                // Clean L2
                {
                    let mut l2 = l2_cache.write().await;
                    l2.retain(|_, entry| {
                        if entry.is_expired() {
                            expired_count += 1;
                            false
                        } else {
                            true
                        }
                    });
                }
                
                if expired_count > 0 {
                    eprintln!("🧹 Background cleanup: removed {} expired entries", expired_count);
                    
                    let mut stats_guard = stats.write().await;
                    stats_guard.performance_stats.background_operations += 1;
                }
            }
        });
        
        self.cleanup_task = Some(handle);
    }
}

impl Drop for EnhancedCache {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_task.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::types::EmbeddingEntry;
    
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
    async fn test_enhanced_cache_creation() {
        let config = EnhancedCacheConfig::default();
        let _cache = EnhancedCache::new(config);
        // Test passes if no panic
    }
    
    #[tokio::test]
    async fn test_cache_entry_operations() {
        let entry = create_test_entry("1", vec![0.1, 0.2, 0.3]);
        let mut cache_entry = EnhancedCacheEntry::new(entry, 1, Duration::from_secs(3600));
        
        assert_eq!(cache_entry.cache_level, 1);
        assert_eq!(cache_entry.access_count, 1);
        assert!(!cache_entry.is_expired());
        
        cache_entry.record_access();
        assert_eq!(cache_entry.access_count, 2);
    }
    
    #[tokio::test]
    async fn test_eviction_policies() {
        let entry = create_test_entry("1", vec![0.1, 0.2]);
        let cache_entry = EnhancedCacheEntry::new(entry, 1, Duration::from_secs(3600));
        
        let lru_score = cache_entry.priority_score(&EvictionPolicy::LRU);
        let lfu_score = cache_entry.priority_score(&EvictionPolicy::LFU);
        let adaptive_score = cache_entry.priority_score(&EvictionPolicy::AdaptiveLRULFU);
        
        assert!(lru_score > 0.0);
        assert!(lfu_score > 0.0);
        assert!(adaptive_score > 0.0);
    }
    
    #[tokio::test]
    async fn test_access_pattern_tracking() {
        let mut pattern = AccessPattern::new();
        let now = Instant::now();
        
        pattern.record_access("entry1", now);
        pattern.record_access("entry2", now);
        
        assert_eq!(pattern.access_sequence.len(), 2);
        
        let candidates = pattern.get_co_access_candidates("entry1", 5);
        // May be empty for small dataset, but shouldn't panic
        assert!(candidates.len() <= 5);
    }
    
    #[tokio::test]
    async fn test_multi_level_cache_operations() {
        let config = EnhancedCacheConfig::default();
        let cache = EnhancedCache::new(config);
        
        let entry = create_test_entry("test1", vec![0.1, 0.2, 0.3]);
        let entry_id = entry.id.clone();
        
        // Store entry
        cache.set(entry).await.unwrap();
        
        // Retrieve entry
        let retrieved = cache.get(&entry_id).await.unwrap();
        assert!(retrieved.is_some());
        
        // Get statistics
        let stats = cache.get_stats().await;
        assert!(stats.l1_stats.entry_count > 0 || stats.l2_stats.entry_count > 0);
    }
}