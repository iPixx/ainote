/**
 * SuggestionCard Component Unit Tests
 * 
 * Tests the enhanced suggestion card component functionality including:
 * - Card creation and rendering
 * - User interactions (click, hover, keyboard)
 * - Navigation requests
 * - Preview functionality
 * - Accessibility features
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';
import SuggestionCard from '../../src/js/components/suggestion-card.js';

// Setup Tauri mocks
setupTauriMocks();

describe('SuggestionCard Component', () => {
  let mockSuggestion;
  let suggestionCard;
  let container;

  beforeEach(() => {
    // Create a container for the card
    container = document.createElement('div');
    document.body.appendChild(container);

    // Mock suggestion data
    mockSuggestion = {
      id: 'test-suggestion-1',
      title: 'Test Note Title',
      content: 'This is test content for the suggestion card that should be rendered properly.',
      contextSnippet: 'This is a context snippet...',
      filePath: '/vault/test-note.md',
      relevanceScore: 0.85,
      metadata: {
        lastModified: '2024-01-15T10:30:00Z',
        wordCount: 150
      }
    };
  });

  afterEach(() => {
    // Clean up DOM
    if (suggestionCard) {
      suggestionCard.destroy();
    }
    document.body.removeChild(container);
    
    // Clear all mocks
    vi.clearAllMocks();
  });

  describe('Card Creation and Rendering', () => {
    it('should create a suggestion card with valid data', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      expect(suggestionCard).toBeDefined();
      expect(suggestionCard.suggestion).toEqual(mockSuggestion);
      expect(suggestionCard.index).toBe(0);
    });

    it('should throw error when suggestion data is missing', () => {
      expect(() => {
        new SuggestionCard(null, 0);
      }).toThrow('Suggestion data is required');
    });

    it('should create DOM element with proper structure', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      const element = suggestionCard.getElement();
      
      expect(element).toBeDefined();
      expect(element.classList.contains('suggestion-card')).toBe(true);
      expect(element.getAttribute('role')).toBe('button');
      expect(element.getAttribute('tabindex')).toBe('0');
      expect(element.getAttribute('data-index')).toBe('0');
      expect(element.getAttribute('data-suggestion-id')).toBe('test-suggestion-1');
    });

    it('should render title and content correctly', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      const element = suggestionCard.getElement();
      
      const title = element.querySelector('.suggestion-card-title');
      expect(title.textContent.trim()).toBe('Test Note Title');
      
      const snippet = element.querySelector('.suggestion-card-snippet p');
      expect(snippet.textContent.trim()).toBe('This is a context snippet...');
    });

    it('should render relevance score correctly', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      const element = suggestionCard.getElement();
      
      const relevanceSection = element.querySelector('.suggestion-card-relevance');
      expect(relevanceSection).toBeDefined();
      
      const percentage = element.querySelector('.relevance-percentage');
      expect(percentage.textContent).toBe('85%');
      
      const activeBars = element.querySelectorAll('.relevance-bar.active');
      expect(activeBars.length).toBeGreaterThan(0);
    });

    it('should render file path correctly', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      const element = suggestionCard.getElement();
      
      const pathElement = element.querySelector('.suggestion-card-path');
      expect(pathElement).toBeDefined();
      expect(pathElement.textContent).toContain('test-note.md');
    });

    it('should render action buttons', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      const element = suggestionCard.getElement();
      
      const navigateBtn = element.querySelector('.navigate-btn');
      const insertBtn = element.querySelector('.insert-btn');
      const referenceBtn = element.querySelector('.reference-btn');
      
      expect(navigateBtn).toBeDefined();
      expect(insertBtn).toBeDefined();
      expect(referenceBtn).toBeDefined();
    });
  });

  describe('User Interactions', () => {
    beforeEach(() => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      container.appendChild(suggestionCard.getElement());
    });

    it('should handle card click events', async () => {
      const mockClickHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.CARD_CLICKED, mockClickHandler);
      
      const element = suggestionCard.getElement();
      element.click();
      
      expect(mockClickHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestion,
          index: 0,
          action: SuggestionCard.ACTIONS.NAVIGATE
        })
      );
    });

    it('should handle action button clicks', async () => {
      const mockActionHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.ACTION_TRIGGERED, mockActionHandler);
      
      const element = suggestionCard.getElement();
      const insertBtn = element.querySelector('.insert-btn');
      insertBtn.click();
      
      expect(mockActionHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestion,
          index: 0,
          action: SuggestionCard.ACTIONS.INSERT
        })
      );
    });

    it('should handle hover events', async () => {
      const mockHoverHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.CARD_HOVERED, mockHoverHandler);
      
      const element = suggestionCard.getElement();
      
      // Mouse enter
      element.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
      expect(element.classList.contains('suggestion-card-hovered')).toBe(true);
      expect(mockHoverHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestion,
          index: 0,
          isHovered: true
        })
      );
      
      // Mouse leave
      element.dispatchEvent(new MouseEvent('mouseleave', { bubbles: true }));
      expect(element.classList.contains('suggestion-card-hovered')).toBe(false);
    });

    it('should handle focus events', async () => {
      const mockFocusHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.CARD_FOCUSED, mockFocusHandler);
      
      const element = suggestionCard.getElement();
      
      element.focus();
      expect(element.classList.contains('suggestion-card-focused')).toBe(true);
      expect(mockFocusHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestion,
          index: 0,
          isFocused: true
        })
      );
    });
  });

  describe('Keyboard Navigation', () => {
    beforeEach(() => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      container.appendChild(suggestionCard.getElement());
    });

    it('should handle Enter key for navigation', async () => {
      const mockActionHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.ACTION_TRIGGERED, mockActionHandler);
      
      const element = suggestionCard.getElement();
      element.focus();
      
      const enterEvent = new KeyboardEvent('keydown', { 
        key: 'Enter',
        bubbles: true,
        cancelable: true 
      });
      element.dispatchEvent(enterEvent);
      
      expect(mockActionHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          action: SuggestionCard.ACTIONS.NAVIGATE
        })
      );
    });

    it('should handle Space key for preview toggle', async () => {
      const mockPreviewHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.PREVIEW_REQUESTED, mockPreviewHandler);
      
      const element = suggestionCard.getElement();
      element.focus();
      
      const spaceEvent = new KeyboardEvent('keydown', { 
        key: ' ',
        bubbles: true,
        cancelable: true 
      });
      element.dispatchEvent(spaceEvent);
      
      expect(mockPreviewHandler).toHaveBeenCalled();
    });

    it('should handle Ctrl+I for insert action', async () => {
      const mockActionHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.ACTION_TRIGGERED, mockActionHandler);
      
      const element = suggestionCard.getElement();
      element.focus();
      
      const ctrlIEvent = new KeyboardEvent('keydown', { 
        key: 'I',
        ctrlKey: true,
        bubbles: true,
        cancelable: true 
      });
      element.dispatchEvent(ctrlIEvent);
      
      expect(mockActionHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          action: SuggestionCard.ACTIONS.INSERT
        })
      );
    });

    it('should handle Escape key to hide preview', () => {
      const element = suggestionCard.getElement();
      element.focus();
      
      // Show preview first
      suggestionCard.showPreview();
      expect(suggestionCard.previewVisible).toBe(true);
      
      const escapeEvent = new KeyboardEvent('keydown', { 
        key: 'Escape',
        bubbles: true,
        cancelable: true 
      });
      element.dispatchEvent(escapeEvent);
      
      expect(suggestionCard.previewVisible).toBe(false);
    });
  });

  describe('Preview Functionality', () => {
    beforeEach(() => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0, {
        enableHoverPreview: true
      });
      container.appendChild(suggestionCard.getElement());
    });

    it('should show preview when requested', () => {
      const mockPreviewHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.PREVIEW_REQUESTED, mockPreviewHandler);
      
      suggestionCard.showPreview();
      
      expect(suggestionCard.previewVisible).toBe(true);
      expect(mockPreviewHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestion,
          visible: true
        })
      );
      
      const previewElement = suggestionCard.getElement().querySelector('.suggestion-card-preview');
      expect(previewElement.style.display).toBe('block');
      expect(previewElement.getAttribute('aria-hidden')).toBe('false');
    });

    it('should hide preview when requested', () => {
      suggestionCard.showPreview();
      expect(suggestionCard.previewVisible).toBe(true);
      
      const mockPreviewHandler = vi.fn();
      suggestionCard.addEventListener(SuggestionCard.EVENTS.PREVIEW_REQUESTED, mockPreviewHandler);
      
      suggestionCard.hidePreview();
      
      expect(suggestionCard.previewVisible).toBe(false);
      expect(mockPreviewHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestion: mockSuggestion,
          visible: false
        })
      );
    });

    it('should load preview content correctly', () => {
      suggestionCard.showPreview();
      
      const previewText = suggestionCard.getElement().querySelector('.preview-text');
      expect(previewText.textContent).toContain(mockSuggestion.content);
    });
  });

  describe('Selection State', () => {
    beforeEach(() => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      container.appendChild(suggestionCard.getElement());
    });

    it('should update selection state correctly', () => {
      const element = suggestionCard.getElement();
      
      expect(suggestionCard.isSelected).toBe(false);
      expect(element.classList.contains('suggestion-card-selected')).toBe(false);
      expect(element.getAttribute('aria-selected')).toBe('false');
      
      suggestionCard.setSelected(true);
      
      expect(suggestionCard.isSelected).toBe(true);
      expect(element.classList.contains('suggestion-card-selected')).toBe(true);
      expect(element.getAttribute('aria-selected')).toBe('true');
    });
  });

  describe('Configuration Options', () => {
    it('should respect showRelevanceScore option', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0, {
        showRelevanceScore: false
      });
      
      const element = suggestionCard.getElement();
      const relevanceSection = element.querySelector('.suggestion-card-relevance');
      expect(relevanceSection).toBeNull();
    });

    it('should respect showContextSnippet option', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0, {
        showContextSnippet: false
      });
      
      const element = suggestionCard.getElement();
      const snippetSection = element.querySelector('.suggestion-card-snippet');
      expect(snippetSection).toBeNull();
    });

    it('should respect enableHoverPreview option', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0, {
        enableHoverPreview: false
      });
      
      const element = suggestionCard.getElement();
      const previewElement = element.querySelector('.suggestion-card-preview');
      expect(previewElement).toBeNull();
    });
  });

  describe('Accessibility', () => {
    beforeEach(() => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      container.appendChild(suggestionCard.getElement());
    });

    it('should have proper ARIA labels', () => {
      const element = suggestionCard.getElement();
      
      expect(element.getAttribute('role')).toBe('button');
      expect(element.getAttribute('aria-label')).toContain('Test Note Title');
      expect(element.getAttribute('aria-label')).toContain('relevance 85%');
      expect(element.getAttribute('tabindex')).toBe('0');
    });

    it('should update aria-selected when selection changes', () => {
      const element = suggestionCard.getElement();
      
      expect(element.getAttribute('aria-selected')).toBe('false');
      
      suggestionCard.setSelected(true);
      expect(element.getAttribute('aria-selected')).toBe('true');
    });

    it('should have proper action button labels', () => {
      const element = suggestionCard.getElement();
      
      const navigateBtn = element.querySelector('.navigate-btn');
      const insertBtn = element.querySelector('.insert-btn');
      const referenceBtn = element.querySelector('.reference-btn');
      
      expect(navigateBtn.getAttribute('aria-label')).toBe('Navigate to note');
      expect(insertBtn.getAttribute('aria-label')).toBe('Insert suggestion content');
      expect(referenceBtn.getAttribute('aria-label')).toBe('Add as reference link');
    });

    it('should have proper preview accessibility attributes', () => {
      const previewElement = suggestionCard.getElement().querySelector('.suggestion-card-preview');
      
      expect(previewElement.getAttribute('role')).toBe('tooltip');
      expect(previewElement.getAttribute('aria-hidden')).toBe('true');
      
      suggestionCard.showPreview();
      expect(previewElement.getAttribute('aria-hidden')).toBe('false');
    });
  });

  describe('Performance', () => {
    it('should create cards within performance targets', () => {
      const startTime = performance.now();
      
      for (let i = 0; i < 50; i++) {
        const card = new SuggestionCard({
          ...mockSuggestion,
          id: `test-suggestion-${i}`,
          title: `Test Note ${i}`
        }, i);
        card.destroy();
      }
      
      const endTime = performance.now();
      const totalTime = endTime - startTime;
      
      // Should create 50 cards in less than 100ms
      expect(totalTime).toBeLessThan(100);
    });

    it('should handle multiple rapid interactions without issues', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      container.appendChild(suggestionCard.getElement());
      
      const element = suggestionCard.getElement();
      
      // Rapid mouse events
      for (let i = 0; i < 10; i++) {
        element.dispatchEvent(new MouseEvent('mouseenter'));
        element.dispatchEvent(new MouseEvent('mouseleave'));
      }
      
      // Rapid focus events
      for (let i = 0; i < 10; i++) {
        element.focus();
        element.blur();
      }
      
      // Should not throw errors or cause memory leaks
      expect(suggestionCard.getElement()).toBeDefined();
    });
  });

  describe('Error Handling', () => {
    it('should handle malformed suggestion data gracefully', () => {
      const malformedSuggestion = {
        id: 'malformed',
        // Missing title
        content: null,
        contextSnippet: undefined,
        filePath: '',
        relevanceScore: 'invalid', // Should be number
        metadata: null
      };
      
      expect(() => {
        suggestionCard = new SuggestionCard(malformedSuggestion, 0);
      }).not.toThrow();
      
      const element = suggestionCard.getElement();
      expect(element).toBeDefined();
    });

    it('should handle missing DOM elements gracefully', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      
      // Simulate missing preview element
      const element = suggestionCard.getElement();
      const previewElement = element.querySelector('.suggestion-card-preview');
      if (previewElement) {
        previewElement.remove();
      }
      
      // Should not throw when trying to show preview
      expect(() => {
        suggestionCard.showPreview();
      }).not.toThrow();
    });
  });

  describe('Memory Management', () => {
    it('should clean up properly when destroyed', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      const element = suggestionCard.getElement();
      
      expect(element.__suggestionCard).toBe(suggestionCard);
      
      suggestionCard.destroy();
      
      expect(suggestionCard.element).toBeNull();
      expect(suggestionCard.suggestion).toBeNull();
      expect(element.__suggestionCard).toBeUndefined();
    });

    it('should clear timeouts when destroyed', () => {
      suggestionCard = new SuggestionCard(mockSuggestion, 0);
      
      // Start hover timeout
      suggestionCard.getElement().dispatchEvent(new MouseEvent('mouseenter'));
      expect(suggestionCard.hoverTimeout).toBeDefined();
      
      suggestionCard.destroy();
      expect(suggestionCard.hoverTimeout).toBeNull();
    });
  });
});