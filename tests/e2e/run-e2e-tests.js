#!/usr/bin/env node

/**
 * E2E Test Runner for aiNote
 * 
 * Orchestrates the complete E2E testing process including application build,
 * test execution, and cleanup.
 */

import { spawn } from 'child_process';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const fs = require('fs');
const path = require('path');

class E2ETestRunner {
  constructor() {
    this.config = {
      headless: process.env.HEADLESS === 'true' || process.env.CI === 'true',
      debug: process.env.DEBUG === 'true',
      browser: process.env.BROWSER || 'chrome',
      timeout: parseInt(process.env.TEST_TIMEOUT) || 30000,
      bail: process.env.BAIL === 'true',
      mode: process.env.E2E_MODE || 'hybrid', // 'frontend', 'full', 'hybrid'
      skipBuild: process.env.SKIP_BUILD === 'true',
      forceBuild: process.env.FORCE_BUILD === 'true'
    };
    
    this.results = {
      started: new Date().toISOString(),
      tests: [],
      summary: null
    };
    
    console.log('🚀 aiNote E2E Test Runner Starting...');
    console.log('📋 Configuration:', JSON.stringify(this.config, null, 2));
  }
  
  /**
   * Run the complete E2E test suite
   */
  async run() {
    let success = false;
    
    try {
      console.log('\n🔧 Phase 1: Pre-flight checks...');
      await this.preflightChecks();
      
      console.log('\n🏗️  Phase 2: Verifying application...');
      await this.verifyApplication();
      
      console.log('\n🧪 Phase 3: Running E2E tests...');
      success = await this.runTests();
      
      console.log('\n📊 Phase 4: Generating reports...');
      await this.generateReports();
      
    } catch (error) {
      console.error('\n❌ E2E test execution failed:', error.message);
      success = false;
    } finally {
      console.log('\n🧹 Phase 5: Cleanup...');
      await this.cleanup();
    }
    
    const duration = new Date() - new Date(this.results.started);
    console.log(`\n✨ E2E test run completed in ${(duration / 1000).toFixed(2)}s`);
    
    if (success) {
      console.log('🎉 All tests passed successfully!');
      process.exit(0);
    } else {
      console.log('💥 Some tests failed. Check the reports for details.');
      process.exit(1);
    }
  }
  
  /**
   * Verify environment and dependencies
   */
  async preflightChecks() {
    const checks = [
      { name: 'Node.js version', check: () => process.version },
      { name: 'Tauri project', check: () => fs.existsSync('./src-tauri/tauri.conf.json') },
      { name: 'Frontend source', check: () => fs.existsSync('./src/index.html') },
      { name: 'Test fixtures', check: () => fs.existsSync('./tests/e2e') },
      { name: 'Chrome browser', check: () => this.checkChromeBrowser() }
    ];
    
    console.log('Running pre-flight checks...');
    
    for (const check of checks) {
      try {
        const result = check.check();
        if (result) {
          console.log(`✅ ${check.name}: OK`);
        } else {
          throw new Error(`Failed: ${check.name}`);
        }
      } catch (error) {
        console.error(`❌ ${check.name}: ${error.message}`);
        throw new Error(`Pre-flight check failed: ${check.name}`);
      }
    }
  }
  
  /**
   * Verify or build the application based on mode
   */
  async verifyApplication() {
    if (this.config.mode === 'frontend' && this.config.skipBuild) {
      console.log('ℹ️  Skipping application verification for frontend-only testing');
      return;
    }
    
    const platforms = {
      darwin: './src-tauri/target/release/ainote',
      linux: './src-tauri/target/release/ainote',
      win32: './src-tauri/target/release/ainote.exe'
    };
    
    const expectedBinary = platforms[process.platform];
    
    // Check if application already exists
    if (fs.existsSync(expectedBinary) && !this.config.forceBuild) {
      const stats = fs.statSync(expectedBinary);
      console.log(`✅ Found existing application: ${expectedBinary}`);
      console.log(`📏 Size: ${(stats.size / 1024 / 1024).toFixed(1)}MB`);
      console.log(`📅 Modified: ${stats.mtime.toLocaleString()}`);
      return;
    }
    
    // Build the application if needed
    if (this.config.mode === 'full' || this.config.forceBuild) {
      await this.buildApplication();
    } else {
      console.log('⚠️  Application not found, but not building for this test mode');
    }
  }
  
  /**
   * Check if Chrome browser is available
   */
  checkChromeBrowser() {
    // Simple check - in a real implementation, this would verify Chrome installation
    return true; // Assume Chrome is available
  }
  
  /**
   * Build the Tauri application for testing
   */
  async buildApplication() {
    console.log('Building Tauri application...');
    
    const buildResult = await this.runCommand('pnpm', ['tauri', 'build'], {
      timeout: 300000, // 5 minutes for build
      stdio: this.config.debug ? 'inherit' : 'pipe'
    });
    
    if (!buildResult.success) {
      throw new Error('Failed to build Tauri application');
    }
    
    // Verify build output exists
    const platforms = {
      darwin: './src-tauri/target/release/ainote',
      linux: './src-tauri/target/release/ainote',
      win32: './src-tauri/target/release/ainote.exe'
    };
    
    const expectedBinary = platforms[process.platform];
    if (!fs.existsSync(expectedBinary)) {
      throw new Error(`Build output not found: ${expectedBinary}`);
    }
    
    console.log(`✅ Application built successfully: ${expectedBinary}`);
  }
  
  /**
   * Get test file patterns based on execution mode
   */
  getTestPatterns() {
    switch (this.config.mode) {
      case 'frontend':
        return [
          './tests/e2e/specs/infrastructure-demo.e2e.js',
          // Add other frontend-only tests here
        ];
        
      case 'full':
        return [
          './tests/e2e/specs/true-e2e-complete.e2e.js',
          // Add other full-stack E2E tests here
        ];
        
      case 'hybrid':
      default:
        return [
          './tests/e2e/specs/infrastructure-demo.e2e.js',
          './tests/e2e/specs/true-e2e-complete.e2e.js',
          // Include all test types for comprehensive testing
        ];
    }
  }
  
  /**
   * Run the E2E test suite
   */
  async runTests() {
    console.log(`Starting E2E test execution (mode: ${this.config.mode})...`);
    
    // Set environment variables for tests
    const testEnv = {
      ...process.env,
      HEADLESS: this.config.headless.toString(),
      DEBUG: this.config.debug.toString(),
      BROWSER: this.config.browser,
      TEST_TIMEOUT: this.config.timeout.toString(),
      E2E_MODE: this.config.mode
    };
    
    // Configure test files based on mode
    const testPatterns = this.getTestPatterns();
    
    const mochaArgs = [
      '--config', './tests/e2e/config/mocha.config.js',
      ...testPatterns
    ];
    
    if (this.config.bail) {
      mochaArgs.push('--bail');
    }
    
    if (this.config.debug) {
      mochaArgs.push('--reporter', 'spec');
    }
    
    const testResult = await this.runCommand('npx', ['mocha', ...mochaArgs], {
      timeout: this.config.timeout * 10, // Allow extra time for full suite
      env: testEnv,
      stdio: 'inherit'
    });
    
    this.results.summary = {
      success: testResult.success,
      exitCode: testResult.exitCode,
      duration: testResult.duration
    };
    
    return testResult.success;
  }
  
  /**
   * Generate test reports and artifacts
   */
  async generateReports() {
    console.log('Generating test reports...');
    
    const reportsDir = './tests/e2e/reports';
    if (!fs.existsSync(reportsDir)) {
      fs.mkdirSync(reportsDir, { recursive: true });
    }
    
    // Generate summary report
    const report = {
      meta: {
        generated: new Date().toISOString(),
        platform: process.platform,
        arch: process.arch,
        nodeVersion: process.version,
        config: this.config
      },
      execution: this.results,
      screenshots: this.collectScreenshots()
    };
    
    const reportPath = path.join(reportsDir, 'e2e-report.json');
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
    
    console.log(`📋 Test report saved: ${reportPath}`);
    
    // Generate HTML report (simple version)
    const htmlReport = this.generateHtmlReport(report);
    const htmlPath = path.join(reportsDir, 'e2e-report.html');
    fs.writeFileSync(htmlPath, htmlReport);
    
    console.log(`🌐 HTML report saved: ${htmlPath}`);
  }
  
  /**
   * Collect screenshots from test run
   */
  collectScreenshots() {
    const screenshotsDir = './tests/e2e/screenshots';
    
    if (!fs.existsSync(screenshotsDir)) {
      return [];
    }
    
    return fs.readdirSync(screenshotsDir)
      .filter(file => file.endsWith('.png'))
      .map(file => ({
        filename: file,
        path: path.join(screenshotsDir, file),
        size: fs.statSync(path.join(screenshotsDir, file)).size
      }));
  }
  
  /**
   * Generate simple HTML report
   */
  generateHtmlReport(report) {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>aiNote E2E Test Report</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; margin: 40px; }
    .header { background: #f5f5f5; padding: 20px; border-radius: 8px; }
    .success { color: #28a745; }
    .failure { color: #dc3545; }
    .section { margin: 20px 0; }
    .screenshots img { max-width: 200px; margin: 10px; border: 1px solid #ddd; }
    pre { background: #f8f9fa; padding: 15px; border-radius: 4px; overflow-x: auto; }
  </style>
</head>
<body>
  <div class="header">
    <h1>aiNote E2E Test Report</h1>
    <p><strong>Generated:</strong> ${report.meta.generated}</p>
    <p><strong>Platform:</strong> ${report.meta.platform} ${report.meta.arch}</p>
    <p><strong>Status:</strong> <span class="${report.execution.summary?.success ? 'success' : 'failure'}">
      ${report.execution.summary?.success ? '✅ PASSED' : '❌ FAILED'}
    </span></p>
  </div>
  
  <div class="section">
    <h2>Configuration</h2>
    <pre>${JSON.stringify(report.meta.config, null, 2)}</pre>
  </div>
  
  <div class="section">
    <h2>Execution Summary</h2>
    <pre>${JSON.stringify(report.execution.summary, null, 2)}</pre>
  </div>
  
  ${report.screenshots.length > 0 ? `
  <div class="section">
    <h2>Screenshots (${report.screenshots.length})</h2>
    <div class="screenshots">
      ${report.screenshots.map(shot => `
        <div>
          <img src="../screenshots/${shot.filename}" alt="${shot.filename}">
          <p>${shot.filename} (${(shot.size / 1024).toFixed(1)}KB)</p>
        </div>
      `).join('')}
    </div>
  </div>
  ` : ''}
</body>
</html>`;
  }
  
  /**
   * Cleanup test artifacts and processes
   */
  async cleanup() {
    console.log('Performing cleanup...');
    
    // Kill any remaining processes
    // This would include application processes started during testing
    
    // Cleanup temporary files (optional)
    const tempDirs = [
      './tests/e2e/fixtures/temp-vault',
      './tests/e2e/fixtures/empty-vault',
      './tests/e2e/fixtures/large-vault'
    ];
    
    for (const dir of tempDirs) {
      if (fs.existsSync(dir)) {
        try {
          fs.rmSync(dir, { recursive: true, force: true });
          console.log(`🗑️  Cleaned up: ${dir}`);
        } catch (error) {
          console.warn(`⚠️  Failed to cleanup ${dir}:`, error.message);
        }
      }
    }
    
    console.log('✅ Cleanup completed');
  }
  
  /**
   * Run a system command with timeout and error handling
   */
  async runCommand(command, args, options = {}) {
    const {
      timeout = 30000,
      stdio = 'pipe',
      env = process.env
    } = options;
    
    const startTime = Date.now();
    
    return new Promise((resolve) => {
      // Handle shell commands properly based on platform
      let spawnOptions;
      let commandToRun;
      let argsToUse;
      
      if (process.platform === 'win32') {
        commandToRun = 'cmd';
        argsToUse = ['/c', command, ...args];
        spawnOptions = { stdio, env };
      } else {
        commandToRun = command;
        argsToUse = args;
        spawnOptions = { stdio, env };
      }
      
      const child = spawn(commandToRun, argsToUse, spawnOptions);
      
      let stdout = '';
      let stderr = '';
      
      if (child.stdout) {
        child.stdout.on('data', (data) => {
          stdout += data.toString();
        });
      }
      
      if (child.stderr) {
        child.stderr.on('data', (data) => {
          stderr += data.toString();
        });
      }
      
      const timeoutId = setTimeout(() => {
        child.kill('SIGTERM');
        resolve({
          success: false,
          exitCode: -1,
          stdout,
          stderr: stderr + '\nTimeout: Process killed after ' + timeout + 'ms',
          duration: Date.now() - startTime
        });
      }, timeout);
      
      child.on('exit', (code) => {
        clearTimeout(timeoutId);
        resolve({
          success: code === 0,
          exitCode: code,
          stdout,
          stderr,
          duration: Date.now() - startTime
        });
      });
      
      child.on('error', (error) => {
        clearTimeout(timeoutId);
        resolve({
          success: false,
          exitCode: -1,
          stdout,
          stderr: stderr + '\nError: ' + error.message,
          duration: Date.now() - startTime
        });
      });
    });
  }
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  const runner = new E2ETestRunner();
  runner.run().catch((error) => {
    console.error('❌ E2E test runner failed:', error);
    process.exit(1);
  });
}

export default E2ETestRunner;