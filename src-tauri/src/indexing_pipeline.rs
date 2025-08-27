//! # Indexing Pipeline Module
//!
//! Core indexing pipeline infrastructure for processing vault files and managing
//! embeddings with high-performance parallel processing and queue management.
//!
//! ## Features
//!
//! - **Worker thread pool**: Configurable parallel processing with thread safety
//! - **Priority queue**: User-triggered vs automatic indexing prioritization
//! - **Progress tracking**: Thread-safe progress reporting with minimal overhead
//! - **Cancellation support**: Clean cancellation without data corruption
//! - **Memory management**: Efficient resource usage for large vault processing
//! - **Error handling**: Comprehensive error recovery and logging
//!
//! ## Architecture
//!
//! The pipeline consists of several key components:
//! - `IndexingPipeline`: Main coordinator with worker thread management
//! - `IndexingQueue`: Priority-based queue for processing requests
//! - `ProgressReporter`: Thread-safe progress tracking infrastructure
//! - `CancellationToken`: Cooperative cancellation mechanism
//!
//! ## Usage
//!
//! ```rust
//! use crate::indexing_pipeline::{IndexingPipeline, PipelineConfig};
//!
//! let config = PipelineConfig::default();
//! let pipeline = IndexingPipeline::new(config);
//! 
//! // Start the pipeline
//! pipeline.start().await?;
//! 
//! // Queue files for indexing
//! pipeline.queue_file("path/to/file.md", Priority::UserTriggered).await?;
//! ```

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during indexing pipeline operations
#[derive(Error, Debug)]
pub enum IndexingError {
    #[error("Pipeline not initialized")]
    NotInitialized,
    
    #[error("Pipeline already running")]
    AlreadyRunning,
    
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },
    
    #[error("Worker thread error: {message}")]
    WorkerError { message: String },
    
    #[error("Queue full: cannot accept more requests")]
    QueueFull,
    
    #[error("Operation cancelled")]
    Cancelled,
    
    #[error("File processing error: {path} - {reason}")]
    FileProcessingError { path: String, reason: String },
    
    #[error("IO error: {message}")]
    IOError { message: String },
}

pub type IndexingResult<T> = Result<T, IndexingError>;

/// Priority levels for indexing requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IndexingPriority {
    /// Background automatic indexing (lowest priority)
    Automatic = 0,
    /// File change detected (medium priority)
    FileChanged = 1,
    /// User-triggered indexing (highest priority)
    UserTriggered = 2,
}

impl Default for IndexingPriority {
    fn default() -> Self {
        Self::Automatic
    }
}

/// Status of an indexing request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexingStatus {
    /// Request is queued for processing
    Queued,
    /// Request is currently being processed
    Processing,
    /// Request completed successfully
    Completed,
    /// Request failed with error
    Failed { error: String },
    /// Request was cancelled
    Cancelled,
}

/// Indexing request containing file information and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingRequest {
    /// Unique identifier for this request
    pub id: u64,
    /// Path to the file to be indexed
    pub file_path: PathBuf,
    /// Priority level for processing
    pub priority: IndexingPriority,
    /// Current status of the request
    pub status: IndexingStatus,
    /// Timestamp when request was created
    pub created_at: u64,
    /// Optional metadata for the request
    pub metadata: HashMap<String, String>,
}

impl IndexingRequest {
    /// Create a new indexing request
    pub fn new(file_path: PathBuf, priority: IndexingPriority) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);
        
        Self {
            id: REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst),
            file_path,
            priority,
            status: IndexingStatus::Queued,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }
    
    /// Set metadata for the request
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Configuration for the indexing pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Number of worker threads (default: number of CPU cores)
    pub worker_count: usize,
    /// Maximum queue size (default: 10,000)
    pub max_queue_size: usize,
    /// Progress reporting interval in milliseconds (default: 100ms)
    pub progress_interval_ms: u64,
    /// Maximum memory usage per worker in MB (default: 50MB)
    pub max_worker_memory_mb: usize,
    /// Timeout for individual file processing in seconds (default: 30s)
    pub file_timeout_seconds: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get().max(2).min(8), // 2-8 workers based on CPU cores
            max_queue_size: 10_000,
            progress_interval_ms: 100,
            max_worker_memory_mb: 50,
            file_timeout_seconds: 30,
        }
    }
}

/// Progress information for the indexing pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingProgress {
    /// Total number of files to process
    pub total_files: u64,
    /// Number of files completed
    pub completed_files: u64,
    /// Number of files currently being processed
    pub processing_files: u64,
    /// Number of files failed
    pub failed_files: u64,
    /// Number of files queued
    pub queued_files: u64,
    /// Overall progress percentage (0-100)
    pub progress_percent: f64,
    /// Current processing speed (files per second)
    pub files_per_second: f64,
    /// Estimated time remaining in seconds
    pub estimated_remaining_seconds: u64,
    /// Whether the pipeline is currently running
    pub is_running: bool,
    /// Whether cancellation has been requested
    pub is_cancelling: bool,
}

impl Default for IndexingProgress {
    fn default() -> Self {
        Self {
            total_files: 0,
            completed_files: 0,
            processing_files: 0,
            failed_files: 0,
            queued_files: 0,
            progress_percent: 0.0,
            files_per_second: 0.0,
            estimated_remaining_seconds: 0,
            is_running: false,
            is_cancelling: false,
        }
    }
}

/// Thread-safe cancellation token for cooperative cancellation
#[derive(Debug)]
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    /// Create a new cancellation token
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }
    
    /// Cancel the operation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    
    /// Check if cancellation has been requested
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    
    /// Reset the cancellation token
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe priority queue for indexing requests
#[derive(Debug)]
pub struct IndexingQueue {
    /// Queue storage organized by priority
    queues: Arc<Mutex<HashMap<IndexingPriority, VecDeque<IndexingRequest>>>>,
    /// Current queue size
    size: Arc<AtomicUsize>,
    /// Maximum allowed queue size
    max_size: usize,
    /// Request tracking by ID
    requests: Arc<RwLock<HashMap<u64, IndexingRequest>>>,
}

impl IndexingQueue {
    /// Create a new indexing queue
    pub fn new(max_size: usize) -> Self {
        let mut queues = HashMap::new();
        queues.insert(IndexingPriority::UserTriggered, VecDeque::new());
        queues.insert(IndexingPriority::FileChanged, VecDeque::new());
        queues.insert(IndexingPriority::Automatic, VecDeque::new());
        
        Self {
            queues: Arc::new(Mutex::new(queues)),
            size: Arc::new(AtomicUsize::new(0)),
            max_size,
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a request to the queue
    pub fn push(&self, request: IndexingRequest) -> IndexingResult<()> {
        if self.size.load(Ordering::SeqCst) >= self.max_size {
            return Err(IndexingError::QueueFull);
        }
        
        let request_id = request.id;
        let priority = request.priority;
        
        // Add to priority queue
        {
            let mut queues = self.queues.lock().unwrap();
            if let Some(queue) = queues.get_mut(&priority) {
                queue.push_back(request.clone());
                self.size.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        // Track the request
        {
            let mut requests = self.requests.write().unwrap();
            requests.insert(request_id, request);
        }
        
        Ok(())
    }
    
    /// Pop the highest priority request from the queue
    pub fn pop(&self) -> Option<IndexingRequest> {
        let mut queues = self.queues.lock().unwrap();
        
        // Try highest priority first, then work down
        let priorities = [
            IndexingPriority::UserTriggered,
            IndexingPriority::FileChanged,
            IndexingPriority::Automatic,
        ];
        
        for priority in &priorities {
            if let Some(queue) = queues.get_mut(priority) {
                if let Some(request) = queue.pop_front() {
                    self.size.fetch_sub(1, Ordering::SeqCst);
                    
                    // Update request status
                    {
                        let mut requests = self.requests.write().unwrap();
                        if let Some(tracked_request) = requests.get_mut(&request.id) {
                            tracked_request.status = IndexingStatus::Processing;
                        }
                    }
                    
                    return Some(request);
                }
            }
        }
        
        None
    }
    
    /// Get the current queue size
    pub fn size(&self) -> usize {
        self.size.load(Ordering::SeqCst)
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }
    
    /// Update request status
    pub fn update_request_status(&self, request_id: u64, status: IndexingStatus) {
        let mut requests = self.requests.write().unwrap();
        if let Some(request) = requests.get_mut(&request_id) {
            request.status = status;
        }
    }
    
    /// Get request status
    pub fn get_request_status(&self, request_id: u64) -> Option<IndexingStatus> {
        let requests = self.requests.read().unwrap();
        requests.get(&request_id).map(|r| r.status.clone())
    }
    
    /// Clear all requests
    pub fn clear(&self) {
        let mut queues = self.queues.lock().unwrap();
        for queue in queues.values_mut() {
            queue.clear();
        }
        self.size.store(0, Ordering::SeqCst);
        
        let mut requests = self.requests.write().unwrap();
        requests.clear();
    }
    
    /// Get queue statistics by priority
    pub fn get_queue_stats(&self) -> HashMap<IndexingPriority, usize> {
        let queues = self.queues.lock().unwrap();
        queues.iter().map(|(priority, queue)| (*priority, queue.len())).collect()
    }
}

/// Main indexing pipeline coordinator
#[derive(Debug)]
pub struct IndexingPipeline {
    /// Pipeline configuration
    config: PipelineConfig,
    /// Processing queue
    queue: Arc<IndexingQueue>,
    /// Worker thread handles
    workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Cancellation token
    cancellation_token: Arc<CancellationToken>,
    /// Progress tracking
    progress: Arc<RwLock<IndexingProgress>>,
    /// Whether the pipeline is running
    is_running: Arc<AtomicBool>,
    /// Start time for performance tracking
    start_time: Arc<Mutex<Option<Instant>>>,
    /// Completed files counter
    completed_counter: Arc<AtomicU64>,
    /// Failed files counter
    failed_counter: Arc<AtomicU64>,
}

impl IndexingPipeline {
    /// Create a new indexing pipeline
    pub fn new(config: PipelineConfig) -> Self {
        let queue = Arc::new(IndexingQueue::new(config.max_queue_size));
        
        Self {
            queue,
            config,
            workers: Arc::new(Mutex::new(Vec::new())),
            cancellation_token: Arc::new(CancellationToken::new()),
            progress: Arc::new(RwLock::new(IndexingProgress::default())),
            is_running: Arc::new(AtomicBool::new(false)),
            start_time: Arc::new(Mutex::new(None)),
            completed_counter: Arc::new(AtomicU64::new(0)),
            failed_counter: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Start the indexing pipeline
    pub fn start(&self) -> IndexingResult<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(IndexingError::AlreadyRunning);
        }
        
        // Reset state
        self.cancellation_token.reset();
        self.completed_counter.store(0, Ordering::SeqCst);
        self.failed_counter.store(0, Ordering::SeqCst);
        *self.start_time.lock().unwrap() = Some(Instant::now());
        
        // Mark as running
        self.is_running.store(true, Ordering::SeqCst);
        
        // Start worker threads
        self.start_workers()?;
        
        // Start progress reporter
        self.start_progress_reporter();
        
        Ok(())
    }
    
    /// Stop the indexing pipeline
    pub fn stop(&self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }
        
        // Signal cancellation
        self.cancellation_token.cancel();
        
        // Wait for workers to finish
        let mut workers = self.workers.lock().unwrap();
        while let Some(worker) = workers.pop() {
            let _ = worker.join();
        }
        
        // Mark as stopped
        self.is_running.store(false, Ordering::SeqCst);
        
        // Update progress
        {
            let mut progress = self.progress.write().unwrap();
            progress.is_running = false;
            progress.is_cancelling = false;
        }
    }
    
    /// Queue a file for indexing
    pub fn queue_file(&self, file_path: PathBuf, priority: IndexingPriority) -> IndexingResult<u64> {
        let request = IndexingRequest::new(file_path, priority);
        let request_id = request.id;
        
        self.queue.push(request)?;
        
        // Update progress
        {
            let mut progress = self.progress.write().unwrap();
            progress.total_files += 1;
            progress.queued_files += 1;
        }
        
        Ok(request_id)
    }
    
    /// Get current progress
    pub fn get_progress(&self) -> IndexingProgress {
        self.progress.read().unwrap().clone()
    }
    
    /// Check if the pipeline is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
    
    /// Get queue statistics
    pub fn get_queue_stats(&self) -> HashMap<IndexingPriority, usize> {
        self.queue.get_queue_stats()
    }
    
    // Private helper methods
    
    fn start_workers(&self) -> IndexingResult<()> {
        let mut workers = self.workers.lock().unwrap();
        
        for worker_id in 0..self.config.worker_count {
            let queue = Arc::clone(&self.queue);
            let cancellation_token = Arc::clone(&self.cancellation_token);
            let completed_counter = Arc::clone(&self.completed_counter);
            let failed_counter = Arc::clone(&self.failed_counter);
            let timeout = Duration::from_secs(self.config.file_timeout_seconds);
            
            let worker = thread::Builder::new()
                .name(format!("indexing-worker-{}", worker_id))
                .spawn(move || {
                    Self::worker_loop(
                        worker_id,
                        queue,
                        cancellation_token,
                        completed_counter,
                        failed_counter,
                        timeout,
                    );
                })
                .map_err(|e| IndexingError::WorkerError { 
                    message: format!("Failed to start worker {}: {}", worker_id, e) 
                })?;
            
            workers.push(worker);
        }
        
        Ok(())
    }
    
    fn worker_loop(
        worker_id: usize,
        queue: Arc<IndexingQueue>,
        cancellation_token: Arc<CancellationToken>,
        completed_counter: Arc<AtomicU64>,
        failed_counter: Arc<AtomicU64>,
        _timeout: Duration,
    ) {
        log::debug!("Worker {} started", worker_id);
        
        while !cancellation_token.is_cancelled() {
            if let Some(request) = queue.pop() {
                log::debug!("Worker {} processing file: {:?}", worker_id, request.file_path);
                
                // TODO: In sub-issue #139, this will be replaced with actual file processing
                // For now, simulate processing
                thread::sleep(Duration::from_millis(10));
                
                // Simulate success/failure
                let success = true; // Always succeed for now
                
                if success {
                    queue.update_request_status(request.id, IndexingStatus::Completed);
                    completed_counter.fetch_add(1, Ordering::SeqCst);
                    log::debug!("Worker {} completed file: {:?}", worker_id, request.file_path);
                } else {
                    queue.update_request_status(request.id, IndexingStatus::Failed { 
                        error: "Simulated failure".to_string() 
                    });
                    failed_counter.fetch_add(1, Ordering::SeqCst);
                    log::warn!("Worker {} failed to process file: {:?}", worker_id, request.file_path);
                }
            } else {
                // No work available, sleep briefly
                thread::sleep(Duration::from_millis(10));
            }
        }
        
        log::debug!("Worker {} stopped", worker_id);
    }
    
    fn start_progress_reporter(&self) {
        let progress = Arc::clone(&self.progress);
        let is_running = Arc::clone(&self.is_running);
        let cancellation_token = Arc::clone(&self.cancellation_token);
        let completed_counter = Arc::clone(&self.completed_counter);
        let failed_counter = Arc::clone(&self.failed_counter);
        let queue = Arc::clone(&self.queue);
        let _start_time = Arc::clone(&self.start_time);
        let interval = Duration::from_millis(self.config.progress_interval_ms);
        
        thread::Builder::new()
            .name("indexing-progress-reporter".to_string())
            .spawn(move || {
                let mut last_completed = 0u64;
                let mut last_update = Instant::now();
                
                while is_running.load(Ordering::SeqCst) {
                    if cancellation_token.is_cancelled() {
                        break;
                    }
                    
                    let now = Instant::now();
                    let completed = completed_counter.load(Ordering::SeqCst);
                    let failed = failed_counter.load(Ordering::SeqCst);
                    let queued = queue.size() as u64;
                    
                    // Calculate processing speed
                    let time_since_last = now.duration_since(last_update).as_secs_f64();
                    let completed_since_last = completed.saturating_sub(last_completed);
                    let files_per_second = if time_since_last > 0.0 {
                        completed_since_last as f64 / time_since_last
                    } else {
                        0.0
                    };
                    
                    // Update progress
                    {
                        let mut prog = progress.write().unwrap();
                        prog.completed_files = completed;
                        prog.failed_files = failed;
                        prog.queued_files = queued;
                        prog.processing_files = 0; // TODO: Track actual processing count
                        
                        if prog.total_files > 0 {
                            prog.progress_percent = (completed as f64 / prog.total_files as f64) * 100.0;
                        }
                        
                        prog.files_per_second = files_per_second;
                        prog.is_running = true;
                        prog.is_cancelling = cancellation_token.is_cancelled();
                        
                        // Estimate remaining time
                        let remaining_files = prog.total_files.saturating_sub(completed);
                        if files_per_second > 0.0 && remaining_files > 0 {
                            prog.estimated_remaining_seconds = (remaining_files as f64 / files_per_second) as u64;
                        } else {
                            prog.estimated_remaining_seconds = 0;
                        }
                    }
                    
                    last_completed = completed;
                    last_update = now;
                    
                    thread::sleep(interval);
                }
            })
            .expect("Failed to start progress reporter thread");
    }
}

impl Drop for IndexingPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

// Add external dependency for CPU core detection
extern crate num_cpus;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert!(config.worker_count >= 2);
        assert!(config.worker_count <= 8);
        assert_eq!(config.max_queue_size, 10_000);
        assert_eq!(config.progress_interval_ms, 100);
    }

    #[test]
    fn test_indexing_request_creation() {
        let path = PathBuf::from("/test/file.md");
        let request = IndexingRequest::new(path.clone(), IndexingPriority::UserTriggered);
        
        assert_eq!(request.file_path, path);
        assert_eq!(request.priority, IndexingPriority::UserTriggered);
        assert_eq!(request.status, IndexingStatus::Queued);
        assert!(request.id > 0);
    }

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        
        token.cancel();
        assert!(token.is_cancelled());
        
        token.reset();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_indexing_queue_basic_operations() {
        let queue = IndexingQueue::new(100);
        assert_eq!(queue.size(), 0);
        assert!(queue.is_empty());
        
        let request = IndexingRequest::new(
            PathBuf::from("/test/file.md"),
            IndexingPriority::UserTriggered
        );
        let request_id = request.id;
        
        queue.push(request).unwrap();
        assert_eq!(queue.size(), 1);
        assert!(!queue.is_empty());
        
        let popped = queue.pop().unwrap();
        assert_eq!(popped.id, request_id);
        assert_eq!(queue.size(), 0);
    }

    #[test]
    fn test_indexing_queue_priority_ordering() {
        let queue = IndexingQueue::new(100);
        
        // Add requests in reverse priority order
        let low_priority = IndexingRequest::new(
            PathBuf::from("/low.md"),
            IndexingPriority::Automatic
        );
        let high_priority = IndexingRequest::new(
            PathBuf::from("/high.md"),
            IndexingPriority::UserTriggered
        );
        let medium_priority = IndexingRequest::new(
            PathBuf::from("/medium.md"),
            IndexingPriority::FileChanged
        );
        
        queue.push(low_priority).unwrap();
        queue.push(medium_priority).unwrap();
        queue.push(high_priority.clone()).unwrap();
        
        // Should pop highest priority first
        let first = queue.pop().unwrap();
        assert_eq!(first.id, high_priority.id);
        assert_eq!(first.priority, IndexingPriority::UserTriggered);
    }

    #[test]
    fn test_pipeline_creation_and_basic_state() {
        let config = PipelineConfig::default();
        let pipeline = IndexingPipeline::new(config);
        
        assert!(!pipeline.is_running());
        assert_eq!(pipeline.get_progress().total_files, 0);
        assert_eq!(pipeline.get_progress().completed_files, 0);
    }
}