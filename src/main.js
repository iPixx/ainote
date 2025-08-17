const { invoke } = window.__TAURI__.core;

// Import AppState for centralized state management
import AppState from './js/state.js';

// Initialize global application state
const appState = new AppState();

/**
 * Display result in the specified element with styling
 * @param {string} elementId - ID of the element to display results in
 * @param {string} message - Message to display
 * @param {boolean} isError - Whether this is an error message
 */
function showResult(elementId, message, isError = false) {
  const element = document.getElementById(elementId);
  const timestamp = new Date().toLocaleTimeString();
  const status = isError ? '❌' : '✅';
  const cssClass = isError ? 'error' : 'success';
  
  const resultDiv = document.createElement('div');
  resultDiv.className = `result ${cssClass}`;
  resultDiv.innerHTML = `<span class="timestamp">[${timestamp}]</span> ${status} ${message}`;
  
  element.appendChild(resultDiv);
  element.scrollTop = element.scrollHeight;
}

/**
 * Clear results from specified element
 * @param {string} elementId - ID of the element to clear
 */
function clearResults(elementId) {
  const element = document.getElementById(elementId);
  element.innerHTML = '';
}

// Vault Operations

/**
 * Test select_vault_folder command
 */
async function testSelectVault() {
  try {
    const result = await invoke('select_vault_folder');
    if (result) {
      appState.setVault(result);
      showResult('vault-results', `Vault selected: ${result}`);
    } else {
      showResult('vault-results', 'No vault selected (user cancelled)', true);
    }
  } catch (error) {
    showResult('vault-results', `Error selecting vault: ${error}`, true);
  }
}

/**
 * Test scan_vault_files command
 */
async function testScanVault() {
  const currentVault = appState.getState().currentVault;
  if (!currentVault) {
    showResult('vault-results', 'Please select a vault folder first', true);
    return;
  }
  
  try {
    const result = await invoke('scan_vault_files', { vaultPath: currentVault });
    const fileCount = result.filter(file => !file.is_dir).length;
    const dirCount = result.filter(file => file.is_dir).length;
    
    // Update state with scanned files
    appState.setFiles(result);
    
    showResult('vault-results', `Scanned vault: ${fileCount} files, ${dirCount} directories`);
    
    // Show first few files as examples
    if (result.length > 0) {
      const examples = result.slice(0, 3).map(file => 
        `${file.is_dir ? '📁' : '📄'} ${file.name}`
      ).join(', ');
      showResult('vault-results', `Examples: ${examples}${result.length > 3 ? '...' : ''}`);
    }
  } catch (error) {
    showResult('vault-results', `Error scanning vault: ${error}`, true);
  }
}

// File Operations

/**
 * Get the full path for a file (using selected vault or current directory)
 * @param {string} fileName - The file name
 * @returns {string} - Full file path
 */
function getFullPath(fileName) {
  const currentVault = appState.getState().currentVault;
  if (currentVault && !fileName.includes('/') && !fileName.includes('\\')) {
    return `${currentVault}/${fileName}`;
  }
  return fileName;
}

/**
 * Test create_file command
 */
async function testCreateFile() {
  const fileName = document.getElementById('file-path').value;
  if (!fileName) {
    showResult('file-results', 'Please enter a file name', true);
    return;
  }
  
  const fullPath = getFullPath(fileName);
  
  try {
    await invoke('create_file', { filePath: fullPath });
    showResult('file-results', `Created file: ${fullPath}`);
  } catch (error) {
    showResult('file-results', `Error creating file: ${error}`, true);
  }
}

/**
 * Test read_file command
 */
async function testReadFile() {
  const fileName = document.getElementById('file-path').value;
  if (!fileName) {
    showResult('file-results', 'Please enter a file name', true);
    return;
  }
  
  const fullPath = getFullPath(fileName);
  
  try {
    const content = await invoke('read_file', { filePath: fullPath });
    const preview = content.length > 100 ? content.substring(0, 100) + '...' : content;
    showResult('file-results', `Read file (${content.length} chars): ${preview}`);
  } catch (error) {
    showResult('file-results', `Error reading file: ${error}`, true);
  }
}

/**
 * Test write_file command
 */
async function testWriteFile() {
  const fileName = document.getElementById('file-path').value;
  const content = document.getElementById('file-content').value;
  
  if (!fileName) {
    showResult('file-results', 'Please enter a file name', true);
    return;
  }
  
  if (!content) {
    showResult('file-results', 'Please enter content to write', true);
    return;
  }
  
  const fullPath = getFullPath(fileName);
  
  try {
    await invoke('write_file', { filePath: fullPath, content: content });
    showResult('file-results', `Wrote ${content.length} characters to: ${fullPath}`);
  } catch (error) {
    showResult('file-results', `Error writing file: ${error}`, true);
  }
}

/**
 * Test delete_file command
 */
async function testDeleteFile() {
  const fileName = document.getElementById('file-path').value;
  if (!fileName) {
    showResult('file-results', 'Please enter a file name', true);
    return;
  }
  
  const fullPath = getFullPath(fileName);
  
  try {
    await invoke('delete_file', { filePath: fullPath });
    showResult('file-results', `Deleted file: ${fullPath}`);
  } catch (error) {
    showResult('file-results', `Error deleting file: ${error}`, true);
  }
}

/**
 * Test rename_file command
 */
async function testRenameFile() {
  const oldPath = document.getElementById('old-path').value;
  const newPath = document.getElementById('new-path').value;
  
  if (!oldPath || !newPath) {
    showResult('file-results', 'Please enter both old and new file names', true);
    return;
  }
  
  const fullOldPath = getFullPath(oldPath);
  const fullNewPath = getFullPath(newPath);
  
  try {
    await invoke('rename_file', { oldPath: fullOldPath, newPath: fullNewPath });
    showResult('file-results', `Renamed: ${fullOldPath} → ${fullNewPath}`);
  } catch (error) {
    showResult('file-results', `Error renaming file: ${error}`, true);
  }
}

// Complete Test Suite

/**
 * Run all backend tests in sequence
 */
async function runAllTests() {
  clearResults('all-results');
  showResult('all-results', 'Starting comprehensive backend validation...');
  
  const testFileName = `test-${Date.now()}.md`;
  const testContent = '# Test File\n\nThis is a test file created during validation.';
  const renamedFileName = `renamed-${Date.now()}.md`;
  
  let testsRun = 0;
  let testsPassed = 0;
  
  // Test 1: Select vault folder (if not already selected)
  const currentVault = appState.getState().currentVault;
  if (!currentVault) {
    showResult('all-results', 'Test 1: Please select a vault folder first', true);
    return;
  }
  
  testsRun++;
  showResult('all-results', `Test 1: Using vault: ${currentVault}`);
  testsPassed++;
  
  // Test 2: Scan vault files
  testsRun++;
  try {
    const scanResult = await invoke('scan_vault_files', { vaultPath: currentVault });
    appState.setFiles(scanResult);
    showResult('all-results', `Test 2: ✅ Scanned ${scanResult.length} items`);
    testsPassed++;
  } catch (error) {
    showResult('all-results', `Test 2: ❌ Scan failed: ${error}`, true);
  }
  
  // Test 3: Create file
  testsRun++;
  const testFilePath = getFullPath(testFileName);
  try {
    await invoke('create_file', { filePath: testFilePath });
    showResult('all-results', `Test 3: ✅ Created file: ${testFileName}`);
    testsPassed++;
  } catch (error) {
    showResult('all-results', `Test 3: ❌ Create failed: ${error}`, true);
  }
  
  // Test 4: Write file
  testsRun++;
  try {
    await invoke('write_file', { filePath: testFilePath, content: testContent });
    showResult('all-results', `Test 4: ✅ Wrote ${testContent.length} characters`);
    testsPassed++;
  } catch (error) {
    showResult('all-results', `Test 4: ❌ Write failed: ${error}`, true);
  }
  
  // Test 5: Read file
  testsRun++;
  try {
    const readContent = await invoke('read_file', { filePath: testFilePath });
    if (readContent === testContent) {
      showResult('all-results', `Test 5: ✅ Read content matches written content`);
      testsPassed++;
    } else {
      showResult('all-results', `Test 5: ❌ Content mismatch: expected ${testContent.length} chars, got ${readContent.length}`, true);
    }
  } catch (error) {
    showResult('all-results', `Test 5: ❌ Read failed: ${error}`, true);
  }
  
  // Test 6: Rename file
  testsRun++;
  const renamedFilePath = getFullPath(renamedFileName);
  try {
    await invoke('rename_file', { oldPath: testFilePath, newPath: renamedFilePath });
    showResult('all-results', `Test 6: ✅ Renamed ${testFileName} → ${renamedFileName}`);
    testsPassed++;
  } catch (error) {
    showResult('all-results', `Test 6: ❌ Rename failed: ${error}`, true);
  }
  
  // Test 7: Delete file
  testsRun++;
  try {
    await invoke('delete_file', { filePath: renamedFilePath });
    showResult('all-results', `Test 7: ✅ Deleted ${renamedFileName}`);
    testsPassed++;
  } catch (error) {
    showResult('all-results', `Test 7: ❌ Delete failed: ${error}`, true);
  }
  
  // Summary
  const successRate = ((testsPassed / testsRun) * 100).toFixed(1);
  if (testsPassed === testsRun) {
    showResult('all-results', `🎉 ALL TESTS PASSED! (${testsPassed}/${testsRun} - ${successRate}%)`);
  } else {
    showResult('all-results', `⚠️ Some tests failed: ${testsPassed}/${testsRun} passed (${successRate}%)`, true);
  }
}

// State management event listeners for demonstration
appState.addEventListener(AppState.EVENTS.VAULT_CHANGED, (data) => {
  console.log('State: Vault changed', data);
  if (data.vault) {
    showResult('vault-results', `State updated: Vault set to ${data.vault}`);
  }
});

appState.addEventListener(AppState.EVENTS.FILES_UPDATED, (data) => {
  console.log('State: Files updated', data);
  showResult('vault-results', `State updated: ${data.count} files in vault`);
});

appState.addEventListener(AppState.EVENTS.FILE_CHANGED, (data) => {
  console.log('State: Current file changed', data);
});

appState.addEventListener(AppState.EVENTS.VIEW_MODE_CHANGED, (data) => {
  console.log('State: View mode changed', data);
});

appState.addEventListener(AppState.EVENTS.DIRTY_STATE_CHANGED, (data) => {
  console.log('State: Dirty state changed', data);
});

// State management demonstration functions
function testToggleViewMode() {
  const newMode = appState.toggleViewMode();
  showResult('state-results', `View mode toggled to: ${newMode}`);
}

function testMarkDirty() {
  appState.markDirty(true);
  showResult('state-results', 'Marked state as dirty (unsaved changes)');
}

function testMarkClean() {
  appState.markDirty(false);
  showResult('state-results', 'Marked state as clean (no unsaved changes)');
}

function testShowState() {
  const state = appState.getState();
  showResult('state-results', `Current state: ${JSON.stringify(state, null, 2)}`);
}

// Make functions globally accessible for HTML onclick handlers
window.testSelectVault = testSelectVault;
window.testScanVault = testScanVault;
window.testCreateFile = testCreateFile;
window.testReadFile = testReadFile;
window.testWriteFile = testWriteFile;
window.testDeleteFile = testDeleteFile;
window.testRenameFile = testRenameFile;
window.runAllTests = runAllTests;

// State management demo functions
window.testToggleViewMode = testToggleViewMode;
window.testMarkDirty = testMarkDirty;
window.testMarkClean = testMarkClean;
window.testShowState = testShowState;

// Initialize the application
window.addEventListener('DOMContentLoaded', () => {
  showResult('vault-results', 'Backend validation interface ready');
  showResult('file-results', 'File operations ready - select a vault first for easier testing');
  showResult('state-results', 'State management system initialized');
  
  // Load persisted state on startup
  const currentState = appState.getState();
  if (currentState.currentVault) {
    showResult('vault-results', `Restored vault from localStorage: ${currentState.currentVault}`);
  }
  if (currentState.viewMode !== AppState.VIEW_MODES.EDITOR) {
    showResult('state-results', `Restored view mode: ${currentState.viewMode}`);
  }
  
  // Show initial state
  showResult('state-results', `Initial state: ${JSON.stringify(currentState, null, 2)}`);
});
