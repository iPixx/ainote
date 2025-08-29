/**
 * Tauri Application Manager for True E2E Testing
 * 
 * Manages the lifecycle of the actual Tauri application for complete
 * end-to-end testing including backend Rust code and Tauri APIs.
 */

import { spawn } from 'child_process';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const fs = require('fs');
const path = require('path');

export class TauriAppManager {
  constructor(options = {}) {
    this.process = null;
    this.config = {
      debug: options.debug || process.env.DEBUG === 'true',
      timeout: options.timeout || 30000,
      port: options.port || null, // Let Tauri choose
      mode: options.mode || 'release', // 'debug' or 'release'
      ...options
    };
    
    this.appPath = this.getApplicationPath();
    this.isRunning = false;
  }
  
  /**
   * Get the path to the Tauri application binary
   */
  getApplicationPath() {
    const mode = this.config.mode;
    const platforms = {
      darwin: `./src-tauri/target/${mode}/ainote`,
      linux: `./src-tauri/target/${mode}/ainote`,
      win32: `./src-tauri/target/${mode}/ainote.exe`
    };
    
    return platforms[process.platform];
  }
  
  /**
   * Verify the application binary exists
   */
  verifyApplication() {
    if (!fs.existsSync(this.appPath)) {
      const mode = this.config.mode;
      throw new Error(
        `Tauri application not found: ${this.appPath}\n` +
        `Run: pnpm tauri build${mode === 'debug' ? ' --debug' : ''}`
      );
    }
    
    const stats = fs.statSync(this.appPath);
    return {
      path: this.appPath,
      size: stats.size,
      modified: stats.mtime,
      mode: this.config.mode
    };
  }
  
  /**
   * Start the Tauri application
   */
  async startApplication() {
    if (this.isRunning) {
      console.log('⚠️  Application already running');
      return true;
    }
    
    try {
      console.log(`🚀 Starting Tauri application: ${this.appPath}`);
      
      // Verify application exists
      this.verifyApplication();
      
      // Start the application process
      this.process = spawn(this.appPath, [], {
        detached: false,
        stdio: this.config.debug ? 'inherit' : 'pipe',
        env: {
          ...process.env,
          // Tauri-specific environment variables for testing
          TAURI_ENV: 'test',
          RUST_LOG: this.config.debug ? 'debug' : 'error'
        }
      });
      
      // Handle process events
      this.setupProcessHandlers();
      
      // Wait for application to be ready
      await this.waitForApplicationReady();
      
      this.isRunning = true;
      console.log('✅ Tauri application started successfully');
      
      return true;
      
    } catch (error) {
      console.error('❌ Failed to start Tauri application:', error.message);
      throw error;
    }
  }
  
  /**
   * Setup process event handlers
   */
  setupProcessHandlers() {
    this.process.on('error', (error) => {
      console.error('❌ Tauri application process error:', error.message);
      this.isRunning = false;
    });
    
    this.process.on('exit', (code, signal) => {
      if (this.config.debug) {
        console.log(`📱 Tauri application exited (code: ${code}, signal: ${signal})`);
      }
      this.isRunning = false;
      this.process = null;
    });
    
    // Capture stdout/stderr if not in debug mode
    if (!this.config.debug && this.process.stdout && this.process.stderr) {
      this.process.stdout.on('data', (data) => {
        if (this.config.debug) {
          console.log('📤 App stdout:', data.toString().trim());
        }
      });
      
      this.process.stderr.on('data', (data) => {
        const output = data.toString().trim();
        if (output && !output.includes('RUST_LOG')) { // Filter noise
          console.warn('📥 App stderr:', output);
        }
      });
    }
  }
  
  /**
   * Wait for the application to be ready for testing
   */
  async waitForApplicationReady() {
    const startTime = Date.now();
    const timeout = this.config.timeout;
    
    return new Promise((resolve, reject) => {
      const checkReady = () => {
        const elapsed = Date.now() - startTime;
        
        // Check if process is still running
        if (!this.process || this.process.killed) {
          reject(new Error('Application process terminated unexpectedly'));
          return;
        }
        
        // For now, use a simple time-based readiness check
        // In a real implementation, you might check for:
        // - Window creation
        // - HTTP endpoint availability
        // - Log output indicating readiness
        
        if (elapsed > 3000) { // Minimum startup time
          resolve(true);
        } else if (elapsed < timeout) {
          setTimeout(checkReady, 500);
        } else {
          reject(new Error(`Application failed to be ready within ${timeout}ms`));
        }
      };
      
      checkReady();
    });
  }
  
  /**
   * Stop the Tauri application
   */
  async stopApplication() {
    if (!this.isRunning || !this.process) {
      console.log('ℹ️  No application running to stop');
      return true;
    }
    
    try {
      console.log('🛑 Stopping Tauri application...');
      
      // Try graceful shutdown first
      this.process.kill('SIGTERM');
      
      // Wait for graceful shutdown
      await new Promise((resolve) => {
        const timeout = setTimeout(() => {
          if (this.process && !this.process.killed) {
            console.log('⚠️  Forcing application shutdown...');
            this.process.kill('SIGKILL');
          }
          resolve();
        }, 5000);
        
        if (this.process) {
          this.process.on('exit', () => {
            clearTimeout(timeout);
            resolve();
          });
        } else {
          clearTimeout(timeout);
          resolve();
        }
      });
      
      this.isRunning = false;
      this.process = null;
      
      console.log('✅ Tauri application stopped');
      return true;
      
    } catch (error) {
      console.error('⚠️  Error stopping application:', error.message);
      this.isRunning = false;
      this.process = null;
      return false;
    }
  }
  
  /**
   * Restart the application
   */
  async restartApplication() {
    console.log('🔄 Restarting Tauri application...');
    await this.stopApplication();
    await new Promise(resolve => setTimeout(resolve, 1000)); // Brief pause
    return await this.startApplication();
  }
  
  /**
   * Get application process information
   */
  getProcessInfo() {
    return {
      isRunning: this.isRunning,
      pid: this.process ? this.process.pid : null,
      appPath: this.appPath,
      mode: this.config.mode,
      uptime: this.process ? Date.now() - this.process.spawnargs : null
    };
  }
  
  /**
   * Check if application is healthy
   */
  async healthCheck() {
    if (!this.isRunning || !this.process) {
      return false;
    }
    
    // Basic health check - process is running
    try {
      process.kill(this.process.pid, 0); // Signal 0 just checks if process exists
      return true;
    } catch (error) {
      console.warn('⚠️  Application health check failed:', error.message);
      this.isRunning = false;
      return false;
    }
  }
  
  /**
   * Get application window information (for WebDriver connection)
   */
  getConnectionInfo() {
    // For true E2E testing, you might need to detect the application's
    // WebDriver endpoint or window handle. This is a placeholder.
    return {
      type: 'native_app',
      binary: this.appPath,
      pid: this.process ? this.process.pid : null,
      // In real implementation, might include:
      // - WebDriver port
      // - Window handles
      // - Application URLs
    };
  }
}

export default TauriAppManager;