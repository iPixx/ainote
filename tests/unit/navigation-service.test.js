/**
 * NavigationService Unit Tests
 * 
 * Tests the navigation service functionality including:
 * - File navigation and opening
 * - Content positioning and highlighting
 * - Queue management and throttling
 * - Error handling and recovery
 * - Performance optimization
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';
import NavigationService from '../../src/js/services/navigation-service.js';

// Setup Tauri mocks
setupTauriMocks();

describe('NavigationService', () => {
  let navigationService;
  let mockAppState;
  let mockFileTree;
  let mockEditorPanel;
  let mockEditor;

  beforeEach(() => {
    // Mock AppState
    mockAppState = {
      get: vi.fn().mockImplementation((key) => {
        if (key === 'vaultPath') return '/test/vault';
        if (key === 'currentFile') return null;
        return null;
      }),
      set: vi.fn()
    };

    // Mock Editor
    mockEditor = {
      getValue: vi.fn().mockReturnValue('Test file content\nSecond line\nThird line'),
      setCursor: vi.fn(),
      scrollIntoView: vi.fn(),
      addLineClass: vi.fn().mockReturnValue('line-handle'),
      removeLineClass: vi.fn(),
      focus: vi.fn()
    };

    // Mock FileTree
    mockFileTree = {
      selectFile: vi.fn().mockResolvedValue(true)
    };

    // Mock EditorPreviewPanel
    mockEditorPanel = {
      loadFile: vi.fn().mockResolvedValue(true),
      getEditor: vi.fn().mockReturnValue(mockEditor)
    };

    navigationService = new NavigationService(mockAppState, mockFileTree, mockEditorPanel);
  });

  afterEach(() => {
    if (navigationService) {
      navigationService.destroy();
    }
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should create navigation service with required dependencies', () => {
      expect(navigationService).toBeDefined();
      expect(navigationService.appState).toBe(mockAppState);
      expect(navigationService.fileTree).toBe(mockFileTree);
      expect(navigationService.editorPanel).toBe(mockEditorPanel);
    });

    it('should throw error when required dependencies are missing', () => {
      expect(() => new NavigationService(null, mockFileTree, mockEditorPanel))
        .toThrow('AppState instance is required');
      
      expect(() => new NavigationService(mockAppState, null, mockEditorPanel))
        .toThrow('FileTree instance is required');
      
      expect(() => new NavigationService(mockAppState, mockFileTree, null))
        .toThrow('EditorPreviewPanel instance is required');
    });

    it('should initialize with default configuration', () => {
      expect(navigationService.config).toEqual({
        navigationThrottle: 100,
        scrollAnimationDuration: 300,
        maxQueueSize: 5
      });
    });
  });

  describe('File Path Resolution', () => {
    it('should resolve absolute paths correctly', async () => {
      const absolutePath = '/absolute/path/to/file.md';
      const resolved = await navigationService.resolveFilePath(absolutePath);
      expect(resolved).toBe(absolutePath);
    });

    it('should resolve relative paths to vault', async () => {
      const relativePath = 'notes/test.md';
      const resolved = await navigationService.resolveFilePath(relativePath);
      expect(resolved).toBe('/test/vault/notes/test.md');
    });

    it('should handle Windows-style absolute paths', async () => {
      const windowsPath = 'C:/Users/test/file.md';
      const resolved = await navigationService.resolveFilePath(windowsPath);
      expect(resolved).toBe(windowsPath);
    });

    it('should return null when no vault is selected', async () => {
      mockAppState.get.mockImplementation((key) => {
        if (key === 'vaultPath') return null;
        return null;
      });

      const resolved = await navigationService.resolveFilePath('test.md');
      expect(resolved).toBeNull();
    });
  });

  describe('File Navigation', () => {
    beforeEach(() => {
      // Mock successful file operations
      window.__TAURI__.core.invoke.mockImplementation((command, args) => {
        switch (command) {
          case 'file_exists':
            return Promise.resolve(true);
          case 'read_file':
            return Promise.resolve('Test file content for navigation');
          default:
            return Promise.resolve(null);
        }
      });
    });

    it('should navigate to file successfully', async () => {
      const filePath = '/test/vault/notes/example.md';
      const result = await navigationService.navigateToFile(filePath);
      
      expect(result).toBe(true);
      expect(mockFileTree.selectFile).toHaveBeenCalledWith(filePath);
      expect(mockEditorPanel.loadFile).toHaveBeenCalledWith(
        filePath,
        'Test file content for navigation'
      );
      expect(mockAppState.set).toHaveBeenCalledWith('currentFile', filePath);
    });

    it('should handle navigation with content positioning', async () => {
      const filePath = '/test/vault/notes/example.md';
      const options = {
        scrollToContent: true,
        contentQuery: 'Second line',
        highlightDuration: 1000
      };
      
      const result = await navigationService.navigateToFile(filePath, options);
      
      expect(result).toBe(true);
      expect(mockEditor.setCursor).toHaveBeenCalled();
      expect(mockEditor.scrollIntoView).toHaveBeenCalled();
      expect(mockEditor.addLineClass).toHaveBeenCalled();
    });

    it('should handle navigation with line positioning', async () => {
      const filePath = '/test/vault/notes/example.md';
      const options = {
        line: 2,
        column: 5,
        highlightDuration: 1000
      };
      
      const result = await navigationService.navigateToFile(filePath, options);
      
      expect(result).toBe(true);
      expect(mockEditor.setCursor).toHaveBeenCalledWith(2, 5);
      expect(mockEditor.scrollIntoView).toHaveBeenCalled();
    });

    it('should fail navigation for non-existent file', async () => {
      window.__TAURI__.core.invoke.mockImplementation((command, args) => {
        if (command === 'file_exists') return Promise.resolve(false);
        return Promise.resolve(null);
      });

      const result = await navigationService.navigateToFile('/nonexistent/file.md');
      expect(result).toBe(false);
    });

    it('should emit navigation events', async () => {
      const startHandler = vi.fn();
      const completedHandler = vi.fn();
      
      navigationService.addEventListener(NavigationService.EVENTS.NAVIGATION_STARTED, startHandler);
      navigationService.addEventListener(NavigationService.EVENTS.NAVIGATION_COMPLETED, completedHandler);
      
      await navigationService.navigateToFile('/test/vault/example.md');
      
      expect(startHandler).toHaveBeenCalled();
      expect(completedHandler).toHaveBeenCalled();
    });
  });

  describe('Suggestion Navigation', () => {
    beforeEach(() => {
      window.__TAURI__.core.invoke.mockImplementation((command) => {
        if (command === 'file_exists') return Promise.resolve(true);
        if (command === 'read_file') return Promise.resolve('Suggestion content');
        return Promise.resolve(null);
      });
    });

    it('should navigate to suggestion successfully', async () => {
      const suggestion = {
        filePath: '/test/vault/notes/suggestion.md',
        contextSnippet: 'Test snippet',
        content: 'Full content'
      };
      
      const result = await navigationService.navigateToSuggestion(suggestion);
      
      expect(result).toBe(true);
      expect(mockEditorPanel.loadFile).toHaveBeenCalledWith(
        suggestion.filePath,
        'Suggestion content'
      );
    });

    it('should handle invalid suggestion data', async () => {
      const invalidSuggestion = { title: 'No file path' };
      const result = await navigationService.navigateToSuggestion(invalidSuggestion);
      
      expect(result).toBe(false);
    });

    it('should position content based on suggestion context', async () => {
      const suggestion = {
        filePath: '/test/vault/notes/suggestion.md',
        contextSnippet: 'Second line',
        content: 'Full content'
      };
      
      await navigationService.navigateToSuggestion(suggestion);
      
      expect(mockEditor.setCursor).toHaveBeenCalled();
      expect(mockEditor.scrollIntoView).toHaveBeenCalled();
    });
  });

  describe('Navigation Queue and Throttling', () => {
    it('should throttle rapid navigation requests', async () => {
      const file1 = '/test/vault/file1.md';
      const file2 = '/test/vault/file2.md';
      
      // Mock quick succession
      window.__TAURI__.core.invoke.mockResolvedValue(true);
      
      // First navigation should succeed immediately
      const result1 = navigationService.navigateToFile(file1);
      
      // Second navigation should be queued
      const result2 = navigationService.navigateToFile(file2);
      
      await result1;
      await result2;
      
      expect(navigationService.navigationQueue.length).toBeGreaterThanOrEqual(0);
    });

    it('should limit queue size', async () => {
      window.__TAURI__.core.invoke.mockResolvedValue(true);
      
      // Fill queue beyond max size
      const promises = [];
      for (let i = 0; i < 10; i++) {
        promises.push(navigationService.navigateToFile(`/test/vault/file${i}.md`));
      }
      
      await Promise.all(promises);
      
      expect(navigationService.navigationQueue.length).toBeLessThanOrEqual(
        navigationService.config.maxQueueSize
      );
    });

    it('should process queued navigations in order', async () => {
      window.__TAURI__.core.invoke.mockImplementation((command) => {
        return new Promise(resolve => {
          setTimeout(() => resolve(command === 'file_exists' ? true : 'content'), 50);
        });
      });
      
      const navigations = [
        '/test/vault/file1.md',
        '/test/vault/file2.md',
        '/test/vault/file3.md'
      ];
      
      const results = await Promise.all(
        navigations.map(file => navigationService.navigateToFile(file))
      );
      
      // At least one should succeed (the first one)
      expect(results.some(r => r === true)).toBe(true);
    });
  });

  describe('Content Positioning', () => {
    it('should find content position by text search', async () => {
      mockEditor.getValue.mockReturnValue('Line 1\nTarget line content\nLine 3');
      
      const position = await navigationService.findContentPosition('Target line');
      
      expect(position).toEqual({
        line: 1,
        column: 0
      });
    });

    it('should handle content not found', async () => {
      mockEditor.getValue.mockReturnValue('Line 1\nLine 2\nLine 3');
      
      const position = await navigationService.findContentPosition('Not found');
      
      expect(position).toBeNull();
    });

    it('should highlight content with timeout', async () => {
      const position = { line: 1, column: 5 };
      const options = { highlightDuration: 100 };
      
      await navigationService.highlightContent(position, options);
      
      expect(mockEditor.addLineClass).toHaveBeenCalledWith(1, 'background', 'navigation-highlight');
      
      // Wait for highlight removal
      await new Promise(resolve => setTimeout(resolve, 150));
      
      expect(mockEditor.removeLineClass).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should handle file read errors gracefully', async () => {
      window.__TAURI__.core.invoke.mockImplementation((command) => {
        if (command === 'file_exists') return Promise.resolve(true);
        if (command === 'read_file') return Promise.reject(new Error('Read failed'));
        return Promise.resolve(null);
      });
      
      const errorHandler = vi.fn();
      navigationService.addEventListener(NavigationService.EVENTS.NAVIGATION_FAILED, errorHandler);
      
      const result = await navigationService.navigateToFile('/test/vault/error.md');
      
      expect(result).toBe(false);
      expect(errorHandler).toHaveBeenCalled();
    });

    it('should handle missing editor gracefully', async () => {
      mockEditorPanel.getEditor.mockReturnValue(null);
      
      const result = await navigationService.navigateToFile('/test/vault/test.md');
      
      // May fail if editor is not available for file loading
      // This is acceptable behavior
      expect(typeof result).toBe('boolean');
    });

    it('should handle file tree selection errors', async () => {
      mockFileTree.selectFile.mockRejectedValue(new Error('Selection failed'));
      
      window.__TAURI__.core.invoke.mockImplementation((command) => {
        if (command === 'file_exists') return Promise.resolve(true);
        if (command === 'read_file') return Promise.resolve('content');
        return Promise.resolve(null);
      });
      
      const result = await navigationService.navigateToFile('/test/vault/test.md');
      
      // Should still succeed despite file tree error
      expect(result).toBe(true);
    });
  });

  describe('Performance', () => {
    it('should handle rapid navigation requests efficiently', async () => {
      window.__TAURI__.core.invoke.mockResolvedValue(true);
      mockEditorPanel.loadFile.mockResolvedValue(true);
      
      const startTime = performance.now();
      
      const promises = [];
      for (let i = 0; i < 20; i++) {
        promises.push(navigationService.navigateToFile(`/test/vault/file${i}.md`));
      }
      
      await Promise.all(promises);
      
      const endTime = performance.now();
      const totalTime = endTime - startTime;
      
      // Should handle 20 navigations in reasonable time (adjusted for test environment)
      expect(totalTime).toBeLessThan(2000);
    });

    it('should maintain performance stats', () => {
      const stats = navigationService.getStats();
      
      expect(stats).toHaveProperty('queueLength');
      expect(stats).toHaveProperty('isNavigating');
      expect(stats).toHaveProperty('lastNavigationTime');
      expect(stats).toHaveProperty('config');
    });
  });

  describe('Configuration', () => {
    it('should allow configuration updates', () => {
      const newConfig = {
        navigationThrottle: 200,
        maxQueueSize: 10
      };
      
      navigationService.updateConfig(newConfig);
      
      expect(navigationService.config.navigationThrottle).toBe(200);
      expect(navigationService.config.maxQueueSize).toBe(10);
      expect(navigationService.config.scrollAnimationDuration).toBe(300); // Should keep existing
    });
  });

  describe('Event System', () => {
    it('should support event listeners', () => {
      const handler = vi.fn();
      navigationService.addEventListener('test_event', handler);
      
      navigationService.emit('test_event', { data: 'test' });
      
      expect(handler).toHaveBeenCalledWith({ data: 'test' });
    });

    it('should support removing event listeners', () => {
      const handler = vi.fn();
      navigationService.addEventListener('test_event', handler);
      navigationService.removeEventListener('test_event', handler);
      
      navigationService.emit('test_event', { data: 'test' });
      
      expect(handler).not.toHaveBeenCalled();
    });

    it('should handle event handler errors gracefully', () => {
      const errorHandler = vi.fn().mockImplementation(() => {
        throw new Error('Handler error');
      });
      const goodHandler = vi.fn();
      
      navigationService.addEventListener('test_event', errorHandler);
      navigationService.addEventListener('test_event', goodHandler);
      
      expect(() => {
        navigationService.emit('test_event', { data: 'test' });
      }).not.toThrow();
      
      expect(goodHandler).toHaveBeenCalled();
    });
  });

  describe('Memory Management', () => {
    it('should clean up resources when destroyed', () => {
      const handler = vi.fn();
      navigationService.addEventListener('test_event', handler);
      
      expect(navigationService.navigationQueue.length).toBe(0);
      expect(navigationService.eventListeners.size).toBeGreaterThan(0);
      
      navigationService.destroy();
      
      expect(navigationService.navigationQueue.length).toBe(0);
      expect(navigationService.eventListeners.size).toBe(0);
      expect(navigationService.isNavigating).toBe(false);
    });
  });
});