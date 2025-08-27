//! Vector Database Deduplication Module
//!
//! This module implements duplicate embedding detection and deduplication algorithms
//! for the aiNote vector database. It provides similarity-based duplicate detection,
//! embedding merging with reference tracking, and configurable deduplication parameters.
//!
//! ## Features
//!
//! - **Cosine Similarity-Based Detection**: Uses >95% threshold by default
//! - **Reference Tracking**: Maintains backward compatibility with existing embeddings
//! - **Batch Processing**: Efficient processing for large embedding sets
//! - **Configurable Thresholds**: Flexible similarity thresholds for different use cases
//! - **Performance Optimized**: <10 seconds per 1000 embeddings target
//!
//! ## Algorithm Overview
//!
//! The deduplication process works as follows:
//!
//! 1. **Similarity Detection**: Compare all embedding pairs using cosine similarity
//! 2. **Clustering**: Group similar embeddings above the threshold
//! 3. **Representative Selection**: Choose the best representative from each cluster
//! 4. **Reference Mapping**: Track which embeddings were merged into representatives
//! 5. **Index Update**: Update database with deduplicated embeddings and references
//!
//! ## Performance Characteristics
//!
//! - **Time Complexity**: O(n²) for similarity comparison, O(n log n) for clustering
//! - **Space Complexity**: O(n) for reference tracking
//! - **Target Performance**: <10 seconds per 1000 embeddings
//! - **Memory Efficiency**: Streaming processing for large datasets

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::vector_db::types::{EmbeddingEntry, VectorDbError};
use crate::similarity_search::{SimilaritySearch, SimilarityError};

/// Errors specific to deduplication operations
#[derive(Error, Debug, Clone)]
pub enum DeduplicationError {
    #[error("Invalid similarity threshold: {threshold} (must be between 0.0 and 1.0)")]
    InvalidThreshold { threshold: f32 },

    #[error("Empty embedding set provided")]
    EmptyEmbeddingSet,

    #[error("Similarity calculation failed: {source}")]
    SimilarityCalculation { source: SimilarityError },

    #[error("Reference tracking error: {message}")]
    ReferenceTracking { message: String },

    #[error("Batch processing error: {message}")]
    BatchProcessing { message: String },
}

impl From<SimilarityError> for DeduplicationError {
    fn from(error: SimilarityError) -> Self {
        DeduplicationError::SimilarityCalculation { source: error }
    }
}

impl From<DeduplicationError> for VectorDbError {
    fn from(error: DeduplicationError) -> Self {
        VectorDbError::Storage { 
            message: error.to_string() 
        }
    }
}

pub type DeduplicationResult<T> = Result<T, DeduplicationError>;

/// Configuration for deduplication operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    /// Similarity threshold for considering embeddings as duplicates
    /// Default: 0.95 (95% similarity)
    pub similarity_threshold: f32,
    
    /// Minimum similarity threshold for any deduplication
    /// Default: 0.80 (80% similarity) - below this, never consider as duplicates
    pub min_similarity_threshold: f32,
    
    /// Maximum batch size for processing embeddings
    /// Default: 1000 - process in batches to manage memory usage
    pub batch_size: usize,
    
    /// Enable parallel processing for large datasets
    /// Default: true
    pub enable_parallel_processing: bool,
    
    /// Threshold for enabling parallel processing
    /// Default: 500 embeddings
    pub parallel_threshold: usize,
    
    /// Strategy for selecting representative embeddings from duplicates
    pub representative_selection_strategy: RepresentativeSelectionStrategy,
    
    /// Enable detailed logging and metrics collection
    /// Default: true
    pub enable_detailed_metrics: bool,
    
    /// Maximum number of references to track per embedding
    /// Default: 100 - prevent memory issues with highly duplicated content
    pub max_references_per_embedding: usize,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.95,
            min_similarity_threshold: 0.80,
            batch_size: 1000,
            enable_parallel_processing: true,
            parallel_threshold: 500,
            representative_selection_strategy: RepresentativeSelectionStrategy::MostRecent,
            enable_detailed_metrics: true,
            max_references_per_embedding: 100,
        }
    }
}

/// Strategy for selecting representative embeddings from duplicate clusters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepresentativeSelectionStrategy {
    /// Select the most recently created embedding
    MostRecent,
    /// Select the earliest created embedding
    Earliest,
    /// Select the embedding with the highest average similarity to all others
    HighestAverageSimilarity,
    /// Select the embedding from the file with the most content
    LongestContent,
}

/// Represents a cluster of duplicate embeddings
#[derive(Debug, Clone)]
pub struct DuplicateCluster {
    /// Representative embedding for this cluster
    pub representative: EmbeddingEntry,
    /// All embeddings in this cluster (including the representative)
    pub members: Vec<EmbeddingEntry>,
    /// Average similarity within the cluster
    pub average_similarity: f32,
    /// Creation timestamp of the cluster
    pub created_at: u64,
}

/// Tracks references from original embeddings to their representatives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceMapping {
    /// Original embedding ID -> Representative embedding ID
    pub mapping: HashMap<String, String>,
    /// Reverse mapping: Representative ID -> Set of original IDs
    pub reverse_mapping: HashMap<String, HashSet<String>>,
    /// Timestamp when the mapping was created
    pub created_at: u64,
    /// Statistics about the mapping
    pub stats: ReferenceMappingStats,
}

/// Statistics about reference mappings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReferenceMappingStats {
    /// Total number of original embeddings
    pub total_original_embeddings: usize,
    /// Total number of representative embeddings
    pub total_representatives: usize,
    /// Total number of duplicates found
    pub total_duplicates_found: usize,
    /// Reduction ratio (duplicates / total)
    pub reduction_ratio: f32,
}

/// Results from a deduplication operation
#[derive(Debug, Clone)]
pub struct DeduplicationResult_ {
    /// Clusters of duplicate embeddings found
    pub clusters: Vec<DuplicateCluster>,
    /// Reference mapping for backward compatibility
    pub reference_mapping: ReferenceMapping,
    /// Performance and operation metrics
    pub metrics: DeduplicationMetrics,
    /// Final deduplicated set of embeddings
    pub deduplicated_embeddings: Vec<EmbeddingEntry>,
}

/// Comprehensive metrics for deduplication operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationMetrics {
    /// Total processing time in milliseconds
    pub total_time_ms: f64,
    /// Number of embeddings processed
    pub embeddings_processed: usize,
    /// Number of similarity comparisons performed
    pub similarity_comparisons: usize,
    /// Number of duplicate clusters found
    pub clusters_found: usize,
    /// Number of embeddings marked as duplicates
    pub duplicates_found: usize,
    /// Processing throughput (embeddings per second)
    pub throughput_embeddings_per_second: f64,
    /// Average similarity comparison time in microseconds
    pub avg_similarity_comparison_time_us: f64,
    /// Memory usage estimation in bytes
    pub estimated_memory_usage_bytes: usize,
    /// Index size reduction percentage
    pub index_size_reduction_percentage: f32,
    /// Whether parallel processing was used
    pub used_parallel_processing: bool,
    /// Batch processing statistics
    pub batch_stats: BatchProcessingStats,
}

/// Statistics for batch processing operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchProcessingStats {
    /// Number of batches processed
    pub batches_processed: usize,
    /// Average batch processing time in milliseconds
    pub avg_batch_time_ms: f64,
    /// Minimum batch processing time in milliseconds
    pub min_batch_time_ms: f64,
    /// Maximum batch processing time in milliseconds
    pub max_batch_time_ms: f64,
    /// Average batch size
    pub avg_batch_size: f64,
}

impl Default for DeduplicationMetrics {
    fn default() -> Self {
        Self {
            total_time_ms: 0.0,
            embeddings_processed: 0,
            similarity_comparisons: 0,
            clusters_found: 0,
            duplicates_found: 0,
            throughput_embeddings_per_second: 0.0,
            avg_similarity_comparison_time_us: 0.0,
            estimated_memory_usage_bytes: 0,
            index_size_reduction_percentage: 0.0,
            used_parallel_processing: false,
            batch_stats: BatchProcessingStats::default(),
        }
    }
}

/// Main deduplication engine implementation
pub struct EmbeddingDeduplicator;

impl EmbeddingDeduplicator {
    /// Perform comprehensive deduplication on a set of embeddings
    /// 
    /// This is the main entry point for deduplication operations. It performs
    /// similarity-based duplicate detection, clustering, representative selection,
    /// and reference mapping creation.
    /// 
    /// # Arguments
    /// 
    /// * `embeddings` - Set of embeddings to deduplicate
    /// * `config` - Deduplication configuration parameters
    /// 
    /// # Returns
    /// 
    /// Complete deduplication results with clusters, mappings, and metrics
    /// 
    /// # Performance
    /// 
    /// - Target: <10 seconds per 1000 embeddings
    /// - Memory: O(n) for reference tracking
    /// - Scales to large datasets through batch processing
    pub fn deduplicate_embeddings(
        embeddings: Vec<EmbeddingEntry>,
        config: &DeduplicationConfig,
    ) -> DeduplicationResult<DeduplicationResult_> {
        let start_time = Instant::now();
        let mut metrics = DeduplicationMetrics::default();
        
        // Validate inputs
        Self::validate_config(config)?;
        if embeddings.is_empty() {
            return Err(DeduplicationError::EmptyEmbeddingSet);
        }
        
        metrics.embeddings_processed = embeddings.len();
        
        eprintln!("🔍 Starting deduplication of {} embeddings", embeddings.len());
        
        // Determine processing approach based on dataset size
        let use_parallel = config.enable_parallel_processing 
            && embeddings.len() >= config.parallel_threshold;
        metrics.used_parallel_processing = use_parallel;
        
        // Step 1: Detect duplicate clusters
        let clusters = if use_parallel && embeddings.len() > config.batch_size {
            Self::detect_duplicates_batch(&embeddings, config, &mut metrics)?
        } else {
            Self::detect_duplicates_sequential(&embeddings, config, &mut metrics)?
        };
        
        metrics.clusters_found = clusters.len();
        metrics.duplicates_found = clusters.iter()
            .map(|c| c.members.len().saturating_sub(1))
            .sum();
        
        eprintln!("📊 Found {} clusters with {} total duplicates", 
                  metrics.clusters_found, metrics.duplicates_found);
        
        // Step 2: Create reference mapping
        let reference_mapping = Self::create_reference_mapping(&clusters, &embeddings);
        
        // Step 3: Extract deduplicated embeddings (representatives only)
        let deduplicated_embeddings = Self::extract_representatives(&clusters);
        
        // Step 4: Calculate final metrics
        metrics.total_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        metrics.throughput_embeddings_per_second = if metrics.total_time_ms > 0.0 {
            (embeddings.len() as f64) / (metrics.total_time_ms / 1000.0)
        } else {
            0.0
        };
        
        let original_count = embeddings.len();
        let final_count = deduplicated_embeddings.len();
        metrics.index_size_reduction_percentage = if original_count > 0 {
            ((original_count - final_count) as f32 / original_count as f32) * 100.0
        } else {
            0.0
        };
        
        // Estimate memory usage
        metrics.estimated_memory_usage_bytes = Self::estimate_memory_usage(
            &clusters, 
            &reference_mapping
        );
        
        if config.enable_detailed_metrics {
            Self::log_detailed_metrics(&metrics, &reference_mapping);
        }
        
        Ok(DeduplicationResult_ {
            clusters,
            reference_mapping,
            metrics,
            deduplicated_embeddings,
        })
    }
    
    /// Detect duplicate clusters using sequential processing
    fn detect_duplicates_sequential(
        embeddings: &[EmbeddingEntry],
        config: &DeduplicationConfig,
        metrics: &mut DeduplicationMetrics,
    ) -> DeduplicationResult<Vec<DuplicateCluster>> {
        let start_time = Instant::now();
        let mut clusters = Vec::new();
        let mut processed_ids = HashSet::new();
        
        eprintln!("🔄 Processing {} embeddings sequentially", embeddings.len());
        
        for (i, embedding) in embeddings.iter().enumerate() {
            if processed_ids.contains(&embedding.id) {
                continue; // Already part of a cluster
            }
            
            // Find all similar embeddings
            let mut cluster_members = vec![embedding.clone()];
            let mut similarity_sum = 0.0;
            let mut similarity_count = 0;
            
            for (j, other_embedding) in embeddings.iter().enumerate() {
                if i == j || processed_ids.contains(&other_embedding.id) {
                    continue;
                }
                
                let similarity = SimilaritySearch::cosine_similarity(
                    &embedding.vector,
                    &other_embedding.vector,
                )?;
                
                metrics.similarity_comparisons += 1;
                
                if similarity >= config.similarity_threshold {
                    cluster_members.push(other_embedding.clone());
                    similarity_sum += similarity;
                    similarity_count += 1;
                    processed_ids.insert(other_embedding.id.clone());
                }
            }
            
            // Only create cluster if we found duplicates
            if cluster_members.len() > 1 {
                let average_similarity = if similarity_count > 0 {
                    similarity_sum / similarity_count as f32
                } else {
                    1.0
                };
                
                let representative = Self::select_representative(
                    &cluster_members,
                    &config.representative_selection_strategy,
                )?;
                
                clusters.push(DuplicateCluster {
                    representative,
                    members: cluster_members,
                    average_similarity,
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
            }
            
            processed_ids.insert(embedding.id.clone());
            
            // Progress reporting for large datasets
            if embeddings.len() > 1000 && i % 100 == 0 {
                let progress = (i as f32 / embeddings.len() as f32) * 100.0;
                eprintln!("⏳ Progress: {:.1}% ({}/{})", progress, i + 1, embeddings.len());
            }
        }
        
        let processing_time = start_time.elapsed().as_secs_f64() * 1000.0;
        if metrics.similarity_comparisons > 0 {
            metrics.avg_similarity_comparison_time_us = 
                (processing_time * 1000.0) / metrics.similarity_comparisons as f64;
        }
        
        Ok(clusters)
    }
    
    /// Detect duplicate clusters using batch processing for large datasets
    fn detect_duplicates_batch(
        embeddings: &[EmbeddingEntry],
        config: &DeduplicationConfig,
        metrics: &mut DeduplicationMetrics,
    ) -> DeduplicationResult<Vec<DuplicateCluster>> {
        let start_time = Instant::now();
        let mut all_clusters = Vec::new();
        let mut processed_ids = HashSet::new();
        let mut batch_times = Vec::new();
        
        eprintln!("🚀 Processing {} embeddings in batches of {}", 
                  embeddings.len(), config.batch_size);
        
        // Process embeddings in batches
        let batches: Vec<_> = embeddings.chunks(config.batch_size).collect();
        
        for (batch_idx, batch) in batches.iter().enumerate() {
            let batch_start = Instant::now();
            
            // Process current batch against all remaining embeddings
            for embedding in *batch {
                if processed_ids.contains(&embedding.id) {
                    continue;
                }
                
                let mut cluster_members = vec![embedding.clone()];
                let mut similarity_sum = 0.0;
                let mut similarity_count = 0;
                
                // Compare with all embeddings (not just current batch)
                for other_embedding in embeddings {
                    if embedding.id == other_embedding.id 
                        || processed_ids.contains(&other_embedding.id) {
                        continue;
                    }
                    
                    let similarity = SimilaritySearch::cosine_similarity(
                        &embedding.vector,
                        &other_embedding.vector,
                    )?;
                    
                    metrics.similarity_comparisons += 1;
                    
                    if similarity >= config.similarity_threshold {
                        cluster_members.push(other_embedding.clone());
                        similarity_sum += similarity;
                        similarity_count += 1;
                        processed_ids.insert(other_embedding.id.clone());
                    }
                }
                
                // Create cluster if duplicates found
                if cluster_members.len() > 1 {
                    let average_similarity = if similarity_count > 0 {
                        similarity_sum / similarity_count as f32
                    } else {
                        1.0
                    };
                    
                    let representative = Self::select_representative(
                        &cluster_members,
                        &config.representative_selection_strategy,
                    )?;
                    
                    all_clusters.push(DuplicateCluster {
                        representative,
                        members: cluster_members,
                        average_similarity,
                        created_at: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    });
                }
                
                processed_ids.insert(embedding.id.clone());
            }
            
            let batch_time = batch_start.elapsed().as_secs_f64() * 1000.0;
            batch_times.push(batch_time);
            
            eprintln!("📦 Batch {}/{} completed in {:.2}ms", 
                      batch_idx + 1, batches.len(), batch_time);
        }
        
        // Calculate batch processing statistics
        metrics.batch_stats = BatchProcessingStats {
            batches_processed: batches.len(),
            avg_batch_time_ms: batch_times.iter().sum::<f64>() / batches.len() as f64,
            min_batch_time_ms: batch_times.iter().cloned().fold(f64::INFINITY, f64::min),
            max_batch_time_ms: batch_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            avg_batch_size: embeddings.len() as f64 / batches.len() as f64,
        };
        
        let total_processing_time = start_time.elapsed().as_secs_f64() * 1000.0;
        if metrics.similarity_comparisons > 0 {
            metrics.avg_similarity_comparison_time_us = 
                (total_processing_time * 1000.0) / metrics.similarity_comparisons as f64;
        }
        
        Ok(all_clusters)
    }
    
    /// Select the best representative from a cluster of duplicates
    fn select_representative(
        cluster_members: &[EmbeddingEntry],
        strategy: &RepresentativeSelectionStrategy,
    ) -> DeduplicationResult<EmbeddingEntry> {
        if cluster_members.is_empty() {
            return Err(DeduplicationError::ReferenceTracking {
                message: "Cannot select representative from empty cluster".to_string(),
            });
        }
        
        if cluster_members.len() == 1 {
            return Ok(cluster_members[0].clone());
        }
        
        let representative = match strategy {
            RepresentativeSelectionStrategy::MostRecent => {
                cluster_members.iter()
                    .max_by_key(|e| e.updated_at)
                    .unwrap()
                    .clone()
            }
            
            RepresentativeSelectionStrategy::Earliest => {
                cluster_members.iter()
                    .min_by_key(|e| e.created_at)
                    .unwrap()
                    .clone()
            }
            
            RepresentativeSelectionStrategy::LongestContent => {
                cluster_members.iter()
                    .max_by_key(|e| e.metadata.text_length)
                    .unwrap()
                    .clone()
            }
            
            RepresentativeSelectionStrategy::HighestAverageSimilarity => {
                // Calculate average similarity for each embedding to all others
                let mut best_embedding = &cluster_members[0];
                let mut best_avg_similarity = 0.0;
                
                for candidate in cluster_members {
                    let mut similarity_sum = 0.0;
                    let mut count = 0;
                    
                    for other in cluster_members {
                        if candidate.id != other.id {
                            if let Ok(similarity) = SimilaritySearch::cosine_similarity(
                                &candidate.vector,
                                &other.vector,
                            ) {
                                similarity_sum += similarity;
                                count += 1;
                            }
                        }
                    }
                    
                    if count > 0 {
                        let avg_similarity = similarity_sum / count as f32;
                        if avg_similarity > best_avg_similarity {
                            best_avg_similarity = avg_similarity;
                            best_embedding = candidate;
                        }
                    }
                }
                
                best_embedding.clone()
            }
        };
        
        Ok(representative)
    }
    
    /// Create comprehensive reference mapping from clusters
    fn create_reference_mapping(
        clusters: &[DuplicateCluster],
        original_embeddings: &[EmbeddingEntry],
    ) -> ReferenceMapping {
        let mut mapping = HashMap::new();
        let mut reverse_mapping = HashMap::new();
        
        // Create mapping for clustered embeddings
        for cluster in clusters {
            let representative_id = cluster.representative.id.clone();
            let mut referenced_ids = HashSet::new();
            
            for member in &cluster.members {
                if member.id != representative_id {
                    mapping.insert(member.id.clone(), representative_id.clone());
                    referenced_ids.insert(member.id.clone());
                }
            }
            
            if !referenced_ids.is_empty() {
                reverse_mapping.insert(representative_id, referenced_ids);
            }
        }
        
        // Calculate statistics
        let total_duplicates = mapping.len();
        let total_representatives = reverse_mapping.len();
        let total_original = original_embeddings.len();
        let reduction_ratio = if total_original > 0 {
            total_duplicates as f32 / total_original as f32
        } else {
            0.0
        };
        
        ReferenceMapping {
            mapping,
            reverse_mapping,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            stats: ReferenceMappingStats {
                total_original_embeddings: total_original,
                total_representatives,
                total_duplicates_found: total_duplicates,
                reduction_ratio,
            },
        }
    }
    
    /// Extract representative embeddings from clusters
    fn extract_representatives(clusters: &[DuplicateCluster]) -> Vec<EmbeddingEntry> {
        let mut representatives = Vec::new();
        let mut seen_ids = HashSet::new();
        
        // Add cluster representatives
        for cluster in clusters {
            if !seen_ids.contains(&cluster.representative.id) {
                representatives.push(cluster.representative.clone());
                seen_ids.insert(cluster.representative.id.clone());
            }
        }
        
        representatives
    }
    
    /// Validate deduplication configuration
    fn validate_config(config: &DeduplicationConfig) -> DeduplicationResult<()> {
        if !(0.0..=1.0).contains(&config.similarity_threshold) {
            return Err(DeduplicationError::InvalidThreshold {
                threshold: config.similarity_threshold,
            });
        }
        
        if !(0.0..=1.0).contains(&config.min_similarity_threshold) {
            return Err(DeduplicationError::InvalidThreshold {
                threshold: config.min_similarity_threshold,
            });
        }
        
        if config.similarity_threshold < config.min_similarity_threshold {
            return Err(DeduplicationError::InvalidThreshold {
                threshold: config.similarity_threshold,
            });
        }
        
        Ok(())
    }
    
    /// Estimate memory usage for deduplication results
    fn estimate_memory_usage(
        clusters: &[DuplicateCluster],
        reference_mapping: &ReferenceMapping,
    ) -> usize {
        let cluster_size = clusters.iter()
            .map(|c| {
                c.members.iter()
                    .map(|m| m.memory_footprint())
                    .sum::<usize>()
            })
            .sum::<usize>();
        
        let mapping_size = reference_mapping.mapping.len() * 64 + // String keys/values
            reference_mapping.reverse_mapping.len() * 64;
        
        cluster_size + mapping_size
    }
    
    /// Log detailed metrics for analysis
    fn log_detailed_metrics(
        metrics: &DeduplicationMetrics,
        reference_mapping: &ReferenceMapping,
    ) {
        eprintln!("📈 Deduplication Metrics:");
        eprintln!("   Total Time: {:.2}ms", metrics.total_time_ms);
        eprintln!("   Embeddings Processed: {}", metrics.embeddings_processed);
        eprintln!("   Similarity Comparisons: {}", metrics.similarity_comparisons);
        eprintln!("   Clusters Found: {}", metrics.clusters_found);
        eprintln!("   Duplicates Found: {}", metrics.duplicates_found);
        eprintln!("   Index Size Reduction: {:.1}%", metrics.index_size_reduction_percentage);
        eprintln!("   Throughput: {:.1} embeddings/sec", metrics.throughput_embeddings_per_second);
        eprintln!("   Avg Similarity Time: {:.2}μs", metrics.avg_similarity_comparison_time_us);
        eprintln!("   Memory Usage: {:.1}KB", metrics.estimated_memory_usage_bytes as f64 / 1024.0);
        
        if metrics.used_parallel_processing {
            eprintln!("   Batch Stats:");
            eprintln!("     Batches: {}", metrics.batch_stats.batches_processed);
            eprintln!("     Avg Batch Time: {:.2}ms", metrics.batch_stats.avg_batch_time_ms);
        }
        
        eprintln!("📊 Reference Mapping Stats:");
        eprintln!("   Total Original: {}", reference_mapping.stats.total_original_embeddings);
        eprintln!("   Representatives: {}", reference_mapping.stats.total_representatives);
        eprintln!("   Reduction Ratio: {:.1}%", reference_mapping.stats.reduction_ratio * 100.0);
    }
}

/// Additional utility functions for deduplication management
impl EmbeddingDeduplicator {
    /// Apply deduplication results to resolve embedding references
    /// 
    /// This function helps resolve embedding references after deduplication,
    /// allowing backward compatibility with existing embedding IDs.
    pub fn resolve_embedding_reference(
        embedding_id: &str,
        reference_mapping: &ReferenceMapping,
    ) -> String {
        reference_mapping.mapping
            .get(embedding_id)
            .cloned()
            .unwrap_or_else(|| embedding_id.to_string())
    }
    
    /// Get all original IDs that were merged into a representative
    pub fn get_merged_ids(
        representative_id: &str,
        reference_mapping: &ReferenceMapping,
    ) -> Vec<String> {
        reference_mapping.reverse_mapping
            .get(representative_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Check if an embedding ID was affected by deduplication
    pub fn was_deduplicated(
        embedding_id: &str,
        reference_mapping: &ReferenceMapping,
    ) -> bool {
        reference_mapping.mapping.contains_key(embedding_id)
    }
    
    /// Calculate deduplication statistics
    pub fn calculate_deduplication_stats(
        original_count: usize,
        deduplicated_count: usize,
    ) -> (f32, f32) {
        let reduction_count = original_count.saturating_sub(deduplicated_count);
        let reduction_percentage = if original_count > 0 {
            (reduction_count as f32 / original_count as f32) * 100.0
        } else {
            0.0
        };
        
        let compression_ratio = if deduplicated_count > 0 {
            original_count as f32 / deduplicated_count as f32
        } else {
            1.0
        };
        
        (reduction_percentage, compression_ratio)
    }
    
    /// Validate deduplication results integrity
    pub fn validate_deduplication_integrity(
        result: &DeduplicationResult_,
        original_embeddings: &[EmbeddingEntry],
    ) -> DeduplicationResult<()> {
        // Verify all original embeddings are accounted for
        let mut accounted_ids = HashSet::new();
        
        // Add representatives
        for embedding in &result.deduplicated_embeddings {
            accounted_ids.insert(embedding.id.clone());
        }
        
        // Add mapped IDs
        for original_id in result.reference_mapping.mapping.keys() {
            accounted_ids.insert(original_id.clone());
        }
        
        // Check that all original IDs are accounted for
        for original in original_embeddings {
            if !accounted_ids.contains(&original.id) {
                return Err(DeduplicationError::ReferenceTracking {
                    message: format!("Original embedding {} not accounted for in deduplication", original.id),
                });
            }
        }
        
        // Verify reference mapping consistency
        for (original_id, representative_id) in &result.reference_mapping.mapping {
            if !result.deduplicated_embeddings.iter().any(|e| e.id == *representative_id) {
                return Err(DeduplicationError::ReferenceTracking {
                    message: format!("Representative {} referenced but not in deduplicated set", representative_id),
                });
            }
            
            // Verify reverse mapping consistency
            if let Some(reverse_set) = result.reference_mapping.reverse_mapping.get(representative_id) {
                if !reverse_set.contains(original_id) {
                    return Err(DeduplicationError::ReferenceTracking {
                        message: "Inconsistent forward and reverse reference mappings".to_string(),
                    });
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::types::EmbeddingEntry;
    
    fn create_test_embedding(id: &str, vector: Vec<f32>, file_path: &str) -> EmbeddingEntry {
        EmbeddingEntry::new(
            vector,
            file_path.to_string(),
            format!("chunk_{}", id),
            "test content",
            "test-model".to_string(),
        )
    }
    
    #[test]
    fn test_deduplication_config_validation() {
        let mut config = DeduplicationConfig::default();
        assert!(EmbeddingDeduplicator::validate_config(&config).is_ok());
        
        config.similarity_threshold = 1.5;
        assert!(EmbeddingDeduplicator::validate_config(&config).is_err());
        
        config.similarity_threshold = 0.85;
        config.min_similarity_threshold = 0.90;
        assert!(EmbeddingDeduplicator::validate_config(&config).is_err());
    }
    
    #[test]
    fn test_representative_selection_most_recent() {
        let mut embeddings = vec![
            create_test_embedding("1", vec![1.0, 0.0], "file1.md"),
            create_test_embedding("2", vec![1.0, 0.0], "file2.md"),
        ];
        
        // Make second embedding more recent
        embeddings[1].updated_at = embeddings[0].updated_at + 100;
        
        let representative = EmbeddingDeduplicator::select_representative(
            &embeddings,
            &RepresentativeSelectionStrategy::MostRecent,
        ).unwrap();
        
        assert_eq!(representative.id, embeddings[1].id);
    }
    
    #[test]
    fn test_representative_selection_longest_content() {
        let mut embeddings = vec![
            create_test_embedding("1", vec![1.0, 0.0], "file1.md"),
            create_test_embedding("2", vec![1.0, 0.0], "file2.md"),
        ];
        
        // Make second embedding have longer content
        embeddings[1].metadata.text_length = 1000;
        embeddings[0].metadata.text_length = 100;
        
        let representative = EmbeddingDeduplicator::select_representative(
            &embeddings,
            &RepresentativeSelectionStrategy::LongestContent,
        ).unwrap();
        
        assert_eq!(representative.id, embeddings[1].id);
    }
    
    #[test]
    fn test_resolve_embedding_reference() {
        let mut mapping = ReferenceMapping {
            mapping: HashMap::new(),
            reverse_mapping: HashMap::new(),
            created_at: 0,
            stats: ReferenceMappingStats::default(),
        };
        
        mapping.mapping.insert("duplicate_id".to_string(), "representative_id".to_string());
        
        // Test existing mapping
        let resolved = EmbeddingDeduplicator::resolve_embedding_reference(
            "duplicate_id",
            &mapping,
        );
        assert_eq!(resolved, "representative_id");
        
        // Test non-existing mapping
        let resolved = EmbeddingDeduplicator::resolve_embedding_reference(
            "unknown_id",
            &mapping,
        );
        assert_eq!(resolved, "unknown_id");
    }
    
    #[test]
    fn test_deduplication_stats_calculation() {
        let (reduction_percentage, compression_ratio) = 
            EmbeddingDeduplicator::calculate_deduplication_stats(1000, 750);
        
        assert_eq!(reduction_percentage, 25.0);
        assert!((compression_ratio - 1.333).abs() < 0.01);
    }
    
    #[test] 
    fn test_empty_embedding_set() {
        let config = DeduplicationConfig::default();
        let result = EmbeddingDeduplicator::deduplicate_embeddings(vec![], &config);
        
        assert!(matches!(result, Err(DeduplicationError::EmptyEmbeddingSet)));
    }
}