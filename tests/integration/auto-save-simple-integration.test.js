/**
 * Simple Auto-Save Integration Test
 * 
 * Tests core auto-save functionality without complex UI components
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';
import AutoSave from '../../src/js/services/auto-save.js';
import AppState from '../../src/js/state.js';

describe('Auto-Save Simple Integration Tests', () => {
  let appState;
  let autoSave;
  let mockTauriInvoke;

  beforeEach(() => {
    setupTauriMocks();
    mockTauriInvoke = window.__TAURI__.core.invoke;
    
    // Initialize services
    appState = new AppState();
    autoSave = new AutoSave(appState);
    
    // Set up test file and vault
    appState.setVault('/test/vault');
    appState.setCurrentFile('/test/vault/test.md');
    
    // Clear any mock calls
    mockTauriInvoke.mockClear();
  });

  afterEach(() => {
    if (autoSave) {
      autoSave.destroy();
    }
  });

  it('should complete auto-save after delay when content changes', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, () => saveEvents.push('success'));

    // Act: Trigger auto-save directly
    const testContent = '# Test Content\n\nThis is test content for auto-save.';
    autoSave.handleContentChange(testContent);
    
    // Wait for auto-save delay (2 seconds) plus buffer
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert
    expect(saveEvents).toContain('started');
    expect(saveEvents).toContain('success');
    expect(mockTauriInvoke).toHaveBeenCalledWith('auto_save_file', {
      file_path: '/test/vault/test.md',
      content: testContent
    });
    expect(autoSave.getStats().totalAutoSaves).toBe(1);
  });

  it('should handle manual save immediately', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, () => saveEvents.push('success'));
    
    // Set up content getter
    const testContent = '# Manual Save Test\n\nContent for manual save.';
    autoSave.setContentGetter(() => testContent);

    // Act: Manual save
    const result = await autoSave.saveNow();

    // Assert
    expect(result).toBe(true);
    expect(saveEvents).toContain('started');
    expect(saveEvents).toContain('success');
    expect(mockTauriInvoke).toHaveBeenCalledWith('write_file', {
      file_path: '/test/vault/test.md',
      content: testContent
    });
    expect(autoSave.getStats().totalManualSaves).toBe(1);
  });

  it('should not save when content is unchanged', async () => {
    // Arrange
    const originalContent = '# Original Content';
    autoSave.handleContentChange(originalContent); // First change to set lastSaveContent
    
    await new Promise(resolve => setTimeout(resolve, 2500)); // Wait for first save
    mockTauriInvoke.mockClear(); // Clear the first save call
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));
    
    // Act: Set the same content again
    autoSave.handleContentChange(originalContent);
    
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: No new save should have been triggered
    expect(saveEvents).toHaveLength(0);
    expect(mockTauriInvoke).not.toHaveBeenCalled();
  });

  it('should cancel previous auto-save when new content arrives', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValue(true);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));

    // Act: Multiple rapid changes
    autoSave.handleContentChange('First change');
    autoSave.handleContentChange('Second change'); 
    autoSave.handleContentChange('Final change');
    
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: Only one save for the final content
    expect(saveEvents).toHaveLength(1);
    expect(mockTauriInvoke).toHaveBeenCalledTimes(1);
    expect(mockTauriInvoke).toHaveBeenCalledWith('auto_save_file', {
      file_path: '/test/vault/test.md',
      content: 'Final change'
    });
  });

  it('should handle save errors gracefully', async () => {
    // Arrange - AutoSave retries 3 times, so we need to mock 3 failures
    const testError = new Error('Save operation failed');
    mockTauriInvoke
      .mockRejectedValueOnce(testError)
      .mockRejectedValueOnce(testError)
      .mockRejectedValueOnce(testError);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, (event) => {
      saveEvents.push({ type: 'error', error: event.error });
    });

    // Act
    autoSave.handleContentChange('Content that will fail to save');
    
    // Wait for auto-save delay + 3 retries with 500ms delay each + buffer
    // 2000ms (auto-save delay) + 3 * 500ms (retries) + 1000ms (buffer) = 4500ms
    await new Promise(resolve => setTimeout(resolve, 4500));

    // Assert
    expect(saveEvents).toHaveLength(1);
    expect(saveEvents[0].type).toBe('error');
    expect(saveEvents[0].error).toBe('Save operation failed');
  });

  it('should respect enabled/disabled state', async () => {
    // Arrange - clear previous mock calls from other tests
    mockTauriInvoke.mockClear();
    autoSave.disable();
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));

    // Act: Try to auto-save while disabled
    autoSave.handleContentChange('Content while disabled');
    
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: No save should occur
    expect(saveEvents).toHaveLength(0);
    expect(mockTauriInvoke).not.toHaveBeenCalled();
    
    // Re-enable and test
    autoSave.enable();
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    autoSave.handleContentChange('Content while enabled');
    await new Promise(resolve => setTimeout(resolve, 2500));
    
    expect(saveEvents).toHaveLength(1);
    expect(mockTauriInvoke).toHaveBeenCalledWith('auto_save_file', {
      file_path: '/test/vault/test.md',
      content: 'Content while enabled'
    });
  });

  it('should update app state during save cycle', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    // Act
    const newContent = '# State Test Content';
    autoSave.handleContentChange(newContent);
    
    // Should mark as dirty immediately
    expect(appState.getState().unsavedChanges).toBe(true);
    
    // Wait for auto-save
    await new Promise(resolve => setTimeout(resolve, 2500));
    
    // Assert: Should be marked as clean after successful save
    expect(appState.getState().unsavedChanges).toBe(false);
  });
});