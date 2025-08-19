/**
 * SyntaxHighlighter - Custom markdown syntax highlighting engine for aiNote
 * 
 * Provides lightweight, performance-optimized syntax highlighting for markdown elements.
 * Features debounced highlighting, visible-content optimization, and modular pattern system.
 * 
 * Performance targets:
 * - Syntax highlighting: <100ms for 10,000 lines
 * - Keystroke response: <16ms
 * - Memory efficient highlighting updates
 * 
 * @class SyntaxHighlighter
 */
class SyntaxHighlighter {
  /**
   * Highlighting token types
   */
  static TOKENS = {
    HEADER: 'header',
    BOLD: 'bold',
    ITALIC: 'italic',
    CODE_INLINE: 'code-inline',
    CODE_BLOCK: 'code-block',
    LINK: 'link',
    LIST: 'list',
    BLOCKQUOTE: 'blockquote',
    STRIKETHROUGH: 'strikethrough',
    TABLE: 'table',
    TEXT: 'text'
  };

  /**
   * Initialize syntax highlighter
   * @param {Object} options - Configuration options
   */
  constructor(options = {}) {
    this.options = {
      debounceDelay: options.debounceDelay || 300,
      maxLinesForFullHighlight: options.maxLinesForFullHighlight || 1000,
      visibleLinesBuffer: options.visibleLinesBuffer || 50,
      enablePerformanceLogging: options.enablePerformanceLogging || false,
      ...options
    };

    // Performance tracking
    this.lastHighlightTime = 0;
    this.highlightCount = 0;
    this.debounceTimeout = null;

    // Initialize regex patterns
    this.patterns = this.createPatterns();
    
    // Cache for processed content
    this.cache = new Map();
    this.maxCacheSize = 100;
    
    console.log('✅ SyntaxHighlighter initialized with options:', this.options);
  }

  /**
   * Create regex patterns for markdown syntax
   * @returns {Map} Map of token types to regex patterns
   * @private
   */
  createPatterns() {
    const patterns = new Map();

    // Headers (# ## ### #### ##### ######)
    patterns.set(SyntaxHighlighter.TOKENS.HEADER, {
      regex: /^(#{1,6})\s+(.*)$/gm,
      replacement: (match, hashes, content) => ({
        type: SyntaxHighlighter.TOKENS.HEADER,
        level: hashes.length,
        content: content.trim(),
        fullMatch: match
      })
    });

    // Bold formatting (**text**, __text__) - must be processed before italic
    patterns.set(SyntaxHighlighter.TOKENS.BOLD, {
      regex: /(\*\*|__)([^\*_\n]+?)\1/g,
      replacement: (match, delimiter, content) => ({
        type: SyntaxHighlighter.TOKENS.BOLD,
        content: content,
        delimiter: delimiter,
        fullMatch: match
      })
    });

    // Italic formatting (*text*, _text_) - processed after bold to avoid conflicts
    patterns.set(SyntaxHighlighter.TOKENS.ITALIC, {
      regex: /(?<!\*)(\*)([^\*\n]+?)\1(?!\*)|(?<!_)(_)([^_\n]+?)\3(?!_)/g,
      replacement: (match, asterisk, asteriskContent, underscore, underscoreContent) => ({
        type: SyntaxHighlighter.TOKENS.ITALIC,
        content: asteriskContent || underscoreContent,
        delimiter: asterisk || underscore,
        fullMatch: match
      })
    });

    // Code blocks (```...```)
    patterns.set(SyntaxHighlighter.TOKENS.CODE_BLOCK, {
      regex: /^```(\w+)?\n([\s\S]*?)^```$/gm,
      replacement: (match, language, content) => ({
        type: SyntaxHighlighter.TOKENS.CODE_BLOCK,
        language: language || 'text',
        content: content.trim(),
        fullMatch: match
      })
    });

    // Inline code (`code`)
    patterns.set(SyntaxHighlighter.TOKENS.CODE_INLINE, {
      regex: /`([^`]+)`/g,
      replacement: (match, content) => ({
        type: SyntaxHighlighter.TOKENS.CODE_INLINE,
        content: content,
        fullMatch: match
      })
    });

    // Links ([text](url), [text][ref])
    patterns.set(SyntaxHighlighter.TOKENS.LINK, {
      regex: /\[([^\]]+)\]\(([^)]+)\)|\[([^\]]+)\]\[([^\]]*)\]/g,
      replacement: (match, text1, url, text2, ref) => ({
        type: SyntaxHighlighter.TOKENS.LINK,
        text: text1 || text2,
        url: url,
        ref: ref,
        fullMatch: match
      })
    });

    // Lists (-, *, 1., 2., etc.)
    patterns.set(SyntaxHighlighter.TOKENS.LIST, {
      regex: /^(\s*)([-*+]|\d+\.)\s+(.*)$/gm,
      replacement: (match, indent, marker, content) => ({
        type: SyntaxHighlighter.TOKENS.LIST,
        indent: indent.length,
        marker: marker,
        content: content.trim(),
        fullMatch: match
      })
    });

    // Blockquotes (> text)
    patterns.set(SyntaxHighlighter.TOKENS.BLOCKQUOTE, {
      regex: /^(\s*)(>+)\s*(.*)$/gm,
      replacement: (match, indent, markers, content) => ({
        type: SyntaxHighlighter.TOKENS.BLOCKQUOTE,
        level: markers.length,
        indent: indent.length,
        content: content.trim(),
        fullMatch: match
      })
    });

    // Strikethrough (~~text~~)
    patterns.set(SyntaxHighlighter.TOKENS.STRIKETHROUGH, {
      regex: /~~((?:(?!~~)[^~])+)~~/g,
      replacement: (match, content) => ({
        type: SyntaxHighlighter.TOKENS.STRIKETHROUGH,
        content: content,
        fullMatch: match
      })
    });

    // Tables (basic support for | syntax)
    patterns.set(SyntaxHighlighter.TOKENS.TABLE, {
      regex: /^\|(.+)\|$/gm,
      replacement: (match, content) => ({
        type: SyntaxHighlighter.TOKENS.TABLE,
        content: content.trim(),
        fullMatch: match
      })
    });

    return patterns;
  }

  /**
   * Highlight markdown content with debouncing
   * @param {string} content - Markdown content to highlight
   * @param {HTMLElement} overlayElement - Element to render highlights in
   * @param {Object} viewportInfo - Viewport information for optimization
   * @returns {Promise} Promise that resolves when highlighting is complete
   */
  highlightWithDebounce(content, overlayElement, viewportInfo = null) {
    return new Promise((resolve) => {
      // Clear existing debounce timeout
      clearTimeout(this.debounceTimeout);

      this.debounceTimeout = setTimeout(async () => {
        try {
          await this.highlight(content, overlayElement, viewportInfo);
          resolve();
        } catch (error) {
          console.error('❌ Error during debounced highlighting:', error);
          resolve(); // Resolve anyway to not block the UI
        }
      }, this.options.debounceDelay);
    });
  }

  /**
   * Highlight markdown content
   * @param {string} content - Markdown content to highlight
   * @param {HTMLElement} overlayElement - Element to render highlights in
   * @param {Object} viewportInfo - Viewport information for optimization
   * @returns {Promise} Promise that resolves when highlighting is complete
   */
  async highlight(content, overlayElement, viewportInfo = null) {
    const startTime = performance.now();
    
    if (!content || !overlayElement) {
      if (this.options.enablePerformanceLogging) {
        console.warn('⚠️ Invalid highlight parameters');
      }
      return;
    }

    try {
      // Check cache first
      const cacheKey = this.getCacheKey(content, viewportInfo);
      if (this.cache.has(cacheKey)) {
        const cachedResult = this.cache.get(cacheKey);
        overlayElement.innerHTML = cachedResult;
        
        if (this.options.enablePerformanceLogging) {
          console.log(`🚀 Used cached highlight result (${(performance.now() - startTime).toFixed(2)}ms)`);
        }
        return;
      }

      // Determine if we should use viewport optimization
      const lines = content.split('\n');
      const shouldOptimize = lines.length > this.options.maxLinesForFullHighlight && viewportInfo;

      let contentToHighlight = content;
      let startLineIndex = 0;

      if (shouldOptimize) {
        const result = this.extractVisibleContent(lines, viewportInfo);
        contentToHighlight = result.content;
        startLineIndex = result.startLineIndex;
      }

      // Process the content
      const highlightedHTML = await this.processContent(contentToHighlight, startLineIndex);

      // Update overlay element
      overlayElement.innerHTML = highlightedHTML;
      overlayElement.style.opacity = '1';

      // Cache the result
      this.cacheResult(cacheKey, highlightedHTML);

      // Performance logging
      const duration = performance.now() - startTime;
      this.lastHighlightTime = duration;
      this.highlightCount++;

      if (this.options.enablePerformanceLogging) {
        console.log(`✨ Highlighted ${lines.length} lines in ${duration.toFixed(2)}ms (${shouldOptimize ? 'optimized' : 'full'})`);
      }

      // Check performance target
      if (duration > 100) {
        console.warn(`⚠️ Slow highlighting: ${duration.toFixed(2)}ms (target: <100ms)`);
      }

    } catch (error) {
      console.error('❌ Error during syntax highlighting:', error);
      overlayElement.style.opacity = '0'; // Hide overlay on error
    }
  }

  /**
   * Extract visible content based on viewport information
   * @param {Array} lines - Array of content lines
   * @param {Object} viewportInfo - Viewport information
   * @returns {Object} Extracted content and metadata
   * @private
   */
  extractVisibleContent(lines, viewportInfo) {
    if (!viewportInfo || !viewportInfo.firstVisibleLine || !viewportInfo.lastVisibleLine) {
      return { content: lines.join('\n'), startLineIndex: 0 };
    }

    const buffer = this.options.visibleLinesBuffer;
    const startLine = Math.max(0, viewportInfo.firstVisibleLine - buffer);
    const endLine = Math.min(lines.length - 1, viewportInfo.lastVisibleLine + buffer);

    const visibleLines = lines.slice(startLine, endLine + 1);
    
    return {
      content: visibleLines.join('\n'),
      startLineIndex: startLine
    };
  }

  /**
   * Process content and generate highlighted HTML
   * @param {string} content - Content to process
   * @param {number} startLineIndex - Starting line index for line numbers
   * @returns {Promise<string>} Promise that resolves to highlighted HTML
   * @private
   */
  async processContent(content, startLineIndex = 0) {
    // Keep original content for pattern matching
    let processedContent = content;

    // Track processed ranges to avoid overlapping replacements
    const processedRanges = [];

    // Process patterns in order of precedence (code blocks first to protect their content)
    const patternOrder = [
      SyntaxHighlighter.TOKENS.CODE_BLOCK,    // Process first to protect code content
      SyntaxHighlighter.TOKENS.CODE_INLINE,   // Protect inline code second
      SyntaxHighlighter.TOKENS.HEADER,        // Headers before other formatting
      SyntaxHighlighter.TOKENS.BOLD,          // Bold before italic to handle overlap
      SyntaxHighlighter.TOKENS.ITALIC,        // Italic after bold
      SyntaxHighlighter.TOKENS.STRIKETHROUGH, // Strikethrough after other formatting
      SyntaxHighlighter.TOKENS.LINK,          // Links before lists/blockquotes
      SyntaxHighlighter.TOKENS.BLOCKQUOTE,    // Block elements next
      SyntaxHighlighter.TOKENS.LIST,          // List items
      SyntaxHighlighter.TOKENS.TABLE          // Tables last
    ];

    // Apply highlighting for each pattern
    for (const tokenType of patternOrder) {
      const pattern = this.patterns.get(tokenType);
      if (pattern) {
        processedContent = this.applyPattern(processedContent, tokenType, pattern, processedRanges);
      }
    }

    // Escape any remaining unprocessed HTML content (outside of our highlighting spans)
    processedContent = this.escapeUnprocessedHtml(processedContent);

    return processedContent;
  }

  /**
   * Apply a specific pattern to content
   * @param {string} content - Content to process
   * @param {string} tokenType - Token type being processed
   * @param {Object} pattern - Pattern configuration
   * @param {Array} processedRanges - Array of already processed ranges
   * @returns {string} Processed content
   * @private
   */
  applyPattern(content, tokenType, pattern, processedRanges) {
    const regex = new RegExp(pattern.regex.source, pattern.regex.flags);
    let match;
    const replacements = [];

    // Find all matches
    while ((match = regex.exec(content)) !== null) {
      const matchStart = match.index;
      const matchEnd = match.index + match[0].length;

      // Check if this range overlaps with already processed ranges
      const overlaps = processedRanges.some(range => 
        (matchStart >= range.start && matchStart < range.end) ||
        (matchEnd > range.start && matchEnd <= range.end) ||
        (matchStart <= range.start && matchEnd >= range.end)
      );

      if (!overlaps) {
        const tokenData = this.createTokenData(tokenType, match);
        const htmlReplacement = this.createHtmlToken(tokenType, tokenData);
        
        replacements.push({
          start: matchStart,
          end: matchEnd,
          replacement: htmlReplacement,
          original: match[0]
        });

        processedRanges.push({ start: matchStart, end: matchEnd });
      }
    }

    // Apply replacements in reverse order to maintain positions
    replacements.reverse().forEach(replacement => {
      content = content.slice(0, replacement.start) + 
                replacement.replacement + 
                content.slice(replacement.end);
    });

    return content;
  }

  /**
   * Create token data from regex match
   * @param {string} tokenType - Type of token
   * @param {Array} match - Regex match array
   * @returns {Object} Token data
   * @private
   */
  createTokenData(tokenType, match) {
    const pattern = this.patterns.get(tokenType);
    
    if (pattern.replacement && typeof pattern.replacement === 'function') {
      return pattern.replacement.apply(null, match);
    }

    // Fallback for simple patterns
    return {
      type: tokenType,
      content: match[1] || match[0],
      fullMatch: match[0]
    };
  }

  /**
   * Create HTML representation of a token
   * @param {string} tokenType - Type of token
   * @param {Object} tokenData - Token data
   * @returns {string} HTML representation
   * @private
   */
  createHtmlToken(tokenType, tokenData) {
    // Helper to escape content that goes inside HTML
    const escapeContent = (text) => this.escapeHtml(text || '');
    
    switch (tokenType) {
      case SyntaxHighlighter.TOKENS.HEADER:
        return `<span class="md-header md-header-${tokenData.level}">` +
               `<span class="md-header-marker">${'#'.repeat(tokenData.level)}</span>` +
               ` <span class="md-header-content">${escapeContent(tokenData.content)}</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.BOLD:
        return `<span class="md-bold">` +
               `<span class="md-bold-marker">${escapeContent(tokenData.delimiter)}</span>` +
               `<span class="md-bold-content">${escapeContent(tokenData.content)}</span>` +
               `<span class="md-bold-marker">${escapeContent(tokenData.delimiter)}</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.ITALIC:
        return `<span class="md-italic">` +
               `<span class="md-italic-marker">${escapeContent(tokenData.delimiter)}</span>` +
               `<span class="md-italic-content">${escapeContent(tokenData.content)}</span>` +
               `<span class="md-italic-marker">${escapeContent(tokenData.delimiter)}</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.CODE_BLOCK:
        return `<span class="md-code-block">` +
               `<span class="md-code-block-marker">\`\`\`${escapeContent(tokenData.language || '')}</span>\n` +
               `<span class="md-code-block-content">${escapeContent(tokenData.content)}</span>\n` +
               `<span class="md-code-block-marker">\`\`\`</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.CODE_INLINE:
        return `<span class="md-code-inline">` +
               `<span class="md-code-inline-marker">\`</span>` +
               `<span class="md-code-inline-content">${escapeContent(tokenData.content)}</span>` +
               `<span class="md-code-inline-marker">\`</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.LINK:
        const linkText = tokenData.text || '';
        const linkUrl = tokenData.url || tokenData.ref || '';
        return `<span class="md-link">` +
               `<span class="md-link-marker">[</span>` +
               `<span class="md-link-text">${escapeContent(linkText)}</span>` +
               `<span class="md-link-marker">]</span>` +
               `<span class="md-link-marker">(</span>` +
               `<span class="md-link-url">${escapeContent(linkUrl)}</span>` +
               `<span class="md-link-marker">)</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.LIST:
        return `<span class="md-list md-list-level-${Math.floor(tokenData.indent / 2)}">` +
               `<span class="md-list-marker">${escapeContent(tokenData.marker)}</span> ` +
               `<span class="md-list-content">${escapeContent(tokenData.content)}</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.BLOCKQUOTE:
        return `<span class="md-blockquote md-blockquote-level-${tokenData.level}">` +
               `<span class="md-blockquote-marker">${'>'.repeat(tokenData.level)}</span> ` +
               `<span class="md-blockquote-content">${escapeContent(tokenData.content)}</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.STRIKETHROUGH:
        return `<span class="md-strikethrough">` +
               `<span class="md-strikethrough-marker">~~</span>` +
               `<span class="md-strikethrough-content">${escapeContent(tokenData.content)}</span>` +
               `<span class="md-strikethrough-marker">~~</span>` +
               `</span>`;

      case SyntaxHighlighter.TOKENS.TABLE:
        return `<span class="md-table">` +
               `<span class="md-table-marker">|</span>` +
               `<span class="md-table-content">${escapeContent(tokenData.content)}</span>` +
               `<span class="md-table-marker">|</span>` +
               `</span>`;

      default:
        return escapeContent(tokenData.fullMatch);
    }
  }

  /**
   * Escape HTML characters to prevent XSS
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   * @private
   */
  escapeHtml(text) {
    const htmlEscapes = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#x27;'
    };
    
    return text.replace(/[&<>"']/g, (match) => htmlEscapes[match]);
  }

  /**
   * Escape HTML characters outside of our highlighting spans
   * @param {string} content - Content with highlighting spans
   * @returns {string} Content with unprocessed HTML escaped
   * @private
   */
  escapeUnprocessedHtml(content) {
    // Split content by HTML spans we created
    const parts = content.split(/(<span[^>]*class="md-[^"]*"[^>]*>.*?<\/span>)/g);
    
    // Escape only the parts outside our spans
    return parts.map((part, index) => {
      // Even indices are outside spans, odd indices are our spans
      if (index % 2 === 0) {
        return this.escapeHtml(part);
      }
      return part; // Keep our spans as-is
    }).join('');
  }

  /**
   * Generate cache key for content and viewport
   * @param {string} content - Content to cache
   * @param {Object} viewportInfo - Viewport information
   * @returns {string} Cache key
   * @private
   */
  getCacheKey(content, viewportInfo) {
    const contentHash = this.simpleHash(content);
    const viewportHash = viewportInfo ? this.simpleHash(JSON.stringify(viewportInfo)) : 'full';
    return `${contentHash}-${viewportHash}`;
  }

  /**
   * Simple hash function for cache keys
   * @param {string} str - String to hash
   * @returns {string} Hash value
   * @private
   */
  simpleHash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(36);
  }

  /**
   * Cache highlighting result
   * @param {string} key - Cache key
   * @param {string} result - Result to cache
   * @private
   */
  cacheResult(key, result) {
    if (this.cache.size >= this.maxCacheSize) {
      // Remove oldest entry
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }
    
    this.cache.set(key, result);
  }

  /**
   * Clear highlighting cache
   */
  clearCache() {
    this.cache.clear();
    console.log('🧹 Syntax highlighting cache cleared');
  }

  /**
   * Get performance statistics
   * @returns {Object} Performance statistics
   */
  getPerformanceStats() {
    return {
      lastHighlightTime: this.lastHighlightTime,
      totalHighlights: this.highlightCount,
      averageTime: this.highlightCount > 0 ? (this.lastHighlightTime / this.highlightCount) : 0,
      cacheSize: this.cache.size,
      maxCacheSize: this.maxCacheSize
    };
  }

  /**
   * Update configuration options
   * @param {Object} newOptions - New options to merge
   */
  updateOptions(newOptions) {
    this.options = { ...this.options, ...newOptions };
    console.log('⚙️ Syntax highlighter options updated:', newOptions);
  }

  /**
   * Destroy highlighter and clean up resources
   */
  destroy() {
    clearTimeout(this.debounceTimeout);
    this.cache.clear();
    this.patterns.clear();
    
    console.log('✅ SyntaxHighlighter destroyed');
  }
}

// Export for ES6 module usage
export default SyntaxHighlighter;