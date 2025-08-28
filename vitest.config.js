import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Test environment settings
    environment: 'jsdom', // Use jsdom for DOM testing
    globals: true, // Enable global test APIs (describe, it, expect, etc.)
    
    // File patterns
    include: [
      'tests/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'
    ],
    
    // Test setup
    setupFiles: ['./tests/setup.js'],
    
    // Mock patterns
    mockReset: true,
    clearMocks: true,
    
    // Coverage configuration
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'json'],
      include: [
        'src/**/*.{js,mjs,cjs}'
      ],
      exclude: [
        'src/**/*.{test,spec}.{js,mjs,cjs}',
        'src/assets/**',
        'src/**/*.html',
        'src/**/*.css',
        'tests/**'
      ],
      thresholds: {
        global: {
          branches: 70,
          functions: 70,
          lines: 70,
          statements: 70
        }
      }
    },
    
    // Timeout settings
    testTimeout: 10000, // 10 seconds per test
    hookTimeout: 10000, // 10 seconds for setup/teardown
    
    // Reporter settings
    reporter: process.env.CI ? 'json' : 'verbose',
    
    // Watch mode settings
    watch: {
      ignore: [
        '**/node_modules/**',
        '**/dist/**',
        '**/target/**',
        '**/src-tauri/target/**',
        '**/.git/**'
      ]
    }
  },
  
  // Resolve configuration for ES modules
  resolve: {
    alias: {
      '@': '/src',
      '@components': '/src/js/components',
      '@services': '/src/js/services',
      '@utils': '/src/js/utils'
    }
  },
  
  // Define globals for compatibility
  define: {
    'process.env.NODE_ENV': '"test"'
  }
});