// Advanced performance regression detection system
// Provides statistical analysis and automated detection of performance regressions

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::benchmarks::BenchmarkResult;
use crate::performance_baseline::PerformanceBaseline;

/// Configuration for regression detection algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionDetectionConfig {
    pub statistical_significance_level: f64,
    pub regression_threshold_percent: f64,
    pub improvement_threshold_percent: f64,
    pub min_samples_for_detection: usize,
    pub trend_analysis_window: usize,
    pub outlier_detection_enabled: bool,
    pub seasonal_adjustment_enabled: bool,
}

impl Default for RegressionDetectionConfig {
    fn default() -> Self {
        Self {
            statistical_significance_level: 0.05, // 95% confidence
            regression_threshold_percent: 20.0,   // 20% slower is regression
            improvement_threshold_percent: 10.0,  // 10% faster is improvement
            min_samples_for_detection: 5,         // Need at least 5 samples
            trend_analysis_window: 10,            // Analyze last 10 measurements
            outlier_detection_enabled: true,      // Remove outliers from analysis
            seasonal_adjustment_enabled: false,   // Adjust for time-based patterns
        }
    }
}

/// Types of performance regressions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegressionType {
    Latency,        // Response time regression
    Throughput,     // Operations per second regression
    Memory,         // Memory usage regression
    SuccessRate,    // Error rate increase
    Stability,      // Increased variability
}

/// Severity levels for regressions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegressionSeverity {
    Minor,     // <30% degradation
    Moderate,  // 30-50% degradation  
    Major,     // 50-100% degradation
    Critical,  // >100% degradation
}

/// Detailed regression detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionDetection {
    pub operation_name: String,
    pub regression_type: RegressionType,
    pub severity: RegressionSeverity,
    pub statistical_confidence: f64,
    pub baseline_value: f64,
    pub current_value: f64,
    pub change_percent: f64,
    pub detected_at: DateTime<Utc>,
    pub trend_direction: TrendDirection,
    pub recommendation: String,
}

/// Trend analysis results
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,      // Performance getting better over time
    Stable,         // Performance relatively stable
    Degrading,      // Performance getting worse over time
    Volatile,       // High variability, no clear trend
}

/// Historical performance data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDataPoint {
    pub timestamp: DateTime<Utc>,
    pub operation_name: String,
    pub duration_ms: f64,
    pub memory_usage_mb: f64,
    pub success_rate: f64,
    pub system_load: Option<f64>,
}

/// Advanced regression detection engine
#[derive(Debug)]
pub struct RegressionDetector {
    config: RegressionDetectionConfig,
    historical_data: Vec<PerformanceDataPoint>,
    baselines: HashMap<String, PerformanceBaseline>,
}

impl RegressionDetector {
    /// Create new regression detector
    pub fn new(config: RegressionDetectionConfig) -> Self {
        Self {
            config,
            historical_data: Vec::new(),
            baselines: HashMap::new(),
        }
    }

    /// Add performance measurement to historical data
    pub fn add_measurement(&mut self, measurement: PerformanceDataPoint) {
        self.historical_data.push(measurement);
        
        // Keep only recent measurements for performance
        if self.historical_data.len() > 1000 {
            self.historical_data.drain(0..500); // Remove oldest 500
        }
    }

    /// Add baseline for comparison
    pub fn add_baseline(&mut self, baseline: PerformanceBaseline) {
        self.baselines.insert(baseline.operation_name.clone(), baseline);
    }

    /// Detect regressions in benchmark result
    pub fn detect_regressions(&self, result: &BenchmarkResult) -> Vec<RegressionDetection> {
        let mut regressions = Vec::new();
        
        // Get baseline for comparison
        if let Some(baseline) = self.baselines.get(&result.operation_name) {
            // Check different types of regressions
            regressions.extend(self.detect_latency_regression(result, baseline));
            regressions.extend(self.detect_memory_regression(result, baseline));
            regressions.extend(self.detect_success_rate_regression(result, baseline));
            regressions.extend(self.detect_stability_regression(result, baseline));
        }
        
        // Trend analysis if we have historical data
        regressions.extend(self.detect_trend_regressions(result));
        
        regressions
    }

    /// Detect latency regression
    fn detect_latency_regression(
        &self, 
        result: &BenchmarkResult, 
        baseline: &PerformanceBaseline
    ) -> Vec<RegressionDetection> {
        let mut regressions = Vec::new();
        
        let baseline_latency = baseline.baseline_metrics.avg_duration_ms;
        let current_latency = result.avg_duration_ms;
        let change_percent = ((current_latency - baseline_latency) / baseline_latency) * 100.0;
        
        if change_percent > self.config.regression_threshold_percent {
            let severity = self.calculate_severity(change_percent);
            let confidence = self.calculate_statistical_confidence(result, baseline);
            
            regressions.push(RegressionDetection {
                operation_name: result.operation_name.clone(),
                regression_type: RegressionType::Latency,
                severity: severity.clone(),
                statistical_confidence: confidence,
                baseline_value: baseline_latency,
                current_value: current_latency,
                change_percent,
                detected_at: Utc::now(),
                trend_direction: self.analyze_trend(&result.operation_name, "latency"),
                recommendation: self.generate_latency_recommendation(change_percent, severity),
            });
        }
        
        regressions
    }

    /// Detect memory usage regression
    fn detect_memory_regression(
        &self, 
        result: &BenchmarkResult, 
        baseline: &PerformanceBaseline
    ) -> Vec<RegressionDetection> {
        let mut regressions = Vec::new();
        
        if !result.memory_usage_mb.is_empty() {
            let baseline_memory = baseline.baseline_metrics.memory_usage_mb;
            let current_memory = result.memory_usage_mb.iter().sum::<f64>() / result.memory_usage_mb.len() as f64;
            let change_percent = ((current_memory - baseline_memory) / baseline_memory) * 100.0;
            
            if change_percent > self.config.regression_threshold_percent {
                let severity = self.calculate_severity(change_percent);
                let confidence = self.calculate_statistical_confidence(result, baseline);
                
                regressions.push(RegressionDetection {
                    operation_name: result.operation_name.clone(),
                    regression_type: RegressionType::Memory,
                    severity,
                    statistical_confidence: confidence,
                    baseline_value: baseline_memory,
                    current_value: current_memory,
                    change_percent,
                    detected_at: Utc::now(),
                    trend_direction: self.analyze_trend(&result.operation_name, "memory"),
                    recommendation: self.generate_memory_recommendation(change_percent, severity),
                });
            }
        }
        
        regressions
    }

    /// Detect success rate regression
    fn detect_success_rate_regression(
        &self, 
        result: &BenchmarkResult, 
        baseline: &PerformanceBaseline
    ) -> Vec<RegressionDetection> {
        let mut regressions = Vec::new();
        
        let baseline_success_rate = baseline.baseline_metrics.success_rate;
        let current_success_rate = result.success_rate;
        let change_percent = ((baseline_success_rate - current_success_rate) / baseline_success_rate) * 100.0;
        
        // For success rate, decrease is bad
        if change_percent > self.config.regression_threshold_percent * 0.5 { // More sensitive for success rate
            let severity = self.calculate_severity(change_percent);
            let confidence = self.calculate_statistical_confidence(result, baseline);
            
            regressions.push(RegressionDetection {
                operation_name: result.operation_name.clone(),
                regression_type: RegressionType::SuccessRate,
                severity: severity.clone(),
                statistical_confidence: confidence,
                baseline_value: baseline_success_rate,
                current_value: current_success_rate,
                change_percent,
                detected_at: Utc::now(),
                trend_direction: self.analyze_trend(&result.operation_name, "success_rate"),
                recommendation: self.generate_success_rate_recommendation(change_percent, severity),
            });
        }
        
        regressions
    }

    /// Detect stability regression (increased variability)
    fn detect_stability_regression(
        &self, 
        result: &BenchmarkResult, 
        baseline: &PerformanceBaseline
    ) -> Vec<RegressionDetection> {
        let mut regressions = Vec::new();
        
        let baseline_std_dev = baseline.baseline_metrics.std_deviation_ms;
        
        // Calculate current standard deviation approximation
        let current_range = result.max_duration_ms - result.min_duration_ms;
        let current_std_dev_approx = current_range / 4.0; // Rough approximation
        
        let change_percent = ((current_std_dev_approx - baseline_std_dev) / baseline_std_dev) * 100.0;
        
        if change_percent > self.config.regression_threshold_percent {
            let severity = self.calculate_severity(change_percent);
            
            regressions.push(RegressionDetection {
                operation_name: result.operation_name.clone(),
                regression_type: RegressionType::Stability,
                severity,
                statistical_confidence: 0.7, // Lower confidence for approximated std dev
                baseline_value: baseline_std_dev,
                current_value: current_std_dev_approx,
                change_percent,
                detected_at: Utc::now(),
                trend_direction: self.analyze_trend(&result.operation_name, "stability"),
                recommendation: self.generate_stability_recommendation(change_percent, severity),
            });
        }
        
        regressions
    }

    /// Detect trend-based regressions from historical data
    fn detect_trend_regressions(&self, result: &BenchmarkResult) -> Vec<RegressionDetection> {
        let mut regressions = Vec::new();
        
        // Get recent measurements for this operation
        let recent_measurements: Vec<_> = self.historical_data
            .iter()
            .filter(|d| d.operation_name == result.operation_name)
            .rev()
            .take(self.config.trend_analysis_window)
            .collect();
        
        if recent_measurements.len() >= self.config.min_samples_for_detection {
            // Analyze trend in latency
            if let Some(trend_regression) = self.analyze_latency_trend(&recent_measurements, result) {
                regressions.push(trend_regression);
            }
            
            // Analyze trend in memory usage
            if let Some(memory_trend) = self.analyze_memory_trend(&recent_measurements, result) {
                regressions.push(memory_trend);
            }
        }
        
        regressions
    }

    /// Analyze latency trend for regression
    fn analyze_latency_trend(&self, measurements: &[&PerformanceDataPoint], current: &BenchmarkResult) -> Option<RegressionDetection> {
        if measurements.len() < self.config.min_samples_for_detection {
            return None;
        }
        
        // Calculate trend using simple linear regression
        let data_points: Vec<(f64, f64)> = measurements.iter().enumerate().map(|(i, m)| (i as f64, m.duration_ms)).collect();
        let (slope, _intercept) = self.calculate_linear_trend(&data_points);
        
        // Positive slope indicates increasing latency (regression)
        if slope > 0.5 { // More than 0.5ms increase per measurement
            let baseline_avg = measurements.iter().map(|m| m.duration_ms).sum::<f64>() / measurements.len() as f64;
            let current_avg = current.avg_duration_ms;
            let change_percent = ((current_avg - baseline_avg) / baseline_avg) * 100.0;
            
            if change_percent > self.config.regression_threshold_percent * 0.5 { // More sensitive for trends
                let severity = self.calculate_severity(change_percent);
                
                return Some(RegressionDetection {
                    operation_name: current.operation_name.clone(),
                    regression_type: RegressionType::Latency,
                    severity,
                    statistical_confidence: 0.8, // Trend analysis has lower confidence
                    baseline_value: baseline_avg,
                    current_value: current_avg,
                    change_percent,
                    detected_at: Utc::now(),
                    trend_direction: TrendDirection::Degrading,
                    recommendation: format!("Latency trend shows degradation: {:.2}ms increase per measurement. Investigate recent changes.", slope),
                });
            }
        }
        
        None
    }

    /// Analyze memory usage trend for regression
    fn analyze_memory_trend(&self, measurements: &[&PerformanceDataPoint], current: &BenchmarkResult) -> Option<RegressionDetection> {
        if measurements.len() < self.config.min_samples_for_detection || current.memory_usage_mb.is_empty() {
            return None;
        }
        
        // Calculate memory trend
        let data_points: Vec<(f64, f64)> = measurements.iter().enumerate().map(|(i, m)| (i as f64, m.memory_usage_mb)).collect();
        let (slope, _intercept) = self.calculate_linear_trend(&data_points);
        
        // Positive slope indicates increasing memory usage (potential regression)
        if slope > 0.1 { // More than 0.1MB increase per measurement
            let baseline_avg = measurements.iter().map(|m| m.memory_usage_mb).sum::<f64>() / measurements.len() as f64;
            let current_avg = current.memory_usage_mb.iter().sum::<f64>() / current.memory_usage_mb.len() as f64;
            let change_percent = ((current_avg - baseline_avg) / baseline_avg) * 100.0;
            
            if change_percent > self.config.regression_threshold_percent * 0.5 {
                let severity = self.calculate_severity(change_percent);
                
                return Some(RegressionDetection {
                    operation_name: current.operation_name.clone(),
                    regression_type: RegressionType::Memory,
                    severity,
                    statistical_confidence: 0.8,
                    baseline_value: baseline_avg,
                    current_value: current_avg,
                    change_percent,
                    detected_at: Utc::now(),
                    trend_direction: TrendDirection::Degrading,
                    recommendation: format!("Memory usage trend shows growth: {:.2}MB increase per measurement. Check for memory leaks.", slope),
                });
            }
        }
        
        None
    }

    /// Calculate linear trend using simple linear regression
    fn calculate_linear_trend(&self, data_points: &[(f64, f64)]) -> (f64, f64) {
        if data_points.len() < 2 {
            return (0.0, 0.0);
        }
        
        let n = data_points.len() as f64;
        let sum_x: f64 = data_points.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = data_points.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = data_points.iter().map(|(x, y)| x * y).sum();
        let sum_x_squared: f64 = data_points.iter().map(|(x, _)| x * x).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x_squared - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;
        
        (slope, intercept)
    }

    /// Calculate severity based on change percentage
    fn calculate_severity(&self, change_percent: f64) -> RegressionSeverity {
        let abs_change = change_percent.abs();
        
        if abs_change >= 100.0 {
            RegressionSeverity::Critical
        } else if abs_change >= 50.0 {
            RegressionSeverity::Major
        } else if abs_change >= 30.0 {
            RegressionSeverity::Moderate
        } else {
            RegressionSeverity::Minor
        }
    }

    /// Calculate statistical confidence for regression detection
    fn calculate_statistical_confidence(&self, result: &BenchmarkResult, baseline: &PerformanceBaseline) -> f64 {
        // Simplified confidence calculation based on sample sizes and variability
        let sample_size_factor = (result.iterations as f64 / 10.0).min(1.0);
        let baseline_factor = (baseline.sample_count as f64 / 20.0).min(1.0);
        let baseline_confidence_factor = baseline.confidence_level;
        
        (sample_size_factor * 0.3 + baseline_factor * 0.3 + baseline_confidence_factor * 0.4)
            .max(0.0).min(1.0)
    }

    /// Analyze trend direction from historical data
    fn analyze_trend(&self, operation_name: &str, metric_type: &str) -> TrendDirection {
        let recent_data: Vec<_> = self.historical_data
            .iter()
            .filter(|d| d.operation_name == operation_name)
            .rev()
            .take(self.config.trend_analysis_window)
            .collect();
        
        if recent_data.len() < 3 {
            return TrendDirection::Stable;
        }
        
        let values: Vec<f64> = match metric_type {
            "latency" => recent_data.iter().map(|d| d.duration_ms).collect(),
            "memory" => recent_data.iter().map(|d| d.memory_usage_mb).collect(),
            "success_rate" => recent_data.iter().map(|d| d.success_rate).collect(),
            _ => return TrendDirection::Stable,
        };
        
        let data_points: Vec<(f64, f64)> = values.iter().enumerate().map(|(i, &v)| (i as f64, v)).collect();
        let (slope, _) = self.calculate_linear_trend(&data_points);
        
        // Determine trend direction based on slope and metric type
        match metric_type {
            "success_rate" => {
                // For success rate, positive slope is improvement
                if slope > 0.01 { TrendDirection::Improving } 
                else if slope < -0.01 { TrendDirection::Degrading }
                else { TrendDirection::Stable }
            }
            _ => {
                // For latency and memory, negative slope is improvement
                if slope < -0.1 { TrendDirection::Improving }
                else if slope > 0.1 { TrendDirection::Degrading }
                else { TrendDirection::Stable }
            }
        }
    }

    /// Generate recommendation for latency regression
    fn generate_latency_recommendation(&self, change_percent: f64, severity: RegressionSeverity) -> String {
        match severity {
            RegressionSeverity::Critical => {
                format!("CRITICAL: Latency increased by {:.1}%. Immediate investigation required. Check for blocking operations, network issues, or resource contention.", change_percent)
            }
            RegressionSeverity::Major => {
                format!("MAJOR: Latency increased by {:.1}%. Investigate recent changes, optimize critical paths, or increase timeouts.", change_percent)
            }
            RegressionSeverity::Moderate => {
                format!("MODERATE: Latency increased by {:.1}%. Monitor trend and consider optimization opportunities.", change_percent)
            }
            RegressionSeverity::Minor => {
                format!("MINOR: Latency increased by {:.1}%. Continue monitoring for trend development.", change_percent)
            }
        }
    }

    /// Generate recommendation for memory regression
    fn generate_memory_recommendation(&self, change_percent: f64, severity: RegressionSeverity) -> String {
        match severity {
            RegressionSeverity::Critical => {
                format!("CRITICAL: Memory usage increased by {:.1}%. Check for memory leaks, unbounded collections, or resource retention.", change_percent)
            }
            RegressionSeverity::Major => {
                format!("MAJOR: Memory usage increased by {:.1}%. Profile memory allocation patterns and optimize data structures.", change_percent)
            }
            RegressionSeverity::Moderate => {
                format!("MODERATE: Memory usage increased by {:.1}%. Review recent changes for unnecessary allocations.", change_percent)
            }
            RegressionSeverity::Minor => {
                format!("MINOR: Memory usage increased by {:.1}%. Monitor for continued growth.", change_percent)
            }
        }
    }

    /// Generate recommendation for success rate regression
    fn generate_success_rate_recommendation(&self, change_percent: f64, severity: RegressionSeverity) -> String {
        match severity {
            RegressionSeverity::Critical => {
                format!("CRITICAL: Success rate decreased by {:.1}%. Check error handling, network stability, and service dependencies.", change_percent)
            }
            RegressionSeverity::Major => {
                format!("MAJOR: Success rate decreased by {:.1}%. Investigate error patterns and improve retry logic.", change_percent)
            }
            RegressionSeverity::Moderate => {
                format!("MODERATE: Success rate decreased by {:.1}%. Review error conditions and timeout settings.", change_percent)
            }
            RegressionSeverity::Minor => {
                format!("MINOR: Success rate decreased by {:.1}%. Monitor error patterns for trends.", change_percent)
            }
        }
    }

    /// Generate recommendation for stability regression
    fn generate_stability_recommendation(&self, change_percent: f64, severity: RegressionSeverity) -> String {
        match severity {
            RegressionSeverity::Critical => {
                format!("CRITICAL: Performance variability increased by {:.1}%. System may be under resource stress or have timing issues.", change_percent)
            }
            RegressionSeverity::Major => {
                format!("MAJOR: Performance variability increased by {:.1}%. Check for resource contention or inconsistent execution paths.", change_percent)
            }
            RegressionSeverity::Moderate => {
                format!("MODERATE: Performance variability increased by {:.1}%. Monitor for pattern changes or load variations.", change_percent)
            }
            RegressionSeverity::Minor => {
                format!("MINOR: Performance variability increased by {:.1}%. Continue monitoring stability metrics.", change_percent)
            }
        }
    }

    /// Perform comprehensive regression analysis
    pub fn analyze_performance_regressions(
        &self, 
        results: &[BenchmarkResult]
    ) -> RegressionAnalysisReport {
        let mut all_regressions = Vec::new();
        let mut operation_summaries = HashMap::new();
        
        for result in results {
            let regressions = self.detect_regressions(result);
            
            // Categorize regressions by severity
            let critical_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Critical).count();
            let major_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Major).count();
            let moderate_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Moderate).count();
            let minor_count = regressions.iter().filter(|r| r.severity == RegressionSeverity::Minor).count();
            
            operation_summaries.insert(result.operation_name.clone(), OperationRegressionSummary {
                operation_name: result.operation_name.clone(),
                total_regressions: regressions.len(),
                critical_regressions: critical_count,
                major_regressions: major_count,
                moderate_regressions: moderate_count,
                minor_regressions: minor_count,
                baseline_available: self.baselines.contains_key(&result.operation_name),
                current_performance: result.avg_duration_ms,
                trend_direction: self.analyze_trend(&result.operation_name, "latency"),
            });
            
            all_regressions.extend(regressions);
        }
        
        RegressionAnalysisReport {
            analysis_timestamp: Utc::now(),
            total_operations_analyzed: results.len(),
            total_regressions_detected: all_regressions.len(),
            regressions: all_regressions,
            overall_health: self.calculate_overall_health(&operation_summaries),
            recommendations: self.generate_analysis_recommendations(&operation_summaries),
            operation_summaries,
        }
    }

    /// Calculate overall system health based on regressions
    fn calculate_overall_health(&self, summaries: &HashMap<String, OperationRegressionSummary>) -> SystemHealthStatus {
        let total_operations = summaries.len();
        let operations_with_critical = summaries.values().filter(|s| s.critical_regressions > 0).count();
        let operations_with_major = summaries.values().filter(|s| s.major_regressions > 0).count();
        let total_regressions: usize = summaries.values().map(|s| s.total_regressions).sum();
        
        if operations_with_critical > 0 {
            SystemHealthStatus::Critical
        } else if operations_with_major > total_operations / 2 {
            SystemHealthStatus::Poor
        } else if total_regressions > total_operations {
            SystemHealthStatus::Fair
        } else {
            SystemHealthStatus::Good
        }
    }

    /// Generate analysis recommendations
    fn generate_analysis_recommendations(&self, summaries: &HashMap<String, OperationRegressionSummary>) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        let critical_operations: Vec<_> = summaries.values()
            .filter(|s| s.critical_regressions > 0)
            .collect();
        
        if !critical_operations.is_empty() {
            recommendations.push(format!("🔴 URGENT: {} operations have critical regressions requiring immediate attention", 
                                        critical_operations.len()));
        }
        
        let degrading_operations: Vec<_> = summaries.values()
            .filter(|s| matches!(s.trend_direction, TrendDirection::Degrading))
            .collect();
        
        if degrading_operations.len() > summaries.len() / 2 {
            recommendations.push("📉 Multiple operations showing degrading trends - investigate system-wide issues".to_string());
        }
        
        let no_baseline_operations: Vec<_> = summaries.values()
            .filter(|s| !s.baseline_available)
            .collect();
        
        if !no_baseline_operations.is_empty() {
            recommendations.push(format!("📊 {} operations lack baselines - establish baselines for better regression detection", 
                                        no_baseline_operations.len()));
        }
        
        if recommendations.is_empty() {
            recommendations.push("✅ No immediate performance concerns detected".to_string());
        }
        
        recommendations
    }
}

/// Summary of regressions for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRegressionSummary {
    pub operation_name: String,
    pub total_regressions: usize,
    pub critical_regressions: usize,
    pub major_regressions: usize,
    pub moderate_regressions: usize,
    pub minor_regressions: usize,
    pub baseline_available: bool,
    pub current_performance: f64,
    pub trend_direction: TrendDirection,
}

/// Overall system health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemHealthStatus {
    Good,     // No significant regressions
    Fair,     // Some minor/moderate regressions
    Poor,     // Multiple major regressions
    Critical, // Any critical regressions
}

/// Comprehensive regression analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysisReport {
    pub analysis_timestamp: DateTime<Utc>,
    pub total_operations_analyzed: usize,
    pub total_regressions_detected: usize,
    pub regressions: Vec<RegressionDetection>,
    pub operation_summaries: HashMap<String, OperationRegressionSummary>,
    pub overall_health: SystemHealthStatus,
    pub recommendations: Vec<String>,
}

impl RegressionAnalysisReport {
    /// Generate human-readable report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== PERFORMANCE REGRESSION ANALYSIS REPORT ===\n\n");
        report.push_str(&format!("Analysis Date: {}\n", self.analysis_timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        report.push_str(&format!("Operations Analyzed: {}\n", self.total_operations_analyzed));
        report.push_str(&format!("Regressions Detected: {}\n", self.total_regressions_detected));
        report.push_str(&format!("Overall Health: {:?}\n\n", self.overall_health));
        
        // Summary by severity
        let critical_count = self.regressions.iter().filter(|r| r.severity == RegressionSeverity::Critical).count();
        let major_count = self.regressions.iter().filter(|r| r.severity == RegressionSeverity::Major).count();
        let moderate_count = self.regressions.iter().filter(|r| r.severity == RegressionSeverity::Moderate).count();
        let minor_count = self.regressions.iter().filter(|r| r.severity == RegressionSeverity::Minor).count();
        
        report.push_str("Regression Summary by Severity:\n");
        report.push_str(&format!("  🔴 Critical: {}\n", critical_count));
        report.push_str(&format!("  🟠 Major: {}\n", major_count));
        report.push_str(&format!("  🟡 Moderate: {}\n", moderate_count));
        report.push_str(&format!("  🔵 Minor: {}\n", minor_count));
        report.push_str("\n");
        
        // Detailed regression information
        if !self.regressions.is_empty() {
            report.push_str("=== DETAILED REGRESSION ANALYSIS ===\n\n");
            
            for regression in &self.regressions {
                report.push_str(&format!("--- {} ({:?} Regression) ---\n", 
                                        regression.operation_name, regression.regression_type));
                report.push_str(&format!("  Severity: {:?}\n", regression.severity));
                report.push_str(&format!("  Change: {:.1}% ({:.2} → {:.2})\n", 
                                        regression.change_percent, regression.baseline_value, regression.current_value));
                report.push_str(&format!("  Confidence: {:.1}%\n", regression.statistical_confidence * 100.0));
                report.push_str(&format!("  Trend: {:?}\n", regression.trend_direction));
                report.push_str(&format!("  Detected: {}\n", regression.detected_at.format("%H:%M:%S")));
                report.push_str(&format!("  Recommendation: {}\n", regression.recommendation));
                report.push_str("\n");
            }
        }
        
        // Operation summaries
        report.push_str("=== OPERATION-LEVEL ANALYSIS ===\n\n");
        for summary in self.operation_summaries.values() {
            report.push_str(&format!("--- {} ---\n", summary.operation_name.to_uppercase()));
            report.push_str(&format!("  Total Regressions: {}\n", summary.total_regressions));
            if summary.total_regressions > 0 {
                report.push_str(&format!("    Critical: {}, Major: {}, Moderate: {}, Minor: {}\n",
                                        summary.critical_regressions, summary.major_regressions,
                                        summary.moderate_regressions, summary.minor_regressions));
            }
            report.push_str(&format!("  Baseline Available: {}\n", if summary.baseline_available { "✅ YES" } else { "❌ NO" }));
            report.push_str(&format!("  Current Performance: {:.2}ms\n", summary.current_performance));
            report.push_str(&format!("  Trend: {:?}\n", summary.trend_direction));
            report.push_str("\n");
        }
        
        // Recommendations
        report.push_str("=== RECOMMENDATIONS ===\n");
        for recommendation in &self.recommendations {
            report.push_str(&format!("• {}\n", recommendation));
        }
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_detection_config_default() {
        let config = RegressionDetectionConfig::default();
        
        assert_eq!(config.statistical_significance_level, 0.05);
        assert_eq!(config.regression_threshold_percent, 20.0);
        assert_eq!(config.improvement_threshold_percent, 10.0);
        assert_eq!(config.min_samples_for_detection, 5);
        assert_eq!(config.trend_analysis_window, 10);
        assert!(config.outlier_detection_enabled);
        assert!(!config.seasonal_adjustment_enabled);
    }

    #[test]
    fn test_severity_calculation() {
        let config = RegressionDetectionConfig::default();
        let detector = RegressionDetector::new(config);
        
        assert_eq!(detector.calculate_severity(15.0), RegressionSeverity::Minor);
        assert_eq!(detector.calculate_severity(35.0), RegressionSeverity::Moderate);
        assert_eq!(detector.calculate_severity(75.0), RegressionSeverity::Major);
        assert_eq!(detector.calculate_severity(150.0), RegressionSeverity::Critical);
    }

    #[test]
    fn test_trend_calculation() {
        let config = RegressionDetectionConfig::default();
        let detector = RegressionDetector::new(config);
        
        // Test upward trend
        let upward_data = vec![(0.0, 10.0), (1.0, 12.0), (2.0, 14.0), (3.0, 16.0)];
        let (slope, _) = detector.calculate_linear_trend(&upward_data);
        assert!(slope > 0.0, "Should detect upward trend");
        
        // Test downward trend
        let downward_data = vec![(0.0, 16.0), (1.0, 14.0), (2.0, 12.0), (3.0, 10.0)];
        let (slope, _) = detector.calculate_linear_trend(&downward_data);
        assert!(slope < 0.0, "Should detect downward trend");
        
        // Test stable trend
        let stable_data = vec![(0.0, 10.0), (1.0, 10.1), (2.0, 9.9), (3.0, 10.0)];
        let (slope, _) = detector.calculate_linear_trend(&stable_data);
        assert!(slope.abs() < 0.1, "Should detect stable trend");
    }

    #[test]
    fn test_regression_analysis_report_generation() {
        let report = RegressionAnalysisReport {
            analysis_timestamp: Utc::now(),
            total_operations_analyzed: 5,
            total_regressions_detected: 2,
            regressions: vec![
                RegressionDetection {
                    operation_name: "test_op".to_string(),
                    regression_type: RegressionType::Latency,
                    severity: RegressionSeverity::Major,
                    statistical_confidence: 0.95,
                    baseline_value: 10.0,
                    current_value: 18.0,
                    change_percent: 80.0,
                    detected_at: Utc::now(),
                    trend_direction: TrendDirection::Degrading,
                    recommendation: "Test recommendation".to_string(),
                }
            ],
            operation_summaries: HashMap::new(),
            overall_health: SystemHealthStatus::Poor,
            recommendations: vec!["Test recommendation".to_string()],
        };
        
        let report_text = report.generate_report();
        
        assert!(report_text.contains("PERFORMANCE REGRESSION ANALYSIS REPORT"));
        assert!(report_text.contains("Operations Analyzed: 5"));
        assert!(report_text.contains("Regressions Detected: 2"));
        assert!(report_text.contains("Overall Health: Poor"));
        assert!(report_text.contains("Major: 1"));
        assert!(report_text.contains("RECOMMENDATIONS"));
    }

    #[test]
    fn test_performance_data_point_creation() {
        let data_point = PerformanceDataPoint {
            timestamp: Utc::now(),
            operation_name: "test_operation".to_string(),
            duration_ms: 15.5,
            memory_usage_mb: 25.0,
            success_rate: 0.95,
            system_load: Some(0.7),
        };
        
        assert_eq!(data_point.operation_name, "test_operation");
        assert_eq!(data_point.duration_ms, 15.5);
        assert_eq!(data_point.memory_usage_mb, 25.0);
        assert_eq!(data_point.success_rate, 0.95);
        assert_eq!(data_point.system_load, Some(0.7));
    }

    #[test]
    fn test_regression_detection_creation() {
        let config = RegressionDetectionConfig::default();
        let mut detector = RegressionDetector::new(config);
        
        // Add some historical data
        let data_point = PerformanceDataPoint {
            timestamp: Utc::now(),
            operation_name: "test_op".to_string(),
            duration_ms: 10.0,
            memory_usage_mb: 5.0,
            success_rate: 1.0,
            system_load: None,
        };
        
        detector.add_measurement(data_point);
        assert_eq!(detector.historical_data.len(), 1);
        
        // Test data management
        for i in 0..1005 {
            let data_point = PerformanceDataPoint {
                timestamp: Utc::now(),
                operation_name: format!("op_{}", i),
                duration_ms: 10.0 + i as f64,
                memory_usage_mb: 5.0,
                success_rate: 1.0,
                system_load: None,
            };
            detector.add_measurement(data_point);
        }
        
        // Should have pruned old measurements
        assert!(detector.historical_data.len() <= 1000, "Should limit historical data size");
    }
}