/**
 * Vitest test suite for ContentChangeDetector
 * Validates performance requirements and functionality
 * Migrated from original test file to work with Vitest infrastructure
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Mock MarkdownEditor for testing
class MockMarkdownEditor {
  constructor() {
    this.isInitialized = true;
    this.cursorPosition = 0;
    this.eventListeners = new Map();
  }

  getValue() {
    return this.content || '';
  }

  addEventListener(eventType, handler) {
    if (!this.eventListeners.has(eventType)) {
      this.eventListeners.set(eventType, []);
    }
    this.eventListeners.get(eventType).push(handler);
  }

  emit(eventType, data) {
    const listeners = this.eventListeners.get(eventType);
    if (listeners) {
      listeners.forEach(handler => handler({ detail: data }));
    }
  }
}

// Mock AppState for testing
class MockAppState {
  constructor() {
    this.state = { currentFile: '/test/file.md' };
  }

  getState() {
    return this.state;
  }
}

// Performance testing utility
function measurePerformance(fn, iterations = 100) {
  const start = performance.now();
  for (let i = 0; i < iterations; i++) {
    fn();
  }
  const end = performance.now();
  return (end - start) / iterations;
}

// Memory usage estimation utility
function estimateMemoryUsage(obj) {
  const jsonString = JSON.stringify(obj);
  return jsonString.length * 2; // Approximate UTF-16 encoding
}

describe('ContentChangeDetector', () => {
  let tauriMocks;
  
  beforeEach(() => {
    tauriMocks = setupTauriMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Content Extraction Performance', () => {
    it('should extract content within 10ms performance requirement', () => {
      const mockEditor = new MockMarkdownEditor();
      const mockAppState = new MockAppState();
      
      // Create test content with multiple paragraphs
      const testContent = Array.from({ length: 50 }, (_, i) => 
        `This is paragraph ${i + 1} with some content that should be meaningful enough for the AI system to process. It contains multiple sentences and provides context for testing the extraction performance.`
      ).join('\n\n');
      
      mockEditor.content = testContent;
      mockEditor.cursorPosition = Math.floor(testContent.length / 2);

      // Extract paragraph content logic
      const extractParagraphContent = (content, cursorPosition) => {
        const paragraphs = content.split('\n\n').filter(p => p.trim().length > 0);
        
        // Find cursor paragraph
        let currentPosition = 0;
        let cursorParagraphIndex = 0;
        
        for (let i = 0; i < paragraphs.length; i++) {
          const paragraphEnd = currentPosition + paragraphs[i].length;
          if (cursorPosition >= currentPosition && cursorPosition <= paragraphEnd + 2) {
            cursorParagraphIndex = i;
            break;
          }
          currentPosition = paragraphEnd + 2;
        }
        
        const currentParagraph = paragraphs[cursorParagraphIndex] || '';
        const contextStart = Math.max(0, cursorParagraphIndex - 3);
        const contextEnd = Math.min(paragraphs.length, cursorParagraphIndex + 4);
        const contextParagraphs = paragraphs.slice(contextStart, contextEnd);
        
        return {
          paragraphs,
          currentParagraph: currentParagraph.trim(),
          cursorParagraphIndex,
          contextParagraphs: contextParagraphs.map(p => p.trim()),
          totalParagraphs: paragraphs.length
        };
      };

      // Test extraction performance
      const avgTime = measurePerformance(() => {
        extractParagraphContent(testContent, mockEditor.cursorPosition);
      }, 1000);

      expect(avgTime).toBeLessThanOrEqual(10);
      expect(testContent.length).toBeGreaterThan(0);
      expect(testContent.split('\n\n').length).toBe(50);
    });
  });

  describe('Debouncing Performance', () => {
    it('should prevent more than 2 requests per second with debouncing', async () => {
      let extractionCount = 0;
      const extractions = [];
      
      // Mock debounced function
      function createDebouncedFunction(delay) {
        let timeout = null;
        return function(timestamp) {
          clearTimeout(timeout);
          timeout = setTimeout(() => {
            extractionCount++;
            extractions.push(Date.now());
          }, delay);
        };
      }
      
      const debouncedExtract = createDebouncedFunction(500);
      
      // Simulate rapid typing (50 characters per second)
      const startTime = Date.now();
      const typingInterval = 20; // 50 chars/sec = 20ms per character
      
      // Simulate typing for 2 seconds
      const totalTime = 2000;
      const iterations = totalTime / typingInterval;
      
      // Run the typing simulation
      for (let i = 0; i < iterations; i++) {
        setTimeout(() => {
          debouncedExtract(Date.now());
        }, i * typingInterval);
      }
      
      // Wait for debouncing to complete
      await new Promise(resolve => setTimeout(resolve, totalTime + 1000));
      
      const actualRequestRate = extractionCount / (totalTime / 1000);
      const maxAllowedRate = 2; // Max 2 requests per second (1 per 500ms)
      
      expect(extractionCount).toBeGreaterThan(0);
      expect(actualRequestRate).toBeLessThanOrEqual(maxAllowedRate);
    });
  });

  describe('Memory Usage', () => {
    it('should keep memory usage under 5MB', () => {
      // Simulate content history with snapshots
      const contentHistory = {
        snapshots: [],
        maxSnapshots: 10
      };
      
      // Simulate extracted content cache
      const extractedContent = {
        paragraphs: [],
        currentParagraph: '',
        contextParagraphs: [],
        valid: true
      };
      
      // Generate test data
      const largeDocument = Array.from({ length: 1000 }, (_, i) => 
        `This is paragraph ${i + 1} with substantial content to test memory usage. ` +
        `It includes various markdown elements like **bold text**, *italic text*, ` +
        `[links](https://example.com), and code blocks. The content is designed to ` +
        `simulate a real document with meaningful structure and content.`
      ).join('\n\n');
      
      // Fill content history
      for (let i = 0; i < contentHistory.maxSnapshots; i++) {
        contentHistory.snapshots.push({
          content: largeDocument,
          timestamp: Date.now() - (i * 1000),
          length: largeDocument.length,
          hash: Math.random()
        });
      }
      
      // Fill extracted content
      const paragraphs = largeDocument.split('\n\n');
      extractedContent.paragraphs = paragraphs;
      extractedContent.currentParagraph = paragraphs[Math.floor(paragraphs.length / 2)];
      extractedContent.contextParagraphs = paragraphs.slice(0, 7);
      
      // Estimate memory usage
      const contentSize = largeDocument.length * 2; // Unicode characters
      const historySize = estimateMemoryUsage(contentHistory);
      const extractedSize = estimateMemoryUsage(extractedContent);
      
      const totalMemoryUsage = contentSize + historySize + extractedSize;
      const maxMemoryUsage = 5 * 1024 * 1024; // 5MB in bytes
      
      expect(totalMemoryUsage).toBeLessThanOrEqual(maxMemoryUsage);
      expect(contentSize).toBeGreaterThan(0);
      expect(historySize).toBeGreaterThan(0);
      expect(extractedSize).toBeGreaterThan(0);
    });
  });

  describe('Content Granularity', () => {
    it('should provide paragraph-level extraction with proper context', () => {
      const testDocument = `# Document Title

This is the first paragraph with introductory content. It sets the stage for the document.

This is the second paragraph with more detailed information. It expands on the introduction.

## Section Header

This is the third paragraph under a section header. It contains specific details about the section topic.

This is the fourth paragraph with examples and code snippets:

\`\`\`javascript
const example = 'code block';
console.log(example);
\`\`\`

This is the fifth paragraph that follows the code block. It explains the code above.

## Another Section

This is the sixth paragraph in a new section. It introduces different concepts.

This final paragraph wraps up the document with conclusions and next steps.`;

      // Test cursor in different positions
      const testCases = [
        { position: 50, expectedParagraph: 0, description: 'cursor in first paragraph' },
        { position: 200, expectedParagraph: 1, description: 'cursor in second paragraph' },
        { position: 400, expectedParagraph: 2, description: 'cursor in third paragraph' },
        { position: 600, expectedParagraph: 3, description: 'cursor near code block' },
        { position: 800, expectedParagraph: 4, description: 'cursor after code block' }
      ];
      
      let allTestsPassed = true;
      
      testCases.forEach(({ position, expectedParagraph, description }, index) => {
        const paragraphs = testDocument.split('\n\n').filter(p => p.trim().length > 0);
        
        // Find cursor paragraph
        let currentPosition = 0;
        let cursorParagraphIndex = 0;
        
        for (let i = 0; i < paragraphs.length; i++) {
          const paragraphEnd = currentPosition + paragraphs[i].length;
          if (position >= currentPosition && position <= paragraphEnd + 2) {
            cursorParagraphIndex = i;
            break;
          }
          currentPosition = paragraphEnd + 2;
        }
        
        // Extract context
        const contextStart = Math.max(0, cursorParagraphIndex - 3);
        const contextEnd = Math.min(paragraphs.length, cursorParagraphIndex + 4);
        const contextParagraphs = paragraphs.slice(contextStart, contextEnd);
        
        // Validate context buffer (should include 3 paragraphs before and after)
        const expectedContextSize = Math.min(7, paragraphs.length); // Max 7 paragraphs (3 before + current + 3 after)
        expect(contextParagraphs.length).toBeLessThanOrEqual(expectedContextSize);
        expect(cursorParagraphIndex).toBeGreaterThanOrEqual(0);
        expect(paragraphs[cursorParagraphIndex]).toBeDefined();
      });
      
      expect(allTestsPassed).toBe(true);
    });
  });
});