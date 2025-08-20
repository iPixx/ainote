/**
 * ContextMenu - Reusable context menu component for file operations
 * 
 * Features:
 * - Right-click detection and positioning
 * - File operations: New File, New Folder, Rename, Delete, Refresh, Reveal
 * - Confirmation dialogs for destructive operations
 * - Keyboard navigation and accessibility
 * - Integration with backend file operations
 * 
 * @class ContextMenu
 */
class ContextMenu {
  /**
   * Context menu events
   */
  static EVENTS = {
    MENU_OPENED: 'menu_opened',
    MENU_CLOSED: 'menu_closed',
    ACTION_EXECUTED: 'action_executed'
  };

  /**
   * Initialize ContextMenu component
   * @param {AppState} appState - Application state instance
   */
  constructor(appState) {
    if (!appState) {
      throw new Error('ContextMenu requires an AppState instance');
    }

    this.appState = appState;
    this.isOpen = false;
    this.currentTarget = null;
    this.currentFile = null;
    this.menuElement = null;
    this.clickOutsideHandler = null;
    this.keyHandler = null;

    // Initialize menu HTML
    this.createMenuElement();
    this.setupEventListeners();
  }

  /**
   * Create the context menu DOM element
   */
  createMenuElement() {
    const menu = document.createElement('div');
    menu.className = 'context-menu';
    menu.setAttribute('role', 'menu');
    menu.setAttribute('aria-hidden', 'true');
    menu.style.cssText = `
      position: fixed;
      z-index: 9999;
      background-color: var(--color-bg-primary, #ffffff);
      border: 1px solid var(--color-border-primary, #e2e8f0);
      border-radius: 6px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      padding: 4px;
      min-width: 180px;
      display: none;
      font-family: inherit;
      font-size: 13px;
    `;

    // Initially empty - items will be populated based on context
    document.body.appendChild(menu);
    this.menuElement = menu;
  }

  /**
   * Show context menu at specified coordinates
   * @param {number} x - X coordinate
   * @param {number} y - Y coordinate
   * @param {Object} file - File object (null for empty space)
   * @param {HTMLElement} target - Target element
   */
  show(x, y, file, target) {
    if (this.isOpen) {
      this.hide();
    }

    this.currentTarget = target;
    this.currentFile = file;
    this.isOpen = true;

    // Populate menu items based on context
    this.populateMenuItems(file);

    // Position menu
    this.positionMenu(x, y);

    // Show menu
    this.menuElement.style.display = 'block';
    this.menuElement.setAttribute('aria-hidden', 'false');

    // Focus first menu item
    const firstMenuItem = this.menuElement.querySelector('.context-menu-item');
    if (firstMenuItem) {
      firstMenuItem.focus();
    }

    // Set up event listeners
    this.setupActiveListeners();

    // Emit event
    this.emit(ContextMenu.EVENTS.MENU_OPENED, { file, target });
  }

  /**
   * Hide context menu
   */
  hide() {
    if (!this.isOpen) return;

    this.isOpen = false;
    this.menuElement.style.display = 'none';
    this.menuElement.setAttribute('aria-hidden', 'true');

    // Clean up event listeners
    this.cleanupActiveListeners();

    // Emit event
    this.emit(ContextMenu.EVENTS.MENU_CLOSED, { 
      file: this.currentFile, 
      target: this.currentTarget 
    });

    // Reset state
    this.currentTarget = null;
    this.currentFile = null;
  }

  /**
   * Populate menu items based on context
   * @param {Object|null} file - File object or null for empty space
   */
  populateMenuItems(file) {
    const isFile = file && !file.is_dir;
    const isFolder = file && file.is_dir;
    const isEmpty = !file;

    this.menuElement.innerHTML = '';

    // New File - always available
    this.addMenuItem({
      label: 'New File',
      icon: '📝',
      action: () => this.handleNewFile(),
      enabled: true,
      shortcut: 'Ctrl+N'
    });

    // New Folder - always available
    this.addMenuItem({
      label: 'New Folder',
      icon: '📁',
      action: () => this.handleNewFolder(),
      enabled: true,
      shortcut: 'Ctrl+Shift+N'
    });

    // Separator
    this.addSeparator();

    if (file) {
      // Rename - available for files and folders
      this.addMenuItem({
        label: 'Rename',
        icon: '✏️',
        action: () => this.handleRename(file),
        enabled: true,
        shortcut: 'F2'
      });

      // Delete - available for files and folders
      this.addMenuItem({
        label: 'Delete',
        icon: '🗑️',
        action: () => this.handleDelete(file),
        enabled: true,
        shortcut: 'Delete',
        dangerous: true
      });

      // Separator
      this.addSeparator();
    }

    // Refresh - always available
    this.addMenuItem({
      label: 'Refresh',
      icon: '🔄',
      action: () => this.handleRefresh(),
      enabled: true,
      shortcut: 'F5'
    });

    if (file) {
      // Reveal in Explorer/Finder - available for files and folders
      this.addMenuItem({
        label: 'Reveal in Finder',
        icon: '👁️',
        action: () => this.handleReveal(file),
        enabled: true
      });
    }
  }

  /**
   * Add a menu item
   * @param {Object} options - Menu item options
   */
  addMenuItem(options) {
    const {
      label,
      icon = '',
      action,
      enabled = true,
      shortcut = '',
      dangerous = false
    } = options;

    const item = document.createElement('button');
    item.className = `context-menu-item ${dangerous ? 'dangerous' : ''}`;
    item.setAttribute('role', 'menuitem');
    item.setAttribute('tabindex', '-1');
    item.disabled = !enabled;

    if (!enabled) {
      item.setAttribute('aria-disabled', 'true');
    }

    item.innerHTML = `
      <span class="menu-item-icon">${icon}</span>
      <span class="menu-item-label">${label}</span>
      ${shortcut ? `<span class="menu-item-shortcut">${shortcut}</span>` : ''}
    `;

    if (enabled && action) {
      item.addEventListener('click', () => {
        this.hide();
        action();
      });
    }

    this.menuElement.appendChild(item);
  }

  /**
   * Add a separator
   */
  addSeparator() {
    const separator = document.createElement('div');
    separator.className = 'context-menu-separator';
    separator.setAttribute('role', 'separator');
    this.menuElement.appendChild(separator);
  }

  /**
   * Position menu at coordinates with viewport boundary checking
   * @param {number} x - X coordinate
   * @param {number} y - Y coordinate
   */
  positionMenu(x, y) {
    // Get menu dimensions
    this.menuElement.style.display = 'block';
    const rect = this.menuElement.getBoundingClientRect();
    const menuWidth = rect.width;
    const menuHeight = rect.height;

    // Get viewport dimensions
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    // Calculate position with boundary checking
    let menuX = x;
    let menuY = y;

    // Adjust horizontal position if menu would overflow
    if (x + menuWidth > viewportWidth) {
      menuX = Math.max(0, x - menuWidth);
    }

    // Adjust vertical position if menu would overflow
    if (y + menuHeight > viewportHeight) {
      menuY = Math.max(0, y - menuHeight);
    }

    // Apply position
    this.menuElement.style.left = `${menuX}px`;
    this.menuElement.style.top = `${menuY}px`;
  }

  /**
   * Set up active event listeners when menu is open
   */
  setupActiveListeners() {
    // Click outside to close
    this.clickOutsideHandler = (event) => {
      if (!this.menuElement.contains(event.target)) {
        this.hide();
      }
    };
    document.addEventListener('click', this.clickOutsideHandler, true);

    // Keyboard navigation
    this.keyHandler = (event) => {
      this.handleKeyboard(event);
    };
    this.menuElement.addEventListener('keydown', this.keyHandler);
  }

  /**
   * Clean up active event listeners
   */
  cleanupActiveListeners() {
    if (this.clickOutsideHandler) {
      document.removeEventListener('click', this.clickOutsideHandler, true);
      this.clickOutsideHandler = null;
    }

    if (this.keyHandler) {
      this.menuElement.removeEventListener('keydown', this.keyHandler);
      this.keyHandler = null;
    }
  }

  /**
   * Handle keyboard navigation in menu
   * @param {KeyboardEvent} event - Keyboard event
   */
  handleKeyboard(event) {
    const menuItems = Array.from(this.menuElement.querySelectorAll('.context-menu-item:not([disabled])'));
    const currentIndex = menuItems.findIndex(item => item === document.activeElement);

    switch (event.key) {
      case 'ArrowUp':
        event.preventDefault();
        const prevIndex = currentIndex <= 0 ? menuItems.length - 1 : currentIndex - 1;
        menuItems[prevIndex]?.focus();
        break;

      case 'ArrowDown':
        event.preventDefault();
        const nextIndex = currentIndex >= menuItems.length - 1 ? 0 : currentIndex + 1;
        menuItems[nextIndex]?.focus();
        break;

      case 'Enter':
      case ' ':
        event.preventDefault();
        if (currentIndex >= 0) {
          menuItems[currentIndex]?.click();
        }
        break;

      case 'Escape':
        event.preventDefault();
        this.hide();
        break;

      case 'Tab':
        event.preventDefault();
        // Tab closes menu and returns focus to original target
        this.hide();
        if (this.currentTarget) {
          this.currentTarget.focus();
        }
        break;
    }
  }

  /**
   * Set up general event listeners
   */
  setupEventListeners() {
    // Global escape key to close menu
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && this.isOpen) {
        this.hide();
      }
    });

    // Window resize/scroll to hide menu
    window.addEventListener('resize', () => {
      if (this.isOpen) {
        this.hide();
      }
    });

    window.addEventListener('scroll', () => {
      if (this.isOpen) {
        this.hide();
      }
    });
  }

  /**
   * Handle New File action
   */
  async handleNewFile() {
    const currentVault = this.appState.getState().currentVault;
    if (!currentVault) {
      this.showNotification('Please select a vault first', 'warning');
      return;
    }

    // Determine parent folder
    let parentPath = currentVault;
    if (this.currentFile && this.currentFile.is_dir) {
      parentPath = this.currentFile.path;
    } else if (this.currentFile) {
      // Get parent folder of the file
      parentPath = this.currentFile.path.substring(0, this.currentFile.path.lastIndexOf('/'));
    }

    const fileName = await this.promptForInput('Enter file name:', '', 'new-note.md');
    if (!fileName) return;

    if (!fileName.endsWith('.md')) {
      this.showNotification('File name must end with .md', 'warning');
      return;
    }

    const fullPath = `${parentPath}/${fileName}`;

    try {
      await window.__TAURI__.core.invoke('create_file', { file_path: fullPath });
      await window.__TAURI__.core.invoke('write_file', { 
        file_path: fullPath, 
        content: `# ${fileName.replace('.md', '')}\n\n` 
      });

      // Refresh file tree
      await this.refreshVault();
      
      this.showNotification(`Created: ${fileName}`, 'success');
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { 
        action: 'new_file', 
        file: { path: fullPath, name: fileName } 
      });
    } catch (error) {
      this.showNotification(`Error creating file: ${error}`, 'error');
    }
  }

  /**
   * Handle New Folder action
   */
  async handleNewFolder() {
    const currentVault = this.appState.getState().currentVault;
    if (!currentVault) {
      this.showNotification('Please select a vault first', 'warning');
      return;
    }

    // Determine parent folder
    let parentPath = currentVault;
    if (this.currentFile && this.currentFile.is_dir) {
      parentPath = this.currentFile.path;
    } else if (this.currentFile) {
      // Get parent folder of the file
      parentPath = this.currentFile.path.substring(0, this.currentFile.path.lastIndexOf('/'));
    }

    const folderName = await this.promptForInput('Enter folder name:', '', 'New Folder');
    if (!folderName) return;

    // Validate folder name
    if (folderName.includes('/') || folderName.includes('\\')) {
      this.showNotification('Folder name cannot contain slashes', 'warning');
      return;
    }

    const fullPath = `${parentPath}/${folderName}`;

    try {
      await window.__TAURI__.core.invoke('create_folder', { folder_path: fullPath });

      // Refresh file tree
      await this.refreshVault();
      
      this.showNotification(`Created folder: ${folderName}`, 'success');
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { 
        action: 'new_folder', 
        file: { path: fullPath, name: folderName } 
      });
    } catch (error) {
      this.showNotification(`Error creating folder: ${error}`, 'error');
    }
  }

  /**
   * Handle Rename action
   * @param {Object} file - File to rename
   */
  async handleRename(file) {
    const currentName = file.name;
    const newName = await this.promptForInput('Rename to:', currentName, currentName);
    if (!newName || newName === currentName) return;

    // Validate new name
    if (newName.includes('/') || newName.includes('\\')) {
      this.showNotification('Name cannot contain slashes', 'warning');
      return;
    }

    if (!file.is_dir && !newName.endsWith('.md')) {
      this.showNotification('File name must end with .md', 'warning');
      return;
    }

    const parentPath = file.path.substring(0, file.path.lastIndexOf('/'));
    const newPath = `${parentPath}/${newName}`;

    try {
      await window.__TAURI__.core.invoke('rename_file', { 
        old_path: file.path, 
        new_path: newPath 
      });

      // Update current file path if it was the renamed file
      const currentFile = this.appState.getState().currentFile;
      if (currentFile === file.path) {
        this.appState.setCurrentFile(newPath);
      }

      // Refresh file tree
      await this.refreshVault();
      
      this.showNotification(`Renamed to: ${newName}`, 'success');
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { 
        action: 'rename', 
        oldFile: file,
        newFile: { ...file, path: newPath, name: newName }
      });
    } catch (error) {
      this.showNotification(`Error renaming: ${error}`, 'error');
    }
  }

  /**
   * Handle Delete action with confirmation
   * @param {Object} file - File to delete
   */
  async handleDelete(file) {
    const itemType = file.is_dir ? 'folder' : 'file';
    const confirmed = await this.showConfirmation(
      `Delete ${itemType}?`,
      `Are you sure you want to delete the ${itemType} "${file.name}"? This action cannot be undone.`,
      'Delete',
      'dangerous'
    );

    if (!confirmed) return;

    try {
      await window.__TAURI__.core.invoke('delete_file', { file_path: file.path });

      // Clear current file if it was the deleted file
      const currentFile = this.appState.getState().currentFile;
      if (currentFile === file.path) {
        this.appState.setCurrentFile(null);
      }

      // Refresh file tree
      await this.refreshVault();
      
      this.showNotification(`Deleted: ${file.name}`, 'success');
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { 
        action: 'delete', 
        file: file
      });
    } catch (error) {
      this.showNotification(`Error deleting: ${error}`, 'error');
    }
  }

  /**
   * Handle Refresh action
   */
  async handleRefresh() {
    try {
      await this.refreshVault();
      this.showNotification('Vault refreshed', 'success');
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { action: 'refresh' });
    } catch (error) {
      this.showNotification(`Error refreshing: ${error}`, 'error');
    }
  }

  /**
   * Handle Reveal in Finder/Explorer action
   * @param {Object} file - File to reveal
   */
  async handleReveal(file) {
    try {
      await window.__TAURI__.core.invoke('reveal_in_finder', { file_path: file.path });
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { action: 'reveal', file: file });
    } catch (error) {
      // Fallback: show the path in a notification
      this.showNotification(`File location: ${file.path}`, 'info');
      this.emit(ContextMenu.EVENTS.ACTION_EXECUTED, { action: 'reveal', file: file });
    }
  }

  /**
   * Show a simple input prompt
   * @param {string} message - Prompt message
   * @param {string} defaultValue - Default input value
   * @param {string} placeholder - Input placeholder
   * @returns {Promise<string|null>} User input or null if cancelled
   */
  async promptForInput(message, defaultValue = '', placeholder = '') {
    return new Promise((resolve) => {
      const dialog = document.createElement('div');
      dialog.className = 'input-dialog-overlay';
      dialog.innerHTML = `
        <div class="input-dialog">
          <div class="input-dialog-header">
            <h3>${message}</h3>
          </div>
          <div class="input-dialog-body">
            <input type="text" class="input-dialog-input" value="${defaultValue}" placeholder="${placeholder}" />
          </div>
          <div class="input-dialog-footer">
            <button type="button" class="btn-secondary input-dialog-cancel">Cancel</button>
            <button type="button" class="btn-primary input-dialog-confirm">OK</button>
          </div>
        </div>
      `;

      document.body.appendChild(dialog);

      const input = dialog.querySelector('.input-dialog-input');
      const cancelBtn = dialog.querySelector('.input-dialog-cancel');
      const confirmBtn = dialog.querySelector('.input-dialog-confirm');

      // Focus and select text
      input.focus();
      input.select();

      const cleanup = () => {
        document.body.removeChild(dialog);
      };

      const confirm = () => {
        const value = input.value.trim();
        cleanup();
        resolve(value || null);
      };

      const cancel = () => {
        cleanup();
        resolve(null);
      };

      // Event listeners
      confirmBtn.addEventListener('click', confirm);
      cancelBtn.addEventListener('click', cancel);
      
      input.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          confirm();
        } else if (event.key === 'Escape') {
          event.preventDefault();
          cancel();
        }
      });

      // Click outside to cancel
      dialog.addEventListener('click', (event) => {
        if (event.target === dialog) {
          cancel();
        }
      });
    });
  }

  /**
   * Show a confirmation dialog
   * @param {string} title - Dialog title
   * @param {string} message - Dialog message
   * @param {string} confirmText - Confirm button text
   * @param {string} type - Dialog type (normal, dangerous)
   * @returns {Promise<boolean>} True if confirmed, false if cancelled
   */
  async showConfirmation(title, message, confirmText = 'OK', type = 'normal') {
    return new Promise((resolve) => {
      const dialog = document.createElement('div');
      dialog.className = 'confirmation-dialog-overlay';
      dialog.innerHTML = `
        <div class="confirmation-dialog">
          <div class="confirmation-dialog-header">
            <h3>${title}</h3>
          </div>
          <div class="confirmation-dialog-body">
            <p>${message}</p>
          </div>
          <div class="confirmation-dialog-footer">
            <button type="button" class="btn-secondary confirmation-dialog-cancel">Cancel</button>
            <button type="button" class="btn-${type === 'dangerous' ? 'danger' : 'primary'} confirmation-dialog-confirm">${confirmText}</button>
          </div>
        </div>
      `;

      document.body.appendChild(dialog);

      const cancelBtn = dialog.querySelector('.confirmation-dialog-cancel');
      const confirmBtn = dialog.querySelector('.confirmation-dialog-confirm');

      // Focus cancel button by default for safety
      cancelBtn.focus();

      const cleanup = () => {
        document.body.removeChild(dialog);
      };

      const confirm = () => {
        cleanup();
        resolve(true);
      };

      const cancel = () => {
        cleanup();
        resolve(false);
      };

      // Event listeners
      confirmBtn.addEventListener('click', confirm);
      cancelBtn.addEventListener('click', cancel);

      // Keyboard handling
      dialog.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          cancel();
        }
      });

      // Click outside to cancel
      dialog.addEventListener('click', (event) => {
        if (event.target === dialog) {
          cancel();
        }
      });
    });
  }

  /**
   * Refresh vault files
   */
  async refreshVault() {
    if (window.refreshVault) {
      await window.refreshVault();
    }
  }

  /**
   * Show notification message
   * @param {string} message - Message to display
   * @param {string} type - Type of notification
   */
  showNotification(message, type = 'info') {
    if (window.showNotification) {
      window.showNotification(message, type);
    } else {
      console.log(`[${type.toUpperCase()}] ${message}`);
    }
  }

  /**
   * Emit custom events
   * @param {string} event - Event name
   * @param {Object} data - Event data
   */
  emit(event, data = {}) {
    const customEvent = new CustomEvent(event, { 
      detail: data,
      bubbles: true 
    });
    document.dispatchEvent(customEvent);
  }

  /**
   * Clean up component resources
   */
  destroy() {
    this.hide();
    this.cleanupActiveListeners();
    
    if (this.menuElement && this.menuElement.parentNode) {
      this.menuElement.parentNode.removeChild(this.menuElement);
    }
    
    this.menuElement = null;
    this.appState = null;
  }
}

// Export for ES6 module usage
export default ContextMenu;