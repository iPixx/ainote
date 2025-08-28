/**
 * ContentChangeDetector - Detects meaningful content changes for AI suggestions
 * 
 * Monitors editor content changes with intelligent debouncing to trigger
 * real-time suggestion system. Designed for paragraph-level granularity
 * and optimized to prevent excessive API calls while maintaining responsiveness.
 * 
 * Performance targets:
 * - Debouncing prevents >1 request per 500ms
 * - Content extraction completes in <10ms
 * - No editor lag during content monitoring
 * - Memory usage <5MB for monitoring system
 * 
 * @class ContentChangeDetector
 */
class ContentChangeDetector {
  /**
   * Content change detection events
   */
  static EVENTS = {
    CONTENT_CHANGE_DETECTED: 'content_change_detected',
    PARAGRAPH_EXTRACTED: 'paragraph_extracted',
    DEBOUNCE_TRIGGERED: 'debounce_triggered',
    PERFORMANCE_WARNING: 'performance_warning',
    MEMORY_WARNING: 'memory_warning'
  };

  /**
   * Default configuration for content detection
   */
  static DEFAULTS = {
    DEBOUNCE_DELAY: 500, // 500ms as specified in requirements
    MIN_CHANGE_THRESHOLD: 5, // Minimum characters changed to trigger
    PARAGRAPH_BUFFER_SIZE: 3, // Number of paragraphs around cursor to extract
    MAX_CONTENT_LENGTH: 10000, // Maximum content length for extraction (10KB)
    PERFORMANCE_TARGET: 10, // Content extraction target time in ms
    MAX_MEMORY_USAGE: 5242880 // 5MB memory target
  };

  /**
   * Initialize content change detector
   * @param {MarkdownEditor} editor - The markdown editor instance to monitor
   * @param {AppState} appState - Application state manager
   */
  constructor(editor, appState) {
    if (!editor) {
      throw new Error('MarkdownEditor instance required');
    }
    if (!appState) {
      throw new Error('AppState instance required');
    }

    this.editor = editor;
    this.appState = appState;
    
    // Configuration
    this.debounceDelay = ContentChangeDetector.DEFAULTS.DEBOUNCE_DELAY;
    this.minChangeThreshold = ContentChangeDetector.DEFAULTS.MIN_CHANGE_THRESHOLD;
    this.paragraphBufferSize = ContentChangeDetector.DEFAULTS.PARAGRAPH_BUFFER_SIZE;
    this.maxContentLength = ContentChangeDetector.DEFAULTS.MAX_CONTENT_LENGTH;
    this.performanceTarget = ContentChangeDetector.DEFAULTS.PERFORMANCE_TARGET;
    this.maxMemoryUsage = ContentChangeDetector.DEFAULTS.MAX_MEMORY_USAGE;
    
    // State tracking
    this.isEnabled = true;
    this.lastContent = '';
    this.lastCursorPosition = 0;
    this.lastExtractionTime = 0;
    this.debounceTimeout = null;
    this.isExtracting = false;
    
    // Event listeners
    this.eventListeners = new Map();
    
    // Performance monitoring
    this.performanceStats = {
      totalExtractions: 0,
      averageExtractionTime: 0,
      lastExtractionDuration: 0,
      peakMemoryUsage: 0,
      currentMemoryUsage: 0,
      performanceWarnings: 0,
      totalDebounces: 0,
      skippedExtractions: 0
    };
    
    // Content tracking for intelligent change detection
    this.contentHistory = {
      snapshots: [], // Ring buffer of content snapshots
      maxSnapshots: 10, // Keep last 10 snapshots for comparison
      currentIndex: 0
    };
    
    // Current extracted content cache
    this.extractedContent = {
      paragraphs: [],
      currentParagraph: '',
      cursorParagraphIndex: -1,
      contextParagraphs: [],
      timestamp: 0,
      valid: false
    };
    
    // Initialize the detector
    this.init();
  }

  /**
   * Initialize content change detection
   * @private
   */
  init() {
    if (!this.editor.isInitialized) {
      // Wait for editor initialization
      setTimeout(() => this.init(), 100);
      return;
    }

    this.setupEditorEventListeners();
    this.initializeContentState();
    
    console.log('✅ ContentChangeDetector initialized with 500ms debouncing');
  }

  /**
   * Setup event listeners on the markdown editor
   * @private
   */
  setupEditorEventListeners() {
    // Listen to content change events from the editor
    this.editor.addEventListener('content_changed', (event) => {
      this.handleContentChange(event.detail);
    });

    // Listen to cursor movement for context-aware extraction
    this.editor.addEventListener('selection_changed', (event) => {
      this.handleCursorChange(event.detail);
    });

    // Listen to cursor movement events for responsive updates
    this.editor.addEventListener('cursor_moved', (event) => {
      this.handleCursorMovement(event.detail);
    });
  }

  /**
   * Initialize content state tracking
   * @private
   */
  initializeContentState() {
    this.lastContent = this.editor.getValue() || '';
    this.lastCursorPosition = this.editor.cursorPosition || 0;
    this.addContentSnapshot(this.lastContent);
  }

  /**
   * Handle content change event from editor
   * @param {Object} changeData - Content change data from editor
   * @private
   */
  handleContentChange(changeData) {
    if (!this.isEnabled) return;

    const { content, timestamp } = changeData;
    const currentTime = performance.now();
    
    try {
      // Check if content actually changed meaningfully
      if (!this.isSignificantChange(content)) {
        this.performanceStats.skippedExtractions++;
        return;
      }

      // Cancel any pending debounced extraction
      this.cancelPendingExtraction();
      
      // Update content history
      this.updateContentHistory(content);
      
      // Schedule debounced extraction
      this.debounceTimeout = setTimeout(() => {
        this.performContentExtraction(content, timestamp);
      }, this.debounceDelay);
      
      this.performanceStats.totalDebounces++;
      
      // Emit debounce triggered event
      this.emit(ContentChangeDetector.EVENTS.DEBOUNCE_TRIGGERED, {
        debounceDelay: this.debounceDelay,
        contentLength: content.length,
        changeSize: Math.abs(content.length - this.lastContent.length),
        timestamp: currentTime
      });
      
    } catch (error) {
      console.error('Error handling content change:', error);
      this.handleError('content-change-handler', error);
    }
  }

  /**
   * Handle cursor position changes for context-aware extraction
   * @param {Object} cursorData - Cursor position data
   * @private
   */
  handleCursorChange(cursorData) {
    if (!this.isEnabled) return;
    
    const { cursor } = cursorData;
    this.lastCursorPosition = cursor;
    
    // If we have valid extracted content, update cursor paragraph
    if (this.extractedContent.valid) {
      this.updateCursorParagraphContext();
    }
  }

  /**
   * Handle cursor movement for responsive context updates
   * @param {Object} movementData - Cursor movement data
   * @private
   */
  handleCursorMovement(movementData) {
    if (!this.isEnabled) return;
    
    const { position } = movementData;
    
    // Only trigger rapid re-extraction if cursor moved to different paragraph
    if (this.extractedContent.valid && this.hasCursorChangedParagraph(position)) {
      // Quick extraction without full debouncing for cursor-driven changes
      setTimeout(() => {
        if (this.isEnabled && !this.isExtracting) {
          this.performQuickContextExtraction();
        }
      }, 50); // Quick 50ms delay for cursor responsiveness
    }
  }

  /**
   * Check if content change is significant enough to process
   * @param {string} newContent - New editor content
   * @returns {boolean} True if change is significant
   * @private
   */
  isSignificantChange(newContent) {
    const changeSize = Math.abs(newContent.length - this.lastContent.length);
    
    // Check minimum change threshold
    if (changeSize < this.minChangeThreshold) {
      return false;
    }
    
    // Check if content is actually different (not just cursor movement)
    if (newContent === this.lastContent) {
      return false;
    }
    
    // Check if content is too large to process efficiently
    if (newContent.length > this.maxContentLength) {
      console.warn(`Content too large (${newContent.length} chars), using truncated extraction`);
      return true; // Still process but will be truncated
    }
    
    return true;
  }

  /**
   * Update content history for intelligent change detection
   * @param {string} content - New content to add to history
   * @private
   */
  updateContentHistory(content) {
    this.addContentSnapshot(content);
    this.lastContent = content;
  }

  /**
   * Add content snapshot to ring buffer
   * @param {string} content - Content to snapshot
   * @private
   */
  addContentSnapshot(content) {
    const snapshot = {
      content,
      timestamp: Date.now(),
      length: content.length,
      hash: this.simpleHash(content)
    };
    
    this.contentHistory.snapshots[this.contentHistory.currentIndex] = snapshot;
    this.contentHistory.currentIndex = (this.contentHistory.currentIndex + 1) % this.contentHistory.maxSnapshots;
  }

  /**
   * Simple hash function for content comparison
   * @param {string} str - String to hash
   * @returns {number} Simple hash value
   * @private
   */
  simpleHash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return hash;
  }

  /**
   * Check if cursor has moved to a different paragraph
   * @param {number} newPosition - New cursor position
   * @returns {boolean} True if cursor changed paragraphs
   * @private
   */
  hasCursorChangedParagraph(newPosition) {
    if (!this.extractedContent.valid) return true;
    
    const currentParagraphIndex = this.extractedContent.cursorParagraphIndex;
    const newParagraphIndex = this.findParagraphIndexAtPosition(newPosition);
    
    return currentParagraphIndex !== newParagraphIndex;
  }

  /**
   * Find paragraph index at given cursor position
   * @param {number} position - Cursor position
   * @returns {number} Paragraph index
   * @private
   */
  findParagraphIndexAtPosition(position) {
    const content = this.editor.getValue();
    const beforeCursor = content.substring(0, position);
    return beforeCursor.split('\n\n').length - 1;
  }

  /**
   * Cancel any pending content extraction
   * @private
   */
  cancelPendingExtraction() {
    if (this.debounceTimeout) {
      clearTimeout(this.debounceTimeout);
      this.debounceTimeout = null;
    }
  }

  /**
   * Perform content extraction with performance monitoring
   * @param {string} content - Content to extract from
   * @param {number} timestamp - Original change timestamp
   * @private
   */
  async performContentExtraction(content, timestamp) {
    if (this.isExtracting) {
      console.log('Extraction already in progress, skipping');
      return;
    }

    this.isExtracting = true;
    const startTime = performance.now();

    try {
      // Truncate content if too large
      const workingContent = content.length > this.maxContentLength 
        ? content.substring(0, this.maxContentLength)
        : content;

      // Extract paragraph-level content
      const extractionResult = this.extractParagraphContent(workingContent);
      
      // Update extracted content cache
      this.extractedContent = {
        ...extractionResult,
        timestamp: Date.now(),
        valid: true
      };

      // Performance tracking
      const duration = performance.now() - startTime;
      this.updatePerformanceStats(duration);

      // Check performance target
      if (duration > this.performanceTarget) {
        this.emit(ContentChangeDetector.EVENTS.PERFORMANCE_WARNING, {
          duration,
          target: this.performanceTarget,
          contentLength: workingContent.length,
          timestamp: startTime
        });
      }

      // Emit extraction completed event
      this.emit(ContentChangeDetector.EVENTS.PARAGRAPH_EXTRACTED, {
        ...this.extractedContent,
        extractionDuration: duration,
        originalTimestamp: timestamp
      });

      // Emit content change detected event for AI system
      this.emit(ContentChangeDetector.EVENTS.CONTENT_CHANGE_DETECTED, {
        currentParagraph: extractionResult.currentParagraph,
        contextParagraphs: extractionResult.contextParagraphs,
        cursorPosition: this.lastCursorPosition,
        extractionTime: duration,
        timestamp: timestamp,
        changeId: `${timestamp}-${this.simpleHash(extractionResult.currentParagraph)}`
      });

      console.log(`📝 Content extraction completed in ${duration.toFixed(2)}ms`);

    } catch (error) {
      console.error('Content extraction failed:', error);
      this.handleError('content-extraction', error);
    } finally {
      this.isExtracting = false;
    }
  }

  /**
   * Perform quick context extraction for cursor movement
   * @private
   */
  performQuickContextExtraction() {
    if (this.isExtracting) return;

    const startTime = performance.now();
    
    try {
      const content = this.editor.getValue();
      const extractionResult = this.extractParagraphContent(content);
      
      // Only update if cursor paragraph actually changed
      if (extractionResult.currentParagraph !== this.extractedContent.currentParagraph) {
        this.extractedContent = {
          ...extractionResult,
          timestamp: Date.now(),
          valid: true
        };

        // Emit quick update event
        this.emit(ContentChangeDetector.EVENTS.CONTENT_CHANGE_DETECTED, {
          currentParagraph: extractionResult.currentParagraph,
          contextParagraphs: extractionResult.contextParagraphs,
          cursorPosition: this.lastCursorPosition,
          extractionTime: performance.now() - startTime,
          timestamp: Date.now(),
          changeId: `cursor-${Date.now()}-${this.simpleHash(extractionResult.currentParagraph)}`,
          quickUpdate: true
        });
      }
      
    } catch (error) {
      console.error('Quick context extraction failed:', error);
      this.handleError('quick-context-extraction', error);
    }
  }

  /**
   * Extract paragraph-level content around cursor position
   * @param {string} content - Content to extract from
   * @returns {Object} Extraction result with paragraphs and context
   * @private
   */
  extractParagraphContent(content) {
    const cursorPosition = this.lastCursorPosition;
    
    // Split content into paragraphs (double newline separation)
    const paragraphs = content.split('\n\n').filter(p => p.trim().length > 0);
    
    // Find cursor paragraph
    const cursorParagraphIndex = this.findCursorParagraphIndex(content, cursorPosition, paragraphs);
    const currentParagraph = paragraphs[cursorParagraphIndex] || '';
    
    // Extract context paragraphs around cursor
    const contextStart = Math.max(0, cursorParagraphIndex - this.paragraphBufferSize);
    const contextEnd = Math.min(paragraphs.length, cursorParagraphIndex + this.paragraphBufferSize + 1);
    const contextParagraphs = paragraphs.slice(contextStart, contextEnd);
    
    return {
      paragraphs,
      currentParagraph: currentParagraph.trim(),
      cursorParagraphIndex,
      contextParagraphs: contextParagraphs.map(p => p.trim()),
      totalParagraphs: paragraphs.length,
      contextRange: {
        start: contextStart,
        end: contextEnd - 1
      }
    };
  }

  /**
   * Find the paragraph index where the cursor is located
   * @param {string} content - Full content
   * @param {number} cursorPosition - Current cursor position
   * @param {Array} paragraphs - Array of paragraphs
   * @returns {number} Paragraph index
   * @private
   */
  findCursorParagraphIndex(content, cursorPosition, paragraphs) {
    let currentPosition = 0;
    
    for (let i = 0; i < paragraphs.length; i++) {
      const paragraphEnd = currentPosition + paragraphs[i].length;
      
      // Check if cursor is within this paragraph
      if (cursorPosition >= currentPosition && cursorPosition <= paragraphEnd + 2) { // +2 for \n\n
        return i;
      }
      
      currentPosition = paragraphEnd + 2; // +2 for paragraph separator
    }
    
    // Return last paragraph if cursor is at end
    return Math.max(0, paragraphs.length - 1);
  }

  /**
   * Update cursor paragraph context without full re-extraction
   * @private
   */
  updateCursorParagraphContext() {
    const newParagraphIndex = this.findParagraphIndexAtPosition(this.lastCursorPosition);
    
    if (newParagraphIndex !== this.extractedContent.cursorParagraphIndex) {
      // Update context around new cursor position
      const paragraphs = this.extractedContent.paragraphs;
      const contextStart = Math.max(0, newParagraphIndex - this.paragraphBufferSize);
      const contextEnd = Math.min(paragraphs.length, newParagraphIndex + this.paragraphBufferSize + 1);
      
      this.extractedContent.cursorParagraphIndex = newParagraphIndex;
      this.extractedContent.currentParagraph = paragraphs[newParagraphIndex] || '';
      this.extractedContent.contextParagraphs = paragraphs.slice(contextStart, contextEnd);
      this.extractedContent.timestamp = Date.now();
    }
  }

  /**
   * Update performance statistics
   * @param {number} duration - Extraction duration in milliseconds
   * @private
   */
  updatePerformanceStats(duration) {
    this.performanceStats.totalExtractions++;
    this.performanceStats.lastExtractionDuration = duration;
    
    // Update moving average
    if (this.performanceStats.averageExtractionTime === 0) {
      this.performanceStats.averageExtractionTime = duration;
    } else {
      this.performanceStats.averageExtractionTime = 
        (this.performanceStats.averageExtractionTime * 0.8) + (duration * 0.2);
    }
    
    // Update performance warnings counter
    if (duration > this.performanceTarget) {
      this.performanceStats.performanceWarnings++;
    }
    
    // Estimate memory usage (approximate)
    this.updateMemoryUsage();
  }

  /**
   * Update memory usage estimation
   * @private
   */
  updateMemoryUsage() {
    // Rough estimation of memory usage
    const contentSize = this.lastContent.length * 2; // Unicode characters
    const historySize = this.contentHistory.snapshots.reduce((total, snap) => 
      total + (snap ? snap.content.length * 2 : 0), 0);
    const extractedSize = JSON.stringify(this.extractedContent).length * 2;
    
    this.performanceStats.currentMemoryUsage = contentSize + historySize + extractedSize;
    
    if (this.performanceStats.currentMemoryUsage > this.performanceStats.peakMemoryUsage) {
      this.performanceStats.peakMemoryUsage = this.performanceStats.currentMemoryUsage;
    }
    
    // Check memory warning threshold
    if (this.performanceStats.currentMemoryUsage > this.maxMemoryUsage) {
      this.emit(ContentChangeDetector.EVENTS.MEMORY_WARNING, {
        currentUsage: this.performanceStats.currentMemoryUsage,
        maxUsage: this.maxMemoryUsage,
        peakUsage: this.performanceStats.peakMemoryUsage
      });
    }
  }

  /**
   * Handle errors with context-aware recovery
   * @param {string} context - Error context
   * @param {Error} error - Error object
   * @private
   */
  handleError(context, error) {
    console.error(`ContentChangeDetector [${context}]:`, error);
    
    // Attempt recovery based on error type
    if (context.includes('extraction')) {
      // Clear invalid extracted content
      this.extractedContent.valid = false;
      
      // Reduce extraction frequency temporarily
      this.debounceDelay = Math.min(this.debounceDelay * 1.5, 2000);
      
      setTimeout(() => {
        this.debounceDelay = ContentChangeDetector.DEFAULTS.DEBOUNCE_DELAY;
      }, 10000); // Reset after 10 seconds
    }
    
    if (context.includes('memory')) {
      // Clear content history to free memory
      this.clearContentHistory();
    }
  }

  /**
   * Clear content history to free memory
   * @private
   */
  clearContentHistory() {
    this.contentHistory.snapshots = [];
    this.contentHistory.currentIndex = 0;
    console.log('📝 Content history cleared for memory optimization');
  }

  /**
   * Enable content change detection
   */
  enable() {
    if (this.isEnabled) return;
    
    this.isEnabled = true;
    this.initializeContentState();
    console.log('📝 ContentChangeDetector enabled');
  }

  /**
   * Disable content change detection
   */
  disable() {
    if (!this.isEnabled) return;
    
    this.isEnabled = false;
    this.cancelPendingExtraction();
    console.log('📝 ContentChangeDetector disabled');
  }

  /**
   * Set debounce delay
   * @param {number} delay - Debounce delay in milliseconds (minimum 100ms)
   */
  setDebounceDelay(delay) {
    if (delay < 100 || delay > 5000) {
      throw new Error('Debounce delay must be between 100ms and 5000ms');
    }
    
    this.debounceDelay = delay;
    console.log(`📝 ContentChangeDetector debounce delay set to ${delay}ms`);
  }

  /**
   * Set minimum change threshold
   * @param {number} threshold - Minimum characters that must change
   */
  setMinChangeThreshold(threshold) {
    if (threshold < 1 || threshold > 100) {
      throw new Error('Change threshold must be between 1 and 100 characters');
    }
    
    this.minChangeThreshold = threshold;
    console.log(`📝 ContentChangeDetector change threshold set to ${threshold} characters`);
  }

  /**
   * Get current extracted content
   * @returns {Object|null} Current extracted content or null if invalid
   */
  getCurrentExtraction() {
    return this.extractedContent.valid ? { ...this.extractedContent } : null;
  }

  /**
   * Force immediate content extraction
   * @returns {Promise<Object|null>} Extraction result or null on error
   */
  async forceExtraction() {
    this.cancelPendingExtraction();
    
    const content = this.editor.getValue();
    await this.performContentExtraction(content, Date.now());
    
    return this.getCurrentExtraction();
  }

  /**
   * Add event listener for content change events
   * @param {string} eventType - Event type from ContentChangeDetector.EVENTS
   * @param {Function} handler - Event handler function
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
   * @param {Function} handler - Event handler function
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
        console.error(`Error in ContentChangeDetector event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Get performance statistics
   * @returns {Object} Performance stats object
   */
  getPerformanceStats() {
    return {
      ...this.performanceStats,
      configuration: {
        debounceDelay: this.debounceDelay,
        minChangeThreshold: this.minChangeThreshold,
        paragraphBufferSize: this.paragraphBufferSize,
        performanceTarget: this.performanceTarget,
        maxMemoryUsage: this.maxMemoryUsage
      },
      status: {
        enabled: this.isEnabled,
        extracting: this.isExtracting,
        hasValidExtraction: this.extractedContent.valid,
        lastExtractionAge: this.extractedContent.valid ? Date.now() - this.extractedContent.timestamp : null
      }
    };
  }

  /**
   * Get current status
   * @returns {Object} Current detector status
   */
  getStatus() {
    return {
      enabled: this.isEnabled,
      extracting: this.isExtracting,
      hasValidExtraction: this.extractedContent.valid,
      lastExtractionTime: this.extractedContent.timestamp,
      pendingExtraction: this.debounceTimeout !== null,
      contentLength: this.lastContent.length,
      cursorPosition: this.lastCursorPosition,
      stats: this.getPerformanceStats()
    };
  }

  /**
   * Cleanup detector resources
   */
  destroy() {
    // Cancel pending operations
    this.cancelPendingExtraction();
    
    // Clear event listeners
    this.eventListeners.clear();
    
    // Clear content history
    this.clearContentHistory();
    
    // Reset state
    this.isEnabled = false;
    this.extractedContent.valid = false;
    this.lastContent = '';
    
    // Clear references
    this.editor = null;
    this.appState = null;
    
    console.log('✅ ContentChangeDetector destroyed');
  }
}

// Export for ES6 module usage
export default ContentChangeDetector;