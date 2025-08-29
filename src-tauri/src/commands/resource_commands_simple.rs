//! Simplified Resource Allocation Tauri Commands
//!
//! This module provides simplified Tauri command handlers for the resource allocation system.

use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

use crate::resource_allocator::{
    ResourceAllocator, ResourceAllocatorConfig
};
use crate::performance::PerformanceTracker;

// Global resource allocator instance
static RESOURCE_ALLOCATOR: Lazy<RwLock<Option<Arc<ResourceAllocator>>>> = 
    Lazy::new(|| RwLock::new(None));

/// Initialize and start the resource allocation system
#[tauri::command]
pub async fn start_resource_allocation(
    config: Option<ResourceAllocatorConfig>
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    
    // Create performance tracker
    let performance_tracker = Arc::new(PerformanceTracker::start("resource_allocation"));
    
    // Create resource allocator
    let allocator = Arc::new(ResourceAllocator::new(config, performance_tracker)
        .map_err(|e| format!("ResourceAllocator creation failed: {}", e))?);
    
    // Start the allocator
    allocator.start().await
        .map_err(|e| format!("ResourceAllocator start failed: {}", e))?;
    
    // Store global reference
    {
        let mut global_allocator = RESOURCE_ALLOCATOR.write().await;
        *global_allocator = Some(allocator);
    }
    
    Ok("Resource allocation system started successfully".to_string())
}

/// Stop the resource allocation system
#[tauri::command]
pub async fn stop_resource_allocation() -> Result<String, String> {
    let allocator = {
        let mut global_allocator = RESOURCE_ALLOCATOR.write().await;
        global_allocator.take()
    };
    
    if let Some(allocator) = allocator {
        allocator.stop().await
            .map_err(|e| format!("ResourceAllocator stop failed: {}", e))?;
        Ok("Resource allocation system stopped successfully".to_string())
    } else {
        Ok("Resource allocation system was not running".to_string())
    }
}

/// Get current resource allocation status
#[tauri::command]
pub async fn get_resource_allocation_status() -> Result<bool, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    Ok(global_allocator.is_some())
}

/// Get basic resource information as JSON string
#[tauri::command]
pub async fn get_resource_metrics_json() -> Result<String, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    let metrics = allocator.get_metrics().await;
    
    serde_json::to_string(&metrics)
        .map_err(|e| format!("Failed to serialize metrics: {}", e))
}

/// Check if system is under resource pressure
#[tauri::command]
pub async fn is_system_under_pressure() -> Result<bool, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    Ok(allocator.is_under_pressure().await)
}

/// Schedule high priority UI operation
#[tauri::command]
pub async fn request_ui_operation_priority(operation_id: String) -> Result<String, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let _allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    // For now, just acknowledge the request
    Ok(format!("UI operation {} acknowledged with critical priority", operation_id))
}

/// Schedule AI operation with background priority
#[tauri::command]
pub async fn schedule_ai_operation(
    task_id: String, 
    operation_type: String,
    priority: Option<String>
) -> Result<String, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let _allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    let priority = priority.as_deref().unwrap_or("normal");
    
    Ok(format!("AI operation {} scheduled with {} priority for {}", task_id, priority, operation_type))
}

/// Schedule file I/O operation
#[tauri::command]
pub async fn schedule_file_io_operation(
    operation_id: String,
    priority: Option<String>
) -> Result<String, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let _allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    let priority = priority.as_deref().unwrap_or("normal");
    
    Ok(format!("File I/O operation {} scheduled with {} priority", operation_id, priority))
}

/// Enable graceful degradation mode
#[tauri::command]
pub async fn enable_graceful_degradation() -> Result<String, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    allocator.enable_degradation_mode().await
        .map_err(|e| format!("Failed to enable degradation mode: {}", e))?;
    
    Ok("Graceful degradation mode enabled".to_string())
}

/// Clean up completed background tasks
#[tauri::command]
pub async fn cleanup_completed_tasks() -> Result<usize, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    allocator.cleanup_completed_tasks().await
        .map_err(|e| format!("Cleanup failed: {}", e))
}

/// Update resource allocation configuration
#[tauri::command]
pub async fn update_resource_config(config: ResourceAllocatorConfig) -> Result<String, String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| "Resource allocator not initialized".to_string())?;
    
    allocator.update_config(config).await
        .map_err(|e| format!("Config update failed: {}", e))?;
    
    Ok("Resource allocation configuration updated successfully".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simple_command_lifecycle() {
        // Start resource allocation
        let result = start_resource_allocation(None).await;
        assert!(result.is_ok());
        
        // Check status
        let status = get_resource_allocation_status().await.unwrap();
        assert!(status);
        
        // Get metrics
        let metrics = get_resource_metrics_json().await;
        assert!(metrics.is_ok());
        
        // Stop resource allocation
        let result = stop_resource_allocation().await;
        assert!(result.is_ok());
    }
}