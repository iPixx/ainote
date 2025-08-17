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
    TREE_UPDATED: 'tree_updated'
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
    
    // Set up event delegation for performance
    this.setupEventDelegation();
    
    // Listen to app state changes
    this.setupStateListeners();
    
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

    // Build hierarchical structure for performance
    this.buildTreeStructure(files);
    
    // Clear container and render tree
    this.container.innerHTML = '';
    this.renderTreeLevel(this.getRootItems(), this.container, 0);
    
    // Emit tree updated event
    this.emit(FileTree.EVENTS.TREE_UPDATED, { 
      files: this.files, 
      count: files.length 
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
      
      // If it's a folder, create children container
      if (item.is_dir) {
        const childrenContainer = this.createChildrenContainer();
        itemElement.appendChild(childrenContainer);
        
        // Render children if folder is expanded
        if (this.expandedFolders.has(item.path)) {
          const children = this.getFolderChildren(item.path);
          this.renderTreeLevel(children, childrenContainer, depth + 1);
          itemElement.classList.add(FileTree.CSS_CLASSES.EXPANDED);
        } else {
          itemElement.classList.add(FileTree.CSS_CLASSES.COLLAPSED);
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
    
    // Add VSCode-style indentation
    if (depth > 0) {
      item.classList.add(FileTree.CSS_CLASSES.INDENTED);
      // VSCode uses 16px per level indentation
      item.style.paddingLeft = `${8 + (depth * 16)}px`;
    } else {
      item.style.paddingLeft = '8px';
    }
    
    // Store file data and depth
    item.dataset.filePath = file.path;
    item.dataset.isDir = file.is_dir.toString();
    item.dataset.depth = depth.toString();
    
    // Accessibility attributes
    item.setAttribute('role', 'treeitem');
    item.setAttribute('aria-label', `${file.is_dir ? 'Folder' : 'File'}: ${file.name}`);
    item.setAttribute('tabindex', '0');
    
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
   * Handle keyboard navigation
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
        if (isDir && !this.expandedFolders.has(filePath)) {
          event.preventDefault();
          this.expandFolder(filePath);
        }
        break;
        
      case 'ArrowLeft':
        if (isDir && this.expandedFolders.has(filePath)) {
          event.preventDefault();
          this.collapseFolder(filePath);
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
      
      // Show children
      const childrenContainer = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_CHILDREN}`);
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
      
      // Hide children
      const childrenContainer = folderElement.querySelector(`.${FileTree.CSS_CLASSES.TREE_CHILDREN}`);
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
    this.container.innerHTML = `
      <div class="file-tree-empty-state">
        <p>No files found in vault</p>
        <button type="button" class="btn-secondary" onclick="window.refreshVault()">
          Refresh Vault
        </button>
      </div>
    `;
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
   * Clean up component resources
   */
  destroy() {
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
  }
}

// Export for ES6 module usage
export default FileTree;