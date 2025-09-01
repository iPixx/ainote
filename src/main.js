const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

// Import modules for application functionality
import AppState from './js/state.js';
import { LayoutManager, MobileNavManager } from './js/layout-manager.js';
import FileTree from './js/components/file-tree.js';
import EditorPreviewPanel from './js/components/editor-preview-panel.js';
import AiStatusPanel from './js/components/ai-status-panel.js';
import AiPanel from './js/components/ai-panel.js';
import AiPanelController from './js/components/ai-panel-controller.js';
import { PerformanceMonitoringDashboard } from './js/components/performance-monitoring-dashboard.js';
import { realTimeMetricsService } from './js/services/real-time-metrics-service.js';
import AiSuggestionService from './js/services/ai-suggestion-service.js';
import ContentChangeDetector from './js/services/content-change-detector.js';
import SuggestionCacheManager from './js/services/suggestion-cache-manager.js';
import OllamaConnectionMonitor from './js/services/ollama-connection-monitor.js';

// Initialize global application state
const appState = new AppState();

// Initialize layout managers (will be initialized after DOM load)
let layoutManager;
let mobileNavManager;
let fileTreeComponent;
let editorPreviewPanel;
let aiStatusPanel;
let aiPanel; // Basic AI panel for Phase 2A
let aiPanelController; // Enhanced AI panel controller for Phase 2B+
let performanceDashboard;

// Initialize service instances
let vaultManager;
let autoSave;
let ollamaConnectionMonitor;

// AI pipeline initialization state
let aiPipelineInitialized = false;

// Window instance for state management
let mainWindow;

// Application start time for suppressing startup notifications
let appStartTime = Date.now();

// File opening state to prevent double-clicks
let isFileOpening = false;

/**
 * Update vault information display
 * @param {string|null} vaultPath - Path to the vault directory
 */
function updateVaultInfo(vaultPath) {
  const vaultInfo = document.getElementById('vaultInfo');
  const vaultPathSpan = vaultInfo.querySelector('.vault-path');
  
  if (vaultPath) {
    // Extract just the folder name from the full path
    const folderName = vaultPath.split('/').pop() || vaultPath.split('\\').pop();
    vaultPathSpan.textContent = folderName;
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

/**
 * Initialize AI Panel Controller when editor becomes available
 * @param {MarkdownEditor} markdownEditor - The markdown editor instance
 */
async function initializeAiPanelController(markdownEditor) {
  if (aiPipelineInitialized || !markdownEditor) {
    return;
  }
  
  try {
    console.log('🤖 Initializing AI Panel Controller with suggestion pipeline...');
    
    const aiPanelElement = document.getElementById('aiPanel');
    if (!aiPanelElement || !layoutManager) {
      console.warn('⚠️ AI Panel element or layout manager not available');
      return;
    }
    
    // Initialize the enhanced AI Panel Controller with the markdown editor
    aiPanelController = new AiPanelController(
      aiPanelElement, 
      markdownEditor,
      appState, 
      layoutManager,
      fileTreeComponent, // Pass file tree for navigation
      editorPreviewPanel  // Pass editor panel for navigation
    );
    
    // Listen to enhanced AI panel events
    aiPanelController.addEventListener(AiPanelController.EVENTS.PANEL_ACTIVATED, (event) => {
      console.log('🚀 AI Panel activated:', event.detail);
      showNotification('AI Assistant ready with suggestions', 'success');
    });
    
    aiPanelController.addEventListener(AiPanelController.EVENTS.PANEL_DEACTIVATED, (event) => {
      console.log('🔄 AI Panel deactivated:', event.detail);
    });
    
    aiPanelController.addEventListener(AiPanelController.EVENTS.SUGGESTIONS_READY, (event) => {
      console.log('✅ AI Suggestions ready:', event.detail);
    });
    
    aiPanelController.addEventListener(AiPanelController.EVENTS.SUGGESTION_INSERTED, (event) => {
      console.log('📝 Suggestion inserted:', event.detail);
      showNotification('Content inserted from AI suggestion', 'success');
    });
    
    aiPanelController.addEventListener(AiPanelController.EVENTS.SERVICE_ERROR, (event) => {
      console.warn('⚠️ AI Service error:', event.detail);
      showNotification('AI service error: ' + event.detail.message, 'error');
    });
    
    // Make AI panel controller globally accessible
    window.aiPanelController = aiPanelController;
    window.aiPanel = aiPanelController;
    
    // Access the initialized services from the controller
    window.aiSuggestionService = aiPanelController.suggestionService;
    window.contentChangeDetector = aiPanelController.contentDetector;
    window.suggestionCacheManager = aiPanelController.cacheManager;
    
    // Integrate AI services with connection monitoring
    if (ollamaConnectionMonitor && aiPanelController.suggestionService) {
      console.log('🔗 Integrating AI suggestion service with connection monitoring...');
      
      // Listen to connection status changes to enable/disable AI features
      ollamaConnectionMonitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, (data) => {
        const { currentStatus } = data;
        const isConnected = currentStatus === OllamaConnectionMonitor.STATUS.CONNECTED;
        
        // Enable/disable suggestion service based on connection
        if (aiPanelController.suggestionService) {
          aiPanelController.suggestionService.setEnabled(isConnected);
          
          if (isConnected) {
            console.log('✅ AI suggestions enabled - Ollama connected');
          } else {
            console.log('⚠️ AI suggestions disabled - Ollama disconnected');
            aiPanelController.suggestionService.clearSuggestions();
          }
        }
      });
      
      // Listen to model status for enhanced functionality
      ollamaConnectionMonitor.addEventListener(OllamaConnectionMonitor.EVENTS.MODEL_STATUS_UPDATED, (data) => {
        if (data.modelName === 'nomic-embed-text' && aiPanelController.suggestionService) {
          const config = {
            ENABLED: data.isAvailable && data.isCompatible,
            MODEL_NAME: data.modelName
          };
          
          aiPanelController.suggestionService.updateConfig(config);
          console.log('📦 AI suggestion service updated for model status:', data);
        }
      });
      
      console.log('✅ AI services integrated with connection monitoring');
    }
    
    aiPipelineInitialized = true;
    
    console.log('✅ AI Panel Controller initialized with full suggestion pipeline');
    showNotification('AI-powered suggestions are now active', 'success');
    
  } catch (error) {
    console.error('❌ Failed to initialize AI Panel Controller:', error);
    showNotification('Failed to activate AI suggestions', 'error');
  }
}

// Vault Operations

/**
 * Select vault folder and update UI
 */
async function selectVault() {
  console.log('📁 Starting vault selection...');
  try {
    const result = await invoke('select_vault');
    console.log('📁 Vault selection result:', result);
    
    if (result) {
      console.log('✅ Vault selected:', result);
      appState.setVault(result);
      updateVaultInfo(result);
      updateVaultStatusBar(result);
      
      // Close vault dialog
      const vaultDialog = document.getElementById('vaultDialog');
      if (vaultDialog) {
        vaultDialog.style.display = 'none';
        console.log('📁 Vault dialog closed');
      }
      
      showNotification(`Vault selected: ${result.split('/').pop()}`, 'success');
      
      // Automatically scan the vault
      await refreshVault();
    } else {
      console.log('ℹ️ Vault selection cancelled');
      showNotification('No vault selected', 'info');
    }
  } catch (error) {
    console.error('❌ Error selecting vault:', error);
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
    
    // Trigger indexing for vector generation
    console.log('🚀 Starting vault indexing for vector generation...');
    try {
      const requestIds = await invoke('index_vault_notes', { 
        vaultPath: currentVault,
        filePattern: null, // Use default pattern (**/*.md)
        priority: 'UserTriggered'
      });
      console.log(`✅ Started indexing with ${requestIds.length} files queued for processing`);
      showNotification(`Indexing started: ${requestIds.length} files queued`, 'info');
    } catch (indexError) {
      console.error('❌ Failed to start indexing:', indexError);
      showNotification(`Indexing failed: ${indexError}`, 'warning');
    }
  } catch (error) {
    showNotification(`Error scanning vault: ${error}`, 'error');
  }
}

/**
 * Update file tree display with files using FileTree component
 * @param {Array} files - Array of file objects
 */
function updateFileTree(files) {
  if (fileTreeComponent) {
    fileTreeComponent.render(files);
  } else {
    // Fallback to old method if component not initialized
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
    // Remove onclick handler - let FileTree component handle clicks
    
    return `
      <div class="tree-item ${cssClass}" data-file-path="${file.path}" data-is-dir="${file.is_dir}" title="${file.path}">
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
 * Handle file move operation from drag-and-drop
 * @param {Object} sourceFile - File being moved
 * @param {Object} targetFolder - Target folder
 * @param {string} newPath - New file path
 */
async function handleFileMove(sourceFile, targetFolder, newPath) {
  try {
    // Check if source file still exists by trying to get its info
    try {
      await invoke('get_file_info', { filePath: sourceFile.path });
    } catch {
      throw new Error('Source file no longer exists');
    }
    
    // Move the file using rename_file command (which is essentially move)
    await invoke('rename_file', { 
      oldPath: sourceFile.path, 
      newPath: newPath 
    });
    
    // Update current file path if it was the moved file
    const currentFile = appState.getState().currentFile;
    if (currentFile === sourceFile.path) {
      appState.setCurrentFile(newPath);
    }
    
    // Refresh the file tree to show the change
    await refreshVault();
    
    showNotification(`Moved ${sourceFile.name} to ${targetFolder.name}`, 'success');
    
  } catch (error) {
    console.error('Failed to move file:', error);
    throw error; // Re-throw so the FileTree component can handle it
  }
}

/**
 * Activate file tree search
 */
function activateFileTreeSearch() {
  if (fileTreeComponent && fileTreeComponent.activateSearch) {
    fileTreeComponent.activateSearch();
  }
}

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
  if (isFileOpening) {
    console.log('File opening already in progress, skipping');
    return;
  }
  
  isFileOpening = true;
  try {
    console.log('Opening file:', filePath);
    const content = await invoke('read_file', { filePath });
    
    // Update state
    appState.setCurrentFile(filePath);
    
    // Update UI
    const fileName = filePath.split('/').pop();
    updateCurrentFileName(fileName, false);
    
    // Initialize or update editor/preview panel
    const editorContent = document.getElementById('editorContent');
    
    if (!editorPreviewPanel) {
      // Create new editor/preview panel instance
      editorPreviewPanel = new EditorPreviewPanel(editorContent, appState);
      
      // Initialize the panel
      editorPreviewPanel.init();
      
      // Initialize AI Panel Controller with the markdown editor
      if (editorPreviewPanel.markdownEditor) {
        await initializeAiPanelController(editorPreviewPanel.markdownEditor);
      } else {
        // Wait for editor to be fully initialized
        setTimeout(async () => {
          if (editorPreviewPanel.markdownEditor) {
            await initializeAiPanelController(editorPreviewPanel.markdownEditor);
          }
        }, 100);
      }
      
      // Listen for content changes from the editor component within the panel
      editorPreviewPanel.addEventListener('content_changed', () => {
        appState.markDirty(true);
        updateSaveStatus('unsaved');
        
        const currentFileName = appState.getState().currentFile?.split('/').pop();
        if (currentFileName) {
          updateCurrentFileName(currentFileName, true);
        }
        
        // Trigger auto-save if initialized
        if (autoSave) {
          autoSave.handleContentChange();
        }
      });
      
      // Listen for save requests from keyboard shortcuts
      editorPreviewPanel.addEventListener('save_requested', () => {
        saveFile();
      });
      
      // Listen for auto-save requests from the editor component
      editorPreviewPanel.addEventListener('auto_save_requested', async (event) => {
        // Auto-save is now handled by the AutoSave service
        // This event is kept for compatibility but delegates to AutoSave
        if (autoSave) {
          const content = event.detail?.content || editorPreviewPanel.getContent();
          autoSave.handleContentChange(content);
        }
      });
      
      // Listen for performance events
      editorPreviewPanel.addEventListener('large_document_detected', (event) => {
        const { size } = event.detail;
        showNotification(`Large document detected (${(size / 1024).toFixed(1)}KB) - optimizations enabled`, 'info');
      });
      
      // Listen for mode changes
      editorPreviewPanel.addEventListener(EditorPreviewPanel.EVENTS.MODE_CHANGED, (event) => {
        console.log(`🔄 Panel mode changed to: ${event.detail.mode}`);
        updateViewModeDisplay();
      });
      
      console.log('✅ EditorPreviewPanel initialized for file editing');
    }
    
    // Set the content in the panel (handles both editor and preview)
    editorPreviewPanel.setContent(content);

    // Update view mode display based on current state
    updateViewModeDisplay();
    
    // Show success notification only if not restoring from startup
    if (Date.now() - appStartTime > 3000) {
      showNotification(`Opened: ${fileName}`, 'success');
    }
    
    console.log('✅ File opened successfully:', fileName);
  } catch (error) {
    console.error('Error opening file:', error);
    showNotification(`Error opening file: ${error}`, 'error');
  } finally {
    // Reduce the timeout to allow faster subsequent file opening
    setTimeout(() => {
      isFileOpening = false;
    }, 100);
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
  
  if (!editorPreviewPanel) {
    showNotification('No editor content to save', 'warning');
    return;
  }
  
  const content = editorPreviewPanel.getContent();
  
  try {
    await invoke('write_file', { filePath: currentFile, content });
    
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
 * Toggle AI panel
 */
function toggleAiPanel() {
  if (layoutManager) {
    layoutManager.toggleAiPanel();
  }
}

/**
 * Update AI panel header based on connection status
 * @param {string} status - Current AI connection status
 */
function updateAiPanelHeader(status) {
  const aiPanelHeader = document.querySelector('.ai-panel .panel-header h2');
  if (aiPanelHeader) {
    const statusEmojis = {
      'Connected': '🤖 AI Assistant',
      'Disconnected': '🔴 AI Assistant',
      'Connecting': '🟡 AI Assistant',
      'Retrying': '🟠 AI Assistant',
      'Failed': '❌ AI Assistant'
    };
    
    aiPanelHeader.textContent = statusEmojis[status] || '🤖 AI Assistant';
  }
}

/**
 * Conditionally show/hide AI panel based on connection status
 * @param {string} status - Current AI connection status
 */
function conditionallyShowAiPanel(status) {
  // For now, we always keep the panel available for configuration
  // In future versions, we might hide it completely when disconnected
  
  const aiPanel = document.getElementById('aiPanel');
  if (aiPanel) {
    // Update panel appearance based on status
    aiPanel.classList.remove('ai-connected', 'ai-disconnected', 'ai-connecting', 'ai-retrying', 'ai-failed');
    aiPanel.classList.add(`ai-${status.toLowerCase()}`);
  }
}

/**
 * Toggle view mode between editor and preview
 */
function toggleViewMode() {
  if (editorPreviewPanel) {
    editorPreviewPanel.toggleMode();
  } else {
    // Fallback to AppState toggle if panel not initialized
    const newMode = appState.toggleViewMode();
    updateViewModeDisplay();
    showNotification(`Switched to ${newMode} mode`, 'info');
  }
}

/**
 * Update the view mode display (editor vs preview)
 */
function updateViewModeDisplay() {
  // The EditorPreviewPanel handles its own display updates
  // This function is kept for compatibility but may be used for additional UI updates
  const currentMode = appState.getState().viewMode;
  const toggleBtn = document.getElementById('toggleModeBtn');
  
  // Update button appearance (EditorPreviewPanel also updates this, but ensure consistency)
  if (toggleBtn) {
    toggleBtn.textContent = currentMode === 'editor' ? '👁' : '✏️';
    toggleBtn.title = currentMode === 'editor' ? 'Switch to preview (Ctrl+Shift+P)' : 'Switch to editor (Ctrl+Shift+P)';
  }
  
  console.log(`📋 View mode display updated: ${currentMode}`);
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
      
      await mainWindow.setSize({ 
        type: 'Logical', 
        width: validWidth, 
        height: validHeight 
      });
      
      // Set position if available and valid
      if (x !== null && y !== null && x !== undefined && y !== undefined) {
        // Ensure position is on screen (basic validation) - increased for larger screens
        const validX = Math.max(-200, Math.min(x, 4000)); // Allow some off-screen
        const validY = Math.max(-200, Math.min(y, 3000));
        await mainWindow.setPosition({ 
          type: 'Logical', 
          x: validX, 
          y: validY 
        });
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
    console.log('📐 Window size (physical):', size);
    
    const position = await mainWindow.outerPosition();
    console.log('📍 Window position (physical):', position);
    
    const isMaximized = await mainWindow.isMaximized();
    console.log('🔳 Window maximized:', isMaximized);
    
    const scaleFactor = await mainWindow.scaleFactor();
    console.log('🔍 Scale factor:', scaleFactor);
    
    // Convert physical pixels to logical pixels to match backend coordinate system
    const logicalSize = {
      width: size.width / scaleFactor,
      height: size.height / scaleFactor
    };
    
    const logicalPosition = {
      x: Math.round(position.x / scaleFactor),
      y: Math.round(position.y / scaleFactor)
    };
    
    console.log('📐 Window size (logical):', logicalSize);
    console.log('📍 Window position (logical):', logicalPosition);
    
    const windowState = {
      width: logicalSize.width,
      height: logicalSize.height,
      x: logicalPosition.x,
      y: logicalPosition.y,
      maximized: isMaximized
    };
    
    console.log('💾 Saving window state (logical pixels):', windowState);
    
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
 * Now uses unified JSON file storage only
 */
async function forceSaveAllState() {
  try {
    console.log('🔄 Force saving all application state...');
    
    await saveWindowState();
    
    if (layoutManager && layoutManager.saveState) {
      await layoutManager.saveState();
    }
    
    if (appState && appState.saveState) {
      await appState.saveState();
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
  
  // Update save status indicator
  updateSaveStatus(data.isDirty ? 'unsaved' : 'saved');
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
window.activateFileTreeSearch = activateFileTreeSearch;
window.saveWindowState = saveWindowState;
window.showNotification = showNotification;
window.togglePerformanceDashboard = togglePerformanceDashboard;

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
  
  // Load saved window and layout state from unified JSON file
  console.log('📥 Loading saved application state...');
  const savedAppState = await loadWindowState();
  
  // Additional delay after loading state to ensure changes are applied
  await new Promise(resolve => setTimeout(resolve, 200));
  
  // Initialize layout managers after DOM is ready
  layoutManager = new LayoutManager();
  mobileNavManager = new MobileNavManager();
  
  // Initialize service instances
  try {
    // Import services dynamically to ensure modules are loaded
    const VaultManagerModule = await import('./js/services/vault-manager.js');
    const AutoSaveModule = await import('./js/services/auto-save.js');
    
    // Store class references for static access
    const AutoSave = AutoSaveModule.default;
    
    vaultManager = new VaultManagerModule.default(appState);
    autoSave = new AutoSave(appState);
    
    // Initialize Ollama Connection Monitor
    ollamaConnectionMonitor = new OllamaConnectionMonitor();
    
    // Make services and class globally accessible
    window.vaultManager = vaultManager;
    window.autoSave = autoSave;
    window.AutoSave = AutoSave; // Make AutoSave class globally accessible
    window.ollamaConnectionMonitor = ollamaConnectionMonitor;
    
    console.log('✅ VaultManager, AutoSave, and OllamaConnectionMonitor services initialized');
  } catch (error) {
    console.error('❌ Failed to initialize services:', error);
    showNotification('Failed to initialize application services', 'error');
    
    // Fallback: still try to show the proper UI state
    const initialState = appState.getState();
    if (initialState.currentVault) {
      updateVaultInfo(initialState.currentVault);
      updateVaultStatusBar(initialState.currentVault);
    }
  }
  
  // Initialize FileTree component
  const fileTreeContent = document.getElementById('fileTreeContent');
  if (fileTreeContent) {
    fileTreeComponent = new FileTree(fileTreeContent, appState);
    
    // Listen to file selection events from the tree
    fileTreeContent.addEventListener(FileTree.EVENTS.FILE_SELECTED, async (event) => {
      const { filePath } = event.detail;
      await openFile(filePath);
    });
    
    // Listen to file move requests from drag-and-drop
    fileTreeContent.addEventListener(FileTree.EVENTS.FILE_MOVE_REQUESTED, async (event) => {
      const { sourceFile, targetFolder, newPath } = event.detail;
      try {
        await handleFileMove(sourceFile, targetFolder, newPath);
      } catch (error) {
        showNotification(`Failed to move ${sourceFile.name}: ${error.message}`, 'error');
      }
    });
    
    // Listen to drag events for feedback
    fileTreeContent.addEventListener(FileTree.EVENTS.DRAG_START, (event) => {
      showNotification(`Moving ${event.detail.file.name}...`, 'info');
    });
    
    fileTreeContent.addEventListener(FileTree.EVENTS.DRAG_END, (event) => {
      // Drag operation completed (success handled by move event)
    });
    
    console.log('🌳 FileTree component initialized with advanced features');
  } else {
    console.warn('⚠️ FileTree container not found');
  }
  
  // Initialize AI Status Panel component
  const aiContent = document.getElementById('aiContent');
  if (aiContent) {
    aiStatusPanel = new AiStatusPanel(aiContent);
    
    // Listen to AI status panel events
    aiStatusPanel.addEventListener(AiStatusPanel.EVENTS.STATUS_CHANGED, (event) => {
      const { status, connectionState } = event.detail;
      console.log('🤖 AI Status changed:', status, connectionState);
      
      // Update AI panel header based on connection status
      updateAiPanelHeader(status);
      
      // Show/hide AI panel conditionally based on status
      conditionallyShowAiPanel(status);
    });
    
    aiStatusPanel.addEventListener(AiStatusPanel.EVENTS.CONNECTION_REQUESTED, (event) => {
      const { action } = event.detail;
      console.log('🔄 AI Connection requested:', action);
      showNotification(`Retrying AI connection...`, 'info');
    });
    
    aiStatusPanel.addEventListener(AiStatusPanel.EVENTS.SETTINGS_CHANGED, (event) => {
      const { baseUrl } = event.detail;
      console.log('⚙️ AI Settings changed:', baseUrl);
      showNotification(`AI service URL updated: ${baseUrl}`, 'success');
    });
    
    console.log('🤖 AI Status Panel initialized');
    
    // Make AI status panel globally accessible for debugging
    window.aiStatusPanel = aiStatusPanel;
    
    // Start Ollama connection monitoring and integrate with AI status panel
    if (ollamaConnectionMonitor) {
      console.log('🚀 Starting Ollama connection monitoring...');
      
      // Set up event listeners for connection monitoring
      ollamaConnectionMonitor.addEventListener(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, (data) => {
        console.log('📊 Ollama connection status changed:', data);
        // The AI Status Panel will handle its own status checks, but we can log here
      });
      
      ollamaConnectionMonitor.addEventListener(OllamaConnectionMonitor.EVENTS.MODEL_STATUS_UPDATED, (data) => {
        console.log('📦 Model status updated:', data);
        showNotification(`Model ${data.modelName}: ${data.isAvailable ? 'Available' : 'Not Available'}`, 
                        data.isAvailable ? 'success' : 'warning');
      });
      
      ollamaConnectionMonitor.addEventListener(OllamaConnectionMonitor.EVENTS.ERROR_OCCURRED, (data) => {
        console.warn('⚠️ Ollama monitor error:', data);
        if (data.type !== 'health_check_failed') {
          showNotification(`AI Service Error: ${data.message}`, 'error');
        }
      });
      
      ollamaConnectionMonitor.addEventListener(OllamaConnectionMonitor.EVENTS.RECONNECTION_ATTEMPT, (data) => {
        console.log('🔄 Ollama reconnection attempt:', data);
        showNotification(`Reconnecting to AI service... (${data.attempt}/${data.maxAttempts})`, 'info');
      });
      
      // Start the monitoring service
      ollamaConnectionMonitor.start().catch(error => {
        console.error('❌ Failed to start Ollama monitoring:', error);
        showNotification('Failed to start AI service monitoring', 'warning');
      });
      
      console.log('✅ Ollama connection monitoring started');
    }
    
  } else {
    console.warn('⚠️ AI content container not found');
  }
  
  // AI Panel Controller will be initialized when editor becomes available
  // This ensures the suggestion pipeline activates with real-time content detection
  
  // Apply saved layout state if available
  if (savedAppState && savedAppState.layout) {
    if (layoutManager.applyLayoutState) {
      layoutManager.applyLayoutState(savedAppState.layout);
    }
  }
  
  // FileTree component styling is now loaded via CSS file
  
  // Load persisted state on startup will be handled after vault manager initialization
  // This section moved below to avoid variable conflicts
  
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
          // Stop monitoring services
          if (ollamaConnectionMonitor) {
            ollamaConnectionMonitor.stop();
            console.log('🛑 Ollama monitoring stopped');
          }
          
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
  
  // Setup auto-save integration with editor (delayed until editor is ready)
  const setupAutoSaveIntegration = () => {
    if (autoSave && editorPreviewPanel) {
      // Set up content getter for auto-save
      autoSave.setContentGetter(() => {
        return editorPreviewPanel ? editorPreviewPanel.getContent() : null;
      });
      
      // Listen for auto-save events
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_SUCCESS, (event) => {
        const { saveType, saveTime } = event;
        updateSaveStatus('saved');
        updateOperationStatus('');
        if (saveType === 'manual') {
          showNotification(`File saved (${saveTime.toFixed(0)}ms)`, 'success', 2000);
        }
      });
      
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_ERROR, (event) => {
        const { error, saveType } = event;
        updateSaveStatus('error');
        updateOperationStatus('');
        showNotification(`Save failed: ${error}`, 'error');
      });
      
      autoSave.addEventListener(AutoSave.EVENTS.SAVE_STARTED, (event) => {
        const { saveType } = event;
        updateSaveStatus('saving');
        if (saveType === 'manual') {
          updateOperationStatus('Saving file...');
        }
      });
      
      console.log('✅ Auto-save integration configured');
      return true;
    }
    return false;
  };
  
  // Try to setup auto-save integration now, or retry later
  if (!setupAutoSaveIntegration()) {
    // Retry when editor becomes available
    const retryIntegration = setInterval(() => {
      if (setupAutoSaveIntegration()) {
        clearInterval(retryIntegration);
      }
    }, 500);
    
    // Give up after 10 seconds
    setTimeout(() => {
      clearInterval(retryIntegration);
    }, 10000);
  }
  
  // Add keyboard shortcut handling for macOS
  document.addEventListener('keydown', async (event) => {
    // Handle Ctrl/Cmd+O for vault selection
    if ((event.metaKey || event.ctrlKey) && event.key === 'o') {
      event.preventDefault();
      showVaultDialog();
      return;
    }
    
    // Handle file tree search activation (Ctrl/Cmd + F when file tree is focused)
    if ((event.metaKey || event.ctrlKey) && event.key === 'f') {
      const activeElement = document.activeElement;
      const isFileTreeFocused = activeElement && (
        activeElement.closest('#fileTreeContent') ||
        activeElement.classList.contains('tree-item')
      );
      
      if (isFileTreeFocused) {
        event.preventDefault();
        activateFileTreeSearch();
        return;
      }
    }
    
    // Handle Escape key to close dialogs
    if (event.key === 'Escape') {
      const vaultDialog = document.getElementById('vaultDialog');
      const vaultSwitcher = document.getElementById('vaultSwitcher');
      const shortcutsHelp = document.getElementById('shortcutsHelp');
      
      if (vaultDialog && vaultDialog.style.display === 'flex') {
        closeVaultDialog();
        return;
      }
      
      if (vaultSwitcher && vaultSwitcher.style.display !== 'none') {
        hideVaultSwitcher();
        return;
      }
      
      if (shortcutsHelp && shortcutsHelp.style.display === 'flex') {
        toggleShortcutsHelp();
        return;
      }
    }
    
    // Handle Cmd+Q (quit) on macOS or Ctrl+Q on other platforms
    if ((event.metaKey && event.key === 'q') || (event.ctrlKey && event.key === 'q')) {
      event.preventDefault();
      console.log('🔄 Quit shortcut detected - saving state...');
      
      try {
        await forceSaveAllState();
        console.log('✅ State saved, closing application');
        
        // Close the window which will trigger the quit
        if (mainWindow) {
          await mainWindow.close();
        }
      } catch (error) {
        console.error('❌ Error saving state on quit:', error);
        // Still close even if save fails
        if (mainWindow) {
          await mainWindow.close();
        }
      }
    }
    
    // Handle Cmd+W (close window) on macOS or Ctrl+W on other platforms
    if ((event.metaKey && event.key === 'w') || (event.ctrlKey && event.key === 'w')) {
      event.preventDefault();
      console.log('🔄 Close window shortcut detected - saving state...');
      
      try {
        await forceSaveAllState();
        console.log('✅ State saved, closing window');
        
        // Close the window
        if (mainWindow) {
          await mainWindow.close();
        }
      } catch (error) {
        console.error('❌ Error saving state on close:', error);
        // Still close even if save fails
        if (mainWindow) {
          await mainWindow.close();
        }
      }
    }
    
    // Handle performance dashboard keyboard shortcut (Ctrl+Shift+M or Cmd+Shift+M)
    if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === 'M') {
      event.preventDefault();
      console.log('📊 Performance dashboard shortcut detected');
      togglePerformanceDashboard();
    }
    
    // Handle performance testing panel keyboard shortcut (Ctrl+Shift+Alt+P or Cmd+Shift+Alt+P)
    if (((event.metaKey || event.ctrlKey) && event.shiftKey && event.altKey && event.key === 'P')) {
      event.preventDefault();
      console.log('🔧 Performance testing panel shortcut detected');
      
      if (performanceTestingPanelVisible) {
        hidePerformanceTestingPanel();
      } else {
        showPerformanceTestingPanel();
        logToTestOutput('🔧 Performance testing panel activated via keyboard shortcut', 'success');
        logToTestOutput('💡 This panel provides access to performance monitoring commands', 'info');
      }
    }
  });

  // Initialize UI state after services are ready
  const initializeAppState = () => {
    const initialState = appState.getState();
    console.log('🔍 Initial state loaded:', initialState);
    
    if (initialState.currentVault) {
      console.log('📁 Found saved vault:', initialState.currentVault);
      
      if (vaultManager) {
        // Verify vault is still valid and load it
        console.log('🔧 Validating saved vault...');
        vaultManager.validateVault(initialState.currentVault).then(isValid => {
          console.log('✅ Vault validation result:', isValid);
          
          if (isValid) {
            updateVaultInfo(initialState.currentVault);
            updateVaultStatusBar(initialState.currentVault);
            console.log('✅ Valid vault restored, refreshing files...');
            
            // Auto-refresh vault on startup
            refreshVault().then(() => {
              // Restore last opened file if available
              if (initialState.currentFile) {
                console.log('Restoring last opened file:', initialState.currentFile);
                // Check if the file still exists before trying to open it
                invoke('read_file', { filePath: initialState.currentFile })
                  .then(() => {
                    // Wait for all components to be fully initialized before restoring file
                    const restoreFile = () => {
                      if (document.getElementById('editorContent')) {
                        console.log('✅ Editor container ready, restoring file...');
                        openFile(initialState.currentFile);
                      } else {
                        console.log('⏳ Editor container not ready, retrying...');
                        setTimeout(restoreFile, 200);
                      }
                    };
                    
                    // Start restoration attempt after a short delay
                    setTimeout(restoreFile, 800);
                  })
                  .catch((error) => {
                    console.warn('Last opened file no longer exists:', error);
                    // Clear the invalid file from state
                    appState.setCurrentFile(null);
                  });
              }
            });
          } else {
            // Vault no longer valid, clear it and show selection dialog
            console.log('❌ Vault no longer valid, clearing and showing dialog');
            appState.setVault(null);
            setTimeout(() => {
              showVaultDialog();
            }, 1000);
          }
        }).catch(error => {
          console.error('❌ Failed to validate saved vault:', error);
          setTimeout(() => {
            showVaultDialog();
          }, 1000);
        });
      } else {
        console.warn('⚠️ VaultManager not initialized yet, showing dialog');
        setTimeout(() => {
          showVaultDialog();
        }, 1000);
      }
    } else {
      // Show vault selection dialog on first launch
      console.log('🚀 No saved vault found, showing selection dialog');
      setTimeout(() => {
        showVaultDialog();
      }, 1000);
    }
    
    // Load persisted state for UI initialization
    if (initialState.currentVault) {
      updateVaultInfo(initialState.currentVault);
    }
    
    if (initialState.currentFile) {
      const fileName = initialState.currentFile.split('/').pop();
      updateCurrentFileName(fileName, false);
      
      // If we have a file but no vault manager yet (shouldn't happen but fallback)
      if (!initialState.currentVault && vaultManager) {
        console.log('Found saved file without vault, attempting to restore file:', initialState.currentFile);
        // Try to open the file directly with proper timing
        const restoreFileWithoutVault = () => {
          if (document.getElementById('editorContent')) {
            console.log('✅ Editor ready for fallback file restoration');
            invoke('read_file', { filePath: initialState.currentFile })
              .then(() => {
                openFile(initialState.currentFile);
              })
              .catch((error) => {
                console.warn('Cannot restore file without valid vault:', error);
                appState.setCurrentFile(null);
                updateCurrentFileName(null);
              });
          } else {
            console.log('⏳ Editor not ready for fallback, retrying...');
            setTimeout(restoreFileWithoutVault, 200);
          }
        };
        
        setTimeout(restoreFileWithoutVault, 1000);
      }
    }
  };

  // Call the initialization after services are ready
  setTimeout(initializeAppState, 100);
  
  // Initialize view mode button
  const toggleBtn = document.getElementById('toggleModeBtn');
  const currentState = appState.getState();
  if (toggleBtn) {
    toggleBtn.textContent = currentState.viewMode === 'editor' ? '👁' : '✏️';
    toggleBtn.title = currentState.viewMode === 'editor' ? 'Switch to preview' : 'Switch to editor';
  }
  
  // Initialize save status
  updateSaveStatus(currentState.unsavedChanges ? 'unsaved' : 'saved');
  
  // Initialize performance monitoring dashboard (after all DOM elements are ready)
  setTimeout(async () => {
    try {
      console.log('🔄 Initializing performance monitoring dashboard...');
      performanceDashboard = new PerformanceMonitoringDashboard();
      console.log('✅ Performance monitoring dashboard initialized');
      
      // Start real-time metrics service
      await realTimeMetricsService.start();
      console.log('✅ Real-time metrics service started');
      
      // Make dashboard globally accessible
      window.performanceDashboard = performanceDashboard;
      window.realTimeMetricsService = realTimeMetricsService;
      
      console.log('🎉 Performance monitoring system fully initialized');
      
    } catch (error) {
      console.error('❌ Failed to initialize performance monitoring:', error);
      console.error('Error details:', error);
      
      // Provide a fallback
      window.performanceDashboard = {
        toggle: () => showNotification('Performance dashboard failed to initialize: ' + error.message, 'error'),
        show: () => showNotification('Performance dashboard failed to initialize: ' + error.message, 'error'),
        hide: () => {},
      };
    }
  }, 1000); // Wait 1 second for all DOM elements to be ready
  
  // Show welcome notification
  setTimeout(() => {
    const welcomeState = appState.getState();
    if (welcomeState.currentVault) {
      showNotification('Welcome back to aiNote!', 'info', 3000);
    } else {
      showNotification('Welcome to aiNote! Please select a vault to get started.', 'info', 0, true); // Persistent until vault selected
    }
  }, 1500);
});

// UI Helper Functions

/**
 * Update vault information in the status bar
 * @param {string} vaultPath - Path to the current vault
 */
function updateVaultStatusBar(vaultPath) {
  const vaultName = document.getElementById('vaultName');
  const vaultStatus = document.getElementById('vaultStatus');
  
  if (vaultName && vaultPath) {
    const folderName = vaultPath.split('/').pop() || vaultPath.split('\\').pop() || 'Unknown';
    vaultName.textContent = folderName;
    vaultName.title = vaultPath; // Full path on hover
  } else if (vaultName) {
    vaultName.textContent = 'No vault selected';
    vaultName.title = '';
  }
}

/**
 * Update save status indicator
 * @param {string} status - Save status (saved, unsaved, saving, error)
 */
function updateSaveStatus(status) {
  const indicator = document.getElementById('saveStatusIndicator');
  const icon = document.getElementById('saveIcon');
  const text = document.getElementById('saveText');
  
  if (!indicator || !icon || !text) return;
  
  // Remove all status classes
  indicator.classList.remove('saved', 'unsaved', 'saving', 'error');
  
  // Add current status class and update content
  indicator.classList.add(status);
  
  switch (status) {
    case 'saved':
      icon.textContent = '💾';
      text.textContent = 'Saved';
      break;
    case 'unsaved':
      icon.textContent = '●';
      text.textContent = 'Unsaved';
      break;
    case 'saving':
      icon.textContent = '⏳';
      text.textContent = 'Saving...';
      break;
    case 'error':
      icon.textContent = '❌';
      text.textContent = 'Save Error';
      break;
    default:
      icon.textContent = '💾';
      text.textContent = 'Ready';
  }
}

/**
 * Update operation status in the center of status bar
 * @param {string} status - Current operation status
 */
function updateOperationStatus(status) {
  const operationStatus = document.getElementById('operationStatus');
  if (operationStatus) {
    operationStatus.textContent = status;
  }
}

/**
 * Toggle performance monitoring dashboard
 */
function togglePerformanceDashboard() {
  if (window.performanceDashboard && typeof window.performanceDashboard.toggle === 'function') {
    window.performanceDashboard.toggle();
  } else {
    showNotification('Performance dashboard is still initializing... Please wait a moment.', 'info');
    
    // Try again after a short delay
    setTimeout(() => {
      if (window.performanceDashboard && typeof window.performanceDashboard.toggle === 'function') {
        window.performanceDashboard.toggle();
        showNotification('Performance dashboard is now ready!', 'success');
      } else {
        console.error('Performance dashboard initialization failed or is taking too long');
        showNotification('Performance dashboard failed to initialize. Check console for details.', 'error');
      }
    }, 2000);
  }
}

/**
 * Show vault switcher menu
 */
function showVaultSwitcher() {
  const switcher = document.getElementById('vaultSwitcher');
  const currentVaultPath = document.getElementById('currentVaultPath');
  
  if (switcher) {
    // Update current vault display
    if (currentVaultPath && vaultManager) {
      const currentVault = vaultManager.getCurrentVault();
      currentVaultPath.textContent = currentVault || 'None';
      currentVaultPath.title = currentVault || '';
    }
    
    // Populate recent vaults
    populateRecentVaultsSwitcher();
    
    switcher.style.display = 'block';
  }
}

/**
 * Hide vault switcher menu
 */
function hideVaultSwitcher() {
  const switcher = document.getElementById('vaultSwitcher');
  if (switcher) {
    switcher.style.display = 'none';
  }
}

/**
 * Populate recent vaults in the vault dialog
 */
async function populateRecentVaults() {
  if (!vaultManager) return;
  
  const recentVaults = vaultManager.getRecentVaults();
  const container = document.getElementById('recentVaults');
  const list = document.getElementById('recentVaultsList');
  
  if (!container || !list) return;
  
  if (recentVaults.length === 0) {
    container.style.display = 'none';
    return;
  }
  
  container.style.display = 'block';
  list.innerHTML = '';
  
  for (const vaultPath of recentVaults) {
    const item = document.createElement('div');
    item.className = 'recent-vault-item';
    item.onclick = () => switchToRecentVault(vaultPath);
    
    const folderName = vaultPath.split('/').pop() || vaultPath.split('\\').pop();
    
    item.innerHTML = `
      <span class="vault-icon">📁</span>
      <div class="recent-vault-info">
        <div class="recent-vault-name">${folderName}</div>
        <div class="recent-vault-path">${vaultPath}</div>
      </div>
    `;
    
    list.appendChild(item);
  }
}

/**
 * Populate recent vaults in the vault switcher
 */
async function populateRecentVaultsSwitcher() {
  if (!vaultManager) return;
  
  const recentVaults = vaultManager.getRecentVaults();
  const list = document.getElementById('recentVaultsListSwitcher');
  
  if (!list) return;
  
  list.innerHTML = '';
  
  if (recentVaults.length === 0) {
    list.innerHTML = '<p style="color: var(--color-text-tertiary); text-align: center; padding: var(--space-4);">No recent vaults</p>';
    return;
  }
  
  for (const vaultPath of recentVaults) {
    const item = document.createElement('div');
    item.className = 'recent-vault-item';
    item.onclick = () => switchToRecentVault(vaultPath);
    
    const folderName = vaultPath.split('/').pop() || vaultPath.split('\\').pop();
    
    item.innerHTML = `
      <span class="vault-icon">📁</span>
      <div class="recent-vault-info">
        <div class="recent-vault-name">${folderName}</div>
        <div class="recent-vault-path">${vaultPath}</div>
      </div>
    `;
    
    list.appendChild(item);
  }
}

// Global functions for HTML onclick handlers
window.showVaultDialog = function() {
  const dialog = document.getElementById('vaultDialog');
  if (dialog) {
    // Populate recent vaults if available
    populateRecentVaults();
    dialog.style.display = 'flex';
    
    // Focus on the dialog for accessibility
    dialog.setAttribute('tabindex', '-1');
    dialog.focus();
  }
};

window.closeVaultDialog = function() {
  const dialog = document.getElementById('vaultDialog');
  if (dialog) {
    dialog.style.display = 'none';
  }
};

window.showVaultSwitcher = function() {
  const switcher = document.getElementById('vaultSwitcher');
  const currentVaultPath = document.getElementById('currentVaultPath');
  
  if (switcher) {
    // Update current vault display
    if (currentVaultPath && window.vaultManager) {
      const currentVault = window.vaultManager.getCurrentVault();
      currentVaultPath.textContent = currentVault || 'None';
      currentVaultPath.title = currentVault || '';
    }
    
    // Populate recent vaults
    populateRecentVaultsSwitcher();
    
    switcher.style.display = 'block';
  }
};

window.hideVaultSwitcher = function() {
  const switcher = document.getElementById('vaultSwitcher');
  if (switcher) {
    switcher.style.display = 'none';
  }
};

// Override selectVault to ensure proper vault dialog closing and status updates
window.selectVault = async function() {
  console.log('🔧 Global selectVault called, vaultManager available:', !!window.vaultManager);
  
  if (!window.vaultManager) {
    console.error('❌ VaultManager not available, falling back to direct invoke');
    // Fallback to direct selectVault function if VaultManager is not available
    await selectVault();
    return;
  }
  
  try {
    console.log('📁 Using VaultManager to select vault...');
    const result = await window.vaultManager.selectVault();
    console.log('📁 VaultManager selection result:', result);
    
    if (result) {
      console.log('✅ Attempting to switch to selected vault:', result);
      
      try {
        await window.vaultManager.switchVault(result);
        console.log('✅ Successfully switched to vault via VaultManager');
      } catch (switchError) {
        console.warn('⚠️ VaultManager switchVault failed, trying direct approach:', switchError);
        
        // Fallback: set vault directly and refresh
        appState.setVault(result);
        console.log('✅ Vault set directly in AppState');
      }
      
      // Update both vault displays
      updateVaultInfo(result);
      updateVaultStatusBar(result);
      
      // Close vault dialog
      const vaultDialog = document.getElementById('vaultDialog');
      if (vaultDialog) {
        vaultDialog.style.display = 'none';
        console.log('📁 Vault dialog closed');
      }
      
      showNotification(`Vault selected: ${result.split('/').pop() || result.split('\\\\').pop()}`, 'success');
      
      // Refresh vault
      await refreshVault();
    } else {
      console.log('ℹ️ Vault selection cancelled');
      showNotification('No vault selected', 'info');
    }
  } catch (error) {
    console.error('❌ VaultManager selection error:', error);
    
    // Try the direct method as final fallback
    console.log('🔄 Trying direct vault selection as fallback...');
    try {
      await selectVault();
    } catch (fallbackError) {
      console.error('❌ Fallback selection also failed:', fallbackError);
      showNotification(`Error selecting vault: ${error.message}`, 'error');
    }
  }
};

window.selectNewVault = async function() {
  await window.selectVault();
};

window.switchToRecentVault = async function(vaultPath) {
  if (!window.vaultManager) {
    showNotification('Vault manager not initialized', 'error');
    return;
  }
  
  try {
    await window.vaultManager.switchVault(vaultPath);
    updateVaultInfo(vaultPath);
    updateVaultStatusBar(vaultPath);
    showNotification(`Switched to vault: ${vaultPath.split('/').pop() || vaultPath.split('\\\\').pop()}`, 'success');
    
    // Close dialogs
    window.closeVaultDialog();
    window.hideVaultSwitcher();
    
    // Refresh vault
    await refreshVault();
  } catch (error) {
    showNotification(`Error switching vault: ${error.message}`, 'error');
    console.error('Vault switching error:', error);
  }
};

// Add missing global utility functions
window.showProgressIndicator = function(message) {
  const indicator = document.getElementById('progressIndicator');
  const text = document.getElementById('progressText');
  if (indicator && text) {
    text.textContent = message || 'Loading...';
    indicator.style.display = 'flex';
  }
};

window.hideProgressIndicator = function() {
  const indicator = document.getElementById('progressIndicator');
  if (indicator) {
    indicator.style.display = 'none';
  }
};

// === HIDDEN PERFORMANCE TESTING INTERFACE ===
// This is a discrete testing interface for performance monitoring system
// Access methods:
//   1. Triple-click on the app title (or hidden trigger element)
//   2. Keyboard shortcut: Ctrl+Shift+Alt+P (or Cmd+Shift+Alt+P on macOS)

let performanceTestingPanelVisible = false;
let titleClickCount = 0;
let titleClickTimer = null;

// Create hidden performance testing panel
function createPerformanceTestingPanel() {
  if (document.getElementById('performanceTestingPanel')) {
    return; // Already created
  }

  const panel = document.createElement('div');
  panel.id = 'performanceTestingPanel';
  panel.style.cssText = `
    position: fixed;
    top: 10px;
    right: 10px;
    width: 320px;
    background: rgba(30, 30, 30, 0.95);
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 12px;
    z-index: 10000;
    display: none;
    font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
    font-size: 11px;
    color: #e0e0e0;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  `;
  
  panel.innerHTML = `
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; border-bottom: 1px solid rgba(255, 255, 255, 0.1); padding-bottom: 8px;">
      <div style="color: #4fc3f7; font-weight: 600;">🔧 Performance Testing</div>
      <button onclick="hidePerformanceTestingPanel()" style="background: none; border: none; color: #f44336; cursor: pointer; font-size: 16px; line-height: 1;">&times;</button>
    </div>
    
    <div style="margin-bottom: 8px;">
      <button onclick="runBenchmarkTest()" style="width: 100%; padding: 6px; background: #1976d2; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 10px; margin-bottom: 4px;">Run Benchmarks</button>
      <button onclick="establishBaselineTest()" style="width: 100%; padding: 6px; background: #388e3c; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 10px; margin-bottom: 4px;">Establish Baseline</button>
      <button onclick="runRegressionAnalysisTest()" style="width: 100%; padding: 6px; background: #f57c00; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 10px; margin-bottom: 4px;">Regression Analysis</button>
      <button onclick="runFullPerformanceTest()" style="width: 100%; padding: 6px; background: #7b1fa2; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 10px; margin-bottom: 4px;">Full Test Suite</button>
    </div>
    
    <div id="performanceTestOutput" style="background: rgba(0, 0, 0, 0.3); padding: 8px; border-radius: 4px; max-height: 200px; overflow-y: auto; font-size: 9px; line-height: 1.3;">
      <div style="color: #81c784;">Ready for testing...</div>
    </div>
  `;
  
  document.body.appendChild(panel);
}

// Show/hide performance testing panel
function showPerformanceTestingPanel() {
  createPerformanceTestingPanel();
  const panel = document.getElementById('performanceTestingPanel');
  if (panel) {
    panel.style.display = 'block';
    performanceTestingPanelVisible = true;
  }
}

function hidePerformanceTestingPanel() {
  const panel = document.getElementById('performanceTestingPanel');
  if (panel) {
    panel.style.display = 'none';
    performanceTestingPanelVisible = false;
  }
}

// Output helper for testing panel
function logToTestOutput(message, type = 'info') {
  const output = document.getElementById('performanceTestOutput');
  if (!output) return;
  
  const colors = {
    info: '#81c784',
    success: '#4fc3f7', 
    error: '#f44336',
    warning: '#ff9800'
  };
  
  const timestamp = new Date().toLocaleTimeString();
  const logEntry = document.createElement('div');
  logEntry.style.color = colors[type] || colors.info;
  logEntry.innerHTML = `<span style="color: #666;">[${timestamp}]</span> ${message}`;
  
  output.appendChild(logEntry);
  output.scrollTop = output.scrollHeight;
}

// Performance testing functions
window.runBenchmarkTest = async function() {
  logToTestOutput('🔄 Running performance benchmarks...', 'info');
  try {
    const results = await invoke('run_embedding_benchmarks');
    logToTestOutput(`✅ Benchmarks completed: ${results.length} operations tested`, 'success');
    
    // Show summary
    results.forEach((result, i) => {
      logToTestOutput(`  ${i+1}. ${result.operation_name}: ${result.avg_duration_ms.toFixed(1)}ms avg`, 'info');
    });
    
    return results;
  } catch (error) {
    logToTestOutput(`❌ Benchmark failed: ${error}`, 'error');
    console.error('Benchmark test error:', error);
  }
};

window.establishBaselineTest = async function() {
  logToTestOutput('📏 Establishing performance baseline...', 'info');
  try {
    const result = await invoke('establish_performance_baseline', { 
      operationName: 'embedding_generation' 
    });
    logToTestOutput(`✅ ${result}`, 'success');
  } catch (error) {
    logToTestOutput(`❌ Baseline establishment failed: ${error}`, 'error');
    console.error('Baseline test error:', error);
  }
};

window.runRegressionAnalysisTest = async function() {
  logToTestOutput('🔍 Running regression analysis...', 'info');
  try {
    // First run benchmarks to get current data
    const results = await invoke('run_embedding_benchmarks');
    logToTestOutput(`📊 Got ${results.length} benchmark results`, 'info');
    
    // Then analyze for regressions
    const analysis = await invoke('analyze_performance_regressions', { 
      benchmarkResults: results 
    });
    
    logToTestOutput(`✅ Analysis complete: ${analysis.total_regressions_detected} regressions detected`, 'success');
    logToTestOutput(`   Overall health: ${analysis.overall_health}`, 'info');
    
    if (analysis.recommendations.length > 0) {
      logToTestOutput('📋 Recommendations:', 'warning');
      analysis.recommendations.forEach(rec => {
        logToTestOutput(`   • ${rec}`, 'warning');
      });
    }
    
  } catch (error) {
    logToTestOutput(`❌ Regression analysis failed: ${error}`, 'error');
    console.error('Regression analysis error:', error);
  }
};

window.runFullPerformanceTest = async function() {
  logToTestOutput('🚀 Running full performance test suite...', 'info');
  
  try {
    // Step 1: Benchmarks
    logToTestOutput('Step 1/4: Running benchmarks...', 'info');
    const benchmarkResults = await window.runBenchmarkTest();
    
    // Step 2: Generate report
    logToTestOutput('Step 2/4: Generating performance report...', 'info');
    const report = await invoke('generate_benchmark_report', { results: benchmarkResults });
    logToTestOutput('📄 Performance report generated', 'success');
    
    // Step 3: Establish baseline
    logToTestOutput('Step 3/4: Establishing baseline...', 'info');
    await window.establishBaselineTest();
    
    // Step 4: Regression analysis
    logToTestOutput('Step 4/4: Analyzing regressions...', 'info');
    await window.runRegressionAnalysisTest();
    
    logToTestOutput('🎉 Full performance test suite completed!', 'success');
    
    // Show brief report summary
    const reportLines = report.split('\n').slice(0, 5);
    logToTestOutput('📋 Report preview:', 'info');
    reportLines.forEach(line => {
      if (line.trim()) {
        logToTestOutput(`   ${line.trim()}`, 'info');
      }
    });
    
  } catch (error) {
    logToTestOutput(`❌ Full test suite failed: ${error}`, 'error');
    console.error('Full performance test error:', error);
  }
};

// Triple-click detection on app title to show testing panel
document.addEventListener('DOMContentLoaded', () => {
  // Wait a bit for DOM to be ready
  setTimeout(() => {
    const titleElement = document.querySelector('h1') || document.querySelector('.app-title') || document.querySelector('title');
    
    // Try to find a title element to attach the click handler
    const titleSelectors = [
      '.header h1',
      '.app-title', 
      '.title',
      'h1',
      '.header .title-text',
      '.header-title'
    ];
    
    let targetElement = null;
    for (const selector of titleSelectors) {
      targetElement = document.querySelector(selector);
      if (targetElement) break;
    }
    
    // If no title found, create a small hidden trigger element
    if (!targetElement) {
      targetElement = document.createElement('div');
      targetElement.style.cssText = `
        position: fixed;
        top: 10px;
        left: 10px;
        width: 20px;
        height: 20px;
        opacity: 0.1;
        z-index: 9999;
        cursor: pointer;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 50%;
      `;
      targetElement.title = 'Triple-click to show performance testing panel';
      document.body.appendChild(targetElement);
    }
    
    if (targetElement) {
      targetElement.addEventListener('click', () => {
        titleClickCount++;
        
        if (titleClickTimer) {
          clearTimeout(titleClickTimer);
        }
        
        titleClickTimer = setTimeout(() => {
          titleClickCount = 0;
        }, 1000);
        
        if (titleClickCount === 3) {
          titleClickCount = 0;
          clearTimeout(titleClickTimer);
          
          if (performanceTestingPanelVisible) {
            hidePerformanceTestingPanel();
          } else {
            showPerformanceTestingPanel();
            logToTestOutput('🔧 Performance testing panel activated', 'success');
            logToTestOutput('💡 This panel provides access to performance monitoring commands', 'info');
          }
        }
      });
    }
  }, 1000);
});

// Make functions globally accessible
window.showPerformanceTestingPanel = showPerformanceTestingPanel;
window.hidePerformanceTestingPanel = hidePerformanceTestingPanel;