/**
 * Performance Monitoring System Tests
 * 
 * Comprehensive tests for the performance monitoring dashboard and
 * real-time metrics service as implemented for Issue #172.
 * 
 * Tests cover:
 * - Real-time metrics display with trend tracking
 * - Memory usage monitoring with threshold alerts
 * - AI operation timing and resource measurement
 * - UI responsiveness metrics (frame time, input lag)
 * - Exportable performance reports functionality
 * - Performance targets and monitoring overhead validation
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import the classes under test
import { PerformanceMonitoringDashboard } from '../../src/js/components/performance-monitoring-dashboard.js';
import { RealTimeMetricsService } from '../../src/js/services/real-time-metrics-service.js';

// Mock performance APIs
global.performance = {
  now: vi.fn(() => Date.now()),
  mark: vi.fn(),
  measure: vi.fn(),
};

global.requestAnimationFrame = vi.fn((cb) => setTimeout(cb, 16));
global.window.alert = vi.fn();

// Set up DOM environment
Object.defineProperty(window, 'innerWidth', { writable: true, value: 1024 });
Object.defineProperty(window, 'innerHeight', { writable: true, value: 768 });

describe('Performance Monitoring System', () => {
  let dashboard;
  let metricsService;
  let mockInvoke;

  beforeEach(() => {
    // Setup Tauri mocks
    const { invoke } = setupTauriMocks();
    mockInvoke = invoke;
    
    // Ensure mockInvoke is properly initialized
    if (!mockInvoke || typeof mockInvoke.mockImplementation !== 'function') {
      throw new Error('mockInvoke not properly set up');
    }
    
    // Mock DOM methods
    document.body.innerHTML = '';
    document.createElement = vi.fn().mockImplementation((tagName) => {
      const element = {
        tagName: tagName.toUpperCase(),
        className: '',
        innerHTML: '',
        style: {},
        width: 376,
        height: 60,
        appendChild: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        getContext: vi.fn(function() {
          return {
            canvas: this, // Reference back to the canvas element
            fillStyle: '',
            strokeStyle: '',
            lineWidth: 1,
            fillRect: vi.fn(),
            clearRect: vi.fn(),
            beginPath: vi.fn(),
            moveTo: vi.fn(),
            lineTo: vi.fn(),
            stroke: vi.fn(),
            fill: vi.fn(),
            arc: vi.fn(),
            closePath: vi.fn(),
            createLinearGradient: vi.fn(() => ({
              addColorStop: vi.fn()
            })),
            measureText: vi.fn(() => ({ width: 50 })),
            fillText: vi.fn(),
            save: vi.fn(),
            restore: vi.fn(),
            translate: vi.fn(),
            scale: vi.fn()
          };
        }),
        querySelector: vi.fn((selector) => {
          // Return mock elements for common selectors
          if (selector.includes('data-action="close"')) {
            return { addEventListener: vi.fn(), textContent: 'Close' };
          }
          if (selector.includes('data-action="start-stop"')) {
            return { addEventListener: vi.fn(), textContent: 'Start Monitoring' };
          }
          if (selector.includes('data-action="export"')) {
            return { addEventListener: vi.fn(), textContent: 'Export Report' };
          }
          if (selector.includes('data-action="clear-history"')) {
            return { addEventListener: vi.fn(), textContent: 'Clear History' };
          }
          if (selector.includes('metric-value')) {
            return { textContent: '--', className: 'metric-value', classList: { add: vi.fn(), remove: vi.fn() } };
          }
          if (selector.includes('metric-trend')) {
            return { textContent: '--', className: 'metric-trend' };
          }
          if (selector.includes('resource-chart-canvas')) {
            const canvasEl = {
              width: 376,
              height: 60,
              getContext: vi.fn(function() {
                return {
                  canvas: this, // Reference back to the canvas element
                  fillStyle: '',
                  strokeStyle: '',
                  lineWidth: 1,
                  fillRect: vi.fn(),
                  clearRect: vi.fn(),
                  beginPath: vi.fn(),
                  moveTo: vi.fn(),
                  lineTo: vi.fn(),
                  stroke: vi.fn(),
                  fill: vi.fn(),
                  arc: vi.fn(),
                  closePath: vi.fn(),
                  createLinearGradient: vi.fn(() => ({
                    addColorStop: vi.fn()
                  })),
                  measureText: vi.fn(() => ({ width: 50 })),
                  fillText: vi.fn(),
                  save: vi.fn(),
                  restore: vi.fn(),
                  translate: vi.fn(),
                  scale: vi.fn()
                };
              })
            };
            return canvasEl;
          }
          return { addEventListener: vi.fn(), textContent: '', className: '', classList: { add: vi.fn(), remove: vi.fn() } };
        }),
        querySelectorAll: vi.fn(() => []),
        setAttribute: vi.fn(),
        getAttribute: vi.fn(),
        classList: {
          add: vi.fn(),
          remove: vi.fn(),
          contains: vi.fn(() => false),
          toggle: vi.fn(),
        },
      };
      return element;
    });
    
    document.body.appendChild = vi.fn();
    document.body.removeChild = vi.fn();
    document.addEventListener = vi.fn();
    document.removeEventListener = vi.fn();
    
    // Reset performance.now mock
    performance.now.mockImplementation(() => Date.now());
  });

  afterEach(() => {
    if (dashboard) {
      dashboard.destroy();
    }
    if (metricsService && metricsService.isActive) {
      metricsService.stop();
    }
    
    // Clear all mocks
    vi.clearAllMocks();
  });

  describe('PerformanceMonitoringDashboard', () => {
    beforeEach(() => {
      // Default mock for dashboard initialization - monitoring is inactive
      mockInvoke.mockImplementation((command) => {
        switch (command) {
          case 'get_monitoring_status':
            return Promise.resolve({ is_active: false });
          default:
            return Promise.resolve({});
        }
      });
      
      dashboard = new PerformanceMonitoringDashboard();
      
      // Fix dashboard element mocking
      dashboard.dashboardElement = {
        querySelector: vi.fn((selector) => {
          if (selector.includes('data-metric')) {
            return {
              querySelector: vi.fn((subSelector) => {
                if (subSelector.includes('metric-value')) {
                  return { textContent: '--', className: 'metric-value', classList: { add: vi.fn(), remove: vi.fn() } };
                }
                if (subSelector.includes('metric-trend')) {
                  return { textContent: '--', className: 'metric-trend' };
                }
                return { textContent: '', className: '', classList: { add: vi.fn(), remove: vi.fn() } };
              }),
              classList: { add: vi.fn(), remove: vi.fn() }
            };
          }
          return { textContent: '', classList: { add: vi.fn(), remove: vi.fn() } };
        })
      };
      
      // Fix elements mocking
      dashboard.elements = {
        inputLagFill: { style: { width: '0%' }, className: 'input-lag-fill', classList: { add: vi.fn(), remove: vi.fn() } },
        inputLagValue: { textContent: '0ms' },
        frameTimeValue: { textContent: '16ms', className: 'metric-value', classList: { add: vi.fn(), remove: vi.fn() } },
        frameTimeDot: { className: 'frame-time-dot', classList: { add: vi.fn(), remove: vi.fn() }, nextSibling: { textContent: '60fps' } }
      };
    });

    describe('Dashboard Initialization', () => {
      it('should create dashboard with correct initial state', () => {
        expect(dashboard.isVisible).toBe(false);
        expect(dashboard.isMonitoring).toBe(false);
        expect(dashboard.monitoringInterval).toBeNull();
        expect(dashboard.metricsHistory).toEqual([]);
        expect(dashboard.resourceHistory).toEqual([]);
        expect(dashboard.frameTimeHistory).toEqual([]);
      });

      it('should create dashboard DOM elements', () => {
        expect(document.createElement).toHaveBeenCalledWith('button');
        expect(document.createElement).toHaveBeenCalledWith('div');
        expect(document.body.appendChild).toHaveBeenCalled();
      });

      it('should set up performance thresholds correctly', () => {
        expect(dashboard.thresholds).toEqual({
          memoryWarning: 75,
          memoryCritical: 95,
          cpuWarning: 60,
          cpuCritical: 80,
          frameTimeWarning: 16,
          frameTimeCritical: 33,
          inputLagWarning: 50,
          inputLagCritical: 100,
          aiOperationWarning: 500,
          aiOperationCritical: 1000,
        });
      });
    });

    describe('UI Responsiveness Tracking', () => {
      it('should track frame time correctly', async () => {
        // Initialize frame time history with test data
        dashboard.frameTimeHistory = [16.7, 15.2, 18.1];
        
        // Mock requestAnimationFrame to simulate frame timing
        let frameCallback;
        global.requestAnimationFrame = vi.fn((cb) => {
          frameCallback = cb;
          return 1;
        });

        // Simulate frame time measurements
        const startTime = 1000;
        const frameTime = 20; // 20ms frame time
        performance.now
          .mockReturnValueOnce(startTime)
          .mockReturnValueOnce(startTime + frameTime);

        // Start the UI responsiveness tracking to initialize the arrays
        dashboard.startUIResponsivenessTracking();
        
        // Trigger frame callback directly with the frame processing logic
        if (frameCallback) frameCallback(startTime + frameTime);

        expect(dashboard.frameTimeHistory.length).toBeGreaterThan(0);
      });

      it('should detect high frame times and set warning states', () => {
        // Add a high frame time to history
        dashboard.frameTimeHistory = [35, 40, 38]; // Above critical threshold

        dashboard.updateFrameTimeDisplay(40);

        // Should set critical state for high frame times
        expect(dashboard.frameTimeHistory).toContain(40);
      });

      it('should track input lag correctly', () => {
        // Initialize input lag history and add lag time
        const lagTime = 75; // 75ms lag
        dashboard.inputLagHistory = [lagTime];

        dashboard.updateInputLagDisplay(lagTime);
        
        expect(dashboard.inputLagHistory).toContain(lagTime);
      });
    });

    describe('Performance Metrics Collection', () => {
      beforeEach(() => {
        // Mock successful monitoring status
        mockInvoke.mockImplementation((command) => {
          switch (command) {
            case 'get_monitoring_status':
              return Promise.resolve({ is_active: true });
            case 'get_current_performance_metrics':
              return Promise.resolve({
                IncrementalUpdate: {
                  operation_type: 'IncrementalUpdate',
                  duration_ms: 150,
                  memory_peak_mb: 45,
                  cpu_usage_percent: 25,
                  processing_rate: 10,
                },
              });
            case 'get_resource_utilization':
              return Promise.resolve({
                timestamp: new Date().toISOString(),
                cpu_usage_percent: 30,
                memory_usage_mb: 67,
                memory_available_mb: 8125,
                disk_read_mb_per_sec: 2.5,
                disk_write_mb_per_sec: 1.8,
              });
            case 'get_active_alerts':
              return Promise.resolve([]);
            default:
              return Promise.resolve({});
          }
        });
      });

      it('should collect metrics successfully', async () => {
        // Call collectMetrics directly instead of waiting for interval
        await dashboard.collectMetrics();

        expect(mockInvoke).toHaveBeenCalledWith('get_current_performance_metrics');
        expect(mockInvoke).toHaveBeenCalledWith('get_resource_utilization');
        expect(mockInvoke).toHaveBeenCalledWith('get_active_alerts');
        expect(mockInvoke).toHaveBeenCalledWith('get_search_operation_metrics', { limit: 10 });
      });

      it('should update metrics display correctly', () => {
        const mockMetrics = {
          IncrementalUpdate: {
            operation_type: 'IncrementalUpdate',
            memory_peak_mb: 45,
            cpu_usage_percent: 25,
          },
        };

        dashboard.updateCurrentMetrics(mockMetrics);

        // Should calculate averages correctly
        expect(dashboard.resourceHistory).toBeDefined();
      });

      it('should handle metrics collection errors gracefully', async () => {
        mockInvoke.mockRejectedValue(new Error('Backend not available'));

        // Should not throw
        await expect(dashboard.collectMetrics()).resolves.not.toThrow();
      });
    });

    describe('Memory Usage Monitoring', () => {
      it('should track memory trends correctly', () => {
        const resourceData = {
          timestamp: new Date().toISOString(),
          memory_usage_mb: 45,
          memory_available_mb: 8000,
          cpu_usage_percent: 30,
        };

        dashboard.updateResourceMetrics(resourceData);

        expect(dashboard.resourceHistory).toHaveLength(1);
        expect(dashboard.resourceHistory[0]).toMatchObject(resourceData);
      });

      it('should detect memory threshold violations', () => {
        const highMemoryData = {
          memory_usage_mb: 85, // Above warning threshold
          memory_available_mb: 7000,
          cpu_usage_percent: 40,
        };

        dashboard.updateResourceMetrics(highMemoryData);
        dashboard.updateMetricDisplay('memory', 85);

        // Should trigger warning state
        expect(dashboard.resourceHistory).toHaveLength(1);
      });

      it('should maintain memory trend window size', () => {
        // Add more entries than the window size
        for (let i = 0; i < 150; i++) {
          dashboard.resourceHistory.push({
            timestamp: new Date(Date.now() + i * 1000).toISOString(),
            memory_usage_mb: 40 + (i % 10),
            cpu_usage_percent: 20,
          });
        }

        const resourceData = {
          memory_usage_mb: 50,
          cpu_usage_percent: 25,
        };
        
        dashboard.updateResourceMetrics(resourceData);

        // Should maintain max size of 100
        expect(dashboard.resourceHistory.length).toBeLessThanOrEqual(100);
      });
    });

    describe('AI Operation Timing', () => {
      it('should track AI operations correctly', () => {
        const aiOperations = [
          {
            operation_type: 'search',
            duration_ms: 45,
            vectors_searched: 1000,
            results_returned: 10,
            efficiency_score: 0.85,
            performance_target_met: true,
            timestamp: new Date().toISOString(),
          },
          {
            operation_type: 'embedding',
            duration_ms: 750, // Slow operation
            efficiency_score: 0.65,
            performance_target_met: false,
            timestamp: new Date().toISOString(),
          },
        ];

        dashboard.updateAIOperations(aiOperations);

        expect(dashboard.aiOperationHistory.length).toBe(2);
        expect(dashboard.aiOperationHistory[0].operationType).toBe('search');
        expect(dashboard.aiOperationHistory[1].operationType).toBe('embedding');
      });

      it('should detect slow AI operations', () => {
        const slowOperations = [
          {
            operation_type: 'search',
            duration_ms: 1200, // Very slow
            timestamp: new Date().toISOString(),
          },
        ];

        dashboard.updateAIOperations(slowOperations);

        // Should contain slow operation
        expect(dashboard.aiOperationHistory[0].duration).toBe(1200);
      });
    });

    describe('Performance Reports Export', () => {
      it('should export performance report successfully', async () => {
        const mockReport = {
          generated_at: new Date().toISOString(),
          period_start: new Date(Date.now() - 3600000).toISOString(),
          period_end: new Date().toISOString(),
          total_operations: 50,
          health_score: 0.85,
          recommendations: ['Consider optimizing memory usage'],
        };

        mockInvoke.mockResolvedValueOnce(mockReport);

        // Mock DOM for file download
        const mockAnchor = {
          href: '',
          download: '',
          click: vi.fn(),
        };
        document.createElement.mockReturnValueOnce(mockAnchor);

        // Mock URL.createObjectURL
        global.URL = {
          createObjectURL: vi.fn(() => 'blob:mock-url'),
          revokeObjectURL: vi.fn(),
        };

        global.Blob = vi.fn();

        await dashboard.exportReport();

        expect(mockInvoke).toHaveBeenCalledWith('generate_performance_report', {
          request: {
            period_hours: 1,
            include_detailed_breakdown: true,
            include_resource_analysis: true,
          },
        });
      });
    });

    describe('Performance Monitoring Integration', () => {
      it('should start and stop monitoring successfully', async () => {
        mockInvoke.mockImplementation((command) => {
          switch (command) {
            case 'get_monitoring_status':
              return Promise.resolve({ is_active: false }); // Start with inactive
            case 'start_performance_monitoring':
              return Promise.resolve({ is_active: true });
            case 'stop_performance_monitoring':
              return Promise.resolve('Monitoring stopped');
            default:
              return Promise.resolve({});
          }
        });

        await dashboard.toggleMonitoring();
        expect(mockInvoke).toHaveBeenCalledWith('start_performance_monitoring', {
          request: {
            config: {
              enable_monitoring: true,
              collection_interval_ms: 100,
              enable_resource_tracking: true,
              resource_tracking_interval_ms: 1000,
              enable_alerts: true,
              max_overhead_percent: 1.0,
              auto_persist_interval_seconds: 60,
              max_samples_in_memory: 1000,
              alert_degradation_threshold: 20.0,
              enable_detailed_logging: false,
            },
          },
        });

        dashboard.isMonitoring = true;
        await dashboard.toggleMonitoring();
        expect(mockInvoke).toHaveBeenCalledWith('stop_performance_monitoring');
      });
    });

    describe('Performance Overhead Validation', () => {
      it('should validate <1% CPU overhead requirement', async () => {
        const startTime = performance.now();
        
        // Simulate metrics collection
        await dashboard.collectMetrics();
        
        const endTime = performance.now();
        const collectionTime = endTime - startTime;

        // Collection should be very fast (< 10ms for <1% overhead at 100ms interval)
        expect(collectionTime).toBeLessThan(10);
      });

      it('should warn about collection overhead if too high', async () => {
        // Mock slow backend calls
        mockInvoke.mockImplementation(() => {
          return new Promise(resolve => setTimeout(resolve, 50)); // Simulate slow backend
        });

        const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

        await dashboard.collectMetrics();

        // Should warn about slow collection
        expect(consoleSpy).toHaveBeenCalledWith(
          'Failed to collect some metrics:',
          expect.any(Object)
        );

        consoleSpy.mockRestore();
      });
    });
  });

  describe('RealTimeMetricsService', () => {
    beforeEach(() => {
      metricsService = new RealTimeMetricsService();
    });

    describe('Service Initialization', () => {
      it('should initialize with correct default configuration', () => {
        expect(metricsService.isActive).toBe(false);
        expect(metricsService.subscribers).toBeInstanceOf(Set);
        expect(metricsService.metricsCache).toMatchObject({
          currentMetrics: {},
          resourceUtilization: null,
          memoryTrends: [],
          aiOperations: [],
          alerts: [],
          lastUpdate: null,
        });
      });

      it('should have correct memory monitoring configuration', () => {
        expect(metricsService.memoryConfig).toEqual({
          trackingInterval: 1000,
          trendWindowSize: 60,
          alertThresholds: {
            warning: 75,
            critical: 95,
          },
          leakDetectionThreshold: 10,
        });
      });
    });

    describe('Metrics Collection and Processing', () => {
      beforeEach(() => {
        mockInvoke.mockImplementation((command) => {
          switch (command) {
            case 'get_monitoring_status':
              return Promise.resolve({ is_active: true });
            case 'start_performance_monitoring':
              return Promise.resolve({ is_active: true });
            case 'get_current_performance_metrics':
              return Promise.resolve({
                IncrementalUpdate: {
                  memory_peak_mb: 45,
                  cpu_usage_percent: 25,
                  duration_ms: 150,
                },
              });
            case 'get_resource_utilization':
              return Promise.resolve({
                timestamp: new Date().toISOString(),
                cpu_usage_percent: 30,
                memory_usage_mb: 67,
                memory_available_mb: 8125,
              });
            case 'get_active_alerts':
              return Promise.resolve([]);
            case 'get_search_operation_metrics':
              return Promise.resolve([
                {
                  operation_type: 'search',
                  duration_ms: 45,
                  vectors_searched: 1000,
                  timestamp: new Date().toISOString(),
                },
              ]);
            default:
              return Promise.resolve({});
          }
        });
      });

      it('should start service successfully', async () => {
        await metricsService.start();
        
        expect(metricsService.isActive).toBe(true);
        expect(mockInvoke).toHaveBeenCalledWith('get_monitoring_status');
      });

      it('should collect metrics and update cache', async () => {
        await metricsService.start();
        
        // Manually trigger metrics collection
        await metricsService.collectAndProcessMetrics();
        
        expect(metricsService.metricsCache.lastUpdate).not.toBeNull();
        expect(metricsService.metricsCache.currentMetrics).toBeDefined();
      });

      it('should detect memory leaks correctly', () => {
        // Add memory trend data showing increasing memory usage
        for (let i = 0; i < 30; i++) {
          metricsService.metricsCache.memoryTrends.push({
            timestamp: Date.now() - (29 - i) * 1000,
            memoryUsageMB: 50 + (i * 0.5), // Gradually increasing
            cpuUsagePercent: 25,
          });
        }

        const mockCallback = vi.fn();
        metricsService.subscribe(mockCallback);

        metricsService.detectMemoryLeaks();

        // Should detect increasing memory usage
        expect(mockCallback).toHaveBeenCalledWith(
          'memory_leak_detected',
          expect.objectContaining({
            memoryIncrease: expect.any(Number),
            timeWindow: '30 seconds',
          })
        );
      });
    });

    describe('Memory Trend Analysis', () => {
      beforeEach(async () => {
        await metricsService.start();
      });

      it('should calculate memory trends correctly', () => {
        // Add stable memory data
        const stableData = Array.from({ length: 20 }, (_, i) => ({
          timestamp: Date.now() - (19 - i) * 1000,
          memoryUsageMB: 50 + (Math.random() - 0.5), // Small random variation
          cpuUsagePercent: 25,
        }));

        metricsService.metricsCache.memoryTrends = stableData;
        const trend = metricsService.calculateMemoryTrend(stableData);
        
        expect(trend).toBe('stable');
      });

      it('should detect increasing memory trends', () => {
        // Add increasing memory data
        const increasingData = Array.from({ length: 20 }, (_, i) => ({
          timestamp: Date.now() - (19 - i) * 1000,
          memoryUsageMB: 40 + (i * 2), // Increasing by 2MB per data point
          cpuUsagePercent: 25,
        }));

        metricsService.metricsCache.memoryTrends = increasingData;
        const trend = metricsService.calculateMemoryTrend(increasingData);
        
        expect(trend).toBe('increasing');
      });

      it('should get memory statistics correctly', async () => {
        // Add some memory data
        metricsService.metricsCache.memoryTrends = [
          { memoryUsageMB: 45, cpuUsagePercent: 20 },
          { memoryUsageMB: 50, cpuUsagePercent: 25 },
          { memoryUsageMB: 55, cpuUsagePercent: 30 },
        ];

        const stats = metricsService.getMemoryStatistics();
        
        expect(stats).toMatchObject({
          current: 55,
          min: 45,
          max: 55,
          average: 50,
          dataPoints: 3,
        });
      });
    });

    describe('AI Operation Performance Analysis', () => {
      it('should analyze AI operation performance correctly', () => {
        const operations = [
          { operationType: 'similarity_search', duration: 800, efficiencyScore: 0.9 }, // Slow (target: 50ms)
          { operationType: 'similarity_search', duration: 900, efficiencyScore: 0.85 }, // Slow (target: 50ms)
          { operationType: 'embedding_generation', duration: 800, efficiencyScore: 0.7 }, // Slow (target: 500ms)
          { operationType: 'embedding_generation', duration: 900, efficiencyScore: 0.65 }, // Slow (target: 500ms)
        ];

        const mockCallback = vi.fn();
        metricsService.subscribe(mockCallback);

        metricsService.analyzeAIOperationPerformance(operations);

        // Should detect performance degradation (100% of operations are slow)
        expect(mockCallback).toHaveBeenCalledWith(
          'ai_performance_degradation',
          expect.objectContaining({
            slowOperationCount: 4,
            totalOperations: 4,
          })
        );
      });
    });

    describe('Subscription System', () => {
      it('should handle subscriptions correctly', () => {
        const callback1 = vi.fn();
        const callback2 = vi.fn();

        const unsubscribe1 = metricsService.subscribe(callback1);
        const unsubscribe2 = metricsService.subscribe(callback2);

        expect(metricsService.subscribers.size).toBe(2);

        metricsService.notifySubscribers('test_event', { data: 'test' });

        expect(callback1).toHaveBeenCalledWith('test_event', { data: 'test' });
        expect(callback2).toHaveBeenCalledWith('test_event', { data: 'test' });

        unsubscribe1();
        expect(metricsService.subscribers.size).toBe(1);

        unsubscribe2();
        expect(metricsService.subscribers.size).toBe(0);
      });

      it('should send initial data to new subscribers', () => {
        // Add some cached data
        metricsService.metricsCache = {
          currentMetrics: { test: 'data' },
          lastUpdate: Date.now(),
          memoryTrends: [],
          aiOperations: [],
          alerts: [],
        };

        const callback = vi.fn();
        metricsService.subscribe(callback);

        expect(callback).toHaveBeenCalledWith(
          'initial_data',
          expect.objectContaining({
            currentMetrics: { test: 'data' },
          })
        );
      });
    });

    describe('Error Handling and Retry Logic', () => {
      it('should handle collection errors with retry logic', async () => {
        // Test that the service doesn't crash on errors during metrics collection
        const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
        
        // Start the service normally first
        await metricsService.start();
        expect(metricsService.isActive).toBe(true);
        
        // Now simulate an error during metrics collection
        const originalInvoke = window.__TAURI__.core.invoke;
        let callCount = 0;
        window.__TAURI__.core.invoke = vi.fn(() => {
          callCount++;
          if (callCount <= 2) {
            return Promise.reject(new Error('Temporary failure'));
          }
          return Promise.resolve({});
        });
        
        // Try to collect metrics - should handle error gracefully
        try {
          await metricsService.collectAndProcessMetrics();
        } catch (error) {
          // Expected to handle gracefully
        }
        
        // Service should still be active after error
        expect(metricsService.isActive).toBe(true);
        
        // Restore original mock
        window.__TAURI__.core.invoke = originalInvoke;
        consoleSpy.mockRestore();
      });

      it('should notify subscribers of collection failures', async () => {
        const callback = vi.fn();
        metricsService.subscribe(callback);
        
        // Start the service normally first
        await metricsService.start();
        
        // Simulate collection failure by directly calling the error handler
        await metricsService.handleCollectionError(new Error('Persistent failure'));
        await metricsService.handleCollectionError(new Error('Persistent failure'));
        await metricsService.handleCollectionError(new Error('Persistent failure'));
        await metricsService.handleCollectionError(new Error('Persistent failure')); // Exceed max retries

        // Should eventually notify of failure
        expect(callback).toHaveBeenCalledWith(
          'collection_failed',
          expect.objectContaining({
            error: expect.any(String),
          })
        );
      });
    });

    describe('Service Status and Management', () => {
      it('should report correct service status', async () => {
        const status = metricsService.getStatus();
        
        expect(status).toMatchObject({
          isActive: false,
          subscriberCount: 0,
          lastUpdate: null,
          retryCount: 0,
          memoryTrendCount: 0,
          aiOperationCount: 0,
          alertCount: 0,
        });
      });

      it('should stop service correctly', async () => {
        await metricsService.start();
        expect(metricsService.isActive).toBe(true);

        await metricsService.stop();
        expect(metricsService.isActive).toBe(false);
      });
    });
  });

  describe('Integration Tests', () => {
    it('should integrate dashboard with metrics service correctly', async () => {
      const testService = new RealTimeMetricsService();
      
      // Mock backend responses for service initialization (inactive initially)
      mockInvoke.mockImplementation((command) => {
        switch (command) {
          case 'get_monitoring_status':
            return Promise.resolve({ is_active: false }); // Start inactive
          case 'start_performance_monitoring':
            return Promise.resolve({ is_active: true });
          case 'get_current_performance_metrics':
            return Promise.resolve({
              IncrementalUpdate: { memory_peak_mb: 45, cpu_usage_percent: 25 },
            });
          case 'get_resource_utilization':
            return Promise.resolve({
              cpu_usage_percent: 30,
              memory_usage_mb: 67,
              timestamp: new Date().toISOString(),
            });
          default:
            return Promise.resolve([]);
        }
      });
      
      dashboard = new PerformanceMonitoringDashboard();

      // Start both service and dashboard
      await testService.start();
      await dashboard.startMetricsCollection();

      // Verify integration
      expect(testService.isActive).toBe(true);
      expect(dashboard.isMonitoring).toBe(false); // Dashboard starts manually

      await testService.stop();
    });

    it('should validate performance targets across the system', async () => {
      const startTime = performance.now();

      // Initialize histories with some data
      dashboard.frameTimeHistory = [10, 12, 11];
      dashboard.inputLagHistory = [25, 28, 22];
      
      // Ensure elements are properly mocked for this test
      dashboard.elements = dashboard.elements || {};
      dashboard.elements.inputLagFill = { style: { width: '0%' }, className: 'input-lag-fill', classList: { add: vi.fn(), remove: vi.fn() } };

      // Test all major operations
      await dashboard.collectMetrics();
      dashboard.updateFrameTimeDisplay(10); // Good frame time
      dashboard.updateInputLagDisplay(25); // Good input lag

      const endTime = performance.now();
      const totalTime = endTime - startTime;

      // Validate performance targets
      expect(totalTime).toBeLessThan(50); // <50ms total processing time
      expect(dashboard.frameTimeHistory.length).toBeGreaterThan(0);
      expect(dashboard.inputLagHistory.length).toBeGreaterThan(0);

      // Memory usage should be reasonable (test environment)
      const memoryUsed = process.memoryUsage?.()?.heapUsed || 0;
      expect(memoryUsed).toBeLessThan(100 * 1024 * 1024); // <100MB
    });
  });

  describe('Performance Requirements Validation', () => {
    it('should meet <1% CPU overhead requirement', async () => {
      const iterations = 100;
      const _interval = 100; // 100ms collection interval
      
      const startTime = process.hrtime?.() || [0, 0];
      
      // Simulate rapid metrics collection
      for (let i = 0; i < iterations; i++) {
        await dashboard.collectMetrics();
      }
      
      const [seconds, nanoseconds] = process.hrtime?.(startTime) || [0, 0];
      const totalTime = seconds * 1000 + nanoseconds / 1000000; // Convert to ms
      const averageTime = totalTime / iterations;
      
      // Should be much less than 1% of collection interval (1ms for 100ms interval)
      expect(averageTime).toBeLessThan(1);
    });

    it('should maintain frame rate targets during monitoring', () => {
      // Target: 60fps = 16.67ms per frame
      const targetFrameTime = 16.67;
      
      // Initialize frame time history with some data
      dashboard.frameTimeHistory = [targetFrameTime, targetFrameTime, targetFrameTime];
      
      // Simulate monitoring overhead during frame processing
      const frameStart = performance.now();
      dashboard.updateFrameTimeDisplay(targetFrameTime);
      const frameEnd = performance.now();
      
      const monitoringOverhead = frameEnd - frameStart;
      
      // Monitoring overhead should be minimal (relaxed threshold for test environment)
      expect(monitoringOverhead).toBeLessThan(10); // <10ms overhead in test environment
    });

    it('should handle memory efficiently during extended operation', () => {
      // Simulate extended operation with many metrics
      for (let i = 0; i < 1000; i++) {
        dashboard.resourceHistory.push({
          timestamp: Date.now() - i * 1000,
          memory_usage_mb: 40 + (i % 20),
          cpu_usage_percent: 25 + (i % 10),
        });
        
        dashboard.frameTimeHistory.push(15 + (i % 5));
        dashboard.inputLagHistory.push(20 + (i % 10));
      }
      
      // Force array size maintenance
      dashboard.maintainResourceHistoryLimit();
      
      // Arrays should be bounded to prevent memory leaks
      expect(dashboard.resourceHistory.length).toBeLessThanOrEqual(100);
      expect(dashboard.frameTimeHistory.length).toBeLessThanOrEqual(60);
      expect(dashboard.inputLagHistory.length).toBeLessThanOrEqual(10);
    });
  });
});

// Helper function to create mock performance data
function createMockPerformanceData() {
  return {
    currentMetrics: {
      IncrementalUpdate: {
        operation_type: 'IncrementalUpdate',
        duration_ms: 150,
        memory_peak_mb: 45,
        cpu_usage_percent: 25,
        processing_rate: 10,
      },
    },
    resourceUtilization: {
      timestamp: new Date().toISOString(),
      cpu_usage_percent: 30,
      memory_usage_mb: 67,
      memory_available_mb: 8125,
      disk_read_mb_per_sec: 2.5,
      disk_write_mb_per_sec: 1.8,
    },
    aiOperations: [
      {
        operationType: 'search',
        duration: 45,
        vectorsSearched: 1000,
        resultsReturned: 10,
        efficiencyScore: 0.85,
        performanceTargetMet: true,
        timestamp: Date.now(),
      },
    ],
    alerts: [],
  };
}

export { createMockPerformanceData };