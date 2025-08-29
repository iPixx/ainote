/**
 * AI Panel Controller - Enhanced AI panel with suggestion display integration
 * 
 * Extends the base AI panel functionality to include real-time suggestion display,
 * user interactions, and seamless integration with the suggestion system.
 * This controller manages the complete AI panel experience for issue #157.
 * 
 * @class AiPanelController
 */
import AiPanel from './ai-panel.js';
import SuggestionList from './suggestion-list.js';
import EnhancedSuggestionList from './enhanced-suggestion-list.js';
import AiSuggestionService from '../services/ai-suggestion-service.js';
import ContentChangeDetector from '../services/content-change-detector.js';
import SuggestionCacheManager from '../services/suggestion-cache-manager.js';
import NavigationService from '../services/navigation-service.js';

class AiPanelController {
  /**
   * Enhanced AI panel events
   */
  static EVENTS = {
    ...AiPanel.EVENTS,
    SUGGESTIONS_READY: 'ai_panel_suggestions_ready',
    SUGGESTION_SELECTED: 'ai_panel_suggestion_selected',
    SUGGESTION_INSERTED: 'ai_panel_suggestion_inserted',
    SERVICE_ERROR: 'ai_panel_service_error',
    LOADING_STATE_CHANGED: 'ai_panel_loading_changed'
  };

  /**
   * Initialize the enhanced AI panel controller
   * @param {HTMLElement} panelElement - AI panel container element
   * @param {MarkdownEditor} editor - Markdown editor instance
   * @param {AppState} appState - Application state manager
   * @param {Object} layoutManager - Layout manager instance
   * @param {FileTree} fileTree - File tree component (for navigation)
   * @param {EditorPreviewPanel} editorPanel - Editor/preview panel (for navigation)
   */
  constructor(panelElement, editor, appState, layoutManager, fileTree = null, editorPanel = null) {
    if (!panelElement) {
      throw new Error('AI panel element is required');
    }
    if (!editor) {
      throw new Error('MarkdownEditor instance is required');
    }
    if (!appState) {
      throw new Error('AppState instance is required');
    }

    // Store references
    this.panelElement = panelElement;
    this.editor = editor;
    this.appState = appState;
    this.layoutManager = layoutManager;
    this.fileTree = fileTree;
    this.editorPanel = editorPanel;
    
    // Component instances
    this.aiPanel = null;
    this.suggestionList = null;
    this.suggestionService = null;
    this.contentDetector = null;
    this.cacheManager = null;
    this.navigationService = null;
    
    // UI elements
    this.contentContainer = null;
    this.suggestionContainer = null;
    this.statusIndicator = null;
    this.errorContainer = null;
    
    // State
    this.isInitialized = false;
    this.currentLoadingState = false;
    this.lastError = null;
    
    // Event listeners
    this.eventListeners = new Map();
    
    // Initialize the controller
    this.init();
  }

  /**
   * Initialize the AI panel controller
   * @private
   */
  async init() {
    if (this.isInitialized) {
      console.warn('⚠️ AI Panel Controller already initialized');
      return;
    }

    try {
      console.log('🤖 Initializing AI Panel Controller...');
      
      // Initialize base AI panel
      this.aiPanel = new AiPanel(this.panelElement, this.layoutManager);
      
      // Wait for panel to be ready
      await this.waitForPanelReady();
      
      // Setup UI structure
      this.setupUIStructure();
      
      // Initialize core services
      await this.initializeServices();
      
      // Initialize suggestion components
      this.initializeSuggestionComponents();
      
      // Setup event integration
      this.setupEventIntegration();
      
      // Mark as initialized
      this.isInitialized = true;
      
      console.log('✅ AI Panel Controller initialized successfully');
      
      // Emit ready event
      this.emit(AiPanelController.EVENTS.SUGGESTIONS_READY, {
        controller: this,
        timestamp: Date.now()
      });
      
    } catch (error) {
      console.error('❌ Failed to initialize AI Panel Controller:', error);
      this.handleInitializationError(error);
    }
  }

  /**
   * Wait for base AI panel to be ready
   * @private
   */
  waitForPanelReady() {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('AI Panel initialization timeout'));
      }, 5000);

      const handleReady = () => {
        clearTimeout(timeout);
        this.aiPanel.removeEventListener(AiPanel.EVENTS.PANEL_READY, handleReady);
        resolve();
      };

      if (this.aiPanel.isInitialized) {
        clearTimeout(timeout);
        resolve();
      } else {
        this.aiPanel.addEventListener(AiPanel.EVENTS.PANEL_READY, handleReady);
      }
    });
  }

  /**
   * Setup enhanced UI structure for suggestions
   * @private
   */
  setupUIStructure() {
    // Get the AI content container
    this.contentContainer = this.panelElement.querySelector('.ai-content') || 
                           this.panelElement.querySelector('#aiContent');
    
    if (!this.contentContainer) {
      console.error('AI content container not found');
      return;
    }

    // Clear existing content
    this.contentContainer.innerHTML = '';
    
    // Create main layout
    const mainLayout = document.createElement('div');
    mainLayout.className = 'ai-panel-main-layout';
    
    // Create status indicator
    this.statusIndicator = document.createElement('div');
    this.statusIndicator.className = 'ai-panel-status-indicator';
    this.statusIndicator.innerHTML = `
      <div class="status-content">
        <div class="status-icon">🤖</div>
        <div class="status-text">AI Assistant Ready</div>
      </div>
    `;
    
    // Create suggestion container
    this.suggestionContainer = document.createElement('div');
    this.suggestionContainer.className = 'ai-panel-suggestions';
    this.suggestionContainer.setAttribute('aria-label', 'AI Suggestions');
    
    // Create error container
    this.errorContainer = document.createElement('div');
    this.errorContainer.className = 'ai-panel-error-container';
    this.errorContainer.style.display = 'none';
    
    // Assemble layout
    mainLayout.appendChild(this.statusIndicator);
    mainLayout.appendChild(this.suggestionContainer);
    mainLayout.appendChild(this.errorContainer);
    this.contentContainer.appendChild(mainLayout);
    
    console.log('✅ AI Panel UI structure setup complete');
  }

  /**
   * Initialize core AI services
   * @private
   */
  async initializeServices() {
    try {
      // Initialize content change detector
      this.contentDetector = new ContentChangeDetector(this.editor, this.appState);
      
      // Initialize cache manager
      this.cacheManager = new SuggestionCacheManager(this.appState);
      
      // Initialize navigation service (if components are available)
      if (this.fileTree && this.editorPanel) {
        this.navigationService = new NavigationService(
          this.appState,
          this.fileTree,
          this.editorPanel
        );
        console.log('✅ Navigation service initialized');
      } else {
        console.warn('⚠️ Navigation service not initialized - FileTree or EditorPanel not available');
      }
      
      // Initialize suggestion service
      this.suggestionService = new AiSuggestionService(
        this.editor,
        this.appState,
        this.contentDetector,
        this.cacheManager
      );
      
      console.log('✅ AI services initialized');
      
    } catch (error) {
      console.error('❌ Failed to initialize AI services:', error);
      throw error;
    }
  }

  /**
   * Initialize suggestion display components
   * @private
   */
  initializeSuggestionComponents() {
    try {
      // Use enhanced suggestion list if navigation service is available
      if (this.navigationService) {
        console.log('🚀 Initializing enhanced suggestion list with navigation');
        this.suggestionList = new EnhancedSuggestionList(
          this.suggestionContainer,
          this.editor,
          this.appState,
          this.navigationService,
          {
            useCardComponents: true,
            enableNavigationIntegration: true,
            enableHoverPreviews: true,
            enableKeyboardNavigation: true
          }
        );
      } else {
        console.log('📝 Using basic suggestion list (no navigation service)');
        this.suggestionList = new SuggestionList(
          this.suggestionContainer,
          this.editor,
          this.appState
        );
      }
      
      // Enable keyboard navigation when panel is visible
      if (this.aiPanel.isVisible()) {
        this.suggestionList.enableKeyboardNavigation();
      }
      
      console.log('✅ Suggestion components initialized');
      
    } catch (error) {
      console.error('❌ Failed to initialize suggestion components:', error);
      throw error;
    }
  }

  /**
   * Setup event integration between components
   * @private
   */
  setupEventIntegration() {
    // AI Panel visibility events
    this.aiPanel.addEventListener(AiPanel.EVENTS.PANEL_ACTIVATED, () => {
      this.suggestionList.enableKeyboardNavigation();
      this.updateStatusIndicator('active', 'AI Assistant Active');
    });
    
    this.aiPanel.addEventListener(AiPanel.EVENTS.PANEL_DEACTIVATED, () => {
      this.suggestionList.disableKeyboardNavigation();
      this.updateStatusIndicator('inactive', 'AI Assistant Inactive');
    });
    
    // Suggestion service events
    this.suggestionService.addEventListener(
      AiSuggestionService.EVENTS.SUGGESTIONS_UPDATED,
      (data) => this.handleSuggestionsUpdated(data)
    );
    
    this.suggestionService.addEventListener(
      AiSuggestionService.EVENTS.SUGGESTIONS_LOADING,
      (data) => this.handleSuggestionsLoading(data)
    );
    
    this.suggestionService.addEventListener(
      AiSuggestionService.EVENTS.SUGGESTIONS_ERROR,
      (data) => this.handleSuggestionsError(data)
    );
    
    this.suggestionService.addEventListener(
      AiSuggestionService.EVENTS.SERVICE_STATUS_CHANGED,
      (data) => this.handleServiceStatusChange(data)
    );
    
    // Suggestion list events
    this.suggestionList.addEventListener(
      SuggestionList.EVENTS.SUGGESTION_SELECTED,
      (data) => this.handleSuggestionSelected(data)
    );
    
    this.suggestionList.addEventListener(
      SuggestionList.EVENTS.SUGGESTION_INSERTED,
      (data) => this.handleSuggestionInserted(data)
    );
    
    this.suggestionList.addEventListener(
      SuggestionList.EVENTS.SUGGESTION_REFERENCED,
      (data) => this.handleSuggestionReferenced(data)
    );
    
    this.suggestionList.addEventListener(
      'reload_suggestions_requested',
      () => this.reloadSuggestions()
    );
    
    console.log('✅ Event integration setup complete');
  }

  /**
   * Handle suggestions updated event
   * @param {Object} data - Suggestions data
   * @private
   */
  handleSuggestionsUpdated(data) {
    // Update suggestion list
    this.suggestionList.updateSuggestions(data.suggestions, false);
    
    // Update status indicator
    if (data.suggestions.length > 0) {
      const cacheStatus = data.fromCache ? ' (cached)' : '';
      this.updateStatusIndicator('loaded', `${data.suggestions.length} suggestions${cacheStatus}`);
    } else {
      this.updateStatusIndicator('empty', 'No suggestions available');
    }
    
    // Clear any previous errors
    this.clearError();
    
    // Emit controller event
    this.emit(AiPanelController.EVENTS.SUGGESTIONS_READY, {
      suggestions: data.suggestions,
      fromCache: data.fromCache,
      generationTime: data.generationTime
    });
  }

  /**
   * Handle suggestions loading event
   * @param {Object} data - Loading data
   * @private
   */
  handleSuggestionsLoading(data) {
    this.currentLoadingState = true;
    
    // Update suggestion list
    this.suggestionList.setLoading(true, 'Generating suggestions...');
    
    // Update status indicator
    this.updateStatusIndicator('loading', 'Generating suggestions...');
    
    // Emit controller event
    this.emit(AiPanelController.EVENTS.LOADING_STATE_CHANGED, {
      isLoading: true,
      context: data.context
    });
  }

  /**
   * Handle suggestions error event
   * @param {Object} data - Error data
   * @private
   */
  handleSuggestionsError(data) {
    this.currentLoadingState = false;
    this.lastError = data.error;
    
    // Update suggestion list
    this.suggestionList.setError(data.message);
    
    // Update status indicator
    this.updateStatusIndicator('error', 'Error generating suggestions');
    
    // Show error container
    this.showError(data.message, data.error);
    
    // Emit controller event
    this.emit(AiPanelController.EVENTS.SERVICE_ERROR, {
      error: data.error,
      message: data.message,
      context: data.context
    });
  }

  /**
   * Handle service status change event
   * @param {Object} data - Status change data
   * @private
   */
  handleServiceStatusChange(data) {
    const statusText = this.getStatusText(data.newStatus);
    this.updateStatusIndicator(data.newStatus.toLowerCase(), statusText);
    
    // Update suggestion list based on status
    if (data.newStatus === AiSuggestionService.STATUS.DISABLED) {
      this.suggestionList.updateSuggestions([], false);
    }
  }

  /**
   * Handle suggestion selection event
   * @param {Object} data - Selection data
   * @private
   */
  handleSuggestionSelected(data) {
    this.emit(AiPanelController.EVENTS.SUGGESTION_SELECTED, {
      suggestion: data.selectedSuggestion,
      index: data.selectedIndex
    });
  }

  /**
   * Handle suggestion insertion event
   * @param {Object} data - Insertion data
   * @private
   */
  handleSuggestionInserted(data) {
    // Update status indicator
    this.updateStatusIndicator('inserted', 'Content inserted');
    
    // Reset status after a delay
    setTimeout(() => {
      if (this.suggestionService.getCurrentSuggestions().length > 0) {
        this.updateStatusIndicator('loaded', 'Suggestions available');
      } else {
        this.updateStatusIndicator('active', 'AI Assistant Ready');
      }
    }, 2000);
    
    this.emit(AiPanelController.EVENTS.SUGGESTION_INSERTED, {
      suggestion: data.suggestion,
      insertedAt: data.insertedAt,
      contentLength: data.contentLength
    });
  }

  /**
   * Handle suggestion reference event
   * @param {Object} data - Reference data
   * @private
   */
  handleSuggestionReferenced(data) {
    // Update status indicator
    this.updateStatusIndicator('referenced', 'Reference added');
    
    // Reset status after a delay
    setTimeout(() => {
      if (this.suggestionService.getCurrentSuggestions().length > 0) {
        this.updateStatusIndicator('loaded', 'Suggestions available');
      } else {
        this.updateStatusIndicator('active', 'AI Assistant Ready');
      }
    }, 2000);
  }

  /**
   * Handle initialization errors
   * @param {Error} error - Initialization error
   * @private
   */
  handleInitializationError(error) {
    this.lastError = error;
    
    if (this.statusIndicator) {
      this.updateStatusIndicator('error', 'Initialization failed');
    }
    
    if (this.errorContainer) {
      this.showError('Failed to initialize AI features', error.message);
    }
  }

  /**
   * Update status indicator display
   * @param {string} status - Status type
   * @param {string} text - Status text
   * @private
   */
  updateStatusIndicator(status, text) {
    if (!this.statusIndicator) return;
    
    const statusIcon = this.statusIndicator.querySelector('.status-icon');
    const statusText = this.statusIndicator.querySelector('.status-text');
    
    if (statusIcon && statusText) {
      statusIcon.textContent = this.getStatusIcon(status);
      statusText.textContent = text;
      
      // Update CSS class for status-based styling
      this.statusIndicator.className = `ai-panel-status-indicator ai-panel-status-${status}`;
    }
  }

  /**
   * Get status icon for different states
   * @param {string} status - Status type
   * @returns {string} Status icon
   * @private
   */
  getStatusIcon(status) {
    const icons = {
      'active': '🤖',
      'inactive': '😴',
      'loading': '🔄',
      'loaded': '✅',
      'empty': '🤷',
      'error': '❌',
      'inserted': '📝',
      'referenced': '🔗',
      'initializing': '⚡',
      'ready': '🤖',
      'disabled': '⏸️'
    };
    
    return icons[status] || '🤖';
  }

  /**
   * Get status text for service states
   * @param {string} status - Service status
   * @returns {string} Status text
   * @private
   */
  getStatusText(status) {
    const texts = {
      [AiSuggestionService.STATUS.INITIALIZING]: 'Initializing AI services...',
      [AiSuggestionService.STATUS.READY]: 'AI Assistant Ready',
      [AiSuggestionService.STATUS.LOADING]: 'Generating suggestions...',
      [AiSuggestionService.STATUS.ERROR]: 'AI service error',
      [AiSuggestionService.STATUS.DISABLED]: 'AI Assistant Disabled'
    };
    
    return texts[status] || 'AI Assistant';
  }

  /**
   * Show error message
   * @param {string} title - Error title
   * @param {string} message - Error message
   * @private
   */
  showError(title, message) {
    if (!this.errorContainer) return;
    
    this.errorContainer.innerHTML = `
      <div class="error-content">
        <div class="error-icon">⚠️</div>
        <div class="error-title">${title}</div>
        <div class="error-message">${message}</div>
        <button class="error-retry-btn" onclick="this.closest('.ai-panel-error-container').__controller.reloadSuggestions()">
          Retry
        </button>
      </div>
    `;
    
    this.errorContainer.style.display = 'block';
    this.errorContainer.__controller = this;
  }

  /**
   * Clear error display
   * @private
   */
  clearError() {
    if (this.errorContainer) {
      this.errorContainer.style.display = 'none';
      this.errorContainer.innerHTML = '';
    }
    
    this.lastError = null;
  }

  /**
   * Reload suggestions manually
   */
  async reloadSuggestions() {
    try {
      this.clearError();
      await this.suggestionService.requestSuggestions();
    } catch (error) {
      console.error('Failed to reload suggestions:', error);
      this.handleSuggestionsError({
        message: 'Failed to reload suggestions',
        error: error.message
      });
    }
  }

  /**
   * Request immediate suggestion generation
   * @returns {Promise<Array>} Generated suggestions
   */
  async requestSuggestions() {
    if (!this.suggestionService) {
      throw new Error('Suggestion service not available');
    }
    
    return await this.suggestionService.requestSuggestions();
  }

  /**
   * Get current suggestions
   * @returns {Array} Current suggestions
   */
  getCurrentSuggestions() {
    return this.suggestionService ? this.suggestionService.getCurrentSuggestions() : [];
  }

  /**
   * Enable/disable AI suggestions
   * @param {boolean} enabled - Whether to enable suggestions
   */
  setSuggestionsEnabled(enabled) {
    if (this.suggestionService) {
      this.suggestionService.setEnabled(enabled);
    }
    
    // Update UI state
    if (enabled) {
      this.panelElement.classList.remove('suggestions-disabled');
      this.updateStatusIndicator('active', 'AI Assistant Active');
    } else {
      this.panelElement.classList.add('suggestions-disabled');
      this.updateStatusIndicator('disabled', 'AI Suggestions Disabled');
      this.suggestionList?.updateSuggestions([], false);
    }
  }

  /**
   * Update configuration for suggestion components
   * @param {Object} config - Configuration updates
   */
  updateConfig(config) {
    if (this.suggestionService) {
      this.suggestionService.updateConfig(config);
    }
    
    if (this.suggestionList && config.suggestionList) {
      this.suggestionList.updateConfig(config.suggestionList);
    }
  }

  /**
   * Get comprehensive status of all components
   * @returns {Object} Complete status object
   */
  getStatus() {
    return {
      isInitialized: this.isInitialized,
      panelVisible: this.aiPanel?.isVisible() || false,
      panelActive: this.aiPanel?.isActive() || false,
      currentLoadingState: this.currentLoadingState,
      lastError: this.lastError?.message || null,
      suggestionService: this.suggestionService?.getStatus() || null,
      suggestionList: this.suggestionList?.getStatus() || null,
      currentSuggestionsCount: this.getCurrentSuggestions().length
    };
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
   * @param {string} eventType - Event type
   * @param {Object} data - Event data
   * @private
   */
  emit(eventType, data) {
    const listeners = this.eventListeners.get(eventType);
    if (!listeners) return;

    listeners.forEach(handler => {
      try {
        handler(data);
      } catch (error) {
        console.error(`Error in AI panel controller event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Cleanup controller and all components
   */
  destroy() {
    console.log('🤖 Destroying AI Panel Controller...');
    
    // Cleanup components
    if (this.suggestionList) {
      this.suggestionList.destroy();
      this.suggestionList = null;
    }
    
    if (this.suggestionService) {
      this.suggestionService.destroy();
      this.suggestionService = null;
    }
    
    if (this.contentDetector) {
      this.contentDetector.destroy();
      this.contentDetector = null;
    }
    
    if (this.cacheManager) {
      this.cacheManager.destroy();
      this.cacheManager = null;
    }
    
    if (this.aiPanel) {
      this.aiPanel.destroy();
      this.aiPanel = null;
    }
    
    // Clear event listeners
    this.eventListeners.clear();
    
    // Clear DOM references
    if (this.errorContainer?.__controller === this) {
      delete this.errorContainer.__controller;
    }
    
    // Reset state
    this.isInitialized = false;
    this.currentLoadingState = false;
    this.lastError = null;
    
    console.log('✅ AI Panel Controller destroyed');
  }
}

export default AiPanelController;