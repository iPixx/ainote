use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ainote_lib::ollama_client::{OllamaClient, OllamaConfig};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark client creation and configuration
fn bench_client_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_creation");
    
    group.bench_function("new_default", |b| {
        b.iter(|| {
            black_box(OllamaClient::new())
        })
    });

    let config = OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        timeout_ms: 1000,
        max_retries: 3,
        initial_retry_delay_ms: 1000,
        max_retry_delay_ms: 30000,
    };

    group.bench_function("new_with_config", |b| {
        b.iter(|| {
            black_box(OllamaClient::with_config(config.clone()))
        })
    });

    group.finish();
}

/// Benchmark state access operations
fn bench_state_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = OllamaClient::new();

    let mut group = c.benchmark_group("state_access");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_connection_state", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(client.get_connection_state().await)
        })
    });

    group.bench_function("get_config", |b| {
        b.iter(|| {
            black_box(client.get_config())
        })
    });

    group.finish();
}

/// Benchmark configuration operations
fn bench_config_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("config_operations");

    let configs = vec![
        ("localhost", OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            timeout_ms: 100,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
        }),
        ("remote", OllamaConfig {
            base_url: "https://remote.ollama.com:8443".to_string(),
            timeout_ms: 1000,
            max_retries: 5,
            initial_retry_delay_ms: 2000,
            max_retry_delay_ms: 60000,
        }),
    ];

    for (name, config) in configs {
        group.bench_with_input(BenchmarkId::new("update_config", name), &config, |b, config| {
            let mut client = OllamaClient::new();
            b.to_async(&rt).iter(|| async {
                black_box(client.update_config(config.clone()).await)
            })
        });
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = std::sync::Arc::new(OllamaClient::new());

    let mut group = c.benchmark_group("concurrent_operations");
    
    let concurrency_levels = vec![1, 5, 10, 20];

    for level in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::new("state_access_concurrent", level),
            &level,
            |b, &level| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    
                    for _ in 0..level {
                        let client_clone = std::sync::Arc::clone(&client);
                        let handle = tokio::spawn(async move {
                            client_clone.get_connection_state().await
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        black_box(handle.await.unwrap());
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark serialization performance
fn bench_serialization(c: &mut Criterion) {
    use ainote_lib::ollama_client::{ConnectionStatus, ConnectionState, HealthResponse};
    use serde_json;

    let mut group = c.benchmark_group("serialization");

    let connection_state = ConnectionState {
        status: ConnectionStatus::Connected,
        last_check: Some(chrono::Utc::now()),
        last_successful_connection: Some(chrono::Utc::now()),
        retry_count: 0,
        next_retry_at: None,
        health_info: Some(HealthResponse {
            status: "healthy".to_string(),
            version: Some("0.1.0".to_string()),
            models: Some(vec!["llama2:latest".to_string(), "codellama:latest".to_string()]),
        }),
    };

    group.bench_function("serialize_connection_state", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&connection_state).unwrap())
        })
    });

    let json_state = serde_json::to_string(&connection_state).unwrap();
    group.bench_function("deserialize_connection_state", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<ConnectionState>(&json_state).unwrap())
        })
    });

    let health_response = HealthResponse {
        status: "healthy".to_string(),
        version: Some("0.1.0".to_string()),
        models: Some(vec!["llama2:latest".to_string(), "codellama:latest".to_string()]),
    };

    group.bench_function("serialize_health_response", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&health_response).unwrap())
        })
    });

    let json_health = serde_json::to_string(&health_response).unwrap();
    group.bench_function("deserialize_health_response", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<HealthResponse>(&json_health).unwrap())
        })
    });

    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_memory_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_operations");
    
    group.bench_function("client_creation_and_cleanup", |b| {
        b.iter(|| {
            let client = black_box(OllamaClient::new());
            drop(client);
        })
    });

    group.bench_function("state_clone_operations", |b| {
        let client = OllamaClient::new();
        b.to_async(&rt).iter(|| async {
            let state = client.get_connection_state().await;
            let cloned_state = black_box(state.clone());
            drop(cloned_state);
        })
    });

    group.finish();
}

/// Benchmark error handling performance
fn bench_error_handling(c: &mut Criterion) {
    use ainote_lib::ollama_client::OllamaClientError;

    let mut group = c.benchmark_group("error_handling");

    let errors = vec![
        OllamaClientError::NetworkError {
            message: "Connection refused".to_string(),
            is_timeout: false,
        },
        OllamaClientError::HttpError {
            status_code: 404,
            message: "Not Found".to_string(),
        },
        OllamaClientError::ConfigError {
            message: "Invalid configuration".to_string(),
        },
        OllamaClientError::ServiceUnavailable {
            message: "Service temporarily unavailable".to_string(),
        },
    ];

    for (i, error) in errors.iter().enumerate() {
        group.bench_function(BenchmarkId::new("error_to_string", i), |b| {
            b.iter(|| {
                black_box(error.to_string())
            })
        });

        group.bench_function(BenchmarkId::new("error_serialize", i), |b| {
            b.iter(|| {
                black_box(serde_json::to_string(error).unwrap())
            })
        });
    }

    group.finish();
}

/// Configuration for benchmarks
fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(2))
}

criterion_group!(
    name = benches;
    config = criterion_config();
    targets = 
        bench_client_creation,
        bench_state_access,
        bench_config_operations,
        bench_concurrent_operations,
        bench_serialization,
        bench_memory_operations,
        bench_error_handling
);

criterion_main!(benches);