/**
 * SuggestionList Component - Real-time AI suggestion display system
 * 
 * Handles rendering and interactions for AI-powered note suggestions in the AI panel.
 * Provides smooth animations, loading states, empty states, and responsive interactions
 * with keyboard navigation support.
 * 
 * Performance targets:
 * - Suggestion UI updates in <16ms (60fps)
 * - Smooth animations without frame drops
 * - No UI blocking during suggestion loading
 * - Responsive interactions on all target devices
 * 
 * @class SuggestionList
 */
class SuggestionList {
  /**
   * Suggestion list events
   */
  static EVENTS = {
    SUGGESTION_SELECTED: 'suggestion_selected',
    SUGGESTION_INSERTED: 'suggestion_inserted',
    SUGGESTION_REFERENCED: 'suggestion_referenced',
    NAVIGATION_CHANGED: 'suggestion_navigation_changed',
    LOADING_CHANGED: 'suggestion_loading_changed',
    ERROR_OCCURRED: 'suggestion_error'
  };

  /**
   * UI state constants
   */
  static STATES = {
    EMPTY: 'empty',
    LOADING: 'loading',
    LOADED: 'loaded',
    ERROR: 'error'
  };

  /**
   * Initialize suggestion list component
   * @param {HTMLElement} container - Container element for the suggestion list
   * @param {MarkdownEditor} editor - Markdown editor instance for content insertion
   * @param {AppState} appState - Application state manager
   */
  constructor(container, editor, appState) {
    if (!container) {
      throw new Error('Container element is required');
    }
    if (!editor) {
      throw new Error('MarkdownEditor instance is required');
    }
    if (!appState) {
      throw new Error('AppState instance is required');
    }

    this.container = container;
    this.editor = editor;
    this.appState = appState;
    
    // Component state
    this.state = SuggestionList.STATES.EMPTY;
    this.suggestions = [];
    this.selectedIndex = -1;
    this.isLoading = false;
    this.error = null;
    
    // Performance optimization
    this.animationFrame = null;
    this.lastUpdateTime = 0;
    this.updateThrottleDelay = 16; // 60fps target
    
    // Configuration
    this.config = {
      maxSuggestions: 10,
      showRelevanceScores: true,
      showContextSnippets: true,
      enableKeyboardNavigation: true,
      animationDuration: 200,
      scrollThreshold: 3 // Items visible before scrolling
    };
    
    // Event listeners storage
    this.eventListeners = new Map();
    
    // Keyboard navigation state
    this.keyboardNavigationEnabled = false;
    
    // Initialize the component
    this.init();
  }

  /**
   * Initialize the suggestion list component
   * @private
   */
  init() {
    this.createElements();
    this.attachEventListeners();
    this.setupKeyboardNavigation();
    this.render();
    
    console.log('✅ SuggestionList initialized');
  }

  /**
   * Create DOM elements for the suggestion list
   * @private
   */
  createElements() {
    // Clear container
    this.container.innerHTML = '';
    
    // Create main suggestion list element
    this.listElement = document.createElement('div');
    this.listElement.className = 'suggestion-list';
    this.listElement.setAttribute('role', 'listbox');
    this.listElement.setAttribute('aria-label', 'AI Suggestions');
    
    // Create loading indicator
    this.loadingElement = document.createElement('div');
    this.loadingElement.className = 'suggestion-loading';
    this.loadingElement.innerHTML = `
      <div class="loading-spinner"></div>
      <div class="loading-text">Generating suggestions...</div>
    `;
    
    // Create empty state element
    this.emptyElement = document.createElement('div');
    this.emptyElement.className = 'suggestion-empty-state';
    this.emptyElement.innerHTML = `
      <div class="empty-icon">🤖</div>
      <div class="empty-title">No suggestions yet</div>
      <div class="empty-description">Start typing to get AI-powered suggestions</div>
    `;
    
    // Create error state element
    this.errorElement = document.createElement('div');
    this.errorElement.className = 'suggestion-error-state';
    this.errorElement.innerHTML = `
      <div class="error-icon">⚠️</div>
      <div class="error-title">Unable to load suggestions</div>
      <div class="error-description"></div>
      <button class="retry-btn" onclick="this.parentElement.parentElement.__suggestionList.reloadSuggestions()">
        Retry
      </button>
    `;
    
    // Create suggestions container
    this.suggestionsContainer = document.createElement('div');
    this.suggestionsContainer.className = 'suggestions-container';
    
    // Add elements to main container
    this.container.appendChild(this.listElement);
    this.listElement.appendChild(this.loadingElement);
    this.listElement.appendChild(this.emptyElement);
    this.listElement.appendChild(this.errorElement);
    this.listElement.appendChild(this.suggestionsContainer);
    
    // Store reference for event handlers
    this.container.__suggestionList = this;
  }

  /**
   * Attach event listeners
   * @private
   */
  attachEventListeners() {
    // Click handling for suggestion selection
    this.suggestionsContainer.addEventListener('click', this.handleSuggestionClick.bind(this));
    
    // Mouse hover for visual feedback
    this.suggestionsContainer.addEventListener('mouseover', this.handleSuggestionHover.bind(this));
    this.suggestionsContainer.addEventListener('mouseleave', this.handleSuggestionLeave.bind(this));
    
    // Keyboard navigation
    if (this.config.enableKeyboardNavigation) {
      document.addEventListener('keydown', this.handleKeyboardNavigation.bind(this));
    }
  }

  /**
   * Setup keyboard navigation
   * @private
   */
  setupKeyboardNavigation() {
    this.keyboardShortcuts = {
      'ArrowUp': () => this.navigateSuggestions(-1),
      'ArrowDown': () => this.navigateSuggestions(1),
      'Enter': () => this.insertSelectedSuggestion(),
      'Tab': () => this.insertSelectedSuggestion(),
      'Escape': () => this.clearSelection()
    };
  }

  /**
   * Update suggestions with smooth animation
   * @param {Array} suggestions - Array of suggestion objects
   * @param {boolean} isLoading - Whether suggestions are currently loading
   */
  updateSuggestions(suggestions = [], isLoading = false) {
    // Throttle updates for performance
    const now = performance.now();
    if (now - this.lastUpdateTime < this.updateThrottleDelay && !isLoading) {
      if (this.animationFrame) {
        cancelAnimationFrame(this.animationFrame);
      }
      this.animationFrame = requestAnimationFrame(() => {
        this.updateSuggestions(suggestions, isLoading);
      });
      return;
    }
    
    this.lastUpdateTime = now;
    this.isLoading = isLoading;
    
    // Validate and process suggestions
    const validSuggestions = this.validateAndProcessSuggestions(suggestions);
    
    // Update state based on suggestions and loading status
    if (isLoading) {
      this.setState(SuggestionList.STATES.LOADING);
    } else if (this.error) {
      this.setState(SuggestionList.STATES.ERROR);
    } else if (validSuggestions.length === 0) {
      this.setState(SuggestionList.STATES.EMPTY);
    } else {
      this.setState(SuggestionList.STATES.LOADED);
      this.suggestions = validSuggestions;
    }
    
    this.render();
    
    // Emit loading changed event
    this.emit(SuggestionList.EVENTS.LOADING_CHANGED, {
      isLoading: this.isLoading,
      suggestionCount: validSuggestions.length
    });
  }

  /**
   * Validate and process raw suggestions data
   * @param {Array} rawSuggestions - Raw suggestions from backend
   * @returns {Array} Processed suggestions
   * @private
   */
  validateAndProcessSuggestions(rawSuggestions) {
    if (!Array.isArray(rawSuggestions)) {
      console.warn('Invalid suggestions data: not an array');
      return [];
    }
    
    return rawSuggestions
      .filter(suggestion => this.isValidSuggestion(suggestion))
      .slice(0, this.config.maxSuggestions)
      .map((suggestion, index) => ({
        id: suggestion.id || `suggestion-${index}`,
        title: suggestion.title || suggestion.file_path || 'Untitled',
        content: suggestion.content || suggestion.snippet || '',
        relevanceScore: parseFloat(suggestion.relevance_score || suggestion.similarity || 0),
        contextSnippet: this.extractContextSnippet(suggestion),
        filePath: suggestion.file_path || '',
        metadata: suggestion.metadata || {},
        index: index
      }));
  }

  /**
   * Check if a suggestion object is valid
   * @param {Object} suggestion - Suggestion object to validate
   * @returns {boolean} True if valid
   * @private
   */
  isValidSuggestion(suggestion) {
    return suggestion && 
           typeof suggestion === 'object' && 
           (suggestion.title || suggestion.file_path || suggestion.content);
  }

  /**
   * Extract context snippet from suggestion
   * @param {Object} suggestion - Suggestion object
   * @returns {string} Context snippet
   * @private
   */
  extractContextSnippet(suggestion) {
    const snippet = suggestion.context_snippet || suggestion.content || '';
    const maxLength = 120;
    
    if (snippet.length <= maxLength) {
      return snippet;
    }
    
    return snippet.substring(0, maxLength).trim() + '...';
  }

  /**
   * Set component state and update UI accordingly
   * @param {string} newState - New state from SuggestionList.STATES
   * @private
   */
  setState(newState) {
    if (this.state === newState) return;
    
    this.state = newState;
    
    // Update CSS classes for state-based styling
    this.listElement.className = `suggestion-list suggestion-list--${newState}`;
    
    // Reset selection when state changes
    this.selectedIndex = -1;
  }

  /**
   * Render the suggestion list based on current state
   * @private
   */
  render() {
    // Show/hide state-based elements
    this.loadingElement.style.display = this.state === SuggestionList.STATES.LOADING ? 'block' : 'none';
    this.emptyElement.style.display = this.state === SuggestionList.STATES.EMPTY ? 'block' : 'none';
    this.errorElement.style.display = this.state === SuggestionList.STATES.ERROR ? 'block' : 'none';
    this.suggestionsContainer.style.display = this.state === SuggestionList.STATES.LOADED ? 'block' : 'none';
    
    // Render suggestions if in loaded state
    if (this.state === SuggestionList.STATES.LOADED) {
      this.renderSuggestions();
    }
    
    // Update error message if in error state
    if (this.state === SuggestionList.STATES.ERROR && this.error) {
      const errorDescription = this.errorElement.querySelector('.error-description');
      if (errorDescription) {
        errorDescription.textContent = this.error.message || 'An unexpected error occurred';
      }
    }
  }

  /**
   * Render individual suggestions
   * @private
   */
  renderSuggestions() {
    // Clear existing suggestions
    this.suggestionsContainer.innerHTML = '';
    
    // Create document fragment for efficient DOM manipulation
    const fragment = document.createDocumentFragment();
    
    this.suggestions.forEach((suggestion, index) => {
      const suggestionElement = this.createSuggestionElement(suggestion, index);
      fragment.appendChild(suggestionElement);
    });
    
    // Add all suggestions at once
    this.suggestionsContainer.appendChild(fragment);
    
    // Apply entrance animation
    requestAnimationFrame(() => {
      this.suggestionsContainer.classList.add('suggestions-loaded');
    });
  }

  /**
   * Create DOM element for a single suggestion
   * @param {Object} suggestion - Suggestion data
   * @param {number} index - Suggestion index
   * @returns {HTMLElement} Suggestion element
   * @private
   */
  createSuggestionElement(suggestion, index) {
    const element = document.createElement('div');
    element.className = 'suggestion-item';
    element.setAttribute('role', 'option');
    element.setAttribute('aria-label', `Suggestion ${index + 1}: ${suggestion.title}`);
    element.setAttribute('data-index', index);
    element.setAttribute('data-id', suggestion.id);
    
    // Build suggestion HTML
    let html = `
      <div class="suggestion-header">
        <div class="suggestion-title">${this.escapeHtml(suggestion.title)}</div>
        ${this.config.showRelevanceScores ? `
          <div class="suggestion-score" title="Relevance: ${(suggestion.relevanceScore * 100).toFixed(1)}%">
            ${this.renderRelevanceIndicator(suggestion.relevanceScore)}
          </div>
        ` : ''}
      </div>
    `;
    
    if (this.config.showContextSnippets && suggestion.contextSnippet) {
      html += `
        <div class="suggestion-snippet">
          ${this.escapeHtml(suggestion.contextSnippet)}
        </div>
      `;
    }
    
    html += `
      <div class="suggestion-actions">
        <button class="suggestion-action-btn insert-btn" 
                data-action="insert" 
                title="Insert content (Enter/Tab)"
                aria-label="Insert suggestion content">
          📝
        </button>
        <button class="suggestion-action-btn reference-btn" 
                data-action="reference" 
                title="Add as reference link"
                aria-label="Add as reference">
          🔗
        </button>
      </div>
    `;
    
    element.innerHTML = html;
    
    // Add click handlers for action buttons
    const insertBtn = element.querySelector('.insert-btn');
    const referenceBtn = element.querySelector('.reference-btn');
    
    insertBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.insertSuggestion(suggestion);
    });
    
    referenceBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.referenceSuggestion(suggestion);
    });
    
    return element;
  }

  /**
   * Render relevance indicator based on score
   * @param {number} score - Relevance score (0-1)
   * @returns {string} HTML for relevance indicator
   * @private
   */
  renderRelevanceIndicator(score) {
    const percentage = Math.round(score * 100);
    const bars = Math.ceil(score * 5); // 5 bars max
    
    let indicator = '<div class="relevance-indicator">';
    for (let i = 1; i <= 5; i++) {
      const active = i <= bars ? 'active' : '';
      indicator += `<div class="relevance-bar ${active}"></div>`;
    }
    indicator += `</div><span class="relevance-text">${percentage}%</span>`;
    
    return indicator;
  }

  /**
   * Handle suggestion click events
   * @param {Event} event - Click event
   * @private
   */
  handleSuggestionClick(event) {
    const suggestionElement = event.target.closest('.suggestion-item');
    if (!suggestionElement) return;
    
    const index = parseInt(suggestionElement.dataset.index);
    this.selectSuggestion(index);
    
    // Default action is to insert suggestion
    if (!event.target.closest('.suggestion-action-btn')) {
      this.insertSelectedSuggestion();
    }
  }

  /**
   * Handle suggestion hover events
   * @param {Event} event - Mouse event
   * @private
   */
  handleSuggestionHover(event) {
    const suggestionElement = event.target.closest('.suggestion-item');
    if (!suggestionElement) return;
    
    const index = parseInt(suggestionElement.dataset.index);
    this.selectSuggestion(index, false); // Don't emit navigation event for mouse hover
  }

  /**
   * Handle mouse leave events
   * @private
   */
  handleSuggestionLeave() {
    // Keep selection but remove hover effect
    if (this.selectedIndex >= 0) {
      this.updateSelectionDisplay();
    }
  }

  /**
   * Handle keyboard navigation
   * @param {KeyboardEvent} event - Keyboard event
   * @private
   */
  handleKeyboardNavigation(event) {
    // Only handle when AI panel is focused and has suggestions
    if (!this.keyboardNavigationEnabled || 
        this.state !== SuggestionList.STATES.LOADED || 
        this.suggestions.length === 0) {
      return;
    }
    
    // Check if AI panel is focused or active
    if (!this.container.closest('.ai-panel').classList.contains('ai-panel-visible')) {
      return;
    }
    
    const handler = this.keyboardShortcuts[event.key];
    if (handler) {
      event.preventDefault();
      event.stopPropagation();
      handler();
    }
  }

  /**
   * Navigate suggestions with keyboard
   * @param {number} direction - Navigation direction (-1 for up, 1 for down)
   * @private
   */
  navigateSuggestions(direction) {
    if (this.suggestions.length === 0) return;
    
    const newIndex = this.selectedIndex + direction;
    
    if (newIndex < -1) {
      this.selectSuggestion(this.suggestions.length - 1);
    } else if (newIndex >= this.suggestions.length) {
      this.selectSuggestion(-1);
    } else {
      this.selectSuggestion(newIndex);
    }
  }

  /**
   * Select a suggestion by index
   * @param {number} index - Suggestion index (-1 to clear selection)
   * @param {boolean} emitEvent - Whether to emit navigation event
   */
  selectSuggestion(index, emitEvent = true) {
    if (index < -1 || index >= this.suggestions.length) return;
    
    this.selectedIndex = index;
    this.updateSelectionDisplay();
    
    // Scroll selected item into view
    if (index >= 0) {
      this.scrollToSuggestion(index);
    }
    
    if (emitEvent) {
      this.emit(SuggestionList.EVENTS.NAVIGATION_CHANGED, {
        selectedIndex: this.selectedIndex,
        selectedSuggestion: this.selectedIndex >= 0 ? this.suggestions[this.selectedIndex] : null
      });
    }
  }

  /**
   * Update visual selection display
   * @private
   */
  updateSelectionDisplay() {
    const suggestionElements = this.suggestionsContainer.querySelectorAll('.suggestion-item');
    
    suggestionElements.forEach((element, index) => {
      const isSelected = index === this.selectedIndex;
      element.classList.toggle('selected', isSelected);
      element.setAttribute('aria-selected', isSelected);
    });
  }

  /**
   * Scroll selected suggestion into view
   * @param {number} index - Suggestion index to scroll to
   * @private
   */
  scrollToSuggestion(index) {
    const suggestionElement = this.suggestionsContainer.querySelector(`[data-index="${index}"]`);
    if (!suggestionElement) return;
    
    suggestionElement.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
      inline: 'nearest'
    });
  }

  /**
   * Insert selected suggestion content
   */
  insertSelectedSuggestion() {
    if (this.selectedIndex < 0 || this.selectedIndex >= this.suggestions.length) return;
    
    const suggestion = this.suggestions[this.selectedIndex];
    this.insertSuggestion(suggestion);
  }

  /**
   * Insert suggestion content into editor
   * @param {Object} suggestion - Suggestion to insert
   */
  insertSuggestion(suggestion) {
    try {
      // Get current cursor position
      const cursorPosition = this.editor.cursorPosition || 0;
      
      // Insert content at current cursor position
      this.editor.insertText(suggestion.content, false);
      
      // Emit suggestion inserted event
      this.emit(SuggestionList.EVENTS.SUGGESTION_INSERTED, {
        suggestion,
        insertedAt: cursorPosition,
        contentLength: suggestion.content.length
      });
      
      console.log(`📝 Inserted suggestion: ${suggestion.title}`);
      
    } catch (error) {
      console.error('Failed to insert suggestion:', error);
      this.handleError('Failed to insert suggestion content');
    }
  }

  /**
   * Add suggestion as reference link
   * @param {Object} suggestion - Suggestion to reference
   */
  referenceSuggestion(suggestion) {
    try {
      // Create markdown link reference
      const linkText = `[${suggestion.title}]`;
      const referencePath = suggestion.filePath || suggestion.title;
      
      // Get current cursor position
      const cursorPosition = this.editor.cursorPosition || 0;
      
      // Insert link at current cursor position
      this.editor.insertText(linkText, false);
      
      // Emit suggestion referenced event
      this.emit(SuggestionList.EVENTS.SUGGESTION_REFERENCED, {
        suggestion,
        linkText,
        referencePath,
        insertedAt: cursorPosition
      });
      
      console.log(`🔗 Referenced suggestion: ${suggestion.title}`);
      
    } catch (error) {
      console.error('Failed to reference suggestion:', error);
      this.handleError('Failed to add suggestion reference');
    }
  }

  /**
   * Clear current selection
   */
  clearSelection() {
    this.selectSuggestion(-1);
  }

  /**
   * Enable keyboard navigation
   */
  enableKeyboardNavigation() {
    this.keyboardNavigationEnabled = true;
  }

  /**
   * Disable keyboard navigation
   */
  disableKeyboardNavigation() {
    this.keyboardNavigationEnabled = false;
    this.clearSelection();
  }

  /**
   * Set loading state
   * @param {boolean} isLoading - Loading state
   * @param {string} message - Optional loading message
   */
  setLoading(isLoading, message = 'Generating suggestions...') {
    this.isLoading = isLoading;
    
    if (isLoading) {
      this.setState(SuggestionList.STATES.LOADING);
      const loadingText = this.loadingElement.querySelector('.loading-text');
      if (loadingText) {
        loadingText.textContent = message;
      }
    }
    
    this.render();
  }

  /**
   * Set error state
   * @param {string|Error} error - Error message or Error object
   */
  setError(error) {
    this.error = typeof error === 'string' ? new Error(error) : error;
    this.setState(SuggestionList.STATES.ERROR);
    this.render();
    
    this.emit(SuggestionList.EVENTS.ERROR_OCCURRED, {
      error: this.error,
      timestamp: Date.now()
    });
  }

  /**
   * Clear error state
   */
  clearError() {
    this.error = null;
    if (this.state === SuggestionList.STATES.ERROR) {
      this.setState(this.suggestions.length > 0 ? 
                   SuggestionList.STATES.LOADED : 
                   SuggestionList.STATES.EMPTY);
      this.render();
    }
  }

  /**
   * Handle errors with user-friendly display
   * @param {string} message - Error message to display
   * @private
   */
  handleError(message) {
    console.error('SuggestionList error:', message);
    this.setError(message);
  }

  /**
   * Reload suggestions (for retry functionality)
   */
  reloadSuggestions() {
    this.clearError();
    this.setLoading(true);
    
    // Emit event for parent component to handle reload
    this.emit('reload_suggestions_requested', {
      timestamp: Date.now()
    });
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
        console.error(`Error in suggestion list event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Escape HTML to prevent XSS
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   * @private
   */
  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Update component configuration
   * @param {Object} config - Configuration updates
   */
  updateConfig(config) {
    this.config = { ...this.config, ...config };
    
    if (config.enableKeyboardNavigation !== undefined) {
      if (config.enableKeyboardNavigation) {
        this.enableKeyboardNavigation();
      } else {
        this.disableKeyboardNavigation();
      }
    }
    
    // Re-render if needed
    if (this.state === SuggestionList.STATES.LOADED) {
      this.render();
    }
  }

  /**
   * Get current component status
   * @returns {Object} Current status
   */
  getStatus() {
    return {
      state: this.state,
      isLoading: this.isLoading,
      suggestionCount: this.suggestions.length,
      selectedIndex: this.selectedIndex,
      keyboardNavigationEnabled: this.keyboardNavigationEnabled,
      hasError: !!this.error
    };
  }

  /**
   * Cleanup component resources
   */
  destroy() {
    // Cancel animation frames
    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
    }
    
    // Remove event listeners
    if (this.config.enableKeyboardNavigation) {
      document.removeEventListener('keydown', this.handleKeyboardNavigation.bind(this));
    }
    
    // Clear event listeners
    this.eventListeners.clear();
    
    // Clear DOM references
    if (this.container.__suggestionList === this) {
      delete this.container.__suggestionList;
    }
    
    // Reset state
    this.suggestions = [];
    this.selectedIndex = -1;
    this.isLoading = false;
    this.error = null;
    
    console.log('✅ SuggestionList destroyed');
  }
}

export default SuggestionList;