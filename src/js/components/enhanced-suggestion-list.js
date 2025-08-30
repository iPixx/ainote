/**
 * Enhanced Suggestion List Component - Integrates SuggestionCard components
 * 
 * Extends the base SuggestionList with enhanced SuggestionCard components
 * that provide click-to-navigate, hover previews, and improved interactions.
 * This component implements the requirements from GitHub issue #74.
 * 
 * @class EnhancedSuggestionList
 */
import SuggestionList from './suggestion-list.js';
import SuggestionCard from './suggestion-card.js';
import NavigationService from '../services/navigation-service.js';

class EnhancedSuggestionList extends SuggestionList {
  /**
   * Enhanced suggestion list events
   */
  static EVENTS = {
    ...SuggestionList.EVENTS,
    CARD_NAVIGATION_REQUESTED: 'card_navigation_requested',
    CARD_PREVIEW_SHOWN: 'card_preview_shown',
    CARD_PREVIEW_HIDDEN: 'card_preview_hidden',
    CARD_ACTION_PERFORMED: 'card_action_performed'
  };

  /**
   * Initialize enhanced suggestion list
   * @param {HTMLElement} container - Container element
   * @param {MarkdownEditor} editor - Markdown editor instance  
   * @param {AppState} appState - Application state manager
   * @param {NavigationService} navigationService - Navigation service instance
   * @param {Object} options - Enhanced configuration options
   */
  constructor(container, editor, appState, navigationService, options = {}) {
    // Initialize base suggestion list
    super(container, editor, appState);

    if (!navigationService) {
      throw new Error('NavigationService instance is required');
    }

    this.navigationService = navigationService;
    
    // Enhanced configuration
    this.enhancedConfig = {
      useCardComponents: true,
      enableNavigationIntegration: true,
      enableHoverPreviews: true,
      enableKeyboardNavigation: true,
      previewDelay: 500,
      animationDuration: 200,
      maxPreviewLength: 300,
      ...options
    };

    // Card components storage
    this.suggestionCards = new Map();
    this.activePreviewCard = null;
    
    // Performance tracking
    this.cardPerformanceStats = {
      cardsCreated: 0,
      cardsDestroyed: 0,
      previewsShown: 0,
      navigationsTriggered: 0,
      averageCardCreationTime: 0
    };
    
    // Override base configuration with enhanced options
    this.config = {
      ...this.config,
      ...this.enhancedConfig
    };

    console.log('✅ Enhanced Suggestion List initialized');
  }


  /**
   * Render suggestions using SuggestionCard components
   * @private
   */
  renderSuggestions() {
    if (!this.enhancedConfig.useCardComponents) {
      // Fallback to base implementation
      return super.renderSuggestions();
    }

    console.log('🎨 Rendering suggestions with enhanced cards');
    const startTime = performance.now();

    // Clear existing cards first
    this.clearSuggestionCards();
    
    // Clear the container
    this.suggestionsContainer.innerHTML = '';

    // Create document fragment for efficient DOM manipulation
    const fragment = document.createDocumentFragment();

    this.suggestions.forEach((suggestion, index) => {
      const card = this.createSuggestionCard(suggestion, index);
      if (card) {
        fragment.appendChild(card.getElement());
        this.suggestionCards.set(suggestion.id, card);
      }
    });

    // Add all cards at once
    this.suggestionsContainer.appendChild(fragment);

    // Apply entrance animation
    requestAnimationFrame(() => {
      this.suggestionsContainer.classList.add('suggestions-loaded');
    });

    // Update performance stats
    const renderTime = performance.now() - startTime;
    this.updateCardPerformanceStats('render', renderTime);

    console.log(`✅ Rendered ${this.suggestions.length} suggestion cards in ${renderTime.toFixed(2)}ms`);
  }

  /**
   * Override updateSuggestions to handle card cleanup for empty states
   */
  updateSuggestions(suggestions = [], isLoading = false) {
    // Call parent implementation
    super.updateSuggestions(suggestions, isLoading);
    
    // Handle empty suggestions case - parent won't call renderSuggestions when empty
    if (!isLoading && (this.state === SuggestionList.STATES.EMPTY || suggestions.length === 0)) {
      this.clearSuggestionCards();
    }
  }

  /**
   * Create enhanced suggestion card component
   * @private
   */
  createSuggestionCard(suggestion, index) {
    const startTime = performance.now();

    try {
      const cardOptions = {
        showRelevanceScore: this.config.showRelevanceScores,
        showContextSnippet: this.config.showContextSnippets,
        enableHoverPreview: this.enhancedConfig.enableHoverPreviews,
        enableKeyboardNavigation: this.enhancedConfig.enableKeyboardNavigation,
        animationDuration: this.enhancedConfig.animationDuration,
        previewDelay: this.enhancedConfig.previewDelay,
        maxPreviewLength: this.enhancedConfig.maxPreviewLength
      };

      const card = new SuggestionCard(suggestion, index, cardOptions);
      
      // Set up card event listeners
      this.setupCardEventListeners(card);
      
      // Update performance stats
      const creationTime = performance.now() - startTime;
      this.cardPerformanceStats.cardsCreated++;
      this.updateAverageCardCreationTime(creationTime);

      return card;

    } catch (error) {
      console.error('Failed to create suggestion card:', error);
      return null;
    }
  }

  /**
   * Setup event listeners for individual suggestion cards
   * @private
   */
  setupCardEventListeners(card) {
    // Navigation requests
    card.addEventListener(SuggestionCard.EVENTS.NAVIGATE_REQUESTED, (data) => {
      this.handleCardNavigation(data);
    });

    // Action triggers (insert, reference, etc.)
    card.addEventListener(SuggestionCard.EVENTS.ACTION_TRIGGERED, (data) => {
      this.handleCardAction(data);
    });

    // Preview events
    card.addEventListener(SuggestionCard.EVENTS.PREVIEW_REQUESTED, (data) => {
      this.handleCardPreview(data);
    });

    // Card focus/selection events
    card.addEventListener(SuggestionCard.EVENTS.CARD_FOCUSED, (data) => {
      this.handleCardFocus(data);
    });

    card.addEventListener(SuggestionCard.EVENTS.CARD_CLICKED, (data) => {
      this.handleCardClick(data);
    });
  }

  /**
   * Handle navigation request from suggestion card
   * @private
   */
  async handleCardNavigation(data) {
    if (!this.enhancedConfig.enableNavigationIntegration) {
      console.warn('Navigation integration is disabled');
      return;
    }

    console.log('🧭 Handling card navigation request:', data.suggestion.title);
    
    try {
      this.cardPerformanceStats.navigationsTriggered++;

      // Use navigation service to open the file
      const navigationSuccess = await this.navigationService.navigateToSuggestion(data.suggestion);
      
      if (navigationSuccess) {
        // Emit navigation event
        this.emit(EnhancedSuggestionList.EVENTS.CARD_NAVIGATION_REQUESTED, {
          suggestion: data.suggestion,
          success: true,
          timestamp: Date.now()
        });

        // Optional: Hide AI panel after successful navigation
        // This could be made configurable
        this.emitNavigationSuccess(data.suggestion);
      } else {
        console.warn('Navigation failed for suggestion:', data.suggestion.title);
        this.emitNavigationError(data.suggestion);
      }

    } catch (error) {
      console.error('Card navigation error:', error);
      this.emitNavigationError(data.suggestion, error);
    }
  }

  /**
   * Handle action triggers from suggestion cards
   * @private
   */
  handleCardAction(data) {
    console.log('⚡ Handling card action:', data.action, data.suggestion.title);

    switch (data.action) {
      case SuggestionCard.ACTIONS.INSERT:
        this.insertSuggestion(data.suggestion);
        break;
        
      case SuggestionCard.ACTIONS.REFERENCE:
        this.referenceSuggestion(data.suggestion);
        break;
        
      case SuggestionCard.ACTIONS.NAVIGATE:
        this.handleCardNavigation(data);
        break;
        
      default:
        console.warn('Unknown card action:', data.action);
    }

    // Emit action event
    this.emit(EnhancedSuggestionList.EVENTS.CARD_ACTION_PERFORMED, {
      suggestion: data.suggestion,
      action: data.action,
      timestamp: Date.now()
    });
  }

  /**
   * Handle preview events from suggestion cards
   * @private
   */
  handleCardPreview(data) {
    if (data.visible) {
      console.log('👁️ Showing card preview:', data.suggestion.title);
      this.activePreviewCard = data.suggestion.id;
      this.cardPerformanceStats.previewsShown++;
      
      this.emit(EnhancedSuggestionList.EVENTS.CARD_PREVIEW_SHOWN, data);
    } else {
      console.log('🙈 Hiding card preview:', data.suggestion.title);
      this.activePreviewCard = null;
      
      this.emit(EnhancedSuggestionList.EVENTS.CARD_PREVIEW_HIDDEN, data);
    }
  }

  /**
   * Handle card focus events for keyboard navigation
   * @private
   */
  handleCardFocus(data) {
    if (data.isFocused) {
      // Update selected index to match focused card
      this.selectedIndex = data.index;
      this.updateCardSelection();
    }
  }

  /**
   * Handle card click events
   * @private
   */
  handleCardClick(data) {
    // Update selection
    this.selectSuggestion(data.index);
    
    // Emit selection event
    this.emit(SuggestionList.EVENTS.SUGGESTION_SELECTED, {
      suggestion: data.suggestion,
      index: data.index
    });
  }

  /**
   * Override selectSuggestion to work with card components
   */
  selectSuggestion(index, emitEvent = true) {
    // Call base implementation
    super.selectSuggestion(index, emitEvent);
    
    // Update card selection states
    this.updateCardSelection();
  }

  /**
   * Update selection state for all cards
   * @private
   */
  updateCardSelection() {
    this.suggestionCards.forEach((card, suggestionId) => {
      const isSelected = card.index === this.selectedIndex;
      card.setSelected(isSelected);
    });
  }

  /**
   * Override keyboard navigation to work with cards
   */
  handleKeyboardNavigation(event) {
    // Only handle if cards are enabled and navigation is active
    if (!this.enhancedConfig.useCardComponents || !this.keyboardNavigationEnabled) {
      return super.handleKeyboardNavigation(event);
    }

    // Check if AI panel is focused
    if (!this.container.closest('.ai-panel').classList.contains('ai-panel-visible')) {
      return;
    }

    switch (event.key) {
      case 'ArrowUp':
      case 'ArrowDown':
        event.preventDefault();
        const direction = event.key === 'ArrowUp' ? -1 : 1;
        this.navigateSuggestions(direction);
        this.focusSelectedCard();
        break;
        
      case 'Enter':
        event.preventDefault();
        this.triggerSelectedCardAction(SuggestionCard.ACTIONS.NAVIGATE);
        break;
        
      case ' ':
        event.preventDefault();
        this.toggleSelectedCardPreview();
        break;
        
      case 'i':
      case 'I':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          this.triggerSelectedCardAction(SuggestionCard.ACTIONS.INSERT);
        }
        break;
        
      case 'l':
      case 'L':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          this.triggerSelectedCardAction(SuggestionCard.ACTIONS.REFERENCE);
        }
        break;
        
      case 'Escape':
        event.preventDefault();
        this.hideAllPreviews();
        this.clearSelection();
        break;
    }
  }

  /**
   * Focus the currently selected card
   * @private
   */
  focusSelectedCard() {
    if (this.selectedIndex < 0) return;
    
    const selectedSuggestion = this.suggestions[this.selectedIndex];
    if (!selectedSuggestion) return;
    
    const card = this.suggestionCards.get(selectedSuggestion.id);
    if (card) {
      card.getElement().focus();
    }
  }

  /**
   * Trigger action on selected card
   * @private
   */
  triggerSelectedCardAction(action) {
    if (this.selectedIndex < 0) return;
    
    const selectedSuggestion = this.suggestions[this.selectedIndex];
    if (!selectedSuggestion) return;
    
    const card = this.suggestionCards.get(selectedSuggestion.id);
    if (card) {
      card.triggerAction(action);
    }
  }

  /**
   * Toggle preview for selected card
   * @private
   */
  toggleSelectedCardPreview() {
    if (this.selectedIndex < 0) return;
    
    const selectedSuggestion = this.suggestions[this.selectedIndex];
    if (!selectedSuggestion) return;
    
    const card = this.suggestionCards.get(selectedSuggestion.id);
    if (card) {
      if (this.activePreviewCard === selectedSuggestion.id) {
        card.hidePreview();
      } else {
        card.showPreview();
      }
    }
  }

  /**
   * Hide all active previews
   * @private
   */
  hideAllPreviews() {
    this.suggestionCards.forEach(card => {
      card.hidePreview();
    });
    this.activePreviewCard = null;
  }

  /**
   * Clear all suggestion cards
   * @private
   */
  clearSuggestionCards() {
    this.suggestionCards.forEach(card => {
      card.destroy();
      this.cardPerformanceStats.cardsDestroyed++;
    });
    this.suggestionCards.clear();
    this.activePreviewCard = null;
  }

  /**
   * Emit navigation success event
   * @private
   */
  emitNavigationSuccess(suggestion) {
    console.log('✅ Navigation successful for:', suggestion.title);
    
    // Could trigger additional UI updates here
    // For example, briefly highlighting the success
  }

  /**
   * Emit navigation error event
   * @private
   */
  emitNavigationError(suggestion, error = null) {
    console.error('❌ Navigation failed for:', suggestion.title, error);
    
    // Show user-friendly error message
    if (typeof window.showNotification === 'function') {
      window.showNotification(
        `Could not open "${suggestion.title}". File may have been moved or deleted.`, 
        'error'
      );
    }
  }

  /**
   * Update card performance statistics
   * @private
   */
  updateCardPerformanceStats(operation, time) {
    switch (operation) {
      case 'render':
        console.log(`📊 Card rendering performance: ${time.toFixed(2)}ms for ${this.suggestions.length} cards`);
        break;
    }
  }

  /**
   * Update average card creation time
   * @private
   */
  updateAverageCardCreationTime(newTime) {
    const currentAvg = this.cardPerformanceStats.averageCardCreationTime;
    const count = this.cardPerformanceStats.cardsCreated;
    
    this.cardPerformanceStats.averageCardCreationTime = 
      ((currentAvg * (count - 1)) + newTime) / count;
  }

  /**
   * Get enhanced performance statistics
   * @returns {Object} Performance statistics including card metrics
   */
  getEnhancedStats() {
    return {
      ...this.getStatus(),
      cardStats: { ...this.cardPerformanceStats },
      activePreviewCard: this.activePreviewCard,
      cardCount: this.suggestionCards.size,
      navigationServiceStats: this.navigationService.getStats()
    };
  }

  /**
   * Update enhanced configuration
   * @param {Object} config - Configuration updates
   */
  updateEnhancedConfig(config) {
    this.enhancedConfig = { ...this.enhancedConfig, ...config };
    this.config = { ...this.config, ...this.enhancedConfig };
    
    // Update existing cards if needed
    if (this.state === SuggestionList.STATES.LOADED) {
      this.renderSuggestions();
    }
    
    console.log('⚙️ Enhanced suggestion list configuration updated');
  }

  /**
   * Override destroy method to cleanup cards
   */
  destroy() {
    console.log('🧹 Destroying enhanced suggestion list...');
    
    // Clear all cards
    this.clearSuggestionCards();
    
    // Call base destroy
    super.destroy();
    
    console.log('✅ Enhanced suggestion list destroyed');
  }
}

export default EnhancedSuggestionList;