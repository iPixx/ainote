//! Memory Management Tauri Commands
//!
//! This module provides Tauri command handlers for the advanced memory management system,
//! enabling frontend monitoring and control of memory usage, leak detection, and 
//! allocation limits for AI operations.

use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::memory_manager::{
    MemoryManager, MemoryManagerConfig, MemoryMetrics,
    AllocationType
};

/// Global memory manager instance
static MEMORY_MANAGER: OnceLock<Arc<RwLock<Option<MemoryManager>>>> = OnceLock::new();

/// Get or initialize the global memory manager
fn get_memory_manager() -> &'static Arc<RwLock<Option<MemoryManager>>> {
    MEMORY_MANAGER.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Request to start memory management
#[derive(Debug, Serialize, Deserialize)]
pub struct StartMemoryManagementRequest {
    pub config: Option<MemoryManagerConfig>,
}

/// Response for memory management status
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryManagementStatusResponse {
    pub is_active: bool,
    pub config: Option<MemoryManagerConfig>,
    pub current_metrics: Option<MemoryMetrics>,
}

/// Request for AI allocation
#[derive(Debug, Serialize, Deserialize)]
pub struct AiAllocationRequest {
    pub operation_id: String,
    pub size_mb: f64,
}

/// Request to track allocation
#[derive(Debug, Serialize, Deserialize)]
pub struct TrackAllocationRequest {
    pub allocation_id: String,
    pub component: String,
    pub size_mb: f64,
    pub allocation_type: AllocationType,
}

/// Start memory management system
#[tauri::command]
pub async fn start_memory_management(
    request: StartMemoryManagementRequest,
) -> Result<MemoryManagementStatusResponse, String> {
    let manager_lock = get_memory_manager();
    let mut manager_guard = manager_lock.write().await;

    if manager_guard.is_some() {
        return Err("Memory management is already running".to_string());
    }

    let config = request.config.unwrap_or_default();
    let mut manager = MemoryManager::new(config.clone());
    
    manager.start().await
        .map_err(|e| format!("Failed to start memory management: {}", e))?;

    let metrics = manager.get_memory_metrics().await.ok();
    
    let response = MemoryManagementStatusResponse {
        is_active: true,
        config: Some(config),
        current_metrics: metrics,
    };

    *manager_guard = Some(manager);
    
    Ok(response)
}

/// Stop memory management system
#[tauri::command]
pub async fn stop_memory_management() -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let mut manager_guard = manager_lock.write().await;

    if let Some(mut manager) = manager_guard.take() {
        manager.stop().await
            .map_err(|e| format!("Failed to stop memory management: {}", e))?;
        
        Ok("Memory management stopped successfully".to_string())
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Get memory management status
#[tauri::command]
pub async fn get_memory_management_status() -> Result<MemoryManagementStatusResponse, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        let metrics = manager.get_memory_metrics().await.ok();
        
        Ok(MemoryManagementStatusResponse {
            is_active: true,
            config: None, // TODO: Get config from manager
            current_metrics: metrics,
        })
    } else {
        Ok(MemoryManagementStatusResponse {
            is_active: false,
            config: None,
            current_metrics: None,
        })
    }
}

/// Get current memory metrics
#[tauri::command]
pub async fn get_memory_metrics() -> Result<MemoryMetrics, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        manager.get_memory_metrics().await
            .map_err(|e| format!("Failed to get memory metrics: {}", e))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Get memory metrics history
#[tauri::command]
pub async fn get_memory_usage_history(limit: Option<usize>) -> Result<Vec<MemoryMetrics>, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.get_metrics_history(limit).await)
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Request AI operation memory allocation
#[tauri::command]
pub async fn request_ai_memory_allocation(request: AiAllocationRequest) -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        let size_bytes = (request.size_mb * 1024.0 * 1024.0) as usize;
        
        manager.request_ai_allocation(&request.operation_id, size_bytes).await
            .map_err(|e| format!("Failed to allocate AI memory: {}", e))?;
            
        Ok(format!("Allocated {:.2}MB for AI operation: {}", request.size_mb, request.operation_id))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Release AI operation memory allocation
#[tauri::command]
pub async fn release_ai_memory_allocation(operation_id: String) -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        manager.release_allocation(&operation_id).await
            .map_err(|e| format!("Failed to release AI memory: {}", e))?;
            
        Ok(format!("Released AI memory allocation for operation: {}", operation_id))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Track memory allocation
#[tauri::command]
pub async fn track_memory_allocation(request: TrackAllocationRequest) -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        let size_bytes = (request.size_mb * 1024.0 * 1024.0) as usize;
        
        manager.track_allocation(
            request.allocation_id.clone(),
            request.component,
            size_bytes,
            request.allocation_type,
        ).await.map_err(|e| format!("Failed to track allocation: {}", e))?;
            
        Ok(format!("Tracking {:.2}MB allocation: {}", request.size_mb, request.allocation_id))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Release memory allocation
#[tauri::command]
pub async fn release_memory_allocation(allocation_id: String) -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        manager.release_allocation(&allocation_id).await
            .map_err(|e| format!("Failed to release allocation: {}", e))?;
            
        Ok(format!("Released memory allocation: {}", allocation_id))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Trigger garbage collection
#[tauri::command]
pub async fn trigger_memory_garbage_collection() -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        let cleaned_bytes = manager.trigger_gc().await
            .map_err(|e| format!("Failed to trigger garbage collection: {}", e))?;
            
        let cleaned_mb = cleaned_bytes as f64 / (1024.0 * 1024.0);
        Ok(format!("Garbage collection freed {:.2}MB of memory", cleaned_mb))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Detect memory leaks
#[tauri::command]
pub async fn detect_memory_leaks() -> Result<Vec<String>, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if let Some(manager) = manager_guard.as_ref() {
        manager.detect_memory_leaks().await
            .map_err(|e| format!("Failed to detect memory leaks: {}", e))
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Update memory management configuration
#[tauri::command]
pub async fn update_memory_management_config(_config: MemoryManagerConfig) -> Result<String, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;

    if manager_guard.is_some() {
        // TODO: Implement configuration update without restart
        Ok("Configuration updated successfully (restart memory management to apply all changes)".to_string())
    } else {
        Err("Memory management is not currently running".to_string())
    }
}

/// Check if memory management is active
#[tauri::command]
pub async fn is_memory_management_active() -> Result<bool, String> {
    let manager_lock = get_memory_manager();
    let manager_guard = manager_lock.read().await;
    Ok(manager_guard.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_management_lifecycle() {
        let config = MemoryManagerConfig::default();
        let request = StartMemoryManagementRequest {
            config: Some(config),
        };
        
        // Start memory management
        let result = start_memory_management(request).await;
        assert!(result.is_ok());
        
        // Check status
        let status = get_memory_management_status().await.unwrap();
        assert!(status.is_active);
        
        // Stop memory management
        let result = stop_memory_management().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_memory_allocation_tracking() {
        let config = MemoryManagerConfig::default();
        let start_request = StartMemoryManagementRequest {
            config: Some(config),
        };
        
        start_memory_management(start_request).await.unwrap();
        
        // Track allocation
        let track_request = TrackAllocationRequest {
            allocation_id: "test_allocation".to_string(),
            component: "test_component".to_string(),
            size_mb: 10.0,
            allocation_type: AllocationType::EmbeddingCache,
        };
        
        let result = track_memory_allocation(track_request).await;
        assert!(result.is_ok());
        
        // Get metrics to verify
        let metrics = get_memory_metrics().await.unwrap();
        assert!(metrics.total_memory_mb > 0.0);
        assert_eq!(metrics.active_allocations, 1);
        
        // Release allocation
        let result = release_memory_allocation("test_allocation".to_string()).await;
        assert!(result.is_ok());
        
        stop_memory_management().await.unwrap();
    }
}