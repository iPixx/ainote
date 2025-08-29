/**
 * Navigation Service - Handles navigation between notes and files
 * 
 * Provides centralized navigation functionality for the suggestion system,
 * including file opening, content positioning, and smooth transitions.
 * Integrates with the existing file tree and editor components.
 * 
 * @class NavigationService
 */
class NavigationService {
  /**
   * Navigation events
   */
  static EVENTS = {
    NAVIGATION_STARTED: 'navigation_started',
    NAVIGATION_COMPLETED: 'navigation_completed',
    NAVIGATION_FAILED: 'navigation_failed',
    FILE_OPENED: 'file_opened',
    CONTENT_POSITIONED: 'content_positioned'
  };

  /**
   * Initialize navigation service
   * @param {AppState} appState - Application state manager
   * @param {FileTree} fileTree - File tree component
   * @param {EditorPreviewPanel} editorPanel - Editor/preview panel component
   */
  constructor(appState, fileTree, editorPanel) {
    if (!appState) {
      throw new Error('AppState instance is required');
    }
    if (!fileTree) {
      throw new Error('FileTree instance is required');
    }
    if (!editorPanel) {
      throw new Error('EditorPreviewPanel instance is required');
    }

    this.appState = appState;
    this.fileTree = fileTree;
    this.editorPanel = editorPanel;
    
    // Navigation state
    this.isNavigating = false;
    this.navigationQueue = [];
    this.lastNavigationTime = 0;
    
    // Event listeners
    this.eventListeners = new Map();
    
    // Performance settings
    this.config = {
      navigationThrottle: 100, // Minimum time between navigations
      scrollAnimationDuration: 300,
      maxQueueSize: 5
    };
    
    console.log('✅ Navigation Service initialized');
  }

  /**
   * Navigate to a specific file and optional content position
   * @param {string} filePath - Absolute or relative path to the file
   * @param {Object} options - Navigation options
   * @returns {Promise<boolean>} True if navigation was successful
   */
  async navigateToFile(filePath, options = {}) {
    const navigationOptions = {
      scrollToContent: false,
      contentQuery: null,
      line: null,
      column: null,
      highlightDuration: 2000,
      focusEditor: true,
      addToHistory: true,
      ...options
    };

    // Check if already navigating
    if (this.isNavigating) {
      return this.queueNavigation(filePath, navigationOptions);
    }

    // Throttle navigation attempts
    const now = Date.now();
    if (now - this.lastNavigationTime < this.config.navigationThrottle) {
      console.log('⏳ Navigation throttled, queuing request');
      return this.queueNavigation(filePath, navigationOptions);
    }

    this.lastNavigationTime = now;
    this.isNavigating = true;

    try {
      console.log('🧭 Starting navigation to:', filePath);
      
      this.emit(NavigationService.EVENTS.NAVIGATION_STARTED, {
        filePath,
        options: navigationOptions,
        timestamp: now
      });

      // Resolve file path
      const resolvedPath = await this.resolveFilePath(filePath);
      if (!resolvedPath) {
        throw new Error(`File not found: ${filePath}`);
      }

      // Check if file exists and is accessible
      const fileExists = await this.checkFileExists(resolvedPath);
      if (!fileExists) {
        throw new Error(`File is not accessible: ${resolvedPath}`);
      }

      // Open the file
      const openResult = await this.openFile(resolvedPath, navigationOptions);
      if (!openResult) {
        throw new Error(`Failed to open file: ${resolvedPath}`);
      }

      // Position content if requested
      if (navigationOptions.scrollToContent || navigationOptions.line !== null) {
        await this.positionContent(navigationOptions);
      }

      // Focus editor if requested
      if (navigationOptions.focusEditor) {
        this.focusEditor();
      }

      this.emit(NavigationService.EVENTS.NAVIGATION_COMPLETED, {
        filePath: resolvedPath,
        options: navigationOptions,
        timestamp: Date.now()
      });

      console.log('✅ Navigation completed successfully');
      return true;

    } catch (error) {
      console.error('❌ Navigation failed:', error);
      
      this.emit(NavigationService.EVENTS.NAVIGATION_FAILED, {
        filePath,
        error: error.message,
        timestamp: Date.now()
      });

      // Show user-friendly error message
      this.showNavigationError(error.message);
      return false;

    } finally {
      this.isNavigating = false;
      
      // Process queue if there are pending navigations
      if (this.navigationQueue.length > 0) {
        const next = this.navigationQueue.shift();
        setTimeout(() => {
          this.navigateToFile(next.filePath, next.options);
        }, 50);
      }
    }
  }

  /**
   * Navigate to a specific suggestion
   * @param {Object} suggestion - Suggestion object with file path and content info
   * @returns {Promise<boolean>} True if navigation was successful
   */
  async navigateToSuggestion(suggestion) {
    if (!suggestion || !suggestion.filePath) {
      console.error('❌ Invalid suggestion for navigation:', suggestion);
      return false;
    }

    const options = {
      scrollToContent: true,
      contentQuery: suggestion.contextSnippet || suggestion.content,
      highlightDuration: 3000,
      focusEditor: true,
      addToHistory: true
    };

    return this.navigateToFile(suggestion.filePath, options);
  }

  /**
   * Queue navigation request for later processing
   * @private
   */
  queueNavigation(filePath, options) {
    if (this.navigationQueue.length >= this.config.maxQueueSize) {
      console.warn('⚠️ Navigation queue full, dropping oldest request');
      this.navigationQueue.shift();
    }

    this.navigationQueue.push({ filePath, options });
    console.log('📋 Navigation queued:', filePath);
    
    return new Promise((resolve) => {
      // This will resolve when the navigation is actually processed
      setTimeout(() => resolve(false), 1000);
    });
  }

  /**
   * Resolve file path to absolute path
   * @private
   */
  async resolveFilePath(filePath) {
    try {
      // If already absolute path, return as-is
      if (filePath.startsWith('/') || filePath.match(/^[A-Za-z]:/)) {
        return filePath;
      }

      // Get current vault path
      const vaultPath = this.appState.get('vaultPath');
      if (!vaultPath) {
        throw new Error('No vault selected');
      }

      // Resolve relative to vault
      const resolvedPath = `${vaultPath}/${filePath}`;
      return resolvedPath.replace(/\/+/g, '/'); // Normalize path separators

    } catch (error) {
      console.error('Failed to resolve file path:', error);
      return null;
    }
  }

  /**
   * Check if file exists and is accessible
   * @private
   */
  async checkFileExists(filePath) {
    try {
      const exists = await window.__TAURI__.core.invoke('file_exists', {
        path: filePath
      });
      return exists;
    } catch (error) {
      console.warn('Error checking file existence:', error);
      return false;
    }
  }

  /**
   * Open file in the editor
   * @private
   */
  async openFile(filePath, options) {
    try {
      // Update file tree selection first
      await this.updateFileTreeSelection(filePath);

      // Open file in editor
      const fileContent = await window.__TAURI__.core.invoke('read_file', {
        path: filePath
      });

      // Update editor content
      await this.editorPanel.loadFile(filePath, fileContent);

      // Update app state
      this.appState.set('currentFile', filePath);
      
      this.emit(NavigationService.EVENTS.FILE_OPENED, {
        filePath,
        contentLength: fileContent.length,
        timestamp: Date.now()
      });

      return true;

    } catch (error) {
      console.error('Failed to open file:', error);
      return false;
    }
  }

  /**
   * Update file tree selection to match opened file
   * @private
   */
  async updateFileTreeSelection(filePath) {
    try {
      // Find and select the file in the tree
      if (this.fileTree && typeof this.fileTree.selectFile === 'function') {
        await this.fileTree.selectFile(filePath);
      }
    } catch (error) {
      console.warn('Failed to update file tree selection:', error);
    }
  }

  /**
   * Position content within the editor (scroll to specific content or line)
   * @private
   */
  async positionContent(options) {
    try {
      const editor = this.editorPanel.getEditor();
      if (!editor) {
        console.warn('Editor not available for content positioning');
        return;
      }

      let targetPosition = null;

      // Position by line number
      if (options.line !== null) {
        targetPosition = {
          line: options.line,
          column: options.column || 0
        };
      }
      // Position by content search
      else if (options.contentQuery) {
        targetPosition = await this.findContentPosition(options.contentQuery);
      }

      if (targetPosition) {
        // Scroll to position
        await this.scrollToPosition(targetPosition, options);
        
        // Highlight content if requested
        if (options.highlightDuration > 0) {
          await this.highlightContent(targetPosition, options);
        }

        this.emit(NavigationService.EVENTS.CONTENT_POSITIONED, {
          position: targetPosition,
          timestamp: Date.now()
        });
      }

    } catch (error) {
      console.warn('Failed to position content:', error);
    }
  }

  /**
   * Find position of content within the editor
   * @private
   */
  async findContentPosition(contentQuery) {
    try {
      const editor = this.editorPanel.getEditor();
      const content = editor.getValue();
      
      if (!content || !contentQuery) return null;

      // Simple text search - could be enhanced with fuzzy matching
      const index = content.toLowerCase().indexOf(contentQuery.toLowerCase().substring(0, 50));
      if (index === -1) return null;

      // Convert character index to line/column
      const lines = content.substring(0, index).split('\n');
      const line = lines.length - 1;
      const column = lines[lines.length - 1].length;

      return { line, column };

    } catch (error) {
      console.warn('Failed to find content position:', error);
      return null;
    }
  }

  /**
   * Scroll editor to specific position
   * @private
   */
  async scrollToPosition(position, options) {
    try {
      const editor = this.editorPanel.getEditor();
      if (!editor || typeof editor.setCursor !== 'function') {
        console.warn('Editor does not support cursor positioning');
        return;
      }

      // Set cursor position
      editor.setCursor(position.line, position.column);

      // Scroll into view with animation
      if (typeof editor.scrollIntoView === 'function') {
        editor.scrollIntoView({ 
          line: position.line, 
          ch: position.column 
        }, options.scrollAnimationDuration || 300);
      }

    } catch (error) {
      console.warn('Failed to scroll to position:', error);
    }
  }

  /**
   * Highlight content at specific position
   * @private
   */
  async highlightContent(position, options) {
    try {
      const editor = this.editorPanel.getEditor();
      if (!editor || typeof editor.addLineClass !== 'function') {
        console.warn('Editor does not support line highlighting');
        return;
      }

      // Add highlight class
      const lineHandle = editor.addLineClass(position.line, 'background', 'navigation-highlight');

      // Remove highlight after duration
      setTimeout(() => {
        if (lineHandle && typeof editor.removeLineClass === 'function') {
          editor.removeLineClass(lineHandle, 'background', 'navigation-highlight');
        }
      }, options.highlightDuration);

    } catch (error) {
      console.warn('Failed to highlight content:', error);
    }
  }

  /**
   * Focus the editor element
   * @private
   */
  focusEditor() {
    try {
      const editor = this.editorPanel.getEditor();
      if (editor && typeof editor.focus === 'function') {
        setTimeout(() => {
          editor.focus();
        }, 100); // Small delay to ensure DOM is ready
      }
    } catch (error) {
      console.warn('Failed to focus editor:', error);
    }
  }

  /**
   * Show navigation error to user
   * @private
   */
  showNavigationError(errorMessage) {
    // This would integrate with the app's notification system
    console.error('Navigation Error:', errorMessage);
    
    // If there's a global notification function, use it
    if (typeof window.showNotification === 'function') {
      window.showNotification(`Navigation failed: ${errorMessage}`, 'error');
    }
  }

  /**
   * Get navigation statistics
   * @returns {Object} Navigation performance stats
   */
  getStats() {
    return {
      queueLength: this.navigationQueue.length,
      isNavigating: this.isNavigating,
      lastNavigationTime: this.lastNavigationTime,
      config: { ...this.config }
    };
  }

  /**
   * Update configuration
   * @param {Object} newConfig - Configuration updates
   */
  updateConfig(newConfig) {
    this.config = { ...this.config, ...newConfig };
    console.log('⚙️ Navigation Service configuration updated');
  }

  /**
   * Add event listener
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   */
  addEventListener(eventType, handler) {
    if (!this.eventListeners.has(eventType)) {
      this.eventListeners.set(eventType, new Set());
    }
    this.eventListeners.get(eventType).add(handler);
  }

  /**
   * Remove event listener
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   */
  removeEventListener(eventType, handler) {
    const listeners = this.eventListeners.get(eventType);
    if (listeners) {
      listeners.delete(handler);
      if (listeners.size === 0) {
        this.eventListeners.delete(eventType);
      }
    }
  }

  /**
   * Emit event to listeners
   * @private
   */
  emit(eventType, data) {
    const listeners = this.eventListeners.get(eventType);
    if (!listeners) return;

    listeners.forEach(handler => {
      try {
        handler(data);
      } catch (error) {
        console.error(`Error in navigation service event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Cleanup service
   */
  destroy() {
    // Clear queue
    this.navigationQueue = [];
    
    // Clear event listeners
    this.eventListeners.clear();
    
    // Reset state
    this.isNavigating = false;
    
    console.log('✅ Navigation Service destroyed');
  }
}

export default NavigationService;