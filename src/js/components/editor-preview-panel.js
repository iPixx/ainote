/**
 * EditorPreviewPanel - Toggle system for switching between editor and preview modes
 * 
 * Provides seamless transitions between markdown editing and rendered preview modes
 * while maintaining content, scroll position, and state consistency.
 * 
 * Performance targets:
 * - Mode switch time: <100ms
 * - Scroll synchronization: <50ms
 * - Animation smoothness: 60fps
 * - Memory overhead: <1MB for toggle system
 * 
 * @class EditorPreviewPanel
 */
class EditorPreviewPanel {
  /**
   * Panel event types for communication
   */
  static EVENTS = {
    MODE_CHANGED: 'mode_changed',
    CONTENT_SYNCED: 'content_synced',
    SCROLL_SYNCED: 'scroll_synced',
    ANIMATION_STARTED: 'animation_started',
    ANIMATION_COMPLETED: 'animation_completed',
    STATE_SAVED: 'state_saved',
    STATE_LOADED: 'state_loaded'
  };

  /**
   * Initialize the editor/preview toggle panel
   * @param {HTMLElement} container - Container element for the panel
   * @param {AppState} appState - Application state manager
   */
  constructor(container, appState) {
    if (!(container instanceof HTMLElement)) {
      throw new Error('Container must be a valid HTML element');
    }
    if (!appState) {
      throw new Error('AppState instance required');
    }

    this.container = container;
    this.appState = appState;
    
    // Panel state
    this.currentMode = 'editor'; // 'editor' | 'preview'
    this.isInitialized = false;
    this.isTransitioning = false;
    this.content = '';
    
    // Component instances
    this.markdownEditor = null;
    this.previewRenderer = null;
    
    // UI elements
    this.editorContainer = null;
    this.previewContainer = null;
    this.modeToggleButton = null;
    this.modeIndicator = null;
    
    // Animation state
    this.animationDuration = 250; // milliseconds
    this.currentAnimation = null;
    this.animationSettings = {
      easing: 'cubic-bezier(0.4, 0.0, 0.2, 1)', // Material design standard
      duration: this.animationDuration
    };
    
    // Scroll synchronization state
    this.scrollSync = {
      enabled: true,
      editorScrollPosition: 0,
      previewScrollPosition: 0,
      scrollRatio: 1.0,
      syncInProgress: false,
      tolerance: 3 // pixels
    };
    
    // State persistence
    this.stateKey = 'editor-preview-panel-state';
    this.perFileStateKey = 'editor-preview-panel-file-states';
    this.defaultState = {
      lastMode: 'editor',
      autoPreview: false,
      animationsEnabled: true
    };
    
    // Keyboard shortcuts
    this.keyboardShortcuts = new Map();
    
    // Performance tracking
    this.performanceMetrics = {
      modeSwitches: 0,
      averageSwitchTime: 0,
      maxSwitchTime: 0,
      scrollSyncs: 0,
      averageScrollSyncTime: 0
    };
    
    // Error handling
    this.errorHandler = this.createErrorHandler();
    
    // Initialize the panel
    try {
      this.init();
    } catch (error) {
      this.handleError('initialization', error);
    }
  }

  /**
   * Initialize the editor/preview panel
   */
  init() {
    if (this.isInitialized) {
      console.warn('EditorPreviewPanel already initialized');
      return;
    }

    // Load saved state
    this.loadState();
    
    // Set up DOM structure
    this.createPanelStructure();
    
    // Initialize components
    this.initializeEditor();
    this.initializePreview();
    
    // Set up event listeners
    this.setupEventListeners();
    
    // Set up keyboard shortcuts
    this.setupKeyboardShortcuts();
    
    // Apply initial mode
    this.setMode(this.currentMode, false); // No animation on init
    
    // Mark as initialized
    this.isInitialized = true;
    
    console.log('✅ EditorPreviewPanel initialized successfully');
    
    // Emit initialization event
    this.emit(EditorPreviewPanel.EVENTS.MODE_CHANGED, {
      mode: this.currentMode,
      timestamp: Date.now(),
      isInitial: true
    });
  }

  /**
   * Create the DOM structure for the panel
   * @private
   */
  createPanelStructure() {
    // Clear existing content
    this.container.innerHTML = '';
    this.container.className = 'editor-preview-panel';

    // Create mode indicator
    this.modeIndicator = document.createElement('div');
    this.modeIndicator.className = 'mode-indicator';
    this.modeIndicator.setAttribute('aria-live', 'polite');
    this.modeIndicator.setAttribute('aria-atomic', 'true');

    // Create editor container
    this.editorContainer = document.createElement('div');
    this.editorContainer.className = 'editor-container';
    this.editorContainer.setAttribute('role', 'tabpanel');
    this.editorContainer.setAttribute('aria-labelledby', 'editor-tab');
    this.editorContainer.setAttribute('data-mode', 'editor');

    // Create preview container
    this.previewContainer = document.createElement('div');
    this.previewContainer.className = 'preview-container';
    this.previewContainer.setAttribute('role', 'tabpanel');
    this.previewContainer.setAttribute('aria-labelledby', 'preview-tab');
    this.previewContainer.setAttribute('data-mode', 'preview');
    this.previewContainer.style.display = 'none'; // Initially hidden

    // Assemble structure
    this.container.appendChild(this.modeIndicator);
    this.container.appendChild(this.editorContainer);
    this.container.appendChild(this.previewContainer);

    // Find or create toggle button
    this.modeToggleButton = document.getElementById('toggleModeBtn');
    if (this.modeToggleButton) {
      this.updateToggleButton();
    }

    console.log('📐 Panel DOM structure created');
  }

  /**
   * Initialize the markdown editor
   * @private
   */
  initializeEditor() {
    try {
      // Import MarkdownEditor class
      import('./markdown-editor.js').then(({ default: MarkdownEditor }) => {
        this.markdownEditor = new MarkdownEditor(this.editorContainer, this.appState);
        
        // Listen for content changes
        this.markdownEditor.addEventListener(MarkdownEditor.EVENTS.CONTENT_CHANGED, (event) => {
          this.handleContentChange(event.detail.content);
        });
        
        // Listen for scroll events
        this.markdownEditor.addEventListener('scroll', () => {
          if (this.scrollSync.enabled && this.currentMode === 'editor') {
            this.syncScrollToPreview();
          }
        });
        
        console.log('✅ MarkdownEditor initialized in panel');
      }).catch(error => {
        this.handleError('editor-initialization', error);
      });
    } catch (error) {
      this.handleError('editor-initialization', error);
    }
  }

  /**
   * Initialize the preview renderer
   * @private
   */
  initializePreview() {
    try {
      // Import PreviewRenderer class
      import('./preview-renderer.js').then(({ default: PreviewRenderer }) => {
        this.previewRenderer = new PreviewRenderer(this.previewContainer, this.appState);
        
        // Listen for scroll events
        this.previewContainer.addEventListener('scroll', () => {
          if (this.scrollSync.enabled && this.currentMode === 'preview') {
            this.syncScrollToEditor();
          }
        });
        
        console.log('✅ PreviewRenderer initialized in panel');
      }).catch(error => {
        this.handleError('preview-initialization', error);
      });
    } catch (error) {
      this.handleError('preview-initialization', error);
    }
  }

  /**
   * Set up event listeners
   * @private
   */
  setupEventListeners() {
    // Toggle button click
    if (this.modeToggleButton) {
      this.modeToggleButton.addEventListener('click', () => {
        this.toggleMode();
      });
    }

    // AppState events
    if (this.appState) {
      this.appState.addEventListener('view_mode_changed', (event) => {
        this.setMode(event.detail.mode);
      });
      
      this.appState.addEventListener('file_changed', (event) => {
        this.handleFileChange(event.detail.file);
      });
    }

    // Window events for state persistence
    window.addEventListener('beforeunload', () => {
      this.saveState();
    });

    // Resize observer for responsive handling
    if (window.ResizeObserver) {
      this.resizeObserver = new ResizeObserver(() => {
        this.handleResize();
      });
      this.resizeObserver.observe(this.container);
    }
  }

  /**
   * Set up keyboard shortcuts
   * @private
   */
  setupKeyboardShortcuts() {
    // Main toggle shortcut (Ctrl+Shift+P)
    this.addKeyboardShortcut('ctrl+shift+p', () => {
      this.toggleMode();
    }, 'Toggle editor/preview mode');
    
    this.addKeyboardShortcut('cmd+shift+p', () => {
      this.toggleMode();
    }, 'Toggle editor/preview mode (Mac)');

    // Quick access shortcuts (using Alt to avoid conflicts with browser shortcuts)
    this.addKeyboardShortcut('alt+e', () => {
      this.setMode('editor');
    }, 'Switch to editor mode');
    
    this.addKeyboardShortcut('cmd+alt+e', () => {
      this.setMode('editor');
    }, 'Switch to editor mode (Mac)');

    this.addKeyboardShortcut('alt+p', () => {
      this.setMode('preview');
    }, 'Switch to preview mode');
    
    this.addKeyboardShortcut('cmd+alt+p', () => {
      this.setMode('preview');
    }, 'Switch to preview mode (Mac)');

    // Set up global keyboard handler
    document.addEventListener('keydown', (event) => {
      this.handleKeyboardShortcut(event);
    });

    console.log(`⌨️ Keyboard shortcuts configured: ${this.keyboardShortcuts.size} shortcuts`);
  }

  /**
   * Add a keyboard shortcut
   * @param {string} shortcut - Keyboard shortcut string
   * @param {Function} handler - Function to execute
   * @param {string} description - Description of the shortcut
   */
  addKeyboardShortcut(shortcut, handler, description = '') {
    this.keyboardShortcuts.set(shortcut.toLowerCase(), {
      handler,
      description,
      shortcut
    });
  }

  /**
   * Handle keyboard shortcut events
   * @param {KeyboardEvent} event - Keyboard event
   * @private
   */
  handleKeyboardShortcut(event) {
    // Build shortcut string
    const parts = [];
    
    if (event.ctrlKey) parts.push('ctrl');
    if (event.metaKey) parts.push('cmd');
    if (event.shiftKey) parts.push('shift');
    if (event.altKey) parts.push('alt');
    
    // Add the main key
    let key = event.key.toLowerCase();
    if (key === ' ') key = 'space';
    parts.push(key);
    
    const shortcutString = parts.join('+');
    const shortcutData = this.keyboardShortcuts.get(shortcutString);
    
    if (shortcutData) {
      event.preventDefault();
      event.stopPropagation();
      
      try {
        shortcutData.handler();
        console.log(`⌨️ Executed panel shortcut: ${shortcutData.shortcut}`);
      } catch (error) {
        this.handleError(`keyboard-shortcut-${shortcutData.shortcut}`, error);
      }
    }
  }

  /**
   * Toggle between editor and preview modes
   */
  toggleMode() {
    const newMode = this.currentMode === 'editor' ? 'preview' : 'editor';
    this.setMode(newMode);
  }

  /**
   * Set the current mode (editor or preview)
   * @param {string} mode - 'editor' or 'preview'
   * @param {boolean} animate - Whether to animate the transition (default: true)
   */
  setMode(mode, animate = true) {
    if (!['editor', 'preview'].includes(mode)) {
      this.handleError('setMode', new Error(`Invalid mode: ${mode}`));
      return;
    }

    if (mode === this.currentMode) {
      return; // No change needed
    }

    if (this.isTransitioning) {
      console.log('Mode switch already in progress, ignoring request');
      return;
    }

    const startTime = performance.now();
    this.isTransitioning = true;

    console.log(`🔄 Switching to ${mode} mode`);

    // Save current scroll position
    this.saveScrollPosition();

    // Sync content before switching
    this.syncContent();

    // Perform the mode switch
    this.performModeSwitch(mode, animate).then(() => {
      // Update state
      this.currentMode = mode;
      this.isTransitioning = false;

      // Restore scroll position
      this.restoreScrollPosition();

      // Update UI elements
      this.updateModeIndicator();
      this.updateToggleButton();

      // Update AppState
      if (this.appState && this.appState.setViewMode) {
        this.appState.setViewMode(mode);
      }

      // Save state
      this.saveState();

      // Performance tracking
      const switchTime = performance.now() - startTime;
      this.trackPerformanceMetric('modeSwitch', switchTime);

      // Force component resize to ensure proper dimensions
      this.forceComponentResize();

      // Emit event
      this.emit(EditorPreviewPanel.EVENTS.MODE_CHANGED, {
        mode,
        previousMode: this.currentMode === mode ? (mode === 'editor' ? 'preview' : 'editor') : this.currentMode,
        switchTime,
        animated: animate,
        timestamp: Date.now()
      });

      console.log(`✅ Mode switched to ${mode} in ${switchTime.toFixed(2)}ms`);
    }).catch(error => {
      this.isTransitioning = false;
      this.handleError('mode-switch', error);
    });
  }

  /**
   * Perform the actual mode switch with animation
   * @param {string} mode - Target mode
   * @param {boolean} animate - Whether to animate
   * @returns {Promise} Animation completion promise
   * @private
   */
  async performModeSwitch(mode, animate) {
    if (!animate) {
      // Instant switch
      this.showContainer(mode === 'editor' ? this.editorContainer : this.previewContainer);
      this.hideContainer(mode === 'editor' ? this.previewContainer : this.editorContainer);
      
      // Force resize for instant switches too
      setTimeout(() => {
        this.forceComponentResize();
      }, 0);
      
      return Promise.resolve();
    }

    // Emit animation started event
    this.emit(EditorPreviewPanel.EVENTS.ANIMATION_STARTED, {
      fromMode: this.currentMode,
      toMode: mode,
      duration: this.animationDuration
    });

    const fromContainer = mode === 'editor' ? this.previewContainer : this.editorContainer;
    const toContainer = mode === 'editor' ? this.editorContainer : this.previewContainer;

    // Set up animation
    return new Promise((resolve, reject) => {
      try {
        // Prepare containers for animation
        this.prepareAnimation(fromContainer, toContainer);

        // Create and run animation
        this.runSwitchAnimation(fromContainer, toContainer).then(() => {
          // Clean up animation
          this.cleanupAnimation(fromContainer, toContainer);

          // Emit animation completed event
          this.emit(EditorPreviewPanel.EVENTS.ANIMATION_COMPLETED, {
            mode,
            duration: this.animationDuration,
            timestamp: Date.now()
          });

          resolve();
        }).catch(reject);
      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Prepare containers for animation
   * @param {HTMLElement} fromContainer - Container to hide
   * @param {HTMLElement} toContainer - Container to show
   * @private
   */
  prepareAnimation(fromContainer, toContainer) {
    // Ensure both containers are visible during transition
    fromContainer.style.display = 'block';
    toContainer.style.display = 'block';
    
    // Set initial opacity states
    fromContainer.style.opacity = '1';
    toContainer.style.opacity = '0';
    
    // Position containers for smooth transition
    fromContainer.style.position = 'absolute';
    toContainer.style.position = 'absolute';
    fromContainer.style.top = '0';
    toContainer.style.top = '0';
    fromContainer.style.left = '0';
    toContainer.style.left = '0';
    fromContainer.style.width = '100%';
    toContainer.style.width = '100%';
    fromContainer.style.height = '100%';
    toContainer.style.height = '100%';
  }

  /**
   * Run the switch animation
   * @param {HTMLElement} fromContainer - Container to hide
   * @param {HTMLElement} toContainer - Container to show
   * @returns {Promise} Animation completion promise
   * @private
   */
  runSwitchAnimation(fromContainer, toContainer) {
    return new Promise((resolve, reject) => {
      // Use CSS transitions for smooth animation
      const transition = `opacity ${this.animationDuration}ms ${this.animationSettings.easing}`;
      
      fromContainer.style.transition = transition;
      toContainer.style.transition = transition;
      
      // Start the fade transition
      requestAnimationFrame(() => {
        fromContainer.style.opacity = '0';
        toContainer.style.opacity = '1';
      });
      
      // Wait for animation to complete
      setTimeout(() => {
        resolve();
      }, this.animationDuration);
    });
  }

  /**
   * Clean up after animation
   * @param {HTMLElement} fromContainer - Container that was hidden
   * @param {HTMLElement} toContainer - Container that was shown
   * @private
   */
  cleanupAnimation(fromContainer, toContainer) {
    // Hide the inactive container
    this.hideContainer(fromContainer);
    
    // Reset animation styles and ensure active container is properly displayed
    [fromContainer, toContainer].forEach(container => {
      container.style.position = '';
      container.style.top = '';
      container.style.left = '';
      container.style.width = '';
      container.style.transition = '';
      container.style.opacity = '';
      // Don't reset height here - let showContainer handle it
    });
    
    // Ensure the active container is properly shown
    this.showContainer(toContainer);
    
    // Force a resize event to ensure editor components recalculate their dimensions
    setTimeout(() => {
      window.dispatchEvent(new Event('resize'));
    }, 50);
  }

  /**
   * Show container (helper method)
   * @param {HTMLElement} container - Container to show
   * @private
   */
  showContainer(container) {
    container.style.display = 'flex'; // Use flex instead of block
    container.setAttribute('aria-hidden', 'false');
    
    // Ensure container takes full height
    container.style.height = 'calc(100% - 2.5rem)';
    container.style.flex = '1';
  }

  /**
   * Hide container (helper method)
   * @param {HTMLElement} container - Container to hide
   * @private
   */
  hideContainer(container) {
    container.style.display = 'none';
    container.setAttribute('aria-hidden', 'true');
  }

  /**
   * Sync content between editor and preview
   * @private
   */
  syncContent() {
    try {
      if (this.currentMode === 'editor' && this.markdownEditor && this.previewRenderer) {
        // Get content from editor and update preview
        const content = this.markdownEditor.getValue();
        if (content !== this.content) {
          this.content = content;
          this.previewRenderer.render(content);
          
          // Emit content synced event
          this.emit(EditorPreviewPanel.EVENTS.CONTENT_SYNCED, {
            content,
            direction: 'editor-to-preview',
            timestamp: Date.now()
          });
        }
      }
      // Note: Preview to editor sync is not needed as preview is read-only
    } catch (error) {
      this.handleError('content-sync', error);
    }
  }

  /**
   * Save current scroll position
   * @private
   */
  saveScrollPosition() {
    try {
      if (this.currentMode === 'editor' && this.markdownEditor && this.markdownEditor.textarea) {
        this.scrollSync.editorScrollPosition = this.markdownEditor.textarea.scrollTop;
      } else if (this.currentMode === 'preview' && this.previewRenderer && this.previewRenderer.elements.scrollContainer) {
        this.scrollSync.previewScrollPosition = this.previewRenderer.elements.scrollContainer.scrollTop;
      }
    } catch (error) {
      this.handleError('save-scroll-position', error);
    }
  }

  /**
   * Restore scroll position after mode switch
   * @private
   */
  restoreScrollPosition() {
    try {
      if (this.scrollSync.enabled) {
        requestAnimationFrame(() => {
          this.maintainScrollPosition();
        });
      }
    } catch (error) {
      this.handleError('restore-scroll-position', error);
    }
  }

  /**
   * Maintain scroll position across mode switches
   * Performance target: <50ms for scroll synchronization
   */
  maintainScrollPosition() {
    if (!this.scrollSync.enabled || this.scrollSync.syncInProgress) {
      return;
    }

    const startTime = performance.now();
    this.scrollSync.syncInProgress = true;

    try {
      if (this.currentMode === 'editor' && this.markdownEditor && this.markdownEditor.textarea) {
        // Calculate and apply scroll position based on content ratio
        const scrollPosition = this.calculateScrollPosition('editor');
        this.markdownEditor.textarea.scrollTop = scrollPosition;
      } else if (this.currentMode === 'preview' && this.previewRenderer && this.previewRenderer.elements.scrollContainer) {
        // Calculate and apply scroll position based on content ratio
        const scrollPosition = this.calculateScrollPosition('preview');
        this.previewRenderer.elements.scrollContainer.scrollTop = scrollPosition;
      }

      const syncTime = performance.now() - startTime;
      this.trackPerformanceMetric('scrollSync', syncTime);

      // Emit scroll synced event
      this.emit(EditorPreviewPanel.EVENTS.SCROLL_SYNCED, {
        mode: this.currentMode,
        syncTime,
        timestamp: Date.now()
      });

      if (syncTime > 50) {
        console.warn(`⚠️ Scroll sync exceeded target: ${syncTime.toFixed(2)}ms (target: <50ms)`);
      }
    } catch (error) {
      this.handleError('maintain-scroll-position', error);
    } finally {
      this.scrollSync.syncInProgress = false;
    }
  }

  /**
   * Calculate appropriate scroll position for mode switch
   * @param {string} targetMode - Target mode for scroll calculation
   * @returns {number} Calculated scroll position
   * @private
   */
  calculateScrollPosition(targetMode) {
    if (targetMode === 'editor') {
      // Convert preview scroll to editor scroll
      if (this.scrollSync.previewScrollPosition > 0 && this.previewRenderer && this.previewRenderer.elements.scrollContainer) {
        const previewContainer = this.previewRenderer.elements.scrollContainer;
        const previewScrollRatio = this.scrollSync.previewScrollPosition / 
          Math.max(1, previewContainer.scrollHeight - previewContainer.clientHeight);
        
        if (this.markdownEditor && this.markdownEditor.textarea) {
          const editorContainer = this.markdownEditor.textarea;
          return previewScrollRatio * Math.max(0, editorContainer.scrollHeight - editorContainer.clientHeight);
        }
      }
      return this.scrollSync.editorScrollPosition;
    } else {
      // Convert editor scroll to preview scroll
      if (this.scrollSync.editorScrollPosition > 0 && this.markdownEditor && this.markdownEditor.textarea) {
        const editorContainer = this.markdownEditor.textarea;
        const editorScrollRatio = this.scrollSync.editorScrollPosition / 
          Math.max(1, editorContainer.scrollHeight - editorContainer.clientHeight);
        
        if (this.previewRenderer && this.previewRenderer.elements.scrollContainer) {
          const previewContainer = this.previewRenderer.elements.scrollContainer;
          return editorScrollRatio * Math.max(0, previewContainer.scrollHeight - previewContainer.clientHeight);
        }
      }
      return this.scrollSync.previewScrollPosition;
    }
  }

  /**
   * Sync scroll from editor to preview
   * @private
   */
  syncScrollToPreview() {
    if (!this.scrollSync.enabled || this.scrollSync.syncInProgress || !this.markdownEditor || !this.previewRenderer) {
      return;
    }

    const startTime = performance.now();
    this.scrollSync.syncInProgress = true;

    try {
      const editorTextarea = this.markdownEditor.textarea;
      const previewContainer = this.previewRenderer.elements.scrollContainer;
      
      if (editorTextarea && previewContainer) {
        const editorScrollRatio = editorTextarea.scrollTop / 
          Math.max(1, editorTextarea.scrollHeight - editorTextarea.clientHeight);
        
        const targetScrollTop = editorScrollRatio * 
          Math.max(0, previewContainer.scrollHeight - previewContainer.clientHeight);
        
        if (Math.abs(previewContainer.scrollTop - targetScrollTop) > this.scrollSync.tolerance) {
          previewContainer.scrollTop = targetScrollTop;
        }
      }

      const syncTime = performance.now() - startTime;
      this.trackPerformanceMetric('scrollSync', syncTime);
    } catch (error) {
      this.handleError('sync-scroll-to-preview', error);
    } finally {
      setTimeout(() => {
        this.scrollSync.syncInProgress = false;
      }, 50);
    }
  }

  /**
   * Sync scroll from preview to editor
   * @private
   */
  syncScrollToEditor() {
    if (!this.scrollSync.enabled || this.scrollSync.syncInProgress || !this.markdownEditor || !this.previewRenderer) {
      return;
    }

    const startTime = performance.now();
    this.scrollSync.syncInProgress = true;

    try {
      const editorTextarea = this.markdownEditor.textarea;
      const previewContainer = this.previewRenderer.elements.scrollContainer;
      
      if (editorTextarea && previewContainer) {
        const previewScrollRatio = previewContainer.scrollTop / 
          Math.max(1, previewContainer.scrollHeight - previewContainer.clientHeight);
        
        const targetScrollTop = previewScrollRatio * 
          Math.max(0, editorTextarea.scrollHeight - editorTextarea.clientHeight);
        
        if (Math.abs(editorTextarea.scrollTop - targetScrollTop) > this.scrollSync.tolerance) {
          editorTextarea.scrollTop = targetScrollTop;
        }
      }

      const syncTime = performance.now() - startTime;
      this.trackPerformanceMetric('scrollSync', syncTime);
    } catch (error) {
      this.handleError('sync-scroll-to-editor', error);
    } finally {
      setTimeout(() => {
        this.scrollSync.syncInProgress = false;
      }, 50);
    }
  }

  /**
   * Update mode indicator
   * @private
   */
  updateModeIndicator() {
    if (!this.modeIndicator) return;

    const modeText = this.currentMode === 'editor' ? 'Editor' : 'Preview';
    const modeIcon = this.currentMode === 'editor' ? '✏️' : '👁';
    
    this.modeIndicator.innerHTML = `
      <span class="mode-icon">${modeIcon}</span>
      <span class="mode-text">${modeText}</span>
    `;
    
    this.modeIndicator.className = `mode-indicator mode-${this.currentMode}`;
  }

  /**
   * Update toggle button appearance
   * @private
   */
  updateToggleButton() {
    if (!this.modeToggleButton) return;

    const isEditor = this.currentMode === 'editor';
    this.modeToggleButton.textContent = isEditor ? '👁' : '✏️';
    this.modeToggleButton.title = isEditor ? 'Switch to preview (Ctrl+Shift+P)' : 'Switch to editor (Ctrl+Shift+P)';
    this.modeToggleButton.setAttribute('aria-label', 
      isEditor ? 'Switch to preview mode' : 'Switch to editor mode');
    
    // Update button class
    this.modeToggleButton.className = this.modeToggleButton.className
      .replace(/mode-\w+/g, '') + ` mode-${this.currentMode}`;
  }

  /**
   * Handle content changes from editor
   * @param {string} content - New content
   * @private
   */
  handleContentChange(content) {
    this.content = content;
    
    // Auto-update preview if in preview mode or if real-time updates are enabled
    if (this.currentMode === 'preview' || (this.previewRenderer && this.previewRenderer.realTimeEnabled)) {
      if (this.previewRenderer) {
        this.previewRenderer.render(content);
      }
    }
  }

  /**
   * Handle file changes
   * @param {string} filePath - New file path
   * @private
   */
  handleFileChange(filePath) {
    // Save current file state
    this.saveFileState();
    
    // Load new file state
    this.loadFileState(filePath);
    
    // Update content in both editor and preview
    this.syncContent();
  }

  /**
   * Handle resize events
   * @private
   */
  handleResize() {
    // Update container dimensions
    if (this.markdownEditor && this.markdownEditor.handleResize) {
      this.markdownEditor.handleResize();
    }
    
    if (this.previewRenderer && this.previewRenderer.handleResize) {
      this.previewRenderer.handleResize();
    }
  }

  /**
   * Set content for both editor and preview
   * @param {string} content - Content to set
   */
  setContent(content) {
    this.content = content;
    
    if (this.markdownEditor) {
      this.markdownEditor.setValue(content);
    }
    
    if (this.previewRenderer) {
      this.previewRenderer.render(content);
    }
  }

  /**
   * Get current content
   * @returns {string} Current content
   */
  getContent() {
    if (this.markdownEditor) {
      return this.markdownEditor.getValue();
    }
    return this.content;
  }

  /**
   * Enable or disable scroll synchronization
   * @param {boolean} enabled - Whether scroll sync should be enabled
   */
  setScrollSyncEnabled(enabled) {
    this.scrollSync.enabled = enabled;
    console.log(`🔄 Scroll synchronization ${enabled ? 'enabled' : 'disabled'}`);
  }

  /**
   * Force resize of editor components to ensure proper height
   * @private
   */
  forceComponentResize() {
    // Force editor to recalculate its dimensions
    if (this.markdownEditor && this.markdownEditor.handleResize) {
      this.markdownEditor.handleResize();
    }
    
    // Force preview to recalculate its dimensions
    if (this.previewRenderer && this.previewRenderer.handleResize) {
      this.previewRenderer.handleResize();
    }
    
    // Dispatch resize event for any other components listening
    setTimeout(() => {
      const resizeEvent = new Event('resize');
      window.dispatchEvent(resizeEvent);
      
      // Also dispatch it on the container
      if (this.container) {
        this.container.dispatchEvent(resizeEvent);
      }
    }, 0);
  }

  /**
   * Save panel state to localStorage
   * @private
   */
  saveState() {
    try {
      const state = {
        lastMode: this.currentMode,
        scrollSync: this.scrollSync.enabled,
        animationsEnabled: this.animationSettings.duration > 0,
        timestamp: Date.now()
      };
      
      localStorage.setItem(this.stateKey, JSON.stringify(state));
      
      this.emit(EditorPreviewPanel.EVENTS.STATE_SAVED, {
        state,
        timestamp: Date.now()
      });
    } catch (error) {
      this.handleError('save-state', error);
    }
  }

  /**
   * Load panel state from localStorage
   * @private
   */
  loadState() {
    try {
      const savedState = localStorage.getItem(this.stateKey);
      if (savedState) {
        const state = { ...this.defaultState, ...JSON.parse(savedState) };
        
        this.currentMode = state.lastMode || 'editor';
        this.scrollSync.enabled = state.scrollSync !== false;
        
        if (!state.animationsEnabled) {
          this.animationDuration = 0;
          this.animationSettings.duration = 0;
        }
        
        this.emit(EditorPreviewPanel.EVENTS.STATE_LOADED, {
          state,
          timestamp: Date.now()
        });
      }
    } catch (error) {
      this.handleError('load-state', error);
      // Use defaults on error
      this.currentMode = this.defaultState.lastMode;
    }
  }

  /**
   * Save per-file state
   * @private
   */
  saveFileState() {
    try {
      const currentFile = this.appState?.getState?.()?.currentFile;
      if (!currentFile) return;

      let fileStates = {};
      const savedStates = localStorage.getItem(this.perFileStateKey);
      if (savedStates) {
        fileStates = JSON.parse(savedStates);
      }

      fileStates[currentFile] = {
        mode: this.currentMode,
        editorScrollPosition: this.scrollSync.editorScrollPosition,
        previewScrollPosition: this.scrollSync.previewScrollPosition,
        timestamp: Date.now()
      };

      // Keep only last 50 file states to avoid localStorage bloat
      const fileStateEntries = Object.entries(fileStates)
        .sort(([,a], [,b]) => b.timestamp - a.timestamp)
        .slice(0, 50);
      
      fileStates = Object.fromEntries(fileStateEntries);
      localStorage.setItem(this.perFileStateKey, JSON.stringify(fileStates));
    } catch (error) {
      this.handleError('save-file-state', error);
    }
  }

  /**
   * Load per-file state
   * @param {string} filePath - File path to load state for
   * @private
   */
  loadFileState(filePath) {
    try {
      const savedStates = localStorage.getItem(this.perFileStateKey);
      if (!savedStates) return;

      const fileStates = JSON.parse(savedStates);
      const fileState = fileStates[filePath];
      
      if (fileState) {
        this.currentMode = fileState.mode || 'editor';
        this.scrollSync.editorScrollPosition = fileState.editorScrollPosition || 0;
        this.scrollSync.previewScrollPosition = fileState.previewScrollPosition || 0;
        
        // Apply the mode without animation when loading file
        this.setMode(this.currentMode, false);
      }
    } catch (error) {
      this.handleError('load-file-state', error);
    }
  }

  /**
   * Track performance metrics
   * @param {string} metric - Metric name
   * @param {number} value - Metric value
   * @private
   */
  trackPerformanceMetric(metric, value) {
    if (metric === 'modeSwitch') {
      this.performanceMetrics.modeSwitches++;
      this.performanceMetrics.maxSwitchTime = Math.max(this.performanceMetrics.maxSwitchTime, value);
      
      // Update running average
      const alpha = 0.1;
      if (this.performanceMetrics.averageSwitchTime === 0) {
        this.performanceMetrics.averageSwitchTime = value;
      } else {
        this.performanceMetrics.averageSwitchTime = 
          alpha * value + (1 - alpha) * this.performanceMetrics.averageSwitchTime;
      }
    } else if (metric === 'scrollSync') {
      this.performanceMetrics.scrollSyncs++;
      
      // Update running average
      const alpha = 0.1;
      if (this.performanceMetrics.averageScrollSyncTime === 0) {
        this.performanceMetrics.averageScrollSyncTime = value;
      } else {
        this.performanceMetrics.averageScrollSyncTime = 
          alpha * value + (1 - alpha) * this.performanceMetrics.averageScrollSyncTime;
      }
    }
  }

  /**
   * Get performance statistics
   * @returns {Object} Performance metrics
   */
  getPerformanceStats() {
    return {
      ...this.performanceMetrics,
      averageSwitchTime: Math.round(this.performanceMetrics.averageSwitchTime * 100) / 100,
      averageScrollSyncTime: Math.round(this.performanceMetrics.averageScrollSyncTime * 100) / 100,
      currentMode: this.currentMode,
      scrollSyncEnabled: this.scrollSync.enabled,
      animationDuration: this.animationDuration
    };
  }

  /**
   * Create error handler
   * @returns {Object} Error handler instance
   * @private
   */
  createErrorHandler() {
    return {
      errors: [],
      maxErrors: 20,
      
      log: (context, error, severity = 'error') => {
        const errorData = {
          context,
          message: error.message || error,
          stack: error.stack,
          timestamp: Date.now(),
          severity
        };
        
        this.errorHandler.errors.push(errorData);
        
        // Keep only recent errors
        if (this.errorHandler.errors.length > this.errorHandler.maxErrors) {
          this.errorHandler.errors = this.errorHandler.errors.slice(-this.errorHandler.maxErrors);
        }
        
        // Console logging with appropriate level
        if (severity === 'error') {
          console.error(`❌ EditorPreviewPanel [${context}]:`, error);
        } else if (severity === 'warning') {
          console.warn(`⚠️ EditorPreviewPanel [${context}]:`, error);
        } else {
          console.log(`ℹ️ EditorPreviewPanel [${context}]:`, error);
        }
        
        return errorData;
      },
      
      getErrors: () => [...this.errorHandler.errors],
      
      clearErrors: () => {
        this.errorHandler.errors = [];
      }
    };
  }

  /**
   * Handle errors with logging and recovery
   * @param {string} context - Error context
   * @param {Error} error - Error object
   * @param {string} severity - Error severity
   * @private
   */
  handleError(context, error, severity = 'error') {
    const errorData = this.errorHandler.log(context, error, severity);
    
    // Emit error event for external handling
    this.emit('error', {
      context,
      error: errorData,
      timestamp: Date.now()
    });
    
    // Attempt recovery for certain error types
    if (context.includes('animation')) {
      // Reset animation state on animation errors
      this.isTransitioning = false;
      this.currentAnimation = null;
    }
    
    if (context.includes('scroll-sync')) {
      // Temporarily disable scroll sync on sync errors
      this.scrollSync.syncInProgress = false;
    }
  }

  /**
   * Emit events to listeners
   * @param {string} eventType - Event type
   * @param {Object} data - Event data
   * @private
   */
  emit(eventType, data) {
    const event = new CustomEvent(eventType, {
      detail: data,
      bubbles: false
    });
    
    this.container.dispatchEvent(event);
  }

  /**
   * Add event listener
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   */
  addEventListener(eventType, handler) {
    this.container.addEventListener(eventType, handler);
  }

  /**
   * Remove event listener
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   */
  removeEventListener(eventType, handler) {
    this.container.removeEventListener(eventType, handler);
  }

  /**
   * Get diagnostic information
   * @returns {Object} Diagnostic data
   */
  getDiagnostics() {
    return {
      timestamp: Date.now(),
      isInitialized: this.isInitialized,
      currentMode: this.currentMode,
      isTransitioning: this.isTransitioning,
      performance: this.getPerformanceStats(),
      scrollSync: {
        enabled: this.scrollSync.enabled,
        editorScrollPosition: this.scrollSync.editorScrollPosition,
        previewScrollPosition: this.scrollSync.previewScrollPosition,
        syncInProgress: this.scrollSync.syncInProgress
      },
      keyboardShortcuts: this.keyboardShortcuts.size,
      errors: this.errorHandler.getErrors(),
      components: {
        markdownEditor: !!this.markdownEditor,
        previewRenderer: !!this.previewRenderer
      }
    };
  }

  /**
   * Clean up resources
   */
  destroy() {
    // Save state before destroying
    this.saveState();
    this.saveFileState();

    // Remove event listeners
    if (this.modeToggleButton) {
      this.modeToggleButton.removeEventListener('click', this.toggleMode);
    }

    document.removeEventListener('keydown', this.handleKeyboardShortcut);
    window.removeEventListener('beforeunload', this.saveState);

    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }

    // Destroy components
    if (this.markdownEditor && this.markdownEditor.destroy) {
      this.markdownEditor.destroy();
    }
    
    if (this.previewRenderer && this.previewRenderer.destroy) {
      this.previewRenderer.destroy();
    }

    // Clear DOM
    if (this.container) {
      this.container.innerHTML = '';
    }

    // Clear references
    this.markdownEditor = null;
    this.previewRenderer = null;
    this.container = null;
    this.appState = null;
    this.isInitialized = false;

    console.log('🗑️ EditorPreviewPanel destroyed');
  }
}

export default EditorPreviewPanel;