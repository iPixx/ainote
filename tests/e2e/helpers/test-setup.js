/**
 * Global Test Setup for aiNote E2E Tests
 * 
 * Handles global configuration, cleanup, and test environment setup.
 */

import { createRequire } from 'module';
const require = createRequire(import.meta.url);

// Global test configuration
global.TEST_CONFIG = {
  timeout: 30000,
  appStartTimeout: 10000,
  pageLoadTimeout: 15000,
  elementTimeout: 5000,
  headless: process.env.HEADLESS === 'true',
  debug: process.env.DEBUG === 'true'
};

/**
 * Global setup - runs once before all tests
 */
before(async function() {
  this.timeout(60000); // Extended timeout for setup
  
  console.log('🚀 Starting aiNote E2E test setup...');
  console.log(`📋 Configuration:`);
  console.log(`   - Headless: ${global.TEST_CONFIG.headless}`);
  console.log(`   - Debug: ${global.TEST_CONFIG.debug}`);
  console.log(`   - Platform: ${process.platform}`);
  console.log(`   - Environment: ${process.env.NODE_ENV || 'test'}`);
  
  // Verify required binaries and paths
  await verifyTestEnvironment();
  
  console.log('✅ E2E test setup complete');
});

/**
 * Global cleanup - runs once after all tests
 */
after(async function() {
  this.timeout(30000);
  
  console.log('🧹 Running E2E test cleanup...');
  
  // Cleanup any remaining processes or files
  await cleanupTestEnvironment();
  
  console.log('✅ E2E test cleanup complete');
});

/**
 * Per-test setup
 */
beforeEach(function() {
  // Set test timeout based on test type
  const testName = this.currentTest.title;
  
  if (testName.includes('startup') || testName.includes('build')) {
    this.timeout(60000); // Extended timeout for slow operations
  } else {
    this.timeout(global.TEST_CONFIG.timeout);
  }
  
  if (global.TEST_CONFIG.debug) {
    console.log(`\n🧪 Starting test: ${testName}`);
  }
});

/**
 * Per-test cleanup
 */
afterEach(function() {
  const testName = this.currentTest.title;
  const testState = this.currentTest.state;
  
  if (global.TEST_CONFIG.debug) {
    console.log(`📊 Test completed: ${testName} (${testState})`);
  }
});

/**
 * Verify test environment is properly configured
 */
async function verifyTestEnvironment() {
  const fs = require('fs');
  const path = require('path');
  
  console.log('🔍 Verifying test environment...');
  
  // Check for required directories
  const requiredDirs = [
    './src',
    './src-tauri',
    './tests/e2e/fixtures'
  ];
  
  for (const dir of requiredDirs) {
    if (!fs.existsSync(dir)) {
      throw new Error(`Required directory not found: ${dir}`);
    }
  }
  
  // Check for Tauri build
  const tauriConfigPath = './src-tauri/tauri.conf.json';
  if (!fs.existsSync(tauriConfigPath)) {
    throw new Error('Tauri configuration not found. Is this a Tauri project?');
  }
  
  // Check for test fixtures
  const fixturesDir = './tests/e2e/fixtures';
  if (!fs.existsSync(fixturesDir)) {
    console.log('📁 Creating test fixtures directory...');
    fs.mkdirSync(fixturesDir, { recursive: true });
  }
  
  console.log('✅ Test environment verification complete');
}

/**
 * Cleanup test environment
 */
async function cleanupTestEnvironment() {
  // Kill any remaining test processes
  // This will be implemented in driver-manager.js
  
  // Cleanup temporary test files
  const fs = require('fs');
  const path = require('path');
  
  const tempFiles = [
    './test-results.json',
    './tests/e2e/fixtures/temp_*'
  ];
  
  for (const pattern of tempFiles) {
    try {
      if (pattern.includes('*')) {
        // Handle glob patterns if needed
        continue;
      }
      
      if (fs.existsSync(pattern)) {
        fs.unlinkSync(pattern);
        if (global.TEST_CONFIG.debug) {
          console.log(`🗑️  Cleaned up: ${pattern}`);
        }
      }
    } catch (error) {
      console.warn(`⚠️  Failed to cleanup ${pattern}:`, error.message);
    }
  }
}

// Handle uncaught exceptions in tests
process.on('uncaughtException', (error) => {
  console.error('❌ Uncaught exception in E2E test:', error);
  process.exit(1);
});

process.on('unhandledRejection', (reason, promise) => {
  console.error('❌ Unhandled rejection in E2E test:', reason);
  process.exit(1);
});