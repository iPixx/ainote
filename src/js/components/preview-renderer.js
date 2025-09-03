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
    
    // Real-time update state
    this.realTimeEnabled = false;
    this.updateDebounceTimeout = null;
    this.updateDebounceDelay = 200; // 200ms as specified
    this.editorInstance = null;
    this.lastContentHash = '';
    this.pendingUpdate = false;
    
    // Performance tracking
    this.renderStats = {
      totalRenders: 0,
      averageRenderTime: 0,
      maxRenderTime: 0,
      memoryUsage: 0,
      realTimeUpdates: 0,
      incrementalUpdates: 0,
      memoryGrowthRate: 0
    };
    
    // Scroll synchronization state
    this.scrollSync = {
      enabled: false,
      editorScrollElement: null,
      editorScrollRatio: 0,
      previewScrollRatio: 0,
      syncInProgress: false,
      tolerance: 5 // 5px tolerance as specified
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
    
    // Incremental parsing state
    this.incremental = {
      enabled: false,
      chunkSize: 1000, // Lines to process per chunk
      lastProcessedLine: 0,
      contentLines: [],
      processedSections: new Map()
    };
    
    // Memory monitoring
    this.memoryMonitor = {
      lastCheck: 0,
      checkInterval: 10000, // 10 seconds
      baselineMemory: 0,
      maxMemoryGrowth: 1048576, // 1MB per hour target
      cleanupThreshold: 5242880 // 5MB cleanup threshold
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
    this.initializeMemoryMonitoring();
    
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
   * Update accessibility attributes and setup advanced features
   */
  updateAccessibility() {
    if (!this.elements.content) return;

    // Update heading levels for proper document outline
    const headings = this.elements.content.querySelectorAll('h1, h2, h3, h4, h5, h6');
    headings.forEach((heading, index) => {
      heading.setAttribute('id', `heading-${index + 1}`);
      heading.setAttribute('tabindex', '0');
    });

    // Setup link click handling
    this.handleLinkClicks();

    // Update code blocks for better accessibility
    const codeBlocks = this.elements.content.querySelectorAll('pre code');
    codeBlocks.forEach((code, index) => {
      const pre = code.parentElement;
      pre.setAttribute('role', 'region');
      pre.setAttribute('aria-label', `Code block ${index + 1}`);
      pre.setAttribute('tabindex', '0');
    });

    // Setup image loading and error handling
    this.loadImages();

    // Setup table enhancements
    this.renderTables();
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
   * Handle link clicks and navigation
   * Target: <5ms response time for link handling
   */
  handleLinkClicks() {
    if (!this.elements.content) return;

    const links = this.elements.content.querySelectorAll('a');
    
    links.forEach(link => {
      const href = link.getAttribute('href');
      
      if (!href) return;
      
      // Handle external links
      if (href.startsWith('http://') || href.startsWith('https://')) {
        link.setAttribute('target', '_blank');
        link.setAttribute('rel', 'noopener noreferrer');
        link.setAttribute('aria-describedby', 'external-link-description');
        
        // Add click handler for external link validation
        link.addEventListener('click', (e) => {
          const startTime = performance.now();
          this.validateExternalLink(e, href);
          const responseTime = performance.now() - startTime;
          
          if (responseTime > 5) {
            console.warn(`⚠️ Link handling exceeded target: ${responseTime.toFixed(2)}ms (target: <5ms)`);
          }
        });
      }
      // Handle internal/relative links (for future file navigation)
      else if (href.startsWith('./') || href.startsWith('../') || href.endsWith('.md')) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          const startTime = performance.now();
          this.handleInternalLink(href);
          const responseTime = performance.now() - startTime;
          
          if (responseTime > 5) {
            console.warn(`⚠️ Link handling exceeded target: ${responseTime.toFixed(2)}ms (target: <5ms)`);
          }
        });
      }
      // Handle anchor links
      else if (href.startsWith('#')) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          const startTime = performance.now();
          this.scrollToAnchor(href.substring(1));
          const responseTime = performance.now() - startTime;
          
          if (responseTime > 5) {
            console.warn(`⚠️ Link handling exceeded target: ${responseTime.toFixed(2)}ms (target: <5ms)`);
          }
        });
      }
    });
    
    // Add external link description for accessibility
    if (!document.getElementById('external-link-description')) {
      const description = document.createElement('div');
      description.id = 'external-link-description';
      description.className = 'sr-only';
      description.textContent = 'Opens in a new window';
      document.body.appendChild(description);
    }
  }

  /**
   * Validate external link before opening
   * @param {Event} e - Click event
   * @param {string} href - Link URL
   */
  validateExternalLink(e, href) {
    try {
      // Basic URL validation
      const url = new URL(href);
      
      // Check for suspicious protocols
      if (!['http:', 'https:'].includes(url.protocol)) {
        e.preventDefault();
        console.warn('⚠️ Blocked suspicious link protocol:', url.protocol);
        return;
      }
      
      // Emit event for app state tracking (future feature)
      if (this.appState && this.appState.emit) {
        this.appState.emit('preview:external-link', { url: href });
      }
      
    } catch (error) {
      e.preventDefault();
      console.error('❌ Invalid URL:', href, error);
    }
  }

  /**
   * Handle internal link navigation (for future file navigation)
   * @param {string} href - Internal link path
   */
  handleInternalLink(href) {
    
    // Emit event for app state (future file navigation)
    if (this.appState && this.appState.emit) {
      this.appState.emit('preview:internal-link', { path: href });
    }
    
    // Future: integrate with file tree navigation
    // For now, just log the action
  }

  /**
   * Scroll to anchor link within the document
   * @param {string} anchor - Anchor ID to scroll to
   */
  scrollToAnchor(anchor) {
    if (!this.elements.content) return;
    
    const targetElement = this.elements.content.querySelector(`#${anchor}, [name="${anchor}"]`);
    
    if (targetElement) {
      targetElement.scrollIntoView({
        behavior: 'smooth',
        block: 'start'
      });
      
      // Focus the element for accessibility
      if (targetElement.hasAttribute('tabindex')) {
        targetElement.focus();
      }
      
      console.log('⚓ Scrolled to anchor:', anchor);
    } else {
      console.warn('⚠️ Anchor not found:', anchor);
    }
  }

  /**
   * Load images with proper sizing and error handling
   */
  loadImages() {
    if (!this.elements.content) return;

    const images = this.elements.content.querySelectorAll('img');
    
    images.forEach((img, index) => {
      // Set default attributes
      if (!img.getAttribute('alt')) {
        img.setAttribute('alt', `Image ${index + 1} - No description provided`);
      }
      
      img.setAttribute('loading', 'lazy');
      img.setAttribute('decoding', 'async');
      
      // Add responsive sizing classes
      img.classList.add('preview-image');
      
      // Handle image loading
      img.addEventListener('load', () => {
        img.classList.add('loaded');
        console.log('🖼️ Image loaded successfully:', img.src);
      });
      
      // Handle image errors
      img.addEventListener('error', (e) => {
        img.classList.add('error');
        img.setAttribute('alt', `Failed to load image: ${img.src}`);
        
        // Create error placeholder
        const errorDiv = document.createElement('div');
        errorDiv.className = 'image-error';
        errorDiv.innerHTML = `
          <div class="image-error-content">
            <span class="image-error-icon">🚫</span>
            <p class="image-error-text">Image failed to load</p>
            <p class="image-error-src">${this.escapeHtml(img.src)}</p>
          </div>
        `;
        
        img.parentNode.insertBefore(errorDiv, img);
        img.style.display = 'none';
        
        console.error('❌ Image failed to load:', img.src);
      });
      
      // Set proper sizing based on container
      this.optimizeImageSize(img);
    });
  }

  /**
   * Optimize image size for the preview container
   * @param {HTMLImageElement} img - Image element to optimize
   */
  optimizeImageSize(img) {
    if (!img || !this.elements.content) return;
    
    // Wait for image to load to get natural dimensions
    if (img.complete) {
      this.applyImageSizing(img);
    } else {
      img.addEventListener('load', () => this.applyImageSizing(img));
    }
  }

  /**
   * Apply responsive sizing to loaded image
   * @param {HTMLImageElement} img - Loaded image element
   */
  applyImageSizing(img) {
    const containerWidth = this.elements.content.clientWidth;
    const naturalRatio = img.naturalHeight / img.naturalWidth;
    
    // Set max width based on container
    const maxWidth = Math.min(containerWidth - 40, 800); // 40px for padding
    
    if (img.naturalWidth > maxWidth) {
      img.style.maxWidth = `${maxWidth}px`;
      img.style.height = 'auto';
    }
    
    // Add size classes for styling
    if (img.naturalWidth > containerWidth * 0.8) {
      img.classList.add('large-image');
    } else if (img.naturalWidth < 200) {
      img.classList.add('small-image');
    } else {
      img.classList.add('medium-image');
    }
    
    // Add aspect ratio class for styling
    if (naturalRatio > 1.5) {
      img.classList.add('portrait');
    } else if (naturalRatio < 0.7) {
      img.classList.add('landscape');
    } else {
      img.classList.add('square');
    }
  }

  /**
   * Enhance table rendering with alignment and accessibility
   */
  renderTables() {
    if (!this.elements.content) return;

    const tables = this.elements.content.querySelectorAll('table');
    
    tables.forEach((table, index) => {
      // Add table wrapper for scrolling on small screens
      if (!table.parentElement.classList.contains('table-wrapper')) {
        const wrapper = document.createElement('div');
        wrapper.className = 'table-wrapper';
        table.parentNode.insertBefore(wrapper, table);
        wrapper.appendChild(table);
      }
      
      // Add accessibility attributes
      table.setAttribute('role', 'table');
      table.setAttribute('aria-label', `Table ${index + 1}`);
      
      // Process table headers
      const headers = table.querySelectorAll('th');
      headers.forEach((th, colIndex) => {
        th.setAttribute('scope', 'col');
        th.setAttribute('role', 'columnheader');
        
        // Apply text alignment from markdown table syntax
        const textAlign = this.getTableColumnAlignment(th);
        if (textAlign) {
          th.style.textAlign = textAlign;
        }
      });
      
      // Process table cells
      const cells = table.querySelectorAll('td');
      cells.forEach((td, cellIndex) => {
        td.setAttribute('role', 'cell');
        
        // Apply column alignment to cells
        const columnIndex = Array.from(td.parentElement.children).indexOf(td);
        const headerCell = table.querySelector(`th:nth-child(${columnIndex + 1})`);
        
        if (headerCell) {
          const alignment = headerCell.style.textAlign;
          if (alignment) {
            td.style.textAlign = alignment;
          }
        }
      });
      
      // Add table navigation for keyboard users
      table.setAttribute('tabindex', '0');
      table.addEventListener('keydown', (e) => {
        this.handleTableNavigation(e, table);
      });
      
      console.log(`📊 Table ${index + 1} enhanced with accessibility features`);
    });
  }

  /**
   * Get table column alignment from markdown syntax
   * @param {HTMLElement} header - Table header element
   * @returns {string|null} CSS text-align value
   */
  getTableColumnAlignment(header) {
    // Check for alignment indicators in the header content
    const text = header.textContent.trim();
    
    // Look for markdown alignment syntax that might be preserved
    if (text.startsWith(':') && text.endsWith(':')) {
      return 'center';
    } else if (text.endsWith(':')) {
      return 'right';
    } else if (text.startsWith(':')) {
      return 'left';
    }
    
    return null;
  }

  /**
   * Handle keyboard navigation within tables
   * @param {KeyboardEvent} e - Keyboard event
   * @param {HTMLTableElement} table - Table element
   */
  handleTableNavigation(e, table) {
    if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
      return;
    }
    
    e.preventDefault();
    
    const cells = table.querySelectorAll('td, th');
    const currentCell = document.activeElement;
    const currentIndex = Array.from(cells).indexOf(currentCell);
    
    if (currentIndex === -1) return;
    
    let targetIndex = currentIndex;
    const cols = table.rows[0].cells.length;
    
    switch (e.key) {
      case 'ArrowRight':
        targetIndex = Math.min(currentIndex + 1, cells.length - 1);
        break;
      case 'ArrowLeft':
        targetIndex = Math.max(currentIndex - 1, 0);
        break;
      case 'ArrowDown':
        targetIndex = Math.min(currentIndex + cols, cells.length - 1);
        break;
      case 'ArrowUp':
        targetIndex = Math.max(currentIndex - cols, 0);
        break;
    }
    
    if (cells[targetIndex]) {
      cells[targetIndex].focus();
    }
  }

  /**
   * Export preview content to HTML format
   * Target: <200ms for typical documents
   * @param {Object} options - Export options
   * @returns {Promise<string>} Generated HTML
   */
  async exportToHTML(options = {}) {
    const startTime = performance.now();
    
    try {
      if (!this.elements.content) {
        throw new Error('No content to export');
      }
      
      const {
        includeStyles = true,
        includeMetadata = true,
        title = 'Exported Document',
        theme = 'auto'
      } = options;
      
      // Get current theme
      const currentTheme = theme === 'auto' 
        ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
        : theme;
      
      // Create HTML document structure
      const htmlDocument = this.generateHTMLDocument({
        title,
        content: this.elements.content.innerHTML,
        includeStyles,
        includeMetadata,
        theme: currentTheme
      });
      
      const exportTime = performance.now() - startTime;
      
      if (exportTime > 200) {
        console.warn(`⚠️ Export time exceeded target: ${exportTime.toFixed(2)}ms (target: <200ms)`);
      }
      
      console.log(`📤 HTML export completed in ${exportTime.toFixed(2)}ms`);
      return htmlDocument;
      
    } catch (error) {
      console.error('❌ HTML export failed:', error);
      throw error;
    }
  }

  /**
   * Generate complete HTML document for export
   * @param {Object} params - Document parameters
   * @returns {string} Complete HTML document
   */
  generateHTMLDocument({ title, content, includeStyles, includeMetadata, theme }) {
    const styles = includeStyles ? this.generateExportStyles(theme) : '';
    const metadata = includeMetadata ? this.generateMetadata(title) : '';
    
    return `<!DOCTYPE html>
<html lang="en" data-theme="${theme}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${this.escapeHtml(title)}</title>
  ${metadata}
  ${styles}
</head>
<body>
  <main class="exported-content">
    ${content}
  </main>
  <footer class="export-footer">
    <p>Generated by aiNote - ${new Date().toISOString()}</p>
  </footer>
</body>
</html>`;
  }

  /**
   * Generate CSS styles for export
   * @param {string} theme - Theme (light/dark)
   * @returns {string} CSS styles
   */
  generateExportStyles(theme) {
    return `<style>
/* Export styles for aiNote markdown preview */
:root {
  --text-color: ${theme === 'dark' ? '#e4e4e4' : '#333333'};
  --background-color: ${theme === 'dark' ? '#1a1a1a' : '#ffffff'};
  --border-color: ${theme === 'dark' ? '#404040' : '#e1e1e1'};
  --code-background: ${theme === 'dark' ? '#2d2d2d' : '#f5f5f5'};
  --link-color: ${theme === 'dark' ? '#58a6ff' : '#0066cc'};
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  line-height: 1.6;
  color: var(--text-color);
  background-color: var(--background-color);
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
}

.exported-content {
  margin-bottom: 3rem;
}

/* Typography */
h1, h2, h3, h4, h5, h6 {
  margin-top: 2rem;
  margin-bottom: 1rem;
  font-weight: 600;
}

h1 { font-size: 2.25rem; }
h2 { font-size: 1.875rem; }
h3 { font-size: 1.5rem; }
h4 { font-size: 1.25rem; }
h5 { font-size: 1.125rem; }
h6 { font-size: 1rem; }

/* Links */
a {
  color: var(--link-color);
  text-decoration: underline;
}

a:hover {
  text-decoration: none;
}

/* Code */
code {
  background-color: var(--code-background);
  padding: 0.125rem 0.25rem;
  border-radius: 0.25rem;
  font-family: 'SFMono-Regular', Monaco, 'Cascadia Code', monospace;
  font-size: 0.875rem;
}

pre {
  background-color: var(--code-background);
  padding: 1rem;
  border-radius: 0.5rem;
  overflow-x: auto;
  margin: 1rem 0;
}

pre code {
  background: none;
  padding: 0;
}

/* Tables */
.table-wrapper {
  overflow-x: auto;
  margin: 1rem 0;
}

table {
  width: 100%;
  border-collapse: collapse;
  border: 1px solid var(--border-color);
}

th, td {
  padding: 0.75rem;
  text-align: left;
  border-bottom: 1px solid var(--border-color);
}

th {
  background-color: var(--code-background);
  font-weight: 600;
}

/* Images */
.preview-image {
  max-width: 100%;
  height: auto;
  border-radius: 0.5rem;
  margin: 1rem 0;
}

.image-error {
  background-color: var(--code-background);
  border: 1px solid var(--border-color);
  border-radius: 0.5rem;
  padding: 1rem;
  text-align: center;
  margin: 1rem 0;
}

/* Footer */
.export-footer {
  border-top: 1px solid var(--border-color);
  padding-top: 1rem;
  margin-top: 2rem;
  font-size: 0.875rem;
  color: #666;
  text-align: center;
}

/* Print styles */
@media print {
  body {
    max-width: none;
    margin: 0;
    padding: 1rem;
  }
  
  .export-footer {
    page-break-inside: avoid;
  }
}
</style>`;
  }

  /**
   * Generate metadata for export
   * @param {string} title - Document title
   * @returns {string} HTML metadata
   */
  generateMetadata(title) {
    const now = new Date().toISOString();
    
    return `<meta name="generator" content="aiNote">
  <meta name="created" content="${now}">
  <meta name="description" content="Exported markdown document from aiNote">
  <meta property="og:title" content="${this.escapeHtml(title)}">
  <meta property="og:type" content="article">
  <meta property="og:description" content="Exported markdown document from aiNote">`;
  }

  /**
   * Enable real-time preview updates
   * @param {MarkdownEditor} editorInstance - Editor instance for content monitoring
   * @param {HTMLElement} editorScrollElement - Editor scroll container for sync
   */
  enableRealTimeUpdates(editorInstance, editorScrollElement = null) {
    if (!editorInstance) {
      console.error('❌ Editor instance required for real-time updates');
      return;
    }
    
    this.realTimeEnabled = true;
    this.editorInstance = editorInstance;
    this.scrollSync.editorScrollElement = editorScrollElement;
    
    // Listen for content changes from editor
    this.editorInstance.addEventListener('content_changed', (event) => {
      this.handleRealTimeContentChange(event.detail.content);
    });
    
    // Setup scroll synchronization if editor scroll element provided
    if (editorScrollElement) {
      this.setupScrollSynchronization();
    }
    
  }
  
  /**
   * Disable real-time preview updates
   */
  disableRealTimeUpdates() {
    this.realTimeEnabled = false;
    this.editorInstance = null;
    this.scrollSync.editorScrollElement = null;
    
    // Clear any pending updates
    if (this.updateDebounceTimeout) {
      clearTimeout(this.updateDebounceTimeout);
      this.updateDebounceTimeout = null;
    }
    
    console.log('🛑 Real-time preview updates disabled');
  }
  
  /**
   * Handle real-time content changes from editor with debouncing
   * Target: <200ms after typing stops
   * @param {string} content - New markdown content
   */
  handleRealTimeContentChange(content) {
    if (!this.realTimeEnabled || this.pendingUpdate) {
      return;
    }
    
    // Check if content actually changed using hash comparison
    const contentHash = this.generateContentHash(content);
    if (contentHash === this.lastContentHash) {
      return;
    }
    
    this.lastContentHash = contentHash;
    
    // Clear existing debounce timeout
    if (this.updateDebounceTimeout) {
      clearTimeout(this.updateDebounceTimeout);
    }
    
    // Debounce the update
    this.updateDebounceTimeout = setTimeout(() => {
      this.performRealTimeUpdate(content);
    }, this.updateDebounceDelay);
  }
  
  /**
   * Perform real-time preview update
   * Target: <50ms for incremental updates
   * @param {string} content - Markdown content to render
   */
  async performRealTimeUpdate(content) {
    if (this.pendingUpdate) {
      return;
    }
    
    this.pendingUpdate = true;
    const startTime = performance.now();
    
    try {
      // Use incremental parsing for large documents
      if (this.shouldUseIncrementalParsing(content)) {
        await this.performIncrementalUpdate(content);
      } else {
        await this.render(content);
      }
      
      const updateTime = performance.now() - startTime;
      this.renderStats.realTimeUpdates++;
      
      // Log performance warning if target exceeded
      if (updateTime > 50) {
        console.warn(`⚠️ Real-time update exceeded target: ${updateTime.toFixed(2)}ms (target: <50ms)`);
      }
      
      // Update memory monitoring
      this.checkMemoryUsage();
      
    } catch (error) {
      console.error('❌ Real-time update failed:', error);
    } finally {
      this.pendingUpdate = false;
      this.updateDebounceTimeout = null;
    }
  }
  
  /**
   * Determine if incremental parsing should be used
   * @param {string} content - Content to check
   * @returns {boolean} Whether to use incremental parsing
   */
  shouldUseIncrementalParsing(content) {
    const lines = content.split('\n');
    return lines.length > 1000 || content.length > 50000; // Large document threshold
  }
  
  /**
   * Perform incremental update for large documents
   * @param {string} content - Full markdown content
   */
  async performIncrementalUpdate(content) {
    const startTime = performance.now();
    const lines = content.split('\n');
    
    // Detect changed sections
    const changedSections = this.detectChangedSections(lines);
    
    if (changedSections.length === 0) {
      return; // No changes detected
    }
    
    // Parse and update only changed sections
    for (const section of changedSections) {
      await this.updateSection(section, lines);
    }
    
    this.incremental.contentLines = lines;
    this.renderStats.incrementalUpdates++;
    
    const updateTime = performance.now() - startTime;
  }
  
  /**
   * Detect changed sections in the content
   * @param {string[]} newLines - New content lines
   * @returns {Array} Array of changed section objects
   */
  detectChangedSections(newLines) {
    const oldLines = this.incremental.contentLines;
    const sections = [];
    
    if (!oldLines.length) {
      // First time - consider entire document as changed
      return [{ start: 0, end: newLines.length - 1, type: 'full' }];
    }
    
    let sectionStart = null;
    const maxLength = Math.max(oldLines.length, newLines.length);
    
    for (let i = 0; i < maxLength; i++) {
      const oldLine = oldLines[i] || '';
      const newLine = newLines[i] || '';
      
      if (oldLine !== newLine) {
        if (sectionStart === null) {
          sectionStart = i;
        }
      } else if (sectionStart !== null) {
        // End of changed section
        sections.push({ start: sectionStart, end: i - 1, type: 'partial' });
        sectionStart = null;
      }
    }
    
    // Handle case where changes go to the end
    if (sectionStart !== null) {
      sections.push({ start: sectionStart, end: newLines.length - 1, type: 'partial' });
    }
    
    return sections;
  }
  
  /**
   * Update a specific section of the content
   * @param {Object} section - Section to update
   * @param {string[]} lines - All content lines
   */
  async updateSection(section, lines) {
    const sectionContent = lines.slice(section.start, section.end + 1).join('\n');
    
    // Import parser and render section
    const { default: MarkdownParser } = await import('../utils/markdown-parser.js');
    const parser = new MarkdownParser();
    const sectionHtml = parser.parse(sectionContent);
    
    // Update specific DOM section if possible, otherwise fall back to full update
    // For now, use full update - future optimization can target specific DOM elements
    const fullContent = lines.join('\n');
    const fullHtml = parser.parse(fullContent);
    
    await this.updatePreview(fullHtml, fullContent);
    
    // Cache processed section
    this.incremental.processedSections.set(
      `${section.start}-${section.end}`, 
      { html: sectionHtml, content: sectionContent }
    );
  }
  
  /**
   * Setup scroll synchronization between editor and preview
   */
  setupScrollSynchronization() {
    if (!this.scrollSync.editorScrollElement || !this.elements.scrollContainer) {
      return;
    }
    
    this.scrollSync.enabled = true;
    
    // Listen to editor scroll events
    this.scrollSync.editorScrollElement.addEventListener('scroll', 
      this.debounce(() => this.handleEditorScroll(), 50)
    );
    
    // Listen to preview scroll events
    this.elements.scrollContainer.addEventListener('scroll', 
      this.debounce(() => this.handlePreviewScroll(), 50)
    );
    
  }
  
  /**
   * Handle editor scroll events for synchronization
   */
  handleEditorScroll() {
    if (!this.scrollSync.enabled || this.scrollSync.syncInProgress) {
      return;
    }
    
    this.scrollSync.syncInProgress = true;
    
    const editorElement = this.scrollSync.editorScrollElement;
    const previewElement = this.elements.scrollContainer;
    
    // Calculate scroll ratio in editor
    const editorScrollTop = editorElement.scrollTop;
    const editorScrollHeight = editorElement.scrollHeight - editorElement.clientHeight;
    const editorScrollRatio = editorScrollHeight > 0 ? editorScrollTop / editorScrollHeight : 0;
    
    // Apply to preview with tolerance check
    const previewScrollHeight = previewElement.scrollHeight - previewElement.clientHeight;
    const targetScrollTop = editorScrollRatio * previewScrollHeight;
    
    if (Math.abs(previewElement.scrollTop - targetScrollTop) > this.scrollSync.tolerance) {
      previewElement.scrollTop = targetScrollTop;
    }
    
    this.scrollSync.editorScrollRatio = editorScrollRatio;
    
    setTimeout(() => {
      this.scrollSync.syncInProgress = false;
    }, 100);
  }
  
  /**
   * Handle preview scroll events for synchronization
   */
  handlePreviewScroll() {
    if (!this.scrollSync.enabled || this.scrollSync.syncInProgress) {
      return;
    }
    
    this.scrollSync.syncInProgress = true;
    
    const editorElement = this.scrollSync.editorScrollElement;
    const previewElement = this.elements.scrollContainer;
    
    // Calculate scroll ratio in preview
    const previewScrollTop = previewElement.scrollTop;
    const previewScrollHeight = previewElement.scrollHeight - previewElement.clientHeight;
    const previewScrollRatio = previewScrollHeight > 0 ? previewScrollTop / previewScrollHeight : 0;
    
    // Apply to editor with tolerance check
    const editorScrollHeight = editorElement.scrollHeight - editorElement.clientHeight;
    const targetScrollTop = previewScrollRatio * editorScrollHeight;
    
    if (Math.abs(editorElement.scrollTop - targetScrollTop) > this.scrollSync.tolerance) {
      editorElement.scrollTop = targetScrollTop;
    }
    
    this.scrollSync.previewScrollRatio = previewScrollRatio;
    
    setTimeout(() => {
      this.scrollSync.syncInProgress = false;
    }, 100);
  }
  
  /**
   * Initialize memory monitoring for performance optimization
   */
  initializeMemoryMonitoring() {
    // Get baseline memory usage
    if (performance.memory) {
      this.memoryMonitor.baselineMemory = performance.memory.usedJSHeapSize;
    }
    
    // Start periodic memory monitoring
    setInterval(() => {
      this.checkMemoryUsage();
    }, this.memoryMonitor.checkInterval);
    
    console.log('📊 Memory monitoring initialized');
  }
  
  /**
   * Check current memory usage and perform cleanup if needed
   * Target: <1MB per hour of continuous editing
   */
  checkMemoryUsage() {
    if (!performance.memory) {
      return;
    }
    
    const currentMemory = performance.memory.usedJSHeapSize;
    const memoryGrowth = currentMemory - this.memoryMonitor.baselineMemory;
    
    // Update render stats
    this.renderStats.memoryUsage = currentMemory;
    this.renderStats.memoryGrowthRate = memoryGrowth;
    
    // Check if cleanup is needed
    if (memoryGrowth > this.memoryMonitor.cleanupThreshold) {
      this.performMemoryCleanup();
    }
    
    // Log memory usage for monitoring
    const memoryMB = (currentMemory / 1024 / 1024).toFixed(2);
    const growthMB = (memoryGrowth / 1024 / 1024).toFixed(2);
    
    if (memoryGrowth > this.memoryMonitor.maxMemoryGrowth) {
      console.warn(`⚠️ Memory growth exceeding target: ${growthMB}MB (target: <1MB/hour)`);
    }
    
    this.memoryMonitor.lastCheck = Date.now();
  }
  
  /**
   * Perform memory cleanup operations
   */
  performMemoryCleanup() {
    console.log('🧹 Performing memory cleanup...');
    
    // Clear processed sections cache
    this.incremental.processedSections.clear();
    
    // Clear virtual DOM cache
    this.virtualDom.elements.clear();
    this.virtualDom.lastSnapshot = null;
    
    // Force garbage collection if available (development only)
    if (window.gc && typeof window.gc === 'function') {
      window.gc();
    }
    
    // Update baseline after cleanup
    if (performance.memory) {
      this.memoryMonitor.baselineMemory = performance.memory.usedJSHeapSize;
    }
    
  }
  
  /**
   * Generate hash for content change detection
   * @param {string} content - Content to hash
   * @returns {string} Simple hash of content
   */
  generateContentHash(content) {
    let hash = 0;
    if (content.length === 0) return hash.toString();
    
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    
    return hash.toString();
  }
  
  /**
   * Get performance statistics for monitoring
   * @returns {Object} Performance statistics
   */
  getPerformanceStats() {
    const stats = { ...this.renderStats };
    
    if (performance.memory) {
      stats.currentMemoryMB = (performance.memory.usedJSHeapSize / 1024 / 1024).toFixed(2);
      stats.memoryGrowthMB = (stats.memoryGrowthRate / 1024 / 1024).toFixed(2);
    }
    
    stats.realTimeEnabled = this.realTimeEnabled;
    stats.scrollSyncEnabled = this.scrollSync.enabled;
    stats.incrementalEnabled = this.incremental.enabled;
    
    return stats;
  }

  /**
   * Cleanup method for removing event listeners and clearing memory
   */
  destroy() {
    // Disable real-time updates
    this.disableRealTimeUpdates();
    
    // Remove event listeners
    if (this.elements.scrollContainer) {
      this.elements.scrollContainer.removeEventListener('scroll', this.handleScroll);
    }
    
    window.removeEventListener('resize', this.handleResize);
    
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.removeListener(this.handleThemeChange);
    
    // Clean up link event listeners
    if (this.elements.content) {
      const links = this.elements.content.querySelectorAll('a');
      links.forEach(link => {
        link.removeEventListener('click', this.validateExternalLink);
        link.removeEventListener('click', this.handleInternalLink);
        link.removeEventListener('click', this.scrollToAnchor);
      });
      
      // Clean up table event listeners
      const tables = this.elements.content.querySelectorAll('table');
      tables.forEach(table => {
        table.removeEventListener('keydown', this.handleTableNavigation);
      });
    }
    
    // Perform final memory cleanup
    this.performMemoryCleanup();
    
    // Clear DOM elements
    this.clear();
    
    // Clear references
    this.container = null;
    this.elements = {};
    this.virtualDom = { elements: new Map(), lastSnapshot: null };
    this.editorInstance = null;
    this.scrollSync = { enabled: false };
    
    console.log('🗑️ PreviewRenderer destroyed and memory cleared');
  }
}

export default PreviewRenderer;