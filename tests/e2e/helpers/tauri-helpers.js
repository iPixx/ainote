/**
 * Tauri Helper Utilities for aiNote E2E Tests
 * 
 * Provides high-level utilities for testing Tauri-specific functionality
 * with cross-platform compatibility.
 */

import { By, until, Key } from 'selenium-webdriver';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const fs = require('fs');
const path = require('path');

export class TauriHelpers {
  constructor(driver) {
    this.driver = driver;
    this.timeout = global.TEST_CONFIG?.elementTimeout || 5000;
    this.debug = global.TEST_CONFIG?.debug || false;
  }
  
  /**
   * Wait for an element to be present and visible
   */
  async waitForElement(locator, timeout = this.timeout) {
    try {
      const element = await this.driver.wait(
        until.elementLocated(locator),
        timeout,
        `Element not found: ${locator}`
      );
      
      await this.driver.wait(
        until.elementIsVisible(element),
        timeout,
        `Element not visible: ${locator}`
      );
      
      return element;
    } catch (error) {
      if (this.debug) {
        console.error(`❌ Element not found/visible: ${locator}`, error.message);
        // Take screenshot for debugging
        await this.takeScreenshot(`element_not_found_${Date.now()}`);
      }
      throw error;
    }
  }
  
  /**
   * Wait for application to be fully loaded
   */
  async waitForApplicationLoad() {
    try {
      if (this.debug) {
        console.log('⏳ Waiting for application to load...');
      }
      
      // Wait for main application container
      await this.waitForElement(By.id('app'));
      
      // Wait for basic UI components
      const components = [
        By.className('file-tree'),
        By.className('editor-preview-panel')
      ];
      
      for (const component of components) {
        try {
          await this.waitForElement(component, 10000);
        } catch (error) {
          console.warn(`⚠️  Component not loaded: ${component}`);
          // Continue - some components might not be visible initially
        }
      }
      
      if (this.debug) {
        console.log('✅ Application loaded successfully');
      }
      
      return true;
      
    } catch (error) {
      console.error('❌ Failed to load application:', error.message);
      throw error;
    }
  }
  
  /**
   * Select a vault for testing
   */
  async selectVault(vaultPath) {
    try {
      if (this.debug) {
        console.log(`📁 Selecting vault: ${vaultPath}`);
      }
      
      // Find vault selection button or input
      const vaultSelector = await this.waitForElement(By.id('vault-selector'));
      await vaultSelector.click();
      
      // In a real implementation, this would trigger a file dialog
      // For testing, we'll simulate the vault selection
      await this.simulateVaultSelection(vaultPath);
      
      // Wait for file tree to load
      await this.waitForFileTreeLoad();
      
      if (this.debug) {
        console.log('✅ Vault selected successfully');
      }
      
      return true;
      
    } catch (error) {
      console.error(`❌ Failed to select vault: ${vaultPath}`, error.message);
      throw error;
    }
  }
  
  /**
   * Wait for file tree to load
   */
  async waitForFileTreeLoad() {
    try {
      // Wait for file tree container
      const fileTree = await this.waitForElement(By.className('file-tree'));
      
      // Wait for at least one file item (or empty state)
      await this.driver.wait(async () => {
        const items = await fileTree.findElements(By.className('file-item'));
        const emptyState = await fileTree.findElements(By.className('empty-vault'));
        return items.length > 0 || emptyState.length > 0;
      }, 10000, 'File tree did not load');
      
      if (this.debug) {
        const items = await fileTree.findElements(By.className('file-item'));
        console.log(`📂 File tree loaded with ${items.length} items`);
      }
      
      return true;
      
    } catch (error) {
      console.error('❌ Failed to load file tree:', error.message);
      throw error;
    }
  }
  
  /**
   * Select a file in the file tree
   */
  async selectFile(filename) {
    try {
      if (this.debug) {
        console.log(`📄 Selecting file: ${filename}`);
      }
      
      // Find the file in the tree
      const fileTree = await this.waitForElement(By.className('file-tree'));
      const fileItems = await fileTree.findElements(By.className('file-item'));
      
      let targetFile = null;
      for (const item of fileItems) {
        const text = await item.getText();
        if (text.includes(filename)) {
          targetFile = item;
          break;
        }
      }
      
      if (!targetFile) {
        throw new Error(`File not found in tree: ${filename}`);
      }
      
      // Click the file
      await targetFile.click();
      
      // Wait for file content to load
      await this.waitForFileContentLoad();
      
      if (this.debug) {
        console.log('✅ File selected successfully');
      }
      
      return true;
      
    } catch (error) {
      console.error(`❌ Failed to select file: ${filename}`, error.message);
      throw error;
    }
  }
  
  /**
   * Wait for file content to load in editor
   */
  async waitForFileContentLoad() {
    try {
      // Wait for editor to be present
      const editor = await this.waitForElement(By.className('editor'));
      
      // Wait for content to appear (or empty editor)
      await this.driver.wait(async () => {
        const content = await editor.getAttribute('value') || await editor.getText();
        return content !== undefined; // Accept any content, including empty
      }, 5000, 'File content did not load');
      
      return true;
      
    } catch (error) {
      console.error('❌ Failed to load file content:', error.message);
      throw error;
    }
  }
  
  /**
   * Switch between editor and preview modes
   */
  async switchMode(mode) {
    try {
      if (this.debug) {
        console.log(`🔄 Switching to ${mode} mode`);
      }
      
      const modeButton = await this.waitForElement(By.id(`${mode}-mode-btn`));
      await modeButton.click();
      
      // Wait for mode to activate
      const panel = await this.waitForElement(By.className(`${mode}-panel`));
      await this.driver.wait(
        until.elementIsVisible(panel),
        this.timeout,
        `${mode} panel not visible`
      );
      
      if (this.debug) {
        console.log(`✅ Switched to ${mode} mode`);
      }
      
      return true;
      
    } catch (error) {
      console.error(`❌ Failed to switch to ${mode} mode:`, error.message);
      throw error;
    }
  }
  
  /**
   * Check if preview mode is active
   */
  async isPreviewMode() {
    try {
      const previewElements = await this.driver.findElements(
        By.css('[data-testid="preview"], .preview, .markdown-preview')
      );
      
      const editorElements = await this.driver.findElements(
        By.css('[data-testid="editor"], .editor:not(.preview), textarea')
      );
      
      // Preview mode if preview visible and editor hidden
      const previewVisible = previewElements.length > 0 && 
                            await previewElements[0].isDisplayed();
      
      const editorVisible = editorElements.length > 0 && 
                           await editorElements[0].isDisplayed();
      
      return previewVisible && !editorVisible;
      
    } catch (error) {
      console.error('❌ Failed to check preview mode:', error.message);
      return false;
    }
  }
  
  /**
   * Type content in the editor
   */
  async typeInEditor(content) {
    try {
      if (this.debug) {
        console.log(`✏️  Typing content: ${content.substring(0, 50)}...`);
      }
      
      // Find the editor element
      const editor = await this.waitForElement(By.className('editor'));
      
      // Clear existing content
      await editor.clear();
      
      // Type new content
      await editor.sendKeys(content);
      
      // Wait a moment for auto-save or processing
      await this.driver.sleep(1000);
      
      if (this.debug) {
        console.log('✅ Content typed successfully');
      }
      
      return true;
      
    } catch (error) {
      console.error('❌ Failed to type in editor:', error.message);
      throw error;
    }
  }
  
  /**
   * Get content from editor
   */
  async getEditorContent() {
    try {
      const editor = await this.waitForElement(By.className('editor'));
      const content = await editor.getAttribute('value') || await editor.getText();
      
      if (this.debug) {
        console.log(`📖 Retrieved content: ${content.length} characters`);
      }
      
      return content;
      
    } catch (error) {
      console.error('❌ Failed to get editor content:', error.message);
      throw error;
    }
  }
  
  /**
   * Simulate keyboard shortcuts
   */
  async sendKeyboardShortcut(keys) {
    try {
      if (this.debug) {
        console.log(`⌨️  Sending keyboard shortcut: ${keys}`);
      }
      
      const body = await this.driver.findElement(By.tagName('body'));
      await body.sendKeys(keys);
      
      // Wait for shortcut to take effect
      await this.driver.sleep(500);
      
      return true;
      
    } catch (error) {
      console.error(`❌ Failed to send keyboard shortcut: ${keys}`, error.message);
      throw error;
    }
  }
  
  /**
   * Simulate vault selection (for testing)
   */
  async simulateVaultSelection(vaultPath) {
    try {
      // In a real Tauri app, this would involve the native file dialog
      // For testing, we'll simulate by injecting the vault path into the app state
      
      if (this.debug) {
        console.log(`🎭 Simulating vault selection: ${vaultPath}`);
      }
      
      // Execute JavaScript to simulate vault selection
      await this.driver.executeScript(`
        // Simulate Tauri command response
        if (window.tauriTestMock) {
          window.tauriTestMock.simulateVaultSelection('${vaultPath}');
        }
        
        // Trigger vault selection event
        const event = new CustomEvent('vault-selected', { 
          detail: { vaultPath: '${vaultPath}' } 
        });
        window.dispatchEvent(event);
        
        return true;
      `);
      
      return true;
      
    } catch (error) {
      console.error('❌ Failed to simulate vault selection:', error.message);
      throw error;
    }
  }
  
  /**
   * Wait for auto-save to complete
   */
  async waitForAutoSave(timeout = 5000) {
    try {
      // Wait for save indicator to appear and disappear
      await this.driver.wait(async () => {
        const saveIndicators = await this.driver.findElements(
          By.css('.saving, [data-testid="saving"], .auto-save-indicator')
        );
        return saveIndicators.length === 0;
      }, timeout);
      
      if (this.debug) {
        console.log('💾 Auto-save completed');
      }
      
    } catch (error) {
      if (this.debug) {
        console.log('⚠️  Auto-save timeout (this may be normal)');
      }
    }
  }
  
  /**
   * Take a screenshot for debugging
   */
  async takeScreenshot(filename) {
    try {
      const screenshot = await this.driver.takeScreenshot();
      const screenshotPath = path.join('./tests/e2e/screenshots', `${filename}.png`);
      
      // Ensure screenshots directory exists
      const screenshotsDir = path.dirname(screenshotPath);
      if (!fs.existsSync(screenshotsDir)) {
        fs.mkdirSync(screenshotsDir, { recursive: true });
      }
      
      // Save screenshot
      fs.writeFileSync(screenshotPath, screenshot, 'base64');
      
      if (this.debug) {
        console.log(`📸 Screenshot saved: ${screenshotPath}`);
      }
      
      return screenshotPath;
      
    } catch (error) {
      console.error('⚠️  Failed to take screenshot:', error.message);
      return null;
    }
  }
  
  /**
   * Get application performance metrics
   */
  async getPerformanceMetrics() {
    try {
      const metrics = await this.driver.executeScript(`
        return {
          memory: performance.memory ? {
            used: performance.memory.usedJSHeapSize,
            total: performance.memory.totalJSHeapSize,
            limit: performance.memory.jsHeapSizeLimit
          } : null,
          timing: performance.timing ? {
            domContentLoaded: performance.timing.domContentLoadedEventEnd - performance.timing.navigationStart,
            loadComplete: performance.timing.loadEventEnd - performance.timing.navigationStart
          } : null,
          timestamp: Date.now()
        };
      `);
      
      if (this.debug) {
        console.log('📊 Performance metrics:', JSON.stringify(metrics, null, 2));
      }
      
      return metrics;
      
    } catch (error) {
      console.error('⚠️  Failed to get performance metrics:', error.message);
      return null;
    }
  }
}

export default TauriHelpers;