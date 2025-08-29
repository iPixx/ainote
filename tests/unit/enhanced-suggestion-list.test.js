/**
 * Enhanced Suggestion List Unit Tests
 * 
 * Tests the enhanced suggestion list functionality including:
 * - Integration with SuggestionCard components
 * - Navigation service integration
 * - Enhanced keyboard navigation
 * - Performance optimization
 * - Card management and lifecycle
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';
import EnhancedSuggestionList from '../../src/js/components/enhanced-suggestion-list.js';

// Setup Tauri mocks
setupTauriMocks();

describe('EnhancedSuggestionList Component', () => {
  let enhancedSuggestionList;
  let container;
  let mockEditor;
  let mockAppState;
  let mockNavigationService;
  let mockSuggestions;

  beforeEach(() => {
    // Create container
    container = document.createElement('div');
    container.className = 'suggestions-container';
    document.body.appendChild(container);

    // Mock editor
    mockEditor = {
      getValue: vi.fn().mockReturnValue('Test editor content'),
      insertText: vi.fn(),
      cursorPosition: 0
    };

    // Mock app state
    mockAppState = {
      get: vi.fn(),
      set: vi.fn()
    };

    // Mock navigation service
    mockNavigationService = {
      navigateToSuggestion: vi.fn().mockResolvedValue(true),
      getStats: vi.fn().mockReturnValue({
        queueLength: 0,
        isNavigating: false
      })
    };

    // Mock suggestions data
    mockSuggestions = [
      {
        id: 'suggestion-1',
        title: 'First Test Note',
        content: 'First note content for testing',
        contextSnippet: 'First context snippet...',
        filePath: '/vault/first-note.md',
        relevanceScore: 0.9,
        metadata: { lastModified: '2024-01-15T10:30:00Z' }
      },
      {
        id: 'suggestion-2',
        title: 'Second Test Note',
        content: 'Second note content for testing',
        contextSnippet: 'Second context snippet...',
        filePath: '/vault/second-note.md',
        relevanceScore: 0.8,
        metadata: { lastModified: '2024-01-14T15:20:00Z' }
      },
      {
        id: 'suggestion-3',
        title: 'Third Test Note',
        content: 'Third note content for testing',
        contextSnippet: 'Third context snippet...',
        filePath: '/vault/third-note.md',
        relevanceScore: 0.7,
        metadata: { lastModified: '2024-01-13T09:15:00Z' }
      }
    ];

    // Create AI panel mock structure
    const aiPanel = document.createElement('div');
    aiPanel.className = 'ai-panel ai-panel-visible';
    aiPanel.appendChild(container);
    document.body.appendChild(aiPanel);

    enhancedSuggestionList = new EnhancedSuggestionList(
      container,
      mockEditor,
      mockAppState,
      mockNavigationService,
      {
        useCardComponents: true,
        enableNavigationIntegration: true,
        enableHoverPreviews: true
      }
    );
  });

  afterEach(() => {
    if (enhancedSuggestionList) {
      enhancedSuggestionList.destroy();
    }
    document.body.innerHTML = '';
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should create enhanced suggestion list with navigation service', () => {
      expect(enhancedSuggestionList).toBeDefined();
      expect(enhancedSuggestionList.navigationService).toBe(mockNavigationService);
      expect(enhancedSuggestionList.enhancedConfig.useCardComponents).toBe(true);
      expect(enhancedSuggestionList.enhancedConfig.enableNavigationIntegration).toBe(true);
    });

    it('should require navigation service', () => {
      expect(() => {
        new EnhancedSuggestionList(container, mockEditor, mockAppState, null);
      }).toThrow('NavigationService instance is required');
    });

    it('should initialize card storage and stats', () => {
      expect(enhancedSuggestionList.suggestionCards).toBeDefined();
      expect(enhancedSuggestionList.cardPerformanceStats).toEqual({
        cardsCreated: 0,
        cardsDestroyed: 0,
        previewsShown: 0,
        navigationsTriggered: 0,
        averageCardCreationTime: 0
      });
    });
  });

  describe('Card-based Rendering', () => {
    it('should render suggestions using card components', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      
      expect(enhancedSuggestionList.suggestionCards.size).toBe(3);
      
      const cardElements = container.querySelectorAll('.suggestion-card');
      expect(cardElements.length).toBe(3);
      
      expect(enhancedSuggestionList.cardPerformanceStats.cardsCreated).toBe(3);
    });

    it('should create cards with proper configuration', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      expect(firstCard).toBeDefined();
      expect(firstCard.suggestion).toEqual(mockSuggestions[0]);
      expect(firstCard.index).toBe(0);
    });

    it('should clear cards when updating suggestions', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      expect(enhancedSuggestionList.suggestionCards.size).toBe(3);
      
      enhancedSuggestionList.updateSuggestions([mockSuggestions[0]]);
      expect(enhancedSuggestionList.suggestionCards.size).toBe(1);
      expect(enhancedSuggestionList.cardPerformanceStats.cardsDestroyed).toBe(3);
    });

    it('should fallback to base rendering when cards disabled', () => {
      const basicList = new EnhancedSuggestionList(
        container,
        mockEditor,
        mockAppState,
        mockNavigationService,
        { useCardComponents: false }
      );
      
      basicList.updateSuggestions(mockSuggestions);
      
      // Should use base rendering (no cards)
      expect(basicList.suggestionCards.size).toBe(0);
      const suggestionItems = container.querySelectorAll('.suggestion-item');
      expect(suggestionItems.length).toBe(3);
      
      basicList.destroy();
    });
  });

  describe('Navigation Integration', () => {
    beforeEach(() => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
    });

    it('should handle navigation requests from cards', async () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      // Simulate navigation request from card
      firstCard.emit(firstCard.constructor.EVENTS.NAVIGATE_REQUESTED, {
        suggestion: mockSuggestions[0]
      });
      
      // Wait for async navigation handling
      await new Promise(resolve => setTimeout(resolve, 0));
      
      expect(mockNavigationService.navigateToSuggestion).toHaveBeenCalledWith(mockSuggestions[0]);
      expect(enhancedSuggestionList.cardPerformanceStats.navigationsTriggered).toBe(1);
    });

    it('should emit navigation events', async () => {
      const navigationHandler = vi.fn();
      enhancedSuggestionList.addEventListener(
        EnhancedSuggestionList.EVENTS.CARD_NAVIGATION_REQUESTED, 
        navigationHandler
      );
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      firstCard.emit(firstCard.constructor.EVENTS.NAVIGATE_REQUESTED, {
        suggestion: mockSuggestions[0]
      });
      
      await new Promise(resolve => setTimeout(resolve, 0));
      
      expect(navigationHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestions[0],
          success: true
        })
      );
    });

    it('should handle navigation failures gracefully', async () => {
      mockNavigationService.navigateToSuggestion.mockResolvedValue(false);
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      firstCard.emit(firstCard.constructor.EVENTS.NAVIGATE_REQUESTED, {
        suggestion: mockSuggestions[0]
      });
      
      await new Promise(resolve => setTimeout(resolve, 0));
      
      expect(mockNavigationService.navigateToSuggestion).toHaveBeenCalled();
    });

    it('should respect navigation integration setting', async () => {
      const disabledList = new EnhancedSuggestionList(
        container,
        mockEditor,
        mockAppState,
        mockNavigationService,
        { enableNavigationIntegration: false }
      );
      
      disabledList.updateSuggestions(mockSuggestions);
      const firstCard = disabledList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.NAVIGATE_REQUESTED, {
        suggestion: mockSuggestions[0]
      });
      
      await new Promise(resolve => setTimeout(resolve, 0));
      
      expect(mockNavigationService.navigateToSuggestion).not.toHaveBeenCalled();
      disabledList.destroy();
    });
  });

  describe('Card Actions', () => {
    beforeEach(() => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
    });

    it('should handle insert action from cards', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.ACTION_TRIGGERED, {
        suggestion: mockSuggestions[0],
        action: 'insert'
      });
      
      expect(mockEditor.insertText).toHaveBeenCalledWith(mockSuggestions[0].content, false);
    });

    it('should handle reference action from cards', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.ACTION_TRIGGERED, {
        suggestion: mockSuggestions[0],
        action: 'reference'
      });
      
      expect(mockEditor.insertText).toHaveBeenCalledWith('[First Test Note]', false);
    });

    it('should emit action events', () => {
      const actionHandler = vi.fn();
      enhancedSuggestionList.addEventListener(
        EnhancedSuggestionList.EVENTS.CARD_ACTION_PERFORMED,
        actionHandler
      );
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      firstCard.emit(firstCard.constructor.EVENTS.ACTION_TRIGGERED, {
        suggestion: mockSuggestions[0],
        action: 'insert'
      });
      
      expect(actionHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestions[0],
          action: 'insert'
        })
      );
    });
  });

  describe('Preview Management', () => {
    beforeEach(() => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
    });

    it('should track active previews', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.PREVIEW_REQUESTED, {
        suggestion: mockSuggestions[0],
        visible: true
      });
      
      expect(enhancedSuggestionList.activePreviewCard).toBe('suggestion-1');
      expect(enhancedSuggestionList.cardPerformanceStats.previewsShown).toBe(1);
    });

    it('should handle preview hiding', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.PREVIEW_REQUESTED, {
        suggestion: mockSuggestions[0],
        visible: true
      });
      
      firstCard.emit(firstCard.constructor.EVENTS.PREVIEW_REQUESTED, {
        suggestion: mockSuggestions[0],
        visible: false
      });
      
      expect(enhancedSuggestionList.activePreviewCard).toBeNull();
    });

    it('should hide all previews when requested', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      
      const cards = Array.from(enhancedSuggestionList.suggestionCards.values());
      const hidePreviewSpy = vi.spyOn(cards[0], 'hidePreview');
      
      enhancedSuggestionList.hideAllPreviews();
      
      expect(hidePreviewSpy).toHaveBeenCalled();
    });
  });

  describe('Enhanced Keyboard Navigation', () => {
    beforeEach(() => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      enhancedSuggestionList.enableKeyboardNavigation();
    });

    it('should focus selected card on arrow navigation', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      const focusSpy = vi.spyOn(firstCard.getElement(), 'focus');
      
      const arrowEvent = new KeyboardEvent('keydown', {
        key: 'ArrowDown',
        bubbles: true,
        cancelable: true
      });
      
      enhancedSuggestionList.handleKeyboardNavigation(arrowEvent);
      
      expect(enhancedSuggestionList.selectedIndex).toBe(0);
      expect(focusSpy).toHaveBeenCalled();
    });

    it('should trigger navigation on Enter key', () => {
      enhancedSuggestionList.selectSuggestion(0);
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      const triggerActionSpy = vi.spyOn(firstCard, 'triggerAction');
      
      const enterEvent = new KeyboardEvent('keydown', {
        key: 'Enter',
        bubbles: true,
        cancelable: true
      });
      
      enhancedSuggestionList.handleKeyboardNavigation(enterEvent);
      
      expect(triggerActionSpy).toHaveBeenCalledWith('navigate');
    });

    it('should toggle preview on Space key', () => {
      enhancedSuggestionList.selectSuggestion(0);
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      const showPreviewSpy = vi.spyOn(firstCard, 'showPreview');
      
      const spaceEvent = new KeyboardEvent('keydown', {
        key: ' ',
        bubbles: true,
        cancelable: true
      });
      
      enhancedSuggestionList.handleKeyboardNavigation(spaceEvent);
      
      expect(showPreviewSpy).toHaveBeenCalled();
    });

    it('should trigger insert on Ctrl+I', () => {
      enhancedSuggestionList.selectSuggestion(0);
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      const triggerActionSpy = vi.spyOn(firstCard, 'triggerAction');
      
      const ctrlIEvent = new KeyboardEvent('keydown', {
        key: 'I',
        ctrlKey: true,
        bubbles: true,
        cancelable: true
      });
      
      enhancedSuggestionList.handleKeyboardNavigation(ctrlIEvent);
      
      expect(triggerActionSpy).toHaveBeenCalledWith('insert');
    });

    it('should hide previews on Escape', () => {
      const hideAllPreviewsSpy = vi.spyOn(enhancedSuggestionList, 'hideAllPreviews');
      
      const escapeEvent = new KeyboardEvent('keydown', {
        key: 'Escape',
        bubbles: true,
        cancelable: true
      });
      
      enhancedSuggestionList.handleKeyboardNavigation(escapeEvent);
      
      expect(hideAllPreviewsSpy).toHaveBeenCalled();
    });
  });

  describe('Selection Management', () => {
    beforeEach(() => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
    });

    it('should update card selection states', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      const secondCard = enhancedSuggestionList.suggestionCards.get('suggestion-2');
      
      const setSelectedSpy1 = vi.spyOn(firstCard, 'setSelected');
      const setSelectedSpy2 = vi.spyOn(secondCard, 'setSelected');
      
      enhancedSuggestionList.selectSuggestion(0);
      
      expect(setSelectedSpy1).toHaveBeenCalledWith(true);
      expect(setSelectedSpy2).toHaveBeenCalledWith(false);
    });

    it('should handle card focus events', () => {
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.CARD_FOCUSED, {
        index: 0,
        isFocused: true
      });
      
      expect(enhancedSuggestionList.selectedIndex).toBe(0);
    });

    it('should handle card click events', () => {
      const selectionHandler = vi.fn();
      enhancedSuggestionList.addEventListener('suggestion_selected', selectionHandler);
      
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      firstCard.emit(firstCard.constructor.EVENTS.CARD_CLICKED, {
        suggestion: mockSuggestions[0],
        index: 0
      });
      
      expect(selectionHandler).toHaveBeenCalled();
    });
  });

  describe('Performance Optimization', () => {
    it('should track card creation performance', () => {
      const startCardsCreated = enhancedSuggestionList.cardPerformanceStats.cardsCreated;
      
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      
      expect(enhancedSuggestionList.cardPerformanceStats.cardsCreated).toBe(startCardsCreated + 3);
      expect(enhancedSuggestionList.cardPerformanceStats.averageCardCreationTime).toBeGreaterThan(0);
    });

    it('should handle large suggestion lists efficiently', () => {
      const largeSuggestionList = Array.from({ length: 50 }, (_, i) => ({
        ...mockSuggestions[0],
        id: `suggestion-${i}`,
        title: `Test Note ${i}`
      }));
      
      const startTime = performance.now();
      enhancedSuggestionList.updateSuggestions(largeSuggestionList);
      const endTime = performance.now();
      
      expect(endTime - startTime).toBeLessThan(200); // Should render 50 cards in under 200ms
      expect(enhancedSuggestionList.suggestionCards.size).toBe(50);
    });

    it('should provide enhanced performance statistics', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      
      const stats = enhancedSuggestionList.getEnhancedStats();
      
      expect(stats).toHaveProperty('cardStats');
      expect(stats.cardStats).toHaveProperty('cardsCreated');
      expect(stats.cardStats).toHaveProperty('averageCardCreationTime');
      expect(stats).toHaveProperty('navigationServiceStats');
      expect(stats).toHaveProperty('cardCount');
    });
  });

  describe('Configuration Updates', () => {
    beforeEach(() => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
    });

    it('should update enhanced configuration', () => {
      const newConfig = {
        enableHoverPreviews: false,
        previewDelay: 1000
      };
      
      enhancedSuggestionList.updateEnhancedConfig(newConfig);
      
      expect(enhancedSuggestionList.enhancedConfig.enableHoverPreviews).toBe(false);
      expect(enhancedSuggestionList.enhancedConfig.previewDelay).toBe(1000);
    });

    it('should re-render cards when configuration changes', () => {
      const renderSpy = vi.spyOn(enhancedSuggestionList, 'renderSuggestions');
      
      enhancedSuggestionList.updateEnhancedConfig({
        showRelevanceScore: false
      });
      
      expect(renderSpy).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should handle card creation errors gracefully', () => {
      const invalidSuggestions = [
        { id: null, title: null }, // Invalid suggestion
        mockSuggestions[0] // Valid suggestion
      ];
      
      expect(() => {
        enhancedSuggestionList.updateSuggestions(invalidSuggestions);
      }).not.toThrow();
      
      // Should still create valid cards
      expect(enhancedSuggestionList.suggestionCards.size).toBeGreaterThanOrEqual(1);
    });

    it('should handle navigation service errors', async () => {
      mockNavigationService.navigateToSuggestion.mockRejectedValue(new Error('Navigation failed'));
      
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      const firstCard = enhancedSuggestionList.suggestionCards.get('suggestion-1');
      
      firstCard.emit(firstCard.constructor.EVENTS.NAVIGATE_REQUESTED, {
        suggestion: mockSuggestions[0]
      });
      
      // Should not throw
      await new Promise(resolve => setTimeout(resolve, 0));
      
      expect(mockNavigationService.navigateToSuggestion).toHaveBeenCalled();
    });

    it('should handle missing cards gracefully', () => {
      enhancedSuggestionList.selectSuggestion(5); // Index out of range
      
      expect(() => {
        enhancedSuggestionList.focusSelectedCard();
        enhancedSuggestionList.triggerSelectedCardAction('navigate');
      }).not.toThrow();
    });
  });

  describe('Memory Management', () => {
    it('should clean up cards properly when destroyed', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      
      const cards = Array.from(enhancedSuggestionList.suggestionCards.values());
      const destroySpy = vi.spyOn(cards[0], 'destroy');
      
      enhancedSuggestionList.destroy();
      
      expect(destroySpy).toHaveBeenCalled();
      expect(enhancedSuggestionList.suggestionCards.size).toBe(0);
      expect(enhancedSuggestionList.cardPerformanceStats.cardsDestroyed).toBe(3);
    });

    it('should clear cards when updating with empty suggestions', () => {
      enhancedSuggestionList.updateSuggestions(mockSuggestions);
      expect(enhancedSuggestionList.suggestionCards.size).toBe(3);
      
      enhancedSuggestionList.updateSuggestions([]);
      expect(enhancedSuggestionList.suggestionCards.size).toBe(0);
    });
  });
});