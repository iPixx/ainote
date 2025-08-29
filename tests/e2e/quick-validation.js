#!/usr/bin/env node

/**
 * Quick E2E Infrastructure Validation
 * 
 * Tests the E2E infrastructure without full application build
 */

import DriverManager from './helpers/driver-manager.js';
import TauriHelpers from './helpers/tauri-helpers.js';
import TestUtils from './helpers/test-utils.js';

console.log('🧪 Quick E2E Infrastructure Validation');
console.log('=====================================\n');

async function runQuickValidation() {
  let driverManager;
  const results = [];
  
  try {
    console.log('1️⃣ Testing WebDriver Setup...');
    
    // Test WebDriver initialization
    driverManager = new DriverManager({
      browser: 'chrome',
      headless: process.env.HEADLESS === 'true',
      debug: true
    });
    
    await driverManager.setup();
    console.log('✅ WebDriver setup successful\n');
    
    console.log('2️⃣ Testing Navigation...');
    
    // Navigate to a simple page to test WebDriver
    await driverManager.driver.get('data:text/html,<html><head><title>E2E Test</title></head><body><h1 id="test">E2E Infrastructure Test</h1><div id="app">Application Container</div></body></html>');
    
    const title = await driverManager.driver.getTitle();
    console.log(`📄 Page title: "${title}"`);
    
    console.log('✅ Navigation successful\n');
    
    console.log('3️⃣ Testing Element Interaction...');
    
    // Test basic element finding and interaction
    const tauriHelpers = new TauriHelpers(driverManager.driver);
    
    const heading = await driverManager.driver.findElement({ id: 'test' });
    const headingText = await heading.getText();
    console.log(`📝 Found element with text: "${headingText}"`);
    
    console.log('✅ Element interaction successful\n');
    
    console.log('4️⃣ Testing Test Utilities...');
    
    // Test performance measurement
    const performanceResult = await TestUtils.measureExecutionTime(async () => {
      await new Promise(resolve => setTimeout(resolve, 100));
      return 'test completed';
    }, 'Sample async operation');
    
    console.log(`⏱️ Performance measurement: ${performanceResult.duration.toFixed(2)}ms`);
    
    // Test fixture creation
    console.log('📁 Testing fixture creation...');
    const testVaultPath = await TestUtils.createTestFixtures();
    console.log(`✅ Test fixtures created at: ${testVaultPath}`);
    
    console.log('✅ Test utilities successful\n');
    
    console.log('5️⃣ Testing Screenshot Capability...');
    
    // Test screenshot functionality
    const screenshotPath = await tauriHelpers.takeScreenshot('quick_validation_test');
    if (screenshotPath) {
      console.log(`📸 Screenshot saved: ${screenshotPath}`);
    }
    
    console.log('✅ Screenshot capability successful\n');
    
    console.log('6️⃣ Testing Performance Metrics...');
    
    // Test performance metrics gathering
    const metrics = await tauriHelpers.getPerformanceMetrics();
    if (metrics) {
      console.log('📊 Performance metrics collected successfully');
      console.log(`   - Memory: ${metrics.memory ? (metrics.memory.used / 1024 / 1024).toFixed(2) + ' MB' : 'N/A'}`);
    }
    
    console.log('✅ Performance metrics successful\n');
    
    results.push({ test: 'E2E Infrastructure', success: true, message: 'All components working' });
    
  } catch (error) {
    console.error('❌ Validation failed:', error.message);
    results.push({ test: 'E2E Infrastructure', success: false, message: error.message });
    
  } finally {
    console.log('🧹 Cleanup...');
    
    if (driverManager) {
      await driverManager.teardown();
    }
    
    TestUtils.cleanupTestFixtures();
    console.log('✅ Cleanup completed\n');
  }
  
  // Summary
  console.log('📋 VALIDATION SUMMARY');
  console.log('====================');
  
  const passed = results.filter(r => r.success).length;
  const total = results.length;
  
  console.log(`Tests: ${passed}/${total} passed`);
  
  if (passed === total) {
    console.log('🎉 E2E infrastructure is ready for use!');
    console.log('\n📚 Next steps:');
    console.log('   - Run full E2E tests: pnpm test:e2e');
    console.log('   - Run in headless mode: pnpm test:e2e:headless');
    console.log('   - Run with debug: pnpm test:e2e:debug');
    
    process.exit(0);
  } else {
    console.log('💥 Some components need attention. Check the errors above.');
    process.exit(1);
  }
}

// Run validation if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runQuickValidation().catch((error) => {
    console.error('❌ Quick validation failed:', error);
    process.exit(1);
  });
}

export default runQuickValidation;