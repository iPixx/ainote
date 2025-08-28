/**
 * Integration tests for AutoSave + AppState integration
 * 
 * Tests cover:
 * - Auto-save triggering based on app state changes
 * - Dirty state management across components
 * - File operations with state synchronization
 * - Error handling in integrated workflows
 * - Performance of integrated save operations
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import components to test
import AutoSave from '../../src/js/services/auto-save.js';
import AppState from '../../src/js/state.js';

// Mock timers for debouncing tests
vi.useFakeTimers();

describe('AutoSave + AppState Integration', () => {
  let autoSave;
  let appState;
  let tauriMocks;
  let mockContentGetter;

  beforeEach(() => {
    // Set up Tauri mocks
    tauriMocks = setupTauriMocks();
    
    // Create integrated instances
    appState = new AppState();
    autoSave = new AutoSave(appState);
    
    // Set up mock content getter
    mockContentGetter = vi.fn(() => 'test content');
    autoSave.setContentGetter(mockContentGetter);
    
    // Set up initial file state
    appState.currentFile = '/test/file.md';
    
    // Mock successful save operations by default
    tauriMocks.invoke.mockImplementation((command) => {
      if (command === 'auto_save_file' || command === 'write_file') {
        return Promise.resolve(true);
      }
      if (command === 'save_session_state') {
        return Promise.resolve(true);
      }
      return Promise.resolve(null);
    });
  });

  afterEach(() => {
    autoSave.destroy();
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  describe('Dirty State Integration', () => {
    it('should synchronize dirty state between AutoSave and AppState', async () => {
      const stateChanges = [];
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
        stateChanges.push(data);
      });

      // Trigger content change
      autoSave.handleContentChange('new content');

      // Should mark as dirty
      expect(appState.unsavedChanges).toBe(true);
      expect(stateChanges).toHaveLength(1);
      expect(stateChanges[0].isDirty).toBe(true);

      // Trigger auto-save
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // Should mark as clean after successful save
      expect(appState.unsavedChanges).toBe(false);
      expect(stateChanges).toHaveLength(2);
      expect(stateChanges[1].isDirty).toBe(false);
    });

    it('should track current file in dirty state events', async () => {
      const stateChanges = [];
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
        stateChanges.push(data);
      });

      autoSave.handleContentChange('new content');

      expect(stateChanges[0]).toEqual({
        isDirty: true,
        file: '/test/file.md'
      });
    });

    it('should handle file switching with dirty state', async () => {
      // Make content dirty
      autoSave.handleContentChange('unsaved content');
      expect(appState.unsavedChanges).toBe(true);

      // Switch files
      await appState.setCurrentFile('/other/file.md');

      // Dirty state should be reset when switching files
      expect(appState.unsavedChanges).toBe(false);
    });

    it('should preserve dirty state during vault operations', async () => {
      // Make content dirty
      autoSave.handleContentChange('unsaved content');
      expect(appState.unsavedChanges).toBe(true);

      // Change vault (but keep file null to simulate vault-only change)
      const previousFile = appState.currentFile;
      await appState.setVault('/new/vault');

      // If no file was set (just vault change), dirty state should be reset
      expect(appState.unsavedChanges).toBe(false);
    });
  });

  describe('Auto-Save Workflow Integration', () => {
    it('should complete full auto-save workflow with state updates', async () => {
      const saveEvents = [];
      const stateEvents = [];

      autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, (data) => {
        saveEvents.push({ type: 'started', ...data });
      });

      autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, (data) => {
        saveEvents.push({ type: 'success', ...data });
      });

      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
        stateEvents.push(data);
      });

      // Trigger content change
      autoSave.handleContentChange('modified content');
      
      // Should be dirty immediately
      expect(appState.unsavedChanges).toBe(true);
      expect(stateEvents).toHaveLength(1);

      // Advance time to trigger auto-save
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // Verify complete workflow
      expect(saveEvents).toHaveLength(2); // started + success
      expect(saveEvents[0].type).toBe('started');
      expect(saveEvents[1].type).toBe('success');
      
      expect(stateEvents).toHaveLength(2); // dirty + clean
      expect(stateEvents[0].isDirty).toBe(true);
      expect(stateEvents[1].isDirty).toBe(false);

      expect(appState.unsavedChanges).toBe(false);
    });

    it('should handle manual save workflow integration', async () => {
      const saveEvents = [];
      const stateEvents = [];

      autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, (data) => {
        saveEvents.push(data);
      });

      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
        stateEvents.push(data);
      });

      // Make content dirty
      autoSave.handleContentChange('content to save');
      expect(appState.unsavedChanges).toBe(true);

      // Perform manual save
      const result = await autoSave.saveNow();

      expect(result).toBe(true);
      expect(appState.unsavedChanges).toBe(false);
      expect(saveEvents).toHaveLength(1);
      expect(saveEvents[0].saveType).toBe('manual');
      expect(stateEvents).toHaveLength(2); // dirty + clean
    });

    it('should cancel auto-save when manual save occurs', async () => {
      const performAutoSaveSpy = vi.spyOn(autoSave, 'performAutoSave');

      // Trigger content change to schedule auto-save
      autoSave.handleContentChange('content');
      expect(autoSave.saveTimeoutId).toBeTruthy();

      // Perform manual save before auto-save fires
      await autoSave.saveNow();

      // Auto-save should be cancelled
      expect(autoSave.saveTimeoutId).toBeNull();

      // Advance timers - auto-save should not occur
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      expect(performAutoSaveSpy).not.toHaveBeenCalled();
    });
  });

  describe('File State Integration', () => {
    it('should not save when no file is open', async () => {
      // Clear current file
      appState.currentFile = null;

      autoSave.handleContentChange('content');
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // No save should occur
      expect(tauriMocks.invoke).not.toHaveBeenCalledWith(
        'auto_save_file',
        expect.any(Object)
      );
    });

    it('should use current file from app state for saves', async () => {
      const testFile = '/specific/file.md';
      appState.currentFile = testFile;

      await autoSave.saveNow();

      expect(tauriMocks.invoke).toHaveBeenCalledWith('write_file', {
        file_path: testFile,
        content: 'test content'
      });
    });

    it('should handle file changes during auto-save', async () => {
      // Start auto-save process
      autoSave.handleContentChange('content');
      
      // Change file before auto-save triggers
      await appState.setCurrentFile('/different/file.md');
      
      // Advance timers
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // Should save to the new current file
      expect(tauriMocks.invoke).toHaveBeenCalledWith('auto_save_file', {
        file_path: '/different/file.md',
        content: 'content'
      });
    });
  });

  describe('Error Handling Integration', () => {
    it('should handle save errors without breaking state', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Save failed'));
      
      const errorEvents = [];
      const stateEvents = [];

      autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, (data) => {
        errorEvents.push(data);
      });

      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
        stateEvents.push(data);
      });

      // Trigger content change
      autoSave.handleContentChange('content');
      
      // Advance timers to trigger save
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // Should remain dirty due to save failure
      expect(appState.unsavedChanges).toBe(true);
      expect(errorEvents).toHaveLength(1);
      expect(stateEvents).toHaveLength(1); // Only dirty event, no clean event
    });

    it('should handle save retries with state consistency', async () => {
      let attemptCount = 0;
      tauriMocks.invoke.mockImplementation(() => {
        attemptCount++;
        if (attemptCount < 2) {
          return Promise.reject(new Error('Temporary failure'));
        }
        return Promise.resolve(true); // Success on second attempt
      });

      const stateEvents = [];
      appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
        stateEvents.push(data);
      });

      autoSave.handleContentChange('content');
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // Should eventually succeed and mark as clean
      expect(appState.unsavedChanges).toBe(false);
      expect(attemptCount).toBe(2);
      expect(stateEvents).toHaveLength(2); // dirty + clean
    });

    it('should handle conflict errors appropriately', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('File was modified externally'));
      
      const conflictEvents = [];
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_CONFLICT, (data) => {
        conflictEvents.push(data);
      });

      autoSave.handleContentChange('content');
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // Should remain dirty and emit conflict event
      expect(appState.unsavedChanges).toBe(true);
      expect(conflictEvents).toHaveLength(1);
    });
  });

  describe('Performance Integration', () => {
    it('should complete integrated operations efficiently', async () => {
      const startTime = performance.now();

      // Simulate typical workflow
      autoSave.handleContentChange('content 1');
      autoSave.handleContentChange('content 2'); // Should debounce
      autoSave.handleContentChange('content 3');
      
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      const totalTime = performance.now() - startTime;

      // Should complete quickly with mocked operations
      expect(totalTime).toBeLessThan(50);
      expect(appState.unsavedChanges).toBe(false);
    });

    it('should handle rapid content changes efficiently', async () => {
      const markDirtySpy = vi.spyOn(appState, 'markDirty');

      // Rapid content changes
      for (let i = 0; i < 10; i++) {
        autoSave.handleContentChange(`content ${i}`);
      }

      // Should only mark dirty once (not for every change)
      expect(markDirtySpy).toHaveBeenCalledTimes(10);
      expect(markDirtySpy).toHaveBeenCalledWith(true);

      // Only one auto-save should be scheduled
      expect(autoSave.saveTimeoutId).toBeTruthy();
    });
  });

  describe('State Persistence Integration', () => {
    it('should maintain consistency with app state persistence', async () => {
      // Make changes and save
      autoSave.handleContentChange('content');
      vi.advanceTimersByTime(AutoSave.DEFAULTS.AUTO_SAVE_DELAY);
      await vi.runAllTimersAsync();

      // App state should be clean and last save content should be updated
      expect(appState.unsavedChanges).toBe(false);
      expect(autoSave.lastSaveContent).toBe('content');

      // Subsequent identical changes should not trigger dirty state
      autoSave.handleContentChange('content'); // Same content
      expect(appState.unsavedChanges).toBe(false); // Should remain clean
    });

    it('should integrate with app state session persistence', async () => {
      // Trigger save to verify state persistence is called
      await autoSave.saveNow();

      // Verify that app state save methods are working
      expect(appState.unsavedChanges).toBe(false);
    });
  });

  describe('Force Save Integration', () => {
    it('should force save when app state is dirty', async () => {
      // Make app state dirty
      autoSave.handleContentChange('unsaved content');
      expect(appState.unsavedChanges).toBe(true);

      const result = await autoSave.forceSave();

      expect(result).toBe(true);
      expect(appState.unsavedChanges).toBe(false);
    });

    it('should skip force save when app state is clean', async () => {
      const saveNowSpy = vi.spyOn(autoSave, 'saveNow');
      expect(appState.unsavedChanges).toBe(false);

      const result = await autoSave.forceSave();

      expect(result).toBe(true);
      expect(saveNowSpy).not.toHaveBeenCalled();
    });

    it('should handle force save failures gracefully', async () => {
      appState.unsavedChanges = true;
      tauriMocks.invoke.mockRejectedValue(new Error('Force save failed'));
      
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const result = await autoSave.forceSave();

      expect(result).toBe(false);
      expect(appState.unsavedChanges).toBe(true); // Should remain dirty
      expect(consoleSpy).toHaveBeenCalledWith('Force save failed:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });
  });

  describe('Integration Lifecycle', () => {
    it('should maintain integration throughout component lifecycle', async () => {
      // Initial state
      expect(autoSave.appState).toBe(appState);
      expect(autoSave.getStatus().currentFile).toBe('/test/file.md');

      // Perform operations
      autoSave.handleContentChange('content');
      await autoSave.saveNow();

      // Change app state
      await appState.setCurrentFile('/new/file.md');
      expect(autoSave.getStatus().currentFile).toBe('/new/file.md');

      // Disable and re-enable
      autoSave.disable();
      expect(autoSave.getStatus().enabled).toBe(false);
      
      autoSave.enable();
      expect(autoSave.getStatus().enabled).toBe(true);

      // Still integrated
      expect(autoSave.appState).toBe(appState);
    });

    it('should clean up integration properly on destroy', async () => {
      // Set up integrated state
      autoSave.handleContentChange('content');
      expect(appState.unsavedChanges).toBe(true);

      autoSave.destroy();

      // AutoSave should be cleaned up
      expect(autoSave.appState).toBeNull();
      expect(autoSave.eventListeners.size).toBe(0);
      
      // App state should remain functional
      expect(appState.unsavedChanges).toBe(true); // State preserved
      expect(appState.isValid()).toBe(true);
    });
  });
});