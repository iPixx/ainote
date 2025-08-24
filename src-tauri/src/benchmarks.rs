// Performance benchmarking for embedding models and AI operations
// This module provides comprehensive benchmarking for Ollama integration

use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::ollama_client::{OllamaClient, OllamaConfig};
use crate::performance::PerformanceTracker;

/// Benchmark configuration for embedding model testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_rounds: usize,
    pub timeout_ms: u64,
    pub memory_check_interval_ms: u64,
    pub performance_baseline_targets: PerformanceTargets,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 10,
            warmup_rounds: 3,
            timeout_ms: 30000, // 30 seconds max for each benchmark
            memory_check_interval_ms: 1000, // Check memory every second
            performance_baseline_targets: PerformanceTargets::default(),
        }
    }
}

/// Performance targets for baseline establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTargets {
    pub model_verification_max_ms: u64,
    pub embedding_generation_max_ms: u64,
    pub memory_usage_max_mb: u64,
    pub download_progress_interval_max_ms: u64,
    pub health_check_max_ms: u64,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            model_verification_max_ms: 5000,   // <5 seconds per requirement
            embedding_generation_max_ms: 100,  // <100ms per requirement  
            memory_usage_max_mb: 200,          // <200MB per requirement
            download_progress_interval_max_ms: 500,  // 500ms update interval
            health_check_max_ms: 100,          // <100ms per requirement
        }
    }
}

/// Results from a benchmark operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub operation_name: String,
    pub iterations: usize,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub median_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub success_rate: f64,
    pub memory_usage_mb: Vec<f64>,
    pub baseline_met: bool,
    pub target_duration_ms: u64,
    pub regression_detected: bool,
}

/// Memory usage measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMeasurement {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub used_memory_mb: f64,
    pub peak_memory_mb: f64,
    pub operation: String,
}

/// Comprehensive benchmark suite for embedding model operations
#[derive(Debug)]
pub struct EmbeddingBenchmarks {
    client: OllamaClient,
    config: BenchmarkConfig,
    baseline_results: HashMap<String, BenchmarkResult>,
    memory_measurements: Vec<MemoryMeasurement>,
}

impl EmbeddingBenchmarks {
    /// Create a new benchmarking suite
    pub fn new(ollama_config: OllamaConfig, benchmark_config: BenchmarkConfig) -> Self {
        let client = OllamaClient::with_config(ollama_config);
        
        Self {
            client,
            config: benchmark_config,
            baseline_results: HashMap::new(),
            memory_measurements: Vec::new(),
        }
    }

    /// Run comprehensive benchmark suite for embedding models
    pub async fn run_comprehensive_benchmarks(&mut self) -> Result<Vec<BenchmarkResult>, String> {
        let mut results = Vec::new();

        // 1. Health check performance benchmark
        let health_result = self.benchmark_health_checks().await?;
        results.push(health_result);

        // 2. Model verification benchmark
        let verification_result = self.benchmark_model_verification().await?;
        results.push(verification_result);

        // 3. Model availability detection benchmark
        let availability_result = self.benchmark_model_availability().await?;
        results.push(availability_result);

        // 4. Connection state management benchmark
        let state_result = self.benchmark_connection_state().await?;
        results.push(state_result);

        // 5. Memory usage during operations
        let memory_result = self.benchmark_memory_usage().await?;
        results.push(memory_result);

        // 6. Concurrent access benchmark
        let concurrency_result = self.benchmark_concurrent_access().await?;
        results.push(concurrency_result);

        // Store as baseline for future regression detection
        for result in &results {
            self.baseline_results.insert(result.operation_name.clone(), result.clone());
        }

        Ok(results)
    }

    /// Benchmark health check operations
    pub async fn benchmark_health_checks(&mut self) -> Result<BenchmarkResult, String> {
        let operation_name = "health_check";
        let tracker = PerformanceTracker::start(operation_name);
        
        self.warmup_client().await?;
        
        let mut durations = Vec::new();
        let mut successes = 0;
        let memory_start = self.get_memory_usage();

        for i in 0..self.config.iterations {
            let start = Instant::now();
            
            match self.client.check_health().await {
                Ok(_) => {
                    successes += 1;
                    let duration = start.elapsed();
                    durations.push(duration.as_secs_f64() * 1000.0);
                    
                    // Log memory usage periodically
                    if i % 5 == 0 {
                        self.record_memory_measurement(operation_name).await;
                    }
                }
                Err(e) => {
                    eprintln!("Health check {} failed: {:?}", i + 1, e);
                }
            }
            
            // Small delay between iterations
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let memory_end = self.get_memory_usage();
        let memory_usage = vec![memory_start, memory_end];

        tracker.finish();
        
        if durations.is_empty() {
            return Err("No successful health checks completed".to_string());
        }

        Ok(self.create_benchmark_result(
            operation_name,
            durations,
            successes,
            memory_usage,
            self.config.performance_baseline_targets.health_check_max_ms,
        ))
    }

    /// Benchmark model verification operations
    pub async fn benchmark_model_verification(&mut self) -> Result<BenchmarkResult, String> {
        let operation_name = "model_verification";
        let tracker = PerformanceTracker::start(operation_name);
        
        let test_models = vec![
            "nomic-embed-text",
            "mxbai-embed-large",
            "non-existent-model",
            "llama2", // Should be incompatible
        ];

        let mut durations = Vec::new();
        let mut successes = 0;
        let memory_start = self.get_memory_usage();

        for _ in 0..self.config.iterations {
            for model in &test_models {
                let start = Instant::now();
                
                match self.client.verify_model(model).await {
                    Ok(result) => {
                        successes += 1;
                        let duration = start.elapsed();
                        durations.push(duration.as_secs_f64() * 1000.0);
                        
                        // Verify the verification time meets requirements
                        assert!(
                            result.verification_time_ms <= self.config.performance_baseline_targets.model_verification_max_ms,
                            "Model verification for '{}' took {}ms, exceeds {}ms target",
                            model, result.verification_time_ms, self.config.performance_baseline_targets.model_verification_max_ms
                        );
                    }
                    Err(e) => {
                        eprintln!("Model verification for '{}' failed: {:?}", model, e);
                    }
                }
            }
            
            // Record memory usage periodically
            if successes % 10 == 0 {
                self.record_memory_measurement(operation_name).await;
            }
        }

        let memory_end = self.get_memory_usage();
        let memory_usage = vec![memory_start, memory_end];

        tracker.finish();

        if durations.is_empty() {
            return Err("No successful model verifications completed".to_string());
        }

        Ok(self.create_benchmark_result(
            operation_name,
            durations,
            successes,
            memory_usage,
            self.config.performance_baseline_targets.model_verification_max_ms,
        ))
    }

    /// Benchmark model availability detection
    pub async fn benchmark_model_availability(&mut self) -> Result<BenchmarkResult, String> {
        let operation_name = "model_availability";
        let tracker = PerformanceTracker::start(operation_name);
        
        let mut durations = Vec::new();
        let mut successes = 0;
        let memory_start = self.get_memory_usage();

        for i in 0..self.config.iterations {
            let start = Instant::now();
            
            match self.client.get_available_models().await {
                Ok(_models) => {
                    successes += 1;
                    let duration = start.elapsed();
                    durations.push(duration.as_secs_f64() * 1000.0);
                }
                Err(e) => {
                    eprintln!("Model availability check {} failed: {:?}", i + 1, e);
                }
            }
            
            // Record memory usage periodically
            if i % 3 == 0 {
                self.record_memory_measurement(operation_name).await;
            }
        }

        let memory_end = self.get_memory_usage();
        let memory_usage = vec![memory_start, memory_end];

        tracker.finish();

        if durations.is_empty() {
            return Err("No successful model availability checks completed".to_string());
        }

        Ok(self.create_benchmark_result(
            operation_name,
            durations,
            successes,
            memory_usage,
            self.config.performance_baseline_targets.model_verification_max_ms,
        ))
    }

    /// Benchmark connection state management
    pub async fn benchmark_connection_state(&mut self) -> Result<BenchmarkResult, String> {
        let operation_name = "connection_state";
        let tracker = PerformanceTracker::start(operation_name);
        
        let mut durations = Vec::new();
        let memory_start = self.get_memory_usage();

        for i in 0..self.config.iterations * 5 {  // More iterations for fast operation
            let start = Instant::now();
            
            let _state = self.client.get_connection_state().await;
            let duration = start.elapsed();
            durations.push(duration.as_secs_f64() * 1000.0);
            
            // Record memory usage periodically
            if i % 20 == 0 {
                self.record_memory_measurement(operation_name).await;
            }
        }

        let memory_end = self.get_memory_usage();
        let memory_usage = vec![memory_start, memory_end];

        tracker.finish();

        let duration_len = durations.len();
        Ok(self.create_benchmark_result(
            operation_name,
            durations,
            duration_len, // All should succeed
            memory_usage,
            1, // Should be <1ms
        ))
    }

    /// Benchmark memory usage during sustained operations
    pub async fn benchmark_memory_usage(&mut self) -> Result<BenchmarkResult, String> {
        let operation_name = "memory_usage";
        let tracker = PerformanceTracker::start(operation_name);
        
        let mut memory_samples = Vec::new();
        let start_memory = self.get_memory_usage();
        memory_samples.push(start_memory);

        // Perform sustained operations while monitoring memory
        for batch in 0..10 {
            // Batch of operations
            for _ in 0..5 {
                let _ = self.client.check_health().await;
                let _ = self.client.get_available_models().await;
                let _ = self.client.verify_model("nomic-embed-text").await;
            }
            
            // Record memory after each batch
            let current_memory = self.get_memory_usage();
            memory_samples.push(current_memory);
            
            self.record_memory_measurement(&format!("{}_batch_{}", operation_name, batch)).await;
            
            // Brief pause between batches
            tokio::time::sleep(Duration::from_millis(self.config.memory_check_interval_ms)).await;
        }

        let final_memory = self.get_memory_usage();
        memory_samples.push(final_memory);

        tracker.finish();

        // Create pseudo-duration metrics (memory benchmark measures space, not time)
        let memory_growth = final_memory - start_memory;
        let durations = vec![memory_growth]; // Use memory growth as "duration" for analysis

        Ok(self.create_benchmark_result(
            operation_name,
            durations,
            1, // One successful measurement
            memory_samples,
            self.config.performance_baseline_targets.memory_usage_max_mb,
        ))
    }

    /// Benchmark concurrent access performance
    pub async fn benchmark_concurrent_access(&mut self) -> Result<BenchmarkResult, String> {
        let operation_name = "concurrent_access";
        let tracker = PerformanceTracker::start(operation_name);
        
        let concurrency_level = 5;
        let operations_per_task = self.config.iterations / concurrency_level;
        
        let client = std::sync::Arc::new(self.client.clone());
        let mut handles = Vec::new();
        let start_time = Instant::now();
        let memory_start = self.get_memory_usage();

        // Launch concurrent tasks
        for task_id in 0..concurrency_level {
            let client_clone = std::sync::Arc::clone(&client);
            let handle = tokio::spawn(async move {
                let mut task_durations = Vec::new();
                let mut task_successes = 0;

                for i in 0..operations_per_task {
                    let op_start = Instant::now();
                    
                    match client_clone.check_health().await {
                        Ok(_) => {
                            task_successes += 1;
                            let duration = op_start.elapsed();
                            task_durations.push(duration.as_secs_f64() * 1000.0);
                        }
                        Err(e) => {
                            eprintln!("Concurrent task {} operation {} failed: {:?}", task_id, i, e);
                        }
                    }
                    
                    // Small delay to avoid overwhelming
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }

                (task_durations, task_successes)
            });
            handles.push(handle);
        }

        // Collect results from all tasks
        let mut all_durations = Vec::new();
        let mut total_successes = 0;

        for handle in handles {
            let (task_durations, task_successes) = handle.await
                .map_err(|e| format!("Concurrent task failed: {:?}", e))?;
            
            all_durations.extend(task_durations);
            total_successes += task_successes;
        }

        let total_time = start_time.elapsed();
        let memory_end = self.get_memory_usage();
        let memory_usage = vec![memory_start, memory_end];

        tracker.finish();

        eprintln!("Concurrent benchmark: {} operations completed in {:?}", 
                 total_successes, total_time);

        if all_durations.is_empty() {
            return Err("No concurrent operations completed successfully".to_string());
        }

        Ok(self.create_benchmark_result(
            operation_name,
            all_durations,
            total_successes,
            memory_usage,
            self.config.performance_baseline_targets.health_check_max_ms * 2, // Allow more time for concurrent access
        ))
    }

    /// Warm up the client with a few operations
    async fn warmup_client(&self) -> Result<(), String> {
        for _ in 0..self.config.warmup_rounds {
            let _ = self.client.check_health().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    /// Create a benchmark result from collected data
    fn create_benchmark_result(
        &self,
        operation_name: &str,
        mut durations: Vec<f64>,
        successes: usize,
        memory_usage: Vec<f64>,
        target_duration_ms: u64,
    ) -> BenchmarkResult {
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let min_duration = durations.first().copied().unwrap_or(0.0);
        let max_duration = durations.last().copied().unwrap_or(0.0);
        let avg_duration = durations.iter().sum::<f64>() / durations.len() as f64;
        let median_duration = durations[durations.len() / 2];
        let p95_duration = durations[durations.len() * 95 / 100];
        
        let success_rate = successes as f64 / self.config.iterations as f64;
        let baseline_met = avg_duration <= target_duration_ms as f64;
        
        // Check for regression against baseline
        let regression_detected = if let Some(baseline) = self.baseline_results.get(operation_name) {
            avg_duration > baseline.avg_duration_ms * 1.2 // 20% regression threshold
        } else {
            false
        };

        BenchmarkResult {
            operation_name: operation_name.to_string(),
            iterations: self.config.iterations,
            min_duration_ms: min_duration,
            max_duration_ms: max_duration,
            avg_duration_ms: avg_duration,
            median_duration_ms: median_duration,
            p95_duration_ms: p95_duration,
            success_rate,
            memory_usage_mb: memory_usage,
            baseline_met,
            target_duration_ms,
            regression_detected,
        }
    }

    /// Record memory measurement at current point
    async fn record_memory_measurement(&mut self, operation: &str) {
        let memory_mb = self.get_memory_usage();
        let peak_memory = self.memory_measurements
            .iter()
            .map(|m| m.used_memory_mb)
            .fold(memory_mb, f64::max);

        let measurement = MemoryMeasurement {
            timestamp: chrono::Utc::now(),
            used_memory_mb: memory_mb,
            peak_memory_mb: peak_memory,
            operation: operation.to_string(),
        };

        self.memory_measurements.push(measurement);
    }

    /// Get current memory usage (simplified implementation)
    fn get_memory_usage(&self) -> f64 {
        // In a real implementation, this would use system APIs to get actual memory usage
        // For now, we'll use a simple approximation based on object sizes
        let client_size = std::mem::size_of_val(&self.client) as f64;
        let config_size = std::mem::size_of_val(&self.config) as f64;
        let measurements_size = self.memory_measurements.len() as f64 * 100.0; // Rough estimate
        
        (client_size + config_size + measurements_size) / (1024.0 * 1024.0) // Convert to MB
    }

    /// Check for performance regressions against established baselines
    pub fn detect_performance_regressions(&self, current_results: &[BenchmarkResult]) -> Vec<String> {
        let mut regressions = Vec::new();

        for result in current_results {
            if result.regression_detected {
                regressions.push(format!(
                    "Regression detected in {}: current avg {:.2}ms vs baseline {:.2}ms",
                    result.operation_name,
                    result.avg_duration_ms,
                    self.baseline_results.get(&result.operation_name)
                        .map(|b| b.avg_duration_ms)
                        .unwrap_or(0.0)
                ));
            }

            if !result.baseline_met {
                regressions.push(format!(
                    "Baseline not met for {}: {:.2}ms > {}ms target",
                    result.operation_name,
                    result.avg_duration_ms,
                    result.target_duration_ms
                ));
            }

            if result.success_rate < 0.95 {
                regressions.push(format!(
                    "Low success rate for {}: {:.1}% < 95% target",
                    result.operation_name,
                    result.success_rate * 100.0
                ));
            }
        }

        regressions
    }

    /// Generate comprehensive benchmark report
    pub fn generate_report(&self, results: &[BenchmarkResult]) -> String {
        let mut report = String::new();
        
        report.push_str("=== EMBEDDING MODEL PERFORMANCE BENCHMARK REPORT ===\n\n");
        
        // Overall summary
        let total_operations: usize = results.iter().map(|r| r.iterations).sum();
        let avg_success_rate = results.iter().map(|r| r.success_rate).sum::<f64>() / results.len() as f64;
        let baselines_met = results.iter().filter(|r| r.baseline_met).count();
        
        report.push_str(&format!("Total Operations Tested: {}\n", total_operations));
        report.push_str(&format!("Overall Success Rate: {:.1}%\n", avg_success_rate * 100.0));
        report.push_str(&format!("Baselines Met: {}/{}\n", baselines_met, results.len()));
        report.push('\n');

        // Individual operation results
        for result in results {
            report.push_str(&format!("--- {} ---\n", result.operation_name.to_uppercase()));
            report.push_str(&format!("  Iterations: {}\n", result.iterations));
            report.push_str(&format!("  Success Rate: {:.1}%\n", result.success_rate * 100.0));
            report.push_str(&format!("  Min Duration: {:.2}ms\n", result.min_duration_ms));
            report.push_str(&format!("  Average Duration: {:.2}ms\n", result.avg_duration_ms));
            report.push_str(&format!("  Median Duration: {:.2}ms\n", result.median_duration_ms));
            report.push_str(&format!("  P95 Duration: {:.2}ms\n", result.p95_duration_ms));
            report.push_str(&format!("  Max Duration: {:.2}ms\n", result.max_duration_ms));
            report.push_str(&format!("  Target: {}ms\n", result.target_duration_ms));
            report.push_str(&format!("  Baseline Met: {}\n", if result.baseline_met { "✅ YES" } else { "❌ NO" }));
            report.push_str(&format!("  Regression: {}\n", if result.regression_detected { "⚠️ DETECTED" } else { "✅ NONE" }));
            
            if !result.memory_usage_mb.is_empty() {
                let avg_memory = result.memory_usage_mb.iter().sum::<f64>() / result.memory_usage_mb.len() as f64;
                let max_memory = result.memory_usage_mb.iter().copied().fold(0.0f64, f64::max);
                report.push_str(&format!("  Memory Usage: {:.2}MB avg, {:.2}MB peak\n", avg_memory, max_memory));
            }
            
            report.push('\n');
        }

        // Performance regression summary
        let regressions = self.detect_performance_regressions(results);
        if !regressions.is_empty() {
            report.push_str("=== PERFORMANCE ISSUES DETECTED ===\n");
            for regression in &regressions {
                report.push_str(&format!("⚠️ {}\n", regression));
            }
            report.push('\n');
        } else {
            report.push_str("✅ No performance regressions detected\n\n");
        }

        // Memory usage analysis
        if !self.memory_measurements.is_empty() {
            report.push_str("=== MEMORY USAGE ANALYSIS ===\n");
            let max_memory = self.memory_measurements.iter()
                .map(|m| m.used_memory_mb)
                .fold(0.0f64, f64::max);
            let final_memory = self.memory_measurements.last()
                .map(|m| m.used_memory_mb)
                .unwrap_or(0.0);
            
            report.push_str(&format!("Peak Memory Usage: {:.2}MB\n", max_memory));
            report.push_str(&format!("Final Memory Usage: {:.2}MB\n", final_memory));
            report.push_str(&format!("Memory Target: {}MB\n", self.config.performance_baseline_targets.memory_usage_max_mb));
            
            let memory_ok = max_memory <= self.config.performance_baseline_targets.memory_usage_max_mb as f64;
            report.push_str(&format!("Memory Target Met: {}\n", if memory_ok { "✅ YES" } else { "❌ NO" }));
            report.push('\n');
        }

        // Recommendations
        report.push_str("=== RECOMMENDATIONS ===\n");
        if baselines_met == results.len() && regressions.is_empty() {
            report.push_str("✅ All performance targets met - ready for production\n");
        } else {
            report.push_str("⚠️ Performance issues detected - review before deployment:\n");
            for regression in &regressions {
                report.push_str(&format!("  - {}\n", regression));
            }
        }

        report
    }

    /// Get memory measurements for detailed analysis
    pub fn get_memory_measurements(&self) -> &[MemoryMeasurement] {
        &self.memory_measurements
    }

    /// Get baseline results for comparison
    pub fn get_baseline_results(&self) -> &HashMap<String, BenchmarkResult> {
        &self.baseline_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config_default() {
        let config = BenchmarkConfig::default();
        
        assert_eq!(config.iterations, 10);
        assert_eq!(config.warmup_rounds, 3);
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.memory_check_interval_ms, 1000);
    }

    #[test]
    fn test_performance_targets_default() {
        let targets = PerformanceTargets::default();
        
        assert_eq!(targets.model_verification_max_ms, 5000);
        assert_eq!(targets.embedding_generation_max_ms, 100);
        assert_eq!(targets.memory_usage_max_mb, 200);
        assert_eq!(targets.download_progress_interval_max_ms, 500);
        assert_eq!(targets.health_check_max_ms, 100);
    }

    #[test]
    fn test_benchmark_result_creation() {
        let benchmarks = EmbeddingBenchmarks::new(
            OllamaConfig::default(),
            BenchmarkConfig::default()
        );
        
        let durations = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = benchmarks.create_benchmark_result(
            "test_operation",
            durations,
            5,
            vec![1.0, 2.0],
            25
        );
        
        assert_eq!(result.operation_name, "test_operation");
        assert_eq!(result.iterations, 10);
        assert_eq!(result.min_duration_ms, 10.0);
        assert_eq!(result.max_duration_ms, 50.0);
        assert_eq!(result.avg_duration_ms, 30.0);
        assert_eq!(result.median_duration_ms, 30.0);
        assert_eq!(result.success_rate, 0.5); // 5 successes out of 10 iterations
        assert!(!result.baseline_met); // 30ms > 25ms target
    }

    #[test]
    fn test_memory_measurement_creation() {
        let measurement = MemoryMeasurement {
            timestamp: chrono::Utc::now(),
            used_memory_mb: 50.0,
            peak_memory_mb: 75.0,
            operation: "test_operation".to_string(),
        };
        
        assert_eq!(measurement.used_memory_mb, 50.0);
        assert_eq!(measurement.peak_memory_mb, 75.0);
        assert_eq!(measurement.operation, "test_operation");
    }

    #[test]
    fn test_performance_regression_detection() {
        let config = BenchmarkConfig::default();
        let mut benchmarks = EmbeddingBenchmarks::new(
            OllamaConfig::default(),
            config
        );
        
        // Set up baseline
        let baseline = BenchmarkResult {
            operation_name: "test_op".to_string(),
            iterations: 10,
            min_duration_ms: 5.0,
            max_duration_ms: 15.0,
            avg_duration_ms: 10.0,
            median_duration_ms: 10.0,
            p95_duration_ms: 14.0,
            success_rate: 1.0,
            memory_usage_mb: vec![1.0],
            baseline_met: true,
            target_duration_ms: 20,
            regression_detected: false,
        };
        
        benchmarks.baseline_results.insert("test_op".to_string(), baseline);
        
        // Test regression detection
        let current_result = BenchmarkResult {
            operation_name: "test_op".to_string(),
            iterations: 10,
            min_duration_ms: 8.0,
            max_duration_ms: 25.0,
            avg_duration_ms: 15.0, // 50% slower than baseline
            median_duration_ms: 15.0,
            p95_duration_ms: 22.0,
            success_rate: 1.0,
            memory_usage_mb: vec![1.5],
            baseline_met: true,
            target_duration_ms: 20,
            regression_detected: true,
        };
        
        let regressions = benchmarks.detect_performance_regressions(&[current_result]);
        assert!(!regressions.is_empty());
        assert!(regressions[0].contains("Regression detected"));
    }

    #[test]
    fn test_benchmark_report_generation() {
        let config = BenchmarkConfig::default();
        let benchmarks = EmbeddingBenchmarks::new(
            OllamaConfig::default(),
            config
        );
        
        let results = vec![
            BenchmarkResult {
                operation_name: "test_operation".to_string(),
                iterations: 10,
                min_duration_ms: 5.0,
                max_duration_ms: 15.0,
                avg_duration_ms: 10.0,
                median_duration_ms: 10.0,
                p95_duration_ms: 14.0,
                success_rate: 1.0,
                memory_usage_mb: vec![1.0, 1.5],
                baseline_met: true,
                target_duration_ms: 20,
                regression_detected: false,
            }
        ];
        
        let report = benchmarks.generate_report(&results);
        
        assert!(report.contains("EMBEDDING MODEL PERFORMANCE BENCHMARK REPORT"));
        assert!(report.contains("TEST_OPERATION"));
        assert!(report.contains("Success Rate: 100.0%"));
        assert!(report.contains("Average Duration: 10.00ms"));
        assert!(report.contains("✅ YES")); // Baseline met
    }

    #[tokio::test]
    async fn test_benchmarks_creation() {
        let ollama_config = OllamaConfig::default();
        let benchmark_config = BenchmarkConfig::default();
        
        let benchmarks = EmbeddingBenchmarks::new(ollama_config, benchmark_config);
        
        // Verify proper initialization
        assert_eq!(benchmarks.config.iterations, 10);
        assert!(benchmarks.baseline_results.is_empty());
        assert!(benchmarks.memory_measurements.is_empty());
    }

    #[test]
    fn test_memory_usage_calculation() {
        let config = BenchmarkConfig::default();
        let benchmarks = EmbeddingBenchmarks::new(
            OllamaConfig::default(),
            config
        );
        
        let memory_usage = benchmarks.get_memory_usage();
        
        // Should return a reasonable memory estimate
        assert!(memory_usage > 0.0);
        assert!(memory_usage < 100.0); // Should be well under 100MB for basic client
    }
}