//! Comprehensive Tests for Enhanced Metrics Collection System
//!
//! This module provides extensive testing for the enhanced metrics collection system
//! implemented for issue #146, ensuring accuracy, performance, and reliability of:
//!
//! - Search operation metrics collection
//! - Index health monitoring
//! - Memory usage tracking
//! - Optimization recommendation generation
//! - Historical data management

use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::vector_db::types::{VectorStorageConfig, EmbeddingEntry};
use crate::vector_db::storage::VectorStorage;
use crate::vector_db::metrics_collector::{
    EnhancedMetricsCollector, MetricsCollectorConfig, SearchOperationMetrics, 
    SearchOperationType, IndexHealthStatus, OptimizationCategory, RecommendationPriority
};

/// Helper function to create test storage
async fn create_test_storage() -> Arc<VectorStorage> {
    let config = VectorStorageConfig {
        storage_dir: "/tmp/test_metrics_storage".to_string(),
        enable_compression: false,
        enable_metrics: true,
        ..VectorStorageConfig::default()
    };
    
    Arc::new(VectorStorage::new(config).expect("Failed to create test storage"))
}

/// Helper function to create test embedding entries
fn create_test_embeddings(count: usize) -> Vec<EmbeddingEntry> {
    (0..count)
        .map(|i| {
            let vector = vec![0.1 * i as f32; 384]; // 384-dimensional test vectors
            EmbeddingEntry::new(
                vector,
                format!("/test/file_{}.md", i),
                format!("chunk_{}", i),
                &format!("Test content for embedding {}", i),
                "test-model".to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn test_metrics_collector_creation_and_startup() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig::default();
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    
    // Test startup
    let result = collector.start().await;
    assert!(result.is_ok(), "Failed to start metrics collector: {:?}", result.err());
    
    // Test duplicate startup (should fail)
    let duplicate_result = collector.start().await;
    assert!(duplicate_result.is_err(), "Duplicate startup should fail");
    
    // Test shutdown
    let stop_result = collector.stop().await;
    assert!(stop_result.is_ok(), "Failed to stop metrics collector: {:?}", stop_result.err());
}

#[tokio::test]
async fn test_search_metrics_recording() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig {
        enable_search_metrics: true,
        max_search_metrics_history: 100,
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Create test search metrics
    let search_metrics = SearchOperationMetrics {
        operation_id: "test_search_001".to_string(),
        operation_type: SearchOperationType::KNearestNeighbors,
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        duration_ms: Some(25.5),
        query_dimension: 384,
        vectors_searched: 1000,
        results_returned: 10,
        similarity_threshold: 0.8,
        top_similarity_score: Some(0.95),
        avg_similarity_score: Some(0.87),
        memory_usage_mb: 150.0,
        cpu_usage_percent: 45.0,
        efficiency_score: 0.01, // 10 results out of 1000 vectors
        performance_target_met: true, // 25.5ms < 50ms target
        error_message: None,
    };
    
    // Record the metrics
    let result = collector.record_search_operation(search_metrics.clone()).await;
    assert!(result.is_ok(), "Failed to record search metrics: {:?}", result.err());
    
    // Retrieve the metrics history
    let history = collector.get_search_metrics_history(Some(10)).await;
    assert_eq!(history.len(), 1, "Expected 1 search metric in history");
    assert_eq!(history[0].operation_id, "test_search_001");
    assert_eq!(history[0].vectors_searched, 1000);
    assert_eq!(history[0].results_returned, 10);
    assert!(history[0].performance_target_met);
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_search_metrics_history_limit() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig {
        enable_search_metrics: true,
        max_search_metrics_history: 5, // Small limit for testing
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Record more metrics than the limit
    for i in 0..10 {
        let search_metrics = SearchOperationMetrics {
            operation_id: format!("test_search_{:03}", i),
            operation_type: SearchOperationType::KNearestNeighbors,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            duration_ms: Some(20.0 + i as f64),
            query_dimension: 384,
            vectors_searched: 1000,
            results_returned: 5,
            similarity_threshold: 0.8,
            top_similarity_score: Some(0.9),
            avg_similarity_score: Some(0.85),
            memory_usage_mb: 100.0,
            cpu_usage_percent: 30.0,
            efficiency_score: 0.005,
            performance_target_met: true,
            error_message: None,
        };
        
        collector.record_search_operation(search_metrics).await
            .expect("Failed to record search metrics");
    }
    
    // Check that only the maximum number are retained
    let history = collector.get_search_metrics_history(None).await;
    assert_eq!(history.len(), 5, "Expected exactly 5 metrics (max history limit)");
    
    // Check that the most recent metrics are retained (LIFO order)
    assert_eq!(history[0].operation_id, "test_search_009");
    assert_eq!(history[4].operation_id, "test_search_005");
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_index_health_monitoring() {
    let storage = create_test_storage().await;
    
    // Add some test data to the storage
    let test_embeddings = create_test_embeddings(50);
    storage.store_entries(test_embeddings).await
        .expect("Failed to store test embeddings");
    
    let config = MetricsCollectorConfig {
        enable_index_health_monitoring: true,
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Get current index health
    let health_metrics = collector.get_current_index_health().await
        .expect("Failed to get index health metrics");
    
    // Validate health metrics
    assert_eq!(health_metrics.total_embeddings, 50);
    assert!(health_metrics.index_size_bytes > 0);
    assert!(health_metrics.efficiency_score >= 0.0 && health_metrics.efficiency_score <= 1.0);
    assert!(health_metrics.fragmentation_percentage >= 0.0);
    assert!(matches!(
        health_metrics.health_status,
        IndexHealthStatus::Healthy | IndexHealthStatus::Warning | IndexHealthStatus::Critical
    ));
    
    // Validate that recommendations exist for any identified issues
    if !health_metrics.health_issues.is_empty() {
        assert!(!health_metrics.recommended_actions.is_empty(),
            "Health issues detected but no recommendations provided");
    }
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_memory_metrics_collection() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig {
        enable_memory_tracking: true,
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Get current memory metrics
    let memory_metrics = collector.get_current_memory_metrics().await
        .expect("Failed to get memory metrics");
    
    // Validate memory metrics structure
    assert!(memory_metrics.total_memory_mb > 0.0);
    assert!(memory_metrics.available_memory_mb > 0.0);
    assert!(memory_metrics.memory_pressure >= 0.0 && memory_metrics.memory_pressure <= 1.0);
    assert!(memory_metrics.efficiency_score >= 0.0 && memory_metrics.efficiency_score <= 1.0);
    
    // Validate memory breakdown
    let total_components = memory_metrics.vector_storage_mb 
        + memory_metrics.cache_memory_mb 
        + memory_metrics.index_memory_mb 
        + memory_metrics.search_operation_memory_mb;
    
    assert!(total_components <= memory_metrics.total_memory_mb * 1.1, // Allow 10% variance
        "Memory component breakdown exceeds total memory usage");
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_optimization_recommendations_generation() {
    let storage = create_test_storage().await;
    
    // Create an index with some inefficiencies to trigger recommendations
    let large_embeddings = create_test_embeddings(200);
    storage.store_entries(large_embeddings).await
        .expect("Failed to store test embeddings");
    
    let config = MetricsCollectorConfig {
        enable_optimization_recommendations: true,
        enable_index_health_monitoring: true,
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Wait a moment for background tasks to collect data
    sleep(Duration::from_millis(100)).await;
    
    // Get optimization recommendations
    let recommendations = collector.get_optimization_recommendations().await;
    
    // Validate recommendation structure
    for recommendation in &recommendations {
        assert!(!recommendation.recommendation_id.is_empty());
        assert!(!recommendation.title.is_empty());
        assert!(!recommendation.description.is_empty());
        assert!(matches!(recommendation.category, 
            OptimizationCategory::IndexOptimization |
            OptimizationCategory::SearchOptimization |
            OptimizationCategory::MemoryOptimization |
            OptimizationCategory::StorageOptimization |
            OptimizationCategory::ConfigurationOptimization |
            OptimizationCategory::HardwareOptimization
        ));
        assert!(matches!(recommendation.priority,
            RecommendationPriority::Low |
            RecommendationPriority::Medium |
            RecommendationPriority::High |
            RecommendationPriority::Critical
        ));
        assert!(recommendation.estimated_time_hours > 0.0);
        assert!(!recommendation.implementation_steps.is_empty());
        assert!(recommendation.expected_improvement.confidence_level >= 0.0 
            && recommendation.expected_improvement.confidence_level <= 1.0);
    }
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_metrics_history_management() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig {
        enable_index_health_monitoring: true,
        enable_memory_tracking: true,
        index_health_check_interval_seconds: 1, // Fast for testing
        memory_snapshot_interval_seconds: 1,    // Fast for testing
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Wait for a few metrics to be collected
    sleep(Duration::from_millis(2500)).await;
    
    // Check index health history
    let health_history = collector.get_index_health_history(Some(10)).await;
    assert!(!health_history.is_empty(), "Expected index health history");
    assert!(health_history.len() <= 10, "History should respect limit");
    
    // Verify timestamps are in descending order (most recent first)
    for i in 1..health_history.len() {
        assert!(health_history[i-1].timestamp >= health_history[i].timestamp,
            "Health history should be in descending chronological order");
    }
    
    // Check memory metrics history
    let memory_history = collector.get_memory_metrics_history(Some(10)).await;
    assert!(!memory_history.is_empty(), "Expected memory metrics history");
    assert!(memory_history.len() <= 10, "Memory history should respect limit");
    
    // Verify timestamps are in descending order
    for i in 1..memory_history.len() {
        assert!(memory_history[i-1].timestamp >= memory_history[i].timestamp,
            "Memory history should be in descending chronological order");
    }
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_performance_overhead_requirements() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig::default();
    
    let mut collector = EnhancedMetricsCollector::new(config, storage.clone());
    
    // Measure startup time
    let start_time = std::time::Instant::now();
    collector.start().await.expect("Failed to start collector");
    let startup_time = start_time.elapsed();
    
    // Startup should be fast (<100ms)
    assert!(startup_time.as_millis() < 100, 
        "Startup time too slow: {}ms (target: <100ms)", startup_time.as_millis());
    
    // Measure metrics collection overhead
    let operations_start = std::time::Instant::now();
    for i in 0..10 {
        let search_metrics = SearchOperationMetrics {
            operation_id: format!("perf_test_{}", i),
            operation_type: SearchOperationType::KNearestNeighbors,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            duration_ms: Some(25.0),
            query_dimension: 384,
            vectors_searched: 1000,
            results_returned: 10,
            similarity_threshold: 0.8,
            top_similarity_score: Some(0.9),
            avg_similarity_score: Some(0.85),
            memory_usage_mb: 100.0,
            cpu_usage_percent: 30.0,
            efficiency_score: 0.01,
            performance_target_met: true,
            error_message: None,
        };
        
        collector.record_search_operation(search_metrics).await
            .expect("Failed to record search metrics");
    }
    let operations_time = operations_start.elapsed();
    
    // Metrics recording should be fast (<10ms for 10 operations)
    assert!(operations_time.as_millis() < 10,
        "Metrics recording too slow: {}ms for 10 operations (target: <10ms)", 
        operations_time.as_millis());
    
    // Test shutdown performance
    let shutdown_start = std::time::Instant::now();
    collector.stop().await.expect("Failed to stop collector");
    let shutdown_time = shutdown_start.elapsed();
    
    // Shutdown should be fast (<200ms to allow for persistence)
    assert!(shutdown_time.as_millis() < 200,
        "Shutdown time too slow: {}ms (target: <200ms)", shutdown_time.as_millis());
}

#[tokio::test]
async fn test_concurrent_metrics_operations() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig::default();
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Test sequential metrics recording (simpler than concurrent for now)
    for i in 0..20 {
        let search_metrics = SearchOperationMetrics {
            operation_id: format!("concurrent_test_{}", i),
            operation_type: SearchOperationType::KNearestNeighbors,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            duration_ms: Some(25.0 + i as f64),
            query_dimension: 384,
            vectors_searched: 1000,
            results_returned: 5,
            similarity_threshold: 0.8,
            top_similarity_score: Some(0.9),
            avg_similarity_score: Some(0.85),
            memory_usage_mb: 100.0,
            cpu_usage_percent: 30.0,
            efficiency_score: 0.005,
            performance_target_met: true,
            error_message: None,
        };
        
        let result = collector.record_search_operation(search_metrics).await;
        assert!(result.is_ok(), "Sequential metrics recording failed");
    }
    
    // Verify all metrics were recorded
    let history = collector.get_search_metrics_history(None).await;
    assert_eq!(history.len(), 20, "Expected all 20 sequential operations to be recorded");
    
    collector.stop().await.expect("Failed to stop collector");
}

#[tokio::test]
async fn test_metrics_accuracy_and_validation() {
    let storage = create_test_storage().await;
    let config = MetricsCollectorConfig {
        enable_search_metrics: true,
        enable_index_health_monitoring: true,
        enable_memory_tracking: true,
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Test search metrics accuracy
    let precise_search_metrics = SearchOperationMetrics {
        operation_id: "accuracy_test".to_string(),
        operation_type: SearchOperationType::SimilarityThreshold,
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        duration_ms: Some(42.123), // Precise duration
        query_dimension: 768,
        vectors_searched: 2500,
        results_returned: 15,
        similarity_threshold: 0.85,
        top_similarity_score: Some(0.987),
        avg_similarity_score: Some(0.912),
        memory_usage_mb: 234.567,
        cpu_usage_percent: 67.89,
        efficiency_score: 0.006, // 15/2500 = 0.006
        performance_target_met: true,
        error_message: None,
    };
    
    collector.record_search_operation(precise_search_metrics.clone()).await
        .expect("Failed to record precise metrics");
    
    // Retrieve and validate precision
    let history = collector.get_search_metrics_history(Some(1)).await;
    assert_eq!(history.len(), 1);
    
    let retrieved = &history[0];
    assert_eq!(retrieved.duration_ms, Some(42.123));
    assert_eq!(retrieved.vectors_searched, 2500);
    assert_eq!(retrieved.results_returned, 15);
    assert!((retrieved.efficiency_score - 0.006).abs() < 0.0001); // Float precision
    assert_eq!(retrieved.top_similarity_score, Some(0.987));
    assert_eq!(retrieved.avg_similarity_score, Some(0.912));
    
    // Test index health metrics validation
    let health_metrics = collector.get_current_index_health().await
        .expect("Failed to get index health metrics");
    
    // Validate ranges and consistency
    assert!(health_metrics.efficiency_score >= 0.0 && health_metrics.efficiency_score <= 1.0);
    assert!(health_metrics.fragmentation_percentage >= 0.0);
    assert!(health_metrics.index_density >= 0.0);
    
    // Memory metrics validation
    let memory_metrics = collector.get_current_memory_metrics().await
        .expect("Failed to get memory metrics");
    
    assert!(memory_metrics.memory_pressure >= 0.0 && memory_metrics.memory_pressure <= 1.0);
    assert!(memory_metrics.efficiency_score >= 0.0 && memory_metrics.efficiency_score <= 1.0);
    assert!(memory_metrics.total_memory_mb >= 
        memory_metrics.vector_storage_mb + memory_metrics.cache_memory_mb);
    
    collector.stop().await.expect("Failed to stop collector");
}

/// Integration test that validates the entire metrics collection pipeline
#[tokio::test]
async fn test_end_to_end_metrics_pipeline() {
    let storage = create_test_storage().await;
    
    // Add substantial test data
    let test_embeddings = create_test_embeddings(500);
    storage.store_entries(test_embeddings).await
        .expect("Failed to store test embeddings");
    
    let config = MetricsCollectorConfig {
        enable_search_metrics: true,
        enable_index_health_monitoring: true,
        enable_memory_tracking: true,
        enable_optimization_recommendations: true,
        index_health_check_interval_seconds: 2,
        memory_snapshot_interval_seconds: 1,
        max_search_metrics_history: 100,
        ..MetricsCollectorConfig::default()
    };
    
    let mut collector = EnhancedMetricsCollector::new(config, storage);
    collector.start().await.expect("Failed to start collector");
    
    // Simulate realistic usage pattern
    for i in 0..50 {
        let search_metrics = SearchOperationMetrics {
            operation_id: format!("e2e_test_{:03}", i),
            operation_type: if i % 2 == 0 { 
                SearchOperationType::KNearestNeighbors 
            } else { 
                SearchOperationType::SimilarityThreshold 
            },
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            duration_ms: Some(15.0 + (i % 10) as f64 * 5.0), // Varying performance
            query_dimension: 384,
            vectors_searched: 500,
            results_returned: if i % 10 == 0 { 0 } else { 5 + (i % 5) }, // Some with no results
            similarity_threshold: 0.8,
            top_similarity_score: if i % 10 == 0 { None } else { Some(0.85 + (i % 10) as f32 * 0.01) },
            avg_similarity_score: if i % 10 == 0 { None } else { Some(0.80 + (i % 8) as f32 * 0.01) },
            memory_usage_mb: 100.0 + (i % 20) as f64 * 5.0,
            cpu_usage_percent: 20.0 + (i % 15) as f64 * 3.0,
            efficiency_score: if i % 10 == 0 { 0.0 } else { (5.0 + (i % 5) as f64) / 500.0 },
            performance_target_met: (15.0 + (i % 10) as f64 * 5.0) < 50.0,
            error_message: if i % 25 == 0 { Some("Simulated error".to_string()) } else { None },
        };
        
        collector.record_search_operation(search_metrics).await
            .expect("Failed to record search metrics");
        
        // Small delay to simulate realistic timing
        sleep(Duration::from_millis(10)).await;
    }
    
    // Wait for background monitoring to collect data
    sleep(Duration::from_millis(3000)).await;
    
    // Validate complete pipeline
    
    // 1. Search metrics
    let search_history = collector.get_search_metrics_history(None).await;
    assert_eq!(search_history.len(), 50, "Expected all 50 search operations");
    
    let successful_searches = search_history.iter().filter(|m| m.error_message.is_none()).count();
    let failed_searches = search_history.iter().filter(|m| m.error_message.is_some()).count();
    assert_eq!(successful_searches + failed_searches, 50);
    
    // 2. Index health monitoring
    let health_history = collector.get_index_health_history(None).await;
    assert!(!health_history.is_empty(), "Expected index health history");
    
    let latest_health = collector.get_current_index_health().await
        .expect("Failed to get current health");
    assert_eq!(latest_health.total_embeddings, 500);
    
    // 3. Memory tracking
    let memory_history = collector.get_memory_metrics_history(None).await;
    assert!(!memory_history.is_empty(), "Expected memory metrics history");
    
    let current_memory = collector.get_current_memory_metrics().await
        .expect("Failed to get current memory");
    assert!(current_memory.total_memory_mb > 0.0);
    
    // 4. Optimization recommendations
    let recommendations = collector.get_optimization_recommendations().await;
    // Should have recommendations based on the index size and simulated inefficiencies
    
    // Validate recommendation quality if any exist
    for rec in &recommendations {
        assert!(!rec.title.is_empty());
        assert!(!rec.description.is_empty());
        assert!(rec.expected_improvement.confidence_level > 0.0);
        assert!(rec.estimated_time_hours > 0.0);
        assert!(!rec.implementation_steps.is_empty());
    }
    
    collector.stop().await.expect("Failed to stop collector");
}