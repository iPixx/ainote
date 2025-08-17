const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

// Import modules for application functionality
import AppState from './js/state.js';
import { LayoutManager, MobileNavManager } from './js/layout-manager.js';

// Initialize global application state
const appState = new AppState();

// Initialize layout managers (will be initialized after DOM load)
let layoutManager;
let mobileNavManager;

// Window instance for state management
let mainWindow;

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
      top: 70px;
      right: 20px;
      padding: 12px 20px;
      border-radius: 8px;
      color: white;
      font-weight: 500;
      z-index: 9999;
      transition: all 0.3s ease;
      transform: translateX(400px);
      min-width: 200px;
      max-width: 350px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      backdrop-filter: blur(8px);
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
  
  // Auto-hide after 4 seconds
  setTimeout(() => {
    notification.style.transform = 'translateX(400px)';
  }, 4000);
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
  if (layoutManager) {
    layoutManager.toggleFileTree();
  }
}

/**
 * Toggle AI panel (temporary toggle for testing)
 */
function toggleAiPanel() {
  if (layoutManager) {
    layoutManager.toggleAiPanel();
  }
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

// Window and Layout State Management

/**
 * Load and apply saved window state
 */
async function loadWindowState() {
  try {
    const appState = await invoke('load_app_state');
    
    if (mainWindow && appState && appState.window) {
      const { width, height, x, y, maximized } = appState.window;
      
      console.log('📥 Loading window state:', appState.window);
      
      // Ensure window is not maximized before setting size/position
      if (await mainWindow.isMaximized()) {
        await mainWindow.unmaximize();
      }
      
      // Set window size with validation (increased limits for larger screens)
      const validWidth = Math.max(800, Math.min(width || 1920, 6000));
      const validHeight = Math.max(600, Math.min(height || 1080, 4000));
      
      await mainWindow.setSize({ width: validWidth, height: validHeight });
      
      // Set position if available and valid
      if (x !== null && y !== null && x !== undefined && y !== undefined) {
        // Ensure position is on screen (basic validation) - increased for larger screens
        const validX = Math.max(-200, Math.min(x, 4000)); // Allow some off-screen
        const validY = Math.max(-200, Math.min(y, 3000));
        await mainWindow.setPosition({ x: validX, y: validY });
      }
      
      // Apply maximized state last
      if (maximized) {
        await mainWindow.maximize();
      }
      
      console.log('✅ Window state restored successfully');
    } else {
      console.log('📋 No saved window state found, using defaults');
    }
    
    return appState;
  } catch (error) {
    console.warn('⚠️ Failed to load window state:', error);
    return null;
  }
}

/**
 * Save current window state
 */
async function saveWindowState() {
  try {
    if (!mainWindow) {
      console.warn('⚠️ No main window available for saving state');
      return;
    }
    
    console.log('🔄 Getting window properties...');
    
    const size = await mainWindow.innerSize();
    console.log('📐 Window size:', size);
    
    const position = await mainWindow.outerPosition();
    console.log('📍 Window position:', position);
    
    const isMaximized = await mainWindow.isMaximized();
    console.log('🔳 Window maximized:', isMaximized);
    
    const windowState = {
      width: size.width,
      height: size.height,
      x: position.x,
      y: position.y,
      maximized: isMaximized
    };
    
    console.log('💾 Saving window state:', windowState);
    
    await invoke('save_window_state', windowState);
    
    console.log('✅ Window state saved successfully');
  } catch (error) {
    console.error('❌ Failed to save window state:', error);
    console.error('Error details:', error);
  }
}

/**
 * Save layout state (column widths and visibility)
 */
async function saveLayoutState(layoutState) {
  try {
    await invoke('save_layout_state', {
      fileTreeWidth: layoutState.fileTreeWidth,
      aiPanelWidth: layoutState.aiPanelWidth,
      fileTreeVisible: layoutState.fileTreeVisible,
      aiPanelVisible: layoutState.aiPanelVisible,
      editorMode: layoutState.editorMode
    });
    
    console.log('💾 Layout state saved:', layoutState);
  } catch (error) {
    console.warn('⚠️ Failed to save layout state:', error);
  }
}

/**
 * Debounced save function to avoid too frequent saves
 */
const debouncedSaveWindowState = debounce(saveWindowState, 500); // Reduced delay for better responsiveness
const debouncedSaveLayoutState = debounce(saveLayoutState, 300);

/**
 * Force save all application state (useful for manual triggers)
 */
async function forceSaveAllState() {
  try {
    console.log('🔄 Force saving all application state...');
    
    await saveWindowState();
    
    if (layoutManager && layoutManager.getCurrentLayoutState) {
      const layoutState = layoutManager.getCurrentLayoutState();
      await saveLayoutState(layoutState);
    }
    
    console.log('✅ All application state saved successfully');
  } catch (error) {
    console.error('❌ Failed to save application state:', error);
  }
}

/**
 * Debounce utility function
 */
function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
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
window.forceSaveAllState = forceSaveAllState;
window.debouncedSaveLayoutState = debouncedSaveLayoutState;

// Development and testing functions
window.runLayoutTest = function() {
  if (layoutManager && typeof layoutManager.runLayoutTest === 'function') {
    return layoutManager.runLayoutTest();
  } else {
    console.error('❌ Layout manager not available or test method missing');
    return null;
  }
};

// Initialize the application
window.addEventListener('DOMContentLoaded', async () => {
  console.log('🚀 aiNote application initializing...');
  
  // Test basic Tauri availability
  console.log('🔍 Tauri available:', !!window.__TAURI__);
  console.log('🔍 Tauri core:', !!window.__TAURI__?.core);
  console.log('🔍 Tauri window:', !!window.__TAURI__?.window);
  
  // Initialize window instance
  try {
    mainWindow = getCurrentWindow();
    console.log('🪟 Main window instance:', mainWindow);
    console.log('🪟 Main window type:', typeof mainWindow);
  } catch (error) {
    console.error('❌ Failed to get current window:', error);
  }
  
  // Longer delay to ensure window is fully ready on all platforms
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Load saved window and layout state
  console.log('📥 Loading saved application state...');
  const savedAppState = await loadWindowState();
  
  // Additional delay after loading state to ensure changes are applied
  await new Promise(resolve => setTimeout(resolve, 200));
  
  // Initialize layout managers after DOM is ready
  layoutManager = new LayoutManager();
  mobileNavManager = new MobileNavManager();
  
  // Apply saved layout state if available
  if (savedAppState && savedAppState.layout) {
    if (layoutManager.applyLayoutState) {
      layoutManager.applyLayoutState(savedAppState.layout);
    }
  }
  
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
  
  // Set up window state persistence event listeners
  if (mainWindow) {
    console.log('🔧 Setting up window event listeners...');
    
    try {
      // Save window state when it changes
      const resizeUnlisten = await mainWindow.listen('tauri://resize', (event) => {
        console.log('📏 Window resized:', event.payload);
        debouncedSaveWindowState();
      });
      
      const moveUnlisten = await mainWindow.listen('tauri://move', (event) => {
        console.log('📍 Window moved:', event.payload);
        debouncedSaveWindowState();
      });
      
      // Save state before the window closes
      const closeUnlisten = await mainWindow.listen('tauri://close-requested', async (event) => {
        console.log('💾 Window close requested - saving state...');
        
        try {
          // Save state without debouncing for immediate persistence
          await forceSaveAllState();
          console.log('✅ State saved successfully before close');
        } catch (error) {
          console.error('❌ Error saving state before close:', error);
        }
        
        // Don't prevent the close - let Tauri handle it normally
      });

      console.log('✅ Window event listeners set up successfully');
      
      // Store unlisteners for cleanup if needed
      window.windowEventUnlisteners = { resizeUnlisten, moveUnlisten, closeUnlisten };
      
    } catch (error) {
      console.error('❌ Failed to set up window event listeners:', error);
    }
    
    // Listen for beforeunload to save state when page unloads
    window.addEventListener('beforeunload', async (event) => {
      console.log('🚪 Page unloading - saving state...');
      try {
        await forceSaveAllState();
      } catch (error) {
        console.warn('Failed to save state on beforeunload:', error);
      }
    });
  } else {
    console.error('❌ Main window instance not available');
  }
  

  // Show welcome notification
  setTimeout(() => {
    showNotification('Welcome to aiNote! Select a vault to get started.', 'info');
  }, 1000);
});