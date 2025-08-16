# Phase 1 Implementation Plan: Core Editor

## Overview

This implementation plan breaks down Phase 1 of aiNote into specific, actionable tasks that will deliver a usable markdown editor with three-column layout preparation for future AI features.

**Target:** Standalone markdown editor with vault management, file tree navigation, and editor/preview toggle functionality.

**Timeline:** August 2025

**Performance Goals:**
- Memory usage: <100MB
- File operations: <50ms
- UI responsiveness: <16ms frame time

## Current State Assessment

### What's Already Implemented
- ✅ Basic Tauri v2 application shell
- ✅ Rust backend with minimal dependencies (tauri, serde, serde_json, tauri-plugin-opener)
- ✅ Frontend structure ready for vanilla JS implementation
- ✅ Development environment configured

### What Needs Implementation
- File system operations (Rust commands)
- Vault management system
- Three-column responsive layout
- File tree component
- Editor/preview toggle panel
- Markdown syntax highlighting
- Auto-save functionality
- Basic file operations (CRUD)

## Architecture Overview

### Three-Column Layout Design
```
┌─────────────────────────────────────────────────────────────────┐
│ aiNote                                              [─] [□] [×] │
├─────────────────────────────────────────────────────────────────┤
│ ┌─ File Tree ─┐ ┌─ Editor/Preview ──┐ ┌─ AI Panel ──┐         │
│ │ 📁 vault/   │ │ # Document Title  │ │ (Hidden in   │         │
│ │ ├─ note1.md │ │                   │ │ Phase 1)     │         │
│ │ ├─ note2.md │ │ Content editing   │ │              │         │
│ │ └─ note3.md │ │ or preview...     │ │              │         │
│ │             │ │                   │ │              │         │
│ │             │ │ [Edit] [Preview]  │ │              │         │
│ └─────────────┘ └───────────────────┘ └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

### Technology Stack
- **Frontend:** Vanilla JavaScript ES6+ modules, CSS Grid/Flexbox
- **Backend:** Rust with Tauri v2 commands
- **Storage:** Direct filesystem access, no external databases
- **Styling:** Native CSS, no frameworks
- **State Management:** Custom JavaScript state management

## Detailed Implementation Tasks

### 1. Backend Infrastructure (Rust Commands)

#### 1.1 File System Operations
**Files:** `src-tauri/src/lib.rs`

**Commands to implement:**
```rust
// Vault operations
select_vault_folder() -> Result<String>
scan_vault_files(vault_path: String) -> Result<Vec<FileInfo>>

// File operations  
read_file(file_path: String) -> Result<String>
write_file(file_path: String, content: String) -> Result<()>
create_file(file_path: String) -> Result<()>
delete_file(file_path: String) -> Result<()>
rename_file(old_path: String, new_path: String) -> Result<()>

// Data structures
struct FileInfo {
    path: String,
    name: String,
    modified: SystemTime,
    size: u64,
    is_dir: bool,
}
```

**Requirements:**
- Use `Result<T>` for all operations
- Implement proper error handling
- Filter for `.md` files only
- Recursive directory scanning
- Performance optimization for large vaults

#### 1.2 Vault Management
**Features:**
- Folder selection dialog using Tauri's file dialog
- Vault path persistence in application settings
- Validation of selected vault (ensure it's a valid directory)
- Vault switching without application restart

### 2. Frontend Core Architecture

#### 2.1 Application State Management
**File:** `src/js/state.js`

```javascript
class AppState {
    constructor() {
        this.currentVault = null;
        this.currentFile = null;
        this.viewMode = 'editor'; // 'editor' | 'preview'
        this.unsavedChanges = false;
        this.files = [];
    }
    
    // State management methods
    setVault(vaultPath) { }
    setCurrentFile(filePath) { }
    toggleViewMode() { }
    markDirty(isDirty) { }
}
```

#### 2.2 Layout Manager
**File:** `src/js/layout.js`

```javascript
class LayoutManager {
    constructor() {
        this.fileTreeWidth = 250;
        this.aiPanelVisible = false; // Hidden in Phase 1
    }
    
    initLayout() { }
    resizeColumns() { }
    toggleAIPanel() { } // For future phases
}
```

### 3. File Tree Component

#### 3.1 Tree Structure Rendering
**File:** `src/js/components/file-tree.js`

**Features:**
- Hierarchical display of folders and markdown files
- Expandable/collapsible folders
- Visual highlighting of current file
- Keyboard navigation (arrow keys, Enter)
- Context menu for file operations

**Implementation:**
```javascript
class FileTree {
    constructor(container, appState) {
        this.container = container;
        this.appState = appState;
        this.selectedFile = null;
    }
    
    render(files) {
        // Build hierarchical tree structure
        // Render HTML with proper indentation
        // Attach event listeners
    }
    
    handleFileClick(file) {
        // Load file content
        // Update application state
        // Highlight selected file
    }
    
    showContextMenu(file, event) {
        // Create/Delete/Rename options
        // Position menu at mouse location
    }
}
```

#### 3.2 Context Menu Operations
**Features:**
- New File: Create markdown file with template
- Delete File: Confirmation dialog + file removal
- Rename File: Inline editing with validation
- Refresh: Re-scan vault for changes

### 4. Editor/Preview Panel

#### 4.1 Mode Toggle System
**File:** `src/js/components/editor-preview-panel.js`

**Features:**
- Toggle between Editor and Preview modes
- Maintain content and scroll position during toggle
- Keyboard shortcut (Ctrl+Shift+P)
- Visual indicator of current mode

#### 4.2 Markdown Editor
**File:** `src/js/components/markdown-editor.js`

**Features:**
```javascript
class MarkdownEditor {
    constructor(container) {
        this.container = container;
        this.textarea = null;
        this.content = '';
    }
    
    init() {
        // Create textarea with proper styling
        // Add syntax highlighting
        // Setup keyboard shortcuts
        // Configure auto-save
    }
    
    addKeyboardShortcuts() {
        // Ctrl+B: Bold
        // Ctrl+I: Italic  
        // Ctrl+K: Link
        // Ctrl+S: Save
        // Tab: Indent
    }
    
    applySyntaxHighlighting() {
        // Basic markdown syntax highlighting
        // Headers (#, ##, ###)
        // Bold/Italic (**text**, *text*)
        // Links [text](url)
        // Code blocks ```
    }
}
```

#### 4.3 Preview Renderer
**File:** `src/js/components/preview-renderer.js`

**Features:**
```javascript
class PreviewRenderer {
    constructor(container) {
        this.container = container;
    }
    
    render(markdown) {
        // Custom lightweight markdown parser
        // Support for headers, lists, links, bold/italic
        // Code blocks with syntax highlighting
        // Tables support
        // Link click handling
    }
    
    parseMarkdown(text) {
        // Implementation of basic markdown rules
        // No external dependencies
        // Performance optimized
    }
}
```

### 5. File Operations Integration

#### 5.1 Auto-Save System
**File:** `src/js/services/auto-save.js`

**Features:**
- Debounced auto-save (save after 2 seconds of no typing)
- Manual save with Ctrl+S
- Visual indicator of save status
- Error handling for save failures

#### 5.2 File CRUD Operations
**Integration with Rust backend:**
- Create: Template-based markdown file creation
- Read: Load file content into editor
- Update: Save changes with conflict detection
- Delete: Confirmation dialog + cleanup

### 6. UI/UX Implementation

#### 6.1 Responsive Layout
**File:** `src/styles.css`

**Features:**
- CSS Grid for three-column layout
- Responsive breakpoints for different screen sizes
- Resizable columns with drag handles
- Minimum/maximum column widths
- Collapse/expand panels

#### 6.2 Theme System
**Features:**
- Light and dark theme support
- Theme persistence in local storage
- Smooth theme transitions
- High contrast accessibility support

### 7. Performance Optimization

#### 7.1 Memory Management
- Efficient DOM manipulation
- Event listener cleanup
- Large file handling optimizations
- Garbage collection considerations

#### 7.2 File System Performance
- Lazy loading of file content
- Efficient vault scanning algorithms
- Caching of file metadata
- Debounced file system operations

## Testing Strategy

### Unit Tests (Rust)
```bash
cargo test
```

**Test Coverage:**
- File system operations
- Vault management functions
- Error handling scenarios
- Performance with large files

### Integration Tests
- Frontend-backend communication
- File operations workflow
- Large vault handling (1000+ files)
- Memory usage monitoring

### Manual Testing Checklist
- [ ] Application starts in <2 seconds
- [ ] Memory usage <100MB with typical vault
- [ ] File operations complete in <50ms
- [ ] UI remains responsive during operations
- [ ] No console errors or warnings
- [ ] Cross-platform compatibility

## Development Workflow

### Phase 1 Implementation Order

#### Week 1: Backend Foundation
**Priority: High-Priority Items First**

1. **[Issue #1: Backend file system operations](https://github.com/iPixx/ainote/issues/1)** `backend, high-priority`
   - Implement all Rust commands for file operations
   - Set up proper error handling with custom error types
   - Create FileInfo struct and vault scanning functionality

2. **[Issue #2: Application state management](https://github.com/iPixx/ainote/issues/2)** `frontend, high-priority`
   - Build AppState class with event system
   - Implement localStorage persistence
   - Create state change event broadcasting

#### Week 2: Layout and Core Architecture
3. **[Issue #3: Three-column responsive layout](https://github.com/iPixx/ainote/issues/3)** `frontend`
   - Implement CSS Grid-based layout system
   - Add responsive breakpoints and drag-to-resize
   - Prepare AI panel structure (hidden in Phase 1)

4. **[Issue #4: File tree component](https://github.com/iPixx/ainote/issues/4)** `frontend`
   - Build hierarchical file tree with navigation
   - Implement context menus and keyboard navigation
   - Add performance optimization for large vaults

#### Week 3: Editor and Preview Components
5. **[Issue #5: Markdown editor](https://github.com/iPixx/ainote/issues/5)** `frontend`
   - Create custom markdown editor with syntax highlighting
   - Implement keyboard shortcuts and auto-features
   - Add find/replace and performance optimization

6. **[Issue #6: Preview renderer](https://github.com/iPixx/ainote/issues/6)** `frontend`
   - Build custom lightweight markdown parser
   - Implement real-time preview updates
   - Add code syntax highlighting and export preparation

#### Week 4: Integration and Toggle System
7. **[Issue #7: Editor/preview toggle system](https://github.com/iPixx/ainote/issues/7)** `frontend, integration`
   - Create smooth mode switching functionality
   - Implement scroll position synchronization
   - Add keyboard shortcuts and state persistence

8. **[Issue #8: Vault management integration](https://github.com/iPixx/ainote/issues/8)** `integration, high-priority`
   - Integrate vault selection and auto-save systems
   - Connect all file operations with frontend components
   - Implement comprehensive error handling

#### Week 5: Testing and Quality Assurance
9. **[Issue #9: Comprehensive testing and optimization](https://github.com/iPixx/ainote/issues/9)** `testing, high-priority`
   - Run all unit tests and performance benchmarks
   - Conduct cross-platform compatibility testing
   - Perform accessibility compliance validation
   - Optimize for memory usage and performance targets

### Quality Gates
Each component must pass:
- Performance benchmarks
- Memory usage targets
- Local-first compliance
- Cross-platform testing

## Acceptance Criteria

### Functional Requirements
- [ ] User can select a vault folder
- [ ] File tree displays all markdown files hierarchically
- [ ] User can navigate files by clicking in tree
- [ ] Editor mode allows markdown editing with syntax highlighting
- [ ] Preview mode renders markdown correctly
- [ ] Toggle between editor/preview modes works smoothly
- [ ] Auto-save functions properly
- [ ] File operations (create/delete/rename) work correctly
- [ ] Application state persists between sessions

### Performance Requirements
- [ ] Application memory usage <100MB
- [ ] File loading time <50ms for typical notes
- [ ] UI responsiveness <16ms frame time
- [ ] Large vault handling (1000+ files) without degradation
- [ ] Startup time <2 seconds

### Quality Requirements
- [ ] No external JavaScript dependencies
- [ ] Custom markdown parser implementation
- [ ] Proper error handling throughout
- [ ] Cross-platform compatibility
- [ ] Accessibility standards compliance

## Risk Mitigation

### Technical Risks
1. **Performance with Large Vaults**
   - Mitigation: Implement lazy loading and virtualization
   - Testing: Regular testing with 1000+ file vaults

2. **Custom Markdown Parser Complexity**
   - Mitigation: Start with minimal feature set, iterate
   - Fallback: Consider existing lightweight parsers if needed

3. **Cross-Platform File System Issues**
   - Mitigation: Extensive testing on all target platforms
   - Use Tauri's built-in file system APIs

### Timeline Risks
1. **Feature Scope Creep**
   - Mitigation: Strict adherence to Phase 1 scope
   - Defer advanced features to Phase 2

2. **Integration Complexity**
   - Mitigation: Build and test incrementally
   - Maintain working application at all times

## Success Metrics

### User Experience
- Time to create and edit first note: <30 seconds
- User satisfaction with editor responsiveness
- Intuitive navigation without documentation

### Technical Performance
- Memory efficiency: 70% resources available for AI
- File operation speed benchmarks met
- Zero crashes during typical usage scenarios

### Code Quality
- All tests passing
- Code review approval
- Documentation completeness
- No security vulnerabilities

## GitHub Issues Reference

All Phase 1 tasks have been created as GitHub issues in the [aiNote repository](https://github.com/iPixx/ainote) and assigned to the **[Phase 1: Core Editor milestone](https://github.com/iPixx/ainote/milestone/1)**.

### Backend Issues
- **[#1: Implement file system operations commands](https://github.com/iPixx/ainote/issues/1)** `backend, enhancement, high-priority`
  - Foundation for all file operations and vault management

### Frontend Core Issues  
- **[#2: Implement application state management](https://github.com/iPixx/ainote/issues/2)** `frontend, enhancement, high-priority`
  - Centralized state with localStorage persistence
- **[#3: Create three-column responsive layout system](https://github.com/iPixx/ainote/issues/3)** `frontend, enhancement`
  - CSS Grid layout with resizable columns
- **[#4: Implement file tree component with navigation](https://github.com/iPixx/ainote/issues/4)** `frontend, enhancement`
  - Hierarchical file tree with context menus
- **[#5: Create markdown editor with syntax highlighting](https://github.com/iPixx/ainote/issues/5)** `frontend, enhancement`
  - Custom editor with keyboard shortcuts
- **[#6: Create markdown preview renderer](https://github.com/iPixx/ainote/issues/6)** `frontend, enhancement`
  - Lightweight custom markdown parser

### Integration Issues
- **[#7: Implement editor/preview toggle system](https://github.com/iPixx/ainote/issues/7)** `frontend, integration, enhancement`
  - Smooth mode switching with scroll sync
- **[#8: Vault management and auto-save system](https://github.com/iPixx/ainote/issues/8)** `integration, enhancement, high-priority`
  - Complete file management integration

### Quality Assurance
- **[#9: Comprehensive Phase 1 testing and optimization](https://github.com/iPixx/ainote/issues/9)** `testing, high-priority`
  - Performance testing and release preparation

### Quick Access Links
- **[All Phase 1 Issues](https://github.com/iPixx/ainote/issues?q=is%3Aissue+is%3Aopen+milestone%3A%22Phase+1%3A+Core+Editor%22)**
- **[High Priority Issues](https://github.com/iPixx/ainote/issues?q=is%3Aissue+is%3Aopen+label%3Ahigh-priority)**
- **[Backend Issues](https://github.com/iPixx/ainote/issues?q=is%3Aissue+is%3Aopen+label%3Abackend)**
- **[Frontend Issues](https://github.com/iPixx/ainote/issues?q=is%3Aissue+is%3Aopen+label%3Afrontend)**
- **[Integration Issues](https://github.com/iPixx/ainote/issues?q=is%3Aissue+is%3Aopen+label%3Aintegration)**

---

This implementation plan provides a clear roadmap for delivering Phase 1 of aiNote while maintaining the project's core principles of being local-first, lightweight, and AI-optimized. Each task is tracked as a GitHub issue with detailed acceptance criteria, dependencies, and performance targets.