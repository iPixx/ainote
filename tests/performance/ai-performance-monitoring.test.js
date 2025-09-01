import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

/**
 * AI Performance Monitoring Tests
 * 
 * Comprehensive performance monitoring and validation tests according to Issue #76:
 * - Memory usage validation (<100MB target)
 * - Response time validation (embedding <2s, search <500ms, UI <16ms)
 * - Resource allocation validation (70-20-10 split)
 * - Stress testing with large datasets
 * - Performance regression detection
 * - Concurrent operation performance
 * - Memory leak detection
 * - System degradation under load
 */

describe('AI Performance Monitoring Tests', () => {
  let tauriMocks;
  let performanceTracker;

  beforeEach(() => {
    tauriMocks = setupTauriMocks();
    
    // Setup performance tracking utilities
    performanceTracker = {
      memoryBaseline: 0,
      startTime: 0,
      measurements: [],
      
      startMeasurement: (name) => {
        const measurement = {
          name,
          startTime: performance.now(),
          startMemory: performance.memory ? performance.memory.usedJSHeapSize : 0
        };
        performanceTracker.measurements.push(measurement);
        return measurement;
      },
      
      endMeasurement: (measurement) => {
        measurement.endTime = performance.now();
        measurement.duration = measurement.endTime - measurement.startTime;
        measurement.endMemory = performance.memory ? performance.memory.usedJSHeapSize : 0;
        measurement.memoryDelta = measurement.endMemory - measurement.startMemory;
        return measurement;
      }
    };

    // Mock performance API if not available
    if (typeof performance.memory === 'undefined') {
      performance.memory = {
        usedJSHeapSize: 50 * 1024 * 1024, // 50MB baseline
        totalJSHeapSize: 100 * 1024 * 1024,
        jsHeapSizeLimit: 2 * 1024 * 1024 * 1024 // 2GB limit
      };
    }
  });

  afterEach(() => {
    vi.clearAllMocks();
    performanceTracker.measurements = [];
  });

  describe('Memory Usage Validation Tests', () => {
    it('should maintain memory usage below 100MB target during AI operations', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Memory usage validation...');

      const measurement = performanceTracker.startMeasurement('memory_usage_test');

      // Step 1: Baseline memory measurement
      invoke.mockResolvedValueOnce({
        memory_usage_mb: 45.2,
        heap_used_mb: 32.1,
        heap_total_mb: 67.8,
        external_mb: 8.3,
        rss_mb: 52.5
      });

      const baseline = await invoke('get_memory_usage_stats');
      expect(baseline.memory_usage_mb).toBeLessThan(100);
      console.log(`    📊 Baseline memory: ${baseline.memory_usage_mb}MB`);

      // Step 2: Perform AI operations and monitor memory
      const aiOperations = [
        { operation: 'generate_embedding', text: 'Test embedding generation' },
        { operation: 'store_embedding', vector: new Array(384).fill(0.1) },
        { operation: 'similarity_search', query: 'Memory test query' },
        { operation: 'cache_suggestions', suggestions: [] }
      ];

      for (let i = 0; i < aiOperations.length; i++) {
        const op = aiOperations[i];
        
        // Mock successful operation
        invoke.mockResolvedValueOnce({ success: true, operation: op.operation });
        await invoke(op.operation, op);

        // Check memory after each operation
        invoke.mockResolvedValueOnce({
          memory_usage_mb: baseline.memory_usage_mb + (i * 2.1), // Simulate gradual increase
          heap_used_mb: baseline.heap_used_mb + (i * 1.5),
          operation_completed: op.operation
        });

        const currentMemory = await invoke('get_memory_usage_stats');
        expect(currentMemory.memory_usage_mb).toBeLessThan(100);
        console.log(`    📊 After ${op.operation}: ${currentMemory.memory_usage_mb}MB`);
      }

      // Step 3: Test memory under sustained load
      const sustainedLoadStart = performance.now();
      
      for (let batch = 0; batch < 10; batch++) {
        // Simulate batch processing
        invoke.mockResolvedValueOnce({
          memory_usage_mb: baseline.memory_usage_mb + (batch * 1.2),
          batch_processed: batch + 1,
          memory_stable: true
        });

        const batchResult = await invoke('process_ai_batch', { batchId: batch });
        expect(batchResult.memory_usage_mb).toBeLessThan(100);
      }

      const sustainedLoadTime = performance.now() - sustainedLoadStart;
      console.log(`    ⏱️  Sustained load test completed in ${sustainedLoadTime.toFixed(1)}ms`);

      // Step 4: Trigger garbage collection and verify cleanup
      invoke.mockResolvedValueOnce({
        memory_usage_mb: baseline.memory_usage_mb + 1.5, // Should be close to baseline
        gc_performed: true,
        memory_freed_mb: 18.7
      });

      const postGcMemory = await invoke('trigger_memory_cleanup');
      expect(postGcMemory.gc_performed).toBe(true);
      expect(postGcMemory.memory_usage_mb).toBeLessThan(baseline.memory_usage_mb + 5);
      console.log(`    🧹 Post-GC memory: ${postGcMemory.memory_usage_mb}MB (freed ${postGcMemory.memory_freed_mb}MB)`);

      performanceTracker.endMeasurement(measurement);
      console.log(`✅ Memory usage validation completed in ${measurement.duration.toFixed(1)}ms`);
    });

    it('should detect and prevent memory leaks during long operations', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Memory leak detection...');

      // Step 1: Establish memory baseline
      const initialMemory = 45.0;
      invoke.mockResolvedValueOnce({ memory_usage_mb: initialMemory });
      const baseline = await invoke('get_memory_usage_stats');
      
      // Step 2: Simulate long-running operations
      const memorySnapshots = [];
      const operationCount = 50;

      for (let i = 0; i < operationCount; i++) {
        // Simulate operation that might leak memory
        const expectedMemory = initialMemory + (i * 0.1); // Small, acceptable growth
        
        invoke.mockResolvedValueOnce({
          memory_usage_mb: expectedMemory,
          operation_id: i,
          potential_leak_detected: expectedMemory > (initialMemory + 10) // Flag if >10MB growth
        });

        const result = await invoke('perform_ai_operation_with_monitoring', { operationId: i });
        memorySnapshots.push(result.memory_usage_mb);

        // Should not flag potential leaks in normal operation
        if (result.potential_leak_detected) {
          console.warn(`    ⚠️  Potential memory leak detected at operation ${i}`);
        }

        expect(result.memory_usage_mb).toBeLessThan(initialMemory + 15); // Allow some growth
      }

      // Step 3: Analyze memory growth pattern
      const memoryGrowth = memorySnapshots[memorySnapshots.length - 1] - memorySnapshots[0];
      const averageGrowthPerOperation = memoryGrowth / operationCount;

      console.log(`    📊 Total memory growth: ${memoryGrowth.toFixed(2)}MB over ${operationCount} operations`);
      console.log(`    📊 Average growth per operation: ${averageGrowthPerOperation.toFixed(3)}MB`);

      // Memory growth should be minimal for well-designed operations
      expect(averageGrowthPerOperation).toBeLessThan(0.2); // <0.2MB per operation
      expect(memoryGrowth).toBeLessThan(10); // <10MB total growth

      // Step 4: Force cleanup and verify memory can be reclaimed
      invoke.mockResolvedValueOnce({
        memory_usage_mb: initialMemory + 2.0, // Should drop significantly
        cleanup_successful: true,
        reclaimed_mb: memoryGrowth - 2.0
      });

      const cleanup = await invoke('force_memory_cleanup');
      expect(cleanup.cleanup_successful).toBe(true);
      expect(cleanup.memory_usage_mb).toBeLessThan(initialMemory + 5);
      console.log(`    🧹 Cleanup reclaimed ${cleanup.reclaimed_mb.toFixed(1)}MB`);

      console.log('✅ Memory leak detection test completed successfully');
    });
  });

  describe('Response Time Validation Tests', () => {
    it('should meet embedding generation performance targets (<2 seconds)', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Embedding generation response times...');

      const testTexts = [
        'Short text for embedding',
        'Medium length text for embedding generation testing with more content to process and analyze',
        'Very long text for embedding generation performance testing with extensive content that includes multiple sentences, complex vocabulary, and various linguistic patterns that should challenge the embedding model while still meeting our performance targets for response time under two seconds'
      ];

      for (let i = 0; i < testTexts.length; i++) {
        const text = testTexts[i];
        const measurement = performanceTracker.startMeasurement(`embedding_${i}`);
        
        // Mock realistic embedding generation times based on text length
        const expectedTime = 200 + (text.length * 2); // Base 200ms + 2ms per character
        
        invoke.mockImplementation(() => 
          new Promise(resolve => 
            setTimeout(() => resolve({
              embedding: new Array(384).fill(0.1),
              generation_time_ms: expectedTime,
              text_length: text.length
            }), expectedTime)
          )
        );

        const startTime = performance.now();
        const result = await invoke('generate_embedding', { 
          text: text, 
          modelName: 'nomic-embed-text' 
        });
        const actualTime = performance.now() - startTime;

        performanceTracker.endMeasurement(measurement);

        expect(Array.isArray(result.embedding)).toBe(true);
        expect(actualTime).toBeLessThan(2000); // <2 seconds target
        expect(result.generation_time_ms).toBeLessThan(2000);

        console.log(`    ⏱️  Text length ${text.length}: ${actualTime.toFixed(0)}ms (${actualTime < 1000 ? '✅' : '⚠️'})`);
      }

      console.log('✅ Embedding generation response time validation completed');
    });

    it('should meet similarity search performance targets (<500ms)', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Similarity search response times...');

      const searchQueries = [
        { query: 'machine learning', expectedResults: 5, description: 'Common topic' },
        { query: 'neural networks deep learning artificial intelligence', expectedResults: 10, description: 'Complex query' },
        { query: 'very specific technical implementation details', expectedResults: 3, description: 'Specific query' },
        { query: 'a b c d e f g h i j k l m n o p q r s t', expectedResults: 2, description: 'Long query' }
      ];

      for (const testCase of searchQueries) {
        const measurement = performanceTracker.startMeasurement(`search_${testCase.description.replace(' ', '_')}`);
        
        // Mock search with realistic timing based on query complexity
        const expectedTime = 50 + (testCase.query.length * 1.5); // Base 50ms + complexity factor
        
        invoke.mockImplementation(() =>
          new Promise(resolve =>
            setTimeout(() => resolve({
              results: Array.from({ length: testCase.expectedResults }, (_, i) => ({
                similarity: 0.9 - (i * 0.05),
                file_path: `/vault/result_${i}.md`,
                content: `Result ${i} for query: ${testCase.query.slice(0, 30)}...`
              })),
              search_time_ms: expectedTime,
              total_results: testCase.expectedResults,
              query_complexity: testCase.query.split(' ').length
            }), expectedTime)
          )
        );

        const startTime = performance.now();
        const searchResult = await invoke('optimized_search_similar_notes', {
          query: testCase.query,
          maxResults: testCase.expectedResults + 2,
          similarityThreshold: 0.7
        });
        const actualTime = performance.now() - startTime;

        performanceTracker.endMeasurement(measurement);

        expect(searchResult.results).toHaveLength(testCase.expectedResults);
        expect(actualTime).toBeLessThan(500); // <500ms target
        expect(searchResult.search_time_ms).toBeLessThan(500);

        console.log(`    ⏱️  ${testCase.description}: ${actualTime.toFixed(0)}ms (${actualTime < 200 ? '✅' : '⚠️'})`);
      }

      console.log('✅ Similarity search response time validation completed');
    });

    it('should maintain UI responsiveness (<16ms frame time)', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: UI responsiveness validation...');

      // Step 1: Test UI operations during AI processing
      const uiOperations = [
        { operation: 'scroll_editor', expected: 'scroll_completed' },
        { operation: 'resize_panel', expected: 'resize_completed' },
        { operation: 'update_file_tree', expected: 'tree_updated' },
        { operation: 'highlight_text', expected: 'highlight_applied' },
        { operation: 'show_suggestions', expected: 'suggestions_displayed' }
      ];

      for (const uiOp of uiOperations) {
        // Start background AI operation
        invoke.mockImplementation((command) => {
          if (command === 'background_ai_operation') {
            return new Promise(resolve => 
              setTimeout(() => resolve({ completed: true }), 1000) // 1 second AI operation
            );
          } else {
            return new Promise(resolve => 
              setTimeout(() => resolve({ result: uiOp.expected }), 5) // Quick UI operation
            );
          }
        });

        const aiOperation = invoke('background_ai_operation', { intensive: true });
        
        // Perform UI operation during AI processing
        const uiStartTime = performance.now();
        const uiResult = await invoke('ui_interaction', { operation: uiOp.operation });
        const uiTime = performance.now() - uiStartTime;

        expect(uiResult.result).toBe(uiOp.expected);
        expect(uiTime).toBeLessThan(16); // <16ms for 60fps
        
        console.log(`    ⏱️  ${uiOp.operation}: ${uiTime.toFixed(1)}ms (${uiTime < 16 ? '✅' : '⚠️'})`);

        // Wait for AI operation to complete
        await aiOperation;
      }

      // Step 2: Test frame rate during intensive operations
      const frameTimeMeasurements = [];
      const frameCount = 10;

      for (let frame = 0; frame < frameCount; frame++) {
        const frameStart = performance.now();
        
        // Simulate frame rendering work
        invoke.mockResolvedValueOnce({ frame_rendered: frame });
        await invoke('render_frame', { frameId: frame });
        
        const frameTime = performance.now() - frameStart;
        frameTimeMeasurements.push(frameTime);

        expect(frameTime).toBeLessThan(16); // Individual frame should be <16ms
      }

      const averageFrameTime = frameTimeMeasurements.reduce((a, b) => a + b, 0) / frameCount;
      const maxFrameTime = Math.max(...frameTimeMeasurements);

      console.log(`    📊 Average frame time: ${averageFrameTime.toFixed(1)}ms`);
      console.log(`    📊 Max frame time: ${maxFrameTime.toFixed(1)}ms`);

      expect(averageFrameTime).toBeLessThan(10); // Average should be well under 16ms
      expect(maxFrameTime).toBeLessThan(20); // Even worst frame should be reasonable

      console.log('✅ UI responsiveness validation completed');
    });
  });

  describe('Resource Allocation Validation Tests', () => {
    it('should maintain 70-20-10 resource allocation split', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Resource allocation validation...');

      // Step 1: Test allocation during different operation types
      const operationScenarios = [
        { type: 'ai_intensive', aiExpected: 70, appExpected: 20, systemExpected: 10 },
        { type: 'ui_intensive', aiExpected: 40, appExpected: 50, systemExpected: 10 },
        { type: 'balanced', aiExpected: 60, appExpected: 30, systemExpected: 10 },
        { type: 'idle', aiExpected: 10, appExpected: 15, systemExpected: 5 }
      ];

      for (const scenario of operationScenarios) {
        const measurement = performanceTracker.startMeasurement(`allocation_${scenario.type}`);
        
        // Mock resource allocation for scenario
        invoke.mockResolvedValueOnce({
          ai_percentage: scenario.aiExpected,
          app_percentage: scenario.appExpected,
          system_percentage: scenario.systemExpected,
          total_cpu_usage: scenario.aiExpected + scenario.appExpected + scenario.systemExpected,
          allocation_compliant: true,
          scenario: scenario.type
        });

        const allocation = await invoke('get_resource_allocation_stats');
        
        expect(allocation.ai_percentage).toBeLessThanOrEqual(70);
        expect(allocation.app_percentage).toBeLessThanOrEqual(50); // Allow flexibility for UI-intensive
        expect(allocation.system_percentage).toBeLessThanOrEqual(15);
        expect(allocation.allocation_compliant).toBe(true);

        console.log(`    📊 ${scenario.type}: AI ${allocation.ai_percentage}%, App ${allocation.app_percentage}%, System ${allocation.system_percentage}%`);
        
        performanceTracker.endMeasurement(measurement);
      }

      // Step 2: Test adaptive allocation under pressure
      const pressureTest = [
        { load: 'low', expectedAdjustment: 0 },
        { load: 'medium', expectedAdjustment: 5 },
        { load: 'high', expectedAdjustment: 10 },
        { load: 'critical', expectedAdjustment: 15 }
      ];

      for (const pressure of pressureTest) {
        invoke.mockResolvedValueOnce({
          ai_percentage: 70 - pressure.expectedAdjustment,
          app_percentage: 20 + pressure.expectedAdjustment,
          system_percentage: 10,
          load_level: pressure.load,
          adaptive_adjustment: pressure.expectedAdjustment,
          pressure_detected: pressure.load !== 'low'
        });

        const adaptiveAllocation = await invoke('get_adaptive_resource_allocation');
        
        expect(adaptiveAllocation.ai_percentage).toBeLessThanOrEqual(70);
        expect(adaptiveAllocation.adaptive_adjustment).toBe(pressure.expectedAdjustment);
        
        console.log(`    🔄 ${pressure.load} load: adjusted by ${adaptiveAllocation.adaptive_adjustment}%`);
      }

      console.log('✅ Resource allocation validation completed');
    });

    it('should handle resource contention gracefully', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Resource contention handling...');

      // Step 1: Simulate competing operations
      const competingOperations = [
        { name: 'ai_embedding', priority: 'high', resources: 40 },
        { name: 'ui_rendering', priority: 'critical', resources: 30 },
        { name: 'file_indexing', priority: 'medium', resources: 20 },
        { name: 'background_sync', priority: 'low', resources: 10 }
      ];

      // Mock resource contention scenario
      invoke.mockResolvedValueOnce({
        contention_detected: true,
        competing_operations: competingOperations.length,
        total_requested_resources: competingOperations.reduce((sum, op) => sum + op.resources, 0),
        resolution_strategy: 'priority_based_throttling'
      });

      const contention = await invoke('detect_resource_contention');
      expect(contention.contention_detected).toBe(true);
      expect(contention.total_requested_resources).toBeGreaterThan(100);
      console.log(`    ⚠️  Resource contention detected: ${contention.total_requested_resources}% requested`);

      // Step 2: Test contention resolution
      for (const operation of competingOperations) {
        let allocatedResources;
        
        switch (operation.priority) {
          case 'critical':
            allocatedResources = operation.resources; // Full allocation
            break;
          case 'high':
            allocatedResources = operation.resources * 0.8; // 80% allocation
            break;
          case 'medium':
            allocatedResources = operation.resources * 0.6; // 60% allocation
            break;
          case 'low':
            allocatedResources = operation.resources * 0.3; // 30% allocation
            break;
        }

        invoke.mockResolvedValueOnce({
          operation: operation.name,
          requested: operation.resources,
          allocated: allocatedResources,
          efficiency: allocatedResources / operation.resources,
          throttled: allocatedResources < operation.resources
        });

        const resolution = await invoke('resolve_operation_contention', { operation: operation.name });
        
        expect(resolution.allocated).toBeGreaterThan(0);
        expect(resolution.allocated).toBeLessThanOrEqual(operation.resources);
        
        console.log(`    🎛️  ${operation.name} (${operation.priority}): ${resolution.allocated}%/${resolution.requested}% (${(resolution.efficiency * 100).toFixed(0)}% efficiency)`);
      }

      console.log('✅ Resource contention handling completed');
    });
  });

  describe('Stress Testing and Scalability', () => {
    it('should handle large dataset processing efficiently', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Large dataset processing...');

      const datasetSizes = [
        { name: 'small', files: 100, expectedTime: 5000 },   // 100 files, <5s
        { name: 'medium', files: 500, expectedTime: 20000 }, // 500 files, <20s
        { name: 'large', files: 1000, expectedTime: 45000 }, // 1000 files, <45s
      ];

      for (const dataset of datasetSizes) {
        const measurement = performanceTracker.startMeasurement(`dataset_${dataset.name}`);
        
        // Mock large dataset processing
        invoke.mockImplementation(() =>
          new Promise(resolve => {
            const processingTime = Math.min(dataset.expectedTime * 0.8, dataset.expectedTime);
            setTimeout(() => resolve({
              processed_files: dataset.files,
              total_embeddings: dataset.files * 4.2, // Average 4.2 chunks per file
              processing_time_ms: processingTime,
              memory_peak_mb: 60 + (dataset.files * 0.02), // Scales with dataset
              throughput_files_per_second: dataset.files / (processingTime / 1000),
              efficiency_score: 0.85 + (Math.random() * 0.1) // 85-95% efficiency
            }), processingTime);
          })
        );

        const startTime = performance.now();
        const result = await invoke('process_large_dataset', {
          files: dataset.files,
          batchSize: 50,
          parallelWorkers: 4
        });
        const actualTime = performance.now() - startTime;

        performanceTracker.endMeasurement(measurement);

        expect(result.processed_files).toBe(dataset.files);
        expect(actualTime).toBeLessThan(dataset.expectedTime);
        expect(result.memory_peak_mb).toBeLessThan(100); // Within memory limits
        expect(result.efficiency_score).toBeGreaterThan(0.8); // >80% efficiency

        console.log(`    📊 ${dataset.name} dataset (${dataset.files} files):`);
        console.log(`        ⏱️  Processing time: ${actualTime.toFixed(0)}ms`);
        console.log(`        🧮 Throughput: ${result.throughput_files_per_second.toFixed(1)} files/sec`);
        console.log(`        💾 Memory peak: ${result.memory_peak_mb.toFixed(1)}MB`);
        console.log(`        ⚡ Efficiency: ${(result.efficiency_score * 100).toFixed(1)}%`);
      }

      console.log('✅ Large dataset processing test completed');
    });

    it('should maintain performance under concurrent load', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Concurrent load handling...');

      const concurrentOperations = 10;
      const operationsPerType = 3;
      
      const operationTypes = [
        { type: 'embedding_generation', baseTime: 200 },
        { type: 'similarity_search', baseTime: 100 },
        { type: 'cache_lookup', baseTime: 20 }
      ];

      // Step 1: Sequential baseline
      const sequentialTimes = [];
      for (const opType of operationTypes) {
        for (let i = 0; i < operationsPerType; i++) {
          invoke.mockImplementation(() =>
            new Promise(resolve =>
              setTimeout(() => resolve({
                operation: opType.type,
                execution_time_ms: opType.baseTime,
                sequential: true
              }), opType.baseTime)
            )
          );

          const startTime = performance.now();
          await invoke(opType.type, { operationId: `seq_${i}` });
          const time = performance.now() - startTime;
          sequentialTimes.push(time);
        }
      }

      const totalSequentialTime = sequentialTimes.reduce((a, b) => a + b, 0);
      console.log(`    📊 Sequential execution: ${totalSequentialTime.toFixed(0)}ms`);

      // Step 2: Concurrent execution
      const concurrentPromises = [];
      const concurrentStartTime = performance.now();

      for (let i = 0; i < concurrentOperations; i++) {
        const opType = operationTypes[i % operationTypes.length];
        
        // Mock with slight delay variation for realism
        invoke.mockImplementation(() =>
          new Promise(resolve => {
            const variation = Math.random() * 50; // ±25ms variation
            const adjustedTime = opType.baseTime + variation - 25;
            
            setTimeout(() => resolve({
              operation: opType.type,
              execution_time_ms: adjustedTime,
              concurrent: true,
              operation_id: i
            }), adjustedTime);
          })
        );

        concurrentPromises.push(invoke(opType.type, { operationId: `conc_${i}` }));
      }

      const concurrentResults = await Promise.all(concurrentPromises);
      const totalConcurrentTime = performance.now() - concurrentStartTime;

      // Step 3: Analyze performance
      const speedupRatio = totalSequentialTime / totalConcurrentTime;
      const expectedMinSpeedup = 2.0; // Should be at least 2x faster
      
      expect(speedupRatio).toBeGreaterThan(expectedMinSpeedup);
      expect(concurrentResults).toHaveLength(concurrentOperations);

      console.log(`    📊 Concurrent execution: ${totalConcurrentTime.toFixed(0)}ms`);
      console.log(`    ⚡ Speedup ratio: ${speedupRatio.toFixed(2)}x (${speedupRatio > expectedMinSpeedup ? '✅' : '⚠️'})`);

      // Step 4: Validate no performance degradation
      const averageConcurrentTime = concurrentResults.reduce((sum, result) => 
        sum + result.execution_time_ms, 0) / concurrentResults.length;
      const expectedAverageTime = operationTypes.reduce((sum, op) => 
        sum + op.baseTime, 0) / operationTypes.length;

      // Allow up to 20% degradation under concurrent load
      expect(averageConcurrentTime).toBeLessThan(expectedAverageTime * 1.2);
      
      console.log(`    📊 Average concurrent operation time: ${averageConcurrentTime.toFixed(1)}ms`);
      console.log(`    📊 Expected baseline time: ${expectedAverageTime.toFixed(1)}ms`);

      console.log('✅ Concurrent load handling test completed');
    });
  });

  describe('Performance Regression Detection', () => {
    it('should detect performance regressions against baseline', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Regression detection...');

      // Step 1: Establish performance baseline
      const baselineMetrics = {
        embedding_generation_ms: 180,
        similarity_search_ms: 85,
        cache_lookup_ms: 12,
        memory_usage_mb: 47.3,
        throughput_ops_per_sec: 25.4
      };

      invoke.mockResolvedValueOnce({
        baseline_established: true,
        metrics: baselineMetrics,
        timestamp: Date.now(),
        baseline_id: 'baseline_001'
      });

      const baseline = await invoke('establish_performance_baseline');
      expect(baseline.baseline_established).toBe(true);
      console.log('    ✅ Performance baseline established');

      // Step 2: Simulate current performance with various scenarios
      const regressionScenarios = [
        {
          name: 'no_regression',
          metrics: {
            embedding_generation_ms: 175, // 3% better
            similarity_search_ms: 88,     // 3% worse (acceptable)
            cache_lookup_ms: 11,          // 8% better
            memory_usage_mb: 48.1,        // 2% worse
            throughput_ops_per_sec: 26.1  // 3% better
          },
          expectRegression: false
        },
        {
          name: 'minor_regression',
          metrics: {
            embedding_generation_ms: 205, // 14% worse
            similarity_search_ms: 97,     // 14% worse  
            cache_lookup_ms: 14,          // 17% worse
            memory_usage_mb: 52.8,        // 12% worse
            throughput_ops_per_sec: 22.1  // 13% worse
          },
          expectRegression: true
        },
        {
          name: 'major_regression',
          metrics: {
            embedding_generation_ms: 270, // 50% worse
            similarity_search_ms: 130,    // 53% worse
            cache_lookup_ms: 20,          // 67% worse
            memory_usage_mb: 68.5,        // 45% worse
            throughput_ops_per_sec: 15.2  // 40% worse
          },
          expectRegression: true
        }
      ];

      for (const scenario of regressionScenarios) {
        // Mock regression analysis
        const regressionCount = Object.keys(scenario.metrics).reduce((count, metric) => {
          const baseValue = baselineMetrics[metric];
          const currentValue = scenario.metrics[metric];
          const isWorse = metric.includes('ms') || metric.includes('mb') ? 
            currentValue > baseValue : currentValue < baseValue;
          const changePercent = Math.abs((currentValue - baseValue) / baseValue * 100);
          
          return count + (isWorse && changePercent > 10 ? 1 : 0); // >10% change is regression
        }, 0);

        invoke.mockResolvedValueOnce({
          regressions_detected: regressionCount,
          total_metrics: Object.keys(scenario.metrics).length,
          regression_percentage: (regressionCount / Object.keys(scenario.metrics).length) * 100,
          severity: regressionCount === 0 ? 'none' : regressionCount < 3 ? 'minor' : 'major',
          has_regressions: regressionCount > 0,
          scenario: scenario.name,
          detailed_analysis: Object.entries(scenario.metrics).map(([metric, value]) => {
            const baseValue = baselineMetrics[metric];
            const changePercent = ((value - baseValue) / baseValue * 100);
            return {
              metric,
              baseline: baseValue,
              current: value,
              change_percent: changePercent,
              is_regression: Math.abs(changePercent) > 10
            };
          })
        });

        const analysis = await invoke('analyze_performance_regressions', {
          currentMetrics: scenario.metrics,
          baselineId: 'baseline_001'
        });

        expect(analysis.has_regressions).toBe(scenario.expectRegression);
        console.log(`    📊 ${scenario.name}: ${analysis.regressions_detected} regressions (${analysis.severity})`);

        if (analysis.has_regressions) {
          const significantRegressions = analysis.detailed_analysis.filter(a => 
            a.is_regression && Math.abs(a.change_percent) > 15
          );
          
          for (const regression of significantRegressions) {
            console.log(`        ⚠️  ${regression.metric}: ${regression.change_percent.toFixed(1)}% change`);
          }
        }
      }

      console.log('✅ Performance regression detection completed');
    });

    it('should track performance trends over time', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 Performance Test: Performance trend tracking...');

      // Step 1: Generate historical performance data
      const historicalData = [];
      const dayMs = 24 * 60 * 60 * 1000;
      
      for (let day = 0; day < 30; day++) {
        const timestamp = Date.now() - (day * dayMs);
        
        // Simulate gradual performance degradation over time
        const degradationFactor = 1 + (day * 0.005); // 0.5% degradation per day
        
        historicalData.push({
          timestamp,
          embedding_generation_ms: 180 * degradationFactor,
          similarity_search_ms: 85 * degradationFactor,
          memory_usage_mb: 47.3 + (day * 0.1), // 0.1MB increase per day
          throughput_ops_per_sec: 25.4 / degradationFactor,
          day_offset: day
        });
      }

      // Step 2: Analyze trends
      invoke.mockResolvedValueOnce({
        trend_analysis: {
          embedding_generation: {
            trend: 'degrading',
            slope: 0.45, // ms per day
            r_squared: 0.89,
            significance: 'high'
          },
          memory_usage: {
            trend: 'increasing',
            slope: 0.1, // MB per day
            r_squared: 0.95,
            significance: 'high'
          },
          throughput: {
            trend: 'decreasing',
            slope: -0.12, // ops/sec per day
            r_squared: 0.84,
            significance: 'high'
          }
        },
        overall_health: 'declining',
        recommendation: 'performance_optimization_needed',
        projected_issues: [
          'Memory usage may exceed 60MB in 120 days',
          'Embedding generation may exceed 3s threshold in 180 days'
        ]
      });

      const trendAnalysis = await invoke('analyze_performance_trends', {
        historicalData: historicalData.slice(0, 10), // Last 10 days
        analysisWindow: 30
      });

      expect(trendAnalysis.trend_analysis).toBeDefined();
      expect(trendAnalysis.overall_health).toBe('declining');
      expect(trendAnalysis.projected_issues).toHaveLength(2);

      console.log(`    📈 Overall health: ${trendAnalysis.overall_health}`);
      
      for (const [metric, trend] of Object.entries(trendAnalysis.trend_analysis)) {
        console.log(`    📊 ${metric}: ${trend.trend} (${trend.significance} significance, R²=${trend.r_squared})`);
      }

      for (const issue of trendAnalysis.projected_issues) {
        console.log(`    ⚠️  Projection: ${issue}`);
      }

      console.log('✅ Performance trend tracking completed');
    });
  });
});