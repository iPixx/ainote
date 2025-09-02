/**
 * Auto-Save E2E Test Helper
 * 
 * Provides utilities for testing auto-save functionality in real browser environment
 */

export class AutoSaveTestHelper {
  constructor(page) {
    this.page = page;
    this.testVault = null;
  }

  async setup() {
    // Navigate to the application
    await this.page.goto('http://localhost:1420/');
    
    // Wait for app to be fully loaded
    await this.page.waitForSelector('[data-testid="app-container"]', { timeout: 10000 });
    
    // Enable verbose logging in the app for debugging
    await this.page.evaluate(() => {
      window.DEBUG_AUTO_SAVE = true;
      console.log('🔧 Auto-save debugging enabled');
    });
  }

  async openVault(vaultPath) {
    this.testVault = vaultPath;
    
    // Click on vault selector or button to open vault
    // This depends on your UI structure - adjust selector as needed
    const vaultButton = this.page.locator('[data-testid="select-vault-button"], .vault-selector, button:has-text("Select Vault")').first();
    
    if (await vaultButton.isVisible({ timeout: 2000 })) {
      await vaultButton.click();
    }
    
    // In a real app, this would trigger a file dialog
    // For testing, we might need to mock the Tauri file dialog or set the vault directly
    await this.page.evaluate((path) => {
      // Mock the vault selection
      if (window.__TAURI__) {
        window.__TAURI__.core.invoke('select_vault', { path });
      }
    }, vaultPath);
    
    // Wait for vault to be loaded
    await this.page.waitForTimeout(1000);
  }

  async selectFile(fileName) {
    // Look for the file in the file tree
    const fileItem = this.page.locator(`[data-testid="file-item"]:has-text("${fileName}"), .file-item:has-text("${fileName}")`).first();
    
    // If file tree item exists, click it
    if (await fileItem.isVisible({ timeout: 2000 })) {
      await fileItem.click();
    } else {
      // Fallback: directly load the file
      await this.page.evaluate((fileName) => {
        if (window.__TAURI__) {
          window.__TAURI__.core.invoke('open_file', { 
            file_path: fileName 
          });
        }
      }, fileName);
    }
    
    // Wait for file to be loaded in editor
    await this.page.waitForTimeout(1000);
  }

  async waitForAutoSave(timeoutMs = 3000) {
    // Wait for auto-save to complete
    return new Promise((resolve) => {
      let saveDetected = false;
      
      const handleConsole = (msg) => {
        if (msg.text().includes('auto save completed') || 
            msg.text().includes('AutoSave') ||
            msg.text().includes('save_success')) {
          saveDetected = true;
          this.page.off('console', handleConsole);
          resolve(true);
        }
      };
      
      this.page.on('console', handleConsole);
      
      setTimeout(() => {
        if (!saveDetected) {
          this.page.off('console', handleConsole);
          resolve(false);
        }
      }, timeoutMs);
    });
  }

  async getEditorContent() {
    const editor = this.page.locator('[data-testid="markdown-editor"]');
    return await editor.inputValue();
  }

  async setEditorContent(content) {
    const editor = this.page.locator('[data-testid="markdown-editor"]');
    await editor.click();
    await this.page.keyboard.press('Control+A');
    await editor.fill(content);
  }

  async triggerBlurSave() {
    // Click outside the editor to trigger blur
    await this.page.locator('body').click({ position: { x: 50, y: 50 } });
    await this.page.waitForTimeout(500); // Allow time for blur save
  }

  async cleanup() {
    // Clean up any test files or state
    if (this.testVault) {
      // Could clean up test files here if needed
    }
  }

  async debugEventFlow() {
    // Helper to log all auto-save related events
    await this.page.evaluate(() => {
      const originalLog = console.log;
      console.log = (...args) => {
        originalLog.apply(console, ['[DEBUG]', ...args]);
      };
      
      // Override AutoSave methods to add logging if they exist
      if (window.autoSave) {
        const originalHandleContentChange = window.autoSave.handleContentChange;
        window.autoSave.handleContentChange = function(...args) {
          console.log('🔄 AutoSave.handleContentChange called with:', args);
          return originalHandleContentChange.apply(this, args);
        };
        
        const originalPerformSave = window.autoSave.performSave;
        window.autoSave.performSave = function(...args) {
          console.log('💾 AutoSave.performSave called with:', args);
          return originalPerformSave.apply(this, args);
        };
      }
    });
  }
}