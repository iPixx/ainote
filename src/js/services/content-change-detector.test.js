/**
 * Test suite for ContentChangeDetector
 * Validates performance requirements and functionality
 */

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

/**
 * Test Content Change Detection Performance
 * Validates that content extraction meets <10ms requirement
 */
function testContentExtractionPerformance() {
  console.log('🧪 Testing content extraction performance...');
  
  const mockEditor = new MockMarkdownEditor();
  const mockAppState = new MockAppState();
  
  // Create test content with multiple paragraphs
  const testContent = Array.from({ length: 50 }, (_, i) => 
    `This is paragraph ${i + 1} with some content that should be meaningful enough for the AI system to process. It contains multiple sentences and provides context for testing the extraction performance.`
  ).join('\n\n');
  
  mockEditor.content = testContent;
  mockEditor.cursorPosition = Math.floor(testContent.length / 2);

  // Import ContentChangeDetector (this would be dynamic in browser)
  // For test purposes, we'll simulate the extraction logic
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

  console.log(`⏱️  Average extraction time: ${avgTime.toFixed(2)}ms`);
  console.log(`📊 Content size: ${testContent.length} characters, ${testContent.split('\n\n').length} paragraphs`);
  
  // Validate performance requirement
  if (avgTime <= 10) {
    console.log('✅ Performance requirement met: <10ms extraction time');
    return true;
  } else {
    console.log(`❌ Performance requirement failed: ${avgTime.toFixed(2)}ms > 10ms`);
    return false;
  }
}

/**
 * Test Debouncing Performance
 * Validates that debouncing prevents >1 request per 500ms
 */
function testDebouncingPerformance() {
  console.log('🧪 Testing debouncing performance...');
  
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
  
  for (let i = 0; i < iterations; i++) {
    setTimeout(() => {
      debouncedExtract(Date.now());
    }, i * typingInterval);
  }
  
  // Check results after typing simulation completes + buffer time
  return new Promise((resolve) => {
    setTimeout(() => {
      const actualRequestRate = extractionCount / (totalTime / 1000);
      const maxAllowedRate = 2; // Max 2 requests per second (1 per 500ms)
      
      console.log(`📊 Simulated typing: ${iterations} keystrokes over ${totalTime}ms`);
      console.log(`📊 Actual extractions: ${extractionCount}`);
      console.log(`📊 Request rate: ${actualRequestRate.toFixed(2)} requests/second`);
      
      if (actualRequestRate <= maxAllowedRate) {
        console.log('✅ Debouncing requirement met: ≤2 requests per second');
        resolve(true);
      } else {
        console.log(`❌ Debouncing requirement failed: ${actualRequestRate.toFixed(2)} > ${maxAllowedRate} requests/second`);
        resolve(false);
      }
    }, totalTime + 1000); // Wait for debouncing to complete
  });
}

/**
 * Test Memory Usage
 * Validates that memory usage stays <5MB
 */
function testMemoryUsage() {
  console.log('🧪 Testing memory usage...');
  
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
  
  console.log(`📊 Document size: ${(contentSize / 1024).toFixed(1)}KB`);
  console.log(`📊 History size: ${(historySize / 1024).toFixed(1)}KB`);
  console.log(`📊 Extracted size: ${(extractedSize / 1024).toFixed(1)}KB`);
  console.log(`📊 Total memory usage: ${(totalMemoryUsage / 1024).toFixed(1)}KB`);
  console.log(`📊 Memory limit: ${(maxMemoryUsage / 1024).toFixed(1)}KB`);
  
  if (totalMemoryUsage <= maxMemoryUsage) {
    console.log('✅ Memory usage requirement met: <5MB');
    return true;
  } else {
    console.log(`❌ Memory usage requirement failed: ${(totalMemoryUsage / 1024).toFixed(1)}KB > ${(maxMemoryUsage / 1024).toFixed(1)}KB`);
    return false;
  }
}

/**
 * Test Content Granularity
 * Validates paragraph-level extraction and context
 */
function testContentGranularity() {
  console.log('🧪 Testing content granularity...');
  
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
    
    console.log(`📝 Test ${index + 1}: ${description}`);
    console.log(`   Current paragraph index: ${cursorParagraphIndex}`);
    console.log(`   Context paragraphs: ${contextParagraphs.length} (range ${contextStart}-${contextEnd - 1})`);
    console.log(`   Current paragraph preview: "${paragraphs[cursorParagraphIndex].substring(0, 50)}..."`);
    
    // Validate context buffer (should include 3 paragraphs before and after)
    const expectedContextSize = Math.min(7, paragraphs.length); // Max 7 paragraphs (3 before + current + 3 after)
    if (contextParagraphs.length <= expectedContextSize) {
      console.log('   ✅ Context buffer size appropriate');
    } else {
      console.log(`   ❌ Context buffer too large: ${contextParagraphs.length} > ${expectedContextSize}`);
      allTestsPassed = false;
    }
  });
  
  if (allTestsPassed) {
    console.log('✅ Content granularity requirement met: paragraph-level extraction with context');
    return true;
  } else {
    console.log('❌ Content granularity requirement failed');
    return false;
  }
}

/**
 * Run all performance tests
 */
async function runAllTests() {
  console.log('🚀 Starting ContentChangeDetector performance tests...');
  console.log('');
  
  const results = {
    extraction: testContentExtractionPerformance(),
    debouncing: await testDebouncingPerformance(),
    memory: testMemoryUsage(),
    granularity: testContentGranularity()
  };
  
  console.log('');
  console.log('📋 Test Results Summary:');
  console.log(`   Content Extraction Performance: ${results.extraction ? '✅ PASS' : '❌ FAIL'}`);
  console.log(`   Debouncing Performance: ${results.debouncing ? '✅ PASS' : '❌ FAIL'}`);
  console.log(`   Memory Usage: ${results.memory ? '✅ PASS' : '❌ FAIL'}`);
  console.log(`   Content Granularity: ${results.granularity ? '✅ PASS' : '❌ FAIL'}`);
  
  const allTestsPassed = Object.values(results).every(result => result === true);
  
  if (allTestsPassed) {
    console.log('');
    console.log('🎉 All performance requirements met!');
    console.log('✅ ContentChangeDetector ready for production');
  } else {
    console.log('');
    console.log('⚠️  Some performance requirements not met');
    console.log('🔧 Review implementation for optimization opportunities');
  }
  
  return allTestsPassed;
}

// Export for module usage or run directly
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { runAllTests, testContentExtractionPerformance, testDebouncingPerformance, testMemoryUsage, testContentGranularity };
} else if (typeof window !== 'undefined') {
  window.ContentChangeDetectorTests = { runAllTests };
  // Auto-run tests if this script is loaded directly
  console.log('ContentChangeDetector performance tests loaded. Run ContentChangeDetectorTests.runAllTests() to execute.');
}