# aiNote Testing Infrastructure

This directory contains the comprehensive testing infrastructure for aiNote, implemented with Vitest to support the vanilla JavaScript architecture and local-first principles.

## Quick Start

```bash
# Install dependencies (if not already done)
pnpm install

# Run all tests
pnpm test

# Run tests with UI
pnpm test:ui

# Run tests in watch mode
pnpm test:watch

# Run tests with coverage
pnpm test:coverage
```

## Project Structure

```
tests/
├── README.md                 # This documentation
├── setup.js                  # Global test setup and Tauri mocks
├── __mocks__/
│   └── tauri-mocks.js       # Advanced Tauri API mocking utilities
├── unit/
│   ├── smoke-test.test.js   # Infrastructure validation tests
│   ├── content-change-detector.test.js # Component performance tests
│   └── markdown-parser.test.js # Parser functionality tests
└── integration/             # (Future) Integration tests
```

## Testing Architecture

### 1. Test Environment

- **Framework**: Vitest v1.0+ with jsdom environment
- **Module System**: ES6+ modules with dynamic imports
- **Browser APIs**: Complete jsdom environment with mocked APIs
- **Performance**: Built-in performance testing utilities

### 2. Tauri Mocking System

aiNote includes a sophisticated Tauri API mocking system that simulates all Tauri commands used in the application:

```javascript
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// In your test
const tauriMocks = setupTauriMocks();
```

**Available Mock Commands:**
- `select_vault` - Vault selection simulation
- `scan_vault_files` - File system scanning
- `read_file`, `write_file` - File operations
- `load_app_state`, `save_window_state` - State management
- `run_embedding_benchmarks` - Performance monitoring
- All window management operations

### 3. Test Categories

#### Smoke Tests (`smoke-test.test.js`)
Validates the complete testing infrastructure:
- Vitest environment setup
- Tauri API mocking
- ES6 module support
- DOM manipulation
- Mock cleanup and reset
- aiNote-specific command mocking

#### Unit Tests
- **Component Tests**: Individual JavaScript class/module testing
- **Performance Tests**: Validates aiNote's performance requirements
- **Utility Tests**: Helper functions and utilities

#### Integration Tests (Future)
- Cross-component interaction testing
- End-to-end workflows
- State management integration

## Writing Tests

### Basic Test Structure

```javascript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

describe('YourComponent', () => {
  let tauriMocks;
  
  beforeEach(() => {
    tauriMocks = setupTauriMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should do something', () => {
    // Your test here
    expect(true).toBe(true);
  });
});
```

### Testing Tauri Commands

```javascript
it('should handle vault operations', async () => {
  const { invoke } = window.__TAURI__.core;
  
  // Mock specific behavior
  invoke.mockResolvedValueOnce([
    { name: 'test.md', path: '/vault/test.md', is_dir: false }
  ]);
  
  // Test your component
  const files = await invoke('scan_vault_files', { vaultPath: '/vault' });
  expect(files).toHaveLength(1);
  expect(files[0].name).toBe('test.md');
});
```

### Performance Testing

```javascript
it('should meet performance requirements', () => {
  const startTime = performance.now();
  
  // Your performance-critical code
  const result = performExpensiveOperation();
  
  const duration = performance.now() - startTime;
  expect(duration).toBeLessThan(100); // Must complete in <100ms
});
```

## Configuration

### Vitest Configuration (`vitest.config.js`)

Key configuration features:
- **Environment**: jsdom for DOM testing
- **Module Resolution**: ES6+ with alias support
- **Coverage**: v8 provider with comprehensive reporting
- **Setup Files**: Automatic Tauri mock loading
- **Performance**: Optimized for aiNote's lightweight architecture

### Coverage Thresholds

```javascript
coverage: {
  thresholds: {
    global: {
      branches: 70,
      functions: 70,
      lines: 70,
      statements: 70
    }
  }
}
```

## Available Test Utilities

### Mock Creation

```javascript
import { createMockElement, simulateEvent, waitForNextTick } from '../setup.js';

// Create DOM elements for testing
const button = createMockElement('button', { 
  id: 'test-btn', 
  textContent: 'Click me' 
});

// Simulate user events
simulateEvent(button, 'click');

// Async testing helpers
await waitForNextTick();
```

### Tauri Mock Helpers

```javascript
import { createTauriInvokeMock, createWindowMock } from '../__mocks__/tauri-mocks.js';

// Custom invoke mock
const customInvoke = createTauriInvokeMock();
customInvoke.mockImplementation((command) => {
  if (command === 'my_custom_command') {
    return Promise.resolve('custom result');
  }
});
```

## Performance Requirements

aiNote has strict performance requirements that are validated through tests:

### Component Performance
- **Content Extraction**: <10ms per operation
- **Debouncing**: ≤2 requests per second maximum
- **Memory Usage**: <5MB total component memory
- **Document Parsing**: <100ms for typical documents

### Test Performance
- **Unit Tests**: Complete suite should run in <5 seconds
- **Individual Tests**: <100ms timeout for most tests
- **Setup/Teardown**: <10ms per test for mock setup

## Debugging Tests

### Verbose Output
```bash
pnpm test --reporter=verbose
```

### Debug Specific Test
```bash
pnpm test --run tests/unit/your-test.test.js
```

### Coverage Analysis
```bash
pnpm test:coverage
# Open coverage/index.html for detailed report
```

### Test UI
```bash
pnpm test:ui
# Opens browser-based test interface
```

## Integration with aiNote Architecture

### Local-First Compliance
- ✅ **No External Dependencies**: All testing runs locally
- ✅ **No Network Calls**: Mocked Tauri APIs prevent external requests
- ✅ **Lightweight**: Test infrastructure <10MB total
- ✅ **Performance Focused**: Tests validate aiNote's performance targets

### Vanilla JavaScript Support
- ✅ **ES6+ Modules**: Full support for aiNote's module system
- ✅ **No Framework Dependencies**: Pure JavaScript testing
- ✅ **DOM Testing**: jsdom provides complete browser environment
- ✅ **Tauri Integration**: Comprehensive mocking of Tauri APIs

## Continuous Integration (Future)

The testing infrastructure is designed to support CI/CD:

```yaml
# Example GitHub Actions configuration
- name: Run Tests
  run: |
    pnpm install
    pnpm test --run
    pnpm test:coverage
```

## Troubleshooting

### Common Issues

1. **Import Errors**: Ensure all imports use correct paths and file extensions
2. **Tauri Mocks Not Working**: Verify `setupTauriMocks()` is called in `beforeEach`
3. **Performance Test Failures**: Check system load during test execution
4. **Memory Limit Exceeded**: Adjust test data size for memory tests

### Getting Help

1. Check the smoke test: `pnpm test tests/unit/smoke-test.test.js`
2. Verify Vitest configuration: `npx vitest --version`
3. Review test setup in `tests/setup.js`
4. Examine mock implementations in `tests/__mocks__/`

## Future Enhancements

- **E2E Testing**: tauri-driver integration for complete application testing
- **Visual Testing**: Screenshot comparison for UI components
- **Load Testing**: Stress testing with large vaults and documents
- **Cross-Platform Testing**: Automated testing on Windows, macOS, and Linux

---

**Last Updated**: Issue #162 Implementation  
**Next Steps**: Issues #163 (Unit Tests) and #164 (E2E Testing)