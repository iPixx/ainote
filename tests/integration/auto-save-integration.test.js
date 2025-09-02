/**
 * Auto-Save Integration Test
 * 
 * Tests the complete auto-save event chain:
 * MarkdownEditor -> EditorPreviewPanel -> main.js -> AutoSave service -> Tauri backend
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';
import AutoSave from '../../src/js/services/auto-save.js';
import AppState from '../../src/js/state.js';
import EditorPreviewPanel from '../../src/js/components/editor-preview-panel.js';

// Mocks are handled by tests/setup.js - no need to duplicate here

describe('Auto-Save Integration Tests', () => {
  let appState;
  let autoSave;
  let editorPreviewPanel;
  let mockContainer;
  let mockTauriInvoke;

  beforeEach(async () => {
    setupTauriMocks();
    mockTauriInvoke = window.__TAURI__.core.invoke;
    
    // Create mock DOM container
    mockContainer = document.createElement('div');
    mockContainer.id = 'editorContent';
    document.body.appendChild(mockContainer);

    // Initialize services
    appState = new AppState();
    autoSave = new AutoSave(appState);
    
    // Set up test file and vault
    appState.setVault('/test/vault');
    appState.setCurrentFile('/test/vault/test.md');
    
    // Initialize EditorPreviewPanel
    editorPreviewPanel = new EditorPreviewPanel(mockContainer, appState);
    editorPreviewPanel.init();
    
    // Set up auto-save content getter
    autoSave.setContentGetter(() => {
      return editorPreviewPanel ? editorPreviewPanel.getContent() : null;
    });
    
    // Clear any mock calls
    mockTauriInvoke.mockClear();
  });

  afterEach(() => {
    if (mockContainer && mockContainer.parentNode) {
      mockContainer.parentNode.removeChild(mockContainer);
    }
    
    if (autoSave) {
      autoSave.destroy();
    }
  });

  it('should complete full auto-save cycle after content change', async () => {
    // Arrange: Set up success mock for auto_save_file command
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, () => saveEvents.push('success'));

    // Act: Simulate content change in editor
    const newContent = '# Test Content\n\nThis is test content for auto-save integration.';
    editorPreviewPanel.setContent(newContent);
    
    // Simulate the event chain that would happen in real app
    const contentChangeEvent = new CustomEvent('content_changed', {
      detail: { content: newContent, timestamp: Date.now() }
    });
    editorPreviewPanel.dispatchEvent(contentChangeEvent);
    
    // Manually trigger auto-save chain as main.js would do
    if (autoSave) {
      const content = newContent; // This simulates getting content from event detail
      autoSave.handleContentChange(content);
    }

    // Wait for auto-save delay (2 seconds) plus buffer
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: Verify the complete chain worked
    expect(saveEvents).toContain('started');
    expect(saveEvents).toContain('success');
    expect(mockTauriInvoke).toHaveBeenCalledWith('auto_save_file', {
      file_path: '/test/vault/test.md',
      content: newContent
    });
    expect(autoSave.getStats().totalAutoSaves).toBe(1);
  });

  it('should handle blur-triggered save correctly', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, () => saveEvents.push('success'));

    // Act: Set content and trigger blur save
    const content = '# Blur Test\n\nContent that triggers save on blur.';
    editorPreviewPanel.setContent(content);
    
    // Simulate blur event from MarkdownEditor
    const blurEvent = new CustomEvent('save_requested', {
      detail: { content, reason: 'focus_lost', timestamp: Date.now() }
    });
    editorPreviewPanel.dispatchEvent(blurEvent);
    
    // Simulate main.js handling save_requested event
    await autoSave.saveNow();

    // Assert
    expect(saveEvents).toContain('started');
    expect(saveEvents).toContain('success');
    expect(mockTauriInvoke).toHaveBeenCalledWith('write_file', {
      file_path: '/test/vault/test.md',
      content: content
    });
    expect(autoSave.getStats().totalManualSaves).toBe(1);
  });

  it('should not auto-save when content is unchanged', async () => {
    // Arrange
    const originalContent = '# Original Content';
    editorPreviewPanel.setContent(originalContent);
    autoSave.handleContentChange(originalContent); // Set as "last saved content"
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));
    
    // Act: Set the same content again
    autoSave.handleContentChange(originalContent);
    
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: No save should have been triggered
    expect(saveEvents).toHaveLength(0);
    expect(mockTauriInvoke).not.toHaveBeenCalledWith('auto_save_file', expect.any(Object));
  });

  it('should cancel previous auto-save when new content changes arrive', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValue(true);
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));

    // Act: Trigger multiple rapid content changes
    autoSave.handleContentChange('First change');
    autoSave.handleContentChange('Second change');
    autoSave.handleContentChange('Third change');
    
    // Wait for auto-save delay
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: Only one save should have been triggered (for the last change)
    expect(saveEvents).toHaveLength(1);
    expect(mockTauriInvoke).toHaveBeenCalledTimes(1);
    expect(mockTauriInvoke).toHaveBeenCalledWith('auto_save_file', {
      file_path: '/test/vault/test.md',
      content: 'Third change'
    });
  });

  it('should handle save errors gracefully', async () => {
    // Arrange: Mock save failure
    mockTauriInvoke.mockRejectedValueOnce(new Error('Save failed'));
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, (event) => {
      saveEvents.push({ type: 'error', error: event.error });
    });

    // Act: Trigger auto-save
    autoSave.handleContentChange('Content that will fail to save');
    
    await new Promise(resolve => setTimeout(resolve, 2500));

    // Assert: Error should be handled
    expect(saveEvents).toHaveLength(1);
    expect(saveEvents[0].type).toBe('error');
    expect(saveEvents[0].error).toBe('Save failed');
  });

  it('should update app state correctly during save cycle', async () => {
    // Arrange
    mockTauriInvoke.mockResolvedValueOnce(true);
    
    // Act: Trigger content change and auto-save
    const newContent = '# State Test Content';
    autoSave.handleContentChange(newContent);
    
    // Should mark as dirty immediately
    expect(appState.getState().unsavedChanges).toBe(true);
    
    // Wait for auto-save
    await new Promise(resolve => setTimeout(resolve, 2500));
    
    // Assert: Should be marked as clean after successful save
    expect(appState.getState().unsavedChanges).toBe(false);
  });

  it('should respect auto-save enabled/disabled state', async () => {
    // Arrange
    autoSave.disable();
    
    const saveEvents = [];
    autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, () => saveEvents.push('started'));

    // Act: Try to trigger auto-save while disabled
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
  });
});