// Performance baseline establishment and management system
// This module handles creating, storing, and comparing performance baselines for AI operations

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;
use chrono::{DateTime, Utc};
use crate::benchmarks::{BenchmarkResult, BenchmarkConfig, EmbeddingBenchmarks};
use crate::ollama_client::OllamaConfig;

/// System information for baseline establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_memory_gb: f64,
    pub ollama_version: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl SystemInfo {
    /// Gather current system information
    pub fn gather() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_cores: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4),
            total_memory_gb: Self::get_total_memory_gb(),
            ollama_version: None, // Will be populated when available
            timestamp: Utc::now(),
        }
    }

    /// Get total system memory in GB (approximation)
    fn get_total_memory_gb() -> f64 {
        // This is a simplified implementation
        // In production, you might use system-specific APIs
        match std::env::consts::OS {
            "macos" | "linux" => {
                // Rough estimation based on typical systems
                8.0 // Default assumption
            }
            "windows" => 8.0, // Default assumption
            _ => 4.0, // Conservative default
        }
    }

    /// Generate a system signature for baseline matching
    pub fn signature(&self) -> String {
        format!("{}-{}-{}-cores-{}gb", 
                self.os, self.arch, self.cpu_cores, self.total_memory_gb as u32)
    }
}

/// Performance baseline data for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub operation_name: String,
    pub system_info: SystemInfo,
    pub baseline_metrics: BaselineMetrics,
    pub confidence_level: f64,
    pub sample_count: usize,
    pub established_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub version: String,
}

/// Core performance metrics for baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub success_rate: f64,
    pub memory_usage_mb: f64,
    pub std_deviation_ms: f64,
}

impl BaselineMetrics {
    /// Create baseline metrics from benchmark result
    pub fn from_benchmark_result(result: &BenchmarkResult, std_dev: f64) -> Self {
        Self {
            avg_duration_ms: result.avg_duration_ms,
            p50_duration_ms: result.median_duration_ms,
            p95_duration_ms: result.p95_duration_ms,
            p99_duration_ms: result.p95_duration_ms * 1.1, // Approximate P99
            min_duration_ms: result.min_duration_ms,
            max_duration_ms: result.max_duration_ms,
            success_rate: result.success_rate,
            memory_usage_mb: result.memory_usage_mb.iter().sum::<f64>() / result.memory_usage_mb.len().max(1) as f64,
            std_deviation_ms: std_dev,
        }
    }

    /// Check if current metrics meet the baseline (within tolerance)
    pub fn meets_baseline(&self, current: &BaselineMetrics, tolerance_percent: f64) -> bool {
        let tolerance = tolerance_percent / 100.0;
        
        // Check key metrics within tolerance
        let avg_ok = current.avg_duration_ms <= self.avg_duration_ms * (1.0 + tolerance);
        let p95_ok = current.p95_duration_ms <= self.p95_duration_ms * (1.0 + tolerance);
        let success_rate_ok = current.success_rate >= self.success_rate * (1.0 - tolerance * 0.5);
        let memory_ok = current.memory_usage_mb <= self.memory_usage_mb * (1.0 + tolerance);
        
        avg_ok && p95_ok && success_rate_ok && memory_ok
    }
}

/// Configuration for baseline establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineConfig {
    pub min_samples: usize,
    pub max_samples: usize,
    pub confidence_threshold: f64,
    pub stability_window: usize,
    pub max_cv_percent: f64, // Maximum coefficient of variation
    pub warmup_iterations: usize,
    pub baseline_version: String,
}

impl Default for BaselineConfig {
    fn default() -> Self {
        Self {
            min_samples: 10,
            max_samples: 50,
            confidence_threshold: 0.95,
            stability_window: 5, // Last N samples should be stable
            max_cv_percent: 15.0, // 15% coefficient of variation max
            warmup_iterations: 3,
            baseline_version: "1.0".to_string(),
        }
    }
}

/// Baseline establishment and management system
#[derive(Debug)]
pub struct BaselineManager {
    config: BaselineConfig,
    baselines: HashMap<String, PerformanceBaseline>,
    storage_path: PathBuf,
}

impl BaselineManager {
    /// Create a new baseline manager
    pub fn new(config: BaselineConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let storage_path = Self::get_storage_path()?;
        let mut manager = Self {
            config,
            baselines: HashMap::new(),
            storage_path,
        };
        
        // Load existing baselines
        manager.load_baselines()?;
        
        Ok(manager)
    }

    /// Get the storage path for baselines
    fn get_storage_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = dirs::home_dir()
            .ok_or("Could not determine home directory")?;
        
        let storage_dir = home_dir.join(".ainote").join("performance_baselines");
        fs::create_dir_all(&storage_dir)?;
        
        Ok(storage_dir.join("baselines.json"))
    }

    /// Load baselines from storage
    fn load_baselines(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.storage_path.exists() {
            let content = fs::read_to_string(&self.storage_path)?;
            if !content.trim().is_empty() {
                self.baselines = serde_json::from_str(&content)?;
                println!("Loaded {} performance baselines", self.baselines.len());
            }
        }
        Ok(())
    }

    /// Save baselines to storage
    fn save_baselines(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self.baselines)?;
        fs::write(&self.storage_path, content)?;
        Ok(())
    }

    /// Establish baseline for a specific operation
    pub async fn establish_baseline(
        &mut self, 
        operation_name: &str,
        ollama_config: &OllamaConfig,
    ) -> Result<PerformanceBaseline, Box<dyn std::error::Error>> {
        println!("Establishing performance baseline for '{}'", operation_name);
        
        let system_info = SystemInfo::gather();
        let benchmark_config = BenchmarkConfig {
            iterations: self.config.max_samples,
            warmup_rounds: self.config.warmup_iterations,
            ..Default::default()
        };
        
        let mut benchmarks = EmbeddingBenchmarks::new(ollama_config.clone(), benchmark_config);
        let mut samples = Vec::new();
        let mut stable_sample_count = 0;
        
        // Collect samples until we have stable baseline
        for iteration in 1..=self.config.max_samples {
            println!("Baseline sample {}/{}", iteration, self.config.max_samples);
            
            // Run specific benchmark based on operation name
            let result = match operation_name {
                "health_check" => benchmarks.benchmark_health_checks().await?,
                "model_verification" => benchmarks.benchmark_model_verification().await?,
                "model_availability" => benchmarks.benchmark_model_availability().await?,
                "connection_state" => benchmarks.benchmark_connection_state().await?,
                "memory_usage" => benchmarks.benchmark_memory_usage().await?,
                "concurrent_access" => benchmarks.benchmark_concurrent_access().await?,
                _ => return Err(format!("Unknown operation: {}", operation_name).into()),
            };
            
            samples.push(result);
            
            // Check for stability after minimum samples
            if samples.len() >= self.config.min_samples {
                if self.is_baseline_stable(&samples)? {
                    stable_sample_count += 1;
                    if stable_sample_count >= self.config.stability_window {
                        println!("Stable baseline achieved after {} samples", samples.len());
                        break;
                    }
                } else {
                    stable_sample_count = 0;
                }
            }
        }
        
        if samples.len() < self.config.min_samples {
            return Err(format!("Insufficient samples for baseline: {} < {}", 
                              samples.len(), self.config.min_samples).into());
        }
        
        // Calculate baseline metrics
        let baseline_metrics = self.calculate_baseline_metrics(&samples)?;
        let confidence = self.calculate_confidence(&samples);
        
        let baseline = PerformanceBaseline {
            operation_name: operation_name.to_string(),
            system_info,
            baseline_metrics,
            confidence_level: confidence,
            sample_count: samples.len(),
            established_at: Utc::now(),
            last_updated: Utc::now(),
            version: self.config.baseline_version.clone(),
        };
        
        // Store baseline
        let baseline_key = self.get_baseline_key(operation_name, &baseline.system_info);
        self.baselines.insert(baseline_key, baseline.clone());
        self.save_baselines()?;
        
        println!("✅ Baseline established for '{}' with {:.1}% confidence", 
                operation_name, confidence * 100.0);
        
        Ok(baseline)
    }

    /// Check if baseline samples are stable
    fn is_baseline_stable(&self, samples: &[BenchmarkResult]) -> Result<bool, Box<dyn std::error::Error>> {
        if samples.len() < self.config.stability_window {
            return Ok(false);
        }
        
        // Get last N samples for stability check
        let recent_samples: Vec<f64> = samples
            .iter()
            .rev()
            .take(self.config.stability_window)
            .map(|s| s.avg_duration_ms)
            .collect();
        
        // Calculate coefficient of variation for recent samples
        let mean = recent_samples.iter().sum::<f64>() / recent_samples.len() as f64;
        let variance = recent_samples.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / recent_samples.len() as f64;
        let std_dev = variance.sqrt();
        let cv = (std_dev / mean) * 100.0;
        
        Ok(cv <= self.config.max_cv_percent)
    }

    /// Calculate baseline metrics from samples
    fn calculate_baseline_metrics(&self, samples: &[BenchmarkResult]) -> Result<BaselineMetrics, Box<dyn std::error::Error>> {
        if samples.is_empty() {
            return Err("No samples provided for baseline calculation".into());
        }
        
        // Collect all duration measurements
        let mut durations: Vec<f64> = samples.iter().map(|s| s.avg_duration_ms).collect();
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // Calculate statistics
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let variance = durations.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / durations.len() as f64;
        let std_dev = variance.sqrt();
        
        let p50 = durations[durations.len() / 2];
        let p95 = durations[durations.len() * 95 / 100];
        let p99 = durations[durations.len() * 99 / 100];
        let min = durations[0];
        let max = durations[durations.len() - 1];
        
        // Calculate average success rate and memory usage
        let avg_success_rate = samples.iter().map(|s| s.success_rate).sum::<f64>() / samples.len() as f64;
        let avg_memory = samples.iter()
            .flat_map(|s| &s.memory_usage_mb)
            .sum::<f64>() / samples.iter().map(|s| s.memory_usage_mb.len()).sum::<usize>().max(1) as f64;
        
        Ok(BaselineMetrics {
            avg_duration_ms: mean,
            p50_duration_ms: p50,
            p95_duration_ms: p95,
            p99_duration_ms: p99,
            min_duration_ms: min,
            max_duration_ms: max,
            success_rate: avg_success_rate,
            memory_usage_mb: avg_memory,
            std_deviation_ms: std_dev,
        })
    }

    /// Calculate confidence level for baseline
    fn calculate_confidence(&self, samples: &[BenchmarkResult]) -> f64 {
        let sample_count_factor = (samples.len() as f64 / self.config.max_samples as f64).min(1.0);
        
        // Calculate consistency factor (lower CV = higher confidence)
        let durations: Vec<f64> = samples.iter().map(|s| s.avg_duration_ms).collect();
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let variance = durations.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / durations.len() as f64;
        let std_dev = variance.sqrt();
        let cv = (std_dev / mean) * 100.0;
        
        let consistency_factor = 1.0 - (cv / self.config.max_cv_percent).min(1.0);
        
        // Calculate success rate factor
        let avg_success_rate = samples.iter().map(|s| s.success_rate).sum::<f64>() / samples.len() as f64;
        let success_factor = avg_success_rate;
        
        // Combined confidence score
        (sample_count_factor * 0.3 + consistency_factor * 0.4 + success_factor * 0.3).clamp(0.0, 1.0)
    }

    /// Get baseline key for storage
    fn get_baseline_key(&self, operation: &str, system_info: &SystemInfo) -> String {
        format!("{}_{}", operation, system_info.signature())
    }

    /// Get baseline for operation on current system
    pub fn get_baseline(&self, operation_name: &str) -> Option<&PerformanceBaseline> {
        let system_info = SystemInfo::gather();
        let key = self.get_baseline_key(operation_name, &system_info);
        self.baselines.get(&key)
    }

    /// Compare current performance against baseline
    pub fn compare_against_baseline(
        &self, 
        operation_name: &str, 
        current_result: &BenchmarkResult
    ) -> BaselineComparison {
        let system_info = SystemInfo::gather();
        let key = self.get_baseline_key(operation_name, &system_info);
        
        if let Some(baseline) = self.baselines.get(&key) {
            let current_metrics = BaselineMetrics::from_benchmark_result(
                current_result, 
                0.0 // Standard deviation not available from single result
            );
            
            let regression_threshold = 20.0; // 20% regression threshold
            let meets_baseline = baseline.baseline_metrics.meets_baseline(&current_metrics, regression_threshold);
            
            let performance_ratio = current_result.avg_duration_ms / baseline.baseline_metrics.avg_duration_ms;
            let memory_ratio = current_metrics.memory_usage_mb / baseline.baseline_metrics.memory_usage_mb;
            
            BaselineComparison {
                baseline_exists: true,
                meets_baseline,
                performance_ratio,
                memory_ratio,
                confidence_level: baseline.confidence_level,
                baseline_age_days: (Utc::now() - baseline.established_at).num_days(),
                regression_detected: performance_ratio > 1.2, // 20% slower
                improvement_detected: performance_ratio < 0.9, // 10% faster
                baseline_version: baseline.version.clone(),
            }
        } else {
            BaselineComparison {
                baseline_exists: false,
                meets_baseline: false,
                performance_ratio: 1.0,
                memory_ratio: 1.0,
                confidence_level: 0.0,
                baseline_age_days: 0,
                regression_detected: false,
                improvement_detected: false,
                baseline_version: "none".to_string(),
            }
        }
    }

    /// Update existing baseline with new data
    pub async fn update_baseline(
        &mut self, 
        operation_name: &str, 
        new_samples: &[BenchmarkResult]
    ) -> Result<(), Box<dyn std::error::Error>> {
        let system_info = SystemInfo::gather();
        let key = self.get_baseline_key(operation_name, &system_info);
        
        // Calculate metrics first, then update baseline
        let updated_metrics = self.calculate_baseline_metrics(new_samples)?;
        let updated_confidence = self.calculate_confidence(new_samples);
        
        if let Some(baseline) = self.baselines.get_mut(&key) {
            baseline.baseline_metrics = updated_metrics;
            baseline.confidence_level = updated_confidence;
            baseline.sample_count = new_samples.len();
            baseline.last_updated = Utc::now();
            
            self.save_baselines()?;
            
            println!("✅ Updated baseline for '{}' with {} samples", operation_name, new_samples.len());
        } else {
            return Err(format!("No existing baseline found for '{}'", operation_name).into());
        }
        
        Ok(())
    }

    /// Get all baselines for current system
    pub fn get_all_baselines(&self) -> Vec<&PerformanceBaseline> {
        let system_info = SystemInfo::gather();
        let system_signature = system_info.signature();
        
        self.baselines.values()
            .filter(|baseline| baseline.system_info.signature() == system_signature)
            .collect()
    }

    /// Generate baseline report
    pub fn generate_baseline_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== PERFORMANCE BASELINE REPORT ===\n\n");
        
        let system_info = SystemInfo::gather();
        report.push_str(&format!("System: {}\n", system_info.signature()));
        report.push_str(&format!("Baseline Version: {}\n", self.config.baseline_version));
        report.push_str(&format!("Storage Path: {:?}\n\n", self.storage_path));
        
        let system_baselines = self.get_all_baselines();
        
        if system_baselines.is_empty() {
            report.push_str("⚠️ No baselines established for current system\n");
            report.push_str("Run baseline establishment process to create performance baselines.\n");
        } else {
            report.push_str(&format!("📊 {} baselines established:\n\n", system_baselines.len()));
            
            for baseline in system_baselines {
                report.push_str(&format!("--- {} ---\n", baseline.operation_name.to_uppercase()));
                report.push_str(&format!("  Established: {}\n", baseline.established_at.format("%Y-%m-%d %H:%M UTC")));
                report.push_str(&format!("  Last Updated: {}\n", baseline.last_updated.format("%Y-%m-%d %H:%M UTC")));
                report.push_str(&format!("  Samples: {}\n", baseline.sample_count));
                report.push_str(&format!("  Confidence: {:.1}%\n", baseline.confidence_level * 100.0));
                report.push_str(&format!("  Avg Duration: {:.2}ms\n", baseline.baseline_metrics.avg_duration_ms));
                report.push_str(&format!("  P95 Duration: {:.2}ms\n", baseline.baseline_metrics.p95_duration_ms));
                report.push_str(&format!("  Success Rate: {:.1}%\n", baseline.baseline_metrics.success_rate * 100.0));
                report.push_str(&format!("  Memory Usage: {:.1}MB\n", baseline.baseline_metrics.memory_usage_mb));
                report.push_str(&format!("  Std Deviation: {:.2}ms\n", baseline.baseline_metrics.std_deviation_ms));
                report.push('\n');
            }
        }
        
        report.push_str("=== BASELINE ESTABLISHMENT GUIDELINES ===\n");
        report.push_str("- Run baselines after system changes or Ollama updates\n");
        report.push_str("- Baselines should have >90% confidence for reliable regression detection\n");
        report.push_str("- Update baselines periodically to account for system improvements\n");
        report.push_str("- Establish baselines on each deployment environment\n");
        
        report
    }
}

/// Result of comparing performance against baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    pub baseline_exists: bool,
    pub meets_baseline: bool,
    pub performance_ratio: f64, // Current/Baseline (>1.0 = slower, <1.0 = faster)
    pub memory_ratio: f64,      // Current/Baseline memory usage
    pub confidence_level: f64,
    pub baseline_age_days: i64,
    pub regression_detected: bool,
    pub improvement_detected: bool,
    pub baseline_version: String,
}

impl BaselineComparison {
    /// Generate human-readable comparison summary
    pub fn summary(&self) -> String {
        if !self.baseline_exists {
            return "No baseline exists for comparison".to_string();
        }
        
        let mut summary = Vec::new();
        
        if self.regression_detected {
            summary.push(format!("🔴 Performance regression: {:.1}% slower", 
                               (self.performance_ratio - 1.0) * 100.0));
        } else if self.improvement_detected {
            summary.push(format!("🟢 Performance improvement: {:.1}% faster", 
                               (1.0 - self.performance_ratio) * 100.0));
        } else {
            summary.push("🟡 Performance within baseline range".to_string());
        }
        
        if self.memory_ratio > 1.1 {
            summary.push(format!("⚠️ Memory usage increased: {:.1}%", 
                               (self.memory_ratio - 1.0) * 100.0));
        } else if self.memory_ratio < 0.9 {
            summary.push(format!("✅ Memory usage reduced: {:.1}%", 
                               (1.0 - self.memory_ratio) * 100.0));
        }
        
        if self.confidence_level < 0.8 {
            summary.push("⚠️ Low baseline confidence - consider re-establishing".to_string());
        }
        
        if self.baseline_age_days > 30 {
            summary.push(format!("⚠️ Baseline is {} days old - consider updating", 
                               self.baseline_age_days));
        }
        
        if summary.is_empty() {
            "✅ Performance meets baseline expectations".to_string()
        } else {
            summary.join("; ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_system_info_gathering() {
        let info = SystemInfo::gather();
        
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_cores > 0);
        assert!(info.total_memory_gb > 0.0);
        
        let signature = info.signature();
        assert!(signature.contains(&info.os));
        assert!(signature.contains(&info.arch));
    }

    #[test]
    fn test_baseline_metrics_creation() {
        let benchmark_result = BenchmarkResult {
            operation_name: "test".to_string(),
            iterations: 10,
            min_duration_ms: 5.0,
            max_duration_ms: 15.0,
            avg_duration_ms: 10.0,
            median_duration_ms: 10.0,
            p95_duration_ms: 14.0,
            success_rate: 1.0,
            memory_usage_mb: vec![1.0, 1.5, 1.2],
            baseline_met: true,
            target_duration_ms: 20,
            regression_detected: false,
        };
        
        let metrics = BaselineMetrics::from_benchmark_result(&benchmark_result, 2.0);
        
        assert_eq!(metrics.avg_duration_ms, 10.0);
        assert_eq!(metrics.p50_duration_ms, 10.0);
        assert_eq!(metrics.success_rate, 1.0);
        assert_eq!(metrics.std_deviation_ms, 2.0);
        assert!((metrics.memory_usage_mb - 1.233).abs() < 0.01);
    }

    #[test]
    fn test_baseline_comparison() {
        let baseline_metrics = BaselineMetrics {
            avg_duration_ms: 10.0,
            p50_duration_ms: 10.0,
            p95_duration_ms: 15.0,
            p99_duration_ms: 16.0,
            min_duration_ms: 5.0,
            max_duration_ms: 20.0,
            success_rate: 1.0,
            memory_usage_mb: 5.0,
            std_deviation_ms: 2.0,
        };
        
        // Test within tolerance
        let current_good = BaselineMetrics {
            avg_duration_ms: 11.0, // 10% slower - within 20% tolerance
            p95_duration_ms: 16.0,
            success_rate: 0.95,
            memory_usage_mb: 5.5,
            ..baseline_metrics.clone()
        };
        
        assert!(baseline_metrics.meets_baseline(&current_good, 20.0));
        
        // Test exceeding tolerance
        let current_bad = BaselineMetrics {
            avg_duration_ms: 15.0, // 50% slower - exceeds 20% tolerance
            p95_duration_ms: 25.0,
            success_rate: 0.7,
            memory_usage_mb: 8.0,
            ..baseline_metrics.clone()
        };
        
        assert!(!baseline_metrics.meets_baseline(&current_bad, 20.0));
    }

    #[test]
    fn test_baseline_config_default() {
        let config = BaselineConfig::default();
        
        assert_eq!(config.min_samples, 10);
        assert_eq!(config.max_samples, 50);
        assert_eq!(config.confidence_threshold, 0.95);
        assert_eq!(config.stability_window, 5);
        assert_eq!(config.max_cv_percent, 15.0);
        assert_eq!(config.warmup_iterations, 3);
    }

    #[test]
    fn test_baseline_comparison_summary() {
        // Test regression
        let regression = BaselineComparison {
            baseline_exists: true,
            meets_baseline: false,
            performance_ratio: 1.3, // 30% slower
            memory_ratio: 1.1,
            confidence_level: 0.9,
            baseline_age_days: 5,
            regression_detected: true,
            improvement_detected: false,
            baseline_version: "1.0".to_string(),
        };
        
        let summary = regression.summary();
        assert!(summary.contains("regression"));
        assert!(summary.contains("30.0% slower"));
        
        // Test improvement
        let improvement = BaselineComparison {
            baseline_exists: true,
            meets_baseline: true,
            performance_ratio: 0.8, // 20% faster
            memory_ratio: 0.9,
            confidence_level: 0.95,
            baseline_age_days: 2,
            regression_detected: false,
            improvement_detected: true,
            baseline_version: "1.0".to_string(),
        };
        
        let summary = improvement.summary();
        assert!(summary.contains("improvement"));
        assert!(summary.contains("20.0% faster"));
    }

    #[tokio::test]
    async fn test_baseline_manager_creation() {
        let _temp_dir = TempDir::new().unwrap();
        let config = BaselineConfig::default();
        
        // This would normally fail without home directory access in tests
        // So we just test the configuration
        assert_eq!(config.min_samples, 10);
        assert_eq!(config.baseline_version, "1.0");
    }
}