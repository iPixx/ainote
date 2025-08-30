/**
 * Comprehensive Performance Regression Detection Tests
 * 
 * Validates automated performance regression detection, baseline comparison,
 * trend analysis, and performance degradation alerting systems.
 * 
 * Part of Issue #176: Performance Testing - Comprehensive validation and benchmarking
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Mock classes for regression detection testing
class MockRegressionDetector {
  constructor(config = {}) {
    this.config = {
      regression_threshold_percent: config.regression_threshold_percent || 20.0,
      improvement_threshold_percent: config.improvement_threshold_percent || 10.0,
      min_samples_for_detection: config.min_samples_for_detection || 5,
      trend_analysis_window: config.trend_analysis_window || 10,
      statistical_significance_level: config.statistical_significance_level || 0.05
    };
    this.baselines = new Map();
    this.historicalData = [];
    this.regressions = [];
  }

  addBaseline(operation, baseline) {
    this.baselines.set(operation, baseline);
  }

  addMeasurement(measurement) {
    this.historicalData.push(measurement);
    // Keep only recent measurements
    if (this.historicalData.length > 1000) {
      this.historicalData = this.historicalData.slice(-500);
    }
  }

  detectRegressions(result) {
    const regressions = [];
    const baseline = this.baselines.get(result.operation_name);
    
    if (!baseline) return regressions;

    // Detect different types of regressions
    regressions.push(...this.detectLatencyRegression(result, baseline));
    regressions.push(...this.detectMemoryRegression(result, baseline));
    regressions.push(...this.detectSuccessRateRegression(result, baseline));
    regressions.push(...this.detectTrendRegression(result));

    return regressions;
  }

  detectLatencyRegression(result, baseline) {
    const changePercent = ((result.avg_duration_ms - baseline.avg_duration_ms) / baseline.avg_duration_ms) * 100;
    
    if (changePercent > this.config.regression_threshold_percent) {
      return [{
        type: 'latency_regression',
        operation: result.operation_name,
        severity: this.calculateSeverity(changePercent),
        change_percent: changePercent,
        baseline_value: baseline.avg_duration_ms,
        current_value: result.avg_duration_ms,
        confidence: this.calculateConfidence(result, baseline),
        detected_at: new Date().toISOString(),
        recommendation: this.generateLatencyRecommendation(changePercent)
      }];
    }
    return [];
  }

  detectMemoryRegression(result, baseline) {
    if (!result.memory_usage_mb || result.memory_usage_mb.length === 0) return [];
    
    const currentMemory = result.memory_usage_mb.reduce((sum, m) => sum + m, 0) / result.memory_usage_mb.length;
    const changePercent = ((currentMemory - baseline.memory_usage_mb) / baseline.memory_usage_mb) * 100;
    
    if (changePercent > this.config.regression_threshold_percent) {
      return [{
        type: 'memory_regression',
        operation: result.operation_name,
        severity: this.calculateSeverity(changePercent),
        change_percent: changePercent,
        baseline_value: baseline.memory_usage_mb,
        current_value: currentMemory,
        confidence: this.calculateConfidence(result, baseline),
        detected_at: new Date().toISOString(),
        recommendation: this.generateMemoryRecommendation(changePercent)
      }];
    }
    return [];
  }

  detectSuccessRateRegression(result, baseline) {
    const changePercent = ((baseline.success_rate - result.success_rate) / baseline.success_rate) * 100;
    
    if (changePercent > this.config.regression_threshold_percent * 0.5) {
      return [{
        type: 'success_rate_regression',
        operation: result.operation_name,
        severity: this.calculateSeverity(changePercent),
        change_percent: changePercent,
        baseline_value: baseline.success_rate,
        current_value: result.success_rate,
        confidence: this.calculateConfidence(result, baseline),
        detected_at: new Date().toISOString(),
        recommendation: this.generateSuccessRateRecommendation(changePercent)
      }];
    }
    return [];
  }

  detectTrendRegression(result) {
    const recentData = this.historicalData
      .filter(d => d.operation_name === result.operation_name)
      .slice(-this.config.trend_analysis_window);

    if (recentData.length < this.config.min_samples_for_detection) return [];

    const trend = this.calculateTrend(recentData);
    
    if (trend.direction === 'degrading' && Math.abs(trend.slope) > 0.5) {
      return [{
        type: 'trend_regression',
        operation: result.operation_name,
        severity: this.calculateTrendSeverity(trend.slope),
        trend_slope: trend.slope,
        trend_direction: trend.direction,
        samples_analyzed: recentData.length,
        confidence: trend.confidence,
        detected_at: new Date().toISOString(),
        recommendation: this.generateTrendRecommendation(trend)
      }];
    }
    return [];
  }

  calculateTrend(data) {
    if (data.length < 2) return { direction: 'stable', slope: 0, confidence: 0 };

    const n = data.length;
    const values = data.map(d => d.duration_ms);
    const indices = Array.from({ length: n }, (_, i) => i);

    const sumX = indices.reduce((sum, x) => sum + x, 0);
    const sumY = values.reduce((sum, y) => sum + y, 0);
    const sumXY = indices.reduce((sum, x, i) => sum + x * values[i], 0);
    const sumXX = indices.reduce((sum, x) => sum + x * x, 0);

    const slope = (n * sumXY - sumX * sumY) / (n * sumXX - sumX * sumX);
    const intercept = (sumY - slope * sumX) / n;

    // Calculate R-squared for confidence
    const yMean = sumY / n;
    const ssRes = values.reduce((sum, y, i) => {
      const predicted = slope * indices[i] + intercept;
      return sum + Math.pow(y - predicted, 2);
    }, 0);
    const ssTot = values.reduce((sum, y) => sum + Math.pow(y - yMean, 2), 0);
    const rSquared = 1 - (ssRes / ssTot);

    return {
      slope,
      direction: slope > 0.1 ? 'degrading' : slope < -0.1 ? 'improving' : 'stable',
      confidence: Math.max(0, rSquared)
    };
  }

  calculateSeverity(changePercent) {
    const absChange = Math.abs(changePercent);
    if (absChange >= 100) return 'critical';
    if (absChange >= 50) return 'major';
    if (absChange >= 30) return 'moderate';
    return 'minor';
  }

  calculateTrendSeverity(slope) {
    const absSlope = Math.abs(slope);
    if (absSlope >= 5) return 'critical';
    if (absSlope >= 2) return 'major';
    if (absSlope >= 1) return 'moderate';
    return 'minor';
  }

  calculateConfidence(result, baseline) {
    const sampleFactor = Math.min(result.iterations / 10, 1.0);
    const baselineFactor = Math.min(baseline.sample_count / 20, 1.0);
    return (sampleFactor * 0.5 + baselineFactor * 0.5);
  }

  generateLatencyRecommendation(changePercent) {
    if (changePercent >= 100) return 'CRITICAL: Investigate blocking operations, network issues, or resource contention immediately.';
    if (changePercent >= 50) return 'MAJOR: Review recent changes, optimize critical paths, or increase timeouts.';
    if (changePercent >= 30) return 'MODERATE: Monitor trend and consider optimization opportunities.';
    return 'MINOR: Continue monitoring for trend development.';
  }

  generateMemoryRecommendation(changePercent) {
    if (changePercent >= 100) return 'CRITICAL: Check for memory leaks, unbounded collections, or resource retention.';
    if (changePercent >= 50) return 'MAJOR: Profile memory allocation patterns and optimize data structures.';
    if (changePercent >= 30) return 'MODERATE: Review recent changes for unnecessary allocations.';
    return 'MINOR: Monitor for continued growth.';
  }

  generateSuccessRateRecommendation(changePercent) {
    if (changePercent >= 50) return 'CRITICAL: Check error handling, network stability, and service dependencies.';
    if (changePercent >= 25) return 'MAJOR: Investigate error patterns and improve retry logic.';
    if (changePercent >= 15) return 'MODERATE: Review error conditions and timeout settings.';
    return 'MINOR: Monitor error patterns for trends.';
  }

  generateTrendRecommendation(trend) {
    return `Performance trend shows ${trend.direction} pattern with slope ${trend.slope.toFixed(2)}ms per measurement. Confidence: ${(trend.confidence * 100).toFixed(1)}%`;
  }

  generateReport(regressions) {
    const totalRegressions = regressions.length;
    const criticalCount = regressions.filter(r => r.severity === 'critical').length;
    const majorCount = regressions.filter(r => r.severity === 'major').length;
    
    return {
      timestamp: new Date().toISOString(),
      total_regressions: totalRegressions,
      critical_regressions: criticalCount,
      major_regressions: majorCount,
      overall_health: criticalCount > 0 ? 'critical' : majorCount > 0 ? 'poor' : 'good',
      regressions: regressions
    };
  }
}

describe('Performance Regression Detection System', () => {
  let detector;
  let mockInvoke;

  beforeEach(() => {
    const { invoke } = setupTauriMocks();
    mockInvoke = invoke;
    
    detector = new MockRegressionDetector({
      regression_threshold_percent: 20.0,
      improvement_threshold_percent: 10.0,
      min_samples_for_detection: 5,
      trend_analysis_window: 10
    });

    // Mock backend regression detection commands
    setupRegressionMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  function setupRegressionMocks() {
    mockInvoke.mockImplementation((command, payload) => {
      switch (command) {
        case 'establish_baseline':
          return Promise.resolve({
            operation_name: payload.operation_name,
            avg_duration_ms: 100 + (Math.random() * 50),
            success_rate: 0.95 + (Math.random() * 0.05),
            memory_usage_mb: 50 + (Math.random() * 20),
            sample_count: 20,
            established_at: new Date().toISOString()
          });

        case 'compare_against_baseline':
          const baseline = detector.baselines.get(payload.operation_name);
          if (!baseline) {
            return Promise.resolve({ baseline_exists: false });
          }
          
          const performanceRatio = payload.current_avg_duration / baseline.avg_duration_ms;
          return Promise.resolve({
            baseline_exists: true,
            meets_baseline: performanceRatio <= 1.2,
            performance_ratio: performanceRatio,
            regression_detected: performanceRatio > 1.2,
            improvement_detected: performanceRatio < 0.9,
            confidence_level: baseline.confidence || 0.9
          });

        case 'detect_performance_regressions':
          return Promise.resolve(detector.generateReport(payload.regressions || []));

        case 'get_regression_history':
          return Promise.resolve({
            historical_regressions: detector.regressions,
            total_count: detector.regressions.length,
            recent_count: detector.regressions.filter(r => 
              new Date() - new Date(r.detected_at) < 24 * 60 * 60 * 1000
            ).length
          });

        default:
          return Promise.resolve({});
      }
    });
  }

  describe('Baseline Establishment and Comparison', () => {
    it('should establish performance baselines for different operations', async () => {
      const operations = ['embedding_generation', 'similarity_search', 'vault_indexing', 'file_operations'];
      const baselines = [];

      for (const operation of operations) {
        const baseline = await mockInvoke('establish_baseline', {
          operation_name: operation,
          sample_count: 20
        });

        baselines.push(baseline);
        detector.addBaseline(operation, baseline);
      }

      // Validate baseline establishment
      expect(baselines.length).toBe(4);
      
      for (const baseline of baselines) {
        expect(baseline.avg_duration_ms).toBeGreaterThan(0);
        expect(baseline.success_rate).toBeGreaterThan(0.9);
        expect(baseline.memory_usage_mb).toBeGreaterThan(0);
        expect(baseline.sample_count).toBe(20);
        expect(baseline.established_at).toBeDefined();
      }

      // Verify baselines are stored
      expect(detector.baselines.size).toBe(4);
      expect(detector.baselines.has('embedding_generation')).toBe(true);
    });

    it('should detect latency regressions against baselines', async () => {
      // Establish baseline
      const baseline = await mockInvoke('establish_baseline', {
        operation_name: 'test_operation',
        sample_count: 15
      });
      detector.addBaseline('test_operation', baseline);

      // Create test result with regression
      const regressedResult = {
        operation_name: 'test_operation',
        avg_duration_ms: baseline.avg_duration_ms * 1.5, // 50% slower
        success_rate: baseline.success_rate,
        memory_usage_mb: [baseline.memory_usage_mb],
        iterations: 10
      };

      const regressions = detector.detectRegressions(regressedResult);
      
      expect(regressions.length).toBeGreaterThan(0);
      
      const latencyRegression = regressions.find(r => r.type === 'latency_regression');
      expect(latencyRegression).toBeDefined();
      expect(latencyRegression.severity).toBe('major'); // 50% increase
      expect(latencyRegression.change_percent).toBeCloseTo(50, 1);
      expect(latencyRegression.recommendation).toContain('MAJOR');
    });

    it('should detect memory usage regressions', async () => {
      // Establish baseline
      const baseline = await mockInvoke('establish_baseline', {
        operation_name: 'memory_test',
        sample_count: 15
      });
      detector.addBaseline('memory_test', baseline);

      // Create result with memory regression
      const memoryRegressedResult = {
        operation_name: 'memory_test',
        avg_duration_ms: baseline.avg_duration_ms,
        success_rate: baseline.success_rate,
        memory_usage_mb: [baseline.memory_usage_mb * 2.2], // 120% increase
        iterations: 10
      };

      const regressions = detector.detectRegressions(memoryRegressedResult);
      const memoryRegression = regressions.find(r => r.type === 'memory_regression');

      expect(memoryRegression).toBeDefined();
      expect(memoryRegression.severity).toBe('critical'); // >100% increase
      expect(memoryRegression.change_percent).toBeCloseTo(120, 5);
      expect(memoryRegression.recommendation).toContain('CRITICAL');
      expect(memoryRegression.recommendation).toContain('memory leaks');
    });

    it('should detect success rate regressions', async () => {
      // Establish baseline
      const baseline = await mockInvoke('establish_baseline', {
        operation_name: 'reliability_test',
        sample_count: 20
      });
      detector.addBaseline('reliability_test', baseline);

      // Create result with success rate regression
      const reliabilityRegressedResult = {
        operation_name: 'reliability_test',
        avg_duration_ms: baseline.avg_duration_ms,
        success_rate: baseline.success_rate * 0.7, // 30% decrease in success rate
        memory_usage_mb: [baseline.memory_usage_mb],
        iterations: 15
      };

      const regressions = detector.detectRegressions(reliabilityRegressedResult);
      const successRegression = regressions.find(r => r.type === 'success_rate_regression');

      expect(successRegression).toBeDefined();
      expect(successRegression.severity).toBe('major');
      expect(successRegression.change_percent).toBeCloseTo(30, 2);
      expect(successRegression.recommendation).toContain('error handling');
    });

    it('should validate statistical confidence in regression detection', async () => {
      // Establish high-confidence baseline
      const baseline = await mockInvoke('establish_baseline', {
        operation_name: 'confidence_test',
        sample_count: 50
      });
      detector.addBaseline('confidence_test', baseline);

      // Test with different sample sizes
      const testCases = [
        { iterations: 5, expectedMinConfidence: 0.3 },
        { iterations: 15, expectedMinConfidence: 0.6 },
        { iterations: 30, expectedMinConfidence: 0.8 }
      ];

      for (const testCase of testCases) {
        const result = {
          operation_name: 'confidence_test',
          avg_duration_ms: baseline.avg_duration_ms * 1.3, // 30% regression
          success_rate: baseline.success_rate,
          memory_usage_mb: [baseline.memory_usage_mb],
          iterations: testCase.iterations
        };

        const regressions = detector.detectRegressions(result);
        const regression = regressions.find(r => r.type === 'latency_regression');

        expect(regression).toBeDefined();
        expect(regression.confidence).toBeGreaterThanOrEqual(testCase.expectedMinConfidence);
      }
    });
  });

  describe('Trend Analysis and Detection', () => {
    it('should detect degrading performance trends', async () => {
      const operation = 'trend_analysis_test';
      
      // Add historical data showing degrading trend
      const baseTime = 100;
      for (let i = 0; i < 10; i++) {
        detector.addMeasurement({
          operation_name: operation,
          duration_ms: baseTime + (i * 5), // Increasing by 5ms each measurement
          timestamp: new Date(Date.now() - (9 - i) * 60000).toISOString()
        });
      }

      // Test current measurement
      const currentResult = {
        operation_name: operation,
        avg_duration_ms: baseTime + 45, // Continuing the trend
        success_rate: 0.95,
        memory_usage_mb: [50],
        iterations: 10
      };

      const regressions = detector.detectRegressions(currentResult);
      const trendRegression = regressions.find(r => r.type === 'trend_regression');

      expect(trendRegression).toBeDefined();
      expect(trendRegression.trend_direction).toBe('degrading');
      expect(trendRegression.trend_slope).toBeGreaterThan(0);
      expect(trendRegression.samples_analyzed).toBe(10);
      expect(trendRegression.confidence).toBeGreaterThan(0.7);
    });

    it('should analyze performance trends over different time windows', async () => {
      const operation = 'window_analysis_test';
      
      // Create performance data with different patterns in different windows
      // First 20 measurements: stable performance
      for (let i = 0; i < 20; i++) {
        detector.addMeasurement({
          operation_name: operation,
          duration_ms: 100 + (Math.random() * 5), // Stable around 100ms
          timestamp: new Date(Date.now() - (39 - i) * 60000).toISOString()
        });
      }
      
      // Last 10 measurements: degrading performance
      for (let i = 0; i < 10; i++) {
        detector.addMeasurement({
          operation_name: operation,
          duration_ms: 105 + (i * 3), // Increasing trend
          timestamp: new Date(Date.now() - (9 - i) * 60000).toISOString()
        });
      }

      // Different window sizes should detect the recent degradation
      const windowSizes = [5, 10, 15];
      const trendResults = [];

      for (const windowSize of windowSizes) {
        const testDetector = new MockRegressionDetector({
          trend_analysis_window: windowSize
        });
        testDetector.historicalData = detector.historicalData;

        const currentResult = {
          operation_name: operation,
          avg_duration_ms: 135, // Recent high value
          success_rate: 0.95,
          memory_usage_mb: [50],
          iterations: 10
        };

        const regressions = testDetector.detectRegressions(currentResult);
        const trendRegression = regressions.find(r => r.type === 'trend_regression');

        trendResults.push({
          window_size: windowSize,
          regression_detected: !!trendRegression,
          trend_slope: trendRegression ? trendRegression.trend_slope : 0,
          confidence: trendRegression ? trendRegression.confidence : 0
        });
      }

      // Smaller windows should detect recent degradation more clearly
      const smallWindow = trendResults.find(r => r.window_size === 5);
      const largeWindow = trendResults.find(r => r.window_size === 15);

      expect(smallWindow.regression_detected).toBe(true);
      expect(smallWindow.trend_slope).toBeGreaterThan(largeWindow.trend_slope);
    });

    it('should handle volatile performance data appropriately', async () => {
      const operation = 'volatility_test';
      
      // Add highly volatile but not trending data
      const baseTime = 100;
      for (let i = 0; i < 15; i++) {
        const volatileTime = baseTime + (Math.random() - 0.5) * 40; // ±20ms variance
        detector.addMeasurement({
          operation_name: operation,
          duration_ms: volatileTime,
          timestamp: new Date(Date.now() - (14 - i) * 60000).toISOString()
        });
      }

      const currentResult = {
        operation_name: operation,
        avg_duration_ms: baseTime + 15, // Slightly higher but within volatility range
        success_rate: 0.95,
        memory_usage_mb: [50],
        iterations: 10
      };

      const regressions = detector.detectRegressions(currentResult);
      const trendRegression = regressions.find(r => r.type === 'trend_regression');

      // Should not detect significant trend in highly volatile data
      if (trendRegression) {
        expect(trendRegression.confidence).toBeLessThan(0.6);
        expect(Math.abs(trendRegression.trend_slope)).toBeLessThan(1);
      }
    });
  });

  describe('Comprehensive Regression Analysis', () => {
    it('should perform multi-operation regression analysis', async () => {
      const operations = [
        'embedding_generation',
        'similarity_search', 
        'vault_indexing',
        'file_operations',
        'ai_processing'
      ];

      // Establish baselines for all operations
      const baselines = [];
      for (const operation of operations) {
        const baseline = await mockInvoke('establish_baseline', {
          operation_name: operation,
          sample_count: 20
        });
        baselines.push(baseline);
        detector.addBaseline(operation, baseline);
      }

      // Create test results with various regression scenarios
      const testResults = [
        {
          operation_name: 'embedding_generation',
          avg_duration_ms: baselines[0].avg_duration_ms * 1.1, // Minor increase
          success_rate: baselines[0].success_rate,
          memory_usage_mb: [baselines[0].memory_usage_mb],
          iterations: 15
        },
        {
          operation_name: 'similarity_search',
          avg_duration_ms: baselines[1].avg_duration_ms * 1.6, // Major regression
          success_rate: baselines[1].success_rate * 0.8, // Success rate regression
          memory_usage_mb: [baselines[1].memory_usage_mb * 1.3],
          iterations: 12
        },
        {
          operation_name: 'vault_indexing',
          avg_duration_ms: baselines[2].avg_duration_ms * 0.8, // Improvement
          success_rate: baselines[2].success_rate,
          memory_usage_mb: [baselines[2].memory_usage_mb],
          iterations: 18
        },
        {
          operation_name: 'file_operations',
          avg_duration_ms: baselines[3].avg_duration_ms * 2.5, // Critical regression
          success_rate: baselines[3].success_rate,
          memory_usage_mb: [baselines[3].memory_usage_mb * 1.8],
          iterations: 10
        },
        {
          operation_name: 'ai_processing',
          avg_duration_ms: baselines[4].avg_duration_ms * 1.05, // Within threshold
          success_rate: baselines[4].success_rate,
          memory_usage_mb: [baselines[4].memory_usage_mb],
          iterations: 20
        }
      ];

      // Analyze all results
      const allRegressions = [];
      for (const result of testResults) {
        const regressions = detector.detectRegressions(result);
        allRegressions.push(...regressions);
      }

      const report = detector.generateReport(allRegressions);

      // Validate comprehensive analysis
      expect(report.total_regressions).toBeGreaterThan(0);
      expect(report.critical_regressions).toBeGreaterThan(0); // file_operations
      expect(report.major_regressions).toBeGreaterThan(0); // similarity_search
      expect(report.overall_health).toBe('critical'); // Due to critical regressions

      // Verify specific regression detections
      const criticalRegressions = allRegressions.filter(r => r.severity === 'critical');
      const majorRegressions = allRegressions.filter(r => r.severity === 'major');
      
      expect(criticalRegressions.length).toBeGreaterThan(0);
      expect(majorRegressions.length).toBeGreaterThan(0);

      // Check that improvements and minor changes are handled appropriately
      const vaultIndexingRegressions = allRegressions.filter(r => r.operation === 'vault_indexing');
      const aiProcessingRegressions = allRegressions.filter(r => r.operation === 'ai_processing');
      
      expect(vaultIndexingRegressions.length).toBe(0); // Improvement, no regression
      expect(aiProcessingRegressions.length).toBe(0); // Within threshold, no regression
    });

    it('should provide actionable regression recommendations', async () => {
      // Setup different regression scenarios
      const regressionScenarios = [
        {
          type: 'latency',
          severity: 'critical',
          change_percent: 150,
          expectedKeywords: ['CRITICAL', 'blocking operations', 'resource contention']
        },
        {
          type: 'memory',
          severity: 'major',
          change_percent: 75,
          expectedKeywords: ['MAJOR', 'memory allocation', 'data structures']
        },
        {
          type: 'success_rate',
          severity: 'moderate',
          change_percent: 20,
          expectedKeywords: ['error conditions', 'timeout settings']
        }
      ];

      const recommendations = [];
      
      for (const scenario of regressionScenarios) {
        let recommendation;
        
        if (scenario.type === 'latency') {
          recommendation = detector.generateLatencyRecommendation(scenario.change_percent);
        } else if (scenario.type === 'memory') {
          recommendation = detector.generateMemoryRecommendation(scenario.change_percent);
        } else if (scenario.type === 'success_rate') {
          recommendation = detector.generateSuccessRateRecommendation(scenario.change_percent);
        }
        
        recommendations.push({
          scenario: scenario.type,
          severity: scenario.severity,
          recommendation
        });

        // Validate recommendation content
        expect(recommendation).toBeDefined();
        expect(recommendation.length).toBeGreaterThan(20); // Meaningful recommendation
        
        for (const keyword of scenario.expectedKeywords) {
          expect(recommendation.toLowerCase()).toContain(keyword.toLowerCase());
        }
      }

      // Verify recommendations are severity-appropriate
      const criticalRec = recommendations.find(r => r.severity === 'critical');
      const majorRec = recommendations.find(r => r.severity === 'major');
      const moderateRec = recommendations.find(r => r.severity === 'moderate');

      expect(criticalRec.recommendation).toContain('CRITICAL');
      expect(majorRec.recommendation).toContain('MAJOR');
      expect(moderateRec.recommendation.length).toBeLessThan(criticalRec.recommendation.length);
    });

    it('should track regression history and patterns', async () => {
      const operation = 'history_tracking_test';
      
      // Establish baseline
      const baseline = await mockInvoke('establish_baseline', {
        operation_name: operation,
        sample_count: 15
      });
      detector.addBaseline(operation, baseline);

      // Simulate multiple regression detections over time
      const regressionHistory = [];
      const timeIntervals = [0, 3600000, 7200000, 10800000]; // 0h, 1h, 2h, 3h ago

      for (let i = 0; i < timeIntervals.length; i++) {
        const result = {
          operation_name: operation,
          avg_duration_ms: baseline.avg_duration_ms * (1.2 + (i * 0.1)), // Progressively worse
          success_rate: baseline.success_rate - (i * 0.05), // Declining success rate
          memory_usage_mb: [baseline.memory_usage_mb * (1.1 + (i * 0.1))],
          iterations: 10
        };

        const regressions = detector.detectRegressions(result);
        
        // Simulate timestamping regressions
        regressions.forEach(regression => {
          regression.detected_at = new Date(Date.now() - timeIntervals[i]).toISOString();
          regressionHistory.push(regression);
        });
      }

      detector.regressions = regressionHistory;

      // Get regression history
      const history = await mockInvoke('get_regression_history');

      expect(history.total_count).toBe(regressionHistory.length);
      expect(history.historical_regressions).toHaveLength(regressionHistory.length);

      // Analyze patterns
      const latencyRegressions = history.historical_regressions.filter(r => r.type === 'latency_regression');
      const memoryRegressions = history.historical_regressions.filter(r => r.type === 'memory_regression');

      expect(latencyRegressions.length).toBeGreaterThan(0);
      expect(memoryRegressions.length).toBeGreaterThan(0);

      // Check severity progression
      const severityOrder = ['minor', 'moderate', 'major', 'critical'];
      let foundProgression = false;
      
      for (let i = 0; i < latencyRegressions.length - 1; i++) {
        const currentIndex = severityOrder.indexOf(latencyRegressions[i].severity);
        const nextIndex = severityOrder.indexOf(latencyRegressions[i + 1].severity);
        
        if (nextIndex > currentIndex) {
          foundProgression = true;
          break;
        }
      }
      
      expect(foundProgression).toBe(true); // Should show progression from minor to more severe
    });
  });

  describe('Integration with Existing Performance Monitoring', () => {
    it('should integrate with baseline establishment system', async () => {
      const operations = ['integrated_test_1', 'integrated_test_2'];
      
      // Test integration with baseline establishment
      for (const operation of operations) {
        const baselineResult = await mockInvoke('establish_baseline', {
          operation_name: operation,
          sample_count: 25
        });

        expect(baselineResult.operation_name).toBe(operation);
        expect(baselineResult.sample_count).toBe(25);
        expect(baselineResult.established_at).toBeDefined();

        // Store in detector for later comparison
        detector.addBaseline(operation, baselineResult);
      }

      // Test comparison integration
      const testResult = {
        operation_name: 'integrated_test_1',
        avg_duration_ms: detector.baselines.get('integrated_test_1').avg_duration_ms * 1.4,
        success_rate: 0.92,
        memory_usage_mb: [65],
        iterations: 15
      };

      const comparisonResult = await mockInvoke('compare_against_baseline', {
        operation_name: 'integrated_test_1',
        current_avg_duration: testResult.avg_duration_ms
      });

      expect(comparisonResult.baseline_exists).toBe(true);
      expect(comparisonResult.regression_detected).toBe(true);
      expect(comparisonResult.performance_ratio).toBeCloseTo(1.4, 1);
      expect(comparisonResult.meets_baseline).toBe(false);
    });

    it('should generate comprehensive regression reports', async () => {
      // Create mock regressions for report testing
      const mockRegressions = [
        {
          type: 'latency_regression',
          operation: 'report_test_1',
          severity: 'critical',
          change_percent: 120,
          confidence: 0.95,
          detected_at: new Date().toISOString()
        },
        {
          type: 'memory_regression', 
          operation: 'report_test_2',
          severity: 'major',
          change_percent: 60,
          confidence: 0.88,
          detected_at: new Date().toISOString()
        },
        {
          type: 'trend_regression',
          operation: 'report_test_3',
          severity: 'moderate',
          trend_slope: 2.5,
          confidence: 0.75,
          detected_at: new Date().toISOString()
        }
      ];

      const report = await mockInvoke('detect_performance_regressions', {
        regressions: mockRegressions
      });

      expect(report.total_regressions).toBe(3);
      expect(report.critical_regressions).toBe(1);
      expect(report.major_regressions).toBe(1);
      expect(report.overall_health).toBe('critical');
      expect(report.timestamp).toBeDefined();
      expect(report.regressions).toHaveLength(3);

      // Validate report structure
      for (const regression of report.regressions) {
        expect(regression.type).toBeDefined();
        expect(regression.operation).toBeDefined();
        expect(regression.severity).toBeDefined();
        expect(regression.confidence).toBeGreaterThan(0);
        expect(regression.detected_at).toBeDefined();
      }
    });

    it('should handle missing baselines gracefully', async () => {
      const result = {
        operation_name: 'no_baseline_operation',
        avg_duration_ms: 150,
        success_rate: 0.95,
        memory_usage_mb: [45],
        iterations: 10
      };

      // Test regression detection without baseline
      const regressions = detector.detectRegressions(result);
      expect(regressions.length).toBe(0); // No regressions without baseline

      // Test backend comparison without baseline
      const comparisonResult = await mockInvoke('compare_against_baseline', {
        operation_name: 'no_baseline_operation',
        current_avg_duration: result.avg_duration_ms
      });

      expect(comparisonResult.baseline_exists).toBe(false);
      expect(comparisonResult.regression_detected).toBe(false);
      expect(comparisonResult.improvement_detected).toBe(false);
    });
  });
});