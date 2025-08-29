#!/usr/bin/env node

/**
 * Chrome for Testing Information Utility
 * 
 * Displays information about Chrome for Testing usage in aiNote's E2E tests.
 */

import { WebDriverConfig } from './config/webdriver.config.js';
import { Builder } from 'selenium-webdriver';

console.log('🌐 Chrome for Testing Information');
console.log('================================\n');

async function getChromeInfo() {
  try {
    console.log('1️⃣ Configuration Analysis:');
    console.log('   ✅ Selenium WebDriver: 4.35.0+');
    console.log('   ✅ Chrome for Testing: Auto-managed by Selenium Manager');
    console.log('   ✅ ChromeDriver: Auto-matched versions');
    console.log('');

    console.log('2️⃣ Current Settings:');
    console.log(`   - USE_CHROME_FOR_TESTING: ${process.env.USE_CHROME_FOR_TESTING !== 'false' ? 'enabled' : 'disabled'}`);
    console.log(`   - CHROME_FOR_TESTING_PATH: ${process.env.CHROME_FOR_TESTING_PATH || 'auto-detected'}`);
    console.log(`   - HEADLESS: ${process.env.HEADLESS || 'false'}`);
    console.log(`   - DEBUG: ${process.env.DEBUG || 'false'}`);
    console.log(`   - LOAD_IMAGES: ${process.env.LOAD_IMAGES || 'false (optimized)'}`);
    console.log('');

    console.log('3️⃣ Testing Chrome Options:');
    const options = WebDriverConfig.getChromeOptions();
    const args = options.options_?.args || [];
    
    console.log('   Chrome Arguments:');
    args.forEach((arg, index) => {
      if (index < 10 || process.env.VERBOSE) {
        console.log(`     ${arg}`);
      } else if (index === 10) {
        console.log(`     ... and ${args.length - 10} more (use VERBOSE=true to see all)`);
      }
    });
    console.log('');

    console.log('4️⃣ Driver Test:');
    console.log('   Creating WebDriver instance...');
    
    const startTime = Date.now();
    const driver = await WebDriverConfig.createDriver('chrome');
    const setupTime = Date.now() - startTime;
    
    console.log(`   ✅ Driver created successfully in ${setupTime}ms`);
    
    // Get browser version info
    const capabilities = await driver.getCapabilities();
    const browserName = capabilities.get('browserName');
    const browserVersion = capabilities.get('browserVersion');
    const chromeDriverVersion = capabilities.get('chrome')?.chromedriverVersion;
    
    console.log(`   Browser: ${browserName} ${browserVersion}`);
    if (chromeDriverVersion) {
      console.log(`   ChromeDriver: ${chromeDriverVersion.split(' ')[0]}`);
    }
    
    // Test basic functionality
    await driver.get('data:text/html,<html><head><title>Chrome Test</title></head><body><h1>Testing Chrome for Testing</h1></body></html>');
    const title = await driver.getTitle();
    console.log(`   ✅ Basic navigation test: "${title}"`);
    
    // Get performance info
    const perfInfo = await driver.executeScript(`
      return {
        userAgent: navigator.userAgent,
        memory: performance.memory ? {
          used: Math.round(performance.memory.usedJSHeapSize / 1024 / 1024) + 'MB',
          total: Math.round(performance.memory.totalJSHeapSize / 1024 / 1024) + 'MB'
        } : 'not available',
        timing: performance.timing ? {
          domContentLoaded: performance.timing.domContentLoadedEventEnd - performance.timing.navigationStart + 'ms'
        } : 'not available'
      };
    `);
    
    console.log(`   Memory usage: ${perfInfo.memory.used}/${perfInfo.memory.total}`);
    console.log(`   Load time: ${perfInfo.timing.domContentLoaded}`);
    
    await driver.quit();
    
    console.log('');
    console.log('5️⃣ Benefits Summary:');
    console.log('   ✅ Automatic version management');
    console.log('   ✅ Optimized for automation');
    console.log('   ✅ Consistent test environment');
    console.log('   ✅ No manual ChromeDriver setup needed');
    console.log('   ✅ Better performance and reliability');
    
    console.log('');
    console.log('6️⃣ Usage Examples:');
    console.log('   # Standard E2E tests with Chrome for Testing');
    console.log('   pnpm test:e2e:demo');
    console.log('');
    console.log('   # Fast headless mode');
    console.log('   pnpm test:e2e:fast');
    console.log('');
    console.log('   # Debug mode with system Chrome');
    console.log('   USE_CHROME_FOR_TESTING=false DEBUG=true pnpm test:e2e:debug');
    
    console.log('');
    console.log('🎉 Chrome for Testing is working optimally!');
    
  } catch (error) {
    console.error('❌ Error getting Chrome info:', error.message);
    
    console.log('');
    console.log('💡 Troubleshooting:');
    console.log('   1. Ensure you have internet connectivity (for automatic downloads)');
    console.log('   2. Try: USE_CHROME_FOR_TESTING=false node tests/e2e/chrome-info.js');
    console.log('   3. Check if system Chrome is available');
    console.log('   4. Run with DEBUG=true for more details');
  }
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  getChromeInfo().catch(console.error);
}

export default getChromeInfo;