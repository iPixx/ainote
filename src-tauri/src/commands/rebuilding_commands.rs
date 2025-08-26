//! Tauri Commands for Vector Database Index Rebuilding and Health Check Operations
//! 
//! This module provides Tauri commands for managing index rebuilding operations,
//! health checks, corruption detection, and recovery systems.

use std::sync::Arc;
use serde::{Serialize, Deserialize};

use crate::vector_db::{
    rebuilding::{RebuildingConfig, HealthCheckConfig, RebuildResult, HealthCheckResult, RebuildProgress},
    RebuildPhase, HealthStatus, RebuildMetrics
};
use crate::globals::VECTOR_DATABASE;

/// Configuration request for enabling index rebuilding
#[derive(Debug, Serialize, Deserialize)]
pub struct EnableRebuildingRequest {
    /// Enable parallel processing during rebuild
    pub enable_parallel_processing: Option<bool>,
    /// Number of parallel worker threads (0 = auto-detect)
    pub parallel_workers: Option<usize>,
    /// Batch size for processing embeddings during rebuild
    pub rebuild_batch_size: Option<usize>,
    /// Timeout for individual rebuild operations (seconds)
    pub operation_timeout_seconds: Option<u64>,
    /// Enable detailed progress reporting
    pub enable_progress_reporting: Option<bool>,
    /// Progress reporting interval (milliseconds)
    pub progress_report_interval_ms: Option<u64>,
    /// Enable health check validation after rebuild
    pub validate_after_rebuild: Option<bool>,
    /// Enable automatic backup before rebuild
    pub backup_before_rebuild: Option<bool>,
    /// Temporary directory for rebuild operations
    pub temp_directory: Option<String>,
    /// Enable debug logging during rebuild
    pub enable_debug_logging: Option<bool>,
}

impl EnableRebuildingRequest {
    /// Convert request to RebuildingConfig with defaults
    pub fn to_config(&self) -> RebuildingConfig {
        let mut config = RebuildingConfig::default();
        
        if let Some(parallel) = self.enable_parallel_processing {
            config.enable_parallel_processing = parallel;
        }
        if let Some(workers) = self.parallel_workers {
            config.parallel_workers = workers;
        }
        if let Some(batch_size) = self.rebuild_batch_size {
            config.rebuild_batch_size = batch_size;
        }
        if let Some(timeout) = self.operation_timeout_seconds {
            config.operation_timeout_seconds = timeout;
        }
        if let Some(reporting) = self.enable_progress_reporting {
            config.enable_progress_reporting = reporting;
        }
        if let Some(interval) = self.progress_report_interval_ms {
            config.progress_report_interval_ms = interval;
        }
        if let Some(validate) = self.validate_after_rebuild {
            config.validate_after_rebuild = validate;
        }
        if let Some(backup) = self.backup_before_rebuild {
            config.backup_before_rebuild = backup;
        }
        if let Some(temp_dir) = &self.temp_directory {
            config.temp_directory = Some(temp_dir.into());
        }
        if let Some(debug) = self.enable_debug_logging {
            config.enable_debug_logging = debug;
        }
        
        config
    }
}

/// Configuration request for enabling health checks
#[derive(Debug, Serialize, Deserialize)]
pub struct EnableHealthChecksRequest {
    /// Enable comprehensive integrity validation
    pub enable_integrity_validation: Option<bool>,
    /// Enable performance validation
    pub enable_performance_validation: Option<bool>,
    /// Enable corruption detection
    pub enable_corruption_detection: Option<bool>,
    /// Sample size for performance testing (percentage)
    pub performance_sample_percentage: Option<f64>,
    /// Target time for health checks (seconds)
    pub target_check_time_seconds: Option<u64>,
    /// Enable detailed reporting
    pub enable_detailed_reporting: Option<bool>,
}

impl EnableHealthChecksRequest {
    /// Convert request to HealthCheckConfig with defaults
    pub fn to_config(&self) -> HealthCheckConfig {
        let mut config = HealthCheckConfig::default();
        
        if let Some(integrity) = self.enable_integrity_validation {
            config.enable_integrity_validation = integrity;
        }
        if let Some(performance) = self.enable_performance_validation {
            config.enable_performance_validation = performance;
        }
        if let Some(corruption) = self.enable_corruption_detection {
            config.enable_corruption_detection = corruption;
        }
        if let Some(sample) = self.performance_sample_percentage {
            config.performance_sample_percentage = sample;
        }
        if let Some(target) = self.target_check_time_seconds {
            config.target_check_time_seconds = target;
        }
        if let Some(detailed) = self.enable_detailed_reporting {
            config.enable_detailed_reporting = detailed;
        }
        
        config
    }
}

/// Response for rebuilding operations
#[derive(Debug, Serialize, Deserialize)]
pub struct RebuildingResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Optional rebuild result details
    pub result: Option<RebuildResult>,
}

impl RebuildingResponse {
    /// Create a success response
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            result: None,
        }
    }
    
    /// Create a success response with rebuild result
    pub fn success_with_result(message: impl Into<String>, result: RebuildResult) -> Self {
        Self {
            success: true,
            message: message.into(),
            result: Some(result),
        }
    }
    
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            result: None,
        }
    }
}

/// Response for health check operations
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Optional health check result details
    pub result: Option<HealthCheckResult>,
}

impl HealthCheckResponse {
    /// Create a success response with health check result
    pub fn success_with_result(message: impl Into<String>, result: HealthCheckResult) -> Self {
        Self {
            success: true,
            message: message.into(),
            result: Some(result),
        }
    }
    
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            result: None,
        }
    }
}

/// Enable index rebuilding system for the vector database
/// 
/// This command initializes the index rebuilding system with the provided configuration
/// and prepares it for rebuild operations.
#[tauri::command]
pub async fn enable_index_rebuilding(
    request: EnableRebuildingRequest,
) -> Result<RebuildingResponse, String> {
    eprintln!("🏗️ Enable index rebuilding request: {:?}", request);
    
    let config = request.to_config();
    
    // Get mutable reference to the vector database
    let mut db_guard = VECTOR_DATABASE.write().await;
    if let Some(ref mut database) = *db_guard {
        match database.enable_index_rebuilding(config).await {
            Ok(_) => {
                Ok(RebuildingResponse::success("Index rebuilding system enabled successfully"))
            },
            Err(e) => {
                eprintln!("❌ Failed to enable index rebuilding: {}", e);
                Ok(RebuildingResponse::error(format!("Failed to enable index rebuilding: {}", e)))
            }
        }
    } else {
        Ok(RebuildingResponse::error("Vector database not initialized"))
    }
}

/// Enable health check system for the vector database
/// 
/// This command initializes the health check system with the provided configuration
/// and prepares it for health validation operations.
#[tauri::command]
pub async fn enable_health_checks(
    request: EnableHealthChecksRequest,
) -> Result<HealthCheckResponse, String> {
    eprintln!("🏥 Enable health checks request: {:?}", request);
    
    let config = request.to_config();
    
    // Get mutable reference to the vector database
    let mut db_guard = VECTOR_DATABASE.write().await;
    if let Some(ref mut database) = *db_guard {
        match database.enable_health_checks(config).await {
            Ok(_) => {
                Ok(HealthCheckResponse::success_with_result(
                    "Health check system enabled successfully",
                    HealthCheckResult {
                        overall_health: HealthStatus::Healthy,
                        check_time_ms: 0,
                        integrity_results: None,
                        performance_results: None,
                        corruption_results: None,
                        issues_found: Vec::new(),
                        recommendations: vec!["Health check system ready for use".to_string()],
                    }
                ))
            },
            Err(e) => {
                eprintln!("❌ Failed to enable health checks: {}", e);
                Ok(HealthCheckResponse::error(format!("Failed to enable health checks: {}", e)))
            }
        }
    } else {
        Ok(HealthCheckResponse::error("Vector database not initialized"))
    }
}

/// Perform a complete index rebuild
/// 
/// This command performs a full reconstruction of the vector database index
/// with progress tracking and optional parallel processing.
#[tauri::command]
pub async fn rebuild_index_complete() -> Result<RebuildingResponse, String> {
    eprintln!("🔄 Starting complete index rebuild...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.rebuild_index_full().await {
            Ok(result) => {
                let message = if result.success {
                    format!(
                        "Index rebuild completed successfully: {} embeddings processed in {}ms",
                        result.embeddings_processed,
                        result.total_time_ms
                    )
                } else {
                    format!(
                        "Index rebuild completed with issues: {} embeddings processed, {} failed",
                        result.embeddings_processed,
                        result.embeddings_failed
                    )
                };
                Ok(RebuildingResponse::success_with_result(message, result))
            },
            Err(e) => {
                eprintln!("❌ Failed to rebuild index: {}", e);
                Ok(RebuildingResponse::error(format!("Failed to rebuild index: {}", e)))
            }
        }
    } else {
        Ok(RebuildingResponse::error("Vector database not initialized"))
    }
}

/// Cancel any currently running index rebuild operation
/// 
/// This command allows graceful cancellation of long-running rebuild operations.
#[tauri::command]
pub async fn cancel_index_rebuild() -> Result<RebuildingResponse, String> {
    eprintln!("⏹️ Cancelling index rebuild...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        database.cancel_index_rebuild().await;
        Ok(RebuildingResponse::success("Index rebuild cancellation requested"))
    } else {
        Ok(RebuildingResponse::error("Vector database not initialized"))
    }
}

/// Perform a comprehensive health check of the index
/// 
/// This command performs integrity validation, performance testing, and corruption
/// detection to assess the overall health of the vector database index.
#[tauri::command]
pub async fn perform_comprehensive_health_check() -> Result<HealthCheckResponse, String> {
    eprintln!("🏥 Performing comprehensive health check...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.perform_health_check().await {
            Ok(result) => {
                let message = format!(
                    "Health check completed: {} ({} issues found, {}ms)",
                    match result.overall_health {
                        HealthStatus::Healthy => "Index is healthy",
                        HealthStatus::Warning => "Index has minor issues",
                        HealthStatus::Degraded => "Index has performance issues",
                        HealthStatus::Critical => "Index has critical issues",
                    },
                    result.issues_found.len(),
                    result.check_time_ms
                );
                Ok(HealthCheckResponse::success_with_result(message, result))
            },
            Err(e) => {
                eprintln!("❌ Failed to perform health check: {}", e);
                Ok(HealthCheckResponse::error(format!("Failed to perform health check: {}", e)))
            }
        }
    } else {
        Ok(HealthCheckResponse::error("Vector database not initialized"))
    }
}

/// Perform a quick health check focused on performance
/// 
/// This command performs a faster health check that meets the <1 second target requirement.
#[tauri::command]
pub async fn perform_quick_health_check() -> Result<HealthCheckResponse, String> {
    eprintln!("⚡ Performing quick health check...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.perform_quick_health_check().await {
            Ok(result) => {
                let meets_target = result.meets_performance_targets();
                let message = format!(
                    "Quick health check completed: {} ({}ms) - Performance target {}",
                    match result.overall_health {
                        HealthStatus::Healthy => "Healthy",
                        HealthStatus::Warning => "Warning", 
                        HealthStatus::Degraded => "Degraded",
                        HealthStatus::Critical => "Critical",
                    },
                    result.check_time_ms,
                    if meets_target { "MET" } else { "NOT MET" }
                );
                Ok(HealthCheckResponse::success_with_result(message, result))
            },
            Err(e) => {
                eprintln!("❌ Failed to perform quick health check: {}", e);
                Ok(HealthCheckResponse::error(format!("Failed to perform quick health check: {}", e)))
            }
        }
    } else {
        Ok(HealthCheckResponse::error("Vector database not initialized"))
    }
}

/// Detect potential index corruption
/// 
/// This command performs focused corruption detection to identify data integrity
/// issues that may require index rebuilding or recovery.
#[tauri::command]
pub async fn detect_index_corruption() -> Result<HealthCheckResponse, String> {
    eprintln!("🔎 Detecting index corruption...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.detect_index_corruption().await {
            Ok(result) => {
                let corruption_detected = result.corruption_results
                    .as_ref()
                    .map(|r| r.corruption_detected)
                    .unwrap_or(false);
                
                let message = if corruption_detected {
                    let severity = result.corruption_results
                        .as_ref()
                        .map(|r| format!("{:?}", r.corruption_severity))
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    format!(
                        "Corruption detected: {} severity ({} issues found, {}ms)",
                        severity,
                        result.issues_found.len(),
                        result.check_time_ms
                    )
                } else {
                    format!(
                        "No corruption detected ({} issues found, {}ms)",
                        result.issues_found.len(),
                        result.check_time_ms
                    )
                };
                
                Ok(HealthCheckResponse::success_with_result(message, result))
            },
            Err(e) => {
                eprintln!("❌ Failed to detect corruption: {}", e);
                Ok(HealthCheckResponse::error(format!("Failed to detect corruption: {}", e)))
            }
        }
    } else {
        Ok(HealthCheckResponse::error("Vector database not initialized"))
    }
}

/// Get the status of rebuilding and health check systems
/// 
/// This command returns information about whether the systems are enabled and configured.
#[tauri::command]
pub async fn get_rebuilding_health_status() -> Result<RebuildingResponse, String> {
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        let rebuilding_enabled = database.is_rebuilding_enabled();
        let health_checks_enabled = database.is_health_checks_enabled();
        
        let status_message = match (rebuilding_enabled, health_checks_enabled) {
            (true, true) => "Both rebuilding and health check systems are enabled".to_string(),
            (true, false) => "Rebuilding system enabled, health checks disabled".to_string(),
            (false, true) => "Health check system enabled, rebuilding disabled".to_string(),
            (false, false) => "Neither rebuilding nor health check systems are enabled".to_string(),
        };
        
        Ok(RebuildingResponse::success(status_message))
    } else {
        Ok(RebuildingResponse::error("Vector database not initialized"))
    }
}

/// Recover from index corruption by performing automatic rebuild
/// 
/// This command detects corruption and automatically performs a rebuild if corruption is found.
#[tauri::command]
pub async fn recover_from_corruption() -> Result<RebuildingResponse, String> {
    eprintln!("🔧 Starting corruption recovery...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        // First, detect corruption
        match database.detect_index_corruption().await {
            Ok(health_result) => {
                let corruption_detected = health_result.corruption_results
                    .as_ref()
                    .map(|r| r.corruption_detected)
                    .unwrap_or(false);
                
                if corruption_detected {
                    eprintln!("🚨 Corruption detected, starting automatic rebuild...");
                    
                    // Perform rebuild
                    match database.rebuild_index_full().await {
                        Ok(rebuild_result) => {
                            let message = if rebuild_result.success {
                                format!(
                                    "Corruption recovery completed successfully: {} embeddings processed in {}ms",
                                    rebuild_result.embeddings_processed,
                                    rebuild_result.total_time_ms
                                )
                            } else {
                                format!(
                                    "Corruption recovery completed with issues: {} processed, {} failed",
                                    rebuild_result.embeddings_processed,
                                    rebuild_result.embeddings_failed
                                )
                            };
                            Ok(RebuildingResponse::success_with_result(message, rebuild_result))
                        },
                        Err(e) => {
                            eprintln!("❌ Failed to recover from corruption: {}", e);
                            Ok(RebuildingResponse::error(format!("Failed to recover from corruption: {}", e)))
                        }
                    }
                } else {
                    Ok(RebuildingResponse::success("No corruption detected - recovery not needed"))
                }
            },
            Err(e) => {
                eprintln!("❌ Failed to detect corruption: {}", e);
                Ok(RebuildingResponse::error(format!("Failed to detect corruption for recovery: {}", e)))
            }
        }
    } else {
        Ok(RebuildingResponse::error("Vector database not initialized"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_rebuilding_request() {
        let request = EnableRebuildingRequest {
            enable_parallel_processing: Some(true),
            parallel_workers: Some(4),
            rebuild_batch_size: Some(50),
            operation_timeout_seconds: Some(600),
            enable_progress_reporting: Some(true),
            progress_report_interval_ms: Some(500),
            validate_after_rebuild: Some(true),
            backup_before_rebuild: Some(false),
            temp_directory: Some("/tmp/rebuild".to_string()),
            enable_debug_logging: Some(true),
        };
        
        let config = request.to_config();
        
        assert!(config.enable_parallel_processing);
        assert_eq!(config.parallel_workers, 4);
        assert_eq!(config.rebuild_batch_size, 50);
        assert_eq!(config.operation_timeout_seconds, 600);
        assert!(config.enable_progress_reporting);
        assert_eq!(config.progress_report_interval_ms, 500);
        assert!(config.validate_after_rebuild);
        assert!(!config.backup_before_rebuild);
        assert_eq!(config.temp_directory, Some("/tmp/rebuild".into()));
        assert!(config.enable_debug_logging);
    }

    #[test]
    fn test_enable_health_checks_request() {
        let request = EnableHealthChecksRequest {
            enable_integrity_validation: Some(true),
            enable_performance_validation: Some(true),
            enable_corruption_detection: Some(false),
            performance_sample_percentage: Some(0.2),
            target_check_time_seconds: Some(2),
            enable_detailed_reporting: Some(false),
        };
        
        let config = request.to_config();
        
        assert!(config.enable_integrity_validation);
        assert!(config.enable_performance_validation);
        assert!(!config.enable_corruption_detection);
        assert_eq!(config.performance_sample_percentage, 0.2);
        assert_eq!(config.target_check_time_seconds, 2);
        assert!(!config.enable_detailed_reporting);
    }

    #[test]
    fn test_rebuilding_response_creation() {
        let success_response = RebuildingResponse::success("Rebuild completed");
        assert!(success_response.success);
        assert_eq!(success_response.message, "Rebuild completed");
        assert!(success_response.result.is_none());
        
        let error_response = RebuildingResponse::error("Rebuild failed");
        assert!(!error_response.success);
        assert_eq!(error_response.message, "Rebuild failed");
        assert!(error_response.result.is_none());
    }

    #[test]
    fn test_health_check_response_creation() {
        let health_result = HealthCheckResult {
            overall_health: HealthStatus::Healthy,
            check_time_ms: 500,
            integrity_results: None,
            performance_results: None,
            corruption_results: None,
            issues_found: Vec::new(),
            recommendations: Vec::new(),
        };
        
        let response = HealthCheckResponse::success_with_result("Check completed", health_result.clone());
        
        assert!(response.success);
        assert_eq!(response.message, "Check completed");
        assert!(response.result.is_some());
        
        let error_response = HealthCheckResponse::error("Check failed");
        assert!(!error_response.success);
        assert_eq!(error_response.message, "Check failed");
        assert!(error_response.result.is_none());
    }
}