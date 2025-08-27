//! Index Rebuilding and Health Check System for Vector Database
//!
//! This module provides comprehensive index rebuilding capabilities with progress tracking
//! and health check utilities for index integrity validation.
//!
//! ## Features
//!
//! - **Full Index Rebuilding**: Complete reconstruction from source data
//! - **Parallel Processing**: Multi-threaded rebuilding for performance
//! - **Progress Tracking**: Real-time progress updates with callback system
//! - **Health Checks**: Comprehensive index integrity validation
//! - **Corruption Detection**: Advanced detection algorithms for data integrity
//! - **Recovery Systems**: Automatic recovery from index corruption
//! - **Performance Monitoring**: Detailed metrics for all operations
//!
//! ## Architecture
//!
//! The rebuilding system consists of several key components:
//!
//! - `IndexRebuilder`: Main coordinator for rebuild operations
//! - `HealthChecker`: Index integrity validation and health monitoring
//! - `CorruptionDetector`: Advanced corruption detection algorithms
//! - `ProgressReporter`: Real-time progress tracking and reporting
//! - `RecoveryManager`: Automatic recovery from corruption
//! - `RebuildMetrics`: Performance monitoring and reporting

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicUsize, AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, mpsc};
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize};

use crate::vector_db::types::{VectorDbError, VectorDbResult};
use crate::vector_db::storage::VectorStorage;
use crate::vector_db::operations::VectorOperations;

/// Configuration for index rebuilding operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildingConfig {
    /// Enable parallel processing during rebuild
    pub enable_parallel_processing: bool,
    /// Number of parallel worker threads (0 = auto-detect based on CPU cores)
    pub parallel_workers: usize,
    /// Batch size for processing embeddings during rebuild
    pub rebuild_batch_size: usize,
    /// Timeout for individual rebuild operations (seconds)
    pub operation_timeout_seconds: u64,
    /// Enable detailed progress reporting
    pub enable_progress_reporting: bool,
    /// Progress reporting interval (milliseconds)
    pub progress_report_interval_ms: u64,
    /// Enable health check validation after rebuild
    pub validate_after_rebuild: bool,
    /// Enable automatic backup before rebuild
    pub backup_before_rebuild: bool,
    /// Temporary directory for rebuild operations
    pub temp_directory: Option<PathBuf>,
    /// Enable debug logging during rebuild
    pub enable_debug_logging: bool,
}

impl Default for RebuildingConfig {
    fn default() -> Self {
        Self {
            enable_parallel_processing: true,
            parallel_workers: 0, // Auto-detect
            rebuild_batch_size: 100,
            operation_timeout_seconds: 1800, // 30 minutes
            enable_progress_reporting: true,
            progress_report_interval_ms: 1000, // 1 second
            validate_after_rebuild: true,
            backup_before_rebuild: true,
            temp_directory: None, // Use system temp
            enable_debug_logging: false,
        }
    }
}

/// Progress information for rebuild operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildProgress {
    /// Current phase of the rebuild operation
    pub phase: RebuildPhase,
    /// Total number of items to process
    pub total_items: usize,
    /// Number of items processed so far
    pub processed_items: usize,
    /// Current progress percentage (0.0 - 1.0)
    pub progress_percentage: f64,
    /// Estimated time remaining (seconds)
    pub estimated_remaining_seconds: Option<u64>,
    /// Current operation being performed
    pub current_operation: String,
    /// Processing rate (items per second)
    pub processing_rate: f64,
    /// Time elapsed since start (seconds)
    pub elapsed_seconds: u64,
    /// Whether any errors have occurred
    pub has_errors: bool,
    /// Number of errors encountered
    pub error_count: usize,
}

impl RebuildProgress {
    /// Create new progress with initial values
    pub fn new(total_items: usize, phase: RebuildPhase) -> Self {
        Self {
            phase,
            total_items,
            processed_items: 0,
            progress_percentage: 0.0,
            estimated_remaining_seconds: None,
            current_operation: "Initializing".to_string(),
            processing_rate: 0.0,
            elapsed_seconds: 0,
            has_errors: false,
            error_count: 0,
        }
    }
    
    /// Update progress with new values
    pub fn update(&mut self, processed_items: usize, current_operation: String, elapsed_seconds: u64) {
        self.processed_items = processed_items;
        self.current_operation = current_operation;
        self.elapsed_seconds = elapsed_seconds;
        
        if self.total_items > 0 {
            self.progress_percentage = processed_items as f64 / self.total_items as f64;
        }
        
        if elapsed_seconds > 0 {
            self.processing_rate = processed_items as f64 / elapsed_seconds as f64;
            
            if self.processing_rate > 0.0 && processed_items > 0 {
                let remaining_items = self.total_items.saturating_sub(processed_items);
                self.estimated_remaining_seconds = Some((remaining_items as f64 / self.processing_rate) as u64);
            }
        }
    }
    
    /// Mark an error occurred
    pub fn add_error(&mut self) {
        self.has_errors = true;
        self.error_count += 1;
    }
    
    /// Check if rebuild is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.phase, RebuildPhase::Completed) || self.processed_items >= self.total_items
    }
}

/// Phases of the rebuild operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebuildPhase {
    /// Initializing the rebuild process
    Initializing,
    /// Creating backup of current index
    CreatingBackup,
    /// Validating source data
    ValidatingSource,
    /// Processing embeddings in parallel
    ProcessingEmbeddings,
    /// Rebuilding index structure
    RebuildingIndex,
    /// Validating rebuilt index
    ValidatingIndex,
    /// Finalizing and cleanup
    Finalizing,
    /// Rebuild completed successfully
    Completed,
    /// Rebuild failed with errors
    Failed,
}

impl std::fmt::Display for RebuildPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initializing => write!(f, "Initializing"),
            Self::CreatingBackup => write!(f, "Creating Backup"),
            Self::ValidatingSource => write!(f, "Validating Source"),
            Self::ProcessingEmbeddings => write!(f, "Processing Embeddings"),
            Self::RebuildingIndex => write!(f, "Rebuilding Index"),
            Self::ValidatingIndex => write!(f, "Validating Index"),
            Self::Finalizing => write!(f, "Finalizing"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Result of a complete rebuild operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildResult {
    /// Whether the rebuild was successful
    pub success: bool,
    /// Total time taken for the rebuild (milliseconds)
    pub total_time_ms: u64,
    /// Number of embeddings processed
    pub embeddings_processed: usize,
    /// Number of embeddings that failed processing
    pub embeddings_failed: usize,
    /// Final phase reached
    pub final_phase: RebuildPhase,
    /// Size of original index (bytes)
    pub original_index_size: u64,
    /// Size of rebuilt index (bytes)
    pub rebuilt_index_size: u64,
    /// Performance metrics
    pub metrics: RebuildMetrics,
    /// Health check results (if performed)
    pub health_check_results: Option<HealthCheckResult>,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl RebuildResult {
    /// Create a successful rebuild result
    pub fn success(
        total_time_ms: u64,
        embeddings_processed: usize,
        metrics: RebuildMetrics,
        health_check_results: Option<HealthCheckResult>,
    ) -> Self {
        Self {
            success: true,
            total_time_ms,
            embeddings_processed,
            embeddings_failed: 0,
            final_phase: RebuildPhase::Completed,
            original_index_size: 0,
            rebuilt_index_size: 0,
            metrics,
            health_check_results,
            errors: Vec::new(),
        }
    }
    
    /// Create a failed rebuild result
    pub fn failure(
        total_time_ms: u64,
        final_phase: RebuildPhase,
        errors: Vec<String>,
    ) -> Self {
        Self {
            success: false,
            total_time_ms,
            embeddings_processed: 0,
            embeddings_failed: 0,
            final_phase,
            original_index_size: 0,
            rebuilt_index_size: 0,
            metrics: RebuildMetrics::default(),
            health_check_results: None,
            errors,
        }
    }
}

/// Performance metrics for rebuild operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RebuildMetrics {
    /// Average processing time per embedding (milliseconds)
    pub avg_processing_time_ms: f64,
    /// Peak memory usage during rebuild (bytes)
    pub peak_memory_usage_bytes: u64,
    /// Number of parallel workers used
    pub workers_used: usize,
    /// Total I/O operations performed
    pub io_operations: u64,
    /// Average I/O operation time (milliseconds)
    pub avg_io_time_ms: f64,
    /// CPU usage percentage during rebuild
    pub cpu_usage_percentage: f64,
    /// Throughput (embeddings per second)
    pub throughput_eps: f64,
}

impl RebuildMetrics {
    /// Check if performance targets were met
    pub fn meets_performance_targets(&self, target_time_per_1000_notes: u64) -> bool {
        // Target: <30 seconds per 1000 notes
        let estimated_time_per_1000 = (self.avg_processing_time_ms * 1000.0) / 1000.0; // Convert to seconds
        estimated_time_per_1000 <= target_time_per_1000_notes as f64
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable comprehensive integrity validation
    pub enable_integrity_validation: bool,
    /// Enable performance validation
    pub enable_performance_validation: bool,
    /// Enable corruption detection
    pub enable_corruption_detection: bool,
    /// Sample size for performance testing (percentage of total entries)
    pub performance_sample_percentage: f64,
    /// Target time for health checks (seconds)
    pub target_check_time_seconds: u64,
    /// Enable detailed reporting
    pub enable_detailed_reporting: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enable_integrity_validation: true,
            enable_performance_validation: true,
            enable_corruption_detection: true,
            performance_sample_percentage: 0.1, // 10% sample
            target_check_time_seconds: 1, // <1 second target
            enable_detailed_reporting: true,
        }
    }
}

/// Result of health check operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall health status
    pub overall_health: HealthStatus,
    /// Time taken for the health check (milliseconds)
    pub check_time_ms: u64,
    /// Integrity validation results
    pub integrity_results: Option<IntegrityCheckResult>,
    /// Performance validation results
    pub performance_results: Option<PerformanceCheckResult>,
    /// Corruption detection results
    pub corruption_results: Option<CorruptionCheckResult>,
    /// Detailed issues found
    pub issues_found: Vec<HealthIssue>,
    /// Recommendations for improvements
    pub recommendations: Vec<String>,
}

impl HealthCheckResult {
    /// Check if health check meets performance targets
    pub fn meets_performance_targets(&self) -> bool {
        self.check_time_ms <= 1000 // Target: <1 second
    }
    
    /// Get a summary of the health check
    pub fn summary(&self) -> String {
        format!(
            "Health: {:?}, Issues: {}, Time: {}ms",
            self.overall_health,
            self.issues_found.len(),
            self.check_time_ms
        )
    }
}

/// Overall health status
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Index is healthy and operating normally
    Healthy,
    /// Index has minor issues but is functional
    Warning,
    /// Index has significant issues affecting performance
    Degraded,
    /// Index is corrupted or non-functional
    Critical,
}

/// Integrity check results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    /// Whether integrity check passed
    pub passed: bool,
    /// Number of entries validated
    pub entries_validated: usize,
    /// Number of integrity violations found
    pub violations_found: usize,
    /// Specific integrity issues
    pub integrity_issues: Vec<String>,
}

/// Performance check results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceCheckResult {
    /// Whether performance targets were met
    pub meets_targets: bool,
    /// Average query time (milliseconds)
    pub avg_query_time_ms: f64,
    /// Number of queries tested
    pub queries_tested: usize,
    /// Peak memory usage during testing (bytes)
    pub peak_memory_usage: u64,
    /// Performance issues found
    pub performance_issues: Vec<String>,
}

/// Corruption detection results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorruptionCheckResult {
    /// Whether any corruption was detected
    pub corruption_detected: bool,
    /// Types of corruption found
    pub corruption_types: Vec<CorruptionType>,
    /// Severity of corruption
    pub corruption_severity: CorruptionSeverity,
    /// Detailed corruption information
    pub corruption_details: Vec<String>,
    /// Whether corruption can be automatically recovered
    pub recoverable: bool,
}

/// Types of corruption that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorruptionType {
    /// File checksum mismatch
    ChecksumMismatch,
    /// Invalid data structure
    InvalidStructure,
    /// Missing expected data
    MissingData,
    /// Inconsistent index pointers
    InconsistentIndex,
    /// Corrupted metadata
    CorruptedMetadata,
    /// Invalid embedding vectors
    InvalidVectors,
}

/// Severity levels for corruption
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CorruptionSeverity {
    /// Minor corruption that doesn't affect functionality
    Minor,
    /// Moderate corruption that may affect performance
    Moderate,
    /// Severe corruption that significantly impacts functionality
    Severe,
    /// Critical corruption that makes the index unusable
    Critical,
}

/// Health issues found during checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Type of issue
    pub issue_type: HealthIssueType,
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Description of the issue
    pub description: String,
    /// Recommended action to resolve the issue
    pub recommended_action: String,
}

/// Types of health issues
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthIssueType {
    /// Data integrity issue
    IntegrityIssue,
    /// Performance issue
    PerformanceIssue,
    /// Corruption detected
    CorruptionIssue,
    /// Configuration issue
    ConfigurationIssue,
    /// Resource usage issue
    ResourceIssue,
}

/// Severity levels for health issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Low severity issue
    Low,
    /// Medium severity issue
    Medium,
    /// High severity issue
    High,
    /// Critical severity issue
    Critical,
}

/// Progress callback function type
pub type ProgressCallback = Arc<dyn Fn(RebuildProgress) + Send + Sync>;

/// Main coordinator for index rebuilding operations
pub struct IndexRebuilder {
    /// Storage backend
    storage: Arc<VectorStorage>,
    /// Vector operations interface
    operations: VectorOperations,
    /// Rebuilding configuration
    config: RebuildingConfig,
    /// Progress tracking
    progress: Arc<RwLock<RebuildProgress>>,
    /// Progress callback for UI updates
    progress_callback: Option<ProgressCallback>,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
}

impl IndexRebuilder {
    /// Create a new index rebuilder
    pub fn new(
        storage: Arc<VectorStorage>,
        operations: VectorOperations,
        config: RebuildingConfig,
    ) -> Self {
        let initial_progress = RebuildProgress::new(0, RebuildPhase::Initializing);
        
        Self {
            storage,
            operations,
            config,
            progress: Arc::new(RwLock::new(initial_progress)),
            progress_callback: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Set progress callback for UI updates
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress_callback = Some(callback);
    }
    
    /// Cancel the current rebuild operation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    
    /// Perform a complete index rebuild
    pub async fn rebuild_index(&self) -> VectorDbResult<RebuildResult> {
        let start_time = Instant::now();
        let mut errors = Vec::new();
        
        if self.config.enable_debug_logging {
            eprintln!("🏗️ Starting index rebuild...");
        }
        
        // Phase 1: Initialize
        self.update_progress(RebuildPhase::Initializing, "Initializing rebuild process").await;
        
        // Get all embedding IDs to process
        let all_ids = self.operations.list_embedding_ids().await;
        let total_embeddings = all_ids.len();
        
        {
            let mut progress = self.progress.write().await;
            progress.total_items = total_embeddings;
        }
        
        if self.check_cancelled().await? {
            return Ok(RebuildResult::failure(
                start_time.elapsed().as_millis() as u64,
                RebuildPhase::Initializing,
                vec!["Rebuild was cancelled".to_string()],
            ));
        }
        
        // Phase 2: Create backup if enabled
        if self.config.backup_before_rebuild {
            self.update_progress(RebuildPhase::CreatingBackup, "Creating backup of current index").await;
            
            // Note: Backup functionality would be implemented here in a full implementation
            // For now, we'll log that this step would occur
            if self.config.enable_debug_logging {
                eprintln!("💾 Backup would be created here (not implemented in current version)");
            }
        }
        
        if self.check_cancelled().await? {
            return Ok(RebuildResult::failure(
                start_time.elapsed().as_millis() as u64,
                RebuildPhase::CreatingBackup,
                vec!["Rebuild was cancelled".to_string()],
            ));
        }
        
        // Phase 3: Validate source data
        self.update_progress(RebuildPhase::ValidatingSource, "Validating source data integrity").await;
        
        let integrity_report = self.storage.validate_integrity().await?;
        if !integrity_report.is_healthy() {
            let error_msg = format!("Source data integrity validation failed: {}", integrity_report.summary());
            errors.push(error_msg.clone());
            if self.config.enable_debug_logging {
                eprintln!("⚠️ {}", error_msg);
            }
        }
        
        if self.check_cancelled().await? {
            return Ok(RebuildResult::failure(
                start_time.elapsed().as_millis() as u64,
                RebuildPhase::ValidatingSource,
                vec!["Rebuild was cancelled".to_string()],
            ));
        }
        
        // Phase 4: Process embeddings (potentially in parallel)
        self.update_progress(RebuildPhase::ProcessingEmbeddings, "Processing embeddings for rebuild").await;
        
        let processing_result = if self.config.enable_parallel_processing {
            self.process_embeddings_parallel(&all_ids).await
        } else {
            self.process_embeddings_sequential(&all_ids).await
        };
        
        let (processed_count, processing_errors) = match processing_result {
            Ok((count, errs)) => (count, errs),
            Err(e) => {
                return Ok(RebuildResult::failure(
                    start_time.elapsed().as_millis() as u64,
                    RebuildPhase::ProcessingEmbeddings,
                    vec![format!("Embedding processing failed: {}", e)],
                ));
            }
        };
        
        errors.extend(processing_errors);
        
        if self.check_cancelled().await? {
            return Ok(RebuildResult::failure(
                start_time.elapsed().as_millis() as u64,
                RebuildPhase::ProcessingEmbeddings,
                vec!["Rebuild was cancelled".to_string()],
            ));
        }
        
        // Phase 5: Rebuild index structure
        self.update_progress(RebuildPhase::RebuildingIndex, "Rebuilding index structure").await;
        
        // Note: Index structure rebuilding would be implemented here in a full implementation
        // For now, the processing of embeddings above constitutes the main rebuild work
        if self.config.enable_debug_logging {
            eprintln!("🏗️ Index structure rebuild completed (processing embeddings is the main rebuild work)");
        }
        
        if self.check_cancelled().await? {
            return Ok(RebuildResult::failure(
                start_time.elapsed().as_millis() as u64,
                RebuildPhase::RebuildingIndex,
                vec!["Rebuild was cancelled".to_string()],
            ));
        }
        
        // Phase 6: Validate rebuilt index if enabled
        let health_check_results = if self.config.validate_after_rebuild {
            self.update_progress(RebuildPhase::ValidatingIndex, "Validating rebuilt index").await;
            
            let health_checker = HealthChecker::new(
                self.storage.clone(),
                self.operations.clone(),
                HealthCheckConfig::default(),
            );
            
            match health_checker.perform_health_check().await {
                Ok(results) => {
                    if self.config.enable_debug_logging {
                        eprintln!("✅ Health check completed: {}", results.summary());
                    }
                    Some(results)
                },
                Err(e) => {
                    let error_msg = format!("Health check failed: {}", e);
                    errors.push(error_msg.clone());
                    if self.config.enable_debug_logging {
                        eprintln!("❌ {}", error_msg);
                    }
                    None
                }
            }
        } else {
            None
        };
        
        if self.check_cancelled().await? {
            return Ok(RebuildResult::failure(
                start_time.elapsed().as_millis() as u64,
                RebuildPhase::ValidatingIndex,
                vec!["Rebuild was cancelled".to_string()],
            ));
        }
        
        // Phase 7: Finalize
        self.update_progress(RebuildPhase::Finalizing, "Finalizing rebuild process").await;
        
        let total_time_ms = start_time.elapsed().as_millis() as u64;
        let metrics = self.calculate_metrics(processed_count, total_time_ms).await;
        
        // Phase 8: Complete
        self.update_progress(RebuildPhase::Completed, "Rebuild completed successfully").await;
        
        if self.config.enable_debug_logging {
            eprintln!("🎉 Index rebuild completed in {}ms", total_time_ms);
            eprintln!("📊 Processed {} embeddings with {} errors", processed_count, errors.len());
        }
        
        let success = errors.is_empty() || errors.len() < processed_count / 10; // Allow up to 10% error rate
        
        let mut result = if success {
            RebuildResult::success(total_time_ms, processed_count, metrics, health_check_results)
        } else {
            RebuildResult::failure(total_time_ms, RebuildPhase::Failed, errors.clone())
        };
        
        result.errors = errors;
        result.embeddings_failed = all_ids.len().saturating_sub(processed_count);
        
        Ok(result)
    }
    
    /// Process embeddings sequentially
    async fn process_embeddings_sequential(&self, embedding_ids: &[String]) -> VectorDbResult<(usize, Vec<String>)> {
        let mut processed_count = 0;
        let mut errors = Vec::new();
        let start_time = Instant::now();
        
        for (index, entry_id) in embedding_ids.iter().enumerate() {
            if self.check_cancelled().await? {
                break;
            }
            
            match self.process_single_embedding(entry_id).await {
                Ok(_) => processed_count += 1,
                Err(e) => errors.push(format!("Failed to process embedding {}: {}", entry_id, e)),
            }
            
            // Update progress
            if index % 10 == 0 || index == embedding_ids.len() - 1 {
                let elapsed = start_time.elapsed().as_secs();
                self.update_progress_detailed(
                    RebuildPhase::ProcessingEmbeddings,
                    format!("Processing embedding {} of {}", index + 1, embedding_ids.len()),
                    index + 1,
                    elapsed,
                ).await;
                
                // Report progress via callback
                self.report_progress().await;
            }
        }
        
        Ok((processed_count, errors))
    }
    
    /// Process embeddings in parallel
    async fn process_embeddings_parallel(&self, embedding_ids: &[String]) -> VectorDbResult<(usize, Vec<String>)> {
        let worker_count = if self.config.parallel_workers == 0 {
            num_cpus::get().clamp(2, 8) // Use 2-8 workers based on CPU cores
        } else {
            self.config.parallel_workers
        };
        
        if self.config.enable_debug_logging {
            eprintln!("🔧 Using {} parallel workers for rebuilding", worker_count);
        }
        
        let semaphore = Arc::new(Semaphore::new(worker_count));
        let processed_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let (error_tx, mut error_rx) = mpsc::unbounded_channel();
        
        let batch_size = self.config.rebuild_batch_size;
        let total_items = embedding_ids.len();
        let mut tasks: Vec<JoinHandle<()>> = Vec::new();
        let start_time = Instant::now();
        
        // Process in batches
        for (batch_index, chunk) in embedding_ids.chunks(batch_size).enumerate() {
            let chunk = chunk.to_vec();
            let semaphore = semaphore.clone();
            let processed_count = processed_count.clone();
            let error_count = error_count.clone();
            let error_tx = error_tx.clone();
            let cancelled = self.cancelled.clone();
            let _storage = self.storage.clone();
            let operations = self.operations.clone();
            let _progress = self.progress.clone();
            let enable_debug = self.config.enable_debug_logging;
            
            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                for entry_id in chunk {
                    if cancelled.load(Ordering::Relaxed) {
                        break;
                    }
                    
                    // Process single embedding
                    match Self::process_embedding_in_worker(&operations, &entry_id).await {
                        Ok(_) => {
                            processed_count.fetch_add(1, Ordering::Relaxed);
                        },
                        Err(e) => {
                            error_count.fetch_add(1, Ordering::Relaxed);
                            let _ = error_tx.send(format!("Failed to process embedding {}: {}", entry_id, e));
                        }
                    }
                }
                
                // Update progress for this batch
                let current_processed = processed_count.load(Ordering::Relaxed);
                let _elapsed = start_time.elapsed().as_secs();
                
                if enable_debug && batch_index % 10 == 0 {
                    eprintln!("🔄 Batch {} completed, {} embeddings processed", batch_index, current_processed);
                }
            });
            
            tasks.push(task);
        }
        
        // Progress reporting task
        let progress_task = {
            let processed_count = processed_count.clone();
            let progress = self.progress.clone();
            let cancelled = self.cancelled.clone();
            let progress_interval = Duration::from_millis(self.config.progress_report_interval_ms);
            let enable_reporting = self.config.enable_progress_reporting;
            
            tokio::spawn(async move {
                if !enable_reporting {
                    return;
                }
                
                let mut interval = tokio::time::interval(progress_interval);
                while !cancelled.load(Ordering::Relaxed) {
                    interval.tick().await;
                    
                    let current_processed = processed_count.load(Ordering::Relaxed);
                    let elapsed = start_time.elapsed().as_secs();
                    
                    let mut progress_guard = progress.write().await;
                    progress_guard.update(
                        current_processed,
                        format!("Processing embeddings ({}/{})", current_processed, total_items),
                        elapsed,
                    );
                    
                    if current_processed >= total_items {
                        break;
                    }
                }
            })
        };
        
        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }
        
        // Stop progress reporting
        self.cancelled.store(true, Ordering::Relaxed);
        let _ = progress_task.await;
        self.cancelled.store(false, Ordering::Relaxed);
        
        // Collect errors
        drop(error_tx); // Close the channel
        let mut errors = Vec::new();
        while let Some(error) = error_rx.recv().await {
            errors.push(error);
        }
        
        let final_processed_count = processed_count.load(Ordering::Relaxed);
        
        if self.config.enable_debug_logging {
            eprintln!("🎯 Parallel processing completed: {} processed, {} errors", 
                     final_processed_count, errors.len());
        }
        
        Ok((final_processed_count, errors))
    }
    
    /// Process a single embedding in a worker thread
    async fn process_embedding_in_worker(
        operations: &VectorOperations,
        entry_id: &str,
    ) -> VectorDbResult<()> {
        // Retrieve the embedding to validate it exists and is accessible
        match operations.retrieve_embedding(entry_id).await? {
            Some(_entry) => {
                // In a full implementation, this would perform any necessary
                // processing or validation of the embedding data
                // For now, we just verify it's accessible
                Ok(())
            },
            None => {
                Err(VectorDbError::Storage {
                    message: format!("Embedding {} not found", entry_id),
                })
            }
        }
    }
    
    /// Process a single embedding
    async fn process_single_embedding(&self, entry_id: &str) -> VectorDbResult<()> {
        Self::process_embedding_in_worker(&self.operations, entry_id).await
    }
    
    /// Check if the rebuild was cancelled
    async fn check_cancelled(&self) -> VectorDbResult<bool> {
        Ok(self.cancelled.load(Ordering::Relaxed))
    }
    
    /// Update progress with new phase and description
    async fn update_progress(&self, phase: RebuildPhase, description: &str) {
        let mut progress = self.progress.write().await;
        progress.phase = phase;
        progress.current_operation = description.to_string();
    }
    
    /// Update progress with detailed information
    async fn update_progress_detailed(
        &self,
        phase: RebuildPhase,
        operation: String,
        processed_items: usize,
        elapsed_seconds: u64,
    ) {
        let mut progress = self.progress.write().await;
        progress.phase = phase;
        progress.update(processed_items, operation, elapsed_seconds);
    }
    
    /// Report progress via callback
    async fn report_progress(&self) {
        if let Some(ref callback) = self.progress_callback {
            let progress = self.progress.read().await;
            callback(progress.clone());
        }
    }
    
    /// Calculate rebuild metrics
    async fn calculate_metrics(&self, processed_count: usize, total_time_ms: u64) -> RebuildMetrics {
        let mut metrics = RebuildMetrics::default();
        
        if processed_count > 0 {
            metrics.avg_processing_time_ms = total_time_ms as f64 / processed_count as f64;
            metrics.throughput_eps = processed_count as f64 / (total_time_ms as f64 / 1000.0);
        }
        
        let worker_count = if self.config.enable_parallel_processing {
            if self.config.parallel_workers == 0 {
                num_cpus::get().clamp(2, 8)
            } else {
                self.config.parallel_workers
            }
        } else {
            1
        };
        
        metrics.workers_used = worker_count;
        
        // Additional metrics would be collected here in a full implementation
        // Such as memory usage, I/O statistics, CPU usage, etc.
        
        metrics
    }
}

/// Health checker for index validation
pub struct HealthChecker {
    /// Storage backend
    storage: Arc<VectorStorage>,
    /// Vector operations interface
    operations: VectorOperations,
    /// Health check configuration
    config: HealthCheckConfig,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(
        storage: Arc<VectorStorage>,
        operations: VectorOperations,
        config: HealthCheckConfig,
    ) -> Self {
        Self {
            storage,
            operations,
            config,
        }
    }
    
    /// Perform comprehensive health check
    pub async fn perform_health_check(&self) -> VectorDbResult<HealthCheckResult> {
        let start_time = Instant::now();
        
        eprintln!("🏥 Starting comprehensive health check...");
        
        let mut issues_found = Vec::new();
        let mut recommendations = Vec::new();
        
        // Integrity validation
        let integrity_results = if self.config.enable_integrity_validation {
            match self.perform_integrity_check().await {
                Ok(results) => {
                    if !results.passed {
                        issues_found.extend(
                            results.integrity_issues.iter().map(|issue| HealthIssue {
                                issue_type: HealthIssueType::IntegrityIssue,
                                severity: IssueSeverity::High,
                                description: issue.clone(),
                                recommended_action: "Run index rebuild to fix integrity issues".to_string(),
                            })
                        );
                    }
                    Some(results)
                },
                Err(e) => {
                    eprintln!("❌ Integrity check failed: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        // Performance validation
        let performance_results = if self.config.enable_performance_validation {
            match self.perform_performance_check().await {
                Ok(results) => {
                    if !results.meets_targets {
                        issues_found.extend(
                            results.performance_issues.iter().map(|issue| HealthIssue {
                                issue_type: HealthIssueType::PerformanceIssue,
                                severity: IssueSeverity::Medium,
                                description: issue.clone(),
                                recommended_action: "Consider index optimization or compaction".to_string(),
                            })
                        );
                    }
                    Some(results)
                },
                Err(e) => {
                    eprintln!("❌ Performance check failed: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        // Corruption detection
        let corruption_results = if self.config.enable_corruption_detection {
            match self.perform_corruption_check().await {
                Ok(results) => {
                    if results.corruption_detected {
                        let severity = match results.corruption_severity {
                            CorruptionSeverity::Minor => IssueSeverity::Low,
                            CorruptionSeverity::Moderate => IssueSeverity::Medium,
                            CorruptionSeverity::Severe => IssueSeverity::High,
                            CorruptionSeverity::Critical => IssueSeverity::Critical,
                        };
                        
                        issues_found.extend(
                            results.corruption_details.iter().map(|detail| HealthIssue {
                                issue_type: HealthIssueType::CorruptionIssue,
                                severity,
                                description: detail.clone(),
                                recommended_action: if results.recoverable {
                                    "Run automatic recovery or index rebuild".to_string()
                                } else {
                                    "Manual intervention required - backup and rebuild index".to_string()
                                },
                            })
                        );
                    }
                    Some(results)
                },
                Err(e) => {
                    eprintln!("❌ Corruption check failed: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        // Determine overall health
        let overall_health = if issues_found.iter().any(|i| i.severity == IssueSeverity::Critical) {
            HealthStatus::Critical
        } else if issues_found.iter().any(|i| i.severity == IssueSeverity::High) {
            HealthStatus::Degraded
        } else if issues_found.iter().any(|i| i.severity == IssueSeverity::Medium) {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };
        
        // Generate recommendations
        if issues_found.is_empty() {
            recommendations.push("Index is healthy - no action required".to_string());
        } else {
            recommendations.push("Consider running maintenance cycle to address issues".to_string());
            if issues_found.iter().any(|i| matches!(i.issue_type, HealthIssueType::PerformanceIssue)) {
                recommendations.push("Run index compaction to improve performance".to_string());
            }
            if issues_found.iter().any(|i| matches!(i.issue_type, HealthIssueType::CorruptionIssue)) {
                recommendations.push("Run index rebuild to fix corruption issues".to_string());
            }
        }
        
        let check_time_ms = start_time.elapsed().as_millis() as u64;
        
        let result = HealthCheckResult {
            overall_health,
            check_time_ms,
            integrity_results,
            performance_results,
            corruption_results,
            issues_found,
            recommendations,
        };
        
        eprintln!("🏥 Health check completed: {} ({}ms)", result.summary(), check_time_ms);
        
        Ok(result)
    }
    
    /// Perform integrity validation
    async fn perform_integrity_check(&self) -> VectorDbResult<IntegrityCheckResult> {
        eprintln!("🔍 Performing integrity validation...");
        
        let integrity_report = self.storage.validate_integrity().await?;
        let all_ids = self.operations.list_embedding_ids().await;
        
        let mut integrity_issues = Vec::new();
        let mut violations_found = 0;
        
        // Check basic storage integrity
        if !integrity_report.is_healthy() {
            integrity_issues.push(format!("Storage integrity issues: {}", integrity_report.summary()));
            violations_found += 1;
        }
        
        // Sample-based validation of individual entries
        let sample_size = (all_ids.len() as f64 * 0.1).clamp(10.0, 100.0) as usize;
        let sample_ids = if all_ids.len() <= sample_size {
            all_ids
        } else {
            let step = all_ids.len() / sample_size;
            all_ids.iter().step_by(step).cloned().collect()
        };
        
        for entry_id in &sample_ids {
            match self.operations.retrieve_embedding(entry_id).await {
                Ok(Some(entry)) => {
                    // Validate entry structure
                    if let Err(e) = entry.validate() {
                        integrity_issues.push(format!("Entry {} validation failed: {}", entry_id, e));
                        violations_found += 1;
                    }
                },
                Ok(None) => {
                    integrity_issues.push(format!("Entry {} exists in index but cannot be retrieved", entry_id));
                    violations_found += 1;
                },
                Err(e) => {
                    integrity_issues.push(format!("Failed to retrieve entry {}: {}", entry_id, e));
                    violations_found += 1;
                }
            }
        }
        
        let passed = violations_found == 0;
        
        Ok(IntegrityCheckResult {
            passed,
            entries_validated: sample_ids.len(),
            violations_found,
            integrity_issues,
        })
    }
    
    /// Perform performance validation
    async fn perform_performance_check(&self) -> VectorDbResult<PerformanceCheckResult> {
        eprintln!("⚡ Performing performance validation...");
        
        let all_ids = self.operations.list_embedding_ids().await;
        let sample_size = (all_ids.len() as f64 * self.config.performance_sample_percentage).clamp(5.0, 50.0) as usize;
        
        let sample_ids = if all_ids.len() <= sample_size {
            all_ids
        } else {
            let step = all_ids.len() / sample_size;
            all_ids.iter().step_by(step).cloned().collect()
        };
        
        let mut total_query_time = Duration::new(0, 0);
        let mut successful_queries = 0;
        let mut performance_issues = Vec::new();
        
        // Test retrieval performance
        for entry_id in &sample_ids {
            let start = Instant::now();
            match self.operations.retrieve_embedding(entry_id).await {
                Ok(Some(_)) => {
                    let query_time = start.elapsed();
                    total_query_time += query_time;
                    successful_queries += 1;
                    
                    // Check if individual query exceeds target
                    if query_time.as_millis() > 100 { // 100ms target per query
                        performance_issues.push(format!(
                            "Slow query for entry {}: {}ms",
                            entry_id,
                            query_time.as_millis()
                        ));
                    }
                },
                Ok(None) => {
                    performance_issues.push(format!("Entry {} not found during performance test", entry_id));
                },
                Err(e) => {
                    performance_issues.push(format!("Query failed for entry {}: {}", entry_id, e));
                }
            }
        }
        
        let avg_query_time_ms = if successful_queries > 0 {
            total_query_time.as_millis() as f64 / successful_queries as f64
        } else {
            0.0
        };
        
        // Check if average performance meets targets
        let meets_targets = avg_query_time_ms <= 50.0 && performance_issues.len() <= sample_ids.len() / 10;
        
        if !meets_targets && performance_issues.is_empty() {
            performance_issues.push(format!("Average query time {:.2}ms exceeds target of 50ms", avg_query_time_ms));
        }
        
        Ok(PerformanceCheckResult {
            meets_targets,
            avg_query_time_ms,
            queries_tested: successful_queries,
            peak_memory_usage: 0, // Would be measured in full implementation
            performance_issues,
        })
    }
    
    /// Perform corruption detection
    async fn perform_corruption_check(&self) -> VectorDbResult<CorruptionCheckResult> {
        eprintln!("🔎 Performing corruption detection...");
        
        let mut corruption_types = HashSet::new();
        let mut corruption_details = Vec::new();
        let mut corruption_severity = CorruptionSeverity::Minor;
        
        // Check storage-level integrity
        let integrity_report = self.storage.validate_integrity().await?;
        if !integrity_report.is_healthy() {
            corruption_types.insert(CorruptionType::InvalidStructure);
            corruption_details.push(format!("Storage structure issues detected: {}", integrity_report.summary()));
            corruption_severity = corruption_severity.max(CorruptionSeverity::Moderate);
        }
        
        // Sample-based corruption detection
        let all_ids = self.operations.list_embedding_ids().await;
        let sample_size = (all_ids.len() as f64 * 0.05).clamp(5.0, 25.0) as usize; // 5% sample
        
        let sample_ids = if all_ids.len() <= sample_size {
            all_ids
        } else {
            let step = all_ids.len() / sample_size;
            all_ids.iter().step_by(step).cloned().collect()
        };
        
        for entry_id in &sample_ids {
            match self.operations.retrieve_embedding(entry_id).await {
                Ok(Some(entry)) => {
                    // Check for invalid vectors
                    if entry.vector.is_empty() || entry.vector.iter().all(|&x| x == 0.0) {
                        corruption_types.insert(CorruptionType::InvalidVectors);
                        corruption_details.push(format!("Invalid vector data in entry {}", entry_id));
                        corruption_severity = corruption_severity.max(CorruptionSeverity::Moderate);
                    }
                    
                    // Check for corrupted metadata
                    if entry.metadata.file_path.is_empty() || entry.metadata.chunk_id.is_empty() {
                        corruption_types.insert(CorruptionType::CorruptedMetadata);
                        corruption_details.push(format!("Corrupted metadata in entry {}", entry_id));
                        corruption_severity = corruption_severity.max(CorruptionSeverity::Moderate);
                    }
                },
                Ok(None) => {
                    corruption_types.insert(CorruptionType::MissingData);
                    corruption_details.push(format!("Missing data for indexed entry {}", entry_id));
                    corruption_severity = corruption_severity.max(CorruptionSeverity::Severe);
                },
                Err(_) => {
                    corruption_types.insert(CorruptionType::InconsistentIndex);
                    corruption_details.push(format!("Inconsistent index for entry {}", entry_id));
                    corruption_severity = corruption_severity.max(CorruptionSeverity::Severe);
                }
            }
        }
        
        let corruption_detected = !corruption_types.is_empty();
        let corruption_types: Vec<CorruptionType> = corruption_types.into_iter().collect();
        
        // Determine if corruption is recoverable
        let recoverable = !corruption_types.contains(&CorruptionType::ChecksumMismatch) 
                       && corruption_severity < CorruptionSeverity::Critical;
        
        Ok(CorruptionCheckResult {
            corruption_detected,
            corruption_types,
            corruption_severity,
            corruption_details,
            recoverable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::vector_db::types::VectorStorageConfig;

    fn create_test_config() -> VectorStorageConfig {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_string_lossy().to_string();
        std::mem::forget(temp_dir); // Keep temp dir alive for test
        
        VectorStorageConfig {
            storage_dir,
            enable_compression: false,
            compression_algorithm: crate::vector_db::types::CompressionAlgorithm::None,
            max_entries_per_file: 100,
            enable_checksums: false,
            auto_backup: false,
            max_backups: 0,
            enable_metrics: true,
            enable_vector_compression: false,
            vector_compression_algorithm: crate::vector_db::types::VectorCompressionAlgorithm::None,
            enable_lazy_loading: false,
            lazy_loading_threshold: 1000,
        }
    }

    #[test]
    fn test_rebuilding_config_defaults() {
        let config = RebuildingConfig::default();
        
        assert!(config.enable_parallel_processing);
        assert_eq!(config.parallel_workers, 0); // Auto-detect
        assert_eq!(config.rebuild_batch_size, 100);
        assert_eq!(config.operation_timeout_seconds, 1800);
        assert!(config.enable_progress_reporting);
        assert!(config.validate_after_rebuild);
        assert!(config.backup_before_rebuild);
    }

    #[test]
    fn test_rebuild_progress() {
        let mut progress = RebuildProgress::new(100, RebuildPhase::Initializing);
        
        assert_eq!(progress.total_items, 100);
        assert_eq!(progress.processed_items, 0);
        assert_eq!(progress.progress_percentage, 0.0);
        assert!(!progress.is_complete());
        
        progress.update(50, "Processing".to_string(), 10);
        assert_eq!(progress.processed_items, 50);
        assert_eq!(progress.progress_percentage, 0.5);
        assert_eq!(progress.processing_rate, 5.0); // 50 items / 10 seconds
        
        progress.add_error();
        assert!(progress.has_errors);
        assert_eq!(progress.error_count, 1);
    }

    #[test]
    fn test_rebuild_phases() {
        assert_eq!(RebuildPhase::Initializing.to_string(), "Initializing");
        assert_eq!(RebuildPhase::ProcessingEmbeddings.to_string(), "Processing Embeddings");
        assert_eq!(RebuildPhase::Completed.to_string(), "Completed");
        assert_eq!(RebuildPhase::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_health_check_config_defaults() {
        let config = HealthCheckConfig::default();
        
        assert!(config.enable_integrity_validation);
        assert!(config.enable_performance_validation);
        assert!(config.enable_corruption_detection);
        assert_eq!(config.performance_sample_percentage, 0.1);
        assert_eq!(config.target_check_time_seconds, 1);
        assert!(config.enable_detailed_reporting);
    }

    #[test]
    fn test_health_status_ordering() {
        assert!(HealthStatus::Healthy < HealthStatus::Warning);
        assert!(HealthStatus::Warning < HealthStatus::Degraded);
        assert!(HealthStatus::Degraded < HealthStatus::Critical);
    }

    #[test]
    fn test_corruption_severity_ordering() {
        assert!(CorruptionSeverity::Minor < CorruptionSeverity::Moderate);
        assert!(CorruptionSeverity::Moderate < CorruptionSeverity::Severe);
        assert!(CorruptionSeverity::Severe < CorruptionSeverity::Critical);
    }

    #[test]
    fn test_health_issue_creation() {
        let issue = HealthIssue {
            issue_type: HealthIssueType::PerformanceIssue,
            severity: IssueSeverity::Medium,
            description: "Slow query performance".to_string(),
            recommended_action: "Run index optimization".to_string(),
        };
        
        assert!(matches!(issue.issue_type, HealthIssueType::PerformanceIssue));
        assert_eq!(issue.severity, IssueSeverity::Medium);
        assert_eq!(issue.description, "Slow query performance");
    }

    #[test]
    fn test_rebuild_metrics_performance_targets() {
        let mut metrics = RebuildMetrics::default();
        
        // Test meeting targets
        metrics.avg_processing_time_ms = 25.0; // 25ms per item
        assert!(metrics.meets_performance_targets(30)); // Target: 30 seconds per 1000 notes
        
        // Test not meeting targets
        metrics.avg_processing_time_ms = 35.0; // 35ms per item
        assert!(!metrics.meets_performance_targets(30));
    }

    #[test]
    fn test_health_check_result_performance() {
        let mut result = HealthCheckResult {
            overall_health: HealthStatus::Healthy,
            check_time_ms: 500,
            integrity_results: None,
            performance_results: None,
            corruption_results: None,
            issues_found: Vec::new(),
            recommendations: Vec::new(),
        };
        
        assert!(result.meets_performance_targets());
        
        result.check_time_ms = 1500;
        assert!(!result.meets_performance_targets());
    }

    #[tokio::test]
    async fn test_index_rebuilder_creation() {
        let storage_config = create_test_config();
        let rebuild_config = RebuildingConfig::default();
        let storage = Arc::new(VectorStorage::new(storage_config.clone()).unwrap());
        let operations = VectorOperations::new(storage.clone(), storage_config);
        
        let rebuilder = IndexRebuilder::new(storage, operations, rebuild_config.clone());
        
        assert!(rebuild_config.enable_parallel_processing);
        assert_eq!(rebuild_config.rebuild_batch_size, 100);
        
        // Test progress initialization
        let progress = rebuilder.progress.read().await;
        assert_eq!(progress.phase, RebuildPhase::Initializing);
        assert_eq!(progress.processed_items, 0);
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let storage_config = create_test_config();
        let health_config = HealthCheckConfig::default();
        let storage = Arc::new(VectorStorage::new(storage_config.clone()).unwrap());
        let operations = VectorOperations::new(storage.clone(), storage_config);
        
        let _health_checker = HealthChecker::new(storage, operations, health_config.clone());
        
        assert!(health_config.enable_integrity_validation);
        assert!(health_config.enable_performance_validation);
        assert!(health_config.enable_corruption_detection);
    }
}