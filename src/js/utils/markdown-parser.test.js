/**
 * Test suite for MarkdownParser
 * 
 * Comprehensive tests for all markdown elements and parsing scenarios
 * including performance validation and edge cases.
 */

import MarkdownParser from './markdown-parser.js';

/**
 * Simple test runner for browser environment
 */
class TestRunner {
  constructor() {
    this.tests = [];
    this.passed = 0;
    this.failed = 0;
  }

  test(name, fn) {
    this.tests.push({ name, fn });
  }

  assertEqual(actual, expected, message = '') {
    if (actual !== expected) {
      throw new Error(`${message}\nExpected: ${JSON.stringify(expected)}\nActual: ${JSON.stringify(actual)}`);
    }
  }

  assertContains(actual, expected, message = '') {
    if (!actual.includes(expected)) {
      throw new Error(`${message}\nExpected "${actual}" to contain "${expected}"`);
    }
  }

  async run() {
    console.log(`Running ${this.tests.length} tests...`);
    
    for (const test of this.tests) {
      try {
        await test.fn();
        console.log(`✅ ${test.name}`);
        this.passed++;
      } catch (error) {
        console.error(`❌ ${test.name}: ${error.message}`);
        this.failed++;
      }
    }

    console.log(`\nResults: ${this.passed} passed, ${this.failed} failed`);
    return this.failed === 0;
  }
}

// Initialize test runner and parser
const test = new TestRunner();
const parser = new MarkdownParser();

// Test: Basic functionality
test.test('should create parser instance', () => {
  test.assertEqual(parser instanceof MarkdownParser, true);
});

test.test('should handle empty input', () => {
  test.assertEqual(parser.parse(''), '');
  test.assertEqual(parser.parse(null), '');
  test.assertEqual(parser.parse(undefined), '');
});

// Test: Headers
test.test('should parse H1 headers', () => {
  const result = parser.parse('# Header 1');
  test.assertEqual(result, '<h1>Header 1</h1>');
});

test.test('should parse all header levels', () => {
  const markdown = `# H1
## H2
### H3
#### H4
##### H5
###### H6`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<h1>H1</h1>');
  test.assertContains(result, '<h2>H2</h2>');
  test.assertContains(result, '<h3>H3</h3>');
  test.assertContains(result, '<h4>H4</h4>');
  test.assertContains(result, '<h5>H5</h5>');
  test.assertContains(result, '<h6>H6</h6>');
});

test.test('should handle headers with trailing hashes', () => {
  const result = parser.parse('# Header #');
  test.assertEqual(result, '<h1>Header</h1>');
});

// Test: Paragraphs
test.test('should parse simple paragraphs', () => {
  const result = parser.parse('This is a paragraph.');
  test.assertEqual(result, '<p>This is a paragraph.</p>');
});

test.test('should handle multiple paragraphs', () => {
  const markdown = `First paragraph.

Second paragraph.`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<p>First paragraph.</p>');
  test.assertContains(result, '<p>Second paragraph.</p>');
});

// Test: Bold and Italic
test.test('should parse bold text with asterisks', () => {
  const result = parser.parse('**bold text**');
  test.assertEqual(result, '<p><strong>bold text</strong></p>');
});

test.test('should parse bold text with underscores', () => {
  const result = parser.parse('__bold text__');
  test.assertEqual(result, '<p><strong>bold text</strong></p>');
});

test.test('should parse italic text with asterisks', () => {
  const result = parser.parse('*italic text*');
  test.assertEqual(result, '<p><em>italic text</em></p>');
});

test.test('should parse italic text with underscores', () => {
  const result = parser.parse('_italic text_');
  test.assertEqual(result, '<p><em>italic text</em></p>');
});

test.test('should handle combined bold and italic', () => {
  const result = parser.parse('**bold** and *italic* text');
  test.assertContains(result, '<strong>bold</strong>');
  test.assertContains(result, '<em>italic</em>');
});

// Test: Strikethrough
test.test('should parse strikethrough text', () => {
  const result = parser.parse('~~strikethrough~~');
  test.assertEqual(result, '<p><del>strikethrough</del></p>');
});

// Test: Inline Code
test.test('should parse inline code', () => {
  const result = parser.parse('`inline code`');
  test.assertEqual(result, '<p><code>inline code</code></p>');
});

test.test('should escape HTML in inline code', () => {
  const result = parser.parse('`<script>alert("test")</script>`');
  test.assertContains(result, '&lt;script&gt;');
  test.assertContains(result, '&lt;/script&gt;');
});

// Test: Code Blocks
test.test('should parse fenced code blocks', () => {
  const markdown = '```\ncode block\n```';
  const result = parser.parse(markdown);
  test.assertContains(result, '<pre><code>');
  test.assertContains(result, 'code block');
  test.assertContains(result, '</code></pre>');
});

test.test('should parse code blocks with language', () => {
  const markdown = '```javascript\nconsole.log("hello");\n```';
  const result = parser.parse(markdown);
  test.assertContains(result, 'class="language-javascript"');
  test.assertContains(result, 'console.log("hello");');
});

test.test('should escape HTML in code blocks', () => {
  const markdown = '```\n<script>alert("test")</script>\n```';
  const result = parser.parse(markdown);
  test.assertContains(result, '&lt;script&gt;');
});

// Test: Lists
test.test('should parse unordered lists', () => {
  const markdown = `* Item 1
* Item 2
* Item 3`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<ul>');
  test.assertContains(result, '<li>Item 1</li>');
  test.assertContains(result, '<li>Item 2</li>');
  test.assertContains(result, '<li>Item 3</li>');
  test.assertContains(result, '</ul>');
});

test.test('should parse ordered lists', () => {
  const markdown = `1. Item 1
2. Item 2
3. Item 3`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<ol>');
  test.assertContains(result, '<li>Item 1</li>');
  test.assertContains(result, '<li>Item 2</li>');
  test.assertContains(result, '<li>Item 3</li>');
  test.assertContains(result, '</ol>');
});

test.test('should handle nested lists', () => {
  const markdown = `* Item 1
  * Nested item
* Item 2`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<ul>');
  test.assertContains(result, '<li>Item 1</li>');
  test.assertContains(result, '<li>Nested item</li>');
  test.assertContains(result, '<li>Item 2</li>');
});

// Test: Links
test.test('should parse inline links', () => {
  const result = parser.parse('[link text](http://example.com)');
  test.assertEqual(result, '<p><a href="http://example.com">link text</a></p>');
});

test.test('should handle reference links', () => {
  const markdown = `[link text][1]

[1]: http://example.com`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<a href="http://example.com">link text</a>');
});

test.test('should escape URLs in links', () => {
  const result = parser.parse('[text](<script>alert("test")</script>)');
  test.assertContains(result, '&lt;script&gt;');
});

// Test: Images
test.test('should parse images', () => {
  const result = parser.parse('![alt text](image.jpg)');
  test.assertEqual(result, '<p><img src="image.jpg" alt="alt text"></p>');
});

test.test('should escape image attributes', () => {
  const result = parser.parse('![<script>](image.jpg)');
  test.assertContains(result, 'alt="&lt;script&gt;"');
});

// Test: Blockquotes
test.test('should parse blockquotes', () => {
  const result = parser.parse('> This is a quote');
  test.assertContains(result, '<blockquote>');
  test.assertContains(result, '<p>This is a quote</p>');
  test.assertContains(result, '</blockquote>');
});

test.test('should handle nested blockquotes', () => {
  const markdown = `> Level 1
>> Level 2
> Back to level 1`;
  const result = parser.parse(markdown);
  test.assertContains(result, '<blockquote>');
  test.assertContains(result, '<p>Level 1</p>');
  test.assertContains(result, '<p>Level 2</p>');
  test.assertContains(result, '<p>Back to level 1</p>');
});

// Test: Horizontal Rules
test.test('should parse horizontal rules with asterisks', () => {
  const result = parser.parse('***');
  test.assertEqual(result, '<hr>');
});

test.test('should parse horizontal rules with dashes', () => {
  const result = parser.parse('---');
  test.assertEqual(result, '<hr>');
});

test.test('should parse horizontal rules with underscores', () => {
  const result = parser.parse('___');
  test.assertEqual(result, '<hr>');
});

// Test: Line Breaks
test.test('should parse hard line breaks', () => {
  const result = parser.parse('Line 1  \nLine 2');
  test.assertContains(result, 'Line 1<br>');
  test.assertContains(result, 'Line 2');
});

// Test: HTML Escaping
test.test('should escape HTML characters', () => {
  const result = parser.parse('<script>alert("test")</script>');
  test.assertContains(result, '&lt;script&gt;');
  test.assertContains(result, '&lt;/script&gt;');
});

test.test('should escape HTML in text content', () => {
  const result = parser.parse('**<b>bold</b>**');
  test.assertContains(result, '<strong>&lt;b&gt;bold&lt;/b&gt;</strong>');
});

// Test: Complex Documents
test.test('should handle complex mixed content', () => {
  const markdown = `# Main Header

This is a **bold** paragraph with *italic* text and \`inline code\`.

## Subheader

* List item with [link](http://example.com)
* Another item

\`\`\`javascript
console.log("code block");
\`\`\`

> This is a quote with **bold** text.

---

Final paragraph.`;

  const result = parser.parse(markdown);
  
  test.assertContains(result, '<h1>Main Header</h1>');
  test.assertContains(result, '<strong>bold</strong>');
  test.assertContains(result, '<em>italic</em>');
  test.assertContains(result, '<code>inline code</code>');
  test.assertContains(result, '<h2>Subheader</h2>');
  test.assertContains(result, '<ul>');
  test.assertContains(result, '<a href="http://example.com">link</a>');
  test.assertContains(result, 'class="language-javascript"');
  test.assertContains(result, '<blockquote>');
  test.assertContains(result, '<hr>');
});

// Test: Performance
test.test('should meet performance targets for typical documents', () => {
  // Create a typical document (around 1000 lines)
  const lines = [];
  for (let i = 0; i < 1000; i++) {
    lines.push(`This is paragraph ${i} with **bold** text and *italic* formatting.`);
    if (i % 10 === 0) {
      lines.push(`## Header ${i}`);
    }
    if (i % 20 === 0) {
      lines.push('```javascript');
      lines.push('console.log("code block");');
      lines.push('```');
    }
  }
  
  const markdown = lines.join('\n');
  const startTime = performance.now();
  const result = parser.parse(markdown);
  const parseTime = performance.now() - startTime;
  
  test.assertEqual(result.length > 0, true, 'Should produce output');
  test.assertEqual(parseTime < 100, true, `Parse time ${parseTime}ms should be < 100ms`);
});

test.test('should handle large documents efficiently', () => {
  // Create a large document (around 5000 lines)
  const lines = [];
  for (let i = 0; i < 5000; i++) {
    lines.push(`Line ${i}: This is test content with various **formatting** and *styles*.`);
  }
  
  const markdown = lines.join('\n');
  const startTime = performance.now();
  const result = parser.parse(markdown);
  const parseTime = performance.now() - startTime;
  
  test.assertEqual(result.length > 0, true, 'Should produce output');
  test.assertEqual(parseTime < 200, true, `Parse time ${parseTime}ms should be reasonable for large docs`);
});

// Test: Edge Cases
test.test('should handle malformed markdown gracefully', () => {
  const malformed = `# Header without closing
**unclosed bold
\`unclosed code
[invalid link](
> unclosed quote`;

  const result = parser.parse(malformed);
  test.assertEqual(result.length > 0, true, 'Should produce some output even with malformed input');
});

test.test('should handle special characters', () => {
  const result = parser.parse('Text with special chars: & < > " \' /');
  test.assertContains(result, '&amp;');
  test.assertContains(result, '&lt;');
  test.assertContains(result, '&gt;');
  test.assertContains(result, '&quot;');
});

// Export test runner for browser usage
if (typeof window !== 'undefined') {
  window.runMarkdownParserTests = () => test.run();
}

export { test as markdownParserTests };