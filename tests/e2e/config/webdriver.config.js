/**
 * WebDriver Configuration for aiNote E2E Tests
 * 
 * Cross-platform WebDriver setup with macOS Chrome support as primary,
 * designed for future tauri-driver integration on Linux/Windows.
 */

import { Builder, Browser, Capabilities } from 'selenium-webdriver';
import chrome from 'selenium-webdriver/chrome.js';

/**
 * WebDriver configuration for different platforms
 */
export const WebDriverConfig = {
  
  /**
   * Get Chrome WebDriver options optimized for Chrome for Testing
   */
  getChromeOptions() {
    const options = new chrome.Options();
    
    // Chrome for Testing optimizations
    options.addArguments([
      '--disable-dev-shm-usage',
      '--no-sandbox',
      '--disable-gpu',
      '--disable-web-security',
      '--disable-features=VizDisplayCompositor',
      '--disable-extensions',
      '--disable-plugins',
      '--disable-background-timer-throttling',
      '--disable-backgrounding-occluded-windows',
      '--disable-renderer-backgrounding',
      '--disable-default-apps',
      '--disable-sync',
      '--disable-translate',
      '--hide-scrollbars',
      '--metrics-recording-only',
      '--mute-audio',
      '--no-first-run',
      '--safebrowsing-disable-auto-update',
      '--window-size=1200,800'
    ]);
    
    // Chrome for Testing specific optimizations
    if (process.env.USE_CHROME_FOR_TESTING !== 'false') {
      options.addArguments([
        '--enable-automation',
        '--disable-browser-side-navigation',
        '--disable-client-side-phishing-detection',
        '--disable-component-update',
        '--disable-hang-monitor',
        '--disable-ipc-flooding-protection',
        '--disable-popup-blocking',
        '--disable-prompt-on-repost',
        '--password-store=basic',
        '--use-mock-keychain'
      ]);
    }
    
    // Headless mode for CI/CD
    if (process.env.HEADLESS === 'true' || process.env.CI) {
      options.addArguments(['--headless=new']); // Use new headless mode
    }
    
    // Debug mode settings
    if (process.env.DEBUG === 'true') {
      options.addArguments([
        '--enable-logging',
        '--v=1'
      ]);
    } else {
      // Reduce logging in production
      options.addArguments([
        '--log-level=3',
        '--silent'
      ]);
    }
    
    // Performance optimizations for E2E testing
    options.setUserPreferences({
      'profile.default_content_setting_values': {
        notifications: 2, // Block notifications
        media_stream: 2,  // Block media access
        geolocation: 2,   // Block location requests
        plugins: 2,       // Block plugins
        popups: 2,        // Block popups
        mixed_script: 2   // Block mixed scripts
      },
      'profile.managed_default_content_settings': {
        images: process.env.LOAD_IMAGES === 'true' ? 1 : 2 // Block images by default for speed
      },
      'profile.content_settings.exceptions.automatic_downloads.*.setting': 2 // Block downloads
    });
    
    // Set Chrome for Testing binary path if available
    if (process.env.CHROME_FOR_TESTING_PATH) {
      options.setChromeBinaryPath(process.env.CHROME_FOR_TESTING_PATH);
    }
    
    return options;
  },
  
  /**
   * Get capabilities for different browsers/platforms
   */
  getCapabilities(browser = 'chrome') {
    const capabilities = new Capabilities();
    
    switch (browser.toLowerCase()) {
      case 'chrome':
        capabilities.setBrowserName(Browser.CHROME);
        // Chrome options are handled directly in createDriver method
        break;
        
      case 'firefox':
        capabilities.setBrowserName(Browser.FIREFOX);
        // Firefox options for future use
        break;
        
      case 'safari':
        capabilities.setBrowserName(Browser.SAFARI);
        // Safari options for macOS (limited functionality)
        break;
        
      default:
        throw new Error(`Unsupported browser: ${browser}`);
    }
    
    return capabilities;
  },
  
  /**
   * Create WebDriver instance based on platform and configuration
   */
  async createDriver(browser = 'chrome') {
    const builder = new Builder();
    
    // Set browser 
    builder.forBrowser(browser);
    
    // Platform-specific driver setup
    if (browser === 'chrome') {
      const options = this.getChromeOptions();
      builder.setChromeOptions(options);
    }
    
    // Set timeouts
    const driver = await builder.build();
    await driver.manage().setTimeouts({
      implicit: 5000,      // Element finding timeout
      pageLoad: 15000,     // Page load timeout
      script: 10000        // Script execution timeout
    });
    
    return driver;
  },
  
  /**
   * Configuration for future tauri-driver integration
   */
  getTauriDriverConfig() {
    return {
      // Linux configuration
      linux: {
        driver: 'webkit',
        nativeDriver: 'WebKitWebDriver',
        capabilities: {
          'webkit:WebKitOptions': {
            args: ['--automation']
          }
        }
      },
      
      // Windows configuration  
      windows: {
        driver: 'edge',
        nativeDriver: 'msedgedriver',
        capabilities: {
          'ms:edgeOptions': {
            args: ['--automation']
          }
        }
      }
    };
  },
  
  /**
   * Get test application URL/path configuration
   */
  getApplicationConfig() {
    const platform = process.platform;
    const isDebug = process.env.NODE_ENV === 'development';
    
    return {
      // For Chrome WebDriver testing (current macOS approach)
      webUrl: 'file://' + process.cwd() + '/src/index.html',
      
      // For future tauri-driver integration
      tauriApp: {
        darwin: isDebug 
          ? './src-tauri/target/debug/ainote'
          : './src-tauri/target/release/ainote',
        linux: isDebug
          ? './src-tauri/target/debug/ainote'
          : './src-tauri/target/release/ainote',
        win32: isDebug
          ? './src-tauri/target/debug/ainote.exe'
          : './src-tauri/target/release/ainote.exe'
      }[platform],
      
      // Test data paths
      testVaultPath: './tests/e2e/fixtures/test-vault',
      testDataPath: './tests/e2e/fixtures/test-files.json'
    };
  }
};

export default WebDriverConfig;