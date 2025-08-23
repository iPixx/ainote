# Performance Monitoring & Testing Guide

This guide provides comprehensive instructions for testing and using the performance monitoring infrastructure implemented for embedding model management in aiNote.

## Overview

The performance monitoring system includes:
- **Benchmarking Infrastructure**: Statistical performance analysis and tracking
- **Baseline Management**: Performance baseline establishment and comparison
- **Regression Detection**: Advanced statistical analysis for performance degradation
- **Comprehensive Testing**: Integration, corruption recovery, and UI testing

## Table of Contents

- [Standard Rust Testing](#standard-rust-testing)
- [Performance Benchmarking via Tauri Commands](#performance-benchmarking-via-tauri-commands)
- [Memory Usage Testing](#memory-usage-testing)
- [Integration Testing](#integration-testing)
- [Corruption and Recovery Testing](#corruption-and-recovery-testing)
- [Performance Regression Testing](#performance-regression-testing)
- [Advanced Testing Scenarios](#advanced-testing-scenarios)
- [Custom Test Execution](#custom-test-execution)
- [Continuous Testing](#continuous-testing)
- [Production Monitoring](#production-monitoring)
- [Test Output Examples](#test-output-examples)

## Standard Rust Testing

### Run All Tests
```bash
# Run all unit and integration tests
cargo test

# Run tests with output (shows println! statements)
cargo test -- --nocapture

# Run tests in release mode (faster execution)
cargo test --release

# Run tests with full backtrace on failure
RUST_BACKTRACE=full cargo test
```

### Run Specific Test Categories
```bash
# Run only unit tests (inline tests in modules)
cargo test --lib

# Run only integration tests (tests/ directory)
cargo test --test '*'

# Run specific test file
cargo test --test model_download_integration_tests
cargo test --test model_management_ui_e2e_tests
cargo test --test model_corruption_recovery_tests
```

### Run Specific Test Functions
```bash
# Run specific test by name
cargo test test_comprehensive_benchmarks
cargo test test_baseline_establishment
cargo test test_regression_detection

# Run tests matching a pattern
cargo test benchmark
cargo test baseline
cargo test regression

# Run with specific number of threads
cargo test -- --test-threads=1
```

## Performance Benchmarking via Tauri Commands

### Using Tauri Dev Mode
```bash
# Start the application in development mode
pnpm tauri dev
```

### Frontend JavaScript API Usage
```javascript
// Run comprehensive benchmarks
const results = await invoke('run_embedding_benchmarks');
console.log('Benchmark results:', results);

// Generate performance report
const report = await invoke('generate_benchmark_report', { results });
console.log('Performance report:', report);

// Establish baseline for an operation
const baseline = await invoke('establish_performance_baseline', { 
    operationName: 'embedding_generation' 
});
console.log('Baseline established:', baseline);

// Compare performance against baseline
const comparison = await invoke('compare_performance_against_baseline', {
    operationName: 'embedding_generation',
    benchmarkResult: results[0]
});
console.log('Baseline comparison:', comparison);

// Analyze for regressions
const analysis = await invoke('analyze_performance_regressions', { 
    benchmarkResults: results 
});
console.log('Regression analysis:', analysis);

// Get baseline report
const baselineReport = await invoke('get_baseline_report');
console.log('Baseline report:', baselineReport);
```

### Available Tauri Commands
- `run_embedding_benchmarks()`: Execute comprehensive performance benchmarks
- `generate_benchmark_report(results)`: Create detailed analysis reports
- `establish_performance_baseline(operationName)`: Set performance baselines
- `compare_performance_against_baseline(operationName, result)`: Compare against baseline
- `analyze_performance_regressions(results)`: Advanced regression detection
- `get_baseline_report()`: Get comprehensive baseline report

## Memory Usage Testing

### Monitor Memory During Tests
```bash
# Run tests with memory profiling
RUST_BACKTRACE=1 cargo test test_memory_usage_monitoring -- --nocapture

# Use system monitoring tools during tests
# On macOS:
top -pid $(pgrep -f "cargo test") &
cargo test

# On Linux:
htop -p $(pgrep -f "cargo test") &
cargo test

# Monitor memory with valgrind (Linux)
valgrind --tool=memcheck --leak-check=full cargo test
```

### Specific Memory Tests
```bash
# Run memory leak detection tests
cargo test test_memory_leak_detection -- --nocapture

# Run concurrent access memory tests
cargo test test_concurrent_benchmark_execution -- --nocapture

# Memory usage under load
cargo test test_memory_under_load -- --nocapture
```

### Memory Monitoring Commands
```bash
# Check memory usage during benchmarks
cargo test test_benchmark_memory_usage -- --nocapture

# Test memory cleanup after operations
cargo test test_memory_cleanup -- --nocapture
```

## Integration Testing

### Download Workflow Testing
```bash
# Run complete download integration tests
cargo test --test model_download_integration_tests -- --nocapture

# Test specific download scenarios
cargo test test_complete_download_workflow
cargo test test_download_progress_tracking
cargo test test_download_cancellation
cargo test test_download_error_recovery
```

### UI End-to-End Testing
```bash
# Run UI integration tests
cargo test --test model_management_ui_e2e_tests -- --nocapture

# Test specific UI scenarios
cargo test test_model_status_ui_updates
cargo test test_download_progress_ui
cargo test test_error_handling_ui
cargo test test_user_workflow_complete
```

### Integration Test Scenarios
- Complete model download workflow validation
- Progress tracking accuracy verification
- Cancellation and resumption testing
- Error handling and recovery testing
- UI responsiveness during long operations

## Corruption and Recovery Testing

### Model Corruption Testing
```bash
# Run corruption recovery tests
cargo test --test model_corruption_recovery_tests -- --nocapture

# Test specific corruption scenarios
cargo test test_partial_corruption_detection
cargo test test_complete_corruption_recovery
cargo test test_hash_mismatch_handling
cargo test test_size_mismatch_detection
```

### Corruption Scenarios Covered
- Partial file corruption during download
- Complete file corruption detection
- Hash mismatch validation
- Size validation failures
- Recovery strategy effectiveness

## Performance Regression Testing

### Real-time Regression Detection
```bash
# Run regression detection tests
cargo test test_regression_detection_system -- --nocapture

# Test trend analysis
cargo test test_trend_analysis
cargo test test_statistical_confidence
cargo test test_severity_classification
```

### Baseline Comparison Testing
```bash
# Test baseline establishment
cargo test test_baseline_establishment -- --nocapture

# Test baseline comparison
cargo test test_baseline_comparison
cargo test test_baseline_confidence_calculation
```

### Regression Analysis Features
- Multi-dimensional performance analysis (latency, memory, success rate, stability)
- Linear trend analysis with configurable thresholds
- Statistical confidence calculation (>95% accuracy)
- Severity classification (Minor, Moderate, Major, Critical)
- Automated recommendation generation

## Advanced Testing Scenarios

### Load Testing
```bash
# Run high-load scenarios
cargo test test_concurrent_benchmark_execution -- --nocapture
cargo test test_memory_under_load -- --nocapture
cargo test test_performance_under_stress -- --nocapture
```

### Error Scenario Testing
```bash
# Test various error conditions
cargo test test_ollama_connection_failure
cargo test test_insufficient_memory_handling
cargo test test_network_interruption_recovery
cargo test test_disk_space_exhaustion
```

### Concurrent Operations Testing
```bash
# Test concurrent benchmark execution
cargo test test_concurrent_benchmarks -- --nocapture

# Test thread safety
cargo test test_thread_safety -- --nocapture
```

## Custom Test Execution

### Run Tests with Specific Ollama Setup
```bash
# Ensure Ollama is running first
ollama serve &

# Wait for Ollama to start
sleep 5

# Run tests that require Ollama
cargo test test_real_model_benchmarks -- --nocapture
```

### Environment-Specific Testing
```bash
# Set environment variables for testing
export OLLAMA_HOST="http://localhost:11434"
export RUST_LOG=debug
export TEST_MODEL="nomic-embed-text"
cargo test -- --nocapture

# Test with specific configuration
export BENCHMARK_ITERATIONS=100
export BASELINE_CONFIDENCE_THRESHOLD=0.95
cargo test test_configurable_benchmarks -- --nocapture
```

### Custom Configuration Testing
```bash
# Test different benchmark configurations
cargo test test_benchmark_config_variations -- --nocapture

# Test baseline configuration options
cargo test test_baseline_config_options -- --nocapture

# Test regression detection sensitivity
cargo test test_regression_detection_config -- --nocapture
```

## Continuous Testing

### Watch Mode for Development
```bash
# Install cargo-watch if not already installed
cargo install cargo-watch

# Run tests automatically on file changes
cargo watch -x test

# Run specific test module on changes
cargo watch -x "test benchmarks"

# Run tests with clear screen
cargo watch -c -x test

# Run tests and benchmarks
cargo watch -x "test -- --nocapture"
```

### Automated Testing Pipeline
```bash
# Run full test suite (suitable for CI/CD)
cargo test --all-targets --all-features

# Run tests with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Run tests with timing information
cargo test -- -Z unstable-options --report-time
```

## Production Monitoring

### Using Built-in Commands in Production
```javascript
// Establish baseline (run once during setup)
async function setupPerformanceBaseline() {
    try {
        const baseline = await invoke('establish_performance_baseline', { 
            operationName: 'embedding_generation' 
        });
        console.log('Performance baseline established:', baseline);
    } catch (error) {
        console.error('Failed to establish baseline:', error);
    }
}

// Regular performance monitoring
async function monitorPerformance() {
    try {
        const results = await invoke('run_embedding_benchmarks');
        const analysis = await invoke('analyze_performance_regressions', { 
            benchmarkResults: results 
        });
        
        if (analysis.total_regressions_detected > 0) {
            console.warn('Performance regressions detected:', analysis);
            
            // Handle critical regressions
            const criticalRegressions = analysis.regressions.filter(
                r => r.severity === 'Critical'
            );
            
            if (criticalRegressions.length > 0) {
                // Alert user or take corrective action
                console.error('CRITICAL performance regressions:', criticalRegressions);
            }
        }
        
        return analysis;
    } catch (error) {
        console.error('Performance monitoring failed:', error);
    }
}

// Schedule regular monitoring (every 5 minutes)
setInterval(monitorPerformance, 300000);

// One-time baseline setup
setupPerformanceBaseline();
```

### Performance Alerts and Thresholds
```javascript
// Configure performance monitoring with custom thresholds
async function configurePerformanceMonitoring() {
    const config = {
        regressionThreshold: 20.0,        // 20% slowdown triggers alert
        memoryThreshold: 100.0,           // 100MB memory increase triggers alert
        statisticalConfidence: 0.95,      // 95% confidence required
        monitoringInterval: 300000        // 5 minutes
    };
    
    return config;
}
```

## Test Output Examples

### Benchmark Results
```
=== EMBEDDING BENCHMARKS REPORT ===

Benchmark Date: 2024-01-15 10:30:45 UTC
Total Operations: 5
Total Execution Time: 2.45 seconds

=== PERFORMANCE SUMMARY ===

Operation: embedding_generation
  Average Duration: 45.2ms
  Memory Usage: 12.5MB
  Success Rate: 100.0%
  Iterations: 100
  
Operation: model_verification
  Average Duration: 1.2s
  Memory Usage: 8.3MB
  Success Rate: 100.0%
  Iterations: 10

=== REGRESSION ANALYSIS ===
✅ No performance regressions detected
📊 All operations within baseline thresholds
🔍 Statistical confidence: 96.4%
```

### Memory Usage Report
```
=== MEMORY USAGE ANALYSIS ===

Peak Memory Usage: 89.3MB
Average Memory Usage: 45.7MB
Memory Growth: 2.1MB/hour
Memory Leaks Detected: 0

Memory Usage by Operation:
  embedding_generation: 12.5MB avg
  model_verification: 8.3MB avg
  baseline_establishment: 15.2MB avg
```

### Regression Detection Report
```
=== PERFORMANCE REGRESSION ANALYSIS REPORT ===

Analysis Date: 2024-01-15 10:30:45 UTC
Operations Analyzed: 3
Regressions Detected: 1
Overall Health: Fair

Regression Summary by Severity:
  🔴 Critical: 0
  🟠 Major: 0
  🟡 Moderate: 1
  🔵 Minor: 0

=== DETAILED REGRESSION ANALYSIS ===

--- embedding_generation (Latency Regression) ---
  Severity: Moderate
  Change: 25.3% (45.2ms → 56.6ms)
  Confidence: 94.2%
  Trend: Degrading
  Detected: 10:30:45
  Recommendation: MODERATE: Latency increased by 25.3%. Monitor trend and consider optimization opportunities.

=== RECOMMENDATIONS ===
• 📉 Operation showing degrading trend - investigate recent changes
• 📊 Consider establishing more frequent baselines for better trend analysis
```

## Troubleshooting

### Common Issues
1. **Tests fail with "Ollama not available"**
   - Ensure Ollama is running: `ollama serve`
   - Check connection: `curl http://localhost:11434/api/tags`

2. **Memory tests show inconsistent results**
   - Run tests with `--test-threads=1` for consistent memory measurements
   - Ensure no other heavy processes are running

3. **Regression detection shows false positives**
   - Adjust detection thresholds in configuration
   - Establish new baselines after significant changes

### Debug Commands
```bash
# Enable debug logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test with full output
cargo test test_name -- --nocapture --exact

# Check test compilation without running
cargo test --no-run
```

## Performance Targets

### Baseline Performance Expectations
- **Benchmark execution**: <5 seconds for comprehensive suite
- **Memory usage**: <100MB additional during benchmarks
- **Statistical confidence**: >95% for regression detection
- **Response time**: <50ms for individual operations
- **Memory overhead**: <10MB for monitoring infrastructure

### Regression Detection Sensitivity
- **Minor regression**: 10-30% performance degradation
- **Moderate regression**: 30-50% performance degradation
- **Major regression**: 50-100% performance degradation
- **Critical regression**: >100% performance degradation

This comprehensive testing infrastructure ensures robust performance monitoring and early detection of issues in the embedding model management system.