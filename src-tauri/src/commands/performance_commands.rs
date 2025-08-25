//! # Performance Commands
//!
//! This module contains all Tauri commands related to performance benchmarking,
//! baseline management, and regression detection. It provides comprehensive
//! functionality for monitoring and analyzing system performance over time.
//!
//! ## Command Overview
//!
//! ### Benchmarking Operations
//! - `run_embedding_benchmarks`: Execute comprehensive performance benchmarks
//! - `generate_benchmark_report`: Generate detailed benchmark reports
//! - `detect_performance_regressions`: Identify performance degradations
//!
//! ### Baseline Management
//! - `establish_performance_baseline`: Create new performance baselines
//! - `compare_performance_against_baseline`: Compare current performance
//! - `get_baseline_report`: Get comprehensive baseline analysis
//!
//! ### Regression Analysis
//! - `analyze_performance_regressions`: Deep analysis of performance changes
//!
//! ## Benchmarking Framework
//!
//! The performance benchmarking system provides:
//!
//! ### Comprehensive Metrics
//! - **Execution Time**: Wall clock and CPU time measurements
//! - **Memory Usage**: Peak and average memory consumption
//! - **Throughput**: Operations per second and batch processing rates
//! - **Latency**: Request/response time distributions
//! - **Resource Utilization**: CPU, memory, and network usage
//!
//! ### Statistical Analysis
//! - **Central Tendencies**: Mean, median, and mode calculations
//! - **Variability**: Standard deviation and variance analysis
//! - **Distributions**: Percentile analysis (P50, P95, P99)
//! - **Confidence Intervals**: Statistical significance testing
//! - **Outlier Detection**: Automatic identification of anomalous results
//!
//! ## Baseline Management System
//!
//! ### Baseline Establishment
//! Performance baselines provide reference points for comparison:
//! - **Multiple Runs**: Statistical aggregation of multiple benchmark runs
//! - **Environment Capture**: System configuration and conditions
//! - **Version Tracking**: Software version and configuration tracking
//! - **Confidence Levels**: Statistical confidence in baseline measurements
//!
//! ### Comparison Framework
//! - **Statistical Significance**: Determines if differences are meaningful
//! - **Performance Ratios**: Relative performance improvements/regressions
//! - **Trend Analysis**: Performance changes over time
//! - **Threshold Alerts**: Configurable performance degradation warnings
//!
//! ## Regression Detection
//!
//! ### Detection Algorithms
//! - **Statistical Tests**: T-tests and variance analysis
//! - **Threshold-based**: Absolute and relative performance thresholds
//! - **Trend Analysis**: Moving averages and trend detection
//! - **Anomaly Detection**: Machine learning-based anomaly identification
//!
//! ### Reporting Features
//! - **Detailed Analysis**: Root cause analysis for regressions
//! - **Visual Summaries**: Performance trend visualization data
//! - **Action Items**: Recommended actions for performance issues
//! - **Historical Context**: Performance evolution over time
//!
//! ## Performance Metrics
//!
//! ### Core Measurements
//! - **Embedding Generation**: Time to generate single/batch embeddings
//! - **Cache Performance**: Hit ratios and lookup times
//! - **Memory Efficiency**: Memory usage patterns and leaks
//! - **Network Performance**: API call latencies and throughput
//! - **Disk I/O**: File operations and cache persistence
//!
//! ### System Resources
//! - **CPU Utilization**: Processing efficiency and core usage
//! - **Memory Consumption**: Peak, average, and persistent memory usage
//! - **Network Bandwidth**: Data transfer rates and efficiency
//! - **Disk Space**: Storage usage and growth patterns
//! - **Thread Utilization**: Concurrent processing effectiveness

use crate::globals::OLLAMA_CLIENT;
use crate::ollama_client::OllamaConfig;
use crate::benchmarks::{EmbeddingBenchmarks, BenchmarkConfig, BenchmarkResult};
use crate::performance_baseline::{BaselineManager, BaselineConfig, BaselineComparison};
use crate::regression_detection::{RegressionDetector, RegressionDetectionConfig, RegressionAnalysisReport};

/// Execute comprehensive embedding performance benchmarks
///
/// This command runs a full suite of performance benchmarks to measure
/// embedding generation performance, cache efficiency, and system resource
/// utilization. It provides detailed metrics for performance analysis.
///
/// # Returns
/// * `Ok(Vec<BenchmarkResult>)` - Comprehensive benchmark results
/// * `Err(String)` - Error message if benchmarking fails
///
/// # Benchmark Categories
/// 
/// ### Embedding Generation Benchmarks
/// - **Single Text**: Individual embedding generation performance
/// - **Batch Processing**: Batch embedding efficiency and throughput
/// - **Cache Performance**: Cache hit/miss ratios and lookup speeds
/// - **Model Loading**: Model initialization and memory usage
/// - **Error Recovery**: Performance under error conditions
///
/// ### Resource Utilization Benchmarks
/// - **Memory Usage**: Peak and average memory consumption
/// - **CPU Utilization**: Processing efficiency across cores
/// - **Network Performance**: API call latencies and throughput
/// - **Disk I/O**: Cache persistence and file operation speeds
/// - **Concurrent Processing**: Multi-threaded performance scaling
///
/// ### Stress Testing
/// - **High Load**: Performance under sustained high request rates
/// - **Memory Pressure**: Behavior under memory constraints
/// - **Network Issues**: Resilience to network problems
/// - **Resource Contention**: Performance with competing processes
///
/// # Example Usage (from frontend)
/// ```javascript
/// const benchmarkResults = await invoke('run_embedding_benchmarks');
/// 
/// console.log(`Completed ${benchmarkResults.length} benchmark tests`);
/// benchmarkResults.forEach(result => {
///     console.log(`${result.operation_name}: ${result.avg_time_ms}ms`);
///     console.log(`  Memory: ${result.peak_memory_mb}MB`);
///     console.log(`  Throughput: ${result.operations_per_second} ops/sec`);
/// });
/// ```
#[tauri::command]
pub async fn run_embedding_benchmarks() -> Result<Vec<BenchmarkResult>, String> {
    let ollama_config = {
        let client_lock = OLLAMA_CLIENT.read().await;
        if let Some(client) = client_lock.as_ref() {
            client.get_config().clone()
        } else {
            OllamaConfig::default()
        }
    };
    
    let benchmark_config = BenchmarkConfig::default();
    let mut benchmarks = EmbeddingBenchmarks::new(ollama_config, benchmark_config);
    
    benchmarks.run_comprehensive_benchmarks().await
        .map_err(|e| e.to_string())
}

/// Generate detailed report from benchmark results
///
/// This command analyzes benchmark results and generates a comprehensive
/// performance report with summaries, trends, and actionable insights.
/// The report includes statistical analysis and performance recommendations.
///
/// # Arguments
/// * `results` - Benchmark results to analyze and report on
///
/// # Returns
/// * `Ok(String)` - Formatted performance report
/// * `Err(String)` - Error message if report generation fails
///
/// # Report Content
///
/// ### Executive Summary
/// - Overall performance rating
/// - Key performance indicators (KPIs)
/// - Critical issues and recommendations
/// - Performance trends and patterns
///
/// ### Detailed Analysis
/// - **Operation Breakdown**: Performance by operation type
/// - **Resource Analysis**: CPU, memory, and I/O utilization
/// - **Efficiency Metrics**: Throughput and latency analysis
/// - **Comparative Analysis**: Performance vs historical data
/// - **Statistical Summary**: Confidence intervals and distributions
///
/// ### Visualizations and Charts
/// - Performance trend data for charting
/// - Resource utilization patterns
/// - Latency distribution histograms
/// - Throughput vs load curves
/// - Memory usage over time
///
/// # Example Usage (from frontend)
/// ```javascript
/// const benchmarkResults = await invoke('run_embedding_benchmarks');
/// const report = await invoke('generate_benchmark_report', {
///     results: benchmarkResults
/// });
/// 
/// console.log('Performance Report:');
/// console.log(report);
/// 
/// // Display in UI or save to file
/// document.getElementById('performance-report').innerText = report;
/// ```
#[tauri::command]
pub async fn generate_benchmark_report(results: Vec<BenchmarkResult>) -> Result<String, String> {
    let ollama_config = OllamaConfig::default();
    let benchmark_config = BenchmarkConfig::default();
    let benchmarks = EmbeddingBenchmarks::new(ollama_config, benchmark_config);
    
    Ok(benchmarks.generate_report(&results))
}

/// Detect performance regressions in benchmark results
///
/// This command analyzes benchmark results to identify potential performance
/// regressions by comparing against historical baselines and detecting
/// significant performance degradations.
///
/// # Arguments
/// * `results` - Current benchmark results to analyze for regressions
///
/// # Returns
/// * `Ok(Vec<String>)` - List of detected regression warnings and issues
/// * `Err(String)` - Error message if regression detection fails
///
/// # Detection Methods
///
/// ### Statistical Analysis
/// - **Threshold Detection**: Performance drops below acceptable levels
/// - **Variance Analysis**: Unusual performance variability
/// - **Trend Analysis**: Declining performance trends over time
/// - **Outlier Detection**: Abnormal performance measurements
///
/// ### Comparative Analysis
/// - **Baseline Comparison**: Performance vs established baselines
/// - **Historical Trends**: Changes from previous measurements
/// - **Peer Comparison**: Performance vs similar operations
/// - **Resource Efficiency**: Degradation in resource utilization
///
/// ### Regression Categories
/// - **Critical**: Severe performance degradation requiring immediate action
/// - **Warning**: Moderate degradation needing investigation
/// - **Advisory**: Minor changes worth monitoring
/// - **Information**: Notable changes within acceptable ranges
///
/// # Example Usage (from frontend)
/// ```javascript
/// const benchmarkResults = await invoke('run_embedding_benchmarks');
/// const regressions = await invoke('detect_performance_regressions', {
///     results: benchmarkResults
/// });
/// 
/// if (regressions.length > 0) {
///     console.log('Performance issues detected:');
///     regressions.forEach(issue => {
///         console.log('- ' + issue);
///     });
/// } else {
///     console.log('No performance regressions detected');
/// }
/// ```
#[tauri::command]
pub async fn detect_performance_regressions(results: Vec<BenchmarkResult>) -> Result<Vec<String>, String> {
    let ollama_config = OllamaConfig::default();
    let benchmark_config = BenchmarkConfig::default();
    let benchmarks = EmbeddingBenchmarks::new(ollama_config, benchmark_config);
    
    Ok(benchmarks.detect_performance_regressions(&results))
}

/// Establish performance baseline for specific operation
///
/// This command creates a new performance baseline by running multiple
/// benchmark iterations and establishing statistical reference points
/// for future performance comparisons.
///
/// # Arguments
/// * `operation_name` - Name of the operation to establish baseline for
///
/// # Returns
/// * `Ok(String)` - Confirmation message with baseline statistics
/// * `Err(String)` - Error message if baseline establishment fails
///
/// # Baseline Creation Process
///
/// ### Data Collection
/// - **Multiple Runs**: Execute operation multiple times for statistical validity
/// - **Environment Capture**: Record system configuration and conditions
/// - **Resource Monitoring**: Track CPU, memory, and I/O during benchmarks
/// - **Error Handling**: Account for and filter out error conditions
///
/// ### Statistical Processing
/// - **Central Tendencies**: Calculate mean, median, and mode
/// - **Variability Measures**: Standard deviation and confidence intervals
/// - **Outlier Removal**: Filter out statistical outliers
/// - **Distribution Analysis**: Understand performance characteristics
///
/// ### Baseline Storage
/// - **Persistent Storage**: Save baseline data for long-term comparison
/// - **Version Tracking**: Associate baselines with software versions
/// - **Configuration Context**: Record relevant system configuration
/// - **Metadata**: Timestamp, environment, and benchmark conditions
///
/// # Example Usage (from frontend)
/// ```javascript
/// const baselineResult = await invoke('establish_performance_baseline', {
///     operationName: 'embedding_generation_single'
/// });
/// 
/// console.log(baselineResult);
/// // Example output: "Baseline established for 'embedding_generation_single' with 95.2% confidence"
/// 
/// // Now future benchmarks can be compared against this baseline
/// ```
#[tauri::command]
pub async fn establish_performance_baseline(operation_name: String) -> Result<String, String> {
    let ollama_config = {
        let client_lock = OLLAMA_CLIENT.read().await;
        if let Some(client) = client_lock.as_ref() {
            client.get_config().clone()
        } else {
            OllamaConfig::default()
        }
    };
    
    let baseline_config = BaselineConfig::default();
    let mut manager = BaselineManager::new(baseline_config)
        .map_err(|e| format!("Failed to create baseline manager: {}", e))?;
    
    let baseline = manager.establish_baseline(&operation_name, &ollama_config).await
        .map_err(|e| format!("Failed to establish baseline: {}", e))?;
    
    Ok(format!("Baseline established for '{}' with {:.1}% confidence", 
               baseline.operation_name, baseline.confidence_level * 100.0))
}

/// Compare current performance against established baseline
///
/// This command compares a benchmark result against the established baseline
/// for the same operation, providing statistical analysis of performance
/// changes and their significance.
///
/// # Arguments
/// * `operation_name` - Name of the operation to compare
/// * `benchmark_result` - Current benchmark result to compare against baseline
///
/// # Returns
/// * `Ok(BaselineComparison)` - Detailed comparison analysis
/// * `Err(String)` - Error message if comparison fails
///
/// # Comparison Analysis
///
/// ### Statistical Comparison
/// - **Performance Ratio**: Current vs baseline performance ratio
/// - **Statistical Significance**: Whether difference is statistically meaningful
/// - **Confidence Level**: Confidence in the comparison results
/// - **P-Value**: Statistical significance of the difference
///
/// ### Performance Assessment
/// - **Improvement/Regression**: Direction and magnitude of change
/// - **Severity Rating**: How significant the change is
/// - **Trend Analysis**: Whether change fits expected patterns
/// - **Recommendations**: Suggested actions based on results
///
/// ### Context Information
/// - **Baseline Age**: How old the baseline is
/// - **Sample Size**: Number of measurements in comparison
/// - **Environmental Factors**: Relevant system condition changes
/// - **Version Differences**: Software changes since baseline
///
/// # Example Usage (from frontend)
/// ```javascript
/// const benchmarkResults = await invoke('run_embedding_benchmarks');
/// const singleEmbeddingResult = benchmarkResults.find(
///     r => r.operation_name === 'embedding_generation_single'
/// );
/// 
/// const comparison = await invoke('compare_performance_against_baseline', {
///     operationName: 'embedding_generation_single',
///     benchmarkResult: singleEmbeddingResult
/// });
/// 
/// console.log('Performance Comparison:');
/// console.log(`- Change: ${comparison.performance_change_percent.toFixed(1)}%`);
/// console.log(`- Significance: ${comparison.is_statistically_significant ? 'Yes' : 'No'}`);
/// console.log(`- Assessment: ${comparison.assessment}`);
/// ```
#[tauri::command]
pub async fn compare_performance_against_baseline(
    operation_name: String, 
    benchmark_result: BenchmarkResult
) -> Result<BaselineComparison, String> {
    let baseline_config = BaselineConfig::default();
    let manager = BaselineManager::new(baseline_config)
        .map_err(|e| format!("Failed to create baseline manager: {}", e))?;
    
    Ok(manager.compare_against_baseline(&operation_name, &benchmark_result))
}

/// Get comprehensive baseline performance report
///
/// This command generates a detailed report of all established performance
/// baselines, including their statistics, age, and relevance for current
/// performance monitoring.
///
/// # Returns
/// * `Ok(String)` - Formatted baseline report
/// * `Err(String)` - Error message if report generation fails
///
/// # Report Contents
///
/// ### Baseline Inventory
/// - **All Baselines**: Complete list of established baselines
/// - **Operation Coverage**: Operations with and without baselines
/// - **Baseline Age**: How recent each baseline is
/// - **Statistical Quality**: Confidence levels and sample sizes
///
/// ### Performance Summary
/// - **Current Status**: Overall performance health
/// - **Trend Analysis**: Performance trends across operations
/// - **Comparative Analysis**: Relative performance between operations
/// - **Recommendation Summary**: Actions needed for baseline maintenance
///
/// ### Maintenance Information
/// - **Outdated Baselines**: Baselines needing updates
/// - **Missing Baselines**: Operations lacking baseline references
/// - **Quality Issues**: Baselines with low confidence or small samples
/// - **Update Schedule**: Recommended baseline refresh timeline
///
/// # Example Usage (from frontend)
/// ```javascript
/// const baselineReport = await invoke('get_baseline_report');
/// 
/// console.log('Baseline Performance Report:');
/// console.log(baselineReport);
/// 
/// // Display in performance dashboard
/// document.getElementById('baseline-report').innerHTML = 
///     baselineReport.replace(/\n/g, '<br>');
/// ```
#[tauri::command]
pub async fn get_baseline_report() -> Result<String, String> {
    let baseline_config = BaselineConfig::default();
    let manager = BaselineManager::new(baseline_config)
        .map_err(|e| format!("Failed to create baseline manager: {}", e))?;
    
    Ok(manager.generate_baseline_report())
}

/// Analyze performance regressions with detailed statistical analysis
///
/// This command performs comprehensive analysis of benchmark results to
/// identify, categorize, and analyze performance regressions using advanced
/// statistical methods and machine learning techniques.
///
/// # Arguments
/// * `benchmark_results` - Complete set of benchmark results to analyze
///
/// # Returns
/// * `Ok(RegressionAnalysisReport)` - Comprehensive regression analysis
/// * `Err(String)` - Error message if analysis fails
///
/// # Analysis Framework
///
/// ### Advanced Detection Methods
/// - **Machine Learning**: Anomaly detection using trained models
/// - **Time Series Analysis**: Trend detection and forecasting
/// - **Multivariate Analysis**: Cross-operation performance correlation
/// - **Pattern Recognition**: Recurring performance patterns and cycles
///
/// ### Statistical Techniques
/// - **Hypothesis Testing**: T-tests, ANOVA, and non-parametric tests
/// - **Regression Analysis**: Linear and polynomial trend fitting
/// - **Change Point Detection**: Identification of performance shifts
/// - **Distribution Analysis**: Performance characteristic changes
///
/// ### Contextual Analysis
/// - **Environmental Factors**: System load, configuration changes
/// - **Resource Correlation**: Performance vs resource utilization
/// - **Operation Dependencies**: Inter-operation performance relationships
/// - **Historical Context**: Long-term performance evolution
///
/// ### Report Structure
/// The analysis report includes:
/// - **Executive Summary**: Key findings and critical issues
/// - **Detailed Findings**: Individual regression analysis
/// - **Root Cause Analysis**: Potential causes for performance changes
/// - **Impact Assessment**: Business and technical impact evaluation
/// - **Recommendations**: Specific actions to address issues
/// - **Monitoring Suggestions**: Ongoing monitoring recommendations
///
/// # Example Usage (from frontend)
/// ```javascript
/// const benchmarkResults = await invoke('run_embedding_benchmarks');
/// const analysisReport = await invoke('analyze_performance_regressions', {
///     benchmarkResults: benchmarkResults
/// });
/// 
/// console.log('Regression Analysis Report:');
/// console.log('Summary:', analysisReport.summary);
/// console.log('Critical Issues:', analysisReport.critical_regressions.length);
/// console.log('Recommendations:', analysisReport.recommendations);
/// 
/// // Process detailed findings
/// analysisReport.detailed_findings.forEach(finding => {
///     console.log(`Operation: ${finding.operation_name}`);
///     console.log(`Severity: ${finding.severity}`);
///     console.log(`Impact: ${finding.impact_description}`);
/// });
/// ```
#[tauri::command]
pub async fn analyze_performance_regressions(benchmark_results: Vec<BenchmarkResult>) -> Result<RegressionAnalysisReport, String> {
    let config = RegressionDetectionConfig::default();
    let mut detector = RegressionDetector::new(config);
    
    // Load existing baselines if available
    let baseline_config = BaselineConfig::default();
    let baseline_manager = BaselineManager::new(baseline_config)
        .map_err(|e| format!("Failed to load baselines: {}", e))?;
    
    for baseline in baseline_manager.get_all_baselines() {
        detector.add_baseline(baseline.clone());
    }
    
    Ok(detector.analyze_performance_regressions(&benchmark_results))
}