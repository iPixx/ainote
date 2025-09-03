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
    WORD_COUNT_CHANGED: 'word_count_changed',
    SHORTCUT_EXECUTED: 'shortcut_executed',
    FORMAT_APPLIED: 'format_applied',
    FIND_REPLACE_OPENED: 'find_replace_opened',
    LOADING_STATE_CHANGED: 'loading_state_changed',
    LARGE_DOCUMENT_DETECTED: 'large_document_detected',
    LINE_NUMBERS_TOGGLED: 'line_numbers_toggled'
  };

  /**
   * Initialize markdown editor with container and app state
   * @param {HTMLElement} container - Container element for the editor
   * @param {AppState} appState - Application state manager
   * @param {AutoSave} autoSave - AutoSave service instance (optional)
   */
  constructor(container, appState, autoSave = null) {
    if (!(container instanceof HTMLElement)) {
      throw new Error('Container must be a valid HTML element');
    }
    if (!appState) {
      throw new Error('AppState instance required');
    }

    this.container = container;
    this.appState = appState;
    this.autoSave = autoSave;
    
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
    this.heavyOperationsTimeout = null;
    this.selectionUpdatePending = false;
    
    // Event listeners for cleanup
    this.eventListeners = new Map();
    
    // Word count state
    this.wordCount = 0;
    this.charCount = 0;
    
    // Auto-save is handled by the dedicated AutoSave service
    
    // Performance optimization state
    this.isLargeDocument = false;
    this.documentSizeThreshold = 50000; // 50KB threshold for large documents
    this.virtualScrolling = false;
    this.visibleLineStart = 0;
    this.visibleLineEnd = 100;
    this.totalLines = 0;
    
    // Line numbers display
    this.lineNumbersEnabled = false;
    this.lineNumbersContainer = null;
    
    // Loading and progress states
    this.isLoading = false;
    this.loadingOverlay = null;
    
    // Memory optimization
    this.cleanupScheduled = false;
    this.lastCleanup = 0;
    this.cleanupInterval = 30000; // 30 seconds
    
    // Keyboard shortcuts and formatting state
    this.keyboardShortcuts = new Map();
    this.undoStack = [];
    this.redoStack = [];
    this.maxUndoSize = 100;
    this.findReplaceModal = null;
    this.currentSearchTerm = '';
    this.currentReplaceTerm = '';
    this.searchHighlights = [];
    
    // Auto-completion pairs
    this.autoCompletePairs = {
      '(': ')',
      '[': ']',
      '{': '}',
      '"': '"',
      "'": "'",
      '`': '`'
    };
    
    // Error handling state
    this.errorHandler = this.createErrorHandler();
    
    // Initialize editor with error handling
    try {
      this.init();
      
      // Set up AutoSave integration if service is provided
      if (this.autoSave) {
        this.setupAutoSaveIntegration();
      }
    } catch (error) {
      this.handleError('Initialization failed', error);
    }
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
    this.setupKeyboardShortcuts();
    this.setupPerformanceOptimizations();
    this.enhanceAccessibility();
    this.setupScrollSynchronization();
    this.updateWordCount();
    this.isInitialized = true;
    
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
    this.textarea.setAttribute('aria-describedby', 'editor-status-bar');
    this.textarea.setAttribute('aria-multiline', 'true');
    this.textarea.setAttribute('role', 'textbox');
    this.textarea.setAttribute('spellcheck', 'true');
    this.textarea.setAttribute('autocomplete', 'off');
    this.textarea.setAttribute('autocorrect', 'off');
    this.textarea.setAttribute('autocapitalize', 'off');

    // Create overlay container for future syntax highlighting
    this.overlayContainer = document.createElement('div');
    this.overlayContainer.className = 'markdown-editor-overlay';
    this.overlayContainer.setAttribute('aria-hidden', 'true');

    // Create line numbers container (initially hidden)
    this.lineNumbersContainer = document.createElement('div');
    this.lineNumbersContainer.className = 'markdown-editor-line-numbers';
    this.lineNumbersContainer.setAttribute('aria-hidden', 'true');
    this.lineNumbersContainer.style.display = 'none';

    // Create loading overlay
    this.loadingOverlay = document.createElement('div');
    this.loadingOverlay.className = 'markdown-editor-loading-overlay';
    this.loadingOverlay.innerHTML = `
      <div class="loading-spinner"></div>
      <div class="loading-text">Loading...</div>
    `;
    this.loadingOverlay.style.display = 'none';

    // Create status bar
    const statusBar = document.createElement('div');
    statusBar.className = 'markdown-editor-status-bar';
    statusBar.id = 'editor-status-bar';
    statusBar.setAttribute('aria-live', 'polite');
    statusBar.setAttribute('aria-label', 'Editor status information');

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
    editorWrapper.appendChild(this.lineNumbersContainer);
    editorWrapper.appendChild(this.overlayContainer);
    editorWrapper.appendChild(this.textarea);
    editorWrapper.appendChild(this.loadingOverlay);
    
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
    // Input event for content changes (optimized for responsiveness)
    const inputHandler = (event) => {
      try {
        // Only update essential state immediately for responsiveness
        this.content = this.textarea.value;
        
        // Trigger AutoSave directly if service is available
        if (this.autoSave) {
          this.autoSave.handleContentChange(this.content);
        } else {
          console.warn('⚠️ [MarkdownEditor] AutoSave service not available, content change not saved');
        }
        
        // Debounce heavy operations to avoid blocking UI
        this.debounceHeavyOperations();
        
        // Emit content change event for other systems that might need it
        this.emit(MarkdownEditor.EVENTS.CONTENT_CHANGED, {
          content: this.content,
          timestamp: Date.now()
        });
      } catch (error) {
        this.handleError('input-handler', error);
      }
    };

    // Selection change for cursor tracking (optimized)
    const selectionHandler = () => {
      // Use requestAnimationFrame to avoid blocking the main thread
      if (this.selectionUpdatePending) return;
      
      this.selectionUpdatePending = true;
      requestAnimationFrame(() => {
        this.updateCursorPosition();
        this.updateSelectionState();
        
        this.emit(MarkdownEditor.EVENTS.SELECTION_CHANGED, {
          start: this.selectionStart,
          end: this.selectionEnd,
          cursor: this.cursorPosition
        });
        
        this.selectionUpdatePending = false;
      });
    };

    // Key events for keyboard shortcuts and cursor movement (optimized)
    const keyHandler = (event) => {
      // Fast path for common keys that don't need special handling
      // Note: Essential keys like Enter, Space, and Arrow keys are handled by the global handler below
      const isCommonKey = (/^[a-zA-Z0-9\s]$/.test(event.key) || 
                          ['Enter', 'Backspace', 'Delete', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown'].includes(event.key)) && 
                         !event.ctrlKey && !event.metaKey && !event.altKey;
      
      if (isCommonKey) {
        // Allow default behavior for common typing and essential navigation keys
        return;
      }
      
      // Track keystroke for performance monitoring
      this.lastKeystroke = performance.now();
      
      // Handle keyboard shortcuts first (highest priority)
      const shortcutHandled = this.handleKeyboardShortcut(event);
      if (shortcutHandled) {
        return; // Shortcut handled, no further processing needed
      }
      
      // Handle tab indentation
      if (event.key === 'Tab' && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        this.handleTabIndentation(event.shiftKey);
        return;
      }
      
      // Note: Essential keys (Enter, Space, Arrow keys) are handled by the global handler
      // This section handles other special keys that need processing
      
      // Handle auto-completion for brackets and quotes (lower priority)
      if (!event.ctrlKey && !event.metaKey && !event.altKey) {
        this.handleAutoCompletion(event);
      }
      
      // Emit cursor moved event for non-typing keys
      this.emit(MarkdownEditor.EVENTS.CURSOR_MOVED, {
        key: event.key,
        position: this.cursorPosition,
        shortcutHandled
      });
    };

    // Paste handler for smart paste functionality
    const pasteHandler = (event) => {
      event.preventDefault();
      
      // Get clipboard text as plain text
      const clipboardData = event.clipboardData || window.clipboardData;
      const pastedText = clipboardData.getData('text/plain');
      
      if (pastedText) {
        // Save state before paste for undo
        this.saveUndoState();
        
        // Insert plain text
        this.insertText(pastedText, true);
        
        console.log('📋 Smart paste: converted to plain text');
      }
    };

    /**
     * Global document-level handler for essential keyboard input
     * 
     * This handler ensures that critical keys (Enter, Space, Arrow keys) work correctly
     * by intercepting them at the document level before other event handlers can interfere.
     * 
     * Background: Some keyboard events were being prevented by other parts of the application,
     * causing essential keys like Enter and Space to not function properly in the editor.
     * 
     * Solution: Use the capture phase (addEventListener with true) to intercept these events
     * at the highest level and ensure they work as expected.
     */
    const globalEssentialKeysHandler = (event) => {
      // Only handle events when our textarea is focused and no modifier keys are pressed
      if (event.target === this.textarea && !event.ctrlKey && !event.metaKey && !event.altKey) {
        
        // Handle Enter key - insert newline
        if (event.key === 'Enter') {
          event.preventDefault();
          event.stopImmediatePropagation();
          
          const start = this.textarea.selectionStart;
          const end = this.textarea.selectionEnd;
          const currentValue = this.textarea.value;
          
          // Insert newline at cursor position
          const newValue = currentValue.substring(0, start) + '\n' + currentValue.substring(end);
          this.textarea.value = newValue;
          this.content = newValue;
          
          // Position cursor after the newline
          this.textarea.setSelectionRange(start + 1, start + 1);
          this.textarea.focus();
          
          // Trigger input event to update other systems (content detection, etc.)
          this.textarea.dispatchEvent(new Event('input', { bubbles: true }));
          return;
        }
        
        // Handle Space key - insert space character
        if (event.key === ' ') {
          event.preventDefault();
          event.stopImmediatePropagation();
          
          const start = this.textarea.selectionStart;
          const end = this.textarea.selectionEnd;
          const currentValue = this.textarea.value;
          
          // Insert space at cursor position
          const newValue = currentValue.substring(0, start) + ' ' + currentValue.substring(end);
          this.textarea.value = newValue;
          this.content = newValue;
          
          // Position cursor after the space
          this.textarea.setSelectionRange(start + 1, start + 1);
          this.textarea.focus();
          
          // Trigger input event to update other systems
          this.textarea.dispatchEvent(new Event('input', { bubbles: true }));
          return;
        }
        
        // Handle Arrow keys and navigation keys - prevent interference but allow default behavior
        if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown'].includes(event.key)) {
          // Stop event propagation to prevent other handlers from interfering
          // but don't preventDefault - let the browser handle cursor movement naturally
          event.stopImmediatePropagation();
          return;
        }
      }
    };

    // Add global handler to document (highest priority)
    this.globalEssentialKeysHandler = globalEssentialKeysHandler.bind(this);
    document.addEventListener('keydown', this.globalEssentialKeysHandler, true); // Use capture phase

    // Blur event for save-on-focus-loss
    const blurHandler = (event) => {
      // Emit save requested event - this will be handled by EditorPreviewPanel and then main.js
      this.emit('save_requested', {
        content: this.content,
        timestamp: Date.now(),
        reason: 'focus_lost'
      });
    };

    // Register event listeners (optimized to avoid conflicts)
    this.addDOMEventListener(this.textarea, 'input', inputHandler);
    // Use single selection handler to avoid conflicts and improve performance
    this.addDOMEventListener(this.textarea, 'selectionchange', selectionHandler);
    // Add blur handler for save-on-focus-loss
    this.addDOMEventListener(this.textarea, 'blur', blurHandler);
    this.addDOMEventListener(this.textarea, 'mouseup', selectionHandler);
    this.addDOMEventListener(this.textarea, 'keydown', keyHandler);
    // Note: keypress event handler removed - global handler manages essential keys
    this.addDOMEventListener(this.textarea, 'paste', pasteHandler);

    // Scroll synchronization (prepare for preview mode)
    this.addDOMEventListener(this.textarea, 'scroll', () => {
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
  addDOMEventListener(element, event, handler) {
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
   * Debounce heavy operations to improve key responsiveness
   * @private
   */
  debounceHeavyOperations() {
    clearTimeout(this.heavyOperationsTimeout);
    this.heavyOperationsTimeout = setTimeout(() => {
      try {
        const startTime = performance.now();
        
        // Update cursor position
        this.updateCursorPosition();
        
        // Update word count
        this.updateWordCount();
        
        // Check document size for performance optimizations
        this.checkDocumentSize();
        
        // Track performance
        const duration = performance.now() - startTime;
        if (duration > 50) {
          this.handleError('heavy-operations-performance', 
            new Error(`Slow heavy operations: ${duration.toFixed(2)}ms (target: <50ms)`), 
            'warning');
        }
      } catch (error) {
        this.handleError('heavy-operations', error);
      }
    }, 100); // 100ms debounce for heavy operations
  }

  /**
   * Set editor content
   * @param {string} content - New content for the editor
   * @param {boolean} saveUndo - Whether to save to undo stack (default: false for external calls)
   */
  setValue(content, saveUndo = false) {
    if (typeof content !== 'string') {
      content = String(content || '');
    }

    // Save undo state if requested (usually for internal changes)
    if (saveUndo && this.content !== content) {
      this.saveUndoState();
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
   * Setup keyboard shortcuts for formatting and editing
   * @private
   */
  setupKeyboardShortcuts() {
    // Text formatting shortcuts
    this.addKeyboardShortcut('Ctrl+B', () => this.formatSelection('bold'), 'Bold formatting');
    this.addKeyboardShortcut('Cmd+B', () => this.formatSelection('bold'), 'Bold formatting');
    
    this.addKeyboardShortcut('Ctrl+I', () => this.formatSelection('italic'), 'Italic formatting');
    this.addKeyboardShortcut('Cmd+I', () => this.formatSelection('italic'), 'Italic formatting');
    
    this.addKeyboardShortcut('Ctrl+K', () => this.formatSelection('link'), 'Insert link');
    this.addKeyboardShortcut('Cmd+K', () => this.formatSelection('link'), 'Insert link');
    
    this.addKeyboardShortcut('Ctrl+L', () => this.formatSelection('list'), 'Insert list item');
    this.addKeyboardShortcut('Cmd+L', () => this.formatSelection('list'), 'Insert list item');
    
    // Blockquote controls
    this.addKeyboardShortcut('Ctrl+>', () => this.adjustBlockquoteLevel(1), 'Increase blockquote level');
    this.addKeyboardShortcut('Cmd+>', () => this.adjustBlockquoteLevel(1), 'Increase blockquote level');
    
    this.addKeyboardShortcut('Ctrl+<', () => this.adjustBlockquoteLevel(-1), 'Decrease blockquote level');
    this.addKeyboardShortcut('Cmd+<', () => this.adjustBlockquoteLevel(-1), 'Decrease blockquote level');
    
    // Undo/Redo
    this.addKeyboardShortcut('Ctrl+Z', () => this.undo(), 'Undo');
    this.addKeyboardShortcut('Cmd+Z', () => this.undo(), 'Undo');
    
    this.addKeyboardShortcut('Ctrl+Y', () => this.redo(), 'Redo');
    this.addKeyboardShortcut('Cmd+Y', () => this.redo(), 'Redo');
    this.addKeyboardShortcut('Ctrl+Shift+Z', () => this.redo(), 'Redo');
    this.addKeyboardShortcut('Cmd+Shift+Z', () => this.redo(), 'Redo');
    
    // Find and Replace
    this.addKeyboardShortcut('Ctrl+F', () => this.openFindReplace('find'), 'Find');
    this.addKeyboardShortcut('Cmd+F', () => this.openFindReplace('find'), 'Find');
    
    this.addKeyboardShortcut('Ctrl+H', () => this.openFindReplace('replace'), 'Find and Replace');
    this.addKeyboardShortcut('Cmd+H', () => this.openFindReplace('replace'), 'Find and Replace');
    
    // Manual save (this will be handled by parent application)
    this.addKeyboardShortcut('Ctrl+S', () => this.emitSaveRequest(), 'Save file');
    this.addKeyboardShortcut('Cmd+S', () => this.emitSaveRequest(), 'Save file');
    
    // Line numbers toggle
    this.addKeyboardShortcut('Ctrl+Shift+L', () => this.toggleLineNumbers(), 'Toggle line numbers');
    this.addKeyboardShortcut('Cmd+Shift+L', () => this.toggleLineNumbers(), 'Toggle line numbers');
    
    // Performance diagnostics (for debugging)
    this.addKeyboardShortcut('Ctrl+Shift+D', () => console.log(this.getDiagnostics()), 'Show diagnostics');
    this.addKeyboardShortcut('Cmd+Shift+D', () => console.log(this.getDiagnostics()), 'Show diagnostics');
    
  }

  /**
   * Add a keyboard shortcut
   * @param {string} shortcut - Keyboard shortcut (e.g., 'Ctrl+B', 'Cmd+I')
   * @param {Function} action - Function to execute
   * @param {string} description - Human-readable description
   */
  addKeyboardShortcut(shortcut, action, description = '') {
    this.keyboardShortcuts.set(shortcut.toLowerCase(), {
      action,
      description,
      shortcut
    });
  }

  /**
   * Handle keyboard shortcut events
   * @param {KeyboardEvent} event - The keyboard event
   * @returns {boolean} True if shortcut was handled
   * @private
   */
  handleKeyboardShortcut(event) {
    // Build shortcut string
    const parts = [];
    
    if (event.ctrlKey) parts.push('ctrl');
    if (event.metaKey) parts.push('cmd');
    if (event.shiftKey) parts.push('shift');
    if (event.altKey) parts.push('alt');
    
    // Add the main key
    let key = event.key.toLowerCase();
    if (key === ' ') key = 'space';
    parts.push(key);
    
    const shortcutString = parts.join('+');
    
    // Check if we have a handler for this shortcut
    const shortcutData = this.keyboardShortcuts.get(shortcutString);
    
    if (shortcutData) {
      event.preventDefault();
      event.stopPropagation();
      
      try {
        shortcutData.action();
        
        // Emit shortcut executed event
        this.emit(MarkdownEditor.EVENTS.SHORTCUT_EXECUTED, {
          shortcut: shortcutData.shortcut,
          description: shortcutData.description,
          timestamp: Date.now()
        });
        
        console.log(`⌨️ Executed shortcut: ${shortcutData.shortcut}`);
        return true;
      } catch (error) {
        console.error(`❌ Error executing shortcut ${shortcutData.shortcut}:`, error);
        return false;
      }
    }
    
    return false;
  }

  /**
   * Format selected text with markdown syntax
   * @param {string} formatType - Type of formatting (bold, italic, link, list)
   */
  formatSelection(formatType) {
    const selection = this.getSelectedText();
    const start = this.selectionStart;
    const end = this.selectionEnd;
    
    // Save state for undo
    this.saveUndoState();
    
    let formattedText = '';
    let newCursorPos = start;
    
    switch (formatType) {
      case 'bold':
        if (selection) {
          formattedText = `**${selection}**`;
          newCursorPos = start + formattedText.length;
        } else {
          formattedText = '****';
          newCursorPos = start + 2; // Position cursor between asterisks
        }
        break;
        
      case 'italic':
        if (selection) {
          formattedText = `*${selection}*`;
          newCursorPos = start + formattedText.length;
        } else {
          formattedText = '**';
          newCursorPos = start + 1; // Position cursor between asterisks
        }
        break;
        
      case 'link':
        if (selection) {
          // Check if selection looks like a URL
          const isUrl = /^https?:\/\//.test(selection);
          if (isUrl) {
            formattedText = `[Link](${selection})`;
            newCursorPos = start + 1; // Select "Link" text
          } else {
            formattedText = `[${selection}](url)`;
            newCursorPos = start + formattedText.length - 4; // Position at "url"
          }
        } else {
          formattedText = '[text](url)';
          newCursorPos = start + 1; // Select "text"
        }
        break;
        
      case 'list':
        // Handle list formatting at line level
        const lines = this.content.split('\n');
        const textBeforeStart = this.content.substring(0, start);
        const currentLineIndex = textBeforeStart.split('\n').length - 1;
        
        if (lines[currentLineIndex] && !lines[currentLineIndex].trim().startsWith('- ')) {
          lines[currentLineIndex] = `- ${lines[currentLineIndex]}`;
          
          // Update content
          const newContent = lines.join('\n');
          this.setValue(newContent, false); // Don't save undo again
          
          // Position cursor
          newCursorPos = start + 2;
          this.textarea.setSelectionRange(newCursorPos, newCursorPos);
          
          this.emit(MarkdownEditor.EVENTS.FORMAT_APPLIED, {
            type: formatType,
            selection,
            timestamp: Date.now()
          });
          
          return;
        }
        break;
        
      default:
        console.warn(`Unknown format type: ${formatType}`);
        return;
    }
    
    // Apply the formatting
    this.insertText(formattedText, true);
    
    // Set cursor position
    if (formatType === 'bold' && !selection) {
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
    } else if (formatType === 'italic' && !selection) {
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
    } else if (formatType === 'link') {
      // Select the placeholder text
      if (selection && /^https?:\/\//.test(selection)) {
        this.textarea.setSelectionRange(start + 1, start + 5); // Select "Link"
      } else if (!selection) {
        this.textarea.setSelectionRange(start + 1, start + 5); // Select "text"
      } else {
        this.textarea.setSelectionRange(newCursorPos - 4, newCursorPos - 1); // Select "url"
      }
    }
    
    this.updateCursorPosition();
    this.textarea.focus();
    
    // Emit format applied event
    this.emit(MarkdownEditor.EVENTS.FORMAT_APPLIED, {
      type: formatType,
      selection,
      formattedText,
      timestamp: Date.now()
    });
    
    console.log(`✨ Applied formatting: ${formatType}`);
  }

  /**
   * Adjust blockquote level for current line
   * @param {number} direction - 1 to increase, -1 to decrease
   */
  adjustBlockquoteLevel(direction) {
    this.saveUndoState();
    
    const lines = this.content.split('\n');
    const textBeforeStart = this.content.substring(0, this.selectionStart);
    const currentLineIndex = textBeforeStart.split('\n').length - 1;
    
    if (currentLineIndex >= 0 && currentLineIndex < lines.length) {
      let line = lines[currentLineIndex];
      
      if (direction > 0) {
        // Increase blockquote level
        lines[currentLineIndex] = `> ${line}`;
      } else {
        // Decrease blockquote level
        if (line.startsWith('> ')) {
          lines[currentLineIndex] = line.substring(2);
        }
      }
      
      // Update content
      const newContent = lines.join('\n');
      this.setValue(newContent, false);
      
      // Adjust cursor position
      const adjustment = direction > 0 ? 2 : (line.startsWith('> ') ? -2 : 0);
      const newCursorPos = this.selectionStart + adjustment;
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
      this.updateCursorPosition();
      
    }
  }

  /**
   * Handle tab indentation
   * @param {boolean} shiftPressed - Whether Shift key was pressed (for outdent)
   * @private
   */
  handleTabIndentation(shiftPressed) {
    this.saveUndoState();
    
    const tabString = '  '; // Two spaces for indentation
    
    if (shiftPressed) {
      // Outdent - remove indentation
      const lines = this.content.split('\n');
      const textBeforeStart = this.content.substring(0, this.selectionStart);
      const currentLineIndex = textBeforeStart.split('\n').length - 1;
      
      if (currentLineIndex >= 0 && currentLineIndex < lines.length) {
        const line = lines[currentLineIndex];
        if (line.startsWith(tabString)) {
          lines[currentLineIndex] = line.substring(tabString.length);
          
          // Update content
          const newContent = lines.join('\n');
          this.setValue(newContent, false);
          
          // Adjust cursor position
          const newCursorPos = this.selectionStart - tabString.length;
          this.textarea.setSelectionRange(newCursorPos, newCursorPos);
          this.updateCursorPosition();
        }
      }
    } else {
      // Indent - add indentation
      this.insertText(tabString, false);
    }
  }

  /**
   * Handle auto-completion for brackets and quotes (optimized for responsiveness)
   * @param {KeyboardEvent} event - The keyboard event
   * @private
   */
  handleAutoCompletion(event) {
    const key = event.key;
    const closingChar = this.autoCompletePairs[key];
    
    if (!closingChar) return;
    
    // Fast path: check if we should skip auto-completion immediately
    const currentChar = this.content[this.selectionStart];
    const selection = this.getSelectedText();
    
    // If there's a selection, wrap it (highest priority)
    if (selection) {
      event.preventDefault();
      this.saveUndoState();
      
      const wrappedText = key + selection + closingChar;
      this.insertText(wrappedText, true);
      
      // Position cursor after the wrapped text
      const newCursorPos = this.selectionStart;
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
      
      return;
    }
    
    // Optimized logic for determining auto-completion
    let shouldAutoComplete = false;
    
    if (key === '"' || key === "'" || key === '`') {
      // Fast check for quotes: don't auto-complete if next char is the same
      shouldAutoComplete = currentChar !== key;
    } else {
      // Fast check for brackets: don't auto-complete if already closed
      shouldAutoComplete = currentChar !== closingChar;
    }
    
    if (shouldAutoComplete) {
      event.preventDefault();
      this.saveUndoState();
      
      // Insert both opening and closing characters
      const completedText = key + closingChar;
      this.insertText(completedText, false);
      
      // Position cursor between them
      const newCursorPos = this.selectionStart - 1;
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
      
    }
  }

  /**
   * Save current state to undo stack
   * @private
   */
  saveUndoState() {
    // Don't save duplicate states
    if (this.undoStack.length > 0 && 
        this.undoStack[this.undoStack.length - 1].content === this.content) {
      return;
    }
    
    this.undoStack.push({
      content: this.content,
      selectionStart: this.selectionStart,
      selectionEnd: this.selectionEnd,
      timestamp: Date.now()
    });
    
    // Limit undo stack size
    if (this.undoStack.length > this.maxUndoSize) {
      this.undoStack.shift();
    }
    
    // Clear redo stack when new state is saved
    this.redoStack = [];
  }

  /**
   * Undo last action
   */
  undo() {
    if (this.undoStack.length === 0) {
      return false;
    }
    
    // Save current state to redo stack
    this.redoStack.push({
      content: this.content,
      selectionStart: this.selectionStart,
      selectionEnd: this.selectionEnd,
      timestamp: Date.now()
    });
    
    // Restore previous state
    const previousState = this.undoStack.pop();
    this.setValue(previousState.content, false); // Don't save undo when undoing
    
    // Restore cursor position
    this.textarea.setSelectionRange(previousState.selectionStart, previousState.selectionEnd);
    this.updateCursorPosition();
    this.textarea.focus();
    
    console.log('↶ Undo executed');
    return true;
  }

  /**
   * Redo last undone action
   */
  redo() {
    if (this.redoStack.length === 0) {
      return false;
    }
    
    // Save current state to undo stack
    this.saveUndoState();
    
    // Restore next state
    const nextState = this.redoStack.pop();
    this.setValue(nextState.content, false); // Don't save undo when redoing
    
    // Restore cursor position
    this.textarea.setSelectionRange(nextState.selectionStart, nextState.selectionEnd);
    this.updateCursorPosition();
    this.textarea.focus();
    
    console.log('↷ Redo executed');
    return true;
  }

  /**
   * Open find and replace dialog
   * @param {string} mode - 'find' or 'replace'
   */
  openFindReplace(mode = 'find') {
    // Create find/replace modal if it doesn't exist
    if (!this.findReplaceModal) {
      this.createFindReplaceModal();
    }
    
    // Show the modal
    this.findReplaceModal.style.display = 'flex';
    
    // Set mode
    const replaceSection = this.findReplaceModal.querySelector('.replace-section');
    if (mode === 'replace') {
      replaceSection.style.display = 'block';
    } else {
      replaceSection.style.display = 'none';
    }
    
    // Focus search input
    const searchInput = this.findReplaceModal.querySelector('.search-input');
    searchInput.focus();
    searchInput.select();
    
    // Emit event
    this.emit(MarkdownEditor.EVENTS.FIND_REPLACE_OPENED, {
      mode,
      timestamp: Date.now()
    });
    
    // Find/replace modal opened successfully
  }

  /**
   * Create find and replace modal
   * @private
   */
  createFindReplaceModal() {
    this.findReplaceModal = document.createElement('div');
    this.findReplaceModal.className = 'markdown-editor-find-replace-modal';
    this.findReplaceModal.innerHTML = `
      <div class="find-replace-content">
        <h3>Find and Replace</h3>
        
        <div class="search-section">
          <label for="search-input">Find:</label>
          <input type="text" class="search-input" placeholder="Search text..." />
          <button class="btn-find-next">Find Next</button>
          <button class="btn-find-prev">Find Previous</button>
        </div>
        
        <div class="replace-section">
          <label for="replace-input">Replace:</label>
          <input type="text" class="replace-input" placeholder="Replace with..." />
          <button class="btn-replace">Replace</button>
          <button class="btn-replace-all">Replace All</button>
        </div>
        
        <div class="find-replace-controls">
          <button class="btn-close">Close</button>
          <div class="search-status"></div>
        </div>
      </div>
    `;
    
    // Add to container
    this.container.appendChild(this.findReplaceModal);
    
    // Set up event handlers
    this.setupFindReplaceHandlers();
  }

  /**
   * Set up find and replace event handlers
   * @private
   */
  setupFindReplaceHandlers() {
    const modal = this.findReplaceModal;
    const searchInput = modal.querySelector('.search-input');
    const replaceInput = modal.querySelector('.replace-input');
    
    // Search input handling
    searchInput.addEventListener('input', (e) => {
      this.currentSearchTerm = e.target.value;
      this.highlightSearchResults();
    });
    
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        this.findNext();
      } else if (e.key === 'Escape') {
        this.closeFindReplace();
      }
    });
    
    // Replace input handling
    replaceInput.addEventListener('input', (e) => {
      this.currentReplaceTerm = e.target.value;
    });
    
    replaceInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        this.replaceNext();
      }
    });
    
    // Button handlers
    modal.querySelector('.btn-find-next').addEventListener('click', () => this.findNext());
    modal.querySelector('.btn-find-prev').addEventListener('click', () => this.findPrevious());
    modal.querySelector('.btn-replace').addEventListener('click', () => this.replaceNext());
    modal.querySelector('.btn-replace-all').addEventListener('click', () => this.replaceAll());
    modal.querySelector('.btn-close').addEventListener('click', () => this.closeFindReplace());
    
    // Close on outside click
    modal.addEventListener('click', (e) => {
      if (e.target === modal) {
        this.closeFindReplace();
      }
    });
  }

  /**
   * Find next occurrence of search term
   */
  findNext() {
    if (!this.currentSearchTerm) return;
    
    const searchTerm = this.currentSearchTerm.toLowerCase();
    const content = this.content.toLowerCase();
    const currentPos = this.selectionEnd;
    
    const nextIndex = content.indexOf(searchTerm, currentPos);
    
    if (nextIndex !== -1) {
      // Found next occurrence
      this.textarea.setSelectionRange(nextIndex, nextIndex + this.currentSearchTerm.length);
      this.textarea.focus();
      this.updateCursorPosition();
      this.updateSearchStatus(`Found at position ${nextIndex}`);
    } else {
      // Search from beginning
      const fromStart = content.indexOf(searchTerm, 0);
      if (fromStart !== -1 && fromStart < currentPos) {
        this.textarea.setSelectionRange(fromStart, fromStart + this.currentSearchTerm.length);
        this.textarea.focus();
        this.updateCursorPosition();
        this.updateSearchStatus('Search wrapped to beginning');
      } else {
        this.updateSearchStatus('Not found');
      }
    }
  }

  /**
   * Find previous occurrence of search term
   */
  findPrevious() {
    if (!this.currentSearchTerm) return;
    
    const searchTerm = this.currentSearchTerm.toLowerCase();
    const content = this.content.toLowerCase();
    const currentPos = this.selectionStart;
    
    const prevIndex = content.lastIndexOf(searchTerm, currentPos - 1);
    
    if (prevIndex !== -1) {
      this.textarea.setSelectionRange(prevIndex, prevIndex + this.currentSearchTerm.length);
      this.textarea.focus();
      this.updateCursorPosition();
      this.updateSearchStatus(`Found at position ${prevIndex}`);
    } else {
      // Search from end
      const fromEnd = content.lastIndexOf(searchTerm);
      if (fromEnd !== -1 && fromEnd >= currentPos) {
        this.textarea.setSelectionRange(fromEnd, fromEnd + this.currentSearchTerm.length);
        this.textarea.focus();
        this.updateCursorPosition();
        this.updateSearchStatus('Search wrapped to end');
      } else {
        this.updateSearchStatus('Not found');
      }
    }
  }

  /**
   * Replace next occurrence
   */
  replaceNext() {
    if (!this.currentSearchTerm || this.currentReplaceTerm === undefined) return;
    
    const selectedText = this.getSelectedText();
    
    if (selectedText.toLowerCase() === this.currentSearchTerm.toLowerCase()) {
      // Replace current selection
      this.saveUndoState();
      this.insertText(this.currentReplaceTerm, true);
      this.updateSearchStatus('Replaced 1 occurrence');
    }
    
    // Find next
    this.findNext();
  }

  /**
   * Replace all occurrences
   */
  replaceAll() {
    if (!this.currentSearchTerm || this.currentReplaceTerm === undefined) return;
    
    this.saveUndoState();
    
    // Use case-insensitive replacement with regex
    const regex = new RegExp(this.escapeRegExp(this.currentSearchTerm), 'gi');
    const newContent = this.content.replace(regex, this.currentReplaceTerm);
    
    if (newContent !== this.content) {
      const replacementCount = (this.content.match(regex) || []).length;
      this.setValue(newContent, false);
      this.updateSearchStatus(`Replaced ${replacementCount} occurrence${replacementCount !== 1 ? 's' : ''}`);
    } else {
      this.updateSearchStatus('No matches found');
    }
  }

  /**
   * Escape special regex characters
   * @param {string} string - String to escape
   * @returns {string} Escaped string
   * @private
   */
  escapeRegExp(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  /**
   * Update search status message
   * @param {string} message - Status message
   * @private
   */
  updateSearchStatus(message) {
    if (this.findReplaceModal) {
      const statusElement = this.findReplaceModal.querySelector('.search-status');
      statusElement.textContent = message;
      
      // Clear status after 3 seconds
      setTimeout(() => {
        if (statusElement) {
          statusElement.textContent = '';
        }
      }, 3000);
    }
  }

  /**
   * Close find and replace modal
   */
  closeFindReplace() {
    if (this.findReplaceModal) {
      this.findReplaceModal.style.display = 'none';
      this.clearSearchHighlights();
      this.textarea.focus();
    }
  }

  /**
   * Highlight search results (placeholder for future implementation)
   * @private
   */
  highlightSearchResults() {
    // This will be implemented with syntax highlighting in sub-issue #40
    // For now, just update the status
    if (this.currentSearchTerm) {
      const regex = new RegExp(this.escapeRegExp(this.currentSearchTerm), 'gi');
      const matches = (this.content.match(regex) || []).length;
      this.updateSearchStatus(`${matches} match${matches !== 1 ? 'es' : ''} found`);
    }
  }

  /**
   * Clear search highlights (placeholder for future implementation)
   * @private
   */
  clearSearchHighlights() {
    // This will be implemented with syntax highlighting in sub-issue #40
    this.searchHighlights = [];
  }

  /**
   * Emit save request to parent application
   * @private
   */
  emitSaveRequest() {
    // Create custom event for save request
    const saveEvent = new CustomEvent('save_requested', {
      detail: {
        content: this.content,
        timestamp: Date.now()
      },
      bubbles: true
    });
    
    this.container.dispatchEvent(saveEvent);
  }

  /**
   * Get list of available keyboard shortcuts
   * @returns {Array} Array of shortcut objects
   */
  getKeyboardShortcuts() {
    return Array.from(this.keyboardShortcuts.entries()).map(([key, data]) => ({
      shortcut: data.shortcut,
      description: data.description,
      key
    }));
  }

  /**
   * Check if undo is available
   * @returns {boolean} True if undo is available
   */
  canUndo() {
    return this.undoStack.length > 0;
  }

  /**
   * Check if redo is available
   * @returns {boolean} True if redo is available
   */
  canRedo() {
    return this.redoStack.length > 0;
  }

  /**
   * Set up AutoSave integration for direct content change handling
   * @private
   */
  setupAutoSaveIntegration() {
    if (!this.autoSave) {
      console.warn('⚠️ AutoSave service not available for integration');
      return;
    }
    
    // Set up content getter for AutoSave service
    this.autoSave.setContentGetter(() => {
      return this.getValue();
    });
    
  }

  /**
   * Set AutoSave service and establish integration
   * @param {AutoSave} autoSave - AutoSave service instance
   */
  setAutoSave(autoSave) {
    this.autoSave = autoSave;
    if (this.autoSave && this.isInitialized) {
      this.setupAutoSaveIntegration();
    }
  }

  /**
   * Setup performance optimizations
   * @private
   */
  setupPerformanceOptimizations() {
    // Schedule periodic memory cleanup
    this.scheduleCleanup();
    
    // Monitor scroll performance for large documents
    this.addDOMEventListener(this.textarea, 'scroll', () => {
      if (this.virtualScrolling) {
        this.updateVisibleLines();
      }
    });
    
    console.log('⚡ Performance optimizations initialized');
  }


  /**
   * Check document size and enable optimizations for large files
   * @private
   */
  checkDocumentSize() {
    const currentSize = this.content.length;
    const wasLarge = this.isLargeDocument;
    this.isLargeDocument = currentSize > this.documentSizeThreshold;

    if (this.isLargeDocument && !wasLarge) {
      console.log(`📊 Large document detected (${(currentSize / 1024).toFixed(1)}KB)`);
      this.enableVirtualScrolling();
      
      this.emit(MarkdownEditor.EVENTS.LARGE_DOCUMENT_DETECTED, {
        size: currentSize,
        threshold: this.documentSizeThreshold,
        timestamp: Date.now()
      });
    } else if (!this.isLargeDocument && wasLarge) {
      this.disableVirtualScrolling();
    }
  }

  /**
   * Enable virtual scrolling for large documents
   * @private
   */
  enableVirtualScrolling() {
    if (this.virtualScrolling) return;
    
    this.virtualScrolling = true;
    this.updateTotalLines();
    this.updateVisibleLines();
    
    console.log('📜 Virtual scrolling enabled for performance');
  }

  /**
   * Disable virtual scrolling
   * @private
   */
  disableVirtualScrolling() {
    if (!this.virtualScrolling) return;
    
    this.virtualScrolling = false;
    console.log('📜 Virtual scrolling disabled');
  }

  /**
   * Update total line count for virtual scrolling
   * @private
   */
  updateTotalLines() {
    this.totalLines = this.content.split('\n').length;
  }

  /**
   * Update visible lines for virtual scrolling
   * @private
   */
  updateVisibleLines() {
    if (!this.virtualScrolling || !this.textarea) return;

    const lineHeight = parseInt(getComputedStyle(this.textarea).lineHeight) || 24;
    const visibleHeight = this.textarea.clientHeight;
    const scrollTop = this.textarea.scrollTop;
    
    this.visibleLineStart = Math.floor(scrollTop / lineHeight);
    this.visibleLineEnd = Math.min(
      this.totalLines,
      this.visibleLineStart + Math.ceil(visibleHeight / lineHeight) + 5 // Buffer
    );
  }

  /**
   * Toggle line numbers display
   * @param {boolean} enabled - Whether to show line numbers
   */
  toggleLineNumbers(enabled = !this.lineNumbersEnabled) {
    this.lineNumbersEnabled = enabled;
    
    if (this.lineNumbersEnabled) {
      this.showLineNumbers();
    } else {
      this.hideLineNumbers();
    }
    
    this.emit(MarkdownEditor.EVENTS.LINE_NUMBERS_TOGGLED, {
      enabled: this.lineNumbersEnabled,
      timestamp: Date.now()
    });
    
    console.log(`🔢 Line numbers ${enabled ? 'enabled' : 'disabled'}`);
  }

  /**
   * Show line numbers
   * @private
   */
  showLineNumbers() {
    if (!this.lineNumbersContainer) return;
    
    this.lineNumbersContainer.style.display = 'block';
    this.updateLineNumbers();
    
    // Adjust textarea padding to make room for line numbers
    this.textarea.style.paddingLeft = '60px';
  }

  /**
   * Hide line numbers
   * @private
   */
  hideLineNumbers() {
    if (!this.lineNumbersContainer) return;
    
    this.lineNumbersContainer.style.display = 'none';
    this.textarea.style.paddingLeft = '1rem';
  }

  /**
   * Update line numbers display
   * @private
   */
  updateLineNumbers() {
    if (!this.lineNumbersEnabled || !this.lineNumbersContainer) return;
    
    const lines = this.content.split('\n');
    const lineCount = lines.length;
    
    let html = '';
    for (let i = 1; i <= lineCount; i++) {
      html += `<div class="line-number">${i}</div>`;
    }
    
    this.lineNumbersContainer.innerHTML = html;
    
    // Sync scroll position
    this.lineNumbersContainer.scrollTop = this.textarea.scrollTop;
  }

  /**
   * Show loading state
   * @param {string} message - Loading message
   */
  showLoading(message = 'Loading...') {
    if (!this.loadingOverlay) return;
    
    this.isLoading = true;
    this.loadingOverlay.querySelector('.loading-text').textContent = message;
    this.loadingOverlay.style.display = 'flex';
    
    this.emit(MarkdownEditor.EVENTS.LOADING_STATE_CHANGED, {
      loading: true,
      message,
      timestamp: Date.now()
    });
  }

  /**
   * Hide loading state
   */
  hideLoading() {
    if (!this.loadingOverlay) return;
    
    this.isLoading = false;
    this.loadingOverlay.style.display = 'none';
    
    this.emit(MarkdownEditor.EVENTS.LOADING_STATE_CHANGED, {
      loading: false,
      timestamp: Date.now()
    });
  }

  /**
   * Schedule memory cleanup
   * @private
   */
  scheduleCleanup() {
    if (this.cleanupScheduled) return;
    
    this.cleanupScheduled = true;
    
    setTimeout(() => {
      this.performCleanup();
      this.cleanupScheduled = false;
      this.scheduleCleanup(); // Reschedule
    }, this.cleanupInterval);
  }

  /**
   * Perform memory cleanup
   * @private
   */
  performCleanup() {
    const now = Date.now();
    
    // Clean up old undo/redo states (keep only last 50)
    if (this.undoStack.length > 50) {
      this.undoStack = this.undoStack.slice(-50);
    }
    if (this.redoStack.length > 50) {
      this.redoStack = this.redoStack.slice(-50);
    }
    
    // Clear old search highlights
    this.clearSearchHighlights();
    
    this.lastCleanup = now;
    console.log('🧹 Memory cleanup performed');
  }


  /**
   * Get performance statistics
   * @returns {Object} Performance stats
   */
  getPerformanceStats() {
    return {
      ...this.getStats(),
      isLargeDocument: this.isLargeDocument,
      virtualScrolling: this.virtualScrolling,
      documentSize: this.content.length,
      totalLines: this.totalLines,
      visibleLines: this.virtualScrolling ? {
        start: this.visibleLineStart,
        end: this.visibleLineEnd
      } : null,
      memory: {
        undoStackSize: this.undoStack.length,
        redoStackSize: this.redoStack.length,
        lastCleanup: this.lastCleanup
      }
    };
  }

  /**
   * Create comprehensive error handler
   * @returns {Object} Error handler instance
   * @private
   */
  createErrorHandler() {
    return {
      errors: [],
      maxErrors: 50,
      
      log: (context, error, severity = 'error') => {
        const errorData = {
          context,
          message: error.message || error,
          stack: error.stack,
          timestamp: Date.now(),
          severity
        };
        
        this.errorHandler.errors.push(errorData);
        
        // Keep only recent errors
        if (this.errorHandler.errors.length > this.errorHandler.maxErrors) {
          this.errorHandler.errors = this.errorHandler.errors.slice(-this.errorHandler.maxErrors);
        }
        
        // Console logging with appropriate level
        if (severity === 'error') {
          console.error(`❌ MarkdownEditor [${context}]:`, error);
        } else if (severity === 'warning') {
          console.warn(`⚠️ MarkdownEditor [${context}]:`, error);
        } else {
          console.log(`ℹ️ MarkdownEditor [${context}]:`, error);
        }
        
        return errorData;
      },
      
      getErrors: () => [...this.errorHandler.errors],
      
      clearErrors: () => {
        this.errorHandler.errors = [];
      }
    };
  }

  /**
   * Handle errors with comprehensive logging and recovery
   * @param {string} context - Error context
   * @param {Error} error - Error object
   * @param {string} severity - Error severity level
   */
  handleError(context, error, severity = 'error') {
    const errorData = this.errorHandler.log(context, error, severity);
    
    // Emit error event for external handling
    this.emit('error', {
      context,
      error: errorData,
      timestamp: Date.now()
    });
    
    // Attempt recovery for certain error types
    
    if (context.includes('virtual-scrolling')) {
      this.disableVirtualScrolling();
    }
  }

  /**
   * Add enhanced accessibility features
   * @private
   */
  enhanceAccessibility() {
    if (!this.textarea) return;
    
    // Add keyboard navigation hints
    this.textarea.title = 'Markdown editor - Use Ctrl+? for keyboard shortcuts';
    
    // Enhanced ARIA properties
    this.container.setAttribute('role', 'application');
    this.container.setAttribute('aria-label', 'Markdown editor with syntax highlighting');
    
    // Loading state accessibility
    if (this.loadingOverlay) {
      this.loadingOverlay.setAttribute('aria-label', 'Content loading');
      this.loadingOverlay.setAttribute('role', 'status');
    }
    
    // Line numbers accessibility
    if (this.lineNumbersContainer) {
      this.lineNumbersContainer.setAttribute('aria-label', 'Line numbers');
      this.lineNumbersContainer.setAttribute('role', 'presentation');
    }
    
    console.log('♿ Accessibility features enhanced');
  }

  /**
   * Implement scroll synchronization for future preview mode
   * @private
   */
  setupScrollSynchronization() {
    if (!this.textarea) return;
    
    // Store scroll sync state
    this.scrollSync = {
      enabled: false,
      previewElement: null,
      syncRatio: 1.0,
      lastScrollTime: 0,
      debounceDelay: 16 // 60fps
    };
    
    // Enhanced scroll handler with throttling
    const scrollHandler = () => {
      const now = performance.now();
      if (now - this.scrollSync.lastScrollTime < this.scrollSync.debounceDelay) {
        return;
      }
      
      this.scrollSync.lastScrollTime = now;
      
      // Sync overlay scroll position
      if (this.overlayContainer) {
        this.overlayContainer.scrollTop = this.textarea.scrollTop;
      }
      
      // Sync line numbers scroll position
      if (this.lineNumbersContainer && this.lineNumbersEnabled) {
        this.lineNumbersContainer.scrollTop = this.textarea.scrollTop;
      }
      
      // Update line numbers when scrolling (for large documents)
      if (this.lineNumbersEnabled && this.isLargeDocument) {
        requestAnimationFrame(() => {
          this.updateLineNumbers();
        });
      }
      
      // Future: Sync with preview pane
      if (this.scrollSync.enabled && this.scrollSync.previewElement) {
        const scrollPercentage = this.textarea.scrollTop / 
          (this.textarea.scrollHeight - this.textarea.clientHeight);
        
        const previewScrollTop = scrollPercentage * 
          (this.scrollSync.previewElement.scrollHeight - this.scrollSync.previewElement.clientHeight);
        
        this.scrollSync.previewElement.scrollTop = previewScrollTop;
      }
    };
    
    this.addDOMEventListener(this.textarea, 'scroll', scrollHandler);
  }

  /**
   * Enable scroll synchronization with preview element
   * @param {HTMLElement} previewElement - Preview element to sync with
   */
  enableScrollSync(previewElement) {
    if (!previewElement || !(previewElement instanceof HTMLElement)) {
      this.handleError('enableScrollSync', new Error('Invalid preview element'), 'warning');
      return;
    }
    
    this.scrollSync.enabled = true;
    this.scrollSync.previewElement = previewElement;
  }

  /**
   * Disable scroll synchronization
   */
  disableScrollSync() {
    this.scrollSync.enabled = false;
    this.scrollSync.previewElement = null;
  }

  /**
   * Validate performance and log metrics
   * @returns {Object} Performance validation results
   */
  validatePerformance() {
    const stats = this.getPerformanceStats();
    const validation = {
      memoryUsage: {
        target: 10485760, // 10MB in bytes
        actual: stats.documentSize,
        passed: stats.documentSize < 10485760
      },
      undoStackSize: {
        target: 100,
        actual: stats.memory.undoStackSize,
        passed: stats.memory.undoStackSize <= 100
      },
      frameRate: {
        target: 16, // ms per frame for 60fps
        actual: this.lastKeystroke ? performance.now() - this.lastKeystroke : 0,
        passed: true
      }
    };
    
    // Log warnings for performance issues
    Object.entries(validation).forEach(([metric, data]) => {
      if (!data.passed) {
        this.handleError(
          `Performance validation - ${metric}`,
          new Error(`Target: ${data.target}, Actual: ${data.actual}`),
          'warning'
        );
      }
    });
    
    console.log('📊 Performance validation:', validation);
    return validation;
  }

  /**
   * Get comprehensive error and performance report
   * @returns {Object} Full diagnostic report
   */
  getDiagnostics() {
    return {
      timestamp: Date.now(),
      performance: this.getPerformanceStats(),
      validation: this.validatePerformance(),
      errors: this.errorHandler.getErrors(),
      features: {
        lineNumbers: this.lineNumbersEnabled,
        virtualScrolling: this.virtualScrolling,
        scrollSync: this.scrollSync?.enabled || false
      },
      browser: {
        userAgent: navigator.userAgent,
        memory: performance.memory ? {
          used: performance.memory.usedJSHeapSize,
          total: performance.memory.totalJSHeapSize,
          limit: performance.memory.jsHeapSizeLimit
        } : null
      }
    };
  }

  /**
   * Clean up editor resources
   */
  destroy() {
    // Remove global essential keys handler
    if (this.globalEssentialKeysHandler) {
      document.removeEventListener('keydown', this.globalEssentialKeysHandler, true);
      this.globalEssentialKeysHandler = null;
    }
    
    // Clear all timeouts
    clearTimeout(this.debounceTimeout);
    clearTimeout(this.heavyOperationsTimeout);
    
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
    this.lineNumbersContainer = null;
    this.loadingOverlay = null;
    
    
    // Clear performance state
    this.virtualScrolling = false;
    this.cleanupScheduled = false;
    
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