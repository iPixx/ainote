//! Advanced Maintenance and Cleanup Operations Module
//!
//! This module provides comprehensive index maintenance including orphaned embedding
//! detection, automatic cleanup scheduling, index compaction, and storage optimization.
//! 
//! ## Features
//!
//! - **Orphaned Detection**: Identify embeddings for non-existent files  
//! - **Scheduled Maintenance**: Background maintenance with configurable schedules
//! - **Index Compaction**: Optimize index structure for better performance
//! - **Storage Optimization**: Reclaim storage space and defragment indexes
//! - **Performance Monitoring**: Track maintenance operation performance
//! - **Automatic Scheduling**: Self-managing maintenance cycles
//!
//! ## Architecture
//!
//! The maintenance system consists of several key components:
//!
//! - `MaintenanceManager`: Central coordinator for all maintenance operations
//! - `MaintenanceScheduler`: Background task scheduling and execution
//! - `OrphanDetector`: Advanced orphaned embedding detection algorithms  
//! - `IndexOptimizer`: Index compaction and optimization utilities
//! - `StorageReclaimer`: Storage space reclamation and defragmentation
//! - `MaintenanceMetrics`: Performance monitoring and reporting

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, timeout};
use serde::{Serialize, Deserialize};

use crate::vector_db::types::{VectorDbError, VectorDbResult};
use crate::vector_db::storage::{VectorStorage, CompactionResult};
use crate::vector_db::operations::{VectorOperations, BatchOperations};

/// Configuration for maintenance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    /// Enable automatic scheduled maintenance
    pub enable_automatic_maintenance: bool,
    /// Maintenance check interval in seconds  
    pub maintenance_interval_seconds: u64,
    /// Maximum orphan cleanup operations per cycle
    pub max_orphan_cleanup_per_cycle: usize,
    /// Enable index compaction during maintenance
    pub enable_index_compaction: bool,
    /// Minimum time between compaction operations (hours)
    pub compaction_cooldown_hours: u64,
    /// Enable storage defragmentation
    pub enable_defragmentation: bool,
    /// Storage utilization threshold to trigger compaction (0.0-1.0)
    pub compaction_threshold: f64,
    /// Maximum maintenance operation duration (seconds)
    pub max_operation_duration_seconds: u64,
    /// Enable detailed logging during maintenance
    pub enable_debug_logging: bool,
    /// Paths to monitor for file existence validation
    pub monitored_vault_paths: Vec<PathBuf>,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            enable_automatic_maintenance: true,
            maintenance_interval_seconds: 300, // 5 minutes
            max_orphan_cleanup_per_cycle: 100,
            enable_index_compaction: true,
            compaction_cooldown_hours: 24, // Once per day
            enable_defragmentation: true,
            compaction_threshold: 0.3, // 30% fragmentation
            max_operation_duration_seconds: 30, // 30 second timeout
            enable_debug_logging: false,
            monitored_vault_paths: Vec::new(),
        }
    }
}

/// Statistics and metrics for maintenance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceStats {
    /// Total number of maintenance cycles executed
    pub maintenance_cycles: u64,
    /// Total orphaned embeddings detected and removed
    pub orphaned_embeddings_removed: u64,
    /// Total compaction operations performed  
    pub compaction_operations: u64,
    /// Total storage space reclaimed (bytes)
    pub storage_space_reclaimed: u64,
    /// Total defragmentation operations performed
    pub defragmentation_operations: u64,
    /// Average time per maintenance cycle (milliseconds)
    pub avg_cycle_time_ms: f64,
    /// Average time per orphan cleanup (milliseconds)
    pub avg_orphan_cleanup_time_ms: f64,
    /// Last maintenance timestamp
    pub last_maintenance_at: u64,
    /// Last compaction timestamp
    pub last_compaction_at: u64,
    /// Performance metrics for recent operations
    pub recent_cycle_times: Vec<u64>,
}

impl Default for MaintenanceStats {
    fn default() -> Self {
        Self {
            maintenance_cycles: 0,
            orphaned_embeddings_removed: 0,
            compaction_operations: 0,
            storage_space_reclaimed: 0,
            defragmentation_operations: 0,
            avg_cycle_time_ms: 0.0,
            avg_orphan_cleanup_time_ms: 0.0,
            last_maintenance_at: 0,
            last_compaction_at: 0,
            recent_cycle_times: Vec::new(),
        }
    }
}

impl MaintenanceStats {
    /// Update average cycle time with new measurement
    pub fn update_cycle_time(&mut self, cycle_time_ms: u64) {
        self.recent_cycle_times.push(cycle_time_ms);
        
        // Keep only recent 100 measurements
        if self.recent_cycle_times.len() > 100 {
            self.recent_cycle_times.remove(0);
        }
        
        // Calculate new average
        if !self.recent_cycle_times.is_empty() {
            self.avg_cycle_time_ms = self.recent_cycle_times.iter().sum::<u64>() as f64 
                / self.recent_cycle_times.len() as f64;
        }
    }
    
    /// Check if performance targets are being met
    pub fn meets_performance_targets(&self) -> bool {
        // Target: orphan cleanup <5 seconds per 1000 entries
        self.avg_orphan_cleanup_time_ms <= 5000.0
    }
}

/// Result of orphaned embedding detection
#[derive(Debug, Clone)]
pub struct OrphanDetectionResult {
    /// IDs of orphaned embeddings found
    pub orphaned_entry_ids: Vec<String>,
    /// File paths that no longer exist
    pub missing_file_paths: HashSet<PathBuf>,
    /// Total embeddings checked
    pub total_embeddings_checked: usize,
    /// Time taken for detection (milliseconds)
    pub detection_time_ms: u64,
    /// Validation method used for detection
    pub validation_method: OrphanValidationMethod,
}

/// Methods for validating file existence during orphan detection
#[derive(Debug, Clone)]
pub enum OrphanValidationMethod {
    /// File system validation (check if files exist on disk)
    FileSystem,
    /// Vault path validation (check against monitored vault paths)
    VaultPath { monitored_paths: Vec<PathBuf> },
    /// Combined validation (both file system and vault paths)
    Combined { monitored_paths: Vec<PathBuf> },
}

/// Advanced orphaned embedding detector
pub struct OrphanDetector {
    /// Storage backend
    storage: Arc<VectorStorage>,
    /// Vector operations interface
    operations: VectorOperations,
    /// Configuration
    config: MaintenanceConfig,
}

impl OrphanDetector {
    /// Create a new orphan detector
    pub fn new(
        storage: Arc<VectorStorage>,
        operations: VectorOperations,
        config: MaintenanceConfig,
    ) -> Self {
        Self {
            storage,
            operations,
            config,
        }
    }
    
    /// Detect orphaned embeddings using file system validation
    pub async fn detect_orphaned_embeddings(&self) -> VectorDbResult<OrphanDetectionResult> {
        let start_time = Instant::now();
        
        if self.config.enable_debug_logging {
            eprintln!("🔍 Starting orphaned embedding detection...");
        }
        
        let all_ids = self.storage.list_entry_ids().await;
        let total_embeddings = all_ids.len();
        
        // Use timeout to prevent hanging
        let timeout_duration = Duration::from_secs(self.config.max_operation_duration_seconds);
        
        let detection_result = timeout(timeout_duration, async {
            let mut orphaned_ids = Vec::new();
            let mut missing_paths = HashSet::new();
            
            // Process embeddings in batches for better performance
            let batch_size = 50; // Process 50 embeddings at a time
            
            for chunk in all_ids.chunks(batch_size) {
                let entries = self.storage.retrieve_entries(chunk).await?;
                
                for entry in entries {
                    let file_path = Path::new(&entry.metadata.file_path);
                    let is_orphaned = self.validate_file_existence(file_path).await;
                    
                    if is_orphaned {
                        orphaned_ids.push(entry.id);
                        missing_paths.insert(file_path.to_path_buf());
                    }
                }
            }
            
            VectorDbResult::Ok((orphaned_ids, missing_paths))
        }).await;
        
        let (orphaned_entry_ids, missing_file_paths) = match detection_result {
            Ok(result) => result?,
            Err(_) => {
                return Err(VectorDbError::Storage {
                    message: format!("Orphan detection timed out after {} seconds", 
                                   self.config.max_operation_duration_seconds),
                });
            }
        };
        
        let detection_time_ms = start_time.elapsed().as_millis() as u64;
        
        let validation_method = if !self.config.monitored_vault_paths.is_empty() {
            OrphanValidationMethod::Combined { 
                monitored_paths: self.config.monitored_vault_paths.clone() 
            }
        } else {
            OrphanValidationMethod::FileSystem
        };
        
        if self.config.enable_debug_logging {
            eprintln!("🔍 Orphan detection completed: {} orphaned out of {} total ({}ms)", 
                     orphaned_entry_ids.len(), total_embeddings, detection_time_ms);
        }
        
        Ok(OrphanDetectionResult {
            orphaned_entry_ids,
            missing_file_paths,
            total_embeddings_checked: total_embeddings,
            detection_time_ms,
            validation_method,
        })
    }
    
    /// Validate file existence using configured method
    async fn validate_file_existence(&self, file_path: &Path) -> bool {
        // Check file system first
        if !file_path.exists() {
            return true; // File is missing, so embedding is orphaned
        }
        
        // If vault paths are configured, also check if file is within monitored vaults
        if !self.config.monitored_vault_paths.is_empty() {
            let mut within_vault = false;
            for vault_path in &self.config.monitored_vault_paths {
                if file_path.starts_with(vault_path) {
                    within_vault = true;
                    break;
                }
            }
            
            if !within_vault {
                return true; // File exists but is outside monitored vaults
            }
        }
        
        false // File exists and is valid
    }
    
    /// Remove detected orphaned embeddings
    pub async fn cleanup_orphaned_embeddings(
        &self,
        orphaned_ids: &[String],
    ) -> VectorDbResult<usize> {
        if orphaned_ids.is_empty() {
            return Ok(0);
        }
        
        let cleanup_start = Instant::now();
        let max_to_cleanup = self.config.max_orphan_cleanup_per_cycle;
        let ids_to_cleanup = if orphaned_ids.len() > max_to_cleanup {
            &orphaned_ids[..max_to_cleanup]
        } else {
            orphaned_ids
        };
        
        if self.config.enable_debug_logging {
            eprintln!("🧹 Cleaning up {} orphaned embeddings...", ids_to_cleanup.len());
        }
        
        // Use batch deletion for efficiency
        let batch_ops = BatchOperations::new(self.operations.clone());
        let deleted_count = batch_ops.delete_embeddings_batch(ids_to_cleanup).await?;
        
        let cleanup_time_ms = cleanup_start.elapsed().as_millis() as u64;
        
        if self.config.enable_debug_logging {
            eprintln!("🧹 Orphan cleanup completed: {} embeddings removed ({}ms)", 
                     deleted_count, cleanup_time_ms);
        }
        
        Ok(deleted_count)
    }
}

/// Index optimization and compaction utilities
pub struct IndexOptimizer {
    /// Storage backend
    storage: Arc<VectorStorage>,
    /// Configuration
    config: MaintenanceConfig,
    /// Last compaction timestamp
    last_compaction: Arc<RwLock<Option<SystemTime>>>,
}

impl IndexOptimizer {
    /// Create a new index optimizer
    pub fn new(storage: Arc<VectorStorage>, config: MaintenanceConfig) -> Self {
        Self {
            storage,
            config,
            last_compaction: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Check if compaction should be performed
    pub async fn should_compact(&self) -> bool {
        if !self.config.enable_index_compaction {
            return false;
        }
        
        let last_compaction = self.last_compaction.read().await;
        
        // Check cooldown period
        if let Some(last_time) = *last_compaction {
            let cooldown_duration = Duration::from_secs(self.config.compaction_cooldown_hours * 3600);
            if last_time.elapsed().unwrap_or_default() < cooldown_duration {
                return false;
            }
        }
        
        // Check storage utilization threshold
        let metrics = self.storage.get_metrics().await;
        let fragmentation_ratio = if metrics.uncompressed_size_bytes > 0 {
            1.0 - (metrics.total_size_bytes as f64 / metrics.uncompressed_size_bytes as f64)
        } else {
            0.0
        };
        
        fragmentation_ratio > self.config.compaction_threshold
    }
    
    /// Perform index compaction
    pub async fn compact_index(&self) -> VectorDbResult<CompactionResult> {
        if self.config.enable_debug_logging {
            eprintln!("🗜️ Starting index compaction...");
        }
        
        let compaction_start = Instant::now();
        
        // Use timeout to prevent hanging
        let timeout_duration = Duration::from_secs(self.config.max_operation_duration_seconds);
        
        let compaction_result = timeout(timeout_duration, async {
            self.storage.compact_storage().await
        }).await;
        
        let result = match compaction_result {
            Ok(result) => result?,
            Err(_) => {
                return Err(VectorDbError::Storage {
                    message: format!("Index compaction timed out after {} seconds", 
                                   self.config.max_operation_duration_seconds),
                });
            }
        };
        
        // Update last compaction timestamp
        {
            let mut last_compaction = self.last_compaction.write().await;
            *last_compaction = Some(SystemTime::now());
        }
        
        let compaction_time_ms = compaction_start.elapsed().as_millis() as u64;
        
        if self.config.enable_debug_logging {
            eprintln!("🗜️ Index compaction completed: {} files removed, {} files compacted ({}ms)", 
                     result.files_removed, result.files_compacted, compaction_time_ms);
        }
        
        Ok(result)
    }
    
    /// Defragment the index for better performance
    pub async fn defragment_index(&self) -> VectorDbResult<u64> {
        if !self.config.enable_defragmentation {
            return Ok(0);
        }
        
        if self.config.enable_debug_logging {
            eprintln!("🔧 Starting index defragmentation...");
        }
        
        let defrag_start = Instant::now();
        
        // For now, defragmentation consists of compaction followed by index rebuild
        let _compaction_result = self.compact_index().await?;
        self.storage.rebuild_index_async().await?;
        
        let defrag_time_ms = defrag_start.elapsed().as_millis() as u64;
        
        if self.config.enable_debug_logging {
            eprintln!("🔧 Index defragmentation completed ({}ms)", defrag_time_ms);
        }
        
        Ok(defrag_time_ms)
    }
}

/// Storage space reclamation utilities  
pub struct StorageReclaimer {
    /// Storage backend
    storage: Arc<VectorStorage>,
    /// Configuration
    config: MaintenanceConfig,
}

impl StorageReclaimer {
    /// Create a new storage reclaimer
    pub fn new(storage: Arc<VectorStorage>, config: MaintenanceConfig) -> Self {
        Self { storage, config }
    }
    
    /// Reclaim storage space from deleted embeddings
    pub async fn reclaim_storage(&self) -> VectorDbResult<u64> {
        if self.config.enable_debug_logging {
            eprintln!("💾 Starting storage space reclamation...");
        }
        
        let reclaim_start = Instant::now();
        
        // Get metrics before reclamation
        let metrics_before = self.storage.get_metrics().await;
        let size_before = metrics_before.total_size_bytes;
        
        // Perform compaction to reclaim space
        let _compaction_result = self.storage.compact_storage().await?;
        
        // Get metrics after reclamation
        let metrics_after = self.storage.get_metrics().await;
        let size_after = metrics_after.total_size_bytes;
        
        let space_reclaimed = size_before.saturating_sub(size_after);
        let reclaim_time_ms = reclaim_start.elapsed().as_millis() as u64;
        
        if self.config.enable_debug_logging {
            eprintln!("💾 Storage reclamation completed: {} bytes reclaimed ({}ms)", 
                     space_reclaimed, reclaim_time_ms);
        }
        
        Ok(space_reclaimed as u64)
    }
    
    /// Optimize storage layout for better performance
    pub async fn optimize_storage_layout(&self) -> VectorDbResult<()> {
        if self.config.enable_debug_logging {
            eprintln!("⚡ Starting storage layout optimization...");
        }
        
        let optimize_start = Instant::now();
        
        // Rebuild storage index for optimal layout
        self.storage.rebuild_index_async().await?;
        
        let optimize_time_ms = optimize_start.elapsed().as_millis() as u64;
        
        if self.config.enable_debug_logging {
            eprintln!("⚡ Storage optimization completed ({}ms)", optimize_time_ms);
        }
        
        Ok(())
    }
}

/// Background maintenance scheduler and coordinator
pub struct MaintenanceScheduler {
    /// Orphan detector
    orphan_detector: OrphanDetector,
    /// Index optimizer
    index_optimizer: IndexOptimizer,
    /// Storage reclaimer
    storage_reclaimer: StorageReclaimer,
    /// Configuration
    config: MaintenanceConfig,
    /// Maintenance statistics
    stats: Arc<RwLock<MaintenanceStats>>,
    /// Running state
    is_running: Arc<Mutex<bool>>,
}

impl MaintenanceScheduler {
    /// Create a new maintenance scheduler
    pub fn new(
        storage: Arc<VectorStorage>,
        operations: VectorOperations,
        config: MaintenanceConfig,
    ) -> Self {
        let orphan_detector = OrphanDetector::new(storage.clone(), operations, config.clone());
        let index_optimizer = IndexOptimizer::new(storage.clone(), config.clone());
        let storage_reclaimer = StorageReclaimer::new(storage.clone(), config.clone());
        
        Self {
            orphan_detector,
            index_optimizer,
            storage_reclaimer,
            config,
            stats: Arc::new(RwLock::new(MaintenanceStats::default())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Start the automatic maintenance scheduler
    pub async fn start_automatic_maintenance(&self) -> VectorDbResult<()> {
        if !self.config.enable_automatic_maintenance {
            return Ok(());
        }
        
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return Ok(()); // Already running
        }
        *is_running = true;
        
        eprintln!("🚀 Starting automatic maintenance scheduler (interval: {}s)", 
                 self.config.maintenance_interval_seconds);
        
        let interval_duration = Duration::from_secs(self.config.maintenance_interval_seconds);
        let mut interval_timer = interval(interval_duration);
        
        // Clone necessary data for the background task
        let orphan_detector = OrphanDetector::new(
            self.orphan_detector.storage.clone(),
            self.orphan_detector.operations.clone(),
            self.config.clone(),
        );
        let index_optimizer = IndexOptimizer::new(
            self.index_optimizer.storage.clone(),
            self.config.clone(),
        );
        let storage_reclaimer = StorageReclaimer::new(
            self.storage_reclaimer.storage.clone(),
            self.config.clone(),
        );
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        let config = self.config.clone();
        
        // Spawn background maintenance task
        tokio::spawn(async move {
            while *is_running.lock().await {
                interval_timer.tick().await;
                
                if config.enable_debug_logging {
                    eprintln!("🔄 Starting maintenance cycle...");
                }
                
                let cycle_start = Instant::now();
                
                // Perform maintenance operations
                let orphan_detection_result = orphan_detector.detect_orphaned_embeddings().await;
                if let Ok(detection_result) = orphan_detection_result {
                    if !detection_result.orphaned_entry_ids.is_empty() {
                        let _cleanup_result = orphan_detector
                            .cleanup_orphaned_embeddings(&detection_result.orphaned_entry_ids)
                            .await;
                    }
                }
                
                // Check if compaction is needed
                if index_optimizer.should_compact().await {
                    let _compaction_result = index_optimizer.compact_index().await;
                }
                
                // Periodically reclaim storage
                if stats.read().await.maintenance_cycles % 10 == 0 {
                    let _reclaim_result = storage_reclaimer.reclaim_storage().await;
                }
                
                let cycle_time_ms = cycle_start.elapsed().as_millis() as u64;
                
                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.maintenance_cycles += 1;
                    stats_guard.last_maintenance_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    stats_guard.update_cycle_time(cycle_time_ms);
                }
                
                if config.enable_debug_logging {
                    eprintln!("✅ Maintenance cycle completed ({}ms)", cycle_time_ms);
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the automatic maintenance scheduler
    pub async fn stop_automatic_maintenance(&self) {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
        
        eprintln!("⏹️ Stopped automatic maintenance scheduler");
    }
    
    /// Perform a single manual maintenance cycle
    pub async fn run_maintenance_cycle(&self) -> VectorDbResult<MaintenanceStats> {
        let cycle_start = Instant::now();
        
        eprintln!("🔄 Starting manual maintenance cycle...");
        
        let mut cycle_stats = MaintenanceStats::default();
        
        // 1. Orphaned embedding detection and cleanup
        let orphan_cleanup_start = Instant::now();
        let detection_result = self.orphan_detector.detect_orphaned_embeddings().await?;
        let orphaned_count = detection_result.orphaned_entry_ids.len();
        
        if orphaned_count > 0 {
            let cleaned_up = self.orphan_detector
                .cleanup_orphaned_embeddings(&detection_result.orphaned_entry_ids)
                .await?;
            cycle_stats.orphaned_embeddings_removed = cleaned_up as u64;
        }
        
        cycle_stats.avg_orphan_cleanup_time_ms = orphan_cleanup_start.elapsed().as_millis() as f64;
        
        // 2. Index compaction (if needed)
        if self.index_optimizer.should_compact().await {
            let _compaction_result = self.index_optimizer.compact_index().await?;
            cycle_stats.compaction_operations = 1;
        }
        
        // 3. Storage reclamation
        let space_reclaimed = self.storage_reclaimer.reclaim_storage().await?;
        cycle_stats.storage_space_reclaimed = space_reclaimed;
        
        // 4. Defragmentation (if enabled)
        if self.config.enable_defragmentation {
            let _defrag_time = self.index_optimizer.defragment_index().await?;
            cycle_stats.defragmentation_operations = 1;
        }
        
        let cycle_time_ms = cycle_start.elapsed().as_millis() as u64;
        cycle_stats.avg_cycle_time_ms = cycle_time_ms as f64;
        cycle_stats.maintenance_cycles = 1;
        cycle_stats.last_maintenance_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Update internal statistics
        {
            let mut stats = self.stats.write().await;
            stats.maintenance_cycles += 1;
            stats.orphaned_embeddings_removed += cycle_stats.orphaned_embeddings_removed;
            stats.compaction_operations += cycle_stats.compaction_operations;
            stats.storage_space_reclaimed += cycle_stats.storage_space_reclaimed;
            stats.defragmentation_operations += cycle_stats.defragmentation_operations;
            stats.last_maintenance_at = cycle_stats.last_maintenance_at;
            stats.update_cycle_time(cycle_time_ms);
        }
        
        eprintln!("✅ Manual maintenance cycle completed: {} orphans removed, {} bytes reclaimed ({}ms)", 
                 cycle_stats.orphaned_embeddings_removed,
                 cycle_stats.storage_space_reclaimed,
                 cycle_time_ms);
        
        Ok(cycle_stats)
    }
    
    /// Get current maintenance statistics
    pub async fn get_stats(&self) -> MaintenanceStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Reset maintenance statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = MaintenanceStats::default();
        eprintln!("📊 Maintenance statistics reset");
    }
    
    /// Check if automatic maintenance is currently running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.lock().await;
        *is_running
    }
}

/// Main maintenance manager that coordinates all maintenance operations
pub struct MaintenanceManager {
    /// Maintenance scheduler
    scheduler: MaintenanceScheduler,
    /// Configuration  
    config: MaintenanceConfig,
}

impl MaintenanceManager {
    /// Create a new maintenance manager
    pub async fn new(
        storage: Arc<VectorStorage>,
        operations: VectorOperations,
        config: MaintenanceConfig,
    ) -> VectorDbResult<Self> {
        let scheduler = MaintenanceScheduler::new(storage, operations, config.clone());
        
        Ok(Self {
            scheduler,
            config,
        })
    }
    
    /// Start automatic maintenance operations
    pub async fn start_maintenance(&self) -> VectorDbResult<()> {
        self.scheduler.start_automatic_maintenance().await
    }
    
    /// Stop automatic maintenance operations  
    pub async fn stop_maintenance(&self) {
        self.scheduler.stop_automatic_maintenance().await
    }
    
    /// Run a manual maintenance cycle
    pub async fn run_maintenance_cycle(&self) -> VectorDbResult<MaintenanceStats> {
        self.scheduler.run_maintenance_cycle().await
    }
    
    /// Get maintenance statistics
    pub async fn get_maintenance_stats(&self) -> MaintenanceStats {
        self.scheduler.get_stats().await
    }
    
    /// Update configuration
    pub fn update_config(&mut self, new_config: MaintenanceConfig) {
        self.config = new_config;
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &MaintenanceConfig {
        &self.config
    }
    
    /// Check if maintenance is currently running
    pub async fn is_maintenance_running(&self) -> bool {
        self.scheduler.is_running().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::vector_db::types::VectorStorageConfig;

    fn create_test_config() -> VectorStorageConfig {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_string_lossy().to_string();
        std::mem::forget(temp_dir); // Keep temp dir alive for test
        
        VectorStorageConfig {
            storage_dir,
            enable_compression: false,
            compression_algorithm: crate::vector_db::types::CompressionAlgorithm::None,
            max_entries_per_file: 100,
            enable_checksums: false,
            auto_backup: false,
            max_backups: 0,
            enable_metrics: true,
        }
    }

    fn create_test_maintenance_config() -> MaintenanceConfig {
        MaintenanceConfig {
            enable_automatic_maintenance: false, // Disable for tests
            maintenance_interval_seconds: 1,
            max_orphan_cleanup_per_cycle: 10,
            enable_index_compaction: true,
            compaction_cooldown_hours: 1,
            enable_defragmentation: true,
            compaction_threshold: 0.1,
            max_operation_duration_seconds: 5,
            enable_debug_logging: false, // Reduce test noise
            monitored_vault_paths: Vec::new(),
        }
    }

    #[test]
    fn test_maintenance_config_default() {
        let config = MaintenanceConfig::default();
        assert!(config.enable_automatic_maintenance);
        assert_eq!(config.maintenance_interval_seconds, 300);
        assert_eq!(config.max_orphan_cleanup_per_cycle, 100);
        assert!(config.enable_index_compaction);
    }

    #[test]
    fn test_maintenance_stats_update() {
        let mut stats = MaintenanceStats::default();
        
        // Test cycle time updates
        stats.update_cycle_time(100);
        stats.update_cycle_time(200);
        stats.update_cycle_time(300);
        
        assert_eq!(stats.avg_cycle_time_ms, 200.0); // Average of 100, 200, 300
        assert_eq!(stats.recent_cycle_times.len(), 3);
    }

    #[test]
    fn test_maintenance_stats_performance_targets() {
        let mut stats = MaintenanceStats::default();
        
        // Within target
        stats.avg_orphan_cleanup_time_ms = 4000.0; // 4 seconds
        assert!(stats.meets_performance_targets());
        
        // Exceeds target
        stats.avg_orphan_cleanup_time_ms = 6000.0; // 6 seconds  
        assert!(!stats.meets_performance_targets());
    }

    #[test]
    fn test_orphan_detection_result() {
        let result = OrphanDetectionResult {
            orphaned_entry_ids: vec!["id1".to_string(), "id2".to_string()],
            missing_file_paths: HashSet::new(),
            total_embeddings_checked: 100,
            detection_time_ms: 1500,
            validation_method: OrphanValidationMethod::FileSystem,
        };
        
        assert_eq!(result.orphaned_entry_ids.len(), 2);
        assert_eq!(result.total_embeddings_checked, 100);
        assert_eq!(result.detection_time_ms, 1500);
    }

    #[test]
    fn test_orphan_validation_method() {
        let method = OrphanValidationMethod::Combined {
            monitored_paths: vec![PathBuf::from("/test/vault")],
        };
        
        match method {
            OrphanValidationMethod::Combined { monitored_paths } => {
                assert_eq!(monitored_paths.len(), 1);
                assert_eq!(monitored_paths[0], PathBuf::from("/test/vault"));
            },
            _ => panic!("Expected Combined validation method"),
        }
    }

    #[tokio::test]
    async fn test_orphan_detector_creation() {
        let storage_config = create_test_config();
        let maintenance_config = create_test_maintenance_config();
        let storage = Arc::new(VectorStorage::new(storage_config.clone()).unwrap());
        let operations = VectorOperations::new(storage.clone(), storage_config);
        
        let detector = OrphanDetector::new(storage, operations, maintenance_config);
        
        assert_eq!(detector.config.max_orphan_cleanup_per_cycle, 10);
        assert!(detector.config.enable_index_compaction);
    }

    #[tokio::test]
    async fn test_index_optimizer_creation() {
        let storage_config = create_test_config();
        let maintenance_config = create_test_maintenance_config();
        let storage = Arc::new(VectorStorage::new(storage_config).unwrap());
        
        let optimizer = IndexOptimizer::new(storage, maintenance_config);
        
        assert!(optimizer.config.enable_index_compaction);
        assert_eq!(optimizer.config.compaction_cooldown_hours, 1);
    }

    #[tokio::test]
    async fn test_storage_reclaimer_creation() {
        let storage_config = create_test_config();
        let maintenance_config = create_test_maintenance_config();
        let storage = Arc::new(VectorStorage::new(storage_config).unwrap());
        
        let reclaimer = StorageReclaimer::new(storage, maintenance_config);
        
        assert!(reclaimer.config.enable_defragmentation);
        assert_eq!(reclaimer.config.max_operation_duration_seconds, 5);
    }

    #[tokio::test]
    async fn test_maintenance_scheduler_creation() {
        let storage_config = create_test_config();
        let maintenance_config = create_test_maintenance_config();
        let storage = Arc::new(VectorStorage::new(storage_config.clone()).unwrap());
        let operations = VectorOperations::new(storage.clone(), storage_config);
        
        let scheduler = MaintenanceScheduler::new(storage, operations, maintenance_config);
        
        assert!(!scheduler.is_running().await);
        
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.maintenance_cycles, 0);
        assert_eq!(stats.orphaned_embeddings_removed, 0);
    }

    // Note: Full integration tests with actual file operations will be in the integration test suite
    // These unit tests focus on structure validation and basic functionality
}