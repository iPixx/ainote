/**
 * Vitest test setup file for aiNote
 * This file is run before all tests to set up the testing environment
 */

import { vi, beforeEach, afterEach } from 'vitest';

// Mock Tauri API
const mockTauriApi = {
  core: {
    invoke: vi.fn()
  },
  window: {
    getCurrentWindow: vi.fn(() => ({
      scaleFactor: vi.fn(() => Promise.resolve(1)),
      innerSize: vi.fn(() => Promise.resolve({ width: 1200, height: 800 })),
      outerPosition: vi.fn(() => Promise.resolve({ x: 100, y: 100 })),
      isMaximized: vi.fn(() => Promise.resolve(false)),
      maximize: vi.fn(() => Promise.resolve()),
      unmaximize: vi.fn(() => Promise.resolve()),
      setSize: vi.fn(() => Promise.resolve()),
      setPosition: vi.fn(() => Promise.resolve()),
      close: vi.fn(() => Promise.resolve()),
      listen: vi.fn(() => Promise.resolve(() => {}))
    }))
  },
  event: {
    listen: vi.fn(() => Promise.resolve(() => {})),
    emit: vi.fn(() => Promise.resolve())
  },
  fs: {
    readTextFile: vi.fn(),
    writeTextFile: vi.fn(),
    exists: vi.fn(),
    createDir: vi.fn(),
    readDir: vi.fn()
  },
  dialog: {
    open: vi.fn()
  }
};

// Set up global Tauri mock
global.window = global.window || {};
global.window.__TAURI__ = mockTauriApi;

// Mock DOM APIs that might not be available in jsdom
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // deprecated
    removeListener: vi.fn(), // deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock IntersectionObserver - this ensures it's always available in the test environment
global.IntersectionObserver = vi.fn().mockImplementation((callback, options) => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
  root: options?.root || null,
  rootMargin: options?.rootMargin || '0px',
  thresholds: options?.threshold || [0],
  // Add callback property for testing
  _callback: callback
}));

// Make sure window also has IntersectionObserver
Object.defineProperty(window, 'IntersectionObserver', {
  writable: true,
  value: global.IntersectionObserver
});

// Mock ResizeObserver
global.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}));

// Mock performance API
global.performance = global.performance || {
  now: vi.fn(() => Date.now()),
  mark: vi.fn(),
  measure: vi.fn(),
  getEntriesByName: vi.fn(() => []),
  getEntriesByType: vi.fn(() => []),
  // Add memory API mock for FileTree performance monitoring
  memory: {
    usedJSHeapSize: 1024 * 1024 * 10, // 10MB
    totalJSHeapSize: 1024 * 1024 * 50, // 50MB
    jsHeapSizeLimit: 1024 * 1024 * 100 // 100MB
  }
};

// Mock requestIdleCallback and cancelIdleCallback
global.requestIdleCallback = vi.fn((callback) => {
  return setTimeout(() => callback({ timeRemaining: () => 50 }), 0);
});
global.cancelIdleCallback = vi.fn((id) => clearTimeout(id));

// Mock scrollIntoView method for DOM elements
Element.prototype.scrollIntoView = vi.fn();

// Set up localStorage mock
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
  length: 0,
  key: vi.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
  writable: true,
});

// Set up sessionStorage mock
Object.defineProperty(window, 'sessionStorage', {
  value: localStorageMock,
  writable: true,
});

// Mock console methods to reduce noise in tests (can be overridden per test)
global.console = {
  ...console,
  log: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
  debug: vi.fn(),
};

// Clean up after each test
beforeEach(() => {
  // Reset all mocks before each test
  vi.clearAllMocks();
  
  // Reset DOM state
  document.body.innerHTML = '';
  document.head.innerHTML = '';
  
  // Reset any custom properties that might have been added
  delete window.appState;
  delete window.vaultManager;
  delete window.autoSave;
});

afterEach(() => {
  // Clean up any timers, intervals, etc.
  vi.clearAllTimers();
  
  // Clean up any event listeners
  vi.restoreAllMocks();
});

// Export mock helpers for use in tests
export { mockTauriApi };

// Export common test utilities
export const createMockElement = (tagName = 'div', attributes = {}) => {
  const element = document.createElement(tagName);
  Object.entries(attributes).forEach(([key, value]) => {
    if (key === 'textContent' || key === 'innerHTML') {
      element[key] = value;
    } else {
      element.setAttribute(key, value);
    }
  });
  return element;
};

export const simulateEvent = (element, eventType, eventInit = {}) => {
  const event = new Event(eventType, {
    bubbles: true,
    cancelable: true,
    ...eventInit
  });
  element.dispatchEvent(event);
  return event;
};

export const waitForNextTick = () => new Promise(resolve => setTimeout(resolve, 0));

export const waitFor = (condition, timeout = 1000) => {
  return new Promise((resolve, reject) => {
    const startTime = Date.now();
    const check = () => {
      if (condition()) {
        resolve();
      } else if (Date.now() - startTime > timeout) {
        reject(new Error('Timeout waiting for condition'));
      } else {
        setTimeout(check, 10);
      }
    };
    check();
  });
};