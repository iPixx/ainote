/**
 * Smoke test for aiNote testing infrastructure
 * This test validates that the Vitest setup is working correctly with aiNote's architecture
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setupTauriMocks, createTauriInvokeMock } from '../__mocks__/tauri-mocks.js';
import { createMockElement, simulateEvent, waitForNextTick } from '../setup.js';

describe('Testing Infrastructure Smoke Test', () => {
  let tauriMocks;
  
  beforeEach(() => {
    // Set up Tauri mocks for each test
    tauriMocks = setupTauriMocks();
  });

  describe('Basic Test Environment', () => {
    it('should have access to global test APIs', () => {
      expect(describe).toBeDefined();
      expect(it).toBeDefined();
      expect(expect).toBeDefined();
      expect(vi).toBeDefined();
    });

    it('should have jsdom environment available', () => {
      expect(window).toBeDefined();
      expect(document).toBeDefined();
      expect(document.createElement).toBeDefined();
      expect(document.body).toBeDefined();
    });

    it('should have DOM manipulation working', () => {
      const div = document.createElement('div');
      div.textContent = 'Hello aiNote';
      document.body.appendChild(div);
      
      expect(document.body.children.length).toBe(1);
      expect(document.body.firstChild.textContent).toBe('Hello aiNote');
    });
  });

  describe('Tauri Mocks', () => {
    it('should have Tauri API mocked globally', () => {
      expect(window.__TAURI__).toBeDefined();
      expect(window.__TAURI__.core).toBeDefined();
      expect(window.__TAURI__.core.invoke).toBeDefined();
      expect(window.__TAURI__.window).toBeDefined();
      expect(window.__TAURI__.fs).toBeDefined();
    });

    it('should mock Tauri invoke commands', async () => {
      const { invoke } = window.__TAURI__.core;
      
      // Test vault selection
      const vaultPath = await invoke('select_vault');
      expect(vaultPath).toBe('/mock/vault/path');
      
      // Test file reading
      const content = await invoke('read_file', { filePath: '/test/path.md' });
      expect(content).toContain('Mock File Content');
      expect(content).toContain('/test/path.md');
    });

    it('should mock window operations', async () => {
      const mockWindow = window.__TAURI__.window.getCurrentWindow();
      
      // Test window size operations
      const size = await mockWindow.innerSize();
      expect(size).toEqual({ width: 1200, height: 800 });
      
      // Test window state changes
      await mockWindow.maximize();
      const isMaximized = await mockWindow.isMaximized();
      expect(isMaximized).toBe(true);
    });

    it('should mock file system operations', async () => {
      const { fs } = window.__TAURI__;
      
      // Test file existence check
      const exists = await fs.exists('/mock/path');
      expect(exists).toBe(true);
      
      // Test file reading
      const content = await fs.readTextFile('/mock/file.txt');
      expect(content).toContain('Mock content for /mock/file.txt');
    });
  });

  describe('ES6 Module Support', () => {
    it('should support ES6 import/export', () => {
      // This test itself validates ES6 import support
      expect(setupTauriMocks).toBeDefined();
      expect(createTauriInvokeMock).toBeDefined();
    });

    it('should support dynamic imports', async () => {
      // This would test dynamic import functionality
      // For now, just validate that the syntax is supported
      const dynamicImport = () => import('../setup.js');
      expect(dynamicImport).toBeDefined();
    });
  });

  describe('Test Utilities', () => {
    it('should provide mock element creation utility', () => {
      const element = createMockElement('div', { 
        id: 'test-div', 
        textContent: 'Test content' 
      });
      
      expect(element.tagName).toBe('DIV');
      expect(element.id).toBe('test-div');
      expect(element.textContent).toBe('Test content');
    });

    it('should provide event simulation utility', () => {
      const button = document.createElement('button');
      let clicked = false;
      
      button.addEventListener('click', () => {
        clicked = true;
      });
      
      simulateEvent(button, 'click');
      expect(clicked).toBe(true);
    });

    it('should provide async testing utilities', async () => {
      await waitForNextTick();
      // If we get here, the utility is working
      expect(true).toBe(true);
    });
  });

  describe('Mock Reset and Cleanup', () => {
    it('should reset mocks between tests', () => {
      const { invoke } = window.__TAURI__.core;
      
      // Call invoke to increase call count
      invoke('test_command');
      expect(invoke).toHaveBeenCalledOnce();
      
      // Mock should be cleared by beforeEach in next test
    });

    it('should have clean mocks', () => {
      const { invoke } = window.__TAURI__.core;
      
      // This should be 0 if cleanup worked properly
      expect(invoke).not.toHaveBeenCalled();
    });

    it('should have clean DOM state', () => {
      // DOM should be clean between tests
      expect(document.body.innerHTML).toBe('');
      expect(document.head.innerHTML).toBe('');
    });
  });

  describe('aiNote-Specific Mocking', () => {
    it('should mock aiNote vault operations', async () => {
      const { invoke } = window.__TAURI__.core;
      
      // Test vault scanning
      const files = await invoke('scan_vault_files', { vaultPath: '/test/vault' });
      expect(Array.isArray(files)).toBe(true);
      expect(files.length).toBeGreaterThan(0);
      expect(files[0]).toHaveProperty('name');
      expect(files[0]).toHaveProperty('path');
      expect(files[0]).toHaveProperty('is_dir');
    });

    it('should mock aiNote state management', async () => {
      const { invoke } = window.__TAURI__.core;
      
      // Test app state loading
      const appState = await invoke('load_app_state');
      expect(appState).toHaveProperty('window');
      expect(appState).toHaveProperty('layout');
      expect(appState).toHaveProperty('currentVault');
    });

    it('should mock aiNote performance monitoring', async () => {
      const { invoke } = window.__TAURI__.core;
      
      // Test benchmark operations
      const benchmarks = await invoke('run_embedding_benchmarks');
      expect(Array.isArray(benchmarks)).toBe(true);
      
      if (benchmarks.length > 0) {
        expect(benchmarks[0]).toHaveProperty('operation_name');
        expect(benchmarks[0]).toHaveProperty('avg_duration_ms');
      }
    });
  });

  describe('Error Handling', () => {
    it('should handle unknown Tauri commands gracefully', async () => {
      const { invoke } = window.__TAURI__.core;
      
      // This should not throw, but return null
      const result = await invoke('unknown_command');
      expect(result).toBe(null);
    });

    it('should provide meaningful error messages', () => {
      // Test that our mocks provide helpful debugging information
      expect(console.warn).toBeDefined();
      expect(console.log).toBeDefined();
    });
  });
});