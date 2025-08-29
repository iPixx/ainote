# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Documentation

**ALWAYS READ THESE FIRST:**
- [README.md](README.md) - Complete project overview, architecture, and development roadmap
- [CONTRIBUTING.md](CONTRIBUTING.md) - Detailed development guidelines, technical requirements, and local-first principles
- [tests/README.md](tests/README.md) - Testing infrastructure documentation and usage

## Project Overview

aiNote is a **local-first, AI-powered markdown note-taking application** built with Tauri v2. The project follows strict lightweight design principles to maximize resources for AI inference while maintaining complete privacy.

**Current Status:** Phase 1 implementation with testing infrastructure
**Next Phase:** AI-powered note suggestions via Ollama integration

## Development Commands

### Essential Commands
- `pnpm tauri dev` - Start development server with hot reload
- `pnpm tauri build` - Build production application
- `cargo check --manifest-path src-tauri/Cargo.toml` - Check Rust code compilation
- `cargo test --manifest-path src-tauri/Cargo.toml` - Run Rust tests
- `cargo clippy --manifest-path src-tauri/Cargo.toml` - Rust linting

### Testing Commands (ALWAYS RUN BEFORE COMMITTING)
- `pnpm test` - Run complete frontend test suite (Vitest)
- `pnpm test:ui` - Run tests with browser UI
- `pnpm test:watch` - Run tests in watch mode during development
- `pnpm test:coverage` - Generate coverage report
- `pnpm test:e2e` - Run end-to-end tests with browser UI
- `pnpm test:e2e:headless` - Run E2E tests in headless mode (for CI)
- `pnpm test:all` - Run complete test suite (unit + E2E)
- `pnpm test tests/unit/smoke-test.test.js --run` - Validate testing infrastructure

### Performance Monitoring
- Monitor memory usage during development (target: <100MB)
- Test with Ollama running to simulate AI resource usage
- Profile file operations (target: <50ms for typical notes)

## Project Structure & Tools

### Current Directory Structure
```
ainote/
├── README.md              # Complete project documentation
├── CONTRIBUTING.md        # Development guidelines & principles
├── CLAUDE.md             # This file - Claude Code guidance
├── package.json          # Frontend dependencies + testing
├── vitest.config.js      # Vitest testing configuration
├── src/                  # Frontend (Vanilla JavaScript)
│   ├── index.html       # Main HTML entry point
│   ├── main.js          # Application entry point
│   ├── styles.css       # Native CSS styling
│   ├── js/              # JavaScript modules (ES6+)
│   │   ├── components/  # UI components (FileTree, EditorPanel, etc.)
│   │   ├── services/    # Business logic (VaultManager, AutoSave, etc.)
│   │   ├── utils/       # Utility functions (MarkdownParser, etc.)
│   │   ├── state.js     # Global application state
│   │   └── layout-manager.js # Layout and responsive management
│   └── assets/          # Static assets (images, icons)
├── tests/               # Frontend testing infrastructure
│   ├── README.md       # Testing documentation
│   ├── setup.js        # Global test setup and mocks
│   ├── __mocks__/      # Mock utilities
│   │   └── tauri-mocks.js # Comprehensive Tauri API mocks
│   ├── unit/           # Unit tests
│   │   ├── smoke-test.test.js # Infrastructure validation
│   │   └── *.test.js   # Component and utility tests
│   ├── integration/    # Integration tests
│   └── e2e/            # End-to-end tests
│       ├── README.md   # E2E testing documentation
│       ├── config/     # E2E test configuration
│       ├── helpers/    # E2E testing utilities
│       ├── specs/      # E2E test specifications
│       ├── fixtures/   # Test data and sample vaults
│       └── run-e2e-tests.js # E2E test runner
└── src-tauri/          # Backend (Rust)
    ├── src/
    │   ├── main.rs     # Application entry point
    │   ├── lib.rs      # Tauri commands registration
    │   ├── commands/   # Tauri command implementations
    │   ├── vector_db/  # AI vector database system
    │   └── *.rs        # Core business logic modules
    ├── tests/          # Rust integration tests
    ├── tauri.conf.json # Tauri v2 configuration
    └── Cargo.toml      # Rust dependencies
```

### Technology Stack & Tools
- **Framework:** Tauri v2 (cross-platform desktop)
- **Frontend:** Vanilla JavaScript (ES6+), HTML5, CSS3
- **Backend:** Rust with minimal external crates
- **Package Manager:** pnpm (fast, efficient)
- **Testing:** Vitest with jsdom environment, Tauri API mocking
- **Development:** Hot reload via Tauri dev server
- **Future AI:** Ollama integration (Phase 2+)

## CRITICAL DEVELOPMENT PRINCIPLES

**⚠️ ALWAYS ENFORCE THESE RULES:**

### Local-First & Lightweight Requirements
- **NO external dependencies** without explicit approval in CONTRIBUTING.md
- **NO frontend frameworks** (React, Vue, Angular, Svelte) - use vanilla JS only
- **NO heavy libraries** (jQuery, Lodash, Moment.js, Bootstrap, Tailwind)
- **NO external markdown parsers** - implement custom lightweight solution
- **NO bundlers** - use native ES modules
- **Memory target:** <100MB application footprint (excluding AI models)
- **Performance target:** File operations <50ms, UI <16ms frame time

### Resource Allocation Strategy
- **70% system resources** reserved for AI inference (Ollama)
- **20% application logic** (file operations, UI)
- **10% system overhead** (OS, Tauri runtime)

### Implementation Approach
- **Custom implementations** preferred over external libraries
- **Native browser APIs** over polyfills or abstractions
- **Rust std library** preferred over external crates
- **File-based storage** instead of databases
- **Vanilla CSS** with Grid/Flexbox for layout

## Current Architecture

### Three-Column Layout (Phase 1)
```
┌─ File Tree ─┐ ┌─ Editor/Preview ─┐ ┌─ AI Panel ─┐
│ Vault files │ │ Toggle between   │ │ (Hidden in │
│ navigation  │ │ edit/preview     │ │ Phase 1)   │
└─────────────┘ └──────────────────┘ └────────────┘
```

### Technology Implementation
- **Frontend:** Vanilla JavaScript ES6+ modules, native CSS Grid/Flexbox
- **Backend:** Rust with Tauri v2, minimal external crates
- **Communication:** Tauri's invoke system for frontend-backend calls
- **File Handling:** Direct filesystem access via Tauri APIs
- **UI State:** Custom JavaScript state management (no frameworks)

## Development Phases & Current Status

### Phase 1: Core Editor (🔄 IN PROGRESS)
**Implementation Priority:**
1. Vault management (folder selection, file scanning)
2. File tree component (vanilla JS, hierarchical display)
3. Editor/preview toggle (custom markdown parser)
4. File operations (create, read, update, delete)
5. Hidden AI panel structure (prepared for Phase 2)

**Technical Requirements:**
- Three-column responsive layout
- Custom markdown syntax highlighting
- Auto-save functionality
- Keyboard shortcuts (Ctrl+B, Ctrl+I, etc.)
- Context menus for file operations

### Phase 2: AI Knowledge Weaver (⏳ PLANNED)
- Ollama integration for embeddings
- Custom vector database (JSON/SQLite)
- Real-time note suggestions
- AI panel becomes visible

### Phase 3: Creative Assistant (⏳ PLANNED)
- LLM integration via Ollama
- RAG-powered text generation
- Enhanced AI panel with chat interface

## Code Quality Requirements

### Before Any Code Changes
1. **Read README.md and CONTRIBUTING.md** thoroughly
2. **Verify no external dependencies** are being added
3. **Check memory usage** stays within 100MB target
4. **Test performance** with file operations
5. **Run cargo test** and ensure all tests pass

### Implementation Standards
- **Rust:** Use `Result<T>` for all commands, proper error handling
- **JavaScript:** ES6+ modules, classes, async/await, JSDoc comments
- **CSS:** Native CSS with Grid/Flexbox, no preprocessors
- **Files:** Group related functionality, minimize file count
- **Performance:** Profile memory usage, optimize for AI coexistence

### Testing Requirements
- **Frontend Tests:** Unit tests with Vitest (`pnpm test`) - ALWAYS run before committing
- **Rust Tests:** Integration tests (`cargo test --manifest-path src-tauri/Cargo.toml`)
- **Performance Tests:** Built into test suite, validates <10ms content extraction, <100ms parsing
- **Manual Testing:** Large vaults (100+ files), Ollama running simulation
- **Cross-platform:** Windows, macOS, Linux compatibility

### Application State Storage
- **Window and app state** saved to: `~/.ainote/app_state.json`
- **Contains:** Window dimensions, position, maximized state, layout settings, current vault/file

## Key Implementation Notes

### Current State (Phase 1)
- **Testing Infrastructure:** Complete Vitest setup with Tauri mocking (Issue #162 ✅)
- **Frontend Components:** FileTree, EditorPreviewPanel, AiPanel, MarkdownEditor implemented
- **Backend Commands:** Comprehensive Tauri commands for vault, file, and state operations
- **Architecture:** Three-column responsive layout with state management
- **AI Infrastructure:** Vector database, embeddings, performance monitoring (hidden UI)

### Testing Infrastructure (✅ COMPLETE)
- **Framework:** Vitest with jsdom environment
- **Mocking:** Complete Tauri API mocking system (`tests/__mocks__/tauri-mocks.js`)
- **Coverage:** Performance testing, unit tests, smoke tests
- **Commands:** `pnpm test`, `pnpm test:ui`, `pnpm test:watch`, `pnpm test:coverage`
- **Validation:** 20/20 smoke tests passing

### Next Implementation Priorities
1. **Issue #163:** Comprehensive unit tests for components and services
2. **Issue #164:** E2E testing with tauri-driver
3. **Phase 2:** AI integration and Ollama connectivity

### Constraints & Guidelines
- **No external API calls** - everything local
- **Standard markdown files** - no proprietary formats  
- **Custom algorithms** - implement cosine similarity, markdown parsing
- **Efficient memory usage** - explicit resource management
- **AI preparation** - design for future Ollama integration
- **Testing Required** - All new features must include tests

## Testing Guidelines for Claude Code

### Before Making Code Changes
1. **Run smoke test:** `pnpm test tests/unit/smoke-test.test.js --run` 
2. **Verify infrastructure:** Ensure 20/20 tests pass
3. **Review test docs:** Check `tests/README.md` for patterns

### When Writing New Code
1. **Create tests first:** TDD approach recommended
2. **Use mocks:** Import `setupTauriMocks()` from `tests/__mocks__/tauri-mocks.js`
3. **Test performance:** Validate against aiNote performance targets (<10ms, <100ms)
4. **Mock Tauri commands:** Use `window.__TAURI__.core.invoke` with proper mocking

### Before Committing
1. **Run full test suite:** `pnpm test --run`
2. **Check Rust tests:** `cargo test --manifest-path src-tauri/Cargo.toml`
3. **Verify performance:** Ensure no degradation in test timings
4. **Update tests:** Add tests for new functionality

When implementing features, always prioritize the local-first, lightweight, and AI-optimized approach as detailed in README.md and CONTRIBUTING.md.