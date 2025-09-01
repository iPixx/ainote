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
use tokio::time::timeout;
use glob::glob;

use crate::text_chunker::ChunkProcessor;
use crate::embedding_generator::EmbeddingGenerator;
use crate::vector_db::VectorDatabase;

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
    /// Enable resume capability (saves pipeline state)
    pub enable_resume: bool,
    /// Path for saving pipeline state
    pub state_file_path: Option<String>,
    /// Embedding model name (default: "nomic-embed-text")
    pub embedding_model: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get().clamp(2, 8), // 2-8 workers based on CPU cores
            max_queue_size: 10_000,
            progress_interval_ms: 100,
            max_worker_memory_mb: 50,
            file_timeout_seconds: 30,
            enable_resume: true,
            state_file_path: Some(".ainote/indexing_pipeline_state.json".to_string()),
            embedding_model: "nomic-embed-text".to_string(),
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

/// Serializable pipeline state for resume capability
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineState {
    /// Pending requests in the queue
    pub pending_requests: Vec<IndexingRequest>,
    /// Progress information
    pub progress: IndexingProgress,
    /// Timestamp when state was saved
    pub saved_at: u64,
    /// Configuration used when state was saved
    pub config: PipelineConfig,
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
    /// Text chunker for processing files
    text_chunker: Arc<ChunkProcessor>,
    /// Embedding generator for creating vectors
    embedding_generator: Arc<EmbeddingGenerator>,
    /// Vector database for storing embeddings
    vector_db: Arc<VectorDatabase>,
}

impl IndexingPipeline {
    /// Create a new indexing pipeline
    pub fn new(
        config: PipelineConfig,
        text_chunker: Arc<ChunkProcessor>,
        embedding_generator: Arc<EmbeddingGenerator>,
        vector_db: Arc<VectorDatabase>,
    ) -> Self {
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
            text_chunker,
            embedding_generator,
            vector_db,
        }
    }
    
    /// Start the indexing pipeline
    pub async fn start(&self) -> IndexingResult<()> {
        log::info!("🚀 Starting indexing pipeline with {} workers", self.config.worker_count);
        if self.is_running.load(Ordering::SeqCst) {
            log::warn!("⚠️ Pipeline already running");
            return Err(IndexingError::AlreadyRunning);
        }
        
        // Try to restore state if resume is enabled
        let restored = self.restore_state().await?;
        if restored {
            log::info!("🔄 Pipeline resumed from saved state");
        }
        
        // Reset counters but preserve queue state if restored
        self.cancellation_token.reset();
        if !restored {
            self.completed_counter.store(0, Ordering::SeqCst);
            self.failed_counter.store(0, Ordering::SeqCst);
        }
        *self.start_time.lock().unwrap() = Some(Instant::now());
        
        // Mark as running
        self.is_running.store(true, Ordering::SeqCst);
        
        // Start worker threads
        self.start_workers()?;
        
        // Start progress reporter
        self.start_progress_reporter();
        
        log::info!("▶️ Indexing pipeline started with {} workers", self.config.worker_count);
        
        Ok(())
    }
    
    /// Stop the indexing pipeline
    pub async fn stop(&self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }
        
        log::info!("⏹️ Stopping indexing pipeline...");
        
        // Save state before stopping (if resume is enabled)
        if let Err(e) = self.save_state().await {
            log::warn!("⚠️ Failed to save pipeline state: {}", e);
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
        
        log::info!("✅ Indexing pipeline stopped");
    }
    
    /// Queue a file for indexing
    pub fn queue_file(&self, file_path: PathBuf, priority: IndexingPriority) -> IndexingResult<u64> {
        let request = IndexingRequest::new(file_path.clone(), priority);
        let request_id = request.id;
        log::info!("📝 Queuing file for indexing: {:?} (priority: {:?}, id: {})", file_path, priority, request_id);
        
        self.queue.push(request)?;
        
        // Update progress
        {
            let mut progress = self.progress.write().unwrap();
            progress.total_files += 1;
            progress.queued_files += 1;
        }
        
        Ok(request_id)
    }
    
    /// Bulk index an entire vault directory
    /// 
    /// This method scans the vault directory for markdown files and queues them
    /// for bulk indexing with configurable concurrency.
    /// 
    /// # Arguments
    /// 
    /// * `vault_path` - Path to the vault directory
    /// * `priority` - Priority level for the indexing requests  
    /// * `file_pattern` - Optional glob pattern to filter files (default: "**/*.md")
    /// 
    /// # Returns
    /// 
    /// Vector of request IDs for the queued files
    pub async fn bulk_index_vault(
        &self,
        vault_path: PathBuf,
        priority: IndexingPriority,
        file_pattern: Option<String>,
    ) -> IndexingResult<Vec<u64>> {
        
        log::info!("🗂️ Starting bulk vault indexing: {:?}", vault_path);
        
        if !vault_path.exists() {
            return Err(IndexingError::IOError {
                message: format!("Vault path does not exist: {:?}", vault_path),
            });
        }
        
        if !vault_path.is_dir() {
            return Err(IndexingError::IOError {
                message: format!("Vault path is not a directory: {:?}", vault_path),
            });
        }
        
        // Build glob pattern for markdown files
        let pattern = file_pattern.unwrap_or_else(|| "**/*.md".to_string());
        let full_pattern = vault_path.join(&pattern);
        
        let pattern_str = full_pattern.to_string_lossy();
        log::debug!("📂 Scanning vault with pattern: {}", pattern_str);
        
        // Find all matching files
        let mut markdown_files = Vec::new();
        
        match glob(&pattern_str) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                markdown_files.push(path);
                            }
                        }
                        Err(e) => {
                            log::warn!("⚠️ Error reading file entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(IndexingError::IOError {
                    message: format!("Failed to scan vault directory: {}", e),
                });
            }
        }
        
        log::info!("📝 Found {} markdown files to index", markdown_files.len());
        
        // Queue all files for indexing
        let mut request_ids = Vec::new();
        
        for file_path in markdown_files {
            match self.queue_file(file_path.clone(), priority) {
                Ok(request_id) => {
                    request_ids.push(request_id);
                    log::debug!("📋 Queued file for indexing: {:?} (ID: {})", file_path, request_id);
                }
                Err(IndexingError::QueueFull) => {
                    log::warn!("⚠️ Queue is full, cannot queue more files. Consider increasing max_queue_size.");
                    break;
                }
                Err(e) => {
                    log::error!("❌ Failed to queue file {:?}: {}", file_path, e);
                    return Err(e);
                }
            }
        }
        
        log::info!("✅ Bulk vault indexing queued {} files with {} request IDs", 
                   request_ids.len(), request_ids.len());
        
        Ok(request_ids)
    }
    
    /// Index files with real-time debouncing
    /// 
    /// This method handles real-time file changes with debouncing to avoid
    /// excessive indexing when files are rapidly modified.
    /// 
    /// # Arguments
    /// 
    /// * `file_paths` - Files that have changed
    /// * `debounce_ms` - Debounce time in milliseconds (default: 1000ms)
    /// 
    /// # Returns
    /// 
    /// Vector of request IDs for queued files after debouncing
    pub async fn index_files_debounced(
        &self,
        file_paths: Vec<PathBuf>,
        debounce_ms: Option<u64>,
    ) -> IndexingResult<Vec<u64>> {
        let debounce_duration = debounce_ms.unwrap_or(1000);
        
        log::debug!("⏰ Debouncing {} file changes ({}ms)", file_paths.len(), debounce_duration);
        
        // Simple debouncing: wait for the debounce period before processing
        tokio::time::sleep(Duration::from_millis(debounce_duration)).await;
        
        // Check if cancellation was requested during debounce
        if self.cancellation_token.is_cancelled() {
            return Err(IndexingError::Cancelled);
        }
        
        let mut request_ids = Vec::new();
        
        for file_path in file_paths {
            // Check if file still exists after debounce period
            if !file_path.exists() {
                log::debug!("📁 File no longer exists, skipping: {:?}", file_path);
                continue;
            }
            
            match self.queue_file(file_path.clone(), IndexingPriority::FileChanged) {
                Ok(request_id) => {
                    request_ids.push(request_id);
                    log::debug!("🔄 Queued file change for indexing: {:?} (ID: {})", file_path, request_id);
                }
                Err(e) => {
                    log::error!("❌ Failed to queue file change {:?}: {}", file_path, e);
                    return Err(e);
                }
            }
        }
        
        log::info!("✅ Real-time indexing queued {} files after debouncing", request_ids.len());
        
        Ok(request_ids)
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
    
    /// Save pipeline state for resume capability
    pub async fn save_state(&self) -> IndexingResult<()> {
        if !self.config.enable_resume {
            return Ok(());
        }
        
        let state_file = match &self.config.state_file_path {
            Some(path) => path,
            None => return Ok(()),
        };
        
        // Create state directory if it doesn't exist
        if let Some(parent_dir) = std::path::Path::new(state_file).parent() {
            if !parent_dir.exists() {
                std::fs::create_dir_all(parent_dir).map_err(|e| {
                    IndexingError::IOError {
                        message: format!("Failed to create state directory: {}", e),
                    }
                })?;
            }
        }
        
        // Collect pending requests from queue
        let mut pending_requests = Vec::new();
        
        // Note: This is a simplified approach - in a production system,
        // you'd want to implement a more sophisticated queue draining mechanism
        while let Some(request) = self.queue.pop() {
            pending_requests.push(request);
        }
        
        // Re-add requests to queue (preserving order)
        for request in &pending_requests {
            let _ = self.queue.push(request.clone());
        }
        
        let state = PipelineState {
            pending_requests,
            progress: self.get_progress(),
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            config: self.config.clone(),
        };
        
        let serialized = serde_json::to_string_pretty(&state).map_err(|e| {
            IndexingError::IOError {
                message: format!("Failed to serialize pipeline state: {}", e),
            }
        })?;
        
        std::fs::write(state_file, serialized).map_err(|e| {
            IndexingError::IOError {
                message: format!("Failed to write pipeline state: {}", e),
            }
        })?;
        
        log::info!("💾 Saved pipeline state to {}", state_file);
        Ok(())
    }
    
    /// Load and restore pipeline state for resume capability
    pub async fn restore_state(&self) -> IndexingResult<bool> {
        if !self.config.enable_resume {
            return Ok(false);
        }
        
        let state_file = match &self.config.state_file_path {
            Some(path) => path,
            None => return Ok(false),
        };
        
        if !std::path::Path::new(state_file).exists() {
            log::debug!("📜 No pipeline state file found: {}", state_file);
            return Ok(false);
        }
        
        let content = std::fs::read_to_string(state_file).map_err(|e| {
            IndexingError::IOError {
                message: format!("Failed to read pipeline state: {}", e),
            }
        })?;
        
        let state: PipelineState = serde_json::from_str(&content).map_err(|e| {
            IndexingError::IOError {
                message: format!("Failed to deserialize pipeline state: {}", e),
            }
        })?;
        
        log::info!("🔄 Restoring pipeline state from {} (saved at: {})", 
                  state_file, state.saved_at);
        
        // Restore pending requests to queue
        for request in state.pending_requests {
            let _ = self.queue.push(request);
        }
        
        // Restore progress information
        {
            let mut progress = self.progress.write().unwrap();
            *progress = state.progress;
            progress.is_running = false; // Reset running state
            progress.is_cancelling = false;
        }
        
        log::info!("✅ Restored {} pending requests from state", 
                  self.queue.size());
        
        Ok(true)
    }
    
    /// Clear saved pipeline state
    pub async fn clear_saved_state(&self) -> IndexingResult<()> {
        if !self.config.enable_resume {
            return Ok(());
        }
        
        let state_file = match &self.config.state_file_path {
            Some(path) => path,
            None => return Ok(()),
        };
        
        if std::path::Path::new(state_file).exists() {
            std::fs::remove_file(state_file).map_err(|e| {
                IndexingError::IOError {
                    message: format!("Failed to remove pipeline state file: {}", e),
                }
            })?;
            
            log::info!("🗑️ Cleared saved pipeline state: {}", state_file);
        }
        
        Ok(())
    }
    
    // Private helper methods
    
    fn start_workers(&self) -> IndexingResult<()> {
        let mut workers = self.workers.lock().unwrap();
        
        for worker_id in 0..self.config.worker_count {
            let queue = Arc::clone(&self.queue);
            let cancellation_token = Arc::clone(&self.cancellation_token);
            let completed_counter = Arc::clone(&self.completed_counter);
            let failed_counter = Arc::clone(&self.failed_counter);
            let text_chunker = Arc::clone(&self.text_chunker);
            let embedding_generator = Arc::clone(&self.embedding_generator);
            let vector_db = Arc::clone(&self.vector_db);
            let timeout = Duration::from_secs(self.config.file_timeout_seconds);
            let embedding_model = self.config.embedding_model.clone();
            
            let worker = thread::Builder::new()
                .name(format!("indexing-worker-{}", worker_id))
                .spawn(move || {
                    // Create async runtime for this worker thread
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to create async runtime for worker");
                    
                    rt.block_on(Self::async_worker_loop(
                        worker_id,
                        queue,
                        cancellation_token,
                        completed_counter,
                        failed_counter,
                        text_chunker,
                        embedding_generator,
                        vector_db,
                        timeout,
                        embedding_model,
                    ));
                })
                .map_err(|e| IndexingError::WorkerError { 
                    message: format!("Failed to start worker {}: {}", worker_id, e) 
                })?;
            
            workers.push(worker);
        }
        
        Ok(())
    }
    
    async fn async_worker_loop(
        worker_id: usize,
        queue: Arc<IndexingQueue>,
        cancellation_token: Arc<CancellationToken>,
        completed_counter: Arc<AtomicU64>,
        failed_counter: Arc<AtomicU64>,
        text_chunker: Arc<ChunkProcessor>,
        embedding_generator: Arc<EmbeddingGenerator>,
        vector_db: Arc<VectorDatabase>,
        file_timeout: Duration,
        embedding_model: String,
    ) {
        log::debug!("🔧 Worker {} started", worker_id);
        
        log::debug!("🔧 Worker {} started and waiting for files", worker_id);
        while !cancellation_token.is_cancelled() {
            if let Some(request) = queue.pop() {
                log::info!("🔄 Worker {} processing file: {:?}", worker_id, request.file_path);
                
                let file_path = request.file_path.clone();
                let request_id = request.id;
                
                // Process the file with timeout
                let processing_result = timeout(
                    file_timeout,
                    Self::process_file(
                        worker_id,
                        &file_path,
                        &text_chunker,
                        &embedding_generator,
                        &vector_db,
                        &cancellation_token,
                        &embedding_model,
                    ),
                ).await;
                
                match processing_result {
                    Ok(Ok(())) => {
                        // File processed successfully
                        queue.update_request_status(request_id, IndexingStatus::Completed);
                        completed_counter.fetch_add(1, Ordering::SeqCst);
                        log::debug!("✅ Worker {} completed file: {:?}", worker_id, file_path);
                    }
                    Ok(Err(error)) => {
                        // File processing failed
                        let error_msg = format!("Processing failed: {}", error);
                        queue.update_request_status(request_id, IndexingStatus::Failed { 
                            error: error_msg.clone()
                        });
                        failed_counter.fetch_add(1, Ordering::SeqCst);
                        log::warn!("⚠️ Worker {} failed to process file {:?}: {}", worker_id, file_path, error_msg);
                    }
                    Err(_timeout_error) => {
                        // File processing timed out
                        let error_msg = format!("Processing timed out after {:?}", file_timeout);
                        queue.update_request_status(request_id, IndexingStatus::Failed { 
                            error: error_msg.clone()
                        });
                        failed_counter.fetch_add(1, Ordering::SeqCst);
                        log::warn!("⏰ Worker {} timed out processing file {:?}: {}", worker_id, file_path, error_msg);
                    }
                }
            } else {
                // No work available, sleep briefly
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        
        log::debug!("🛑 Worker {} stopped", worker_id);
    }
    
    /// Process a single file by chunking, generating embeddings, and storing them
    async fn process_file(
        worker_id: usize,
        file_path: &PathBuf,
        text_chunker: &ChunkProcessor,
        embedding_generator: &EmbeddingGenerator,
        vector_db: &VectorDatabase,
        cancellation_token: &CancellationToken,
        embedding_model: &str,
    ) -> IndexingResult<()> {
        // Check cancellation before starting
        if cancellation_token.is_cancelled() {
            return Err(IndexingError::Cancelled);
        }
        
        // Read file content
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            IndexingError::FileProcessingError {
                path: file_path.to_string_lossy().to_string(),
                reason: format!("Failed to read file: {}", e),
            }
        })?;
        
        if content.is_empty() {
            log::debug!("📄 File is empty, skipping: {:?}", file_path);
            return Ok(());
        }
        
        log::debug!("📝 Worker {} read {} characters from {:?}", worker_id, content.len(), file_path);
        
        // Check cancellation before chunking
        if cancellation_token.is_cancelled() {
            return Err(IndexingError::Cancelled);
        }
        
        // Chunk the text content
        let chunks = text_chunker.chunk_text(&content).map_err(|e| {
            IndexingError::FileProcessingError {
                path: file_path.to_string_lossy().to_string(),
                reason: format!("Text chunking failed: {}", e),
            }
        })?;
        
        log::debug!("🧩 Worker {} created {} chunks from {:?}", worker_id, chunks.len(), file_path);
        
        if chunks.is_empty() {
            log::debug!("📄 No chunks created from file, skipping: {:?}", file_path);
            return Ok(());
        }
        
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Process each chunk
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            // Check cancellation for each chunk
            if cancellation_token.is_cancelled() {
                return Err(IndexingError::Cancelled);
            }
            
            let chunk_id = format!("chunk_{}", chunk_index);
            
            log::debug!("🔄 Worker {} processing chunk {} ({} chars) from {:?}", 
                       worker_id, chunk_index, chunk.content.len(), file_path);
            
            // Generate embedding for chunk
            log::info!("🤖 Worker {} requesting embedding for chunk {} using model '{}'", 
                       worker_id, chunk_index, embedding_model);
            let embedding = embedding_generator.generate_embedding(chunk.content.clone(), embedding_model.to_string()).await.map_err(|e| {
                IndexingError::FileProcessingError {
                    path: file_path_str.clone(),
                    reason: format!("Embedding generation failed for chunk {}: {}", chunk_index, e),
                }
            })?;
            
            log::debug!("🔢 Worker {} generated embedding (dim: {}) for chunk {} from {:?}", 
                       worker_id, embedding.len(), chunk_index, file_path);
            
            // Store embedding directly in vector database
            let entry_id = vector_db.store_embedding(
                embedding,
                file_path_str.clone(),
                chunk_id.clone(),
                &chunk.content,
                embedding_model.to_string(),
            ).await.map_err(|e| {
                IndexingError::FileProcessingError {
                    path: file_path_str.clone(),
                    reason: format!("Failed to store embedding for chunk {}: {}", chunk_index, e),
                }
            })?;
            
            log::debug!("💾 Worker {} stored embedding {} for chunk {} from {:?}", 
                       worker_id, entry_id, chunk_index, file_path);
        }
        
        log::info!("✅ Worker {} successfully processed file {:?} ({} chunks)", 
                  worker_id, file_path, chunks.len());
        
        Ok(())
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

impl std::fmt::Debug for IndexingPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexingPipeline")
            .field("config", &self.config)
            .field("queue_size", &self.queue.size())
            .field("is_running", &self.is_running())
            .field("progress", &self.get_progress())
            .finish()
    }
}

impl Drop for IndexingPipeline {
    fn drop(&mut self) {
        // Note: Can't use async in Drop, so we create a runtime
        let rt = tokio::runtime::Runtime::new();
        if let Ok(rt) = rt {
            rt.block_on(self.stop());
        }
    }
}

// Add external dependencies
extern crate num_cpus;
extern crate glob;
extern crate tokio;
extern crate log;

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
        // Note: First request will have ID 0, subsequent requests increment
        // ID is always non-negative for unsigned types
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
    fn test_pipeline_config_creation() {
        let config = PipelineConfig::default();
        
        assert!(config.worker_count >= 2);
        assert!(config.worker_count <= 8);
        assert_eq!(config.max_queue_size, 10_000);
        assert_eq!(config.progress_interval_ms, 100);
        assert_eq!(config.max_worker_memory_mb, 50);
        assert_eq!(config.file_timeout_seconds, 30);
        assert!(config.enable_resume);
        assert!(config.state_file_path.is_some());
    }
}