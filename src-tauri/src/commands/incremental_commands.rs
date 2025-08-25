//! Tauri Commands for Incremental Update System
//!
//! This module provides Tauri command handlers for the incremental update system,
//! allowing the frontend to interact with file change monitoring and automatic
//! embedding updates.
//!
//! ## Available Commands
//!
//! - `enable_incremental_updates`: Initialize the incremental update system
//! - `start_incremental_monitoring`: Begin monitoring a vault path
//! - `stop_incremental_monitoring`: Stop monitoring a vault path
//! - `process_incremental_updates`: Manually trigger processing of pending changes
//! - `get_incremental_update_stats`: Retrieve update history and statistics
//! - `get_incremental_config`: Get current incremental update configuration

use std::path::Path;
use serde::{Serialize, Deserialize};

use crate::vector_db::{
    incremental::{IncrementalConfig, UpdateStats},
    types::VectorDbResult,
};
use crate::globals::VECTOR_DATABASE;

/// Request structure for enabling incremental updates
#[derive(Debug, Serialize, Deserialize)]
pub struct EnableIncrementalRequest {
    /// Configuration for the incremental update system
    pub config: Option<IncrementalConfig>,
}

/// Request structure for starting/stopping monitoring
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitoringRequest {
    /// Path to the vault directory
    pub vault_path: String,
}

/// Response structure for incremental update statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct IncrementalStatsResponse {
    /// Recent update history
    pub update_history: Vec<UpdateStats>,
    /// Whether updates are currently being processed
    pub is_processing: bool,
    /// Current configuration (if enabled)
    pub config: Option<IncrementalConfig>,
    /// Number of updates in history
    pub total_updates: usize,
    /// Average processing time across all updates
    pub avg_processing_time_ms: f64,
}

/// Response structure for processing updates
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessUpdatesResponse {
    /// Statistics from the processed updates (if any)
    pub stats: Option<UpdateStats>,
    /// Whether any updates were processed
    pub processed_changes: bool,
    /// Message describing the operation result
    pub message: String,
}

/// Enable the incremental update system
/// 
/// This command initializes the incremental update manager with the specified
/// configuration. The system must be enabled before monitoring can begin.
/// 
/// # Arguments
/// 
/// * `request` - Configuration for the incremental update system
/// 
/// # Returns
/// 
/// Success message or error if initialization fails
#[tauri::command]
pub async fn enable_incremental_updates(
    request: EnableIncrementalRequest,
) -> Result<String, String> {
    let config = request.config.unwrap_or_default();
    
    // Get mutable reference to the vector database
    let result: VectorDbResult<()> = {
        let mut db_guard = VECTOR_DATABASE.write().await;
        if let Some(ref mut db) = *db_guard {
            db.enable_incremental_updates(config.clone()).await
        } else {
            return Err("Vector database not initialized".to_string());
        }
    };
    
    match result {
        Ok(()) => {
            eprintln!("✅ Incremental updates enabled with config: batch_size={}, timeout={}ms", 
                     config.max_batch_size, config.batch_timeout_ms);
            Ok("Incremental update system enabled successfully".to_string())
        },
        Err(e) => {
            eprintln!("❌ Failed to enable incremental updates: {}", e);
            Err(format!("Failed to enable incremental updates: {}", e))
        }
    }
}

/// Start monitoring a vault path for file changes
/// 
/// This command begins monitoring the specified directory for file changes.
/// The incremental update system must be enabled first.
/// 
/// # Arguments
/// 
/// * `request` - Vault path to monitor
/// 
/// # Returns
/// 
/// Success message or error if monitoring fails to start
#[tauri::command]
pub async fn start_incremental_monitoring(
    request: MonitoringRequest,
) -> Result<String, String> {
    let vault_path = Path::new(&request.vault_path);
    
    if !vault_path.exists() {
        return Err(format!("Vault path does not exist: {}", request.vault_path));
    }
    
    if !vault_path.is_dir() {
        return Err(format!("Vault path is not a directory: {}", request.vault_path));
    }
    
    // Get mutable reference to the vector database
    let result: VectorDbResult<()> = {
        let mut db_guard = VECTOR_DATABASE.write().await;
        if let Some(ref mut db) = *db_guard {
            db.start_incremental_monitoring(vault_path).await
        } else {
            return Err("Vector database not initialized".to_string());
        }
    };
    
    match result {
        Ok(()) => {
            eprintln!("👀 Started monitoring vault: {}", request.vault_path);
            Ok(format!("Started monitoring vault: {}", request.vault_path))
        },
        Err(e) => {
            eprintln!("❌ Failed to start monitoring: {}", e);
            Err(format!("Failed to start monitoring: {}", e))
        }
    }
}

/// Stop monitoring a vault path for file changes
/// 
/// This command stops monitoring the specified directory for file changes.
/// 
/// # Arguments
/// 
/// * `request` - Vault path to stop monitoring
/// 
/// # Returns
/// 
/// Success message or error if monitoring fails to stop
#[tauri::command]
pub async fn stop_incremental_monitoring(
    request: MonitoringRequest,
) -> Result<String, String> {
    let vault_path = Path::new(&request.vault_path);
    
    // Get mutable reference to the vector database
    let result: VectorDbResult<()> = {
        let mut db_guard = VECTOR_DATABASE.write().await;
        if let Some(ref mut db) = *db_guard {
            db.stop_incremental_monitoring(vault_path).await
        } else {
            return Err("Vector database not initialized".to_string());
        }
    };
    
    match result {
        Ok(()) => {
            eprintln!("⏹️ Stopped monitoring vault: {}", request.vault_path);
            Ok(format!("Stopped monitoring vault: {}", request.vault_path))
        },
        Err(e) => {
            eprintln!("❌ Failed to stop monitoring: {}", e);
            Err(format!("Failed to stop monitoring: {}", e))
        }
    }
}

/// Process pending incremental updates
/// 
/// This command manually triggers processing of any pending file changes
/// detected by the incremental update system.
/// 
/// # Returns
/// 
/// Processing results including statistics about changes processed
#[tauri::command]
pub async fn process_incremental_updates() -> Result<ProcessUpdatesResponse, String> {
    // Get reference to the vector database
    let result: VectorDbResult<Option<UpdateStats>> = {
        let db_guard = VECTOR_DATABASE.read().await;
        if let Some(ref db) = *db_guard {
            db.process_incremental_updates().await
        } else {
            return Err("Vector database not initialized".to_string());
        }
    };
    
    match result {
        Ok(stats_opt) => {
            let processed_changes = stats_opt.is_some();
            let message = if let Some(ref stats) = stats_opt {
                format!(
                    "Processed {} files: {} added, {} updated, {} deleted ({}ms)", 
                    stats.files_processed,
                    stats.embeddings_added,
                    stats.embeddings_updated,
                    stats.embeddings_deleted,
                    stats.processing_time_ms
                )
            } else {
                "No pending changes to process".to_string()
            };
            
            eprintln!("🔄 {}", message);
            
            Ok(ProcessUpdatesResponse {
                stats: stats_opt,
                processed_changes,
                message,
            })
        },
        Err(e) => {
            eprintln!("❌ Failed to process incremental updates: {}", e);
            Err(format!("Failed to process updates: {}", e))
        }
    }
}

/// Get incremental update statistics and history
/// 
/// This command retrieves comprehensive statistics about the incremental
/// update system including recent update history and current status.
/// 
/// # Returns
/// 
/// Detailed statistics and configuration information
#[tauri::command]
pub async fn get_incremental_update_stats() -> Result<IncrementalStatsResponse, String> {
    // Get reference to the vector database
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref db) = *db_guard {
        let update_history: Vec<UpdateStats> = db.get_incremental_update_history().await;
        let is_processing: bool = db.is_processing_incremental_updates().await;
        let config: Option<IncrementalConfig> = db.get_incremental_config().cloned();
        
        let total_updates = update_history.len();
        let avg_processing_time_ms = if total_updates > 0 {
            update_history.iter()
                .map(|stats| stats.processing_time_ms as f64)
                .sum::<f64>() / total_updates as f64
        } else {
            0.0
        };
        
        Ok(IncrementalStatsResponse {
            update_history,
            is_processing,
            config,
            total_updates,
            avg_processing_time_ms,
        })
    } else {
        Err("Vector database not initialized".to_string())
    }
}

/// Get current incremental update configuration
/// 
/// This command retrieves the current configuration of the incremental
/// update system, if enabled.
/// 
/// # Returns
/// 
/// Current configuration or None if not enabled
#[tauri::command]
pub async fn get_incremental_config() -> Result<Option<IncrementalConfig>, String> {
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref db) = *db_guard {
        let config: Option<IncrementalConfig> = db.get_incremental_config().cloned();
        Ok(config)
    } else {
        Err("Vector database not initialized".to_string())
    }
}

/// Check if incremental updates are currently being processed
/// 
/// This command returns whether the incremental update system is currently
/// processing file changes.
/// 
/// # Returns
/// 
/// True if processing, false otherwise
#[tauri::command]
pub async fn is_processing_incremental_updates() -> Result<bool, String> {
    let db_guard = VECTOR_DATABASE.read().await;
    if let Some(ref db) = *db_guard {
        let is_processing: bool = db.is_processing_incremental_updates().await;
        Ok(is_processing)
    } else {
        Ok(false) // If database not initialized, not processing
    }
}

/// Create default incremental update configuration
/// 
/// This command returns a default configuration that can be used to
/// initialize the incremental update system.
/// 
/// # Returns
/// 
/// Default incremental update configuration
#[tauri::command]
pub async fn get_default_incremental_config() -> Result<IncrementalConfig, String> {
    Ok(IncrementalConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enable_incremental_request_serialization() {
        let config = IncrementalConfig {
            batch_timeout_ms: 1000,
            max_batch_size: 20,
            enable_content_hashing: true,
            excluded_paths: vec![],
            monitored_extensions: vec!["md".to_string()],
            enable_debug_logging: false,
        };
        
        let request = EnableIncrementalRequest {
            config: Some(config),
        };
        
        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("batch_timeout_ms"));
        
        // Test deserialization
        let deserialized: EnableIncrementalRequest = serde_json::from_str(&json).unwrap();
        assert!(deserialized.config.is_some());
        assert_eq!(deserialized.config.unwrap().batch_timeout_ms, 1000);
    }
    
    #[test]
    fn test_monitoring_request_serialization() {
        let request = MonitoringRequest {
            vault_path: "/test/vault".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: MonitoringRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.vault_path, "/test/vault");
    }
    
    #[test]
    fn test_process_updates_response_serialization() {
        let response = ProcessUpdatesResponse {
            stats: None,
            processed_changes: false,
            message: "No changes".to_string(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ProcessUpdatesResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.processed_changes, false);
        assert_eq!(deserialized.message, "No changes");
    }
}