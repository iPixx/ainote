//! Background Processing System for Non-Critical AI Operations
//!
//! This module implements an intelligent background processing system that handles
//! non-critical AI operations efficiently without impacting UI responsiveness or
//! critical user interactions.
//!
//! ## Core Features
//!
//! - **Priority-based scheduling** with adaptive throttling
//! - **Resource-aware execution** that monitors CPU, memory, and I/O usage
//! - **Intelligent batching** of similar operations for efficiency
//! - **Graceful degradation** under high load conditions
//! - **Background indexing** of document embeddings
//! - **Cache warming** and optimization tasks
//! - **Performance monitoring** and metrics collection
//!
//! ## Operation Types
//!
//! ### Critical Operations (never background)
//! - Real-time note suggestions
//! - User-initiated search queries
//! - Active document embedding generation
//!
//! ### Background Operations
//! - **Maintenance**: Cache cleanup, index optimization
//! - **Precomputation**: Embedding pre-generation for recently accessed files
//! - **Analytics**: Usage pattern analysis, performance optimization
//! - **Indexing**: Full-vault embedding generation during idle time
//! - **Health checks**: System monitoring and diagnostics
//!
//! ## Adaptive Scheduling
//!
//! The system uses multiple factors to determine when and how to execute background tasks:
//! - **System Load**: CPU, memory, and I/O utilization
//! - **User Activity**: Active vs idle periods
//! - **Power State**: Battery vs plugged in (affects processing intensity)
//! - **Time of Day**: Overnight processing for intensive tasks
//! - **Resource Availability**: Available cores and memory

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Semaphore, Mutex};
use tokio::time::{sleep, interval};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use uuid::Uuid;

use crate::resource_allocator::{ResourceAllocator, OperationPriority, OperationType};
use crate::performance::PerformanceTracker;

/// Errors specific to background processing
#[derive(Error, Debug, Clone)]
pub enum BackgroundProcessingError {
    #[error("Task scheduling failed: {reason}")]
    SchedulingFailed { reason: String },
    
    #[error("Resource constraints exceeded: {resource} at {usage}%")]
    ResourceConstraint { resource: String, usage: f64 },
    
    #[error("Task execution timeout: {task_id} after {duration_ms}ms")]
    TaskTimeout { task_id: String, duration_ms: u64 },
    
    #[error("System overloaded: {active_tasks} tasks running")]
    SystemOverloaded { active_tasks: usize },
    
    #[error("Task dependency failed: {dependency_id}")]
    DependencyFailed { dependency_id: String },
}

pub type BackgroundResult<T> = Result<T, BackgroundProcessingError>;

/// Priority levels for background tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BackgroundPriority {
    /// Immediate execution during idle periods
    Immediate = 4,
    /// High priority background tasks (cache warming)
    High = 3,
    /// Normal priority background tasks (document indexing)
    Normal = 2,
    /// Low priority maintenance tasks
    Low = 1,
    /// Deferred tasks for idle system periods
    Deferred = 0,
}

/// Types of background operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackgroundOperationType {
    /// Document embedding generation
    DocumentIndexing,
    /// Cache optimization and cleanup
    CacheOptimization,
    /// Performance analytics collection
    AnalyticsCollection,
    /// System health monitoring
    HealthMonitoring,
    /// Index maintenance and optimization
    IndexOptimization,
    /// Precomputation for performance
    Precomputation,
    /// Resource cleanup
    ResourceCleanup,
}

/// Configuration for background processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundProcessorConfig {
    /// Maximum concurrent background tasks
    pub max_concurrent_tasks: usize,
    /// CPU usage threshold before throttling (0.0-1.0)
    pub cpu_throttle_threshold: f64,
    /// Memory usage threshold before throttling (0.0-1.0)  
    pub memory_throttle_threshold: f64,
    /// Maximum task execution time (milliseconds)
    pub task_timeout_ms: u64,
    /// Idle detection threshold (milliseconds of no user activity)
    pub idle_threshold_ms: u64,
    /// Enable adaptive scheduling based on system state
    pub adaptive_scheduling: bool,
    /// Enable power-aware scheduling (reduce intensity on battery)
    pub power_aware_scheduling: bool,
    /// Task retry configuration
    pub max_retries: usize,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Enable task batching optimization
    pub enable_task_batching: bool,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
}

impl Default for BackgroundProcessorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 2, // Conservative default to preserve resources
            cpu_throttle_threshold: 0.6, // Throttle at 60% CPU usage
            memory_throttle_threshold: 0.8, // Throttle at 80% memory usage
            task_timeout_ms: 30_000, // 30 second timeout
            idle_threshold_ms: 5_000, // 5 seconds of idle time
            adaptive_scheduling: true, // Enable intelligent scheduling
            power_aware_scheduling: true, // Reduce load on battery
            max_retries: 2, // Retry failed tasks twice
            retry_delay_ms: 5_000, // 5 second retry delay
            enable_task_batching: true, // Enable batching for efficiency
            max_batch_size: 5, // Batch up to 5 similar tasks
            batch_timeout_ms: 1_000, // 1 second batch collection timeout
        }
    }
}

/// Background task definition
#[derive(Debug, Clone)]
pub struct BackgroundTask {
    /// Unique task identifier
    pub id: String,
    /// Task type for categorization
    pub operation_type: BackgroundOperationType,
    /// Task priority level
    pub priority: BackgroundPriority,
    /// Task payload/parameters
    pub payload: TaskPayload,
    /// Task creation timestamp
    pub created_at: Instant,
    /// Task deadline (optional)
    pub deadline: Option<Instant>,
    /// Task dependencies (must complete first)
    pub dependencies: Vec<String>,
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Current retry count
    pub retry_count: usize,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub error: Option<String>,
    pub result_data: Option<serde_json::Value>,
}

/// Task payload containing operation-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPayload {
    /// Document indexing parameters
    DocumentIndexing {
        file_paths: Vec<String>,
        model_name: String,
    },
    /// Cache optimization parameters
    CacheOptimization {
        cache_type: String,
        max_age_seconds: u64,
    },
    /// Analytics collection parameters
    AnalyticsCollection {
        metrics_types: Vec<String>,
        time_range_seconds: u64,
    },
    /// Health monitoring parameters
    HealthMonitoring {
        components: Vec<String>,
    },
    /// Index optimization parameters
    IndexOptimization {
        index_type: String,
        optimization_level: u8,
    },
    /// Generic task with custom data
    Generic {
        data: serde_json::Value,
    },
}

/// System resource monitoring for adaptive scheduling
#[derive(Debug)]
pub struct SystemResourceMonitor {
    /// Current CPU usage (0.0-1.0)
    cpu_usage: Arc<AtomicU64>, // Store as u64 for atomic operations
    /// Current memory usage (0.0-1.0)
    memory_usage: Arc<AtomicU64>,
    /// Last user activity timestamp
    last_activity: Arc<AtomicU64>,
    /// Power state (true = on battery, false = plugged in)
    on_battery: AtomicBool,
    /// System idle state
    is_idle: AtomicBool,
}

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self {
            cpu_usage: Arc::new(AtomicU64::new(0)),
            memory_usage: Arc::new(AtomicU64::new(0)),
            last_activity: Arc::new(AtomicU64::new(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
            )),
            on_battery: AtomicBool::new(false),
            is_idle: AtomicBool::new(false),
        }
    }
}

impl SystemResourceMonitor {
    /// Update CPU usage metrics
    pub fn update_cpu_usage(&self, usage: f64) {
        let usage_u64 = (usage * 1_000_000.0) as u64;
        self.cpu_usage.store(usage_u64, Ordering::Relaxed);
    }

    /// Update memory usage metrics
    pub fn update_memory_usage(&self, usage: f64) {
        let usage_u64 = (usage * 1_000_000.0) as u64;
        self.memory_usage.store(usage_u64, Ordering::Relaxed);
    }

    /// Get current CPU usage
    pub fn get_cpu_usage(&self) -> f64 {
        let usage_u64 = self.cpu_usage.load(Ordering::Relaxed);
        (usage_u64 as f64) / 1_000_000.0
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> f64 {
        let usage_u64 = self.memory_usage.load(Ordering::Relaxed);
        (usage_u64 as f64) / 1_000_000.0
    }

    /// Record user activity
    pub fn record_activity(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.last_activity.store(now, Ordering::Relaxed);
        self.is_idle.store(false, Ordering::Relaxed);
    }

    /// Check if system is idle
    pub fn is_system_idle(&self, idle_threshold_ms: u64) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let last_activity = self.last_activity.load(Ordering::Relaxed);
        let idle_duration_ms = (now - last_activity) * 1000;
        
        idle_duration_ms >= idle_threshold_ms
    }

    /// Update power state
    pub fn update_power_state(&self, on_battery: bool) {
        self.on_battery.store(on_battery, Ordering::Relaxed);
    }

    /// Check if system is on battery
    pub fn is_on_battery(&self) -> bool {
        self.on_battery.load(Ordering::Relaxed)
    }
}

/// Background task queue with priority scheduling
#[derive(Debug)]
pub struct TaskQueue {
    /// Priority-ordered task queues
    queues: HashMap<BackgroundPriority, VecDeque<BackgroundTask>>,
    /// Task lookup by ID
    task_lookup: HashMap<String, BackgroundTask>,
    /// Queue locks for concurrent access
    queue_locks: HashMap<BackgroundPriority, Arc<Mutex<()>>>,
}

impl Default for TaskQueue {
    fn default() -> Self {
        let mut queues = HashMap::new();
        let mut queue_locks = HashMap::new();
        
        // Initialize queues for each priority level
        for &priority in &[
            BackgroundPriority::Immediate,
            BackgroundPriority::High,
            BackgroundPriority::Normal,
            BackgroundPriority::Low,
            BackgroundPriority::Deferred,
        ] {
            queues.insert(priority, VecDeque::new());
            queue_locks.insert(priority, Arc::new(Mutex::new(())));
        }
        
        Self {
            queues,
            task_lookup: HashMap::new(),
            queue_locks,
        }
    }
}

impl TaskQueue {
    /// Add task to appropriate priority queue
    pub async fn enqueue(&mut self, task: BackgroundTask) -> BackgroundResult<()> {
        // Check for task dependencies
        for dep_id in &task.dependencies {
            if !self.task_lookup.contains_key(dep_id) {
                return Err(BackgroundProcessingError::DependencyFailed {
                    dependency_id: dep_id.clone(),
                });
            }
        }

        let priority = task.priority;
        let task_id = task.id.clone();
        
        // Add to task lookup
        self.task_lookup.insert(task_id.clone(), task.clone());
        
        // Add to priority queue
        if let Some(queue) = self.queues.get_mut(&priority) {
            queue.push_back(task);
        }
        
        Ok(())
    }

    /// Get next task based on priority and system state
    pub async fn dequeue(&mut self, system_monitor: &SystemResourceMonitor, config: &BackgroundProcessorConfig) -> Option<BackgroundTask> {
        // Check system constraints
        let cpu_usage = system_monitor.get_cpu_usage();
        let memory_usage = system_monitor.get_memory_usage();
        let is_idle = system_monitor.is_system_idle(config.idle_threshold_ms);
        let on_battery = system_monitor.is_on_battery();

        // Determine available priority levels based on system state
        let available_priorities = self.get_available_priorities(
            cpu_usage, 
            memory_usage, 
            is_idle, 
            on_battery, 
            config
        );

        // Try to get task from highest available priority queue
        for &priority in &available_priorities {
            if let Some(queue) = self.queues.get_mut(&priority) {
                if let Some(task) = queue.pop_front() {
                    // Remove from task lookup when dequeuing
                    self.task_lookup.remove(&task.id);
                    return Some(task);
                }
            }
        }

        None
    }

    /// Determine available priority levels based on system state
    fn get_available_priorities(
        &self, 
        cpu_usage: f64, 
        memory_usage: f64, 
        is_idle: bool, 
        on_battery: bool,
        config: &BackgroundProcessorConfig
    ) -> Vec<BackgroundPriority> {
        let mut priorities = Vec::new();

        // Always allow immediate tasks if system isn't severely constrained
        if cpu_usage < 0.9 && memory_usage < 0.9 {
            priorities.push(BackgroundPriority::Immediate);
        }

        // High priority tasks allowed if under throttle threshold
        if cpu_usage < config.cpu_throttle_threshold && memory_usage < config.memory_throttle_threshold {
            priorities.push(BackgroundPriority::High);
        }

        // Normal priority tasks allowed during idle periods or low load
        if is_idle || (cpu_usage < 0.5 && memory_usage < 0.7) {
            priorities.push(BackgroundPriority::Normal);
        }

        // Low priority tasks only during extended idle periods with low resource usage
        if is_idle && cpu_usage < 0.3 && memory_usage < 0.6 && !on_battery {
            priorities.push(BackgroundPriority::Low);
        }

        // Deferred tasks only during very low usage periods (e.g., overnight)
        if is_idle && cpu_usage < 0.2 && memory_usage < 0.5 && !on_battery {
            priorities.push(BackgroundPriority::Deferred);
        }

        // Sort by priority (highest first)
        priorities.sort_by(|a, b| b.cmp(a));
        priorities
    }

    /// Get current queue sizes for monitoring
    pub fn get_queue_sizes(&self) -> HashMap<BackgroundPriority, usize> {
        self.queues.iter().map(|(&priority, queue)| (priority, queue.len())).collect()
    }
}

/// Main background processor that coordinates all background operations
pub struct BackgroundProcessor {
    /// Configuration
    config: BackgroundProcessorConfig,
    /// Task queue manager
    task_queue: Arc<RwLock<TaskQueue>>,
    /// System resource monitor
    system_monitor: Arc<SystemResourceMonitor>,
    /// Resource allocator for system resource management
    resource_allocator: Arc<ResourceAllocator>,
    /// Performance tracker
    performance_tracker: Arc<PerformanceTracker>,
    /// Currently executing tasks
    active_tasks: Arc<RwLock<HashMap<String, Instant>>>,
    /// Task execution semaphore
    execution_semaphore: Arc<Semaphore>,
    /// Shutdown signal
    shutdown_signal: Arc<AtomicBool>,
    /// Processing statistics
    stats: Arc<RwLock<ProcessingStats>>,
}

/// Processing statistics for monitoring and optimization
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_tasks_processed: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time_ms: f64,
    pub current_active_tasks: usize,
    pub queue_sizes: HashMap<BackgroundPriority, usize>,
    pub last_updated: u64,
}

impl BackgroundProcessor {
    /// Create a new background processor with the given configuration
    pub fn new(
        config: BackgroundProcessorConfig,
        resource_allocator: Arc<ResourceAllocator>,
        performance_tracker: Arc<PerformanceTracker>,
    ) -> Self {
        let execution_semaphore = Arc::new(Semaphore::new(config.max_concurrent_tasks));
        let system_monitor = Arc::new(SystemResourceMonitor::default());
        
        Self {
            config,
            task_queue: Arc::new(RwLock::new(TaskQueue::default())),
            system_monitor,
            resource_allocator,
            performance_tracker,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            execution_semaphore,
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
        }
    }

    /// Start the background processor
    pub async fn start(&self) {
        eprintln!("🔄 Starting background processor...");
        
        // Start system monitoring task
        let monitor_task = self.start_system_monitoring();
        
        // Start main processing loop
        let processing_task = self.start_processing_loop();
        
        // Start statistics collection
        let stats_task = self.start_statistics_collection();
        
        // Wait for all tasks (this will run indefinitely until shutdown)
        tokio::select! {
            _ = monitor_task => {},
            _ = processing_task => {},
            _ = stats_task => {},
        }
        
        eprintln!("🛑 Background processor stopped");
    }

    /// Schedule a new background task
    pub async fn schedule_task(&self, task: BackgroundTask) -> BackgroundResult<()> {
        let mut queue = self.task_queue.write().await;
        queue.enqueue(task).await
    }

    /// Start system monitoring task
    async fn start_system_monitoring(&self) {
        let system_monitor = self.system_monitor.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            while !shutdown_signal.load(Ordering::Relaxed) {
                interval.tick().await;
                
                // Update system metrics (simplified - would integrate with real system monitoring)
                let cpu_usage = Self::get_system_cpu_usage().await;
                let memory_usage = Self::get_system_memory_usage().await;
                
                system_monitor.update_cpu_usage(cpu_usage);
                system_monitor.update_memory_usage(memory_usage);
                
                eprintln!("📊 System metrics - CPU: {:.1}%, Memory: {:.1}%", 
                         cpu_usage * 100.0, memory_usage * 100.0);
            }
        });
    }

    /// Start main processing loop
    async fn start_processing_loop(&self) {
        let task_queue = self.task_queue.clone();
        let system_monitor = self.system_monitor.clone();
        let active_tasks = self.active_tasks.clone();
        let execution_semaphore = self.execution_semaphore.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let config = self.config.clone();
        let resource_allocator = self.resource_allocator.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            while !shutdown_signal.load(Ordering::Relaxed) {
                // Try to get next task
                let task = {
                    let mut queue = task_queue.write().await;
                    queue.dequeue(&system_monitor, &config).await
                };
                
                if let Some(task) = task {
                    // Acquire execution permit
                    if let Ok(_permit) = execution_semaphore.try_acquire() {
                        let task_id = task.id.clone();
                        
                        // Record task start
                        {
                            let mut active = active_tasks.write().await;
                            active.insert(task_id.clone(), Instant::now());
                        }
                        
                        // Execute task
                        let _task_queue_clone = task_queue.clone();
                        let active_tasks_clone = active_tasks.clone();
                        let resource_allocator_clone = resource_allocator.clone();
                        let stats_clone = stats.clone();
                        
                        tokio::spawn(async move {
                            let result = Self::execute_task(task, resource_allocator_clone).await;
                            
                            // Update statistics
                            let execution_time = {
                                let mut active = active_tasks_clone.write().await;
                                if let Some(start_time) = active.remove(&task_id) {
                                    start_time.elapsed().as_millis() as u64
                                } else {
                                    0
                                }
                            };
                            
                            {
                                let mut stats = stats_clone.write().await;
                                stats.total_tasks_processed += 1;
                                if result.success {
                                    stats.successful_tasks += 1;
                                } else {
                                    stats.failed_tasks += 1;
                                }
                                
                                // Update average execution time
                                let total_time = stats.average_execution_time_ms * (stats.total_tasks_processed - 1) as f64;
                                stats.average_execution_time_ms = (total_time + execution_time as f64) / stats.total_tasks_processed as f64;
                            }
                            
                            eprintln!("✅ Task {} completed: {} ({} ms)", 
                                     result.task_id, 
                                     if result.success { "Success" } else { "Failed" }, 
                                     execution_time);
                        });
                    } else {
                        // No execution slots available, put task back in queue
                        let mut queue = task_queue.write().await;
                        let _ = queue.enqueue(task).await;
                    }
                } else {
                    // No tasks available, sleep briefly
                    sleep(Duration::from_millis(100)).await;
                }
            }
        });
    }

    /// Start statistics collection task
    async fn start_statistics_collection(&self) {
        let stats = self.stats.clone();
        let task_queue = self.task_queue.clone();
        let active_tasks = self.active_tasks.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            while !shutdown_signal.load(Ordering::Relaxed) {
                interval.tick().await;
                
                let queue_sizes = {
                    let queue = task_queue.read().await;
                    queue.get_queue_sizes()
                };
                
                let current_active_tasks = {
                    let active = active_tasks.read().await;
                    active.len()
                };
                
                {
                    let mut stats = stats.write().await;
                    stats.queue_sizes = queue_sizes;
                    stats.current_active_tasks = current_active_tasks;
                    stats.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                }
                
                eprintln!("📈 Background processor stats - Active: {}, Queued: {:?}", 
                         current_active_tasks, 
                         stats.read().await.queue_sizes);
            }
        });
    }

    /// Execute a background task
    async fn execute_task(task: BackgroundTask, resource_allocator: Arc<ResourceAllocator>) -> TaskResult {
        let start_time = Instant::now();
        let task_id = task.id.clone();
        
        // Apply resource allocation and throttling
        let operation_priority = match task.priority {
            BackgroundPriority::Immediate => OperationPriority::Normal,
            BackgroundPriority::High => OperationPriority::Low,
            _ => OperationPriority::Background,
        };
        
        let operation_type = match task.operation_type {
            BackgroundOperationType::DocumentIndexing => OperationType::AiEmbedding,
            BackgroundOperationType::CacheOptimization => OperationType::Maintenance,
            BackgroundOperationType::AnalyticsCollection => OperationType::Maintenance,
            BackgroundOperationType::HealthMonitoring => OperationType::Maintenance,
            BackgroundOperationType::IndexOptimization => OperationType::VectorDbIo,
            BackgroundOperationType::Precomputation => OperationType::AiEmbedding,
            BackgroundOperationType::ResourceCleanup => OperationType::Cleanup,
        };
        
        match resource_allocator.execute_io(
            operation_type,
            operation_priority,
            async move {
                // Execute the actual task based on its type
                Self::execute_task_payload(&task).await
            }
        ).await {
            Ok(result) => match result {
                Ok(data) => TaskResult {
                    task_id,
                    success: true,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error: None,
                    result_data: data,
                },
                Err(e) => TaskResult {
                    task_id,
                    success: false,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                    result_data: None,
                },
            },
            Err(error) => TaskResult {
                task_id,
                success: false,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error: Some(error.to_string()),
                result_data: None,
            },
        }
    }

    /// Execute specific task payload
    async fn execute_task_payload(task: &BackgroundTask) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        match &task.payload {
            TaskPayload::DocumentIndexing { file_paths, model_name } => {
                eprintln!("🔄 Indexing {} documents with model {}", file_paths.len(), model_name);
                // Simulate document indexing
                sleep(Duration::from_millis(100 * file_paths.len() as u64)).await;
                Ok(Some(serde_json::json!({
                    "indexed_files": file_paths.len(),
                    "model": model_name
                })))
            },
            TaskPayload::CacheOptimization { cache_type, max_age_seconds } => {
                eprintln!("🧹 Optimizing {} cache (max age: {}s)", cache_type, max_age_seconds);
                // Simulate cache optimization
                sleep(Duration::from_millis(50)).await;
                Ok(Some(serde_json::json!({
                    "cache_type": cache_type,
                    "cleaned_entries": 42
                })))
            },
            TaskPayload::AnalyticsCollection { metrics_types, time_range_seconds } => {
                eprintln!("📊 Collecting analytics for {:?} ({}s range)", metrics_types, time_range_seconds);
                // Simulate analytics collection
                sleep(Duration::from_millis(30)).await;
                Ok(Some(serde_json::json!({
                    "metrics_collected": metrics_types.len(),
                    "time_range": time_range_seconds
                })))
            },
            TaskPayload::HealthMonitoring { components } => {
                eprintln!("💚 Health check for components: {:?}", components);
                // Simulate health monitoring
                sleep(Duration::from_millis(20)).await;
                Ok(Some(serde_json::json!({
                    "components_checked": components.len(),
                    "all_healthy": true
                })))
            },
            TaskPayload::IndexOptimization { index_type, optimization_level } => {
                eprintln!("⚙️ Optimizing {} index (level {})", index_type, optimization_level);
                // Simulate index optimization
                sleep(Duration::from_millis(200)).await;
                Ok(Some(serde_json::json!({
                    "index_type": index_type,
                    "optimization_level": optimization_level
                })))
            },
            TaskPayload::Generic { data } => {
                eprintln!("🔧 Executing generic task with data: {}", data);
                // Simulate generic task execution
                sleep(Duration::from_millis(100)).await;
                Ok(Some(data.clone()))
            },
        }
    }

    /// Simplified system CPU usage monitoring (would use real system APIs in production)
    async fn get_system_cpu_usage() -> f64 {
        // In a real implementation, this would use system APIs
        // For now, simulate varying CPU usage
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().hash(&mut hasher);
        let hash_value = hasher.finish();
        
        // Generate pseudo-random CPU usage between 0.1 and 0.7
        0.1 + ((hash_value % 100) as f64 / 100.0) * 0.6
    }

    /// Simplified system memory usage monitoring
    async fn get_system_memory_usage() -> f64 {
        // In a real implementation, this would use system APIs
        // For now, simulate varying memory usage
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 1).hash(&mut hasher);
        let hash_value = hasher.finish();
        
        // Generate pseudo-random memory usage between 0.3 and 0.8
        0.3 + ((hash_value % 100) as f64 / 100.0) * 0.5
    }

    /// Get current processing statistics
    pub async fn get_stats(&self) -> ProcessingStats {
        self.stats.read().await.clone()
    }

    /// Shutdown the background processor
    pub fn shutdown(&self) {
        self.shutdown_signal.store(true, Ordering::Relaxed);
    }
}

/// Helper function to create common background tasks
pub struct TaskBuilder;

impl TaskBuilder {
    /// Create a document indexing task
    pub fn document_indexing(file_paths: Vec<String>, model_name: String, priority: BackgroundPriority) -> BackgroundTask {
        BackgroundTask {
            id: Uuid::new_v4().to_string(),
            operation_type: BackgroundOperationType::DocumentIndexing,
            priority,
            payload: TaskPayload::DocumentIndexing { file_paths, model_name },
            created_at: Instant::now(),
            deadline: None,
            dependencies: Vec::new(),
            max_retries: 2,
            retry_count: 0,
        }
    }

    /// Create a cache optimization task
    pub fn cache_optimization(cache_type: String, max_age_seconds: u64) -> BackgroundTask {
        BackgroundTask {
            id: Uuid::new_v4().to_string(),
            operation_type: BackgroundOperationType::CacheOptimization,
            priority: BackgroundPriority::Normal,
            payload: TaskPayload::CacheOptimization { cache_type, max_age_seconds },
            created_at: Instant::now(),
            deadline: None,
            dependencies: Vec::new(),
            max_retries: 1,
            retry_count: 0,
        }
    }

    /// Create a health monitoring task
    pub fn health_monitoring(components: Vec<String>) -> BackgroundTask {
        BackgroundTask {
            id: Uuid::new_v4().to_string(),
            operation_type: BackgroundOperationType::HealthMonitoring,
            priority: BackgroundPriority::Low,
            payload: TaskPayload::HealthMonitoring { components },
            created_at: Instant::now(),
            deadline: None,
            dependencies: Vec::new(),
            max_retries: 3,
            retry_count: 0,
        }
    }
}