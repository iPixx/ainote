// Import syntax highlighter
import SyntaxHighlighter from '../utils/syntax-highlighter.js';

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
    FIND_REPLACE_OPENED: 'find_replace_opened'
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

    // Initialize syntax highlighter
    this.syntaxHighlighter = new SyntaxHighlighter({
      debounceDelay: 300,
      maxLinesForFullHighlight: 1000,
      visibleLinesBuffer: 50,
      enablePerformanceLogging: true // Enable for debugging
    });

    // Syntax highlighting state
    this.highlightingEnabled = true;
    this.lastHighlightedContent = '';
    this.highlightingInProgress = false;
    
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
    this.setupKeyboardShortcuts();
    this.updateWordCount();
    this.isInitialized = true;
    
    console.log('✅ MarkdownEditor initialized with keyboard shortcuts');
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
      
      // Trigger syntax highlighting if enabled
      if (this.highlightingEnabled && this.content !== this.lastHighlightedContent) {
        this.updateSyntaxHighlighting();
      }
      
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

    // Key events for keyboard shortcuts and cursor movement
    const keyHandler = (event) => {
      // Track keystroke performance
      this.lastKeystroke = performance.now();
      
      // Handle keyboard shortcuts
      const shortcutHandled = this.handleKeyboardShortcut(event);
      
      // Handle auto-completion for brackets and quotes
      if (!shortcutHandled && !event.ctrlKey && !event.metaKey && !event.altKey) {
        this.handleAutoCompletion(event);
      }
      
      // Handle tab indentation
      if (event.key === 'Tab' && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        this.handleTabIndentation(event.shiftKey);
        return;
      }
      
      // Emit cursor moved event
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

    // Register event listeners
    this.addDOMEventListener(this.textarea, 'input', inputHandler);
    this.addDOMEventListener(this.textarea, 'selectionchange', selectionHandler);
    this.addDOMEventListener(this.textarea, 'select', selectionHandler);
    this.addDOMEventListener(this.textarea, 'keyup', selectionHandler);
    this.addDOMEventListener(this.textarea, 'mouseup', selectionHandler);
    this.addDOMEventListener(this.textarea, 'keydown', keyHandler);
    this.addDOMEventListener(this.textarea, 'paste', pasteHandler);

    // Scroll synchronization for syntax highlighting overlay
    this.addDOMEventListener(this.textarea, 'scroll', () => {
      // Sync overlay scroll position with textarea immediately
      if (this.overlayContainer) {
        this.overlayContainer.scrollTop = this.textarea.scrollTop;
        this.overlayContainer.scrollLeft = this.textarea.scrollLeft;
      }
      
      // Note: No need to re-highlight on scroll - viewport optimization is handled internally
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
      
      // Trigger syntax highlighting if content changed
      if (this.highlightingEnabled && this.content !== this.lastHighlightedContent) {
        this.updateSyntaxHighlighting();
      }
      
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
    
    console.log(`✅ Keyboard shortcuts configured: ${this.keyboardShortcuts.size} shortcuts`);
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
      
      console.log(`📝 Adjusted blockquote level: ${direction > 0 ? 'increased' : 'decreased'}`);
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
   * Handle auto-completion for brackets and quotes
   * @param {KeyboardEvent} event - The keyboard event
   * @private
   */
  handleAutoCompletion(event) {
    const key = event.key;
    const closingChar = this.autoCompletePairs[key];
    
    if (!closingChar) return;
    
    // Check if we should auto-complete
    const currentChar = this.content[this.selectionStart];
    const selection = this.getSelectedText();
    
    // If there's a selection, wrap it
    if (selection) {
      event.preventDefault();
      this.saveUndoState();
      
      const wrappedText = key + selection + closingChar;
      this.insertText(wrappedText, true);
      
      // Position cursor after the wrapped text
      const newCursorPos = this.selectionStart;
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
      this.updateCursorPosition();
      
      console.log(`🔗 Auto-wrapped selection with ${key}${closingChar}`);
      return;
    }
    
    // Determine if we should auto-complete based on context
    let shouldAutoComplete = false;
    
    // Simple logic: auto-complete unless we're about to create a duplicate pair
    // Exception: don't auto-complete quotes if we're in the middle of a word
    if (key === '"' || key === "'" || key === '`') {
      // For quotes, be more careful about context
      const prevChar = this.selectionStart > 0 ? this.content[this.selectionStart - 1] : '';
      const isInsideWord = /\w/.test(prevChar) && /\w/.test(currentChar || '');
      const isDuplicate = currentChar === key;
      
      shouldAutoComplete = !isInsideWord && !isDuplicate;
    } else {
      // For brackets: (,  [, {
      // Auto-complete unless the next character is already the closing bracket
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
      this.updateCursorPosition();
      
      console.log(`🔧 Auto-completed ${key} with ${closingChar}`);
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
      console.log('📝 Nothing to undo');
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
      console.log('📝 Nothing to redo');
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
    
    console.log(`🔍 Opened find/replace in ${mode} mode`);
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
   * Highlight search results with syntax highlighting integration
   * @private
   */
  highlightSearchResults() {
    if (this.currentSearchTerm) {
      this.updateSearchHighlighting(this.currentSearchTerm);
    } else {
      this.clearSearchHighlights();
    }
  }

  /**
   * Clear search highlights and refresh syntax highlighting
   * @private
   */
  clearSearchHighlights() {
    this.searchHighlights = [];
    
    // Refresh syntax highlighting to remove search highlights
    if (this.highlightingEnabled && this.content.trim()) {
      this.updateSyntaxHighlighting();
    }
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
    console.log('💾 Save request emitted');
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
   * Clean up editor resources
   */
  destroy() {
    // Clear debounce timeouts
    clearTimeout(this.debounceTimeout);
    clearTimeout(this.highlightUpdateTimeout);
    
    // Destroy syntax highlighter
    if (this.syntaxHighlighter) {
      this.syntaxHighlighter.destroy();
      this.syntaxHighlighter = null;
    }
    
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
    this.highlightingEnabled = false;
    this.lastHighlightedContent = '';
    this.highlightingInProgress = false;
    
    console.log('✅ MarkdownEditor destroyed');
  }

  /**
   * Update syntax highlighting for current content
   * @private
   */
  async updateSyntaxHighlighting() {
    if (!this.highlightingEnabled || this.highlightingInProgress || !this.syntaxHighlighter) {
      return;
    }

    // Skip if content hasn't changed
    if (this.content === this.lastHighlightedContent) {
      return;
    }

    try {
      this.highlightingInProgress = true;
      
      // Get viewport information for optimization
      const viewportInfo = this.getViewportInfo();
      
      // Highlight content with debouncing
      await this.syntaxHighlighter.highlightWithDebounce(
        this.content,
        this.overlayContainer,
        viewportInfo
      );
      
      // Mark overlay as active when highlighting is present
      if (this.content.trim()) {
        this.overlayContainer.classList.add('highlighting-active');
        this.overlayContainer.style.opacity = '0.95';
      } else {
        this.overlayContainer.classList.remove('highlighting-active');
        this.overlayContainer.style.opacity = '0';
      }
      
      this.lastHighlightedContent = this.content;
      
    } catch (error) {
      console.error('❌ Error updating syntax highlighting:', error);
      this.overlayContainer.classList.remove('highlighting-active');
      this.overlayContainer.style.opacity = '0';
    } finally {
      this.highlightingInProgress = false;
    }
  }

  /**
   * Debounced syntax highlighting update for scroll events
   * @private
   */
  debouncedHighlightUpdate() {
    clearTimeout(this.highlightUpdateTimeout);
    this.highlightUpdateTimeout = setTimeout(() => {
      if (this.highlightingEnabled && this.content.trim()) {
        this.updateSyntaxHighlighting();
      }
    }, 100); // Shorter delay for scroll-triggered updates
  }

  /**
   * Get viewport information for performance optimization
   * @returns {Object} Viewport information
   * @private
   */
  getViewportInfo() {
    if (!this.textarea) return null;

    try {
      const textareaRect = this.textarea.getBoundingClientRect();
      const lineHeight = parseFloat(getComputedStyle(this.textarea).lineHeight) || 20;
      const scrollTop = this.textarea.scrollTop;
      
      const firstVisibleLine = Math.floor(scrollTop / lineHeight);
      const visibleLines = Math.ceil(textareaRect.height / lineHeight);
      const lastVisibleLine = firstVisibleLine + visibleLines;

      return {
        firstVisibleLine,
        lastVisibleLine,
        lineHeight,
        scrollTop,
        scrollLeft: this.textarea.scrollLeft,
        visibleHeight: textareaRect.height,
        visibleWidth: textareaRect.width
      };
    } catch (error) {
      console.warn('⚠️ Could not get viewport info:', error);
      return null;
    }
  }

  /**
   * Toggle syntax highlighting on/off
   * @param {boolean} enabled - Whether to enable highlighting
   */
  toggleSyntaxHighlighting(enabled) {
    this.highlightingEnabled = enabled;
    
    if (enabled && this.content.trim()) {
      // Reset lastHighlightedContent to force re-highlighting
      this.lastHighlightedContent = null;
      this.updateSyntaxHighlighting();
    } else {
      this.overlayContainer.classList.remove('highlighting-active');
      this.overlayContainer.style.opacity = '0';
      this.overlayContainer.innerHTML = '';
    }
    
    console.log(`✨ Syntax highlighting ${enabled ? 'enabled' : 'disabled'}`);
  }

  /**
   * Update find/replace highlighting with syntax highlighting integration
   * @param {string} searchTerm - Term to search for
   * @private
   */
  updateSearchHighlighting(searchTerm) {
    if (!searchTerm || !this.highlightingEnabled) {
      this.clearSearchHighlights();
      return;
    }

    // This integrates with the existing highlightSearchResults method
    // The search highlighting is now handled by the CSS styles we added
    const regex = new RegExp(this.escapeRegExp(searchTerm), 'gi');
    const matches = (this.content.match(regex) || []).length;
    
    // Update search status
    this.updateSearchStatus(`${matches} match${matches !== 1 ? 'es' : ''} found`);
    
    // Re-highlight content to include search highlights
    if (matches > 0) {
      this.updateSyntaxHighlighting();
    }
  }

  /**
   * Get syntax highlighting performance statistics
   * @returns {Object} Performance statistics
   */
  getSyntaxHighlightingStats() {
    if (!this.syntaxHighlighter) {
      return { enabled: false };
    }

    return {
      enabled: this.highlightingEnabled,
      inProgress: this.highlightingInProgress,
      lastContent: this.lastHighlightedContent.length,
      ...this.syntaxHighlighter.getPerformanceStats()
    };
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
      lastKeystrokeTime: this.lastKeystroke,
      syntaxHighlighting: this.getSyntaxHighlightingStats()
    };
  }
}

// Export for ES6 module usage
export default MarkdownEditor;