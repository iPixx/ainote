/**
 * MarkdownEditor - Core markdown editor component for aiNote
 * 
 * Provides foundational text editing functionality with performance optimization
 * and preparation for advanced features like syntax highlighting and keyboard shortcuts.
 * 
 * Performance targets:
 * - Keystroke response: <16ms
 * - Memory usage: <10MB for large documents
 * - Initialization: <100ms
 * 
 * @class MarkdownEditor
 */
class MarkdownEditor {
  /**
   * Editor event types for component communication
   */
  static EVENTS = {
    CONTENT_CHANGED: 'content_changed',
    SELECTION_CHANGED: 'selection_changed',
    CURSOR_MOVED: 'cursor_moved',
    WORD_COUNT_CHANGED: 'word_count_changed'
  };

  /**
   * Initialize markdown editor with container and app state
   * @param {HTMLElement} container - Container element for the editor
   * @param {AppState} appState - Application state manager
   */
  constructor(container, appState) {
    if (!(container instanceof HTMLElement)) {
      throw new Error('Container must be a valid HTML element');
    }
    if (!appState) {
      throw new Error('AppState instance required');
    }

    this.container = container;
    this.appState = appState;
    
    // Editor elements
    this.textarea = null;
    this.overlayContainer = null;
    this.wordCountElement = null;
    this.charCountElement = null;
    
    // Editor state
    this.content = '';
    this.isInitialized = false;
    this.cursorPosition = 0;
    this.selectionStart = 0;
    this.selectionEnd = 0;
    
    // Performance tracking
    this.lastKeystroke = 0;
    this.debounceTimeout = null;
    
    // Event listeners for cleanup
    this.eventListeners = new Map();
    
    // Word count state
    this.wordCount = 0;
    this.charCount = 0;
    
    // Initialize editor
    this.init();
  }

  /**
   * Initialize the editor DOM structure and event listeners
   */
  init() {
    if (this.isInitialized) {
      console.warn('MarkdownEditor already initialized');
      return;
    }

    this.createEditorStructure();
    this.setupEventListeners();
    this.updateWordCount();
    this.isInitialized = true;
    
    console.log('✅ MarkdownEditor initialized');
  }

  /**
   * Create the DOM structure for the editor
   * @private
   */
  createEditorStructure() {
    // Clear existing content
    this.container.innerHTML = '';
    this.container.className = 'markdown-editor-container';

    // Create main editor wrapper
    const editorWrapper = document.createElement('div');
    editorWrapper.className = 'markdown-editor-wrapper';

    // Create textarea for input
    this.textarea = document.createElement('textarea');
    this.textarea.className = 'markdown-editor-textarea';
    this.textarea.placeholder = 'Start writing your markdown...';
    this.textarea.value = this.content;
    this.textarea.setAttribute('aria-label', 'Markdown editor');
    this.textarea.setAttribute('spellcheck', 'true');

    // Create overlay container for future syntax highlighting
    this.overlayContainer = document.createElement('div');
    this.overlayContainer.className = 'markdown-editor-overlay';
    this.overlayContainer.setAttribute('aria-hidden', 'true');

    // Create status bar
    const statusBar = document.createElement('div');
    statusBar.className = 'markdown-editor-status-bar';

    // Word count display
    this.wordCountElement = document.createElement('span');
    this.wordCountElement.className = 'word-count';
    this.wordCountElement.textContent = '0 words';

    // Character count display
    this.charCountElement = document.createElement('span');
    this.charCountElement.className = 'char-count';
    this.charCountElement.textContent = '0 characters';

    // Cursor position display
    this.cursorPositionElement = document.createElement('span');
    this.cursorPositionElement.className = 'cursor-position';
    this.cursorPositionElement.textContent = 'Line 1, Column 1';

    // Assemble status bar
    statusBar.appendChild(this.wordCountElement);
    statusBar.appendChild(document.createTextNode(' • '));
    statusBar.appendChild(this.charCountElement);
    statusBar.appendChild(document.createTextNode(' • '));
    statusBar.appendChild(this.cursorPositionElement);

    // Assemble editor
    editorWrapper.appendChild(this.overlayContainer);
    editorWrapper.appendChild(this.textarea);
    
    this.container.appendChild(editorWrapper);
    this.container.appendChild(statusBar);

    // Focus the textarea
    this.textarea.focus();
  }

  /**
   * Set up event listeners for editor functionality
   * @private
   */
  setupEventListeners() {
    // Input event for content changes (high performance)
    const inputHandler = (event) => {
      const startTime = performance.now();
      
      this.content = this.textarea.value;
      this.updateCursorPosition();
      this.debouncedWordCountUpdate();
      
      // Emit content change event
      this.emit(MarkdownEditor.EVENTS.CONTENT_CHANGED, {
        content: this.content,
        timestamp: Date.now()
      });

      // Track performance
      const duration = performance.now() - startTime;
      if (duration > 16) {
        console.warn(`⚠️ Slow input handler: ${duration.toFixed(2)}ms`);
      }
    };

    // Selection change for cursor tracking
    const selectionHandler = () => {
      this.updateCursorPosition();
      this.updateSelectionState();
      
      this.emit(MarkdownEditor.EVENTS.SELECTION_CHANGED, {
        start: this.selectionStart,
        end: this.selectionEnd,
        cursor: this.cursorPosition
      });
    };

    // Key events for cursor movement
    const keyHandler = (event) => {
      // Track keystroke performance
      this.lastKeystroke = performance.now();
      
      // Allow future keyboard shortcut system to hook here
      // This will be implemented in sub-issue #39
      this.emit(MarkdownEditor.EVENTS.CURSOR_MOVED, {
        key: event.key,
        position: this.cursorPosition
      });
    };

    // Register event listeners
    this.addEventListener(this.textarea, 'input', inputHandler);
    this.addEventListener(this.textarea, 'selectionchange', selectionHandler);
    this.addEventListener(this.textarea, 'select', selectionHandler);
    this.addEventListener(this.textarea, 'keyup', selectionHandler);
    this.addEventListener(this.textarea, 'mouseup', selectionHandler);
    this.addEventListener(this.textarea, 'keydown', keyHandler);

    // Scroll synchronization (prepare for preview mode)
    this.addEventListener(this.textarea, 'scroll', () => {
      // Future implementation: sync with preview pane
      this.overlayContainer.scrollTop = this.textarea.scrollTop;
    });
  }

  /**
   * Add event listener and track for cleanup
   * @param {HTMLElement} element - Element to add listener to
   * @param {string} event - Event name
   * @param {Function} handler - Event handler
   * @private
   */
  addEventListener(element, event, handler) {
    element.addEventListener(event, handler);
    
    if (!this.eventListeners.has(element)) {
      this.eventListeners.set(element, []);
    }
    this.eventListeners.get(element).push({ event, handler });
  }

  /**
   * Update cursor position and line/column information
   * @private
   */
  updateCursorPosition() {
    const textarea = this.textarea;
    this.cursorPosition = textarea.selectionStart;
    this.selectionStart = textarea.selectionStart;
    this.selectionEnd = textarea.selectionEnd;

    // Calculate line and column
    const textBeforeCursor = this.content.substring(0, this.cursorPosition);
    const lines = textBeforeCursor.split('\n');
    const currentLine = lines.length;
    const currentColumn = lines[lines.length - 1].length + 1;

    // Update cursor position display
    if (this.cursorPositionElement) {
      this.cursorPositionElement.textContent = `Line ${currentLine}, Column ${currentColumn}`;
    }
  }

  /**
   * Update selection state for future formatting features
   * @private
   */
  updateSelectionState() {
    const hasSelection = this.selectionStart !== this.selectionEnd;
    
    // Add selection class for future styling
    if (hasSelection) {
      this.container.classList.add('has-selection');
    } else {
      this.container.classList.remove('has-selection');
    }
  }

  /**
   * Update word and character count (debounced for performance)
   * @private
   */
  updateWordCount() {
    const text = this.content.trim();
    
    // Character count (including spaces)
    this.charCount = this.content.length;
    
    // Word count (split by whitespace, filter empty)
    this.wordCount = text === '' ? 0 : text.split(/\s+/).filter(word => word.length > 0).length;
    
    // Update display
    if (this.wordCountElement) {
      this.wordCountElement.textContent = `${this.wordCount} word${this.wordCount !== 1 ? 's' : ''}`;
    }
    if (this.charCountElement) {
      this.charCountElement.textContent = `${this.charCount} character${this.charCount !== 1 ? 's' : ''}`;
    }

    // Emit word count change event
    this.emit(MarkdownEditor.EVENTS.WORD_COUNT_CHANGED, {
      words: this.wordCount,
      characters: this.charCount
    });
  }

  /**
   * Debounced word count update for performance
   * @private
   */
  debouncedWordCountUpdate() {
    clearTimeout(this.debounceTimeout);
    this.debounceTimeout = setTimeout(() => {
      this.updateWordCount();
    }, 300); // 300ms debounce
  }

  /**
   * Set editor content
   * @param {string} content - New content for the editor
   */
  setValue(content) {
    if (typeof content !== 'string') {
      content = String(content || '');
    }

    this.content = content;
    
    if (this.textarea) {
      this.textarea.value = content;
      this.updateCursorPosition();
      this.updateWordCount();
      
      // Emit content change
      this.emit(MarkdownEditor.EVENTS.CONTENT_CHANGED, {
        content: this.content,
        timestamp: Date.now()
      });
    }
  }

  /**
   * Get current editor content
   * @returns {string} Current editor content
   */
  getValue() {
    return this.content;
  }

  /**
   * Insert text at current cursor position
   * @param {string} text - Text to insert
   * @param {boolean} replaceSelection - Whether to replace current selection
   */
  insertText(text, replaceSelection = true) {
    if (!this.textarea) {
      console.warn('Editor not initialized');
      return;
    }

    const startPos = replaceSelection ? this.selectionStart : this.cursorPosition;
    const endPos = replaceSelection ? this.selectionEnd : this.cursorPosition;
    
    // Insert text
    const before = this.content.substring(0, startPos);
    const after = this.content.substring(endPos);
    const newContent = before + text + after;
    
    this.setValue(newContent);
    
    // Position cursor after inserted text
    const newCursorPos = startPos + text.length;
    this.textarea.setSelectionRange(newCursorPos, newCursorPos);
    this.updateCursorPosition();
    this.textarea.focus();
  }

  /**
   * Get selected text
   * @returns {string} Currently selected text
   */
  getSelectedText() {
    return this.content.substring(this.selectionStart, this.selectionEnd);
  }

  /**
   * Set selection range
   * @param {number} start - Selection start position
   * @param {number} end - Selection end position
   */
  setSelection(start, end) {
    if (!this.textarea) return;
    
    this.textarea.setSelectionRange(start, end);
    this.updateCursorPosition();
    this.textarea.focus();
  }

  /**
   * Preserve scroll position (useful for content updates)
   * @returns {Object} Current scroll state
   */
  getScrollState() {
    if (!this.textarea) return { top: 0, left: 0 };
    
    return {
      top: this.textarea.scrollTop,
      left: this.textarea.scrollLeft
    };
  }

  /**
   * Restore scroll position
   * @param {Object} scrollState - Scroll state to restore
   */
  setScrollState(scrollState) {
    if (!this.textarea || !scrollState) return;
    
    this.textarea.scrollTop = scrollState.top;
    this.textarea.scrollLeft = scrollState.left;
  }

  /**
   * Focus the editor
   */
  focus() {
    if (this.textarea) {
      this.textarea.focus();
    }
  }

  /**
   * Check if editor has focus
   * @returns {boolean} True if editor is focused
   */
  hasFocus() {
    return document.activeElement === this.textarea;
  }

  /**
   * Emit event to listeners
   * @param {string} eventType - Event type
   * @param {Object} data - Event data
   * @private
   */
  emit(eventType, data) {
    // Create custom event
    const event = new CustomEvent(eventType, {
      detail: data,
      bubbles: false
    });
    
    // Dispatch on container
    this.container.dispatchEvent(event);
  }

  /**
   * Add event listener for editor events
   * @param {string} eventType - Event type from MarkdownEditor.EVENTS
   * @param {Function} handler - Event handler
   */
  addEventListener(eventType, handler) {
    this.container.addEventListener(eventType, handler);
  }

  /**
   * Remove event listener for editor events
   * @param {string} eventType - Event type from MarkdownEditor.EVENTS
   * @param {Function} handler - Event handler
   */
  removeEventListener(eventType, handler) {
    this.container.removeEventListener(eventType, handler);
  }

  /**
   * Clean up editor resources
   */
  destroy() {
    // Clear debounce timeout
    clearTimeout(this.debounceTimeout);
    
    // Remove all event listeners
    this.eventListeners.forEach((listeners, element) => {
      listeners.forEach(({ event, handler }) => {
        element.removeEventListener(event, handler);
      });
    });
    this.eventListeners.clear();
    
    // Clear DOM
    if (this.container) {
      this.container.innerHTML = '';
    }
    
    // Reset state
    this.isInitialized = false;
    this.content = '';
    this.textarea = null;
    this.overlayContainer = null;
    
    console.log('✅ MarkdownEditor destroyed');
  }

  /**
   * Get editor statistics for debugging/monitoring
   * @returns {Object} Editor statistics
   */
  getStats() {
    return {
      initialized: this.isInitialized,
      contentLength: this.content.length,
      wordCount: this.wordCount,
      charCount: this.charCount,
      cursorPosition: this.cursorPosition,
      hasSelection: this.selectionStart !== this.selectionEnd,
      selectionLength: this.selectionEnd - this.selectionStart,
      lastKeystrokeTime: this.lastKeystroke
    };
  }
}

// Export for ES6 module usage
export default MarkdownEditor;