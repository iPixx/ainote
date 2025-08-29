/**
 * Mocha Configuration for aiNote E2E Tests
 * 
 * Optimized for Tauri application testing with performance requirements.
 */

module.exports = {
  // Test execution settings
  timeout: 30000,        // 30 seconds per test (generous for E2E)
  slow: 5000,           // Mark tests as slow if they take >5s
  
  // Output and reporting
  reporter: process.env.CI ? 'json' : 'spec',
  reporterOptions: {
    output: process.env.CI ? 'test-results.json' : undefined
  },
  
  // Test discovery
  recursive: true,       // Find tests in subdirectories
  extension: ['js'],     // Test file extensions
  spec: ['tests/e2e/specs/**/*.e2e.js'], // Test file patterns
  
  // Execution control
  exit: true,           // Force exit after tests complete
  bail: false,          // Continue running tests after failures
  parallel: false,      // Run tests sequentially for E2E stability
  
  // Environment
  require: [
    'tests/e2e/helpers/test-setup.js'  // Global test setup
  ],
  
  // Grep patterns (can be overridden by CLI)
  grep: process.env.TEST_GREP || undefined,
  
  // Retries (for flaky E2E tests)
  retries: process.env.CI ? 2 : 0,
  
  // Global settings
  globals: {
    // Test configuration available in all tests
    TEST_CONFIG: {
      timeout: 30000,
      appStartTimeout: 10000,
      pageLoadTimeout: 15000,
      elementTimeout: 5000,
      headless: process.env.HEADLESS === 'true',
      debug: process.env.DEBUG === 'true'
    }
  }
};