/**
 * Unit tests for AppState class
 * 
 * Tests cover:
 * - Core state management functionality
 * - Event system and listeners
 * - State persistence via Tauri commands
 * - Validation and error handling
 * - Performance requirements (<50ms operations, <1ms events)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import AppState class
import AppState from '../../src/js/state.js';

describe('AppState', () => {
  let appState;
  let tauriMocks;

  beforeEach(() => {
    // Set up Tauri mocks
    tauriMocks = setupTauriMocks();
    
    // Create fresh AppState instance
    appState = new AppState();
  });

  afterEach(() => {
    // Clean up mocks
    vi.clearAllMocks();
    
    // Reset AppState if needed
    if (appState) {
      appState.eventListeners?.clear();
    }
  });

  describe('Initialization', () => {
    it('should initialize with default values', () => {
      expect(appState.currentVault).toBeNull();
      expect(appState.currentFile).toBeNull();
      expect(appState.viewMode).toBe(AppState.VIEW_MODES.EDITOR);
      expect(appState.unsavedChanges).toBe(false);
      expect(appState.files).toEqual([]);
    });

    it('should initialize event listener system', () => {
      expect(appState.eventListeners).toBeInstanceOf(Map);
      expect(appState.eventListeners.size).toBe(0);
    });

    it('should have valid constants defined', () => {
      expect(AppState.EVENTS).toEqual({
        VAULT_CHANGED: 'vault_changed',
        FILE_CHANGED: 'file_changed',
        VIEW_MODE_CHANGED: 'view_mode_changed',
        DIRTY_STATE_CHANGED: 'dirty_state_changed',
        FILES_UPDATED: 'files_updated'
      });

      expect(AppState.VIEW_MODES).toEqual({
        EDITOR: 'editor',
        PREVIEW: 'preview'
      });
    });
  });

  describe('Vault Management', () => {
    it('should set vault and emit vault_changed event', async () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, mockCallback);

      // Mock save state command
      tauriMocks.invoke.mockResolvedValue(true);

      await appState.setVault('/test/vault');

      expect(appState.currentVault).toBe('/test/vault');
      expect(mockCallback).toHaveBeenCalledWith({
        vault: '/test/vault',
        previousVault: null
      });
      expect(tauriMocks.invoke).toHaveBeenCalledWith('save_session_state', {
        currentVault: '/test/vault',
        currentFile: null,
        viewMode: 'editor'
      });
    });

    it('should clear current file when vault changes', async () => {
      // Set up initial state
      appState.currentFile = '/old/file.md';
      appState.files = [{ name: 'test.md', path: '/old/file.md' }];
      appState.unsavedChanges = true;

      tauriMocks.invoke.mockResolvedValue(true);

      await appState.setVault('/new/vault');

      expect(appState.currentFile).toBeNull();
      expect(appState.files).toEqual([]);
      expect(appState.unsavedChanges).toBe(false);
    });

    it('should not emit event if vault is the same', async () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, mockCallback);
      
      appState.currentVault = '/same/vault';
      await appState.setVault('/same/vault');

      expect(mockCallback).not.toHaveBeenCalled();
    });
  });

  describe('File Management', () => {
    it('should set current file and emit file_changed event', async () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.FILE_CHANGED, mockCallback);

      tauriMocks.invoke.mockResolvedValue(true);

      await appState.setCurrentFile('/test/file.md');

      expect(appState.currentFile).toBe('/test/file.md');
      expect(mockCallback).toHaveBeenCalledWith({
        file: '/test/file.md',
        previousFile: null
      });
    });

    it('should reset unsaved changes when switching files', async () => {
      appState.unsavedChanges = true;
      tauriMocks.invoke.mockResolvedValue(true);

      await appState.setCurrentFile('/new/file.md');

      expect(appState.unsavedChanges).toBe(false);
    });

    it('should set files array and emit files_updated event', () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.FILES_UPDATED, mockCallback);

      const testFiles = [
        { name: 'test1.md', path: '/vault/test1.md', is_dir: false },
        { name: 'folder', path: '/vault/folder', is_dir: true }
      ];

      appState.setFiles(testFiles);

      expect(appState.files).toEqual(testFiles);
      expect(mockCallback).toHaveBeenCalledWith({
        files: testFiles,
        count: 2
      });
    });

    it('should throw error for invalid files input', () => {
      expect(() => {
        appState.setFiles('not-an-array');
      }).toThrow('Files must be an array');

      expect(() => {
        appState.setFiles(null);
      }).toThrow('Files must be an array');
    });
  });

  describe('View Mode Management', () => {
    it('should toggle between editor and preview modes', () => {
      expect(appState.viewMode).toBe(AppState.VIEW_MODES.EDITOR);
      
      const newMode = appState.toggleViewMode();
      expect(newMode).toBe(AppState.VIEW_MODES.PREVIEW);
      expect(appState.viewMode).toBe(AppState.VIEW_MODES.PREVIEW);
      
      appState.toggleViewMode();
      expect(appState.viewMode).toBe(AppState.VIEW_MODES.EDITOR);
    });

    it('should set view mode explicitly and emit event', async () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.VIEW_MODE_CHANGED, mockCallback);

      tauriMocks.invoke.mockResolvedValue(true);

      await appState.setViewMode(AppState.VIEW_MODES.PREVIEW);

      expect(appState.viewMode).toBe(AppState.VIEW_MODES.PREVIEW);
      expect(mockCallback).toHaveBeenCalledWith({
        mode: AppState.VIEW_MODES.PREVIEW,
        previousMode: AppState.VIEW_MODES.EDITOR
      });
    });

    it('should throw error for invalid view mode', async () => {
      await expect(appState.setViewMode('invalid-mode')).rejects.toThrow('Invalid view mode: invalid-mode');
    });

    it('should not emit event if view mode is the same', async () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.VIEW_MODE_CHANGED, mockCallback);

      await appState.setViewMode(AppState.VIEW_MODES.EDITOR); // Already editor mode

      expect(mockCallback).not.toHaveBeenCalled();
    });
  });

  describe('Dirty State Management', () => {
    it('should mark as dirty and emit dirty_state_changed event', () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, mockCallback);

      appState.markDirty(true);

      expect(appState.unsavedChanges).toBe(true);
      expect(mockCallback).toHaveBeenCalledWith({
        isDirty: true,
        file: null
      });
    });

    it('should mark as clean and emit event', () => {
      appState.unsavedChanges = true;
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, mockCallback);

      appState.markDirty(false);

      expect(appState.unsavedChanges).toBe(false);
      expect(mockCallback).toHaveBeenCalledWith({
        isDirty: false,
        file: null
      });
    });

    it('should not emit event if dirty state is the same', () => {
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, mockCallback);

      appState.markDirty(false); // Already false

      expect(mockCallback).not.toHaveBeenCalled();
    });

    it('should include current file in dirty state event', () => {
      appState.currentFile = '/test/file.md';
      const mockCallback = vi.fn();
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, mockCallback);

      appState.markDirty(true);

      expect(mockCallback).toHaveBeenCalledWith({
        isDirty: true,
        file: '/test/file.md'
      });
    });
  });

  describe('Event System', () => {
    it('should add and remove event listeners', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();

      appState.addEventListener('test_event', callback1);
      appState.addEventListener('test_event', callback2);

      expect(appState.eventListeners.has('test_event')).toBe(true);
      expect(appState.eventListeners.get('test_event').size).toBe(2);

      appState.removeEventListener('test_event', callback1);
      expect(appState.eventListeners.get('test_event').size).toBe(1);

      appState.removeEventListener('test_event', callback2);
      expect(appState.eventListeners.has('test_event')).toBe(false);
    });

    it('should throw error for non-function callback', () => {
      expect(() => {
        appState.addEventListener('test_event', 'not-a-function');
      }).toThrow('Event callback must be a function');
    });

    it('should emit events to all listeners', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();
      const testData = { test: 'data' };

      appState.addEventListener('test_event', callback1);
      appState.addEventListener('test_event', callback2);

      appState.emit('test_event', testData);

      expect(callback1).toHaveBeenCalledWith(testData);
      expect(callback2).toHaveBeenCalledWith(testData);
    });

    it('should handle errors in event listeners gracefully', () => {
      const errorCallback = vi.fn(() => { throw new Error('Test error'); });
      const successCallback = vi.fn();
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      appState.addEventListener('test_event', errorCallback);
      appState.addEventListener('test_event', successCallback);

      appState.emit('test_event', { test: 'data' });

      expect(errorCallback).toHaveBeenCalled();
      expect(successCallback).toHaveBeenCalled();
      expect(consoleSpy).toHaveBeenCalled();

      consoleSpy.mockRestore();
    });

    it('should handle event emission for non-existent events', () => {
      // Should not throw error
      expect(() => {
        appState.emit('non_existent_event', {});
      }).not.toThrow();
    });
  });

  describe('State Persistence', () => {
    it('should save state via Tauri command', async () => {
      tauriMocks.invoke.mockResolvedValue(true);

      await appState.saveState();

      expect(tauriMocks.invoke).toHaveBeenCalledWith('save_session_state', {
        currentVault: null,
        currentFile: null,
        viewMode: 'editor'
      });
    });

    it('should handle save state errors gracefully', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Save failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      await appState.saveState();

      expect(consoleSpy).toHaveBeenCalledWith('Failed to save session state:', expect.any(Error));
      consoleSpy.mockRestore();
    });

    it('should load state from Tauri on initialization', async () => {
      const mockState = {
        session: {
          current_vault: '/test/vault',
          current_file: '/test/file.md',
          view_mode: 'preview'
        }
      };

      tauriMocks.invoke.mockResolvedValue(mockState);

      const newAppState = new AppState();
      await newAppState.loadState();

      expect(newAppState.currentVault).toBe('/test/vault');
      expect(newAppState.currentFile).toBe('/test/file.md');
      expect(newAppState.viewMode).toBe('preview');
    });

    it('should handle load state errors gracefully', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Load failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const newAppState = new AppState();
      await newAppState.loadState();

      // Should reset to defaults
      expect(newAppState.currentVault).toBeNull();
      expect(newAppState.currentFile).toBeNull();
      expect(newAppState.viewMode).toBe(AppState.VIEW_MODES.EDITOR);
      expect(consoleSpy).toHaveBeenCalledWith('Failed to load session state:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });

    it('should validate loaded state properties', async () => {
      const invalidState = {
        session: {
          current_vault: 123, // Invalid type
          current_file: {}, // Invalid type
          view_mode: 'invalid_mode' // Invalid value
        }
      };

      tauriMocks.invoke.mockResolvedValue(invalidState);

      const newAppState = new AppState();
      await newAppState.loadState();

      // Should ignore invalid values and use defaults
      expect(newAppState.currentVault).toBeNull();
      expect(newAppState.currentFile).toBeNull();
      expect(newAppState.viewMode).toBe(AppState.VIEW_MODES.EDITOR);
    });
  });

  describe('State Utilities', () => {
    it('should return current state snapshot', () => {
      appState.currentVault = '/test/vault';
      appState.currentFile = '/test/file.md';
      appState.viewMode = AppState.VIEW_MODES.PREVIEW;
      appState.unsavedChanges = true;
      appState.files = [{ name: 'test.md', path: '/test/file.md' }];

      const state = appState.getState();

      expect(state).toEqual({
        currentVault: '/test/vault',
        currentFile: '/test/file.md',
        viewMode: 'preview',
        unsavedChanges: true,
        files: [{ name: 'test.md', path: '/test/file.md' }]
      });

      // Should be a copy, not a reference
      state.files.push({ name: 'new.md' });
      expect(appState.files).toHaveLength(1);
    });

    it('should validate state integrity', () => {
      // Valid state
      expect(appState.isValid()).toBe(true);

      // Invalid view mode
      appState.viewMode = 'invalid';
      expect(appState.isValid()).toBe(false);

      appState.viewMode = AppState.VIEW_MODES.EDITOR;

      // Invalid unsaved changes type
      appState.unsavedChanges = 'not-boolean';
      expect(appState.isValid()).toBe(false);

      appState.unsavedChanges = false;

      // Invalid files type
      appState.files = 'not-array';
      expect(appState.isValid()).toBe(false);
    });

    it('should reset all state and emit events', async () => {
      // Set up initial state
      appState.currentVault = '/test/vault';
      appState.currentFile = '/test/file.md';
      appState.viewMode = AppState.VIEW_MODES.PREVIEW;
      appState.unsavedChanges = true;
      appState.files = [{ name: 'test.md' }];

      const callbacks = {
        vaultChanged: vi.fn(),
        fileChanged: vi.fn(),
        viewModeChanged: vi.fn(),
        dirtyStateChanged: vi.fn(),
        filesUpdated: vi.fn()
      };

      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, callbacks.vaultChanged);
      appState.addEventListener(AppState.EVENTS.FILE_CHANGED, callbacks.fileChanged);
      appState.addEventListener(AppState.EVENTS.VIEW_MODE_CHANGED, callbacks.viewModeChanged);
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, callbacks.dirtyStateChanged);
      appState.addEventListener(AppState.EVENTS.FILES_UPDATED, callbacks.filesUpdated);

      tauriMocks.invoke.mockResolvedValue(true);

      await appState.reset();

      // Check state is reset
      expect(appState.currentVault).toBeNull();
      expect(appState.currentFile).toBeNull();
      expect(appState.viewMode).toBe(AppState.VIEW_MODES.EDITOR);
      expect(appState.unsavedChanges).toBe(false);
      expect(appState.files).toEqual([]);

      // Check events were emitted
      expect(callbacks.vaultChanged).toHaveBeenCalledWith({ vault: null, previousVault: null });
      expect(callbacks.fileChanged).toHaveBeenCalledWith({ file: null, previousFile: null });
      expect(callbacks.viewModeChanged).toHaveBeenCalledWith({ mode: AppState.VIEW_MODES.EDITOR });
      expect(callbacks.dirtyStateChanged).toHaveBeenCalledWith({ isDirty: false });
      expect(callbacks.filesUpdated).toHaveBeenCalledWith({ files: [], count: 0 });
    });
  });

  describe('Performance', () => {
    it('should complete state operations within performance targets', async () => {
      tauriMocks.invoke.mockResolvedValue(true);

      // Test saveState performance (<50ms target)
      const saveStartTime = performance.now();
      await appState.saveState();
      const saveTime = performance.now() - saveStartTime;
      expect(saveTime).toBeLessThan(50);

      // Test event emission performance (<1ms target)
      const callback = vi.fn();
      appState.addEventListener('test_event', callback);

      const emitStartTime = performance.now();
      appState.emit('test_event', { test: 'data' });
      const emitTime = performance.now() - emitStartTime;
      expect(emitTime).toBeLessThan(1);

      // Test state getter performance
      const getStateStartTime = performance.now();
      appState.getState();
      const getStateTime = performance.now() - getStateStartTime;
      expect(getStateTime).toBeLessThan(1);
    });

    it('should handle large files arrays efficiently', () => {
      const largeFileList = Array.from({ length: 1000 }, (_, i) => ({
        name: `file${i}.md`,
        path: `/vault/file${i}.md`,
        is_dir: false
      }));

      const startTime = performance.now();
      appState.setFiles(largeFileList);
      const operationTime = performance.now() - startTime;

      expect(operationTime).toBeLessThan(10); // Should be very fast for in-memory operations
      expect(appState.files).toHaveLength(1000);
    });

    it('should handle multiple rapid event emissions efficiently', () => {
      const callback = vi.fn();
      appState.addEventListener('test_event', callback);

      const startTime = performance.now();
      for (let i = 0; i < 100; i++) {
        appState.emit('test_event', { iteration: i });
      }
      const totalTime = performance.now() - startTime;

      expect(totalTime).toBeLessThan(10); // 100 events in <10ms
      expect(callback).toHaveBeenCalledTimes(100);
    });
  });
});