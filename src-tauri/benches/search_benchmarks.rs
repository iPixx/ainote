//! Performance Benchmarks for Similarity Search Engine
//! 
//! This benchmark suite provides comprehensive performance testing for all similarity 
//! search algorithms and optimizations according to issue #113 requirements.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use ainote_lib::similarity_search::{
    SimilaritySearch, SearchConfig, PerformanceConfig, ConcurrentSearchManager
};
use ainote_lib::vector_db::types::EmbeddingEntry;
use std::sync::Arc;
use std::time::Duration;

/// Generate deterministic test vectors for benchmarking
fn generate_benchmark_vector(seed: u32, dim: usize) -> Vec<f32> {
    let mut vector = Vec::with_capacity(dim);
    let mut x = seed as f32;
    
    for _i in 0..dim {
        x = ((x * 9301.0 + 49297.0) % 233280.0) / 233280.0; // Simple LCG
        vector.push(x - 0.5); // Center around 0
    }
    
    vector
}

/// Create test embedding entry for benchmarking
fn create_benchmark_entry(vector: Vec<f32>, id: usize) -> EmbeddingEntry {
    EmbeddingEntry::new(
        vector,
        format!("benchmark_doc_{:06}.md", id),
        format!("chunk_{:03}", (id % 10) + 1),
        &format!("This is benchmark content for document {}", id),
        "benchmark-model".to_string(),
    )
}

/// Generate benchmark dataset with specified size and dimension
fn generate_benchmark_dataset(size: usize, dim: usize) -> Vec<EmbeddingEntry> {
    let mut entries = Vec::with_capacity(size);
    
    for i in 0..size {
        let vector = generate_benchmark_vector((i + 1) as u32, dim);
        let entry = create_benchmark_entry(vector, i);
        entries.push(entry);
    }
    
    entries
}

/// Benchmark cosine similarity calculation
fn bench_cosine_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_similarity");
    
    let dimensions = [64, 128, 256, 384, 512, 768, 1024, 1536];
    
    for dim in dimensions {
        let vec_a = generate_benchmark_vector(1, dim);
        let vec_b = generate_benchmark_vector(2, dim);
        
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(
            BenchmarkId::new("standard", dim),
            &dim,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::cosine_similarity(
                        black_box(&vec_a),
                        black_box(&vec_b)
                    ))
                })
            }
        );
        
        // Benchmark normalized version
        let norm_a = SimilaritySearch::normalize_vector(&vec_a).unwrap();
        let norm_b = SimilaritySearch::normalize_vector(&vec_b).unwrap();
        
        group.bench_with_input(
            BenchmarkId::new("normalized", dim),
            &dim,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::cosine_similarity_normalized(
                        black_box(&norm_a),
                        black_box(&norm_b)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark vector normalization
fn bench_vector_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_normalization");
    
    let dimensions = [64, 128, 256, 384, 512, 768, 1024, 1536];
    
    for dim in dimensions {
        let vector = generate_benchmark_vector(1, dim);
        
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(
            BenchmarkId::new("normalize", dim),
            &dim,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::normalize_vector(
                        black_box(&vector)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark k-NN search with different dataset sizes
fn bench_knn_search_dataset_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("knn_dataset_scaling");
    group.sample_size(50); // Reduce samples for large datasets
    group.measurement_time(Duration::from_secs(30));
    
    let dataset_sizes = [100, 500, 1000, 2000, 5000];
    let vector_dim = 384; // Common embedding dimension
    let k = 10;
    
    let query = generate_benchmark_vector(1, vector_dim);
    let config = SearchConfig::default();
    
    for size in dataset_sizes {
        let entries = generate_benchmark_dataset(size, vector_dim);
        
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("standard_knn", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark parallel k-NN search performance
fn bench_parallel_knn_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_knn_search");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(45));
    
    let dataset_sizes = [500, 1000, 2000, 5000];
    let vector_dim = 384;
    let k = 10;
    
    let query = generate_benchmark_vector(1, vector_dim);
    let config = SearchConfig::default();
    let perf_config = PerformanceConfig::default();
    
    for size in dataset_sizes {
        let entries = generate_benchmark_dataset(size, vector_dim);
        
        group.throughput(Throughput::Elements(size as u64));
        
        // Benchmark standard k-NN
        group.bench_with_input(
            BenchmarkId::new("standard", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config)
                    ))
                })
            }
        );
        
        // Benchmark parallel k-NN
        group.bench_with_input(
            BenchmarkId::new("parallel", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::parallel_k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config),
                        black_box(&perf_config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark approximate nearest neighbors search
fn bench_approximate_knn_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("approximate_knn_search");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(60));
    
    let dataset_sizes = [1000, 2000, 5000, 10000];
    let vector_dim = 384;
    let k = 10;
    
    let query = generate_benchmark_vector(1, vector_dim);
    let config = SearchConfig::default();
    let perf_config = PerformanceConfig {
        approximate_threshold: 1000, // Enable for all test sizes
        ..PerformanceConfig::default()
    };
    
    for size in dataset_sizes {
        let entries = generate_benchmark_dataset(size, vector_dim);
        
        group.throughput(Throughput::Elements(size as u64));
        
        // Benchmark exact search
        group.bench_with_input(
            BenchmarkId::new("exact", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::parallel_k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config),
                        black_box(&perf_config)
                    ))
                })
            }
        );
        
        // Benchmark approximate search
        group.bench_with_input(
            BenchmarkId::new("approximate", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::approximate_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config),
                        black_box(&perf_config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark batch processing performance
fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(45));
    
    let batch_sizes = [1, 5, 10, 20, 50];
    let dataset_size = 1000;
    let vector_dim = 256;
    let k = 5;
    
    let entries = generate_benchmark_dataset(dataset_size, vector_dim);
    let config = SearchConfig::default();
    let perf_config = PerformanceConfig::default();
    
    for batch_size in batch_sizes {
        let queries: Vec<Vec<f32>> = (0..batch_size)
            .map(|i| generate_benchmark_vector((i + 1) as u32, vector_dim))
            .collect();
        
        group.throughput(Throughput::Elements((batch_size * dataset_size) as u64));
        
        // Benchmark individual queries
        group.bench_with_input(
            BenchmarkId::new("individual", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    let mut results = Vec::new();
                    for query in &queries {
                        let result = black_box(SimilaritySearch::k_nearest_neighbors(
                            black_box(query),
                            black_box(&entries),
                            black_box(k),
                            black_box(&config)
                        ));
                        results.push(result);
                    }
                    black_box(results)
                })
            }
        );
        
        // Benchmark batch processing
        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::batch_k_nearest_neighbors(
                        black_box(&queries),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config)
                    ))
                })
            }
        );
        
        // Benchmark memory-efficient batch processing
        group.bench_with_input(
            BenchmarkId::new("memory_efficient_batch", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::memory_efficient_batch_search(
                        black_box(&queries),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config),
                        black_box(&perf_config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark threshold search performance
fn bench_threshold_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("threshold_search");
    group.sample_size(50);
    
    let thresholds = [0.0, 0.3, 0.5, 0.7, 0.9];
    let dataset_size = 1000;
    let vector_dim = 256;
    
    let query = generate_benchmark_vector(1, vector_dim);
    let entries = generate_benchmark_dataset(dataset_size, vector_dim);
    let config = SearchConfig::default();
    
    for threshold in thresholds {
        group.throughput(Throughput::Elements(dataset_size as u64));
        group.bench_with_input(
            BenchmarkId::new("threshold", (threshold * 100.0) as u32),
            &threshold,
            |b, &threshold| {
                b.iter(|| {
                    black_box(SimilaritySearch::threshold_search(
                        black_box(&query),
                        black_box(&entries),
                        black_box(threshold),
                        black_box(&config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark concurrent search performance
fn bench_concurrent_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_search");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    
    let concurrent_requests = [1, 2, 4, 8, 16];
    let dataset_size = 500;
    let vector_dim = 256;
    let k = 5;
    
    let entries = Arc::new(generate_benchmark_dataset(dataset_size, vector_dim));
    let config = SearchConfig::default();
    let perf_config = PerformanceConfig {
        max_concurrent_requests: 20,
        ..PerformanceConfig::default()
    };
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    for num_concurrent in concurrent_requests {
        let queries: Vec<Vec<f32>> = (0..num_concurrent)
            .map(|i| generate_benchmark_vector((i + 1) as u32, vector_dim))
            .collect();
        
        group.throughput(Throughput::Elements((num_concurrent * dataset_size) as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent", num_concurrent),
            &num_concurrent,
            |b, _| {
                b.to_async(&runtime).iter(|| async {
                    let manager = ConcurrentSearchManager::new(perf_config.clone());
                    let entries_clone = Arc::clone(&entries);
                    
                    black_box(manager.execute_batch_search(
                        black_box(queries.clone()),
                        black_box(entries_clone),
                        black_box(k),
                        black_box(config.clone())
                    ).await)
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark different vector dimensions
fn bench_vector_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_dimensions");
    
    let dimensions = [64, 128, 256, 384, 512, 768, 1024, 1536];
    let dataset_size = 1000;
    let k = 10;
    
    let config = SearchConfig::default();
    
    for dim in dimensions {
        let query = generate_benchmark_vector(1, dim);
        let entries = generate_benchmark_dataset(dataset_size, dim);
        
        group.throughput(Throughput::Elements((dataset_size * dim) as u64));
        group.bench_with_input(
            BenchmarkId::new("knn_search", dim),
            &dim,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark different k values
fn bench_k_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("k_values");
    
    let k_values = [1, 5, 10, 20, 50, 100];
    let dataset_size = 1000;
    let vector_dim = 384;
    
    let query = generate_benchmark_vector(1, vector_dim);
    let entries = generate_benchmark_dataset(dataset_size, vector_dim);
    let config = SearchConfig::default();
    
    for k in k_values {
        group.throughput(Throughput::Elements(k as u64));
        group.bench_with_input(
            BenchmarkId::new("knn_search", k),
            &k,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Memory usage benchmark
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(20);
    
    let dataset_sizes = [100, 500, 1000, 2000];
    let vector_dim = 384;
    let k = 10;
    
    let query = generate_benchmark_vector(1, vector_dim);
    let config = SearchConfig::default();
    let perf_config = PerformanceConfig {
        enable_memory_optimization: true,
        ..PerformanceConfig::default()
    };
    
    for size in dataset_sizes {
        let entries = generate_benchmark_dataset(size, vector_dim);
        
        group.throughput(Throughput::Elements(size as u64));
        
        // Standard search
        group.bench_with_input(
            BenchmarkId::new("standard", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config)
                    ))
                })
            }
        );
        
        // Memory-optimized search
        group.bench_with_input(
            BenchmarkId::new("memory_optimized", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(SimilaritySearch::parallel_k_nearest_neighbors(
                        black_box(&query),
                        black_box(&entries),
                        black_box(k),
                        black_box(&config),
                        black_box(&perf_config)
                    ))
                })
            }
        );
    }
    
    group.finish();
}

/// Comprehensive accuracy vs performance benchmark
fn bench_accuracy_vs_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("accuracy_vs_performance");
    group.sample_size(15);
    group.measurement_time(Duration::from_secs(60));
    
    let dataset_size = 3000;
    let vector_dim = 384;
    let k = 20;
    
    let query = generate_benchmark_vector(42, vector_dim);
    let entries = generate_benchmark_dataset(dataset_size, vector_dim);
    
    let configs = [
        ("exact_standard", SearchConfig::default(), PerformanceConfig::default()),
        ("exact_parallel", SearchConfig::default(), PerformanceConfig {
            parallel_threshold: 500,
            ..PerformanceConfig::default()
        }),
        ("approximate", SearchConfig::default(), PerformanceConfig {
            enable_approximate: true,
            approximate_threshold: 1000,
            ..PerformanceConfig::default()
        }),
        ("memory_optimized", SearchConfig::default(), PerformanceConfig {
            enable_memory_optimization: true,
            parallel_threshold: 500,
            ..PerformanceConfig::default()
        }),
    ];
    
    for (name, config, perf_config) in configs {
        group.throughput(Throughput::Elements(dataset_size as u64));
        group.bench_with_input(
            BenchmarkId::new("algorithm", name),
            &name,
            |b, _| {
                b.iter(|| {
                    match name {
                        "exact_standard" => {
                            black_box(SimilaritySearch::k_nearest_neighbors(
                                black_box(&query),
                                black_box(&entries),
                                black_box(k),
                                black_box(&config)
                            ).map(|r| r))
                        },
                        name if name.starts_with("approximate") => {
                            black_box(SimilaritySearch::approximate_nearest_neighbors(
                                black_box(&query),
                                black_box(&entries),
                                black_box(k),
                                black_box(&config),
                                black_box(&perf_config)
                            ).map(|r| r.results))
                        },
                        _ => {
                            black_box(SimilaritySearch::parallel_k_nearest_neighbors(
                                black_box(&query),
                                black_box(&entries),
                                black_box(k),
                                black_box(&config),
                                black_box(&perf_config)
                            ).map(|r| r.results))
                        }
                    }
                })
            }
        );
    }
    
    group.finish();
}

// Define criterion groups
criterion_group!(
    similarity_benches,
    bench_cosine_similarity,
    bench_vector_normalization
);

criterion_group!(
    knn_benches,
    bench_knn_search_dataset_scaling,
    bench_parallel_knn_search,
    bench_approximate_knn_search,
    bench_vector_dimensions,
    bench_k_values
);

criterion_group!(
    batch_benches,
    bench_batch_processing,
    bench_threshold_search,
    bench_concurrent_search
);

criterion_group!(
    optimization_benches,
    bench_memory_usage,
    bench_accuracy_vs_performance
);

// Main criterion entry point
criterion_main!(
    similarity_benches,
    knn_benches,
    batch_benches,
    optimization_benches
);