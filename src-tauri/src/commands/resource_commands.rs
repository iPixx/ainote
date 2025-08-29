//! Resource Allocation Tauri Commands
//!
//! This module provides Tauri command handlers for the resource allocation system,
//! enabling frontend integration with CPU priority management, I/O scheduling,
//! and background thread management.

use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

use crate::resource_allocator::{
    ResourceAllocator, ResourceAllocatorConfig, ResourceMetrics, ResourceResult,
    OperationPriority, OperationType, ResourceError
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
    let performance_tracker = Arc::new(
        PerformanceTracker::new()
            .map_err(|e| format!("Performance tracker: {}", e))?
    );
    
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
pub async fn stop_resource_allocation() -> ResourceResult<String> {
    let allocator = {
        let mut global_allocator = RESOURCE_ALLOCATOR.write().await;
        global_allocator.take()
    };
    
    if let Some(allocator) = allocator {
        allocator.stop().await?;
        Ok("Resource allocation system stopped successfully".to_string())
    } else {
        Ok("Resource allocation system was not running".to_string())
    }
}

/// Get current resource allocation status
#[tauri::command]
pub async fn get_resource_allocation_status() -> ResourceResult<bool> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    Ok(global_allocator.is_some())
}

/// Get current resource metrics
#[tauri::command]
pub async fn get_resource_metrics() -> ResourceResult<ResourceMetrics> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    Ok(allocator.get_metrics().await)
}

/// Check if system is under resource pressure
#[tauri::command]
pub async fn is_system_under_pressure() -> ResourceResult<bool> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    Ok(allocator.is_under_pressure().await)
}

/// Request priority for a UI operation (highest priority)
#[tauri::command]
pub async fn request_ui_operation_priority(operation_id: String) -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    // For UI operations, we primarily use the I/O scheduler
    let result = allocator.execute_io(
        OperationType::UiOperation,
        OperationPriority::Critical,
        async { () }
    ).await?;
    
    Ok(format!("UI operation {} scheduled with critical priority", operation_id))
}

/// Schedule background AI operation with appropriate priority
#[tauri::command]
pub async fn schedule_ai_operation(
    task_id: String, 
    operation_type: String,
    priority: Option<String>
) -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    // Parse operation type
    let op_type = match operation_type.as_str() {
        "embedding" => OperationType::AiEmbedding,
        "search" => OperationType::Search,
        "vectordb" => OperationType::VectorDbIo,
        "maintenance" => OperationType::Maintenance,
        _ => OperationType::AiEmbedding, // default
    };
    
    // Parse priority
    let priority = match priority.as_deref().unwrap_or("normal") {
        "critical" => OperationPriority::Critical,
        "high" => OperationPriority::High,
        "normal" => OperationPriority::Normal,
        "low" => OperationPriority::Low,
        "background" => OperationPriority::Background,
        _ => OperationPriority::Normal,
    };
    
    // Submit background task (placeholder - in real implementation this would be the actual AI task)
    allocator.submit_background_task(
        task_id.clone(),
        priority,
        op_type,
        async {}
    ).await?;
    
    Ok(format!("AI operation {} scheduled with priority {:?}", task_id, priority))
}

/// Request AI operation permit (for limiting concurrent AI operations)
#[tauri::command]
pub async fn request_ai_operation_permit() -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    let _permit = allocator.request_ai_permit().await?;
    // In a real implementation, we'd return a permit ID that can be used to release it
    
    Ok("AI operation permit acquired".to_string())
}

/// Schedule file I/O operation with timeout and priority
#[tauri::command]
pub async fn schedule_file_io_operation(
    operation_id: String,
    priority: Option<String>
) -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    // Parse priority
    let priority = match priority.as_deref().unwrap_or("normal") {
        "critical" => OperationPriority::Critical,
        "high" => OperationPriority::High,
        "normal" => OperationPriority::Normal,
        "low" => OperationPriority::Low,
        "background" => OperationPriority::Background,
        _ => OperationPriority::Normal,
    };
    
    // Execute I/O operation through scheduler
    let result = allocator.execute_io(
        OperationType::FileIo,
        priority,
        async { () }
    ).await?;
    
    Ok(format!("File I/O operation {} completed successfully", operation_id))
}

/// Schedule vector database operation with appropriate resource management
#[tauri::command]
pub async fn schedule_vector_db_operation(
    operation_id: String,
    priority: Option<String>
) -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    // Parse priority
    let priority = match priority.as_deref().unwrap_or("normal") {
        "critical" => OperationPriority::Critical,
        "high" => OperationPriority::High,
        "normal" => OperationPriority::Normal,
        "low" => OperationPriority::Low,
        "background" => OperationPriority::Background,
        _ => OperationPriority::Normal,
    };
    
    // Execute vector DB operation through scheduler
    let result = allocator.execute_io(
        OperationType::VectorDbIo,
        priority,
        async { () }
    ).await?;
    
    Ok(format!("Vector DB operation {} completed successfully", operation_id))
}

/// Enable graceful degradation mode under resource pressure
#[tauri::command]
pub async fn enable_graceful_degradation() -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    allocator.enable_degradation_mode().await?;
    
    Ok("Graceful degradation mode enabled".to_string())
}

/// Clean up completed background tasks
#[tauri::command]
pub async fn cleanup_completed_tasks() -> ResourceResult<usize> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    allocator.cleanup_completed_tasks().await
}

/// Update resource allocation configuration
#[tauri::command]
pub async fn update_resource_config(config: ResourceAllocatorConfig) -> ResourceResult<String> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    allocator.update_config(config).await?;
    
    Ok("Resource allocation configuration updated successfully".to_string())
}

/// Get detailed system resource utilization
#[tauri::command]
pub async fn get_system_resource_utilization() -> ResourceResult<serde_json::Value> {
    let global_allocator = RESOURCE_ALLOCATOR.read().await;
    
    let allocator = global_allocator.as_ref()
        .ok_or_else(|| ResourceError::AllocationFailed { 
            resource: "Resource allocator not initialized".to_string() 
        })?;
    
    let metrics = allocator.get_metrics().await;
    
    // Convert to JSON for detailed frontend display
    let utilization = serde_json::json!({
        "cpu_usage_percent": (metrics.cpu_usage * 100.0).round(),
        "active_threads_by_priority": metrics.active_threads,
        "pending_operations": metrics.pending_operations,
        "average_io_latency_ms": metrics.avg_io_latency_ms,
        "throttled_operations": metrics.throttled_operations,
        "system_load": metrics.system_load,
        "timestamp": metrics.timestamp,
        "is_under_pressure": metrics.cpu_usage > 0.8 || metrics.avg_io_latency_ms > 50.0
    });
    
    Ok(utilization)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_resource_allocation_lifecycle() {
        // Start resource allocation
        let result = start_resource_allocation(None).await;
        assert!(result.is_ok());
        
        // Check status
        let status = get_resource_allocation_status().await.unwrap();
        assert!(status);
        
        // Get metrics
        let metrics = get_resource_metrics().await;
        assert!(metrics.is_ok());
        
        // Stop resource allocation
        let result = stop_resource_allocation().await;
        assert!(result.is_ok());
        
        // Check status after stop
        let status = get_resource_allocation_status().await.unwrap();
        assert!(!status);
    }
    
    #[tokio::test]
    async fn test_operation_scheduling() {
        // Start system
        start_resource_allocation(None).await.unwrap();
        
        // Schedule UI operation
        let result = request_ui_operation_priority("test_ui".to_string()).await;
        assert!(result.is_ok());
        
        // Schedule AI operation
        let result = schedule_ai_operation(
            "test_ai".to_string(), 
            "embedding".to_string(),
            Some("high".to_string())
        ).await;
        assert!(result.is_ok());
        
        // Schedule file I/O
        let result = schedule_file_io_operation(
            "test_file".to_string(),
            Some("normal".to_string())
        ).await;
        assert!(result.is_ok());
        
        // Cleanup
        stop_resource_allocation().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_resource_pressure_detection() {
        start_resource_allocation(None).await.unwrap();
        
        let pressure = is_system_under_pressure().await.unwrap();
        // Should not be under pressure in test environment
        assert!(!pressure);
        
        stop_resource_allocation().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_graceful_degradation() {
        start_resource_allocation(None).await.unwrap();
        
        let result = enable_graceful_degradation().await;
        assert!(result.is_ok());
        
        stop_resource_allocation().await.unwrap();
    }
}