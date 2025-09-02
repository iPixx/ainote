/**
 * Unit tests for AutoSave service
 * 
 * Tests cover:
 * - Auto-save functionality with debouncing
 * - Manual save operations (Ctrl+S)
 * - Content change handling and dirty state
 * - Error handling and retry logic
 * - Statistics and performance tracking
 * - Event system and notifications
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import dependencies
import AutoSave from '../../src/js/services/auto-save.js';
import AppState from '../../src/js/state.js';

// Mock timers for debouncing tests
vi.useFakeTimers();

describe('AutoSave', () => {
  let autoSave;
  let appState;
  let tauriMocks;
  let mockContentGetter;

  beforeEach(() => {
    // Set up Tauri mocks
    tauriMocks = setupTauriMocks();
    
    // Create AppState instance
    appState = new AppState();
    appState.currentFile = '/test/file.md';
    
    // Create mock content getter
    mockContentGetter = vi.fn(() => 'test content');
    
    // Create AutoSave instance
    autoSave = new AutoSave(appState);
    autoSave.setContentGetter(mockContentGetter);
    
    // Mock successful save operations by default
    tauriMocks.invoke.mockResolvedValue(true);
  });

  afterEach(() => {
    // Clean up
    autoSave.destroy();
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  describe('Initialization', () => {
    it('should throw error for missing AppState', () => {
      expect(() => {
        new AutoSave(null);
      }).toThrow('AppState instance is required for AutoSave');
    });

    it('should initialize with proper default values', () => {
      expect(autoSave.appState).toBe(appState);
      expect(autoSave.saveDelay).toBe(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      expect(autoSave.isEnabled).toBe(true);
      expect(autoSave.saveTimeoutId).toBeNull();
      expect(autoSave.isSaving).toBe(false);
      expect(autoSave.lastSaveContent).toBeNull();
    });

    it('should have valid constants defined', () => {
      expect(AutoSave.DEFAULTS).toEqual({
        AUTO_SAVE_DELAY: 2000,
        MAX_RETRY_ATTEMPTS: 3,
        RETRY_DELAY: 500
      });

      expect(AutoSave.EVENTS).toEqual({
        SAVE_STARTED: 'save_started',
        SAVE_SUCCESS: 'save_success',
        SAVE_ERROR: 'save_error',
        SAVE_CONFLICT: 'save_conflict',
        AUTO_SAVE_ENABLED: 'auto_save_enabled',
        AUTO_SAVE_DISABLED: 'auto_save_disabled'
      });
    });

    it('should initialize event listener system', () => {
      expect(autoSave.eventListeners).toBeInstanceOf(Map);
      expect(autoSave.eventListeners.size).toBe(0);
    });

    it('should initialize save statistics', () => {
      expect(autoSave.saveStats).toEqual({
        totalSaves: 0,
        totalAutoSaves: 0,
        totalManualSaves: 0,
        averageSaveTime: 0,
        lastSaveTime: null,
        saveErrors: 0
      });
    });
  });

  describe('Enable/Disable Functionality', () => {
    it('should enable auto-save and emit event', () => {
      autoSave.disable(); // First disable it
      
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.AUTO_SAVE_ENABLED, mockCallback);
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      autoSave.enable();

      expect(autoSave.isEnabled).toBe(true);
      expect(mockCallback).toHaveBeenCalled();
      expect(consoleSpy).toHaveBeenCalledWith('AutoSave enabled');
      
      consoleSpy.mockRestore();
    });

    it('should disable auto-save and emit event', () => {
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.AUTO_SAVE_DISABLED, mockCallback);
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      autoSave.disable();

      expect(autoSave.isEnabled).toBe(false);
      expect(mockCallback).toHaveBeenCalled();
      expect(consoleSpy).toHaveBeenCalledWith('AutoSave disabled');
      
      consoleSpy.mockRestore();
    });

    it('should not emit event if already enabled', () => {
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.AUTO_SAVE_ENABLED, mockCallback);

      autoSave.enable(); // Already enabled

      expect(mockCallback).not.toHaveBeenCalled();
    });

    it('should not emit event if already disabled', () => {
      autoSave.disable(); // First disable
      
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.AUTO_SAVE_DISABLED, mockCallback);

      autoSave.disable(); // Already disabled

      expect(mockCallback).not.toHaveBeenCalled();
    });

    it('should cancel pending save when disabling', () => {
      // Schedule an auto-save
      autoSave.handleContentChange('new content');
      expect(autoSave.saveTimeoutId).toBeTruthy();

      autoSave.disable();

      expect(autoSave.saveTimeoutId).toBeNull();
    });
  });

  describe('Save Delay Configuration', () => {
    it('should set valid save delay', () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      autoSave.setSaveDelay(3000);

      expect(autoSave.saveDelay).toBe(3000);
      expect(consoleSpy).toHaveBeenCalledWith('AutoSave delay set to 3000ms');
      
      consoleSpy.mockRestore();
    });

    it('should throw error for invalid save delay', () => {
      expect(() => {
        autoSave.setSaveDelay(100); // Too short
      }).toThrow('Save delay must be at least 500ms');

      expect(() => {
        autoSave.setSaveDelay('not-a-number');
      }).toThrow('Save delay must be at least 500ms');
    });

    it('should accept minimum valid delay', () => {
      autoSave.setSaveDelay(500);
      expect(autoSave.saveDelay).toBe(500);
    });
  });

  describe('Content Getter Configuration', () => {
    it('should set valid content getter function', () => {
      const newGetter = vi.fn(() => 'different content');
      
      autoSave.setContentGetter(newGetter);

      expect(autoSave.getEditorContent).toBe(newGetter);
    });

    it('should throw error for invalid content getter', () => {
      expect(() => {
        autoSave.setContentGetter('not-a-function');
      }).toThrow('Content getter must be a function');

      expect(() => {
        autoSave.setContentGetter(null);
      }).toThrow('Content getter must be a function');
    });
  });

  describe('Content Change Handling', () => {
    it('should mark app as dirty when content changes', () => {
      const markDirtySpy = vi.spyOn(appState, 'markDirty');

      autoSave.handleContentChange('new content');

      expect(markDirtySpy).toHaveBeenCalledWith(true);
    });

    it('should schedule auto-save after content change', () => {
      autoSave.handleContentChange('new content');

      expect(autoSave.saveTimeoutId).toBeTruthy();
    });

    it('should use content getter when no content provided', () => {
      autoSave.handleContentChange(); // No content parameter

      expect(mockContentGetter).toHaveBeenCalled();
    });

    it('should warn when no content and no getter available', () => {
      // Directly set getEditorContent to null to bypass validation
      autoSave.getEditorContent = null;
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      autoSave.handleContentChange();

      expect(consoleSpy).toHaveBeenCalledWith('⚠️ [AutoSave] No content provided and no content getter set');
      
      consoleSpy.mockRestore();
    });

    it('should skip save if content unchanged', () => {
      const content = 'same content';
      autoSave.lastSaveContent = content;
      const markDirtySpy = vi.spyOn(appState, 'markDirty');

      autoSave.handleContentChange(content);

      expect(markDirtySpy).not.toHaveBeenCalled();
      expect(autoSave.saveTimeoutId).toBeNull();
    });

    it('should cancel previous auto-save timer on new change', () => {
      autoSave.handleContentChange('first content');
      const firstTimeoutId = autoSave.saveTimeoutId;

      autoSave.handleContentChange('second content');

      expect(autoSave.saveTimeoutId).not.toBe(firstTimeoutId);
      expect(autoSave.saveTimeoutId).toBeTruthy();
    });

    it('should not schedule auto-save when disabled', () => {
      autoSave.disable();

      autoSave.handleContentChange('new content');

      expect(autoSave.saveTimeoutId).toBeNull();
    });

    it('should handle errors in content change gracefully', () => {
      // Mock content getter to throw error
      mockContentGetter.mockImplementation(() => { throw new Error('Getter failed'); });
      
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, mockCallback);
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      autoSave.handleContentChange();

      expect(consoleSpy).toHaveBeenCalledWith('❌ [AutoSave] Error handling content change:', expect.any(Error));
      expect(mockCallback).toHaveBeenCalledWith(expect.objectContaining({
        error: 'Getter failed'
      }));
      
      consoleSpy.mockRestore();
    });
  });

  describe('Auto-Save Execution', () => {
    it('should trigger auto-save after delay', async () => {
      const performSaveSpy = vi.spyOn(autoSave, 'performSave').mockResolvedValue(true);

      autoSave.handleContentChange('new content');
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);

      await vi.runAllTimersAsync();

      expect(performSaveSpy).toHaveBeenCalledWith(
        '/test/file.md',
        'new content',
        'auto'
      );
    });

    it('should not trigger auto-save if disabled', async () => {
      const performSaveSpy = vi.spyOn(autoSave, 'performSave');
      autoSave.disable();

      autoSave.handleContentChange('new content');
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);

      await vi.runAllTimersAsync();

      expect(performSaveSpy).not.toHaveBeenCalled();
    });

    it('should not auto-save if no file is open', async () => {
      appState.currentFile = null;
      const performSaveSpy = vi.spyOn(autoSave, 'performSave');

      const result = await autoSave.performAutoSave('content');

      expect(result).toBe(false);
      expect(performSaveSpy).not.toHaveBeenCalled();
    });

    it('should not auto-save if already saving', async () => {
      autoSave.isSaving = true;

      const result = await autoSave.performAutoSave('content');

      expect(result).toBe(false);
    });

    it('should increment auto-save statistics on success', async () => {
      vi.spyOn(autoSave, 'performSave').mockResolvedValue(true);

      await autoSave.performAutoSave('content');

      expect(autoSave.saveStats.totalAutoSaves).toBe(1);
    });
  });

  describe('Manual Save (Ctrl+S)', () => {
    it('should perform manual save immediately', async () => {
      const performSaveSpy = vi.spyOn(autoSave, 'performSave').mockResolvedValue(true);

      const result = await autoSave.saveNow();

      expect(result).toBe(true);
      expect(performSaveSpy).toHaveBeenCalledWith(
        '/test/file.md',
        'test content',
        'manual'
      );
    });

    it('should cancel pending auto-save when manual save triggered', async () => {
      autoSave.handleContentChange('content');
      expect(autoSave.saveTimeoutId).toBeTruthy();

      await autoSave.saveNow();

      expect(autoSave.saveTimeoutId).toBeNull();
    });

    it('should throw error when no file is open for manual save', async () => {
      appState.currentFile = null;

      const result = await autoSave.saveNow();

      expect(result).toBe(false);
    });

    it('should throw error when no content getter for manual save', async () => {
      // Directly set getEditorContent to null to bypass validation
      autoSave.getEditorContent = null;

      const result = await autoSave.saveNow();

      expect(result).toBe(false);
    });

    it('should increment manual save statistics on success', async () => {
      vi.spyOn(autoSave, 'performSave').mockResolvedValue(true);

      await autoSave.saveNow();

      expect(autoSave.saveStats.totalManualSaves).toBe(1);
    });

    it('should handle manual save errors gracefully', async () => {
      mockContentGetter.mockImplementation(() => { throw new Error('Content error'); });
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, mockCallback);
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const result = await autoSave.saveNow();

      expect(result).toBe(false);
      expect(mockCallback).toHaveBeenCalledWith(expect.objectContaining({
        error: 'Content error',
        type: 'manual'
      }));
      expect(consoleSpy).toHaveBeenCalledWith('Manual save failed:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });
  });

  describe('Core Save Operation', () => {
    it('should perform successful save operation', async () => {
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, mockCallback);
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, mockCallback);

      const result = await autoSave.performSave('/test.md', 'content', 'manual');

      expect(result).toBe(true);
      expect(tauriMocks.invoke).toHaveBeenCalledWith('write_file', {
        file_path: '/test.md',
        content: 'content'
      });
      expect(mockCallback).toHaveBeenCalledTimes(2); // Started and success events
    });

    it('should use auto_save_file command for auto-saves', async () => {
      await autoSave.performSave('/test.md', 'content', 'auto');

      expect(tauriMocks.invoke).toHaveBeenCalledWith('auto_save_file', {
        file_path: '/test.md',
        content: 'content'
      });
    });

    it('should use write_file command for manual saves', async () => {
      await autoSave.performSave('/test.md', 'content', 'manual');

      expect(tauriMocks.invoke).toHaveBeenCalledWith('write_file', {
        file_path: '/test.md',
        content: 'content'
      });
    });

    it('should update last save content and mark app clean on success', async () => {
      const markDirtySpy = vi.spyOn(appState, 'markDirty');

      await autoSave.performSave('/test.md', 'new content', 'manual');

      expect(autoSave.lastSaveContent).toBe('new content');
      expect(markDirtySpy).toHaveBeenCalledWith(false);
    });

    it('should update save statistics on success', async () => {
      const initialStats = { ...autoSave.saveStats };

      await autoSave.performSave('/test.md', 'content', 'manual');

      expect(autoSave.saveStats.totalSaves).toBe(initialStats.totalSaves + 1);
      expect(autoSave.saveStats.averageSaveTime).toBeGreaterThan(0);
      expect(autoSave.saveStats.lastSaveTime).toBeInstanceOf(Date);
    });

    it('should emit save started and success events', async () => {
      const startedCallback = vi.fn();
      const successCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, startedCallback);
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, successCallback);

      await autoSave.performSave('/test.md', 'content', 'manual');

      expect(startedCallback).toHaveBeenCalledWith(expect.objectContaining({
        filePath: '/test.md',
        saveType: 'manual',
        attempt: 1,
        contentLength: 7
      }));

      expect(successCallback).toHaveBeenCalledWith(expect.objectContaining({
        filePath: '/test.md',
        saveType: 'manual',
        saveTime: expect.any(Number),
        contentLength: 7,
        attempt: 1
      }));
    });

    it('should prevent concurrent saves', async () => {
      let resolveFirst;
      const firstSavePromise = new Promise(resolve => {
        resolveFirst = resolve;
      });
      
      tauriMocks.invoke.mockImplementationOnce(() => firstSavePromise);

      const promise1 = autoSave.performSave('/test.md', 'content1', 'manual');
      const promise2 = autoSave.performSave('/test.md', 'content2', 'manual');

      // Second should return immediately as false
      expect(await promise2).toBe(false);
      
      // Resolve first save
      resolveFirst(true);
      expect(await promise1).toBe(true);
    });
  });

  describe('Error Handling and Retry Logic', () => {
    beforeEach(() => {
      vi.useRealTimers();
    });

    afterEach(() => {
      vi.useFakeTimers();
    });

    it('should retry on save failure', async () => {
      let attemptCount = 0;
      tauriMocks.invoke.mockImplementation(() => {
        attemptCount++;
        if (attemptCount === 1) {
          return Promise.reject(new Error('Temporary failure'));
        }
        return Promise.resolve(true);
      });

      // Add some debug logging
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const result = await autoSave.performSave('/test.md', 'content', 'manual');

      expect(attemptCount).toBe(2); // This should pass
      expect(result).toBe(true);
      
      consoleSpy.mockRestore();
    });

    it('should fail after max retry attempts', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Persistent failure'));
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, mockCallback);

      const result = await autoSave.performSave('/test.md', 'content', 'manual');

      expect(result).toBe(false);
      expect(autoSave.saveStats.saveErrors).toBe(1);
      expect(mockCallback).toHaveBeenCalledWith(expect.objectContaining({
        error: 'Persistent failure',
        maxAttemptsReached: true
      }));
    });

    it('should handle conflict errors without retry', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('File was modified externally'));
      const mockCallback = vi.fn();
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_CONFLICT, mockCallback);

      const result = await autoSave.performSave('/test.md', 'content', 'manual');

      expect(result).toBe(false);
      expect(mockCallback).toHaveBeenCalledWith(expect.objectContaining({
        error: 'File was modified externally',
        saveType: 'manual'
      }));
    });

    it('should detect conflict errors correctly', () => {
      expect(autoSave.isConflictError(new Error('conflict detected'))).toBe(true);
      expect(autoSave.isConflictError(new Error('File modified externally'))).toBe(true);
      expect(autoSave.isConflictError(new Error('File changed externally'))).toBe(true);
      expect(autoSave.isConflictError(new Error('Resource lock error'))).toBe(true);
      expect(autoSave.isConflictError(new Error('Network error'))).toBe(false);
    });

    it('should wait between retry attempts', async () => {
      let attemptCount = 0;
      
      tauriMocks.invoke.mockImplementation(() => {
        attemptCount++;
        if (attemptCount < 2) {
          return Promise.reject(new Error('Temporary failure'));
        }
        return Promise.resolve(true);
      });

      const result = await autoSave.performSave('/test.md', 'content', 'manual');
      
      expect(result).toBe(true);
      expect(attemptCount).toBe(2);
    });
  });

  describe('Event System', () => {
    it('should add and remove event listeners', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();

      autoSave.addEventListener('test_event', callback1);
      autoSave.addEventListener('test_event', callback2);

      expect(autoSave.eventListeners.has('test_event')).toBe(true);
      expect(autoSave.eventListeners.get('test_event').size).toBe(2);

      autoSave.removeEventListener('test_event', callback1);
      expect(autoSave.eventListeners.get('test_event').size).toBe(1);

      autoSave.removeEventListener('test_event', callback2);
      expect(autoSave.eventListeners.has('test_event')).toBe(false);
    });

    it('should throw error for non-function callback', () => {
      expect(() => {
        autoSave.addEventListener('test_event', 'not-a-function');
      }).toThrow('Event callback must be a function');
    });

    it('should emit events with timestamp', () => {
      const callback = vi.fn();
      autoSave.addEventListener('test_event', callback);

      autoSave.emit('test_event', { test: 'data' });

      expect(callback).toHaveBeenCalledWith(expect.objectContaining({
        test: 'data',
        timestamp: expect.stringMatching(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/)
      }));
    });

    it('should handle errors in event listeners gracefully', () => {
      const errorCallback = vi.fn(() => { throw new Error('Listener error'); });
      const successCallback = vi.fn();
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      autoSave.addEventListener('test_event', errorCallback);
      autoSave.addEventListener('test_event', successCallback);

      autoSave.emit('test_event', { test: 'data' });

      expect(errorCallback).toHaveBeenCalled();
      expect(successCallback).toHaveBeenCalled();
      expect(consoleSpy).toHaveBeenCalledWith(
        'Error in AutoSave event listener for test_event:',
        expect.any(Error)
      );
      
      consoleSpy.mockRestore();
    });
  });

  describe('Status and Statistics', () => {
    it('should return current status', () => {
      autoSave.isSaving = true;
      autoSave.saveTimeoutId = 123;

      const status = autoSave.getStatus();

      expect(status).toEqual({
        enabled: true,
        delay: 2000,
        saving: true,
        pendingSave: true,
        hasContentGetter: true,
        currentFile: '/test/file.md',
        isDirty: false,
        stats: autoSave.saveStats
      });
    });

    it('should return copy of statistics', () => {
      const stats = autoSave.getStats();
      
      // Modify returned stats
      stats.totalSaves = 999;
      
      // Original stats should be unchanged
      expect(autoSave.saveStats.totalSaves).toBe(0);
    });

    it('should reset statistics', () => {
      // Set some stats
      autoSave.saveStats.totalSaves = 5;
      autoSave.saveStats.totalAutoSaves = 3;
      autoSave.saveStats.saveErrors = 1;

      autoSave.resetStats();

      expect(autoSave.saveStats).toEqual({
        totalSaves: 0,
        totalAutoSaves: 0,
        totalManualSaves: 0,
        averageSaveTime: 0,
        lastSaveTime: null,
        saveErrors: 0
      });
    });

    it('should calculate moving average for save times', () => {
      autoSave.updateSaveStats(100);
      expect(autoSave.saveStats.averageSaveTime).toBe(100);

      autoSave.updateSaveStats(200);
      const expectedAverage = (100 * 0.8) + (200 * 0.2); // 80 + 40 = 120
      expect(autoSave.saveStats.averageSaveTime).toBe(expectedAverage);
    });
  });

  describe('Force Save', () => {
    it('should skip force save when not dirty', async () => {
      appState.unsavedChanges = false;
      const saveNowSpy = vi.spyOn(autoSave, 'saveNow');

      const result = await autoSave.forceSave();

      expect(result).toBe(true);
      expect(saveNowSpy).not.toHaveBeenCalled();
    });

    it('should perform force save when dirty', async () => {
      appState.unsavedChanges = true;
      const saveNowSpy = vi.spyOn(autoSave, 'saveNow').mockResolvedValue(true);
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const result = await autoSave.forceSave();

      expect(result).toBe(true);
      expect(saveNowSpy).toHaveBeenCalled();
      expect(consoleSpy).toHaveBeenCalledWith('Force save completed successfully');
      
      consoleSpy.mockRestore();
    });

    it('should handle force save errors', async () => {
      appState.unsavedChanges = true;
      vi.spyOn(autoSave, 'saveNow').mockRejectedValue(new Error('Force save failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const result = await autoSave.forceSave();

      expect(result).toBe(false);
      expect(consoleSpy).toHaveBeenCalledWith('Force save failed:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });
  });

  describe('Cleanup and Destruction', () => {
    it('should clean up resources on destroy', () => {
      // Set up some state
      autoSave.handleContentChange('content');
      autoSave.addEventListener('test_event', () => {});
      
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      autoSave.destroy();

      expect(autoSave.saveTimeoutId).toBeNull();
      expect(autoSave.eventListeners.size).toBe(0);
      expect(autoSave.isEnabled).toBe(false);
      expect(autoSave.appState).toBeNull();
      expect(autoSave.getEditorContent).toBeNull();
      expect(consoleSpy).toHaveBeenCalledWith('AutoSave destroyed and cleaned up');
      
      consoleSpy.mockRestore();
    });

    it('should cancel pending saves on destroy', () => {
      autoSave.handleContentChange('content');
      expect(autoSave.saveTimeoutId).toBeTruthy();

      autoSave.destroy();

      expect(autoSave.saveTimeoutId).toBeNull();
    });
  });

  describe('Performance', () => {
    it('should complete save operations efficiently', async () => {
      const startTime = performance.now();
      await autoSave.performSave('/test.md', 'content', 'manual');
      const saveTime = performance.now() - startTime;

      // With mocked Tauri calls, this should be very fast
      expect(saveTime).toBeLessThan(10);
    });

    it('should handle rapid content changes with debouncing', () => {
      // Rapid changes should debounce
      autoSave.handleContentChange('content1');
      autoSave.handleContentChange('content2');
      autoSave.handleContentChange('content3');

      // Only one timer should be active
      expect(autoSave.saveTimeoutId).toBeTruthy();
      
      // Should have marked as dirty
      expect(appState.unsavedChanges).toBe(true);
    });

    it('should track performance metrics accurately', async () => {
      await autoSave.performSave('/test.md', 'content', 'manual');

      expect(autoSave.saveStats.totalSaves).toBe(1);
      expect(autoSave.saveStats.averageSaveTime).toBeGreaterThan(0);
      expect(autoSave.saveStats.lastSaveTime).toBeInstanceOf(Date);
    });
  });

  describe('Integration with AppState', () => {
    it('should respond to app state changes', () => {
      // Test that AutoSave properly integrates with AppState
      expect(autoSave.appState).toBe(appState);
      
      const markDirtySpy = vi.spyOn(appState, 'markDirty');
      autoSave.handleContentChange('new content');
      
      expect(markDirtySpy).toHaveBeenCalledWith(true);
    });

    it('should use current file from app state for saves', async () => {
      appState.currentFile = '/different/file.md';

      await autoSave.performSave(appState.currentFile, 'content', 'manual');

      expect(tauriMocks.invoke).toHaveBeenCalledWith('write_file', {
        file_path: '/different/file.md',
        content: 'content'
      });
    });

    it('should mark app as clean after successful save', async () => {
      const markDirtySpy = vi.spyOn(appState, 'markDirty');

      await autoSave.performSave('/test.md', 'content', 'manual');

      expect(markDirtySpy).toHaveBeenCalledWith(false);
    });
  });
});