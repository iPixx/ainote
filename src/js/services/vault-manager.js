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
    
    // Load persisted vault preferences (async, but don't block constructor)
    this.loadVaultPreferences().catch(error => {
      console.error('Failed to load vault preferences during initialization:', error);
    });
    
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
      console.log('🔍 VaultManager: Selected path from dialog:', selectedPath);
      
      if (selectedPath) {
        console.log('✅ VaultManager: Path selected, skipping validation during selection');
        // Skip validation during selection - if user selected it from dialog, it should be valid
        // We'll validate later during actual usage if needed
        return selectedPath;
      }
      
      return null;
    } catch (error) {
      console.error('❌ VaultManager: Failed to select vault:', error);
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
        console.log('🔍 VaultManager: Invalid vault path type:', typeof vaultPath, vaultPath);
        return false;
      }

      console.log('🔧 VaultManager: Calling backend validate_vault with path:', vaultPath);
      
      // Use backend validation command
      const isValid = await window.__TAURI__.core.invoke('validate_vault', { vaultPath });
      
      console.log('✅ VaultManager: Backend validation result:', isValid);
      return isValid === true;
    } catch (error) {
      console.error('❌ VaultManager: Vault validation error:', error);
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
      const files = await window.__TAURI__.core.invoke('load_vault', { vaultPath });

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

      console.log('🔄 VaultManager: Switching to vault:', newVaultPath);
      const previousVault = this.currentVaultPath;

      // Try to load the vault - this will validate accessibility
      let files = [];
      try {
        files = await this.loadVault(newVaultPath);
        console.log('✅ VaultManager: Successfully loaded vault with', files.length, 'items');
      } catch (loadError) {
        console.warn('⚠️ VaultManager: Failed to load vault, but proceeding anyway:', loadError);
        // If loading fails, still proceed but with empty file list
        // This allows users to select empty folders or folders with permission issues
      }

      // Update current vault path
      this.currentVaultPath = newVaultPath;

      // Update app state
      await this.appState.setVault(newVaultPath);

      // Save to recent vaults
      await this.addToRecentVaults(newVaultPath);
      
      // Persist vault preference
      this.saveVaultPreference(newVaultPath);

      console.log(`✅ VaultManager: Switched vault from "${previousVault}" to "${newVaultPath}". Loaded ${files.length} items.`);

    } catch (error) {
      console.error('❌ VaultManager: Failed to switch vault:', error);
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
  async addToRecentVaults(vaultPath) {
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
    await this.saveRecentVaults();
  }

  /**
   * Save current vault preference to app_state.json (current vault is handled by AppState)
   * This method is kept for API compatibility but delegates to AppState
   * @param {string} vaultPath - Vault path to save
   */
  saveVaultPreference(vaultPath) {
    // Current vault is automatically saved by AppState.setVault()
    // This method is kept for API compatibility
    console.log(`Vault preference automatically saved via AppState: ${vaultPath}`);
  }

  /**
   * Load saved vault preference from app_state.json via AppState
   * @returns {string|null} Saved vault path or null
   */
  loadVaultPreference() {
    // Current vault is loaded by AppState on initialization
    return this.appState.currentVault;
  }

  /**
   * Clear saved vault preference (delegates to AppState)
   */
  clearVaultPreference() {
    // This will be handled by AppState.reset() or setVault(null)
    this.appState.setVault(null);
  }

  /**
   * Load all vault preferences including recent vaults from app_state.json
   */
  async loadVaultPreferences() {
    try {
      const recentVaults = await window.__TAURI__.core.invoke('get_vault_preferences');
      if (Array.isArray(recentVaults)) {
        this.recentVaults = recentVaults.slice(0, this.maxRecentVaults);
      } else {
        this.recentVaults = [];
      }
    } catch (error) {
      console.error('Failed to load recent vaults from app_state.json:', error);
      this.recentVaults = [];
    }
  }

  /**
   * Save recent vaults to app_state.json via backend
   */
  async saveRecentVaults() {
    try {
      await window.__TAURI__.core.invoke('save_vault_preferences', { recentVaults: this.recentVaults });
    } catch (error) {
      console.error('Failed to save recent vaults to app_state.json:', error);
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
      
      // Clear app_state.json preferences
      this.clearVaultPreference();
      await this.saveRecentVaults(); // Save empty recent vaults array
      
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