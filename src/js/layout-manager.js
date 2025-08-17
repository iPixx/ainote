/**
 * LayoutManager - Advanced layout management for aiNote three-column system
 * 
 * Manages CSS Grid layout with resizable columns, responsive design,
 * and state persistence following local-first principles.
 * 
 * @class LayoutManager
 */
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
    // Defer resize handle binding until DOM is ready
    this.bindResizeHandles();

    // Global mouse events for resize
    document.addEventListener('mousemove', (e) => this.handleResize(e));
    document.addEventListener('mouseup', () => this.stopResize());

    // Window resize handler
    window.addEventListener('resize', () => this.handleWindowResize());

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => this.handleKeydown(e));
  }

  bindResizeHandles() {
    // Wait for DOM to be ready
    if (document.readyState !== 'complete') {
      setTimeout(() => this.bindResizeHandles(), 100);
      return;
    }
    
    // Remove existing listeners first
    document.querySelectorAll('.resize-handle').forEach(handle => {
      // Clone node to remove all event listeners
      const newHandle = handle.cloneNode(true);
      handle.parentNode.replaceChild(newHandle, handle);
    });
    
    // Bind resize handle events to fresh handles
    document.querySelectorAll('.resize-handle').forEach(handle => {
      handle.addEventListener('mousedown', (e) => this.startResize(e));
      console.log('✅ Bound resize handle:', handle.dataset.panel);
    });
    
    console.log('🔄 Resize handles rebound');
  }

  startResize(e) {
    e.preventDefault();
    this.isResizing = true;
    this.currentResizeHandle = e.target;
    this.initialMouseX = e.clientX;
    
    const panel = this.getPanelFromHandle(this.currentResizeHandle);
    const panelElement = this.getPanelElement(panel);
    
    if (!panelElement) {
      console.warn('Panel element not found for:', panel);
      return;
    }
    
    this.initialPanelWidth = panelElement.getBoundingClientRect().width;
    
    // Add resizing class for visual feedback
    this.currentResizeHandle.classList.add('resizing');
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    
    console.log('🎯 Started resizing:', panel, 'initial width:', this.initialPanelWidth);
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
    
    console.log('🏁 Resize completed');
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
    
    console.log(`📏 Set ${panel} width to:`, width + 'px');
  }

  getAiPanelWidth() {
    const aiPanel = document.getElementById('aiPanel');
    if (!aiPanel || aiPanel.style.display === 'none') return 0;
    return aiPanel.getBoundingClientRect().width;
  }

  toggleFileTree() {
    const fileTreePanel = document.getElementById('fileTreePanel');
    const appContainer = document.querySelector('.app-container');
    const aiPanel = document.getElementById('aiPanel');
    
    if (!fileTreePanel || !appContainer) {
      console.error('Required elements not found for toggleFileTree');
      return;
    }
    
    fileTreePanel.classList.toggle('collapsed');
    
    // Update grid template based on current state
    this.updateGridLayout();
    
    // Re-bind resize handles after layout change
    setTimeout(() => this.bindResizeHandles(), 100);
    
    // Save state and notify
    this.saveLayoutState();
    
    const isCollapsed = fileTreePanel.classList.contains('collapsed');
    if (window.showNotification) {
      window.showNotification(`File tree ${isCollapsed ? 'hidden' : 'shown'} (Cmd+1)`, 'info');
    }
    
    console.log('🔄 File tree toggled:', isCollapsed ? 'collapsed' : 'expanded');
  }

  toggleAiPanel() {
    const aiPanel = document.getElementById('aiPanel');
    const appContainer = document.querySelector('.app-container');
    
    if (!aiPanel || !appContainer) {
      console.error('Required elements not found for toggleAiPanel');
      return;
    }
    
    const isCurrentlyHidden = aiPanel.style.display === 'none' || !aiPanel.style.display;
    
    if (isCurrentlyHidden) {
      aiPanel.style.display = 'flex';
      appContainer.classList.add('show-ai-panel');
    } else {
      aiPanel.style.display = 'none';
      appContainer.classList.remove('show-ai-panel');
    }
    
    // Update grid layout
    this.updateGridLayout();
    
    // Re-bind resize handles after layout change
    setTimeout(() => this.bindResizeHandles(), 100);
    
    // Save state
    this.saveLayoutState();
    
    const isVisible = !isCurrentlyHidden;
    if (window.showNotification) {
      window.showNotification(`AI Panel ${isVisible ? 'shown' : 'hidden'} (Cmd+2)`, 'info');
    }
    
    console.log('🤖 AI panel toggled:', isVisible ? 'visible' : 'hidden');
  }

  handleWindowResize() {
    // Ensure panels maintain their constraints on window resize
    const fileTreePanel = document.getElementById('fileTreePanel');
    if (!fileTreePanel) return;
    
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
          if (window.selectVault) window.selectVault();
          break;
        case 'n':
        case 'N':
          e.preventDefault();
          if (window.createNewFile) window.createNewFile();
          break;
        case 's':
        case 'S':
          e.preventDefault();
          if (window.saveFile) window.saveFile();
          break;
        case 'e':
        case 'E':
          e.preventDefault();
          if (window.toggleViewMode) window.toggleViewMode();
          break;
        case '/':
          // Ctrl+/ for help (like VS Code)
          e.preventDefault();
          if (window.toggleShortcutsHelp) window.toggleShortcutsHelp();
          break;
      }
    }
    
    // Layout shortcuts with Cmd/Ctrl modifiers
    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case '1':
          e.preventDefault();
          this.toggleFileTree();
          break;
        case '2':
          e.preventDefault();
          this.toggleAiPanel();
          break;
      }
    }
  }

  saveLayoutState() {
    const fileTreePanel = document.getElementById('fileTreePanel');
    const aiPanel = document.getElementById('aiPanel');
    
    if (!fileTreePanel || !aiPanel) return;
    
    const fileTreeWidth = getComputedStyle(document.documentElement)
      .getPropertyValue('--file-tree-default-width');
    const aiPanelWidth = getComputedStyle(document.documentElement)
      .getPropertyValue('--ai-panel-default-width');
    
    const layoutState = {
      fileTreeWidth: fileTreeWidth,
      aiPanelWidth: aiPanelWidth,
      fileTreeCollapsed: fileTreePanel.classList.contains('collapsed'),
      aiPanelVisible: aiPanel.style.display !== 'none'
    };
    
    try {
      localStorage.setItem('aiNote_layoutState', JSON.stringify(layoutState));
      console.log('💾 Layout state saved:', layoutState);
    } catch (error) {
      console.error('Failed to save layout state:', error);
    }
  }

  loadLayoutState() {
    try {
      const saved = localStorage.getItem('aiNote_layoutState');
      const state = saved ? JSON.parse(saved) : null;
      if (state) {
        console.log('📂 Layout state loaded:', state);
      }
      return state;
    } catch (error) {
      console.error('Failed to load layout state:', error);
      return null;
    }
  }

  updateGridLayout() {
    const fileTreePanel = document.getElementById('fileTreePanel');
    const aiPanel = document.getElementById('aiPanel');
    const appContainer = document.querySelector('.app-container');
    
    if (!fileTreePanel || !aiPanel || !appContainer) {
      console.error('Cannot update grid layout: missing elements');
      return;
    }
    
    const fileTreeCollapsed = fileTreePanel.classList.contains('collapsed');
    const aiPanelVisible = aiPanel.style.display === 'flex';
    
    const fileTreeWidth = fileTreeCollapsed ? '0' : 
      getComputedStyle(document.documentElement).getPropertyValue('--file-tree-default-width');
    const aiPanelWidth = aiPanelVisible ? 
      getComputedStyle(document.documentElement).getPropertyValue('--ai-panel-default-width') : '';
    
    // Update grid template columns based on panel states
    if (aiPanelVisible) {
      appContainer.style.gridTemplateColumns = `${fileTreeWidth} 1fr ${aiPanelWidth}`;
      appContainer.classList.add('show-ai-panel');
    } else {
      appContainer.style.gridTemplateColumns = `${fileTreeWidth} 1fr`;
      appContainer.classList.remove('show-ai-panel');
    }
    
    console.log('📐 Grid layout updated:', {
      fileTreeCollapsed,
      aiPanelVisible,
      gridTemplate: appContainer.style.gridTemplateColumns
    });
  }
  
  applyLayoutState(layoutState) {
    const root = document.documentElement;
    const fileTreePanel = document.getElementById('fileTreePanel');
    const aiPanel = document.getElementById('aiPanel');
    const appContainer = document.querySelector('.app-container');
    
    if (!fileTreePanel || !aiPanel || !appContainer) {
      // Retry after a short delay if elements aren't ready
      setTimeout(() => this.applyLayoutState(layoutState), 100);
      return;
    }
    
    if (layoutState.fileTreeWidth) {
      root.style.setProperty('--file-tree-default-width', layoutState.fileTreeWidth);
    }
    
    if (layoutState.aiPanelWidth) {
      root.style.setProperty('--ai-panel-default-width', layoutState.aiPanelWidth);
    }
    
    if (layoutState.fileTreeCollapsed) {
      fileTreePanel.classList.add('collapsed');
    }
    
    if (layoutState.aiPanelVisible) {
      aiPanel.style.display = 'flex';
    }
    
    // Apply the correct grid layout
    this.updateGridLayout();
    
    console.log('🎨 Layout state applied successfully');
  }
}

/**
 * MobileNavManager - Mobile navigation overlay management
 * 
 * Handles slide-out navigation for mobile screens with touch-friendly
 * interactions and accessibility support.
 * 
 * @class MobileNavManager
 */
class MobileNavManager {
  constructor() {
    this.isOpen = false;
    this.bindEvents();
  }

  bindEvents() {
    const overlay = document.getElementById('mobileNavOverlay');
    if (overlay) {
      // Close on overlay click
      overlay.addEventListener('click', (e) => {
        if (e.target === e.currentTarget) {
          this.close();
        }
      });
    }
  }

  open() {
    if (this.isOpen) return;
    
    const overlay = document.getElementById('mobileNavOverlay');
    const navContent = document.getElementById('mobileNavContent');
    const fileTreeContent = document.getElementById('fileTreeContent');
    
    if (!overlay || !navContent || !fileTreeContent) return;
    
    // Clone file tree content to mobile nav
    navContent.innerHTML = fileTreeContent.innerHTML;
    
    overlay.style.display = 'block';
    // Force reflow before adding active class for animation
    overlay.offsetHeight;
    overlay.classList.add('active');
    
    this.isOpen = true;
    document.body.style.overflow = 'hidden';
    
    console.log('📱 Mobile navigation opened');
  }

  close() {
    if (!this.isOpen) return;
    
    const overlay = document.getElementById('mobileNavOverlay');
    if (!overlay) return;
    
    overlay.classList.remove('active');
    setTimeout(() => {
      overlay.style.display = 'none';
    }, 250); // Match CSS transition duration
    
    this.isOpen = false;
    document.body.style.overflow = '';
    
    console.log('📱 Mobile navigation closed');
  }
}

// Export for ES6 module usage
export { LayoutManager, MobileNavManager };