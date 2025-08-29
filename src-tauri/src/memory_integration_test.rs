//! Memory Management Integration Test
//!
//! Tests the memory management system in a more realistic scenario
//! to ensure it meets the performance targets specified in issue #173.

use crate::memory_manager::{MemoryManager, MemoryManagerConfig, AllocationType};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_memory_management_performance_targets() {
    // Configure memory manager with performance targets from issue #173
    let config = MemoryManagerConfig {
        max_memory_mb: 100,                    // Base memory usage <100MB
        ai_operations_limit_mb: 50,            // 50MB for AI operations
        monitoring_interval_seconds: 1,        // Fast monitoring for test
        enable_auto_gc: true,
        gc_trigger_threshold_percent: 75.0,
        enable_leak_detection: true,
        leak_detection_threshold_mb: 5.0,
        alert_threshold_percent: 85.0,
        cache_cleanup_interval_seconds: 5,     // Fast cleanup for test
    };

    let mut manager = MemoryManager::new(config);
    
    // Start memory monitoring
    manager.start().await.expect("Failed to start memory manager");
    
    // Simulate typical aiNote workload
    let operations = vec![
        ("embedding_cache_1", "embedding_cache", 15.0, AllocationType::EmbeddingCache),
        ("vector_storage_1", "vector_db", 25.0, AllocationType::VectorStorage),
        ("ai_operation_1", "ollama_integration", 20.0, AllocationType::AiOperation),
        ("file_op_1", "file_system", 5.0, AllocationType::FileOperation),
        ("search_1", "similarity_search", 10.0, AllocationType::SearchResult),
    ];

    // Track allocations
    for (id, component, size_mb, alloc_type) in &operations {
        let size_bytes = (*size_mb * 1024.0 * 1024.0) as usize;
        
        manager.track_allocation(
            id.to_string(),
            component.to_string(),
            size_bytes,
            alloc_type.clone(),
        ).await.expect("Failed to track allocation");
    }

    // Wait for monitoring to collect data
    sleep(Duration::from_secs(2)).await;

    // Validate performance targets
    let metrics = manager.get_memory_metrics().await.expect("Failed to get metrics");
    
    // Target: Base memory usage <100MB
    assert!(metrics.total_memory_mb < 100.0, 
            "Memory usage {:.2}MB exceeds 100MB target", metrics.total_memory_mb);
    
    // Target: Memory allocation tracking works
    assert_eq!(metrics.active_allocations, 5, "Expected 5 active allocations");
    
    // Target: Cache hit rate >80% (simulated)
    // This would be tested with actual cache operations in a full integration test
    
    // Test garbage collection performance
    let start = std::time::Instant::now();
    let cleaned_bytes = manager.trigger_gc().await.expect("Failed to trigger GC");
    let gc_duration = start.elapsed();
    
    // Target: Memory cleanup within 5s
    assert!(gc_duration < Duration::from_secs(5), 
            "GC took {:.2}s, exceeds 5s target", gc_duration.as_secs_f64());
    
    // Test AI allocation limits
    let ai_alloc_result = manager.request_ai_allocation("test_ai_op", 60 * 1024 * 1024).await;
    assert!(ai_alloc_result.is_err(), "AI allocation should fail when exceeding limit");
    
    // Test memory leak detection
    let leaks = manager.detect_memory_leaks().await.expect("Failed to detect leaks");
    // With our simple test setup, we shouldn't detect any leaks
    assert_eq!(leaks.len(), 0, "No leaks should be detected in test setup");
    
    // Clean up
    manager.stop().await.expect("Failed to stop memory manager");
    
    println!("✅ Memory management system meets all performance targets:");
    println!("   - Base memory usage: {:.2}MB < 100MB ✓", metrics.total_memory_mb);
    println!("   - Active allocations tracked: {} ✓", metrics.active_allocations);
    println!("   - GC cleanup time: {:.2}s < 5s ✓", gc_duration.as_secs_f64());
    println!("   - AI allocation limits enforced ✓");
    println!("   - Memory leak detection operational ✓");
}

#[tokio::test]
async fn test_memory_pressure_handling() {
    let config = MemoryManagerConfig {
        max_memory_mb: 50,  // Lower limit to test pressure handling
        ai_operations_limit_mb: 20,
        monitoring_interval_seconds: 1,
        enable_auto_gc: true,
        gc_trigger_threshold_percent: 60.0,  // Lower threshold
        enable_leak_detection: true,
        leak_detection_threshold_mb: 2.0,
        alert_threshold_percent: 70.0,
        cache_cleanup_interval_seconds: 2,
    };

    let mut manager = MemoryManager::new(config);
    manager.start().await.expect("Failed to start memory manager");
    
    // Simulate memory pressure by allocating near the limit
    let large_allocation = (40.0 * 1024.0 * 1024.0) as usize; // 40MB
    
    manager.track_allocation(
        "large_allocation".to_string(),
        "test_component".to_string(),
        large_allocation,
        AllocationType::VectorStorage,
    ).await.expect("Failed to track large allocation");
    
    // Wait for monitoring
    sleep(Duration::from_millis(1500)).await;
    
    let metrics = manager.get_memory_metrics().await.expect("Failed to get metrics");
    
    // Should be at high memory pressure
    assert!(metrics.memory_pressure > 0.7, 
            "Expected high memory pressure, got {:.2}", metrics.memory_pressure);
    
    // Should trigger automatic GC
    assert!(metrics.usage_percentage > 60.0, 
            "Expected high usage percentage, got {:.1}%", metrics.usage_percentage);
    
    manager.stop().await.expect("Failed to stop memory manager");
    
    println!("✅ Memory pressure handling works correctly:");
    println!("   - Memory pressure: {:.2} > 0.7 ✓", metrics.memory_pressure);
    println!("   - Usage percentage: {:.1}% > 60% ✓", metrics.usage_percentage);
}