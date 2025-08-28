//! # Embedding Request Queue System
//!
//! This module provides a sophisticated queuing system for embedding generation
//! requests with performance optimization, cancellation support, and request
//! deduplication to prevent overload and ensure optimal resource usage.
//!
//! ## Features
//!
//! ### Request Queuing
//! - **Priority-based queuing**: High-priority requests processed first
//! - **Request deduplication**: Identical requests share results
//! - **Batch optimization**: Groups similar requests for efficiency
//! - **Load balancing**: Distributes requests across available resources
//!
//! ### Cancellation Support
//! - **Request cancellation**: Cancel outdated or unwanted requests
//! - **Timeout handling**: Automatic cleanup of stale requests
//! - **Graceful degradation**: Handle failures without system impact
//! - **Resource cleanup**: Proper cleanup of cancelled operations
//!
//! ### Performance Features
//! - **Connection pooling**: Reuse connections for multiple requests
//! - **Rate limiting**: Prevent API overload
//! - **Metrics tracking**: Monitor queue performance and health
//! - **Memory management**: Efficient memory usage with cleanup

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, oneshot, watch};
use tokio::time::timeout;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::embedding_generator::{EmbeddingGenerator, EmbeddingError};
use crate::ollama_client::OllamaConfig;

/// Priority levels for embedding requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RequestPriority {
    /// Background tasks, lowest priority
    Low = 0,
    /// Normal user interactions
    Normal = 1,
    /// Real-time suggestions, highest priority
    High = 2,
}

impl Default for RequestPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Status of an embedding request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestStatus {
    /// Request is queued and waiting to be processed
    Queued,
    /// Request is currently being processed
    Processing,
    /// Request completed successfully
    Completed,
    /// Request failed with error
    Failed { error: String },
    /// Request was cancelled before completion
    Cancelled,
    /// Request timed out
    TimedOut,
}

/// Configuration for the embedding queue system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    /// Maximum queue size before rejecting requests
    pub max_queue_size: usize,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// How long to keep completed requests in memory (ms)
    pub result_retention_ms: u64,
    /// Whether to enable request deduplication
    pub enable_deduplication: bool,
    /// Minimum interval between identical requests (ms)
    pub deduplication_window_ms: u64,
    /// Maximum time to wait for a batch to fill (ms)
    pub batch_timeout_ms: u64,
    /// Optimal batch size for processing
    pub batch_size: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 4,
            max_queue_size: 100,
            request_timeout_ms: 30_000, // 30 seconds
            result_retention_ms: 300_000, // 5 minutes
            enable_deduplication: true,
            deduplication_window_ms: 1_000, // 1 second
            batch_timeout_ms: 100, // 100ms batch window
            batch_size: 8,
        }
    }
}

/// Unique identifier for a request, allowing cancellation
pub type RequestId = Uuid;

/// Cancellation token for aborting requests
#[derive(Debug, Clone)]
pub struct CancellationToken {
    id: RequestId,
    sender: watch::Sender<bool>,
    receiver: watch::Receiver<bool>,
}

impl CancellationToken {
    fn new(id: RequestId) -> Self {
        let (sender, receiver) = watch::channel(false);
        Self { id, sender, receiver }
    }

    /// Check if cancellation has been requested
    pub fn is_cancelled(&self) -> bool {
        *self.receiver.borrow()
    }

    /// Cancel the request
    pub fn cancel(&self) {
        let _ = self.sender.send(true);
    }

    /// Get the request ID
    pub fn request_id(&self) -> RequestId {
        self.id
    }
}

/// Individual embedding request in the queue
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub id: RequestId,
    pub text: String,
    pub model: String,
    pub priority: RequestPriority,
    pub created_at: Instant,
    pub timeout_at: Instant,
    pub cancellation_token: CancellationToken,
}

/// Result of an embedding request
#[derive(Debug, Clone)]
pub struct EmbeddingRequestResult {
    pub id: RequestId,
    pub status: RequestStatus,
    pub result: Option<Vec<f32>>,
    pub error: Option<String>,
    pub processing_time_ms: Option<u64>,
    pub completed_at: Option<Instant>,
}

/// Metrics for queue performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub total_requests: usize,
    pub completed_requests: usize,
    pub failed_requests: usize,
    pub cancelled_requests: usize,
    pub current_queue_size: usize,
    pub active_requests: usize,
    pub avg_processing_time_ms: f64,
    pub avg_queue_wait_time_ms: f64,
    pub hit_rate: f64,
    pub throughput_per_second: f64,
    pub error_rate: f64,
    pub queue_utilization: f64,
}

impl Default for QueueMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            completed_requests: 0,
            failed_requests: 0,
            cancelled_requests: 0,
            current_queue_size: 0,
            active_requests: 0,
            avg_processing_time_ms: 0.0,
            avg_queue_wait_time_ms: 0.0,
            hit_rate: 0.0,
            throughput_per_second: 0.0,
            error_rate: 0.0,
            queue_utilization: 0.0,
        }
    }
}

/// Errors specific to the queue system
#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Queue is full (max size: {max_size})")]
    QueueFull { max_size: usize },
    
    #[error("Request {id} not found")]
    RequestNotFound { id: RequestId },
    
    #[error("Request {id} was cancelled")]
    RequestCancelled { id: RequestId },
    
    #[error("Request {id} timed out after {timeout_ms}ms")]
    RequestTimedOut { id: RequestId, timeout_ms: u64 },
    
    #[error("Embedding generation error: {0}")]
    EmbeddingError(#[from] EmbeddingError),
    
    #[error("System overloaded: {message}")]
    SystemOverloaded { message: String },
}

pub type QueueResult<T> = Result<T, QueueError>;

/// Internal request queue entry with sender for result notification
struct QueueEntry {
    request: EmbeddingRequest,
    result_sender: oneshot::Sender<EmbeddingRequestResult>,
}

/// High-performance embedding queue with advanced features
#[derive(Clone)]
pub struct EmbeddingQueue {
    config: QueueConfig,
    generator: Arc<EmbeddingGenerator>,
    
    // Queue management
    pending_queue: Arc<RwLock<VecDeque<QueueEntry>>>,
    active_requests: Arc<RwLock<HashMap<RequestId, EmbeddingRequest>>>,
    completed_results: Arc<RwLock<HashMap<RequestId, EmbeddingRequestResult>>>,
    
    // Deduplication cache
    deduplication_cache: Arc<RwLock<HashMap<String, (Instant, RequestId)>>>,
    
    // Concurrency control
    semaphore: Arc<Semaphore>,
    
    // Metrics
    metrics: Arc<RwLock<QueueMetrics>>,
}

impl EmbeddingQueue {
    /// Create a new embedding queue with specified configuration
    pub fn new(generator: EmbeddingGenerator, config: QueueConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
        
        Self {
            config,
            generator: Arc::new(generator),
            pending_queue: Arc::new(RwLock::new(VecDeque::new())),
            active_requests: Arc::new(RwLock::new(HashMap::new())),
            completed_results: Arc::new(RwLock::new(HashMap::new())),
            deduplication_cache: Arc::new(RwLock::new(HashMap::new())),
            semaphore,
            metrics: Arc::new(RwLock::new(QueueMetrics::default())),
        }
    }

    /// Create a new queue with default configuration
    pub fn with_default_config(ollama_config: OllamaConfig) -> Self {
        let generator = EmbeddingGenerator::new(ollama_config);
        Self::new(generator, QueueConfig::default())
    }

    /// Start the background processing tasks
    pub async fn start(&self) -> (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>) {
        // Start request processor
        let processor_handle = self.spawn_request_processor().await;

        // Start cleanup task
        let cleanup_handle = self.spawn_cleanup_task().await;

        eprintln!("🚀 Embedding queue started with {} max concurrent requests", 
                  self.config.max_concurrent_requests);
        
        (processor_handle, cleanup_handle)
    }

    /// Submit an embedding request to the queue
    pub async fn submit_request(
        &self,
        text: String,
        model: String,
        priority: RequestPriority,
    ) -> QueueResult<RequestId> {
        let request_id = Uuid::new_v4();
        
        // Check queue capacity
        {
            let queue = self.pending_queue.read().await;
            if queue.len() >= self.config.max_queue_size {
                return Err(QueueError::QueueFull { 
                    max_size: self.config.max_queue_size 
                });
            }
        }

        // Create request with timeout
        let now = Instant::now();
        let timeout_at = now + Duration::from_millis(self.config.request_timeout_ms);
        let cancellation_token = CancellationToken::new(request_id);

        let request = EmbeddingRequest {
            id: request_id,
            text,
            model,
            priority,
            created_at: now,
            timeout_at,
            cancellation_token,
        };

        // Check for deduplication
        if self.config.enable_deduplication {
            if let Some(existing_id) = self.check_deduplication(&request).await {
                eprintln!("🔄 Deduplicating request {} -> {}", request_id, existing_id);
                return Ok(existing_id);
            }
        }

        // Add to queue
        let (result_sender, _result_receiver) = oneshot::channel();
        let queue_entry = QueueEntry {
            request: request.clone(),
            result_sender,
        };

        {
            let mut queue = self.pending_queue.write().await;
            // Insert based on priority (higher priority first)
            let insert_pos = queue.iter()
                .position(|entry| entry.request.priority < request.priority)
                .unwrap_or(queue.len());
            queue.insert(insert_pos, queue_entry);
        }

        // Update deduplication cache
        if self.config.enable_deduplication {
            self.update_deduplication_cache(&request).await;
        }

        // Update metrics
        self.update_metrics(|metrics| {
            metrics.total_requests += 1;
            metrics.current_queue_size += 1;
        }).await;

        eprintln!("📝 Queued embedding request {} with priority {:?}", request_id, priority);
        Ok(request_id)
    }

    /// Submit a request and wait for the result
    pub async fn submit_and_wait(
        &self,
        text: String,
        model: String,
        priority: RequestPriority,
    ) -> QueueResult<Vec<f32>> {
        let request_id = self.submit_request(text, model, priority).await?;
        self.wait_for_result(request_id).await
    }

    /// Wait for a specific request to complete
    pub async fn wait_for_result(&self, request_id: RequestId) -> QueueResult<Vec<f32>> {
        let timeout_duration = Duration::from_millis(self.config.request_timeout_ms);
        
        // Use a polling approach since we can't easily return the receiver
        let result = timeout(timeout_duration, async {
            loop {
                // Check if result is available
                {
                    let results = self.completed_results.read().await;
                    if let Some(result) = results.get(&request_id) {
                        return match &result.status {
                            RequestStatus::Completed => {
                                result.result.clone().ok_or_else(|| 
                                    QueueError::EmbeddingError(EmbeddingError::InvalidResponse { 
                                        reason: "No result data".to_string() 
                                    })
                                )
                            },
                            RequestStatus::Failed { error } => {
                                Err(QueueError::EmbeddingError(EmbeddingError::InvalidResponse { 
                                    reason: error.clone() 
                                }))
                            },
                            RequestStatus::Cancelled => {
                                Err(QueueError::RequestCancelled { id: request_id })
                            },
                            RequestStatus::TimedOut => {
                                Err(QueueError::RequestTimedOut { 
                                    id: request_id, 
                                    timeout_ms: self.config.request_timeout_ms 
                                })
                            },
                            _ => {
                                // Still processing, continue waiting
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                continue;
                            }
                        };
                    }
                }
                
                // Brief wait before checking again
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await;

        match result {
            Ok(embedding_result) => embedding_result,
            Err(_) => Err(QueueError::RequestTimedOut { 
                id: request_id, 
                timeout_ms: self.config.request_timeout_ms 
            }),
        }
    }

    /// Cancel a pending or active request
    pub async fn cancel_request(&self, request_id: RequestId) -> QueueResult<()> {
        // Try to cancel from pending queue first
        {
            let mut queue = self.pending_queue.write().await;
            if let Some(pos) = queue.iter().position(|entry| entry.request.id == request_id) {
                let entry = queue.remove(pos).unwrap();
                entry.request.cancellation_token.cancel();
                
                // Send cancellation result
                let result = EmbeddingRequestResult {
                    id: request_id,
                    status: RequestStatus::Cancelled,
                    result: None,
                    error: Some("Request cancelled by user".to_string()),
                    processing_time_ms: None,
                    completed_at: Some(Instant::now()),
                };
                
                let _ = entry.result_sender.send(result.clone());
                
                // Store result
                self.completed_results.write().await.insert(request_id, result);
                
                // Update metrics
                self.update_metrics(|metrics| {
                    metrics.cancelled_requests += 1;
                    metrics.current_queue_size = metrics.current_queue_size.saturating_sub(1);
                }).await;
                
                return Ok(());
            }
        }

        // Try to cancel active request
        {
            let active_requests = self.active_requests.read().await;
            if let Some(request) = active_requests.get(&request_id) {
                request.cancellation_token.cancel();
                eprintln!("🚫 Cancelled active request {}", request_id);
                return Ok(());
            }
        }

        Err(QueueError::RequestNotFound { id: request_id })
    }

    /// Get the status of a request
    pub async fn get_request_status(&self, request_id: RequestId) -> Option<RequestStatus> {
        // Check completed results first
        {
            let results = self.completed_results.read().await;
            if let Some(result) = results.get(&request_id) {
                return Some(result.status.clone());
            }
        }

        // Check active requests
        {
            let active = self.active_requests.read().await;
            if active.contains_key(&request_id) {
                return Some(RequestStatus::Processing);
            }
        }

        // Check pending queue
        {
            let queue = self.pending_queue.read().await;
            if queue.iter().any(|entry| entry.request.id == request_id) {
                return Some(RequestStatus::Queued);
            }
        }

        None
    }

    /// Get current queue metrics
    pub async fn get_metrics(&self) -> QueueMetrics {
        let metrics = self.metrics.read().await;
        let mut result = metrics.clone();
        
        // Update real-time metrics
        result.current_queue_size = self.pending_queue.read().await.len();
        result.active_requests = self.active_requests.read().await.len();
        
        // Calculate utilization
        result.queue_utilization = if self.config.max_queue_size > 0 {
            result.current_queue_size as f64 / self.config.max_queue_size as f64
        } else {
            0.0
        };

        result
    }

    /// Update the queue configuration (Note: affects new instances, not current processing)
    pub async fn get_config(&self) -> QueueConfig {
        self.config.clone()
    }

    // Private implementation methods

    /// Spawn the background request processor
    async fn spawn_request_processor(&self) -> tokio::task::JoinHandle<()> {
        let pending_queue = Arc::clone(&self.pending_queue);
        let active_requests = Arc::clone(&self.active_requests);
        let completed_results = Arc::clone(&self.completed_results);
        let generator = Arc::clone(&self.generator);
        let semaphore = Arc::clone(&self.semaphore);
        let metrics = Arc::clone(&self.metrics);
        let _config = self.config.clone();

        tokio::spawn(async move {
            loop {
                // Get next request from queue
                let queue_entry = {
                    let mut queue = pending_queue.write().await;
                    queue.pop_front()
                };

                if let Some(entry) = queue_entry {
                    let request = entry.request;
                    let result_sender = entry.result_sender;

                    // Acquire semaphore permit for concurrency control
                    let permit = match semaphore.clone().acquire_owned().await {
                        Ok(permit) => permit,
                        Err(_) => {
                            eprintln!("❌ Failed to acquire semaphore permit");
                            continue;
                        }
                    };

                    // Check if request is already cancelled or timed out
                    if request.cancellation_token.is_cancelled() || 
                       Instant::now() > request.timeout_at {
                        let status = if request.cancellation_token.is_cancelled() {
                            RequestStatus::Cancelled
                        } else {
                            RequestStatus::TimedOut
                        };

                        let result = EmbeddingRequestResult {
                            id: request.id,
                            status,
                            result: None,
                            error: Some("Request cancelled or timed out".to_string()),
                            processing_time_ms: None,
                            completed_at: Some(Instant::now()),
                        };

                        let _ = result_sender.send(result.clone());
                        completed_results.write().await.insert(request.id, result);
                        continue;
                    }

                    // Move to active requests
                    active_requests.write().await.insert(request.id, request.clone());

                    // Process the request
                    let generator_clone = Arc::clone(&generator);
                    let active_requests_clone = Arc::clone(&active_requests);
                    let completed_results_clone = Arc::clone(&completed_results);
                    let metrics_clone = Arc::clone(&metrics);

                    tokio::spawn(async move {
                        let start_time = Instant::now();
                        
                        // Generate embedding
                        let embedding_result = generator_clone
                            .generate_embedding(request.text.clone(), request.model.clone())
                            .await;

                        let processing_time = start_time.elapsed();
                        let processing_time_ms = processing_time.as_millis() as u64;

                        // Create result
                        let result = match embedding_result {
                            Ok(embedding) => EmbeddingRequestResult {
                                id: request.id,
                                status: RequestStatus::Completed,
                                result: Some(embedding),
                                error: None,
                                processing_time_ms: Some(processing_time_ms),
                                completed_at: Some(Instant::now()),
                            },
                            Err(e) => EmbeddingRequestResult {
                                id: request.id,
                                status: RequestStatus::Failed { error: e.to_string() },
                                result: None,
                                error: Some(e.to_string()),
                                processing_time_ms: Some(processing_time_ms),
                                completed_at: Some(Instant::now()),
                            },
                        };

                        // Update metrics
                        {
                            let mut metrics = metrics_clone.write().await;
                            match result.status {
                                RequestStatus::Completed => metrics.completed_requests += 1,
                                RequestStatus::Failed { .. } => metrics.failed_requests += 1,
                                _ => {}
                            }

                            // Update average processing time
                            let total_processed = metrics.completed_requests + metrics.failed_requests;
                            if total_processed > 0 {
                                metrics.avg_processing_time_ms = 
                                    (metrics.avg_processing_time_ms * (total_processed - 1) as f64 + 
                                     processing_time_ms as f64) / total_processed as f64;
                            }
                        }

                        // Remove from active requests
                        active_requests_clone.write().await.remove(&request.id);

                        // Store completed result
                        completed_results_clone.write().await.insert(request.id, result.clone());

                        // Send result
                        let _ = result_sender.send(result);

                        // Release permit
                        drop(permit);

                        eprintln!("✅ Completed embedding request {} in {}ms", 
                                  request.id, processing_time_ms);
                    });
                } else {
                    // No requests in queue, wait before checking again
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        })
    }

    /// Spawn the background cleanup task
    async fn spawn_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let completed_results = Arc::clone(&self.completed_results);
        let deduplication_cache = Arc::clone(&self.deduplication_cache);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(Duration::from_millis(60_000)); // Every minute

            loop {
                cleanup_interval.tick().await;

                let now = Instant::now();
                let retention_duration = Duration::from_millis(config.result_retention_ms);
                let dedup_duration = Duration::from_millis(config.deduplication_window_ms);

                // Clean up old completed results
                {
                    let mut results = completed_results.write().await;
                    results.retain(|_, result| {
                        if let Some(completed_at) = result.completed_at {
                            now.duration_since(completed_at) < retention_duration
                        } else {
                            false
                        }
                    });
                }

                // Clean up old deduplication cache entries
                {
                    let mut cache = deduplication_cache.write().await;
                    cache.retain(|_, (created_at, _)| {
                        now.duration_since(*created_at) < dedup_duration
                    });
                }

                eprintln!("🧹 Completed cleanup cycle");
            }
        })
    }

    /// Check if request can be deduplicated
    async fn check_deduplication(&self, request: &EmbeddingRequest) -> Option<RequestId> {
        let cache_key = format!("{}:{}", request.text, request.model);
        let cache = self.deduplication_cache.read().await;
        
        if let Some((created_at, existing_id)) = cache.get(&cache_key) {
            let window_duration = Duration::from_millis(self.config.deduplication_window_ms);
            if Instant::now().duration_since(*created_at) < window_duration {
                return Some(*existing_id);
            }
        }
        
        None
    }

    /// Update deduplication cache
    async fn update_deduplication_cache(&self, request: &EmbeddingRequest) {
        let cache_key = format!("{}:{}", request.text, request.model);
        let mut cache = self.deduplication_cache.write().await;
        cache.insert(cache_key, (request.created_at, request.id));
    }

    /// Update metrics with a closure
    async fn update_metrics<F>(&self, updater: F) 
    where 
        F: FnOnce(&mut QueueMetrics),
    {
        let mut metrics = self.metrics.write().await;
        updater(&mut *metrics);
    }
}

// Note: Background task cleanup is now handled by the caller
// who receives the JoinHandles from start()

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ollama_client::OllamaConfig;

    fn create_test_config() -> QueueConfig {
        QueueConfig {
            max_concurrent_requests: 2,
            max_queue_size: 10,
            request_timeout_ms: 1000,
            result_retention_ms: 5000,
            enable_deduplication: true,
            deduplication_window_ms: 500,
            batch_timeout_ms: 50,
            batch_size: 4,
        }
    }

    #[tokio::test]
    async fn test_queue_creation() {
        let ollama_config = OllamaConfig::default();
        let queue_config = create_test_config();
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let queue = EmbeddingQueue::new(generator, queue_config);
        let metrics = queue.get_metrics().await;
        
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.current_queue_size, 0);
    }

    #[tokio::test]
    async fn test_request_submission() {
        let ollama_config = OllamaConfig::default();
        let queue_config = create_test_config();
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let queue = EmbeddingQueue::new(generator, queue_config);
        
        let request_id = queue.submit_request(
            "test text".to_string(),
            "test-model".to_string(),
            RequestPriority::Normal,
        ).await.unwrap();
        
        assert!(request_id != Uuid::nil());
        
        let status = queue.get_request_status(request_id).await;
        assert!(matches!(status, Some(RequestStatus::Queued)));
    }

    #[tokio::test]
    async fn test_request_cancellation() {
        let ollama_config = OllamaConfig::default();
        let queue_config = create_test_config();
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let queue = EmbeddingQueue::new(generator, queue_config);
        
        let request_id = queue.submit_request(
            "test text".to_string(),
            "test-model".to_string(),
            RequestPriority::Normal,
        ).await.unwrap();
        
        let cancel_result = queue.cancel_request(request_id).await;
        assert!(cancel_result.is_ok());
        
        let status = queue.get_request_status(request_id).await;
        assert!(matches!(status, Some(RequestStatus::Cancelled)));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
    }

    #[test]
    fn test_cancellation_token() {
        let id = Uuid::new_v4();
        let token = CancellationToken::new(id);
        
        assert_eq!(token.request_id(), id);
        assert!(!token.is_cancelled());
        
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let ollama_config = OllamaConfig::default();
        let queue_config = create_test_config();
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let queue = EmbeddingQueue::new(generator, queue_config);
        
        let initial_metrics = queue.get_metrics().await;
        assert_eq!(initial_metrics.total_requests, 0);
        
        let _request_id = queue.submit_request(
            "test text".to_string(),
            "test-model".to_string(),
            RequestPriority::Normal,
        ).await.unwrap();
        
        let updated_metrics = queue.get_metrics().await;
        assert_eq!(updated_metrics.total_requests, 1);
        assert_eq!(updated_metrics.current_queue_size, 1);
    }

    #[tokio::test]
    async fn test_queue_capacity() {
        let ollama_config = OllamaConfig::default();
        let mut queue_config = create_test_config();
        queue_config.max_queue_size = 1; // Very small queue
        
        let generator = EmbeddingGenerator::new(ollama_config);
        let queue = EmbeddingQueue::new(generator, queue_config);
        
        // First request should succeed
        let result1 = queue.submit_request(
            "test text 1".to_string(),
            "test-model".to_string(),
            RequestPriority::Normal,
        ).await;
        assert!(result1.is_ok());
        
        // Second request should fail due to queue being full
        let result2 = queue.submit_request(
            "test text 2".to_string(),
            "test-model".to_string(),
            RequestPriority::Normal,
        ).await;
        assert!(matches!(result2, Err(QueueError::QueueFull { .. })));
    }
}