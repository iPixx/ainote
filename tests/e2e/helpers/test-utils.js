/**
 * Common Test Utilities for aiNote E2E Tests
 * 
 * Shared utilities and helper functions for E2E testing.
 */

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const fs = require('fs');
const path = require('path');

export class TestUtils {
  
  /**
   * Create test fixtures and sample data
   */
  static async createTestFixtures() {
    const fixturesDir = './tests/e2e/fixtures';
    const testVaultDir = path.join(fixturesDir, 'test-vault');
    
    // Ensure directories exist
    if (!fs.existsSync(fixturesDir)) {
      fs.mkdirSync(fixturesDir, { recursive: true });
    }
    
    if (!fs.existsSync(testVaultDir)) {
      fs.mkdirSync(testVaultDir, { recursive: true });
    }
    
    // Create test markdown files
    const testFiles = [
      {
        name: 'welcome.md',
        content: `# Welcome to aiNote

This is a test markdown file for E2E testing.

## Features

- [ ] File management
- [ ] Markdown editing  
- [ ] Real-time preview
- [ ] Auto-save functionality

## Sample Content

**Bold text** and *italic text* with some \`inline code\`.

\`\`\`javascript
console.log('Hello, aiNote!');
\`\`\`

> This is a blockquote for testing.

### Links and Images

[Test link](https://example.com)

---

End of test file.`
      },
      {
        name: 'notes.md',
        content: `# Test Notes

## Section 1

Some test content for section 1.

## Section 2

More content here with a list:

1. First item
2. Second item
3. Third item

### Subsection

Additional content for testing.`
      },
      {
        name: 'empty.md',
        content: ''
      }
    ];
    
    // Write test files
    for (const file of testFiles) {
      const filePath = path.join(testVaultDir, file.name);
      fs.writeFileSync(filePath, file.content, 'utf8');
    }
    
    // Create subdirectory with files
    const subDir = path.join(testVaultDir, 'subfolder');
    if (!fs.existsSync(subDir)) {
      fs.mkdirSync(subDir);
    }
    
    fs.writeFileSync(
      path.join(subDir, 'nested-file.md'),
      '# Nested File\n\nThis file is in a subdirectory.',
      'utf8'
    );
    
    return testVaultDir;
  }
  
  /**
   * Clean up test fixtures
   */
  static cleanupTestFixtures() {
    const fixturesDir = './tests/e2e/fixtures';
    const testVaultDir = path.join(fixturesDir, 'test-vault');
    
    try {
      if (fs.existsSync(testVaultDir)) {
        fs.rmSync(testVaultDir, { recursive: true, force: true });
      }
    } catch (error) {
      console.warn('⚠️  Failed to cleanup test fixtures:', error.message);
    }
  }
  
  /**
   * Generate test data for performance testing
   */
  static generateLargeMarkdownContent(sizeInKb = 100) {
    const targetSize = sizeInKb * 1024;
    const baseContent = `# Large Test File

## Introduction

This is a large markdown file generated for performance testing.

## Content Sections

`;
    
    let content = baseContent;
    let sectionNumber = 1;
    
    while (content.length < targetSize) {
      content += `
### Section ${sectionNumber}

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor 
incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis 
nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

\`\`\`javascript
// Code block ${sectionNumber}
function testFunction${sectionNumber}() {
  console.log('This is section ${sectionNumber}');
  return true;
}
\`\`\`

- List item 1 for section ${sectionNumber}
- List item 2 for section ${sectionNumber}  
- List item 3 for section ${sectionNumber}

> Blockquote for section ${sectionNumber} with some additional content
> to make the file larger and test performance.

`;
      sectionNumber++;
    }
    
    return content;
  }
  
  /**
   * Wait for a condition with timeout
   */
  static async waitForCondition(conditionFn, timeout = 5000, interval = 100) {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeout) {
      try {
        const result = await conditionFn();
        if (result) {
          return result;
        }
      } catch (error) {
        // Condition check failed, continue waiting
      }
      
      await new Promise(resolve => setTimeout(resolve, interval));
    }
    
    throw new Error(`Condition not met within ${timeout}ms`);
  }
  
  /**
   * Measure execution time of an async function
   */
  static async measureExecutionTime(fn, label = 'Operation') {
    const startTime = performance.now();
    
    try {
      const result = await fn();
      const endTime = performance.now();
      const duration = endTime - startTime;
      
      console.log(`⏱️  ${label} took ${duration.toFixed(2)}ms`);
      
      return {
        result,
        duration,
        success: true
      };
      
    } catch (error) {
      const endTime = performance.now();
      const duration = endTime - startTime;
      
      console.error(`❌ ${label} failed after ${duration.toFixed(2)}ms:`, error.message);
      
      return {
        error,
        duration,
        success: false
      };
    }
  }
  
  /**
   * Validate performance requirements
   */
  static validatePerformance(metrics, requirements) {
    const violations = [];
    
    for (const [metric, requirement] of Object.entries(requirements)) {
      const value = metrics[metric];
      
      if (value === undefined) {
        violations.push(`Missing metric: ${metric}`);
        continue;
      }
      
      if (typeof requirement === 'object') {
        if (requirement.max && value > requirement.max) {
          violations.push(`${metric}: ${value} > ${requirement.max} (max)`);
        }
        if (requirement.min && value < requirement.min) {
          violations.push(`${metric}: ${value} < ${requirement.min} (min)`);
        }
      } else if (value > requirement) {
        violations.push(`${metric}: ${value} > ${requirement}`);
      }
    }
    
    return {
      valid: violations.length === 0,
      violations
    };
  }
  
  /**
   * Create mock Tauri environment for testing
   */
  static createTauriTestMock() {
    return `
      // Mock Tauri APIs for testing
      window.tauriTestMock = {
        vaultPath: null,
        files: [],
        
        simulateVaultSelection: function(vaultPath) {
          this.vaultPath = vaultPath;
          this.files = [
            { name: 'welcome.md', path: vaultPath + '/welcome.md', isDir: false },
            { name: 'notes.md', path: vaultPath + '/notes.md', isDir: false },
            { name: 'empty.md', path: vaultPath + '/empty.md', isDir: false },
            { name: 'subfolder', path: vaultPath + '/subfolder', isDir: true }
          ];
          
          // Trigger application state update
          if (window.appState && window.appState.setVault) {
            window.appState.setVault(vaultPath);
          }
        },
        
        simulateFileRead: function(filePath) {
          // Return mock file content based on filename
          if (filePath.includes('welcome.md')) {
            return '# Welcome to aiNote\\n\\nThis is test content.';
          } else if (filePath.includes('notes.md')) {
            return '# Test Notes\\n\\n## Section 1\\n\\nSome content.';
          } else if (filePath.includes('empty.md')) {
            return '';
          }
          return '# Mock File\\n\\nMock content.';
        }
      };
      
      // Mock Tauri invoke function
      if (!window.__TAURI__) {
        window.__TAURI__ = {
          core: {
            invoke: async function(command, args = {}) {
              console.log('Mock Tauri invoke:', command, args);
              
              switch (command) {
                case 'select_vault':
                  return window.tauriTestMock.vaultPath || '/test/vault';
                
                case 'scan_vault_files':
                  return window.tauriTestMock.files;
                
                case 'read_file':
                  return window.tauriTestMock.simulateFileRead(args.filePath);
                
                case 'write_file':
                  return true;
                
                default:
                  return null;
              }
            }
          }
        };
      }
      
      console.log('Tauri test mock initialized');
    `;
  }
  
  /**
   * Get system information for test reporting
   */
  static getSystemInfo() {
    return {
      platform: process.platform,
      arch: process.arch,
      nodeVersion: process.version,
      timestamp: new Date().toISOString(),
      cwd: process.cwd(),
      env: {
        CI: process.env.CI,
        HEADLESS: process.env.HEADLESS,
        DEBUG: process.env.DEBUG
      }
    };
  }
  
  /**
   * Generate test report
   */
  static generateTestReport(results) {
    const systemInfo = this.getSystemInfo();
    
    const report = {
      meta: {
        generated: new Date().toISOString(),
        system: systemInfo,
        summary: {
          total: results.length,
          passed: results.filter(r => r.success).length,
          failed: results.filter(r => !r.success).length,
          duration: results.reduce((sum, r) => sum + r.duration, 0)
        }
      },
      results: results.map(result => ({
        name: result.name,
        success: result.success,
        duration: result.duration,
        error: result.error?.message,
        metrics: result.metrics
      }))
    };
    
    // Save report to file
    const reportPath = './tests/e2e/test-report.json';
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
    
    console.log(`📊 Test report saved: ${reportPath}`);
    return report;
  }
}

export default TestUtils;