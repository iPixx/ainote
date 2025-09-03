/**
 * AI Suggestion Service - Core service for real-time AI-powered note suggestions
 * 
 * Orchestrates the entire suggestion pipeline: content detection, similarity search,
 * caching, and result processing. Provides a high-level API for the UI components
 * to request and manage AI suggestions.
 * 
 * Performance targets:
 * - Suggestion generation within 2 seconds
 * - Cache hit rate >70% for repeat queries
 * - No blocking of editor interactions
 * - Memory usage <50MB for complete system
 * 
 * @class AiSuggestionService
 */
class AiSuggestionService {
  /**
   * Suggestion service events
   */
  static EVENTS = {
    SUGGESTIONS_UPDATED: 'suggestions_updated',
    SUGGESTIONS_LOADING: 'suggestions_loading',
    SUGGESTIONS_ERROR: 'suggestions_error',
    CACHE_PERFORMANCE: 'cache_performance_updated',
    SERVICE_STATUS_CHANGED: 'service_status_changed'
  };

  /**
   * Service status states
   */
  static STATUS = {
    INITIALIZING: 'initializing',
    READY: 'ready',
    LOADING: 'loading',
    ERROR: 'error',
    DISABLED: 'disabled'
  };

  /**
   * Configuration defaults
   */
  static DEFAULTS = {
    ENABLED: true,
    MAX_SUGGESTIONS: 10,
    SIMILARITY_THRESHOLD: 0.3,
    GENERATION_TIMEOUT: 10000, // 10 seconds
    DEBOUNCE_DELAY: 500, // 500ms content change debouncing
    CACHE_ENABLED: true,
    AUTO_GENERATE: true,
    MODEL_NAME: 'nomic-embed-text' // Default embedding model
  };

  /**
   * Initialize AI suggestion service
   * @param {MarkdownEditor} editor - Markdown editor instance
   * @param {AppState} appState - Application state manager
   * @param {ContentChangeDetector} contentDetector - Content change detector
   * @param {SuggestionCacheManager} cacheManager - Suggestion cache manager
   */
  constructor(editor, appState, contentDetector, cacheManager) {
    if (!editor) {
      throw new Error('MarkdownEditor instance required');
    }
    if (!appState) {
      throw new Error('AppState instance required');
    }
    if (!contentDetector) {
      throw new Error('ContentChangeDetector instance required');
    }
    if (!cacheManager) {
      throw new Error('SuggestionCacheManager instance required');
    }

    this.editor = editor;
    this.appState = appState;
    this.contentDetector = contentDetector;
    this.cacheManager = cacheManager;
    
    // Service state
    this.status = AiSuggestionService.STATUS.INITIALIZING;
    this.currentSuggestions = [];
    this.lastGenerationTime = 0;
    this.generationInProgress = false;
    this.currentGenerationAbortController = null;
    
    // Configuration
    this.config = { ...AiSuggestionService.DEFAULTS };
    
    // Performance tracking
    this.performanceStats = {
      totalRequests: 0,
      successfulRequests: 0,
      failedRequests: 0,
      cacheHits: 0,
      cacheMisses: 0,
      averageGenerationTime: 0,
      lastRequestTime: 0
    };
    
    // Event listeners
    this.eventListeners = new Map();
    
    // Current context tracking
    this.currentContext = {
      content: '',
      filePath: '',
      cursorPosition: 0,
      currentParagraph: '',
      contentHash: ''
    };
    
    // Initialize service
    this.init();
  }

  /**
   * Initialize the AI suggestion service
   * @private
   */
  async init() {
    try {
      
      // Set up content change detection
      this.setupContentChangeListeners();
      
      // Set up cache manager listeners
      this.setupCacheListeners();
      
      // Check backend availability
      await this.checkBackendAvailability();
      
      // Mark as ready
      this.setStatus(AiSuggestionService.STATUS.READY);
      
      
    } catch (error) {
      console.error('❌ Failed to initialize AI Suggestion Service:', error);
      this.setStatus(AiSuggestionService.STATUS.ERROR);
      this.handleError('Service initialization failed', error);
    }
  }

  /**
   * Set up content change detection listeners
   * @private
   */
  setupContentChangeListeners() {
    // Listen for content changes that should trigger suggestions
    this.contentDetector.addEventListener(
      'content_change_detected', 
      (data) => this.handleContentChange(data)
    );
    
    // Listen for performance warnings
    this.contentDetector.addEventListener(
      'performance_warning',
      (data) => this.handleContentDetectorPerformanceWarning(data)
    );
  }

  /**
   * Set up cache manager listeners
   * @private
   */
  setupCacheListeners() {
    // Track cache performance
    this.cacheManager.addEventListener('suggestion_cache_hit', (data) => {
      this.performanceStats.cacheHits++;
      this.updatePerformanceStats();
    });
    
    this.cacheManager.addEventListener('suggestion_cache_miss', (data) => {
      this.performanceStats.cacheMisses++;
      this.updatePerformanceStats();
    });
    
    this.cacheManager.addEventListener('suggestion_cache_error', (data) => {
      console.warn('Cache error:', data.error);
    });
  }

  /**
   * Check if backend services are available
   * @private
   */
  async checkBackendAvailability() {
    try {
      // Test cache service
      await this.cacheManager.getCachedSuggestions('test', this.config.MODEL_NAME, {});
      
      // Test similarity search service (this will likely return empty results, which is fine)
      await window.__TAURI__.core.invoke('optimized_search_similar_notes', {
        query: 'test query',
        maxResults: 1,
        similarityThreshold: 0.9
      });
      
      
    } catch (error) {
      console.warn('⚠️ Backend services may be limited:', error.message);
      // Don't fail initialization - some features may still work
    }
  }

  /**
   * Handle content change events from content detector
   * @param {Object} data - Content change data
   * @private
   */
  async handleContentChange(data) {
    if (!this.config.ENABLED || !this.config.AUTO_GENERATE) {
      return;
    }
    
    try {
      // Update current context
      this.updateCurrentContext(data);
      
      // Skip if content is too short or similar to last request
      if (!this.shouldGenerateSuggestions(data)) {
        return;
      }
      
      // Generate suggestions
      await this.generateSuggestions();
      
    } catch (error) {
      console.error('Error handling content change:', error);
      this.handleError('Failed to process content change', error);
    }
  }

  /**
   * Update current context from content change data
   * @param {Object} data - Content change data
   * @private
   */
  updateCurrentContext(data) {
    this.currentContext = {
      content: this.editor.getValue() || '',
      filePath: this.appState.currentFile || '',
      cursorPosition: data.cursorPosition || 0,
      currentParagraph: data.currentParagraph || '',
      contentHash: this.calculateContentHash(data.currentParagraph || '')
    };
  }

  /**
   * Calculate simple hash for content comparison
   * @param {string} content - Content to hash
   * @returns {string} Hash string
   * @private
   */
  calculateContentHash(content) {
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return hash.toString(36);
  }

  /**
   * Determine if suggestions should be generated for current content
   * @param {Object} data - Content change data
   * @returns {boolean} True if suggestions should be generated
   * @private
   */
  shouldGenerateSuggestions(data) {
    const currentParagraph = data.currentParagraph || '';
    
    // Skip if paragraph is too short
    if (currentParagraph.length < 20) {
      return false;
    }
    
    // Skip if generation is already in progress
    if (this.generationInProgress) {
      return false;
    }
    
    // Skip if too soon since last generation
    const timeSinceLastGeneration = Date.now() - this.lastGenerationTime;
    if (timeSinceLastGeneration < this.config.DEBOUNCE_DELAY) {
      return false;
    }
    
    // Skip if content hash is the same (no meaningful change)
    const contentHash = this.calculateContentHash(currentParagraph);
    if (contentHash === this.currentContext.contentHash) {
      return false;
    }
    
    return true;
  }

  /**
   * Generate suggestions for current content
   * @returns {Promise<Array>} Generated suggestions
   */
  async generateSuggestions() {
    if (!this.config.ENABLED || this.generationInProgress) {
      return [];
    }
    
    const startTime = performance.now();
    this.generationInProgress = true;
    this.setStatus(AiSuggestionService.STATUS.LOADING);
    
    // Create abort controller for cancellation
    this.currentGenerationAbortController = new AbortController();
    
    try {
      console.log('🔍 Generating AI suggestions...');
      
      // Emit loading event
      this.emit(AiSuggestionService.EVENTS.SUGGESTIONS_LOADING, {
        context: this.currentContext,
        timestamp: Date.now()
      });
      
      // Try cache first
      const cachedSuggestions = await this.tryGetCachedSuggestions();
      if (cachedSuggestions && cachedSuggestions.length > 0) {
        return this.processSuggestionResults(cachedSuggestions, startTime, true);
      }
      
      // Generate new suggestions
      const freshSuggestions = await this.generateFreshSuggestions();
      
      // Cache the results
      if (freshSuggestions.length > 0) {
        await this.cacheSuggestions(freshSuggestions);
      }
      
      return this.processSuggestionResults(freshSuggestions, startTime, false);
      
    } catch (error) {
      console.error('Failed to generate suggestions:', error);
      this.handleError('Failed to generate suggestions', error);
      return [];
      
    } finally {
      this.generationInProgress = false;
      this.lastGenerationTime = Date.now();
      this.currentGenerationAbortController = null;
      
      if (this.status === AiSuggestionService.STATUS.LOADING) {
        this.setStatus(AiSuggestionService.STATUS.READY);
      }
    }
  }

  /**
   * Try to get suggestions from cache
   * @returns {Promise<Array|null>} Cached suggestions or null
   * @private
   */
  async tryGetCachedSuggestions() {
    if (!this.config.CACHE_ENABLED) {
      return null;
    }
    
    try {
      const context = {
        currentFile: this.currentContext.filePath,
        cursorPosition: this.currentContext.cursorPosition,
        currentParagraph: this.currentContext.currentParagraph
      };
      
      return await this.cacheManager.getCachedSuggestions(
        this.currentContext.content,
        this.config.MODEL_NAME,
        context
      );
      
    } catch (error) {
      console.warn('Cache lookup failed:', error);
      return null;
    }
  }

  /**
   * Generate fresh suggestions from backend
   * @returns {Promise<Array>} Fresh suggestions
   * @private
   */
  async generateFreshSuggestions() {
    const searchQuery = this.currentContext.currentParagraph;
    
    // Use similarity search to find related notes
    const searchResults = await this.performSimilaritySearch(searchQuery);
    
    // Convert search results to suggestion format
    return this.convertSearchResultsToSuggestions(searchResults);
  }

  /**
   * Perform similarity search using backend service
   * @param {string} query - Search query
   * @returns {Promise<Array>} Search results
   * @private
   */
  async performSimilaritySearch(query) {
    try {
      const requestData = {
        query: query,
        maxResults: this.config.MAX_SUGGESTIONS,
        similarityThreshold: this.config.SIMILARITY_THRESHOLD,
        currentFile: this.currentContext.filePath
      };
      
      // Create timeout promise
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('Search timeout')), this.config.GENERATION_TIMEOUT);
      });
      
      // Perform search
      const searchPromise = window.__TAURI__.core.invoke('optimized_search_similar_notes', requestData);
      
      const searchResponse = await Promise.race([searchPromise, timeoutPromise]);
      
      if (searchResponse && searchResponse.results) {
        return searchResponse.results;
      }
      
      return [];
      
    } catch (error) {
      console.error('Similarity search failed:', error);
      throw new Error(`Similarity search failed: ${error.message}`);
    }
  }

  /**
   * Convert search results to suggestion format
   * @param {Array} searchResults - Raw search results
   * @returns {Array} Formatted suggestions
   * @private
   */
  convertSearchResultsToSuggestions(searchResults) {
    if (!Array.isArray(searchResults)) {
      return [];
    }
    
    return searchResults.map((result, index) => ({
      id: `suggestion-${Date.now()}-${index}`,
      title: this.extractTitleFromResult(result),
      content: result.content || result.text || '',
      relevanceScore: parseFloat(result.similarity || result.score || 0),
      contextSnippet: this.extractContextSnippet(result),
      filePath: result.file_path || result.path || '',
      metadata: {
        chunkId: result.chunk_id || null,
        lineNumbers: result.line_numbers || null,
        searchScore: result.similarity || result.score || 0,
        timestamp: Date.now()
      }
    }));
  }

  /**
   * Extract title from search result
   * @param {Object} result - Search result
   * @returns {string} Extracted title
   * @private
   */
  extractTitleFromResult(result) {
    if (result.title) {
      return result.title;
    }
    
    if (result.file_path) {
      return result.file_path.split('/').pop().replace(/\.[^/.]+$/, "");
    }
    
    if (result.content) {
      // Extract first meaningful line as title
      const lines = result.content.split('\n').filter(line => line.trim().length > 0);
      if (lines.length > 0) {
        let title = lines[0].trim();
        // Remove markdown syntax
        title = title.replace(/^#+\s*/, '').replace(/\*\*(.+?)\*\*/g, '$1');
        return title.length > 50 ? title.substring(0, 50) + '...' : title;
      }
    }
    
    return 'Untitled';
  }

  /**
   * Extract context snippet from result
   * @param {Object} result - Search result
   * @returns {string} Context snippet
   * @private
   */
  extractContextSnippet(result) {
    const content = result.content || result.text || '';
    const maxLength = 150;
    
    if (content.length <= maxLength) {
      return content;
    }
    
    // Try to break at word boundary
    const truncated = content.substring(0, maxLength);
    const lastSpace = truncated.lastIndexOf(' ');
    
    if (lastSpace > maxLength * 0.8) {
      return truncated.substring(0, lastSpace) + '...';
    }
    
    return truncated + '...';
  }

  /**
   * Cache generated suggestions
   * @param {Array} suggestions - Suggestions to cache
   * @private
   */
  async cacheSuggestions(suggestions) {
    if (!this.config.CACHE_ENABLED || suggestions.length === 0) {
      return;
    }
    
    try {
      const context = {
        currentFile: this.currentContext.filePath,
        cursorPosition: this.currentContext.cursorPosition,
        currentParagraph: this.currentContext.currentParagraph
      };
      
      await this.cacheManager.cacheSuggestions(
        this.currentContext.content,
        this.config.MODEL_NAME,
        suggestions,
        context
      );
      
    } catch (error) {
      console.warn('Failed to cache suggestions:', error);
    }
  }

  /**
   * Process and finalize suggestion results
   * @param {Array} suggestions - Raw suggestions
   * @param {number} startTime - Generation start time
   * @param {boolean} fromCache - Whether suggestions came from cache
   * @returns {Array} Processed suggestions
   * @private
   */
  processSuggestionResults(suggestions, startTime, fromCache) {
    const duration = performance.now() - startTime;
    
    // Sort by relevance score
    const sortedSuggestions = [...suggestions].sort((a, b) => b.relevanceScore - a.relevanceScore);
    
    // Update current suggestions
    this.currentSuggestions = sortedSuggestions;
    
    // Update performance stats
    this.performanceStats.totalRequests++;
    if (suggestions.length > 0) {
      this.performanceStats.successfulRequests++;
    } else {
      this.performanceStats.failedRequests++;
    }
    
    this.updatePerformanceStats(duration);
    
    // Emit suggestions updated event
    this.emit(AiSuggestionService.EVENTS.SUGGESTIONS_UPDATED, {
      suggestions: sortedSuggestions,
      context: this.currentContext,
      fromCache: fromCache,
      generationTime: duration,
      timestamp: Date.now()
    });
    
    
    return sortedSuggestions;
  }

  /**
   * Update performance statistics
   * @param {number} generationTime - Optional generation time
   * @private
   */
  updatePerformanceStats(generationTime = null) {
    if (generationTime !== null) {
      if (this.performanceStats.averageGenerationTime === 0) {
        this.performanceStats.averageGenerationTime = generationTime;
      } else {
        this.performanceStats.averageGenerationTime = 
          (this.performanceStats.averageGenerationTime * 0.8) + (generationTime * 0.2);
      }
    }
    
    this.performanceStats.lastRequestTime = Date.now();
    
    // Calculate cache hit rate
    const totalCacheAttempts = this.performanceStats.cacheHits + this.performanceStats.cacheMisses;
    const cacheHitRate = totalCacheAttempts > 0 ? this.performanceStats.cacheHits / totalCacheAttempts : 0;
    
    // Emit performance update
    this.emit(AiSuggestionService.EVENTS.CACHE_PERFORMANCE, {
      ...this.performanceStats,
      cacheHitRate: cacheHitRate,
      timestamp: Date.now()
    });
  }

  /**
   * Handle content detector performance warnings
   * @param {Object} data - Performance warning data
   * @private
   */
  handleContentDetectorPerformanceWarning(data) {
    console.warn('Content detector performance warning:', data);
    // Could implement adaptive throttling here
  }

  /**
   * Set service status and emit event
   * @param {string} newStatus - New status
   * @private
   */
  setStatus(newStatus) {
    if (this.status === newStatus) return;
    
    const oldStatus = this.status;
    this.status = newStatus;
    
    this.emit(AiSuggestionService.EVENTS.SERVICE_STATUS_CHANGED, {
      oldStatus,
      newStatus,
      timestamp: Date.now()
    });
    
  }

  /**
   * Handle service errors
   * @param {string} message - Error message
   * @param {Error} error - Error object
   * @private
   */
  handleError(message, error) {
    const errorData = {
      message,
      error: error ? error.message : 'Unknown error',
      stack: error ? error.stack : null,
      context: this.currentContext,
      timestamp: Date.now()
    };
    
    this.emit(AiSuggestionService.EVENTS.SUGGESTIONS_ERROR, errorData);
    
    // Set error status if not already in error state
    if (this.status !== AiSuggestionService.STATUS.ERROR) {
      this.setStatus(AiSuggestionService.STATUS.ERROR);
    }
  }

  /**
   * Manually request suggestions for current content
   * @returns {Promise<Array>} Generated suggestions
   */
  async requestSuggestions() {
    // Update current context from editor
    const currentContent = this.editor.getValue() || '';
    const currentParagraph = this.contentDetector.getCurrentExtraction()?.currentParagraph || '';
    
    this.updateCurrentContext({
      content: currentContent,
      currentParagraph: currentParagraph,
      cursorPosition: this.editor.cursorPosition || 0
    });
    
    return await this.generateSuggestions();
  }

  /**
   * Get current suggestions without generating new ones
   * @returns {Array} Current suggestions
   */
  getCurrentSuggestions() {
    return [...this.currentSuggestions];
  }

  /**
   * Clear current suggestions
   */
  clearSuggestions() {
    this.currentSuggestions = [];
    
    this.emit(AiSuggestionService.EVENTS.SUGGESTIONS_UPDATED, {
      suggestions: [],
      context: this.currentContext,
      cleared: true,
      timestamp: Date.now()
    });
  }

  /**
   * Cancel current suggestion generation
   */
  cancelCurrentGeneration() {
    if (this.currentGenerationAbortController) {
      this.currentGenerationAbortController.abort();
      this.currentGenerationAbortController = null;
    }
    
    this.generationInProgress = false;
    
    if (this.status === AiSuggestionService.STATUS.LOADING) {
      this.setStatus(AiSuggestionService.STATUS.READY);
    }
  }

  /**
   * Enable/disable the service
   * @param {boolean} enabled - Whether to enable the service
   */
  setEnabled(enabled) {
    const wasEnabled = this.config.ENABLED;
    this.config.ENABLED = enabled;
    
    if (!enabled && wasEnabled) {
      this.cancelCurrentGeneration();
      this.clearSuggestions();
      this.setStatus(AiSuggestionService.STATUS.DISABLED);
    } else if (enabled && !wasEnabled) {
      this.setStatus(AiSuggestionService.STATUS.READY);
    }
    
  }

  /**
   * Update service configuration
   * @param {Object} config - Configuration updates
   */
  updateConfig(config) {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get current service status
   * @returns {Object} Service status
   */
  getStatus() {
    return {
      status: this.status,
      enabled: this.config.ENABLED,
      generationInProgress: this.generationInProgress,
      currentSuggestionCount: this.currentSuggestions.length,
      lastGenerationTime: this.lastGenerationTime,
      performanceStats: { ...this.performanceStats },
      config: { ...this.config }
    };
  }

  /**
   * Add event listener
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   */
  addEventListener(eventType, handler) {
    if (!this.eventListeners.has(eventType)) {
      this.eventListeners.set(eventType, new Set());
    }
    this.eventListeners.get(eventType).add(handler);
  }

  /**
   * Remove event listener
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   */
  removeEventListener(eventType, handler) {
    const listeners = this.eventListeners.get(eventType);
    if (listeners) {
      listeners.delete(handler);
      if (listeners.size === 0) {
        this.eventListeners.delete(eventType);
      }
    }
  }

  /**
   * Emit event to listeners
   * @param {string} eventType - Event type
   * @param {Object} data - Event data
   * @private
   */
  emit(eventType, data) {
    const listeners = this.eventListeners.get(eventType);
    if (!listeners) return;

    listeners.forEach(handler => {
      try {
        handler(data);
      } catch (error) {
        console.error(`Error in AI suggestion service event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Cleanup service resources
   */
  destroy() {
    
    // Cancel any ongoing generation
    this.cancelCurrentGeneration();
    
    // Clear event listeners
    this.eventListeners.clear();
    
    // Clear suggestions
    this.currentSuggestions = [];
    
    // Reset state
    this.status = AiSuggestionService.STATUS.DISABLED;
    this.generationInProgress = false;
    
  }
}

export default AiSuggestionService;