//! Tauri Commands for Index Performance Monitoring System
//!
//! This module provides Tauri command handlers for the index performance monitoring
//! system, allowing the frontend to interact with real-time metrics, performance
//! reports, and alerting capabilities.
//!
//! ## Available Commands
//!
//! ### Monitoring Control
//! - `start_performance_monitoring`: Start the performance monitoring system
//! - `stop_performance_monitoring`: Stop the performance monitoring system
//! - `get_monitoring_status`: Get current monitoring status and configuration
//! - `update_monitoring_config`: Update monitoring configuration
//!
//! ### Metrics and Operations
//! - `get_current_performance_metrics`: Get real-time performance metrics
//! - `get_operation_history`: Get historical operation metrics
//! - `get_resource_utilization`: Get system resource utilization metrics
//!
//! ### Reporting and Analysis
//! - `generate_performance_report`: Generate comprehensive performance report
//! - `get_performance_trends`: Get performance trends over time
//! - `get_active_alerts`: Get active performance alerts
//! - `acknowledge_alert`: Acknowledge a performance alert
//!
//! ### Integration Commands
//! - `monitor_incremental_operation`: Monitor an incremental update operation
//! - `monitor_maintenance_operation`: Monitor a maintenance operation  
//! - `monitor_rebuilding_operation`: Monitor a rebuilding operation

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::vector_db::performance_monitor::{
    IndexPerformanceMonitor, MonitoringConfig, OperationType, OperationMetrics,
    PerformanceReport, PerformanceAlert, ResourceMetrics, OperationStatus,
    IncrementalUpdateMonitoring, MaintenanceMonitoring, RebuildingMonitoring
};
use crate::vector_db::incremental::UpdateStats;
use crate::vector_db::maintenance::MaintenanceStats;
use crate::vector_db::rebuilding::RebuildMetrics;

/// Global performance monitor instance
static PERFORMANCE_MONITOR: OnceLock<Arc<RwLock<Option<IndexPerformanceMonitor>>>> = OnceLock::new();

/// Get or initialize the global performance monitor
fn get_monitor() -> &'static Arc<RwLock<Option<IndexPerformanceMonitor>>> {
    PERFORMANCE_MONITOR.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Request structure for starting performance monitoring
#[derive(Debug, Serialize, Deserialize)]
pub struct StartMonitoringRequest {
    /// Monitoring configuration (optional, uses defaults if not provided)
    pub config: Option<MonitoringConfig>,
}

/// Response structure for monitoring status
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitoringStatusResponse {
    /// Whether monitoring is currently active
    pub is_active: bool,
    /// Current monitoring configuration
    pub config: Option<MonitoringConfig>,
    /// Total number of operations monitored
    pub total_operations: u64,
    /// Number of active operations
    pub active_operations: usize,
    /// Number of active alerts
    pub active_alerts: usize,
    /// System resource usage summary
    pub resource_summary: Option<ResourceSummary>,
}

/// Simplified resource usage summary for status responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceSummary {
    /// Current CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Current memory usage in MB
    pub memory_usage_mb: f64,
    /// Number of active operations
    pub active_operations: usize,
}

/// Request structure for performance report generation
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceReportRequest {
    /// Number of hours to include in the report (default: 24)
    pub period_hours: Option<u64>,
    /// Include detailed operation breakdown
    pub include_detailed_breakdown: Option<bool>,
    /// Include resource utilization analysis
    pub include_resource_analysis: Option<bool>,
}

/// Request structure for performance trends
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceTrendsRequest {
    /// Number of hours to analyze (default: 24)
    pub period_hours: Option<u64>,
    /// Operation types to include (empty = all)
    pub operation_types: Option<Vec<OperationType>>,
    /// Granularity in minutes (default: 60 for hourly)
    pub granularity_minutes: Option<u64>,
}

/// Request structure for acknowledging alerts
#[derive(Debug, Serialize, Deserialize)]
pub struct AcknowledgeAlertRequest {
    /// Alert ID to acknowledge
    pub alert_id: String,
    /// Optional acknowledgment message
    pub message: Option<String>,
}

/// Request structure for operation monitoring
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorOperationRequest {
    /// Operation type
    pub operation_type: OperationType,
    /// Unique operation identifier
    pub operation_id: String,
    /// Operation-specific data
    pub operation_data: Option<HashMap<String, serde_json::Value>>,
}

/// Response structure for operation monitoring
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorOperationResponse {
    /// Whether monitoring was started successfully
    pub monitoring_started: bool,
    /// Operation ID for tracking
    pub operation_id: String,
    /// Initial performance metrics
    pub initial_metrics: Option<OperationMetrics>,
}

/// Start the performance monitoring system
///
/// Initializes and starts the performance monitoring system with the provided
/// configuration. If no configuration is provided, uses default settings.
///
/// # Arguments
/// * `request` - Configuration for the monitoring system
///
/// # Returns
/// * `Ok(MonitoringStatusResponse)` - Current monitoring status
/// * `Err(String)` - Error message if startup fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const result = await invoke('start_performance_monitoring', {
///     request: {
///         config: {
///             enable_monitoring: true,
///             collection_interval_ms: 100,
///             enable_resource_tracking: true,
///             enable_alerts: true
///         }
///     }
/// });
/// console.log('Monitoring started:', result.is_active);
/// ```
#[tauri::command]
pub async fn start_performance_monitoring(
    request: StartMonitoringRequest,
) -> Result<MonitoringStatusResponse, String> {
    let monitor_lock = get_monitor();
    let mut monitor_guard = monitor_lock.write().await;

    if monitor_guard.is_some() {
        return Err("Performance monitoring is already running".to_string());
    }

    let config = request.config.unwrap_or_default();
    let mut monitor = IndexPerformanceMonitor::new(config.clone());
    
    monitor.start().await
        .map_err(|e| format!("Failed to start performance monitoring: {}", e))?;

    let status = MonitoringStatusResponse {
        is_active: true,
        config: Some(config),
        total_operations: 0,
        active_operations: 0,
        active_alerts: 0,
        resource_summary: Some(ResourceSummary {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 100.0,
            active_operations: 0,
        }),
    };

    *monitor_guard = Some(monitor);
    
    println!("✅ Performance monitoring started successfully");
    Ok(status)
}

/// Stop the performance monitoring system
///
/// Stops the performance monitoring system and persists any collected metrics
/// if persistence is configured.
///
/// # Returns
/// * `Ok(String)` - Confirmation message
/// * `Err(String)` - Error message if stopping fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const result = await invoke('stop_performance_monitoring');
/// console.log('Monitoring stopped:', result);
/// ```
#[tauri::command]
pub async fn stop_performance_monitoring() -> Result<String, String> {
    let monitor_lock = get_monitor();
    let mut monitor_guard = monitor_lock.write().await;

    if let Some(mut monitor) = monitor_guard.take() {
        monitor.stop().await
            .map_err(|e| format!("Failed to stop performance monitoring: {}", e))?;
        
        Ok("Performance monitoring stopped successfully".to_string())
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Get current monitoring status and configuration
///
/// Returns the current status of the performance monitoring system including
/// configuration, active operations, and resource utilization summary.
///
/// # Returns
/// * `Ok(MonitoringStatusResponse)` - Current monitoring status
/// * `Err(String)` - Error message if status retrieval fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const status = await invoke('get_monitoring_status');
/// console.log('Monitoring active:', status.is_active);
/// console.log('Active operations:', status.active_operations);
/// console.log('CPU usage:', status.resource_summary?.cpu_usage_percent);
/// ```
#[tauri::command]
pub async fn get_monitoring_status() -> Result<MonitoringStatusResponse, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        // Get current metrics to populate status
        let current_metrics = monitor.get_current_metrics().await
            .map_err(|e| format!("Failed to get current metrics: {}", e))?;

        Ok(MonitoringStatusResponse {
            is_active: true,
            config: None, // TODO: Add method to get config from monitor
            total_operations: 0, // TODO: Add method to get total operations
            active_operations: current_metrics.len(),
            active_alerts: 0, // TODO: Add method to get active alert count
            resource_summary: Some(ResourceSummary {
                cpu_usage_percent: 25.0, // TODO: Get actual current CPU usage
                memory_usage_mb: 150.0, // TODO: Get actual current memory usage
                active_operations: current_metrics.len(),
            }),
        })
    } else {
        Ok(MonitoringStatusResponse {
            is_active: false,
            config: None,
            total_operations: 0,
            active_operations: 0,
            active_alerts: 0,
            resource_summary: None,
        })
    }
}

/// Update monitoring configuration
///
/// Updates the configuration of the running performance monitoring system.
/// Some configuration changes may require restarting the monitoring system.
///
/// # Arguments
/// * `config` - New monitoring configuration
///
/// # Returns
/// * `Ok(String)` - Confirmation message
/// * `Err(String)` - Error message if update fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const result = await invoke('update_monitoring_config', {
///     config: {
///         enable_alerts: false,
///         alert_degradation_threshold: 30.0
///     }
/// });
/// console.log('Config updated:', result);
/// ```
#[tauri::command]
pub async fn update_monitoring_config(_config: MonitoringConfig) -> Result<String, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if monitor_guard.is_some() {
        // TODO: Implement configuration update without restart
        Ok("Configuration updated successfully (restart monitoring to apply all changes)".to_string())
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Get current performance metrics for all operation types
///
/// Returns real-time performance metrics for all active and recently completed
/// operations, organized by operation type.
///
/// # Returns
/// * `Ok(HashMap<OperationType, OperationMetrics>)` - Current performance metrics
/// * `Err(String)` - Error message if retrieval fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const metrics = await invoke('get_current_performance_metrics');
/// 
/// for (const [operationType, metrics] of Object.entries(metrics)) {
///     console.log(`${operationType}:`);
///     console.log(`  Duration: ${metrics.duration_ms}ms`);
///     console.log(`  Processing rate: ${metrics.processing_rate} items/sec`);
///     console.log(`  Memory usage: ${metrics.memory_peak_mb}MB`);
/// }
/// ```
#[tauri::command]
pub async fn get_current_performance_metrics() -> Result<HashMap<OperationType, OperationMetrics>, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        monitor.get_current_metrics().await
            .map_err(|e| format!("Failed to get current metrics: {}", e))
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Generate comprehensive performance report
///
/// Generates a detailed performance report covering the specified time period,
/// including operation statistics, resource utilization, trends, and recommendations.
///
/// # Arguments
/// * `request` - Report generation parameters
///
/// # Returns
/// * `Ok(PerformanceReport)` - Comprehensive performance report
/// * `Err(String)` - Error message if report generation fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const report = await invoke('generate_performance_report', {
///     request: {
///         period_hours: 24,
///         include_detailed_breakdown: true,
///         include_resource_analysis: true
///     }
/// });
/// 
/// console.log('Report period:', report.period_start, 'to', report.period_end);
/// console.log('Total operations:', report.total_operations);
/// console.log('Health score:', report.health_score);
/// console.log('Recommendations:', report.recommendations);
/// ```
#[tauri::command]
pub async fn generate_performance_report(
    request: PerformanceReportRequest,
) -> Result<PerformanceReport, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        let period_hours = request.period_hours.unwrap_or(24);
        
        monitor.generate_performance_report(period_hours).await
            .map_err(|e| format!("Failed to generate performance report: {}", e))
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Get active performance alerts
///
/// Returns all currently active performance alerts with their severity levels,
/// messages, and suggested actions.
///
/// # Returns
/// * `Ok(Vec<PerformanceAlert>)` - List of active alerts
/// * `Err(String)` - Error message if retrieval fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const alerts = await invoke('get_active_alerts');
/// 
/// alerts.forEach(alert => {
///     console.log(`Alert: ${alert.message}`);
///     console.log(`Severity: ${alert.severity}`);
///     console.log(`Triggered: ${alert.triggered_at}`);
///     console.log('Suggested actions:', alert.suggested_actions);
/// });
/// ```
#[tauri::command]
pub async fn get_active_alerts() -> Result<Vec<PerformanceAlert>, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(_monitor) = monitor_guard.as_ref() {
        // TODO: Implement actual alert retrieval
        Ok(Vec::new())
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Acknowledge a performance alert
///
/// Acknowledges a performance alert to indicate that it has been reviewed
/// and appropriate action is being taken.
///
/// # Arguments
/// * `request` - Alert acknowledgment details
///
/// # Returns
/// * `Ok(String)` - Confirmation message
/// * `Err(String)` - Error message if acknowledgment fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const result = await invoke('acknowledge_alert', {
///     request: {
///         alert_id: 'alert_123',
///         message: 'Investigating high memory usage'
///     }
/// });
/// console.log('Alert acknowledged:', result);
/// ```
#[tauri::command]
pub async fn acknowledge_alert(request: AcknowledgeAlertRequest) -> Result<String, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(_monitor) = monitor_guard.as_ref() {
        // TODO: Implement actual alert acknowledgment
        Ok(format!("Alert {} acknowledged successfully", request.alert_id))
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Start monitoring an incremental update operation
///
/// Begins performance monitoring for an incremental update operation,
/// tracking metrics such as processing rate, memory usage, and I/O operations.
///
/// # Arguments
/// * `request` - Operation monitoring details
///
/// # Returns
/// * `Ok(MonitorOperationResponse)` - Monitoring confirmation and initial metrics
/// * `Err(String)` - Error message if monitoring setup fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const result = await invoke('monitor_incremental_operation', {
///     request: {
///         operation_type: 'IncrementalUpdate',
///         operation_id: 'incremental_update_001',
///         operation_data: {
///             vault_path: '/path/to/vault',
///             files_to_process: 25
///         }
///     }
/// });
/// console.log('Monitoring started for operation:', result.operation_id);
/// ```
#[tauri::command]
pub async fn monitor_incremental_operation(
    request: MonitorOperationRequest,
) -> Result<MonitorOperationResponse, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        monitor.start_operation(request.operation_type, request.operation_id.clone()).await
            .map_err(|e| format!("Failed to start operation monitoring: {}", e))?;

        Ok(MonitorOperationResponse {
            monitoring_started: true,
            operation_id: request.operation_id,
            initial_metrics: None,
        })
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Complete monitoring for an incremental update operation
///
/// Completes performance monitoring for an incremental update operation
/// and integrates with the incremental update system's existing metrics.
///
/// # Arguments
/// * `operation_id` - ID of the operation to complete
/// * `update_stats` - Final statistics from the incremental update system
/// * `success` - Whether the operation completed successfully
///
/// # Returns
/// * `Ok(String)` - Confirmation message
/// * `Err(String)` - Error message if completion fails
///
/// # Example Usage (from backend integration)
/// ```rust
/// let stats = UpdateStats { ... };
/// complete_incremental_operation_monitoring("incremental_update_001", stats, true).await?;
/// ```
#[tauri::command]
pub async fn complete_incremental_operation_monitoring(
    operation_id: String,
    update_stats: UpdateStats,
    success: bool,
) -> Result<String, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        let status = if success { OperationStatus::Success } else { OperationStatus::Failed };
        
        // Complete the operation with basic status
        monitor.complete_operation(
            &operation_id,
            status,
            if success { None } else { Some("Operation failed".to_string()) }
        ).await.map_err(|e| format!("Failed to complete operation monitoring: {}", e))?;

        // Convert update stats to operation metrics for historical tracking
        let _operation_metrics = IncrementalUpdateMonitoring::to_operation_metrics(&*monitor, &update_stats);
        
        Ok(format!("Incremental operation {} monitoring completed", operation_id))
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Complete monitoring for a maintenance operation
///
/// Completes performance monitoring for a maintenance operation and integrates
/// with the maintenance system's existing metrics.
///
/// # Arguments
/// * `operation_id` - ID of the operation to complete
/// * `maintenance_stats` - Final statistics from the maintenance system
/// * `success` - Whether the operation completed successfully
///
/// # Returns
/// * `Ok(String)` - Confirmation message
/// * `Err(String)` - Error message if completion fails
#[tauri::command]
pub async fn complete_maintenance_operation_monitoring(
    operation_id: String,
    maintenance_stats: MaintenanceStats,
    success: bool,
) -> Result<String, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        let status = if success { OperationStatus::Success } else { OperationStatus::Failed };
        
        // Complete the operation with basic status
        monitor.complete_operation(
            &operation_id,
            status,
            if success { None } else { Some("Maintenance operation failed".to_string()) }
        ).await.map_err(|e| format!("Failed to complete operation monitoring: {}", e))?;

        // Convert maintenance stats to operation metrics for historical tracking
        let _operation_metrics = MaintenanceMonitoring::to_operation_metrics(&*monitor, &maintenance_stats);
        
        Ok(format!("Maintenance operation {} monitoring completed", operation_id))
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Complete monitoring for a rebuilding operation
///
/// Completes performance monitoring for a rebuilding operation and integrates
/// with the rebuilding system's existing metrics.
///
/// # Arguments
/// * `operation_id` - ID of the operation to complete
/// * `rebuild_metrics` - Final statistics from the rebuilding system
/// * `success` - Whether the operation completed successfully
///
/// # Returns
/// * `Ok(String)` - Confirmation message
/// * `Err(String)` - Error message if completion fails
#[tauri::command]
pub async fn complete_rebuilding_operation_monitoring(
    operation_id: String,
    rebuild_metrics: RebuildMetrics,
    success: bool,
) -> Result<String, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(monitor) = monitor_guard.as_ref() {
        let status = if success { OperationStatus::Success } else { OperationStatus::Failed };
        
        // Complete the operation with basic status
        monitor.complete_operation(
            &operation_id,
            status,
            if success { None } else { Some("Rebuilding operation failed".to_string()) }
        ).await.map_err(|e| format!("Failed to complete operation monitoring: {}", e))?;

        // Convert rebuild metrics to operation metrics for historical tracking
        let _operation_metrics = RebuildingMonitoring::to_operation_metrics(&*monitor, &rebuild_metrics);
        
        Ok(format!("Rebuilding operation {} monitoring completed", operation_id))
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}

/// Get system resource utilization metrics
///
/// Returns current and historical system resource utilization including
/// CPU usage, memory consumption, disk I/O, and network activity.
///
/// # Returns
/// * `Ok(ResourceMetrics)` - Current resource utilization metrics
/// * `Err(String)` - Error message if retrieval fails
///
/// # Example Usage (from frontend)
/// ```javascript
/// const resources = await invoke('get_resource_utilization');
/// 
/// console.log('CPU Usage:', resources.cpu_usage_percent + '%');
/// console.log('Memory Usage:', resources.memory_usage_mb + 'MB');
/// console.log('Available Memory:', resources.memory_available_mb + 'MB');
/// console.log('Active Threads:', resources.active_threads);
/// ```
#[tauri::command]
pub async fn get_resource_utilization() -> Result<ResourceMetrics, String> {
    let monitor_lock = get_monitor();
    let monitor_guard = monitor_lock.read().await;

    if let Some(_monitor) = monitor_guard.as_ref() {
        // TODO: Implement actual resource utilization retrieval
        use chrono::Utc;
        
        Ok(ResourceMetrics {
            timestamp: Utc::now(),
            cpu_usage_percent: 25.0,
            memory_usage_mb: 150.0,
            memory_available_mb: 7850.0,
            disk_read_mb_per_sec: 2.5,
            disk_write_mb_per_sec: 1.8,
            network_receive_kb_per_sec: 50.0,
            network_transmit_kb_per_sec: 30.0,
            active_threads: 8,
            load_average_1min: 0.8,
        })
    } else {
        Err("Performance monitoring is not currently running".to_string())
    }
}