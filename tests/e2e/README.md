# aiNote End-to-End Testing

This directory contains end-to-end (E2E) tests for aiNote using Selenium WebDriver with cross-platform compatibility.

## Quick Start

```bash
# Install dependencies (if not already done)
pnpm install

# Build the Tauri application for testing
pnpm tauri build

# Run E2E tests
pnpm test:e2e

# Run E2E tests in headless mode (for CI)
pnpm test:e2e:headless
```

## Architecture

### Testing Framework
- **Selenium WebDriver 4.35.0**: Cross-platform browser automation with Selenium Manager
- **Mocha**: JavaScript test framework
- **Chai**: Assertion library
- **Chrome for Testing**: Optimized Chrome build for automation (auto-managed by Selenium Manager)
- **ChromeDriver**: Auto-managed by Selenium Manager for version compatibility

### Platform Compatibility
- ✅ **macOS**: Uses Chrome WebDriver (this implementation)
- ✅ **Windows**: Can use tauri-driver + Edge WebDriver
- ✅ **Linux**: Can use tauri-driver + WebKit WebDriver

### Directory Structure

```
tests/e2e/
├── README.md                    # This documentation
├── config/
│   ├── mocha.config.js         # Mocha test configuration
│   └── webdriver.config.js     # WebDriver setup and configuration
├── fixtures/
│   ├── test-vault/             # Sample vault for testing
│   └── test-files.json         # Test data files
├── helpers/
│   ├── driver-manager.js       # WebDriver lifecycle management
│   ├── tauri-helpers.js        # Tauri-specific testing utilities
│   └── test-utils.js           # Common test utilities
├── specs/
│   ├── vault-operations.e2e.js # Vault selection and management tests
│   ├── file-operations.e2e.js  # File CRUD operations tests
│   ├── editor-workflow.e2e.js  # Editor/preview functionality tests
│   └── app-lifecycle.e2e.js    # Window state and app lifecycle tests
└── run-e2e-tests.js            # Test runner script
```

## Testing Strategy

### Approach for macOS (Current Implementation)

Since tauri-driver doesn't support macOS desktop applications, this implementation uses:

1. **Selenium WebDriver** with Chrome to test the built Tauri application
2. **Application Process Management** to start/stop the Tauri app during tests
3. **Simulated Tauri API Testing** through the web interface
4. **Cross-Platform Test Design** that can be adapted for Linux/Windows

### Test Categories

#### 1. Vault Operations (`vault-operations.e2e.js`)
- Vault selection and initialization
- Vault switching and validation
- File tree loading and display

#### 2. File Operations (`file-operations.e2e.js`)
- File creation, reading, updating, deletion
- File tree navigation and interaction
- File persistence and state management

#### 3. Editor Workflow (`editor-workflow.e2e.js`)
- Editor/preview mode switching
- Content editing and real-time preview
- Auto-save functionality
- Markdown rendering validation

#### 4. Application Lifecycle (`app-lifecycle.e2e.js`)
- Window state persistence
- Application startup and shutdown
- Error handling and recovery

## Configuration

### Chrome for Testing Integration

aiNote's E2E tests are optimized for **Chrome for Testing** (CfT), which provides:

- **Automatic Management**: Selenium Manager handles Chrome and ChromeDriver versions
- **Testing Optimized**: Purpose-built for automation scenarios
- **Version Consistency**: Eliminates version mismatch issues
- **Isolation**: Doesn't interfere with your regular Chrome installation
- **Performance**: Optimized startup and reduced resource usage

### WebDriver Configuration

The E2E tests use Chrome for Testing by default for optimal compatibility:

```javascript
// webdriver.config.js
const chromeOptions = {
  'goog:chromeOptions': {
    args: [
      '--disable-dev-shm-usage',
      '--no-sandbox',
      '--disable-gpu',
      // Headless mode for CI
      process.env.HEADLESS === 'true' ? '--headless' : ''
    ].filter(Boolean)
  }
};
```

### Mocha Configuration

Test execution settings optimized for aiNote's performance requirements:

```javascript
// mocha.config.js
module.exports = {
  timeout: 30000,        // 30 seconds per test
  reporter: 'spec',      // Detailed test output
  recursive: true,       // Find tests in subdirectories
  exit: true            // Force exit after tests
};
```

## Writing E2E Tests

### Basic Test Structure

```javascript
import { expect } from 'chai';
import { DriverManager } from '../helpers/driver-manager.js';
import { TauriHelpers } from '../helpers/tauri-helpers.js';

describe('Feature Test Suite', () => {
  let driverManager;
  let tauriHelpers;

  before(async () => {
    driverManager = new DriverManager();
    await driverManager.setup();
    tauriHelpers = new TauriHelpers(driverManager.driver);
  });

  after(async () => {
    await driverManager.teardown();
  });

  it('should test specific functionality', async () => {
    // Test implementation
    const result = await tauriHelpers.selectVault('/path/to/test/vault');
    expect(result).to.be.true;
  });
});
```

### Testing Tauri Application

```javascript
// Start the Tauri application
await tauriHelpers.startApplication();

// Interact with application elements
const fileTree = await driver.findElement(By.id('file-tree'));
const files = await fileTree.findElements(By.className('file-item'));

// Validate application state
expect(files).to.have.length.greaterThan(0);

// Clean up
await tauriHelpers.stopApplication();
```

## Performance Requirements

E2E tests must respect aiNote's performance targets:

- **Test Execution Time**: Complete suite should finish in <2 minutes
- **Individual Test Timeout**: 30 seconds maximum
- **Memory Usage**: Monitor application memory during testing
- **Application Startup**: <5 seconds for test application launch

## Cross-Platform Notes

### Platform-Specific Drivers

1. **macOS** (Current): Chrome WebDriver + Selenium
2. **Linux**: tauri-driver + WebKit WebDriver (future)
3. **Windows**: tauri-driver + Edge WebDriver (future)

### Adaptation Instructions

To adapt for Linux/Windows with tauri-driver:

1. Replace Chrome WebDriver setup with tauri-driver configuration
2. Update `driver-manager.js` to use tauri-driver binary
3. Modify test assertions for native application testing

## Limitations and Workarounds

### macOS-Specific Limitations

1. **No Native WebDriver**: Cannot directly test the native Tauri application
2. **Chrome WebDriver Testing**: Tests run against the web interface in Chrome
3. **Limited Native API Testing**: Some Tauri-specific features may not be fully testable

### Workarounds Implemented

1. **Application Process Management**: Start/stop Tauri app for testing
2. **API Mocking**: Mock Tauri APIs for comprehensive testing
3. **Cross-Platform Design**: Tests designed to work with future tauri-driver integration

## Troubleshooting

### Common Issues

1. **ChromeDriver Version Mismatch**: Update ChromeDriver to match Chrome version
2. **Application Not Starting**: Check Tauri build and executable path
3. **Test Timeouts**: Increase timeout values in mocha.config.js
4. **Port Conflicts**: Ensure no other applications are using test ports

### Debug Mode

Run tests with additional debugging:

```bash
DEBUG=true pnpm test:e2e
```

### Platform-Specific Setup

#### macOS
```bash
# Chrome for Testing is automatically managed by Selenium Manager
# No manual installation required!

# Optional: Set custom Chrome for Testing path
export CHROME_FOR_TESTING_PATH="/path/to/chrome-for-testing"

# Optional: Use system Chrome instead
export USE_CHROME_FOR_TESTING=false
```

#### Linux (Future)
```bash
# Install WebKit WebDriver
sudo apt-get install webkit2gtk-driver  # Ubuntu/Debian
```

#### Windows (Future)
```bash
# Install Edge WebDriver
# Download from Microsoft Edge WebDriver page
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
        with:
          node-version: '18'
          cache: 'pnpm'
      
      - run: pnpm install
      - run: pnpm tauri build
      - run: HEADLESS=true pnpm test:e2e
```

## Future Enhancements

1. **Visual Testing**: Screenshot comparison for UI regression testing
2. **Performance Monitoring**: Automated performance regression detection
3. **Mobile Testing**: Appium integration for iOS/Android testing
4. **Native Driver Support**: Full tauri-driver integration when macOS support arrives

---

**Last Updated**: Issue #164 Implementation  
**Platform**: Cross-platform with macOS primary support  
**Status**: Ready for core workflow testing