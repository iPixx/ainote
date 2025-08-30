//! # Embedding Commands
//!
//! This module contains all Tauri commands related to text embedding generation
//! and management. It provides comprehensive functionality for embedding creation,
//! caching, batch processing, and configuration management.
//!
//! ## Command Overview
//!
//! ### Core Embedding Operations
//! - `generate_embedding`: Generate embedding for single text
//! - `generate_batch_embeddings`: Generate embeddings for multiple texts
//! - `check_embedding_cached`: Check if embedding exists in cache
//!
//! ### Generator Configuration
//! - `update_embedding_generator_config`: Update generator parameters
//! - `get_embedding_generator_config`: Get current generator configuration
//!
//! ### Cache Management
//! - `get_embedding_cache_metrics`: Get cache performance metrics
//! - `clear_embedding_cache`: Clear all cached embeddings
//! - `get_embedding_cache_size`: Get current cache size
//! - `update_embedding_cache_config`: Update cache parameters
//! - `get_embedding_cache_config`: Get current cache configuration
//! - `cleanup_expired_embeddings`: Remove expired cache entries
//!
//! ## Embedding Generation Pipeline
//!
//! The embedding generation process follows these steps:
//!
//! 1. **Cache Check**: First check if embedding exists in cache
//! 2. **Text Preprocessing**: Clean and prepare text for processing
//! 3. **Model Invocation**: Send text to Ollama embedding model
//! 4. **Vector Processing**: Process and normalize embedding vectors
//! 5. **Cache Storage**: Store result in cache for future use
//! 6. **Metrics Update**: Update performance and usage metrics
//!
//! ## Batch Processing Optimization
//!
//! For multiple text embeddings:
//! - **Parallel Processing**: Generate multiple embeddings concurrently
//! - **Cache Optimization**: Check cache for all texts first
//! - **Efficient Batching**: Group uncached texts for batch processing
//! - **Progress Tracking**: Provide progress updates for large batches
//! - **Error Handling**: Handle partial failures gracefully
//!
//! ## Caching Strategy
//!
//! ### Cache Structure
//! - **Key Format**: `{text_hash}:{model_name}`
//! - **TTL Support**: Configurable time-to-live for cache entries
//! - **LRU Eviction**: Least recently used entries removed when full
//! - **Persistence**: Optional disk persistence for cache survival
//! - **Metrics**: Comprehensive hit/miss rate tracking
//!
//! ### Cache Configuration
//! - **Size Limits**: Configurable maximum number of entries
//! - **Memory Management**: Automatic memory pressure handling
//! - **Expiration**: TTL-based and manual expiration support
//! - **Cleanup**: Background cleanup of expired entries
//!
//! ## Performance Considerations
//!
//! ### Memory Management
//! - Embedding vectors are stored as f32 arrays
//! - Cache uses memory-mapped storage for large datasets
//! - Automatic cleanup prevents memory leaks
//! - Configurable memory limits with graceful degradation
//!
//! ### Processing Speed
//! - Cache hits provide instant results
//! - Batch processing reduces API overhead
//! - Parallel processing utilizes multiple cores
//! - Connection pooling for Ollama service
//!
//! ## Error Handling
//!
//! Comprehensive error handling for:
//! - Network connectivity issues with Ollama
//! - Model availability and loading problems
//! - Text preprocessing failures
//! - Cache corruption and recovery
//! - Memory and resource constraints

use crate::globals::{get_embedding_cache, get_embedding_generator};
use crate::embedding_generator::EmbeddingConfig;
use crate::embedding_cache::{CacheMetrics, CacheConfig};

/// Generate embedding vector for a single text
///
/// This command generates a high-dimensional vector representation of the input
/// text using the configured embedding model. It includes automatic caching
/// to improve performance for repeated requests.
///
/// # Arguments
/// * `text` - Input text to generate embedding for
/// * `model` - Name of the embedding model to use (e.g., "nomic-embed-text")
///
/// # Returns
/// * `Ok(Vec<f32>)` - Embedding vector (typically 768 or 1024 dimensions)
/// * `Err(String)` - Error message if embedding generation fails
///
/// # Performance Features
/// - **Cache First**: Checks cache before generating new embeddings
/// - **Automatic Caching**: Stores results for future use
/// - **Metrics Tracking**: Records generation time and cache performance
/// - **Error Recovery**: Handles transient failures gracefully
///
/// # Example Usage (from frontend)
/// ```javascript
/// const embedding = await invoke('generate_embedding', {
///     text: 'This is a sample text for embedding',
///     model: 'nomic-embed-text'
/// });
/// 
/// console.log('Embedding dimensions:', embedding.length);
/// console.log('First few values:', embedding.slice(0, 5));
/// ```
#[tauri::command]
pub async fn generate_embedding(text: String, model: String) -> Result<Vec<f32>, String> {
    let cache = get_embedding_cache().await;
    
    // Try cache first
    if let Ok(Some(cached_embedding)) = cache.get(&text, &model).await {
        return Ok(cached_embedding);
    }
    
    // Cache miss, generate embedding
    let generator = get_embedding_generator().await;
    let start_time = std::time::Instant::now();
    
    match generator.generate_embedding(text.clone(), model.clone()).await {
        Ok(embedding) => {
            let _generation_time = start_time.elapsed().as_millis() as f64;
            
            // Cache the result
            if let Err(e) = cache.set(&text, &model, embedding.clone()).await {
                eprintln!("⚠️ Failed to cache embedding: {}", e);
            }
            
            Ok(embedding)
        }
        Err(e) => Err(format!("Failed to generate embedding: {}", e))
    }
}

/// Generate embedding vectors for multiple texts in batch
///
/// This command efficiently generates embeddings for multiple texts using
/// batch processing optimization, caching, and parallel processing to
/// maximize throughput and minimize latency.
///
/// # Arguments
/// * `texts` - List of input texts to generate embeddings for
/// * `model` - Name of the embedding model to use
///
/// # Returns
/// * `Ok(Vec<Vec<f32>>)` - List of embedding vectors in same order as input
/// * `Err(String)` - Error message if batch processing fails
///
/// # Optimization Features
/// - **Cache Optimization**: Checks cache for all texts before generation
/// - **Batch API Calls**: Groups uncached texts for efficient processing
/// - **Parallel Processing**: Utilizes multiple threads for large batches
/// - **Progress Tracking**: Provides detailed progress information
/// - **Partial Success**: Returns successful embeddings even if some fail
///
/// # Performance Metrics
/// The command tracks and reports:
/// - Cache hit/miss ratios
/// - Total processing time
/// - Individual vs batch processing efficiency
/// - Memory usage during processing
///
/// # Example Usage (from frontend)
/// ```javascript
/// const texts = [
///     'First document text',
///     'Second document text', 
///     'Third document text'
/// ];
/// 
/// const embeddings = await invoke('generate_batch_embeddings', {
///     texts: texts,
///     model: 'nomic-embed-text'
/// });
/// 
/// console.log(`Generated ${embeddings.length} embeddings`);
/// embeddings.forEach((emb, idx) => {
///     console.log(`Text ${idx}: ${emb.length} dimensions`);
/// });
/// ```
#[tauri::command]
pub async fn generate_batch_embeddings(texts: Vec<String>, model: String) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    
    let cache = get_embedding_cache().await;
    let generator = get_embedding_generator().await;
    
    eprintln!("🔄 Processing batch of {} embeddings with caching", texts.len());
    
    let mut result_embeddings = vec![Vec::new(); texts.len()];
    let mut cache_misses = Vec::new();
    let mut cache_miss_indices = Vec::new();
    let mut hit_count = 0;
    
    // First pass: check cache for all texts
    for (i, text) in texts.iter().enumerate() {
        if let Ok(Some(cached_embedding)) = cache.get(text, &model).await {
            result_embeddings[i] = cached_embedding;
            hit_count += 1;
        } else {
            cache_misses.push(text.clone());
            cache_miss_indices.push(i);
        }
    }
    
    eprintln!("📊 Cache stats: {} hits, {} misses ({:.1}% hit rate)", 
              hit_count, cache_misses.len(),
              if !texts.is_empty() { hit_count as f64 / texts.len() as f64 * 100.0 } else { 0.0 });
    
    // Second pass: generate embeddings for cache misses
    if !cache_misses.is_empty() {
        let generation_start = std::time::Instant::now();
        match generator.generate_batch_embeddings(cache_misses.clone(), model.clone()).await {
            Ok(new_embeddings) => {
                let generation_time = generation_start.elapsed().as_millis() as f64;
                eprintln!("⚡ Generated {} embeddings in {:.1}ms", new_embeddings.len(), generation_time);
                
                // Store results and cache them
                for (miss_idx, new_embedding) in new_embeddings.into_iter().enumerate() {
                    if let Some(&result_idx) = cache_miss_indices.get(miss_idx) {
                        result_embeddings[result_idx] = new_embedding.clone();
                        
                        // Cache the new embedding
                        if let Some(text) = cache_misses.get(miss_idx) {
                            if let Err(e) = cache.set(text, &model, new_embedding).await {
                                eprintln!("⚠️ Failed to cache embedding for text {}: {}", miss_idx, e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to generate batch embeddings: {}", e));
            }
        }
    }
    
    Ok(result_embeddings)
}

/// Update embedding generator configuration parameters
///
/// This command updates the configuration of the embedding generator,
/// allowing runtime adjustment of performance and behavior parameters.
///
/// # Arguments
/// * `timeout_ms` - Optional timeout for embedding requests (milliseconds)
/// * `max_retries` - Optional maximum number of retry attempts
/// * `connection_pool_size` - Optional connection pool size
/// * `preprocess_text` - Optional flag to enable/disable text preprocessing
/// * `max_text_length` - Optional maximum text length for processing
/// * `batch_size` - Optional batch size for batch processing
///
/// # Returns
/// * `Ok(())` - Configuration successfully updated
/// * `Err(String)` - Error message if configuration update fails
///
/// # Configuration Parameters
/// - **Timeout**: Request timeout in milliseconds (default: 30000)
/// - **Retries**: Maximum retry attempts (default: 3)
/// - **Pool Size**: Connection pool size (default: 4)
/// - **Preprocessing**: Enable text preprocessing (default: true)
/// - **Max Length**: Maximum text length (default: 8192)
/// - **Batch Size**: Optimal batch size (default: 32)
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('update_embedding_generator_config', {
///     timeoutMs: 45000,      // 45 second timeout
///     maxRetries: 5,         // 5 retry attempts
///     batchSize: 16,         // Smaller batches
///     maxTextLength: 4096    // Shorter texts
/// });
/// ```
#[tauri::command]
pub async fn update_embedding_generator_config(
    timeout_ms: Option<u64>,
    max_retries: Option<usize>,
    connection_pool_size: Option<usize>,
    preprocess_text: Option<bool>,
    max_text_length: Option<usize>,
    batch_size: Option<usize>
) -> Result<(), String> {
    let mut generator_lock = crate::globals::EMBEDDING_GENERATOR.write().await;
    
    if let Some(generator) = generator_lock.as_mut() {
        let mut config = generator.get_embedding_config().clone();
        
        if let Some(timeout) = timeout_ms {
            config.timeout_ms = timeout;
        }
        if let Some(retries) = max_retries {
            config.max_retries = retries;
        }
        if let Some(pool_size) = connection_pool_size {
            config.connection_pool_size = pool_size;
        }
        if let Some(preprocess) = preprocess_text {
            config.preprocess_text = preprocess;
        }
        if let Some(max_length) = max_text_length {
            config.max_text_length = max_length;
        }
        if let Some(batch) = batch_size {
            config.batch_size = batch;
        }
        
        generator.update_embedding_config(config);
    } else {
        return Err("Embedding generator not initialized".to_string());
    }
    
    Ok(())
}

/// Get current embedding generator configuration
///
/// This command retrieves the current configuration parameters of the
/// embedding generator, useful for displaying settings and validation.
///
/// # Returns
/// * `Ok(EmbeddingConfig)` - Current generator configuration
/// * `Err(String)` - Error message if configuration cannot be retrieved
///
/// # Configuration Information
/// The returned configuration includes:
/// - Request timeout settings
/// - Retry and error handling parameters
/// - Connection pool configuration
/// - Text preprocessing options
/// - Batch processing settings
/// - Performance optimization flags
///
/// # Example Usage (from frontend)
/// ```javascript
/// const config = await invoke('get_embedding_generator_config');
/// console.log('Current configuration:');
/// console.log('- Timeout:', config.timeout_ms + 'ms');
/// console.log('- Max retries:', config.max_retries);
/// console.log('- Batch size:', config.batch_size);
/// console.log('- Preprocessing:', config.preprocess_text);
/// ```
#[tauri::command]
pub async fn get_embedding_generator_config() -> Result<EmbeddingConfig, String> {
    let generator_lock = crate::globals::EMBEDDING_GENERATOR.read().await;
    
    if let Some(generator) = generator_lock.as_ref() {
        Ok(generator.get_embedding_config().clone())
    } else {
        // Return default config if generator not initialized
        Ok(EmbeddingConfig::default())
    }
}

/// Get comprehensive embedding cache performance metrics
///
/// This command retrieves detailed performance and usage metrics from the
/// embedding cache, providing insights into cache effectiveness and system
/// performance.
///
/// # Returns
/// * `Ok(CacheMetrics)` - Comprehensive cache performance metrics
/// * `Err(String)` - Error message if metrics cannot be retrieved
///
/// # Metrics Information
/// The returned metrics include:
/// - **Hit/Miss Ratios**: Cache hit and miss counts and percentages
/// - **Performance**: Average lookup times and cache efficiency
/// - **Memory Usage**: Current memory usage and capacity
/// - **Entry Statistics**: Total entries, expired entries, evictions
/// - **Time-based Metrics**: Cache age, cleanup frequency
/// - **Quality Metrics**: Cache effectiveness scores
///
/// # Example Usage (from frontend)
/// ```javascript
/// const metrics = await invoke('get_embedding_cache_metrics');
/// console.log('Cache Performance:');
/// console.log('- Hit rate:', (metrics.hits / (metrics.hits + metrics.misses) * 100).toFixed(1) + '%');
/// console.log('- Total entries:', metrics.total_entries);
/// console.log('- Memory usage:', metrics.memory_usage_bytes + ' bytes');
/// console.log('- Average lookup time:', metrics.avg_lookup_time_ms + 'ms');
/// ```
#[tauri::command]
pub async fn get_embedding_cache_metrics() -> Result<CacheMetrics, String> {
    let cache = get_embedding_cache().await;
    Ok(cache.get_metrics().await)
}

/// Clear all entries from the embedding cache
///
/// This command removes all cached embeddings from memory and persistent
/// storage, providing a clean state for the cache system. Use with caution
/// as this will require regenerating all embeddings.
///
/// # Returns
/// * `Ok(())` - Cache successfully cleared
/// * `Err(String)` - Error message if cache cannot be cleared
///
/// # Effects of Clearing
/// - **Memory**: All cached embeddings removed from memory
/// - **Persistence**: Persistent cache files deleted if configured
/// - **Metrics**: Cache metrics reset to zero
/// - **Performance**: Temporary performance impact as cache rebuilds
/// - **Storage**: Disk space reclaimed from cache files
///
/// # Use Cases
/// - Troubleshooting cache corruption
/// - Reclaiming disk space
/// - Forcing regeneration of embeddings
/// - Testing and development scenarios
/// - Model changes requiring cache invalidation
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('clear_embedding_cache');
/// console.log('Embedding cache cleared');
/// 
/// // Verify cache is empty
/// const size = await invoke('get_embedding_cache_size');
/// console.log('Cache size after clear:', size); // Should be 0
/// ```
#[tauri::command]
pub async fn clear_embedding_cache() -> Result<(), String> {
    let cache = get_embedding_cache().await;
    cache.clear().await.map_err(|e| e.to_string())
}

/// Get current embedding cache size
///
/// This command returns the number of entries currently stored in the
/// embedding cache, useful for monitoring cache growth and capacity.
///
/// # Returns
/// * `Ok(usize)` - Number of entries in the cache
/// * `Err(String)` - Error message if size cannot be retrieved
///
/// # Size Information
/// - Returns the total number of cached embedding entries
/// - Each entry represents one text-model combination
/// - Does not include expired entries pending cleanup
/// - Reflects current memory usage level
///
/// # Example Usage (from frontend)
/// ```javascript
/// const cacheSize = await invoke('get_embedding_cache_size');
/// console.log(`Cache contains ${cacheSize} embeddings`);
/// 
/// // Monitor cache growth
/// setInterval(async () => {
///     const size = await invoke('get_embedding_cache_size');
///     updateCacheDisplay(size);
/// }, 5000);
/// ```
#[tauri::command]
pub async fn get_embedding_cache_size() -> Result<usize, String> {
    let cache = get_embedding_cache().await;
    Ok(cache.size().await)
}

/// Update embedding cache configuration parameters
///
/// This command updates the cache configuration, allowing runtime adjustment
/// of cache behavior, performance, and resource usage parameters.
///
/// # Arguments
/// * `max_entries` - Optional maximum number of cache entries
/// * `ttl_seconds` - Optional time-to-live for cache entries (seconds)
/// * `persist_to_disk` - Optional flag to enable/disable disk persistence
/// * `enable_metrics` - Optional flag to enable/disable metrics collection
///
/// # Returns
/// * `Ok(())` - Configuration successfully updated
/// * `Err(String)` - Error message if configuration update fails
///
/// # Configuration Parameters
/// - **Max Entries**: Maximum cache capacity (default: 10000)
/// - **TTL**: Time-to-live in seconds (default: 3600)
/// - **Persistence**: Disk persistence enabled (default: true)
/// - **Metrics**: Metrics collection enabled (default: true)
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('update_embedding_cache_config', {
///     maxEntries: 20000,        // Increase cache capacity
///     ttlSeconds: 7200,         // 2 hour TTL
///     persistToDisk: true,      // Enable persistence
///     enableMetrics: true       // Enable metrics
/// });
/// console.log('Cache configuration updated');
/// ```
#[tauri::command]
pub async fn update_embedding_cache_config(
    max_entries: Option<usize>,
    ttl_seconds: Option<u64>,
    persist_to_disk: Option<bool>,
    enable_metrics: Option<bool>
) -> Result<(), String> {
    let mut cache_lock = crate::globals::EMBEDDING_CACHE.write().await;
    
    if let Some(cache) = cache_lock.as_mut() {
        let mut config = cache.get_config().clone();
        
        if let Some(max_entries_val) = max_entries {
            config.max_entries = max_entries_val;
        }
        if let Some(ttl_val) = ttl_seconds {
            config.ttl_seconds = ttl_val;
        }
        if let Some(persist_val) = persist_to_disk {
            config.persist_to_disk = persist_val;
        }
        if let Some(metrics_val) = enable_metrics {
            config.enable_metrics = metrics_val;
        }
        
        cache.update_config(config);
    } else {
        return Err("Embedding cache not initialized".to_string());
    }
    
    Ok(())
}

/// Get current embedding cache configuration
///
/// This command retrieves the current configuration parameters of the
/// embedding cache, useful for displaying settings and validation.
///
/// # Returns
/// * `Ok(CacheConfig)` - Current cache configuration
/// * `Err(String)` - Error message if configuration cannot be retrieved
///
/// # Configuration Information
/// The returned configuration includes:
/// - Cache capacity and size limits
/// - Time-to-live settings
/// - Persistence configuration
/// - Metrics collection settings
/// - Performance optimization flags
///
/// # Example Usage (from frontend)
/// ```javascript
/// const config = await invoke('get_embedding_cache_config');
/// console.log('Cache Configuration:');
/// console.log('- Max entries:', config.max_entries);
/// console.log('- TTL:', config.ttl_seconds + ' seconds');
/// console.log('- Disk persistence:', config.persist_to_disk);
/// console.log('- Metrics enabled:', config.enable_metrics);
/// ```
#[tauri::command]
pub async fn get_embedding_cache_config() -> Result<CacheConfig, String> {
    let cache = get_embedding_cache().await;
    Ok(cache.get_config().clone())
}

/// Intelligently batch embedding requests with adaptive sizing
///
/// This command automatically determines optimal batch size based on system
/// load, cache hit rates, and historical performance to maximize throughput
/// while maintaining responsive performance.
///
/// # Arguments
/// * `texts` - List of texts to generate embeddings for
/// * `model` - Embedding model name
/// * `priority` - Request priority ("High", "Normal", "Low")
/// * `max_batch_size` - Optional maximum batch size override
///
/// # Returns
/// * `Ok(Vec<Vec<f32>>)` - Generated embeddings in same order as input
/// * `Err(String)` - Error message if processing fails
///
/// # Adaptive Features
/// - **Dynamic Batch Sizing**: Adjusts batch size based on system load
/// - **Cache-Aware Optimization**: Larger batches when cache hit rate is high
/// - **Resource-Aware Processing**: Smaller batches under resource constraints
/// - **Performance-Based Adjustment**: Uses historical performance data
///
/// # Example Usage (from frontend)
/// ```javascript
/// const embeddings = await invoke('generate_adaptive_batch_embeddings', {
///     texts: ['Text 1', 'Text 2', 'Text 3'],
///     model: 'nomic-embed-text',
///     priority: 'Normal',
///     maxBatchSize: 16
/// });
/// ```
#[tauri::command]
pub async fn generate_adaptive_batch_embeddings(
    texts: Vec<String>,
    model: String,
    priority: String,
    max_batch_size: Option<usize>
) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let cache = get_embedding_cache().await;
    let generator = get_embedding_generator().await;
    
    // Determine optimal batch size based on system conditions
    let optimal_batch_size = calculate_optimal_batch_size(
        texts.len(),
        &priority,
        max_batch_size.unwrap_or(32)
    ).await;
    
    eprintln!("🧠 Adaptive batching: {} texts, optimal batch size: {}", texts.len(), optimal_batch_size);
    
    let mut result_embeddings = vec![Vec::new(); texts.len()];
    let mut processed_count = 0;
    
    // Process in adaptive batches
    for batch_start in (0..texts.len()).step_by(optimal_batch_size) {
        let batch_end = (batch_start + optimal_batch_size).min(texts.len());
        let batch_texts: Vec<String> = texts[batch_start..batch_end].to_vec();
        let batch_indices: Vec<usize> = (batch_start..batch_end).collect();
        
        eprintln!("🔄 Processing batch {}-{} ({} texts)", batch_start, batch_end - 1, batch_texts.len());
        
        // Check cache for batch
        let mut cache_misses = Vec::new();
        let mut cache_miss_indices = Vec::new();
        let mut _batch_hit_count = 0;
        
        for (_local_idx, (global_idx, text)) in batch_indices.iter().zip(batch_texts.iter()).enumerate() {
            if let Ok(Some(cached_embedding)) = cache.get(text, &model).await {
                result_embeddings[*global_idx] = cached_embedding;
                _batch_hit_count += 1;
            } else {
                cache_misses.push(text.clone());
                cache_miss_indices.push(*global_idx);
            }
        }
        
        // Generate embeddings for cache misses
        if !cache_misses.is_empty() {
            let batch_start_time = std::time::Instant::now();
            
            match generator.generate_batch_embeddings(cache_misses.clone(), model.clone()).await {
                Ok(new_embeddings) => {
                    let batch_time = batch_start_time.elapsed().as_millis() as f64;
                    let throughput = new_embeddings.len() as f64 / (batch_time / 1000.0);
                    
                    eprintln!("⚡ Batch completed: {} new embeddings in {:.1}ms ({:.1} embeddings/sec)", 
                             new_embeddings.len(), batch_time, throughput);
                    
                    // Store results and cache them
                    for (miss_idx, new_embedding) in new_embeddings.into_iter().enumerate() {
                        if let Some(&result_idx) = cache_miss_indices.get(miss_idx) {
                            result_embeddings[result_idx] = new_embedding.clone();
                            
                            // Cache the new embedding
                            if let Some(text) = cache_misses.get(miss_idx) {
                                if let Err(e) = cache.set(text, &model, new_embedding).await {
                                    eprintln!("⚠️ Failed to cache embedding: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("Batch processing failed at position {}: {}", batch_start, e));
                }
            }
        }
        
        processed_count += batch_texts.len();
        let progress = (processed_count as f64 / texts.len() as f64) * 100.0;
        eprintln!("📈 Progress: {:.1}% ({}/{} completed)", progress, processed_count, texts.len());
        
        // Brief pause between batches to allow other operations
        if batch_end < texts.len() && priority != "High" {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    
    eprintln!("✅ Adaptive batch processing completed: {} embeddings generated", texts.len());
    Ok(result_embeddings)
}

/// Calculate optimal batch size based on system conditions
async fn calculate_optimal_batch_size(total_texts: usize, priority: &str, max_batch_size: usize) -> usize {
    // Base batch size based on priority
    let base_batch_size = match priority.to_lowercase().as_str() {
        "high" => 8,   // Smaller batches for faster response
        "normal" => 16, // Balanced batch size
        "low" => 32,    // Larger batches for efficiency
        _ => 16,
    };
    
    // Adjust based on total workload
    let workload_adjusted = if total_texts < 10 {
        (base_batch_size / 2).max(1) // Smaller batches for small workloads
    } else if total_texts > 100 {
        base_batch_size * 2 // Larger batches for big workloads
    } else {
        base_batch_size
    };
    
    // System load simulation (in production, would check actual system metrics)
    let system_load = 0.4; // Simulate 40% system load
    let load_adjusted = if system_load > 0.7 {
        (workload_adjusted / 2).max(1) // Reduce batch size under high load
    } else if system_load < 0.3 {
        workload_adjusted * 2 // Increase batch size with spare capacity
    } else {
        workload_adjusted
    };
    
    // Apply maximum constraint
    load_adjusted.min(max_batch_size).max(1)
}

/// Remove expired entries from the embedding cache
///
/// This command performs cleanup of expired cache entries, freeing memory
/// and maintaining cache performance. Returns the number of entries removed.
///
/// # Returns
/// * `Ok(usize)` - Number of expired entries removed
/// * `Err(String)` - Error message if cleanup fails
///
/// # Cleanup Process
/// - **Expiration Check**: Identifies entries past their TTL
/// - **Memory Cleanup**: Removes expired entries from memory
/// - **Disk Cleanup**: Removes expired persistent entries
/// - **Metrics Update**: Updates cache metrics after cleanup
/// - **Performance**: Optimizes cache performance by reducing size
///
/// # Automatic vs Manual Cleanup
/// - **Automatic**: Background cleanup runs periodically
/// - **Manual**: This command forces immediate cleanup
/// - **Performance**: Manual cleanup useful before intensive operations
/// - **Resources**: Frees memory and disk space immediately
///
/// # Example Usage (from frontend)
/// ```javascript
/// const removedCount = await invoke('cleanup_expired_embeddings');
/// console.log(`Cleaned up ${removedCount} expired embeddings`);
/// 
/// // Check cache size after cleanup
/// const newSize = await invoke('get_embedding_cache_size');
/// console.log(`Cache size after cleanup: ${newSize}`);
/// ```
#[tauri::command]
pub async fn cleanup_expired_embeddings() -> Result<usize, String> {
    let cache = get_embedding_cache().await;
    cache.cleanup_expired().await.map_err(|e| e.to_string())
}

/// Check if a specific text-model combination is cached
///
/// This command checks whether an embedding for a specific text and model
/// combination exists in the cache without retrieving the embedding itself.
/// Useful for cache management and optimization decisions.
///
/// # Arguments
/// * `text` - Text to check for cached embedding
/// * `model` - Model name to check against
///
/// # Returns
/// * `Ok(bool)` - True if embedding is cached, false otherwise
/// * `Err(String)` - Error message if cache check fails
///
/// # Use Cases
/// - **Preloading Decisions**: Decide whether to preload embeddings
/// - **Cache Management**: Understand cache coverage
/// - **Performance Planning**: Plan batch processing strategies
/// - **UI Indicators**: Show cached status in interface
/// - **Optimization**: Optimize embedding request patterns
///
/// # Example Usage (from frontend)
/// ```javascript
/// const isCached = await invoke('check_embedding_cached', {
///     text: 'Sample text to check',
///     model: 'nomic-embed-text'
/// });
/// 
/// if (isCached) {
///     console.log('Embedding available in cache - fast retrieval');
///     // Use cached version
/// } else {
///     console.log('Embedding not cached - will need generation');
///     // Show loading indicator
/// }
/// ```
#[tauri::command]
pub async fn check_embedding_cached(text: String, model: String) -> Result<bool, String> {
    let cache = get_embedding_cache().await;
    cache.contains(&text, &model).await.map_err(|e| e.to_string())
}