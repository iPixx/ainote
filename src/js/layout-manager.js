/**
 * LayoutManager - Reliable layout management for aiNote three-column system
 * 
 * Manages CSS Grid layout with resizable columns, responsive design,
 * and state persistence following local-first principles.
 * 
 * @class LayoutManager
 */
class LayoutManager {
  constructor() {
    // Resize state
    this.resizeState = {
      isResizing: false,
      currentHandle: null,
      targetPanel: null,
      startX: 0,
      startWidth: 0
    };
    
    // Panel constraints
    this.constraints = {
      'file-tree': { min: 250, max: 400, default: 280 },
      'editor': { min: 600, max: null, default: null }, // Flexible
      'ai-panel': { min: 300, max: 500, default: 350 }
    };
    
    // Panel state
    this.panelState = {
      fileTreeCollapsed: false,
      aiPanelVisible: false
    };
    
    // Initialize when DOM is ready
    this.initialize();
  }

  initialize() {
    // Wait for DOM to be completely ready
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => this.initialize());
      return;
    }
    
    // Initialize in phases for reliability
    this.setupEventListeners();
    this.loadSavedState();
    this.setupResizeHandles();
    this.updateLayout();
    this.updateExpandButton();
    
    console.log('✅ LayoutManager initialized successfully');
  }

  setupEventListeners() {
    // Global mouse events for resize (with proper binding)
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseUp = this.handleMouseUp.bind(this);
    this.handleKeydown = this.handleKeydown.bind(this);
    this.handleWindowResize = this.handleWindowResize.bind(this);

    document.addEventListener('mousemove', this.handleMouseMove);
    document.addEventListener('mouseup', this.handleMouseUp);
    document.addEventListener('keydown', this.handleKeydown);
    window.addEventListener('resize', this.handleWindowResize);
    
    console.log('📡 Event listeners setup complete');
  }

  setupResizeHandles() {
    const handles = document.querySelectorAll('.resize-handle');
    
    // Bind the method once if not already bound
    if (!this.boundHandleMouseDown) {
      this.boundHandleMouseDown = this.handleMouseDown.bind(this);
    }
    
    handles.forEach(handle => {
      const panel = handle.dataset.panel;
      
      // Only setup handles for panels that should be resizable
      if (panel === 'file-tree' || panel === 'editor') {
        // Remove any existing listeners
        handle.removeEventListener('mousedown', this.boundHandleMouseDown);
        
        // Add new listener with proper binding
        handle.addEventListener('mousedown', this.boundHandleMouseDown);
        
        console.log(`🎯 Setup resize handle for: ${panel}`);
      }
    });
    
    console.log('🔧 Resize handles setup complete');
  }

  handleMouseDown(e) {
    e.preventDefault();
    e.stopPropagation();
    
    const handle = e.target;
    const handlePanel = handle.dataset.panel;
    
    // Determine which panel we're actually resizing
    let targetPanel;
    if (handlePanel === 'file-tree') {
      targetPanel = 'file-tree';
    } else if (handlePanel === 'editor') {
      // Editor handle resizes the AI panel
      targetPanel = 'ai-panel';
      
      // Only allow resizing if AI panel is visible
      const aiPanel = this.getElement('ai-panel');
      if (!aiPanel || aiPanel.style.display === 'none') {
        console.log('🚫 Cannot resize AI panel - not visible');
        return;
      }
    } else {
      console.warn('🚫 Unknown resize handle:', handlePanel);
      return;
    }
    
    const targetElement = this.getElement(targetPanel);
    if (!targetElement) {
      console.error('🚫 Target panel element not found:', targetPanel);
      return;
    }
    
    // Set resize state
    this.resizeState = {
      isResizing: true,
      currentHandle: handle,
      targetPanel: targetPanel,
      startX: e.clientX,
      startWidth: targetElement.getBoundingClientRect().width
    };
    
    // Visual feedback
    handle.classList.add('resizing');
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    
    console.log(`🎯 Started resizing ${targetPanel}, initial width: ${this.resizeState.startWidth}px`);
  }

  handleMouseMove(e) {
    if (!this.resizeState.isResizing) return;
    
    e.preventDefault();
    
    const { targetPanel, startX, startWidth } = this.resizeState;
    const deltaX = e.clientX - startX;
    
    let newWidth;
    
    // Calculate new width based on panel type
    if (targetPanel === 'ai-panel') {
      // AI panel resizes in reverse (dragging left makes it larger)
      newWidth = startWidth - deltaX;
    } else {
      // File tree panel resizes normally (dragging right makes it larger)
      newWidth = startWidth + deltaX;
    }
    
    // Apply constraints and update
    const constrainedWidth = this.constrainWidth(targetPanel, newWidth);
    this.setPanelWidth(targetPanel, constrainedWidth);
  }

  handleMouseUp(e) {
    if (!this.resizeState.isResizing) return;
    
    // Clean up resize state
    if (this.resizeState.currentHandle) {
      this.resizeState.currentHandle.classList.remove('resizing');
    }
    
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    
    // Save state
    this.saveState();
    
    console.log(`🏁 Resize completed for ${this.resizeState.targetPanel}`);
    
    // Reset state
    this.resizeState = {
      isResizing: false,
      currentHandle: null,
      targetPanel: null,
      startX: 0,
      startWidth: 0
    };
  }

  getElement(panel) {
    const elements = {
      'file-tree': 'fileTreePanel',
      'editor': 'editorPanel',
      'ai-panel': 'aiPanel',
      'app-container': 'app-container'
    };
    
    const id = elements[panel];
    if (!id) {
      console.error(`🚫 Unknown panel: ${panel}`);
      return null;
    }
    
    const element = document.getElementById(id) || document.querySelector(`.${id}`);
    if (!element) {
      console.error(`🚫 Element not found for panel: ${panel}`);
    }
    
    return element;
  }

  constrainWidth(panel, width) {
    const constraint = this.constraints[panel];
    if (!constraint) {
      console.error(`🚫 No constraints defined for panel: ${panel}`);
      return width;
    }
    
    // Apply basic min/max constraints
    let constrainedWidth = Math.max(width, constraint.min);
    if (constraint.max) {
      constrainedWidth = Math.min(constrainedWidth, constraint.max);
    }
    
    // Apply contextual constraints to ensure layout stability
    const appContainer = this.getElement('app-container');
    if (!appContainer) return constrainedWidth;
    
    const totalWidth = appContainer.getBoundingClientRect().width;
    const editorMinWidth = this.constraints.editor.min;
    const margins = 20; // Account for borders, padding, etc.
    
    if (panel === 'file-tree') {
      const aiPanelWidth = this.getCurrentPanelWidth('ai-panel');
      const maxAllowed = totalWidth - editorMinWidth - aiPanelWidth - margins;
      constrainedWidth = Math.min(constrainedWidth, maxAllowed);
    }
    
    if (panel === 'ai-panel') {
      const fileTreeWidth = this.getCurrentPanelWidth('file-tree');
      const maxAllowed = totalWidth - editorMinWidth - fileTreeWidth - margins;
      constrainedWidth = Math.min(constrainedWidth, maxAllowed);
    }
    
    return Math.max(constrainedWidth, constraint.min); // Ensure we never go below minimum
  }

  setPanelWidth(panel, width) {
    const cssVarMap = {
      'file-tree': '--file-tree-default-width',
      'ai-panel': '--ai-panel-default-width'
    };
    
    const cssVar = cssVarMap[panel];
    if (!cssVar) {
      console.error(`🚫 No CSS variable mapping for panel: ${panel}`);
      return;
    }
    
    document.documentElement.style.setProperty(cssVar, `${width}px`);
    this.updateLayout(); // Ensure layout is updated after width change
    
    console.log(`📏 Set ${panel} width to: ${width}px`);
  }

  getCurrentPanelWidth(panel) {
    const element = this.getElement(panel);
    if (!element) return 0;
    
    // Check if panel is visible/collapsed
    if (panel === 'file-tree' && element.classList.contains('collapsed')) return 0;
    if (panel === 'ai-panel' && element.style.display === 'none') return 0;
    
    return element.getBoundingClientRect().width;
  }

  toggleFileTree() {
    const fileTreePanel = this.getElement('file-tree');
    if (!fileTreePanel) return;
    
    const wasCollapsed = fileTreePanel.classList.contains('collapsed');
    
    if (wasCollapsed) {
      fileTreePanel.classList.remove('collapsed');
      this.panelState.fileTreeCollapsed = false;
    } else {
      fileTreePanel.classList.add('collapsed');
      this.panelState.fileTreeCollapsed = true;
    }
    
    this.updateLayout();
    this.updateExpandButton();
    this.saveState();
    
    const isCollapsed = this.panelState.fileTreeCollapsed;
    if (window.showNotification) {
      window.showNotification(`File tree ${isCollapsed ? 'hidden' : 'shown'} (Ctrl/Cmd+1)`, 'info');
    }
    
    console.log(`🗂️ File tree ${isCollapsed ? 'collapsed' : 'expanded'}`);
  }

  toggleAiPanel() {
    const aiPanel = this.getElement('ai-panel');
    const appContainer = this.getElement('app-container');
    
    if (!aiPanel || !appContainer) return;
    
    const wasVisible = this.panelState.aiPanelVisible;
    
    if (wasVisible) {
      aiPanel.style.display = 'none';
      appContainer.classList.remove('show-ai-panel');
      this.panelState.aiPanelVisible = false;
    } else {
      aiPanel.style.display = 'flex';
      appContainer.classList.add('show-ai-panel');
      this.panelState.aiPanelVisible = true;
    }
    
    this.updateLayout();
    this.saveState();
    
    const isVisible = this.panelState.aiPanelVisible;
    if (window.showNotification) {
      window.showNotification(`AI Panel ${isVisible ? 'shown' : 'hidden'} (Ctrl/Cmd+2)`, 'info');
    }
    
    console.log(`🤖 AI panel ${isVisible ? 'shown' : 'hidden'}`);
  }

  handleWindowResize() {
    // Validate and adjust panel widths on window resize
    console.log('🔄 Window resized - validating panel widths');
    
    // Check file tree width
    const fileTreeWidth = this.getCurrentPanelWidth('file-tree');
    if (fileTreeWidth > 0) {
      const constrainedFileTreeWidth = this.constrainWidth('file-tree', fileTreeWidth);
      if (constrainedFileTreeWidth !== fileTreeWidth) {
        this.setPanelWidth('file-tree', constrainedFileTreeWidth);
      }
    }
    
    // Check AI panel width
    const aiPanelWidth = this.getCurrentPanelWidth('ai-panel');
    if (aiPanelWidth > 0) {
      const constrainedAiPanelWidth = this.constrainWidth('ai-panel', aiPanelWidth);
      if (constrainedAiPanelWidth !== aiPanelWidth) {
        this.setPanelWidth('ai-panel', constrainedAiPanelWidth);
      }
    }
    
    this.updateLayout();
  }

  handleKeydown(e) {
    // Handle Escape key to close modals
    if (e.key === 'Escape') {
      const shortcutsHelp = document.getElementById('shortcutsHelp');
      if (shortcutsHelp && (shortcutsHelp.style.display === 'flex' || shortcutsHelp.style.display === 'block')) {
        e.preventDefault();
        if (window.toggleShortcutsHelp) window.toggleShortcutsHelp();
        return;
      }
    }
    
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
        case 'q':
        case 'Q':
          // Ctrl+Q for force save state (useful for testing)
          e.preventDefault();
          if (window.forceSaveAllState) {
            window.forceSaveAllState();
            if (window.showNotification) {
              window.showNotification('Application state saved manually', 'success');
            }
          }
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

  updateLayout() {
    const appContainer = this.getElement('app-container');
    const fileTreePanel = this.getElement('file-tree');
    
    if (!appContainer) return;
    
    // Ensure DOM state matches panel state
    if (fileTreePanel) {
      if (this.panelState.fileTreeCollapsed) {
        fileTreePanel.classList.add('collapsed');
      } else {
        fileTreePanel.classList.remove('collapsed');
      }
    }
    
    const fileTreeWidth = this.panelState.fileTreeCollapsed ? '0' : 
      getComputedStyle(document.documentElement).getPropertyValue('--file-tree-default-width');
    
    const aiPanelWidth = this.panelState.aiPanelVisible ? 
      getComputedStyle(document.documentElement).getPropertyValue('--ai-panel-default-width') : '';
    
    // Update grid template columns with important to override responsive CSS
    if (this.panelState.aiPanelVisible) {
      appContainer.style.setProperty('grid-template-columns', `${fileTreeWidth} 1fr ${aiPanelWidth}`, 'important');
      appContainer.classList.add('show-ai-panel');
    } else {
      appContainer.style.setProperty('grid-template-columns', `${fileTreeWidth} 1fr`, 'important');
      appContainer.classList.remove('show-ai-panel');
    }
    
    console.log('📐 Layout updated:', {
      fileTreeCollapsed: this.panelState.fileTreeCollapsed,
      aiPanelVisible: this.panelState.aiPanelVisible,
      gridTemplate: appContainer.style.gridTemplateColumns
    });
  }

  updateExpandButton() {
    const expandBtn = document.getElementById('fileTreeExpandBtn');
    const collapseBtn = document.getElementById('collapseTreeBtn');
    
    // Update editor header toggle button
    if (expandBtn) {
      if (this.panelState.fileTreeCollapsed) {
        // When collapsed, show right arrow to expand
        expandBtn.textContent = '➡️';
        expandBtn.setAttribute('aria-label', 'Show file tree');
        expandBtn.setAttribute('title', 'Show file tree (Ctrl/Cmd+1)');
      } else {
        // When expanded, show left arrow to collapse
        expandBtn.textContent = '⬅️';
        expandBtn.setAttribute('aria-label', 'Hide file tree');
        expandBtn.setAttribute('title', 'Hide file tree (Ctrl/Cmd+1)');
      }
    }
    
    // Update file tree header collapse button
    if (collapseBtn) {
      if (this.panelState.fileTreeCollapsed) {
        // When collapsed, show right arrow to expand
        collapseBtn.textContent = '➡️';
        collapseBtn.setAttribute('aria-label', 'Show file tree');
        collapseBtn.setAttribute('title', 'Show file tree (Ctrl/Cmd+1)');
      } else {
        // When expanded, show left arrow to collapse
        collapseBtn.textContent = '⬅️';
        collapseBtn.setAttribute('aria-label', 'Hide file tree');
        collapseBtn.setAttribute('title', 'Hide file tree (Ctrl/Cmd+1)');
      }
    }
    
    console.log(`📍 Toggle buttons updated: ${this.panelState.fileTreeCollapsed ? '➡️ (expand)' : '⬅️ (collapse)'}`);
  }

  saveState() {
    const state = {
      fileTreeWidth: getComputedStyle(document.documentElement).getPropertyValue('--file-tree-default-width'),
      aiPanelWidth: getComputedStyle(document.documentElement).getPropertyValue('--ai-panel-default-width'),
      fileTreeCollapsed: this.panelState.fileTreeCollapsed,
      aiPanelVisible: this.panelState.aiPanelVisible
    };
    
    try {
      localStorage.setItem('aiNote_layoutState', JSON.stringify(state));
      console.log('💾 Layout state saved successfully');
      
      // Also trigger the new persistent save function if available
      if (window.debouncedSaveLayoutState) {
        window.debouncedSaveLayoutState(this.getCurrentLayoutState());
      }
    } catch (error) {
      console.error('❌ Failed to save layout state:', error);
    }
  }

  loadSavedState() {
    try {
      const saved = localStorage.getItem('aiNote_layoutState');
      if (!saved) return;
      
      const state = JSON.parse(saved);
      this.applyState(state);
      console.log('📂 Layout state loaded successfully');
    } catch (error) {
      console.error('❌ Failed to load layout state:', error);
    }
  }

  applyState(state) {
    // Apply panel widths
    if (state.fileTreeWidth) {
      document.documentElement.style.setProperty('--file-tree-default-width', state.fileTreeWidth);
    }
    
    if (state.aiPanelWidth) {
      document.documentElement.style.setProperty('--ai-panel-default-width', state.aiPanelWidth);
    }
    
    // Apply panel visibility states
    this.panelState.fileTreeCollapsed = state.fileTreeCollapsed || false;
    this.panelState.aiPanelVisible = state.aiPanelVisible || false;
    
    // Apply DOM states
    const fileTreePanel = this.getElement('file-tree');
    const aiPanel = this.getElement('ai-panel');
    const appContainer = this.getElement('app-container');
    
    if (fileTreePanel) {
      if (this.panelState.fileTreeCollapsed) {
        fileTreePanel.classList.add('collapsed');
      } else {
        fileTreePanel.classList.remove('collapsed');
      }
    }
    
    if (aiPanel && appContainer) {
      if (this.panelState.aiPanelVisible) {
        aiPanel.style.display = 'flex';
        appContainer.classList.add('show-ai-panel');
      } else {
        aiPanel.style.display = 'none';
        appContainer.classList.remove('show-ai-panel');
      }
    }
    
    this.updateLayout();
    this.updateExpandButton();
    console.log('🎨 State applied successfully');
  }

  /**
   * Get current layout state for persistence
   * @returns {Object} Current layout state
   */
  getCurrentLayoutState() {
    const fileTreeWidthValue = getComputedStyle(document.documentElement)
      .getPropertyValue('--file-tree-default-width').replace('px', '');
    const aiPanelWidthValue = getComputedStyle(document.documentElement)
      .getPropertyValue('--ai-panel-default-width').replace('px', '');

    return {
      fileTreeWidth: parseFloat(fileTreeWidthValue) || 280,
      aiPanelWidth: parseFloat(aiPanelWidthValue) || 350,
      fileTreeVisible: !this.panelState.fileTreeCollapsed,
      aiPanelVisible: this.panelState.aiPanelVisible,
      editorMode: 'edit' // Default for now, can be extended later
    };
  }

  /**
   * Apply layout state from persistence
   * @param {Object} layoutState - Saved layout state
   */
  applyLayoutState(layoutState) {
    if (!layoutState) return;

    console.log('📥 Applying saved layout state:', layoutState);

    // Apply panel widths
    if (layoutState.fileTreeWidth) {
      document.documentElement.style.setProperty('--file-tree-default-width', `${layoutState.fileTreeWidth}px`);
    }

    if (layoutState.aiPanelWidth) {
      document.documentElement.style.setProperty('--ai-panel-default-width', `${layoutState.aiPanelWidth}px`);
    }

    // Apply panel visibility states
    this.panelState.fileTreeCollapsed = !layoutState.fileTreeVisible;
    this.panelState.aiPanelVisible = layoutState.aiPanelVisible || false;

    // Apply DOM states
    const fileTreePanel = this.getElement('file-tree');
    const aiPanel = this.getElement('ai-panel');
    const appContainer = this.getElement('app-container');

    if (fileTreePanel) {
      if (this.panelState.fileTreeCollapsed) {
        fileTreePanel.classList.add('collapsed');
      } else {
        fileTreePanel.classList.remove('collapsed');
      }
    }

    if (aiPanel && appContainer) {
      if (this.panelState.aiPanelVisible) {
        aiPanel.style.display = 'flex';
        appContainer.classList.add('show-ai-panel');
      } else {
        aiPanel.style.display = 'none';
        appContainer.classList.remove('show-ai-panel');
      }
    }

    this.updateLayout();
    this.updateExpandButton();
    
    console.log('✅ Layout state applied from persistence');
  }

  // Layout reliability test
  runLayoutTest() {
    console.log('🧪 Starting Layout Reliability Test...');
    
    const tests = [];
    const appContainer = this.getElement('app-container');
    const fileTreePanel = this.getElement('file-tree');
    const editorPanel = this.getElement('editor');
    const aiPanel = this.getElement('ai-panel');
    
    // Test 1: Basic element presence
    tests.push({
      name: 'Elements Present',
      pass: !!(appContainer && fileTreePanel && editorPanel && aiPanel),
      message: 'All required layout elements exist'
    });
    
    // Test 2: Grid layout structure
    const computedStyle = window.getComputedStyle(appContainer);
    tests.push({
      name: 'CSS Grid Layout',
      pass: computedStyle.display === 'grid',
      message: 'App container uses CSS Grid'
    });
    
    // Test 3: File tree toggle functionality
    const initialState = this.panelState.fileTreeCollapsed;
    this.toggleFileTree();
    const afterToggle = this.panelState.fileTreeCollapsed;
    this.toggleFileTree(); // Reset
    tests.push({
      name: 'File Tree Toggle',
      pass: initialState !== afterToggle,
      message: 'File tree toggle changes state'
    });
    
    // Test 4: Grid template updates
    const initialGrid = appContainer.style.gridTemplateColumns;
    this.updateLayout();
    const afterUpdate = appContainer.style.gridTemplateColumns;
    tests.push({
      name: 'Grid Template Updates',
      pass: afterUpdate.includes('1fr'),
      message: 'Grid template contains flexible column'
    });
    
    // Test 5: Panel width constraints
    const fileTreeWidth = this.getCurrentPanelWidth('file-tree');
    const constraint = this.constraints['file-tree'];
    tests.push({
      name: 'Width Constraints',
      pass: !this.panelState.fileTreeCollapsed ? (fileTreeWidth >= constraint.min) : true,
      message: 'Panel widths respect minimum constraints'
    });
    
    // Test 6: Button state synchronization
    const expandBtn = document.getElementById('fileTreeExpandBtn');
    const collapseBtn = document.getElementById('collapseTreeBtn');
    tests.push({
      name: 'Button Synchronization',
      pass: !!(expandBtn && collapseBtn),
      message: 'Toggle buttons exist and are accessible'
    });
    
    // Test 7: Z-index and positioning
    const fileTreeZIndex = window.getComputedStyle(fileTreePanel).zIndex;
    const editorZIndex = window.getComputedStyle(editorPanel).zIndex;
    tests.push({
      name: 'Panel Positioning',
      pass: !window.getComputedStyle(fileTreePanel).transform.includes('translate'),
      message: 'File tree panel not using problematic transforms'
    });
    
    // Generate test report
    const passed = tests.filter(t => t.pass).length;
    const total = tests.length;
    
    console.log(`🧪 Layout Test Results: ${passed}/${total} tests passed`);
    tests.forEach(test => {
      const icon = test.pass ? '✅' : '❌';
      console.log(`${icon} ${test.name}: ${test.message}`);
    });
    
    if (passed === total) {
      console.log('🎉 All layout tests passed! Layout is reliable.');
    } else {
      console.warn('⚠️ Some layout tests failed. Review implementation.');
    }
    
    return { passed, total, tests };
  }

  // Cleanup method for removing event listeners
  destroy() {
    document.removeEventListener('mousemove', this.handleMouseMove);
    document.removeEventListener('mouseup', this.handleMouseUp);
    document.removeEventListener('keydown', this.handleKeydown);
    window.removeEventListener('resize', this.handleWindowResize);
    
    // Clean up resize handle listeners
    if (this.boundHandleMouseDown) {
      document.querySelectorAll('.resize-handle').forEach(handle => {
        handle.removeEventListener('mousedown', this.boundHandleMouseDown);
      });
    }
    
    console.log('🧹 LayoutManager destroyed');
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