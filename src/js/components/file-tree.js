/**
 * FileTree - Hierarchical file tree component for aiNote vault navigation
 * 
 * Features:
 * - Hierarchical display of folders and markdown files
 * - Expandable/collapsible folder functionality  
 * - Click navigation to open files in editor
 * - Visual highlighting of currently selected file
 * - Accessibility support with ARIA labels
 * - Event delegation for performance
 * - Lightweight vanilla JavaScript implementation
 * 
 * @class FileTree
 */
class FileTree {
  /**
   * File tree events for communication with application
   */
  static EVENTS = {
    FILE_SELECTED: 'file_selected',
    FOLDER_EXPANDED: 'folder_expanded',
    FOLDER_COLLAPSED: 'folder_collapsed',
    TREE_UPDATED: 'tree_updated',
    DRAG_START: 'drag_start',
    DRAG_END: 'drag_end',
    FILE_MOVE_REQUESTED: 'file_move_requested'
  };

  /**
   * CSS classes for tree elements
   */
  static CSS_CLASSES = {
    TREE_CONTAINER: 'file-tree-container',
    TREE_ITEM: 'tree-item',
    TREE_FOLDER: 'tree-folder',
    TREE_FILE: 'tree-file',
    TREE_ICON: 'tree-icon',
    TREE_NAME: 'tree-name',
    TREE_CHILDREN: 'tree-children',
    EXPANDED: 'expanded',
    COLLAPSED: 'collapsed',
    SELECTED: 'selected',
    INDENTED: 'indented'
  };

  /**
   * Initialize FileTree component
   * @param {HTMLElement} container - DOM container for the file tree
   * @param {AppState} appState - Application state instance
   */
  constructor(container, appState) {
    if (!container || !(container instanceof HTMLElement)) {
      throw new Error('FileTree requires a valid DOM container element');
    }

    if (!appState) {
      throw new Error('FileTree requires an AppState instance');
    }

    this.container = container;
    this.appState = appState;
    
    // Component state
    this.files = [];
    this.expandedFolders = new Set();
    this.selectedFile = null;
    this.treeStructure = new Map(); // Optimized hierarchical structure
    
    // Search and filtering state
    this.filteredFiles = null;
    this.searchInput = null;
    this.searchDebounceTimer = null;
    this.isSearchActive = false;
    
    // Virtual scrolling state
    this.isVirtualScrolling = false;
    this.virtualScrollOffset = 0;
    this.visibleItemsCount = 20; // Items visible in viewport
    this.intersectionObserver = null;
    
    // Performance monitoring
    this.performanceMetrics = {
      lastRenderTime: 0,
      renderCount: 0,
      averageRenderTime: 0
    };
    
    // Event listeners registry for cleanup
    this.eventListeners = new Map();
    
    // Initialize component
    this.initialize();
  }

  /**
   * Initialize the file tree component
   */
  initialize() {
    // Set up container
    this.container.className = FileTree.CSS_CLASSES.TREE_CONTAINER;
    this.container.setAttribute('role', 'tree');
    this.container.setAttribute('aria-label', 'File tree navigation');
    
    // Create search input container
    this.createSearchInterface();
    
    // Set up event delegation for performance
    this.setupEventDelegation();
    
    // Set up drag and drop functionality
    this.setupDragAndDrop();
    
    // Listen to app state changes
    this.setupStateListeners();
    
    // Set up virtual scrolling if needed
    this.setupVirtualScrolling();
    
    // Initial empty state
    this.showEmptyState();
  }

  /**
   * Set up event delegation for tree interactions
   */
  setupEventDelegation() {
    // Click handler for file/folder selection
    const clickHandler = (event) => this.handleClick(event);
    this.container.addEventListener('click', clickHandler);
    this.eventListeners.set('click', clickHandler);

    // Keyboard navigation handler
    const keyHandler = (event) => this.handleKeyboard(event);
    this.container.addEventListener('keydown', keyHandler);
    this.eventListeners.set('keydown', keyHandler);
  }

  /**
   * Set up listeners for application state changes
   */
  setupStateListeners() {
    // Listen for file list updates
    this.appState.addEventListener('files_updated', (data) => {
      this.render(data.files);
    });

    // Listen for current file changes to update selection
    this.appState.addEventListener('file_changed', (data) => {
      this.selectFile(data.file);
    });
  }

  /**
   * Render the file tree with provided files
   * @param {Array} files - Array of file objects from vault scan
   */
  render(files) {
    if (!Array.isArray(files)) {
      console.warn('FileTree.render: files must be an array');
      this.showEmptyState();
      return;
    }

    this.files = files;
    
    if (files.length === 0) {
      this.showEmptyState();
      return;
    }

    // Check if virtual scrolling is needed based on file count
    this.checkVirtualScrollingNeeded();

    // Build hierarchical structure for performance
    this.buildTreeStructure(files);
    
    // Clear container but preserve search interface
    const searchContainer = this.container.querySelector('.file-tree-search-container');
    this.container.innerHTML = '';
    if (searchContainer) {
      this.container.appendChild(searchContainer);
    }
    
    // Render tree with performance optimizations
    const startTime = performance.now();
    this.renderTreeLevel(this.getRootItems(), this.container, 0);
    const renderTime = performance.now() - startTime;
    
    // Update performance metrics
    this.updatePerformanceMetrics(renderTime);
    
    // Performance logging for large trees
    if (files.length > 1000) {
      console.debug(`FileTree: Rendered ${files.length} files in ${renderTime.toFixed(2)}ms (virtual: ${this.isVirtualScrolling})`);
    }
    
    // Run performance optimization checks
    this.performanceOptimization();
    
    // Emit tree updated event
    this.emit(FileTree.EVENTS.TREE_UPDATED, { 
      files: this.files, 
      count: files.length,
      renderTime: renderTime,
      isVirtualScrolling: this.isVirtualScrolling
    });
  }

  /**
   * Build optimized hierarchical tree structure
   * @param {Array} files - Flat array of file objects
   */
  buildTreeStructure(files) {
    this.treeStructure.clear();
    
    const vaultPath = this.appState.getState().currentVault || '';
    
    // Sort files: directories first, then by name
    const sortedFiles = [...files].sort((a, b) => {
      // Directories first
      if (a.is_dir && !b.is_dir) return -1;
      if (!a.is_dir && b.is_dir) return 1;
      
      // Then alphabetically by name
      return a.name.localeCompare(b.name, undefined, { 
        numeric: true, 
        sensitivity: 'base' 
      });
    });

    // Build parent-child relationships
    sortedFiles.forEach(file => {
      // Get relative path from vault root
      let relativePath = file.path;
      if (vaultPath && file.path.startsWith(vaultPath)) {
        relativePath = file.path.substring(vaultPath.length).replace(/^\/+/, '');
      }
      
      const parentPath = this.getParentPath(relativePath);
      
      if (!this.treeStructure.has(parentPath)) {
        this.treeStructure.set(parentPath, []);
      }
      
      this.treeStructure.get(parentPath).push(file);
    });
  }

  /**
   * Get parent directory path from file path
   * @param {string} filePath - Full file path
   * @returns {string} Parent directory path
   */
  getParentPath(filePath) {
    const segments = filePath.split('/');
    return segments.slice(0, -1).join('/');
  }

  /**
   * Get root level items (files/folders at vault root)
   * @returns {Array} Root level file objects
   */
  getRootItems() {
    // Root items are stored with empty string as key (vault root)
    return this.treeStructure.get('') || [];
  }

  /**
   * Get children of a folder
   * @param {string} folderPath - Path to the folder
   * @returns {Array} Child file objects
   */
  getFolderChildren(folderPath) {
    const vaultPath = this.appState.getState().currentVault || '';
    
    // Convert absolute path to relative path for lookup
    const relativePath = folderPath.startsWith(vaultPath) 
      ? folderPath.substring(vaultPath.length).replace(/^\/+/, '')
      : folderPath;
    
    return this.treeStructure.get(relativePath) || [];
  }

  /**
   * Render a tree level (recursive)
   * @param {Array} items - Items to render at this level
   * @param {HTMLElement} container - Container element
   * @param {number} depth - Current nesting depth for indentation
   */
  renderTreeLevel(items, container, depth) {
    items.forEach(item => {
      const itemElement = this.createTreeItem(item, depth);
      container.appendChild(itemElement);
      
      // If it's a folder, create children container AFTER the parent item
      if (item.is_dir) {
        const childrenContainer = this.createChildrenContainer();
        container.appendChild(childrenContainer); // Append to same container as parent
        
        // Store reference to children container on the item element
        itemElement.childrenContainer = childrenContainer;
        
        // Only render children if folder is expanded
        if (this.expandedFolders.has(item.path)) {
          const children = this.getFolderChildren(item.path);
          this.renderTreeLevel(children, childrenContainer, depth + 1);
          itemElement.classList.add(FileTree.CSS_CLASSES.EXPANDED);
          childrenContainer.style.display = 'block';
        } else {
          itemElement.classList.add(FileTree.CSS_CLASSES.COLLAPSED);
          childrenContainer.style.display = 'none';
        }
      }
    });
  }

  /**
   * Create a tree item element - VSCode Style
   * @param {Object} file - File object
   * @param {number} depth - Nesting depth
   * @returns {HTMLElement} Tree item element
   */
  createTreeItem(file, depth) {
    const item = document.createElement('div');
    item.className = `${FileTree.CSS_CLASSES.TREE_ITEM} ${file.is_dir ? FileTree.CSS_CLASSES.TREE_FOLDER : FileTree.CSS_CLASSES.TREE_FILE}`;
    
    // Add proper indentation based on depth
    // Base padding: 8px, each level adds 16px
    const indentationPx = 8 + (depth * 16);
    item.style.paddingLeft = `${indentationPx}px`;
    
    if (depth > 0) {
      item.classList.add(FileTree.CSS_CLASSES.INDENTED);
    }
    
    // Store file data and depth
    item.dataset.filePath = file.path;
    item.dataset.isDir = file.is_dir.toString();
    item.dataset.depth = depth.toString();
    
    // Accessibility attributes
    item.setAttribute('role', 'treeitem');
    item.setAttribute('aria-label', `${file.is_dir ? 'Folder' : 'File'}: ${file.name}`);
    item.setAttribute('tabindex', '0');
    
    // Enable drag and drop
    item.setAttribute('draggable', 'true');
    
    if (file.is_dir) {
      const isExpanded = this.expandedFolders.has(file.path);
      item.setAttribute('aria-expanded', isExpanded.toString());
    }
    
    // Create item content
    const icon = this.createIcon(file);
    const name = this.createNameElement(file);
    
    item.appendChild(icon);
    item.appendChild(name);
    
    // Mark as selected if it's the current file
    if (!file.is_dir && file.path === this.selectedFile) {
      item.classList.add(FileTree.CSS_CLASSES.SELECTED);
      item.setAttribute('aria-selected', 'true');
    }
    
    return item;
  }

  /**
   * Create icon element for file/folder - VSCode Style
   * @param {Object} file - File object
   * @returns {HTMLElement} Icon element
   */
  createIcon(file) {
    const icon = document.createElement('span');
    icon.className = FileTree.CSS_CLASSES.TREE_ICON;
    icon.setAttribute('aria-hidden', 'true');
    
    if (file.is_dir) {
      const isExpanded = this.expandedFolders.has(file.path);
      icon.classList.add('folder');
      icon.classList.add(isExpanded ? 'folder-expanded' : 'folder-collapsed');
    } else {
      const iconClass = this.getFileIconClass(file.name);
      icon.classList.add(iconClass);
    }
    
    return icon;
  }

  /**
   * Create name element for file/folder
   * @param {Object} file - File object
   * @returns {HTMLElement} Name element
   */
  createNameElement(file) {
    const name = document.createElement('span');
    name.className = FileTree.CSS_CLASSES.TREE_NAME;
    name.textContent = file.name;
    name.title = file.path; // Full path on hover
    
    return name;
  }

  /**
   * Create children container for folders
   * @returns {HTMLElement} Children container
   */
  createChildrenContainer() {
    const container = document.createElement('div');
    container.className = FileTree.CSS_CLASSES.TREE_CHILDREN;
    container.setAttribute('role', 'group');
    container.style.display = 'none'; // Initially hidden
    
    return container;
  }

  /**
   * Get appropriate CSS class for file type icon - VSCode Style
   * @param {string} fileName - Name of the file
   * @returns {string} CSS class name
   */
  getFileIconClass(fileName) {
    const ext = fileName.split('.').pop().toLowerCase();
    const iconClasses = {
      'md': 'file-md',
      'txt': 'file-default',
      'js': 'file-js',
      'ts': 'file-js',
      'html': 'file-html',
      'css': 'file-css',
      'json': 'file-default',
      'py': 'file-default',
      'rs': 'file-default',
      'go': 'file-default',
      'jpg': 'file-default',
      'jpeg': 'file-default',
      'png': 'file-default',
      'gif': 'file-default',
      'svg': 'file-default',
      'pdf': 'file-default'
    };
    return iconClasses[ext] || 'file-default';
  }

  /**
   * Handle click events on tree items
   * @param {Event} event - Click event
   */
  handleClick(event) {
    const treeItem = event.target.closest(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
    if (!treeItem) return;
    
    event.preventDefault();
    event.stopPropagation();
    
    const filePath = treeItem.dataset.filePath;
    const isDir = treeItem.dataset.isDir === 'true';
    
    if (isDir) {
      this.toggleFolder(filePath);
    } else {
      this.selectFile(filePath);
      this.emit(FileTree.EVENTS.FILE_SELECTED, { filePath });
    }
  }

  /**
   * Handle keyboard navigation with advanced features
   * @param {Event} event - Keyboard event
   */
  handleKeyboard(event) {
    const treeItem = event.target.closest(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
    if (!treeItem) return;

    const filePath = treeItem.dataset.filePath;
    const isDir = treeItem.dataset.isDir === 'true';

    switch (event.key) {
      case 'Enter':
      case ' ':
        event.preventDefault();
        if (isDir) {
          this.toggleFolder(filePath);
        } else {
          this.selectFile(filePath);
          this.emit(FileTree.EVENTS.FILE_SELECTED, { filePath });
        }
        break;
        
      case 'ArrowRight':
        event.preventDefault();
        if (isDir) {
          if (!this.expandedFolders.has(filePath)) {
            this.expandFolder(filePath);
          } else {
            // Move to first child if folder is expanded
            this.navigateToFirstChild(treeItem);
          }
        }
        break;
        
      case 'ArrowLeft':
        event.preventDefault();
        if (isDir && this.expandedFolders.has(filePath)) {
          this.collapseFolder(filePath);
        } else {
          // Move to parent
          this.navigateToParent(treeItem);
        }
        break;
        
      case 'ArrowUp':
        event.preventDefault();
        this.navigateToPrevious(treeItem);
        break;
        
      case 'ArrowDown':
        event.preventDefault();
        this.navigateToNext(treeItem);
        break;
        
      case 'Home':
        event.preventDefault();
        this.navigateToFirst();
        break;
        
      case 'End':
        event.preventDefault();
        this.navigateToLast();
        break;
        
      case 'PageUp':
        event.preventDefault();
        this.navigateByPage(-1);
        break;
        
      case 'PageDown':
        event.preventDefault();
        this.navigateByPage(1);
        break;
        
      // Quick search/filter activation
      case 'f':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault();
          this.activateSearch();
        }
        break;
        
      case 'Escape':
        if (this.searchInput && this.searchInput.style.display !== 'none') {
          event.preventDefault();
          this.deactivateSearch();
        }
        break;
    }
  }

  /**
   * Toggle folder expand/collapse state
   * @param {string} folderPath - Path to the folder
   */
  toggleFolder(folderPath) {
    if (this.expandedFolders.has(folderPath)) {
      this.collapseFolder(folderPath);
    } else {
      this.expandFolder(folderPath);
    }
  }

  /**
   * Expand a folder - VSCode Style
   * @param {string} folderPath - Path to the folder
   */
  expandFolder(folderPath) {
    if (this.expandedFolders.has(folderPath)) return;
    
    this.expandedFolders.add(folderPath);
    
    // Update DOM
    const folderElement = this.findTreeItem(folderPath);
    if (folderElement) {
      folderElement.classList.remove(FileTree.CSS_CLASSES.COLLAPSED);
      folderElement.classList.add(FileTree.CSS_CLASSES.EXPANDED);
      folderElement.setAttribute('aria-expanded', 'true');
      
      // Update icon classes
      const icon = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_ICON}`);
      if (icon) {
        icon.classList.remove('folder-collapsed');
        icon.classList.add('folder-expanded');
      }
      
      // Show children using stored reference
      const childrenContainer = folderElement.childrenContainer;
      if (childrenContainer) {
        childrenContainer.style.display = 'block';
        
        // Render children if not already rendered
        if (childrenContainer.children.length === 0) {
          const children = this.getFolderChildren(folderPath);
          const depth = this.calculateDepth(folderPath);
          this.renderTreeLevel(children, childrenContainer, depth + 1);
        }
      }
    }
    
    this.emit(FileTree.EVENTS.FOLDER_EXPANDED, { folderPath });
  }

  /**
   * Collapse a folder - VSCode Style
   * @param {string} folderPath - Path to the folder
   */
  collapseFolder(folderPath) {
    if (!this.expandedFolders.has(folderPath)) return;
    
    this.expandedFolders.delete(folderPath);
    
    // Update DOM
    const folderElement = this.findTreeItem(folderPath);
    if (folderElement) {
      folderElement.classList.remove(FileTree.CSS_CLASSES.EXPANDED);
      folderElement.classList.add(FileTree.CSS_CLASSES.COLLAPSED);
      folderElement.setAttribute('aria-expanded', 'false');
      
      // Update icon classes
      const icon = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_ICON}`);
      if (icon) {
        icon.classList.remove('folder-expanded');
        icon.classList.add('folder-collapsed');
      }
      
      // Hide children using stored reference
      const childrenContainer = folderElement.childrenContainer;
      if (childrenContainer) {
        childrenContainer.style.display = 'none';
      }
    }
    
    this.emit(FileTree.EVENTS.FOLDER_COLLAPSED, { folderPath });
  }

  /**
   * Select a file and update visual state
   * @param {string} filePath - Path to the file to select
   */
  selectFile(filePath) {
    // Clear previous selection
    const previousSelected = this.container.querySelector(`.${FileTree.CSS_CLASSES.SELECTED}`);
    if (previousSelected) {
      previousSelected.classList.remove(FileTree.CSS_CLASSES.SELECTED);
      previousSelected.removeAttribute('aria-selected');
    }
    
    this.selectedFile = filePath;
    
    // Highlight new selection
    if (filePath) {
      const fileElement = this.findTreeItem(filePath);
      if (fileElement) {
        fileElement.classList.add(FileTree.CSS_CLASSES.SELECTED);
        fileElement.setAttribute('aria-selected', 'true');
        
        // Ensure parent folders are expanded
        this.ensurePathVisible(filePath);
      }
    }
  }

  /**
   * Ensure a file path is visible by expanding parent folders
   * @param {string} filePath - Path to make visible
   */
  ensurePathVisible(filePath) {
    const segments = filePath.split('/');
    let currentPath = '';
    
    for (let i = 0; i < segments.length - 1; i++) {
      currentPath = i === 0 ? segments[i] : `${currentPath}/${segments[i]}`;
      
      // Check if this is a folder that needs expanding
      const folder = this.files.find(f => f.path === currentPath && f.is_dir);
      if (folder && !this.expandedFolders.has(currentPath)) {
        this.expandFolder(currentPath);
      }
    }
  }

  /**
   * Find tree item element by file path
   * @param {string} filePath - Path to find
   * @returns {HTMLElement|null} Tree item element
   */
  findTreeItem(filePath) {
    return this.container.querySelector(`[data-file-path="${filePath}"]`);
  }

  /**
   * Calculate depth of a path relative to vault root
   * @param {string} filePath - File path
   * @returns {number} Depth level
   */
  calculateDepth(filePath) {
    const vaultPath = this.appState.getState().currentVault || '';
    const relativePath = filePath.replace(vaultPath, '').replace(/^\/+/, '');
    return relativePath ? relativePath.split('/').length - 1 : 0;
  }

  /**
   * Show empty state when no files are available
   */
  showEmptyState() {
    // Clear container but preserve search interface
    const searchContainer = this.container.querySelector('.file-tree-search-container');
    this.container.innerHTML = `
      <div class="file-tree-empty-state">
        <p>No files found in vault</p>
        <button type="button" class="btn-secondary" onclick="window.refreshVault()">
          Refresh Vault
        </button>
      </div>
    `;
    // Re-add search interface if it existed
    if (searchContainer) {
      this.container.insertBefore(searchContainer, this.container.firstChild);
    }
  }

  /**
   * Refresh the file tree with current vault files
   */
  async refresh() {
    const currentVault = this.appState.getState().currentVault;
    if (!currentVault) {
      this.showEmptyState();
      return;
    }
    
    try {
      // Use existing refresh function from main app
      if (window.refreshVault) {
        await window.refreshVault();
      }
    } catch (error) {
      console.error('Failed to refresh file tree:', error);
    }
  }

  /**
   * Emit custom events for component communication
   * @param {string} event - Event name
   * @param {Object} data - Event data
   */
  emit(event, data = {}) {
    const customEvent = new CustomEvent(event, { 
      detail: data,
      bubbles: true 
    });
    this.container.dispatchEvent(customEvent);
  }

  /**
   * Navigate to the first child of an expanded folder
   * @param {HTMLElement} folderItem - The folder tree item
   */
  navigateToFirstChild(folderItem) {
    if (folderItem.childrenContainer) {
      const firstChild = folderItem.childrenContainer.querySelector(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
      if (firstChild) {
        this.focusTreeItem(firstChild);
      }
    }
  }

  /**
   * Navigate to the parent of the current item
   * @param {HTMLElement} treeItem - Current tree item
   */
  navigateToParent(treeItem) {
    const depth = parseInt(treeItem.dataset.depth || '0');
    if (depth === 0) return; // Already at root level
    
    const filePath = treeItem.dataset.filePath;
    const parentPath = this.getParentPath(this.getRelativePath(filePath));
    const vaultPath = this.appState.getState().currentVault || '';
    const fullParentPath = vaultPath ? `${vaultPath}/${parentPath}` : parentPath;
    
    const parentItem = this.findTreeItem(fullParentPath);
    if (parentItem) {
      this.focusTreeItem(parentItem);
    }
  }

  /**
   * Navigate to the previous visible tree item
   * @param {HTMLElement} currentItem - Current tree item
   */
  navigateToPrevious(currentItem) {
    const visibleItems = this.getVisibleTreeItems();
    const currentIndex = visibleItems.indexOf(currentItem);
    
    if (currentIndex > 0) {
      this.focusTreeItem(visibleItems[currentIndex - 1]);
    }
  }

  /**
   * Navigate to the next visible tree item
   * @param {HTMLElement} currentItem - Current tree item
   */
  navigateToNext(currentItem) {
    const visibleItems = this.getVisibleTreeItems();
    const currentIndex = visibleItems.indexOf(currentItem);
    
    if (currentIndex < visibleItems.length - 1) {
      this.focusTreeItem(visibleItems[currentIndex + 1]);
    }
  }

  /**
   * Navigate to the first tree item
   */
  navigateToFirst() {
    const visibleItems = this.getVisibleTreeItems();
    if (visibleItems.length > 0) {
      this.focusTreeItem(visibleItems[0]);
    }
  }

  /**
   * Navigate to the last visible tree item
   */
  navigateToLast() {
    const visibleItems = this.getVisibleTreeItems();
    if (visibleItems.length > 0) {
      this.focusTreeItem(visibleItems[visibleItems.length - 1]);
    }
  }

  /**
   * Navigate by page (approximately 10 items)
   * @param {number} direction - 1 for down, -1 for up
   */
  navigateByPage(direction) {
    const visibleItems = this.getVisibleTreeItems();
    const currentFocused = document.activeElement;
    const currentIndex = visibleItems.indexOf(currentFocused);
    
    if (currentIndex === -1) {
      this.navigateToFirst();
      return;
    }
    
    const pageSize = 10;
    let targetIndex = currentIndex + (direction * pageSize);
    
    // Clamp to valid range
    targetIndex = Math.max(0, Math.min(targetIndex, visibleItems.length - 1));
    
    this.focusTreeItem(visibleItems[targetIndex]);
  }

  /**
   * Get all currently visible tree items in DOM order
   * @returns {HTMLElement[]} Array of visible tree item elements
   */
  getVisibleTreeItems() {
    return Array.from(this.container.querySelectorAll(`.${FileTree.CSS_CLASSES.TREE_ITEM}`));
  }

  /**
   * Focus a tree item and scroll it into view
   * @param {HTMLElement} treeItem - Tree item to focus
   */
  focusTreeItem(treeItem) {
    if (!treeItem) return;
    
    treeItem.focus();
    
    // Smooth scroll into view
    treeItem.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
      inline: 'nearest'
    });
  }

  /**
   * Get relative path from absolute path
   * @param {string} absolutePath - Absolute file path
   * @returns {string} Relative path from vault root
   */
  getRelativePath(absolutePath) {
    const vaultPath = this.appState.getState().currentVault || '';
    if (vaultPath && absolutePath.startsWith(vaultPath)) {
      return absolutePath.substring(vaultPath.length).replace(/^\/+/, '');
    }
    return absolutePath;
  }

  /**
   * Set up simple HTML5 drag and drop functionality
   * Uses standard HTML5 drag and drop with dragDropEnabled: false in Tauri config
   * Only folders are valid drop targets
   */
  setupDragAndDrop() {
    if (!('draggable' in document.createElement('div'))) {
      console.warn('FileTree: Drag and drop not supported');
      return;
    }
    
    // Initialize simple drag state
    this.dragState = {
      draggedFile: null,
      isDragging: false
    };
    
    // Set up event listeners with delegation
    this.container.addEventListener('dragstart', (e) => this.handleDragStart(e));
    this.container.addEventListener('dragover', (e) => this.handleDragOver(e));
    this.container.addEventListener('dragenter', (e) => this.handleDragEnter(e));
    this.container.addEventListener('dragleave', (e) => this.handleDragLeave(e));
    this.container.addEventListener('drop', (e) => this.handleDrop(e));
    this.container.addEventListener('dragend', (e) => this.handleDragEnd(e));
    
    // Set up vault-info as drop target for root level drops
    this.setupVaultInfoDropTarget();
  }

  /**
   * Set up vault-info area as a drop target for root level drops
   */
  setupVaultInfoDropTarget() {
    const vaultInfo = document.querySelector('.vault-info');
    if (!vaultInfo) {
      console.warn('FileTree: vault-info element not found for drop target setup');
      return;
    }
    
    // Enable dropping on vault-info area
    vaultInfo.addEventListener('dragover', (e) => {
      if (!this.dragState.isDragging) return;
      e.preventDefault();
      if (this.canDropOnRoot()) {
        e.dataTransfer.dropEffect = 'move';
        vaultInfo.classList.add('drop-target-root');
      } else {
        e.dataTransfer.dropEffect = 'none';
      }
    });
    
    vaultInfo.addEventListener('dragenter', (e) => {
      if (!this.dragState.isDragging) return;
      e.preventDefault();
    });
    
    vaultInfo.addEventListener('dragleave', (e) => {
      if (!this.dragState.isDragging) return;
      // Only clear highlight if we're leaving the vault-info area entirely
      if (!vaultInfo.contains(e.relatedTarget)) {
        vaultInfo.classList.remove('drop-target-root');
      }
    });
    
    vaultInfo.addEventListener('drop', (e) => {
      if (!this.dragState.isDragging) return;
      e.preventDefault();
      e.stopPropagation();
      
      if (this.canDropOnRoot()) {
        const vaultPath = this.appState.getState().currentVault;
        if (vaultPath) {
          this.emit(FileTree.EVENTS.FILE_MOVE_REQUESTED, {
            sourceFile: this.dragState.draggedFile,
            targetFolder: { path: vaultPath, is_dir: true, name: 'vault-root' },
            newPath: `${vaultPath}/${this.dragState.draggedFile.name}`
          });
        }
      }
      
      vaultInfo.classList.remove('drop-target-root');
      this.handleDragEnd();
    });
  }

  /**
   * Handle drag start event
   * @param {DragEvent} event - Drag start event
   */
  handleDragStart(event) {
    const treeItem = event.target.closest(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
    if (!treeItem) {
      event.preventDefault();
      return;
    }
    
    const filePath = treeItem.dataset.filePath;
    const file = this.files.find(f => f.path === filePath);
    
    if (!file) {
      event.preventDefault();
      return;
    }
    
    // Set up simple drag state
    this.dragState.draggedFile = file;
    this.dragState.isDragging = true;
    
    // Set drag data
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', file.path);
    
    // Add visual feedback
    treeItem.classList.add('dragging');
    
    // Emit drag start event
    this.emit(FileTree.EVENTS.DRAG_START, { file });
  }

  /**
   * Handle drag enter event
   * @param {DragEvent} event - Drag enter event
   */
  handleDragEnter(event) {
    if (!this.dragState.isDragging) return;
    event.preventDefault();
  }

  /**
   * Handle drag over event - folder drops and root level drops
   * @param {DragEvent} event - Drag over event
   */
  handleDragOver(event) {
    if (!this.dragState.isDragging) return;
    
    event.preventDefault();
    
    // Find folder drop target
    const folderTarget = this.getFolderDropTarget(event.target);
    
    // Clear previous drop highlights
    this.clearDropHighlights();
    
    if (folderTarget) {
      if (folderTarget === 'root') {
        // Root level drop
        if (this.canDropOnRoot()) {
          event.dataTransfer.dropEffect = 'move';
          // Only highlight vault-info area for root drops
          const vaultInfo = document.querySelector('.vault-info');
          if (vaultInfo) {
            vaultInfo.classList.add('drop-target-root');
          }
        } else {
          event.dataTransfer.dropEffect = 'none';
        }
      } else if (this.canDropOnFolder(folderTarget)) {
        // Regular folder drop
        event.dataTransfer.dropEffect = 'move';
        
        // Check if we're dropping directly on a children container vs. the folder item itself
        const childrenContainer = event.target.closest(`.${FileTree.CSS_CLASSES.TREE_CHILDREN}`);
        if (childrenContainer) {
          // Highlight the children container if we're hovering over it directly
          childrenContainer.classList.add('drop-target-children');
        } else {
          // Highlight the folder item itself
          folderTarget.classList.add('drop-target');
        }
      } else {
        event.dataTransfer.dropEffect = 'none';
      }
    } else {
      event.dataTransfer.dropEffect = 'none';
    }
  }

  /**
   * Handle drag leave event
   * @param {DragEvent} event - Drag leave event
   */
  handleDragLeave(event) {
    if (!this.dragState.isDragging) return;
    
    // Only clear highlights if we're leaving the container entirely
    if (!this.container.contains(event.relatedTarget)) {
      this.clearDropHighlights();
    }
  }


  /**
   * Handle drop event - folder drops and root level drops
   * @param {DragEvent} event - Drop event
   */
  handleDrop(event) {
    if (!this.dragState.isDragging) return;
    
    event.preventDefault();
    event.stopPropagation();
    
    // Find folder drop target
    const folderTarget = this.getFolderDropTarget(event.target);
    
    if (folderTarget) {
      if (folderTarget === 'root') {
        // Root level drop
        if (this.canDropOnRoot()) {
          const vaultPath = this.appState.getState().currentVault;
          if (vaultPath) {
            this.emit(FileTree.EVENTS.FILE_MOVE_REQUESTED, {
              sourceFile: this.dragState.draggedFile,
              targetFolder: { path: vaultPath, is_dir: true, name: 'vault-root' },
              newPath: `${vaultPath}/${this.dragState.draggedFile.name}`
            });
          }
        }
      } else if (this.canDropOnFolder(folderTarget)) {
        // Regular folder drop
        const targetFilePath = folderTarget.dataset.filePath;
        const targetFile = this.files.find(f => f.path === targetFilePath);
        
        if (targetFile && targetFile.is_dir) {
          // Emit move request event for the application to handle
          this.emit(FileTree.EVENTS.FILE_MOVE_REQUESTED, {
            sourceFile: this.dragState.draggedFile,
            targetFolder: targetFile,
            newPath: `${targetFile.path}/${this.dragState.draggedFile.name}`
          });
        }
      }
    }
    
    this.handleDragEnd();
  }

  /**
   * Get folder drop target from event target
   * @param {HTMLElement} target - Event target element
   * @returns {HTMLElement|null} Folder tree item element or 'root' for vault root
   */
  getFolderDropTarget(target) {
    // Check if dropping on vault-info for root level drop
    const vaultInfo = target.closest('.vault-info');
    if (vaultInfo) {
      return 'root'; // Special identifier for root level drops
    }
    
    // Check if we're hovering over a tree-children container directly
    const childrenContainer = target.closest(`.${FileTree.CSS_CLASSES.TREE_CHILDREN}`);
    if (childrenContainer) {
      // Find the parent folder item that owns this children container
      const parentFolder = this.findParentFolderForChildren(childrenContainer);
      if (parentFolder) {
        return parentFolder;
      }
    }
    
    // Check for direct tree item (must be a folder)
    const treeItem = target.closest(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
    if (treeItem) {
      const isDir = treeItem.dataset.isDir === 'true';
      // Only return the item if it's a directory (folder)
      if (isDir) {
        return treeItem;
      }
      // If it's a file, don't allow dropping - return null
      return null;
    }
    
    // Check if we're in the main file tree container but not over a specific item
    // This handles drops to empty space in the tree (root level)
    const treeContainer = target.closest(`.${FileTree.CSS_CLASSES.TREE_CONTAINER}`);
    if (treeContainer && target === treeContainer) {
      // Only allow root drop if we're directly on the container, not on a child element
      return 'root';
    }
    
    return null;
  }

  /**
   * Find the parent folder item for a given children container
   * @param {HTMLElement} childrenContainer - The tree-children container
   * @returns {HTMLElement|null} The parent folder tree item
   */
  findParentFolderForChildren(childrenContainer) {
    // Look through all folder items to find which one has this children container
    const allFolderItems = this.container.querySelectorAll(`.${FileTree.CSS_CLASSES.TREE_FOLDER}`);
    
    for (const folderItem of allFolderItems) {
      if (folderItem.childrenContainer === childrenContainer) {
        return folderItem;
      }
    }
    
    // Alternative approach: look at the previous sibling (the folder should come right before its children)
    const previousElement = childrenContainer.previousElementSibling;
    if (previousElement && 
        previousElement.classList.contains(FileTree.CSS_CLASSES.TREE_FOLDER) &&
        previousElement.childrenContainer === childrenContainer) {
      return previousElement;
    }
    
    return null;
  }

  /**
   * Check if a file can be dropped on a folder
   * @param {HTMLElement} folderElement - Folder tree item element
   * @returns {boolean} Whether drop is allowed
   */
  canDropOnFolder(folderElement) {
    if (!folderElement || !this.dragState.draggedFile) return false;
    
    const folderPath = folderElement.dataset.filePath;
    const draggedPath = this.dragState.draggedFile.path;
    
    // Can't drop on itself
    if (draggedPath === folderPath) return false;
    
    // Can't drop a parent folder into its own child
    if (folderPath.startsWith(draggedPath + '/')) return false;
    
    return true;
  }

  /**
   * Check if a file can be dropped on the root level
   * @returns {boolean} Whether drop is allowed
   */
  canDropOnRoot() {
    if (!this.dragState.draggedFile) return false;
    
    const vaultPath = this.appState.getState().currentVault;
    if (!vaultPath) return false;
    
    const draggedPath = this.dragState.draggedFile.path;
    const draggedParentPath = this.getParentPath(this.getRelativePath(draggedPath));
    
    // Already at root level, no need to move
    if (!draggedParentPath || draggedParentPath === '') return false;
    
    return true;
  }

  /**
   * Find nearest folder from mouse coordinates (for same-level drops)
   * This method is kept for potential future use but not currently used in the simplified logic
   * @param {number} x - Mouse X coordinate
   * @param {number} y - Mouse Y coordinate
   * @returns {HTMLElement|null} Nearest folder element
   */
  findNearestFolderFromPoint(x, y) {
    const allFolders = this.container.querySelectorAll(`.${FileTree.CSS_CLASSES.TREE_FOLDER}`);
    let nearestFolder = null;
    let nearestDistance = Infinity;
    
    allFolders.forEach(folder => {
      const rect = folder.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;
      const distance = Math.sqrt(Math.pow(x - centerX, 2) + Math.pow(y - centerY, 2));
      
      if (distance < nearestDistance && distance < 100) { // Within 100px
        nearestDistance = distance;
        nearestFolder = folder;
      }
    });
    
    return nearestFolder;
  }

  /**
   * Clear all drop target highlights
   */
  clearDropHighlights() {
    const dropTargets = this.container.querySelectorAll('.drop-target');
    dropTargets.forEach(target => {
      target.classList.remove('drop-target');
    });
    
    const childrenDropTargets = this.container.querySelectorAll('.drop-target-children');
    childrenDropTargets.forEach(target => {
      target.classList.remove('drop-target-children');
    });
    
    // Clear root level drop highlights
    const vaultInfo = document.querySelector('.vault-info');
    if (vaultInfo) {
      vaultInfo.classList.remove('drop-target-root');
    }
  }

  /**
   * Handle drag end event
   */
  handleDragEnd() {
    // Clear visual feedback
    const draggingItems = this.container.querySelectorAll('.dragging');
    draggingItems.forEach(item => {
      item.classList.remove('dragging');
    });
    
    this.clearDropHighlights();
    
    // Emit drag end event
    if (this.dragState.isDragging) {
      this.emit(FileTree.EVENTS.DRAG_END, { file: this.dragState.draggedFile });
    }
    
    // Reset drag state
    this.dragState = {
      draggedFile: null,
      isDragging: false
    };
  }


  /**
   * Create search interface for filtering files
   */
  createSearchInterface() {
    const searchContainer = document.createElement('div');
    searchContainer.className = 'file-tree-search-container';
    searchContainer.style.display = 'none';
    
    const searchInput = document.createElement('input');
    searchInput.type = 'text';
    searchInput.className = 'file-tree-search-input';
    searchInput.placeholder = 'Filter files...';
    searchInput.setAttribute('aria-label', 'Filter files');
    
    const closeButton = document.createElement('button');
    closeButton.className = 'file-tree-search-close';
    closeButton.innerHTML = '×';
    closeButton.setAttribute('aria-label', 'Close search');
    closeButton.type = 'button';
    
    searchContainer.appendChild(searchInput);
    searchContainer.appendChild(closeButton);
    
    // Insert at the top of the container
    this.container.insertBefore(searchContainer, this.container.firstChild);
    
    this.searchInput = searchInput;
    this.searchContainer = searchContainer;
    
    // Set up search event handlers
    this.setupSearchHandlers();
  }

  /**
   * Update search references after DOM reconstruction
   */
  updateSearchReferences() {
    const oldSearchContainer = this.searchContainer;
    
    this.searchContainer = this.container.querySelector('.file-tree-search-container');
    this.searchInput = this.container.querySelector('.file-tree-search-input');
    
    // If the DOM elements are the same, event handlers should still be attached
    // Only re-setup if references actually changed (which shouldn't happen with our current approach)
    if (oldSearchContainer !== this.searchContainer && this.searchContainer && this.searchInput) {
      this.setupSearchHandlers();
    }
  }

  /**
   * Set up search event handlers
   */
  setupSearchHandlers() {
    // Debounced search input handler
    const searchHandler = (event) => {
      clearTimeout(this.searchDebounceTimer);
      this.searchDebounceTimer = setTimeout(() => {
        this.performSearch(event.target.value.trim());
      }, 300); // 300ms debounce
    };
    
    this.searchInput.addEventListener('input', searchHandler);
    this.eventListeners.set('search-input', searchHandler);
    
    // Close button handler
    const closeHandler = () => this.deactivateSearch();
    const closeButton = this.searchContainer.querySelector('.file-tree-search-close');
    closeButton.addEventListener('click', closeHandler);
    this.eventListeners.set('search-close', closeHandler);
    
    // Escape key handler for search input
    const keyHandler = (event) => {
      if (event.key === 'Escape') {
        this.deactivateSearch();
      } else if (event.key === 'ArrowDown') {
        event.preventDefault();
        // Move focus to first tree item
        const firstItem = this.container.querySelector(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
        if (firstItem) {
          firstItem.focus();
        }
      }
    };
    
    this.searchInput.addEventListener('keydown', keyHandler);
    this.eventListeners.set('search-keydown', keyHandler);
  }

  /**
   * Activate search mode
   */
  activateSearch() {
    if (!this.searchContainer || !this.searchInput) {
      console.error('FileTree: Search interface not properly initialized');
      return;
    }
    
    this.isSearchActive = true;
    this.searchContainer.style.display = 'flex';
    this.searchContainer.style.visibility = 'visible';
    this.searchInput.focus();
    this.searchInput.select();
  }

  /**
   * Deactivate search mode and return to normal view
   */
  deactivateSearch() {
    this.isSearchActive = false;
    if (this.searchContainer) {
      this.searchContainer.style.display = 'none';
    }
    if (this.searchInput) {
      this.searchInput.value = '';
    }
    
    // Clear search filter
    this.filteredFiles = null;
    
    // Re-render with original files
    this.render(this.files);
    
    // Update references after render since DOM may have been reconstructed
    this.updateSearchReferences();
    
    // Return focus to tree
    const firstItem = this.container.querySelector(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
    if (firstItem) {
      firstItem.focus();
    }
  }

  /**
   * Perform fuzzy search on files and render results
   * @param {string} searchTerm - The search term
   */
  performSearch(searchTerm) {
    if (!searchTerm) {
      this.filteredFiles = null;
      this.render(this.files);
      return;
    }
    
    const startTime = performance.now();
    
    // Fuzzy search algorithm
    const searchResults = this.fuzzySearch(this.files, searchTerm);
    
    // Sort by relevance score
    searchResults.sort((a, b) => b.score - a.score);
    
    this.filteredFiles = searchResults.map(result => result.file);
    
    // Render filtered results
    this.renderSearchResults(this.filteredFiles, searchTerm);
    
    // Performance tracking
    const renderTime = performance.now() - startTime;
    this.updatePerformanceMetrics(renderTime);
    
    // Show search stats
    this.showSearchStats(searchResults.length, searchTerm, renderTime);
  }

  /**
   * Fuzzy search implementation with scoring
   * @param {Array} files - Files to search
   * @param {string} searchTerm - Search term
   * @returns {Array} Array of {file, score} objects
   */
  fuzzySearch(files, searchTerm) {
    const term = searchTerm.toLowerCase();
    const results = [];
    
    files.forEach(file => {
      const fileName = file.name.toLowerCase();
      const filePath = file.path.toLowerCase();
      
      // Exact match gets highest score
      if (fileName === term) {
        results.push({ file, score: 1000 });
        return;
      }
      
      // Start of name match
      if (fileName.startsWith(term)) {
        results.push({ file, score: 800 });
        return;
      }
      
      // Contains term
      if (fileName.includes(term)) {
        results.push({ file, score: 600 });
        return;
      }
      
      // Path contains term
      if (filePath.includes(term)) {
        results.push({ file, score: 400 });
        return;
      }
      
      // Fuzzy character matching
      const fuzzyScore = this.calculateFuzzyScore(fileName, term);
      if (fuzzyScore > 0) {
        results.push({ file, score: fuzzyScore });
      }
    });
    
    return results;
  }

  /**
   * Calculate fuzzy matching score
   * @param {string} text - Text to search in
   * @param {string} term - Search term
   * @returns {number} Fuzzy score (0-300)
   */
  calculateFuzzyScore(text, term) {
    let score = 0;
    let termIndex = 0;
    let lastMatchIndex = -1;
    
    for (let i = 0; i < text.length && termIndex < term.length; i++) {
      if (text[i] === term[termIndex]) {
        score += 10;
        
        // Bonus for consecutive matches
        if (i === lastMatchIndex + 1) {
          score += 5;
        }
        
        // Bonus for word boundary matches
        if (i === 0 || text[i - 1] === ' ' || text[i - 1] === '-' || text[i - 1] === '_') {
          score += 15;
        }
        
        lastMatchIndex = i;
        termIndex++;
      }
    }
    
    // Only return score if all characters were matched
    return termIndex === term.length ? Math.max(score - (text.length - term.length), 0) : 0;
  }

  /**
   * Render search results with highlighted matches
   * @param {Array} filteredFiles - Filtered file list
   * @param {string} searchTerm - The search term for highlighting
   */
  renderSearchResults(filteredFiles, searchTerm) {
    if (filteredFiles.length === 0) {
      this.showNoSearchResults(searchTerm);
      return;
    }
    
    // Store current focus state and cursor position
    const activeElement = document.activeElement;
    const wasSearchInputFocused = activeElement && activeElement.classList.contains('file-tree-search-input');
    const cursorPosition = wasSearchInputFocused ? activeElement.selectionStart : null;
    
    // Clear container but preserve search interface (same pattern as render method)
    const searchContainer = this.container.querySelector('.file-tree-search-container');
    this.container.innerHTML = '';
    if (searchContainer) {
      this.container.appendChild(searchContainer);
    }
    
    // Render search results in flat list (no hierarchy during search)
    filteredFiles.forEach((file, index) => {
      const itemElement = this.createSearchResultItem(file, searchTerm, index);
      this.container.appendChild(itemElement);
    });
    
    // Update references after DOM manipulation
    this.updateSearchReferences();
    
    // Restore focus to search input if it was focused before
    if (wasSearchInputFocused && this.searchInput) {
      this.searchInput.focus();
      if (cursorPosition !== null) {
        this.searchInput.setSelectionRange(cursorPosition, cursorPosition);
      }
    }
    
    // Set tab index for first result (but don't focus it)
    const firstResult = this.container.querySelector(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
    if (firstResult) {
      firstResult.setAttribute('tabindex', '0');
    }
  }

  /**
   * Create a search result item with highlighted text
   * @param {Object} file - File object
   * @param {string} searchTerm - Search term for highlighting
   * @param {number} index - Item index
   * @returns {HTMLElement} Search result element
   */
  createSearchResultItem(file, searchTerm, index) {
    const item = document.createElement('div');
    item.className = `${FileTree.CSS_CLASSES.TREE_ITEM} ${file.is_dir ? FileTree.CSS_CLASSES.TREE_FOLDER : FileTree.CSS_CLASSES.TREE_FILE} search-result`;
    
    // Remove depth-based styling for search results
    item.style.paddingLeft = '8px';
    
    // Store file data
    item.dataset.filePath = file.path;
    item.dataset.isDir = file.is_dir.toString();
    item.dataset.searchIndex = index.toString();
    
    // Accessibility attributes
    item.setAttribute('role', 'option');
    item.setAttribute('aria-label', `Search result ${index + 1}: ${file.is_dir ? 'Folder' : 'File'} ${file.name}`);
    item.setAttribute('tabindex', index === 0 ? '0' : '-1');
    
    // Create item content with highlighting
    const icon = this.createIcon(file);
    const name = this.createHighlightedName(file, searchTerm);
    const path = this.createSearchResultPath(file);
    
    item.appendChild(icon);
    item.appendChild(name);
    if (path) {
      item.appendChild(path);
    }
    
    return item;
  }

  /**
   * Create highlighted name element for search results
   * @param {Object} file - File object
   * @param {string} searchTerm - Search term for highlighting
   * @returns {HTMLElement} Name element with highlighting
   */
  createHighlightedName(file, searchTerm) {
    const name = document.createElement('span');
    name.className = `${FileTree.CSS_CLASSES.TREE_NAME} highlighted`;
    
    // Highlight matching text
    const highlightedText = this.highlightText(file.name, searchTerm);
    name.innerHTML = highlightedText;
    
    return name;
  }

  /**
   * Create path element for search results
   * @param {Object} file - File object
   * @returns {HTMLElement|null} Path element or null
   */
  createSearchResultPath(file) {
    const relativePath = this.getRelativePath(file.path);
    const parentPath = this.getParentPath(relativePath);
    
    if (!parentPath) return null; // Root level files don't need path
    
    const pathElement = document.createElement('span');
    pathElement.className = 'search-result-path';
    pathElement.textContent = parentPath;
    pathElement.title = `Located in: ${parentPath}`;
    
    return pathElement;
  }

  /**
   * Highlight matching text in a string
   * @param {string} text - Original text
   * @param {string} searchTerm - Term to highlight
   * @returns {string} HTML string with highlighted matches
   */
  highlightText(text, searchTerm) {
    if (!searchTerm) return text;
    
    const regex = new RegExp(`(${this.escapeRegex(searchTerm)})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }

  /**
   * Escape special regex characters
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   */
  escapeRegex(text) {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  /**
   * Show message when no search results found
   * @param {string} searchTerm - The search term
   */
  showNoSearchResults(searchTerm) {
    // Store current focus state and cursor position
    const activeElement = document.activeElement;
    const wasSearchInputFocused = activeElement && activeElement.classList.contains('file-tree-search-input');
    const cursorPosition = wasSearchInputFocused ? activeElement.selectionStart : null;
    
    // Clear container but preserve search interface (same pattern as render method)
    const searchContainer = this.container.querySelector('.file-tree-search-container');
    this.container.innerHTML = '';
    if (searchContainer) {
      this.container.appendChild(searchContainer);
    }
    
    const noResultsDiv = document.createElement('div');
    noResultsDiv.className = 'file-tree-no-results';
    noResultsDiv.innerHTML = `
      <p>No files found matching "${searchTerm}"</p>
      <small>Try a different search term or check your spelling</small>
    `;
    
    this.container.appendChild(noResultsDiv);
    
    // Update references after DOM manipulation
    this.updateSearchReferences();
    
    // Restore focus to search input if it was focused before
    if (wasSearchInputFocused && this.searchInput) {
      this.searchInput.focus();
      if (cursorPosition !== null) {
        this.searchInput.setSelectionRange(cursorPosition, cursorPosition);
      }
    }
  }

  /**
   * Show search statistics
   * @param {number} resultCount - Number of results found
   * @param {string} searchTerm - The search term
   * @param {number} renderTime - Time taken to render results
   */
  showSearchStats(resultCount, searchTerm, renderTime) {
    // You could add a stats display here if needed
    console.debug(`Search "${searchTerm}": ${resultCount} results in ${renderTime.toFixed(2)}ms`);
  }

  /**
   * Update performance metrics
   * @param {number} renderTime - Time taken for last render
   */
  updatePerformanceMetrics(renderTime) {
    this.performanceMetrics.lastRenderTime = renderTime;
    this.performanceMetrics.renderCount++;
    
    // Calculate running average
    const totalTime = (this.performanceMetrics.averageRenderTime * (this.performanceMetrics.renderCount - 1)) + renderTime;
    this.performanceMetrics.averageRenderTime = totalTime / this.performanceMetrics.renderCount;
  }

  /**
   * Show loading state with message
   * @param {string} message - Loading message to display
   */
  showLoadingState(message = 'Loading...') {
    // Create or update loading indicator
    let loadingIndicator = this.container.querySelector('.file-tree-loading-indicator');
    if (!loadingIndicator) {
      loadingIndicator = document.createElement('div');
      loadingIndicator.className = 'file-tree-loading-indicator';
      loadingIndicator.setAttribute('role', 'status');
      loadingIndicator.setAttribute('aria-live', 'polite');
      
      // Position it at the top after search container
      const searchContainer = this.container.querySelector('.file-tree-search-container');
      if (searchContainer) {
        this.container.insertBefore(loadingIndicator, searchContainer.nextSibling);
      } else {
        this.container.insertBefore(loadingIndicator, this.container.firstChild);
      }
    }
    
    loadingIndicator.innerHTML = `
      <div class="loading-content">
        <span class="loading-spinner" aria-hidden="true">⏳</span>
        <span class="loading-message">${message}</span>
      </div>
    `;
    
    loadingIndicator.style.display = 'flex';
  }

  /**
   * Hide loading state
   */
  hideLoadingState() {
    const loadingIndicator = this.container.querySelector('.file-tree-loading-indicator');
    if (loadingIndicator) {
      loadingIndicator.style.display = 'none';
    }
  }

  /**
   * Show error state with message
   * @param {string} message - Error message to display
   */
  showErrorState(message) {
    // Create or update error indicator
    let errorIndicator = this.container.querySelector('.file-tree-error-indicator');
    if (!errorIndicator) {
      errorIndicator = document.createElement('div');
      errorIndicator.className = 'file-tree-error-indicator';
      errorIndicator.setAttribute('role', 'alert');
      errorIndicator.setAttribute('aria-live', 'assertive');
      
      // Position it at the top after search container
      const searchContainer = this.container.querySelector('.file-tree-search-container');
      if (searchContainer) {
        this.container.insertBefore(errorIndicator, searchContainer.nextSibling);
      } else {
        this.container.insertBefore(errorIndicator, this.container.firstChild);
      }
    }
    
    errorIndicator.innerHTML = `
      <div class="error-content">
        <span class="error-icon" aria-hidden="true">⚠️</span>
        <span class="error-message">${message}</span>
        <button class="error-dismiss" type="button" aria-label="Dismiss error">×</button>
      </div>
    `;
    
    errorIndicator.style.display = 'flex';
    
    // Add dismiss handler
    const dismissButton = errorIndicator.querySelector('.error-dismiss');
    dismissButton.addEventListener('click', () => {
      this.hideErrorState();
    });
    
    // Auto-hide after 10 seconds
    setTimeout(() => {
      this.hideErrorState();
    }, 10000);
  }

  /**
   * Hide error state
   */
  hideErrorState() {
    const errorIndicator = this.container.querySelector('.file-tree-error-indicator');
    if (errorIndicator) {
      errorIndicator.style.display = 'none';
    }
  }

  /**
   * Implement lazy expansion of folders
   * @param {string} folderPath - Path to the folder to expand
   */
  async expandFolderLazy(folderPath) {
    if (this.expandedFolders.has(folderPath)) return;
    
    const folderElement = this.findTreeItem(folderPath);
    if (!folderElement) return;
    
    // Show loading state for the folder
    this.showFolderLoadingState(folderElement);
    
    try {
      // Get folder children (this might involve async operations)
      const children = await this.getFolderChildrenLazy(folderPath);
      
      // Hide loading state
      this.hideFolderLoadingState(folderElement);
      
      // Expand the folder normally
      this.expandFolder(folderPath);
      
    } catch (error) {
      console.error(`Failed to expand folder ${folderPath}:`, error);
      this.hideFolderLoadingState(folderElement);
      this.showFolderErrorState(folderElement, 'Failed to load folder contents');
    }
  }

  /**
   * Get folder children with lazy loading support
   * @param {string} folderPath - Path to the folder
   * @returns {Promise<Array>} Promise resolving to array of child files
   */
  async getFolderChildrenLazy(folderPath) {
    // For now, use the existing synchronous method
    // In a real implementation, this might make an async call to the backend
    return new Promise((resolve) => {
      // Simulate async operation
      setTimeout(() => {
        const children = this.getFolderChildren(folderPath);
        resolve(children);
      }, 100);
    });
  }

  /**
   * Show loading state for a specific folder
   * @param {HTMLElement} folderElement - The folder tree item element
   */
  showFolderLoadingState(folderElement) {
    const icon = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_ICON}`);
    if (icon) {
      icon.classList.add('loading');
      // Could add a spinning animation here
    }
  }

  /**
   * Hide loading state for a specific folder
   * @param {HTMLElement} folderElement - The folder tree item element
   */
  hideFolderLoadingState(folderElement) {
    const icon = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_ICON}`);
    if (icon) {
      icon.classList.remove('loading');
    }
  }

  /**
   * Show error state for a specific folder
   * @param {HTMLElement} folderElement - The folder tree item element
   * @param {string} message - Error message
   */
  showFolderErrorState(folderElement, message) {
    folderElement.classList.add('folder-error');
    folderElement.title = message;
    
    const icon = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_ICON}`);
    if (icon) {
      icon.classList.add('error');
    }
  }

  /**
   * Implement performance monitoring and optimization
   */
  performanceOptimization() {
    // Monitor memory usage
    if ('memory' in performance) {
      const memoryInfo = performance.memory;
      const usedMemory = memoryInfo.usedJSHeapSize / (1024 * 1024); // MB
      
      // Log memory usage for large trees
      if (this.files.length > 1000) {
        console.debug(`FileTree memory usage: ${usedMemory.toFixed(2)}MB for ${this.files.length} files`);
      }
      
      // Trigger garbage collection hint if memory usage is high
      if (usedMemory > 50) { // 50MB threshold
        this.optimizeMemoryUsage();
      }
    }
    
    // Monitor rendering performance
    const avgRenderTime = this.performanceMetrics.averageRenderTime;
    if (avgRenderTime > 100) { // 100ms threshold
      console.warn(`FileTree: Average render time is ${avgRenderTime.toFixed(2)}ms - consider optimizations`);
      this.suggestPerformanceOptimizations();
    }
  }

  /**
   * Optimize memory usage by cleaning up unused elements
   */
  optimizeMemoryUsage() {
    // Clear cached DOM references that are no longer needed
    const allItems = this.getVisibleTreeItems();
    const viewportItems = this.getItemsInViewport();
    
    allItems.forEach(item => {
      // If item is far from viewport and not expanded, consider recycling
      if (!viewportItems.includes(item) && !item.classList.contains('expanded')) {
        // Remove cached children if they exist
        if (item.childrenContainer && item.childrenContainer.children.length > 50) {
          // Keep only the first 10 and last 10 children
          const children = Array.from(item.childrenContainer.children);
          const toRemove = children.slice(10, -10);
          toRemove.forEach(child => child.remove());
        }
      }
    });
    
    console.debug('FileTree: Performed memory optimization');
  }

  /**
   * Get items currently in viewport
   * @returns {HTMLElement[]} Array of visible items
   */
  getItemsInViewport() {
    const containerRect = this.container.getBoundingClientRect();
    const allItems = this.getVisibleTreeItems();
    
    return allItems.filter(item => {
      const itemRect = item.getBoundingClientRect();
      return itemRect.bottom >= containerRect.top && itemRect.top <= containerRect.bottom;
    });
  }

  /**
   * Suggest performance optimizations based on current state
   */
  suggestPerformanceOptimizations() {
    const suggestions = [];
    
    if (this.files.length > 1000 && !this.isVirtualScrolling) {
      suggestions.push('Enable virtual scrolling for large file trees');
    }
    
    if (this.performanceMetrics.averageRenderTime > 50) {
      suggestions.push('Consider limiting initial folder expansion depth');
    }
    
    if (this.expandedFolders.size > 20) {
      suggestions.push('Consider automatically collapsing distant folders');
    }
    
    if (suggestions.length > 0) {
      console.info('FileTree performance suggestions:', suggestions);
    }
  }

  /**
   * Set up virtual scrolling for large file trees
   */
  setupVirtualScrolling() {
    // Enable virtual scrolling only when we have many files
    this.checkVirtualScrollingNeeded();
    
    // Set up intersection observer for viewport detection
    if ('IntersectionObserver' in window) {
      const observerOptions = {
        root: this.container,
        rootMargin: '100px', // Load items 100px before they come into view
        threshold: [0, 0.1, 0.5, 1]
      };
      
      this.intersectionObserver = new IntersectionObserver((entries) => {
        this.handleIntersectionChanges(entries);
      }, observerOptions);
    }
    
    // Set up scroll event listener for virtual scrolling
    const scrollHandler = (event) => this.handleVirtualScroll(event);
    this.container.addEventListener('scroll', scrollHandler, { passive: true });
    this.eventListeners.set('scroll', scrollHandler);
  }

  /**
   * Check if virtual scrolling is needed based on file count
   */
  checkVirtualScrollingNeeded() {
    const fileCount = this.files ? this.files.length : 0;
    
    if (fileCount > 1000) {
      this.isVirtualScrolling = true;
      this.container.setAttribute('data-large-tree', 'true');
      this.visibleItemsCount = Math.min(50, Math.ceil(this.container.clientHeight / 22)); // 22px per item
      
      console.debug(`FileTree: Enabled virtual scrolling for ${fileCount} files (visible: ${this.visibleItemsCount})`);
    } else {
      this.isVirtualScrolling = false;
      this.container.removeAttribute('data-large-tree');
    }
  }

  /**
   * Handle intersection observer changes for virtual scrolling
   * @param {IntersectionObserverEntry[]} entries - Intersection entries
   */
  handleIntersectionChanges(entries) {
    if (!this.isVirtualScrolling) return;
    
    entries.forEach(entry => {
      const itemElement = entry.target;
      
      if (entry.isIntersecting) {
        // Item is visible, ensure it's fully rendered
        this.renderVirtualItem(itemElement);
      } else {
        // Item is out of view, consider recycling for performance
        const rect = entry.boundingClientRect;
        const containerRect = this.container.getBoundingClientRect();
        
        // Calculate distance from viewport
        const distanceFromViewport = Math.min(
          Math.abs(rect.bottom - containerRect.top),
          Math.abs(containerRect.bottom - rect.top)
        );
        
        // Recycle items that are far from viewport (only for very large trees)
        if (distanceFromViewport > 2000 && this.files.length > 5000) {
          this.recycleVirtualItem(itemElement);
        }
      }
    });
  }

  /**
   * Handle virtual scroll events
   * @param {Event} event - Scroll event
   */
  handleVirtualScroll(event) {
    if (!this.isVirtualScrolling) return;
    
    // Throttle scroll handling for performance
    if (this.scrollTimeout) return;
    
    this.scrollTimeout = setTimeout(() => {
      this.updateVirtualScrollPosition();
      this.scrollTimeout = null;
    }, 16); // ~60fps
  }

  /**
   * Update virtual scroll position and visible items
   */
  updateVirtualScrollPosition() {
    const scrollTop = this.container.scrollTop;
    const containerHeight = this.container.clientHeight;
    const itemHeight = 22; // Average item height in pixels
    
    // Calculate which items should be visible
    const startIndex = Math.floor(scrollTop / itemHeight);
    const endIndex = Math.min(
      startIndex + Math.ceil(containerHeight / itemHeight) + 5, // +5 buffer
      this.getVisibleTreeItems().length - 1
    );
    
    this.virtualScrollOffset = startIndex;
    
    // Update visibility of items
    this.updateVirtualItemVisibility(startIndex, endIndex);
  }

  /**
   * Update visibility of virtual items based on scroll position
   * @param {number} startIndex - First visible item index
   * @param {number} endIndex - Last visible item index
   */
  updateVirtualItemVisibility(startIndex, endIndex) {
    const allItems = this.getVisibleTreeItems();
    
    allItems.forEach((item, index) => {
      const shouldBeVisible = index >= startIndex && index <= endIndex;
      
      if (shouldBeVisible) {
        if (item.style.display === 'none') {
          item.style.display = '';
          this.renderVirtualItem(item);
        }
      } else {
        // Hide items that are far from viewport
        const distance = Math.min(
          Math.abs(index - startIndex),
          Math.abs(index - endIndex)
        );
        
        if (distance > 20) { // Hide items more than 20 positions away
          item.style.display = 'none';
          this.recycleVirtualItem(item);
        }
      }
    });
  }

  /**
   * Render a virtual item that has come into view
   * @param {HTMLElement} itemElement - Item element to render
   */
  renderVirtualItem(itemElement) {
    if (!itemElement || itemElement.hasAttribute('data-rendered')) return;
    
    const filePath = itemElement.dataset.filePath;
    const file = this.files.find(f => f.path === filePath);
    
    if (file) {
      // Mark as rendered
      itemElement.setAttribute('data-rendered', 'true');
      
      // If it's a folder with children, render them lazily
      if (file.is_dir && this.expandedFolders.has(filePath)) {
        const childrenContainer = itemElement.childrenContainer;
        if (childrenContainer && childrenContainer.children.length === 0) {
          // Use requestIdleCallback for non-critical rendering
          if ('requestIdleCallback' in window) {
            requestIdleCallback(() => {
              this.renderFolderChildren(itemElement, file);
            });
          } else {
            // Fallback for browsers without requestIdleCallback
            setTimeout(() => {
              this.renderFolderChildren(itemElement, file);
            }, 0);
          }
        }
      }
      
      // Observe this item for intersection changes
      if (this.intersectionObserver) {
        this.intersectionObserver.observe(itemElement);
      }
    }
  }

  /**
   * Render folder children lazily
   * @param {HTMLElement} folderItem - Folder item element
   * @param {Object} folderFile - Folder file object
   */
  renderFolderChildren(folderItem, folderFile) {
    if (!folderItem.childrenContainer) return;
    
    const children = this.getFolderChildren(folderFile.path);
    const depth = this.calculateDepth(folderFile.path);
    
    // Render children with virtual scrolling considerations
    if (children.length > 100) {
      // For folders with many children, render only a subset initially
      const visibleChildren = children.slice(0, 50);
      this.renderTreeLevel(visibleChildren, folderItem.childrenContainer, depth + 1);
      
      // Add a "load more" indicator if there are more children
      if (children.length > 50) {
        this.addLoadMoreIndicator(folderItem.childrenContainer, children, 50, depth + 1);
      }
    } else {
      // Render all children for smaller folders
      this.renderTreeLevel(children, folderItem.childrenContainer, depth + 1);
    }
  }

  /**
   * Add a "load more" indicator for folders with many children
   * @param {HTMLElement} container - Children container
   * @param {Array} allChildren - All children files
   * @param {number} currentlyLoaded - Number of currently loaded children
   * @param {number} depth - Current depth level
   */
  addLoadMoreIndicator(container, allChildren, currentlyLoaded, depth) {
    const loadMoreItem = document.createElement('div');
    loadMoreItem.className = 'tree-item tree-load-more';
    loadMoreItem.style.paddingLeft = `${8 + (depth * 16)}px`;
    
    const remaining = allChildren.length - currentlyLoaded;
    loadMoreItem.innerHTML = `
      <span class="tree-icon">⋯</span>
      <span class="tree-name">Load ${remaining} more items...</span>
    `;
    
    loadMoreItem.addEventListener('click', () => {
      // Load the remaining children
      const remainingChildren = allChildren.slice(currentlyLoaded);
      
      // Remove the load more indicator
      loadMoreItem.remove();
      
      // Render remaining children in batches for better performance
      this.renderChildrenInBatches(container, remainingChildren, depth);
    });
    
    container.appendChild(loadMoreItem);
  }

  /**
   * Render children in batches to avoid blocking the UI
   * @param {HTMLElement} container - Container to render into
   * @param {Array} children - Children to render
   * @param {number} depth - Current depth level
   */
  renderChildrenInBatches(container, children, depth) {
    const batchSize = 25;
    let currentIndex = 0;
    
    const renderBatch = () => {
      const batch = children.slice(currentIndex, currentIndex + batchSize);
      this.renderTreeLevel(batch, container, depth);
      
      currentIndex += batchSize;
      
      if (currentIndex < children.length) {
        // Use requestAnimationFrame for smooth rendering
        requestAnimationFrame(renderBatch);
      }
    };
    
    renderBatch();
  }

  /**
   * Recycle a virtual item that's far from viewport to save memory
   * @param {HTMLElement} itemElement - Item element to recycle
   */
  recycleVirtualItem(itemElement) {
    if (!itemElement || !itemElement.hasAttribute('data-rendered')) return;
    
    // Remove rendered attribute
    itemElement.removeAttribute('data-rendered');
    
    // Clear children container if it exists
    if (itemElement.childrenContainer) {
      itemElement.childrenContainer.innerHTML = '';
    }
    
    // Unobserve from intersection observer
    if (this.intersectionObserver) {
      this.intersectionObserver.unobserve(itemElement);
    }
    
    // The element structure remains, just content is recycled
  }

  /**
   * Clean up component resources
   */
  destroy() {
    // Clear search debounce timer
    if (this.searchDebounceTimer) {
      clearTimeout(this.searchDebounceTimer);
    }
    
    // Clear virtual scrolling observer
    if (this.intersectionObserver) {
      this.intersectionObserver.disconnect();
    }
    
    // Remove event listeners
    this.eventListeners.forEach((handler, event) => {
      this.container.removeEventListener(event, handler);
    });
    this.eventListeners.clear();
    
    // Clear container
    this.container.innerHTML = '';
    
    // Reset state
    this.files = [];
    this.expandedFolders.clear();
    this.selectedFile = null;
    this.treeStructure.clear();
    this.filteredFiles = null;
    this.isVirtualScrolling = false;
    this.virtualScrollOffset = 0;
    this.visibleItemsCount = 0;
  }
}

// Export for ES6 module usage
export default FileTree;