/**
 * E2E Infrastructure Demo Test
 * 
 * Demonstrates that the E2E testing infrastructure is working correctly
 * by testing basic browser functionality and aiNote's actual index.html.
 */

import { describe, it, before, after, beforeEach } from 'mocha';
import { expect } from 'chai';
import DriverManager from '../helpers/driver-manager.js';
import TauriHelpers from '../helpers/tauri-helpers.js';
import TestUtils from '../helpers/test-utils.js';

describe('E2E Infrastructure Demo', function() {
  let driverManager;
  let tauriHelpers;
  
  before(async function() {
    this.timeout(30000);
    
    console.log('🔧 Setting up E2E Infrastructure Demo...');
    
    // Initialize WebDriver
    driverManager = new DriverManager({
      browser: 'chrome',
      headless: process.env.HEADLESS === 'true',
      debug: true
    });
    
    await driverManager.setup();
    tauriHelpers = new TauriHelpers(driverManager.driver);
    
    console.log('✅ E2E Infrastructure Demo setup complete');
  });
  
  after(async function() {
    this.timeout(15000);
    
    if (driverManager) {
      await driverManager.teardown();
    }
  });
  
  beforeEach(async function() {
    // Take screenshot before each test for debugging
    const testName = this.currentTest.title.replace(/\s+/g, '_');
    await tauriHelpers.takeScreenshot(`demo_before_${testName}`);
  });
  
  describe('Basic Browser Functionality', function() {
    
    it('should navigate to a simple HTML page', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Navigate to a simple test page
        await driverManager.driver.get('data:text/html,<html><head><title>E2E Test</title></head><body><h1 id="heading">E2E Infrastructure Working!</h1><p>This demonstrates the E2E testing infrastructure is functional.</p></body></html>');
        
        // Verify navigation
        const title = await driverManager.driver.getTitle();
        expect(title).to.equal('E2E Test');
        
        return true;
      }, 'Simple HTML navigation');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(5000);
      
      console.log('✅ Successfully navigated to test page');
    });
    
    it('should find and interact with DOM elements', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Find heading element
        const heading = await driverManager.driver.findElement({ id: 'heading' });
        const headingText = await heading.getText();
        
        expect(headingText).to.equal('E2E Infrastructure Working!');
        
        // Test element interaction
        await heading.click();
        
        return true;
      }, 'DOM element interaction');
      
      expect(result.success).to.be.true;
      console.log('✅ Successfully found and interacted with DOM elements');
    });
    
  });
  
  describe('aiNote Application File Testing', function() {
    
    it('should load aiNote index.html file', async function() {
      this.timeout(10000);
      
      const result = await TestUtils.measureExecutionTime(async () => {
        // Navigate to aiNote's actual index.html
        const indexPath = `file://${process.cwd()}/src/index.html`;
        console.log(`🌐 Navigating to: ${indexPath}`);
        
        await driverManager.driver.get(indexPath);
        
        // Wait for page to load
        await driverManager.driver.sleep(2000);
        
        // Verify basic page structure
        const title = await driverManager.driver.getTitle();
        console.log(`📄 Page title: "${title}"`);
        
        // Look for aiNote elements (they might not be functional without Tauri, but should exist)
        const bodyElements = await driverManager.driver.findElements({ tagName: 'body' });
        expect(bodyElements.length).to.be.greaterThan(0);
        
        return true;
      }, 'aiNote index.html loading');
      
      expect(result.success).to.be.true;
      console.log('✅ Successfully loaded aiNote application HTML');
    });
    
    it('should execute JavaScript on aiNote page', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Execute JavaScript to test page functionality
        const jsResult = await driverManager.driver.executeScript(`
          return {
            hasWindow: typeof window !== 'undefined',
            hasDocument: typeof document !== 'undefined',
            bodyExists: document.body !== null,
            title: document.title,
            bodyTagName: document.body ? document.body.tagName : null
          };
        `);
        
        console.log('📊 JavaScript execution result:', JSON.stringify(jsResult, null, 2));
        
        expect(jsResult.hasWindow).to.be.true;
        expect(jsResult.hasDocument).to.be.true;
        expect(jsResult.bodyExists).to.be.true;
        expect(jsResult.bodyTagName).to.equal('BODY');
        
        return true;
      }, 'JavaScript execution test');
      
      expect(result.success).to.be.true;
      console.log('✅ Successfully executed JavaScript on aiNote page');
    });
    
  });
  
  describe('E2E Testing Utilities', function() {
    
    it('should demonstrate screenshot functionality', async function() {
      const screenshotPath = await tauriHelpers.takeScreenshot('demo_screenshot_test');
      
      expect(screenshotPath).to.be.a('string');
      expect(screenshotPath).to.include('demo_screenshot_test.png');
      
      console.log(`📸 Screenshot saved: ${screenshotPath}`);
    });
    
    it('should collect performance metrics', async function() {
      const metrics = await tauriHelpers.getPerformanceMetrics();
      
      expect(metrics).to.be.an('object');
      expect(metrics).to.have.property('timestamp');
      
      if (metrics.memory) {
        const memoryMB = metrics.memory.used / (1024 * 1024);
        console.log(`💾 Memory usage: ${memoryMB.toFixed(2)} MB`);
        expect(memoryMB).to.be.below(500); // Should be reasonable
      }
      
      console.log('📊 Performance metrics collected successfully');
    });
    
    it('should test fixture creation utilities', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Test fixture creation
        const vaultPath = await TestUtils.createTestFixtures();
        expect(vaultPath).to.be.a('string');
        expect(vaultPath).to.include('test-vault');
        
        // Clean up
        TestUtils.cleanupTestFixtures();
        
        return vaultPath;
      }, 'Test fixture operations');
      
      expect(result.success).to.be.true;
      console.log(`✅ Test fixtures created and cleaned up: ${result.result}`);
    });
    
  });
  
  describe('Performance Validation', function() {
    
    it('should meet performance requirements for E2E operations', async function() {
      const operations = [];
      
      // Test multiple operations
      for (let i = 0; i < 5; i++) {
        const opResult = await TestUtils.measureExecutionTime(async () => {
          await driverManager.driver.executeScript('return document.title;');
          return `operation_${i}`;
        }, `Performance test operation ${i + 1}`);
        
        operations.push(opResult);
      }
      
      // Validate all operations completed successfully
      const successful = operations.filter(op => op.success).length;
      expect(successful).to.equal(5);
      
      // Validate average performance
      const avgDuration = operations.reduce((sum, op) => sum + op.duration, 0) / operations.length;
      expect(avgDuration).to.be.below(100); // Should be fast
      
      console.log(`⚡ Average operation time: ${avgDuration.toFixed(2)}ms`);
    });
    
  });
  
});