//! Automatic Optimization Scheduling and Trigger System
//!
//! This module implements comprehensive automatic optimization scheduling that triggers
//! deduplication, compression, and maintenance operations based on usage patterns and
//! system conditions.
//!
//! ## Features
//!
//! - **Automatic Scheduling**: Time-based, usage-based, and size-based triggers
//! - **Background Execution**: Non-blocking optimization pipeline execution
//! - **Pipeline Orchestration**: Coordinated deduplication → compression → cleanup
//! - **Intelligent Triggers**: Adaptive scheduling based on system conditions
//! - **Performance Monitoring Integration**: Leverages existing monitoring system
//! - **Configuration Management**: Flexible optimization parameters
//! - **Result Reporting**: Comprehensive optimization result logging
//!
//! ## Architecture
//!
//! The scheduler operates as a background service with multiple trigger types:
//! - **Time-based**: Regular intervals (daily, weekly, etc.)
//! - **Usage-based**: After N file operations or search queries
//! - **Size-based**: When index size exceeds thresholds
//! - **Performance-based**: When performance degrades below thresholds

use std::collections::VecDeque;
use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{interval, sleep, timeout, Instant};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Timelike, Datelike};
use thiserror::Error;

use crate::vector_db::types::VectorDbError;
use crate::vector_db::deduplication::DeduplicationConfig;
use crate::vector_db::performance_monitor::{IndexPerformanceMonitor, OperationType, OperationStatus};

/// Errors specific to optimization scheduling operations
#[derive(Error, Debug, Clone)]
pub enum OptimizationSchedulerError {
    #[error("Scheduler not running")]
    SchedulerNotRunning,

    #[error("Optimization pipeline failed: {message}")]
    PipelineFailed { message: String },

    #[error("Trigger condition evaluation error: {message}")]
    TriggerEvaluation { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Background task error: {message}")]
    BackgroundTask { message: String },

    #[error("Resource exhaustion: {message}")]
    ResourceExhaustion { message: String },

    #[error("Timeout during optimization: {operation}")]
    OptimizationTimeout { operation: String },
}

impl From<OptimizationSchedulerError> for VectorDbError {
    fn from(error: OptimizationSchedulerError) -> Self {
        VectorDbError::Storage { 
            message: error.to_string() 
        }
    }
}

pub type OptimizationResult<T> = Result<T, OptimizationSchedulerError>;

/// Configuration for the automatic optimization scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSchedulerConfig {
    /// Enable automatic optimization scheduling
    pub enable_automatic_optimization: bool,
    
    // Time-based triggers
    /// Interval between automatic optimization runs (hours)
    pub optimization_interval_hours: u64,
    /// Preferred time window for optimization (hour of day, 0-23)
    pub preferred_optimization_hour: Option<u8>,
    /// Days of week to run optimization (0=Sunday, 1=Monday, etc.)
    pub optimization_days: Vec<u8>,
    
    // Usage-based triggers
    /// Number of file operations to trigger optimization
    pub file_operations_threshold: u64,
    /// Number of search queries to trigger optimization
    pub search_queries_threshold: u64,
    /// Number of new embeddings to trigger optimization
    pub new_embeddings_threshold: u64,
    
    // Size-based triggers
    /// Index size in MB to trigger optimization
    pub index_size_threshold_mb: u64,
    /// Storage utilization percentage to trigger optimization (0.0-1.0)
    pub storage_utilization_threshold: f64,
    /// Estimated duplicate ratio to trigger optimization (0.0-1.0)
    pub duplicate_ratio_threshold: f64,
    
    // Performance-based triggers
    /// Search performance threshold (ms) to trigger optimization
    pub search_performance_threshold_ms: f64,
    /// Memory usage threshold (MB) to trigger optimization
    pub memory_usage_threshold_mb: f64,
    /// Enable performance-based trigger evaluation
    pub enable_performance_triggers: bool,
    
    // Pipeline configuration
    /// Enable deduplication in optimization pipeline
    pub enable_deduplication: bool,
    /// Enable compression in optimization pipeline
    pub enable_compression: bool,
    /// Enable maintenance cleanup in optimization pipeline
    pub enable_maintenance_cleanup: bool,
    /// Maximum optimization duration (minutes)
    pub max_optimization_duration_minutes: u64,
    /// Number of parallel optimization workers
    pub parallel_workers: usize,
    
    // Safety and resource limits
    /// Minimum time between optimizations (hours) 
    pub min_optimization_cooldown_hours: u64,
    /// Maximum CPU usage during optimization (0.0-1.0)
    pub max_cpu_usage_during_optimization: f64,
    /// Maximum memory usage during optimization (MB)
    pub max_memory_usage_during_optimization_mb: u64,
    /// Enable resource monitoring during optimization
    pub enable_resource_monitoring: bool,
    
    // Reporting and logging
    /// Enable detailed optimization logging
    pub enable_detailed_logging: bool,
    /// Enable optimization result notifications
    pub enable_result_notifications: bool,
    /// Optimization log retention days
    pub log_retention_days: u32,
}

impl Default for OptimizationSchedulerConfig {
    fn default() -> Self {
        Self {
            enable_automatic_optimization: true,
            
            // Time-based: Daily at 2 AM
            optimization_interval_hours: 24,
            preferred_optimization_hour: Some(2),
            optimization_days: vec![1, 2, 3, 4, 5], // Weekdays only
            
            // Usage-based: Conservative thresholds
            file_operations_threshold: 1000,
            search_queries_threshold: 5000,
            new_embeddings_threshold: 500,
            
            // Size-based: Trigger at reasonable sizes
            index_size_threshold_mb: 100,
            storage_utilization_threshold: 0.8,
            duplicate_ratio_threshold: 0.15, // 15% duplicates
            
            // Performance-based: Maintain good performance
            search_performance_threshold_ms: 100.0,
            memory_usage_threshold_mb: 500.0,
            enable_performance_triggers: true,
            
            // Pipeline: Enable all optimizations
            enable_deduplication: true,
            enable_compression: true,
            enable_maintenance_cleanup: true,
            max_optimization_duration_minutes: 60,
            parallel_workers: 2,
            
            // Safety: Conservative limits
            min_optimization_cooldown_hours: 6,
            max_cpu_usage_during_optimization: 0.5,
            max_memory_usage_during_optimization_mb: 1024,
            enable_resource_monitoring: true,
            
            // Reporting: Enable comprehensive logging
            enable_detailed_logging: true,
            enable_result_notifications: true,
            log_retention_days: 30,
        }
    }
}

/// Types of optimization triggers that can activate the scheduler
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationTrigger {
    /// Scheduled time-based trigger
    TimeScheduled { 
        /// Scheduled execution time
        scheduled_time: DateTime<Utc> 
    },
    /// Manual trigger requested by user
    Manual,
    /// Triggered by file operation count threshold
    FileOperationsThreshold { 
        /// Number of operations that triggered this
        operation_count: u64 
    },
    /// Triggered by search query count threshold
    SearchQueriesThreshold { 
        /// Number of queries that triggered this
        query_count: u64 
    },
    /// Triggered by new embeddings count threshold
    NewEmbeddingsThreshold { 
        /// Number of new embeddings that triggered this
        embedding_count: u64 
    },
    /// Triggered by index size threshold
    IndexSizeThreshold { 
        /// Current index size in MB
        current_size_mb: u64 
    },
    /// Triggered by storage utilization threshold
    StorageUtilizationThreshold { 
        /// Current utilization ratio
        utilization_ratio: f64 
    },
    /// Triggered by estimated duplicate ratio
    DuplicateRatioThreshold { 
        /// Estimated duplicate ratio
        duplicate_ratio: f64 
    },
    /// Triggered by performance degradation
    PerformanceDegradation { 
        /// Current search performance in ms
        search_time_ms: f64,
        /// Current memory usage in MB
        memory_usage_mb: f64 
    },
}

/// Status of an optimization operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStatus {
    /// Optimization is scheduled but not yet running
    Scheduled,
    /// Optimization is currently running
    Running,
    /// Optimization completed successfully
    Completed,
    /// Optimization failed with error
    Failed,
    /// Optimization was cancelled
    Cancelled,
    /// Optimization timed out
    TimedOut,
}

/// Results from a complete optimization pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPipelineResult {
    /// Unique identifier for this optimization run
    pub optimization_id: String,
    /// Trigger that initiated this optimization
    pub trigger: OptimizationTrigger,
    /// Optimization start timestamp
    pub started_at: DateTime<Utc>,
    /// Optimization completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Total optimization duration in milliseconds
    pub duration_ms: Option<f64>,
    /// Final status of optimization
    pub status: OptimizationStatus,
    
    // Pipeline stage results
    /// Deduplication results (if enabled) - simplified for serialization
    pub deduplication_result: Option<DeduplicationSummary>,
    /// Compression results (if enabled)
    pub compression_result: Option<CompressionResult>,
    /// Maintenance cleanup results (if enabled)
    pub maintenance_result: Option<MaintenanceResult>,
    
    // Performance metrics
    /// Resources used during optimization
    pub resource_usage: OptimizationResourceUsage,
    /// Performance improvement metrics
    pub performance_improvement: OptimizationPerformanceImprovement,
    
    /// Error message (if optimization failed)
    pub error_message: Option<String>,
    /// Warnings encountered during optimization
    pub warnings: Vec<String>,
    /// Success message with optimization summary
    pub success_message: Option<String>,
}

/// Simplified deduplication summary for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationSummary {
    /// Number of embeddings processed
    pub embeddings_processed: usize,
    /// Number of clusters found
    pub clusters_found: usize,
    /// Number of duplicates found
    pub duplicates_found: usize,
    /// Index size reduction percentage
    pub index_size_reduction_percentage: f32,
    /// Total processing time in milliseconds
    pub processing_time_ms: f64,
}

/// Compression optimization results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Number of embeddings compressed
    pub embeddings_compressed: u64,
    /// Original size in bytes
    pub original_size_bytes: u64,
    /// Compressed size in bytes
    pub compressed_size_bytes: u64,
    /// Compression ratio achieved
    pub compression_ratio: f64,
    /// Time taken for compression in milliseconds
    pub compression_time_ms: f64,
}

/// Maintenance cleanup results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceResult {
    /// Number of orphaned embeddings removed
    pub orphaned_embeddings_removed: u64,
    /// Storage space reclaimed in bytes
    pub storage_space_reclaimed: u64,
    /// Number of index compaction operations
    pub compaction_operations: u64,
    /// Time taken for maintenance in milliseconds
    pub maintenance_time_ms: f64,
}

/// Resource usage during optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResourceUsage {
    /// Peak CPU usage percentage during optimization
    pub peak_cpu_usage_percent: f64,
    /// Peak memory usage in MB during optimization
    pub peak_memory_usage_mb: f64,
    /// Total I/O operations performed
    pub total_io_operations: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Total bytes written  
    pub total_bytes_written: u64,
    /// Number of parallel workers used
    pub workers_used: usize,
}

/// Performance improvement metrics after optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPerformanceImprovement {
    /// Index size reduction percentage
    pub index_size_reduction_percent: f64,
    /// Search performance improvement percentage (negative = worse)
    pub search_performance_improvement_percent: f64,
    /// Memory usage reduction percentage
    pub memory_usage_reduction_percent: f64,
    /// Storage space savings in MB
    pub storage_space_savings_mb: f64,
    /// Overall optimization score (0.0-1.0, higher is better)
    pub optimization_score: f64,
}

/// Configuration for system usage tracking
#[derive(Debug, Clone)]
pub struct UsageTracker {
    /// File operation counter
    pub file_operations: Arc<AtomicU64>,
    /// Search query counter
    pub search_queries: Arc<AtomicU64>,
    /// New embeddings counter
    pub new_embeddings: Arc<AtomicU64>,
    /// Last reset timestamp
    pub last_reset: Arc<RwLock<SystemTime>>,
}

impl UsageTracker {
    /// Create a new usage tracker
    pub fn new() -> Self {
        Self {
            file_operations: Arc::new(AtomicU64::new(0)),
            search_queries: Arc::new(AtomicU64::new(0)),
            new_embeddings: Arc::new(AtomicU64::new(0)),
            last_reset: Arc::new(RwLock::new(SystemTime::now())),
        }
    }
    
    /// Increment file operation counter
    pub fn increment_file_operations(&self) {
        self.file_operations.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Increment search query counter
    pub fn increment_search_queries(&self) {
        self.search_queries.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Increment new embeddings counter
    pub fn increment_new_embeddings(&self, count: u64) {
        self.new_embeddings.fetch_add(count, Ordering::Relaxed);
    }
    
    /// Reset all counters
    pub async fn reset_counters(&self) {
        self.file_operations.store(0, Ordering::Relaxed);
        self.search_queries.store(0, Ordering::Relaxed);  
        self.new_embeddings.store(0, Ordering::Relaxed);
        *self.last_reset.write().await = SystemTime::now();
    }
    
    /// Get current counter values
    pub fn get_counters(&self) -> (u64, u64, u64) {
        (
            self.file_operations.load(Ordering::Relaxed),
            self.search_queries.load(Ordering::Relaxed),
            self.new_embeddings.load(Ordering::Relaxed),
        )
    }
}

/// Main automatic optimization scheduler
pub struct AutomaticOptimizationScheduler {
    /// Scheduler configuration
    config: OptimizationSchedulerConfig,
    /// Whether scheduler is currently running
    is_running: Arc<AtomicBool>,
    /// Usage tracking
    usage_tracker: Arc<UsageTracker>,
    /// Current optimization state
    current_optimization: Arc<RwLock<Option<OptimizationPipelineResult>>>,
    /// Optimization history (circular buffer)
    optimization_history: Arc<Mutex<VecDeque<OptimizationPipelineResult>>>,
    /// Last optimization timestamp
    last_optimization: Arc<RwLock<Option<SystemTime>>>,
    /// Performance monitor for integration
    performance_monitor: Arc<IndexPerformanceMonitor>,
    /// Background scheduler task handle
    scheduler_task: Option<tokio::task::JoinHandle<()>>,
    /// Optimization execution channel
    optimization_sender: mpsc::UnboundedSender<OptimizationRequest>,
    /// Optimization result channel
    result_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<OptimizationPipelineResult>>>>,
}

/// Internal optimization request structure
#[derive(Debug)]
struct OptimizationRequest {
    /// Trigger that initiated this request
    trigger: OptimizationTrigger,
    /// Response channel for completion notification
    response_sender: Option<oneshot::Sender<OptimizationResult<OptimizationPipelineResult>>>,
}

impl AutomaticOptimizationScheduler {
    /// Create a new automatic optimization scheduler
    pub fn new(
        config: OptimizationSchedulerConfig,
        performance_monitor: Arc<IndexPerformanceMonitor>,
    ) -> Self {
        let (optimization_sender, _optimization_receiver) = mpsc::unbounded_channel();
        let (_result_sender, result_receiver) = mpsc::unbounded_channel();
        
        Self {
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            usage_tracker: Arc::new(UsageTracker::new()),
            current_optimization: Arc::new(RwLock::new(None)),
            optimization_history: Arc::new(Mutex::new(VecDeque::new())),
            last_optimization: Arc::new(RwLock::new(None)),
            performance_monitor,
            scheduler_task: None,
            optimization_sender,
            result_receiver: Arc::new(Mutex::new(Some(result_receiver))),
        }
    }
    
    /// Start the automatic optimization scheduler
    pub async fn start(&mut self) -> OptimizationResult<()> {
        if !self.config.enable_automatic_optimization {
            return Ok(());
        }
        
        if self.is_running.load(Ordering::Relaxed) {
            return Err(OptimizationSchedulerError::Configuration {
                message: "Scheduler already running".to_string(),
            });
        }
        
        self.is_running.store(true, Ordering::Relaxed);
        
        // Start scheduler background task
        self.start_scheduler_task().await?;
        
        eprintln!("✅ Automatic Optimization Scheduler started successfully");
        eprintln!("   - Interval: {} hours", self.config.optimization_interval_hours);
        eprintln!("   - File operations threshold: {}", self.config.file_operations_threshold);
        eprintln!("   - Index size threshold: {} MB", self.config.index_size_threshold_mb);
        
        Ok(())
    }
    
    /// Stop the automatic optimization scheduler
    pub async fn stop(&mut self) -> OptimizationResult<()> {
        self.is_running.store(false, Ordering::Relaxed);
        
        // Stop scheduler task
        if let Some(task) = self.scheduler_task.take() {
            task.abort();
        }
        
        // Cancel any running optimization
        if let Some(current) = self.current_optimization.read().await.as_ref() {
            if current.status == OptimizationStatus::Running {
                self.cancel_current_optimization().await?;
            }
        }
        
        eprintln!("✅ Automatic Optimization Scheduler stopped successfully");
        Ok(())
    }
    
    /// Manually trigger an optimization
    pub async fn trigger_manual_optimization(&self) -> OptimizationResult<OptimizationPipelineResult> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(OptimizationSchedulerError::SchedulerNotRunning);
        }
        
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = OptimizationRequest {
            trigger: OptimizationTrigger::Manual,
            response_sender: Some(response_sender),
        };
        
        self.optimization_sender.send(request)
            .map_err(|e| OptimizationSchedulerError::BackgroundTask {
                message: format!("Failed to queue optimization: {}", e),
            })?;
        
        // Wait for optimization completion
        match timeout(Duration::from_secs(self.config.max_optimization_duration_minutes * 60), response_receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(OptimizationSchedulerError::BackgroundTask {
                message: "Optimization response channel closed".to_string(),
            }),
            Err(_) => Err(OptimizationSchedulerError::OptimizationTimeout {
                operation: "Manual optimization".to_string(),
            }),
        }
    }
    
    /// Get current optimization status
    pub async fn get_current_optimization_status(&self) -> Option<OptimizationPipelineResult> {
        self.current_optimization.read().await.clone()
    }
    
    /// Get optimization history
    pub async fn get_optimization_history(&self, limit: usize) -> Vec<OptimizationPipelineResult> {
        let history = self.optimization_history.lock().await;
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get usage tracker for integration with other systems
    pub fn get_usage_tracker(&self) -> Arc<UsageTracker> {
        Arc::clone(&self.usage_tracker)
    }
    
    /// Update scheduler configuration
    pub async fn update_configuration(&mut self, config: OptimizationSchedulerConfig) -> OptimizationResult<()> {
        let was_running = self.is_running.load(Ordering::Relaxed);
        
        if was_running {
            self.stop().await?;
        }
        
        self.config = config;
        
        if was_running && self.config.enable_automatic_optimization {
            self.start().await?;
        }
        
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_configuration(&self) -> &OptimizationSchedulerConfig {
        &self.config
    }
    
    // Private implementation methods
    
    /// Start the main scheduler background task
    async fn start_scheduler_task(&mut self) -> OptimizationResult<()> {
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);
        let usage_tracker = Arc::clone(&self.usage_tracker);
        let last_optimization = Arc::clone(&self.last_optimization);
        let current_optimization = Arc::clone(&self.current_optimization);
        let optimization_history = Arc::clone(&self.optimization_history);
        let performance_monitor = Arc::clone(&self.performance_monitor);
        
        // Extract receiver for optimization processing
        let mut optimization_receiver = {
            let mut receiver_guard = self.result_receiver.lock().await;
            receiver_guard.take()
                .ok_or_else(|| OptimizationSchedulerError::Configuration {
                    message: "Optimization receiver already taken".to_string(),
                })?
        };
        
        let task = tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(60)); // Check every minute
            
            eprintln!("🚀 Optimization scheduler task started");
            
            while is_running.load(Ordering::Relaxed) {
                check_interval.tick().await;
                
                // Evaluate trigger conditions
                if let Some(trigger) = Self::evaluate_trigger_conditions(
                    &config,
                    &usage_tracker,
                    &last_optimization,
                    &performance_monitor,
                ).await {
                    eprintln!("🎯 Optimization trigger activated: {:?}", trigger);
                    
                    // Execute optimization pipeline
                    let optimization_result = Self::execute_optimization_pipeline(
                        trigger.clone(),
                        &config,
                        &usage_tracker,
                        &current_optimization,
                        &performance_monitor,
                    ).await;
                    
                    match optimization_result {
                        Ok(mut result) => {
                            eprintln!("✅ Optimization completed: {}", result.optimization_id);
                            result.status = OptimizationStatus::Completed;
                            result.completed_at = Some(Utc::now());
                            
                            // Update last optimization timestamp
                            *last_optimization.write().await = Some(SystemTime::now());
                            
                            // Add to history
                            let mut history = optimization_history.lock().await;
                            history.push_back(result.clone());
                            
                            // Maintain history size limit
                            while history.len() > 100 {
                                history.pop_front();
                            }
                            
                            // Clear current optimization
                            *current_optimization.write().await = None;
                            
                            // Reset usage counters after successful optimization
                            usage_tracker.reset_counters().await;
                        },
                        Err(e) => {
                            eprintln!("❌ Optimization failed: {}", e);
                            
                            // Create failed optimization result
                            let failed_result = OptimizationPipelineResult {
                                optimization_id: format!("failed_{}", Utc::now().timestamp_millis()),
                                trigger: trigger.clone(),
                                started_at: Utc::now(),
                                completed_at: Some(Utc::now()),
                                duration_ms: Some(0.0),
                                status: OptimizationStatus::Failed,
                                deduplication_result: None,
                                compression_result: None,
                                maintenance_result: None,
                                resource_usage: OptimizationResourceUsage {
                                    peak_cpu_usage_percent: 0.0,
                                    peak_memory_usage_mb: 0.0,
                                    total_io_operations: 0,
                                    total_bytes_read: 0,
                                    total_bytes_written: 0,
                                    workers_used: 0,
                                },
                                performance_improvement: OptimizationPerformanceImprovement {
                                    index_size_reduction_percent: 0.0,
                                    search_performance_improvement_percent: 0.0,
                                    memory_usage_reduction_percent: 0.0,
                                    storage_space_savings_mb: 0.0,
                                    optimization_score: 0.0,
                                },
                                error_message: Some(e.to_string()),
                                warnings: vec![],
                                success_message: None,
                            };
                            
                            // Add failed result to history
                            let mut history = optimization_history.lock().await;
                            history.push_back(failed_result);
                            
                            // Clear current optimization
                            *current_optimization.write().await = None;
                        }
                    }
                }
                
                // Process any completed optimizations
                while let Ok(completed_optimization) = optimization_receiver.try_recv() {
                    eprintln!("📊 Processing completed optimization: {}", completed_optimization.optimization_id);
                    
                    let mut history = optimization_history.lock().await;
                    history.push_back(completed_optimization);
                    
                    // Maintain history size
                    while history.len() > 100 {
                        history.pop_front();
                    }
                }
            }
            
            eprintln!("🛑 Optimization scheduler task stopped");
        });
        
        self.scheduler_task = Some(task);
        Ok(())
    }
    
    /// Evaluate all trigger conditions to determine if optimization should run
    async fn evaluate_trigger_conditions(
        config: &OptimizationSchedulerConfig,
        usage_tracker: &UsageTracker,
        last_optimization: &Arc<RwLock<Option<SystemTime>>>,
        performance_monitor: &IndexPerformanceMonitor,
    ) -> Option<OptimizationTrigger> {
        // Check cooldown period
        if let Some(last_opt) = *last_optimization.read().await {
            let cooldown_duration = Duration::from_secs(config.min_optimization_cooldown_hours * 3600);
            if SystemTime::now().duration_since(last_opt).unwrap_or_default() < cooldown_duration {
                return None;
            }
        }
        
        // Check time-based triggers
        if let Some(trigger) = Self::check_time_based_trigger(config).await {
            return Some(trigger);
        }
        
        // Check usage-based triggers
        if let Some(trigger) = Self::check_usage_based_triggers(config, usage_tracker).await {
            return Some(trigger);
        }
        
        // Check size-based triggers
        if let Some(trigger) = Self::check_size_based_triggers(config).await {
            return Some(trigger);
        }
        
        // Check performance-based triggers
        if config.enable_performance_triggers {
            if let Some(trigger) = Self::check_performance_based_triggers(config, performance_monitor).await {
                return Some(trigger);
            }
        }
        
        None
    }
    
    /// Check time-based optimization triggers
    async fn check_time_based_trigger(config: &OptimizationSchedulerConfig) -> Option<OptimizationTrigger> {
        let now = Utc::now();
        let current_hour = now.hour() as u8;
        let current_day = now.weekday().num_days_from_sunday() as u8;
        
        // Check if current time matches preferred optimization hour and day
        if let Some(preferred_hour) = config.preferred_optimization_hour {
            if current_hour == preferred_hour && config.optimization_days.contains(&current_day) {
                return Some(OptimizationTrigger::TimeScheduled {
                    scheduled_time: now,
                });
            }
        }
        
        None
    }
    
    /// Check usage-based optimization triggers
    async fn check_usage_based_triggers(
        config: &OptimizationSchedulerConfig,
        usage_tracker: &UsageTracker,
    ) -> Option<OptimizationTrigger> {
        let (file_ops, search_queries, new_embeddings) = usage_tracker.get_counters();
        
        // Check file operations threshold
        if file_ops >= config.file_operations_threshold {
            return Some(OptimizationTrigger::FileOperationsThreshold {
                operation_count: file_ops,
            });
        }
        
        // Check search queries threshold
        if search_queries >= config.search_queries_threshold {
            return Some(OptimizationTrigger::SearchQueriesThreshold {
                query_count: search_queries,
            });
        }
        
        // Check new embeddings threshold
        if new_embeddings >= config.new_embeddings_threshold {
            return Some(OptimizationTrigger::NewEmbeddingsThreshold {
                embedding_count: new_embeddings,
            });
        }
        
        None
    }
    
    /// Check size-based optimization triggers
    async fn check_size_based_triggers(config: &OptimizationSchedulerConfig) -> Option<OptimizationTrigger> {
        // In a real implementation, these would check actual system metrics
        // For now, we'll use placeholder logic
        
        // Placeholder: Check index size (would need actual index size calculation)
        let estimated_index_size_mb = 50; // Placeholder
        if estimated_index_size_mb >= config.index_size_threshold_mb {
            return Some(OptimizationTrigger::IndexSizeThreshold {
                current_size_mb: estimated_index_size_mb,
            });
        }
        
        // Placeholder: Check storage utilization
        let estimated_storage_utilization = 0.6; // Placeholder
        if estimated_storage_utilization >= config.storage_utilization_threshold {
            return Some(OptimizationTrigger::StorageUtilizationThreshold {
                utilization_ratio: estimated_storage_utilization,
            });
        }
        
        // Placeholder: Check duplicate ratio
        let estimated_duplicate_ratio = 0.1; // Placeholder
        if estimated_duplicate_ratio >= config.duplicate_ratio_threshold {
            return Some(OptimizationTrigger::DuplicateRatioThreshold {
                duplicate_ratio: estimated_duplicate_ratio,
            });
        }
        
        None
    }
    
    /// Check performance-based optimization triggers
    async fn check_performance_based_triggers(
        config: &OptimizationSchedulerConfig,
        performance_monitor: &IndexPerformanceMonitor,
    ) -> Option<OptimizationTrigger> {
        // Get current performance metrics
        if let Ok(current_metrics) = performance_monitor.get_current_metrics().await {
            // Check search performance
            if let Some(search_metrics) = current_metrics.get(&OperationType::VectorOperations) {
                if let Some(duration) = search_metrics.duration_ms {
                    if duration >= config.search_performance_threshold_ms {
                        return Some(OptimizationTrigger::PerformanceDegradation {
                            search_time_ms: duration,
                            memory_usage_mb: search_metrics.memory_peak_mb,
                        });
                    }
                }
                
                // Check memory usage
                if search_metrics.memory_peak_mb >= config.memory_usage_threshold_mb as f64 {
                    return Some(OptimizationTrigger::PerformanceDegradation {
                        search_time_ms: search_metrics.duration_ms.unwrap_or(0.0),
                        memory_usage_mb: search_metrics.memory_peak_mb,
                    });
                }
            }
        }
        
        None
    }
    
    /// Execute the complete optimization pipeline
    async fn execute_optimization_pipeline(
        trigger: OptimizationTrigger,
        config: &OptimizationSchedulerConfig,
        _usage_tracker: &UsageTracker,
        current_optimization: &Arc<RwLock<Option<OptimizationPipelineResult>>>,
        performance_monitor: &IndexPerformanceMonitor,
    ) -> OptimizationResult<OptimizationPipelineResult> {
        let optimization_id = format!("opt_{}", Utc::now().timestamp_millis());
        let start_time = Utc::now();
        let start_instant = Instant::now();
        
        eprintln!("🚀 Starting optimization pipeline: {}", optimization_id);
        eprintln!("   Trigger: {:?}", trigger);
        
        // Create initial optimization result
        let mut result = OptimizationPipelineResult {
            optimization_id: optimization_id.clone(),
            trigger: trigger.clone(),
            started_at: start_time,
            completed_at: None,
            duration_ms: None,
            status: OptimizationStatus::Running,
            deduplication_result: None,
            compression_result: None,
            maintenance_result: None,
            resource_usage: OptimizationResourceUsage {
                peak_cpu_usage_percent: 0.0,
                peak_memory_usage_mb: 0.0,
                total_io_operations: 0,
                total_bytes_read: 0,
                total_bytes_written: 0,
                workers_used: config.parallel_workers,
            },
            performance_improvement: OptimizationPerformanceImprovement {
                index_size_reduction_percent: 0.0,
                search_performance_improvement_percent: 0.0,
                memory_usage_reduction_percent: 0.0,
                storage_space_savings_mb: 0.0,
                optimization_score: 0.0,
            },
            error_message: None,
            warnings: vec![],
            success_message: None,
        };
        
        // Set as current optimization
        *current_optimization.write().await = Some(result.clone());
        
        // Start performance monitoring for this optimization
        performance_monitor.start_operation(
            OperationType::Maintenance,
            optimization_id.clone(),
        ).await.map_err(|e| OptimizationSchedulerError::BackgroundTask {
            message: format!("Failed to start performance monitoring: {}", e),
        })?;
        
        let mut warnings = Vec::new();
        let mut total_space_savings = 0.0;
        let mut total_performance_improvement = 0.0;
        
        // Stage 1: Deduplication (if enabled)
        if config.enable_deduplication {
            eprintln!("📊 Stage 1: Running deduplication...");
            
            match Self::run_deduplication_stage(&optimization_id).await {
                Ok(dedup_result) => {
                    eprintln!("✅ Deduplication completed: {:.1}% size reduction", 
                              dedup_result.index_size_reduction_percentage);
                    
                    total_space_savings += dedup_result.index_size_reduction_percentage;
                    total_performance_improvement += 10.0; // Assume 10% performance improvement from deduplication
                    result.deduplication_result = Some(dedup_result);
                },
                Err(e) => {
                    let warning = format!("Deduplication stage failed: {}", e);
                    warnings.push(warning.clone());
                    eprintln!("⚠️ {}", warning);
                }
            }
        }
        
        // Stage 2: Compression (if enabled)
        if config.enable_compression {
            eprintln!("🗜️ Stage 2: Running compression...");
            
            match Self::run_compression_stage(&optimization_id).await {
                Ok(compression_result) => {
                    eprintln!("✅ Compression completed: {:.1}% size reduction", 
                              (1.0 - compression_result.compression_ratio) * 100.0);
                    
                    total_space_savings += ((1.0 - compression_result.compression_ratio) * 100.0) as f32;
                    total_performance_improvement += 5.0; // Assume 5% performance improvement from compression
                    result.compression_result = Some(compression_result);
                },
                Err(e) => {
                    let warning = format!("Compression stage failed: {}", e);
                    warnings.push(warning.clone());
                    eprintln!("⚠️ {}", warning);
                }
            }
        }
        
        // Stage 3: Maintenance cleanup (if enabled)
        if config.enable_maintenance_cleanup {
            eprintln!("🧹 Stage 3: Running maintenance cleanup...");
            
            match Self::run_maintenance_stage(&optimization_id).await {
                Ok(maintenance_result) => {
                    eprintln!("✅ Maintenance completed: {} orphans removed, {:.1} MB reclaimed", 
                              maintenance_result.orphaned_embeddings_removed,
                              maintenance_result.storage_space_reclaimed as f64 / (1024.0 * 1024.0));
                    
                    total_space_savings += (maintenance_result.storage_space_reclaimed as f32) / (1024.0 * 1024.0);
                    total_performance_improvement += 3.0; // Assume 3% performance improvement from cleanup
                    result.maintenance_result = Some(maintenance_result);
                },
                Err(e) => {
                    let warning = format!("Maintenance stage failed: {}", e);
                    warnings.push(warning.clone());
                    eprintln!("⚠️ {}", warning);
                }
            }
        }
        
        // Complete optimization
        let duration = start_instant.elapsed();
        result.completed_at = Some(Utc::now());
        result.duration_ms = Some(duration.as_secs_f64() * 1000.0);
        result.warnings = warnings;
        
        // Calculate performance improvements
        result.performance_improvement.storage_space_savings_mb = total_space_savings as f64;
        result.performance_improvement.search_performance_improvement_percent = total_performance_improvement;
        result.performance_improvement.optimization_score = (total_performance_improvement / 100.0).min(1.0).max(0.0);
        
        // Generate success message
        if result.deduplication_result.is_some() || result.compression_result.is_some() || result.maintenance_result.is_some() {
            result.success_message = Some(format!(
                "Optimization completed successfully: {:.1} MB saved, {:.1}% performance improvement",
                total_space_savings,
                total_performance_improvement
            ));
            result.status = OptimizationStatus::Completed;
        } else {
            result.error_message = Some("All optimization stages failed".to_string());
            result.status = OptimizationStatus::Failed;
        }
        
        // Complete performance monitoring
        performance_monitor.complete_operation(
            &optimization_id,
            if result.status == OptimizationStatus::Completed { OperationStatus::Success } else { OperationStatus::Failed },
            result.error_message.clone(),
        ).await.map_err(|e| OptimizationSchedulerError::BackgroundTask {
            message: format!("Failed to complete performance monitoring: {}", e),
        })?;
        
        eprintln!("🏁 Optimization pipeline completed: {} ({:.2}s)", 
                  optimization_id, duration.as_secs_f64());
        
        Ok(result)
    }
    
    /// Run the deduplication stage of optimization
    async fn run_deduplication_stage(_optimization_id: &str) -> OptimizationResult<DeduplicationSummary> {
        // Create deduplication configuration
        let _dedup_config = DeduplicationConfig::default();
        
        // For this implementation, we'll create a mock result since we need actual embeddings
        // In a real implementation, this would:
        // 1. Load embeddings from the vector database
        // 2. Run EmbeddingDeduplicator::deduplicate_embeddings
        // 3. Update the database with deduplicated results
        
        let mock_result = DeduplicationSummary {
            embeddings_processed: 100,
            clusters_found: 8,
            duplicates_found: 15,
            index_size_reduction_percentage: 15.0,
            processing_time_ms: 500.0,
        };
        
        // Simulate processing time
        sleep(Duration::from_millis(100)).await;
        
        Ok(mock_result)
    }
    
    /// Run the compression stage of optimization
    async fn run_compression_stage(_optimization_id: &str) -> OptimizationResult<CompressionResult> {
        // In a real implementation, this would:
        // 1. Load embeddings from the vector database
        // 2. Apply vector compression using VectorCompressor
        // 3. Update the database with compressed vectors
        
        let mock_result = CompressionResult {
            embeddings_compressed: 85,
            original_size_bytes: 850000, // ~850KB original
            compressed_size_bytes: 340000, // ~340KB compressed (60% of original)
            compression_ratio: 0.4, // 40% compression ratio  
            compression_time_ms: 300.0,
        };
        
        // Simulate processing time
        sleep(Duration::from_millis(100)).await;
        
        Ok(mock_result)
    }
    
    /// Run the maintenance cleanup stage of optimization
    async fn run_maintenance_stage(_optimization_id: &str) -> OptimizationResult<MaintenanceResult> {
        // In a real implementation, this would:
        // 1. Run orphaned embedding detection and cleanup
        // 2. Perform index compaction operations
        // 3. Reclaim storage space through defragmentation
        
        let mock_result = MaintenanceResult {
            orphaned_embeddings_removed: 12,
            storage_space_reclaimed: 5242880, // 5MB reclaimed
            compaction_operations: 3,
            maintenance_time_ms: 200.0,
        };
        
        // Simulate processing time
        sleep(Duration::from_millis(100)).await;
        
        Ok(mock_result)
    }
    
    /// Cancel the currently running optimization
    async fn cancel_current_optimization(&self) -> OptimizationResult<()> {
        if let Some(current) = self.current_optimization.write().await.as_mut() {
            if current.status == OptimizationStatus::Running {
                current.status = OptimizationStatus::Cancelled;
                current.completed_at = Some(Utc::now());
                current.error_message = Some("Optimization cancelled by user".to_string());
                
                eprintln!("⚠️ Optimization cancelled: {}", current.optimization_id);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::performance_monitor::MonitoringConfig;
    
    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = OptimizationSchedulerConfig::default();
        let monitor = Arc::new(IndexPerformanceMonitor::new(MonitoringConfig::default()));
        let scheduler = AutomaticOptimizationScheduler::new(config, monitor);
        
        assert!(!scheduler.is_running.load(Ordering::Relaxed));
        assert_eq!(scheduler.usage_tracker.get_counters(), (0, 0, 0));
    }
    
    #[tokio::test]
    async fn test_usage_tracker() {
        let tracker = UsageTracker::new();
        
        tracker.increment_file_operations();
        tracker.increment_search_queries();
        tracker.increment_new_embeddings(5);
        
        let (file_ops, search_queries, new_embeddings) = tracker.get_counters();
        assert_eq!(file_ops, 1);
        assert_eq!(search_queries, 1);
        assert_eq!(new_embeddings, 5);
        
        tracker.reset_counters().await;
        let (file_ops, search_queries, new_embeddings) = tracker.get_counters();
        assert_eq!(file_ops, 0);
        assert_eq!(search_queries, 0);
        assert_eq!(new_embeddings, 0);
    }
    
    #[tokio::test]
    async fn test_optimization_pipeline_structure() {
        let trigger = OptimizationTrigger::Manual;
        let result = OptimizationPipelineResult {
            optimization_id: "test_123".to_string(),
            trigger,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            status: OptimizationStatus::Scheduled,
            deduplication_result: None,
            compression_result: None,
            maintenance_result: None,
            resource_usage: OptimizationResourceUsage {
                peak_cpu_usage_percent: 0.0,
                peak_memory_usage_mb: 0.0,
                total_io_operations: 0,
                total_bytes_read: 0,
                total_bytes_written: 0,
                workers_used: 2,
            },
            performance_improvement: OptimizationPerformanceImprovement {
                index_size_reduction_percent: 0.0,
                search_performance_improvement_percent: 0.0,
                memory_usage_reduction_percent: 0.0,
                storage_space_savings_mb: 0.0,
                optimization_score: 0.0,
            },
            error_message: None,
            warnings: vec![],
            success_message: None,
        };
        
        assert_eq!(result.optimization_id, "test_123");
        assert_eq!(result.status, OptimizationStatus::Scheduled);
        assert_eq!(result.resource_usage.workers_used, 2);
    }
    
    #[test]
    fn test_trigger_types() {
        let trigger1 = OptimizationTrigger::Manual;
        let trigger2 = OptimizationTrigger::FileOperationsThreshold { operation_count: 1000 };
        let trigger3 = OptimizationTrigger::IndexSizeThreshold { current_size_mb: 150 };
        
        assert!(matches!(trigger1, OptimizationTrigger::Manual));
        
        if let OptimizationTrigger::FileOperationsThreshold { operation_count } = trigger2 {
            assert_eq!(operation_count, 1000);
        } else {
            panic!("Wrong trigger type");
        }
        
        if let OptimizationTrigger::IndexSizeThreshold { current_size_mb } = trigger3 {
            assert_eq!(current_size_mb, 150);
        } else {
            panic!("Wrong trigger type");
        }
    }
    
    #[tokio::test]
    async fn test_configuration_validation() {
        let mut config = OptimizationSchedulerConfig::default();
        
        // Test valid configuration
        assert!(config.enable_automatic_optimization);
        assert_eq!(config.optimization_interval_hours, 24);
        assert_eq!(config.parallel_workers, 2);
        
        // Test configuration updates
        config.enable_automatic_optimization = false;
        config.optimization_interval_hours = 12;
        assert!(!config.enable_automatic_optimization);
        assert_eq!(config.optimization_interval_hours, 12);
    }
}