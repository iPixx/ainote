/**
 * WebDriver Manager for aiNote E2E Tests
 * 
 * Manages WebDriver lifecycle, application startup, and cross-platform compatibility.
 */

import { WebDriverConfig } from '../config/webdriver.config.js';
import { spawn } from 'child_process';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const fs = require('fs');
const path = require('path');

export class DriverManager {
  constructor(options = {}) {
    this.driver = null;
    this.applicationProcess = null;
    this.options = {
      browser: options.browser || 'chrome',
      headless: options.headless || process.env.HEADLESS === 'true',
      debug: options.debug || process.env.DEBUG === 'true',
      ...options
    };
    
    this.config = WebDriverConfig.getApplicationConfig();
  }
  
  /**
   * Set up WebDriver and test environment
   */
  async setup() {
    try {
      console.log('🔧 Setting up WebDriver for E2E testing...');
      
      // Create WebDriver instance
      this.driver = await WebDriverConfig.createDriver(this.options.browser);
      
      if (this.options.debug) {
        console.log(`✅ WebDriver created (${this.options.browser})`);
      }
      
      return this.driver;
      
    } catch (error) {
      console.error('❌ Failed to setup WebDriver:', error.message);
      throw error;
    }
  }
  
  /**
   * Teardown WebDriver and cleanup
   */
  async teardown() {
    try {
      console.log('🧹 Tearing down WebDriver...');
      
      // Stop application if running
      await this.stopApplication();
      
      // Quit WebDriver
      if (this.driver) {
        await this.driver.quit();
        this.driver = null;
        
        if (this.options.debug) {
          console.log('✅ WebDriver closed');
        }
      }
      
    } catch (error) {
      console.error('⚠️  Error during WebDriver teardown:', error.message);
    }
  }
  
  /**
   * Start the Tauri application for testing
   */
  async startApplication(timeout = 10000) {
    try {
      console.log('🚀 Starting aiNote application...');
      
      // Check if application binary exists
      const appPath = this.config.tauriApp;
      if (!fs.existsSync(appPath)) {
        throw new Error(`Application binary not found: ${appPath}\nRun 'pnpm tauri build' first.`);
      }
      
      // Start application process
      this.applicationProcess = spawn(appPath, [], {
        detached: false,
        stdio: this.options.debug ? 'inherit' : 'pipe'
      });
      
      // Handle application process events
      this.applicationProcess.on('error', (error) => {
        console.error('❌ Application process error:', error.message);
      });
      
      this.applicationProcess.on('exit', (code, signal) => {
        if (this.options.debug) {
          console.log(`📱 Application exited with code ${code}, signal ${signal}`);
        }
        this.applicationProcess = null;
      });
      
      // Wait for application to start
      await this.waitForApplicationStart(timeout);
      
      console.log('✅ aiNote application started');
      return true;
      
    } catch (error) {
      console.error('❌ Failed to start application:', error.message);
      throw error;
    }
  }
  
  /**
   * Stop the Tauri application
   */
  async stopApplication() {
    if (this.applicationProcess) {
      try {
        console.log('🛑 Stopping aiNote application...');
        
        // Send termination signal
        this.applicationProcess.kill('SIGTERM');
        
        // Wait for graceful shutdown
        await new Promise((resolve) => {
          const timeout = setTimeout(() => {
            if (this.applicationProcess) {
              this.applicationProcess.kill('SIGKILL');
            }
            resolve();
          }, 5000);
          
          this.applicationProcess.on('exit', () => {
            clearTimeout(timeout);
            resolve();
          });
        });
        
        this.applicationProcess = null;
        console.log('✅ Application stopped');
        
      } catch (error) {
        console.error('⚠️  Error stopping application:', error.message);
      }
    }
  }
  
  /**
   * Navigate to the application (for web-based testing)
   */
  async navigateToApplication() {
    if (!this.driver) {
      throw new Error('WebDriver not initialized. Call setup() first.');
    }
    
    try {
      // For macOS Chrome WebDriver approach
      const url = this.config.webUrl;
      
      if (this.options.debug) {
        console.log(`🌐 Navigating to: ${url}`);
      }
      
      await this.driver.get(url);
      
      // Wait for page to load
      await this.driver.sleep(2000);
      
      console.log('✅ Navigated to application');
      return true;
      
    } catch (error) {
      console.error('❌ Failed to navigate to application:', error.message);
      throw error;
    }
  }
  
  /**
   * Wait for application to start and be ready
   */
  async waitForApplicationStart(timeout = 10000) {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeout) {
      try {
        // Check if application process is still running
        if (!this.applicationProcess || this.applicationProcess.killed) {
          throw new Error('Application process terminated unexpectedly');
        }
        
        // For now, just wait a fixed time
        // In future, we could implement actual health checks
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        if (Date.now() - startTime > 3000) { // Minimum startup time
          return true;
        }
        
      } catch (error) {
        if (Date.now() - startTime >= timeout) {
          throw error;
        }
        await new Promise(resolve => setTimeout(resolve, 500));
      }
    }
    
    throw new Error(`Application failed to start within ${timeout}ms`);
  }
  
  /**
   * Take a screenshot for debugging
   */
  async takeScreenshot(filename) {
    if (!this.driver) {
      console.warn('⚠️  Cannot take screenshot: WebDriver not initialized');
      return null;
    }
    
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
      
      if (this.options.debug) {
        console.log(`📸 Screenshot saved: ${screenshotPath}`);
      }
      
      return screenshotPath;
      
    } catch (error) {
      console.error('⚠️  Failed to take screenshot:', error.message);
      return null;
    }
  }
  
  /**
   * Get current WebDriver instance
   */
  getDriver() {
    return this.driver;
  }
  
  /**
   * Check if application is running
   */
  isApplicationRunning() {
    return this.applicationProcess && !this.applicationProcess.killed;
  }
}

export default DriverManager;