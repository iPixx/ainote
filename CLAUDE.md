# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Documentation

**ALWAYS READ THESE FIRST:**
- [README.md](README.md) - Complete project overview, architecture, and development roadmap
- [CONTRIBUTING.md](CONTRIBUTING.md) - Detailed development guidelines, technical requirements, and local-first principles

## Project Overview

aiNote is a **local-first, AI-powered markdown note-taking application** built with Tauri v2. The project follows strict lightweight design principles to maximize resources for AI inference while maintaining complete privacy.

**Current Status:** Phase 1 implementation (core markdown editor)
**Next Phase:** AI-powered note suggestions via Ollama integration

## Development Commands

### Essential Commands
- `pnpm tauri dev` - Start development server with hot reload
- `pnpm tauri build` - Build production application
- `cargo check` - Check Rust code for compilation errors
- `cargo test` - Run Rust tests (ALWAYS run before committing)
- `cargo clippy` - Rust linting and suggestions

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
├── CODE_OF_CONDUCT.md    # Community guidelines
├── LICENSE               # MIT license
├── package.json          # Minimal frontend dependencies
├── pnpm-lock.yaml        # Lock file for reproducible builds
├── src/                  # Frontend (Vanilla JavaScript)
│   ├── index.html       # Main HTML entry point
│   ├── main.js          # Application logic (ES6+ modules)
│   ├── styles.css       # Native CSS styling (no frameworks)
│   └── assets/          # Static assets (images, icons)
└── src-tauri/           # Backend (Rust)
    ├── src/
    │   ├── main.rs      # Application entry point
    │   └── lib.rs       # Tauri commands and business logic
    ├── tauri.conf.json  # Tauri v2 configuration
    └── Cargo.toml       # Rust dependencies (minimal set)
```

### Technology Stack & Tools
- **Framework:** Tauri v2 (cross-platform desktop)
- **Frontend:** Vanilla JavaScript (ES6+), HTML5, CSS3
- **Backend:** Rust with minimal external crates
- **Package Manager:** pnpm (fast, efficient)
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
- Unit tests in Rust (`cargo test`)
- Manual testing with large vaults (100+ files)
- Performance testing with Ollama running
- Cross-platform testing (Windows, macOS, Linux)

### Application State Storage
- **Window and app state** saved to: `~/.ainote/app_state.json`
- **Contains:** Window dimensions, position, maximized state, layout settings, current vault/file

## Key Implementation Notes

### Current State (Phase 1)
- Basic Tauri v2 template with "greet" command example
- Frontend structure ready for three-column layout
- Backend prepared for file system operations
- AI panel infrastructure prepared but hidden

### Next Implementation Steps
1. Replace greet command with vault management commands
2. Implement file tree component (vanilla JS)
3. Create editor/preview toggle functionality
4. Add markdown syntax highlighting (custom implementation)
5. Implement file operations with Tauri commands

### Constraints & Guidelines
- **No external API calls** - everything local
- **Standard markdown files** - no proprietary formats
- **Custom algorithms** - implement cosine similarity, markdown parsing
- **Efficient memory usage** - explicit resource management
- **AI preparation** - design for future Ollama integration

When implementing features, always prioritize the local-first, lightweight, and AI-optimized approach as detailed in README.md and CONTRIBUTING.md.