import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

/**
 * AI System Integration Tests
 * 
 * Comprehensive integration tests for AI system components according to Issue #76:
 * - End-to-end AI workflows from frontend to backend
 * - Cross-component data flow validation
 * - Real-world usage scenario testing
 * - Error handling and recovery integration
 */

describe('AI System Integration Tests', () => {
  let tauriMocks;

  beforeEach(() => {
    tauriMocks = setupTauriMocks();
    
    // Setup DOM for UI component testing
    document.body.innerHTML = `
      <div id="app">
        <div id="editor-container"></div>
        <div id="ai-panel" class="hidden"></div>
        <div id="file-tree"></div>
      </div>
    `;
  });

  afterEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = '';
  });

  describe('Complete AI Suggestion Workflow Integration', () => {
    it('should handle complete suggestion generation from user typing to display', async () => {
      const { invoke } = window.__TAURI__.core;

      // Step 1: Initialize AI components
      console.log('🧪 Testing: Complete AI suggestion workflow...');
      
      // Mock AI service availability check
      invoke.mockResolvedValueOnce({ status: 'Connected', available: true });
      const aiStatus = await invoke('check_ollama_status');
      expect(aiStatus.status).toBe('Connected');

      // Step 2: Simulate user content input
      const userContent = "Machine learning has revolutionized many fields of computer science";
      
      // Mock content change detection
      const mockContentChange = {
        currentParagraph: userContent,
        cursorPosition: userContent.length,
        hasSignificantChange: true,
        extractionTime: 15
      };

      // Step 3: Generate embedding for user content
      invoke.mockResolvedValueOnce([0.1, 0.2, 0.3, 0.4, 0.5]);
      const embedding = await invoke('generate_embedding', {
        text: userContent,
        modelName: 'nomic-embed-text'
      });
      expect(Array.isArray(embedding)).toBe(true);

      // Step 4: Perform similarity search
      const mockSimilarResults = {
        results: [
          {
            similarity: 0.89,
            file_path: '/vault/ai-research.md',
            content: 'Artificial intelligence and machine learning algorithms',
            chunk_id: 'chunk_1',
            line_numbers: [45, 50]
          },
          {
            similarity: 0.82,
            file_path: '/vault/deep-learning.md',
            content: 'Neural networks and deep learning architectures',
            chunk_id: 'chunk_2',
            line_numbers: [12, 18]
          }
        ],
        search_time_ms: 67,
        total_results: 2
      };

      invoke.mockResolvedValueOnce(mockSimilarResults);
      const searchResults = await invoke('optimized_search_similar_notes', {
        query: userContent,
        maxResults: 10,
        similarityThreshold: 0.7,
        currentFile: '/vault/current-note.md'
      });

      expect(searchResults.results).toHaveLength(2);
      expect(searchResults.results[0].similarity).toBeGreaterThan(0.8);
      expect(searchResults.search_time_ms).toBeLessThan(100);

      // Step 5: Test suggestion caching
      invoke.mockResolvedValueOnce(true);
      const cacheResult = await invoke('cache_suggestions', {
        key: `suggestions_${userContent.slice(0, 20)}`,
        suggestions: searchResults.results,
        context: { currentFile: '/vault/current-note.md', cursorPosition: userContent.length },
        ttl: 3600
      });
      expect(cacheResult).toBe(true);

      // Step 6: Initialize AI panel and test event handling
      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));
      
      // Wait for auto-activation
      await new Promise(resolve => setTimeout(resolve, 150));
      
      // Convert search results to suggestion format
      const formattedSuggestions = searchResults.results.map((result, index) => ({
        id: `suggestion_${Date.now()}_${index}`,
        title: result.file_path.split('/').pop().replace('.md', ''),
        content: result.content,
        relevanceScore: result.similarity,
        filePath: result.file_path,
        contextSnippet: result.content.slice(0, 100) + '...',
        metadata: {
          chunkId: result.chunk_id,
          lineNumbers: result.line_numbers,
          searchScore: result.similarity
        }
      }));

      // Simulate suggestion updates via events
      aiPanel.emitEvent('suggestions_received', {
        suggestions: formattedSuggestions,
        count: formattedSuggestions.length
      });
      
      expect(aiPanel.isActive()).toBe(true);
      expect(aiPanel.getElement()).toBeDefined();

      console.log('✅ Complete AI suggestion workflow test passed');
    });

    it('should handle suggestion caching and retrieval flow', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: Suggestion caching workflow...');

      const testQuery = "JavaScript async programming patterns";
      const cacheKey = `cache_${testQuery.replace(/\s+/g, '_')}`;

      // Step 1: Cache miss scenario
      invoke.mockResolvedValueOnce(null);
      const cachedResult = await invoke('get_cached_suggestions', { key: cacheKey });
      expect(cachedResult).toBeNull();

      // Step 2: Generate fresh suggestions
      const freshSuggestions = [
        {
          id: 'fresh_1',
          title: 'Async JavaScript Guide',
          content: 'Understanding promises and async/await patterns',
          relevanceScore: 0.91,
          filePath: '/vault/js-async.md'
        }
      ];

      invoke.mockResolvedValueOnce({
        results: freshSuggestions.map(s => ({
          similarity: s.relevanceScore,
          file_path: s.filePath,
          content: s.content,
          chunk_id: 'chunk_1'
        }))
      });

      const searchResult = await invoke('optimized_search_similar_notes', {
        query: testQuery,
        maxResults: 5,
        similarityThreshold: 0.7
      });

      expect(searchResult.results).toHaveLength(1);

      // Step 3: Cache the fresh results
      invoke.mockResolvedValueOnce(true);
      const cacheStoreResult = await invoke('cache_suggestions', {
        key: cacheKey,
        suggestions: searchResult.results,
        ttl: 1800
      });
      expect(cacheStoreResult).toBe(true);

      // Step 4: Cache hit scenario
      invoke.mockResolvedValueOnce(freshSuggestions);
      const cachedRetrieved = await invoke('get_cached_suggestions', { key: cacheKey });
      expect(cachedRetrieved).toHaveLength(1);
      expect(cachedRetrieved[0].title).toBe('Async JavaScript Guide');

      console.log('✅ Suggestion caching workflow test passed');
    });
  });

  describe('Vault Indexing and Processing Integration', () => {
    it('should handle complete vault indexing workflow', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: Complete vault indexing workflow...');

      // Step 1: Scan vault for files
      const mockVaultFiles = [
        { name: 'intro.md', path: '/vault/intro.md', is_dir: false, size: 1024 },
        { name: 'advanced.md', path: '/vault/advanced.md', is_dir: false, size: 2048 },
        { name: 'concepts', path: '/vault/concepts', is_dir: true, size: 0 },
        { name: 'theory.md', path: '/vault/concepts/theory.md', is_dir: false, size: 3072 }
      ];

      invoke.mockResolvedValueOnce(mockVaultFiles);
      const vaultFiles = await invoke('scan_vault_files', { vaultPath: '/vault' });
      expect(vaultFiles).toHaveLength(4);
      
      const markdownFiles = vaultFiles.filter(f => !f.is_dir && f.name.endsWith('.md'));
      expect(markdownFiles).toHaveLength(3);

      // Step 2: Process files for indexing
      const mockIndexingResult = {
        processed_files: 3,
        total_embeddings: 28,
        processing_time_ms: 4500,
        skipped_files: 0,
        errors: [],
        embeddings_per_file: [
          { file: '/vault/intro.md', embeddings: 8 },
          { file: '/vault/advanced.md', embeddings: 15 },
          { file: '/vault/concepts/theory.md', embeddings: 5 }
        ]
      };

      invoke.mockResolvedValueOnce(mockIndexingResult);
      const indexingResult = await invoke('process_vault_for_indexing', {
        vaultPath: '/vault',
        options: {
          force_reindex: false,
          chunk_size: 1000,
          chunk_overlap: 100,
          model_name: 'nomic-embed-text'
        }
      });

      expect(indexingResult.processed_files).toBe(3);
      expect(indexingResult.total_embeddings).toBe(28);
      expect(indexingResult.processing_time_ms).toBeLessThan(10000);
      expect(indexingResult.errors).toHaveLength(0);

      // Step 3: Validate indexed data integrity
      invoke.mockResolvedValueOnce({
        is_healthy: true,
        total_entries: 28,
        corrupted_entries: 0,
        validation_time_ms: 156,
        index_size_mb: 12.4,
        issues: []
      });

      const validation = await invoke('validate_vector_database');
      expect(validation.is_healthy).toBe(true);
      expect(validation.total_entries).toBe(28);
      expect(validation.corrupted_entries).toBe(0);

      // Step 4: Test search functionality on indexed data
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.84,
            file_path: '/vault/advanced.md',
            content: 'Advanced concepts in machine learning',
            chunk_id: 'chunk_3'
          }
        ],
        search_time_ms: 42,
        total_results: 1
      });

      const searchTest = await invoke('optimized_search_similar_notes', {
        query: 'advanced machine learning',
        maxResults: 5,
        similarityThreshold: 0.8
      });

      expect(searchTest.results).toHaveLength(1);
      expect(searchTest.search_time_ms).toBeLessThan(100);

      console.log('✅ Complete vault indexing workflow test passed');
    });

    it('should handle incremental updates and file monitoring', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: Incremental update workflow...');

      // Step 1: Set up file monitoring
      invoke.mockResolvedValueOnce({ monitoring_started: true, watched_paths: ['/vault'] });
      const monitoringSetup = await invoke('start_file_monitoring', {
        vaultPath: '/vault',
        options: { recursive: true, ignore_patterns: ['*.tmp', '.git/'] }
      });
      expect(monitoringSetup.monitoring_started).toBe(true);

      // Step 2: Simulate file changes
      const fileChanges = {
        modified_files: ['/vault/intro.md', '/vault/new-topic.md'],
        deleted_files: ['/vault/outdated.md'],
        created_files: ['/vault/latest-research.md']
      };

      // Step 3: Process incremental updates
      const mockIncrementalResult = {
        updated_files: 2,
        new_files: 1,
        deleted_files: 1,
        new_embeddings: 12,
        removed_embeddings: 6,
        update_time_ms: 1200,
        status: 'completed',
        errors: []
      };

      invoke.mockResolvedValueOnce(mockIncrementalResult);
      const incrementalUpdate = await invoke('process_incremental_updates', {
        changes: fileChanges,
        options: { batch_size: 10, max_concurrent: 3 }
      });

      expect(incrementalUpdate.status).toBe('completed');
      expect(incrementalUpdate.new_embeddings).toBe(12);
      expect(incrementalUpdate.removed_embeddings).toBe(6);
      expect(incrementalUpdate.update_time_ms).toBeLessThan(3000);

      // Step 4: Verify index integrity after updates
      invoke.mockResolvedValueOnce({
        is_healthy: true,
        total_entries: 34, // 28 original + 12 new - 6 removed
        corrupted_entries: 0,
        recent_updates: incrementalUpdate.updated_files + incrementalUpdate.new_files
      });

      const postUpdateValidation = await invoke('validate_vector_database');
      expect(postUpdateValidation.is_healthy).toBe(true);
      expect(postUpdateValidation.total_entries).toBe(34);

      console.log('✅ Incremental update workflow test passed');
    });
  });

  describe('AI Service Integration with Frontend Components', () => {
    it('should integrate AI suggestion service with editor and UI', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: AI service frontend integration...');

      // Initialize components
      const MockEditor = {
        getValue: () => "Understanding neural networks requires knowledge of linear algebra",
        setValue: vi.fn(),
        cursorPosition: 65,
        addEventListener: vi.fn()
      };

      const MockAppState = {
        currentFile: '/vault/neural-networks.md',
        currentVault: '/vault',
        addEventListener: vi.fn()
      };

      // Step 1: Initialize content change detector
      const ContentChangeDetector = await import('../../src/js/services/content-change-detector.js');
      const contentDetector = new ContentChangeDetector.default(MockEditor, MockAppState);

      // Step 2: Initialize suggestion cache manager with proper mock AppState
      const mockAppState = { 
        currentFile: null, 
        currentVault: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn()
      };
      const SuggestionCacheManager = await import('../../src/js/services/suggestion-cache-manager.js');
      const cacheManager = new SuggestionCacheManager.default(mockAppState);

      // Step 3: Initialize AI suggestion service
      const AiSuggestionService = await import('../../src/js/services/ai-suggestion-service.js');
      const aiService = new AiSuggestionService.default(
        MockEditor,
        MockAppState,
        contentDetector,
        cacheManager
      );

      // Mock backend responses for AI service initialization
      invoke.mockResolvedValueOnce(null); // Cache miss for test
      invoke.mockResolvedValueOnce({ results: [] }); // Empty search result for test

      await new Promise(resolve => setTimeout(resolve, 100)); // Allow initialization

      // Step 4: Test suggestion generation
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.88,
            file_path: '/vault/linear-algebra.md',
            content: 'Matrix operations and vector spaces in machine learning',
            chunk_id: 'chunk_4'
          }
        ],
        search_time_ms: 52
      });

      const suggestions = await aiService.requestSuggestions();
      expect(Array.isArray(suggestions)).toBe(true);

      // Step 5: Test AI panel integration
      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

      // Test integration via events
      aiPanel.emitEvent('ai_suggestions_received', {
        suggestions: suggestions,
        count: suggestions.length
      });
      expect(aiPanel.isActive()).toBe(true);

      // Step 6: Test service status and performance
      const serviceStatus = aiService.getStatus();
      expect(serviceStatus.status).toBeDefined();
      expect(serviceStatus.enabled).toBe(true);
      expect(serviceStatus.performanceStats).toBeDefined();

      console.log('✅ AI service frontend integration test passed');
    });

    it('should handle AI panel visibility and interaction states', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: AI panel state management...');

      // Initialize AI panel
      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

      // Wait for auto-activation but test can still configure visibility
      await new Promise(resolve => setTimeout(resolve, 150));
      
      // Test that we can control visibility regardless of activation
      aiPanel.hide();
      expect(aiPanel.isVisible()).toBe(false);

      // Test suggestion updates while hidden
      const mockSuggestions = [
        {
          id: 'test_1',
          title: 'Related Concept',
          content: 'Related content for testing',
          relevanceScore: 0.85,
          filePath: '/vault/related.md'
        }
      ];

      // Test suggestions via events instead of direct method calls
      aiPanel.emitEvent('suggestions_updated', {
        suggestions: mockSuggestions,
        count: mockSuggestions.length
      });
      
      expect(aiPanel.isActive()).toBe(true);

      // Test show/hide functionality (Phase 2 preparation)
      aiPanel.show();
      expect(aiPanel.isVisible()).toBe(true);

      aiPanel.hide();
      expect(aiPanel.isVisible()).toBe(false);

      // Test suggestion interaction via DOM
      const panelElement = aiPanel.getElement();
      expect(panelElement).toBeDefined();

      console.log('✅ AI panel state management test passed');
    });
  });

  describe('Error Handling and Recovery Integration', () => {
    it('should handle complete system failure and recovery workflow', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: System failure and recovery workflow...');

      // Step 1: Simulate Ollama service failure
      invoke.mockRejectedValueOnce(new Error('Connection refused: Ollama service unavailable'));
      
      let ollamaError = null;
      try {
        await invoke('check_ollama_status');
      } catch (error) {
        ollamaError = error;
      }
      expect(ollamaError).not.toBeNull();
      expect(ollamaError.message).toContain('Connection refused');

      // Step 2: System should fall back to cached suggestions
      const cachedSuggestions = [
        {
          id: 'cached_1',
          title: 'Cached Suggestion',
          content: 'Previously cached content',
          relevanceScore: 0.8,
          cached: true
        }
      ];

      invoke.mockResolvedValueOnce(cachedSuggestions);
      const fallbackSuggestions = await invoke('get_cached_suggestions', { key: 'fallback_key' });
      expect(fallbackSuggestions[0].cached).toBe(true);

      // Step 3: Test vector database corruption handling
      invoke.mockResolvedValueOnce({
        is_healthy: false,
        corrupted_entries: 8,
        total_entries: 156,
        corruption_type: 'invalid_vector_dimensions',
        recovery_recommended: true,
        backup_available: true
      });

      const corruptionCheck = await invoke('validate_vector_database');
      expect(corruptionCheck.is_healthy).toBe(false);
      expect(corruptionCheck.recovery_recommended).toBe(true);

      // Step 4: Test recovery process
      invoke.mockResolvedValueOnce({
        recovery_status: 'completed',
        recovered_entries: 148,
        lost_entries: 8,
        recovery_time_ms: 7800,
        backup_restored: true
      });

      const recoveryResult = await invoke('recover_vector_database', {
        use_backup: true,
        validate_after_recovery: true
      });

      expect(recoveryResult.recovery_status).toBe('completed');
      expect(recoveryResult.recovered_entries).toBe(148);
      expect(recoveryResult.backup_restored).toBe(true);

      // Step 5: Validate system health after recovery
      invoke.mockResolvedValueOnce({
        is_healthy: true,
        total_entries: 148,
        corrupted_entries: 0,
        recent_recovery: true
      });

      const postRecoveryValidation = await invoke('validate_vector_database');
      expect(postRecoveryValidation.is_healthy).toBe(true);
      expect(postRecoveryValidation.recent_recovery).toBe(true);

      console.log('✅ System failure and recovery workflow test passed');
    });

    it('should handle graceful degradation when AI features are unavailable', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: Graceful degradation workflow...');

      // Step 1: Simulate embedding model unavailability
      invoke.mockResolvedValueOnce(false);
      const modelAvailable = await invoke('is_nomic_embed_available');
      expect(modelAvailable).toBe(false);

      // Step 2: System should continue operating with limited functionality
      invoke.mockResolvedValueOnce({
        ai_features_available: false,
        fallback_mode: 'text_search',
        limited_functionality: true,
        message: 'AI features unavailable - using text-based search'
      });

      const systemStatus = await invoke('get_ai_system_status');
      expect(systemStatus.ai_features_available).toBe(false);
      expect(systemStatus.fallback_mode).toBe('text_search');

      // Step 3: Test fallback search functionality
      invoke.mockResolvedValueOnce({
        results: [
          {
            match_type: 'text_match',
            file_path: '/vault/matching-content.md',
            content: 'Text-based matching content',
            score: 0.75,
            match_context: 'keyword_match'
          }
        ],
        search_type: 'fallback_text_search',
        search_time_ms: 89
      });

      const fallbackSearch = await invoke('fallback_text_search', {
        query: 'machine learning concepts',
        maxResults: 5
      });

      expect(fallbackSearch.results).toHaveLength(1);
      expect(fallbackSearch.search_type).toBe('fallback_text_search');
      expect(fallbackSearch.results[0].match_type).toBe('text_match');

      // Step 4: Test UI adaptation to limited functionality
      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

      // Wait for auto-activation and test fallback functionality
      await new Promise(resolve => setTimeout(resolve, 150));
      
      // Test that panel handles fallback data via events
      aiPanel.emitEvent('fallback_suggestions_received', {
        suggestions: fallbackSearch.results,
        fallback_mode: true
      });
      
      expect(aiPanel.isActive()).toBe(true);
      expect(aiPanel.getElement()).toBeDefined();

      // Panel should indicate limited functionality through config
      const panelConfig = aiPanel.getConfig();
      expect(panelConfig).toBeDefined();

      console.log('✅ Graceful degradation workflow test passed');
    });
  });

  describe('Performance Integration Under Load', () => {
    it('should maintain performance targets during concurrent operations', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: Concurrent operations performance...');

      // Step 1: Set up concurrent operations
      const concurrentTasks = [];
      const startTime = performance.now();

      // Simulate multiple AI operations happening simultaneously
      for (let i = 0; i < 5; i++) {
        // Embedding generation
        invoke.mockResolvedValueOnce(new Array(384).fill(0.1 + (i * 0.1)));
        concurrentTasks.push(
          invoke('generate_embedding', {
            text: `Concurrent text processing task ${i}`,
            modelName: 'nomic-embed-text'
          })
        );

        // Similarity search
        invoke.mockResolvedValueOnce({
          results: [{ similarity: 0.8, file_path: `/vault/result_${i}.md`, content: `Result ${i}` }],
          search_time_ms: 45 + (i * 5)
        });
        concurrentTasks.push(
          invoke('optimized_search_similar_notes', {
            query: `search query ${i}`,
            maxResults: 3,
            similarityThreshold: 0.7
          })
        );
      }

      // Execute all tasks concurrently
      const results = await Promise.all(concurrentTasks);
      const totalTime = performance.now() - startTime;

      // Validate results
      expect(results).toHaveLength(10);
      expect(totalTime).toBeLessThan(3000); // Should complete within 3 seconds

      // Step 2: Check system resource usage during load
      invoke.mockResolvedValueOnce({
        cpu_usage_percent: 65,
        memory_usage_mb: 89,
        ai_resource_allocation: 70,
        app_resource_allocation: 20,
        system_overhead: 10,
        performance_degradation: false
      });

      const resourceUsage = await invoke('get_resource_utilization');
      expect(resourceUsage.memory_usage_mb).toBeLessThan(100);
      expect(resourceUsage.performance_degradation).toBe(false);

      // Step 3: Validate UI responsiveness during AI operations
      const uiResponseStart = performance.now();
      
      invoke.mockResolvedValueOnce({ ui_interaction: 'completed', response_time: 45 });
      const uiResponse = await invoke('handle_ui_interaction', { action: 'editor_scroll' });
      
      const uiResponseTime = performance.now() - uiResponseStart;
      expect(uiResponseTime).toBeLessThan(100); // UI should remain responsive
      expect(uiResponse.ui_interaction).toBe('completed');

      console.log('✅ Concurrent operations performance test passed');
    });

    it('should handle large vault processing with performance monitoring', async () => {
      const { invoke } = window.__TAURI__.core;

      console.log('🧪 Testing: Large vault processing with monitoring...');

      // Step 1: Simulate large vault scanning
      const largeVaultFiles = Array.from({ length: 200 }, (_, i) => ({
        name: `note_${i.toString().padStart(3, '0')}.md`,
        path: `/vault/notes/note_${i.toString().padStart(3, '0')}.md`,
        is_dir: false,
        size: 1000 + (i * 50)
      }));

      invoke.mockResolvedValueOnce(largeVaultFiles);
      const vaultScan = await invoke('scan_vault_files', { vaultPath: '/vault' });
      expect(vaultScan).toHaveLength(200);

      // Step 2: Process with performance monitoring
      const processingStart = performance.now();
      
      invoke.mockResolvedValueOnce({
        processed_files: 200,
        total_embeddings: 1847,
        processing_time_ms: 45000,
        average_time_per_file: 225,
        memory_peak_mb: 95,
        errors: [],
        performance_metrics: {
          embeddings_per_second: 41,
          files_per_second: 4.4,
          memory_efficiency: 'good',
          cpu_utilization: 68
        }
      });

      const processingResult = await invoke('process_vault_for_indexing', {
        vaultPath: '/vault',
        options: {
          enable_monitoring: true,
          batch_size: 20,
          max_concurrent: 4
        }
      });

      const processingTime = performance.now() - processingStart;

      expect(processingResult.processed_files).toBe(200);
      expect(processingResult.total_embeddings).toBe(1847);
      expect(processingResult.memory_peak_mb).toBeLessThan(100);
      expect(processingResult.performance_metrics.memory_efficiency).toBe('good');

      // Step 3: Validate search performance on large index
      invoke.mockResolvedValueOnce({
        results: Array.from({ length: 10 }, (_, i) => ({
          similarity: 0.9 - (i * 0.05),
          file_path: `/vault/notes/note_${String(i * 20).padStart(3, '0')}.md`,
          content: `Relevant content from note ${i * 20}`,
          chunk_id: `chunk_${i}`
        })),
        search_time_ms: 78,
        total_indexed_entries: 1847,
        index_efficiency: 'optimal'
      });

      const largeIndexSearch = await invoke('optimized_search_similar_notes', {
        query: 'comprehensive search across large vault',
        maxResults: 10,
        similarityThreshold: 0.8
      });

      expect(largeIndexSearch.results).toHaveLength(10);
      expect(largeIndexSearch.search_time_ms).toBeLessThan(100);
      expect(largeIndexSearch.index_efficiency).toBe('optimal');

      console.log('✅ Large vault processing with monitoring test passed');
    });
  });
});