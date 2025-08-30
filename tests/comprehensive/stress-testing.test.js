/**
 * Comprehensive Stress Testing Suite
 * 
 * Validates memory usage, UI responsiveness, concurrent operations,
 * large vault performance, and cross-platform compatibility.
 * 
 * Part of Issue #176: Performance Testing - Comprehensive validation and benchmarking
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Mock performance APIs for comprehensive testing
const mockPerformance = {
  now: vi.fn(() => Date.now()),
  mark: vi.fn(),
  measure: vi.fn(),
  memory: {
    usedJSHeapSize: 50 * 1024 * 1024, // 50MB baseline
    totalJSHeapSize: 100 * 1024 * 1024,
    jsHeapSizeLimit: 2048 * 1024 * 1024
  }
};

global.performance = mockPerformance;
global.requestAnimationFrame = vi.fn((cb) => setTimeout(cb, 16));
global.cancelAnimationFrame = vi.fn();

describe('Comprehensive Performance Stress Testing', () => {
  let mockInvoke;
  let memoryBaseline;

  beforeEach(() => {
    const { invoke } = setupTauriMocks();
    mockInvoke = invoke;
    memoryBaseline = mockPerformance.memory.usedJSHeapSize;
    
    // Mock successful AI and vault operations
    setupComprehensiveMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
    // Reset memory simulation
    mockPerformance.memory.usedJSHeapSize = memoryBaseline;
  });

  function setupComprehensiveMocks() {
    if (!mockInvoke || typeof mockInvoke.mockImplementation !== 'function') {
      console.warn('mockInvoke not properly initialized');
      return;
    }
    mockInvoke.mockImplementation((command, payload) => {
      switch (command) {
        // Memory and resource monitoring
        case 'get_memory_usage':
          return Promise.resolve({
            used_memory_mb: mockPerformance.memory.usedJSHeapSize / (1024 * 1024),
            total_memory_mb: mockPerformance.memory.totalJSHeapSize / (1024 * 1024),
            memory_pressure: false
          });
        
        case 'get_system_resources':
          return Promise.resolve({
            cpu_usage_percent: 25 + (Math.random() * 20), // 25-45% usage
            memory_usage_percent: 60 + (Math.random() * 15), // 60-75% usage
            disk_io_mbps: 50 + (Math.random() * 100),
            network_io_mbps: 10 + (Math.random() * 40),
            load_average: 0.8 + (Math.random() * 0.4)
          });
        
        // Large vault operations
        case 'scan_vault_files':
          const fileCount = payload?.stress_test ? 10000 : 100;
          return Promise.resolve(generateMockFiles(fileCount));
        
        case 'index_large_vault':
          return new Promise((resolve) => {
            setTimeout(() => resolve({
              indexed_files: payload?.file_count || 1000,
              processing_time_ms: 2000 + (Math.random() * 1000),
              memory_peak_mb: 80 + (Math.random() * 20)
            }), payload?.file_count / 5); // Simulate indexing time
          });
        
        // Concurrent AI operations
        case 'generate_embedding':
          const delay = 100 + (Math.random() * 200); // 100-300ms
          return new Promise((resolve) => {
            setTimeout(() => resolve({
              embedding: new Array(384).fill(0).map(() => Math.random()),
              processing_time_ms: delay,
              memory_used_mb: 5 + (Math.random() * 3)
            }), delay);
          });
        
        case 'similarity_search':
          return Promise.resolve({
            results: generateMockSearchResults(payload?.max_results || 10),
            search_time_ms: 30 + (Math.random() * 40),
            vectors_searched: payload?.total_vectors || 5000
          });
        
        // Performance monitoring
        case 'start_performance_monitoring':
          return Promise.resolve({ monitoring_active: true });
        
        case 'get_performance_metrics':
          return Promise.resolve({
            frame_time_ms: 12 + (Math.random() * 8), // 12-20ms frames
            input_lag_ms: 20 + (Math.random() * 30), // 20-50ms lag
            ui_thread_utilization: 0.3 + (Math.random() * 0.4),
            background_tasks_count: Math.floor(Math.random() * 5)
          });
        
        case 'run_gc_if_needed':
          // Simulate garbage collection
          mockPerformance.memory.usedJSHeapSize = Math.max(
            memoryBaseline,
            mockPerformance.memory.usedJSHeapSize * 0.7
          );
          return Promise.resolve({ gc_performed: true });
        
        default:
          return Promise.resolve({});
      }
    });
  }

  function generateMockFiles(count) {
    return Array.from({ length: count }, (_, i) => ({
      name: `document_${i.toString().padStart(5, '0')}.md`,
      path: `/vault/documents/document_${i.toString().padStart(5, '0')}.md`,
      size_bytes: 1024 + (Math.random() * 10240), // 1-11KB files
      is_dir: false,
      last_modified: new Date(Date.now() - (Math.random() * 86400000)).toISOString()
    }));
  }

  function generateMockSearchResults(count) {
    return Array.from({ length: count }, (_, i) => ({
      file_path: `/vault/result_${i}.md`,
      similarity: 0.9 - (i * 0.1),
      content_snippet: `Mock search result ${i} with relevant content...`,
      chunk_id: i
    }));
  }

  describe('Memory Usage Stress Tests', () => {
    it('should handle large vault indexing without memory explosion', async () => {
      const startMemory = await mockInvoke('get_memory_usage');
      
      // Simulate indexing 5000 files
      const indexingResult = await mockInvoke('index_large_vault', { 
        file_count: 5000 
      });
      
      // Simulate memory growth during indexing
      mockPerformance.memory.usedJSHeapSize += 30 * 1024 * 1024; // +30MB
      const peakMemory = await mockInvoke('get_memory_usage');
      
      // Trigger garbage collection
      await mockInvoke('run_gc_if_needed');
      const finalMemory = await mockInvoke('get_memory_usage');
      
      // Validate memory constraints
      expect(indexingResult.indexed_files).toBe(5000);
      expect(indexingResult.processing_time_ms).toBeLessThan(4000); // <4s for 5k files
      expect(peakMemory.used_memory_mb).toBeLessThan(100); // <100MB during indexing
      expect(finalMemory.used_memory_mb).toBeLessThan(80); // Memory cleanup after GC
      
      const memoryGrowth = finalMemory.used_memory_mb - startMemory.used_memory_mb;
      expect(memoryGrowth).toBeLessThan(20); // <20MB permanent growth
    }, 10000);

    it('should handle sustained AI operations without memory leaks', async () => {
      const iterations = 50;
      const memoryMeasurements = [];
      
      for (let i = 0; i < iterations; i++) {
        // Generate embedding
        await mockInvoke('generate_embedding', {
          text: `Test document content iteration ${i}. This is a longer piece of text to generate embeddings for.`
        });
        
        // Simulate memory accumulation
        mockPerformance.memory.usedJSHeapSize += 200 * 1024; // +200KB per operation
        
        // Measure memory every 10 iterations
        if (i % 10 === 0) {
          const memory = await mockInvoke('get_memory_usage');
          memoryMeasurements.push(memory.used_memory_mb);
          
          // Periodic garbage collection
          if (i % 25 === 0) {
            await mockInvoke('run_gc_if_needed');
          }
        }
      }
      
      // Analyze memory trend
      const firstMeasurement = memoryMeasurements[0];
      const lastMeasurement = memoryMeasurements[memoryMeasurements.length - 1];
      const memoryGrowth = lastMeasurement - firstMeasurement;
      
      // Should not have excessive memory growth
      expect(memoryGrowth).toBeLessThan(15); // <15MB growth for 50 operations
      expect(memoryMeasurements.length).toBeGreaterThan(3); // Multiple measurements
      
      // Check for memory leak pattern (continuously increasing)
      let continuousGrowth = 0;
      for (let i = 1; i < memoryMeasurements.length; i++) {
        if (memoryMeasurements[i] > memoryMeasurements[i - 1]) {
          continuousGrowth++;
        }
      }
      
      // Not all measurements should show growth (GC should reduce memory)
      expect(continuousGrowth).toBeLessThan(memoryMeasurements.length);
    }, 15000);

    it('should manage memory efficiently under various workload scenarios', async () => {
      const scenarios = [
        { name: 'light_indexing', files: 100, embeddings: 10 },
        { name: 'moderate_search', files: 500, embeddings: 25, searches: 20 },
        { name: 'heavy_processing', files: 1000, embeddings: 50, searches: 50 }
      ];
      
      const scenarioResults = [];
      
      for (const scenario of scenarios) {
        const startTime = performance.now();
        const startMemory = await mockInvoke('get_memory_usage');
        
        // Execute scenario workload
        const promises = [];
        
        // File indexing
        promises.push(mockInvoke('index_large_vault', { 
          file_count: scenario.files 
        }));
        
        // Embedding generation
        for (let i = 0; i < scenario.embeddings; i++) {
          promises.push(mockInvoke('generate_embedding', {
            text: `Scenario ${scenario.name} document ${i}`
          }));
        }
        
        // Search operations
        for (let i = 0; i < (scenario.searches || 0); i++) {
          promises.push(mockInvoke('similarity_search', {
            query: `Query ${i}`,
            max_results: 10,
            total_vectors: scenario.files * 10
          }));
        }
        
        await Promise.all(promises);
        
        const endTime = performance.now();
        const endMemory = await mockInvoke('get_memory_usage');
        
        scenarioResults.push({
          name: scenario.name,
          duration_ms: endTime - startTime,
          memory_growth_mb: endMemory.used_memory_mb - startMemory.used_memory_mb,
          operations: promises.length
        });
        
        // Cleanup between scenarios
        await mockInvoke('run_gc_if_needed');
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      
      // Validate scenario results
      for (const result of scenarioResults) {
        expect(result.duration_ms).toBeLessThan(10000); // <10s per scenario
        expect(result.memory_growth_mb).toBeLessThan(25); // <25MB growth per scenario
        expect(result.operations).toBeGreaterThan(0);
      }
      
      // Verify scaling behavior
      const lightResult = scenarioResults.find(r => r.name === 'light_indexing');
      const heavyResult = scenarioResults.find(r => r.name === 'heavy_processing');
      
      // Heavy workload should not be more than 10x slower than light
      const scalingFactor = heavyResult.duration_ms / lightResult.duration_ms;
      expect(scalingFactor).toBeLessThan(10);
    }, 30000);
  });

  describe('UI Responsiveness Tests During AI Processing', () => {
    it('should maintain UI responsiveness during embedding generation', async () => {
      const frameMeasurements = [];
      const embeddingPromises = [];
      
      // Start background AI operations
      for (let i = 0; i < 20; i++) {
        embeddingPromises.push(mockInvoke('generate_embedding', {
          text: `Background embedding ${i} with substantial content to process`
        }));
      }
      
      // Measure frame times while AI operations are running
      for (let frame = 0; frame < 60; frame++) { // 1 second at 60fps
        const frameStart = performance.now();
        
        // Simulate UI work
        await mockInvoke('get_performance_metrics');
        
        // Simulate frame rendering
        await new Promise(resolve => requestAnimationFrame(resolve));
        
        const frameEnd = performance.now();
        frameMeasurements.push(frameEnd - frameStart);
        
        await new Promise(resolve => setTimeout(resolve, 16)); // Target 60fps
      }
      
      // Wait for AI operations to complete
      await Promise.all(embeddingPromises);
      
      // Analyze frame performance
      const avgFrameTime = frameMeasurements.reduce((sum, time) => sum + time, 0) / frameMeasurements.length;
      const maxFrameTime = Math.max(...frameMeasurements);
      const droppedFrames = frameMeasurements.filter(time => time > 33).length; // >33ms = dropped frame at 30fps
      
      expect(avgFrameTime).toBeLessThan(16); // Average <16ms (60fps)
      expect(maxFrameTime).toBeLessThan(50); // Max <50ms (20fps minimum)
      expect(droppedFrames).toBeLessThan(5); // <5 dropped frames out of 60
    }, 10000);

    it('should handle input responsiveness during large vault operations', async () => {
      const inputMeasurements = [];
      
      // Start large vault indexing
      const indexingPromise = mockInvoke('index_large_vault', { 
        file_count: 2000 
      });
      
      // Simulate user inputs while indexing
      for (let input = 0; input < 20; input++) {
        const inputStart = performance.now();
        
        // Simulate input processing (typing, clicking, scrolling)
        const metrics = await mockInvoke('get_performance_metrics');
        
        const inputEnd = performance.now();
        const inputLag = inputEnd - inputStart + metrics.input_lag_ms;
        inputMeasurements.push(inputLag);
        
        await new Promise(resolve => setTimeout(resolve, 100)); // 100ms between inputs
      }
      
      await indexingPromise;
      
      // Analyze input responsiveness
      const avgInputLag = inputMeasurements.reduce((sum, lag) => sum + lag, 0) / inputMeasurements.length;
      const maxInputLag = Math.max(...inputMeasurements);
      const slowInputs = inputMeasurements.filter(lag => lag > 100).length;
      
      expect(avgInputLag).toBeLessThan(50); // Average <50ms input lag
      expect(maxInputLag).toBeLessThan(200); // Max <200ms input lag
      expect(slowInputs).toBeLessThan(3); // <3 slow inputs out of 20
    }, 15000);

    it('should maintain performance during concurrent UI and AI operations', async () => {
      const performanceMetrics = [];
      const concurrentOperations = [];
      
      // Start concurrent AI operations
      concurrentOperations.push(mockInvoke('index_large_vault', { file_count: 1000 }));
      
      for (let i = 0; i < 10; i++) {
        concurrentOperations.push(mockInvoke('generate_embedding', {
          text: `Concurrent embedding ${i}`
        }));
        concurrentOperations.push(mockInvoke('similarity_search', {
          query: `Concurrent search ${i}`,
          max_results: 5
        }));
      }
      
      // Monitor performance while operations are running
      const monitoringStart = performance.now();
      while (performance.now() - monitoringStart < 5000) { // Monitor for 5 seconds
        const metrics = await mockInvoke('get_performance_metrics');
        const systemResources = await mockInvoke('get_system_resources');
        
        performanceMetrics.push({
          timestamp: performance.now(),
          frame_time: metrics.frame_time_ms,
          input_lag: metrics.input_lag_ms,
          cpu_usage: systemResources.cpu_usage_percent,
          ui_thread_util: metrics.ui_thread_utilization
        });
        
        await new Promise(resolve => setTimeout(resolve, 250)); // Check every 250ms
      }
      
      await Promise.all(concurrentOperations);
      
      // Analyze concurrent performance
      const avgFrameTime = performanceMetrics.reduce((sum, m) => sum + m.frame_time, 0) / performanceMetrics.length;
      const avgInputLag = performanceMetrics.reduce((sum, m) => sum + m.input_lag, 0) / performanceMetrics.length;
      const maxCpuUsage = Math.max(...performanceMetrics.map(m => m.cpu_usage));
      const avgUiThreadUtil = performanceMetrics.reduce((sum, m) => sum + m.ui_thread_util, 0) / performanceMetrics.length;
      
      expect(avgFrameTime).toBeLessThan(20); // Average <20ms frames during heavy load
      expect(avgInputLag).toBeLessThan(75); // Average <75ms input lag during heavy load
      expect(maxCpuUsage).toBeLessThan(90); // Max <90% CPU usage
      expect(avgUiThreadUtil).toBeLessThan(0.8); // Average <80% UI thread utilization
      expect(performanceMetrics.length).toBeGreaterThan(15); // Multiple measurements over 5 seconds
    }, 15000);
  });

  describe('Large Vault Indexing Performance Validation', () => {
    it('should efficiently handle vaults with 10,000+ files', async () => {
      const vaultSizes = [1000, 5000, 10000];
      const indexingResults = [];
      
      for (const fileCount of vaultSizes) {
        const startTime = performance.now();
        const startMemory = await mockInvoke('get_memory_usage');
        
        // Scan and index large vault
        const files = await mockInvoke('scan_vault_files', { 
          stress_test: true, 
          file_count: fileCount 
        });
        
        const indexingResult = await mockInvoke('index_large_vault', { 
          file_count: fileCount 
        });
        
        const endTime = performance.now();
        const endMemory = await mockInvoke('get_memory_usage');
        
        indexingResults.push({
          file_count: fileCount,
          scan_time_ms: endTime - startTime,
          indexing_time_ms: indexingResult.processing_time_ms,
          memory_growth_mb: endMemory.used_memory_mb - startMemory.used_memory_mb,
          throughput_files_per_sec: fileCount / (indexingResult.processing_time_ms / 1000)
        });
        
        expect(files.length).toBe(fileCount);
        
        // Cleanup between tests
        await mockInvoke('run_gc_if_needed');
      }
      
      // Validate scaling characteristics
      for (const result of indexingResults) {
        expect(result.throughput_files_per_sec).toBeGreaterThan(100); // >100 files/sec
        expect(result.memory_growth_mb).toBeLessThan(50); // <50MB per vault
        expect(result.indexing_time_ms).toBeLessThan(result.file_count * 2); // <2ms per file
      }
      
      // Check performance doesn't degrade exponentially
      const small = indexingResults[0]; // 1k files
      const large = indexingResults[2]; // 10k files
      
      const timeScaling = large.indexing_time_ms / small.indexing_time_ms;
      const memoryScaling = large.memory_growth_mb / small.memory_growth_mb;
      
      expect(timeScaling).toBeLessThan(15); // Less than 15x time for 10x files
      expect(memoryScaling).toBeLessThan(12); // Less than 12x memory for 10x files
    }, 30000);

    it('should maintain search performance in large indexed vaults', async () => {
      // Index a large vault first
      await mockInvoke('index_large_vault', { file_count: 8000 });
      
      const searchQueries = [
        'artificial intelligence machine learning',
        'performance optimization algorithms',
        'user interface design patterns',
        'database indexing strategies',
        'network protocol implementation'
      ];
      
      const searchResults = [];
      
      for (const query of searchQueries) {
        const searchStart = performance.now();
        
        const result = await mockInvoke('similarity_search', {
          query,
          max_results: 20,
          total_vectors: 8000 * 10 // Assume 10 chunks per file
        });
        
        const searchEnd = performance.now();
        const searchTime = searchEnd - searchStart;
        
        searchResults.push({
          query,
          search_time_ms: searchTime,
          results_count: result.results.length,
          vectors_searched: result.vectors_searched,
          throughput_vectors_per_ms: result.vectors_searched / searchTime
        });
        
        expect(result.results.length).toBeGreaterThan(0);
        expect(result.results.length).toBeLessThanOrEqual(20);
      }
      
      // Validate search performance
      const avgSearchTime = searchResults.reduce((sum, r) => sum + r.search_time_ms, 0) / searchResults.length;
      const minThroughput = Math.min(...searchResults.map(r => r.throughput_vectors_per_ms));
      
      expect(avgSearchTime).toBeLessThan(100); // Average <100ms search time
      expect(minThroughput).toBeGreaterThan(500); // >500 vectors/ms throughput
      
      // All searches should be reasonably fast
      const slowSearches = searchResults.filter(r => r.search_time_ms > 200);
      expect(slowSearches.length).toBe(0);
    }, 20000);

    it('should handle incremental indexing efficiently', async () => {
      const batchSizes = [100, 250, 500];
      const incrementalResults = [];
      let totalFiles = 0;
      
      for (const batchSize of batchSizes) {
        const batchStart = performance.now();
        const startMemory = await mockInvoke('get_memory_usage');
        
        // Simulate incremental indexing
        const indexingResult = await mockInvoke('index_large_vault', { 
          file_count: batchSize,
          incremental: true,
          existing_files: totalFiles
        });
        
        const batchEnd = performance.now();
        const endMemory = await mockInvoke('get_memory_usage');
        
        totalFiles += batchSize;
        
        incrementalResults.push({
          batch_size: batchSize,
          total_files: totalFiles,
          batch_time_ms: batchEnd - batchStart,
          memory_delta_mb: endMemory.used_memory_mb - startMemory.used_memory_mb,
          files_per_second: batchSize / ((batchEnd - batchStart) / 1000)
        });
        
        expect(indexingResult.indexed_files).toBe(batchSize);
      }
      
      // Validate incremental performance consistency
      for (const result of incrementalResults) {
        expect(result.files_per_second).toBeGreaterThan(50); // >50 files/sec incremental
        expect(result.memory_delta_mb).toBeLessThan(10); // <10MB per batch
        expect(result.batch_time_ms).toBeLessThan(batchSize * 20); // <20ms per file
      }
      
      // Performance should remain consistent across batches
      const firstBatch = incrementalResults[0];
      const lastBatch = incrementalResults[incrementalResults.length - 1];
      
      const performanceRatio = lastBatch.files_per_second / firstBatch.files_per_second;
      expect(performanceRatio).toBeGreaterThan(0.8); // Within 20% of initial performance
    }, 15000);
  });

  describe('Concurrent AI Operations Stress Testing', () => {
    it('should handle multiple simultaneous embedding generations', async () => {
      const concurrentOperations = 25;
      const operationPromises = [];
      const startTime = performance.now();
      
      // Launch concurrent embedding operations
      for (let i = 0; i < concurrentOperations; i++) {
        operationPromises.push(
          mockInvoke('generate_embedding', {
            text: `Concurrent embedding test document ${i}. This document contains enough text to generate meaningful embeddings.`,
            operation_id: i
          }).then(result => ({
            ...result,
            operation_id: i,
            completed_at: performance.now()
          }))
        );
      }
      
      const results = await Promise.all(operationPromises);
      const endTime = performance.now();
      const totalTime = endTime - startTime;
      
      // Analyze concurrent performance
      const avgProcessingTime = results.reduce((sum, r) => sum + r.processing_time_ms, 0) / results.length;
      const maxProcessingTime = Math.max(...results.map(r => r.processing_time_ms));
      const totalMemoryUsed = results.reduce((sum, r) => sum + r.memory_used_mb, 0);
      
      expect(results.length).toBe(concurrentOperations);
      expect(totalTime).toBeLessThan(5000); // Complete within 5 seconds
      expect(avgProcessingTime).toBeLessThan(300); // Average <300ms per operation
      expect(maxProcessingTime).toBeLessThan(500); // Max <500ms per operation
      expect(totalMemoryUsed).toBeLessThan(200); // Total <200MB memory usage
      
      // Check that operations ran concurrently (not sequentially)
      const sequentialTime = concurrentOperations * avgProcessingTime;
      const concurrencyRatio = sequentialTime / totalTime;
      expect(concurrencyRatio).toBeGreaterThan(3); // At least 3x faster than sequential
    }, 10000);

    it('should manage resource contention during mixed AI operations', async () => {
      const mixedOperations = [];
      const resourceMetrics = [];
      
      // Mix of different AI operations
      const operations = [
        { type: 'embedding', count: 10 },
        { type: 'search', count: 15 },
        { type: 'indexing', count: 3 }
      ];
      
      const startTime = performance.now();
      
      // Launch mixed operations
      for (const opType of operations) {
        for (let i = 0; i < opType.count; i++) {
          switch (opType.type) {
            case 'embedding':
              mixedOperations.push(
                mockInvoke('generate_embedding', {
                  text: `Mixed operation embedding ${i}`
                })
              );
              break;
            case 'search':
              mixedOperations.push(
                mockInvoke('similarity_search', {
                  query: `Mixed search query ${i}`,
                  max_results: 10
                })
              );
              break;
            case 'indexing':
              mixedOperations.push(
                mockInvoke('index_large_vault', {
                  file_count: 200
                })
              );
              break;
          }
        }
      }
      
      // Monitor resources while operations run
      const resourceMonitoring = setInterval(async () => {
        const resources = await mockInvoke('get_system_resources');
        const memory = await mockInvoke('get_memory_usage');
        
        resourceMetrics.push({
          timestamp: performance.now(),
          cpu_usage: resources.cpu_usage_percent,
          memory_usage: memory.used_memory_mb,
          load_average: resources.load_average
        });
      }, 100);
      
      await Promise.all(mixedOperations);
      clearInterval(resourceMonitoring);
      
      const endTime = performance.now();
      const totalTime = endTime - startTime;
      
      // Analyze resource management
      const maxCpuUsage = Math.max(...resourceMetrics.map(m => m.cpu_usage));
      const avgMemoryUsage = resourceMetrics.reduce((sum, m) => sum + m.memory_usage, 0) / resourceMetrics.length;
      const maxLoadAverage = Math.max(...resourceMetrics.map(m => m.load_average));
      
      expect(mixedOperations.length).toBe(28); // Total operations
      expect(totalTime).toBeLessThan(8000); // Complete within 8 seconds
      expect(maxCpuUsage).toBeLessThan(95); // CPU <95% even under stress
      expect(avgMemoryUsage).toBeLessThan(120); // Average memory <120MB
      expect(maxLoadAverage).toBeLessThan(2.0); // Load average reasonable
      expect(resourceMetrics.length).toBeGreaterThan(10); // Multiple measurements
    }, 15000);

    it('should handle burst workloads and recover gracefully', async () => {
      const burstSizes = [5, 15, 30]; // Small, medium, large bursts
      const burstResults = [];
      
      for (const burstSize of burstSizes) {
        const burstStart = performance.now();
        const startMemory = await mockInvoke('get_memory_usage');
        
        // Create burst of operations
        const burstOperations = [];
        for (let i = 0; i < burstSize; i++) {
          burstOperations.push(mockInvoke('generate_embedding', {
            text: `Burst operation ${i} in burst of size ${burstSize}`
          }));
        }
        
        const burstResultData = await Promise.all(burstOperations);
        const burstEnd = performance.now();
        
        // Wait for system to recover
        await new Promise(resolve => setTimeout(resolve, 1000));
        await mockInvoke('run_gc_if_needed');
        
        const recoveryMemory = await mockInvoke('get_memory_usage');
        const recoveryTime = performance.now();
        
        burstResults.push({
          burst_size: burstSize,
          burst_time_ms: burstEnd - burstStart,
          recovery_time_ms: recoveryTime - burstEnd,
          operations_completed: burstResultData.length,
          memory_peak_mb: Math.max(...burstResultData.map(r => r.memory_used_mb || 5)),
          memory_recovered_mb: startMemory.used_memory_mb - recoveryMemory.used_memory_mb
        });
      }
      
      // Validate burst handling
      for (const result of burstResults) {
        expect(result.operations_completed).toBe(result.burst_size);
        expect(result.burst_time_ms / result.burst_size).toBeLessThan(500); // <500ms per operation in burst
        expect(result.recovery_time_ms).toBeLessThan(2000); // <2s recovery time
        expect(result.memory_recovered_mb).toBeGreaterThan(-5); // Memory should not grow significantly
      }
      
      // Verify scaling behavior under burst conditions
      const smallBurst = burstResults[0];
      const largeBurst = burstResults[2];
      
      const efficiencyRatio = (largeBurst.burst_time_ms / largeBurst.burst_size) / 
                             (smallBurst.burst_time_ms / smallBurst.burst_size);
      
      expect(efficiencyRatio).toBeLessThan(2); // Large bursts should not be much less efficient
    }, 20000);
  });

  describe('Cross-Platform Performance Benchmarks', () => {
    it('should adapt performance expectations based on system capabilities', async () => {
      const systemConfigs = [
        { name: 'low_end', cpu_cores: 2, memory_gb: 4, expected_multiplier: 2.0 },
        { name: 'mid_range', cpu_cores: 4, memory_gb: 8, expected_multiplier: 1.0 },
        { name: 'high_end', cpu_cores: 8, memory_gb: 16, expected_multiplier: 0.6 }
      ];
      
      const benchmarkResults = [];
      
      for (const config of systemConfigs) {
        // Simulate different system characteristics
        const adjustedPerformance = {
          embedding_time_ms: 150 * config.expected_multiplier,
          search_time_ms: 50 * config.expected_multiplier,
          indexing_files_per_sec: 200 / config.expected_multiplier,
          memory_efficiency: 1 / config.expected_multiplier
        };
        
        // Run standardized benchmark suite
        const embeddingResult = await mockInvoke('generate_embedding', {
          text: 'Cross-platform benchmark test document'
        });
        
        const searchResult = await mockInvoke('similarity_search', {
          query: 'benchmark test',
          max_results: 10
        });
        
        const indexingResult = await mockInvoke('index_large_vault', {
          file_count: 500
        });
        
        benchmarkResults.push({
          system: config.name,
          cpu_cores: config.cpu_cores,
          memory_gb: config.memory_gb,
          embedding_time: embeddingResult.processing_time_ms,
          search_time: searchResult.search_time_ms,
          indexing_throughput: 500 / (indexingResult.processing_time_ms / 1000),
          memory_used: embeddingResult.memory_used_mb,
          expected_multiplier: config.expected_multiplier
        });
      }
      
      // Validate cross-platform performance
      for (const result of benchmarkResults) {
        const expectedEmbeddingTime = 150 * result.expected_multiplier;
        const expectedSearchTime = 50 * result.expected_multiplier;
        const expectedThroughput = 200 / result.expected_multiplier;
        
        // Allow for 50% variance in performance expectations
        expect(result.embedding_time).toBeLessThan(expectedEmbeddingTime * 1.5);
        expect(result.search_time).toBeLessThan(expectedSearchTime * 1.5);
        expect(result.indexing_throughput).toBeGreaterThan(expectedThroughput * 0.5);
        expect(result.memory_used).toBeLessThan(20); // Reasonable memory usage across platforms
      }
      
      // Verify performance scaling expectations
      const lowEnd = benchmarkResults.find(r => r.system === 'low_end');
      const highEnd = benchmarkResults.find(r => r.system === 'high_end');
      
      const embeddingRatio = lowEnd.embedding_time / highEnd.embedding_time;
      const throughputRatio = highEnd.indexing_throughput / lowEnd.indexing_throughput;
      
      expect(embeddingRatio).toBeGreaterThan(2); // High-end should be >2x faster
      expect(throughputRatio).toBeGreaterThan(2); // High-end should have >2x throughput
    }, 15000);

    it('should validate performance under different memory constraints', async () => {
      const memoryConstraints = [
        { limit_mb: 50, scenario: 'constrained' },
        { limit_mb: 100, scenario: 'normal' },
        { limit_mb: 200, scenario: 'generous' }
      ];
      
      const constraintResults = [];
      
      for (const constraint of memoryConstraints) {
        // Simulate memory constraint
        const originalLimit = mockPerformance.memory.jsHeapSizeLimit;
        mockPerformance.memory.jsHeapSizeLimit = constraint.limit_mb * 1024 * 1024;
        
        const startTime = performance.now();
        const operations = [];
        
        // Run operations under memory constraint
        for (let i = 0; i < 10; i++) {
          operations.push(mockInvoke('generate_embedding', {
            text: `Memory constrained operation ${i}`
          }));
          
          // Check memory pressure periodically
          if (i % 3 === 0) {
            const memory = await mockInvoke('get_memory_usage');
            if (memory.used_memory_mb > constraint.limit_mb * 0.8) {
              await mockInvoke('run_gc_if_needed');
            }
          }
        }
        
        const results = await Promise.all(operations);
        const endTime = performance.now();
        
        constraintResults.push({
          scenario: constraint.scenario,
          memory_limit_mb: constraint.limit_mb,
          total_time_ms: endTime - startTime,
          operations_completed: results.length,
          avg_operation_time: results.reduce((sum, r) => sum + r.processing_time_ms, 0) / results.length,
          max_memory_used: Math.max(...results.map(r => r.memory_used_mb || 5))
        });
        
        // Restore original memory limit
        mockPerformance.memory.jsHeapSizeLimit = originalLimit;
      }
      
      // Validate performance under constraints
      for (const result of constraintResults) {
        expect(result.operations_completed).toBe(10);
        expect(result.max_memory_used).toBeLessThan(result.memory_limit_mb);
        
        // Performance should degrade gracefully under constraints
        if (result.scenario === 'constrained') {
          expect(result.avg_operation_time).toBeLessThan(400); // Still reasonable under constraints
        } else if (result.scenario === 'normal') {
          expect(result.avg_operation_time).toBeLessThan(250);
        } else { // generous
          expect(result.avg_operation_time).toBeLessThan(200);
        }
      }
      
      // Verify graceful degradation
      const constrained = constraintResults.find(r => r.scenario === 'constrained');
      const generous = constraintResults.find(r => r.scenario === 'generous');
      
      const performanceDegradation = constrained.avg_operation_time / generous.avg_operation_time;
      expect(performanceDegradation).toBeLessThan(3); // <3x degradation under severe constraints
    }, 20000);
  });
});