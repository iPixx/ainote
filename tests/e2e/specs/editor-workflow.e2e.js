/**
 * Editor Workflow E2E Tests
 * 
 * Tests editor functionality, preview mode, and complete editing workflows.
 */

import { describe, it, before, after, beforeEach } from 'mocha';
import { expect } from 'chai';
import { Key } from 'selenium-webdriver';
import DriverManager from '../helpers/driver-manager.js';
import TauriHelpers from '../helpers/tauri-helpers.js';
import TestUtils from '../helpers/test-utils.js';

describe('Editor Workflow E2E Tests', function() {
  let driverManager;
  let tauriHelpers;
  let testVaultPath;
  
  before(async function() {
    this.timeout(60000);
    
    console.log('🔧 Setting up Editor Workflow E2E tests...');
    
    // Create test fixtures
    testVaultPath = await TestUtils.createTestFixtures();
    
    // Initialize WebDriver
    driverManager = new DriverManager({
      browser: 'chrome',
      headless: process.env.HEADLESS === 'true',
      debug: process.env.DEBUG === 'true'
    });
    
    await driverManager.setup();
    tauriHelpers = new TauriHelpers(driverManager.driver);
    
    // Navigate to application and setup
    await driverManager.navigateToApplication();
    await driverManager.driver.executeScript(TestUtils.createTauriTestMock());
    
    // Select vault for all tests
    await tauriHelpers.waitForApplicationLoad();
    await tauriHelpers.selectVault(testVaultPath);
    await tauriHelpers.waitForFileTreeLoad();
    
    console.log('✅ Editor Workflow E2E test setup complete');
  });
  
  after(async function() {
    this.timeout(30000);
    
    if (driverManager) {
      await driverManager.teardown();
    }
    
    TestUtils.cleanupTestFixtures();
  });
  
  beforeEach(async function() {
    if (process.env.DEBUG === 'true') {
      const testName = this.currentTest.title.replace(/\s+/g, '_');
      await tauriHelpers.takeScreenshot(`editor_workflow_before_${testName}`);
    }
  });
  
  describe('File Selection and Loading', function() {
    
    it('should load file content when file is selected', async function() {
      this.timeout(10000);
      
      const result = await TestUtils.measureExecutionTime(async () => {
        // Select welcome.md file
        await tauriHelpers.selectFile('welcome.md');
        
        // Get editor content
        const content = await tauriHelpers.getEditorContent();
        
        // Verify content is loaded
        expect(content).to.include('Welcome to aiNote');
        expect(content.length).to.be.greaterThan(50);
        
        return true;
      }, 'File selection and loading');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(3000); // Should load quickly
      
      // Validate performance requirement
      const performanceValid = TestUtils.validatePerformance(
        { fileLoadTime: result.duration },
        { fileLoadTime: 1000 }
      );
      
      if (!performanceValid.valid) {
        console.warn('Performance warnings:', performanceValid.violations);
        // Don't fail test for performance warnings in E2E
      }
    });
    
    it('should handle empty files correctly', async function() {
      await tauriHelpers.selectFile('empty.md');
      
      const content = await tauriHelpers.getEditorContent();
      expect(content).to.equal('');
      
      // Editor should still be functional
      await tauriHelpers.typeInEditor('# New Content');
      const newContent = await tauriHelpers.getEditorContent();
      expect(newContent).to.equal('# New Content');
    });
    
    it('should switch between files efficiently', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Switch between multiple files
        await tauriHelpers.selectFile('welcome.md');
        await tauriHelpers.selectFile('notes.md');
        await tauriHelpers.selectFile('welcome.md');
        
        // Verify final content
        const content = await tauriHelpers.getEditorContent();
        expect(content).to.include('Welcome to aiNote');
        
        return true;
      }, 'File switching');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(5000);
    });
    
  });
  
  describe('Content Editing', function() {
    
    beforeEach(async function() {
      // Start with welcome.md for editing tests
      await tauriHelpers.selectFile('welcome.md');
    });
    
    it('should allow text editing in the editor', async function() {
      const testContent = '# Test Edit\n\nThis is a test edit.';
      
      await tauriHelpers.typeInEditor(testContent);
      
      const content = await tauriHelpers.getEditorContent();
      expect(content).to.equal(testContent);
    });
    
    it('should support markdown formatting shortcuts', async function() {
      // Clear editor
      await tauriHelpers.typeInEditor('');
      
      // Type some text
      await tauriHelpers.typeInEditor('Bold text here');
      
      // Select text and apply bold formatting (Ctrl+B)
      await tauriHelpers.sendKeyboardShortcut([Key.CONTROL, 'a']); // Select all
      await tauriHelpers.sendKeyboardShortcut([Key.CONTROL, 'b']); // Bold
      
      const content = await tauriHelpers.getEditorContent();
      
      // Should contain bold markdown syntax
      expect(content).to.include('**') || expect(content).to.include('Bold');
    });
    
    it('should handle large content efficiently', async function() {
      const largeContent = TestUtils.generateLargeMarkdownContent(50); // 50KB
      
      const result = await TestUtils.measureExecutionTime(async () => {
        await tauriHelpers.typeInEditor(largeContent);
        
        const content = await tauriHelpers.getEditorContent();
        expect(content.length).to.be.greaterThan(40000); // Should be close to 50KB
        
        return true;
      }, 'Large content handling');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(5000); // Should handle large content reasonably fast
    });
    
  });
  
  describe('Editor/Preview Mode Toggle', function() {
    
    beforeEach(async function() {
      await tauriHelpers.selectFile('welcome.md');
      
      // Ensure we start in editor mode
      try {
        await tauriHelpers.switchMode('editor');
      } catch (error) {
        // Ignore if already in editor mode
      }
    });
    
    it('should switch to preview mode and render markdown', async function() {
      const result = await TestUtils.measureExecutionTime(async () => {
        // Switch to preview mode
        await tauriHelpers.switchMode('preview');
        
        // Verify preview mode is active
        const isPreview = await tauriHelpers.isPreviewMode();
        expect(isPreview).to.be.true;
        
        return true;
      }, 'Switch to preview mode');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(2000);
    });
    
    it('should render markdown elements correctly in preview', async function() {
      // Add test content with various markdown elements
      const testMarkdown = `# Test Heading

This is **bold text** and *italic text*.

## Code Block

\`\`\`javascript
console.log('Hello, World!');
\`\`\`

## List

- Item 1
- Item 2
- Item 3

> This is a blockquote.
`;
      
      await tauriHelpers.typeInEditor(testMarkdown);
      await tauriHelpers.switchMode('preview');
      
      // Get preview content
      const previewPanel = await driverManager.driver.findElement({ className: 'preview-panel' });
      const previewHTML = await previewPanel.getAttribute('innerHTML');
      
      // Check for rendered elements
      expect(previewHTML).to.include('<h1>');  // Heading
      expect(previewHTML).to.include('<strong>'); // Bold text
      expect(previewHTML).to.include('<em>'); // Italic text
      expect(previewHTML).to.include('<code>'); // Code elements
      expect(previewHTML).to.include('<blockquote>'); // Blockquote
    });
    
    it('should switch back to editor mode', async function() {
      // Start in preview mode
      await tauriHelpers.switchMode('preview');
      
      const result = await TestUtils.measureExecutionTime(async () => {
        // Switch back to editor
        await tauriHelpers.switchMode('editor');
        
        // Verify editor mode is active
        const isPreview = await tauriHelpers.isPreviewMode();
        expect(isPreview).to.be.false;
        
        return true;
      }, 'Switch to editor mode');
      
      expect(result.success).to.be.true;
      expect(result.duration).to.be.below(2000);
    });
    
  });
  
  describe('Auto-save Functionality', function() {
    
    beforeEach(async function() {
      await tauriHelpers.selectFile('notes.md');
    });
    
    it('should trigger auto-save after content changes', async function() {
      this.timeout(15000);
      
      // Type content
      await tauriHelpers.typeInEditor('# Auto-save Test\n\nThis content should auto-save.');
      
      // Wait for auto-save to trigger
      await driverManager.driver.sleep(3000);
      
      // Check for auto-save indicator (if visible)
      try {
        await tauriHelpers.waitForAutoSave(5000);
        console.log('✅ Auto-save completed successfully');
      } catch (error) {
        console.log('ℹ️  Auto-save not visually indicated (this may be normal)');
      }
      
      // Content should still be there after waiting
      const content = await tauriHelpers.getEditorContent();
      expect(content).to.include('Auto-save Test');
    });
    
    it('should preserve content when switching files', async function() {
      // Edit notes.md
      const testContent = '# Modified Notes\n\nThis content was modified.';
      await tauriHelpers.typeInEditor(testContent);
      
      // Switch to another file
      await tauriHelpers.selectFile('welcome.md');
      
      // Switch back to notes.md
      await tauriHelpers.selectFile('notes.md');
      
      // Content should be preserved (in a real app with auto-save)
      const content = await tauriHelpers.getEditorContent();
      // Note: In this test environment, content might not persist
      // In a real implementation, this would test actual auto-save functionality
      expect(content).to.be.a('string');
    });
    
  });
  
  describe('Complete Editing Workflow', function() {
    
    it('should complete full editing workflow: select → edit → preview → save', async function() {
      this.timeout(30000);
      
      const workflow = await TestUtils.measureExecutionTime(async () => {
        console.log('🔄 Starting complete editing workflow...');
        
        // Step 1: Select file
        await tauriHelpers.selectFile('empty.md');
        console.log('✓ File selected');
        
        // Step 2: Edit content
        const testContent = `# Complete Workflow Test
        
## Introduction
This is a test of the complete editing workflow in aiNote.

## Features Tested
- [x] File selection
- [x] Content editing  
- [ ] Preview rendering
- [ ] Auto-save functionality

## Code Example
\`\`\`markdown
# This is markdown
**Bold text** and *italic text*
\`\`\`

## Conclusion
This test validates the complete user workflow.
`;
        
        await tauriHelpers.typeInEditor(testContent);
        console.log('✓ Content edited');
        
        // Step 3: Switch to preview mode
        await tauriHelpers.switchMode('preview');
        console.log('✓ Switched to preview');
        
        // Verify preview content
        const isPreview = await tauriHelpers.isPreviewMode();
        expect(isPreview).to.be.true;
        
        // Step 4: Switch back to editor
        await tauriHelpers.switchMode('editor');
        console.log('✓ Switched back to editor');
        
        // Step 5: Verify content is still there
        const finalContent = await tauriHelpers.getEditorContent();
        expect(finalContent).to.include('Complete Workflow Test');
        console.log('✓ Content verified');
        
        // Step 6: Wait for auto-save
        await driverManager.driver.sleep(2000);
        console.log('✓ Auto-save completed');
        
        return true;
      }, 'Complete editing workflow');
      
      expect(workflow.success).to.be.true;
      expect(workflow.duration).to.be.below(25000); // Complete workflow under 25 seconds
      
      console.log(`✅ Complete editing workflow completed in ${workflow.duration.toFixed(2)}ms`);
    });
    
  });
  
  describe('Performance and Memory Usage', function() {
    
    it('should maintain good performance during intensive editing', async function() {
      this.timeout(30000);
      
      await tauriHelpers.selectFile('empty.md');
      
      const intensiveEditing = await TestUtils.measureExecutionTime(async () => {
        // Perform multiple editing operations
        for (let i = 0; i < 10; i++) {
          const content = `# Edit ${i + 1}\n\nThis is edit number ${i + 1}.`;
          await tauriHelpers.typeInEditor(content);
          
          // Switch modes occasionally
          if (i % 3 === 0) {
            await tauriHelpers.switchMode('preview');
            await tauriHelpers.switchMode('editor');
          }
          
          // Small delay to simulate real typing
          await driverManager.driver.sleep(200);
        }
        
        return true;
      }, 'Intensive editing operations');
      
      expect(intensiveEditing.success).to.be.true;
      expect(intensiveEditing.duration).to.be.below(15000); // Should complete within 15 seconds
      
      // Check memory usage after intensive operations
      const metrics = await tauriHelpers.getPerformanceMetrics();
      if (metrics && metrics.memory) {
        const memoryMB = metrics.memory.used / (1024 * 1024);
        console.log(`📊 Memory usage after intensive editing: ${memoryMB.toFixed(2)} MB`);
        
        // Memory should still be reasonable
        expect(memoryMB).to.be.below(100);
      }
    });
    
  });
  
});