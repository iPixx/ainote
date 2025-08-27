//! Monitored Similarity Search Integration
//!
//! This module provides performance-monitored wrappers for similarity search operations,
//! integrating with the enhanced metrics collector to track search performance, accuracy,
//! and efficiency metrics as required by issue #146.

use std::sync::Arc;
use std::time::Instant;
use chrono::Utc;

use crate::similarity_search::{SimilaritySearch, SearchConfig, SimilarityResult};
use crate::vector_db::types::EmbeddingEntry;
use crate::vector_db::metrics_collector::{
    EnhancedMetricsCollector, SearchOperationMetrics, SearchOperationType
};

/// Performance-monitored similarity search wrapper
pub struct MonitoredSimilaritySearch {
    /// Reference to metrics collector
    metrics_collector: Option<Arc<EnhancedMetricsCollector>>,
}

impl MonitoredSimilaritySearch {
    /// Create a new monitored search instance
    pub fn new(metrics_collector: Option<Arc<EnhancedMetricsCollector>>) -> Self {
        Self {
            metrics_collector,
        }
    }

    /// Perform monitored k-nearest neighbors search
    pub async fn search_k_nearest_neighbors(
        &self,
        query_vector: &[f32],
        database_entries: &[EmbeddingEntry],
        k: usize,
        config: &SearchConfig,
    ) -> SimilarityResult<Vec<SearchResult>> {
        let operation_id = format!("knn_search_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let start_time = Instant::now();
        let start_memory = self.get_current_memory_usage().await;

        // Start monitoring if collector is available
        if let Some(collector) = &self.metrics_collector {
            let initial_metrics = SearchOperationMetrics {
                operation_id: operation_id.clone(),
                operation_type: SearchOperationType::KNearestNeighbors,
                started_at: Utc::now(),
                completed_at: None,
                duration_ms: None,
                query_dimension: query_vector.len(),
                vectors_searched: database_entries.len(),
                results_returned: 0,
                similarity_threshold: config.min_threshold,
                top_similarity_score: None,
                avg_similarity_score: None,
                memory_usage_mb: start_memory,
                cpu_usage_percent: 0.0, // Would need actual CPU monitoring
                efficiency_score: 0.0,
                performance_target_met: false,
                error_message: None,
            };

            let _ = collector.record_search_operation(initial_metrics).await;
        }

        // Perform the actual search
        let search_result = Self::perform_k_nearest_neighbors_search(
            query_vector,
            database_entries,
            k,
            config,
        ).await;

        // Record completion metrics
        let end_time = Instant::now();
        let duration_ms = end_time.duration_since(start_time).as_secs_f64() * 1000.0;
        let end_memory = self.get_current_memory_usage().await;

        if let Some(collector) = &self.metrics_collector {
            let (results_count, top_score, avg_score, efficiency, target_met, error_msg) = match &search_result {
                Ok(results) => {
                    let count = results.len();
                    let top_score = results.first().map(|r| r.similarity);
                    let avg_score = if !results.is_empty() {
                        Some(results.iter().map(|r| r.similarity).sum::<f32>() / results.len() as f32)
                    } else {
                        None
                    };
                    let efficiency = if database_entries.len() > 0 {
                        count as f64 / database_entries.len() as f64
                    } else {
                        1.0
                    };
                    let target_met = duration_ms < 50.0; // Target: <50ms for search operations
                    (count, top_score, avg_score, efficiency, target_met, None)
                },
                Err(e) => {
                    (0, None, None, 0.0, false, Some(e.to_string()))
                }
            };

            let completion_metrics = SearchOperationMetrics {
                operation_id: operation_id.clone(),
                operation_type: SearchOperationType::KNearestNeighbors,
                started_at: Utc::now() - chrono::Duration::milliseconds(duration_ms as i64),
                completed_at: Some(Utc::now()),
                duration_ms: Some(duration_ms),
                query_dimension: query_vector.len(),
                vectors_searched: database_entries.len(),
                results_returned: results_count,
                similarity_threshold: config.min_threshold,
                top_similarity_score: top_score,
                avg_similarity_score: avg_score,
                memory_usage_mb: end_memory,
                cpu_usage_percent: 0.0, // Would need actual CPU monitoring
                efficiency_score: efficiency,
                performance_target_met: target_met,
                error_message: error_msg,
            };

            let _ = collector.record_search_operation(completion_metrics).await;
        }

        search_result
    }

    /// Perform monitored similarity threshold search
    pub async fn search_by_similarity_threshold(
        &self,
        query_vector: &[f32],
        database_entries: &[EmbeddingEntry],
        threshold: f32,
        config: &SearchConfig,
    ) -> SimilarityResult<Vec<SearchResult>> {
        let operation_id = format!("threshold_search_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let start_time = Instant::now();
        let start_memory = self.get_current_memory_usage().await;

        // Start monitoring
        if let Some(collector) = &self.metrics_collector {
            let initial_metrics = SearchOperationMetrics {
                operation_id: operation_id.clone(),
                operation_type: SearchOperationType::SimilarityThreshold,
                started_at: Utc::now(),
                completed_at: None,
                duration_ms: None,
                query_dimension: query_vector.len(),
                vectors_searched: database_entries.len(),
                results_returned: 0,
                similarity_threshold: threshold,
                top_similarity_score: None,
                avg_similarity_score: None,
                memory_usage_mb: start_memory,
                cpu_usage_percent: 0.0,
                efficiency_score: 0.0,
                performance_target_met: false,
                error_message: None,
            };

            let _ = collector.record_search_operation(initial_metrics).await;
        }

        // Perform the search
        let search_result = Self::perform_similarity_threshold_search(
            query_vector,
            database_entries,
            threshold,
            config,
        ).await;

        // Record completion metrics
        let end_time = Instant::now();
        let duration_ms = end_time.duration_since(start_time).as_secs_f64() * 1000.0;
        let end_memory = self.get_current_memory_usage().await;

        if let Some(collector) = &self.metrics_collector {
            let (results_count, top_score, avg_score, efficiency, target_met, error_msg) = match &search_result {
                Ok(results) => {
                    let count = results.len();
                    let top_score = results.first().map(|r| r.similarity);
                    let avg_score = if !results.is_empty() {
                        Some(results.iter().map(|r| r.similarity).sum::<f32>() / results.len() as f32)
                    } else {
                        None
                    };
                    let efficiency = if database_entries.len() > 0 {
                        count as f64 / database_entries.len() as f64
                    } else {
                        1.0
                    };
                    let target_met = duration_ms < 50.0;
                    (count, top_score, avg_score, efficiency, target_met, None)
                },
                Err(e) => {
                    (0, None, None, 0.0, false, Some(e.to_string()))
                }
            };

            let completion_metrics = SearchOperationMetrics {
                operation_id,
                operation_type: SearchOperationType::SimilarityThreshold,
                started_at: Utc::now() - chrono::Duration::milliseconds(duration_ms as i64),
                completed_at: Some(Utc::now()),
                duration_ms: Some(duration_ms),
                query_dimension: query_vector.len(),
                vectors_searched: database_entries.len(),
                results_returned: results_count,
                similarity_threshold: threshold,
                top_similarity_score: top_score,
                avg_similarity_score: avg_score,
                memory_usage_mb: end_memory,
                cpu_usage_percent: 0.0,
                efficiency_score: efficiency,
                performance_target_met: target_met,
                error_message: error_msg,
            };

            let _ = collector.record_search_operation(completion_metrics).await;
        }

        search_result
    }

    /// Perform monitored batch search operations
    pub async fn batch_search(
        &self,
        queries: &[Vec<f32>],
        database_entries: &[EmbeddingEntry],
        k: usize,
        config: &SearchConfig,
    ) -> Vec<SimilarityResult<Vec<SearchResult>>> {
        let operation_id = format!("batch_search_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let start_time = Instant::now();
        let start_memory = self.get_current_memory_usage().await;

        // Start monitoring
        if let Some(collector) = &self.metrics_collector {
            let initial_metrics = SearchOperationMetrics {
                operation_id: operation_id.clone(),
                operation_type: SearchOperationType::BatchSearch,
                started_at: Utc::now(),
                completed_at: None,
                duration_ms: None,
                query_dimension: queries.first().map(|q| q.len()).unwrap_or(0),
                vectors_searched: database_entries.len() * queries.len(),
                results_returned: 0,
                similarity_threshold: config.min_threshold,
                top_similarity_score: None,
                avg_similarity_score: None,
                memory_usage_mb: start_memory,
                cpu_usage_percent: 0.0,
                efficiency_score: 0.0,
                performance_target_met: false,
                error_message: None,
            };

            let _ = collector.record_search_operation(initial_metrics).await;
        }

        // Perform batch search
        let mut results = Vec::new();
        for query in queries {
            let result = Self::perform_k_nearest_neighbors_search(
                query,
                database_entries,
                k,
                config,
            ).await;
            results.push(result);
        }

        // Record completion metrics
        let end_time = Instant::now();
        let duration_ms = end_time.duration_since(start_time).as_secs_f64() * 1000.0;
        let end_memory = self.get_current_memory_usage().await;

        if let Some(collector) = &self.metrics_collector {
            let total_results: usize = results.iter()
                .map(|r| r.as_ref().map(|res| res.len()).unwrap_or(0))
                .sum();
            
            let all_similarities: Vec<f32> = results.iter()
                .filter_map(|r| r.as_ref().ok())
                .flat_map(|res| res.iter().map(|sr| sr.similarity))
                .collect();
            
            let top_score = all_similarities.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied();
            let avg_score = if !all_similarities.is_empty() {
                Some(all_similarities.iter().sum::<f32>() / all_similarities.len() as f32)
            } else {
                None
            };

            let efficiency = if !queries.is_empty() && !database_entries.is_empty() {
                total_results as f64 / (queries.len() * database_entries.len()) as f64
            } else {
                1.0
            };

            let target_met = duration_ms < (50.0 * queries.len() as f64); // Scale target with query count
            let success_count = results.iter().filter(|r| r.is_ok()).count();
            let error_msg = if success_count < results.len() {
                Some(format!("{} of {} batch operations failed", results.len() - success_count, results.len()))
            } else {
                None
            };

            let completion_metrics = SearchOperationMetrics {
                operation_id,
                operation_type: SearchOperationType::BatchSearch,
                started_at: Utc::now() - chrono::Duration::milliseconds(duration_ms as i64),
                completed_at: Some(Utc::now()),
                duration_ms: Some(duration_ms),
                query_dimension: queries.first().map(|q| q.len()).unwrap_or(0),
                vectors_searched: database_entries.len() * queries.len(),
                results_returned: total_results,
                similarity_threshold: config.min_threshold,
                top_similarity_score: top_score,
                avg_similarity_score: avg_score,
                memory_usage_mb: end_memory,
                cpu_usage_percent: 0.0,
                efficiency_score: efficiency,
                performance_target_met: target_met,
                error_message: error_msg,
            };

            let _ = collector.record_search_operation(completion_metrics).await;
        }

        results
    }

    // Private helper methods

    async fn perform_k_nearest_neighbors_search(
        query_vector: &[f32],
        database_entries: &[EmbeddingEntry],
        k: usize,
        config: &SearchConfig,
    ) -> SimilarityResult<Vec<SearchResult>> {
        // Perform k-NN search using the existing similarity search implementation
        let results = SimilaritySearch::k_nearest_neighbors(
            query_vector,
            database_entries,
            k,
            config,
        )?;

        // Map results back to include entry IDs
        let enhanced_results: Vec<SearchResult> = results.into_iter()
            .enumerate()
            .map(|(index, result)| {
                SearchResult {
                    index,
                    similarity: result.similarity,
                    entry_id: Some(result.entry.id.clone()),
                    metadata: Some(result.entry.metadata.clone()),
                }
            })
            .collect();

        Ok(enhanced_results)
    }

    async fn perform_similarity_threshold_search(
        query_vector: &[f32],
        database_entries: &[EmbeddingEntry],
        threshold: f32,
        config: &SearchConfig,
    ) -> SimilarityResult<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Iterate through all entries and collect those above threshold
        for (index, entry) in database_entries.iter().enumerate() {
            let similarity = SimilaritySearch::cosine_similarity(
                query_vector,
                &entry.vector,
            )?;

            if similarity >= threshold && similarity >= config.min_threshold {
                results.push(SearchResult {
                    index,
                    similarity,
                    entry_id: Some(entry.id.clone()),
                    metadata: Some(entry.metadata.clone()),
                });
            }

            // Apply max_results limit if configured
            if config.max_results > 0 && results.len() >= config.max_results {
                break;
            }
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        Ok(results)
    }

    async fn get_current_memory_usage(&self) -> f64 {
        // Simplified memory usage calculation
        // In production, would use system APIs to get actual memory usage
        100.0 // MB
    }
}

/// Enhanced search result with entry metadata
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Index in the original database
    pub index: usize,
    /// Similarity score
    pub similarity: f32,
    /// Entry ID (if available)
    pub entry_id: Option<String>,
    /// Entry metadata (if available)
    pub metadata: Option<crate::vector_db::types::EmbeddingMetadata>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(index: usize, similarity: f32) -> Self {
        Self {
            index,
            similarity,
            entry_id: None,
            metadata: None,
        }
    }

    /// Create a search result with entry information
    pub fn with_entry(index: usize, similarity: f32, entry_id: String, metadata: crate::vector_db::types::EmbeddingMetadata) -> Self {
        Self {
            index,
            similarity,
            entry_id: Some(entry_id),
            metadata: Some(metadata),
        }
    }
}