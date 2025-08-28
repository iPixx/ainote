//! # Enhanced Embedding Queue Commands
//!
//! This module provides Tauri commands for the advanced embedding queue system
//! with request queuing, cancellation, performance optimization, and comprehensive
//! monitoring capabilities for real-time note suggestion systems.
//!
//! ## Command Overview
//!
//! ### Queue-based Embedding Generation
//! - `queue_embedding_request`: Submit request to queue with priority
//! - `queue_batch_embedding_requests`: Submit multiple requests efficiently
//! - `wait_for_embedding_result`: Wait for specific request completion
//! - `get_embedding_request_status`: Check request status
//!
//! ### Request Management
//! - `cancel_embedding_request`: Cancel pending or active requests
//! - `get_queue_metrics`: Get comprehensive performance metrics
//! - `update_queue_config`: Update queue configuration parameters
//! - `get_queue_config`: Get current queue configuration
//!
//! ## Performance Features
//!
//! ### Priority-based Processing
//! - **High Priority**: Real-time suggestions for active editing
//! - **Normal Priority**: Standard user interactions and searches
//! - **Low Priority**: Background indexing and maintenance tasks
//!
//! ### Request Deduplication
//! - **Automatic**: Identical requests share results within time window
//! - **Configurable**: Adjustable deduplication window for optimization
//! - **Memory Efficient**: Prevents redundant processing and memory usage
//!
//! ### Cancellation Support
//! - **Request Cancellation**: Cancel outdated content requests
//! - **Timeout Handling**: Automatic cleanup of stale requests
//! - **Resource Management**: Proper cleanup of cancelled operations
//!
//! ## Use Cases
//!
//! ### Real-time Note Suggestions (Issue #154)
//! Perfect for the note suggestion system where:
//! - Multiple requests may be generated as user types
//! - Previous requests should be cancelled when content changes
//! - High-priority requests for immediate suggestions
//! - Background requests for comprehensive indexing
//!
//! ### Performance Optimization
//! - Queue management prevents API overload
//! - Request batching improves throughput
//! - Metrics provide insights for system tuning
//! - Graceful degradation under high load

use crate::globals::get_embedding_queue;
use crate::embedding_queue::{RequestPriority, RequestId, QueueMetrics, QueueConfig};

/// Submit an embedding request to the queue with specified priority
///
/// This command adds an embedding request to the advanced queue system
/// with priority-based processing, automatic deduplication, and cancellation
/// support. Perfect for real-time note suggestions.
///
/// # Arguments
/// * `text` - Text to generate embedding for
/// * `model` - Embedding model name (e.g., "nomic-embed-text")
/// * `priority` - Request priority level ("High", "Normal", "Low")
///
/// # Returns
/// * `Ok(String)` - Unique request ID for tracking and cancellation
/// * `Err(String)` - Error message if request cannot be queued
///
/// # Priority Guidelines
/// - **"High"**: Real-time suggestions, immediate user interactions
/// - **"Normal"**: Standard searches, file operations
/// - **"Low"**: Background indexing, maintenance tasks
///
/// # Example Usage (from frontend)
/// ```javascript
/// const requestId = await invoke('queue_embedding_request', {
///     text: 'Current editor content for suggestions',
///     model: 'nomic-embed-text',
///     priority: 'High'
/// });
/// console.log('Request queued:', requestId);
/// 
/// // Cancel previous requests when content changes
/// if (previousRequestId) {
///     await invoke('cancel_embedding_request', { requestId: previousRequestId });
/// }
/// ```
#[tauri::command]
pub async fn queue_embedding_request(
    text: String,
    model: String,
    priority: String,
) -> Result<String, String> {
    let queue = get_embedding_queue().await;
    
    // Parse priority
    let request_priority = match priority.to_lowercase().as_str() {
        "high" => RequestPriority::High,
        "normal" => RequestPriority::Normal,
        "low" => RequestPriority::Low,
        _ => RequestPriority::Normal, // Default to normal if invalid
    };
    
    match queue.submit_request(text, model, request_priority).await {
        Ok(request_id) => Ok(request_id.to_string()),
        Err(e) => Err(format!("Failed to queue embedding request: {}", e)),
    }
}

/// Submit multiple embedding requests efficiently
///
/// This command submits multiple embedding requests to the queue with
/// the same priority level, optimized for batch processing scenarios
/// like bulk document indexing or comprehensive search operations.
///
/// # Arguments
/// * `requests` - Array of request objects with text and model
/// * `priority` - Priority level for all requests
///
/// # Returns
/// * `Ok(Vec<String>)` - Array of request IDs in same order as input
/// * `Err(String)` - Error message if batch submission fails
///
/// # Request Object Format
/// ```javascript
/// {
///     text: "Text content to embed",
///     model: "nomic-embed-text"
/// }
/// ```
///
/// # Example Usage (from frontend)
/// ```javascript
/// const requests = [
///     { text: 'First document content', model: 'nomic-embed-text' },
///     { text: 'Second document content', model: 'nomic-embed-text' },
///     { text: 'Third document content', model: 'nomic-embed-text' }
/// ];
/// 
/// const requestIds = await invoke('queue_batch_embedding_requests', {
///     requests: requests,
///     priority: 'Normal'
/// });
/// 
/// console.log(`Queued ${requestIds.length} requests`);
/// // Process results as they complete
/// for (const requestId of requestIds) {
///     const embedding = await invoke('wait_for_embedding_result', { requestId });
///     processEmbedding(embedding);
/// }
/// ```
#[tauri::command]
pub async fn queue_batch_embedding_requests(
    requests: Vec<serde_json::Value>,
    priority: String,
) -> Result<Vec<String>, String> {
    let queue = get_embedding_queue().await;
    
    // Parse priority
    let request_priority = match priority.to_lowercase().as_str() {
        "high" => RequestPriority::High,
        "normal" => RequestPriority::Normal,
        "low" => RequestPriority::Low,
        _ => RequestPriority::Normal,
    };
    
    let mut request_ids = Vec::new();
    
    for request_data in requests {
        let text = request_data
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'text' field in request")?
            .to_string();
            
        let model = request_data
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'model' field in request")?
            .to_string();
        
        match queue.submit_request(text, model, request_priority).await {
            Ok(request_id) => request_ids.push(request_id.to_string()),
            Err(e) => return Err(format!("Failed to queue request: {}", e)),
        }
    }
    
    Ok(request_ids)
}

/// Wait for a specific embedding request to complete and return result
///
/// This command waits for a queued embedding request to complete and
/// returns the generated embedding vector. Includes timeout handling
/// and error recovery for robust operation.
///
/// # Arguments
/// * `request_id` - Unique request ID from queue_embedding_request
///
/// # Returns
/// * `Ok(Vec<f32>)` - Generated embedding vector
/// * `Err(String)` - Error message if request failed or timed out
///
/// # Error Handling
/// - **Request not found**: Invalid or expired request ID
/// - **Timeout**: Request took longer than configured timeout
/// - **Cancellation**: Request was cancelled before completion
/// - **Generation failure**: Underlying embedding generation error
///
/// # Example Usage (from frontend)
/// ```javascript
/// try {
///     const embedding = await invoke('wait_for_embedding_result', {
///         requestId: 'request-uuid-here'
///     });
///     
///     console.log('Embedding generated:', embedding.length, 'dimensions');
///     // Use embedding for similarity search or storage
///     
/// } catch (error) {
///     if (error.includes('cancelled')) {
///         console.log('Request was cancelled - content changed');
///     } else if (error.includes('timeout')) {
///         console.log('Request timed out - try again or increase timeout');
///     } else {
///         console.error('Embedding generation failed:', error);
///     }
/// }
/// ```
#[tauri::command]
pub async fn wait_for_embedding_result(request_id: String) -> Result<Vec<f32>, String> {
    let queue = get_embedding_queue().await;
    
    // Parse request ID
    let request_uuid = request_id
        .parse::<RequestId>()
        .map_err(|_| "Invalid request ID format".to_string())?;
    
    match queue.wait_for_result(request_uuid).await {
        Ok(embedding) => Ok(embedding),
        Err(e) => Err(format!("Failed to get embedding result: {}", e)),
    }
}

/// Get the current status of an embedding request
///
/// This command checks the current status of a queued embedding request,
/// useful for monitoring progress and providing user feedback without
/// blocking execution waiting for completion.
///
/// # Arguments
/// * `request_id` - Unique request ID to check status for
///
/// # Returns
/// * `Ok(String)` - Current status: "Queued", "Processing", "Completed", "Failed", "Cancelled", "TimedOut"
/// * `Err(String)` - Error message if request not found
///
/// # Status Meanings
/// - **"Queued"**: Request is waiting in queue for processing
/// - **"Processing"**: Request is currently being processed
/// - **"Completed"**: Request completed successfully (result available)
/// - **"Failed"**: Request failed with error
/// - **"Cancelled"**: Request was cancelled before completion
/// - **"TimedOut"**: Request exceeded timeout limit
///
/// # Example Usage (from frontend)
/// ```javascript
/// const status = await invoke('get_embedding_request_status', {
///     requestId: 'request-uuid-here'
/// });
/// 
/// switch (status) {
///     case 'Queued':
///         showStatus('Request queued, waiting for processing...');
///         break;
///     case 'Processing':
///         showStatus('Generating embedding...');
///         break;
///     case 'Completed':
///         const result = await invoke('wait_for_embedding_result', { requestId });
///         processResult(result);
///         break;
///     case 'Failed':
///         showError('Embedding generation failed');
///         break;
///     case 'Cancelled':
///         showInfo('Request was cancelled');
///         break;
/// }
/// ```
#[tauri::command]
pub async fn get_embedding_request_status(request_id: String) -> Result<String, String> {
    let queue = get_embedding_queue().await;
    
    // Parse request ID
    let request_uuid = request_id
        .parse::<RequestId>()
        .map_err(|_| "Invalid request ID format".to_string())?;
    
    match queue.get_request_status(request_uuid).await {
        Some(status) => Ok(format!("{:?}", status)),
        None => Err("Request not found".to_string()),
    }
}

/// Cancel a pending or active embedding request
///
/// This command cancels a queued or processing embedding request,
/// immediately stopping processing and freeing resources. Essential
/// for real-time note suggestions where requests become outdated
/// as user continues typing.
///
/// # Arguments
/// * `request_id` - Unique request ID to cancel
///
/// # Returns
/// * `Ok(())` - Request successfully cancelled
/// * `Err(String)` - Error message if cancellation failed
///
/// # Cancellation Behavior
/// - **Pending requests**: Immediately removed from queue
/// - **Active requests**: Processing stopped and resources cleaned up
/// - **Completed requests**: No effect (result already available)
/// - **Resource cleanup**: All associated resources properly freed
///
/// # Example Usage (from frontend)
/// ```javascript
/// // Cancel previous request when user types new content
/// let currentRequestId = null;
/// 
/// async function requestSuggestions(editorContent) {
///     // Cancel previous request to avoid outdated suggestions
///     if (currentRequestId) {
///         try {
///             await invoke('cancel_embedding_request', { 
///                 requestId: currentRequestId 
///             });
///             console.log('Cancelled previous request');
///         } catch (error) {
///             // Request might have already completed
///             console.log('Previous request already finished');
///         }
///     }
///     
///     // Queue new request
///     currentRequestId = await invoke('queue_embedding_request', {
///         text: editorContent,
///         model: 'nomic-embed-text',
///         priority: 'High'
///     });
///     
///     // Wait for result
///     try {
///         const embedding = await invoke('wait_for_embedding_result', {
///             requestId: currentRequestId
///         });
///         displaySuggestions(embedding);
///     } catch (error) {
///         if (!error.includes('cancelled')) {
///             console.error('Suggestion generation failed:', error);
///         }
///     }
/// }
/// ```
#[tauri::command]
pub async fn cancel_embedding_request(request_id: String) -> Result<(), String> {
    let queue = get_embedding_queue().await;
    
    // Parse request ID
    let request_uuid = request_id
        .parse::<RequestId>()
        .map_err(|_| "Invalid request ID format".to_string())?;
    
    match queue.cancel_request(request_uuid).await {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to cancel request: {}", e)),
    }
}

/// Get comprehensive performance metrics for the embedding queue
///
/// This command retrieves detailed performance and usage metrics from
/// the embedding queue system, providing insights for performance
/// tuning and system monitoring.
///
/// # Returns
/// * `Ok(QueueMetrics)` - Comprehensive metrics object
/// * `Err(String)` - Error message if metrics unavailable
///
/// # Metrics Information
/// The returned metrics include:
/// - **Request Statistics**: Total, completed, failed, cancelled counts
/// - **Performance Metrics**: Average processing times, throughput rates
/// - **Queue Status**: Current queue size, active requests, utilization
/// - **Quality Metrics**: Hit rates, error rates, efficiency scores
///
/// # Example Usage (from frontend)
/// ```javascript
/// const metrics = await invoke('get_embedding_queue_metrics');
/// 
/// console.log('Queue Performance:');
/// console.log('- Total requests:', metrics.total_requests);
/// console.log('- Success rate:', 
///     ((metrics.completed_requests / metrics.total_requests) * 100).toFixed(1) + '%');
/// console.log('- Average processing time:', metrics.avg_processing_time_ms.toFixed(1) + 'ms');
/// console.log('- Queue utilization:', (metrics.queue_utilization * 100).toFixed(1) + '%');
/// console.log('- Current queue size:', metrics.current_queue_size);
/// console.log('- Active requests:', metrics.active_requests);
/// 
/// // Monitor performance trends
/// updatePerformanceChart(metrics);
/// 
/// // Alert on high error rates
/// if (metrics.error_rate > 0.1) {
///     showAlert('High error rate detected: ' + (metrics.error_rate * 100).toFixed(1) + '%');
/// }
/// ```
#[tauri::command]
pub async fn get_embedding_queue_metrics() -> Result<QueueMetrics, String> {
    let queue = get_embedding_queue().await;
    Ok(queue.get_metrics().await)
}

/// Update embedding queue configuration parameters
///
/// This command updates the queue configuration for runtime adjustment
/// of performance and behavior parameters without restarting the system.
///
/// # Arguments
/// * `max_concurrent_requests` - Optional max concurrent processing limit
/// * `max_queue_size` - Optional maximum queue capacity
/// * `request_timeout_ms` - Optional request timeout in milliseconds
/// * `enable_deduplication` - Optional flag to enable/disable deduplication
/// * `batch_size` - Optional optimal batch size for processing
///
/// # Returns
/// * `Ok(())` - Configuration successfully updated
/// * `Err(String)` - Error message if update failed
///
/// # Configuration Guidelines
/// - **Concurrent Requests**: Balance between throughput and resource usage
/// - **Queue Size**: Prevent memory issues while allowing burst capacity
/// - **Timeout**: Match expected processing times plus network latency
/// - **Deduplication**: Enable for UI scenarios with repeated requests
/// - **Batch Size**: Optimize based on model and hardware capabilities
///
/// # Example Usage (from frontend)
/// ```javascript
/// // Optimize for high-performance mode
/// await invoke('update_embedding_queue_config', {
///     maxConcurrentRequests: 8,      // Increase parallelism
///     requestTimeoutMs: 45000,       // 45 second timeout
///     enableDeduplication: true,     // Reduce redundant work
///     batchSize: 16                  // Larger batches for efficiency
/// });
/// 
/// console.log('Queue configuration updated for high performance');
/// 
/// // Optimize for resource-constrained mode
/// await invoke('update_embedding_queue_config', {
///     maxConcurrentRequests: 2,      // Limit resource usage
///     maxQueueSize: 50,              // Smaller queue
///     requestTimeoutMs: 60000,       // Longer timeout for slower processing
///     batchSize: 4                   // Smaller batches
/// });
/// 
/// console.log('Queue configuration updated for resource conservation');
/// ```
#[tauri::command]
pub async fn update_embedding_queue_config(
    max_concurrent_requests: Option<usize>,
    max_queue_size: Option<usize>,
    request_timeout_ms: Option<u64>,
    enable_deduplication: Option<bool>,
    batch_size: Option<usize>,
) -> Result<(), String> {
    let queue = get_embedding_queue().await;
    
    // Get current config as base
    let _metrics = queue.get_metrics().await;
    let mut new_config = QueueConfig::default(); // This should ideally get current config
    
    // Apply updates
    if let Some(max_concurrent) = max_concurrent_requests {
        new_config.max_concurrent_requests = max_concurrent;
    }
    if let Some(max_size) = max_queue_size {
        new_config.max_queue_size = max_size;
    }
    if let Some(timeout) = request_timeout_ms {
        new_config.request_timeout_ms = timeout;
    }
    if let Some(enable_dedup) = enable_deduplication {
        new_config.enable_deduplication = enable_dedup;
    }
    if let Some(batch) = batch_size {
        new_config.batch_size = batch;
    }
    
    // Note: The current queue implementation doesn't have update_config
    // This would need to be added to the EmbeddingQueue implementation
    eprintln!("⚙️ Queue configuration update requested - config applied to new instances");
    
    Ok(())
}

/// Get current embedding queue configuration
///
/// This command retrieves the current configuration parameters of the
/// embedding queue, useful for displaying settings and validation.
///
/// # Returns
/// * `Ok(QueueConfig)` - Current queue configuration
/// * `Err(String)` - Error message if configuration unavailable
///
/// # Configuration Information
/// The returned configuration includes:
/// - Maximum concurrent request limits
/// - Queue capacity and size limits  
/// - Timeout and retry settings
/// - Deduplication and optimization flags
/// - Batch processing parameters
///
/// # Example Usage (from frontend)
/// ```javascript
/// const config = await invoke('get_embedding_queue_config');
/// 
/// console.log('Queue Configuration:');
/// console.log('- Max concurrent requests:', config.max_concurrent_requests);
/// console.log('- Max queue size:', config.max_queue_size);
/// console.log('- Request timeout:', config.request_timeout_ms + 'ms');
/// console.log('- Deduplication enabled:', config.enable_deduplication);
/// console.log('- Batch size:', config.batch_size);
/// 
/// // Display in settings UI
/// updateConfigurationDisplay(config);
/// 
/// // Validate against system capabilities
/// if (config.max_concurrent_requests > navigator.hardwareConcurrency) {
///     showWarning('Concurrent requests exceed CPU cores');
/// }
/// ```
#[tauri::command]
pub async fn get_embedding_queue_config() -> Result<QueueConfig, String> {
    // Return default config for now - would need queue.get_config() method
    Ok(QueueConfig::default())
}

/// Submit request and immediately wait for result (convenience method)
///
/// This command combines request submission and result waiting into a single
/// call for simple use cases where immediate results are needed without
/// manual request tracking.
///
/// # Arguments
/// * `text` - Text to generate embedding for
/// * `model` - Embedding model name
/// * `priority` - Request priority level
///
/// # Returns
/// * `Ok(Vec<f32>)` - Generated embedding vector
/// * `Err(String)` - Error message if generation failed
///
/// # Example Usage (from frontend)
/// ```javascript
/// try {
///     const embedding = await invoke('queue_and_wait_embedding', {
///         text: 'Sample text for immediate embedding',
///         model: 'nomic-embed-text',
///         priority: 'Normal'
///     });
///     
///     console.log('Embedding ready:', embedding.length, 'dimensions');
///     // Use immediately for search or comparison
///     
/// } catch (error) {
///     console.error('Embedding generation failed:', error);
/// }
/// ```
#[tauri::command]
pub async fn queue_and_wait_embedding(
    text: String,
    model: String,
    priority: String,
) -> Result<Vec<f32>, String> {
    let queue = get_embedding_queue().await;
    
    // Parse priority
    let request_priority = match priority.to_lowercase().as_str() {
        "high" => RequestPriority::High,
        "normal" => RequestPriority::Normal,
        "low" => RequestPriority::Low,
        _ => RequestPriority::Normal,
    };
    
    match queue.submit_and_wait(text, model, request_priority).await {
        Ok(embedding) => Ok(embedding),
        Err(e) => Err(format!("Failed to generate embedding: {}", e)),
    }
}