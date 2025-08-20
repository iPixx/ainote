/**
 * MarkdownParser - Custom lightweight markdown parsing engine for aiNote
 * 
 * Implements a state machine architecture for efficient markdown parsing
 * without external dependencies. Converts markdown text to HTML with proper
 * sanitization and performance optimization.
 * 
 * Performance targets:
 * - Parsing time: <50ms for typical documents
 * - Memory usage: <2MB for large documents
 * - Parser performance: <100ms for 10,000 lines
 * 
 * @class MarkdownParser
 */
class MarkdownParser {
  constructor() {
    this.reset();
    this.initializeRegexPatterns();
  }

  /**
   * Reset parser state for new document
   */
  reset() {
    this.state = {
      inCodeBlock: false,
      codeBlockLanguage: '',
      inBlockquote: false,
      blockquoteDepth: 0,
      listStack: [],
      linkReferences: new Map(),
      currentLine: 0,
      totalLines: 0
    };
    this.html = [];
    this.currentParagraph = [];
  }

  /**
   * Initialize optimized regex patterns for parsing
   */
  initializeRegexPatterns() {
    this.patterns = {
      // Headers (H1-H6)
      header: /^(#{1,6})\s+(.+?)(?:\s*#*)?$/,
      
      // Horizontal rules
      horizontalRule: /^(?:\* *\* *\*[* ]*|_ *_ *_[_ ]*|- *- *-[- ]*)$/,
      
      // Code blocks (fenced)
      codeBlockStart: /^```(\w*)?(.*)$/,
      codeBlockEnd: /^```\s*$/,
      
      // Lists (ordered and unordered)
      unorderedList: /^(\s*)[*+-]\s+(.+)$/,
      orderedList: /^(\s*)(\d+)\.\s+(.+)$/,
      
      // Blockquotes
      blockquote: /^(\s*>)+\s?(.*)$/,
      
      // Inline patterns
      strongDouble: /\*\*((?:(?!\*\*).)+)\*\*/g,
      strongUnderscore: /__((?:(?!__).)+)__/g,
      emphasisSingle: /\*((?:(?!\*).)+)\*/g,
      emphasisUnderscore: /_((?:(?!_).)+)_/g,
      strikethrough: /~~((?:(?!~~).)+)~~/g,
      inlineCode: /`([^`\n]+)`/g,
      
      // Links and images
      inlineLink: /\[([^\]]*)\]\(([^)]+)\)/g,
      referenceLink: /\[([^\]]*)\]\[([^\]]*)\]/g,
      referenceDef: /^[ ]{0,3}\[([^\]]+)\]:\s*(.+)$/,
      image: /!\[([^\]]*)\]\(([^)]+)\)/g,
      
      // Line breaks
      hardBreak: /\s{2,}$/,
      softBreak: /\n/
    };
  }

  /**
   * Main parsing method - converts markdown to HTML
   * @param {string} markdown - Raw markdown text
   * @returns {string} Generated HTML
   */
  parse(markdown) {
    if (!markdown || typeof markdown !== 'string') {
      return '';
    }

    const startTime = performance.now();
    this.reset();

    // Split into lines and track progress
    const lines = markdown.split('\n');
    this.state.totalLines = lines.length;

    // First pass: collect reference definitions
    this.collectReferences(lines);

    // Second pass: parse content
    this.parseLines(lines);

    // Finalize any remaining paragraph
    this.finalizeParagraph();

    const result = this.html.join('\n');
    const parseTime = performance.now() - startTime;

    // Performance monitoring
    if (parseTime > 50) {
      console.warn(`Markdown parsing took ${parseTime.toFixed(2)}ms (target: <50ms)`);
    }

    return result;
  }

  /**
   * Collect reference definitions in first pass
   * @param {string[]} lines - Array of markdown lines
   */
  collectReferences(lines) {
    lines.forEach(line => {
      const match = line.match(this.patterns.referenceDef);
      if (match) {
        const [, label, url] = match;
        this.state.linkReferences.set(label.toLowerCase(), url.trim());
      }
    });
  }

  /**
   * Parse lines of markdown content
   * @param {string[]} lines - Array of markdown lines
   */
  parseLines(lines) {
    for (let i = 0; i < lines.length; i++) {
      this.state.currentLine = i;
      const line = lines[i];
      this.parseLine(line);
    }
  }

  /**
   * Parse a single line based on current state
   * @param {string} line - Single line of markdown
   */
  parseLine(line) {
    const trimmed = line.trim();

    // Handle code blocks (fenced)
    if (this.handleCodeBlock(line, trimmed)) return;

    // Skip reference definitions (already collected)
    if (this.patterns.referenceDef.test(line)) return;

    // Handle block-level elements
    if (this.handleHeader(trimmed)) return;
    if (this.handleHorizontalRule(trimmed)) return;
    if (this.handleList(line)) return;
    if (this.handleBlockquote(line, trimmed)) return;

    // Handle empty lines and paragraphs
    if (trimmed === '') {
      this.handleEmptyLine();
    } else {
      this.handleParagraphLine(line);
    }
  }

  /**
   * Handle fenced code blocks
   * @param {string} line - Original line
   * @param {string} trimmed - Trimmed line
   * @returns {boolean} True if handled
   */
  handleCodeBlock(line, trimmed) {
    const startMatch = trimmed.match(this.patterns.codeBlockStart);
    const endMatch = trimmed.match(this.patterns.codeBlockEnd);

    if (this.state.inCodeBlock) {
      if (endMatch) {
        this.html.push('</code></pre>');
        this.state.inCodeBlock = false;
        this.state.codeBlockLanguage = '';
        return true;
      } else {
        this.html.push(this.escapeHtml(line));
        return true;
      }
    } else if (startMatch) {
      this.finalizeParagraph();
      const language = startMatch[1] || '';
      this.state.codeBlockLanguage = language;
      this.state.inCodeBlock = true;
      
      const langClass = language ? ` class="language-${this.escapeHtml(language)}"` : '';
      this.html.push(`<pre><code${langClass}>`);
      return true;
    }

    return false;
  }

  /**
   * Handle headers (H1-H6)
   * @param {string} trimmed - Trimmed line
   * @returns {boolean} True if handled
   */
  handleHeader(trimmed) {
    const match = trimmed.match(this.patterns.header);
    if (match) {
      this.finalizeParagraph();
      const level = match[1].length;
      const text = this.parseInlineElements(match[2].trim());
      this.html.push(`<h${level}>${text}</h${level}>`);
      return true;
    }
    return false;
  }

  /**
   * Handle horizontal rules
   * @param {string} trimmed - Trimmed line
   * @returns {boolean} True if handled
   */
  handleHorizontalRule(trimmed) {
    if (this.patterns.horizontalRule.test(trimmed)) {
      this.finalizeParagraph();
      this.html.push('<hr>');
      return true;
    }
    return false;
  }

  /**
   * Handle list items (ordered and unordered)
   * @param {string} line - Original line
   * @returns {boolean} True if handled
   */
  handleList(line) {
    const unorderedMatch = line.match(this.patterns.unorderedList);
    const orderedMatch = line.match(this.patterns.orderedList);
    
    if (unorderedMatch || orderedMatch) {
      this.finalizeParagraph();
      
      const indent = unorderedMatch ? unorderedMatch[1] : orderedMatch[1];
      const content = unorderedMatch ? unorderedMatch[2] : orderedMatch[3];
      const isOrdered = !!orderedMatch;
      const indentLevel = Math.floor(indent.length / 2);

      this.updateListStack(indentLevel, isOrdered);
      
      const parsedContent = this.parseInlineElements(content);
      this.html.push(`<li>${parsedContent}</li>`);
      return true;
    } else if (this.state.listStack.length > 0) {
      // Close all open lists when we encounter non-list content
      this.closeAllLists();
    }

    return false;
  }

  /**
   * Update list stack for nested lists
   * @param {number} indentLevel - Current indent level
   * @param {boolean} isOrdered - Whether this is an ordered list
   */
  updateListStack(indentLevel, isOrdered) {
    // Close lists that are deeper than current level
    while (this.state.listStack.length > indentLevel + 1) {
      const lastList = this.state.listStack.pop();
      this.html.push(`</${lastList.tag}>`);
    }

    // Open new list if needed or ensure current list type matches
    if (this.state.listStack.length === indentLevel) {
      const tag = isOrdered ? 'ol' : 'ul';
      this.state.listStack.push({ tag, level: indentLevel });
      this.html.push(`<${tag}>`);
    } else if (this.state.listStack.length === indentLevel + 1) {
      // Check if list type changed at this level
      const currentList = this.state.listStack[indentLevel];
      const expectedTag = isOrdered ? 'ol' : 'ul';
      
      if (currentList.tag !== expectedTag) {
        // Close current list and open new one
        this.html.push(`</${currentList.tag}>`);
        this.state.listStack[indentLevel] = { tag: expectedTag, level: indentLevel };
        this.html.push(`<${expectedTag}>`);
      }
    }
  }

  /**
   * Close all open lists
   */
  closeAllLists() {
    while (this.state.listStack.length > 0) {
      const list = this.state.listStack.pop();
      this.html.push(`</${list.tag}>`);
    }
  }

  /**
   * Handle blockquotes
   * @param {string} line - Original line
   * @param {string} trimmed - Trimmed line
   * @returns {boolean} True if handled
   */
  handleBlockquote(line, trimmed) {
    const match = line.match(this.patterns.blockquote);
    
    if (match) {
      this.finalizeParagraph();
      
      const quoteDepth = (line.match(/>/g) || []).length;
      const content = match[2];

      // Adjust blockquote nesting
      if (quoteDepth > this.state.blockquoteDepth) {
        for (let i = this.state.blockquoteDepth; i < quoteDepth; i++) {
          this.html.push('<blockquote>');
        }
      } else if (quoteDepth < this.state.blockquoteDepth) {
        for (let i = quoteDepth; i < this.state.blockquoteDepth; i++) {
          this.html.push('</blockquote>');
        }
      }

      this.state.blockquoteDepth = quoteDepth;
      this.state.inBlockquote = true;

      if (content.trim()) {
        const parsedContent = this.parseInlineElements(content);
        this.html.push(`<p>${parsedContent}</p>`);
      }
      return true;
    } else if (this.state.inBlockquote) {
      // Close all blockquotes when we encounter non-blockquote content
      for (let i = 0; i < this.state.blockquoteDepth; i++) {
        this.html.push('</blockquote>');
      }
      this.state.inBlockquote = false;
      this.state.blockquoteDepth = 0;
    }

    return false;
  }

  /**
   * Handle empty lines
   */
  handleEmptyLine() {
    this.finalizeParagraph();
    this.closeAllLists();
    
    if (this.state.inBlockquote) {
      for (let i = 0; i < this.state.blockquoteDepth; i++) {
        this.html.push('</blockquote>');
      }
      this.state.inBlockquote = false;
      this.state.blockquoteDepth = 0;
    }
  }

  /**
   * Handle regular paragraph lines
   * @param {string} line - Original line
   */
  handleParagraphLine(line) {
    // Check for hard line breaks (two spaces at end)
    if (this.patterns.hardBreak.test(line)) {
      const content = line.replace(this.patterns.hardBreak, '');
      this.currentParagraph.push(this.parseInlineElements(content));
      this.currentParagraph.push('<br>');
    } else {
      this.currentParagraph.push(this.parseInlineElements(line));
    }
  }

  /**
   * Finalize current paragraph and add to HTML
   */
  finalizeParagraph() {
    if (this.currentParagraph.length > 0) {
      const content = this.currentParagraph.join(' ');
      this.html.push(`<p>${content}</p>`);
      this.currentParagraph = [];
    }
  }

  /**
   * Parse inline elements (bold, italic, code, links, etc.)
   * @param {string} text - Text to parse
   * @returns {string} Parsed text with inline HTML
   */
  parseInlineElements(text) {
    if (!text) return '';

    // Parse in specific order to avoid conflicts
    let result = text;

    // Images (before links to avoid conflicts)
    result = result.replace(this.patterns.image, (match, alt, src) => {
      return `<img src="${this.escapeHtml(src)}" alt="${this.escapeHtml(alt)}">`;
    });

    // Links
    result = result.replace(this.patterns.inlineLink, (match, text, url) => {
      return `<a href="${this.escapeHtml(url)}">${this.escapeHtml(text)}</a>`;
    });

    // Reference links
    result = result.replace(this.patterns.referenceLink, (match, text, label) => {
      const url = this.state.linkReferences.get(label.toLowerCase()) || '#';
      return `<a href="${this.escapeHtml(url)}">${this.escapeHtml(text)}</a>`;
    });

    // Inline code (before other formatting to protect it)
    result = result.replace(this.patterns.inlineCode, (match, code) => {
      return `<code>${this.escapeHtml(code)}</code>`;
    });

    // Strong emphasis (double asterisks and underscores)
    result = result.replace(this.patterns.strongDouble, '<strong>$1</strong>');
    result = result.replace(this.patterns.strongUnderscore, '<strong>$1</strong>');

    // Regular emphasis (single asterisks and underscores)
    result = result.replace(this.patterns.emphasisSingle, '<em>$1</em>');
    result = result.replace(this.patterns.emphasisUnderscore, '<em>$1</em>');

    // Strikethrough
    result = result.replace(this.patterns.strikethrough, '<del>$1</del>');

    // Finally escape any remaining HTML characters in regular text
    // This needs to be done carefully to avoid double-escaping content inside tags
    result = this.escapeHtmlInText(result);

    return result;
  }

  /**
   * Escape HTML characters for security
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   */
  escapeHtml(text) {
    if (!text) return '';
    
    const htmlEscapes = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#x27;',
      '/': '&#x2F;'
    };

    return text.replace(/[&<>"'/]/g, char => htmlEscapes[char]);
  }

  /**
   * Escape HTML characters in text while preserving HTML tags we created
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   */
  escapeHtmlInText(text) {
    if (!text) return '';
    
    // Split by HTML tags we created (simple approach)
    const tagRegex = /<\/?(?:strong|em|del|code|a|img|br)(?:\s[^>]*)?>|<code[^>]*>.*?<\/code>/g;
    const parts = text.split(tagRegex);
    const tags = text.match(tagRegex) || [];
    
    let result = '';
    for (let i = 0; i < parts.length; i++) {
      // Escape HTML in text parts
      result += this.escapeHtml(parts[i]);
      // Add back the HTML tag if it exists
      if (tags[i]) {
        result += tags[i];
      }
    }
    
    return result;
  }

  /**
   * Get parsing statistics for performance monitoring
   * @returns {object} Parsing statistics
   */
  getStats() {
    return {
      totalLines: this.state.totalLines,
      currentLine: this.state.currentLine,
      linkReferences: this.state.linkReferences.size,
      htmlLines: this.html.length
    };
  }
}

export default MarkdownParser;