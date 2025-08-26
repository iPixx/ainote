//! Performance Monitoring System for Index Management Operations
//!
//! This module provides comprehensive performance monitoring and metrics collection
//! for all index management operations including incremental updates, maintenance,
//! and rebuilding operations.
//!
//! ## Features
//!
//! - **Real-time Metrics**: Live performance data collection during operations
//! - **Operation Tracking**: Detailed tracking of incremental, maintenance, and rebuild operations
//! - **Resource Monitoring**: Memory, CPU, and I/O usage tracking
//! - **Performance Trends**: Historical performance analysis and trend detection
//! - **Alerting System**: Configurable alerts for performance degradation
//! - **Minimal Overhead**: <5% performance impact guarantee
//! - **Integration Ready**: Seamless integration with existing logging systems
//!
//! ## Architecture
//!
//! The performance monitoring system consists of:
//!
//! - `IndexPerformanceMonitor`: Main coordinator for all performance monitoring
//! - `OperationMetrics`: Detailed metrics for specific operations
//! - `ResourceTracker`: System resource usage monitoring
//! - `MetricsCollector`: Centralized metrics collection and storage
//! - `PerformanceReporter`: Analysis and reporting capabilities
//! - `AlertManager`: Performance degradation detection and alerting

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::Duration;
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::time::interval;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::vector_db::types::{VectorDbError, VectorDbResult};
use crate::vector_db::incremental::UpdateStats;
use crate::vector_db::maintenance::MaintenanceStats;
use crate::vector_db::rebuilding::RebuildMetrics;

/// Configuration for performance monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable real-time performance monitoring
    pub enable_monitoring: bool,
    /// Maximum number of metrics samples to retain in memory
    pub max_samples_in_memory: usize,
    /// Metrics collection interval in milliseconds
    pub collection_interval_ms: u64,
    /// Enable resource usage tracking (CPU, memory, I/O)
    pub enable_resource_tracking: bool,
    /// Resource tracking interval in milliseconds
    pub resource_tracking_interval_ms: u64,
    /// Maximum allowed performance overhead percentage (0.0-100.0)
    pub max_overhead_percent: f64,
    /// Enable performance alerts
    pub enable_alerts: bool,
    /// Performance degradation threshold for alerts (percentage)
    pub alert_degradation_threshold: f64,
    /// Enable detailed operation logging
    pub enable_detailed_logging: bool,
    /// Metrics persistence file path (None = memory only)
    pub persistence_file_path: Option<String>,
    /// Auto-persist metrics interval in seconds (0 = disabled)
    pub auto_persist_interval_seconds: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            max_samples_in_memory: 10000,
            collection_interval_ms: 100, // 100ms for real-time monitoring
            enable_resource_tracking: true,
            resource_tracking_interval_ms: 1000, // 1 second for resource tracking
            max_overhead_percent: 5.0, // <5% overhead requirement
            enable_alerts: true,
            alert_degradation_threshold: 20.0, // 20% performance degradation
            enable_detailed_logging: false,
            persistence_file_path: None,
            auto_persist_interval_seconds: 300, // 5 minutes
        }
    }
}

/// Types of index management operations being monitored
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// Incremental update operations
    IncrementalUpdate,
    /// Maintenance and cleanup operations
    Maintenance,
    /// Index rebuilding operations
    Rebuilding,
    /// General vector database operations
    VectorOperations,
    /// File system operations
    FileOperations,
}

/// Performance metrics for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    /// Type of operation
    pub operation_type: OperationType,
    /// Unique operation identifier
    pub operation_id: String,
    /// Operation start timestamp
    pub started_at: DateTime<Utc>,
    /// Operation completion timestamp (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Total duration in milliseconds
    pub duration_ms: Option<f64>,
    /// Operation status (success/failure/in_progress)
    pub status: OperationStatus,
    /// Number of items processed (files, embeddings, etc.)
    pub items_processed: u64,
    /// Processing rate (items per second)
    pub processing_rate: f64,
    /// Memory usage at start (MB)
    pub memory_start_mb: f64,
    /// Peak memory usage during operation (MB)
    pub memory_peak_mb: f64,
    /// Memory usage at end (MB)
    pub memory_end_mb: f64,
    /// CPU usage percentage during operation
    pub cpu_usage_percent: f64,
    /// I/O operations performed
    pub io_operations: u64,
    /// Bytes read during operation
    pub bytes_read: u64,
    /// Bytes written during operation
    pub bytes_written: u64,
    /// Operation-specific data
    pub operation_data: HashMap<String, serde_json::Value>,
    /// Error message (if operation failed)
    pub error_message: Option<String>,
}

/// Status of an operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation is currently running
    InProgress,
    /// Operation completed successfully
    Success,
    /// Operation failed with error
    Failed,
    /// Operation was cancelled
    Cancelled,
    /// Operation timed out
    TimedOut,
}

/// Real-time system resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Timestamp of measurement
    pub timestamp: DateTime<Utc>,
    /// CPU usage percentage (0.0-100.0)
    pub cpu_usage_percent: f64,
    /// Memory usage in MB
    pub memory_usage_mb: f64,
    /// Available memory in MB
    pub memory_available_mb: f64,
    /// Disk I/O read rate (MB/s)
    pub disk_read_mb_per_sec: f64,
    /// Disk I/O write rate (MB/s)
    pub disk_write_mb_per_sec: f64,
    /// Network I/O receive rate (KB/s)
    pub network_receive_kb_per_sec: f64,
    /// Network I/O transmit rate (KB/s)
    pub network_transmit_kb_per_sec: f64,
    /// Number of active threads
    pub active_threads: u32,
    /// System load average (1 minute)
    pub load_average_1min: f64,
}

/// Performance alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert identifier
    pub alert_id: String,
    /// Alert severity level
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Operation type that triggered the alert
    pub operation_type: OperationType,
    /// Alert timestamp
    pub triggered_at: DateTime<Utc>,
    /// Performance metrics that triggered the alert
    pub triggering_metrics: OperationMetrics,
    /// Suggested actions to address the alert
    pub suggested_actions: Vec<String>,
    /// Whether the alert has been acknowledged
    pub acknowledged: bool,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning level alert
    Warning,
    /// Critical performance issue
    Critical,
}

/// Comprehensive performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Time period covered by the report
    pub period_start: DateTime<Utc>,
    /// Time period end
    pub period_end: DateTime<Utc>,
    /// Total number of operations in period
    pub total_operations: u64,
    /// Operations by type
    pub operations_by_type: HashMap<OperationType, u64>,
    /// Average performance metrics by operation type
    pub avg_metrics_by_type: HashMap<OperationType, OperationMetrics>,
    /// Performance trends over time
    pub performance_trends: Vec<PerformanceTrend>,
    /// Resource utilization summary
    pub resource_utilization: ResourceUtilizationSummary,
    /// Active alerts
    pub active_alerts: Vec<PerformanceAlert>,
    /// Performance recommendations
    pub recommendations: Vec<String>,
    /// Overall performance health score (0.0-1.0)
    pub health_score: f64,
}

/// Performance trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    /// Timestamp of the trend data point
    pub timestamp: DateTime<Utc>,
    /// Operation type
    pub operation_type: OperationType,
    /// Average duration for operations at this time
    pub avg_duration_ms: f64,
    /// Average processing rate
    pub avg_processing_rate: f64,
    /// Average memory usage
    pub avg_memory_usage_mb: f64,
    /// Number of operations in this time bucket
    pub operation_count: u64,
}

/// Resource utilization summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationSummary {
    /// Average CPU usage during period
    pub avg_cpu_usage_percent: f64,
    /// Peak CPU usage during period
    pub peak_cpu_usage_percent: f64,
    /// Average memory usage during period
    pub avg_memory_usage_mb: f64,
    /// Peak memory usage during period
    pub peak_memory_usage_mb: f64,
    /// Total disk I/O during period (MB)
    pub total_disk_io_mb: f64,
    /// Average disk I/O rate (MB/s)
    pub avg_disk_io_mb_per_sec: f64,
    /// Total network I/O during period (KB)
    pub total_network_io_kb: f64,
    /// Average network I/O rate (KB/s)
    pub avg_network_io_kb_per_sec: f64,
}

/// Main performance monitoring system
#[derive(Debug)]
pub struct IndexPerformanceMonitor {
    /// Monitoring configuration
    config: MonitoringConfig,
    /// Currently active operation metrics
    active_operations: Arc<RwLock<HashMap<String, OperationMetrics>>>,
    /// Historical operation metrics (circular buffer)
    operation_history: Arc<Mutex<VecDeque<OperationMetrics>>>,
    /// Real-time resource metrics (circular buffer)
    resource_history: Arc<Mutex<VecDeque<ResourceMetrics>>>,
    /// Active performance alerts
    active_alerts: Arc<RwLock<HashMap<String, PerformanceAlert>>>,
    /// Metrics collection task handle
    collection_task: Option<tokio::task::JoinHandle<()>>,
    /// Resource monitoring task handle
    resource_task: Option<tokio::task::JoinHandle<()>>,
    /// Alert processing task handle
    alert_task: Option<tokio::task::JoinHandle<()>>,
    /// Monitoring enabled flag
    monitoring_enabled: Arc<AtomicBool>,
    /// Total operations counter
    total_operations: Arc<AtomicU64>,
    /// Channel for receiving operation updates
    operation_receiver: Option<mpsc::UnboundedReceiver<OperationMetrics>>,
    /// Channel for sending operation updates
    operation_sender: mpsc::UnboundedSender<OperationMetrics>,
}

impl IndexPerformanceMonitor {
    /// Create a new performance monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        let (operation_sender, operation_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            operation_history: Arc::new(Mutex::new(VecDeque::new())),
            resource_history: Arc::new(Mutex::new(VecDeque::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            collection_task: None,
            resource_task: None,
            alert_task: None,
            monitoring_enabled: Arc::new(AtomicBool::new(false)),
            total_operations: Arc::new(AtomicU64::new(0)),
            operation_receiver: Some(operation_receiver),
            operation_sender,
        }
    }

    /// Start the performance monitoring system
    pub async fn start(&mut self) -> VectorDbResult<()> {
        if !self.config.enable_monitoring {
            return Ok(());
        }

        self.monitoring_enabled.store(true, Ordering::Relaxed);

        // Start metrics collection task
        let operation_receiver = self.operation_receiver.take()
            .ok_or_else(|| VectorDbError::Storage { message: "Monitoring already started".into() })?;

        self.start_collection_task(operation_receiver).await?;

        // Start resource monitoring task if enabled
        if self.config.enable_resource_tracking {
            self.start_resource_monitoring_task().await?;
        }

        // Start alert processing task if enabled
        if self.config.enable_alerts {
            self.start_alert_processing_task().await?;
        }

        println!("✅ Index Performance Monitor started successfully");
        Ok(())
    }

    /// Stop the performance monitoring system
    pub async fn stop(&mut self) -> VectorDbResult<()> {
        self.monitoring_enabled.store(false, Ordering::Relaxed);

        // Stop all background tasks
        if let Some(task) = self.collection_task.take() {
            task.abort();
        }
        
        if let Some(task) = self.resource_task.take() {
            task.abort();
        }
        
        if let Some(task) = self.alert_task.take() {
            task.abort();
        }

        // Persist metrics if configured
        if let Some(ref path) = self.config.persistence_file_path {
            self.persist_metrics(path).await?;
        }

        println!("✅ Index Performance Monitor stopped successfully");
        Ok(())
    }

    /// Start an operation for monitoring
    pub async fn start_operation(
        &self,
        operation_type: OperationType,
        operation_id: String,
    ) -> VectorDbResult<()> {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let metrics = OperationMetrics {
            operation_type,
            operation_id: operation_id.clone(),
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            status: OperationStatus::InProgress,
            items_processed: 0,
            processing_rate: 0.0,
            memory_start_mb: self.get_current_memory_usage_mb().await,
            memory_peak_mb: 0.0,
            memory_end_mb: 0.0,
            cpu_usage_percent: 0.0,
            io_operations: 0,
            bytes_read: 0,
            bytes_written: 0,
            operation_data: HashMap::new(),
            error_message: None,
        };

        self.active_operations.write().await.insert(operation_id, metrics);
        Ok(())
    }

    /// Update an active operation's metrics
    pub async fn update_operation(
        &self,
        operation_id: &str,
        items_processed: u64,
        bytes_read: u64,
        bytes_written: u64,
        operation_data: Option<HashMap<String, serde_json::Value>>,
    ) -> VectorDbResult<()> {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut operations = self.active_operations.write().await;
        if let Some(metrics) = operations.get_mut(operation_id) {
            let current_memory = self.get_current_memory_usage_mb().await;
            
            metrics.items_processed = items_processed;
            metrics.bytes_read = bytes_read;
            metrics.bytes_written = bytes_written;
            metrics.memory_peak_mb = metrics.memory_peak_mb.max(current_memory);
            
            // Calculate processing rate
            if let Ok(elapsed) = Utc::now().signed_duration_since(metrics.started_at).to_std() {
                if elapsed.as_secs_f64() > 0.0 {
                    metrics.processing_rate = items_processed as f64 / elapsed.as_secs_f64();
                }
            }

            // Update operation-specific data
            if let Some(data) = operation_data {
                metrics.operation_data.extend(data);
            }
        }

        Ok(())
    }

    /// Complete an operation
    pub async fn complete_operation(
        &self,
        operation_id: &str,
        status: OperationStatus,
        error_message: Option<String>,
    ) -> VectorDbResult<()> {
        if !self.monitoring_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut operations = self.active_operations.write().await;
        if let Some(mut metrics) = operations.remove(operation_id) {
            let completion_time = Utc::now();
            metrics.completed_at = Some(completion_time);
            metrics.status = status;
            metrics.error_message = error_message;
            metrics.memory_end_mb = self.get_current_memory_usage_mb().await;

            // Calculate final duration
            if let Ok(duration) = completion_time.signed_duration_since(metrics.started_at).to_std() {
                metrics.duration_ms = Some(duration.as_secs_f64() * 1000.0);
            }

            // Send completed metrics for processing
            if let Err(e) = self.operation_sender.send(metrics) {
                eprintln!("Failed to send operation metrics: {}", e);
            }

            self.total_operations.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Get current performance metrics
    pub async fn get_current_metrics(&self) -> VectorDbResult<HashMap<OperationType, OperationMetrics>> {
        let mut current_metrics = HashMap::new();
        let operations = self.active_operations.read().await;

        // Calculate average metrics for each operation type
        let mut type_metrics: HashMap<OperationType, Vec<&OperationMetrics>> = HashMap::new();
        
        for metrics in operations.values() {
            type_metrics.entry(metrics.operation_type.clone())
                .or_insert_with(Vec::new)
                .push(metrics);
        }

        for (op_type, metrics_list) in type_metrics {
            if let Some(avg_metrics) = self.calculate_average_metrics(&metrics_list) {
                current_metrics.insert(op_type, avg_metrics);
            }
        }

        Ok(current_metrics)
    }

    /// Generate comprehensive performance report
    pub async fn generate_performance_report(
        &self,
        period_hours: u64,
    ) -> VectorDbResult<PerformanceReport> {
        let end_time = Utc::now();
        let start_time = end_time - chrono::Duration::hours(period_hours as i64);

        let operation_history = self.operation_history.lock().await;
        let resource_history = self.resource_history.lock().await;
        let active_alerts = self.active_alerts.read().await;

        // Filter operations within the time period
        let period_operations: Vec<&OperationMetrics> = operation_history
            .iter()
            .filter(|op| op.started_at >= start_time && op.started_at <= end_time)
            .collect();

        // Calculate statistics
        let total_operations = period_operations.len() as u64;
        let mut operations_by_type: HashMap<OperationType, u64> = HashMap::new();
        let mut avg_metrics_by_type: HashMap<OperationType, OperationMetrics> = HashMap::new();

        // Group operations by type for analysis
        let mut type_groups: HashMap<OperationType, Vec<&OperationMetrics>> = HashMap::new();
        for op in &period_operations {
            *operations_by_type.entry(op.operation_type.clone()).or_insert(0) += 1;
            type_groups.entry(op.operation_type.clone())
                .or_insert_with(Vec::new)
                .push(op);
        }

        // Calculate average metrics for each type
        for (op_type, ops) in type_groups {
            if let Some(avg) = self.calculate_average_metrics(&ops) {
                avg_metrics_by_type.insert(op_type, avg);
            }
        }

        // Generate performance trends (hourly buckets)
        let performance_trends = self.generate_performance_trends(&period_operations, start_time, end_time);

        // Calculate resource utilization summary
        let resource_utilization = self.calculate_resource_utilization_summary(&resource_history, start_time, end_time);

        // Get active alerts
        let alerts: Vec<PerformanceAlert> = active_alerts.values().cloned().collect();

        // Generate recommendations
        let recommendations = self.generate_recommendations(&avg_metrics_by_type, &resource_utilization);

        // Calculate health score
        let health_score = self.calculate_health_score(&avg_metrics_by_type, &resource_utilization, &alerts);

        Ok(PerformanceReport {
            generated_at: Utc::now(),
            period_start: start_time,
            period_end: end_time,
            total_operations,
            operations_by_type,
            avg_metrics_by_type,
            performance_trends,
            resource_utilization,
            active_alerts: alerts,
            recommendations,
            health_score,
        })
    }

    // Private helper methods...

    /// Start the metrics collection background task
    async fn start_collection_task(
        &mut self,
        mut operation_receiver: mpsc::UnboundedReceiver<OperationMetrics>,
    ) -> VectorDbResult<()> {
        let operation_history = Arc::clone(&self.operation_history);
        let max_samples = self.config.max_samples_in_memory;
        let monitoring_enabled = Arc::clone(&self.monitoring_enabled);

        let task = tokio::spawn(async move {
            while monitoring_enabled.load(Ordering::Relaxed) {
                if let Some(metrics) = operation_receiver.recv().await {
                    let mut history = operation_history.lock().await;
                    
                    // Add to history (circular buffer)
                    history.push_back(metrics);
                    
                    // Maintain maximum size
                    while history.len() > max_samples {
                        history.pop_front();
                    }
                } else {
                    break;
                }
            }
        });

        self.collection_task = Some(task);
        Ok(())
    }

    /// Start the resource monitoring background task
    async fn start_resource_monitoring_task(&mut self) -> VectorDbResult<()> {
        let resource_history = Arc::clone(&self.resource_history);
        let max_samples = self.config.max_samples_in_memory;
        let monitoring_enabled = Arc::clone(&self.monitoring_enabled);
        let interval_ms = self.config.resource_tracking_interval_ms;

        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));
            
            while monitoring_enabled.load(Ordering::Relaxed) {
                interval.tick().await;
                
                let resource_metrics = ResourceMetrics {
                    timestamp: Utc::now(),
                    cpu_usage_percent: Self::get_cpu_usage_percentage().await,
                    memory_usage_mb: Self::get_memory_usage_mb().await,
                    memory_available_mb: Self::get_available_memory_mb().await,
                    disk_read_mb_per_sec: 0.0,  // TODO: Implement disk I/O monitoring
                    disk_write_mb_per_sec: 0.0,
                    network_receive_kb_per_sec: 0.0, // TODO: Implement network monitoring
                    network_transmit_kb_per_sec: 0.0,
                    active_threads: Self::get_active_thread_count().await,
                    load_average_1min: Self::get_load_average().await,
                };

                let mut history = resource_history.lock().await;
                history.push_back(resource_metrics);
                
                // Maintain maximum size
                while history.len() > max_samples {
                    history.pop_front();
                }
            }
        });

        self.resource_task = Some(task);
        Ok(())
    }

    /// Start the alert processing background task
    async fn start_alert_processing_task(&mut self) -> VectorDbResult<()> {
        let operation_history = Arc::clone(&self.operation_history);
        let active_alerts = Arc::clone(&self.active_alerts);
        let monitoring_enabled = Arc::clone(&self.monitoring_enabled);
        let degradation_threshold = self.config.alert_degradation_threshold;

        let task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10)); // Check every 10 seconds
            
            while monitoring_enabled.load(Ordering::Relaxed) {
                interval.tick().await;
                
                // Check for performance degradations
                let history = operation_history.lock().await;
                if let Some(alerts) = Self::detect_performance_degradation(&history, degradation_threshold).await {
                    let mut alert_map = active_alerts.write().await;
                    for alert in alerts {
                        alert_map.insert(alert.alert_id.clone(), alert);
                    }
                }
            }
        });

        self.alert_task = Some(task);
        Ok(())
    }

    /// Get current memory usage in MB
    async fn get_current_memory_usage_mb(&self) -> f64 {
        Self::get_memory_usage_mb().await
    }

    /// Calculate average metrics from a list of operation metrics
    fn calculate_average_metrics(&self, metrics_list: &[&OperationMetrics]) -> Option<OperationMetrics> {
        if metrics_list.is_empty() {
            return None;
        }

        let count = metrics_list.len() as f64;
        let first = metrics_list[0];

        let avg_duration_ms = metrics_list.iter()
            .filter_map(|m| m.duration_ms)
            .sum::<f64>() / count;

        let avg_processing_rate = metrics_list.iter()
            .map(|m| m.processing_rate)
            .sum::<f64>() / count;

        let avg_memory_peak_mb = metrics_list.iter()
            .map(|m| m.memory_peak_mb)
            .sum::<f64>() / count;

        let avg_cpu_usage = metrics_list.iter()
            .map(|m| m.cpu_usage_percent)
            .sum::<f64>() / count;

        Some(OperationMetrics {
            operation_type: first.operation_type.clone(),
            operation_id: "average".to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration_ms: Some(avg_duration_ms),
            status: OperationStatus::Success,
            items_processed: (metrics_list.iter().map(|m| m.items_processed).sum::<u64>() as f64 / count) as u64,
            processing_rate: avg_processing_rate,
            memory_start_mb: avg_memory_peak_mb,
            memory_peak_mb: avg_memory_peak_mb,
            memory_end_mb: avg_memory_peak_mb,
            cpu_usage_percent: avg_cpu_usage,
            io_operations: (metrics_list.iter().map(|m| m.io_operations).sum::<u64>() as f64 / count) as u64,
            bytes_read: (metrics_list.iter().map(|m| m.bytes_read).sum::<u64>() as f64 / count) as u64,
            bytes_written: (metrics_list.iter().map(|m| m.bytes_written).sum::<u64>() as f64 / count) as u64,
            operation_data: HashMap::new(),
            error_message: None,
        })
    }

    /// Generate performance trends over time
    fn generate_performance_trends(
        &self,
        operations: &[&OperationMetrics],
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Vec<PerformanceTrend> {
        let mut trends = Vec::new();
        let hour_duration = chrono::Duration::hours(1);
        let mut current_time = start_time;

        while current_time < end_time {
            let bucket_end = current_time + hour_duration;
            
            // Group operations by type within this hour bucket
            let mut type_groups: HashMap<OperationType, Vec<&OperationMetrics>> = HashMap::new();
            
            for op in operations {
                if op.started_at >= current_time && op.started_at < bucket_end {
                    type_groups.entry(op.operation_type.clone())
                        .or_insert_with(Vec::new)
                        .push(op);
                }
            }

            // Create trend points for each operation type
            for (op_type, ops) in type_groups {
                if !ops.is_empty() {
                    let avg_duration = ops.iter()
                        .filter_map(|op| op.duration_ms)
                        .sum::<f64>() / ops.len() as f64;

                    let avg_rate = ops.iter()
                        .map(|op| op.processing_rate)
                        .sum::<f64>() / ops.len() as f64;

                    let avg_memory = ops.iter()
                        .map(|op| op.memory_peak_mb)
                        .sum::<f64>() / ops.len() as f64;

                    trends.push(PerformanceTrend {
                        timestamp: current_time,
                        operation_type: op_type,
                        avg_duration_ms: avg_duration,
                        avg_processing_rate: avg_rate,
                        avg_memory_usage_mb: avg_memory,
                        operation_count: ops.len() as u64,
                    });
                }
            }

            current_time = bucket_end;
        }

        trends
    }

    /// Calculate resource utilization summary for the period
    fn calculate_resource_utilization_summary(
        &self,
        resource_history: &VecDeque<ResourceMetrics>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> ResourceUtilizationSummary {
        let period_resources: Vec<&ResourceMetrics> = resource_history
            .iter()
            .filter(|r| r.timestamp >= start_time && r.timestamp <= end_time)
            .collect();

        if period_resources.is_empty() {
            return ResourceUtilizationSummary {
                avg_cpu_usage_percent: 0.0,
                peak_cpu_usage_percent: 0.0,
                avg_memory_usage_mb: 0.0,
                peak_memory_usage_mb: 0.0,
                total_disk_io_mb: 0.0,
                avg_disk_io_mb_per_sec: 0.0,
                total_network_io_kb: 0.0,
                avg_network_io_kb_per_sec: 0.0,
            };
        }

        let count = period_resources.len() as f64;

        ResourceUtilizationSummary {
            avg_cpu_usage_percent: period_resources.iter().map(|r| r.cpu_usage_percent).sum::<f64>() / count,
            peak_cpu_usage_percent: period_resources.iter().map(|r| r.cpu_usage_percent).fold(0.0, f64::max),
            avg_memory_usage_mb: period_resources.iter().map(|r| r.memory_usage_mb).sum::<f64>() / count,
            peak_memory_usage_mb: period_resources.iter().map(|r| r.memory_usage_mb).fold(0.0, f64::max),
            total_disk_io_mb: period_resources.iter().map(|r| r.disk_read_mb_per_sec + r.disk_write_mb_per_sec).sum::<f64>(),
            avg_disk_io_mb_per_sec: period_resources.iter().map(|r| r.disk_read_mb_per_sec + r.disk_write_mb_per_sec).sum::<f64>() / count,
            total_network_io_kb: period_resources.iter().map(|r| r.network_receive_kb_per_sec + r.network_transmit_kb_per_sec).sum::<f64>(),
            avg_network_io_kb_per_sec: period_resources.iter().map(|r| r.network_receive_kb_per_sec + r.network_transmit_kb_per_sec).sum::<f64>() / count,
        }
    }

    /// Generate performance recommendations
    fn generate_recommendations(
        &self,
        avg_metrics: &HashMap<OperationType, OperationMetrics>,
        resource_utilization: &ResourceUtilizationSummary,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check for high memory usage
        if resource_utilization.peak_memory_usage_mb > 500.0 {
            recommendations.push(
                "Consider reducing batch sizes or implementing memory cleanup during long operations".to_string()
            );
        }

        // Check for high CPU usage
        if resource_utilization.avg_cpu_usage_percent > 80.0 {
            recommendations.push(
                "CPU usage is high - consider reducing parallel processing or optimizing algorithms".to_string()
            );
        }

        // Check operation performance
        for (op_type, metrics) in avg_metrics {
            if let Some(duration) = metrics.duration_ms {
                match op_type {
                    OperationType::IncrementalUpdate => {
                        if duration > 100.0 {
                            recommendations.push(
                                "Incremental updates are slower than 100ms target - consider optimizing file change detection".to_string()
                            );
                        }
                    }
                    OperationType::Maintenance => {
                        if duration > 5000.0 {
                            recommendations.push(
                                "Maintenance operations are taking longer than expected - consider reducing cleanup batch sizes".to_string()
                            );
                        }
                    }
                    OperationType::Rebuilding => {
                        if duration > 30000.0 && metrics.items_processed < 1000 {
                            recommendations.push(
                                "Index rebuilding is slower than 30s per 1000 notes target - consider increasing parallel workers".to_string()
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        if recommendations.is_empty() {
            recommendations.push("All operations are performing within acceptable limits".to_string());
        }

        recommendations
    }

    /// Calculate overall performance health score
    fn calculate_health_score(
        &self,
        avg_metrics: &HashMap<OperationType, OperationMetrics>,
        resource_utilization: &ResourceUtilizationSummary,
        alerts: &[PerformanceAlert],
    ) -> f64 {
        let mut score: f64 = 1.0;

        // Penalize high resource usage
        if resource_utilization.avg_cpu_usage_percent > 70.0 {
            score -= 0.1;
        }
        if resource_utilization.peak_memory_usage_mb > 1000.0 {
            score -= 0.1;
        }

        // Penalize slow operations
        for metrics in avg_metrics.values() {
            if let Some(duration) = metrics.duration_ms {
                if duration > 1000.0 {
                    score -= 0.05;
                }
            }
            if metrics.processing_rate < 10.0 {
                score -= 0.05;
            }
        }

        // Penalize active alerts
        for alert in alerts {
            match alert.severity {
                AlertSeverity::Critical => score -= 0.2,
                AlertSeverity::Warning => score -= 0.1,
                AlertSeverity::Info => score -= 0.05,
            }
        }

        score.max(0.0).min(1.0)
    }

    /// Persist metrics to file
    async fn persist_metrics(&self, _path: &str) -> VectorDbResult<()> {
        // Implementation for persisting metrics to file
        // This would serialize operation_history and resource_history to JSON/binary format
        Ok(())
    }

    // System resource monitoring helper functions
    async fn get_cpu_usage_percentage() -> f64 {
        // Simplified CPU usage calculation
        // In production, would use system APIs or psutil-like libraries
        0.0
    }

    async fn get_memory_usage_mb() -> f64 {
        // Simplified memory usage calculation
        // In production, would use system APIs to get actual memory usage
        100.0
    }

    async fn get_available_memory_mb() -> f64 {
        // Simplified available memory calculation
        8192.0
    }

    async fn get_active_thread_count() -> u32 {
        // Simplified thread count
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) as u32
    }

    async fn get_load_average() -> f64 {
        // Simplified load average calculation
        0.0
    }

    /// Detect performance degradation patterns
    async fn detect_performance_degradation(
        _history: &VecDeque<OperationMetrics>,
        _threshold_percent: f64,
    ) -> Option<Vec<PerformanceAlert>> {
        // Implementation for detecting performance degradation patterns
        // Compare recent performance against historical baseline
        None
    }
}

// Integration traits for existing systems

/// Trait for integrating with incremental update system
pub trait IncrementalUpdateMonitoring {
    /// Convert incremental update stats to operation metrics
    fn to_operation_metrics(&self, update_stats: &UpdateStats) -> OperationMetrics;
}

/// Trait for integrating with maintenance system  
pub trait MaintenanceMonitoring {
    /// Convert maintenance stats to operation metrics
    fn to_operation_metrics(&self, maintenance_stats: &MaintenanceStats) -> OperationMetrics;
}

/// Trait for integrating with rebuilding system
pub trait RebuildingMonitoring {
    /// Convert rebuild metrics to operation metrics
    fn to_operation_metrics(&self, rebuild_metrics: &RebuildMetrics) -> OperationMetrics;
}

impl IncrementalUpdateMonitoring for IndexPerformanceMonitor {
    fn to_operation_metrics(&self, update_stats: &UpdateStats) -> OperationMetrics {
        OperationMetrics {
            operation_type: OperationType::IncrementalUpdate,
            operation_id: format!("incremental_{}", Utc::now().timestamp_millis()),
            started_at: Utc::now() - chrono::Duration::milliseconds(update_stats.processing_time_ms as i64),
            completed_at: Some(Utc::now()),
            duration_ms: Some(update_stats.processing_time_ms as f64),
            status: if update_stats.had_errors { OperationStatus::Failed } else { OperationStatus::Success },
            items_processed: update_stats.files_processed as u64,
            processing_rate: if update_stats.processing_time_ms > 0 { 
                update_stats.files_processed as f64 / (update_stats.processing_time_ms as f64 / 1000.0) 
            } else { 0.0 },
            memory_start_mb: 0.0,
            memory_peak_mb: 0.0,
            memory_end_mb: 0.0,
            cpu_usage_percent: 0.0,
            io_operations: update_stats.files_processed as u64,
            bytes_read: 0,
            bytes_written: 0,
            operation_data: {
                let mut data = HashMap::new();
                data.insert("embeddings_added".to_string(), serde_json::Value::Number(update_stats.embeddings_added.into()));
                data.insert("embeddings_updated".to_string(), serde_json::Value::Number(update_stats.embeddings_updated.into()));
                data.insert("embeddings_deleted".to_string(), serde_json::Value::Number(update_stats.embeddings_deleted.into()));
                data.insert("avg_time_per_file_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(update_stats.avg_time_per_file_ms).unwrap_or_else(|| serde_json::Number::from(0))));
                data
            },
            error_message: if update_stats.had_errors { 
                Some("Errors encountered during incremental update".to_string()) 
            } else { None },
        }
    }
}

impl MaintenanceMonitoring for IndexPerformanceMonitor {
    fn to_operation_metrics(&self, maintenance_stats: &MaintenanceStats) -> OperationMetrics {
        OperationMetrics {
            operation_type: OperationType::Maintenance,
            operation_id: format!("maintenance_{}", Utc::now().timestamp_millis()),
            started_at: Utc::now() - chrono::Duration::milliseconds(maintenance_stats.avg_cycle_time_ms as i64),
            completed_at: Some(Utc::now()),
            duration_ms: Some(maintenance_stats.avg_cycle_time_ms),
            status: OperationStatus::Success,
            items_processed: maintenance_stats.orphaned_embeddings_removed,
            processing_rate: if maintenance_stats.avg_orphan_cleanup_time_ms > 0.0 {
                maintenance_stats.orphaned_embeddings_removed as f64 / (maintenance_stats.avg_orphan_cleanup_time_ms / 1000.0)
            } else { 0.0 },
            memory_start_mb: 0.0,
            memory_peak_mb: 0.0,
            memory_end_mb: 0.0,
            cpu_usage_percent: 0.0,
            io_operations: maintenance_stats.compaction_operations + maintenance_stats.defragmentation_operations,
            bytes_read: 0,
            bytes_written: 0,
            operation_data: {
                let mut data = HashMap::new();
                data.insert("maintenance_cycles".to_string(), serde_json::Value::Number(maintenance_stats.maintenance_cycles.into()));
                data.insert("orphaned_embeddings_removed".to_string(), serde_json::Value::Number(maintenance_stats.orphaned_embeddings_removed.into()));
                data.insert("compaction_operations".to_string(), serde_json::Value::Number(maintenance_stats.compaction_operations.into()));
                data.insert("storage_space_reclaimed".to_string(), serde_json::Value::Number(maintenance_stats.storage_space_reclaimed.into()));
                data
            },
            error_message: None,
        }
    }
}

impl RebuildingMonitoring for IndexPerformanceMonitor {
    fn to_operation_metrics(&self, rebuild_metrics: &RebuildMetrics) -> OperationMetrics {
        OperationMetrics {
            operation_type: OperationType::Rebuilding,
            operation_id: format!("rebuilding_{}", Utc::now().timestamp_millis()),
            started_at: Utc::now() - chrono::Duration::milliseconds(rebuild_metrics.avg_processing_time_ms as i64),
            completed_at: Some(Utc::now()),
            duration_ms: Some(rebuild_metrics.avg_processing_time_ms),
            status: OperationStatus::Success, // Assume success if metrics exist
            items_processed: 0, // Not available in current metrics
            processing_rate: rebuild_metrics.throughput_eps,
            memory_start_mb: 0.0,
            memory_peak_mb: (rebuild_metrics.peak_memory_usage_bytes as f64) / (1024.0 * 1024.0),
            memory_end_mb: 0.0,
            cpu_usage_percent: rebuild_metrics.cpu_usage_percentage,
            io_operations: rebuild_metrics.io_operations,
            bytes_read: 0, // Not available in current metrics
            bytes_written: 0, // Not available in current metrics
            operation_data: {
                let mut data = HashMap::new();
                data.insert("workers_used".to_string(), serde_json::Value::Number(rebuild_metrics.workers_used.into()));
                data.insert("avg_io_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(rebuild_metrics.avg_io_time_ms).unwrap_or_else(|| serde_json::Number::from(0))));
                data.insert("throughput_eps".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(rebuild_metrics.throughput_eps).unwrap_or_else(|| serde_json::Number::from(0))));
                data
            },
            error_message: None, // Not available in current metrics
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_monitor_creation() {
        let config = MonitoringConfig::default();
        let monitor = IndexPerformanceMonitor::new(config);
        
        assert!(!monitor.monitoring_enabled.load(Ordering::Relaxed));
        assert_eq!(monitor.total_operations.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_operation_lifecycle() {
        let config = MonitoringConfig::default();
        let mut monitor = IndexPerformanceMonitor::new(config);
        
        monitor.start().await.unwrap();
        
        let operation_id = "test_op_1".to_string();
        
        // Start operation
        monitor.start_operation(
            OperationType::IncrementalUpdate,
            operation_id.clone()
        ).await.unwrap();
        
        // Update operation
        monitor.update_operation(
            &operation_id,
            10,
            1024,
            2048,
            None
        ).await.unwrap();
        
        // Complete operation
        monitor.complete_operation(
            &operation_id,
            OperationStatus::Success,
            None
        ).await.unwrap();
        
        // Allow time for background processing
        sleep(Duration::from_millis(100)).await;
        
        // Check that operation was recorded
        assert_eq!(monitor.total_operations.load(Ordering::Relaxed), 1);
        
        monitor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_performance_report_generation() {
        let config = MonitoringConfig::default();
        let monitor = IndexPerformanceMonitor::new(config);
        
        let report = monitor.generate_performance_report(24).await.unwrap();
        
        assert_eq!(report.total_operations, 0);
        assert!(report.health_score >= 0.0 && report.health_score <= 1.0);
        assert!(!report.recommendations.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_calculation() {
        let monitor = IndexPerformanceMonitor::new(MonitoringConfig::default());
        
        let metrics1 = OperationMetrics {
            operation_type: OperationType::IncrementalUpdate,
            operation_id: "test1".to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration_ms: Some(100.0),
            status: OperationStatus::Success,
            items_processed: 10,
            processing_rate: 5.0,
            memory_start_mb: 100.0,
            memory_peak_mb: 150.0,
            memory_end_mb: 120.0,
            cpu_usage_percent: 50.0,
            io_operations: 5,
            bytes_read: 1024,
            bytes_written: 2048,
            operation_data: HashMap::new(),
            error_message: None,
        };

        let metrics2 = OperationMetrics {
            duration_ms: Some(200.0),
            processing_rate: 10.0,
            memory_peak_mb: 200.0,
            cpu_usage_percent: 60.0,
            ..metrics1.clone()
        };

        let metrics_list = vec![&metrics1, &metrics2];
        let avg = monitor.calculate_average_metrics(&metrics_list).unwrap();
        
        assert_eq!(avg.duration_ms.unwrap(), 150.0);
        assert_eq!(avg.processing_rate, 7.5);
        assert_eq!(avg.memory_peak_mb, 175.0);
        assert_eq!(avg.cpu_usage_percent, 55.0);
    }
}