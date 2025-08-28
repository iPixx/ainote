/**
 * Integration tests for VaultManager + AppState integration
 * 
 * Tests cover:
 * - Vault switching with state persistence
 * - File loading and state synchronization
 * - Event coordination between components
 * - Error handling across component boundaries
 * - State consistency during operations
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import components to test
import VaultManager from '../../src/js/services/vault-manager.js';
import AppState from '../../src/js/state.js';

describe('VaultManager + AppState Integration', () => {
  let vaultManager;
  let appState;
  let tauriMocks;

  beforeEach(() => {
    // Set up Tauri mocks
    tauriMocks = setupTauriMocks();
    
    // Create integrated instances
    appState = new AppState();
    vaultManager = new VaultManager(appState);
    
    // Mock successful operations by default
    tauriMocks.invoke.mockImplementation((command, params) => {
      switch (command) {
        case 'validate_vault':
          return Promise.resolve(true);
        case 'load_vault':
          return Promise.resolve([
            { name: 'file1.md', path: params?.vaultPath + '/file1.md', is_dir: false },
            { name: 'folder1', path: params?.vaultPath + '/folder1', is_dir: true },
            { name: 'file2.md', path: params?.vaultPath + '/folder1/file2.md', is_dir: false }
          ]);
        case 'save_session_state':
          return Promise.resolve(true);
        case 'save_vault_preferences':
          return Promise.resolve(true);
        case 'get_vault_preferences':
          return Promise.resolve([]);
        case 'load_app_state':
          return Promise.resolve({
            session: {
              current_vault: null,
              current_file: null,
              view_mode: 'editor'
            }
          });
        default:
          return Promise.resolve(null);
      }
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Vault Switching Integration', () => {
    it('should synchronize vault state between components', async () => {
      const vaultPath = '/test/vault';
      
      // Listen for state changes
      const stateChanges = [];
      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, (data) => {
        stateChanges.push(data);
      });
      
      appState.addEventListener(AppState.EVENTS.FILES_UPDATED, (data) => {
        stateChanges.push(data);
      });

      await vaultManager.switchVault(vaultPath);

      // Verify state synchronization
      expect(appState.currentVault).toBe(vaultPath);
      expect(vaultManager.currentVaultPath).toBe(vaultPath);
      expect(appState.files).toHaveLength(3);
      
      // Verify events were emitted in correct order
      expect(stateChanges).toHaveLength(2);
      expect(stateChanges[0].vault).toBe(vaultPath);
      expect(stateChanges[1].files).toHaveLength(3);
    });

    it('should handle vault switching with state persistence', async () => {
      const vaultPath = '/test/vault';

      await vaultManager.switchVault(vaultPath);

      // Verify state was persisted
      expect(tauriMocks.invoke).toHaveBeenCalledWith('save_session_state', {
        currentVault: vaultPath,
        currentFile: null,
        viewMode: 'editor'
      });

      // Verify vault preferences were updated
      expect(tauriMocks.invoke).toHaveBeenCalledWith('save_vault_preferences', {
        recentVaults: [vaultPath]
      });
    });

    it('should maintain state consistency during vault switching', async () => {
      // Set initial state
      await appState.setCurrentFile('/old/vault/file.md');
      appState.markDirty(true);

      const newVaultPath = '/new/vault';
      await vaultManager.switchVault(newVaultPath);

      // Verify state was properly cleared and updated
      expect(appState.currentVault).toBe(newVaultPath);
      expect(appState.currentFile).toBeNull(); // Should be cleared
      expect(appState.unsavedChanges).toBe(false); // Should be reset
      expect(appState.files).toHaveLength(3); // New vault files
    });

    it('should handle concurrent state operations correctly', async () => {
      const vault1 = '/vault1';
      const vault2 = '/vault2';

      // Start concurrent vault switches
      const promise1 = vaultManager.switchVault(vault1);
      const promise2 = vaultManager.switchVault(vault2);

      await Promise.all([promise1, promise2]);

      // Last operation should win
      expect(appState.currentVault).toBe(vault2);
      expect(vaultManager.currentVaultPath).toBe(vault2);
    });
  });

  describe('File State Integration', () => {
    beforeEach(async () => {
      await vaultManager.switchVault('/test/vault');
    });

    it('should synchronize file selection with app state', async () => {
      const filePath = '/vault/file1.md';
      const fileChanges = [];
      
      appState.addEventListener(AppState.EVENTS.FILE_CHANGED, (data) => {
        fileChanges.push(data);
      });

      await appState.setCurrentFile(filePath);

      expect(appState.currentFile).toBe(filePath);
      expect(fileChanges).toHaveLength(1);
      expect(fileChanges[0].file).toBe(filePath);
    });

    it('should maintain file state during vault operations', async () => {
      // Select a file
      await appState.setCurrentFile('/vault/file1.md');
      expect(appState.currentFile).toBe('/vault/file1.md');

      // Refresh vault - file selection should be maintained if file still exists
      await vaultManager.refreshVault();

      expect(appState.files).toHaveLength(3);
      // File should still be selected since it exists in refreshed vault
      expect(appState.currentFile).toBe('/vault/file1.md');
    });

    it('should clear file state when switching to vault without that file', async () => {
      // Select a file
      await appState.setCurrentFile('/vault/file1.md');
      
      // Switch to different vault
      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'load_vault') {
          return Promise.resolve([
            { name: 'different.md', path: '/new/vault/different.md', is_dir: false }
          ]);
        }
        return Promise.resolve(true);
      });

      await vaultManager.switchVault('/new/vault');

      expect(appState.currentFile).toBeNull();
      expect(appState.files).toHaveLength(1);
      expect(appState.files[0].name).toBe('different.md');
    });
  });

  describe('Error Handling Integration', () => {
    it('should handle vault validation failures gracefully', async () => {
      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'validate_vault') {
          return Promise.resolve(false);
        }
        return Promise.resolve(true);
      });

      await expect(vaultManager.switchVault('/invalid/vault')).rejects.toThrow();
      
      // App state should remain unchanged
      expect(appState.currentVault).toBeNull();
      expect(appState.files).toEqual([]);
    });

    it('should handle vault loading failures with state recovery', async () => {
      const originalVault = '/original/vault';
      await vaultManager.switchVault(originalVault);
      
      // Mock loading failure for new vault
      tauriMocks.invoke.mockImplementation((command, params) => {
        if (command === 'validate_vault' && params?.vaultPath === '/failing/vault') {
          return Promise.reject(new Error('Load failed'));
        }
        if (command === 'save_session_state' || command === 'save_vault_preferences') {
          return Promise.resolve(true);
        }
        return Promise.resolve(true);
      });

      // This should proceed despite loading failure (as per VaultManager design)
      await vaultManager.switchVault('/failing/vault');
      
      // VaultManager allows switching even if loading fails
      expect(vaultManager.currentVaultPath).toBe('/failing/vault');
      expect(appState.currentVault).toBe('/failing/vault');
    });

    it('should handle state persistence failures', async () => {
      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'save_session_state') {
          return Promise.reject(new Error('Save failed'));
        }
        if (command === 'validate_vault') return Promise.resolve(true);
        if (command === 'load_vault') return Promise.resolve([]);
        return Promise.resolve(true);
      });

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      // Should not throw, but log error
      await vaultManager.switchVault('/test/vault');

      expect(vaultManager.currentVaultPath).toBe('/test/vault');
      expect(consoleSpy).toHaveBeenCalledWith(
        'Failed to save session state:',
        expect.any(Error)
      );
      
      consoleSpy.mockRestore();
    });
  });

  describe('Event Coordination', () => {
    it('should coordinate events between VaultManager and AppState', async () => {
      const events = [];
      
      // Listen to all relevant events
      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, (data) => {
        events.push({ type: 'vault_changed', data });
      });
      
      appState.addEventListener(AppState.EVENTS.FILES_UPDATED, (data) => {
        events.push({ type: 'files_updated', data });
      });
      
      appState.addEventListener(AppState.EVENTS.FILE_CHANGED, (data) => {
        events.push({ type: 'file_changed', data });
      });

      // Perform integrated operations
      await vaultManager.switchVault('/test/vault');
      await appState.setCurrentFile('/vault/file1.md');
      appState.markDirty(true);

      // Verify event sequence
      expect(events).toHaveLength(3);
      expect(events[0].type).toBe('vault_changed');
      expect(events[1].type).toBe('files_updated');
      expect(events[2].type).toBe('file_changed');
    });

    it('should handle event listener errors without breaking integration', async () => {
      const errorListener = vi.fn(() => { throw new Error('Listener error'); });
      const successListener = vi.fn();
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, errorListener);
      appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, successListener);

      await vaultManager.switchVault('/test/vault');

      // Both listeners should have been called despite error
      expect(errorListener).toHaveBeenCalled();
      expect(successListener).toHaveBeenCalled();
      expect(consoleSpy).toHaveBeenCalled();
      
      // State should still be updated correctly
      expect(appState.currentVault).toBe('/test/vault');
      
      consoleSpy.mockRestore();
    });
  });

  describe('Performance Integration', () => {
    it('should complete integrated operations within performance targets', async () => {
      const startTime = performance.now();
      
      await vaultManager.switchVault('/test/vault');
      await appState.setCurrentFile('/vault/file1.md');
      await appState.setViewMode(AppState.VIEW_MODES.PREVIEW);
      
      const totalTime = performance.now() - startTime;
      
      // With mocked operations, this should be very fast
      expect(totalTime).toBeLessThan(50);
    });

    it('should handle large file lists efficiently', async () => {
      const largeFileList = Array.from({ length: 1000 }, (_, i) => ({
        name: `file${i}.md`,
        path: `/vault/file${i}.md`,
        is_dir: false
      }));

      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'load_vault') {
          return Promise.resolve(largeFileList);
        }
        return Promise.resolve(true);
      });

      const startTime = performance.now();
      await vaultManager.switchVault('/large/vault');
      const operationTime = performance.now() - startTime;

      expect(appState.files).toHaveLength(1000);
      expect(operationTime).toBeLessThan(100); // Should handle large lists efficiently
    });
  });

  describe('State Consistency', () => {
    it('should maintain consistency during rapid operations', async () => {
      const operations = [
        () => vaultManager.switchVault('/vault1'),
        () => appState.setCurrentFile('/vault1/file.md'),
        () => appState.markDirty(true),
        () => vaultManager.switchVault('/vault2'),
        () => appState.setViewMode(AppState.VIEW_MODES.PREVIEW)
      ];

      // Execute operations rapidly
      await Promise.all(operations.map(op => op()));

      // Final state should be consistent
      expect(appState.currentVault).toBe('/vault2');
      expect(vaultManager.currentVaultPath).toBe('/vault2');
      expect(appState.viewMode).toBe(AppState.VIEW_MODES.PREVIEW);
      expect(appState.isValid()).toBe(true);
    });

    it('should recover from inconsistent state', async () => {
      // Create inconsistent state
      vaultManager.currentVaultPath = '/vault1';
      appState.currentVault = '/vault2';

      // Force consistency through vault switch
      await vaultManager.switchVault('/consistent/vault');

      expect(appState.currentVault).toBe('/consistent/vault');
      expect(vaultManager.currentVaultPath).toBe('/consistent/vault');
    });

    it('should validate state integrity after operations', async () => {
      await vaultManager.switchVault('/test/vault');
      await appState.setCurrentFile('/vault/file1.md');
      await appState.setViewMode(AppState.VIEW_MODES.PREVIEW);

      // Verify integrated state is valid
      expect(appState.isValid()).toBe(true);
      expect(vaultManager.isInitialized()).toBe(true);
      
      const vaultStats = await vaultManager.getVaultStats();
      expect(vaultStats.vaultPath).toBe('/test/vault');
      expect(vaultStats.fileCount).toBeGreaterThan(0);
    });
  });

  describe('Cleanup Integration', () => {
    it('should handle component cleanup properly', async () => {
      await vaultManager.switchVault('/test/vault');
      await appState.setCurrentFile('/vault/file1.md');

      // Clear all vault data
      await vaultManager.clearAllVaultData();

      expect(appState.currentVault).toBeNull();
      expect(appState.currentFile).toBeNull();
      expect(appState.files).toEqual([]);
      expect(vaultManager.currentVaultPath).toBeNull();
      expect(vaultManager.recentVaults).toEqual([]);
    });

    it('should maintain integration after reset operations', async () => {
      await vaultManager.switchVault('/test/vault');
      await appState.reset();

      // After reset, VaultManager should still be functional
      expect(vaultManager.isInitialized()).toBe(true);
      expect(appState.isValid()).toBe(true);
      
      // Should be able to switch vaults again
      await vaultManager.switchVault('/new/vault');
      expect(appState.currentVault).toBe('/new/vault');
    });
  });
});