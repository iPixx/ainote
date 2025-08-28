/**
 * Advanced Tauri API mocks for aiNote testing
 * Provides more detailed mocking for specific Tauri commands used in aiNote
 */

import { vi } from 'vitest';

// Mock vault and file system operations
export const createVaultManagerMocks = () => {
  return {
    // Vault operations
    select_vault: vi.fn(() => Promise.resolve('/mock/vault/path')),
    scan_vault_files: vi.fn(() => Promise.resolve([
      { name: 'test.md', path: '/mock/vault/path/test.md', is_dir: false },
      { name: 'folder', path: '/mock/vault/path/folder', is_dir: true },
      { name: 'nested.md', path: '/mock/vault/path/folder/nested.md', is_dir: false }
    ])),
    
    // File operations
    read_file: vi.fn((args) => {
      const mockContent = `# Mock File Content\n\nThis is mock content for ${args.filePath}`;
      return Promise.resolve(mockContent);
    }),
    write_file: vi.fn(() => Promise.resolve()),
    create_file: vi.fn(() => Promise.resolve()),
    rename_file: vi.fn(() => Promise.resolve()),
    delete_file: vi.fn(() => Promise.resolve()),
    get_file_info: vi.fn(() => Promise.resolve({
      size: 1024,
      modified: Date.now(),
      created: Date.now(),
      is_dir: false
    })),
    
    // State management
    load_app_state: vi.fn(() => Promise.resolve({
      window: {
        width: 1200,
        height: 800,
        x: 100,
        y: 100,
        maximized: false
      },
      layout: {
        fileTreeWidth: 250,
        aiPanelWidth: 300,
        fileTreeVisible: true,
        aiPanelVisible: false,
        editorMode: 'editor'
      },
      currentVault: '/mock/vault/path',
      currentFile: '/mock/vault/path/test.md'
    })),
    save_window_state: vi.fn(() => Promise.resolve()),
    save_layout_state: vi.fn(() => Promise.resolve()),
    
    // AI/Ollama operations (for future testing)
    check_ollama_status: vi.fn(() => Promise.resolve({
      available: true,
      models: ['llama2', 'codellama']
    })),
    generate_embeddings: vi.fn(() => Promise.resolve([0.1, 0.2, 0.3])),
    
    // Performance monitoring (for testing monitoring features)
    run_embedding_benchmarks: vi.fn(() => Promise.resolve([
      {
        operation_name: 'embedding_generation',
        avg_duration_ms: 150.5,
        min_duration_ms: 120.0,
        max_duration_ms: 200.0,
        iterations: 10
      }
    ])),
    establish_performance_baseline: vi.fn(() => Promise.resolve('Baseline established successfully')),
    analyze_performance_regressions: vi.fn(() => Promise.resolve({
      total_regressions_detected: 0,
      overall_health: 'Good',
      recommendations: []
    })),
    generate_benchmark_report: vi.fn(() => Promise.resolve('Mock benchmark report\nAll systems performing well'))
  };
};

// Create a comprehensive mock for the Tauri invoke function
export const createTauriInvokeMock = () => {
  const mockCommands = createVaultManagerMocks();
  
  return vi.fn((command, args = {}) => {
    console.log(`[MOCK] Tauri command called: ${command}`, args);
    
    if (mockCommands[command]) {
      return mockCommands[command](args);
    }
    
    // Fallback for unhandled commands
    console.warn(`[MOCK] Unhandled Tauri command: ${command}`);
    return Promise.resolve(null);
  });
};

// Mock window manager for testing window operations
export const createWindowMock = () => {
  let windowState = {
    size: { width: 1200, height: 800 },
    position: { x: 100, y: 100 },
    maximized: false,
    scaleFactor: 1
  };
  
  return {
    scaleFactor: vi.fn(() => Promise.resolve(windowState.scaleFactor)),
    innerSize: vi.fn(() => Promise.resolve(windowState.size)),
    outerPosition: vi.fn(() => Promise.resolve(windowState.position)),
    isMaximized: vi.fn(() => Promise.resolve(windowState.maximized)),
    
    setSize: vi.fn((size) => {
      windowState.size = size.type === 'Logical' ? size : { width: size.width, height: size.height };
      return Promise.resolve();
    }),
    setPosition: vi.fn((pos) => {
      windowState.position = pos.type === 'Logical' ? pos : { x: pos.x, y: pos.y };
      return Promise.resolve();
    }),
    maximize: vi.fn(() => {
      windowState.maximized = true;
      return Promise.resolve();
    }),
    unmaximize: vi.fn(() => {
      windowState.maximized = false;
      return Promise.resolve();
    }),
    close: vi.fn(() => Promise.resolve()),
    
    listen: vi.fn((event, handler) => {
      // Return an unlisten function
      return Promise.resolve(() => {});
    }),
    
    // Helper to manually trigger window events in tests
    _triggerEvent: (eventType, payload = {}) => {
      // This would be used in tests to simulate window events
      console.log(`[MOCK] Window event triggered: ${eventType}`, payload);
    },
    
    // Helper to get current mock state
    _getState: () => ({ ...windowState })
  };
};

// Mock file system operations for more specific testing
export const createFileSystemMocks = () => {
  const mockFiles = new Map();
  
  return {
    readTextFile: vi.fn((path) => {
      if (mockFiles.has(path)) {
        return Promise.resolve(mockFiles.get(path));
      }
      return Promise.resolve(`Mock content for ${path}`);
    }),
    
    writeTextFile: vi.fn((path, content) => {
      mockFiles.set(path, content);
      return Promise.resolve();
    }),
    
    exists: vi.fn((path) => {
      return Promise.resolve(mockFiles.has(path) || path.includes('mock'));
    }),
    
    createDir: vi.fn(() => Promise.resolve()),
    
    readDir: vi.fn((path) => {
      // Return mock directory structure
      return Promise.resolve([
        { name: 'file1.md', path: `${path}/file1.md` },
        { name: 'folder1', path: `${path}/folder1` }
      ]);
    }),
    
    // Helpers for testing
    _addMockFile: (path, content) => mockFiles.set(path, content),
    _removeMockFile: (path) => mockFiles.delete(path),
    _clearMockFiles: () => mockFiles.clear(),
    _getMockFiles: () => new Map(mockFiles)
  };
};

// Export a complete Tauri mock setup function
export const setupTauriMocks = () => {
  const invokeMock = createTauriInvokeMock();
  const windowMock = createWindowMock();
  const fsMocks = createFileSystemMocks();
  
  const tauriMock = {
    core: {
      invoke: invokeMock
    },
    window: {
      getCurrentWindow: vi.fn(() => windowMock)
    },
    fs: fsMocks,
    dialog: {
      open: vi.fn(() => Promise.resolve('/mock/selected/path'))
    },
    event: {
      listen: vi.fn(() => Promise.resolve(() => {})),
      emit: vi.fn(() => Promise.resolve())
    }
  };
  
  // Set global mock
  global.window = global.window || {};
  global.window.__TAURI__ = tauriMock;
  
  return {
    tauri: tauriMock,
    invoke: invokeMock,
    window: windowMock,
    fs: fsMocks
  };
};