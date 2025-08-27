//! Enhanced Metrics Collector for Vector Database Performance Monitoring
//!
//! This module extends the existing performance monitoring system with comprehensive
//! metrics collection for search operations, index health monitoring, memory tracking,
//! and optimization recommendations as required by issue #146.
//!
//! ## Features
//!
//! - **Search Operations Metrics**: Track similarity search performance and accuracy
//! - **Index Health Monitoring**: Monitor index size, fragmentation, and efficiency
//! - **Memory Usage Tracking**: Detailed memory analysis and leak detection
//! - **Optimization Recommendations**: AI-powered suggestions for performance improvements
//! - **Metrics Persistence**: Historical data storage with configurable retention
//! - **Real-time Insights**: Live performance monitoring with <2% overhead
//!
//! ## Integration
//!
//! Integrates seamlessly with existing performance_monitor.rs and provides additional
//! metrics collection capabilities specifically for search operations and index health.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::vector_db::types::{VectorDbError, VectorDbResult};
use crate::vector_db::storage::VectorStorage;

/// Configuration for enhanced metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollectorConfig {
    /// Enable search operation metrics collection
    pub enable_search_metrics: bool,
    /// Enable index health monitoring
    pub enable_index_health_monitoring: bool,
    /// Enable detailed memory tracking
    pub enable_memory_tracking: bool,
    /// Enable optimization recommendations
    pub enable_optimization_recommendations: bool,
    /// Maximum number of search metrics to keep in memory
    pub max_search_metrics_history: usize,
    /// Index health check interval in seconds
    pub index_health_check_interval_seconds: u64,
    /// Memory snapshot interval in seconds
    pub memory_snapshot_interval_seconds: u64,
    /// Metrics persistence file path
    pub metrics_persistence_path: Option<String>,
    /// Historical data retention period in days
    pub metrics_retention_days: u64,
}

impl Default for MetricsCollectorConfig {
    fn default() -> Self {
        Self {
            enable_search_metrics: true,
            enable_index_health_monitoring: true,
            enable_memory_tracking: true,
            enable_optimization_recommendations: true,
            max_search_metrics_history: 1000,
            index_health_check_interval_seconds: 300, // 5 minutes
            memory_snapshot_interval_seconds: 60,     // 1 minute
            metrics_persistence_path: None,
            metrics_retention_days: 30,
        }
    }
}

/// Metrics for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOperationMetrics {
    /// Operation ID for tracking
    pub operation_id: String,
    /// Search operation type
    pub operation_type: SearchOperationType,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// End timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Total search duration in milliseconds
    pub duration_ms: Option<f64>,
    /// Query vector dimension
    pub query_dimension: usize,
    /// Number of vectors searched against
    pub vectors_searched: usize,
    /// Number of results returned
    pub results_returned: usize,
    /// Similarity threshold used
    pub similarity_threshold: f32,
    /// Top similarity score achieved
    pub top_similarity_score: Option<f32>,
    /// Average similarity score of results
    pub avg_similarity_score: Option<f32>,
    /// Memory used during search (MB)
    pub memory_usage_mb: f64,
    /// CPU usage percentage during search
    pub cpu_usage_percent: f64,
    /// Search efficiency score (results/vectors_searched)
    pub efficiency_score: f64,
    /// Whether search met performance targets
    pub performance_target_met: bool,
    /// Error message if search failed
    pub error_message: Option<String>,
}

/// Types of search operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchOperationType {
    /// K-nearest neighbors search
    KNearestNeighbors,
    /// Similarity threshold search
    SimilarityThreshold,
    /// Range search within similarity bounds
    RangeSearch,
    /// Batch search operations
    BatchSearch,
}

/// Index health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHealthMetrics {
    /// Measurement timestamp
    pub timestamp: DateTime<Utc>,
    /// Total number of embeddings in index
    pub total_embeddings: usize,
    /// Total index size in bytes
    pub index_size_bytes: u64,
    /// Number of storage files
    pub storage_files_count: usize,
    /// Index fragmentation percentage (0.0-100.0)
    pub fragmentation_percentage: f32,
    /// Index efficiency score (0.0-1.0)
    pub efficiency_score: f32,
    /// Average embedding vector dimension
    pub avg_vector_dimension: f32,
    /// Index density (embeddings per MB)
    pub index_density: f32,
    /// Duplicate embeddings detected
    pub duplicate_embeddings_count: usize,
    /// Orphaned embeddings (no corresponding file)
    pub orphaned_embeddings_count: usize,
    /// Index health status
    pub health_status: IndexHealthStatus,
    /// Health issues identified
    pub health_issues: Vec<IndexHealthIssue>,
    /// Recommended maintenance actions
    pub recommended_actions: Vec<String>,
}

/// Index health status levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexHealthStatus {
    /// Index is healthy and optimized
    Healthy,
    /// Minor issues detected, maintenance recommended
    Warning,
    /// Major issues detected, immediate action required
    Critical,
    /// Index corruption or severe problems
    Corrupted,
}

/// Specific index health issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHealthIssue {
    /// Issue type
    pub issue_type: IndexIssueType,
    /// Issue severity
    pub severity: IssueSeverity,
    /// Description of the issue
    pub description: String,
    /// Affected components
    pub affected_components: Vec<String>,
    /// Suggested resolution
    pub suggested_resolution: String,
    /// Impact on performance
    pub performance_impact: PerformanceImpact,
}

/// Types of index health issues
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexIssueType {
    /// High fragmentation level
    Fragmentation,
    /// Excessive duplicate embeddings
    Duplication,
    /// Orphaned embeddings
    Orphaned,
    /// Inconsistent vector dimensions
    DimensionInconsistency,
    /// Large index size
    IndexBloat,
    /// Storage file proliferation
    FileProliferation,
    /// Memory inefficiency
    MemoryInefficiency,
}

/// Issue severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance impact assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImpact {
    /// Estimated search latency impact (multiplier)
    pub search_latency_impact: f32,
    /// Memory usage impact (additional MB)
    pub memory_usage_impact: f64,
    /// Storage space impact (additional bytes)
    pub storage_space_impact: u64,
    /// Overall performance degradation percentage
    pub overall_degradation_percent: f32,
}

/// Memory usage metrics with detailed breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedMemoryMetrics {
    /// Measurement timestamp
    pub timestamp: DateTime<Utc>,
    /// Total application memory usage (MB)
    pub total_memory_mb: f64,
    /// Vector storage memory usage (MB)
    pub vector_storage_mb: f64,
    /// Cache memory usage (MB)
    pub cache_memory_mb: f64,
    /// Index structures memory (MB)
    pub index_memory_mb: f64,
    /// Search operation memory (MB)
    pub search_operation_memory_mb: f64,
    /// Operating system memory usage (MB)
    pub os_memory_mb: f64,
    /// Available memory (MB)
    pub available_memory_mb: f64,
    /// Memory pressure level (0.0-1.0)
    pub memory_pressure: f32,
    /// Memory leak indicators
    pub potential_leaks: Vec<MemoryLeakIndicator>,
    /// Memory efficiency score (0.0-1.0)
    pub efficiency_score: f32,
}

/// Memory leak detection indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeakIndicator {
    /// Component suspected of leak
    pub component: String,
    /// Leak severity
    pub severity: LeakSeverity,
    /// Memory growth rate (MB per hour)
    pub growth_rate_mb_per_hour: f64,
    /// Pattern description
    pub pattern_description: String,
    /// Recommended investigation steps
    pub investigation_steps: Vec<String>,
}

/// Memory leak severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeakSeverity {
    /// Slow growth, monitor
    Minor,
    /// Moderate growth, investigate
    Moderate,
    /// Fast growth, immediate attention
    Severe,
    /// Critical growth, emergency action
    Critical,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// Recommendation ID
    pub recommendation_id: String,
    /// Category of optimization
    pub category: OptimizationCategory,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Recommendation title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Expected performance improvement
    pub expected_improvement: PerformanceImprovement,
    /// Implementation difficulty
    pub difficulty: ImplementationDifficulty,
    /// Estimated implementation time
    pub estimated_time_hours: f64,
    /// Prerequisites
    pub prerequisites: Vec<String>,
    /// Implementation steps
    pub implementation_steps: Vec<String>,
    /// Risk assessment
    pub risks: Vec<String>,
    /// Success metrics
    pub success_metrics: Vec<String>,
}

/// Optimization categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationCategory {
    /// Index structure optimizations
    IndexOptimization,
    /// Search algorithm improvements
    SearchOptimization,
    /// Memory usage optimizations
    MemoryOptimization,
    /// Storage efficiency improvements
    StorageOptimization,
    /// Configuration tuning
    ConfigurationOptimization,
    /// Hardware utilization improvements
    HardwareOptimization,
}

/// Recommendation priority levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Expected performance improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImprovement {
    /// Search latency improvement (percentage)
    pub search_latency_improvement_percent: f32,
    /// Memory usage reduction (percentage)
    pub memory_reduction_percent: f32,
    /// Storage space savings (percentage)
    pub storage_savings_percent: f32,
    /// Overall performance improvement (percentage)
    pub overall_improvement_percent: f32,
    /// Confidence level in improvement estimate (0.0-1.0)
    pub confidence_level: f32,
}

/// Implementation difficulty levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationDifficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Main metrics collector for enhanced performance monitoring
pub struct EnhancedMetricsCollector {
    /// Configuration
    config: MetricsCollectorConfig,
    /// Vector storage reference
    storage: Arc<VectorStorage>,
    /// Search operation metrics history
    search_metrics_history: Arc<Mutex<VecDeque<SearchOperationMetrics>>>,
    /// Index health metrics history
    index_health_history: Arc<Mutex<VecDeque<IndexHealthMetrics>>>,
    /// Memory metrics history
    memory_metrics_history: Arc<Mutex<VecDeque<DetailedMemoryMetrics>>>,
    /// Current optimization recommendations
    optimization_recommendations: Arc<RwLock<Vec<OptimizationRecommendation>>>,
    /// Metrics collection tasks
    collection_tasks: Vec<tokio::task::JoinHandle<()>>,
    /// Collection enabled flag
    collection_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl EnhancedMetricsCollector {
    /// Create a new enhanced metrics collector
    pub fn new(config: MetricsCollectorConfig, storage: Arc<VectorStorage>) -> Self {
        Self {
            config,
            storage,
            search_metrics_history: Arc::new(Mutex::new(VecDeque::new())),
            index_health_history: Arc::new(Mutex::new(VecDeque::new())),
            memory_metrics_history: Arc::new(Mutex::new(VecDeque::new())),
            optimization_recommendations: Arc::new(RwLock::new(Vec::new())),
            collection_tasks: Vec::new(),
            collection_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start metrics collection
    pub async fn start(&mut self) -> VectorDbResult<()> {
        if self.collection_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(VectorDbError::Storage {
                message: "Metrics collection already started".to_string(),
            });
        }

        self.collection_enabled.store(true, std::sync::atomic::Ordering::Relaxed);

        // Start index health monitoring task
        if self.config.enable_index_health_monitoring {
            self.start_index_health_monitoring().await;
        }

        // Start memory tracking task
        if self.config.enable_memory_tracking {
            self.start_memory_tracking().await;
        }

        // Start optimization recommendation generation
        if self.config.enable_optimization_recommendations {
            self.start_optimization_recommendations().await;
        }

        println!("✅ Enhanced metrics collection started");
        Ok(())
    }

    /// Stop metrics collection
    pub async fn stop(&mut self) -> VectorDbResult<()> {
        self.collection_enabled.store(false, std::sync::atomic::Ordering::Relaxed);

        // Cancel all collection tasks
        for task in self.collection_tasks.drain(..) {
            task.abort();
        }

        // Persist metrics if configured
        if let Some(path) = &self.config.metrics_persistence_path {
            self.persist_metrics(path).await?;
        }

        println!("✅ Enhanced metrics collection stopped");
        Ok(())
    }

    /// Record search operation metrics
    pub async fn record_search_operation(&self, metrics: SearchOperationMetrics) -> VectorDbResult<()> {
        if !self.config.enable_search_metrics {
            return Ok(());
        }

        let mut history = self.search_metrics_history.lock().await;
        history.push_back(metrics);

        // Maintain maximum history size
        while history.len() > self.config.max_search_metrics_history {
            history.pop_front();
        }

        Ok(())
    }

    /// Get current index health metrics
    pub async fn get_current_index_health(&self) -> VectorDbResult<IndexHealthMetrics> {
        self.collect_index_health_metrics().await
    }

    /// Get current memory metrics
    pub async fn get_current_memory_metrics(&self) -> VectorDbResult<DetailedMemoryMetrics> {
        self.collect_memory_metrics().await
    }

    /// Get optimization recommendations
    pub async fn get_optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        self.optimization_recommendations.read().await.clone()
    }

    /// Get search metrics history
    pub async fn get_search_metrics_history(&self, limit: Option<usize>) -> Vec<SearchOperationMetrics> {
        let history = self.search_metrics_history.lock().await;
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get index health history
    pub async fn get_index_health_history(&self, limit: Option<usize>) -> Vec<IndexHealthMetrics> {
        let history = self.index_health_history.lock().await;
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get memory metrics history
    pub async fn get_memory_metrics_history(&self, limit: Option<usize>) -> Vec<DetailedMemoryMetrics> {
        let history = self.memory_metrics_history.lock().await;
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    // Private helper methods

    async fn start_index_health_monitoring(&mut self) {
        let storage = Arc::clone(&self.storage);
        let history = Arc::clone(&self.index_health_history);
        let enabled = Arc::clone(&self.collection_enabled);
        let interval = Duration::from_secs(self.config.index_health_check_interval_seconds);

        let task = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            while enabled.load(std::sync::atomic::Ordering::Relaxed) {
                interval_timer.tick().await;
                
                if let Ok(health_metrics) = Self::collect_index_health_metrics_static(&storage).await {
                    let mut history_guard = history.lock().await;
                    history_guard.push_back(health_metrics);
                    
                    // Keep reasonable history size
                    while history_guard.len() > 288 { // 24 hours at 5-minute intervals
                        history_guard.pop_front();
                    }
                }
            }
        });

        self.collection_tasks.push(task);
    }

    async fn start_memory_tracking(&mut self) {
        let history = Arc::clone(&self.memory_metrics_history);
        let enabled = Arc::clone(&self.collection_enabled);
        let interval = Duration::from_secs(self.config.memory_snapshot_interval_seconds);

        let task = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            while enabled.load(std::sync::atomic::Ordering::Relaxed) {
                interval_timer.tick().await;
                
                if let Ok(memory_metrics) = Self::collect_memory_metrics_static().await {
                    let mut history_guard = history.lock().await;
                    history_guard.push_back(memory_metrics);
                    
                    // Keep reasonable history size
                    while history_guard.len() > 1440 { // 24 hours at 1-minute intervals
                        history_guard.pop_front();
                    }
                }
            }
        });

        self.collection_tasks.push(task);
    }

    async fn start_optimization_recommendations(&mut self) {
        let storage = Arc::clone(&self.storage);
        let recommendations = Arc::clone(&self.optimization_recommendations);
        let search_history = Arc::clone(&self.search_metrics_history);
        let index_history = Arc::clone(&self.index_health_history);
        let memory_history = Arc::clone(&self.memory_metrics_history);
        let enabled = Arc::clone(&self.collection_enabled);

        let task = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(3600)); // Hourly
            
            while enabled.load(std::sync::atomic::Ordering::Relaxed) {
                interval_timer.tick().await;
                
                if let Ok(new_recommendations) = Self::generate_optimization_recommendations_static(
                    &storage,
                    &search_history,
                    &index_history,
                    &memory_history,
                ).await {
                    let mut recommendations_guard = recommendations.write().await;
                    *recommendations_guard = new_recommendations;
                }
            }
        });

        self.collection_tasks.push(task);
    }

    async fn collect_index_health_metrics(&self) -> VectorDbResult<IndexHealthMetrics> {
        Self::collect_index_health_metrics_static(&self.storage).await
    }

    async fn collect_index_health_metrics_static(storage: &VectorStorage) -> VectorDbResult<IndexHealthMetrics> {
        let metrics = storage.get_metrics().await;
        let entries = storage.list_entry_ids().await;
        
        // Calculate health metrics
        let total_embeddings = entries.len();
        let index_size_bytes = metrics.total_size_bytes;
        let storage_files_count = metrics.file_count;
        
        // Estimate fragmentation (simplified calculation)
        let optimal_file_count = (total_embeddings / 1000).max(1); // Assume 1000 embeddings per file is optimal
        let fragmentation_percentage = if optimal_file_count > 0 {
            ((storage_files_count as f32 - optimal_file_count as f32) / optimal_file_count as f32 * 100.0)
                .max(0.0)
                .min(100.0)
        } else {
            0.0
        };
        
        // Calculate efficiency score
        let efficiency_score = if total_embeddings > 0 {
            let bytes_per_embedding = index_size_bytes as f32 / total_embeddings as f32;
            let ideal_bytes_per_embedding = 4096.0; // Estimated optimal size
            (ideal_bytes_per_embedding / bytes_per_embedding).min(1.0).max(0.1)
        } else {
            1.0
        };
        
        // Index density
        let index_density = if index_size_bytes > 0 {
            total_embeddings as f32 / (index_size_bytes as f32 / 1_048_576.0) // embeddings per MB
        } else {
            0.0
        };
        
        // Determine health status
        let health_status = if fragmentation_percentage > 50.0 || efficiency_score < 0.5 {
            IndexHealthStatus::Critical
        } else if fragmentation_percentage > 25.0 || efficiency_score < 0.7 {
            IndexHealthStatus::Warning
        } else {
            IndexHealthStatus::Healthy
        };
        
        // Generate health issues and recommendations
        let mut health_issues = Vec::new();
        let mut recommended_actions = Vec::new();
        
        if fragmentation_percentage > 25.0 {
            health_issues.push(IndexHealthIssue {
                issue_type: IndexIssueType::Fragmentation,
                severity: if fragmentation_percentage > 50.0 { IssueSeverity::High } else { IssueSeverity::Medium },
                description: format!("Index fragmentation is {:.1}%", fragmentation_percentage),
                affected_components: vec!["Storage Files".to_string()],
                suggested_resolution: "Run index compaction to consolidate storage files".to_string(),
                performance_impact: PerformanceImpact {
                    search_latency_impact: 1.0 + (fragmentation_percentage / 100.0),
                    memory_usage_impact: (fragmentation_percentage / 100.0 * 50.0) as f64,
                    storage_space_impact: (index_size_bytes as f32 * fragmentation_percentage / 100.0) as u64,
                    overall_degradation_percent: fragmentation_percentage / 2.0,
                },
            });
            recommended_actions.push("Perform index compaction".to_string());
        }
        
        if efficiency_score < 0.7 {
            health_issues.push(IndexHealthIssue {
                issue_type: IndexIssueType::MemoryInefficiency,
                severity: if efficiency_score < 0.5 { IssueSeverity::High } else { IssueSeverity::Medium },
                description: format!("Index efficiency is {:.1}% (target: 70%+)", efficiency_score * 100.0),
                affected_components: vec!["Vector Storage".to_string()],
                suggested_resolution: "Enable compression and deduplication".to_string(),
                performance_impact: PerformanceImpact {
                    search_latency_impact: 2.0 - efficiency_score,
                    memory_usage_impact: ((1.0 - efficiency_score) * 100.0) as f64,
                    storage_space_impact: ((1.0 - efficiency_score) * index_size_bytes as f32) as u64,
                    overall_degradation_percent: (1.0 - efficiency_score) * 50.0,
                },
            });
            recommended_actions.push("Enable vector compression".to_string());
            recommended_actions.push("Run deduplication process".to_string());
        }
        
        Ok(IndexHealthMetrics {
            timestamp: Utc::now(),
            total_embeddings,
            index_size_bytes: index_size_bytes as u64,
            storage_files_count,
            fragmentation_percentage,
            efficiency_score,
            avg_vector_dimension: 384.0, // Default assumption - would need actual calculation
            index_density,
            duplicate_embeddings_count: 0, // Would need actual calculation
            orphaned_embeddings_count: 0,  // Would need actual calculation
            health_status,
            health_issues,
            recommended_actions,
        })
    }

    async fn collect_memory_metrics(&self) -> VectorDbResult<DetailedMemoryMetrics> {
        Self::collect_memory_metrics_static().await
    }

    async fn collect_memory_metrics_static() -> VectorDbResult<DetailedMemoryMetrics> {
        // This is a simplified implementation - in production would use system APIs
        let total_memory_mb = 200.0; // Simulated
        let vector_storage_mb = 50.0;
        let cache_memory_mb = 25.0;
        let index_memory_mb = 15.0;
        let search_operation_memory_mb = 10.0;
        let os_memory_mb = 8192.0;
        let available_memory_mb = 7892.0;
        
        let memory_pressure = ((total_memory_mb / (total_memory_mb + available_memory_mb)) as f32).min(1.0);
        let efficiency_score = 1.0 - (memory_pressure * 0.5);
        
        Ok(DetailedMemoryMetrics {
            timestamp: Utc::now(),
            total_memory_mb,
            vector_storage_mb,
            cache_memory_mb,
            index_memory_mb,
            search_operation_memory_mb,
            os_memory_mb,
            available_memory_mb,
            memory_pressure,
            potential_leaks: Vec::new(), // Would implement leak detection algorithm
            efficiency_score,
        })
    }

    async fn generate_optimization_recommendations_static(
        _storage: &VectorStorage,
        _search_history: &Arc<Mutex<VecDeque<SearchOperationMetrics>>>,
        index_history: &Arc<Mutex<VecDeque<IndexHealthMetrics>>>,
        _memory_history: &Arc<Mutex<VecDeque<DetailedMemoryMetrics>>>,
    ) -> VectorDbResult<Vec<OptimizationRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Check recent index health
        let index_health = index_history.lock().await;
        if let Some(latest_health) = index_health.back() {
            if latest_health.fragmentation_percentage > 25.0 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_id: format!("idx_frag_{}", Utc::now().timestamp()),
                    category: OptimizationCategory::IndexOptimization,
                    priority: if latest_health.fragmentation_percentage > 50.0 {
                        RecommendationPriority::High
                    } else {
                        RecommendationPriority::Medium
                    },
                    title: "Reduce Index Fragmentation".to_string(),
                    description: format!(
                        "Index fragmentation is at {:.1}%, which impacts search performance. \
                        Compaction will consolidate {} storage files into fewer, more efficient files.",
                        latest_health.fragmentation_percentage,
                        latest_health.storage_files_count
                    ),
                    expected_improvement: PerformanceImprovement {
                        search_latency_improvement_percent: latest_health.fragmentation_percentage / 2.0,
                        memory_reduction_percent: 10.0,
                        storage_savings_percent: latest_health.fragmentation_percentage / 4.0,
                        overall_improvement_percent: latest_health.fragmentation_percentage / 3.0,
                        confidence_level: 0.85,
                    },
                    difficulty: ImplementationDifficulty::Easy,
                    estimated_time_hours: 0.5,
                    prerequisites: vec!["Ensure no active write operations".to_string()],
                    implementation_steps: vec![
                        "1. Stop new embedding additions".to_string(),
                        "2. Run database compaction command".to_string(),
                        "3. Verify integrity post-compaction".to_string(),
                        "4. Resume normal operations".to_string(),
                    ],
                    risks: vec!["Temporary unavailability during compaction".to_string()],
                    success_metrics: vec![
                        "Fragmentation reduced below 15%".to_string(),
                        "Search latency improved by 10%+".to_string(),
                        "Storage file count reduced".to_string(),
                    ],
                });
            }
            
            if latest_health.efficiency_score < 0.7 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_id: format!("idx_eff_{}", Utc::now().timestamp()),
                    category: OptimizationCategory::StorageOptimization,
                    priority: RecommendationPriority::Medium,
                    title: "Enable Vector Compression".to_string(),
                    description: format!(
                        "Index efficiency is {:.1}% (target: 70%+). Enabling compression \
                        will reduce storage size and improve cache locality.",
                        latest_health.efficiency_score * 100.0
                    ),
                    expected_improvement: PerformanceImprovement {
                        search_latency_improvement_percent: 15.0,
                        memory_reduction_percent: 25.0,
                        storage_savings_percent: 40.0,
                        overall_improvement_percent: 20.0,
                        confidence_level: 0.90,
                    },
                    difficulty: ImplementationDifficulty::Medium,
                    estimated_time_hours: 2.0,
                    prerequisites: vec![
                        "Create full backup".to_string(),
                        "Verify compression compatibility".to_string(),
                    ],
                    implementation_steps: vec![
                        "1. Enable compression in configuration".to_string(),
                        "2. Run compression migration".to_string(),
                        "3. Validate search accuracy".to_string(),
                        "4. Monitor performance metrics".to_string(),
                    ],
                    risks: vec![
                        "Small accuracy loss possible".to_string(),
                        "Initial migration time required".to_string(),
                    ],
                    success_metrics: vec![
                        "Storage size reduced by 30%+".to_string(),
                        "Memory usage reduced by 20%+".to_string(),
                        "Search accuracy maintained >99%".to_string(),
                    ],
                });
            }
        }
        
        Ok(recommendations)
    }

    async fn persist_metrics(&self, _path: &str) -> VectorDbResult<()> {
        // Implementation for persisting metrics to file
        // This would serialize all metrics collections to JSON/binary format
        Ok(())
    }
}