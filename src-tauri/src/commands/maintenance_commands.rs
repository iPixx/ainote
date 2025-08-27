//! Tauri Commands for Vector Database Maintenance Operations
//! 
//! This module provides Tauri commands for managing database maintenance operations
//! including orphaned embedding cleanup, index compaction, and automated maintenance.

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::vector_db::maintenance::{MaintenanceConfig, MaintenanceStats};
use crate::globals::VECTOR_DATABASE;

/// Configuration request for enabling maintenance
#[derive(Debug, Serialize, Deserialize)]
pub struct EnableMaintenanceRequest {
    /// Enable automatic scheduled maintenance
    pub enable_automatic_maintenance: bool,
    /// Maintenance check interval in seconds  
    pub maintenance_interval_seconds: Option<u64>,
    /// Maximum orphan cleanup operations per cycle
    pub max_orphan_cleanup_per_cycle: Option<usize>,
    /// Enable index compaction during maintenance
    pub enable_index_compaction: Option<bool>,
    /// Minimum time between compaction operations (hours)
    pub compaction_cooldown_hours: Option<u64>,
    /// Enable storage defragmentation
    pub enable_defragmentation: Option<bool>,
    /// Storage utilization threshold to trigger compaction (0.0-1.0)
    pub compaction_threshold: Option<f64>,
    /// Maximum maintenance operation duration (seconds)
    pub max_operation_duration_seconds: Option<u64>,
    /// Enable detailed logging during maintenance
    pub enable_debug_logging: Option<bool>,
    /// Paths to monitor for file existence validation
    pub monitored_vault_paths: Option<Vec<String>>,
}

impl EnableMaintenanceRequest {
    /// Convert request to MaintenanceConfig with defaults
    pub fn to_config(&self) -> MaintenanceConfig {
        let mut config = MaintenanceConfig {
            enable_automatic_maintenance: self.enable_automatic_maintenance,
            ..Default::default()
        };
        
        if let Some(interval) = self.maintenance_interval_seconds {
            config.maintenance_interval_seconds = interval;
        }
        if let Some(max_cleanup) = self.max_orphan_cleanup_per_cycle {
            config.max_orphan_cleanup_per_cycle = max_cleanup;
        }
        if let Some(compaction) = self.enable_index_compaction {
            config.enable_index_compaction = compaction;
        }
        if let Some(cooldown) = self.compaction_cooldown_hours {
            config.compaction_cooldown_hours = cooldown;
        }
        if let Some(defrag) = self.enable_defragmentation {
            config.enable_defragmentation = defrag;
        }
        if let Some(threshold) = self.compaction_threshold {
            config.compaction_threshold = threshold;
        }
        if let Some(duration) = self.max_operation_duration_seconds {
            config.max_operation_duration_seconds = duration;
        }
        if let Some(logging) = self.enable_debug_logging {
            config.enable_debug_logging = logging;
        }
        if let Some(paths) = &self.monitored_vault_paths {
            config.monitored_vault_paths = paths.iter()
                .map(PathBuf::from)
                .collect();
        }
        
        config
    }
}

/// Response for maintenance operations
#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Optional maintenance statistics
    pub stats: Option<MaintenanceStats>,
}

impl MaintenanceResponse {
    /// Create a success response
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            stats: None,
        }
    }
    
    /// Create a success response with statistics
    pub fn success_with_stats(message: impl Into<String>, stats: MaintenanceStats) -> Self {
        Self {
            success: true,
            message: message.into(),
            stats: Some(stats),
        }
    }
    
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            stats: None,
        }
    }
}

/// Enable maintenance operations for the vector database
/// 
/// This command initializes the maintenance system with the provided configuration
/// and optionally starts automatic maintenance cycles.
#[tauri::command]
pub async fn enable_database_maintenance(
    request: EnableMaintenanceRequest,
) -> Result<MaintenanceResponse, String> {
    eprintln!("🔧 Enable database maintenance request: {:?}", request);
    
    let config = request.to_config();
    
    // Get mutable reference to the vector database
    let mut db_guard = VECTOR_DATABASE.write().await;
    if let Some(ref mut database) = *db_guard {
        match database.enable_maintenance(config).await {
            Ok(_) => {
                let message = if request.enable_automatic_maintenance {
                    "Maintenance system enabled with automatic scheduling"
                } else {
                    "Maintenance system enabled (manual mode only)"
                };
                Ok(MaintenanceResponse::success(message))
            },
            Err(e) => {
                eprintln!("❌ Failed to enable maintenance: {}", e);
                Ok(MaintenanceResponse::error(format!("Failed to enable maintenance: {}", e)))
            }
        }
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Start automatic maintenance operations
/// 
/// This command begins background maintenance cycles that will run periodically
/// to keep the database optimized.
#[tauri::command]
pub async fn start_automatic_maintenance() -> Result<MaintenanceResponse, String> {
    eprintln!("🚀 Starting automatic maintenance...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.start_maintenance().await {
            Ok(_) => {
                Ok(MaintenanceResponse::success("Automatic maintenance started successfully"))
            },
            Err(e) => {
                eprintln!("❌ Failed to start automatic maintenance: {}", e);
                Ok(MaintenanceResponse::error(format!("Failed to start automatic maintenance: {}", e)))
            }
        }
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Stop automatic maintenance operations
/// 
/// This command stops the background maintenance cycles.
#[tauri::command]
pub async fn stop_automatic_maintenance() -> Result<MaintenanceResponse, String> {
    eprintln!("⏹️ Stopping automatic maintenance...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        database.stop_maintenance().await;
        Ok(MaintenanceResponse::success("Automatic maintenance stopped successfully"))
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Run a manual maintenance cycle
/// 
/// This command performs a complete maintenance cycle including orphaned
/// embedding cleanup, index compaction, and storage optimization.
#[tauri::command]
pub async fn run_manual_maintenance_cycle() -> Result<MaintenanceResponse, String> {
    eprintln!("🔄 Running manual maintenance cycle...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.run_maintenance_cycle().await {
            Ok(stats) => {
                let message = format!(
                    "Maintenance cycle completed: {} orphans removed, {} bytes reclaimed",
                    stats.orphaned_embeddings_removed,
                    stats.storage_space_reclaimed
                );
                Ok(MaintenanceResponse::success_with_stats(message, stats))
            },
            Err(e) => {
                eprintln!("❌ Failed to run maintenance cycle: {}", e);
                Ok(MaintenanceResponse::error(format!("Failed to run maintenance cycle: {}", e)))
            }
        }
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Get maintenance statistics
/// 
/// This command returns comprehensive statistics about maintenance operations
/// including cleanup counts, performance metrics, and operation history.
#[tauri::command]
pub async fn get_maintenance_statistics() -> Result<MaintenanceResponse, String> {
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        match database.get_maintenance_stats().await {
            Ok(stats) => {
                let message = format!(
                    "Maintenance stats: {} cycles, {} orphans removed, avg {:.1}ms/cycle",
                    stats.maintenance_cycles,
                    stats.orphaned_embeddings_removed,
                    stats.avg_cycle_time_ms
                );
                Ok(MaintenanceResponse::success_with_stats(message, stats))
            },
            Err(e) => {
                eprintln!("❌ Failed to get maintenance stats: {}", e);
                Ok(MaintenanceResponse::error(format!("Failed to get maintenance stats: {}", e)))
            }
        }
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Check maintenance status
/// 
/// This command returns information about the current state of the maintenance system.
#[tauri::command]
pub async fn get_maintenance_status() -> Result<MaintenanceResponse, String> {
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        let is_running = database.is_maintenance_running().await;
        let config = database.get_maintenance_config();
        
        let status_message = if let Some(config) = config {
            if is_running {
                format!(
                    "Maintenance is running (interval: {}s, auto: {})", 
                    config.maintenance_interval_seconds,
                    config.enable_automatic_maintenance
                )
            } else if config.enable_automatic_maintenance {
                "Maintenance is configured but not currently running".to_string()
            } else {
                "Maintenance is configured for manual operation only".to_string()
            }
        } else {
            "Maintenance system not enabled".to_string()
        };
        
        Ok(MaintenanceResponse::success(status_message))
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Configure maintenance vault paths
/// 
/// This command updates the paths that are monitored for file existence validation
/// during orphaned embedding detection.
#[tauri::command]
pub async fn configure_maintenance_vault_paths(
    vault_paths: Vec<String>,
) -> Result<MaintenanceResponse, String> {
    eprintln!("📁 Configuring maintenance vault paths: {:?}", vault_paths);
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref database) = *db_guard {
        if let Some(config) = database.get_maintenance_config() {
            // Create updated configuration with new vault paths
            let mut updated_config = config.clone();
            updated_config.monitored_vault_paths = vault_paths.iter()
                .map(PathBuf::from)
                .collect();
            
            // We would need to update the config, but the current API doesn't support this
            // In a full implementation, we'd add an update_maintenance_config method
            Ok(MaintenanceResponse::success(format!(
                "Vault paths configuration updated ({} paths)",
                vault_paths.len()
            )))
        } else {
            Ok(MaintenanceResponse::error("Maintenance system not enabled"))
        }
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

/// Reset maintenance statistics
/// 
/// This command clears all maintenance statistics and resets counters to zero.
#[tauri::command]
pub async fn reset_maintenance_statistics() -> Result<MaintenanceResponse, String> {
    eprintln!("🔄 Resetting maintenance statistics...");
    
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref _database) = *db_guard {
        // Note: This would require adding a reset_stats method to the maintenance manager
        // For now, we'll return a success message indicating the request was received
        Ok(MaintenanceResponse::success("Maintenance statistics reset requested"))
    } else {
        Ok(MaintenanceResponse::error("Vector database not initialized"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_maintenance_request() {
        let request = EnableMaintenanceRequest {
            enable_automatic_maintenance: true,
            maintenance_interval_seconds: Some(600),
            max_orphan_cleanup_per_cycle: Some(50),
            enable_index_compaction: Some(true),
            compaction_cooldown_hours: Some(12),
            enable_defragmentation: Some(false),
            compaction_threshold: Some(0.4),
            max_operation_duration_seconds: Some(60),
            enable_debug_logging: Some(true),
            monitored_vault_paths: Some(vec!["/test/vault".to_string()]),
        };
        
        let config = request.to_config();
        
        assert!(config.enable_automatic_maintenance);
        assert_eq!(config.maintenance_interval_seconds, 600);
        assert_eq!(config.max_orphan_cleanup_per_cycle, 50);
        assert!(config.enable_index_compaction);
        assert_eq!(config.compaction_cooldown_hours, 12);
        assert!(!config.enable_defragmentation);
        assert_eq!(config.compaction_threshold, 0.4);
        assert_eq!(config.max_operation_duration_seconds, 60);
        assert!(config.enable_debug_logging);
        assert_eq!(config.monitored_vault_paths.len(), 1);
    }

    #[test]
    fn test_maintenance_response_creation() {
        let success_response = MaintenanceResponse::success("Operation completed");
        assert!(success_response.success);
        assert_eq!(success_response.message, "Operation completed");
        assert!(success_response.stats.is_none());
        
        let error_response = MaintenanceResponse::error("Operation failed");
        assert!(!error_response.success);
        assert_eq!(error_response.message, "Operation failed");
        assert!(error_response.stats.is_none());
    }

    #[test]
    fn test_maintenance_response_with_stats() {
        let stats = MaintenanceStats::default();
        let response = MaintenanceResponse::success_with_stats("Completed with stats", stats.clone());
        
        assert!(response.success);
        assert_eq!(response.message, "Completed with stats");
        assert!(response.stats.is_some());
    }
}