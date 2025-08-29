/**
 * SuggestionCard Component - Enhanced individual suggestion display
 * 
 * Provides rich interactive suggestion cards with hover previews, click-to-navigate,
 * relevance indicators, and enhanced accessibility. This component extends the base
 * suggestion display with more sophisticated user interactions.
 * 
 * Features:
 * - Click-to-navigate to suggested notes
 * - Hover previews with extended content
 * - Enhanced relevance score visualization 
 * - Keyboard navigation support
 * - Accessible interactions
 * - Smooth animations and transitions
 * 
 * @class SuggestionCard
 */
class SuggestionCard {
  /**
   * Suggestion card events
   */
  static EVENTS = {
    CARD_CLICKED: 'suggestion_card_clicked',
    CARD_HOVERED: 'suggestion_card_hovered',
    CARD_FOCUSED: 'suggestion_card_focused',
    NAVIGATE_REQUESTED: 'suggestion_navigate_requested',
    PREVIEW_REQUESTED: 'suggestion_preview_requested',
    ACTION_TRIGGERED: 'suggestion_action_triggered'
  };

  /**
   * Card action types
   */
  static ACTIONS = {
    NAVIGATE: 'navigate',
    INSERT: 'insert', 
    REFERENCE: 'reference',
    PREVIEW: 'preview'
  };

  /**
   * Initialize suggestion card
   * @param {Object} suggestion - Suggestion data object
   * @param {number} index - Card index in the list
   * @param {Object} options - Configuration options
   */
  constructor(suggestion, index, options = {}) {
    if (!suggestion) {
      throw new Error('Suggestion data is required');
    }

    this.suggestion = suggestion;
    this.index = index;
    
    // Configuration with defaults
    this.options = {
      showRelevanceScore: true,
      showContextSnippet: true,
      enableHoverPreview: true,
      enableKeyboardNavigation: true,
      animationDuration: 200,
      previewDelay: 500,
      maxPreviewLength: 300,
      ...options
    };
    
    // State
    this.isSelected = false;
    this.isHovered = false;
    this.isFocused = false;
    this.previewVisible = false;
    
    // DOM elements
    this.element = null;
    this.previewElement = null;
    this.hoverTimeout = null;
    
    // Event listeners
    this.eventListeners = new Map();
    
    // Create the card element
    this.createElement();
    this.setupEventListeners();
  }

  /**
   * Create the DOM element for the suggestion card
   * @private
   */
  createElement() {
    this.element = document.createElement('div');
    this.element.className = 'suggestion-card';
    this.element.setAttribute('role', 'button');
    this.element.setAttribute('tabindex', '0');
    this.element.setAttribute('aria-label', this.buildAriaLabel());
    this.element.setAttribute('data-index', this.index);
    this.element.setAttribute('data-suggestion-id', this.suggestion.id);
    this.element.setAttribute('aria-selected', 'false');
    
    this.element.innerHTML = this.buildCardHTML();
    
    // Store reference for event handling
    this.element.__suggestionCard = this;
  }

  /**
   * Build ARIA label for accessibility
   * @private
   */
  buildAriaLabel() {
    const relevanceText = this.options.showRelevanceScore ? 
      `, relevance ${Math.round(this.suggestion.relevanceScore * 100)}%` : '';
    
    return `Suggestion ${this.index + 1}: ${this.suggestion.title}${relevanceText}. Press Enter to navigate or Space to preview.`;
  }

  /**
   * Build the inner HTML for the suggestion card
   * @private
   */
  buildCardHTML() {
    return `
      <div class="suggestion-card-header">
        <div class="suggestion-card-title-section">
          <h3 class="suggestion-card-title" title="${this.escapeHtml(this.suggestion.title)}">
            ${this.escapeHtml(this.suggestion.title)}
          </h3>
          ${this.buildFilePathIndicator()}
        </div>
        ${this.buildRelevanceIndicator()}
      </div>
      
      ${this.buildContextSnippet()}
      
      <div class="suggestion-card-actions">
        ${this.buildActionButtons()}
      </div>
      
      <div class="suggestion-card-metadata">
        ${this.buildMetadata()}
      </div>
      
      ${this.buildHoverPreview()}
    `;
  }

  /**
   * Build file path indicator
   * @private
   */
  buildFilePathIndicator() {
    if (!this.suggestion.filePath) return '';
    
    const pathParts = this.suggestion.filePath.split('/');
    const fileName = pathParts.pop() || '';
    const directory = pathParts.length > 0 ? pathParts[pathParts.length - 1] : '';
    
    return `
      <div class="suggestion-card-path" title="${this.escapeHtml(this.suggestion.filePath)}">
        <span class="path-directory">${this.escapeHtml(directory)}</span>
        ${directory ? '<span class="path-separator">/</span>' : ''}
        <span class="path-filename">${this.escapeHtml(fileName)}</span>
      </div>
    `;
  }

  /**
   * Build relevance score indicator
   * @private
   */
  buildRelevanceIndicator() {
    if (!this.options.showRelevanceScore) return '';
    
    const score = this.suggestion.relevanceScore;
    const percentage = Math.round(score * 100);
    const bars = Math.ceil(score * 5);
    
    let barsHTML = '';
    for (let i = 1; i <= 5; i++) {
      barsHTML += `<div class="relevance-bar ${i <= bars ? 'active' : ''}"></div>`;
    }
    
    return `
      <div class="suggestion-card-relevance" title="Relevance: ${percentage}%" role="meter" aria-valuenow="${percentage}" aria-valuemin="0" aria-valuemax="100">
        <div class="relevance-bars">${barsHTML}</div>
        <span class="relevance-percentage">${percentage}%</span>
      </div>
    `;
  }

  /**
   * Build context snippet section
   * @private
   */
  buildContextSnippet() {
    if (!this.options.showContextSnippet || !this.suggestion.contextSnippet) {
      return '';
    }
    
    return `
      <div class="suggestion-card-snippet">
        <p>${this.escapeHtml(this.suggestion.contextSnippet)}</p>
      </div>
    `;
  }

  /**
   * Build action buttons
   * @private
   */
  buildActionButtons() {
    return `
      <button class="suggestion-action-btn navigate-btn" data-action="${SuggestionCard.ACTIONS.NAVIGATE}" title="Navigate to note (Enter)" aria-label="Navigate to note">
        <span class="action-icon">🧭</span>
        <span class="action-text">Open</span>
      </button>
      <button class="suggestion-action-btn insert-btn" data-action="${SuggestionCard.ACTIONS.INSERT}" title="Insert content" aria-label="Insert suggestion content">
        <span class="action-icon">📝</span>
        <span class="action-text">Insert</span>
      </button>
      <button class="suggestion-action-btn reference-btn" data-action="${SuggestionCard.ACTIONS.REFERENCE}" title="Add as reference" aria-label="Add as reference link">
        <span class="action-icon">🔗</span>
        <span class="action-text">Link</span>
      </button>
    `;
  }

  /**
   * Build metadata section
   * @private
   */
  buildMetadata() {
    const metadata = this.suggestion.metadata || {};
    const lastModified = metadata.lastModified ? new Date(metadata.lastModified).toLocaleDateString() : '';
    const wordCount = metadata.wordCount || '';
    
    let metadataItems = [];
    if (lastModified) metadataItems.push(`Modified: ${lastModified}`);
    if (wordCount) metadataItems.push(`${wordCount} words`);
    
    if (metadataItems.length === 0) return '';
    
    return `
      <div class="suggestion-card-meta">
        <span class="meta-items">${metadataItems.join(' • ')}</span>
      </div>
    `;
  }

  /**
   * Build hover preview element
   * @private
   */
  buildHoverPreview() {
    if (!this.options.enableHoverPreview) return '';
    
    return `
      <div class="suggestion-card-preview" role="tooltip" aria-hidden="true">
        <div class="preview-header">
          <h4 class="preview-title">${this.escapeHtml(this.suggestion.title)}</h4>
          <button class="preview-close" aria-label="Close preview">×</button>
        </div>
        <div class="preview-content">
          <div class="preview-text"></div>
          <div class="preview-metadata">
            <span class="preview-path">${this.escapeHtml(this.suggestion.filePath || '')}</span>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Setup event listeners for card interactions
   * @private
   */
  setupEventListeners() {
    // Click handling
    this.element.addEventListener('click', this.handleCardClick.bind(this));
    
    // Keyboard navigation
    this.element.addEventListener('keydown', this.handleKeyDown.bind(this));
    
    // Hover effects
    this.element.addEventListener('mouseenter', this.handleMouseEnter.bind(this));
    this.element.addEventListener('mouseleave', this.handleMouseLeave.bind(this));
    
    // Focus handling
    this.element.addEventListener('focus', this.handleFocus.bind(this));
    this.element.addEventListener('blur', this.handleBlur.bind(this));
    
    // Action button handling
    this.element.addEventListener('click', this.handleActionClick.bind(this));
    
    // Preview close button
    if (this.options.enableHoverPreview) {
      const previewClose = this.element.querySelector('.preview-close');
      if (previewClose) {
        previewClose.addEventListener('click', this.hidePreview.bind(this));
      }
    }
  }

  /**
   * Handle card click events
   * @private
   */
  handleCardClick(event) {
    // Don't handle if clicking on action buttons
    if (event.target.closest('.suggestion-action-btn')) {
      return;
    }
    
    // Default action is navigate
    this.triggerAction(SuggestionCard.ACTIONS.NAVIGATE);
    
    this.emit(SuggestionCard.EVENTS.CARD_CLICKED, {
      suggestion: this.suggestion,
      index: this.index,
      action: SuggestionCard.ACTIONS.NAVIGATE
    });
  }

  /**
   * Handle keyboard navigation
   * @private
   */
  handleKeyDown(event) {
    if (!this.options.enableKeyboardNavigation) return;
    
    switch (event.key) {
      case 'Enter':
        event.preventDefault();
        this.triggerAction(SuggestionCard.ACTIONS.NAVIGATE);
        break;
        
      case ' ': // Space
        event.preventDefault();
        if (this.previewVisible) {
          this.hidePreview();
        } else {
          this.showPreview();
        }
        break;
        
      case 'i':
      case 'I':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          this.triggerAction(SuggestionCard.ACTIONS.INSERT);
        }
        break;
        
      case 'l':
      case 'L':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          this.triggerAction(SuggestionCard.ACTIONS.REFERENCE);
        }
        break;
        
      case 'Escape':
        event.preventDefault();
        this.hidePreview();
        this.element.blur();
        break;
    }
  }

  /**
   * Handle mouse enter events
   * @private
   */
  handleMouseEnter(event) {
    this.isHovered = true;
    this.element.classList.add('suggestion-card-hovered');
    
    // Start hover preview timer
    if (this.options.enableHoverPreview) {
      this.hoverTimeout = setTimeout(() => {
        this.showPreview();
      }, this.options.previewDelay);
    }
    
    this.emit(SuggestionCard.EVENTS.CARD_HOVERED, {
      suggestion: this.suggestion,
      index: this.index,
      isHovered: true
    });
  }

  /**
   * Handle mouse leave events
   * @private
   */
  handleMouseLeave(event) {
    this.isHovered = false;
    this.element.classList.remove('suggestion-card-hovered');
    
    // Cancel hover preview timer
    if (this.hoverTimeout) {
      clearTimeout(this.hoverTimeout);
      this.hoverTimeout = null;
    }
    
    // Hide preview after a short delay
    setTimeout(() => {
      if (!this.isHovered && !this.isFocused) {
        this.hidePreview();
      }
    }, 200);
    
    this.emit(SuggestionCard.EVENTS.CARD_HOVERED, {
      suggestion: this.suggestion,
      index: this.index,
      isHovered: false
    });
  }

  /**
   * Handle focus events
   * @private
   */
  handleFocus(event) {
    this.isFocused = true;
    this.element.classList.add('suggestion-card-focused');
    
    this.emit(SuggestionCard.EVENTS.CARD_FOCUSED, {
      suggestion: this.suggestion,
      index: this.index,
      isFocused: true
    });
  }

  /**
   * Handle blur events
   * @private
   */
  handleBlur(event) {
    this.isFocused = false;
    this.element.classList.remove('suggestion-card-focused');
    
    // Hide preview if not hovered
    setTimeout(() => {
      if (!this.isHovered && !this.isFocused) {
        this.hidePreview();
      }
    }, 100);
    
    this.emit(SuggestionCard.EVENTS.CARD_FOCUSED, {
      suggestion: this.suggestion,
      index: this.index,
      isFocused: false
    });
  }

  /**
   * Handle action button clicks
   * @private
   */
  handleActionClick(event) {
    const actionBtn = event.target.closest('.suggestion-action-btn');
    if (!actionBtn) return;
    
    event.stopPropagation();
    const action = actionBtn.dataset.action;
    this.triggerAction(action);
  }

  /**
   * Show hover preview
   */
  showPreview() {
    if (!this.options.enableHoverPreview || this.previewVisible) return;
    
    const previewElement = this.element.querySelector('.suggestion-card-preview');
    if (!previewElement) return;
    
    // Load extended content
    this.loadPreviewContent(previewElement);
    
    // Show preview
    previewElement.style.display = 'block';
    previewElement.setAttribute('aria-hidden', 'false');
    
    // Animate in
    requestAnimationFrame(() => {
      previewElement.classList.add('preview-visible');
    });
    
    this.previewVisible = true;
    
    this.emit(SuggestionCard.EVENTS.PREVIEW_REQUESTED, {
      suggestion: this.suggestion,
      index: this.index,
      visible: true
    });
  }

  /**
   * Hide hover preview
   */
  hidePreview() {
    if (!this.previewVisible) return;
    
    const previewElement = this.element.querySelector('.suggestion-card-preview');
    if (!previewElement) return;
    
    // Animate out
    previewElement.classList.remove('preview-visible');
    
    // Hide after animation
    setTimeout(() => {
      previewElement.style.display = 'none';
      previewElement.setAttribute('aria-hidden', 'true');
    }, this.options.animationDuration);
    
    this.previewVisible = false;
    
    this.emit(SuggestionCard.EVENTS.PREVIEW_REQUESTED, {
      suggestion: this.suggestion,
      index: this.index,
      visible: false
    });
  }

  /**
   * Load content for preview
   * @private
   */
  loadPreviewContent(previewElement) {
    const previewText = previewElement.querySelector('.preview-text');
    if (!previewText) return;
    
    // Use extended content or truncated version of main content
    let content = this.suggestion.content || this.suggestion.contextSnippet || '';
    
    if (content.length > this.options.maxPreviewLength) {
      content = content.substring(0, this.options.maxPreviewLength) + '...';
    }
    
    previewText.textContent = content;
  }

  /**
   * Trigger specific action
   * @param {string} action - Action type to trigger
   */
  triggerAction(action) {
    this.emit(SuggestionCard.EVENTS.ACTION_TRIGGERED, {
      suggestion: this.suggestion,
      index: this.index,
      action: action
    });
    
    switch (action) {
      case SuggestionCard.ACTIONS.NAVIGATE:
        this.emit(SuggestionCard.EVENTS.NAVIGATE_REQUESTED, {
          suggestion: this.suggestion,
          filePath: this.suggestion.filePath
        });
        break;
        
      case SuggestionCard.ACTIONS.INSERT:
        // Will be handled by parent component
        break;
        
      case SuggestionCard.ACTIONS.REFERENCE:
        // Will be handled by parent component
        break;
    }
  }

  /**
   * Set card selection state
   * @param {boolean} selected - Whether card is selected
   */
  setSelected(selected) {
    this.isSelected = selected;
    this.element.classList.toggle('suggestion-card-selected', selected);
    this.element.setAttribute('aria-selected', selected.toString());
  }

  /**
   * Update card data
   * @param {Object} newSuggestion - Updated suggestion data
   */
  updateSuggestion(newSuggestion) {
    this.suggestion = { ...this.suggestion, ...newSuggestion };
    
    // Update content
    this.element.innerHTML = this.buildCardHTML();
    
    // Re-setup event listeners
    this.setupEventListeners();
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
        console.error(`Error in suggestion card event handler for ${eventType}:`, error);
      }
    });
  }

  /**
   * Escape HTML to prevent XSS
   * @private
   */
  escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Get DOM element
   * @returns {HTMLElement} The card element
   */
  getElement() {
    return this.element;
  }

  /**
   * Get suggestion data
   * @returns {Object} The suggestion data
   */
  getSuggestion() {
    return { ...this.suggestion };
  }

  /**
   * Cleanup component
   */
  destroy() {
    // Clear timeouts
    if (this.hoverTimeout) {
      clearTimeout(this.hoverTimeout);
      this.hoverTimeout = null;
    }
    
    // Remove event listeners
    this.eventListeners.clear();
    
    // Remove DOM element reference
    if (this.element && this.element.__suggestionCard === this) {
      delete this.element.__suggestionCard;
    }
    
    // Clear references
    this.element = null;
    this.suggestion = null;
  }
}

export default SuggestionCard;