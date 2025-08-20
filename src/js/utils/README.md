# aiNote Markdown Parser

A lightweight, custom markdown parsing engine designed specifically for aiNote's local-first architecture.

## Overview

The `MarkdownParser` class provides efficient markdown-to-HTML conversion without external dependencies, optimized for real-time preview generation and large document handling.

## Features

- **Performance Optimized**: <50ms parsing time for typical documents
- **Memory Efficient**: <2MB memory usage for large documents
- **Security First**: Comprehensive HTML escaping and sanitization
- **State Machine Architecture**: Robust parsing with proper error handling
- **Comprehensive Markdown Support**: All standard markdown elements

## Supported Markdown Elements

### Block Elements
- **Headers**: H1-H6 (`#` to `######`)
- **Paragraphs**: Automatic paragraph detection with proper spacing
- **Code Blocks**: Fenced code blocks with language detection
- **Lists**: Nested ordered and unordered lists
- **Blockquotes**: Single and nested blockquotes
- **Horizontal Rules**: `---`, `***`, `___`

### Inline Elements
- **Bold**: `**text**` and `__text__`
- **Italic**: `*text*` and `_text_`
- **Strikethrough**: `~~text~~`
- **Inline Code**: `` `code` ``
- **Links**: `[text](url)` and reference links
- **Images**: `![alt](src)`
- **Line Breaks**: Two spaces at end of line

## Usage

```javascript
import MarkdownParser from './js/utils/markdown-parser.js';

const parser = new MarkdownParser();
const html = parser.parse(markdownText);
```

## API

### `constructor()`
Creates a new parser instance with initialized state and regex patterns.

### `parse(markdown: string): string`
Main parsing method that converts markdown text to HTML.

### `reset()`
Resets parser state for parsing a new document.

### `getStats(): object`
Returns parsing statistics for performance monitoring.

## Performance Targets

- **Parse Time**: <50ms for typical documents (1-5KB)
- **Large Documents**: <100ms for 10,000 lines
- **Memory Usage**: <2MB for large documents
- **Real-time Updates**: Optimized for 60fps rendering

## Architecture

The parser uses a state machine approach with the following components:

1. **Reference Collection**: First pass to collect link references
2. **Line-by-Line Parsing**: State-aware parsing of each line
3. **Block Element Detection**: Headers, code blocks, lists, blockquotes
4. **Inline Element Processing**: Bold, italic, links, code within text
5. **HTML Generation**: Safe HTML output with proper escaping

## Security

- All user input is properly escaped to prevent XSS attacks
- HTML tags are sanitized and only allowed tags are preserved
- Special characters are converted to HTML entities
- Code blocks are fully escaped to prevent script injection

## Testing

Run the comprehensive test suite:

```bash
# Open test runner in browser
open src/js/utils/test-parser.html
```

The parser includes 25+ test cases covering:
- All markdown elements
- Edge cases and malformed input
- Performance benchmarks
- Security validation
- Large document handling

## Integration with aiNote

The parser is designed to integrate seamlessly with:
- **Issue #47**: HTML renderer for display optimization
- **Issue #48**: Advanced features (syntax highlighting, tables)
- **Issue #49**: Real-time updates and scroll synchronization

## Performance Monitoring

The parser includes built-in performance monitoring:

```javascript
const parser = new MarkdownParser();
const html = parser.parse(markdown);
const stats = parser.getStats();

console.log(`Parsed ${stats.totalLines} lines in ${parseTime}ms`);
```

## Future Enhancements

The parser is designed to be extended for:
- Table parsing (Issue #48)
- Syntax highlighting integration (Issue #48)
- Custom markdown extensions
- Plugin architecture for additional features

## Standards Compliance

The parser follows CommonMark specification for markdown parsing while optimizing for aiNote's specific use cases and performance requirements.