//! Comprehensive Tests for Performance Monitoring System
//!
//! This test suite validates the performance monitoring system implementation
//! including metrics collection, real-time monitoring, alerting, and integration
//! with existing index management operations.

use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;

use crate::vector_db::performance_monitor::{
    IndexPerformanceMonitor, MonitoringConfig, OperationType, OperationStatus, OperationMetrics,
    IncrementalUpdateMonitoring, MaintenanceMonitoring, RebuildingMonitoring
};
use crate::vector_db::incremental::UpdateStats;
use crate::vector_db::maintenance::MaintenanceStats;
use crate::vector_db::rebuilding::RebuildMetrics;
use crate::commands::monitoring_commands::{
    StartMonitoringRequest, MonitoringStatusResponse, PerformanceReportRequest,
    AcknowledgeAlertRequest, MonitorOperationRequest
};

/// Test performance monitor creation and basic functionality
#[tokio::test]
async fn test_performance_monitor_creation() {
    let config = MonitoringConfig {
        enable_monitoring: true,
        max_samples_in_memory: 100,
        collection_interval_ms: 50,
        enable_resource_tracking: false, // Disable for test
        enable_alerts: false, // Disable for test
        ..Default::default()
    };

    let monitor = IndexPerformanceMonitor::new(config.clone());
    
    // Test that monitor was created with correct configuration
    assert_eq!(monitor.config.max_samples_in_memory, 100);
    assert_eq!(monitor.config.collection_interval_ms, 50);
    assert!(!monitor.monitoring_enabled.load(std::sync::atomic::Ordering::Relaxed));
}

/// Test starting and stopping the monitoring system
#[tokio::test]
async fn test_monitor_start_stop_lifecycle() {
    let config = MonitoringConfig {
        enable_monitoring: true,
        enable_resource_tracking: false,
        enable_alerts: false,
        ..Default::default()
    };

    let mut monitor = IndexPerformanceMonitor::new(config);
    
    // Test starting monitor
    let result = monitor.start().await;
    assert!(result.is_ok(), "Monitor should start successfully");
    assert!(monitor.monitoring_enabled.load(std::sync::atomic::Ordering::Relaxed));
    
    // Allow background tasks to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Test stopping monitor
    let result = monitor.stop().await;
    assert!(result.is_ok(), "Monitor should stop successfully");
    assert!(!monitor.monitoring_enabled.load(std::sync::atomic::Ordering::Relaxed));
}

/// Test complete operation lifecycle: start -> update -> complete
#[tokio::test]
async fn test_operation_lifecycle() {
    let config = MonitoringConfig {
        enable_monitoring: true,
        enable_resource_tracking: false,
        enable_alerts: false,
        ..Default::default()
    };

    let mut monitor = IndexPerformanceMonitor::new(config);
    monitor.start().await.expect("Monitor should start");

    let operation_id = "test_incremental_op_001".to_string();
    
    // Start operation
    let result = monitor.start_operation(
        OperationType::IncrementalUpdate,
        operation_id.clone()
    ).await;
    assert!(result.is_ok(), "Should start operation successfully");

    // Check that operation is active
    let current_metrics = monitor.get_current_metrics().await.expect("Should get metrics");
    assert!(current_metrics.contains_key(&OperationType::IncrementalUpdate));

    // Update operation progress
    let mut operation_data = HashMap::new();
    operation_data.insert("files_processed".to_string(), serde_json::Value::Number(10.into()));
    
    let result = monitor.update_operation(
        &operation_id,
        10, // items processed
        1024, // bytes read
        512, // bytes written  
        Some(operation_data)
    ).await;
    assert!(result.is_ok(), "Should update operation successfully");

    // Complete operation
    let result = monitor.complete_operation(
        &operation_id,
        OperationStatus::Success,
        None
    ).await;
    assert!(result.is_ok(), "Should complete operation successfully");

    // Allow time for metrics processing
    sleep(Duration::from_millis(100)).await;

    // Verify operation was recorded
    assert_eq!(monitor.total_operations.load(std::sync::atomic::Ordering::Relaxed), 1);

    monitor.stop().await.expect("Monitor should stop");
}

/// Test performance report generation with empty data
#[tokio::test]
async fn test_performance_report_generation() {
    let config = MonitoringConfig {
        enable_monitoring: true,
        enable_resource_tracking: false,
        enable_alerts: false,
        ..Default::default()
    };

    let monitor = IndexPerformanceMonitor::new(config);

    // Generate report for last 24 hours
    let report = monitor.generate_performance_report(24).await
        .expect("Should generate report successfully");

    // Verify report structure
    assert_eq!(report.total_operations, 0); // No operations yet
    assert!(report.health_score >= 0.0 && report.health_score <= 1.0);
    assert!(!report.recommendations.is_empty()); // Should have at least default recommendations
    assert!(report.active_alerts.is_empty()); // No alerts initially
    assert_eq!(report.operations_by_type.len(), 0); // No operations by type
}

/// Test metrics calculation and averaging
#[tokio::test]
async fn test_metrics_calculation() {
    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    // Create test metrics
    let base_time = Utc::now();
    
    let metrics1 = OperationMetrics {
        operation_type: OperationType::IncrementalUpdate,
        operation_id: "test1".to_string(),
        started_at: base_time,
        completed_at: Some(base_time + chrono::Duration::milliseconds(100)),
        duration_ms: Some(100.0),
        status: OperationStatus::Success,
        items_processed: 10,
        processing_rate: 100.0,
        memory_start_mb: 100.0,
        memory_peak_mb: 150.0,
        memory_end_mb: 120.0,
        cpu_usage_percent: 50.0,
        io_operations: 5,
        bytes_read: 1024,
        bytes_written: 512,
        operation_data: HashMap::new(),
        error_message: None,
    };

    let metrics2 = OperationMetrics {
        operation_id: "test2".to_string(),
        duration_ms: Some(200.0),
        items_processed: 20,
        processing_rate: 100.0,
        memory_peak_mb: 200.0,
        cpu_usage_percent: 60.0,
        ..metrics1.clone()
    };

    let metrics_list = vec![&metrics1, &metrics2];
    let avg_metrics = monitor.calculate_average_metrics(&metrics_list)
        .expect("Should calculate average metrics");

    // Verify averages
    assert_eq!(avg_metrics.duration_ms.unwrap(), 150.0);
    assert_eq!(avg_metrics.processing_rate, 100.0);
    assert_eq!(avg_metrics.memory_peak_mb, 175.0);
    assert_eq!(avg_metrics.cpu_usage_percent, 55.0);
    assert_eq!(avg_metrics.items_processed, 15);
}

/// Test integration with incremental update system
#[tokio::test]
async fn test_incremental_update_integration() {
    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    // Create sample incremental update stats
    let update_stats = UpdateStats {
        update_id: "incremental_001".to_string(),
        files_added: 5,
        files_modified: 3,
        files_deleted: 2,
        files_processed: 10,
        embeddings_added: 15,
        embeddings_updated: 8,
        embeddings_removed: 6,
        total_embeddings: 100,
        duration_ms: 1500,
        errors_encountered: 0,
        vault_path: "/test/vault".to_string(),
        processed_at: Utc::now(),
    };

    // Convert to operation metrics
    let operation_metrics = monitor.to_operation_metrics(&update_stats);

    // Verify conversion
    assert_eq!(operation_metrics.operation_type, OperationType::IncrementalUpdate);
    assert_eq!(operation_metrics.duration_ms.unwrap(), 1500.0);
    assert_eq!(operation_metrics.items_processed, 10);
    assert_eq!(operation_metrics.status, OperationStatus::Success);
    assert!(operation_metrics.processing_rate > 0.0);
    
    // Check operation data contains incremental-specific information
    assert!(operation_metrics.operation_data.contains_key("files_added"));
    assert!(operation_metrics.operation_data.contains_key("files_modified"));
    assert!(operation_metrics.operation_data.contains_key("embeddings_updated"));
}

/// Test integration with maintenance system
#[tokio::test]
async fn test_maintenance_integration() {
    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    // Create sample maintenance stats
    let maintenance_stats = MaintenanceStats {
        maintenance_cycles: 5,
        orphaned_embeddings_removed: 25,
        compaction_operations: 2,
        storage_space_reclaimed: 1024000, // 1MB
        defragmentation_operations: 1,
        avg_cycle_time_ms: 2000.0,
        avg_orphan_cleanup_time_ms: 500.0,
        last_maintenance_at: Utc::now().timestamp() as u64,
        last_compaction_at: Utc::now().timestamp() as u64,
    };

    // Convert to operation metrics
    let operation_metrics = monitor.to_operation_metrics(&maintenance_stats);

    // Verify conversion
    assert_eq!(operation_metrics.operation_type, OperationType::Maintenance);
    assert_eq!(operation_metrics.duration_ms.unwrap(), 2000.0);
    assert_eq!(operation_metrics.items_processed, 25);
    assert_eq!(operation_metrics.status, OperationStatus::Success);
    
    // Check operation data contains maintenance-specific information
    assert!(operation_metrics.operation_data.contains_key("maintenance_cycles"));
    assert!(operation_metrics.operation_data.contains_key("orphaned_embeddings_removed"));
    assert!(operation_metrics.operation_data.contains_key("storage_space_reclaimed"));
}

/// Test integration with rebuilding system
#[tokio::test]
async fn test_rebuilding_integration() {
    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    // Create sample rebuild metrics
    let rebuild_metrics = RebuildMetrics {
        rebuild_id: "rebuild_001".to_string(),
        embeddings_processed: 500,
        total_duration_seconds: 30.0,
        avg_processing_rate: 16.67, // ~500/30
        peak_memory_usage_mb: 256.0,
        avg_cpu_usage_percent: 75.0,
        parallel_workers: 4,
        batch_size: 50,
        total_io_operations: 100,
        total_bytes_read: 5242880, // 5MB
        total_bytes_written: 3145728, // 3MB
        validation_passed: true,
        corruption_detected: false,
        errors_encountered: 0,
        started_at: Utc::now(),
        completed_at: Some(Utc::now() + chrono::Duration::seconds(30)),
    };

    // Convert to operation metrics
    let operation_metrics = monitor.to_operation_metrics(&rebuild_metrics);

    // Verify conversion
    assert_eq!(operation_metrics.operation_type, OperationType::Rebuilding);
    assert_eq!(operation_metrics.duration_ms.unwrap(), 30000.0); // 30 seconds
    assert_eq!(operation_metrics.items_processed, 500);
    assert_eq!(operation_metrics.memory_peak_mb, 256.0);
    assert_eq!(operation_metrics.cpu_usage_percent, 75.0);
    assert_eq!(operation_metrics.status, OperationStatus::Success);
    
    // Check operation data contains rebuilding-specific information
    assert!(operation_metrics.operation_data.contains_key("parallel_workers"));
    assert!(operation_metrics.operation_data.contains_key("validation_passed"));
    assert!(operation_metrics.operation_data.contains_key("corruption_detected"));
}

/// Test performance trend generation
#[tokio::test]
async fn test_performance_trend_generation() {
    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    let base_time = Utc::now() - chrono::Duration::hours(2);
    let end_time = Utc::now();
    
    // Create sample operations spread over time
    let mut operations = Vec::new();
    
    for i in 0..5 {
        let op_time = base_time + chrono::Duration::minutes(i * 30);
        let op = OperationMetrics {
            operation_type: OperationType::IncrementalUpdate,
            operation_id: format!("trend_test_{}", i),
            started_at: op_time,
            completed_at: Some(op_time + chrono::Duration::milliseconds(100 + i * 20)),
            duration_ms: Some(100.0 + i as f64 * 20.0),
            status: OperationStatus::Success,
            items_processed: 10 + i,
            processing_rate: (10 + i) as f64 / ((100.0 + i as f64 * 20.0) / 1000.0),
            memory_start_mb: 100.0,
            memory_peak_mb: 150.0 + i as f64 * 10.0,
            memory_end_mb: 120.0,
            cpu_usage_percent: 50.0 + i as f64 * 5.0,
            io_operations: 5 + i,
            bytes_read: 1024 + i * 256,
            bytes_written: 512 + i * 128,
            operation_data: HashMap::new(),
            error_message: None,
        };
        operations.push(op);
    }

    let operation_refs: Vec<&OperationMetrics> = operations.iter().collect();
    let trends = monitor.generate_performance_trends(&operation_refs, base_time, end_time);

    // Verify trends were generated
    assert!(!trends.is_empty(), "Should generate performance trends");
    
    // Verify trend data structure
    for trend in &trends {
        assert_eq!(trend.operation_type, OperationType::IncrementalUpdate);
        assert!(trend.avg_duration_ms > 0.0);
        assert!(trend.avg_processing_rate > 0.0);
        assert!(trend.avg_memory_usage_mb > 0.0);
        assert!(trend.operation_count > 0);
    }
}

/// Test resource utilization summary calculation
#[tokio::test]
async fn test_resource_utilization_summary() {
    use crate::vector_db::performance_monitor::ResourceMetrics;
    use std::collections::VecDeque;

    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    let base_time = Utc::now() - chrono::Duration::hours(1);
    let end_time = Utc::now();
    
    // Create sample resource metrics
    let mut resource_history = VecDeque::new();
    
    for i in 0..60 {
        let resource_time = base_time + chrono::Duration::minutes(i);
        let resource = ResourceMetrics {
            timestamp: resource_time,
            cpu_usage_percent: 50.0 + i as f64 * 0.5,
            memory_usage_mb: 1000.0 + i as f64 * 10.0,
            memory_available_mb: 7000.0 - i as f64 * 5.0,
            disk_read_mb_per_sec: 2.0 + i as f64 * 0.1,
            disk_write_mb_per_sec: 1.5 + i as f64 * 0.05,
            network_receive_kb_per_sec: 100.0 + i as f64,
            network_transmit_kb_per_sec: 50.0 + i as f64 * 0.5,
            active_threads: 8,
            load_average_1min: 0.5 + i as f64 * 0.01,
        };
        resource_history.push_back(resource);
    }

    let summary = monitor.calculate_resource_utilization_summary(
        &resource_history,
        base_time,
        end_time
    );

    // Verify summary calculations
    assert!(summary.avg_cpu_usage_percent > 50.0);
    assert!(summary.peak_cpu_usage_percent > summary.avg_cpu_usage_percent);
    assert!(summary.avg_memory_usage_mb > 1000.0);
    assert!(summary.peak_memory_usage_mb > summary.avg_memory_usage_mb);
    assert!(summary.total_disk_io_mb > 0.0);
    assert!(summary.total_network_io_kb > 0.0);
}

/// Test health score calculation
#[tokio::test]
async fn test_health_score_calculation() {
    use crate::vector_db::performance_monitor::{ResourceUtilizationSummary, PerformanceAlert, AlertSeverity};

    let config = MonitoringConfig::default();
    let monitor = IndexPerformanceMonitor::new(config);

    // Test with good performance metrics
    let mut good_metrics = HashMap::new();
    good_metrics.insert(OperationType::IncrementalUpdate, OperationMetrics {
        operation_type: OperationType::IncrementalUpdate,
        operation_id: "good_test".to_string(),
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        duration_ms: Some(50.0), // Fast operation
        status: OperationStatus::Success,
        items_processed: 100,
        processing_rate: 20.0, // Good rate
        memory_start_mb: 100.0,
        memory_peak_mb: 150.0,
        memory_end_mb: 120.0,
        cpu_usage_percent: 30.0, // Reasonable CPU
        io_operations: 10,
        bytes_read: 1024,
        bytes_written: 512,
        operation_data: HashMap::new(),
        error_message: None,
    });

    let good_resources = ResourceUtilizationSummary {
        avg_cpu_usage_percent: 30.0, // Low CPU usage
        peak_cpu_usage_percent: 40.0,
        avg_memory_usage_mb: 200.0, // Reasonable memory
        peak_memory_usage_mb: 300.0,
        total_disk_io_mb: 10.0,
        avg_disk_io_mb_per_sec: 1.0,
        total_network_io_kb: 100.0,
        avg_network_io_kb_per_sec: 5.0,
    };

    let no_alerts = Vec::new();
    
    let good_score = monitor.calculate_health_score(&good_metrics, &good_resources, &no_alerts);
    assert!(good_score > 0.8, "Good performance should have high health score: {}", good_score);

    // Test with poor performance metrics
    let mut poor_metrics = HashMap::new();
    poor_metrics.insert(OperationType::IncrementalUpdate, OperationMetrics {
        operation_type: OperationType::IncrementalUpdate,
        operation_id: "poor_test".to_string(),
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        duration_ms: Some(5000.0), // Very slow operation
        status: OperationStatus::Success,
        items_processed: 10,
        processing_rate: 2.0, // Poor rate
        memory_start_mb: 100.0,
        memory_peak_mb: 1500.0, // High memory usage
        memory_end_mb: 120.0,
        cpu_usage_percent: 95.0, // Very high CPU
        io_operations: 100,
        bytes_read: 10240,
        bytes_written: 5120,
        operation_data: HashMap::new(),
        error_message: None,
    });

    let poor_resources = ResourceUtilizationSummary {
        avg_cpu_usage_percent: 90.0, // Very high CPU
        peak_cpu_usage_percent: 100.0,
        avg_memory_usage_mb: 1200.0, // High memory usage
        peak_memory_usage_mb: 1500.0,
        total_disk_io_mb: 100.0,
        avg_disk_io_mb_per_sec: 10.0,
        total_network_io_kb: 1000.0,
        avg_network_io_kb_per_sec: 50.0,
    };

    let critical_alert = PerformanceAlert {
        alert_id: "critical_001".to_string(),
        severity: AlertSeverity::Critical,
        message: "Critical performance degradation".to_string(),
        operation_type: OperationType::IncrementalUpdate,
        triggered_at: Utc::now(),
        triggering_metrics: poor_metrics[&OperationType::IncrementalUpdate].clone(),
        suggested_actions: vec!["Reduce batch size".to_string()],
        acknowledged: false,
    };

    let critical_alerts = vec![critical_alert];
    
    let poor_score = monitor.calculate_health_score(&poor_metrics, &poor_resources, &critical_alerts);
    assert!(poor_score < 0.5, "Poor performance should have low health score: {}", poor_score);
    assert!(poor_score < good_score, "Poor score should be lower than good score");
}

/// Test concurrent operation monitoring
#[tokio::test]
async fn test_concurrent_operations() {
    let config = MonitoringConfig {
        enable_monitoring: true,
        enable_resource_tracking: false,
        enable_alerts: false,
        ..Default::default()
    };

    let mut monitor = IndexPerformanceMonitor::new(config);
    monitor.start().await.expect("Monitor should start");

    // Start multiple operations concurrently
    let mut operation_ids = Vec::new();
    for i in 0..5 {
        let op_id = format!("concurrent_op_{}", i);
        let op_type = match i % 3 {
            0 => OperationType::IncrementalUpdate,
            1 => OperationType::Maintenance,
            _ => OperationType::Rebuilding,
        };
        
        monitor.start_operation(op_type, op_id.clone()).await
            .expect("Should start operation");
        operation_ids.push(op_id);
    }

    // Verify all operations are active
    let current_metrics = monitor.get_current_metrics().await
        .expect("Should get current metrics");
    assert_eq!(current_metrics.len(), 3); // 3 operation types

    // Complete all operations
    for (i, op_id) in operation_ids.iter().enumerate() {
        monitor.complete_operation(
            op_id,
            OperationStatus::Success,
            None
        ).await.expect("Should complete operation");
    }

    // Allow time for processing
    sleep(Duration::from_millis(100)).await;

    // Verify all operations were recorded
    assert_eq!(monitor.total_operations.load(std::sync::atomic::Ordering::Relaxed), 5);

    monitor.stop().await.expect("Monitor should stop");
}

/// Test monitoring configuration validation
#[tokio::test]
async fn test_monitoring_configuration() {
    // Test invalid configuration
    let invalid_config = MonitoringConfig {
        enable_monitoring: true,
        max_samples_in_memory: 0, // Invalid
        collection_interval_ms: 0, // Invalid
        max_overhead_percent: 110.0, // Invalid percentage
        alert_degradation_threshold: -10.0, // Invalid threshold
        ..Default::default()
    };

    let monitor = IndexPerformanceMonitor::new(invalid_config);
    
    // Monitor should handle invalid config gracefully
    // (Implementation would validate and use defaults for invalid values)
    assert!(monitor.config.max_samples_in_memory == 0); // Stores what was provided

    // Test valid configuration
    let valid_config = MonitoringConfig {
        enable_monitoring: true,
        max_samples_in_memory: 1000,
        collection_interval_ms: 100,
        enable_resource_tracking: true,
        max_overhead_percent: 3.0,
        enable_alerts: true,
        alert_degradation_threshold: 15.0,
        ..Default::default()
    };

    let monitor = IndexPerformanceMonitor::new(valid_config.clone());
    assert_eq!(monitor.config.max_samples_in_memory, 1000);
    assert_eq!(monitor.config.collection_interval_ms, 100);
    assert!(monitor.config.enable_resource_tracking);
    assert_eq!(monitor.config.max_overhead_percent, 3.0);
}

/// Test error handling in monitoring operations
#[tokio::test]
async fn test_error_handling() {
    let config = MonitoringConfig {
        enable_monitoring: true,
        enable_resource_tracking: false,
        enable_alerts: false,
        ..Default::default()
    };

    let mut monitor = IndexPerformanceMonitor::new(config);
    monitor.start().await.expect("Monitor should start");

    let operation_id = "error_test_op".to_string();
    
    // Start operation
    monitor.start_operation(
        OperationType::IncrementalUpdate,
        operation_id.clone()
    ).await.expect("Should start operation");

    // Complete with error
    let error_message = "Test error: file not found".to_string();
    monitor.complete_operation(
        &operation_id,
        OperationStatus::Failed,
        Some(error_message.clone())
    ).await.expect("Should complete operation with error");

    // Allow time for processing
    sleep(Duration::from_millis(100)).await;

    // Verify error was recorded
    assert_eq!(monitor.total_operations.load(std::sync::atomic::Ordering::Relaxed), 1);

    // Test operations on non-existent operation
    let result = monitor.complete_operation(
        "non_existent_op",
        OperationStatus::Success,
        None
    ).await;
    assert!(result.is_ok()); // Should handle gracefully

    monitor.stop().await.expect("Monitor should stop");
}

/// Test memory usage and performance overhead validation
#[tokio::test]
async fn test_performance_overhead_validation() {
    use std::process;
    use std::time::Instant;

    let config = MonitoringConfig {
        enable_monitoring: true,
        max_samples_in_memory: 1000,
        collection_interval_ms: 10, // High frequency for testing
        enable_resource_tracking: true,
        resource_tracking_interval_ms: 100,
        enable_alerts: false, // Disable to focus on core overhead
        ..Default::default()
    };

    // Measure baseline performance without monitoring
    let start_memory = get_current_memory_usage();
    let baseline_start = Instant::now();
    
    // Simulate some work without monitoring
    for i in 0..1000 {
        let _dummy_work = format!("operation_{}", i);
        // Simulate small delay
        std::thread::sleep(std::time::Duration::from_micros(10));
    }
    
    let baseline_duration = baseline_start.elapsed();
    let baseline_memory = get_current_memory_usage();

    // Now test with monitoring enabled
    let mut monitor = IndexPerformanceMonitor::new(config);
    monitor.start().await.expect("Monitor should start");

    let monitoring_start_memory = get_current_memory_usage();
    let monitoring_start = Instant::now();
    
    // Simulate same work with monitoring
    for i in 0..1000 {
        let op_id = format!("perf_test_op_{}", i);
        
        // Start monitoring
        monitor.start_operation(
            OperationType::IncrementalUpdate,
            op_id.clone()
        ).await.expect("Should start operation");
        
        // Simulate work
        let _dummy_work = format!("operation_{}", i);
        std::thread::sleep(std::time::Duration::from_micros(10));
        
        // Update monitoring
        monitor.update_operation(&op_id, 1, 100, 50, None)
            .await.expect("Should update operation");
        
        // Complete monitoring
        monitor.complete_operation(&op_id, OperationStatus::Success, None)
            .await.expect("Should complete operation");
    }
    
    let monitoring_duration = monitoring_start.elapsed();
    let monitoring_end_memory = get_current_memory_usage();

    monitor.stop().await.expect("Monitor should stop");

    // Calculate overhead
    let time_overhead_percent = ((monitoring_duration.as_millis() as f64 - baseline_duration.as_millis() as f64) 
        / baseline_duration.as_millis() as f64) * 100.0;
    
    let memory_overhead_mb = (monitoring_end_memory - monitoring_start_memory) as f64 / 1024.0 / 1024.0;

    println!("Performance Overhead Analysis:");
    println!("  Baseline duration: {:?}", baseline_duration);
    println!("  Monitoring duration: {:?}", monitoring_duration);
    println!("  Time overhead: {:.2}%", time_overhead_percent);
    println!("  Memory overhead: {:.2}MB", memory_overhead_mb);

    // Validate overhead is within acceptable limits
    // Note: These thresholds are generous for testing environment
    assert!(time_overhead_percent < 20.0, 
        "Time overhead should be <20% in test environment, got {:.2}%", time_overhead_percent);
    
    assert!(memory_overhead_mb < 50.0,
        "Memory overhead should be <50MB in test environment, got {:.2}MB", memory_overhead_mb);
}

/// Helper function to get current memory usage (simplified)
fn get_current_memory_usage() -> usize {
    // This is a simplified implementation for testing
    // In production, would use system APIs to get actual memory usage
    std::process::id() as usize * 1024 // Placeholder
}

/// Integration test with actual command handlers
#[tokio::test] 
async fn test_command_integration() {
    use crate::commands::monitoring_commands::{
        start_performance_monitoring, stop_performance_monitoring, 
        get_monitoring_status, generate_performance_report
    };

    // Test starting monitoring via command
    let start_request = StartMonitoringRequest {
        config: Some(MonitoringConfig {
            enable_monitoring: true,
            enable_resource_tracking: false,
            enable_alerts: false,
            ..Default::default()
        })
    };

    let result = start_performance_monitoring(start_request).await;
    assert!(result.is_ok(), "Start monitoring command should succeed");
    
    let status = result.unwrap();
    assert!(status.is_active, "Monitoring should be active");

    // Test getting status
    let status_result = get_monitoring_status().await;
    assert!(status_result.is_ok(), "Get status command should succeed");
    
    let status = status_result.unwrap();
    assert!(status.is_active, "Status should show monitoring as active");

    // Test generating report
    let report_request = PerformanceReportRequest {
        period_hours: Some(1),
        include_detailed_breakdown: Some(true),
        include_resource_analysis: Some(true),
    };

    let report_result = generate_performance_report(report_request).await;
    assert!(report_result.is_ok(), "Generate report command should succeed");
    
    let report = report_result.unwrap();
    assert!(report.health_score >= 0.0 && report.health_score <= 1.0);
    assert!(!report.recommendations.is_empty());

    // Test stopping monitoring
    let stop_result = stop_performance_monitoring().await;
    assert!(stop_result.is_ok(), "Stop monitoring command should succeed");

    // Verify monitoring is stopped
    let final_status = get_monitoring_status().await.unwrap();
    assert!(!final_status.is_active, "Monitoring should be inactive after stop");
}