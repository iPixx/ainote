//! Search Commands for aiNote Similarity Search Engine
//! 
//! This module implements the Tauri commands for similarity search functionality,
//! providing the interface between the frontend and the core similarity algorithms
//! and vector database system.
//! 
//! ## Features
//! 
//! - **search_similar_notes()** - Main similarity search command with configurable parameters
//! - **Result ranking and scoring** - Intelligent ranking of search results
//! - **Batch search support** - Efficient processing of multiple search queries
//! - **Search result caching** - Performance optimization for repeated queries
//! - **Configurable thresholds** - Flexible similarity thresholds and result limits
//! - **Comprehensive error handling** - Robust error handling with detailed error messages
//! 
//! ## Performance Requirements
//! 
//! - Search 1000 vectors in <50ms
//! - Configurable result limits (default 10, max 50)
//! - Memory efficient caching with automatic cleanup
//! - Support for concurrent search requests
//! 
//! ## Integration
//! 
//! This module integrates with:
//! - `similarity_search.rs` - Core mathematical algorithms
//! - `vector_db` - Vector storage and retrieval
//! - `embedding_cache` - Caching layer for performance
//! - Tauri command system for frontend communication

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use thiserror::Error;

// Import core functionality
use crate::similarity_search::{SimilaritySearch, SearchConfig, SearchResult, SimilarityError};
use crate::vector_db::VectorDatabase;
use crate::vector_db::types::{EmbeddingEntry, VectorDbError};

/// Errors specific to search command operations
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum SearchCommandError {
    #[error("Similarity search error: {0}")]
    SimilarityError(#[from] SimilarityError),
    
    #[error("Vector database error: {message}")]
    VectorDbError { message: String },
    
    #[error("Invalid search parameters: {reason}")]
    InvalidParameters { reason: String },
    
    #[error("Search timeout: operation took longer than {timeout_ms}ms")]
    SearchTimeout { timeout_ms: u64 },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("Search result limit exceeded: requested {requested}, max allowed {max_allowed}")]
    ResultLimitExceeded { requested: usize, max_allowed: usize },
    
    #[error("Vector dimension mismatch: query has {query_dim}, database has {db_dim}")]
    DimensionMismatch { query_dim: usize, db_dim: usize },
    
    #[error("No embeddings found in database")]
    EmptyDatabase,
}

impl From<VectorDbError> for SearchCommandError {
    fn from(error: VectorDbError) -> Self {
        SearchCommandError::VectorDbError {
            message: error.to_string(),
        }
    }
}

pub type SearchCommandResult<T> = Result<T, SearchCommandError>;

/// Configuration for similarity search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilaritySearchConfig {
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_similarity: f32,
    /// Maximum number of results to return (default 10, max 50)
    pub max_results: usize,
    /// Enable result caching for performance
    pub enable_caching: bool,
    /// Cache TTL in seconds (default 300 = 5 minutes)
    pub cache_ttl_seconds: u64,
    /// Timeout for search operations in milliseconds
    pub timeout_ms: u64,
    /// Enable early termination optimization
    pub enable_early_termination: bool,
    /// Pre-normalize query vectors for faster computation
    pub normalize_query: bool,
    /// Exclude current file from results (if specified)
    pub exclude_file_path: Option<String>,
}

impl Default for SimilaritySearchConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.1,
            max_results: 10,
            enable_caching: true,
            cache_ttl_seconds: 300,
            timeout_ms: 5000, // 5 second timeout
            enable_early_termination: true,
            normalize_query: true,
            exclude_file_path: None,
        }
    }
}

/// Similarity search result with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilaritySearchResult {
    /// Unique ID of the embedding entry
    pub entry_id: String,
    /// File path of the source document
    pub file_path: String,
    /// Chunk ID within the file
    pub chunk_id: String,
    /// Cosine similarity score (0.0 to 1.0)
    pub similarity_score: f32,
    /// Preview of the original text content
    pub text_preview: String,
    /// Model used to generate the embedding
    pub model_name: String,
    /// Timestamp when the embedding was created
    pub created_at: u64,
    /// Relevance rank in the result set (1-based)
    pub relevance_rank: usize,
    /// Additional metadata for context
    pub metadata: HashMap<String, String>,
}

impl From<SearchResult> for SimilaritySearchResult {
    fn from(search_result: SearchResult) -> Self {
        let entry = search_result.entry;
        let metadata = HashMap::from([
            ("file_size".to_string(), format!("{}", entry.vector.len())),
            ("vector_dimension".to_string(), format!("{}", entry.vector.len())),
        ]);

        Self {
            entry_id: entry.id.clone(),
            file_path: entry.metadata.file_path.clone(),
            chunk_id: entry.metadata.chunk_id.clone(),
            similarity_score: search_result.similarity,
            text_preview: entry.metadata.content_preview.chars().take(200).collect(),
            model_name: entry.metadata.model_name.clone(),
            created_at: entry.metadata.created_at,
            relevance_rank: 1, // Will be set later during ranking
            metadata,
        }
    }
}

/// Batch search request for multiple queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSearchRequest {
    /// List of query vectors to search for
    pub query_vectors: Vec<Vec<f32>>,
    /// Search configuration applied to all queries
    pub config: SimilaritySearchConfig,
    /// Optional labels for each query (for result identification)
    pub query_labels: Option<Vec<String>>,
}

/// Batch search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSearchResult {
    /// Results for each query in the same order
    pub results: Vec<Vec<SimilaritySearchResult>>,
    /// Optional labels corresponding to each query
    pub query_labels: Option<Vec<String>>,
    /// Total number of queries processed
    pub total_queries: usize,
    /// Total search time in milliseconds
    pub total_search_time_ms: u64,
    /// Average search time per query in milliseconds
    pub avg_search_time_ms: u64,
}

/// Search result cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached search results
    results: Vec<SimilaritySearchResult>,
    /// Timestamp when cached
    cached_at: Instant,
    /// TTL for this cache entry
    ttl: Duration,
    /// Hit count for LRU eviction
    hit_count: usize,
}

impl CacheEntry {
    fn new(results: Vec<SimilaritySearchResult>, ttl: Duration) -> Self {
        Self {
            results,
            cached_at: Instant::now(),
            ttl,
            hit_count: 1,
        }
    }
    
    /// Check if this cache entry has expired
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
    
    /// Record a cache hit and return the results
    fn hit(&mut self) -> Vec<SimilaritySearchResult> {
        self.hit_count += 1;
        self.results.clone()
    }
}

/// Search result cache with TTL and LRU eviction
#[derive(Debug)]
pub struct SearchResultCache {
    /// Cache storage
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Maximum number of cached entries
    max_entries: usize,
    /// Default TTL for cache entries
    #[allow(dead_code)]
    default_ttl: Duration,
}

impl SearchResultCache {
    /// Create a new search result cache
    pub fn new(max_entries: usize, default_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            default_ttl: Duration::from_secs(default_ttl_seconds),
        }
    }
    
    /// Generate cache key from query vector and config
    fn generate_cache_key(query_vector: &[f32], config: &SimilaritySearchConfig) -> String {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        
        // Hash the query vector
        for &value in query_vector {
            hasher.update(value.to_le_bytes());
        }
        
        // Hash the search configuration
        hasher.update(config.min_similarity.to_le_bytes());
        hasher.update((config.max_results as u32).to_le_bytes());
        hasher.update(if config.normalize_query { [1u8] } else { [0u8] });
        
        if let Some(ref exclude_path) = config.exclude_file_path {
            hasher.update(exclude_path.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    /// Get cached results if available and not expired
    pub async fn get(&self, query_vector: &[f32], config: &SimilaritySearchConfig) -> Option<Vec<SimilaritySearchResult>> {
        let key = Self::generate_cache_key(query_vector, config);
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&key) {
            if !entry.is_expired() {
                return Some(entry.hit());
            } else {
                // Remove expired entry
                cache.remove(&key);
            }
        }
        
        None
    }
    
    /// Cache search results
    pub async fn put(&self, 
        query_vector: &[f32], 
        config: &SimilaritySearchConfig, 
        results: Vec<SimilaritySearchResult>
    ) -> Result<(), SearchCommandError> {
        let key = Self::generate_cache_key(query_vector, config);
        let ttl = Duration::from_secs(config.cache_ttl_seconds);
        let entry = CacheEntry::new(results, ttl);
        
        let mut cache = self.cache.write().await;
        
        // Check if cache is full and evict LRU entry
        if cache.len() >= self.max_entries && !cache.contains_key(&key) {
            if let Some(lru_key) = cache.iter()
                .min_by_key(|(_, entry)| entry.hit_count)
                .map(|(key, _)| key.clone()) {
                cache.remove(&lru_key);
            }
        }
        
        cache.insert(key, entry);
        Ok(())
    }
    
    /// Clear all cached entries
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    /// Remove expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        
        cache.retain(|_, entry| !entry.is_expired());
        
        initial_size - cache.len()
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> HashMap<String, u64> {
        let cache = self.cache.read().await;
        
        let mut stats = HashMap::new();
        stats.insert("total_entries".to_string(), cache.len() as u64);
        stats.insert("max_entries".to_string(), self.max_entries as u64);
        
        let expired_count = cache.values().filter(|entry| entry.is_expired()).count();
        stats.insert("expired_entries".to_string(), expired_count as u64);
        
        let total_hits: usize = cache.values().map(|entry| entry.hit_count).sum();
        stats.insert("total_hits".to_string(), total_hits as u64);
        
        stats
    }
}

/// Search engine implementation
pub struct SearchEngine {
    /// Vector database reference
    vector_db: Option<Arc<VectorDatabase>>,
    /// Search result cache
    cache: SearchResultCache,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self {
            vector_db: None,
            cache: SearchResultCache::new(1000, 300), // Cache 1000 entries for 5 minutes
        }
    }
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the vector database reference
    pub fn set_vector_database(&mut self, vector_db: Arc<VectorDatabase>) {
        self.vector_db = Some(vector_db);
    }
    
    /// Get vector database reference or return error
    fn get_vector_db(&self) -> SearchCommandResult<&VectorDatabase> {
        self.vector_db
            .as_ref()
            .map(|db| db.as_ref())
            .ok_or(SearchCommandError::VectorDbError {
                message: "Vector database not initialized".to_string(),
            })
    }
    
    /// Validate search configuration
    fn validate_config(&self, config: &SimilaritySearchConfig) -> SearchCommandResult<()> {
        if config.min_similarity < 0.0 || config.min_similarity > 1.0 {
            return Err(SearchCommandError::InvalidParameters {
                reason: format!("min_similarity must be between 0.0 and 1.0, got {}", config.min_similarity),
            });
        }
        
        if config.max_results == 0 || config.max_results > 50 {
            return Err(SearchCommandError::ResultLimitExceeded {
                requested: config.max_results,
                max_allowed: 50,
            });
        }
        
        if config.timeout_ms < 100 || config.timeout_ms > 30000 {
            return Err(SearchCommandError::InvalidParameters {
                reason: format!("timeout_ms must be between 100 and 30000, got {}", config.timeout_ms),
            });
        }
        
        Ok(())
    }
    
    /// Perform similarity search with caching
    pub async fn search_similar_notes(
        &self,
        query_vector: Vec<f32>,
        config: SimilaritySearchConfig,
    ) -> SearchCommandResult<Vec<SimilaritySearchResult>> {
        let start_time = Instant::now();
        
        // Validate configuration
        self.validate_config(&config)?;
        
        // Check cache first if enabled
        if config.enable_caching {
            if let Some(cached_results) = self.cache.get(&query_vector, &config).await {
                return Ok(cached_results);
            }
        }
        
        // Get vector database
        let vector_db = self.get_vector_db()?;
        
        // Get all embeddings from database
        let all_embedding_ids = vector_db.list_embedding_ids().await;
        
        if all_embedding_ids.is_empty() {
            return Err(SearchCommandError::EmptyDatabase);
        }
        
        // Retrieve all embeddings
        let all_embeddings = vector_db
            .retrieve_embeddings(&all_embedding_ids)
            .await
            .map_err(SearchCommandError::from)?;
        
        // Filter out excluded file if specified
        let filtered_embeddings: Vec<EmbeddingEntry> = if let Some(ref exclude_path) = config.exclude_file_path {
            all_embeddings
                .into_iter()
                .filter(|entry| entry.metadata.file_path != *exclude_path)
                .collect()
        } else {
            all_embeddings
        };
        
        if filtered_embeddings.is_empty() {
            return Ok(vec![]);
        }
        
        // Convert search config to similarity search config
        let similarity_config = SearchConfig {
            min_threshold: config.min_similarity,
            max_results: config.max_results,
            early_termination: config.enable_early_termination,
            normalize_query: config.normalize_query,
            enable_diversity_filter: false,  // Disabled for basic search commands
            diversity_threshold: 0.95,
            enable_recency_weighting: false, // Disabled for basic search commands
            recency_weight: 0.0,
            exclude_current_file: None,
            exclude_recent_suggestions: Vec::new(),
        };
        
        // Perform similarity search with timeout
        let search_future = tokio::time::timeout(
            Duration::from_millis(config.timeout_ms),
            async {
                SimilaritySearch::k_nearest_neighbors(
                    &query_vector,
                    &filtered_embeddings,
                    config.max_results,
                    &similarity_config,
                )
            }
        );
        
        let search_results = match search_future.await {
            Ok(Ok(results)) => results,
            Ok(Err(similarity_error)) => {
                return Err(SearchCommandError::from(similarity_error));
            }
            Err(_) => {
                return Err(SearchCommandError::SearchTimeout {
                    timeout_ms: config.timeout_ms,
                });
            }
        };
        
        // Convert and rank results
        let mut final_results: Vec<SimilaritySearchResult> = search_results
            .into_iter()
            .enumerate()
            .map(|(index, search_result)| {
                let mut result = SimilaritySearchResult::from(search_result);
                result.relevance_rank = index + 1; // 1-based ranking
                result
            })
            .collect();
        
        // Additional ranking based on multiple factors (similarity score is primary)
        self.enhance_ranking(&mut final_results);
        
        // Cache results if caching is enabled
        if config.enable_caching {
            if let Err(cache_error) = self.cache.put(&query_vector, &config, final_results.clone()).await {
                // Log cache error but don't fail the search
                eprintln!("Cache error: {:?}", cache_error);
            }
        }
        
        let search_time = start_time.elapsed();
        
        // Log performance metrics
        if search_time.as_millis() > 100 {
            eprintln!(
                "Slow search: {}ms for {} embeddings, {} results",
                search_time.as_millis(),
                filtered_embeddings.len(),
                final_results.len()
            );
        }
        
        Ok(final_results)
    }
    
    /// Enhance ranking of search results using multiple factors
    fn enhance_ranking(&self, results: &mut [SimilaritySearchResult]) {
        // Primary: Sort by similarity score (descending)
        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Secondary: Boost more recent entries slightly
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        results.sort_by(|a, b| {
            let time_factor_a = 1.0 + (0.1 * (now.saturating_sub(a.created_at) as f32 / 86400.0).min(30.0)); // Age in days, capped at 30
            let time_factor_b = 1.0 + (0.1 * (now.saturating_sub(b.created_at) as f32 / 86400.0).min(30.0));
            
            let score_a = a.similarity_score * time_factor_a;
            let score_b = b.similarity_score * time_factor_b;
            
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Update relevance ranks after reordering
        for (index, result) in results.iter_mut().enumerate() {
            result.relevance_rank = index + 1;
        }
    }
    
    /// Perform batch similarity search
    pub async fn batch_search_similar_notes(
        &self,
        request: BatchSearchRequest,
    ) -> SearchCommandResult<BatchSearchResult> {
        let start_time = Instant::now();
        let mut batch_results = Vec::new();
        
        // Validate that query_labels length matches query_vectors if provided
        if let Some(ref labels) = request.query_labels {
            if labels.len() != request.query_vectors.len() {
                return Err(SearchCommandError::InvalidParameters {
                    reason: format!(
                        "query_labels length ({}) does not match query_vectors length ({})",
                        labels.len(),
                        request.query_vectors.len()
                    ),
                });
            }
        }
        
        // Process each query vector
        for query_vector in &request.query_vectors {
            let results = self
                .search_similar_notes(query_vector.clone(), request.config.clone())
                .await?;
            batch_results.push(results);
        }
        
        let total_time = start_time.elapsed();
        let total_queries = request.query_vectors.len();
        
        Ok(BatchSearchResult {
            results: batch_results,
            query_labels: request.query_labels,
            total_queries,
            total_search_time_ms: total_time.as_millis() as u64,
            avg_search_time_ms: (total_time.as_millis() / total_queries as u128) as u64,
        })
    }
    
    /// Clear search cache
    pub async fn clear_cache(&self) -> SearchCommandResult<()> {
        self.cache.clear().await;
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, u64> {
        self.cache.get_stats().await
    }
    
    /// Cleanup expired cache entries
    pub async fn cleanup_cache(&self) -> SearchCommandResult<usize> {
        let removed_count = self.cache.cleanup_expired().await;
        Ok(removed_count)
    }
}

// Global search engine instance
static SEARCH_ENGINE: once_cell::sync::Lazy<Arc<tokio::sync::RwLock<SearchEngine>>> =
    once_cell::sync::Lazy::new(|| Arc::new(tokio::sync::RwLock::new(SearchEngine::new())));

/// Initialize the search engine with vector database
pub async fn initialize_search_engine(vector_db: Arc<VectorDatabase>) -> Result<(), String> {
    let mut engine = SEARCH_ENGINE.write().await;
    engine.set_vector_database(vector_db);
    Ok(())
}

/// Get search engine statistics
pub async fn get_search_engine_stats() -> HashMap<String, u64> {
    let engine = SEARCH_ENGINE.read().await;
    engine.get_cache_stats().await
}

// ============================================================================
// TAURI COMMAND IMPLEMENTATIONS
// ============================================================================

/// Search for similar notes using cosine similarity
/// 
/// This is the main similarity search command that finds notes similar to a given
/// query vector. It supports configurable similarity thresholds, result limits,
/// caching, and various optimization options.
/// 
/// # Arguments
/// 
/// * `query_vector` - The embedding vector to find similarities for
/// * `config` - Search configuration including thresholds and limits
/// 
/// # Returns
/// 
/// Vector of similarity search results ranked by relevance
#[tauri::command]
pub async fn search_similar_notes(
    query_vector: Vec<f32>,
    config: Option<SimilaritySearchConfig>,
) -> Result<Vec<SimilaritySearchResult>, String> {
    let search_config = config.unwrap_or_default();
    
    let engine = SEARCH_ENGINE.read().await;
    
    engine
        .search_similar_notes(query_vector, search_config)
        .await
        .map_err(|e| e.to_string())
}

/// Perform batch similarity search for multiple query vectors
/// 
/// This command efficiently processes multiple search queries in a single request,
/// optimizing for performance when searching for similarities to multiple vectors.
/// 
/// # Arguments
/// 
/// * `request` - Batch search request containing multiple query vectors and config
/// 
/// # Returns
/// 
/// Batch search result with results for each query vector
#[tauri::command]
pub async fn batch_search_similar_notes(
    request: BatchSearchRequest,
) -> Result<BatchSearchResult, String> {
    let engine = SEARCH_ENGINE.read().await;
    
    engine
        .batch_search_similar_notes(request)
        .await
        .map_err(|e| e.to_string())
}

/// Configure similarity search thresholds and parameters
/// 
/// Creates a new search configuration with the specified parameters.
/// All parameters are optional and will use default values if not provided.
/// 
/// # Arguments
/// 
/// * `min_similarity` - Minimum similarity threshold (0.0 to 1.0)
/// * `max_results` - Maximum number of results (1 to 50)
/// * `enable_caching` - Enable result caching for performance
/// * `cache_ttl_seconds` - Cache time-to-live in seconds
/// * `timeout_ms` - Search timeout in milliseconds
/// * `enable_early_termination` - Enable early termination optimization
/// * `normalize_query` - Pre-normalize query vectors
/// * `exclude_file_path` - File path to exclude from results
/// 
/// # Returns
/// 
/// Configured search configuration object

#[derive(Debug, serde::Deserialize)]
pub struct SimilaritySearchParams {
    pub min_similarity: Option<f32>,
    pub max_results: Option<usize>,
    pub enable_caching: Option<bool>,
    pub cache_ttl_seconds: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub enable_early_termination: Option<bool>,
    pub normalize_query: Option<bool>,
    pub exclude_file_path: Option<String>,
}

#[tauri::command]
pub async fn configure_similarity_search(params: SimilaritySearchParams) -> Result<SimilaritySearchConfig, String> {
    let default_config = SimilaritySearchConfig::default();
    
    let config = SimilaritySearchConfig {
        min_similarity: params.min_similarity.unwrap_or(default_config.min_similarity),
        max_results: params.max_results.unwrap_or(default_config.max_results),
        enable_caching: params.enable_caching.unwrap_or(default_config.enable_caching),
        cache_ttl_seconds: params.cache_ttl_seconds.unwrap_or(default_config.cache_ttl_seconds),
        timeout_ms: params.timeout_ms.unwrap_or(default_config.timeout_ms),
        enable_early_termination: params.enable_early_termination.unwrap_or(default_config.enable_early_termination),
        normalize_query: params.normalize_query.unwrap_or(default_config.normalize_query),
        exclude_file_path: params.exclude_file_path.or(default_config.exclude_file_path),
    };
    
    // Validate the configuration
    let engine = SEARCH_ENGINE.read().await;
    engine.validate_config(&config).map_err(|e| e.to_string())?;
    
    Ok(config)
}

/// Perform threshold-based similarity search
/// 
/// This command finds all notes with similarity above a specified threshold,
/// without limiting the number of results (unlike k-NN search).
/// 
/// # Arguments
/// 
/// * `query_vector` - The embedding vector to find similarities for
/// * `threshold` - Minimum similarity threshold (0.0 to 1.0)
/// * `max_results` - Optional maximum number of results
/// * `exclude_file_path` - Optional file path to exclude from results
/// 
/// # Returns
/// 
/// All notes with similarity above the threshold, sorted by relevance
#[tauri::command]
pub async fn threshold_search_similar_notes(
    query_vector: Vec<f32>,
    threshold: f32,
    max_results: Option<usize>,
    exclude_file_path: Option<String>,
) -> Result<Vec<SimilaritySearchResult>, String> {
    let config = SimilaritySearchConfig {
        min_similarity: threshold,
        max_results: max_results.unwrap_or(50), // Use max allowed if not specified
        enable_caching: true,
        cache_ttl_seconds: 300,
        timeout_ms: 5000,
        enable_early_termination: false, // Don't terminate early for threshold search
        normalize_query: true,
        exclude_file_path,
    };
    
    let engine = SEARCH_ENGINE.read().await;
    
    engine
        .search_similar_notes(query_vector, config)
        .await
        .map_err(|e| e.to_string())
}

/// Get search engine cache statistics
/// 
/// Returns detailed statistics about the search result cache including
/// entry counts, hit rates, memory usage, and performance metrics.
/// 
/// # Returns
/// 
/// HashMap containing cache statistics
#[tauri::command]
pub async fn get_search_cache_stats() -> Result<HashMap<String, u64>, String> {
    Ok(get_search_engine_stats().await)
}

/// Clear the search result cache
/// 
/// Removes all cached search results to free memory or force fresh searches.
/// This is useful for debugging or after database updates.
/// 
/// # Returns
/// 
/// Success confirmation
#[tauri::command]
pub async fn clear_search_cache() -> Result<(), String> {
    let engine = SEARCH_ENGINE.read().await;
    
    engine
        .clear_cache()
        .await
        .map_err(|e| e.to_string())
}

/// Cleanup expired cache entries
/// 
/// Removes expired entries from the search result cache to free memory
/// and maintain optimal performance.
/// 
/// # Returns
/// 
/// Number of expired entries that were removed
#[tauri::command]
pub async fn cleanup_search_cache() -> Result<usize, String> {
    let engine = SEARCH_ENGINE.read().await;
    
    engine
        .cleanup_cache()
        .await
        .map_err(|e| e.to_string())
}

/// Initialize the search engine with vector database
/// 
/// This command sets up the search engine with a reference to the vector database.
/// It must be called before performing any search operations.
/// 
/// # Arguments
/// 
/// * `storage_dir` - Directory path for the vector database storage
/// 
/// # Returns
/// 
/// Success confirmation
#[tauri::command]
pub async fn initialize_search_system(storage_dir: String) -> Result<(), String> {
    use crate::vector_db::VectorDatabase;
    use crate::vector_db::types::VectorStorageConfig;
    
    // Create vector database configuration
    let config = VectorStorageConfig {
        storage_dir,
        ..VectorStorageConfig::default()
    };
    
    // Create vector database instance
    let vector_db = Arc::new(
        VectorDatabase::new(config)
            .await
            .map_err(|e| format!("Failed to create vector database: {}", e))?
    );
    
    // Initialize the vector database
    vector_db
        .initialize()
        .await
        .map_err(|e| format!("Failed to initialize vector database: {}", e))?;
    
    // Set up the search engine
    initialize_search_engine(vector_db)
        .await
        .map_err(|e| format!("Failed to initialize search engine: {}", e))?;
    
    Ok(())
}

/// Get comprehensive search system status
/// 
/// Returns detailed information about the search system including database
/// status, cache statistics, and performance metrics.
/// 
/// # Returns
/// 
/// HashMap containing comprehensive system status information
#[tauri::command]
pub async fn get_search_system_status() -> Result<HashMap<String, serde_json::Value>, String> {
    let mut status = HashMap::new();
    
    // Get cache statistics
    let cache_stats = get_search_engine_stats().await;
    status.insert("cache_stats".to_string(), serde_json::to_value(cache_stats).unwrap());
    
    // Check if search engine is initialized
    let engine = SEARCH_ENGINE.read().await;
    let is_initialized = engine.vector_db.is_some();
    status.insert("search_engine_initialized".to_string(), serde_json::Value::Bool(is_initialized));
    
    // Get vector database metrics if available
    if let Ok(vector_db) = engine.get_vector_db() {
        let embedding_count = vector_db.count_embeddings().await;
        status.insert("total_embeddings".to_string(), serde_json::Value::Number(serde_json::Number::from(embedding_count)));
        
        let is_empty = vector_db.is_empty().await;
        status.insert("database_empty".to_string(), serde_json::Value::Bool(is_empty));
        
        if let Ok(metrics) = vector_db.get_metrics().await {
            status.insert("database_metrics".to_string(), serde_json::to_value(metrics.summary()).unwrap());
        }
    } else {
        status.insert("total_embeddings".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
        status.insert("database_empty".to_string(), serde_json::Value::Bool(true));
        status.insert("database_metrics".to_string(), serde_json::Value::String("Database not initialized".to_string()));
    }
    
    // Add system timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    status.insert("status_timestamp".to_string(), serde_json::Value::Number(serde_json::Number::from(timestamp)));
    
    Ok(status)
}