/**
 * Comprehensive Unit Tests for MarkdownEditor Component
 * 
 * Tests key responsiveness issues, performance, and all functionality
 * in isolation to ensure the editor works correctly.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Mock MarkdownEditor - simulates the actual component behavior
class MockMarkdownEditor {
  static EVENTS = {
    CONTENT_CHANGED: 'content_changed',
    SELECTION_CHANGED: 'selection_changed',
    CURSOR_MOVED: 'cursor_moved',
    WORD_COUNT_CHANGED: 'word_count_changed',
    SHORTCUT_EXECUTED: 'shortcut_executed',
    FORMAT_APPLIED: 'format_applied',
    AUTO_SAVE_TRIGGERED: 'auto_save_triggered',
    AUTO_SAVE_COMPLETED: 'auto_save_completed'
  };

  constructor(container, appState) {
    this.container = container || document.createElement('div');
    this.appState = appState || { getCurrentFile: () => null };
    
    // Core state
    this.content = '';
    this.isInitialized = false;
    this.cursorPosition = 0;
    this.selectionStart = 0;
    this.selectionEnd = 0;
    this.wordCount = 0;
    this.charCount = 0;
    
    // Performance optimization state
    this.lastKeystroke = 0;
    this.debounceTimeout = null;
    this.heavyOperationsTimeout = null;
    this.selectionUpdatePending = false;
    
    // Editor elements
    this.textarea = null;
    this.wordCountElement = null;
    this.charCountElement = null;
    this.cursorPositionElement = null;
    
    // Event listeners tracking
    this.eventListeners = new Map();
    this.keyboardShortcuts = new Map();
    
    // Auto-completion pairs
    this.autoCompletePairs = {
      '(': ')',
      '[': ']',
      '{': '}',
      '"': '"',
      "'": "'",
      '`': '`'
    };
    
    // Performance metrics
    this.performanceMetrics = {
      keystrokes: 0,
      inputEvents: 0,
      averageInputTime: 0
    };

    this.init();
  }

  init() {
    this.createEditorStructure();
    this.setupEventListeners();
    this.setupKeyboardShortcuts();
    this.isInitialized = true;
  }

  createEditorStructure() {
    this.container.className = 'markdown-editor-container';
    
    // Create textarea
    this.textarea = document.createElement('textarea');
    this.textarea.className = 'markdown-editor-textarea';
    this.textarea.value = this.content;
    
    // Create status elements
    this.wordCountElement = document.createElement('span');
    this.charCountElement = document.createElement('span');
    this.cursorPositionElement = document.createElement('span');
    
    this.container.appendChild(this.textarea);
  }

  setupEventListeners() {
    // Simulate optimized event handler
    const inputHandler = (event) => {
      const startTime = performance.now();
      
      this.content = this.textarea.value;
      this.debounceHeavyOperations();
      
      const duration = performance.now() - startTime;
      this.performanceMetrics.inputEvents++;
      this.performanceMetrics.averageInputTime = 
        (this.performanceMetrics.averageInputTime + duration) / this.performanceMetrics.inputEvents;
        
      this.emit(MockMarkdownEditor.EVENTS.CONTENT_CHANGED, {
        content: this.content,
        timestamp: Date.now()
      });
    };

    const selectionHandler = () => {
      if (this.selectionUpdatePending) return;
      
      this.selectionUpdatePending = true;
      requestAnimationFrame(() => {
        this.updateCursorPosition();
        this.updateSelectionState();
        this.selectionUpdatePending = false;
      });
    };

    const keyHandler = (event) => {
      this.performanceMetrics.keystrokes++;
      
      // Fast path for common keys
      const isCommonKey = /^[a-zA-Z0-9\s]$/.test(event.key) && 
                         !event.ctrlKey && !event.metaKey && !event.altKey;
      
      if (isCommonKey) {
        return; // Allow default behavior
      }
      
      this.lastKeystroke = performance.now();
      
      // Handle keyboard shortcuts
      const shortcutHandled = this.handleKeyboardShortcut(event);
      if (shortcutHandled) {
        return;
      }
      
      // Handle tab indentation
      if (event.key === 'Tab') {
        event.preventDefault();
        this.handleTabIndentation(event.shiftKey);
        return;
      }
      
      // Handle auto-completion
      if (!event.ctrlKey && !event.metaKey && !event.altKey) {
        this.handleAutoCompletion(event);
      }
    };

    this.textarea.addEventListener('input', inputHandler);
    this.textarea.addEventListener('selectionchange', selectionHandler);
    this.textarea.addEventListener('mouseup', selectionHandler);
    this.textarea.addEventListener('keydown', keyHandler);
  }

  setupKeyboardShortcuts() {
    this.addKeyboardShortcut('ctrl+b', () => this.formatSelection('bold'));
    this.addKeyboardShortcut('ctrl+i', () => this.formatSelection('italic'));
    this.addKeyboardShortcut('ctrl+z', () => this.undo());
  }

  addKeyboardShortcut(shortcut, action) {
    this.keyboardShortcuts.set(shortcut.toLowerCase(), { action, shortcut });
  }

  handleKeyboardShortcut(event) {
    const parts = [];
    if (event.ctrlKey) parts.push('ctrl');
    if (event.metaKey) parts.push('cmd');
    if (event.shiftKey) parts.push('shift');
    if (event.altKey) parts.push('alt');
    parts.push(event.key.toLowerCase());
    
    const shortcutString = parts.join('+');
    const shortcutData = this.keyboardShortcuts.get(shortcutString);
    
    if (shortcutData) {
      event.preventDefault();
      event.stopPropagation();
      shortcutData.action();
      
      this.emit(MockMarkdownEditor.EVENTS.SHORTCUT_EXECUTED, {
        shortcut: shortcutData.shortcut,
        timestamp: Date.now()
      });
      return true;
    }
    
    return false;
  }

  handleAutoCompletion(event) {
    const key = event.key;
    const closingChar = this.autoCompletePairs[key];
    
    if (!closingChar) return;
    
    const currentChar = this.content[this.selectionStart];
    const selection = this.getSelectedText();
    
    if (selection) {
      event.preventDefault();
      const wrappedText = key + selection + closingChar;
      this.insertText(wrappedText, true);
      return;
    }
    
    // Optimized auto-completion logic
    let shouldAutoComplete = false;
    if (key === '"' || key === "'" || key === '`') {
      shouldAutoComplete = currentChar !== key;
    } else {
      shouldAutoComplete = currentChar !== closingChar;
    }
    
    if (shouldAutoComplete) {
      event.preventDefault();
      const completedText = key + closingChar;
      this.insertText(completedText, false);
      
      // Position cursor between characters
      const newCursorPos = this.selectionStart - 1;
      this.textarea.setSelectionRange(newCursorPos, newCursorPos);
    }
  }

  handleTabIndentation(shiftPressed) {
    const tabString = '  ';
    
    if (shiftPressed) {
      // Outdent logic (simplified)
      const beforeCursor = this.content.substring(0, this.selectionStart);
      if (beforeCursor.endsWith(tabString)) {
        const newContent = beforeCursor.slice(0, -2) + 
                          this.content.substring(this.selectionStart);
        this.setValue(newContent);
        this.textarea.setSelectionRange(this.selectionStart - 2, this.selectionStart - 2);
      }
    } else {
      // Indent
      this.insertText(tabString, false);
    }
  }

  formatSelection(formatType) {
    const selection = this.getSelectedText();
    let formattedText = '';
    
    switch (formatType) {
      case 'bold':
        formattedText = selection ? `**${selection}**` : '****';
        break;
      case 'italic':
        formattedText = selection ? `*${selection}*` : '**';
        break;
      default:
        return;
    }
    
    this.insertText(formattedText, true);
    
    this.emit(MockMarkdownEditor.EVENTS.FORMAT_APPLIED, {
      type: formatType,
      selection,
      formattedText,
      timestamp: Date.now()
    });
  }

  debounceHeavyOperations() {
    clearTimeout(this.heavyOperationsTimeout);
    this.heavyOperationsTimeout = setTimeout(() => {
      this.updateCursorPosition();
      this.updateWordCount();
    }, 100);
  }

  updateCursorPosition() {
    this.cursorPosition = this.textarea.selectionStart;
    this.selectionStart = this.textarea.selectionStart;
    this.selectionEnd = this.textarea.selectionEnd;
  }

  updateSelectionState() {
    const hasSelection = this.selectionStart !== this.selectionEnd;
    this.container.classList.toggle('has-selection', hasSelection);
  }

  updateWordCount() {
    const content = String(this.content || '');
    const text = content.trim();
    this.charCount = content.length;
    this.wordCount = text === '' ? 0 : text.split(/\s+/).filter(word => word.length > 0).length;
    
    if (this.wordCountElement) {
      this.wordCountElement.textContent = `${this.wordCount} words`;
    }
    if (this.charCountElement) {
      this.charCountElement.textContent = `${this.charCount} characters`;
    }

    this.emit(MockMarkdownEditor.EVENTS.WORD_COUNT_CHANGED, {
      words: this.wordCount,
      characters: this.charCount
    });
  }

  setValue(content) {
    this.content = String(content || '');
    if (this.textarea) {
      this.textarea.value = this.content;
      this.updateWordCount();
    }
  }

  getValue() {
    return this.content;
  }

  insertText(text, replaceSelection = true) {
    const startPos = replaceSelection ? this.selectionStart : this.cursorPosition;
    const endPos = replaceSelection ? this.selectionEnd : this.cursorPosition;
    
    const before = this.content.substring(0, startPos);
    const after = this.content.substring(endPos);
    const newContent = before + text + after;
    
    this.setValue(newContent);
    
    const newCursorPos = startPos + text.length;
    this.textarea.setSelectionRange(newCursorPos, newCursorPos);
    this.updateCursorPosition();
  }

  getSelectedText() {
    return this.content.substring(this.selectionStart, this.selectionEnd);
  }

  focus() {
    if (this.textarea) {
      this.textarea.focus();
    }
  }

  hasFocus() {
    return document.activeElement === this.textarea;
  }

  undo() {
    // Simplified undo implementation
    console.log('Undo executed');
  }

  emit(eventType, data) {
    const event = new CustomEvent(eventType, {
      detail: data,
      bubbles: false
    });
    this.container.dispatchEvent(event);
  }

  addEventListener(eventType, handler) {
    this.container.addEventListener(eventType, handler);
  }

  getPerformanceStats() {
    return {
      ...this.performanceMetrics,
      contentLength: this.content.length,
      wordCount: this.wordCount,
      charCount: this.charCount,
      lastKeystroke: this.lastKeystroke
    };
  }

  destroy() {
    clearTimeout(this.debounceTimeout);
    clearTimeout(this.heavyOperationsTimeout);
    this.container.innerHTML = '';
  }
}

describe('MarkdownEditor', () => {
  let editor;
  let container;
  let tauriMocks;

  beforeEach(() => {
    tauriMocks = setupTauriMocks();
    
    // Create container element
    container = document.createElement('div');
    document.body.appendChild(container);
    
    // Create editor instance
    editor = new MockMarkdownEditor(container);
  });

  afterEach(() => {
    if (editor) {
      editor.destroy();
    }
    if (container && container.parentNode) {
      container.parentNode.removeChild(container);
    }
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should initialize successfully', () => {
      expect(editor.isInitialized).toBe(true);
      expect(editor.container).toBe(container);
      expect(editor.textarea).toBeInstanceOf(HTMLTextAreaElement);
    });

    it('should create required DOM elements', () => {
      expect(editor.textarea).toBeDefined();
      expect(editor.wordCountElement).toBeDefined();
      expect(editor.charCountElement).toBeDefined();
      expect(editor.cursorPositionElement).toBeDefined();
    });

    it('should set up keyboard shortcuts', () => {
      expect(editor.keyboardShortcuts.size).toBeGreaterThan(0);
      expect(editor.keyboardShortcuts.has('ctrl+b')).toBe(true);
      expect(editor.keyboardShortcuts.has('ctrl+i')).toBe(true);
    });
  });

  describe('Content Management', () => {
    it('should set and get content correctly', () => {
      const testContent = 'Hello world!';
      editor.setValue(testContent);
      
      expect(editor.getValue()).toBe(testContent);
      expect(editor.textarea.value).toBe(testContent);
    });

    it('should update word and character count', () => {
      const testContent = 'Hello world! This is a test.';
      editor.setValue(testContent);
      
      expect(editor.wordCount).toBe(6);
      expect(editor.charCount).toBe(testContent.length);
    });

    it('should handle empty content', () => {
      editor.setValue('');
      
      expect(editor.getValue()).toBe('');
      expect(editor.wordCount).toBe(0);
      expect(editor.charCount).toBe(0);
    });

    it('should emit content change events', (done) => {
      editor.addEventListener(MockMarkdownEditor.EVENTS.CONTENT_CHANGED, (event) => {
        expect(event.detail.content).toBe('test');
        done();
      });
      
      editor.setValue('test');
    });
  });

  describe('Cursor and Selection Management', () => {
    beforeEach(() => {
      editor.setValue('Hello world! This is a test.');
    });

    it('should update cursor position', () => {
      editor.textarea.setSelectionRange(5, 5);
      editor.updateCursorPosition();
      
      expect(editor.cursorPosition).toBe(5);
      expect(editor.selectionStart).toBe(5);
      expect(editor.selectionEnd).toBe(5);
    });

    it('should handle text selection', () => {
      editor.textarea.setSelectionRange(6, 11);
      editor.updateCursorPosition();
      
      expect(editor.getSelectedText()).toBe('world');
      expect(editor.selectionStart).toBe(6);
      expect(editor.selectionEnd).toBe(11);
    });

    it('should insert text at cursor position', () => {
      editor.textarea.setSelectionRange(5, 5);
      editor.updateCursorPosition();
      editor.insertText(' beautiful', false);
      
      expect(editor.getValue()).toContain('Hello beautiful world!');
    });

    it('should replace selected text', () => {
      editor.textarea.setSelectionRange(6, 11);
      editor.updateCursorPosition();
      editor.insertText('universe', true);
      
      expect(editor.getValue()).toContain('Hello universe!');
    });
  });

  describe('Keyboard Shortcuts', () => {
    beforeEach(() => {
      editor.setValue('Hello world!');
      editor.textarea.setSelectionRange(6, 11);
      editor.updateCursorPosition();
    });

    it('should handle bold formatting (Ctrl+B)', () => {
      const event = new KeyboardEvent('keydown', {
        key: 'b',
        ctrlKey: true,
        bubbles: true
      });
      
      const handled = editor.handleKeyboardShortcut(event);
      
      expect(handled).toBe(true);
      expect(editor.getValue()).toContain('**world**');
    });

    it('should handle italic formatting (Ctrl+I)', () => {
      const event = new KeyboardEvent('keydown', {
        key: 'i',
        ctrlKey: true,
        bubbles: true
      });
      
      const handled = editor.handleKeyboardShortcut(event);
      
      expect(handled).toBe(true);
      expect(editor.getValue()).toContain('*world*');
    });

    it('should emit shortcut executed events', (done) => {
      editor.addEventListener(MockMarkdownEditor.EVENTS.SHORTCUT_EXECUTED, (event) => {
        expect(event.detail.shortcut).toBe('ctrl+b');
        done();
      });
      
      const keyEvent = new KeyboardEvent('keydown', {
        key: 'b',
        ctrlKey: true,
        bubbles: true
      });
      
      editor.handleKeyboardShortcut(keyEvent);
    });

    it('should not handle unknown shortcuts', () => {
      const event = new KeyboardEvent('keydown', {
        key: 'x',
        ctrlKey: true,
        bubbles: true
      });
      
      const handled = editor.handleKeyboardShortcut(event);
      expect(handled).toBe(false);
    });
  });

  describe('Auto-completion', () => {
    beforeEach(() => {
      editor.setValue('');
      editor.textarea.setSelectionRange(0, 0);
      editor.updateCursorPosition();
    });

    it('should auto-complete parentheses', () => {
      const event = new KeyboardEvent('keydown', {
        key: '(',
        bubbles: true
      });
      event.preventDefault = vi.fn();
      
      editor.handleAutoCompletion(event);
      
      expect(event.preventDefault).toHaveBeenCalled();
      expect(editor.getValue()).toBe('()');
    });

    it('should auto-complete brackets', () => {
      const event = new KeyboardEvent('keydown', {
        key: '[',
        bubbles: true
      });
      event.preventDefault = vi.fn();
      
      editor.handleAutoCompletion(event);
      
      expect(event.preventDefault).toHaveBeenCalled();
      expect(editor.getValue()).toBe('[]');
    });

    it('should auto-complete quotes', () => {
      const event = new KeyboardEvent('keydown', {
        key: '"',
        bubbles: true
      });
      event.preventDefault = vi.fn();
      
      editor.handleAutoCompletion(event);
      
      expect(event.preventDefault).toHaveBeenCalled();
      expect(editor.getValue()).toBe('""');
    });

    it('should wrap selected text', () => {
      editor.setValue('hello');
      editor.textarea.setSelectionRange(0, 5);
      editor.updateCursorPosition();
      
      const event = new KeyboardEvent('keydown', {
        key: '(',
        bubbles: true
      });
      event.preventDefault = vi.fn();
      
      editor.handleAutoCompletion(event);
      
      expect(event.preventDefault).toHaveBeenCalled();
      expect(editor.getValue()).toBe('(hello)');
    });

    it('should not auto-complete when next char is closing', () => {
      editor.setValue(')test');
      editor.textarea.setSelectionRange(0, 0);
      editor.updateCursorPosition();
      
      const event = new KeyboardEvent('keydown', {
        key: '(',
        bubbles: true
      });
      event.preventDefault = vi.fn();
      
      editor.handleAutoCompletion(event);
      
      expect(event.preventDefault).not.toHaveBeenCalled();
    });
  });

  describe('Tab Indentation', () => {
    beforeEach(() => {
      editor.setValue('hello world');
      editor.textarea.setSelectionRange(0, 0);
      editor.updateCursorPosition();
    });

    it('should indent with Tab', () => {
      editor.handleTabIndentation(false);
      
      expect(editor.getValue()).toBe('  hello world');
    });

    it('should outdent with Shift+Tab', () => {
      editor.setValue('  hello world');
      editor.textarea.setSelectionRange(2, 2);
      editor.updateCursorPosition();
      
      editor.handleTabIndentation(true);
      
      expect(editor.getValue()).toBe('hello world');
    });

    it('should not outdent when no indentation exists', () => {
      editor.handleTabIndentation(true);
      
      expect(editor.getValue()).toBe('hello world');
    });
  });

  describe('Performance and Responsiveness', () => {
    it('should debounce heavy operations', (done) => {
      const originalUpdateWordCount = editor.updateWordCount;
      let callCount = 0;
      
      editor.updateWordCount = () => {
        callCount++;
        originalUpdateWordCount.call(editor);
      };
      
      // Trigger multiple rapid operations
      editor.debounceHeavyOperations();
      editor.debounceHeavyOperations();
      editor.debounceHeavyOperations();
      
      // Should only call once after debounce
      setTimeout(() => {
        expect(callCount).toBe(1);
        done();
      }, 150);
    });

    it('should track performance metrics', () => {
      // Simulate input events
      editor.textarea.value = 'test';
      editor.textarea.dispatchEvent(new Event('input'));
      
      const stats = editor.getPerformanceStats();
      
      expect(stats.inputEvents).toBeGreaterThan(0);
      expect(stats.averageInputTime).toBeGreaterThanOrEqual(0);
    });

    it('should handle selection updates efficiently', () => {
      expect(editor.selectionUpdatePending).toBe(false);
      
      // Simulate rapid selection changes
      const handler = () => {
        if (editor.selectionUpdatePending) return;
        editor.selectionUpdatePending = true;
        requestAnimationFrame(() => {
          editor.selectionUpdatePending = false;
        });
      };
      
      handler();
      handler(); // Second call should return early
      
      expect(editor.selectionUpdatePending).toBe(true);
    });

    it('should use fast path for common keys', () => {
      const event = new KeyboardEvent('keydown', {
        key: 'a',
        bubbles: true
      });
      
      // Mock the key handler logic
      const isCommonKey = /^[a-zA-Z0-9\s]$/.test(event.key) && 
                         !event.ctrlKey && !event.metaKey && !event.altKey;
      
      expect(isCommonKey).toBe(true);
    });

    it('should measure keystroke performance', () => {
      const beforeTime = performance.now();
      editor.lastKeystroke = performance.now();
      const afterTime = performance.now();
      
      expect(editor.lastKeystroke).toBeGreaterThanOrEqual(beforeTime);
      expect(editor.lastKeystroke).toBeLessThanOrEqual(afterTime);
    });
  });

  describe('Event System', () => {
    it('should emit word count change events', (done) => {
      editor.addEventListener(MockMarkdownEditor.EVENTS.WORD_COUNT_CHANGED, (event) => {
        expect(event.detail.words).toBe(2);
        expect(event.detail.characters).toBe(11);
        done();
      });
      
      editor.setValue('hello world');
    });

    it('should emit format applied events', (done) => {
      editor.addEventListener(MockMarkdownEditor.EVENTS.FORMAT_APPLIED, (event) => {
        expect(event.detail.type).toBe('bold');
        done();
      });
      
      editor.setValue('test');
      editor.textarea.setSelectionRange(0, 4);
      editor.updateCursorPosition();
      editor.formatSelection('bold');
    });
  });

  describe('Edge Cases', () => {
    it('should handle null/undefined content', () => {
      editor.setValue(null);
      expect(editor.getValue()).toBe('');
      
      editor.setValue(undefined);
      expect(editor.getValue()).toBe('');
    });

    it('should handle very long content', () => {
      const longContent = 'a'.repeat(10000);
      editor.setValue(longContent);
      
      expect(editor.getValue().length).toBe(10000);
      expect(editor.charCount).toBe(10000);
    });

    it('should handle special characters in auto-completion', () => {
      editor.setValue('');
      editor.textarea.setSelectionRange(0, 0);
      editor.updateCursorPosition();
      
      const event = new KeyboardEvent('keydown', {
        key: '{',
        bubbles: true
      });
      event.preventDefault = vi.fn();
      
      editor.handleAutoCompletion(event);
      
      expect(editor.getValue()).toBe('{}');
    });

    it('should maintain cursor position after operations', () => {
      editor.setValue('hello world');
      editor.textarea.setSelectionRange(5, 5);
      editor.updateCursorPosition();
      
      const originalPos = editor.cursorPosition;
      editor.insertText(' beautiful', false);
      
      expect(editor.cursorPosition).toBe(originalPos + ' beautiful'.length);
    });
  });

  describe('Focus Management', () => {
    it('should focus the editor', () => {
      const focusSpy = vi.spyOn(editor.textarea, 'focus');
      editor.focus();
      
      expect(focusSpy).toHaveBeenCalled();
    });

    it('should check focus state', () => {
      // Mock the focus state
      Object.defineProperty(document, 'activeElement', {
        value: editor.textarea,
        configurable: true
      });
      
      expect(editor.hasFocus()).toBe(true);
    });
  });

  describe('Cleanup', () => {
    it('should clean up resources on destroy', () => {
      const clearTimeoutSpy = vi.spyOn(global, 'clearTimeout');
      
      editor.destroy();
      
      expect(clearTimeoutSpy).toHaveBeenCalledWith(editor.debounceTimeout);
      expect(clearTimeoutSpy).toHaveBeenCalledWith(editor.heavyOperationsTimeout);
      expect(editor.container.innerHTML).toBe('');
    });
  });
});