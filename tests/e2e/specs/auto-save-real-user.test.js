/**
 * Real User Auto-Save End-to-End Test
 * 
 * This test simulates exactly what a user would do:
 * 1. Open a vault
 * 2. Select a file
 * 3. Type some text
 * 4. Wait 2 seconds -> should auto-save
 * 5. Type more text 
 * 6. Click outside editor -> should save on blur
 */

import { test, expect } from '@playwright/test';
import { AutoSaveTestHelper } from '../helpers/auto-save-helper.js';
import path from 'path';
import fs from 'fs';

test.describe('Auto-Save Real User Behavior', () => {
  let helper;
  
  test.beforeEach(async ({ page }) => {
    helper = new AutoSaveTestHelper(page);
    await helper.setup();
  });

  test.afterEach(async ({ page }) => {
    await helper.cleanup();
  });

  test('should auto-save after 2 seconds of inactivity like a real user', async ({ page }) => {
    // Step 1: Open vault and select file - REAL USER BEHAVIOR
    const testVault = path.join(process.cwd(), 'tests/fixtures/test-vault');
    const testFile = path.join(testVault, 'test-note.md');
    
    // Ensure test file exists
    await fs.promises.writeFile(testFile, '# Original Content\n\nThis is the original content.');
    
    // Open vault like user would
    await helper.openVault(testVault);
    await helper.selectFile('test-note.md');
    
    // Step 2: Wait for editor to be ready
    await page.waitForSelector('[data-testid="markdown-editor"]');
    const editor = page.locator('[data-testid="markdown-editor"]');
    
    // Step 3: User starts typing - this should trigger content change
    const newContent = '# Updated Content\n\nUser is typing this new content...';
    
    // Clear and type like a real user would
    await editor.click();
    await page.keyboard.press('Control+A'); // Select all
    await editor.fill(newContent);
    
    // Step 4: Track save operations with console logging
    const saveOperations = [];
    page.on('console', msg => {
      if (msg.text().includes('auto save') || msg.text().includes('AutoSave') || msg.text().includes('save completed')) {
        saveOperations.push({
          timestamp: Date.now(),
          message: msg.text()
        });
      }
    });
    
    // Step 5: Wait exactly 2.5 seconds (2s delay + buffer) - REAL USER WAIT TIME
    console.log('Waiting for auto-save after 2 seconds...');
    await page.waitForTimeout(2500);
    
    // Step 6: Verify file was actually saved to disk
    const savedContent = await fs.promises.readFile(testFile, 'utf-8');
    expect(savedContent).toBe(newContent);
    
    // Step 7: Verify save operation was logged
    expect(saveOperations.length).toBeGreaterThan(0);
    
    // Step 8: Test blur save - user clicks outside editor
    const nextContent = newContent + '\n\nAdded after auto-save.';
    await editor.fill(nextContent);
    
    // Click outside editor to trigger blur
    await page.locator('body').click({ position: { x: 50, y: 50 } });
    
    // Small delay for blur save
    await page.waitForTimeout(500);
    
    // Verify blur save worked
    const blurSavedContent = await fs.promises.readFile(testFile, 'utf-8');
    expect(blurSavedContent).toBe(nextContent);
    
    console.log('✅ Real user auto-save test completed successfully');
  });

  test('should show exact event flow with detailed logging', async ({ page }) => {
    // This test focuses on debugging the event chain
    const testVault = path.join(process.cwd(), 'tests/fixtures/test-vault');
    const testFile = path.join(testVault, 'debug-test.md');
    
    await fs.promises.writeFile(testFile, 'Initial content for debugging');
    
    // Capture ALL console messages for debugging
    const allLogs = [];
    page.on('console', msg => {
      allLogs.push({
        timestamp: Date.now(),
        type: msg.type(),
        text: msg.text()
      });
    });
    
    await helper.openVault(testVault);
    await helper.selectFile('debug-test.md');
    
    const editor = page.locator('[data-testid="markdown-editor"]');
    await editor.click();
    
    // Type and observe the exact event flow
    await editor.fill('New content for event flow debugging');
    
    // Wait and capture what happens
    await page.waitForTimeout(3000);
    
    // Print all logs for analysis
    console.log('\n=== COMPLETE EVENT FLOW LOG ===');
    allLogs.forEach(log => {
      console.log(`[${log.type}] ${log.text}`);
    });
    console.log('=== END EVENT FLOW LOG ===\n');
    
    // Verify save occurred
    const savedContent = await fs.promises.readFile(testFile, 'utf-8');
    expect(savedContent).toBe('New content for event flow debugging');
  });
});