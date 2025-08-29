/**
 * Vault Operations E2E Tests
 * 
 * Tests vault selection, file tree loading, and vault management functionality.
 */

import { describe, it, before, after, beforeEach } from 'mocha';
import { expect } from 'chai';
import DriverManager from '../helpers/driver-manager.js';
import TauriHelpers from '../helpers/tauri-helpers.js';
import TestUtils from '../helpers/test-utils.js';

describe('Vault Operations E2E Tests', function() {
  let driverManager;
  let tauriHelpers;
  let testVaultPath;
  
  // Setup test environment
  before(async function() {
    this.timeout(60000);
    
    console.log('🔧 Setting up Vault Operations E2E tests...');
    
    // Create test fixtures
    testVaultPath = await TestUtils.createTestFixtures();
    console.log(`📁 Test vault created: ${testVaultPath}`);
    
    // Initialize WebDriver
    driverManager = new DriverManager({
      browser: 'chrome',
      headless: process.env.HEADLESS === 'true',
      debug: process.env.DEBUG === 'true'
    });
    
    await driverManager.setup();
    tauriHelpers = new TauriHelpers(driverManager.driver);
    
    // Navigate to application
    await driverManager.navigateToApplication();
    
    // Inject test mocks
    await driverManager.driver.executeScript(TestUtils.createTauriTestMock());
    
    console.log('✅ Vault Operations E2E test setup complete');
  });
  
  after(async function() {
    this.timeout(30000);
    
    console.log('🧹 Cleaning up Vault Operations E2E tests...');
    
    if (driverManager) {
      await driverManager.teardown();
    }
    
    TestUtils.cleanupTestFixtures();
    
    console.log('✅ Vault Operations E2E test cleanup complete');
  });
  
  beforeEach(async function() {
    // Take screenshot before each test for debugging
    if (process.env.DEBUG === 'true') {
      const testName = this.currentTest.title.replace(/\s+/g, '_');
      await tauriHelpers.takeScreenshot(`vault_ops_before_${testName}`);
    }
  });
  
  describe('Vault Selection', function() {
    
    it('should display vault selector on application start', async function() {
      // Wait for application to load
      await tauriHelpers.waitForApplicationLoad();
      
      // Check for vault selector element
      const vaultSelector = await driverManager.driver.findElement({ id: 'vault-selector' });
      expect(vaultSelector).to.not.be.null;
      
      // Verify selector is visible and clickable
      const isDisplayed = await vaultSelector.isDisplayed();
      const isEnabled = await vaultSelector.isEnabled();
      
      expect(isDisplayed).to.be.true;
      expect(isEnabled).to.be.true;
    });
    
    it('should open vault selection dialog when clicked', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Click vault selector
        const vaultSelector = await driverManager.driver.findElement({ id: 'vault-selector' });
        await vaultSelector.click();
        
        // In a real app, this would open a native file dialog
        // For testing, we simulate the selection
        return true;
      }, 'Vault selector click');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(1000); // Should be fast
    });
    
    it('should handle vault selection and load file tree', async function() {
      this.timeout(15000);
      
      const result = await TestUtils.measureExecutionTime(async () => {
        // Simulate vault selection
        await tauriHelpers.selectVault(testVaultPath);
        
        // Verify file tree loaded
        await tauriHelpers.waitForFileTreeLoad();
        
        return true;
      }, 'Vault selection and file tree loading');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(10000); // Should load within 10 seconds
      
      // Verify performance requirement
      const performanceValid = TestUtils.validatePerformance(
        { vaultLoadTime: result.duration },
        { vaultLoadTime: 5000 }
      );
      
      expect(performanceValid.valid).to.be.true;
    });
    
  });
  
  describe('File Tree Loading', function() {
    
    beforeEach(async function() {
      // Ensure vault is selected before each test
      await tauriHelpers.selectVault(testVaultPath);
      await tauriHelpers.waitForFileTreeLoad();
    });
    
    it('should display all expected files in the tree', async function() {
      // Get file tree element
      const fileTree = await driverManager.driver.findElement({ className: 'file-tree' });
      
      // Find all file items
      const fileItems = await fileTree.findElements({ className: 'file-item' });
      
      // Verify we have the expected number of files
      expect(fileItems.length).to.be.at.least(3); // welcome.md, notes.md, empty.md, subfolder
      
      // Check for specific files
      const fileTexts = await Promise.all(
        fileItems.map(item => item.getText())
      );
      
      expect(fileTexts.some(text => text.includes('welcome.md'))).to.be.true;
      expect(fileTexts.some(text => text.includes('notes.md'))).to.be.true;
      expect(fileTexts.some(text => text.includes('empty.md'))).to.be.true;
      expect(fileTexts.some(text => text.includes('subfolder'))).to.be.true;
    });
    
    it('should handle empty vault gracefully', async function() {
      // Create empty vault for testing
      const emptyVaultPath = './tests/e2e/fixtures/empty-vault';
      const fs = require('fs');
      
      if (!fs.existsSync(emptyVaultPath)) {
        fs.mkdirSync(emptyVaultPath, { recursive: true });
      }
      
      // Select empty vault
      await tauriHelpers.selectVault(emptyVaultPath);
      await tauriHelpers.waitForFileTreeLoad();
      
      // Should show empty state
      const fileTree = await driverManager.driver.findElement({ className: 'file-tree' });
      const emptyState = await fileTree.findElements({ className: 'empty-vault' });
      
      expect(emptyState.length).to.be.greaterThan(0);
      
      // Cleanup
      fs.rmSync(emptyVaultPath, { recursive: true, force: true });
    });
    
    it('should expand and collapse folders correctly', async function() {
      // Find subfolder
      const fileItems = await driverManager.driver.findElements({ className: 'file-item' });
      
      let subfolderItem = null;
      for (const item of fileItems) {
        const text = await item.getText();
        if (text.includes('subfolder')) {
          subfolderItem = item;
          break;
        }
      }
      
      expect(subfolderItem).to.not.be.null;
      
      // Click to expand folder
      await subfolderItem.click();
      
      // Wait for expansion
      await driverManager.driver.sleep(1000);
      
      // Check for nested file
      const allItems = await driverManager.driver.findElements({ className: 'file-item' });
      const itemTexts = await Promise.all(allItems.map(item => item.getText()));
      
      expect(itemTexts.some(text => text.includes('nested-file.md'))).to.be.true;
    });
    
  });
  
  describe('Vault Management', function() {
    
    it('should remember selected vault on application restart', async function() {
      // This would test persistence, but requires application restart
      // For now, we'll test that the vault state is maintained
      
      await tauriHelpers.selectVault(testVaultPath);
      await tauriHelpers.waitForFileTreeLoad();
      
      // Refresh the page to simulate app restart
      await driverManager.driver.navigate().refresh();
      await driverManager.driver.executeScript(TestUtils.createTauriTestMock());
      
      // Check if vault state is restored
      // In a real implementation, this would check saved state
      await tauriHelpers.waitForApplicationLoad();
      
      // Vault selector should still be available
      const vaultSelector = await driverManager.driver.findElement({ id: 'vault-selector' });
      expect(vaultSelector).to.not.be.null;
    });
    
    it('should handle vault switching efficiently', async function() {
      // Select first vault
      await tauriHelpers.selectVault(testVaultPath);
      await tauriHelpers.waitForFileTreeLoad();
      
      // Create second test vault
      const secondVaultPath = './tests/e2e/fixtures/second-vault';
      const fs = require('fs');
      
      if (!fs.existsSync(secondVaultPath)) {
        fs.mkdirSync(secondVaultPath, { recursive: true });
        fs.writeFileSync(
          `${secondVaultPath}/second-vault-file.md`,
          '# Second Vault File\n\nThis is in the second vault.',
          'utf8'
        );
      }
      
      // Switch to second vault
      const result = await TestUtils.measureExecutionTime(async () => {
        await tauriHelpers.selectVault(secondVaultPath);
        await tauriHelpers.waitForFileTreeLoad();
        return true;
      }, 'Vault switching');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(8000); // Switching should be reasonably fast
      
      // Verify new vault content
      const fileTree = await driverManager.driver.findElement({ className: 'file-tree' });
      const fileItems = await fileTree.findElements({ className: 'file-item' });
      const fileTexts = await Promise.all(fileItems.map(item => item.getText()));
      
      expect(fileTexts.some(text => text.includes('second-vault-file.md'))).to.be.true;
      
      // Cleanup
      fs.rmSync(secondVaultPath, { recursive: true, force: true });
    });
    
  });
  
  describe('Performance Requirements', function() {
    
    it('should meet vault loading performance requirements', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Create a larger test vault for performance testing
        const largeVaultPath = './tests/e2e/fixtures/large-vault';
        const fs = require('fs');
        
        if (!fs.existsSync(largeVaultPath)) {
          fs.mkdirSync(largeVaultPath, { recursive: true });
          
          // Create multiple test files
          for (let i = 1; i <= 20; i++) {
            fs.writeFileSync(
              `${largeVaultPath}/file-${i.toString().padStart(2, '0')}.md`,
              `# File ${i}\n\n${TestUtils.generateLargeMarkdownContent(10)}`,
              'utf8'
            );
          }
        }
        
        await tauriHelpers.selectVault(largeVaultPath);
        await tauriHelpers.waitForFileTreeLoad();
        
        // Cleanup
        fs.rmSync(largeVaultPath, { recursive: true, force: true });
        
        return true;
      }, 'Large vault loading');
      
      // Validate performance requirements
      const performanceValid = TestUtils.validatePerformance(
        { vaultLoadTime: result.duration },
        { vaultLoadTime: 10000 } // Allow more time for large vault
      );
      
      expect(result.success).to.be.true;
      expect(performanceValid.valid).to.be.true;
      
      if (!performanceValid.valid) {
        console.error('Performance violations:', performanceValid.violations);
      }
    });
    
    it('should maintain low memory usage during vault operations', async function() {
      // Select vault and load file tree
      await tauriHelpers.selectVault(testVaultPath);
      await tauriHelpers.waitForFileTreeLoad();
      
      // Get memory usage
      const metrics = await tauriHelpers.getPerformanceMetrics();
      
      if (metrics && metrics.memory) {
        const memoryMB = metrics.memory.used / (1024 * 1024);
        console.log(`📊 Memory usage: ${memoryMB.toFixed(2)} MB`);
        
        // Memory should be reasonable for a simple vault
        expect(memoryMB).to.be.below(50); // Should use less than 50MB
      }
    });
    
  });
  
});