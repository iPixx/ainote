//! Comprehensive Performance Benchmarks for Vector Database
//! 
//! This module provides detailed performance benchmarks for all vector database
//! operations to validate performance requirements and identify optimization
//! opportunities.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use ainote_lib::vector_db::{
    VectorDatabase,
    types::{EmbeddingEntry, VectorStorageConfig, CompressionAlgorithm},
};

/// Benchmark configuration factory
struct BenchmarkConfig;

impl BenchmarkConfig {
    fn performance_optimized() -> (VectorStorageConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = VectorStorageConfig {
            storage_dir: temp_dir.path().to_string_lossy().to_string(),
            enable_compression: false, // Disabled for pure performance
            compression_algorithm: CompressionAlgorithm::None,
            max_entries_per_file: 1000,
            enable_checksums: false, // Disabled for pure performance
            auto_backup: false, // Disabled for pure performance
            max_backups: 0,
            enable_metrics: true, // Keep for validation
        };
        (config, temp_dir)
    }

    fn compression_enabled() -> (VectorStorageConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = VectorStorageConfig {
            storage_dir: temp_dir.path().to_string_lossy().to_string(),
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Gzip,
            max_entries_per_file: 500,
            enable_checksums: true,
            auto_backup: false,
            max_backups: 0,
            enable_metrics: true,
        };
        (config, temp_dir)
    }
}

/// Test data factory for benchmarks
struct BenchmarkData;

impl BenchmarkData {
    fn create_embedding(id: usize, vector_size: usize) -> EmbeddingEntry {
        let vector = (0..vector_size).map(|i| ((i + id) as f32) * 0.001).collect();
        EmbeddingEntry::new(
            vector,
            format!("/benchmark/file_{}.md", id % 100),
            format!("chunk_{}", id),
            &format!("Benchmark document content {} with sufficient text length for realistic testing scenarios in performance evaluation", id),
            "benchmark-model-v1".to_string(),
        )
    }

    fn create_batch(count: usize, vector_size: usize) -> Vec<EmbeddingEntry> {
        (0..count)
            .map(|i| Self::create_embedding(i, vector_size))
            .collect()
    }
}

/// Core CRUD Operations Benchmarks
fn bench_crud_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("crud_operations");
    
    // Benchmark single embedding storage
    group.bench_function("store_single_embedding", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let entry = BenchmarkData::create_embedding(1, 384);
            let result = db.store_embedding(
                black_box(entry.vector.clone()),
                entry.metadata.file_path.clone(),
                entry.metadata.chunk_id.clone(),
                "Benchmark test content",
                entry.metadata.model_name.clone(),
            ).await;
            
            black_box(result.unwrap());
        });
    });

    // Benchmark single embedding retrieval
    group.bench_function("retrieve_single_embedding", |b| {
        let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
        let setup_db = rt.block_on(async {
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let entry = BenchmarkData::create_embedding(1, 384);
            let stored_id = db.store_embedding(
                entry.vector.clone(),
                entry.metadata.file_path.clone(),
                entry.metadata.chunk_id.clone(),
                "Benchmark test content",
                entry.metadata.model_name.clone(),
            ).await.unwrap();
            
            (db, stored_id)
        });

        b.to_async(&rt).iter(|| async {
            let result = setup_db.0.retrieve_embedding(black_box(&setup_db.1)).await;
            black_box(result.unwrap());
        });
    });

    // Benchmark embedding update
    group.bench_function("update_embedding", |b| {
        let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
        let setup_db = rt.block_on(async {
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let entry = BenchmarkData::create_embedding(1, 384);
            let stored_id = db.store_embedding(
                entry.vector.clone(),
                entry.metadata.file_path.clone(),
                entry.metadata.chunk_id.clone(),
                "Benchmark test content",
                entry.metadata.model_name.clone(),
            ).await.unwrap();
            
            (db, stored_id)
        });

        b.to_async(&rt).iter(|| async {
            let new_vector = vec![0.99; 384];
            let result = setup_db.0.update_embedding(black_box(&setup_db.1), black_box(new_vector)).await;
            black_box(result.unwrap());
        });
    });

    // Benchmark embedding deletion
    group.bench_function("delete_embedding", |b| {
        b.to_async(&rt).iter_batched(
            || {
                rt.block_on(async {
                    let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
                    let db = VectorDatabase::new(config).await.unwrap();
                    db.initialize().await.unwrap();
                    
                    let entry = BenchmarkData::create_embedding(1, 384);
                    let stored_id = db.store_embedding(
                        entry.vector.clone(),
                        entry.metadata.file_path.clone(),
                        entry.metadata.chunk_id.clone(),
                        "Benchmark test content",
                        entry.metadata.model_name.clone(),
                    ).await.unwrap();
                    
                    (db, stored_id)
                })
            },
            |(db, stored_id)| async move {
                let result = db.delete_embedding(black_box(&stored_id)).await;
                black_box(result.unwrap());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Batch Operations Benchmarks
fn bench_batch_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("batch_operations");
    
    // Test different batch sizes
    let batch_sizes = vec![10, 50, 100, 500, 1000];
    
    for batch_size in batch_sizes {
        // Batch store benchmark
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_store", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
                    let db = VectorDatabase::new(config).await.unwrap();
                    db.initialize().await.unwrap();
                    
                    let batch_data = BenchmarkData::create_batch(batch_size, 384);
                    let result = db.store_embeddings_batch(black_box(batch_data)).await;
                    black_box(result.unwrap());
                });
            },
        );

        // Batch retrieve benchmark
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_retrieve", batch_size),
            &batch_size,
            |b, &batch_size| {
                let setup_data = rt.block_on(async {
                    let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
                    let db = VectorDatabase::new(config).await.unwrap();
                    db.initialize().await.unwrap();
                    
                    let batch_data = BenchmarkData::create_batch(batch_size, 384);
                    let stored_ids = db.store_embeddings_batch(batch_data).await.unwrap();
                    
                    (db, stored_ids)
                });

                b.to_async(&rt).iter(|| async {
                    let result = setup_data.0.retrieve_embeddings(black_box(&setup_data.1)).await;
                    black_box(result.unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Vector Size Performance Impact Benchmarks
fn bench_vector_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("vector_sizes");
    
    // Test different vector dimensions
    let vector_sizes = vec![128, 256, 384, 512, 768, 1024, 1536, 2048, 4096];
    
    for vector_size in vector_sizes {
        group.throughput(Throughput::Elements(vector_size as u64));
        group.bench_with_input(
            BenchmarkId::new("store_by_vector_size", vector_size),
            &vector_size,
            |b, &vector_size| {
                b.to_async(&rt).iter(|| async {
                    let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
                    let db = VectorDatabase::new(config).await.unwrap();
                    db.initialize().await.unwrap();
                    
                    let entry = BenchmarkData::create_embedding(1, vector_size);
                    let result = db.store_embedding(
                        black_box(entry.vector.clone()),
                        entry.metadata.file_path.clone(),
                        entry.metadata.chunk_id.clone(),
                        "Vector size benchmark",
                        entry.metadata.model_name.clone(),
                    ).await;
                    
                    black_box(result.unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("retrieve_by_vector_size", vector_size),
            &vector_size,
            |b, &vector_size| {
                let setup_data = rt.block_on(async {
                    let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
                    let db = VectorDatabase::new(config).await.unwrap();
                    db.initialize().await.unwrap();
                    
                    let entry = BenchmarkData::create_embedding(1, vector_size);
                    let stored_id = db.store_embedding(
                        entry.vector.clone(),
                        entry.metadata.file_path.clone(),
                        entry.metadata.chunk_id.clone(),
                        "Vector size benchmark",
                        entry.metadata.model_name.clone(),
                    ).await.unwrap();
                    
                    (db, stored_id)
                });

                b.to_async(&rt).iter(|| async {
                    let result = setup_data.0.retrieve_embedding(black_box(&setup_data.1)).await;
                    black_box(result.unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Compression Performance Benchmarks
fn bench_compression_impact(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("compression_impact");
    
    // Benchmark without compression
    group.bench_function("store_batch_no_compression", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized(); // No compression
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let batch_data = BenchmarkData::create_batch(100, 768);
            let result = db.store_embeddings_batch(black_box(batch_data)).await;
            black_box(result.unwrap());
        });
    });

    // Benchmark with compression
    group.bench_function("store_batch_with_compression", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = BenchmarkConfig::compression_enabled(); // With compression
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let batch_data = BenchmarkData::create_batch(100, 768);
            let result = db.store_embeddings_batch(black_box(batch_data)).await;
            black_box(result.unwrap());
        });
    });

    // Benchmark retrieval without compression
    group.bench_function("retrieve_batch_no_compression", |b| {
        let setup_data = rt.block_on(async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let batch_data = BenchmarkData::create_batch(100, 768);
            let stored_ids = db.store_embeddings_batch(batch_data).await.unwrap();
            
            (db, stored_ids)
        });

        b.to_async(&rt).iter(|| async {
            let result = setup_data.0.retrieve_embeddings(black_box(&setup_data.1)).await;
            black_box(result.unwrap());
        });
    });

    // Benchmark retrieval with compression
    group.bench_function("retrieve_batch_with_compression", |b| {
        let setup_data = rt.block_on(async {
            let (config, _temp_dir) = BenchmarkConfig::compression_enabled();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let batch_data = BenchmarkData::create_batch(100, 768);
            let stored_ids = db.store_embeddings_batch(batch_data).await.unwrap();
            
            (db, stored_ids)
        });

        b.to_async(&rt).iter(|| async {
            let result = setup_data.0.retrieve_embeddings(black_box(&setup_data.1)).await;
            black_box(result.unwrap());
        });
    });

    group.finish();
}

/// Database Scale Benchmarks (Large datasets)
fn bench_database_scale(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_scale");
    group.sample_size(10); // Fewer samples for large scale tests
    group.measurement_time(Duration::from_secs(30)); // Longer measurement time

    // Benchmark database startup time with existing data
    let database_sizes = vec![100, 500, 1000, 2000];
    
    for db_size in database_sizes {
        group.bench_with_input(
            BenchmarkId::new("startup_time", db_size),
            &db_size,
            |b, &db_size| {
                // Pre-populate database
                let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
                let populated_config = rt.block_on(async {
                    let db = VectorDatabase::new(config.clone()).await.unwrap();
                    db.initialize().await.unwrap();
                    
                    // Store data in batches to simulate realistic usage
                    let batch_size = 100;
                    for i in (0..db_size).step_by(batch_size) {
                        let remaining = std::cmp::min(batch_size, db_size - i);
                        let batch_data = BenchmarkData::create_batch(remaining, 384);
                        let _ = db.store_embeddings_batch(batch_data).await.unwrap();
                    }
                    
                    config
                });

                b.to_async(&rt).iter(|| async {
                    // Measure startup time of new database instance with existing data
                    let db = VectorDatabase::new(black_box(populated_config.clone())).await.unwrap();
                    let init_result = db.initialize().await;
                    black_box(init_result.unwrap());
                });
            },
        );
    }

    // Benchmark search operations on large datasets
    group.bench_function("search_large_dataset", |b| {
        let setup_db = rt.block_on(async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            // Pre-populate with 1000 entries across different files
            let batch_data = BenchmarkData::create_batch(1000, 384);
            let _ = db.store_embeddings_batch(batch_data).await.unwrap();
            
            db
        });

        b.to_async(&rt).iter(|| async {
            let result = setup_db.find_embeddings_by_file(black_box("/benchmark/file_50.md")).await;
            black_box(result.unwrap());
        });
    });

    group.finish();
}

/// Memory Usage and Resource Efficiency Benchmarks
fn bench_memory_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_efficiency");
    
    // Benchmark memory usage with different cache sizes
    group.bench_function("memory_usage_1000_entries", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            // Store 1000 entries and measure memory footprint
            let batch_data = BenchmarkData::create_batch(1000, 384);
            let stored_ids = db.store_embeddings_batch(batch_data).await.unwrap();
            
            // Access some entries to populate cache
            let sample_ids = &stored_ids[0..50];
            let _retrieved = db.retrieve_embeddings(sample_ids).await.unwrap();
            
            // Get metrics to measure memory usage
            let metrics = db.get_metrics().await.unwrap();
            
            // Ensure memory usage is within requirements (<50MB for 1000 notes)
            let memory_mb = metrics.cache.memory_usage_bytes as f64 / (1024.0 * 1024.0);
            assert!(memory_mb < 50.0, "Memory usage {:.2}MB exceeds 50MB limit", memory_mb);
            
            black_box(metrics);
        });
    });

    // Benchmark disk usage efficiency
    group.bench_function("disk_usage_1000_entries", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            // Store 1000 entries and measure disk usage
            let batch_data = BenchmarkData::create_batch(1000, 384);
            let _stored_ids = db.store_embeddings_batch(batch_data).await.unwrap();
            
            let file_metrics = db.get_file_metrics().await.unwrap();
            
            // Ensure disk usage is within requirements (<10MB per 1000 embeddings)
            let disk_mb = file_metrics.total_size_bytes as f64 / (1024.0 * 1024.0);
            assert!(disk_mb < 10.0, "Disk usage {:.2}MB exceeds 10MB limit", disk_mb);
            
            black_box(file_metrics);
        });
    });

    group.finish();
}

/// Concurrent Operations Benchmarks
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_operations");
    group.sample_size(20); // Fewer samples due to complexity
    
    // Benchmark concurrent write operations
    group.bench_function("concurrent_writes", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = std::sync::Arc::new(VectorDatabase::new(config).await.unwrap());
            db.initialize().await.unwrap();
            
            // Spawn 10 concurrent write tasks
            let mut handles = vec![];
            for i in 0..10 {
                let db_clone = std::sync::Arc::clone(&db);
                let handle = tokio::spawn(async move {
                    let batch_data = BenchmarkData::create_batch(50, 384);
                    db_clone.store_embeddings_batch(batch_data).await
                });
                handles.push(handle);
            }
            
            // Wait for all writes to complete
            for handle in handles {
                let result = handle.await.unwrap();
                black_box(result.unwrap());
            }
        });
    });

    // Benchmark concurrent read operations
    group.bench_function("concurrent_reads", |b| {
        let setup_db = rt.block_on(async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = std::sync::Arc::new(VectorDatabase::new(config).await.unwrap());
            db.initialize().await.unwrap();
            
            // Pre-populate with data
            let batch_data = BenchmarkData::create_batch(500, 384);
            let stored_ids = db.store_embeddings_batch(batch_data).await.unwrap();
            
            (db, stored_ids)
        });

        b.to_async(&rt).iter(|| async {
            // Spawn 10 concurrent read tasks
            let mut handles = vec![];
            for i in 0..10 {
                let db_clone = std::sync::Arc::clone(&setup_db.0);
                let ids_to_read = setup_db.1[(i * 10)..((i + 1) * 10)].to_vec();
                
                let handle = tokio::spawn(async move {
                    db_clone.retrieve_embeddings(&ids_to_read).await
                });
                handles.push(handle);
            }
            
            // Wait for all reads to complete
            for handle in handles {
                let result = handle.await.unwrap();
                black_box(result.unwrap());
            }
        });
    });

    group.finish();
}

/// Performance Requirements Validation Benchmarks
fn bench_performance_requirements(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("performance_requirements");
    group.sample_size(30);
    
    // Requirement 1: Store 1000 embeddings <5 seconds
    group.bench_function("requirement_store_1000_under_5s", |b| {
        b.to_async(&rt).iter(|| async {
            let start = std::time::Instant::now();
            
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let batch_data = BenchmarkData::create_batch(1000, 384);
            let result = db.store_embeddings_batch(batch_data).await.unwrap();
            
            let duration = start.elapsed();
            assert!(duration.as_secs() < 5, "Store 1000 embeddings took {:?} (requirement: <5s)", duration);
            
            black_box((result, duration));
        });
    });

    // Requirement 2: Retrieve single embedding <1ms
    group.bench_function("requirement_retrieve_single_under_1ms", |b| {
        let setup_data = rt.block_on(async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config).await.unwrap();
            db.initialize().await.unwrap();
            
            let entry = BenchmarkData::create_embedding(1, 384);
            let stored_id = db.store_embedding(
                entry.vector.clone(),
                entry.metadata.file_path.clone(),
                entry.metadata.chunk_id.clone(),
                "Performance test",
                entry.metadata.model_name.clone(),
            ).await.unwrap();
            
            (db, stored_id)
        });

        b.to_async(&rt).iter(|| async {
            let start = std::time::Instant::now();
            let result = setup_data.0.retrieve_embedding(&setup_data.1).await.unwrap();
            let duration = start.elapsed();
            
            assert!(duration.as_millis() < 1, "Single retrieve took {:?} (requirement: <1ms)", duration);
            black_box((result, duration));
        });
    });

    // Requirement 3: Database startup <2 seconds
    group.bench_function("requirement_startup_under_2s", |b| {
        // Pre-populate database
        let config = rt.block_on(async {
            let (config, _temp_dir) = BenchmarkConfig::performance_optimized();
            let db = VectorDatabase::new(config.clone()).await.unwrap();
            db.initialize().await.unwrap();
            
            // Store moderate amount of data
            let batch_data = BenchmarkData::create_batch(500, 384);
            let _ = db.store_embeddings_batch(batch_data).await.unwrap();
            
            config
        });

        b.to_async(&rt).iter(|| async {
            let start = std::time::Instant::now();
            
            let db = VectorDatabase::new(config.clone()).await.unwrap();
            let init_result = db.initialize().await.unwrap();
            
            let duration = start.elapsed();
            assert!(duration.as_secs() < 2, "Database startup took {:?} (requirement: <2s)", duration);
            
            black_box((init_result, duration));
        });
    });

    group.finish();
}

// Configure all benchmark groups
criterion_group!(
    benches,
    bench_crud_operations,
    bench_batch_operations,
    bench_vector_sizes,
    bench_compression_impact,
    bench_database_scale,
    bench_memory_efficiency,
    bench_concurrent_operations,
    bench_performance_requirements
);

criterion_main!(benches);