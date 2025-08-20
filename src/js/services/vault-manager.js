/**
 * VaultManager - Handles vault selection, validation, and management
 * 
 * Provides vault management functionality including selection, validation,
 * persistence, and switching between vaults without application restart.
 * Integrates with AppState for state management and event emission.
 * 
 * @class VaultManager
 */
class VaultManager {
  /**
   * Initialize VaultManager with AppState integration
   * @param {AppState} appState - Application state management instance
   */
  constructor(appState) {
    if (!appState) {
      throw new Error('AppState instance is required for VaultManager');
    }
    
    this.appState = appState;
    this.currentVaultPath = null;
    this.recentVaults = [];
    this.maxRecentVaults = 5;
    
    // Load persisted vault preferences
    this.loadVaultPreferences();
    
    // Restore current vault from app state
    this.currentVaultPath = appState.currentVault;
  }

  /**
   * Open native folder picker to select a new vault
   * @returns {Promise<string|null>} Selected vault path or null if cancelled
   */
  async selectVault() {
    try {
      const selectedPath = await window.__TAURI__.core.invoke('select_vault');
      
      if (selectedPath) {
        // Validate the selected vault before accepting it
        const isValid = await this.validateVault(selectedPath);
        
        if (!isValid) {
          throw new Error(`Selected folder is not accessible: ${selectedPath}`);
        }
        
        return selectedPath;
      }
      
      return null;
    } catch (error) {
      console.error('Failed to select vault:', error);
      throw new Error(`Vault selection failed: ${error.message}`);
    }
  }

  /**
   * Validate that a vault path is accessible and suitable
   * @param {string} vaultPath - Path to validate
   * @returns {Promise<boolean>} True if vault is valid and accessible
   */
  async validateVault(vaultPath) {
    try {
      if (!vaultPath || typeof vaultPath !== 'string') {
        return false;
      }

      // Use backend validation command
      const isValid = await window.__TAURI__.core.invoke('validate_vault', {
        vaultPath: vaultPath
      });
      
      return isValid === true;
    } catch (error) {
      console.error('Vault validation error:', error);
      return false;
    }
  }

  /**
   * Load files from the current vault
   * @param {string} vaultPath - Path to the vault to load
   * @returns {Promise<Array>} Array of file information objects
   */
  async loadVault(vaultPath) {
    try {
      if (!vaultPath) {
        throw new Error('Vault path is required');
      }

      // Validate vault before loading
      const isValid = await this.validateVault(vaultPath);
      if (!isValid) {
        throw new Error(`Invalid or inaccessible vault: ${vaultPath}`);
      }

      // Load vault files using backend command
      const files = await window.__TAURI__.core.invoke('load_vault', {
        vaultPath: vaultPath
      });

      if (!Array.isArray(files)) {
        throw new Error('Invalid vault files response format');
      }

      // Update app state with loaded files
      this.appState.setFiles(files);

      return files;
    } catch (error) {
      console.error('Failed to load vault:', error);
      throw new Error(`Vault loading failed: ${error.message}`);
    }
  }

  /**
   * Switch to a different vault
   * @param {string} newVaultPath - Path to the new vault
   * @returns {Promise<void>}
   */
  async switchVault(newVaultPath) {
    try {
      if (!newVaultPath) {
        throw new Error('New vault path is required');
      }

      // Validate the new vault
      const isValid = await this.validateVault(newVaultPath);
      if (!isValid) {
        throw new Error(`Invalid vault path: ${newVaultPath}`);
      }

      const previousVault = this.currentVaultPath;

      // Load the new vault to ensure it's accessible
      const files = await this.loadVault(newVaultPath);

      // Update current vault path
      this.currentVaultPath = newVaultPath;

      // Update app state
      await this.appState.setVault(newVaultPath);

      // Save to recent vaults
      this.addToRecentVaults(newVaultPath);
      
      // Persist vault preference
      this.saveVaultPreference(newVaultPath);

      console.log(`Switched vault from "${previousVault}" to "${newVaultPath}". Loaded ${files.length} items.`);

    } catch (error) {
      console.error('Failed to switch vault:', error);
      throw new Error(`Vault switching failed: ${error.message}`);
    }
  }

  /**
   * Set up a vault on first application launch
   * @returns {Promise<string|null>} Selected vault path or null if cancelled
   */
  async setupInitialVault() {
    try {
      // Check if we already have a vault in preferences
      const savedVault = this.loadVaultPreference();
      
      if (savedVault) {
        // Validate the saved vault is still accessible
        const isValid = await this.validateVault(savedVault);
        
        if (isValid) {
          // Automatically load the saved vault
          await this.switchVault(savedVault);
          return savedVault;
        }
        
        // Saved vault is no longer valid, clear it
        this.clearVaultPreference();
      }

      // No valid saved vault, prompt user to select one
      const selectedPath = await this.selectVault();
      
      if (selectedPath) {
        await this.switchVault(selectedPath);
        return selectedPath;
      }

      return null;
    } catch (error) {
      console.error('Failed to setup initial vault:', error);
      throw new Error(`Initial vault setup failed: ${error.message}`);
    }
  }

  /**
   * Get the current vault path
   * @returns {string|null} Current vault path
   */
  getCurrentVault() {
    return this.currentVaultPath;
  }

  /**
   * Get list of recently used vaults
   * @returns {Array<string>} Array of recent vault paths
   */
  getRecentVaults() {
    return [...this.recentVaults];
  }

  /**
   * Add a vault to the recent vaults list
   * @param {string} vaultPath - Path to add to recent list
   */
  addToRecentVaults(vaultPath) {
    if (!vaultPath || typeof vaultPath !== 'string') {
      return;
    }

    // Remove if already exists to avoid duplicates
    this.recentVaults = this.recentVaults.filter(path => path !== vaultPath);
    
    // Add to beginning of list
    this.recentVaults.unshift(vaultPath);
    
    // Limit to max recent vaults
    if (this.recentVaults.length > this.maxRecentVaults) {
      this.recentVaults = this.recentVaults.slice(0, this.maxRecentVaults);
    }
    
    // Persist recent vaults
    this.saveRecentVaults();
  }

  /**
   * Save current vault preference to localStorage
   * @param {string} vaultPath - Vault path to save
   */
  saveVaultPreference(vaultPath) {
    try {
      if (vaultPath && typeof vaultPath === 'string') {
        localStorage.setItem('ainote_current_vault', vaultPath);
      }
    } catch (error) {
      console.error('Failed to save vault preference:', error);
    }
  }

  /**
   * Load saved vault preference from localStorage
   * @returns {string|null} Saved vault path or null
   */
  loadVaultPreference() {
    try {
      return localStorage.getItem('ainote_current_vault');
    } catch (error) {
      console.error('Failed to load vault preference:', error);
      return null;
    }
  }

  /**
   * Clear saved vault preference
   */
  clearVaultPreference() {
    try {
      localStorage.removeItem('ainote_current_vault');
    } catch (error) {
      console.error('Failed to clear vault preference:', error);
    }
  }

  /**
   * Load all vault preferences including recent vaults
   */
  loadVaultPreferences() {
    // Load recent vaults
    try {
      const savedRecent = localStorage.getItem('ainote_recent_vaults');
      if (savedRecent) {
        const parsed = JSON.parse(savedRecent);
        if (Array.isArray(parsed)) {
          this.recentVaults = parsed.slice(0, this.maxRecentVaults);
        }
      }
    } catch (error) {
      console.error('Failed to load recent vaults:', error);
      this.recentVaults = [];
    }
  }

  /**
   * Save recent vaults to localStorage
   */
  saveRecentVaults() {
    try {
      localStorage.setItem('ainote_recent_vaults', JSON.stringify(this.recentVaults));
    } catch (error) {
      console.error('Failed to save recent vaults:', error);
    }
  }

  /**
   * Clear all vault preferences and reset state
   * @returns {Promise<void>}
   */
  async clearAllVaultData() {
    try {
      // Clear current vault
      this.currentVaultPath = null;
      
      // Clear recent vaults
      this.recentVaults = [];
      
      // Clear localStorage
      this.clearVaultPreference();
      localStorage.removeItem('ainote_recent_vaults');
      
      // Reset app state
      await this.appState.setVault(null);
      this.appState.setFiles([]);
      
      console.log('All vault data cleared');
    } catch (error) {
      console.error('Failed to clear vault data:', error);
      throw new Error(`Failed to clear vault data: ${error.message}`);
    }
  }

  /**
   * Get vault statistics and information
   * @returns {Promise<Object>} Vault statistics object
   */
  async getVaultStats() {
    try {
      if (!this.currentVaultPath) {
        return {
          vaultPath: null,
          fileCount: 0,
          directoryCount: 0,
          totalSize: 0,
          lastScanned: null
        };
      }

      const files = this.appState.files || [];
      const markdownFiles = files.filter(f => !f.is_dir && f.name.endsWith('.md'));
      const directories = files.filter(f => f.is_dir);

      return {
        vaultPath: this.currentVaultPath,
        fileCount: markdownFiles.length,
        directoryCount: directories.length,
        totalSize: markdownFiles.reduce((total, file) => total + (file.size || 0), 0),
        lastScanned: new Date().toISOString()
      };
    } catch (error) {
      console.error('Failed to get vault stats:', error);
      throw new Error(`Failed to get vault statistics: ${error.message}`);
    }
  }

  /**
   * Refresh the current vault by re-scanning files
   * @returns {Promise<Array>} Updated files array
   */
  async refreshVault() {
    try {
      if (!this.currentVaultPath) {
        throw new Error('No vault currently loaded');
      }

      console.log(`Refreshing vault: ${this.currentVaultPath}`);
      
      // Re-load the vault to get updated file list
      const files = await this.loadVault(this.currentVaultPath);
      
      console.log(`Vault refreshed. Found ${files.length} items.`);
      return files;
    } catch (error) {
      console.error('Failed to refresh vault:', error);
      throw new Error(`Vault refresh failed: ${error.message}`);
    }
  }

  /**
   * Check if vault manager is properly initialized
   * @returns {boolean} True if properly initialized
   */
  isInitialized() {
    return !!(this.appState && typeof this.appState.setVault === 'function');
  }

  /**
   * Get vault manager status for debugging
   * @returns {Object} Status object with current state
   */
  getStatus() {
    return {
      initialized: this.isInitialized(),
      currentVault: this.currentVaultPath,
      recentVaultsCount: this.recentVaults.length,
      appStateVault: this.appState?.currentVault || null,
      filesLoaded: this.appState?.files?.length || 0
    };
  }
}

// Export for ES6 module usage
export default VaultManager;