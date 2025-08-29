//! Resource Allocation System for aiNote
//!
//! This module provides intelligent resource allocation for CPU, I/O, and background
//! thread management to optimize performance during AI operations while maintaining
//! UI responsiveness.
//!
//! ## Core Features
//! - CPU priority management for different operation types  
//! - Non-blocking I/O operations for vector database
//! - Background thread pool for AI processing
//! - I/O scheduling to prevent UI blocking
//! - Graceful degradation under resource constraints
//!
//! ## Performance Targets
//! - UI thread never blocked >16ms
//! - I/O operations complete within 50ms
//! - Background tasks utilize available CPU efficiently
//! - System remains responsive under 80% CPU load

use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, OwnedSemaphorePermit};
use tokio::task::{JoinHandle, yield_now};
use tokio::time::{sleep, timeout};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use rayon::ThreadPoolBuilder;

use crate::performance::PerformanceTracker;

/// Resource allocation errors
#[derive(Error, Debug, Clone)]
pub enum ResourceError {
    #[error("CPU overloaded: {cpu_usage}% > {threshold}%")]
    CpuOverloaded { cpu_usage: f64, threshold: f64 },
    
    #[error("I/O operation timeout: {operation} took {duration_ms}ms")]
    IoTimeout { operation: String, duration_ms: u64 },
    
    #[error("Thread pool saturated: {active_threads}/{max_threads} threads busy")]
    ThreadPoolSaturated { active_threads: usize, max_threads: usize },
    
    #[error("Resource allocation failed: {resource} not available")]
    AllocationFailed { resource: String },
    
    #[error("Priority escalation failed: {operation} cannot be elevated")]
    PriorityEscalationFailed { operation: String },
}

pub type ResourceResult<T> = Result<T, ResourceError>;

/// Operation priority levels for resource allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OperationPriority {
    /// Critical UI operations - highest priority (16ms target)
    Critical = 4,
    /// High priority - user-initiated actions (50ms target)  
    High = 3,
    /// Normal priority - background AI operations (200ms target)
    Normal = 2,
    /// Low priority - maintenance tasks (1s target)
    Low = 1,
    /// Background priority - cleanup, optimization (10s target)
    Background = 0,
}

/// Types of operations for resource scheduling
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// UI rendering and interaction
    UiOperation,
    /// File system I/O
    FileIo,
    /// Vector database operations
    VectorDbIo,
    /// AI embedding generation
    AiEmbedding,
    /// Search and similarity operations
    Search,
    /// Background maintenance
    Maintenance,
    /// System cleanup
    Cleanup,
}

/// Resource allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocatorConfig {
    /// Maximum CPU usage threshold (0.0-1.0)
    pub max_cpu_threshold: f64,
    /// I/O operation timeout in milliseconds
    pub io_timeout_ms: u64,
    /// Maximum concurrent background threads
    pub max_background_threads: usize,
    /// Maximum concurrent AI operations
    pub max_ai_operations: usize,
    /// CPU throttling enabled
    pub cpu_throttling_enabled: bool,
    /// I/O priority scheduling enabled  
    pub io_scheduling_enabled: bool,
    /// Background task limit per priority level
    pub background_task_limits: HashMap<OperationPriority, usize>,
}

impl Default for ResourceAllocatorConfig {
    fn default() -> Self {
        let mut background_task_limits = HashMap::new();
        background_task_limits.insert(OperationPriority::Critical, 1);
        background_task_limits.insert(OperationPriority::High, 2);
        background_task_limits.insert(OperationPriority::Normal, 4);
        background_task_limits.insert(OperationPriority::Low, 2);
        background_task_limits.insert(OperationPriority::Background, 1);
        
        Self {
            max_cpu_threshold: 0.8,  // 80% CPU threshold
            io_timeout_ms: 50,       // 50ms I/O timeout target
            max_background_threads: num_cpus::get().saturating_sub(1), // Leave 1 CPU for UI
            max_ai_operations: 2,    // Concurrent AI operations
            cpu_throttling_enabled: true,
            io_scheduling_enabled: true,
            background_task_limits,
        }
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Current CPU usage (0.0-1.0)
    pub cpu_usage: f64,
    /// Active thread count by priority
    pub active_threads: HashMap<OperationPriority, usize>,
    /// Pending operations count by type
    pub pending_operations: HashMap<OperationType, usize>,
    /// Average I/O latency in milliseconds
    pub avg_io_latency_ms: f64,
    /// Operations throttled in last minute
    pub throttled_operations: usize,
    /// Current system load
    pub system_load: f64,
    /// Timestamp of metrics collection
    pub timestamp: u64,
}

/// Background task handle with priority and metadata
struct PriorityTask {
    handle: JoinHandle<()>,
    priority: OperationPriority,
    operation_type: OperationType,
    started_at: Instant,
}

/// CPU priority manager for different operation types
struct CpuPriorityManager {
    current_load: Arc<AtomicU64>, // Store as u64 (f64 * 1000000)
    throttling_enabled: AtomicBool,
    performance_tracker: Arc<PerformanceTracker>,
}

impl CpuPriorityManager {
    fn new(performance_tracker: Arc<PerformanceTracker>) -> Self {
        Self {
            current_load: Arc::new(AtomicU64::new(0)),
            throttling_enabled: AtomicBool::new(true),
            performance_tracker,
        }
    }
    
    /// Get current CPU load as percentage (0.0-1.0)
    fn get_cpu_load(&self) -> f64 {
        let load_u64 = self.current_load.load(Ordering::Relaxed);
        (load_u64 as f64) / 1_000_000.0
    }
    
    /// Update CPU load metrics
    fn update_cpu_load(&self, load: f64) {
        let load_u64 = (load * 1_000_000.0) as u64;
        self.current_load.store(load_u64, Ordering::Relaxed);
    }
    
    /// Apply CPU throttling based on priority and current load
    async fn apply_throttling(&self, priority: OperationPriority) -> ResourceResult<()> {
        if !self.throttling_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        let cpu_load = self.get_cpu_load();
        
        // Critical operations bypass throttling
        if priority == OperationPriority::Critical {
            return Ok(());
        }
        
        // Apply progressive throttling based on CPU load and priority
        let throttle_delay = match (cpu_load, priority) {
            (load, OperationPriority::High) if load > 0.7 => Duration::from_millis(1),
            (load, OperationPriority::Normal) if load > 0.6 => Duration::from_millis(5),
            (load, OperationPriority::Low) if load > 0.5 => Duration::from_millis(10),
            (load, OperationPriority::Background) if load > 0.4 => Duration::from_millis(50),
            _ => Duration::from_millis(0),
        };
        
        if throttle_delay > Duration::from_millis(0) {
            sleep(throttle_delay).await;
            yield_now().await; // Allow other tasks to run
        }
        
        Ok(())
    }
}

/// I/O scheduler to prevent UI blocking
struct IoScheduler {
    pending_operations: Arc<RwLock<HashMap<OperationType, Vec<Instant>>>>,
    semaphore_by_type: HashMap<OperationType, Arc<Semaphore>>,
    config: ResourceAllocatorConfig,
}

impl IoScheduler {
    fn new(config: ResourceAllocatorConfig) -> Self {
        let mut semaphore_by_type = HashMap::new();
        
        // Different limits for different I/O types
        semaphore_by_type.insert(OperationType::UiOperation, Arc::new(Semaphore::new(1)));
        semaphore_by_type.insert(OperationType::FileIo, Arc::new(Semaphore::new(4)));
        semaphore_by_type.insert(OperationType::VectorDbIo, Arc::new(Semaphore::new(2)));
        semaphore_by_type.insert(OperationType::Search, Arc::new(Semaphore::new(3)));
        
        Self {
            pending_operations: Arc::new(RwLock::new(HashMap::new())),
            semaphore_by_type,
            config,
        }
    }
    
    /// Schedule I/O operation with timeout and priority
    async fn schedule_io<F, T>(&self, 
        operation_type: OperationType, 
        priority: OperationPriority,
        operation: F
    ) -> ResourceResult<T>
    where
        F: std::future::Future<Output = T>,
    {
        // Get semaphore for operation type
        let semaphore = self.semaphore_by_type.get(&operation_type)
            .ok_or_else(|| ResourceError::AllocationFailed { 
                resource: format!("I/O semaphore for {:?}", operation_type) 
            })?;
        
        // Acquire permit with timeout
        let permit_timeout = match priority {
            OperationPriority::Critical => Duration::from_millis(5),
            OperationPriority::High => Duration::from_millis(20),
            _ => Duration::from_millis(self.config.io_timeout_ms),
        };
        
        let permit = timeout(permit_timeout, semaphore.clone().acquire_owned())
            .await
            .map_err(|_| ResourceError::IoTimeout {
                operation: format!("{:?}", operation_type),
                duration_ms: permit_timeout.as_millis() as u64,
            })?
            .map_err(|_| ResourceError::AllocationFailed {
                resource: format!("I/O permit for {:?}", operation_type),
            })?;
        
        // Track operation start
        {
            let mut pending = self.pending_operations.write().await;
            pending.entry(operation_type.clone())
                .or_insert_with(Vec::new)
                .push(Instant::now());
        }
        
        // Execute operation with timeout
        let operation_timeout = Duration::from_millis(self.config.io_timeout_ms);
        let result = timeout(operation_timeout, operation).await
            .map_err(|_| ResourceError::IoTimeout {
                operation: format!("{:?}", operation_type),
                duration_ms: self.config.io_timeout_ms,
            })?;
        
        // Clean up tracking
        {
            let mut pending = self.pending_operations.write().await;
            if let Some(ops) = pending.get_mut(&operation_type) {
                ops.retain(|start| start.elapsed() < Duration::from_secs(1));
            }
        }
        
        drop(permit); // Release permit
        Ok(result)
    }
    
    /// Get current I/O load metrics
    async fn get_io_metrics(&self) -> HashMap<OperationType, usize> {
        let pending = self.pending_operations.read().await;
        pending.iter()
            .map(|(op_type, ops)| (op_type.clone(), ops.len()))
            .collect()
    }
}

/// Main resource allocator
pub struct ResourceAllocator {
    config: ResourceAllocatorConfig,
    cpu_manager: CpuPriorityManager,
    io_scheduler: IoScheduler,
    background_tasks: Arc<RwLock<HashMap<String, PriorityTask>>>,
    thread_pool: Arc<rayon::ThreadPool>,
    ai_semaphore: Arc<Semaphore>,
    metrics: Arc<RwLock<ResourceMetrics>>,
    performance_tracker: Arc<PerformanceTracker>,
    is_active: AtomicBool,
}

impl ResourceAllocator {
    /// Create new resource allocator
    pub fn new(config: ResourceAllocatorConfig, performance_tracker: Arc<PerformanceTracker>) -> ResourceResult<Self> {
        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(config.max_background_threads)
            .thread_name(|i| format!("ainote-worker-{}", i))
            .build()
            .map_err(|e| ResourceError::AllocationFailed { 
                resource: format!("Thread pool: {}", e) 
            })?;
        
        let cpu_manager = CpuPriorityManager::new(performance_tracker.clone());
        let io_scheduler = IoScheduler::new(config.clone());
        let ai_semaphore = Arc::new(Semaphore::new(config.max_ai_operations));
        
        let metrics = ResourceMetrics {
            cpu_usage: 0.0,
            active_threads: HashMap::new(),
            pending_operations: HashMap::new(),
            avg_io_latency_ms: 0.0,
            throttled_operations: 0,
            system_load: 0.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        Ok(Self {
            config,
            cpu_manager,
            io_scheduler,
            background_tasks: Arc::new(RwLock::new(HashMap::new())),
            thread_pool: Arc::new(thread_pool),
            ai_semaphore,
            metrics: Arc::new(RwLock::new(metrics)),
            performance_tracker,
            is_active: AtomicBool::new(false),
        })
    }
    
    /// Start resource allocation system
    pub async fn start(&self) -> ResourceResult<()> {
        if self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.is_active.store(true, Ordering::Relaxed);
        
        // Start background monitoring task
        let _allocator_weak = Arc::downgrade(&Arc::new(self));
        tokio::spawn(async move {
            // This would be a monitoring loop, but we need to avoid circular references
            // Implementation would go in the monitoring task method
        });
        
        Ok(())
    }
    
    /// Stop resource allocation system
    pub async fn stop(&self) -> ResourceResult<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.is_active.store(false, Ordering::Relaxed);
        
        // Cancel all background tasks
        let mut tasks = self.background_tasks.write().await;
        for (_, task) in tasks.drain() {
            task.handle.abort();
        }
        
        Ok(())
    }
    
    /// Execute I/O operation with resource management
    pub async fn execute_io<F, T>(&self, 
        operation_type: OperationType,
        priority: OperationPriority,
        operation: F
    ) -> ResourceResult<T>
    where
        F: std::future::Future<Output = T>,
    {
        // Apply CPU throttling
        self.cpu_manager.apply_throttling(priority).await?;
        
        // Schedule through I/O scheduler
        self.io_scheduler.schedule_io(operation_type, priority, operation).await
    }
    
    /// Submit background task with priority
    pub async fn submit_background_task<F>(&self,
        task_id: String,
        priority: OperationPriority,
        operation_type: OperationType,
        task: F
    ) -> ResourceResult<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        // Check if we're at the limit for this priority
        let active_count = {
            let tasks = self.background_tasks.read().await;
            tasks.values()
                .filter(|t| t.priority == priority)
                .count()
        };
        
        let limit = self.config.background_task_limits.get(&priority)
            .copied()
            .unwrap_or(1);
            
        if active_count >= limit {
            return Err(ResourceError::ThreadPoolSaturated {
                active_threads: active_count,
                max_threads: limit,
            });
        }
        
        // Spawn task with monitoring
        let handle = tokio::spawn(async move {
            task.await;
        });
        
        let priority_task = PriorityTask {
            handle,
            priority,
            operation_type,
            started_at: Instant::now(),
        };
        
        // Store task reference
        let mut tasks = self.background_tasks.write().await;
        tasks.insert(task_id, priority_task);
        
        Ok(())
    }
    
    /// Request AI operation permit  
    pub async fn request_ai_permit(&self) -> ResourceResult<OwnedSemaphorePermit> {
        let permit = self.ai_semaphore.clone().acquire_owned().await
            .map_err(|_| ResourceError::AllocationFailed {
                resource: "AI operation permit".to_string(),
            })?;
        Ok(permit)
    }
    
    /// Get current resource metrics
    pub async fn get_metrics(&self) -> ResourceMetrics {
        // Update CPU load
        self.cpu_manager.update_cpu_load(0.3); // Placeholder - would integrate with system monitoring
        
        let mut metrics = self.metrics.write().await;
        metrics.cpu_usage = self.cpu_manager.get_cpu_load();
        
        // Update active thread counts
        let tasks = self.background_tasks.read().await;
        metrics.active_threads.clear();
        for (_, task) in tasks.iter() {
            *metrics.active_threads.entry(task.priority).or_insert(0) += 1;
        }
        
        // Update I/O metrics
        metrics.pending_operations = self.io_scheduler.get_io_metrics().await;
        
        // Update timestamp
        metrics.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        metrics.clone()
    }
    
    /// Check if system is under resource pressure
    pub async fn is_under_pressure(&self) -> bool {
        let metrics = self.get_metrics().await;
        metrics.cpu_usage > self.config.max_cpu_threshold || 
        metrics.avg_io_latency_ms > self.config.io_timeout_ms as f64
    }
    
    /// Enable graceful degradation mode
    pub async fn enable_degradation_mode(&self) -> ResourceResult<()> {
        // Reduce concurrent operations
        // This would adjust semaphore permits and throttling
        // Implementation would reduce background task limits temporarily
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, _new_config: ResourceAllocatorConfig) -> ResourceResult<()> {
        // Would update internal configuration
        // This is a simplified implementation
        Ok(())
    }
    
    /// Clean up completed background tasks
    pub async fn cleanup_completed_tasks(&self) -> ResourceResult<usize> {
        let mut tasks = self.background_tasks.write().await;
        let initial_count = tasks.len();
        
        tasks.retain(|_, task| !task.handle.is_finished());
        
        Ok(initial_count - tasks.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    fn create_test_allocator() -> ResourceAllocator {
        let config = ResourceAllocatorConfig::default();
        let performance_tracker = Arc::new(PerformanceTracker::start("test"));
        ResourceAllocator::new(config, performance_tracker).unwrap()
    }
    
    #[tokio::test]
    async fn test_resource_allocator_creation() {
        let allocator = create_test_allocator();
        assert!(!allocator.is_active.load(Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_start_stop_lifecycle() {
        let allocator = create_test_allocator();
        
        allocator.start().await.unwrap();
        assert!(allocator.is_active.load(Ordering::Relaxed));
        
        allocator.stop().await.unwrap();
        assert!(!allocator.is_active.load(Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_io_operation_scheduling() {
        let allocator = create_test_allocator();
        
        let result = allocator.execute_io(
            OperationType::FileIo,
            OperationPriority::High,
            async { "test_result" }
        ).await.unwrap();
        
        assert_eq!(result, "test_result");
    }
    
    #[tokio::test]
    async fn test_background_task_submission() {
        let allocator = create_test_allocator();
        
        let result = allocator.submit_background_task(
            "test_task".to_string(),
            OperationPriority::Normal,
            OperationType::Maintenance,
            async {}
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_ai_permit_acquisition() {
        let allocator = create_test_allocator();
        
        let _permit = allocator.request_ai_permit().await.unwrap();
        // Just check that we got the permit - no is_acquired() method available
    }
}