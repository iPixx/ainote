//! Tauri Commands for Similarity Search
//! 
//! This module provides Tauri command interface for similarity search operations,
//! including parallel processing, concurrent request handling, and performance
//! monitoring for the aiNote application.

use crate::similarity_search::{
    SimilaritySearch, SearchConfig, PerformanceConfig, EnhancedSearchResult,
    ConcurrentSearchManager, GlobalSearchMetrics, BenchmarkReport,
};
use crate::vector_db::types::EmbeddingEntry;
use std::sync::Arc;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

// Global concurrent search manager
static SEARCH_MANAGER: Lazy<ConcurrentSearchManager> = Lazy::new(|| {
    ConcurrentSearchManager::new(PerformanceConfig::default())
});

/// Search request structure for Tauri commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Query vector for similarity search
    pub query_vector: Vec<f32>,
    /// Number of nearest neighbors to return
    pub k: usize,
    /// Search configuration
    pub config: Option<SearchConfig>,
    /// Performance configuration
    pub perf_config: Option<PerformanceConfig>,
}

/// Batch search request for multiple queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSearchRequest {
    /// Multiple query vectors
    pub query_vectors: Vec<Vec<f32>>,
    /// Number of nearest neighbors per query
    pub k: usize,
    /// Search configuration
    pub config: Option<SearchConfig>,
    /// Performance configuration
    pub perf_config: Option<PerformanceConfig>,
}

/// Search response with results and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results
    pub results: Vec<SearchResultJson>,
    /// Performance metrics
    pub metrics: SearchMetricsJson,
}

/// JSON-serializable search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultJson {
    /// File path of the result
    pub file_path: String,
    /// Chunk ID within the file
    pub chunk_id: String,
    /// Content preview
    pub content_preview: String,
    /// Model used for embedding
    pub model: String,
    /// Similarity score
    pub similarity: f32,
    /// Vector dimension
    pub vector_dimension: usize,
}

/// JSON-serializable search metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetricsJson {
    /// Total search time in milliseconds
    pub total_time_ms: f64,
    /// Number of vectors processed
    pub vectors_processed: usize,
    /// Number of results returned
    pub results_count: usize,
    /// Whether parallel processing was used
    pub used_parallel_processing: bool,
    /// Whether approximate search was used
    pub used_approximate_search: bool,
    /// Memory usage in bytes (estimated)
    pub estimated_memory_bytes: usize,
    /// Throughput in vectors per second
    pub vectors_per_second: f64,
}

// Conversion functions
impl From<&EnhancedSearchResult> for SearchResponse {
    fn from(enhanced_result: &EnhancedSearchResult) -> Self {
        let results = enhanced_result.results.iter()
            .map(SearchResultJson::from)
            .collect();
        
        SearchResponse {
            results,
            metrics: SearchMetricsJson::from(&enhanced_result.metrics),
        }
    }
}

impl From<&crate::similarity_search::SearchResult> for SearchResultJson {
    fn from(result: &crate::similarity_search::SearchResult) -> Self {
        SearchResultJson {
            file_path: result.entry.metadata.file_path.clone(),
            chunk_id: result.entry.metadata.chunk_id.clone(),
            content_preview: result.entry.metadata.content_preview.chars().take(200).collect(),
            model: result.entry.metadata.model_name.clone(),
            similarity: result.similarity,
            vector_dimension: result.entry.vector.len(),
        }
    }
}

impl From<&crate::similarity_search::SearchMetrics> for SearchMetricsJson {
    fn from(metrics: &crate::similarity_search::SearchMetrics) -> Self {
        SearchMetricsJson {
            total_time_ms: metrics.total_time_ms,
            vectors_processed: metrics.vectors_processed,
            results_count: metrics.results_count,
            used_parallel_processing: metrics.used_parallel_processing,
            used_approximate_search: metrics.used_approximate_search,
            estimated_memory_bytes: metrics.estimated_memory_bytes,
            vectors_per_second: metrics.vectors_per_second,
        }
    }
}

/// Execute optimized similarity search with automatic algorithm selection
#[tauri::command]
pub async fn optimized_search_similar_notes(
    request: SearchRequest,
    database_entries: Vec<EmbeddingEntry>,
) -> Result<SearchResponse, String> {
    let config = request.config.unwrap_or_default();
    let perf_config = request.perf_config.unwrap_or_default();
    
    // Use the global concurrent search manager
    let result = SEARCH_MANAGER.execute_search(move || {
        SimilaritySearch::parallel_k_nearest_neighbors(
            &request.query_vector,
            &database_entries,
            request.k,
            &config,
            &perf_config,
        )
    }).await
    .map_err(|e| format!("Search failed: {}", e))?;
    
    Ok(SearchResponse::from(&result))
}

/// Execute optimized batch similarity search with concurrency
#[tauri::command]
pub async fn optimized_batch_search_similar_notes(
    request: BatchSearchRequest,
    database_entries: Vec<EmbeddingEntry>,
) -> Result<Vec<SearchResponse>, String> {
    let config = request.config.unwrap_or_default();
    let database_arc = Arc::new(database_entries);
    
    // Use the global concurrent search manager for batch processing
    let results = SEARCH_MANAGER.execute_batch_search(
        request.query_vectors,
        database_arc,
        request.k,
        config,
    ).await
    .map_err(|e| format!("Batch search failed: {}", e))?;
    
    Ok(results.iter().map(SearchResponse::from).collect())
}

/// Execute approximate nearest neighbors search for large datasets
#[tauri::command]
pub async fn approximate_search_similar_notes(
    request: SearchRequest,
    database_entries: Vec<EmbeddingEntry>,
) -> Result<SearchResponse, String> {
    let config = request.config.unwrap_or_default();
    let perf_config = request.perf_config.unwrap_or_default();
    
    let result = SEARCH_MANAGER.execute_search(move || {
        SimilaritySearch::approximate_nearest_neighbors(
            &request.query_vector,
            &database_entries,
            request.k,
            &config,
            &perf_config,
        )
    }).await
    .map_err(|e| format!("Approximate search failed: {}", e))?;
    
    Ok(SearchResponse::from(&result))
}

/// Get current search performance metrics
#[tauri::command]
pub async fn get_search_metrics() -> Result<GlobalSearchMetrics, String> {
    Ok(SEARCH_MANAGER.get_metrics().await)
}

/// Check if the search system is under high load
#[tauri::command]
pub async fn is_search_high_load() -> Result<bool, String> {
    Ok(SEARCH_MANAGER.is_high_load().await)
}

/// Get current number of active search requests
#[tauri::command]
pub async fn get_active_search_count() -> Result<usize, String> {
    Ok(SEARCH_MANAGER.get_active_request_count())
}

/// Run comprehensive search performance benchmarks
#[tauri::command]
pub async fn benchmark_search_performance(
    test_queries: Vec<Vec<f32>>,
    database_entries: Vec<EmbeddingEntry>,
    k_values: Vec<usize>,
) -> Result<BenchmarkReport, String> {
    let result = tokio::task::spawn_blocking(move || {
        SimilaritySearch::benchmark_search_performance(
            &test_queries,
            &database_entries,
            &k_values,
        )
    }).await
    .map_err(|e| format!("Benchmark task failed: {}", e))?
    .map_err(|e| format!("Benchmark failed: {}", e))?;
    
    Ok(result)
}

/// Configure search performance settings
#[tauri::command]
pub async fn configure_search_performance(
    _config: PerformanceConfig,
) -> Result<(), String> {
    // Note: This would require updating the global manager
    // For now, return success - in a full implementation,
    // you might want to make the manager reconfigurable
    Ok(())
}

/// Test search functionality with sample data
#[tauri::command]
pub async fn test_search_functionality() -> Result<SearchResponse, String> {
    // Create sample data for testing
    let query = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let entries = vec![
        EmbeddingEntry::new(
            vec![0.1, 0.2, 0.3, 0.4, 0.5],
            "test1.md".to_string(),
            "chunk1".to_string(),
            "This is a test document for similarity search.",
            "test-model".to_string(),
        ),
        EmbeddingEntry::new(
            vec![0.9, 0.8, 0.7, 0.6, 0.5],
            "test2.md".to_string(),
            "chunk1".to_string(),
            "This is another test document with different content.",
            "test-model".to_string(),
        ),
    ];
    
    let request = SearchRequest {
        query_vector: query,
        k: 2,
        config: Some(SearchConfig::default()),
        perf_config: Some(PerformanceConfig::default()),
    };
    
    optimized_search_similar_notes(request, entries).await
}