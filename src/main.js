const { invoke } = window.__TAURI__.core;

// Import AppState for centralized state management
import AppState from './js/state.js';

// Initialize global application state
const appState = new AppState();

// Layout Management System
class LayoutManager {
  constructor() {
    this.isResizing = false;
    this.currentResizeHandle = null;
    this.initialMouseX = 0;
    this.initialPanelWidth = 0;
    this.minWidths = {
      'file-tree': 250,
      'editor': 600,
      'ai-panel': 300
    };
    this.maxWidths = {
      'file-tree': 400,
      'editor': null, // No max width
      'ai-panel': 500
    };
    
    this.initializeLayout();
    this.bindEvents();
  }

  initializeLayout() {
    // Load saved layout preferences from localStorage
    const savedLayout = this.loadLayoutState();
    if (savedLayout) {
      this.applyLayoutState(savedLayout);
    }
  }

  bindEvents() {
    // Bind resize handle events
    document.querySelectorAll('.resize-handle').forEach(handle => {
      handle.addEventListener('mousedown', (e) => this.startResize(e));
    });

    // Global mouse events for resize
    document.addEventListener('mousemove', (e) => this.handleResize(e));
    document.addEventListener('mouseup', () => this.stopResize());

    // Window resize handler
    window.addEventListener('resize', () => this.handleWindowResize());

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => this.handleKeydown(e));
  }

  startResize(e) {
    e.preventDefault();
    this.isResizing = true;
    this.currentResizeHandle = e.target;
    this.initialMouseX = e.clientX;
    
    const panel = this.getPanelFromHandle(this.currentResizeHandle);
    const panelElement = this.getPanelElement(panel);
    this.initialPanelWidth = panelElement.getBoundingClientRect().width;
    
    // Add resizing class for visual feedback
    this.currentResizeHandle.classList.add('resizing');
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }

  handleResize(e) {
    if (!this.isResizing || !this.currentResizeHandle) return;
    
    e.preventDefault();
    const deltaX = e.clientX - this.initialMouseX;
    const panel = this.getPanelFromHandle(this.currentResizeHandle);
    const newWidth = this.initialPanelWidth + deltaX;
    
    // Apply width constraints
    const constrainedWidth = this.constrainWidth(panel, newWidth);
    this.setPanelWidth(panel, constrainedWidth);
  }

  stopResize() {
    if (!this.isResizing) return;
    
    this.isResizing = false;
    if (this.currentResizeHandle) {
      this.currentResizeHandle.classList.remove('resizing');
    }
    this.currentResizeHandle = null;
    
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    
    // Save layout state
    this.saveLayoutState();
  }

  getPanelFromHandle(handle) {
    return handle.dataset.panel;
  }

  getPanelElement(panel) {
    switch (panel) {
      case 'file-tree': return document.getElementById('fileTreePanel');
      case 'editor': return document.getElementById('editorPanel');
      case 'ai-panel': return document.getElementById('aiPanel');
      default: return null;
    }
  }

  constrainWidth(panel, width) {
    const min = this.minWidths[panel];
    const max = this.maxWidths[panel];
    
    if (width < min) return min;
    if (max && width > max) return max;
    
    // Additional constraint: ensure editor panel maintains minimum width
    if (panel === 'file-tree') {
      const appContainer = document.querySelector('.app-container');
      const totalWidth = appContainer.getBoundingClientRect().width;
      const aiPanelWidth = this.getAiPanelWidth();
      const maxFileTreeWidth = totalWidth - this.minWidths.editor - aiPanelWidth - 20; // 20px for borders/margins
      
      if (width > maxFileTreeWidth) return maxFileTreeWidth;
    }
    
    return width;
  }

  setPanelWidth(panel, width) {
    const root = document.documentElement;
    
    switch (panel) {
      case 'file-tree':
        root.style.setProperty('--file-tree-default-width', `${width}px`);
        break;
      case 'ai-panel':
        root.style.setProperty('--ai-panel-default-width', `${width}px`);
        break;
    }
  }

  getAiPanelWidth() {
    const aiPanel = document.getElementById('aiPanel');
    if (!aiPanel || aiPanel.style.display === 'none') return 0;
    return aiPanel.getBoundingClientRect().width;
  }

  toggleFileTree() {
    const fileTreePanel = document.getElementById('fileTreePanel');
    const appContainer = document.querySelector('.app-container');
    
    fileTreePanel.classList.toggle('collapsed');
    
    // Update grid template to hide/show file tree
    if (fileTreePanel.classList.contains('collapsed')) {
      appContainer.style.gridTemplateColumns = '0 1fr';
    } else {
      const fileTreeWidth = getComputedStyle(document.documentElement)
        .getPropertyValue('--file-tree-default-width');
      appContainer.style.gridTemplateColumns = `${fileTreeWidth} 1fr`;
    }
  }

  toggleAiPanel() {
    const aiPanel = document.getElementById('aiPanel');
    const appContainer = document.querySelector('.app-container');
    
    if (aiPanel.style.display === 'none') {
      aiPanel.style.display = 'flex';
      appContainer.classList.add('show-ai-panel');
    } else {
      aiPanel.style.display = 'none';
      appContainer.classList.remove('show-ai-panel');
    }
  }

  handleWindowResize() {
    // Ensure panels maintain their constraints on window resize
    const fileTreePanel = document.getElementById('fileTreePanel');
    const currentWidth = fileTreePanel.getBoundingClientRect().width;
    const constrainedWidth = this.constrainWidth('file-tree', currentWidth);
    
    if (constrainedWidth !== currentWidth) {
      this.setPanelWidth('file-tree', constrainedWidth);
    }
  }

  handleKeydown(e) {
    // Keyboard shortcuts for layout management
    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case 'o':
        case 'O':
          e.preventDefault();
          selectVault();
          break;
        case 'n':
        case 'N':
          e.preventDefault();
          createNewFile();
          break;
        case 's':
        case 'S':
          e.preventDefault();
          saveFile();
          break;
        case 'e':
        case 'E':
          e.preventDefault();
          toggleViewMode();
          break;
      }
    }
    
    switch (e.key) {
      case 'F1':
        e.preventDefault();
        this.toggleFileTree();
        break;
      case '?':
        if (!e.ctrlKey && !e.metaKey) {
          e.preventDefault();
          toggleShortcutsHelp();
        }
        break;
    }
  }

  saveLayoutState() {
    const fileTreeWidth = getComputedStyle(document.documentElement)
      .getPropertyValue('--file-tree-default-width');
    const aiPanelWidth = getComputedStyle(document.documentElement)
      .getPropertyValue('--ai-panel-default-width');
    
    const layoutState = {
      fileTreeWidth: fileTreeWidth,
      aiPanelWidth: aiPanelWidth,
      fileTreeCollapsed: document.getElementById('fileTreePanel').classList.contains('collapsed'),
      aiPanelVisible: document.getElementById('aiPanel').style.display !== 'none'
    };
    
    try {
      localStorage.setItem('aiNote_layoutState', JSON.stringify(layoutState));
    } catch (error) {
      console.error('Failed to save layout state:', error);
    }
  }

  loadLayoutState() {
    try {
      const saved = localStorage.getItem('aiNote_layoutState');
      return saved ? JSON.parse(saved) : null;
    } catch (error) {
      console.error('Failed to load layout state:', error);
      return null;
    }
  }

  applyLayoutState(layoutState) {
    const root = document.documentElement;
    const fileTreePanel = document.getElementById('fileTreePanel');
    const aiPanel = document.getElementById('aiPanel');
    const appContainer = document.querySelector('.app-container');
    
    if (layoutState.fileTreeWidth) {
      root.style.setProperty('--file-tree-default-width', layoutState.fileTreeWidth);
    }
    
    if (layoutState.aiPanelWidth) {
      root.style.setProperty('--ai-panel-default-width', layoutState.aiPanelWidth);
    }
    
    if (layoutState.fileTreeCollapsed) {
      fileTreePanel.classList.add('collapsed');
      appContainer.style.gridTemplateColumns = '0 1fr';
    }
    
    if (layoutState.aiPanelVisible) {
      aiPanel.style.display = 'flex';
      appContainer.classList.add('show-ai-panel');
    }
  }
}

// Mobile Navigation Manager
class MobileNavManager {
  constructor() {
    this.isOpen = false;
    this.bindEvents();
  }

  bindEvents() {
    // Close on overlay click
    document.getElementById('mobileNavOverlay').addEventListener('click', (e) => {
      if (e.target === e.currentTarget) {
        this.close();
      }
    });
  }

  open() {
    if (this.isOpen) return;
    
    const overlay = document.getElementById('mobileNavOverlay');
    const navContent = document.getElementById('mobileNavContent');
    const fileTreeContent = document.getElementById('fileTreeContent');
    
    // Clone file tree content to mobile nav
    navContent.innerHTML = fileTreeContent.innerHTML;
    
    overlay.style.display = 'block';
    // Force reflow before adding active class for animation
    overlay.offsetHeight;
    overlay.classList.add('active');
    
    this.isOpen = true;
    document.body.style.overflow = 'hidden';
  }

  close() {
    if (!this.isOpen) return;
    
    const overlay = document.getElementById('mobileNavOverlay');
    
    overlay.classList.remove('active');
    setTimeout(() => {
      overlay.style.display = 'none';
    }, 250); // Match CSS transition duration
    
    this.isOpen = false;
    document.body.style.overflow = '';
  }
}

// Initialize layout managers
const layoutManager = new LayoutManager();
const mobileNavManager = new MobileNavManager();

/**
 * Update vault information display
 * @param {string|null} vaultPath - Path to the vault directory
 */
function updateVaultInfo(vaultPath) {
  const vaultInfo = document.getElementById('vaultInfo');
  const vaultPathSpan = vaultInfo.querySelector('.vault-path');
  
  if (vaultPath) {
    const displayPath = vaultPath.length > 50 ? 
      '...' + vaultPath.slice(-47) : vaultPath;
    vaultPathSpan.textContent = displayPath;
    vaultPathSpan.title = vaultPath; // Full path on hover
  } else {
    vaultPathSpan.textContent = 'No vault selected';
    vaultPathSpan.title = '';
  }
}

/**
 * Update current file name display
 * @param {string|null} fileName - Name of the current file
 * @param {boolean} isUnsaved - Whether the file has unsaved changes
 */
function updateCurrentFileName(fileName, isUnsaved = false) {
  const fileNameElement = document.getElementById('currentFileName');
  const fileStatusElement = document.getElementById('fileStatus');
  const saveBtn = document.getElementById('saveFileBtn');
  
  if (fileName) {
    fileNameElement.textContent = fileName;
    
    if (isUnsaved) {
      fileStatusElement.textContent = 'Unsaved';
      fileStatusElement.className = 'file-status unsaved';
      saveBtn.disabled = false;
    } else {
      fileStatusElement.textContent = 'Saved';
      fileStatusElement.className = 'file-status saved';
      saveBtn.disabled = true;
    }
  } else {
    fileNameElement.textContent = 'Welcome to aiNote';
    fileStatusElement.textContent = '';
    fileStatusElement.className = 'file-status';
    saveBtn.disabled = true;
  }
}

/**
 * Show notification message
 * @param {string} message - Message to display
 * @param {string} type - Type of notification (success, error, info)
 */
function showNotification(message, type = 'info') {
  // Create or update notification element
  let notification = document.getElementById('notification');
  if (!notification) {
    notification = document.createElement('div');
    notification.id = 'notification';
    notification.style.cssText = `
      position: fixed;
      top: 20px;
      right: 20px;
      padding: 12px 20px;
      border-radius: 8px;
      color: white;
      font-weight: 500;
      z-index: 1080;
      transition: all 0.3s ease;
      transform: translateX(100%);
    `;
    document.body.appendChild(notification);
  }
  
  // Set message and styling based on type
  notification.textContent = message;
  const colors = {
    success: '#10b981',
    error: '#ef4444',
    warning: '#f59e0b',
    info: '#3b82f6'
  };
  notification.style.backgroundColor = colors[type] || colors.info;
  
  // Show notification
  notification.style.transform = 'translateX(0)';
  
  // Auto-hide after 3 seconds
  setTimeout(() => {
    notification.style.transform = 'translateX(100%)';
  }, 3000);
}

// Vault Operations

/**
 * Select vault folder and update UI
 */
async function selectVault() {
  try {
    const result = await invoke('select_vault_folder');
    if (result) {
      appState.setVault(result);
      updateVaultInfo(result);
      showNotification(`Vault selected: ${result.split('/').pop()}`, 'success');
      
      // Automatically scan the vault
      await refreshVault();
    } else {
      showNotification('No vault selected', 'info');
    }
  } catch (error) {
    showNotification(`Error selecting vault: ${error}`, 'error');
  }
}

/**
 * Refresh vault files and update file tree
 */
async function refreshVault() {
  const currentVault = appState.getState().currentVault;
  if (!currentVault) {
    showNotification('Please select a vault folder first', 'warning');
    return;
  }
  
  try {
    const result = await invoke('scan_vault_files', { vaultPath: currentVault });
    appState.setFiles(result);
    
    // Update file tree display
    updateFileTree(result);
    
    const fileCount = result.filter(file => !file.is_dir).length;
    const dirCount = result.filter(file => file.is_dir).length;
    
    showNotification(`Refreshed: ${fileCount} files, ${dirCount} folders`, 'success');
  } catch (error) {
    showNotification(`Error scanning vault: ${error}`, 'error');
  }
}

/**
 * Update file tree display with files
 * @param {Array} files - Array of file objects
 */
function updateFileTree(files) {
  const fileTreeContent = document.getElementById('fileTreeContent');
  
  if (!files || files.length === 0) {
    fileTreeContent.innerHTML = `
      <div class="empty-state">
        <p>No files found in vault</p>
        <button onclick="refreshVault()" class="btn-secondary">Refresh</button>
      </div>
    `;
    return;
  }
  
  // Create file tree structure
  const treeHTML = createFileTreeHTML(files);
  fileTreeContent.innerHTML = `<div class="file-tree">${treeHTML}</div>`;
}

/**
 * Create HTML for file tree structure
 * @param {Array} files - Array of file objects
 * @returns {string} HTML string for file tree
 */
function createFileTreeHTML(files) {
  // Sort files: directories first, then files, both alphabetically
  const sortedFiles = files.sort((a, b) => {
    if (a.is_dir && !b.is_dir) return -1;
    if (!a.is_dir && b.is_dir) return 1;
    return a.name.localeCompare(b.name);
  });
  
  return sortedFiles.map(file => {
    const icon = file.is_dir ? '📁' : getFileIcon(file.name);
    const cssClass = file.is_dir ? 'tree-folder' : 'tree-file';
    const onClick = file.is_dir ? '' : `onclick="openFile('${file.path}')"`;
    
    return `
      <div class="tree-item ${cssClass}" ${onClick} title="${file.path}">
        <span class="tree-icon">${icon}</span>
        <span class="tree-name">${file.name}</span>
      </div>
    `;
  }).join('');
}

/**
 * Get appropriate icon for file type
 * @param {string} fileName - Name of the file
 * @returns {string} Unicode icon
 */
function getFileIcon(fileName) {
  const ext = fileName.split('.').pop().toLowerCase();
  const icons = {
    'md': '📝',
    'txt': '📄',
    'js': '🟨',
    'ts': '🔷',
    'html': '🌐',
    'css': '🎨',
    'json': '📋',
    'py': '🐍',
    'rs': '🦀',
    'go': '🐹',
    'jpg': '🖼️',
    'jpeg': '🖼️',
    'png': '🖼️',
    'gif': '🖼️',
    'svg': '🎨',
    'pdf': '📕'
  };
  return icons[ext] || '📄';
}

// File Operations

/**
 * Get the full path for a file (using selected vault or current directory)
 * @param {string} fileName - The file name
 * @returns {string} - Full file path
 */
function getFullPath(fileName) {
  const currentVault = appState.getState().currentVault;
  if (currentVault && !fileName.includes('/') && !fileName.includes('\\')) {
    return `${currentVault}/${fileName}`;
  }
  return fileName;
}

/**
 * Open a file in the editor
 * @param {string} filePath - Path to the file to open
 */
async function openFile(filePath) {
  try {
    const content = await invoke('read_file', { filePath });
    
    // Update state
    appState.setCurrentFile(filePath);
    
    // Update UI
    const fileName = filePath.split('/').pop();
    updateCurrentFileName(fileName, false);
    
    // Update editor content (placeholder for now)
    const editorContent = document.getElementById('editorContent');
    editorContent.innerHTML = `
      <div class="editor-wrapper">
        <textarea class="editor-textarea" placeholder="Start writing...">${content}</textarea>
      </div>
    `;
    
    // Add CSS for editor textarea
    if (!document.getElementById('editor-styles')) {
      const style = document.createElement('style');
      style.id = 'editor-styles';
      style.textContent = `
        .editor-wrapper {
          height: 100%;
          display: flex;
          flex-direction: column;
        }
        .editor-textarea {
          flex: 1;
          width: 100%;
          border: none;
          outline: none;
          padding: 1rem;
          font-family: var(--font-family-mono);
          font-size: 14px;
          line-height: 1.6;
          background: transparent;
          color: inherit;
          resize: none;
        }
      `;
      document.head.appendChild(style);
    }
    
    // Monitor for changes
    const textarea = editorContent.querySelector('.editor-textarea');
    textarea.addEventListener('input', () => {
      appState.markDirty(true);
      updateCurrentFileName(fileName, true);
    });
    
    showNotification(`Opened: ${fileName}`, 'success');
  } catch (error) {
    showNotification(`Error opening file: ${error}`, 'error');
  }
}

/**
 * Create a new file
 */
async function createNewFile() {
  const currentVault = appState.getState().currentVault;
  if (!currentVault) {
    showNotification('Please select a vault first', 'warning');
    return;
  }
  
  const fileName = prompt('Enter file name (with .md extension):');
  if (!fileName) return;
  
  if (!fileName.endsWith('.md')) {
    showNotification('File name must end with .md', 'warning');
    return;
  }
  
  const fullPath = getFullPath(fileName);
  
  try {
    await invoke('create_file', { filePath: fullPath });
    await invoke('write_file', { filePath: fullPath, content: `# ${fileName.replace('.md', '')}\n\n` });
    
    // Refresh file tree
    await refreshVault();
    
    // Open the new file
    await openFile(fullPath);
    
    showNotification(`Created: ${fileName}`, 'success');
  } catch (error) {
    showNotification(`Error creating file: ${error}`, 'error');
  }
}

/**
 * Save the current file
 */
async function saveFile() {
  const currentFile = appState.getState().currentFile;
  if (!currentFile) {
    showNotification('No file open to save', 'warning');
    return;
  }
  
  const textarea = document.querySelector('.editor-textarea');
  if (!textarea) {
    showNotification('No editor content to save', 'warning');
    return;
  }
  
  try {
    await invoke('write_file', { filePath: currentFile, content: textarea.value });
    
    // Update state
    appState.markDirty(false);
    
    // Update UI
    const fileName = currentFile.split('/').pop();
    updateCurrentFileName(fileName, false);
    
    showNotification(`Saved: ${fileName}`, 'success');
  } catch (error) {
    showNotification(`Error saving file: ${error}`, 'error');
  }
}

// Layout Control Functions

/**
 * Toggle file tree panel
 */
function toggleFileTree() {
  layoutManager.toggleFileTree();
}

/**
 * Toggle AI panel (for future use)
 */
function toggleAiPanel() {
  layoutManager.toggleAiPanel();
}

/**
 * Toggle view mode between editor and preview
 */
function toggleViewMode() {
  const newMode = appState.toggleViewMode();
  
  // Update button appearance
  const toggleBtn = document.getElementById('toggleModeBtn');
  toggleBtn.textContent = newMode === 'editor' ? '👁' : '✏️';
  toggleBtn.title = newMode === 'editor' ? 'Switch to preview' : 'Switch to editor';
  
  showNotification(`Switched to ${newMode} mode`, 'info');
}

/**
 * Open mobile navigation
 */
function openMobileNav() {
  mobileNavManager.open();
}

/**
 * Close mobile navigation
 */
function closeMobileNav() {
  mobileNavManager.close();
}

/**
 * Toggle keyboard shortcuts help
 */
function toggleShortcutsHelp() {
  const help = document.getElementById('shortcutsHelp');
  if (help.style.display === 'none' || !help.style.display) {
    help.style.display = 'flex';
    document.body.style.overflow = 'hidden';
  } else {
    help.style.display = 'none';
    document.body.style.overflow = '';
  }
}

// State management event listeners
appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, (data) => {
  console.log('State: Vault changed', data);
  updateVaultInfo(data.vault);
});

appState.addEventListener(AppState.EVENTS.FILES_UPDATED, (data) => {
  console.log('State: Files updated', data);
  updateFileTree(data.files);
});

appState.addEventListener(AppState.EVENTS.FILE_CHANGED, (data) => {
  console.log('State: Current file changed', data);
  if (data.file) {
    const fileName = data.file.split('/').pop();
    updateCurrentFileName(fileName, false);
  } else {
    updateCurrentFileName(null);
  }
});

appState.addEventListener(AppState.EVENTS.VIEW_MODE_CHANGED, (data) => {
  console.log('State: View mode changed', data);
  const toggleBtn = document.getElementById('toggleModeBtn');
  if (toggleBtn) {
    toggleBtn.textContent = data.mode === 'editor' ? '👁' : '✏️';
    toggleBtn.title = data.mode === 'editor' ? 'Switch to preview' : 'Switch to editor';
  }
});

appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
  console.log('State: Dirty state changed', data);
  if (data.file) {
    const fileName = data.file.split('/').pop();
    updateCurrentFileName(fileName, data.isDirty);
  }
});

// Make functions globally accessible for HTML onclick handlers
window.selectVault = selectVault;
window.refreshVault = refreshVault;
window.openFile = openFile;
window.createNewFile = createNewFile;
window.saveFile = saveFile;
window.toggleViewMode = toggleViewMode;
window.toggleFileTree = toggleFileTree;
window.toggleAiPanel = toggleAiPanel;
window.openMobileNav = openMobileNav;
window.closeMobileNav = closeMobileNav;
window.toggleShortcutsHelp = toggleShortcutsHelp;

// Initialize the application
window.addEventListener('DOMContentLoaded', () => {
  console.log('aiNote application initialized');
  
  // Add file tree styling
  const fileTreeStyles = document.createElement('style');
  fileTreeStyles.textContent = `
    .file-tree {
      padding: var(--space-2) 0;
    }
    
    .tree-item {
      display: flex;
      align-items: center;
      padding: var(--space-2) var(--space-4);
      cursor: pointer;
      border-radius: 0.375rem;
      margin: 0 var(--space-2);
      transition: background-color var(--transition-fast);
    }
    
    .tree-item:hover {
      background-color: var(--color-bg-hover);
    }
    
    .tree-item.tree-file:hover {
      background-color: var(--color-bg-accent);
      color: var(--color-text-inverse);
    }
    
    .tree-icon {
      margin-right: var(--space-2);
      font-size: var(--font-size-sm);
    }
    
    .tree-name {
      font-size: var(--font-size-sm);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    
    .tree-folder .tree-name {
      font-weight: 500;
    }
    
    .empty-state {
      padding: var(--space-6);
    }
    
    .empty-state p {
      margin-bottom: var(--space-4);
      color: var(--color-text-secondary);
    }
  `;
  document.head.appendChild(fileTreeStyles);
  
  // Load persisted state on startup
  const currentState = appState.getState();
  
  if (currentState.currentVault) {
    updateVaultInfo(currentState.currentVault);
    // Auto-refresh vault if one is persisted
    refreshVault();
  }
  
  if (currentState.currentFile) {
    const fileName = currentState.currentFile.split('/').pop();
    updateCurrentFileName(fileName, false);
  }
  
  // Initialize view mode button
  const toggleBtn = document.getElementById('toggleModeBtn');
  if (toggleBtn) {
    toggleBtn.textContent = currentState.viewMode === 'editor' ? '👁' : '✏️';
    toggleBtn.title = currentState.viewMode === 'editor' ? 'Switch to preview' : 'Switch to editor';
  }
  
  // Handle keyboard shortcuts help overlay click-outside
  const shortcutsHelp = document.getElementById('shortcutsHelp');
  if (shortcutsHelp) {
    shortcutsHelp.addEventListener('click', (e) => {
      if (e.target === shortcutsHelp) {
        toggleShortcutsHelp();
      }
    });
  }
  
  // Show welcome notification
  setTimeout(() => {
    showNotification('Welcome to aiNote! Select a vault to get started.', 'info');
  }, 1000);
});