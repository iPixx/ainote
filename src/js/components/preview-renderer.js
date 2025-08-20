/**
 * PreviewRenderer - HTML generation and display component for aiNote
 * 
 * Takes parsed markdown AST from MarkdownParser and generates optimized HTML
 * for display in the preview panel with performance and accessibility features.
 * 
 * Features:
 * - Memory efficient HTML generation with virtual DOM-like updates
 * - Smooth updates without flickering
 * - Scroll position preservation
 * - Dark/light theme support
 * - Accessibility compliance
 * - Mobile-responsive rendering
 * 
 * Performance targets:
 * - Rendering time: <50ms for typical documents
 * - Memory usage: <3MB for large documents
 * - Update frequency: 60fps during smooth scrolling
 * 
 * @class PreviewRenderer
 */
class PreviewRenderer {
  constructor(container, appState) {
    this.container = container;
    this.appState = appState;
    
    // Rendering state
    this.currentContent = '';
    this.currentHtml = '';
    this.scrollPosition = 0;
    this.lastUpdateTime = 0;
    
    // Performance tracking
    this.renderStats = {
      totalRenders: 0,
      averageRenderTime: 0,
      maxRenderTime: 0,
      memoryUsage: 0
    };
    
    // DOM elements cache
    this.elements = {
      content: null,
      scrollContainer: null
    };
    
    // Virtual DOM for efficient updates
    this.virtualDom = {
      elements: new Map(),
      lastSnapshot: null
    };
    
    this.initialize();
  }

  /**
   * Initialize the preview renderer
   */
  initialize() {
    if (!this.container) {
      console.error('❌ PreviewRenderer: No container provided');
      return;
    }

    this.setupDOMStructure();
    this.setupEventListeners();
    this.loadTheme();
    
    console.log('✅ PreviewRenderer initialized successfully');
  }

  /**
   * Setup the DOM structure for the preview panel
   */
  setupDOMStructure() {
    // Clear existing content
    this.container.innerHTML = '';
    
    // Create main preview container
    const previewContainer = document.createElement('div');
    previewContainer.className = 'preview-container';
    previewContainer.setAttribute('role', 'main');
    previewContainer.setAttribute('aria-label', 'Markdown preview');
    
    // Create scroll container for content
    const scrollContainer = document.createElement('div');
    scrollContainer.className = 'preview-scroll-container';
    scrollContainer.setAttribute('tabindex', '0');
    scrollContainer.setAttribute('aria-live', 'polite');
    
    // Create content area
    const contentArea = document.createElement('article');
    contentArea.className = 'preview-content';
    contentArea.setAttribute('role', 'document');
    
    // Build hierarchy
    scrollContainer.appendChild(contentArea);
    previewContainer.appendChild(scrollContainer);
    this.container.appendChild(previewContainer);
    
    // Cache elements
    this.elements.content = contentArea;
    this.elements.scrollContainer = scrollContainer;
    
    console.log('📐 Preview DOM structure created');
  }

  /**
   * Setup event listeners for scroll and resize handling
   */
  setupEventListeners() {
    if (!this.elements.scrollContainer) return;

    // Scroll position tracking
    this.elements.scrollContainer.addEventListener('scroll', 
      this.throttle(this.handleScroll.bind(this), 16)); // 60fps
      
    // Resize handling
    window.addEventListener('resize', 
      this.debounce(this.handleResize.bind(this), 250));
      
    // Theme change detection
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addListener(this.handleThemeChange.bind(this));
    
    console.log('🎧 Event listeners setup complete');
  }

  /**
   * Load and apply theme
   */
  loadTheme() {
    const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    this.container.classList.toggle('dark-theme', isDark);
    this.container.classList.toggle('light-theme', !isDark);
    
    console.log(`🎨 Theme loaded: ${isDark ? 'dark' : 'light'}`);
  }

  /**
   * Main render method - converts markdown to HTML and displays it
   * @param {string} markdown - Raw markdown content
   * @returns {Promise<void>}
   */
  async render(markdown) {
    if (!markdown || typeof markdown !== 'string') {
      this.clear();
      return;
    }

    const startTime = performance.now();
    
    try {
      // Import and use the MarkdownParser
      const { default: MarkdownParser } = await import('../utils/markdown-parser.js');
      const parser = new MarkdownParser();
      
      // Parse markdown to HTML
      const html = parser.parse(markdown);
      
      // Update preview if content changed
      if (html !== this.currentHtml) {
        await this.updatePreview(html, markdown);
      }
      
      // Update performance stats
      const renderTime = performance.now() - startTime;
      this.updateRenderStats(renderTime);
      
      // Log performance warning if needed
      if (renderTime > 50) {
        console.warn(`⚠️ Render time exceeded target: ${renderTime.toFixed(2)}ms (target: <50ms)`);
      }
      
    } catch (error) {
      console.error('❌ Error rendering markdown:', error);
      this.renderError(error);
    }
  }

  /**
   * Update preview content with efficient DOM manipulation
   * @param {string} html - Generated HTML content
   * @param {string} markdown - Original markdown content
   */
  async updatePreview(html, markdown) {
    if (!this.elements.content) return;

    const startTime = performance.now();
    
    // Save current scroll position
    this.saveScrollPosition();
    
    // Perform efficient DOM update
    await this.performDOMUpdate(html);
    
    // Store current state
    this.currentHtml = html;
    this.currentContent = markdown;
    
    // Restore scroll position
    this.restoreScrollPosition();
    
    // Update memory tracking
    this.trackMemoryUsage();
    
    const updateTime = performance.now() - startTime;
    console.log(`📊 Preview updated in ${updateTime.toFixed(2)}ms`);
  }

  /**
   * Perform efficient DOM update with virtual DOM-like approach
   * @param {string} newHtml - New HTML content
   */
  async performDOMUpdate(newHtml) {
    if (!this.elements.content) return;

    // Create virtual representation of new content
    const tempContainer = document.createElement('div');
    tempContainer.innerHTML = newHtml;
    
    // Get current and new elements
    const currentElements = Array.from(this.elements.content.children);
    const newElements = Array.from(tempContainer.children);
    
    // Perform minimal DOM updates
    this.reconcileDOMChanges(currentElements, newElements);
    
    // Update accessibility attributes
    this.updateAccessibility();
  }

  /**
   * Reconcile DOM changes efficiently (virtual DOM-like approach)
   * @param {Element[]} currentElements - Current DOM elements
   * @param {Element[]} newElements - New elements to render
   */
  reconcileDOMChanges(currentElements, newElements) {
    const maxLength = Math.max(currentElements.length, newElements.length);
    
    for (let i = 0; i < maxLength; i++) {
      const currentElement = currentElements[i];
      const newElement = newElements[i];
      
      if (!currentElement && newElement) {
        // Add new element
        this.elements.content.appendChild(newElement.cloneNode(true));
      } else if (currentElement && !newElement) {
        // Remove element
        currentElement.remove();
      } else if (currentElement && newElement) {
        // Update existing element if different
        if (currentElement.outerHTML !== newElement.outerHTML) {
          currentElement.replaceWith(newElement.cloneNode(true));
        }
      }
    }
  }

  /**
   * Update accessibility attributes
   */
  updateAccessibility() {
    if (!this.elements.content) return;

    // Update heading levels for proper document outline
    const headings = this.elements.content.querySelectorAll('h1, h2, h3, h4, h5, h6');
    headings.forEach((heading, index) => {
      heading.setAttribute('id', `heading-${index + 1}`);
      heading.setAttribute('tabindex', '0');
    });

    // Update links for external navigation
    const links = this.elements.content.querySelectorAll('a[href^="http"]');
    links.forEach(link => {
      link.setAttribute('target', '_blank');
      link.setAttribute('rel', 'noopener noreferrer');
      link.setAttribute('aria-describedby', 'external-link-description');
    });

    // Update code blocks for better accessibility
    const codeBlocks = this.elements.content.querySelectorAll('pre code');
    codeBlocks.forEach((code, index) => {
      const pre = code.parentElement;
      pre.setAttribute('role', 'region');
      pre.setAttribute('aria-label', `Code block ${index + 1}`);
      pre.setAttribute('tabindex', '0');
    });

    // Update images with proper alt text handling
    const images = this.elements.content.querySelectorAll('img');
    images.forEach(img => {
      if (!img.getAttribute('alt')) {
        img.setAttribute('alt', 'Image without description');
      }
      img.setAttribute('loading', 'lazy');
    });
  }

  /**
   * Save current scroll position
   */
  saveScrollPosition() {
    if (this.elements.scrollContainer) {
      this.scrollPosition = this.elements.scrollContainer.scrollTop;
    }
  }

  /**
   * Restore scroll position
   */
  restoreScrollPosition() {
    if (this.elements.scrollContainer && this.scrollPosition !== undefined) {
      this.elements.scrollContainer.scrollTop = this.scrollPosition;
    }
  }

  /**
   * Scroll to specific position (percentage of document height)
   * @param {number} percentage - Scroll position as percentage (0-100)
   */
  scrollToPosition(percentage) {
    if (!this.elements.scrollContainer) return;

    const maxScroll = this.elements.scrollContainer.scrollHeight - 
                     this.elements.scrollContainer.clientHeight;
    const targetScroll = (percentage / 100) * maxScroll;
    
    this.elements.scrollContainer.scrollTo({
      top: targetScroll,
      behavior: 'smooth'
    });
  }

  /**
   * Handle scroll events
   * @param {Event} event - Scroll event
   */
  handleScroll(event) {
    this.scrollPosition = event.target.scrollTop;
    
    // Emit scroll event for editor synchronization (future feature)
    if (this.appState && this.appState.emit) {
      const scrollPercentage = this.getScrollPercentage();
      this.appState.emit('preview:scroll', { percentage: scrollPercentage });
    }
  }

  /**
   * Get current scroll position as percentage
   * @returns {number} Scroll percentage (0-100)
   */
  getScrollPercentage() {
    if (!this.elements.scrollContainer) return 0;
    
    const { scrollTop, scrollHeight, clientHeight } = this.elements.scrollContainer;
    const maxScroll = scrollHeight - clientHeight;
    
    return maxScroll > 0 ? (scrollTop / maxScroll) * 100 : 0;
  }

  /**
   * Handle resize events
   */
  handleResize() {
    // Recalculate layout if needed
    if (this.elements.content) {
      this.elements.content.style.maxWidth = 'none';
      // Force reflow
      this.elements.content.offsetHeight;
      this.elements.content.style.maxWidth = '';
    }
    
    console.log('📐 Preview resized and recalculated');
  }

  /**
   * Handle theme changes
   * @param {MediaQueryListEvent} event - Theme change event
   */
  handleThemeChange(event) {
    const isDark = event.matches;
    this.container.classList.toggle('dark-theme', isDark);
    this.container.classList.toggle('light-theme', !isDark);
    
    console.log(`🎨 Theme changed to: ${isDark ? 'dark' : 'light'}`);
  }

  /**
   * Clear preview content
   */
  clear() {
    if (this.elements.content) {
      this.elements.content.innerHTML = '';
    }
    
    this.currentContent = '';
    this.currentHtml = '';
    this.scrollPosition = 0;
    
    console.log('🧹 Preview cleared');
  }

  /**
   * Render error state
   * @param {Error} error - Error to display
   */
  renderError(error) {
    if (!this.elements.content) return;

    this.elements.content.innerHTML = `
      <div class="preview-error" role="alert">
        <h2>Preview Error</h2>
        <p>Failed to render markdown content:</p>
        <pre><code>${this.escapeHtml(error.message)}</code></pre>
        <p><em>Please check your markdown syntax and try again.</em></p>
      </div>
    `;
    
    console.error('❌ Rendered error state:', error);
  }

  /**
   * Update rendering performance statistics
   * @param {number} renderTime - Time taken for render
   */
  updateRenderStats(renderTime) {
    this.renderStats.totalRenders++;
    this.renderStats.maxRenderTime = Math.max(this.renderStats.maxRenderTime, renderTime);
    
    // Calculate running average
    const alpha = 0.1; // Exponential moving average factor
    if (this.renderStats.averageRenderTime === 0) {
      this.renderStats.averageRenderTime = renderTime;
    } else {
      this.renderStats.averageRenderTime = 
        alpha * renderTime + (1 - alpha) * this.renderStats.averageRenderTime;
    }
  }

  /**
   * Track memory usage
   */
  trackMemoryUsage() {
    if (window.performance && window.performance.memory) {
      this.renderStats.memoryUsage = window.performance.memory.usedJSHeapSize / 1024 / 1024; // MB
      
      if (this.renderStats.memoryUsage > 3) { // Target: <3MB for large documents
        console.warn(`⚠️ Memory usage exceeded target: ${this.renderStats.memoryUsage.toFixed(2)}MB (target: <3MB)`);
      }
    }
  }

  /**
   * Get performance statistics
   * @returns {Object} Performance statistics
   */
  getPerformanceStats() {
    return {
      ...this.renderStats,
      averageRenderTime: Math.round(this.renderStats.averageRenderTime * 100) / 100,
      maxRenderTime: Math.round(this.renderStats.maxRenderTime * 100) / 100,
      memoryUsage: Math.round(this.renderStats.memoryUsage * 100) / 100
    };
  }

  /**
   * Escape HTML characters for security
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   */
  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Throttle function execution
   * @param {Function} func - Function to throttle
   * @param {number} limit - Time limit in milliseconds
   * @returns {Function} Throttled function
   */
  throttle(func, limit) {
    let inThrottle;
    return function(...args) {
      if (!inThrottle) {
        func.apply(this, args);
        inThrottle = true;
        setTimeout(() => inThrottle = false, limit);
      }
    };
  }

  /**
   * Debounce function execution
   * @param {Function} func - Function to debounce
   * @param {number} delay - Delay in milliseconds
   * @returns {Function} Debounced function
   */
  debounce(func, delay) {
    let timeoutId;
    return function(...args) {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => func.apply(this, args), delay);
    };
  }

  /**
   * Cleanup method for removing event listeners and clearing memory
   */
  destroy() {
    // Remove event listeners
    if (this.elements.scrollContainer) {
      this.elements.scrollContainer.removeEventListener('scroll', this.handleScroll);
    }
    
    window.removeEventListener('resize', this.handleResize);
    
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.removeListener(this.handleThemeChange);
    
    // Clear DOM elements
    this.clear();
    
    // Clear references
    this.container = null;
    this.elements = {};
    this.virtualDom = { elements: new Map(), lastSnapshot: null };
    
    console.log('🧹 PreviewRenderer destroyed');
  }
}

export default PreviewRenderer;