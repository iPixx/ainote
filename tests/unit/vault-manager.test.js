/**
 * Unit tests for VaultManager service
 * 
 * Tests cover:
 * - Vault selection and validation
 * - Vault switching and loading
 * - Recent vaults management
 * - State persistence and integration
 * - Error handling and recovery
 * - Performance requirements
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import dependencies
import VaultManager from '../../src/js/services/vault-manager.js';
import AppState from '../../src/js/state.js';

describe('VaultManager', () => {
  let vaultManager;
  let appState;
  let tauriMocks;

  beforeEach(() => {
    // Set up Tauri mocks
    tauriMocks = setupTauriMocks();
    
    // Create AppState instance
    appState = new AppState();
    
    // Create VaultManager instance
    vaultManager = new VaultManager(appState);
  });

  afterEach(() => {
    // Clean up mocks
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should throw error for missing AppState', () => {
      expect(() => {
        new VaultManager(null);
      }).toThrow('AppState instance is required for VaultManager');
    });

    it('should initialize with proper default values', () => {
      expect(vaultManager.appState).toBe(appState);
      expect(vaultManager.currentVaultPath).toBeNull();
      expect(vaultManager.recentVaults).toEqual([]);
      expect(vaultManager.maxRecentVaults).toBe(5);
    });

    it('should restore current vault from app state', () => {
      appState.currentVault = '/test/vault';
      const newVaultManager = new VaultManager(appState);
      
      expect(newVaultManager.currentVaultPath).toBe('/test/vault');
    });

    it('should handle vault preference loading errors gracefully', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Failed to load preferences'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      // Create new instance to trigger loadVaultPreferences
      new VaultManager(appState);
      
      // Wait for async operation to complete
      await new Promise(resolve => setTimeout(resolve, 0));

      expect(consoleSpy).toHaveBeenCalledWith(
        'Failed to load vault preferences during initialization:',
        expect.any(Error)
      );
      
      consoleSpy.mockRestore();
    });

    it('should load recent vaults during initialization', async () => {
      const mockRecentVaults = ['/vault1', '/vault2', '/vault3'];
      tauriMocks.invoke.mockResolvedValue(mockRecentVaults);

      const newVaultManager = new VaultManager(appState);
      await newVaultManager.loadVaultPreferences();

      expect(newVaultManager.recentVaults).toEqual(mockRecentVaults);
    });
  });

  describe('Vault Selection', () => {
    it('should select vault using file dialog', async () => {
      const mockPath = '/selected/vault';
      tauriMocks.invoke.mockResolvedValue(mockPath);

      const selectedPath = await vaultManager.selectVault();

      expect(selectedPath).toBe(mockPath);
      expect(tauriMocks.invoke).toHaveBeenCalledWith('select_vault');
    });

    it('should return null when vault selection is cancelled', async () => {
      tauriMocks.invoke.mockResolvedValue(null);

      const selectedPath = await vaultManager.selectVault();

      expect(selectedPath).toBeNull();
    });

    it('should handle vault selection errors', async () => {
      const errorMessage = 'Dialog failed';
      tauriMocks.invoke.mockRejectedValue(new Error(errorMessage));

      await expect(vaultManager.selectVault()).rejects.toThrow(`Vault selection failed: ${errorMessage}`);
    });

    it('should log selection process', async () => {
      const mockPath = '/test/vault';
      tauriMocks.invoke.mockResolvedValue(mockPath);
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      await vaultManager.selectVault();

      expect(consoleSpy).toHaveBeenCalledWith('🔍 VaultManager: Selected path from dialog:', mockPath);
      expect(consoleSpy).toHaveBeenCalledWith('✅ VaultManager: Path selected, skipping validation during selection');
      
      consoleSpy.mockRestore();
    });
  });

  describe('Vault Validation', () => {
    it('should validate vault path using backend command', async () => {
      const mockPath = '/valid/vault';
      tauriMocks.invoke.mockResolvedValue(true);

      const isValid = await vaultManager.validateVault(mockPath);

      expect(isValid).toBe(true);
      expect(tauriMocks.invoke).toHaveBeenCalledWith('validate_vault', { vaultPath: mockPath });
    });

    it('should return false for invalid vault path', async () => {
      tauriMocks.invoke.mockResolvedValue(false);

      const isValid = await vaultManager.validateVault('/invalid/vault');

      expect(isValid).toBe(false);
    });

    it('should return false for null or invalid path types', async () => {
      expect(await vaultManager.validateVault(null)).toBe(false);
      expect(await vaultManager.validateVault(undefined)).toBe(false);
      expect(await vaultManager.validateVault(123)).toBe(false);
      expect(await vaultManager.validateVault('')).toBe(false);
    });

    it('should handle validation errors gracefully', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Validation failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const isValid = await vaultManager.validateVault('/test/vault');

      expect(isValid).toBe(false);
      expect(consoleSpy).toHaveBeenCalledWith('❌ VaultManager: Vault validation error:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });

    it('should log validation process', async () => {
      const mockPath = '/test/vault';
      tauriMocks.invoke.mockResolvedValue(true);
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      await vaultManager.validateVault(mockPath);

      expect(consoleSpy).toHaveBeenCalledWith('🔧 VaultManager: Calling backend validate_vault with path:', mockPath);
      expect(consoleSpy).toHaveBeenCalledWith('✅ VaultManager: Backend validation result:', true);
      
      consoleSpy.mockRestore();
    });
  });

  describe('Vault Loading', () => {
    const mockFiles = [
      { name: 'file1.md', path: '/vault/file1.md', is_dir: false },
      { name: 'folder1', path: '/vault/folder1', is_dir: true }
    ];

    beforeEach(() => {
      // Mock successful validation and loading
      tauriMocks.invoke
        .mockImplementationOnce(() => Promise.resolve(true)) // validate_vault
        .mockImplementationOnce(() => Promise.resolve(mockFiles)); // load_vault
    });

    it('should load vault files after validation', async () => {
      const files = await vaultManager.loadVault('/test/vault');

      expect(files).toEqual(mockFiles);
      expect(tauriMocks.invoke).toHaveBeenCalledWith('validate_vault', { vaultPath: '/test/vault' });
      expect(tauriMocks.invoke).toHaveBeenCalledWith('load_vault', { vaultPath: '/test/vault' });
    });

    it('should update app state with loaded files', async () => {
      const setFilesSpy = vi.spyOn(appState, 'setFiles');

      await vaultManager.loadVault('/test/vault');

      expect(setFilesSpy).toHaveBeenCalledWith(mockFiles);
    });

    it('should throw error for missing vault path', async () => {
      await expect(vaultManager.loadVault('')).rejects.toThrow('Vault path is required');
      await expect(vaultManager.loadVault(null)).rejects.toThrow('Vault path is required');
    });

    it('should throw error for invalid vault', async () => {
      tauriMocks.invoke.mockResolvedValue(false); // Invalid vault

      await expect(vaultManager.loadVault('/invalid/vault')).rejects.toThrow('Invalid or inaccessible vault: /invalid/vault');
    });

    it('should handle loading errors', async () => {
      tauriMocks.invoke
        .mockImplementationOnce(() => Promise.resolve(true)) // validate_vault succeeds
        .mockImplementationOnce(() => Promise.reject(new Error('Load failed'))); // load_vault fails

      await expect(vaultManager.loadVault('/test/vault')).rejects.toThrow('Vault loading failed: Load failed');
    });

    it('should validate files response format', async () => {
      tauriMocks.invoke
        .mockImplementationOnce(() => Promise.resolve(true)) // validate_vault
        .mockImplementationOnce(() => Promise.resolve('not-an-array')); // load_vault with invalid response

      await expect(vaultManager.loadVault('/test/vault')).rejects.toThrow('Invalid vault files response format');
    });
  });

  describe('Vault Switching', () => {
    const mockFiles = [
      { name: 'test.md', path: '/new/vault/test.md', is_dir: false }
    ];

    beforeEach(() => {
      // Mock successful validation and loading
      tauriMocks.invoke
        .mockImplementation((command, params) => {
          if (command === 'validate_vault') return Promise.resolve(true);
          if (command === 'load_vault') return Promise.resolve(mockFiles);
          if (command === 'save_session_state') return Promise.resolve(true);
          if (command === 'save_vault_preferences') return Promise.resolve(true);
          return Promise.resolve(null);
        });
    });

    it('should switch to new vault successfully', async () => {
      const newVaultPath = '/new/vault';
      const setVaultSpy = vi.spyOn(appState, 'setVault');
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      await vaultManager.switchVault(newVaultPath);

      expect(vaultManager.currentVaultPath).toBe(newVaultPath);
      expect(setVaultSpy).toHaveBeenCalledWith(newVaultPath);
      expect(consoleSpy).toHaveBeenCalledWith(`✅ VaultManager: Switched vault from "null" to "${newVaultPath}". Loaded ${mockFiles.length} items.`);
      
      consoleSpy.mockRestore();
    });

    it('should add vault to recent vaults after switching', async () => {
      const newVaultPath = '/new/vault';
      const addToRecentSpy = vi.spyOn(vaultManager, 'addToRecentVaults');

      await vaultManager.switchVault(newVaultPath);

      expect(addToRecentSpy).toHaveBeenCalledWith(newVaultPath);
    });

    it('should handle switch failures gracefully', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Switch failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      await expect(vaultManager.switchVault('/failing/vault')).rejects.toThrow('Vault switching failed: Switch failed');
      expect(consoleSpy).toHaveBeenCalledWith('❌ VaultManager: Failed to switch vault:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });

    it('should proceed even if vault loading fails', async () => {
      tauriMocks.invoke
        .mockImplementation((command) => {
          if (command === 'validate_vault') return Promise.reject(new Error('Load failed'));
          if (command === 'save_session_state') return Promise.resolve(true);
          if (command === 'save_vault_preferences') return Promise.resolve(true);
          return Promise.resolve(null);
        });

      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      const newVaultPath = '/problematic/vault';

      await vaultManager.switchVault(newVaultPath);

      expect(vaultManager.currentVaultPath).toBe(newVaultPath);
      expect(consoleSpy).toHaveBeenCalledWith('⚠️ VaultManager: Failed to load vault, but proceeding anyway:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });

    it('should throw error for missing new vault path', async () => {
      await expect(vaultManager.switchVault('')).rejects.toThrow('New vault path is required');
      await expect(vaultManager.switchVault(null)).rejects.toThrow('New vault path is required');
    });
  });

  describe('Recent Vaults Management', () => {
    it('should add vault to recent vaults list', async () => {
      tauriMocks.invoke.mockResolvedValue(true);

      await vaultManager.addToRecentVaults('/vault1');
      await vaultManager.addToRecentVaults('/vault2');

      expect(vaultManager.recentVaults).toEqual(['/vault2', '/vault1']);
      expect(tauriMocks.invoke).toHaveBeenCalledWith('save_vault_preferences', { 
        recentVaults: ['/vault2', '/vault1'] 
      });
    });

    it('should avoid duplicates in recent vaults', async () => {
      tauriMocks.invoke.mockResolvedValue(true);

      await vaultManager.addToRecentVaults('/vault1');
      await vaultManager.addToRecentVaults('/vault2');
      await vaultManager.addToRecentVaults('/vault1'); // Duplicate

      expect(vaultManager.recentVaults).toEqual(['/vault1', '/vault2']);
    });

    it('should limit recent vaults to maximum count', async () => {
      tauriMocks.invoke.mockResolvedValue(true);

      const vaults = Array.from({ length: 7 }, (_, i) => `/vault${i}`);
      for (const vault of vaults) {
        await vaultManager.addToRecentVaults(vault);
      }

      expect(vaultManager.recentVaults).toHaveLength(5);
      expect(vaultManager.recentVaults[0]).toBe('/vault6'); // Most recent first
    });

    it('should ignore invalid vault paths', async () => {
      const initialLength = vaultManager.recentVaults.length;

      await vaultManager.addToRecentVaults(null);
      await vaultManager.addToRecentVaults(undefined);
      await vaultManager.addToRecentVaults(123);
      await vaultManager.addToRecentVaults('');

      expect(vaultManager.recentVaults).toHaveLength(initialLength);
    });

    it('should return copy of recent vaults', () => {
      vaultManager.recentVaults = ['/vault1', '/vault2'];

      const recent = vaultManager.getRecentVaults();
      recent.push('/vault3');

      expect(vaultManager.recentVaults).toEqual(['/vault1', '/vault2']);
    });

    it('should handle save recent vaults errors', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Save failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      await vaultManager.addToRecentVaults('/test/vault');

      expect(consoleSpy).toHaveBeenCalledWith('Failed to save recent vaults to app_state.json:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });
  });

  describe('Initial Setup', () => {
    beforeEach(() => {
      // Mock vault preferences loading to return empty
      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'get_vault_preferences') return Promise.resolve([]);
        if (command === 'validate_vault') return Promise.resolve(true);
        if (command === 'load_vault') return Promise.resolve([]);
        if (command === 'save_session_state') return Promise.resolve(true);
        if (command === 'save_vault_preferences') return Promise.resolve(true);
        if (command === 'select_vault') return Promise.resolve('/selected/vault');
        return Promise.resolve(null);
      });
    });

    it('should setup initial vault when none exists', async () => {
      const switchVaultSpy = vi.spyOn(vaultManager, 'switchVault');

      const selectedPath = await vaultManager.setupInitialVault();

      expect(selectedPath).toBe('/selected/vault');
      expect(switchVaultSpy).toHaveBeenCalledWith('/selected/vault');
    });

    it('should use existing valid vault', async () => {
      appState.currentVault = '/existing/vault';
      vaultManager = new VaultManager(appState); // Recreate with existing vault
      
      const switchVaultSpy = vi.spyOn(vaultManager, 'switchVault');

      const selectedPath = await vaultManager.setupInitialVault();

      expect(selectedPath).toBe('/existing/vault');
      expect(switchVaultSpy).toHaveBeenCalledWith('/existing/vault');
    });

    it('should handle setup cancellation', async () => {
      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'select_vault') return Promise.resolve(null);
        return Promise.resolve([]);
      });

      const selectedPath = await vaultManager.setupInitialVault();

      expect(selectedPath).toBeNull();
    });

    it('should clear invalid saved vault', async () => {
      appState.currentVault = '/invalid/vault';
      vaultManager = new VaultManager(appState);

      tauriMocks.invoke.mockImplementation((command) => {
        if (command === 'validate_vault') return Promise.resolve(false);
        if (command === 'select_vault') return Promise.resolve('/new/vault');
        if (command === 'load_vault') return Promise.resolve([]);
        return Promise.resolve(true);
      });

      const clearVaultSpy = vi.spyOn(vaultManager, 'clearVaultPreference');

      const selectedPath = await vaultManager.setupInitialVault();

      expect(clearVaultSpy).toHaveBeenCalled();
      expect(selectedPath).toBe('/new/vault');
    });
  });

  describe('Vault Statistics', () => {
    it('should calculate vault statistics correctly', async () => {
      vaultManager.currentVaultPath = '/test/vault';
      appState.files = [
        { name: 'file1.md', path: '/vault/file1.md', is_dir: false, size: 1024 },
        { name: 'file2.md', path: '/vault/file2.md', is_dir: false, size: 2048 },
        { name: 'folder1', path: '/vault/folder1', is_dir: true },
        { name: 'file3.txt', path: '/vault/file3.txt', is_dir: false, size: 512 }
      ];

      const stats = await vaultManager.getVaultStats();

      expect(stats.vaultPath).toBe('/test/vault');
      expect(stats.fileCount).toBe(2); // Only .md files
      expect(stats.directoryCount).toBe(1);
      expect(stats.totalSize).toBe(3072); // 1024 + 2048
      expect(stats.lastScanned).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
    });

    it('should return empty stats when no vault loaded', async () => {
      const stats = await vaultManager.getVaultStats();

      expect(stats).toEqual({
        vaultPath: null,
        fileCount: 0,
        directoryCount: 0,
        totalSize: 0,
        lastScanned: null
      });
    });

    it('should handle missing file sizes gracefully', async () => {
      vaultManager.currentVaultPath = '/test/vault';
      appState.files = [
        { name: 'file1.md', path: '/vault/file1.md', is_dir: false }, // No size property
        { name: 'file2.md', path: '/vault/file2.md', is_dir: false, size: 1024 }
      ];

      const stats = await vaultManager.getVaultStats();

      expect(stats.fileCount).toBe(2);
      expect(stats.totalSize).toBe(1024); // Should handle missing size as 0
    });
  });

  describe('Vault Refresh', () => {
    beforeEach(() => {
      vaultManager.currentVaultPath = '/test/vault';
      tauriMocks.invoke
        .mockImplementation((command) => {
          if (command === 'validate_vault') return Promise.resolve(true);
          if (command === 'load_vault') return Promise.resolve([
            { name: 'refreshed.md', path: '/test/vault/refreshed.md', is_dir: false }
          ]);
          return Promise.resolve(null);
        });
    });

    it('should refresh current vault', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const files = await vaultManager.refreshVault();

      expect(files).toHaveLength(1);
      expect(files[0].name).toBe('refreshed.md');
      expect(consoleSpy).toHaveBeenCalledWith('Refreshing vault: /test/vault');
      expect(consoleSpy).toHaveBeenCalledWith('Vault refreshed. Found 1 items.');
      
      consoleSpy.mockRestore();
    });

    it('should throw error when no vault loaded', async () => {
      vaultManager.currentVaultPath = null;

      await expect(vaultManager.refreshVault()).rejects.toThrow('No vault currently loaded');
    });

    it('should handle refresh errors', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Refresh failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      await expect(vaultManager.refreshVault()).rejects.toThrow('Vault refresh failed: Refresh failed');
      expect(consoleSpy).toHaveBeenCalledWith('Failed to refresh vault:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });
  });

  describe('State Management', () => {
    it('should get current vault path', () => {
      vaultManager.currentVaultPath = '/test/vault';
      expect(vaultManager.getCurrentVault()).toBe('/test/vault');
    });

    it('should load vault preference from AppState', () => {
      appState.currentVault = '/app/state/vault';
      expect(vaultManager.loadVaultPreference()).toBe('/app/state/vault');
    });

    it('should clear vault preference via AppState', () => {
      const setVaultSpy = vi.spyOn(appState, 'setVault');
      
      vaultManager.clearVaultPreference();
      
      expect(setVaultSpy).toHaveBeenCalledWith(null);
    });

    it('should clear all vault data', async () => {
      // Set up initial state
      vaultManager.currentVaultPath = '/test/vault';
      vaultManager.recentVaults = ['/vault1', '/vault2'];
      appState.files = [{ name: 'test.md', path: '/test.md', is_dir: false }];

      tauriMocks.invoke.mockResolvedValue(true);
      const setVaultSpy = vi.spyOn(appState, 'setVault');
      const setFilesSpy = vi.spyOn(appState, 'setFiles');

      await vaultManager.clearAllVaultData();

      expect(vaultManager.currentVaultPath).toBeNull();
      expect(vaultManager.recentVaults).toEqual([]);
      expect(setVaultSpy).toHaveBeenCalledWith(null);
      expect(setFilesSpy).toHaveBeenCalledWith([]);
    });
  });

  describe('Status and Utilities', () => {
    it('should report initialization status', () => {
      expect(vaultManager.isInitialized()).toBe(true);
      
      // Test with invalid AppState
      vaultManager.appState = null;
      expect(vaultManager.isInitialized()).toBe(false);
    });

    it('should return detailed status', () => {
      vaultManager.currentVaultPath = '/test/vault';
      vaultManager.recentVaults = ['/vault1', '/vault2'];
      appState.currentVault = '/app/vault';
      appState.files = [{ name: 'test.md' }];

      const status = vaultManager.getStatus();

      expect(status).toEqual({
        initialized: true,
        currentVault: '/test/vault',
        recentVaultsCount: 2,
        appStateVault: '/app/vault',
        filesLoaded: 1
      });
    });

    it('should handle status with missing AppState', () => {
      vaultManager.appState = null;

      const status = vaultManager.getStatus();

      expect(status).toEqual({
        initialized: false,
        currentVault: null,
        recentVaultsCount: 0,
        appStateVault: null,
        filesLoaded: 0
      });
    });
  });

  describe('Error Handling', () => {
    it('should handle vault preference loading errors', async () => {
      tauriMocks.invoke.mockRejectedValue(new Error('Preferences failed'));
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      await vaultManager.loadVaultPreferences();

      expect(vaultManager.recentVaults).toEqual([]);
      expect(consoleSpy).toHaveBeenCalledWith('Failed to load recent vaults from app_state.json:', expect.any(Error));
      
      consoleSpy.mockRestore();
    });

    it('should handle invalid recent vaults data', async () => {
      tauriMocks.invoke.mockResolvedValue('not-an-array');

      await vaultManager.loadVaultPreferences();

      expect(vaultManager.recentVaults).toEqual([]);
    });

    it('should limit loaded recent vaults to maximum', async () => {
      const tooManyVaults = Array.from({ length: 10 }, (_, i) => `/vault${i}`);
      tauriMocks.invoke.mockResolvedValue(tooManyVaults);

      await vaultManager.loadVaultPreferences();

      expect(vaultManager.recentVaults).toHaveLength(5);
    });
  });

  describe('Performance', () => {
    it('should complete operations within performance targets', async () => {
      tauriMocks.invoke.mockResolvedValue(true);

      // Test vault validation performance
      const validateStart = performance.now();
      await vaultManager.validateVault('/test/vault');
      const validateTime = performance.now() - validateStart;
      expect(validateTime).toBeLessThan(100); // Should be fast with mocked backend

      // Test recent vault operations
      const recentStart = performance.now();
      await vaultManager.addToRecentVaults('/test/vault');
      const recentTime = performance.now() - recentStart;
      expect(recentTime).toBeLessThan(50);
    });

    it('should handle large recent vault lists efficiently', async () => {
      const largeVaultList = Array.from({ length: 100 }, (_, i) => `/vault${i}`);
      tauriMocks.invoke.mockResolvedValue(largeVaultList);

      const loadStart = performance.now();
      await vaultManager.loadVaultPreferences();
      const loadTime = performance.now() - loadStart;

      expect(loadTime).toBeLessThan(10);
      expect(vaultManager.recentVaults).toHaveLength(5); // Should be trimmed to max
    });
  });
});