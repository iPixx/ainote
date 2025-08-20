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

// Test: Real-world comprehensive markdown (Test.md content)
test.test('should handle comprehensive real-world Test.md content', () => {
  const testMd = `# 🚀 Complete Markdown Syntax Demo

## Headings

# Heading 1

## Heading 2

### Heading 3

#### Heading 4

##### Heading 5

###### Heading 6

## Text Formatting

This paragraph has **bold text**, _italic text_, **_bold and italic_**, and ~~strikethrough~~.

You can also use **double underscore bold** and _single underscore italic_.

## Code Examples

Here's some \`inline code\` in a sentence.

\`\`\`javascript
// Code block with syntax highlighting
function greetUser(name) {
    const message = \\\`Hello, \\\${name}!\\\`;
    console.log(message);
    return message;
}

greetUser('World');
\`\`\`

\`\`\`python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

print([fibonacci(i) for i in range(10)])
\`\`\`

## Links and References

Visit [Google](https://google.com) or check out [GitHub](https://github.com).

You can also use reference-style links like [this one][1].

[1]: https://example.com

## Lists

### Unordered Lists

- First item
- Second item
  - Nested item
  - Another nested item
    - Deep nesting
- Third item

### Ordered Lists

1. First ordered item
2. Second ordered item
   1. Nested ordered item
   2. Another nested ordered item
3. Third ordered item

## Blockquotes

> This is a simple blockquote.
> It can span multiple lines.

> > This is a nested blockquote.
> > It shows multiple levels of quoting.

> ### Blockquotes can contain other elements
>
> Including **formatted text** and \`code\`.

## Mixed Content

This paragraph demonstrates **bold _italic_** nesting, along with \`inline code\` and [links](https://example.com).

> **Quote with formatting**: This shows how _various_ **elements** can be \`combined\` in a [blockquote](https://example.com).

## Performance Test

This is a longer paragraph that tests the performance of the syntax highlighter with more content.

**Bold text performance test**: Bold content repeated multiple times.

_Italic text performance test_: Italic content repeated multiple times.

\`\`\`
Code block performance test: Line of code that is repeated to test performance.
Code block performance test: Line of code that is repeated to test performance.
\`\`\`

That's a comprehensive demonstration of all supported markdown syntax elements! 🎉`;

  const startTime = performance.now();
  const result = parser.parse(testMd);
  const parseTime = performance.now() - startTime;
  
  // Verify core parsing worked
  test.assertEqual(result.length > 0, true, 'Should produce HTML output');
  
  // Test headers (all levels)
  test.assertContains(result, '<h1>🚀 Complete Markdown Syntax Demo</h1>');
  test.assertContains(result, '<h2>Headings</h2>');
  test.assertContains(result, '<h3>Heading 3</h3>');
  test.assertContains(result, '<h4>Heading 4</h4>');
  test.assertContains(result, '<h5>Heading 5</h5>');
  test.assertContains(result, '<h6>Heading 6</h6>');
  
  // Test text formatting
  test.assertContains(result, '<strong>bold text</strong>');
  test.assertContains(result, '<em>italic text</em>');
  test.assertContains(result, '<del>strikethrough</del>');
  test.assertContains(result, '<strong><em>bold and italic</em></strong>');
  
  // Test inline code
  test.assertContains(result, '<code>inline code</code>');
  
  // Test code blocks with languages
  test.assertContains(result, 'class="language-javascript"');
  test.assertContains(result, 'class="language-python"');
  test.assertContains(result, 'function greetUser(name)');
  test.assertContains(result, 'def fibonacci(n):');
  
  // Test links
  test.assertContains(result, '<a href="https://google.com">Google</a>');
  test.assertContains(result, '<a href="https://github.com">GitHub</a>');
  test.assertContains(result, '<a href="https://example.com">this one</a>'); // reference link
  
  // Test lists
  test.assertContains(result, '<ul>');
  test.assertContains(result, '<ol>');
  test.assertContains(result, '<li>First item</li>');
  test.assertContains(result, '<li>Nested item</li>');
  test.assertContains(result, '<li>First ordered item</li>');
  
  // Test blockquotes
  test.assertContains(result, '<blockquote>');
  test.assertContains(result, '<p>This is a simple blockquote.</p>');
  test.assertContains(result, '<p>This is a nested blockquote.</p>');
  
  // Test performance (should parse complex document quickly)
  test.assertEqual(parseTime < 100, true, `Parse time ${parseTime.toFixed(2)}ms should be < 100ms for complex document`);
  
  // Test HTML escaping (emoji should be preserved)
  test.assertContains(result, '🚀');
  test.assertContains(result, '🎉');
  
  console.log(`✅ Real-world Test.md parsed successfully in ${parseTime.toFixed(2)}ms`);
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