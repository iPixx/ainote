/**
 * AppState - Centralized state management for aiNote application
 * 
 * Manages application state with persistence and event-driven updates.
 * Follows local-first principles with localStorage integration.
 * 
 * @class AppState
 */
class AppState {
  /**
   * State change event types
   */
  static EVENTS = {
    VAULT_CHANGED: 'vault_changed',
    FILE_CHANGED: 'file_changed',
    VIEW_MODE_CHANGED: 'view_mode_changed',
    DIRTY_STATE_CHANGED: 'dirty_state_changed',
    FILES_UPDATED: 'files_updated'
  };

  /**
   * View modes for editor/preview toggle
   */
  static VIEW_MODES = {
    EDITOR: 'editor',
    PREVIEW: 'preview'
  };

  /**
   * Initialize AppState with default values
   */
  constructor() {
    // Core state properties
    this.currentVault = null;
    this.currentFile = null;
    this.viewMode = AppState.VIEW_MODES.EDITOR;
    this.unsavedChanges = false;
    this.files = [];

    // Event system for component communication
    this.eventListeners = new Map();

    // Initialize state from localStorage if available
    this.loadState();
  }

  /**
   * Set the current vault path and persist to localStorage
   * @param {string|null} vaultPath - Path to the vault directory
   */
  setVault(vaultPath) {
    if (this.currentVault === vaultPath) return;

    const previousVault = this.currentVault;
    this.currentVault = vaultPath;

    // Clear current file when vault changes
    if (previousVault !== vaultPath) {
      this.currentFile = null;
      this.files = [];
      this.unsavedChanges = false;
    }

    this.saveState();
    this.emit(AppState.EVENTS.VAULT_CHANGED, { 
      vault: vaultPath, 
      previousVault 
    });
  }

  /**
   * Set the current file path and persist to localStorage
   * @param {string|null} filePath - Path to the current file
   */
  setCurrentFile(filePath) {
    if (this.currentFile === filePath) return;

    const previousFile = this.currentFile;
    this.currentFile = filePath;

    // Reset unsaved changes when switching files
    if (previousFile !== filePath) {
      this.unsavedChanges = false;
    }

    this.saveState();
    this.emit(AppState.EVENTS.FILE_CHANGED, { 
      file: filePath, 
      previousFile 
    });
  }

  /**
   * Toggle between editor and preview modes
   * @returns {string} The new view mode
   */
  toggleViewMode() {
    const newMode = this.viewMode === AppState.VIEW_MODES.EDITOR 
      ? AppState.VIEW_MODES.PREVIEW 
      : AppState.VIEW_MODES.EDITOR;

    this.setViewMode(newMode);
    return newMode;
  }

  /**
   * Set the view mode explicitly
   * @param {string} mode - View mode (editor or preview)
   */
  setViewMode(mode) {
    if (!Object.values(AppState.VIEW_MODES).includes(mode)) {
      throw new Error(`Invalid view mode: ${mode}`);
    }

    if (this.viewMode === mode) return;

    const previousMode = this.viewMode;
    this.viewMode = mode;

    this.saveState();
    this.emit(AppState.EVENTS.VIEW_MODE_CHANGED, { 
      mode, 
      previousMode 
    });
  }

  /**
   * Mark or unmark the application as having unsaved changes
   * @param {boolean} isDirty - Whether there are unsaved changes
   */
  markDirty(isDirty = true) {
    if (this.unsavedChanges === isDirty) return;

    this.unsavedChanges = isDirty;
    this.saveState();
    this.emit(AppState.EVENTS.DIRTY_STATE_CHANGED, { 
      isDirty, 
      file: this.currentFile 
    });
  }

  /**
   * Update the files list for the current vault
   * @param {Array} fileList - Array of file objects from vault scan
   */
  setFiles(fileList) {
    if (!Array.isArray(fileList)) {
      throw new Error('Files must be an array');
    }

    this.files = [...fileList];
    this.emit(AppState.EVENTS.FILES_UPDATED, { 
      files: this.files, 
      count: this.files.length 
    });
  }

  /**
   * Get current state as a plain object
   * @returns {Object} Current state snapshot
   */
  getState() {
    return {
      currentVault: this.currentVault,
      currentFile: this.currentFile,
      viewMode: this.viewMode,
      unsavedChanges: this.unsavedChanges,
      files: [...this.files]
    };
  }

  /**
   * Save current state to localStorage
   * Performance target: <5ms
   */
  saveState() {
    try {
      const state = {
        currentVault: this.currentVault,
        currentFile: this.currentFile,
        viewMode: this.viewMode,
        // Note: Don't persist unsavedChanges or files array
        // These should be reset on application restart
      };

      localStorage.setItem('aiNote_appState', JSON.stringify(state));
    } catch (error) {
      console.error('Failed to save state to localStorage:', error);
    }
  }

  /**
   * Load state from localStorage
   * Restores vault, file, and view mode preferences
   */
  loadState() {
    try {
      const savedState = localStorage.getItem('aiNote_appState');
      if (!savedState) return;

      const state = JSON.parse(savedState);

      // Validate and restore state properties
      if (state.currentVault && typeof state.currentVault === 'string') {
        this.currentVault = state.currentVault;
      }

      if (state.currentFile && typeof state.currentFile === 'string') {
        this.currentFile = state.currentFile;
      }

      if (state.viewMode && Object.values(AppState.VIEW_MODES).includes(state.viewMode)) {
        this.viewMode = state.viewMode;
      }

    } catch (error) {
      console.error('Failed to load state from localStorage:', error);
      // Reset to defaults on error
      this.currentVault = null;
      this.currentFile = null;
      this.viewMode = AppState.VIEW_MODES.EDITOR;
    }
  }

  /**
   * Add event listener for state changes
   * @param {string} event - Event type from AppState.EVENTS
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
   * @param {string} event - Event type from AppState.EVENTS
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
   * Performance target: <1ms for state updates
   * @param {string} event - Event type
   * @param {Object} data - Event data
   */
  emit(event, data = {}) {
    const listeners = this.eventListeners.get(event);
    if (!listeners) return;

    // Execute all listeners synchronously for immediate UI updates
    listeners.forEach(callback => {
      try {
        callback(data);
      } catch (error) {
        console.error(`Error in event listener for ${event}:`, error);
      }
    });
  }

  /**
   * Clear all state and reset to defaults
   * Useful for logout or vault switching
   */
  reset() {
    this.currentVault = null;
    this.currentFile = null;
    this.viewMode = AppState.VIEW_MODES.EDITOR;
    this.unsavedChanges = false;
    this.files = [];

    // Clear localStorage
    try {
      localStorage.removeItem('aiNote_appState');
    } catch (error) {
      console.error('Failed to clear localStorage:', error);
    }

    // Emit reset events
    this.emit(AppState.EVENTS.VAULT_CHANGED, { vault: null, previousVault: null });
    this.emit(AppState.EVENTS.FILE_CHANGED, { file: null, previousFile: null });
    this.emit(AppState.EVENTS.VIEW_MODE_CHANGED, { mode: AppState.VIEW_MODES.EDITOR });
    this.emit(AppState.EVENTS.DIRTY_STATE_CHANGED, { isDirty: false });
    this.emit(AppState.EVENTS.FILES_UPDATED, { files: [], count: 0 });
  }

  /**
   * Validate state integrity
   * @returns {boolean} True if state is valid
   */
  isValid() {
    // Check that view mode is valid
    if (!Object.values(AppState.VIEW_MODES).includes(this.viewMode)) {
      return false;
    }

    // Check that unsavedChanges is boolean
    if (typeof this.unsavedChanges !== 'boolean') {
      return false;
    }

    // Check that files is an array
    if (!Array.isArray(this.files)) {
      return false;
    }

    return true;
  }
}

// Export for ES6 module usage
export default AppState;