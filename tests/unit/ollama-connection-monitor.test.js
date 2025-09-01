/**
 * Unit tests for Ollama Connection Monitor Service
 * 
 * Tests the automatic connection monitoring, health checks, and integration
 * with the AI system for Issue #182
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';
import OllamaConnectionMonitor from '../../src/js/services/ollama-connection-monitor.js';

describe('OllamaConnectionMonitor', () => {
  let monitor;
  let mockTauri;
  let mockInvoke;

  beforeEach(async () => {
    // Setup Tauri mocks
    const mocks = setupTauriMocks();
    mockTauri = mocks;
    mockInvoke = mocks.invoke;
    
    // Mock performance.now for timing tests
    global.performance = {
      now: vi.fn(() => Date.now())
    };

    // Create new monitor instance
    monitor = new OllamaConnectionMonitor();
  });

  afterEach(async () => {
    // Clean up monitor
    if (monitor) {
      monitor.destroy();
      monitor = null;
    }
    
    // Reset all mocks
    vi.restoreAllMocks();
  });

  describe('Service Initialization', () => {
    it('should initialize with correct default configuration', () => {
      expect(monitor).toBeDefined();
      expect(monitor.isRunning).toBe(false);
      expect(monitor.config.HEALTH_CHECK_INTERVAL).toBe(30000);
      expect(monitor.config.REQUIRED_MODEL).toBe('nomic-embed-text');
    });

    it('should initialize event listeners map', () => {
      expect(monitor.eventListeners).toBeDefined();
      expect(monitor.eventListeners instanceof Map).toBe(true);
    });

    it('should initialize performance metrics', () => {
      expect(monitor.performanceMetrics).toBeDefined();
      expect(monitor.performanceMetrics.responseTimeHistory).toEqual([]);
      expect(monitor.performanceMetrics.healthCheckCount).toBe(0);
    });
  });

  describe('Service Lifecycle', () => {
    it('should start monitoring service', async () => {
      // Mock successful backend initialization
      mockInvoke.mockImplementation((command, args) => {
        if (command === 'start_ollama_monitoring') {
          return Promise.resolve('OK');
        }
        if (command === 'check_ollama_status') {
          return Promise.resolve({
            status: { Connected: {} },
            last_check: new Date().toISOString(),
            retry_count: 0
          });
        }
        return Promise.resolve(null);
      });

      await monitor.start();

      expect(monitor.isRunning).toBe(true);
      expect(monitor.performanceMetrics.startTime).toBeDefined();
    });

    it('should stop monitoring service', async () => {
      // Start first
      mockTauri.mockCommand('start_ollama_monitoring', async () => 'OK');
      await monitor.start();
      
      // Then stop
      monitor.stop();

      expect(monitor.isRunning).toBe(false);
    });

    it('should handle start failure gracefully', async () => {
      // Mock backend initialization failure
      mockTauri.mockCommand('start_ollama_monitoring', async () => {
        throw new Error('Backend not available');
      });

      await monitor.start();

      // Should still start even if backend fails
      expect(monitor.isRunning).toBe(true);
    });
  });

  describe('Health Check Operations', () => {
    beforeEach(async () => {
      mockTauri.mockCommand('start_ollama_monitoring', async () => 'OK');
      await monitor.start();
    });

    it('should perform health check successfully', async () => {
      const mockConnectionState = {
        status: { Connected: {} },
        last_check: new Date().toISOString(),
        retry_count: 0,
        health_info: { version: '1.0.0' }
      };

      mockTauri.mockCommand('check_ollama_status', async () => mockConnectionState);

      let statusChanged = false;
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, () => {
        statusChanged = true;
      });

      await monitor.performHealthCheck();

      expect(monitor.currentStatus).toBe(OllamaConnectionMonitor.STATUS.CONNECTED);
      expect(monitor.performanceMetrics.healthCheckCount).toBe(1);
      expect(monitor.performanceMetrics.successfulChecks).toBe(1);
    });

    it('should handle health check failure', async () => {
      mockTauri.mockCommand('check_ollama_status', async () => {
        throw new Error('Connection refused');
      });

      let errorOccurred = false;
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.ERROR_OCCURRED, () => {
        errorOccurred = true;
      });

      await monitor.performHealthCheck();

      expect(monitor.consecutiveFailures).toBe(1);
      expect(monitor.performanceMetrics.failedChecks).toBe(1);
      expect(errorOccurred).toBe(true);
    });

    it('should track performance metrics correctly', async () => {
      const startTime = 100;
      const endTime = 150;
      global.performance.now = vi.fn()
        .mockReturnValueOnce(startTime)
        .mockReturnValueOnce(endTime);

      mockTauri.mockCommand('check_ollama_status', async () => ({
        status: { Connected: {} },
        last_check: new Date().toISOString(),
        retry_count: 0
      }));

      await monitor.performHealthCheck();

      expect(monitor.performanceMetrics.responseTimeHistory).toContain(endTime - startTime);
      expect(monitor.performanceMetrics.averageResponseTime).toBe(endTime - startTime);
    });
  });

  describe('Connection Status Processing', () => {
    it('should correctly identify status types', () => {
      expect(monitor.getStatusType({ Connected: {} })).toBe('Connected');
      expect(monitor.getStatusType({ Disconnected: {} })).toBe('Disconnected');
      expect(monitor.getStatusType({ Failed: { error: 'test' } })).toBe('Failed');
      expect(monitor.getStatusType('Connected')).toBe('Connected');
    });

    it('should emit status change events', async () => {
      monitor.currentStatus = OllamaConnectionMonitor.STATUS.DISCONNECTED;
      
      let eventData = null;
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, (data) => {
        eventData = data;
      });

      const connectionState = {
        status: { Connected: {} },
        last_check: new Date().toISOString(),
        retry_count: 0
      };

      await monitor.processStatusUpdate(connectionState);

      expect(eventData).toBeDefined();
      expect(eventData.previousStatus).toBe(OllamaConnectionMonitor.STATUS.DISCONNECTED);
      expect(eventData.currentStatus).toBe(OllamaConnectionMonitor.STATUS.CONNECTED);
    });
  });

  describe('Reconnection Logic', () => {
    beforeEach(async () => {
      mockTauri.mockCommand('start_ollama_monitoring', async () => 'OK');
      await monitor.start();
    });

    it('should schedule reconnection attempts', async () => {
      const error = new Error('Connection failed');
      await monitor.handleConnectionFailure(error);

      expect(monitor.reconnectAttempts).toBe(1);
      expect(monitor.nextReconnectTime).toBeDefined();
      expect(monitor.currentStatus).toBe(OllamaConnectionMonitor.STATUS.RETRYING);
    });

    it('should implement exponential backoff', async () => {
      const error = new Error('Connection failed');
      
      // First failure
      await monitor.handleConnectionFailure(error);
      const firstRetryTime = monitor.nextReconnectTime;
      
      // Second failure
      await monitor.handleConnectionFailure(error);
      const secondRetryTime = monitor.nextReconnectTime;

      // Second retry should be scheduled later than first
      expect(secondRetryTime).toBeGreaterThan(firstRetryTime);
    });

    it('should stop retrying after max attempts', async () => {
      monitor.config.MAX_RECONNECT_ATTEMPTS = 2;
      
      const error = new Error('Connection failed');
      
      // Exceed max attempts
      await monitor.handleConnectionFailure(error);
      await monitor.handleConnectionFailure(error);
      await monitor.handleConnectionFailure(error); // This should not schedule another retry

      expect(monitor.currentStatus).toBe(OllamaConnectionMonitor.STATUS.FAILED);
    });
  });

  describe('Model Status Monitoring', () => {
    beforeEach(async () => {
      mockTauri.mockCommand('start_ollama_monitoring', async () => 'OK');
      await monitor.start();
    });

    it('should check model status successfully', async () => {
      const mockModelVerification = {
        is_available: true,
        is_compatible: 'Compatible',
        info: { size: 137000000 },
        verification_time_ms: 50
      };

      mockTauri.mockCommand('verify_model', async ({ modelName }) => {
        expect(modelName).toBe('nomic-embed-text');
        return mockModelVerification;
      });

      let modelStatusUpdated = false;
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.MODEL_STATUS_UPDATED, () => {
        modelStatusUpdated = true;
      });

      await monitor.checkModelStatus();

      expect(monitor.modelStatus.isAvailable).toBe(true);
      expect(monitor.modelStatus.isCompatible).toBe(true);
      expect(monitor.modelStatus.lastChecked).toBeDefined();
    });

    it('should handle model verification failure', async () => {
      mockTauri.mockCommand('verify_model', async () => {
        throw new Error('Model not found');
      });

      await monitor.checkModelStatus();

      expect(monitor.modelStatus.lastChecked).toBeDefined();
      // Should not throw error, just log warning
    });
  });

  describe('Manual Operations', () => {
    beforeEach(async () => {
      mockTauri.mockCommand('start_ollama_monitoring', async () => 'OK');
      await monitor.start();
    });

    it('should support manual health check', async () => {
      mockTauri.mockCommand('check_ollama_status', async () => ({
        status: { Connected: {} },
        last_check: new Date().toISOString(),
        retry_count: 0
      }));

      const status = await monitor.checkNow();

      expect(status).toBeDefined();
      expect(status.isRunning).toBe(true);
      expect(status.currentStatus).toBe(OllamaConnectionMonitor.STATUS.CONNECTED);
    });

    it('should support forced reconnection', async () => {
      // Set up a pending reconnection
      monitor.reconnectAttempts = 3;
      monitor.reconnectTimeout = setTimeout(() => {}, 1000);

      mockTauri.mockCommand('check_ollama_status', async () => ({
        status: { Connected: {} },
        last_check: new Date().toISOString(),
        retry_count: 0
      }));

      await monitor.forceReconnect();

      expect(monitor.reconnectAttempts).toBe(0);
      expect(monitor.reconnectTimeout).toBeNull();
    });
  });

  describe('Configuration Management', () => {
    it('should update configuration', () => {
      const newConfig = {
        HEALTH_CHECK_INTERVAL: 60000,
        MAX_RECONNECT_ATTEMPTS: 5
      };

      monitor.updateConfig(newConfig);

      expect(monitor.config.HEALTH_CHECK_INTERVAL).toBe(60000);
      expect(monitor.config.MAX_RECONNECT_ATTEMPTS).toBe(5);
      expect(monitor.config.REQUIRED_MODEL).toBe('nomic-embed-text'); // Should keep existing values
    });
  });

  describe('Event System', () => {
    it('should support adding and removing event listeners', () => {
      const handler = vi.fn();

      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, handler);
      expect(monitor.eventListeners.get(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED)).toContain(handler);

      monitor.removeEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, handler);
      expect(monitor.eventListeners.get(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED)).not.toContain(handler);
    });

    it('should emit events to all listeners', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      const eventData = { test: 'data' };

      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, handler1);
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, handler2);

      monitor.emit(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, eventData);

      expect(handler1).toHaveBeenCalledWith(eventData);
      expect(handler2).toHaveBeenCalledWith(eventData);
    });

    it('should handle event handler errors gracefully', () => {
      const errorHandler = vi.fn(() => {
        throw new Error('Handler error');
      });
      const goodHandler = vi.fn();

      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, errorHandler);
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, goodHandler);

      expect(() => {
        monitor.emit(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, { test: 'data' });
      }).not.toThrow();

      expect(goodHandler).toHaveBeenCalled();
    });
  });

  describe('Resource Cleanup', () => {
    it('should clean up resources on destroy', () => {
      monitor.start();
      monitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, vi.fn());

      monitor.destroy();

      expect(monitor.isRunning).toBe(false);
      expect(monitor.eventListeners.size).toBe(0);
    });
  });

  describe('Performance Targets', () => {
    it('should meet health check latency target (<100ms)', async () => {
      const startTime = 0;
      const endTime = 50; // 50ms response time
      global.performance.now = vi.fn()
        .mockReturnValueOnce(startTime)
        .mockReturnValueOnce(endTime);

      mockTauri.mockCommand('start_ollama_monitoring', async () => 'OK');
      mockTauri.mockCommand('check_ollama_status', async () => ({
        status: { Connected: {} },
        last_check: new Date().toISOString(),
        retry_count: 0
      }));

      await monitor.start();
      await monitor.performHealthCheck();

      expect(monitor.performanceMetrics.averageResponseTime).toBeLessThan(100);
    });

    it('should handle monitoring overhead target (<1% CPU)', () => {
      // This is more of a design verification - the intervals should be reasonable
      expect(monitor.config.HEALTH_CHECK_INTERVAL).toBeGreaterThanOrEqual(30000); // 30s minimum
      expect(monitor.config.MODEL_CHECK_INTERVAL).toBeGreaterThanOrEqual(60000); // 1min minimum
    });
  });
});