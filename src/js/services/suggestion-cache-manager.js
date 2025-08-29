/**
 * Suggestion Cache Manager - Frontend integration for AI suggestion caching
 * 
 * Manages intelligent caching of AI-powered note suggestions with context awareness,
 * cache invalidation, and performance optimization. This service integrates with
 * the Rust-based caching system and provides seamless suggestion management
 * for the frontend components.
 * 
 * Performance targets:
 * - Cache hit rate >70% for repeat queries
 * - Cache lookup completes in <10ms
 * - Memory usage <25MB for cache system
 * - Cache operations don't block suggestion generation
 * 
 * @class SuggestionCacheManager
 */
class SuggestionCacheManager {
  /**
   * Cache manager events
   */
  static EVENTS = {
    CACHE_HIT: 'suggestion_cache_hit',
    CACHE_MISS: 'suggestion_cache_miss',
    CACHE_INVALIDATED: 'suggestion_cache_invalidated',
    CACHE_ERROR: 'suggestion_cache_error',
    METRICS_UPDATED: 'suggestion_cache_metrics_updated'
  };

  /**
   * Default configuration for cache manager
   */
  static DEFAULTS = {
    ENABLE_CACHING: true,
    ENABLE_AUTO_INVALIDATION: true,
    ENABLE_METRICS_TRACKING: true,
    CACHE_LOOKUP_TIMEOUT: 50, // 50ms timeout for cache lookups
    INVALIDATION_DEBOUNCE: 1000, // 1s debounce for content-based invalidation
    METRICS_UPDATE_INTERVAL: 30000, // 30s metrics update interval
    MAX_CONTENT_HASH_LENGTH: 10000 // 10KB max content for hashing
  };

  /**
   * Initialize suggestion cache manager
   * @param {AppState} appState - Application state manager
   */
  constructor(appState) {
    if (!appState) {
      throw new Error('AppState instance required');
    }

    this.appState = appState;
    
    // Configuration
    this.isEnabled = SuggestionCacheManager.DEFAULTS.ENABLE_CACHING;
    this.enableAutoInvalidation = SuggestionCacheManager.DEFAULTS.ENABLE_AUTO_INVALIDATION;
    this.enableMetricsTracking = SuggestionCacheManager.DEFAULTS.ENABLE_METRICS_TRACKING;
    this.cacheLookupTimeout = SuggestionCacheManager.DEFAULTS.CACHE_LOOKUP_TIMEOUT;
    this.invalidationDebounce = SuggestionCacheManager.DEFAULTS.INVALIDATION_DEBOUNCE;
    this.metricsUpdateInterval = SuggestionCacheManager.DEFAULTS.METRICS_UPDATE_INTERVAL;
    this.maxContentHashLength = SuggestionCacheManager.DEFAULTS.MAX_CONTENT_HASH_LENGTH;
    
    // State tracking
    this.currentFile = null;
    this.currentVault = null;
    this.lastContentHash = null;
    this.lastInvalidationTime = 0;
    this.invalidationTimeout = null;
    this.metricsInterval = null;
    
    // Event listeners
    this.eventListeners = new Map();
    
    // Performance metrics
    this.performanceStats = {
      totalLookups: 0,
      cacheHits: 0,
      cacheMisses: 0,
      averageLookupTime: 0,
      totalInvalidations: 0,
      lastMetricsUpdate: 0
    };
    
    // Initialize the manager
    this.init();
  }

  /**
   * Initialize cache manager
   * @private
   */
  init() {
    this.setupAppStateListeners();
    
    if (this.enableMetricsTracking) {
      this.startMetricsTracking();
    }
    
    console.log('✅ SuggestionCacheManager initialized');
  }

  /**
   * Setup application state listeners for cache invalidation
   * @private
   */
  setupAppStateListeners() {
    // Listen for file changes
    this.appState.addEventListener('currentFileChanged', (event) => {
      const newFile = event.detail.filePath;
      if (this.currentFile !== newFile) {
        this.currentFile = newFile;
        if (this.enableAutoInvalidation) {
          this.scheduleInvalidationForFile(this.currentFile);
        }
      }
    });

    // Listen for vault changes
    this.appState.addEventListener('vaultChanged', (event) => {
      const newVault = event.detail.vaultPath;
      if (this.currentVault !== newVault) {
        this.currentVault = newVault;
        if (this.enableAutoInvalidation) {
          this.clearAllCache();
        }
      }
    });
  }

  /**
   * Get cached suggestions for content and context
   * @param {string} content - Current editor content
   * @param {string} model - AI model name
   * @param {Object} context - Current editing context
   * @returns {Promise<Array|null>} Cached suggestions or null if cache miss
   */
  async getCachedSuggestions(content, model, context = {}) {
    if (!this.isEnabled) {
      return null;
    }

    const startTime = performance.now();
    
    try {
      // Prepare context for cache lookup
      const cacheContext = this.prepareCacheContext(content, context);
      
      // Create timeout promise
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('Cache lookup timeout')), this.cacheLookupTimeout);
      });

      // Race between cache lookup and timeout
      const cachePromise = window.__TAURI__.core.invoke('get_cached_suggestions', {
        content: this.truncateContent(content),
        model: model,
        currentFile: cacheContext.currentFile,
        vaultPath: cacheContext.vaultPath,
        contentLength: content.length,
        cursorPosition: cacheContext.cursorPosition,
        currentParagraph: cacheContext.currentParagraph
      });

      const suggestions = await Promise.race([cachePromise, timeoutPromise]);
      
      // Update performance stats
      const lookupTime = performance.now() - startTime;
      this.updatePerformanceStats(true, lookupTime);
      
      if (suggestions && suggestions.length > 0) {
        this.emit(SuggestionCacheManager.EVENTS.CACHE_HIT, {
          model,
          suggestionsCount: suggestions.length,
          lookupTime,
          context: cacheContext
        });
        
        console.log(`✅ Suggestion cache HIT: ${suggestions.length} suggestions (${lookupTime.toFixed(1)}ms)`);
        return suggestions;
      } else {
        this.emit(SuggestionCacheManager.EVENTS.CACHE_MISS, {
          model,
          lookupTime,
          context: cacheContext
        });
        
        console.log(`❌ Suggestion cache MISS (${lookupTime.toFixed(1)}ms)`);
        return null;
      }
      
    } catch (error) {
      const lookupTime = performance.now() - startTime;
      this.updatePerformanceStats(false, lookupTime);
      
      this.emit(SuggestionCacheManager.EVENTS.CACHE_ERROR, {
        error: error.message,
        operation: 'get_cached_suggestions',
        lookupTime
      });
      
      console.warn('⚠️ Suggestion cache lookup failed:', error.message);
      return null;
    }
  }

  /**
   * Cache suggestions with context
   * @param {string} content - Content suggestions were generated for
   * @param {string} model - AI model name
   * @param {Array} suggestions - Generated suggestions
   * @param {Object} context - Current editing context
   * @returns {Promise<boolean>} True if successfully cached
   */
  async cacheSuggestions(content, model, suggestions, context = {}) {
    if (!this.isEnabled || !suggestions || suggestions.length === 0) {
      return false;
    }

    try {
      const cacheContext = this.prepareCacheContext(content, context);
      
      await window.__TAURI__.core.invoke('cache_suggestions', {
        content: this.truncateContent(content),
        model: model,
        suggestions: suggestions,
        currentFile: cacheContext.currentFile,
        vaultPath: cacheContext.vaultPath,
        contentLength: content.length,
        cursorPosition: cacheContext.cursorPosition,
        currentParagraph: cacheContext.currentParagraph
      });

      console.log(`💾 Cached ${suggestions.length} suggestions for model: ${model}`);
      return true;
      
    } catch (error) {
      this.emit(SuggestionCacheManager.EVENTS.CACHE_ERROR, {
        error: error.message,
        operation: 'cache_suggestions'
      });
      
      console.error('❌ Failed to cache suggestions:', error.message);
      return false;
    }
  }

  /**
   * Check if suggestions are cached for content
   * @param {string} content - Content to check
   * @param {string} model - AI model name
   * @param {Object} context - Current editing context
   * @returns {Promise<boolean>} True if suggestions are cached
   */
  async isSuggestionCached(content, model, context = {}) {
    if (!this.isEnabled) {
      return false;
    }

    try {
      const cacheContext = this.prepareCacheContext(content, context);
      
      const isCached = await window.__TAURI__.core.invoke('check_suggestion_cached', {
        content: this.truncateContent(content),
        model: model,
        currentFile: cacheContext.currentFile,
        contentLength: content.length,
        cursorPosition: cacheContext.cursorPosition,
        currentParagraph: cacheContext.currentParagraph
      });

      return isCached;
      
    } catch (error) {
      console.warn('⚠️ Failed to check suggestion cache:', error.message);
      return false;
    }
  }

  /**
   * Invalidate cache for current file
   * @param {string} filePath - Optional file path (uses current file if not provided)
   * @returns {Promise<number>} Number of invalidated entries
   */
  async invalidateFile(filePath = null) {
    if (!this.isEnabled) {
      return 0;
    }

    const targetFile = filePath || this.currentFile;
    if (!targetFile) {
      return 0;
    }

    try {
      const invalidatedCount = await window.__TAURI__.core.invoke('invalidate_suggestions_for_file', {
        filePath: targetFile
      });

      this.performanceStats.totalInvalidations += invalidatedCount;
      
      this.emit(SuggestionCacheManager.EVENTS.CACHE_INVALIDATED, {
        filePath: targetFile,
        invalidatedCount
      });

      if (invalidatedCount > 0) {
        console.log(`🗑️ Invalidated ${invalidatedCount} suggestion cache entries for: ${targetFile}`);
      }
      
      return invalidatedCount;
      
    } catch (error) {
      this.emit(SuggestionCacheManager.EVENTS.CACHE_ERROR, {
        error: error.message,
        operation: 'invalidate_file'
      });
      
      console.error('❌ Failed to invalidate suggestion cache:', error.message);
      return 0;
    }
  }

  /**
   * Clear all cached suggestions
   * @returns {Promise<boolean>} True if successfully cleared
   */
  async clearAllCache() {
    if (!this.isEnabled) {
      return false;
    }

    try {
      await window.__TAURI__.core.invoke('clear_suggestion_cache');
      
      this.emit(SuggestionCacheManager.EVENTS.CACHE_INVALIDATED, {
        filePath: null,
        invalidatedCount: -1 // -1 indicates full clear
      });

      console.log('🗑️ All suggestion cache cleared');
      return true;
      
    } catch (error) {
      this.emit(SuggestionCacheManager.EVENTS.CACHE_ERROR, {
        error: error.message,
        operation: 'clear_cache'
      });
      
      console.error('❌ Failed to clear suggestion cache:', error.message);
      return false;
    }
  }

  /**
   * Get cache performance metrics
   * @returns {Promise<Object>} Cache performance metrics
   */
  async getCacheMetrics() {
    if (!this.isEnabled) {
      return {};
    }

    try {
      const metrics = await window.__TAURI__.core.invoke('get_suggestion_cache_metrics');
      return {
        ...metrics,
        frontend: this.performanceStats
      };
      
    } catch (error) {
      console.error('❌ Failed to get cache metrics:', error.message);
      return { frontend: this.performanceStats };
    }
  }

  /**
   * Warm cache for frequently accessed file
   * @param {string} filePath - File path to warm cache for
   * @returns {Promise<boolean>} True if cache warming initiated
   */
  async warmCacheForFile(filePath) {
    if (!this.isEnabled || !filePath) {
      return false;
    }

    try {
      await window.__TAURI__.core.invoke('warm_suggestion_cache_for_file', {
        filePath: filePath
      });

      console.log(`🔥 Cache warming initiated for: ${filePath}`);
      return true;
      
    } catch (error) {
      console.error('❌ Failed to warm cache:', error.message);
      return false;
    }
  }

  /**
   * Prepare cache context from current state
   * @param {string} content - Current content
   * @param {Object} context - Additional context
   * @returns {Object} Prepared cache context
   * @private
   */
  prepareCacheContext(content, context) {
    return {
      currentFile: context.currentFile || this.currentFile,
      vaultPath: context.vaultPath || this.currentVault,
      cursorPosition: context.cursorPosition || 0,
      currentParagraph: context.currentParagraph || this.extractCurrentParagraph(content, context.cursorPosition)
    };
  }

  /**
   * Extract current paragraph from content
   * @param {string} content - Full content
   * @param {number} cursorPosition - Cursor position
   * @returns {string} Current paragraph
   * @private
   */
  extractCurrentParagraph(content, cursorPosition = 0) {
    if (!content) return '';
    
    const paragraphs = content.split('\n\n');
    let currentPos = 0;
    
    for (const paragraph of paragraphs) {
      const paragraphEnd = currentPos + paragraph.length + 2;
      if (cursorPosition >= currentPos && cursorPosition <= paragraphEnd) {
        return paragraph.trim();
      }
      currentPos = paragraphEnd;
    }
    
    return paragraphs[paragraphs.length - 1] || '';
  }

  /**
   * Truncate content for cache key generation
   * @param {string} content - Content to truncate
   * @returns {string} Truncated content
   * @private
   */
  truncateContent(content) {
    if (content.length <= this.maxContentHashLength) {
      return content;
    }
    return content.substring(0, this.maxContentHashLength);
  }

  /**
   * Schedule invalidation for file with debouncing
   * @param {string} filePath - File path to invalidate
   * @private
   */
  scheduleInvalidationForFile(filePath) {
    if (!filePath) return;
    
    // Clear existing timeout
    if (this.invalidationTimeout) {
      clearTimeout(this.invalidationTimeout);
    }
    
    // Schedule debounced invalidation
    this.invalidationTimeout = setTimeout(async () => {
      await this.invalidateFile(filePath);
      this.invalidationTimeout = null;
    }, this.invalidationDebounce);
  }

  /**
   * Update performance statistics
   * @param {boolean} isHit - Whether this was a cache hit
   * @param {number} lookupTime - Lookup time in milliseconds
   * @private
   */
  updatePerformanceStats(isHit, lookupTime) {
    this.performanceStats.totalLookups++;
    
    if (isHit) {
      this.performanceStats.cacheHits++;
    } else {
      this.performanceStats.cacheMisses++;
    }
    
    // Update moving average
    if (this.performanceStats.averageLookupTime === 0) {
      this.performanceStats.averageLookupTime = lookupTime;
    } else {
      this.performanceStats.averageLookupTime = 
        (this.performanceStats.averageLookupTime * 0.8) + (lookupTime * 0.2);
    }
    
    this.performanceStats.hitRate = this.performanceStats.totalLookups > 0
      ? this.performanceStats.cacheHits / this.performanceStats.totalLookups
      : 0;
  }

  /**
   * Start metrics tracking interval
   * @private
   */
  startMetricsTracking() {
    this.metricsInterval = setInterval(async () => {
      try {
        const metrics = await this.getCacheMetrics();
        this.performanceStats.lastMetricsUpdate = Date.now();
        
        this.emit(SuggestionCacheManager.EVENTS.METRICS_UPDATED, metrics);
        
      } catch (error) {
        console.warn('⚠️ Failed to update cache metrics:', error.message);
      }
    }, this.metricsUpdateInterval);
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
        console.error(`Error in suggestion cache event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Enable cache manager
   */
  enable() {
    this.isEnabled = true;
    console.log('✅ SuggestionCacheManager enabled');
  }

  /**
   * Disable cache manager
   */
  disable() {
    this.isEnabled = false;
    
    if (this.invalidationTimeout) {
      clearTimeout(this.invalidationTimeout);
      this.invalidationTimeout = null;
    }
    
    console.log('❌ SuggestionCacheManager disabled');
  }

  /**
   * Get current status
   * @returns {Object} Current cache manager status
   */
  getStatus() {
    return {
      enabled: this.isEnabled,
      autoInvalidation: this.enableAutoInvalidation,
      metricsTracking: this.enableMetricsTracking,
      currentFile: this.currentFile,
      currentVault: this.currentVault,
      performanceStats: this.performanceStats
    };
  }

  /**
   * Cleanup manager resources
   */
  destroy() {
    // Clear timers
    if (this.invalidationTimeout) {
      clearTimeout(this.invalidationTimeout);
    }
    if (this.metricsInterval) {
      clearInterval(this.metricsInterval);
    }
    
    // Clear listeners
    this.eventListeners.clear();
    
    // Reset state
    this.isEnabled = false;
    this.currentFile = null;
    this.currentVault = null;
    
    console.log('✅ SuggestionCacheManager destroyed');
  }
}

// Export for ES6 module usage
export default SuggestionCacheManager;