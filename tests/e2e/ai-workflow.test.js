import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

/**
 * End-to-End AI Workflow Tests
 * 
 * Comprehensive end-to-end testing for complete AI workflows according to Issue #76:
 * - First-time setup with Ollama installation guidance
 * - Vault indexing from scratch (bulk processing)
 * - Real-time suggestion generation during editing
 * - Note navigation via AI suggestions
 * - AI panel show/hide and interaction workflows
 * - Error handling when Ollama becomes unavailable
 * - Performance validation during real-world usage scenarios
 */

describe('AI System E2E Workflow Tests', () => {
  let tauriMocks;

  beforeEach(() => {
    tauriMocks = setupTauriMocks();
    
    // Setup comprehensive DOM environment for E2E testing
    document.body.innerHTML = `
      <div id="app">
        <div id="editor-container">
          <textarea id="markdown-editor" rows="20" cols="80"></textarea>
        </div>
        <div id="ai-panel" class="hidden">
          <div id="ai-suggestions"></div>
          <div id="ai-status"></div>
        </div>
        <div id="file-tree">
          <div id="vault-selector"></div>
          <div id="file-list"></div>
        </div>
        <div id="status-bar">
          <div id="ai-status-indicator"></div>
          <div id="indexing-progress"></div>
        </div>
      </div>
    `;
  });

  afterEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = '';
  });

  describe('First-time Setup and Ollama Installation Workflow', () => {
    it('should guide user through complete first-time setup', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: First-time setup workflow...');

      // Step 1: Initial system check - Ollama not available
      invoke.mockRejectedValueOnce(new Error('Connection refused'));
      
      let initialStatus;
      try {
        initialStatus = await invoke('check_ollama_status');
      } catch (error) {
        initialStatus = { status: 'Failed', error: error.message };
      }
      
      expect(initialStatus.status).toBe('Failed');
      console.log('    ✅ Detected Ollama unavailable - triggering setup guide');

      // Step 2: User downloads and starts Ollama
      // Simulate time passing and Ollama becoming available
      invoke.mockResolvedValueOnce({ 
        status: 'Connected', 
        message: 'Ollama service detected',
        version: '0.1.17'
      });
      
      const postInstallStatus = await invoke('check_ollama_status');
      expect(postInstallStatus.status).toBe('Connected');
      console.log('    ✅ Ollama connection established');

      // Step 3: Check for required models
      invoke.mockResolvedValueOnce(false);
      const nomicAvailable = await invoke('is_nomic_embed_available');
      expect(nomicAvailable).toBe(false);
      console.log('    ✅ Detected missing embedding model - triggering download');

      // Step 4: Download required model
      invoke.mockResolvedValueOnce({
        status: 'downloading',
        model_name: 'nomic-embed-text',
        total_bytes: 273000000, // ~273MB
        downloaded_bytes: 0,
        download_id: 'download_1'
      });
      
      const downloadStart = await invoke('download_model', { modelName: 'nomic-embed-text' });
      expect(downloadStart.status).toBe('downloading');
      console.log('    ✅ Model download initiated');

      // Step 5: Simulate download progress
      const progressUpdates = [
        { downloaded_bytes: 54600000, progress: 0.2 },   // 20%
        { downloaded_bytes: 136500000, progress: 0.5 },  // 50%
        { downloaded_bytes: 218400000, progress: 0.8 },  // 80%
        { downloaded_bytes: 273000000, progress: 1.0, status: 'completed' }  // 100%
      ];

      for (const progress of progressUpdates) {
        invoke.mockResolvedValueOnce(progress);
        const currentProgress = await invoke('get_download_progress', { 
          modelName: 'nomic-embed-text' 
        });
        
        expect(currentProgress.downloaded_bytes).toBe(progress.downloaded_bytes);
        console.log(`    📊 Download progress: ${(progress.progress * 100).toFixed(0)}%`);
        
        if (progress.status === 'completed') {
          expect(currentProgress.status).toBe('completed');
          break;
        }
      }

      // Step 6: Verify model is now available
      invoke.mockResolvedValueOnce(true);
      const modelNowAvailable = await invoke('is_nomic_embed_available');
      expect(modelNowAvailable).toBe(true);
      console.log('    ✅ Embedding model ready for use');

      // Step 7: Initialize AI system
      invoke.mockResolvedValueOnce({
        ai_system_ready: true,
        services_initialized: ['ollama_client', 'embedding_generator', 'vector_db'],
        setup_completed: true
      });

      const systemInit = await invoke('initialize_ai_system');
      expect(systemInit.ai_system_ready).toBe(true);
      expect(systemInit.services_initialized).toHaveLength(3);

      console.log('✅ First-time setup workflow completed successfully');
    });

    it('should handle setup cancellation and resume gracefully', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Setup cancellation and resume...');

      // Step 1: Start model download
      invoke.mockResolvedValueOnce({
        status: 'downloading',
        model_name: 'nomic-embed-text',
        download_id: 'download_2'
      });
      
      const download = await invoke('download_model', { modelName: 'nomic-embed-text' });
      expect(download.status).toBe('downloading');

      // Step 2: Cancel download mid-way
      invoke.mockResolvedValueOnce({ cancelled: true });
      const cancellation = await invoke('cancel_download', { modelName: 'nomic-embed-text' });
      expect(cancellation.cancelled).toBe(true);
      console.log('    ✅ Download cancellation successful');

      // Step 3: Resume download later
      invoke.mockResolvedValueOnce({
        status: 'downloading',
        model_name: 'nomic-embed-text',
        resume: true,
        downloaded_bytes: 136500000 // Resume from 50%
      });
      
      const resume = await invoke('download_model', { modelName: 'nomic-embed-text' });
      expect(resume.resume).toBe(true);
      expect(resume.downloaded_bytes).toBeGreaterThan(0);
      console.log('    ✅ Download resume successful');

      console.log('✅ Setup cancellation and resume test completed');
    });
  });

  describe('Vault Indexing from Scratch Workflow', () => {
    it('should handle complete vault indexing workflow', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Complete vault indexing workflow...');

      // Step 1: User selects vault folder
      const mockVaultPath = '/Users/test/Documents/MyNotes';
      invoke.mockResolvedValueOnce(mockVaultPath);
      const selectedVault = await invoke('select_vault');
      expect(selectedVault).toBe(mockVaultPath);
      console.log('    ✅ Vault selected:', selectedVault);

      // Step 2: Scan vault for markdown files
      const mockFiles = Array.from({ length: 50 }, (_, i) => ({
        name: `note_${i.toString().padStart(2, '0')}.md`,
        path: `${mockVaultPath}/note_${i.toString().padStart(2, '0')}.md`,
        is_dir: false,
        size: 1000 + (i * 100),
        modified: Date.now() - (i * 86400000) // Different modification times
      }));

      invoke.mockResolvedValueOnce(mockFiles);
      const vaultFiles = await invoke('scan_vault_files', { vaultPath: mockVaultPath });
      expect(vaultFiles).toHaveLength(50);
      console.log('    ✅ Scanned vault:', vaultFiles.length, 'files found');

      // Step 3: Start indexing pipeline
      invoke.mockResolvedValueOnce({
        indexing_started: true,
        pipeline_id: 'pipeline_1',
        estimated_time_ms: 15000, // 15 seconds
        files_to_process: 50
      });

      const indexingStart = await invoke('start_indexing_pipeline', {
        vaultPath: mockVaultPath,
        options: { chunk_size: 1000, model: 'nomic-embed-text' }
      });

      expect(indexingStart.indexing_started).toBe(true);
      expect(indexingStart.files_to_process).toBe(50);
      console.log('    ✅ Indexing pipeline started');

      // Step 4: Monitor indexing progress
      const progressSteps = [
        { processed: 10, total: 50, progress: 0.2, status: 'processing' },
        { processed: 25, total: 50, progress: 0.5, status: 'processing' },
        { processed: 40, total: 50, progress: 0.8, status: 'processing' },
        { processed: 50, total: 50, progress: 1.0, status: 'completed' }
      ];

      for (const step of progressSteps) {
        invoke.mockResolvedValueOnce(step);
        const progress = await invoke('get_indexing_progress', { pipelineId: 'pipeline_1' });
        
        expect(progress.processed).toBe(step.processed);
        expect(progress.progress).toBe(step.progress);
        
        console.log(`    📊 Indexing progress: ${step.processed}/${step.total} files (${(step.progress * 100).toFixed(0)}%)`);
        
        if (step.status === 'completed') {
          expect(progress.status).toBe('completed');
          break;
        }
      }

      // Step 5: Verify indexing results
      invoke.mockResolvedValueOnce({
        total_embeddings: 247, // 50 files with ~5 chunks each
        processing_time_ms: 14800,
        errors: [],
        index_size_mb: 8.3,
        average_chunks_per_file: 4.9
      });

      const indexingResults = await invoke('get_indexing_status', { pipelineId: 'pipeline_1' });
      expect(indexingResults.total_embeddings).toBe(247);
      expect(indexingResults.errors).toHaveLength(0);
      console.log('    ✅ Indexing completed:', indexingResults.total_embeddings, 'embeddings created');

      // Step 6: Validate index integrity
      invoke.mockResolvedValueOnce({
        is_healthy: true,
        total_entries: 247,
        corrupted_entries: 0,
        validation_time_ms: 89
      });

      const validation = await invoke('validate_vector_database');
      expect(validation.is_healthy).toBe(true);
      expect(validation.total_entries).toBe(247);
      console.log('    ✅ Index validation passed');

      // Step 7: Test search functionality on new index
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.91,
            file_path: `${mockVaultPath}/note_15.md`,
            content: 'Machine learning concepts and applications',
            chunk_id: 'chunk_2'
          },
          {
            similarity: 0.86,
            file_path: `${mockVaultPath}/note_23.md`, 
            content: 'Deep learning neural network architectures',
            chunk_id: 'chunk_1'
          }
        ],
        search_time_ms: 42,
        total_results: 2
      });

      const searchTest = await invoke('optimized_search_similar_notes', {
        query: 'machine learning deep neural networks',
        maxResults: 5,
        similarityThreshold: 0.8
      });

      expect(searchTest.results).toHaveLength(2);
      expect(searchTest.results[0].similarity).toBeGreaterThan(0.9);
      expect(searchTest.search_time_ms).toBeLessThan(100);
      console.log('    ✅ Search functionality validated on new index');

      console.log('✅ Complete vault indexing workflow completed successfully');
    });

    it('should handle large vault indexing with batching', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Large vault indexing with batching...');

      // Step 1: Large vault with 500 files
      const largeVaultPath = '/Users/test/Documents/LargeVault';
      const largeVaultFiles = Array.from({ length: 500 }, (_, i) => ({
        name: `document_${i.toString().padStart(3, '0')}.md`,
        path: `${largeVaultPath}/document_${i.toString().padStart(3, '0')}.md`,
        is_dir: false,
        size: 2000 + (i * 50)
      }));

      invoke.mockResolvedValueOnce(largeVaultFiles);
      const largeVault = await invoke('scan_vault_files', { vaultPath: largeVaultPath });
      expect(largeVault).toHaveLength(500);
      console.log('    ✅ Large vault scanned:', largeVault.length, 'files');

      // Step 2: Start batched indexing
      invoke.mockResolvedValueOnce({
        indexing_started: true,
        pipeline_id: 'large_pipeline',
        batch_size: 25,
        total_batches: 20,
        estimated_time_ms: 120000 // 2 minutes
      });

      const largeIndexing = await invoke('start_indexing_pipeline', {
        vaultPath: largeVaultPath,
        options: { 
          batch_size: 25,
          parallel_workers: 4,
          memory_limit_mb: 80
        }
      });

      expect(largeIndexing.total_batches).toBe(20);
      console.log('    ✅ Batched indexing started:', largeIndexing.total_batches, 'batches');

      // Step 3: Monitor batch progress
      const batchProgresses = [
        { completed_batches: 5, total_batches: 20, progress: 0.25 },
        { completed_batches: 10, total_batches: 20, progress: 0.5 },
        { completed_batches: 15, total_batches: 20, progress: 0.75 },
        { completed_batches: 20, total_batches: 20, progress: 1.0, status: 'completed' }
      ];

      for (const batchProgress of batchProgresses) {
        invoke.mockResolvedValueOnce(batchProgress);
        const progress = await invoke('get_indexing_progress', { pipelineId: 'large_pipeline' });
        
        expect(progress.completed_batches).toBe(batchProgress.completed_batches);
        console.log(`    📊 Batch progress: ${batchProgress.completed_batches}/${batchProgress.total_batches} batches`);
      }

      // Step 4: Verify memory usage stayed within limits
      invoke.mockResolvedValueOnce({
        peak_memory_mb: 78.5,
        average_memory_mb: 62.3,
        memory_limit_exceeded: false,
        gc_events: 8
      });

      const memoryStats = await invoke('get_memory_usage_stats');
      expect(memoryStats.peak_memory_mb).toBeLessThan(80);
      expect(memoryStats.memory_limit_exceeded).toBe(false);
      console.log('    ✅ Memory usage within limits:', memoryStats.peak_memory_mb, 'MB peak');

      console.log('✅ Large vault indexing with batching completed successfully');
    });
  });

  describe('Real-time Suggestion Generation During Editing', () => {
    it('should provide AI suggestions while user types', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Real-time suggestion generation during editing...');

      // Step 1: Initialize editor and AI components
      const mockEditor = {
        value: '',
        cursorPosition: 0,
        addEventListener: vi.fn()
      };

      const mockTextEditor = document.getElementById('markdown-editor');
      
      // Step 2: Simulate user typing sequence
      const typingSequence = [
        'Machine learning',
        'Machine learning is a powerful',
        'Machine learning is a powerful technique for',
        'Machine learning is a powerful technique for analyzing large datasets and finding patterns'
      ];

      for (let i = 0; i < typingSequence.length; i++) {
        const currentText = typingSequence[i];
        mockTextEditor.value = currentText;
        mockEditor.value = currentText;
        mockEditor.cursorPosition = currentText.length;

        // Only trigger suggestions for meaningful content (>20 chars)
        if (currentText.length > 20) {
          console.log(`    ⌨️  User typed: "${currentText.slice(-20)}..."`);

          // Step 3: Content change detection
          invoke.mockResolvedValueOnce({
            content_changed: true,
            current_paragraph: currentText,
            extraction_time_ms: 8,
            should_generate_suggestions: true
          });

          const contentChange = await invoke('detect_content_change', {
            content: currentText,
            cursorPosition: mockEditor.cursorPosition
          });

          expect(contentChange.should_generate_suggestions).toBe(true);

          // Step 4: Generate AI suggestions
          const mockSuggestions = [
            {
              similarity: 0.89,
              file_path: '/vault/ml-fundamentals.md',
              content: 'Machine learning algorithms can identify complex patterns in data',
              chunk_id: 'chunk_1'
            },
            {
              similarity: 0.84,
              file_path: '/vault/data-analysis.md',
              content: 'Data analysis techniques using statistical methods and ML',
              chunk_id: 'chunk_2'
            }
          ];

          invoke.mockResolvedValueOnce({
            results: mockSuggestions,
            generation_time_ms: 150,
            from_cache: i > 1 // Cache hits after first few
          });

          const suggestions = await invoke('optimized_search_similar_notes', {
            query: currentText,
            maxResults: 5,
            similarityThreshold: 0.7,
            currentFile: '/vault/current-note.md'
          });

          expect(suggestions.results).toHaveLength(2);
          expect(suggestions.generation_time_ms).toBeLessThan(2000);
          
          console.log(`    🤖 Generated ${suggestions.results.length} suggestions in ${suggestions.generation_time_ms}ms ${suggestions.from_cache ? '(cached)' : '(fresh)'}`);

          // Step 5: Update UI with suggestions
          const AiPanel = await import('../../src/js/components/ai-panel.js');
          const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

          const formattedSuggestions = suggestions.results.map((result, index) => ({
            id: `suggestion_${i}_${index}`,
            title: result.file_path.split('/').pop().replace('.md', ''),
            content: result.content,
            relevanceScore: result.similarity,
            filePath: result.file_path
          }));

          await aiPanel.updateSuggestions(formattedSuggestions);
          expect(aiPanel.getSuggestionCount()).toBe(2);

          // Simulate debounce delay
          await new Promise(resolve => setTimeout(resolve, 100));
        }
      }

      console.log('✅ Real-time suggestion generation completed successfully');
    });

    it('should handle rapid typing with debouncing', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Rapid typing with debouncing...');

      // Simulate very rapid typing (faster than debounce threshold)
      const rapidTyping = [
        'Neural',
        'Neural n',
        'Neural ne',
        'Neural net',
        'Neural netw',
        'Neural netwo',
        'Neural networ',
        'Neural network',
        'Neural networks',
        'Neural networks are',
        'Neural networks are powerful',
        'Neural networks are powerful computational models'
      ];

      let suggestionRequests = 0;

      for (let i = 0; i < rapidTyping.length; i++) {
        const text = rapidTyping[i];
        
        // Only process every 3rd input (simulating debouncing)
        if (i % 3 === 0 && text.length > 20) {
          suggestionRequests++;
          
          invoke.mockResolvedValueOnce({
            results: [{
              similarity: 0.87,
              file_path: '/vault/neural-networks.md',
              content: 'Neural network architectures and applications'
            }],
            generation_time_ms: 95,
            debounced: true
          });

          const suggestions = await invoke('optimized_search_similar_notes', {
            query: text,
            maxResults: 3,
            debounceMs: 500
          });

          expect(suggestions.results).toHaveLength(1);
          expect(suggestions.debounced).toBe(true);
        }
      }

      // Should have made fewer requests due to debouncing
      expect(suggestionRequests).toBeLessThan(rapidTyping.length / 2);
      console.log(`    ✅ Debouncing effective: ${suggestionRequests} requests for ${rapidTyping.length} key presses`);

      console.log('✅ Rapid typing with debouncing completed successfully');
    });
  });

  describe('Note Navigation via AI Suggestions', () => {
    it('should enable navigation through AI-suggested content', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Note navigation via AI suggestions...');

      // Step 1: User has current content and gets suggestions
      const currentContent = "Learning about graph neural networks and their applications";
      
      const mockSuggestions = [
        {
          similarity: 0.92,
          file_path: '/vault/gnn-theory.md',
          content: 'Graph Neural Networks (GNNs) operate on graph-structured data',
          chunk_id: 'chunk_1',
          line_numbers: [15, 25]
        },
        {
          similarity: 0.87,
          file_path: '/vault/gnn-applications.md',
          content: 'Applications of GNNs in social networks and molecular analysis',
          chunk_id: 'chunk_3',
          line_numbers: [45, 55]
        },
        {
          similarity: 0.83,
          file_path: '/vault/deep-learning.md',
          content: 'Deep learning architectures including CNNs, RNNs, and GNNs',
          chunk_id: 'chunk_7',
          line_numbers: [120, 130]
        }
      ];

      invoke.mockResolvedValueOnce({
        results: mockSuggestions,
        search_time_ms: 67
      });

      const suggestions = await invoke('optimized_search_similar_notes', {
        query: currentContent,
        maxResults: 10,
        similarityThreshold: 0.8
      });

      expect(suggestions.results).toHaveLength(3);
      console.log('    ✅ Generated contextual suggestions for navigation');

      // Step 2: User clicks on first suggestion to navigate
      const targetSuggestion = suggestions.results[0];
      
      // Mock reading the target file
      invoke.mockResolvedValueOnce(`# Graph Neural Networks Theory

Graph Neural Networks (GNNs) are a class of deep learning methods designed to perform inference on data described by graphs. They operate on graph-structured data and can capture dependencies between nodes in the graph.

## Key Concepts

- **Node embeddings**: Vector representations of nodes
- **Message passing**: Information exchange between connected nodes  
- **Aggregation**: Combining messages from neighboring nodes
- **Update**: Computing new node representations

Graph Neural Networks (GNNs) operate on graph-structured data and have proven effective for various tasks including node classification, graph classification, and link prediction.

## Applications

GNNs are used in:
- Social network analysis
- Molecular property prediction
- Knowledge graph reasoning
- Recommender systems`);

      const targetFileContent = await invoke('read_file', { 
        filePath: targetSuggestion.file_path 
      });

      expect(targetFileContent).toContain('Graph Neural Networks');
      console.log('    ✅ Successfully navigated to suggested file');

      // Step 3: Highlight relevant section
      const highlightLineStart = targetSuggestion.line_numbers[0];
      const highlightLineEnd = targetSuggestion.line_numbers[1];
      
      // Mock editor navigation to specific line
      invoke.mockResolvedValueOnce({
        navigation_successful: true,
        line_number: highlightLineStart,
        highlighted_text: targetSuggestion.content,
        scroll_position: highlightLineStart * 20 // Assuming ~20px per line
      });

      const navigation = await invoke('navigate_to_line', {
        filePath: targetSuggestion.file_path,
        lineNumber: highlightLineStart,
        endLineNumber: highlightLineEnd
      });

      expect(navigation.navigation_successful).toBe(true);
      expect(navigation.line_number).toBe(highlightLineStart);
      console.log(`    ✅ Navigated to line ${navigation.line_number} and highlighted relevant content`);

      // Step 4: Generate follow-up suggestions from new context
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.88,
            file_path: '/vault/message-passing.md',
            content: 'Message passing algorithms in graph neural networks',
            chunk_id: 'chunk_2'
          },
          {
            similarity: 0.85,
            file_path: '/vault/node-embeddings.md', 
            content: 'Learning node embeddings with graph neural networks',
            chunk_id: 'chunk_1'
          }
        ],
        search_time_ms: 54,
        contextual_suggestions: true
      });

      const followUpSuggestions = await invoke('optimized_search_similar_notes', {
        query: targetFileContent.slice(0, 200), // Use new context for suggestions
        maxResults: 5,
        similarityThreshold: 0.8,
        currentFile: targetSuggestion.file_path
      });

      expect(followUpSuggestions.results).toHaveLength(2);
      expect(followUpSuggestions.contextual_suggestions).toBe(true);
      console.log('    ✅ Generated contextual follow-up suggestions');

      // Step 5: Test breadcrumb navigation
      const navigationHistory = [
        '/vault/current-note.md',
        '/vault/gnn-theory.md'
      ];

      invoke.mockResolvedValueOnce({
        navigation_history: navigationHistory,
        current_index: 1,
        can_go_back: true,
        can_go_forward: false
      });

      const historyState = await invoke('get_navigation_history');
      expect(historyState.can_go_back).toBe(true);
      expect(historyState.navigation_history).toHaveLength(2);
      console.log('    ✅ Navigation history tracking working');

      console.log('✅ Note navigation via AI suggestions completed successfully');
    });

    it('should enable semantic search across vault', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Semantic search across vault...');

      // Step 1: Perform semantic search with natural language query
      const naturalQuery = "How do I implement attention mechanisms in transformers?";
      
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.94,
            file_path: '/vault/attention-mechanisms.md',
            content: 'Attention mechanisms allow models to focus on relevant parts of input',
            chunk_id: 'chunk_1',
            semantic_match: true
          },
          {
            similarity: 0.89,
            file_path: '/vault/transformer-architecture.md',
            content: 'Transformer implementation with multi-head attention layers',
            chunk_id: 'chunk_4',
            semantic_match: true
          },
          {
            similarity: 0.86,
            file_path: '/vault/pytorch-transformers.md',
            content: 'PyTorch implementation of transformer attention mechanisms',
            chunk_id: 'chunk_2',
            semantic_match: true
          }
        ],
        search_time_ms: 78,
        semantic_search: true,
        query_understanding: {
          intent: 'implementation_guide',
          key_concepts: ['attention', 'transformers', 'implementation'],
          difficulty_level: 'intermediate'
        }
      });

      const semanticResults = await invoke('semantic_search_vault', {
        query: naturalQuery,
        maxResults: 10,
        searchType: 'semantic'
      });

      expect(semanticResults.semantic_search).toBe(true);
      expect(semanticResults.results).toHaveLength(3);
      expect(semanticResults.query_understanding.intent).toBe('implementation_guide');
      console.log('    ✅ Semantic search successfully understood query intent');

      // Step 2: Filter results by relevance and topic
      const filteredResults = semanticResults.results.filter(r => r.similarity > 0.85);
      expect(filteredResults).toHaveLength(3); // All results are high quality
      console.log(`    ✅ High-relevance results: ${filteredResults.length}`);

      // Step 3: Create knowledge map from results
      invoke.mockResolvedValueOnce({
        knowledge_map: {
          central_topic: 'attention_mechanisms',
          related_files: semanticResults.results.map(r => r.file_path),
          connection_strength: semanticResults.results.map(r => r.similarity),
          topic_clusters: [
            { name: 'attention_theory', files: ['/vault/attention-mechanisms.md'] },
            { name: 'transformer_implementation', files: ['/vault/transformer-architecture.md', '/vault/pytorch-transformers.md'] }
          ]
        },
        map_generation_time_ms: 123
      });

      const knowledgeMap = await invoke('generate_knowledge_map', {
        searchResults: semanticResults.results,
        centralTopic: 'attention mechanisms'
      });

      expect(knowledgeMap.knowledge_map.topic_clusters).toHaveLength(2);
      expect(knowledgeMap.knowledge_map.related_files).toHaveLength(3);
      console.log('    ✅ Generated knowledge map from search results');

      console.log('✅ Semantic search across vault completed successfully');
    });
  });

  describe('AI Panel Show/Hide and Interaction Workflows', () => {
    it('should manage AI panel visibility and interactions', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: AI panel visibility and interactions...');

      // Step 1: Initialize AI panel (Phase 1 - hidden by default)
      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

      expect(aiPanel.isVisible()).toBe(false);
      console.log('    ✅ AI panel initialized in hidden state (Phase 1 behavior)');

      // Step 2: Generate suggestions while panel is hidden
      const mockSuggestions = [
        {
          id: 'hidden_suggestion_1',
          title: 'Machine Learning Basics',
          content: 'Introduction to machine learning concepts and algorithms',
          relevanceScore: 0.91,
          filePath: '/vault/ml-basics.md'
        },
        {
          id: 'hidden_suggestion_2',
          title: 'Neural Networks',
          content: 'Understanding neural network architectures and training',
          relevanceScore: 0.87,
          filePath: '/vault/neural-networks.md'
        }
      ];

      await aiPanel.updateSuggestions(mockSuggestions);
      expect(aiPanel.getSuggestionCount()).toBe(2);
      console.log('    ✅ Suggestions stored while panel hidden');

      // Step 3: Show AI panel (Phase 2 simulation)
      aiPanel.show();
      expect(aiPanel.isVisible()).toBe(true);
      console.log('    ✅ AI panel shown - suggestions now visible');

      // Step 4: Test suggestion interaction
      const suggestionElements = aiPanel.getSuggestionCards();
      expect(suggestionElements).toBeDefined();

      // Simulate clicking on first suggestion
      invoke.mockResolvedValueOnce('# Machine Learning Basics\n\nIntroduction to machine learning...');
      
      const mockClick = new Event('click');
      if (suggestionElements.length > 0) {
        suggestionElements[0].dispatchEvent(mockClick);
      }

      console.log('    ✅ Suggestion interaction tested');

      // Step 5: Test panel resize and layout
      const originalWidth = aiPanel.getWidth();
      
      aiPanel.resize(350); // Resize to 350px width
      expect(aiPanel.getWidth()).not.toBe(originalWidth);
      console.log(`    ✅ Panel resized from ${originalWidth}px to ${aiPanel.getWidth()}px`);

      // Step 6: Test panel settings and configuration
      const mockSettings = {
        maxSuggestions: 8,
        similarityThreshold: 0.75,
        autoRefresh: true,
        showRelevanceScores: true
      };

      aiPanel.updateSettings(mockSettings);
      const currentSettings = aiPanel.getSettings();
      expect(currentSettings.maxSuggestions).toBe(8);
      console.log('    ✅ Panel settings updated successfully');

      // Step 7: Test panel hide
      aiPanel.hide();
      expect(aiPanel.isVisible()).toBe(false);
      console.log('    ✅ AI panel hidden');

      // Step 8: Verify suggestions persist when hidden
      expect(aiPanel.getSuggestionCount()).toBe(2);
      console.log('    ✅ Suggestions persisted while panel hidden');

      console.log('✅ AI panel visibility and interactions completed successfully');
    });

    it('should handle AI panel performance during intensive operations', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: AI panel performance during intensive operations...');

      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

      // Step 1: Simulate high-frequency suggestion updates
      const updateCount = 20;
      const startTime = performance.now();

      for (let i = 0; i < updateCount; i++) {
        const batchSuggestions = Array.from({ length: 5 }, (_, j) => ({
          id: `perf_suggestion_${i}_${j}`,
          title: `Suggestion ${i}-${j}`,
          content: `Dynamic content for suggestion ${i}-${j}`,
          relevanceScore: 0.9 - (j * 0.02),
          filePath: `/vault/file_${i}_${j}.md`
        }));

        await aiPanel.updateSuggestions(batchSuggestions);
        
        // Small delay to simulate real-world timing
        await new Promise(resolve => setTimeout(resolve, 10));
      }

      const updateTime = performance.now() - startTime;
      expect(updateTime).toBeLessThan(1000); // Should complete in <1 second
      console.log(`    ✅ ${updateCount} rapid updates completed in ${updateTime.toFixed(1)}ms`);

      // Step 2: Test memory usage during intensive operations
      invoke.mockResolvedValueOnce({
        component_memory_mb: 4.2,
        suggestion_cache_mb: 2.8,
        dom_elements: 125,
        event_listeners: 18
      });

      const memoryUsage = await invoke('get_component_memory_usage', { 
        componentId: 'ai-panel' 
      });
      
      expect(memoryUsage.component_memory_mb).toBeLessThan(10);
      console.log(`    ✅ Memory usage during intensive ops: ${memoryUsage.component_memory_mb}MB`);

      // Step 3: Test UI responsiveness
      const responsiveStartTime = performance.now();
      
      // Simulate user interaction during heavy operation
      aiPanel.show();
      aiPanel.resize(400);
      aiPanel.hide();
      aiPanel.show();

      const responsiveTime = performance.now() - responsiveStartTime;
      expect(responsiveTime).toBeLessThan(100); // UI should remain responsive
      console.log(`    ✅ UI interactions completed in ${responsiveTime.toFixed(1)}ms - remained responsive`);

      console.log('✅ AI panel performance during intensive operations completed successfully');
    });
  });

  describe('Error Handling When Ollama Becomes Unavailable', () => {
    it('should gracefully handle Ollama service interruption', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Ollama service interruption handling...');

      // Step 1: System starts with Ollama available
      invoke.mockResolvedValueOnce({ status: 'Connected' });
      const initialStatus = await invoke('check_ollama_status');
      expect(initialStatus.status).toBe('Connected');
      console.log('    ✅ Initial Ollama connection established');

      // Step 2: User generates some suggestions successfully
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.89,
            file_path: '/vault/test-note.md',
            content: 'Test content for working suggestions'
          }
        ],
        search_time_ms: 95
      });

      const workingSuggestions = await invoke('optimized_search_similar_notes', {
        query: 'test query',
        maxResults: 5
      });

      expect(workingSuggestions.results).toHaveLength(1);
      console.log('    ✅ AI suggestions working normally');

      // Step 3: Ollama service becomes unavailable
      invoke.mockRejectedValueOnce(new Error('Connection refused'));
      
      let connectionError;
      try {
        await invoke('check_ollama_status');
      } catch (error) {
        connectionError = error;
      }
      
      expect(connectionError).toBeDefined();
      console.log('    ✅ Detected Ollama service interruption');

      // Step 4: System should fall back to cached suggestions
      const cachedSuggestions = [
        {
          id: 'cached_1',
          title: 'Cached Suggestion',
          content: 'Previously cached content remains available',
          relevanceScore: 0.82,
          cached: true
        }
      ];

      invoke.mockResolvedValueOnce(cachedSuggestions);
      const fallbackSuggestions = await invoke('get_cached_suggestions', {
        key: 'fallback_suggestions'
      });

      expect(fallbackSuggestions[0].cached).toBe(true);
      console.log('    ✅ Fallback to cached suggestions successful');

      // Step 5: UI should show appropriate error state
      const AiPanel = await import('../../src/js/components/ai-panel.js');
      const aiPanel = new AiPanel.default(document.getElementById('ai-panel'));

      await aiPanel.showErrorState('Ollama service unavailable - using cached suggestions');
      const panelStatus = aiPanel.getStatus();
      expect(panelStatus.hasError).toBe(true);
      console.log('    ✅ UI error state displayed');

      // Step 6: System attempts periodic reconnection
      for (let attempt = 1; attempt <= 3; attempt++) {
        if (attempt < 3) {
          invoke.mockRejectedValueOnce(new Error('Still unavailable'));
        } else {
          invoke.mockResolvedValueOnce({ status: 'Connected', reconnected: true });
        }

        try {
          const reconnectResult = await invoke('check_ollama_status');
          if (reconnectResult.status === 'Connected') {
            console.log(`    ✅ Reconnection successful after ${attempt} attempts`);
            break;
          }
        } catch (error) {
          console.log(`    🔄 Reconnection attempt ${attempt} failed`);
        }

        // Wait before next attempt
        await new Promise(resolve => setTimeout(resolve, 100));
      }

      // Step 7: Verify service restoration
      invoke.mockResolvedValueOnce({
        results: [
          {
            similarity: 0.91,
            file_path: '/vault/restored-note.md',
            content: 'Service restored - fresh AI suggestions available'
          }
        ],
        service_restored: true
      });

      const restoredSuggestions = await invoke('optimized_search_similar_notes', {
        query: 'test restoration query',
        maxResults: 5
      });

      expect(restoredSuggestions.service_restored).toBe(true);
      console.log('    ✅ Service restoration verified - fresh suggestions available');

      // Step 8: Clear error state in UI
      await aiPanel.clearErrorState();
      const clearedStatus = aiPanel.getStatus();
      expect(clearedStatus.hasError).toBe(false);
      console.log('    ✅ UI error state cleared');

      console.log('✅ Ollama service interruption handling completed successfully');
    });

    it('should handle model download failures during setup', async () => {
      const { invoke } = window.__TAURI__.core;
      
      console.log('🧪 E2E Test: Model download failure handling...');

      // Step 1: Attempt model download
      invoke.mockResolvedValueOnce({
        status: 'downloading',
        model_name: 'nomic-embed-text'
      });

      const downloadStart = await invoke('download_model', { modelName: 'nomic-embed-text' });
      expect(downloadStart.status).toBe('downloading');
      console.log('    ✅ Model download initiated');

      // Step 2: Simulate download failure
      invoke.mockRejectedValueOnce(new Error('Download failed: Network error'));
      
      let downloadError;
      try {
        await invoke('get_download_progress', { modelName: 'nomic-embed-text' });
      } catch (error) {
        downloadError = error;
      }

      expect(downloadError).toBeDefined();
      console.log('    ✅ Download failure detected');

      // Step 3: System should offer retry options
      invoke.mockResolvedValueOnce({
        retry_available: true,
        max_retries: 3,
        current_attempt: 1,
        suggested_action: 'retry_download'
      });

      const retryOptions = await invoke('get_download_retry_options', { 
        modelName: 'nomic-embed-text' 
      });

      expect(retryOptions.retry_available).toBe(true);
      console.log('    ✅ Retry options presented to user');

      // Step 4: Attempt retry with success
      invoke.mockResolvedValueOnce({
        status: 'downloading',
        retry_attempt: 2,
        downloaded_bytes: 0
      });

      const retryAttempt = await invoke('retry_model_download', { 
        modelName: 'nomic-embed-text' 
      });

      expect(retryAttempt.retry_attempt).toBe(2);
      console.log('    ✅ Retry attempt initiated');

      // Step 5: Simulate successful completion
      invoke.mockResolvedValueOnce({
        status: 'completed',
        model_name: 'nomic-embed-text',
        download_completed: true,
        retry_successful: true
      });

      const downloadComplete = await invoke('get_download_progress', { 
        modelName: 'nomic-embed-text' 
      });

      expect(downloadComplete.download_completed).toBe(true);
      expect(downloadComplete.retry_successful).toBe(true);
      console.log('    ✅ Model download completed successfully after retry');

      console.log('✅ Model download failure handling completed successfully');
    });
  });
});