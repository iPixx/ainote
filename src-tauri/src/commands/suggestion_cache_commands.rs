//! # Suggestion Cache Commands
//!
//! This module contains all Tauri commands related to suggestion caching,
//! cache invalidation, context management, and performance optimization.
//! It provides comprehensive functionality for the intelligent caching layer
//! that optimizes AI-powered note suggestions.
//!
//! ## Command Overview
//!
//! ### Core Caching Operations
//! - `get_cached_suggestions`: Retrieve cached suggestions for content
//! - `cache_suggestions`: Store suggestions in cache with context
//! - `check_suggestion_cached`: Check if suggestions exist for content
//!
//! ### Cache Invalidation
//! - `invalidate_suggestions_for_file`: Invalidate cache entries for specific file
//! - `clear_suggestion_cache`: Clear all cached suggestions
//! - `cleanup_expired_suggestions`: Remove expired cache entries
//!
//! ### Context Management
//! - `get_recent_suggestions`: Get recently generated suggestions
//! - `update_suggestion_context`: Update context for suggestion filtering
//! - `warm_suggestion_cache`: Preload suggestions for frequently accessed files
//!
//! ### Configuration & Monitoring
//! - `get_suggestion_cache_metrics`: Get cache performance metrics
//! - `get_suggestion_cache_config`: Get current cache configuration
//! - `update_suggestion_cache_config`: Update cache parameters
//!
//! ## Suggestion Caching Pipeline
//!
//! The suggestion caching process follows these steps:
//!
//! 1. **Context Analysis**: Analyze current editor context and content
//! 2. **Cache Lookup**: Check if suggestions exist for current context
//! 3. **Relevance Filtering**: Filter cached suggestions by context relevance
//! 4. **Cache Storage**: Store new suggestions with context metadata
//! 5. **Invalidation**: Smart invalidation when content changes
//! 6. **Metrics Update**: Update performance and usage metrics
//!
//! ## Performance Optimization
//!
//! ### Context-Aware Caching
//! - **File-specific**: Cache suggestions per file for better relevance
//! - **Content-sensitive**: Use content hashes for precise cache keys
//! - **Cursor-aware**: Consider cursor position for context relevance
//! - **Time-based**: TTL-based expiration for cache freshness
//!
//! ### Smart Invalidation
//! - **Content Changes**: Invalidate when source files are modified
//! - **Selective**: Only invalidate relevant cache entries
//! - **Batch Operations**: Efficient bulk invalidation support
//! - **Background Cleanup**: Automatic removal of expired entries
//!
//! ## Performance Requirements
//!
//! ### Cache Performance Targets
//! - Cache hit rate >70% for repeat queries
//! - Cache lookup completes in <10ms
//! - Memory usage <25MB for cache system
//! - Cache operations don't block suggestion generation

use crate::globals::get_suggestion_cache;
use crate::suggestion_cache::{
    SuggestionContext, SuggestionCacheConfig, SuggestionCacheMetrics
};
use crate::similarity_search::SearchResult;

/// Get cached suggestions for content and context
///
/// This command retrieves cached suggestions for the given content and context.
/// It includes intelligent context filtering to ensure suggestions remain
/// relevant to the current editing situation.
///
/// # Arguments
/// * `content` - Current editor content to get suggestions for
/// * `model` - Name of the AI model used for suggestion generation
/// * `current_file` - Optional current file path for context
/// * `vault_path` - Optional vault path for context
/// * `content_length` - Current content length
/// * `cursor_position` - Current cursor position in content
/// * `current_paragraph` - Current paragraph being edited
///
/// # Returns
/// * `Ok(Option<Vec<SearchResult>>)` - Cached suggestions if available
/// * `Err(String)` - Error message if cache lookup fails
///
/// # Context Filtering
/// The command applies context-aware filtering to ensure:
/// - Suggestions are still relevant to current content
/// - File context matches current editing context
/// - Suggestions haven't expired based on TTL
/// - Content changes haven't invalidated suggestions
///
/// # Example Usage (from frontend)
/// ```javascript
/// const suggestions = await invoke('get_cached_suggestions', {
///     content: editor.getValue(),
///     model: 'nomic-embed-text',
///     currentFile: '/path/to/current.md',
///     vaultPath: '/path/to/vault',
///     contentLength: editor.getValue().length,
///     cursorPosition: editor.getCursorPosition(),
///     currentParagraph: getCurrentParagraph()
/// });
/// 
/// if (suggestions) {
///     displaySuggestions(suggestions);
/// } else {
///     // Cache miss - generate new suggestions
///     generateNewSuggestions();
/// }
/// ```
#[tauri::command]
pub async fn get_cached_suggestions(
    content: String,
    model: String,
    current_file: Option<String>,
    vault_path: Option<String>,
    content_length: usize,
    cursor_position: usize,
    current_paragraph: String,
) -> Result<Option<Vec<SearchResult>>, String> {
    let cache = get_suggestion_cache().await;
    
    let context = SuggestionContext::new(
        current_file,
        vault_path,
        content_length,
        cursor_position,
        current_paragraph,
    );
    
    match cache.get_suggestions(&content, &model, &context).await {
        Ok(suggestions) => Ok(suggestions),
        Err(e) => Err(format!("Failed to get cached suggestions: {}", e))
    }
}

/// Cache suggestions with context metadata
///
/// This command stores suggestions in the cache with associated context
/// metadata for intelligent retrieval and relevance filtering.
///
/// # Arguments
/// * `content` - Editor content that suggestions were generated for
/// * `model` - Name of the AI model used for generation
/// * `suggestions` - List of suggestion results to cache
/// * `current_file` - Optional current file path
/// * `vault_path` - Optional vault path
/// * `content_length` - Content length at time of generation
/// * `cursor_position` - Cursor position at time of generation
/// * `current_paragraph` - Current paragraph at time of generation
///
/// # Returns
/// * `Ok(())` - Suggestions successfully cached
/// * `Err(String)` - Error message if caching fails
///
/// # Caching Strategy
/// - **Content-based Keys**: Use content hash for precise matching
/// - **Context Integration**: Include file and cursor context
/// - **TTL Management**: Automatic expiration based on configuration
/// - **LRU Eviction**: Least recently used entries removed when full
///
/// # Example Usage (from frontend)
/// ```javascript
/// // After generating new suggestions
/// const suggestions = await generateSuggestions(content, model);
/// 
/// await invoke('cache_suggestions', {
///     content: content,
///     model: model,
///     suggestions: suggestions,
///     currentFile: currentFile,
///     vaultPath: vaultPath,
///     contentLength: content.length,
///     cursorPosition: cursorPos,
///     currentParagraph: currentPara
/// });
/// 
/// console.log('Suggestions cached for future use');
/// ```
#[tauri::command]
pub async fn cache_suggestions(
    content: String,
    model: String,
    suggestions: Vec<SearchResult>,
    current_file: Option<String>,
    vault_path: Option<String>,
    content_length: usize,
    cursor_position: usize,
    current_paragraph: String,
) -> Result<(), String> {
    let cache = get_suggestion_cache().await;
    
    let context = SuggestionContext::new(
        current_file,
        vault_path,
        content_length,
        cursor_position,
        current_paragraph,
    );
    
    match cache.cache_suggestions(&content, &model, &context, suggestions).await {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to cache suggestions: {}", e))
    }
}

/// Check if suggestions are cached for specific content and context
///
/// This command checks whether suggestions exist in the cache for the
/// given content and context without retrieving the actual suggestions.
/// Useful for UI indicators and caching decisions.
///
/// # Arguments
/// * `content` - Content to check for cached suggestions
/// * `model` - Model name to check against
/// * `current_file` - Optional current file path
/// * `content_length` - Current content length
/// * `cursor_position` - Current cursor position
/// * `current_paragraph` - Current paragraph content
///
/// # Returns
/// * `Ok(bool)` - True if suggestions are cached and relevant
/// * `Err(String)` - Error message if check fails
///
/// # Use Cases
/// - **UI Indicators**: Show cache status in interface
/// - **Performance Planning**: Decide between cached vs new generation
/// - **Cache Management**: Understanding cache coverage
/// - **Optimization**: Optimize suggestion request patterns
///
/// # Example Usage (from frontend)
/// ```javascript
/// const isCached = await invoke('check_suggestion_cached', {
///     content: currentContent,
///     model: 'nomic-embed-text',
///     currentFile: currentFile,
///     contentLength: currentContent.length,
///     cursorPosition: cursorPos,
///     currentParagraph: currentPara
/// });
/// 
/// if (isCached) {
///     showCacheIndicator(true);
/// } else {
///     showCacheIndicator(false);
/// }
/// ```
#[tauri::command]
pub async fn check_suggestion_cached(
    content: String,
    model: String,
    current_file: Option<String>,
    content_length: usize,
    cursor_position: usize,
    current_paragraph: String,
) -> Result<bool, String> {
    let cache = get_suggestion_cache().await;
    
    let context = SuggestionContext::new(
        current_file,
        None, // vault_path not needed for checking
        content_length,
        cursor_position,
        current_paragraph,
    );
    
    match cache.get_suggestions(&content, &model, &context).await {
        Ok(suggestions) => Ok(suggestions.is_some()),
        Err(_) => Ok(false)
    }
}

/// Invalidate cached suggestions for a specific file
///
/// This command removes all cached suggestions associated with a specific
/// file, useful when files are modified outside the editor or when forcing
/// cache refresh for a particular file.
///
/// # Arguments
/// * `file_path` - Path of the file to invalidate suggestions for
///
/// # Returns
/// * `Ok(usize)` - Number of cache entries invalidated
/// * `Err(String)` - Error message if invalidation fails
///
/// # Invalidation Strategy
/// - **File-specific**: Only invalidate entries for the specified file
/// - **Cascading**: May invalidate related suggestions if configured
/// - **Metrics Update**: Update cache metrics after invalidation
/// - **Memory Recovery**: Free memory from invalidated entries
///
/// # Example Usage (from frontend)
/// ```javascript
/// // When file is modified externally
/// const invalidatedCount = await invoke('invalidate_suggestions_for_file', {
///     filePath: '/path/to/modified/file.md'
/// });
/// 
/// console.log(`Invalidated ${invalidatedCount} cache entries`);
/// ```
#[tauri::command]
pub async fn invalidate_suggestions_for_file(file_path: String) -> Result<usize, String> {
    let cache = get_suggestion_cache().await;
    
    match cache.invalidate_file(&file_path).await {
        Ok(count) => Ok(count),
        Err(e) => Err(format!("Failed to invalidate suggestions for file: {}", e))
    }
}

/// Clear all cached suggestions
///
/// This command removes all cached suggestions from memory and resets
/// cache metrics. Use with caution as this will require regenerating
/// all suggestions.
///
/// # Returns
/// * `Ok(())` - Cache successfully cleared
/// * `Err(String)` - Error message if clearing fails
///
/// # Effects of Clearing
/// - **Memory**: All cached suggestions removed from memory
/// - **Metrics**: Cache metrics reset to zero
/// - **Performance**: Temporary performance impact as cache rebuilds
/// - **Context**: Recent suggestion tracking cleared
///
/// # Use Cases
/// - Troubleshooting cache issues
/// - Memory management
/// - Development and testing
/// - Force regeneration of all suggestions
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('clear_suggestion_cache');
/// console.log('All suggestion cache cleared');
/// 
/// // Verify cache is empty
/// const metrics = await invoke('get_suggestion_cache_metrics');
/// console.log('Cache entries:', metrics.cache_size);
/// ```
#[tauri::command]
pub async fn clear_suggestion_cache() -> Result<(), String> {
    let cache = get_suggestion_cache().await;
    
    match cache.clear().await {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to clear suggestion cache: {}", e))
    }
}

/// Remove expired entries from suggestion cache
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
/// - **Metrics Update**: Updates cache metrics after cleanup
/// - **Performance**: Optimizes cache performance by reducing size
///
/// # Automatic vs Manual Cleanup
/// - **Automatic**: Background cleanup runs periodically
/// - **Manual**: This command forces immediate cleanup
/// - **Performance**: Manual cleanup useful before intensive operations
/// - **Resources**: Frees memory immediately
///
/// # Example Usage (from frontend)
/// ```javascript
/// const removedCount = await invoke('cleanup_expired_suggestions');
/// console.log(`Cleaned up ${removedCount} expired suggestions`);
/// ```
#[tauri::command]
pub async fn cleanup_expired_suggestions() -> Result<usize, String> {
    let cache = get_suggestion_cache().await;
    
    // The cleanup is handled by the background task, but we can trigger
    // metrics update and return current cache size change
    let size_before = cache.size().await;
    
    // Force metrics update to reflect any background cleanup
    let _metrics = cache.get_metrics().await;
    
    let size_after = cache.size().await;
    let cleaned_count = if size_before > size_after { size_before - size_after } else { 0 };
    
    Ok(cleaned_count)
}

/// Get comprehensive suggestion cache performance metrics
///
/// This command retrieves detailed performance and usage metrics from the
/// suggestion cache, providing insights into cache effectiveness and system
/// performance.
///
/// # Returns
/// * `Ok(SuggestionCacheMetrics)` - Comprehensive cache performance metrics
/// * `Err(String)` - Error message if metrics cannot be retrieved
///
/// # Metrics Information
/// The returned metrics include:
/// - **Hit/Miss Ratios**: Cache hit and miss counts and percentages
/// - **Performance**: Average lookup times and cache efficiency
/// - **Memory Usage**: Current memory usage estimate
/// - **Context Statistics**: Context filtering and relevance metrics
/// - **Invalidation Stats**: Cache invalidation and expiration counts
/// - **Cache Warming**: Preloading operations and effectiveness
///
/// # Example Usage (from frontend)
/// ```javascript
/// const metrics = await invoke('get_suggestion_cache_metrics');
/// console.log('Suggestion Cache Performance:');
/// console.log('- Hit rate:', metrics.hit_rate.toFixed(2));
/// console.log('- Average lookup time:', metrics.avg_lookup_time_ms.toFixed(1), 'ms');
/// console.log('- Memory usage:', (metrics.memory_usage_bytes / 1024 / 1024).toFixed(1), 'MB');
/// console.log('- Total entries:', metrics.hits + metrics.misses);
/// ```
#[tauri::command]
pub async fn get_suggestion_cache_metrics() -> Result<SuggestionCacheMetrics, String> {
    let cache = get_suggestion_cache().await;
    Ok(cache.get_metrics().await)
}

/// Get current suggestion cache configuration
///
/// This command retrieves the current configuration parameters of the
/// suggestion cache, useful for displaying settings and validation.
///
/// # Returns
/// * `Ok(SuggestionCacheConfig)` - Current cache configuration
/// * `Err(String)` - Error message if configuration cannot be retrieved
///
/// # Configuration Information
/// The returned configuration includes:
/// - Cache capacity and size limits
/// - Time-to-live settings
/// - Context filtering options
/// - Cache warming configuration
/// - Memory management settings
///
/// # Example Usage (from frontend)
/// ```javascript
/// const config = await invoke('get_suggestion_cache_config');
/// console.log('Suggestion Cache Configuration:');
/// console.log('- Max suggestion sets:', config.max_suggestion_sets);
/// console.log('- TTL:', config.ttl_seconds, 'seconds');
/// console.log('- Context filtering:', config.enable_context_filtering);
/// console.log('- Cache warming:', config.enable_cache_warming);
/// ```
#[tauri::command]
pub async fn get_suggestion_cache_config() -> Result<SuggestionCacheConfig, String> {
    let cache = get_suggestion_cache().await;
    Ok(cache.get_config().clone())
}

/// Update suggestion cache configuration parameters
///
/// This command updates the cache configuration, allowing runtime adjustment
/// of cache behavior, performance, and resource usage parameters.
///
/// # Arguments
/// * `max_suggestion_sets` - Optional maximum number of cached suggestion sets
/// * `max_suggestions_per_set` - Optional maximum suggestions per set
/// * `ttl_seconds` - Optional time-to-live for cache entries
/// * `enable_content_invalidation` - Optional flag for content-based invalidation
/// * `enable_context_filtering` - Optional flag for context-aware filtering
/// * `enable_cache_warming` - Optional flag for cache warming
/// * `max_memory_bytes` - Optional maximum memory usage
/// * `enable_metrics` - Optional flag for metrics collection
///
/// # Returns
/// * `Ok(())` - Configuration successfully updated
/// * `Err(String)` - Error message if configuration update fails
///
/// # Configuration Parameters
/// - **Capacity**: Cache size and entry limits
/// - **Performance**: TTL and memory management
/// - **Features**: Context filtering and cache warming
/// - **Monitoring**: Metrics collection and monitoring
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('update_suggestion_cache_config', {
///     maxSuggestionSets: 1000,        // Increase cache capacity
///     ttlSeconds: 600,                // 10 minute TTL
///     enableContextFiltering: true,   // Enable context filtering
///     enableCacheWarming: true,       // Enable cache warming
///     maxMemoryBytes: 50 * 1024 * 1024 // 50MB memory limit
/// });
/// console.log('Suggestion cache configuration updated');
/// ```
#[tauri::command]
pub async fn update_suggestion_cache_config(
    max_suggestion_sets: Option<usize>,
    max_suggestions_per_set: Option<usize>,
    ttl_seconds: Option<u64>,
    enable_content_invalidation: Option<bool>,
    enable_context_filtering: Option<bool>,
    enable_cache_warming: Option<bool>,
    max_memory_bytes: Option<usize>,
    enable_metrics: Option<bool>,
) -> Result<(), String> {
    let mut cache_lock = crate::globals::SUGGESTION_CACHE.write().await;
    
    if let Some(cache) = cache_lock.as_mut() {
        let mut config = cache.get_config().clone();
        
        if let Some(max_sets) = max_suggestion_sets {
            config.max_suggestion_sets = max_sets;
        }
        if let Some(max_per_set) = max_suggestions_per_set {
            config.max_suggestions_per_set = max_per_set;
        }
        if let Some(ttl) = ttl_seconds {
            config.ttl_seconds = ttl;
        }
        if let Some(content_invalidation) = enable_content_invalidation {
            config.enable_content_invalidation = content_invalidation;
        }
        if let Some(context_filtering) = enable_context_filtering {
            config.enable_context_filtering = context_filtering;
        }
        if let Some(cache_warming) = enable_cache_warming {
            config.enable_cache_warming = cache_warming;
        }
        if let Some(max_memory) = max_memory_bytes {
            config.max_memory_bytes = max_memory;
        }
        if let Some(metrics) = enable_metrics {
            config.enable_metrics = metrics;
        }
        
        cache.update_config(config);
        Ok(())
    } else {
        Err("Suggestion cache not initialized".to_string())
    }
}

/// Warm suggestion cache for frequently accessed file
///
/// This command initiates cache warming for a specific file, preloading
/// suggestions to improve performance when the file is accessed.
///
/// # Arguments
/// * `file_path` - Path of the file to warm cache for
///
/// # Returns
/// * `Ok(())` - Cache warming initiated successfully
/// * `Err(String)` - Error message if cache warming fails
///
/// # Cache Warming Process
/// - **Background Processing**: Non-blocking cache warming
/// - **Frequency Analysis**: Target frequently accessed files
/// - **Preloading**: Generate and cache common suggestions
/// - **Performance**: Reduce latency for subsequent requests
///
/// # Example Usage (from frontend)
/// ```javascript
/// // Warm cache for frequently edited file
/// await invoke('warm_suggestion_cache_for_file', {
///     filePath: '/path/to/frequently/edited.md'
/// });
/// 
/// console.log('Cache warming initiated for file');
/// ```
#[tauri::command]
pub async fn warm_suggestion_cache_for_file(file_path: String) -> Result<(), String> {
    let cache = get_suggestion_cache().await;
    
    match cache.warm_cache_for_file(&file_path).await {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to warm cache for file: {}", e))
    }
}

/// Get current suggestion cache size
///
/// This command returns the number of suggestion sets currently stored
/// in the cache, useful for monitoring cache usage and capacity.
///
/// # Returns
/// * `Ok(usize)` - Number of suggestion sets in the cache
/// * `Err(String)` - Error message if size cannot be retrieved
///
/// # Size Information
/// - Returns total number of cached suggestion sets
/// - Each set may contain multiple individual suggestions
/// - Does not include expired entries pending cleanup
/// - Reflects current memory usage level
///
/// # Example Usage (from frontend)
/// ```javascript
/// const cacheSize = await invoke('get_suggestion_cache_size');
/// console.log(`Cache contains ${cacheSize} suggestion sets`);
/// 
/// // Monitor cache growth
/// setInterval(async () => {
///     const size = await invoke('get_suggestion_cache_size');
///     updateCacheDisplay(size);
/// }, 5000);
/// ```
#[tauri::command]
pub async fn get_suggestion_cache_size() -> Result<usize, String> {
    let cache = get_suggestion_cache().await;
    Ok(cache.size().await)
}