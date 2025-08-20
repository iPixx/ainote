/**
 * AutoSave - Handles automatic saving of file content with debouncing
 * 
 * Provides debounced auto-save functionality, manual save operations,
 * and conflict detection for file modifications. Integrates with AppState
 * for tracking dirty state and file operations.
 * 
 * @class AutoSave
 */
class AutoSave {
  /**
   * AutoSave default configuration
   */
  static DEFAULTS = {
    AUTO_SAVE_DELAY: 2000, // 2 seconds default delay
    MAX_RETRY_ATTEMPTS: 3,
    RETRY_DELAY: 500, // 500ms retry delay
  };

  /**
   * AutoSave events for external listeners
   */
  static EVENTS = {
    SAVE_STARTED: 'save_started',
    SAVE_SUCCESS: 'save_success',
    SAVE_ERROR: 'save_error',
    SAVE_CONFLICT: 'save_conflict',
    AUTO_SAVE_ENABLED: 'auto_save_enabled',
    AUTO_SAVE_DISABLED: 'auto_save_disabled'
  };

  /**
   * Initialize AutoSave with dependencies
   * @param {AppState} appState - Application state management instance
   */
  constructor(appState) {
    if (!appState) {
      throw new Error('AppState instance is required for AutoSave');
    }

    this.appState = appState;
    this.saveDelay = AutoSave.DEFAULTS.AUTO_SAVE_DELAY;
    this.isEnabled = true;
    this.saveTimeoutId = null;
    this.isSaving = false;
    this.lastSaveContent = null;
    
    // Event system for save notifications
    this.eventListeners = new Map();
    
    // Current editor content accessor (set by editor component)
    this.getEditorContent = null;
    
    // Performance tracking
    this.saveStats = {
      totalSaves: 0,
      totalAutoSaves: 0,
      totalManualSaves: 0,
      averageSaveTime: 0,
      lastSaveTime: null,
      saveErrors: 0
    };

    // Setup keyboard shortcut listener for manual save
    this.setupKeyboardShortcuts();
  }

  /**
   * Enable auto-save functionality
   */
  enable() {
    if (this.isEnabled) return;
    
    this.isEnabled = true;
    this.emit(AutoSave.EVENTS.AUTO_SAVE_ENABLED);
    console.log('AutoSave enabled');
  }

  /**
   * Disable auto-save functionality
   * Cancels any pending auto-save operations
   */
  disable() {
    if (!this.isEnabled) return;
    
    this.isEnabled = false;
    this.cancelPendingSave();
    this.emit(AutoSave.EVENTS.AUTO_SAVE_DISABLED);
    console.log('AutoSave disabled');
  }

  /**
   * Set the auto-save delay in milliseconds
   * @param {number} delayMs - Delay in milliseconds (minimum 500ms)
   */
  setSaveDelay(delayMs) {
    if (typeof delayMs !== 'number' || delayMs < 500) {
      throw new Error('Save delay must be at least 500ms');
    }
    
    this.saveDelay = delayMs;
    console.log(`AutoSave delay set to ${delayMs}ms`);
  }

  /**
   * Set the function to get current editor content
   * @param {Function} getContentFn - Function that returns current editor content
   */
  setContentGetter(getContentFn) {
    if (typeof getContentFn !== 'function') {
      throw new Error('Content getter must be a function');
    }
    
    this.getEditorContent = getContentFn;
  }

  /**
   * Handle content change event from editor
   * Triggers debounced auto-save if enabled
   * @param {string} newContent - New editor content
   */
  handleContentChange(newContent = null) {
    try {
      // Get content from getter function or parameter
      const content = newContent || (this.getEditorContent ? this.getEditorContent() : null);
      
      if (content === null) {
        console.warn('No content provided and no content getter set');
        return;
      }

      // Check if content actually changed
      if (content === this.lastSaveContent) {
        return; // No changes, skip auto-save
      }

      // Mark application as dirty
      this.appState.markDirty(true);

      // Cancel previous auto-save timer
      this.cancelPendingSave();

      // Only schedule auto-save if enabled
      if (this.isEnabled) {
        this.saveTimeoutId = setTimeout(() => {
          this.performAutoSave(content);
        }, this.saveDelay);
      }

    } catch (error) {
      console.error('Error handling content change:', error);
      this.emit(AutoSave.EVENTS.SAVE_ERROR, { error: error.message });
    }
  }

  /**
   * Perform manual save immediately (Ctrl+S)
   * @returns {Promise<boolean>} True if save was successful
   */
  async saveNow() {
    try {
      // Get current content
      const content = this.getEditorContent ? this.getEditorContent() : null;
      
      if (content === null) {
        throw new Error('Cannot save: No content getter configured');
      }

      // Get current file path
      const currentFile = this.appState.currentFile;
      if (!currentFile) {
        throw new Error('Cannot save: No file currently open');
      }

      // Cancel any pending auto-save
      this.cancelPendingSave();

      // Perform manual save
      const success = await this.performSave(currentFile, content, 'manual');
      
      if (success) {
        this.saveStats.totalManualSaves++;
      }
      
      return success;

    } catch (error) {
      console.error('Manual save failed:', error);
      this.emit(AutoSave.EVENTS.SAVE_ERROR, { 
        error: error.message,
        type: 'manual'
      });
      return false;
    }
  }

  /**
   * Perform auto-save operation
   * @param {string} content - Content to save
   * @returns {Promise<boolean>} True if save was successful
   */
  async performAutoSave(content) {
    if (!this.isEnabled || this.isSaving) {
      return false;
    }

    try {
      const currentFile = this.appState.currentFile;
      if (!currentFile) {
        return false; // No file open, skip auto-save
      }

      const success = await this.performSave(currentFile, content, 'auto');
      
      if (success) {
        this.saveStats.totalAutoSaves++;
      }
      
      return success;

    } catch (error) {
      console.error('Auto-save failed:', error);
      this.emit(AutoSave.EVENTS.SAVE_ERROR, { 
        error: error.message,
        type: 'auto'
      });
      return false;
    }
  }

  /**
   * Core save operation with retry logic and conflict detection
   * @param {string} filePath - Path to file to save
   * @param {string} content - Content to save
   * @param {string} saveType - Type of save ('auto' or 'manual')
   * @param {number} attempt - Current retry attempt (internal)
   * @returns {Promise<boolean>} True if save was successful
   */
  async performSave(filePath, content, saveType = 'auto', attempt = 1) {
    if (this.isSaving) {
      console.log('Save already in progress, skipping');
      return false;
    }

    this.isSaving = true;
    const saveStartTime = performance.now();

    try {
      // Emit save started event
      this.emit(AutoSave.EVENTS.SAVE_STARTED, { 
        filePath, 
        saveType, 
        attempt,
        contentLength: content.length 
      });

      // Use auto_save_file command for auto-saves, write_file for manual saves
      const command = saveType === 'auto' ? 'auto_save_file' : 'write_file';
      
      await window.__TAURI__.core.invoke(command, {
        filePath: filePath,
        content: content
      });

      // Save successful
      const saveTime = performance.now() - saveStartTime;
      this.updateSaveStats(saveTime);
      
      // Update last saved content
      this.lastSaveContent = content;
      
      // Mark application as clean
      this.appState.markDirty(false);

      this.emit(AutoSave.EVENTS.SAVE_SUCCESS, {
        filePath,
        saveType,
        saveTime,
        contentLength: content.length,
        attempt
      });

      console.log(`${saveType} save completed in ${saveTime.toFixed(2)}ms`);
      return true;

    } catch (error) {
      console.error(`${saveType} save failed (attempt ${attempt}):`, error);
      
      // Check if this is a conflict error
      if (this.isConflictError(error)) {
        this.emit(AutoSave.EVENTS.SAVE_CONFLICT, {
          filePath,
          error: error.message,
          saveType,
          attempt
        });
        return false; // Don't retry conflicts automatically
      }

      // Retry logic for other errors
      if (attempt < AutoSave.DEFAULTS.MAX_RETRY_ATTEMPTS) {
        console.log(`Retrying ${saveType} save in ${AutoSave.DEFAULTS.RETRY_DELAY}ms...`);
        
        // Wait before retry
        await new Promise(resolve => setTimeout(resolve, AutoSave.DEFAULTS.RETRY_DELAY));
        
        return this.performSave(filePath, content, saveType, attempt + 1);
      }

      // Max retries exceeded
      this.saveStats.saveErrors++;
      this.emit(AutoSave.EVENTS.SAVE_ERROR, {
        filePath,
        error: error.message,
        saveType,
        attempt,
        maxAttemptsReached: true
      });

      return false;

    } finally {
      this.isSaving = false;
    }
  }

  /**
   * Check if an error indicates a file conflict
   * @param {Error} error - Error to check
   * @returns {boolean} True if error indicates a conflict
   */
  isConflictError(error) {
    const conflictKeywords = ['conflict', 'modified', 'changed externally', 'lock'];
    const errorMessage = error.message?.toLowerCase() || '';
    return conflictKeywords.some(keyword => errorMessage.includes(keyword));
  }

  /**
   * Cancel any pending auto-save operation
   */
  cancelPendingSave() {
    if (this.saveTimeoutId) {
      clearTimeout(this.saveTimeoutId);
      this.saveTimeoutId = null;
    }
  }

  /**
   * Setup keyboard shortcuts for manual save
   */
  setupKeyboardShortcuts() {
    document.addEventListener('keydown', (event) => {
      // Ctrl+S (or Cmd+S on Mac) for manual save
      if ((event.ctrlKey || event.metaKey) && event.key === 's') {
        event.preventDefault();
        this.saveNow();
      }
    });
  }

  /**
   * Update save statistics
   * @param {number} saveTime - Time taken for the save operation
   */
  updateSaveStats(saveTime) {
    this.saveStats.totalSaves++;
    this.saveStats.lastSaveTime = new Date();
    
    // Update average save time (moving average)
    if (this.saveStats.averageSaveTime === 0) {
      this.saveStats.averageSaveTime = saveTime;
    } else {
      this.saveStats.averageSaveTime = 
        (this.saveStats.averageSaveTime * 0.8) + (saveTime * 0.2);
    }
  }

  /**
   * Add event listener for save events
   * @param {string} event - Event type from AutoSave.EVENTS
   * @param {Function} callback - Event handler function
   */
  addEventListener(event, callback) {
    if (typeof callback !== 'function') {
      throw new Error('Event callback must be a function');
    }

    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }

    this.eventListeners.get(event).add(callback);
  }

  /**
   * Remove event listener
   * @param {string} event - Event type from AutoSave.EVENTS
   * @param {Function} callback - Event handler function to remove
   */
  removeEventListener(event, callback) {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.delete(callback);
      if (listeners.size === 0) {
        this.eventListeners.delete(event);
      }
    }
  }

  /**
   * Emit event to all registered listeners
   * @param {string} event - Event type
   * @param {Object} data - Event data
   */
  emit(event, data = {}) {
    const listeners = this.eventListeners.get(event);
    if (!listeners) return;

    // Execute all listeners synchronously for immediate feedback
    listeners.forEach(callback => {
      try {
        callback({ ...data, timestamp: new Date().toISOString() });
      } catch (error) {
        console.error(`Error in AutoSave event listener for ${event}:`, error);
      }
    });
  }

  /**
   * Get current auto-save status
   * @returns {Object} Status object with current state
   */
  getStatus() {
    return {
      enabled: this.isEnabled,
      delay: this.saveDelay,
      saving: this.isSaving,
      pendingSave: this.saveTimeoutId !== null,
      hasContentGetter: this.getEditorContent !== null,
      currentFile: this.appState.currentFile,
      isDirty: this.appState.unsavedChanges,
      stats: { ...this.saveStats }
    };
  }

  /**
   * Get save statistics
   * @returns {Object} Copy of save statistics
   */
  getStats() {
    return { ...this.saveStats };
  }

  /**
   * Reset save statistics
   */
  resetStats() {
    this.saveStats = {
      totalSaves: 0,
      totalAutoSaves: 0,
      totalManualSaves: 0,
      averageSaveTime: 0,
      lastSaveTime: null,
      saveErrors: 0
    };
  }

  /**
   * Cleanup auto-save resources
   * Call this when the AutoSave instance is no longer needed
   */
  destroy() {
    // Cancel pending saves
    this.cancelPendingSave();
    
    // Clear event listeners
    this.eventListeners.clear();
    
    // Remove keyboard listener (cannot be easily removed, but flag for cleanup)
    this.isEnabled = false;
    
    // Clear references
    this.appState = null;
    this.getEditorContent = null;
    
    console.log('AutoSave destroyed and cleaned up');
  }

  /**
   * Force save current content if dirty
   * Useful for application shutdown or vault switching
   * @returns {Promise<boolean>} True if save was successful
   */
  async forceSave() {
    if (!this.appState.unsavedChanges) {
      return true; // Nothing to save
    }

    try {
      const success = await this.saveNow();
      if (success) {
        console.log('Force save completed successfully');
      }
      return success;
    } catch (error) {
      console.error('Force save failed:', error);
      return false;
    }
  }
}

// Export for ES6 module usage
export default AutoSave;