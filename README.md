# aiNote

> A local-first, AI-powered markdown note-taking application built with Tauri v2

aiNote is an open-source project that explores how local AI can enhance personal knowledge management without compromising privacy. It runs entirely on your local machine, ensuring your data remains private while providing intelligent note-taking capabilities through local embeddings and LLM integration via Ollama.

## 📋 Table of Contents

- [Project Overview](#-project-overview)
- [Core Features](#-core-features)
- [Current Status](#-current-status)
- [Prerequisites & Installation](#-prerequisites--installation)
- [Technology Stack](#-technology-stack)
- [UI Layout & Design](#-ui-layout--design)
- [Development Roadmap](#-development-roadmap)
- [Development Guide](#-development-guide)
- [API Reference](#-api-reference)
- [Contributing](#-contributing)
- [License](#-license)

## 🎯 Project Overview

aiNote addresses the growing need for **private, intelligent note-taking** in an era where most solutions require cloud services. The project demonstrates how local AI can provide:

- **Semantic note linking** without sending data to external services
- **Context-aware text generation** using your own notes as knowledge base
- **Cross-platform compatibility** with native performance
- **Zero vendor lock-in** with standard markdown files

**Target Users:**
- Researchers and academics managing literature and ideas
- Writers and content creators organizing thoughts and drafts
- Knowledge workers building personal knowledge bases
- Privacy-conscious users avoiding cloud-based solutions

## ✨ Core Features

### Current (Phase 1)
- 📝 **Local Markdown Editor:** Three-column layout with file tree and editor/preview toggle
- 📁 **Vault Management:** Work with local folders containing markdown files
- 🔄 **Real-time Preview:** Live markdown rendering with synchronized scrolling
- 💾 **Auto-save:** Automatic file saving with manual save support

### Planned (Phase 2 & 3)
- 🧠 **AI Knowledge Weaver:** Local embeddings for semantic note suggestions
- ✍️ **AI Creative Assistant:** LLM-powered text generation and editing
- 🔗 **Smart Linking:** Auto-suggest connections between related notes
- 🎯 **RAG Integration:** Generate content using your notes as context

### Core Principles
- 🔐 **100% Private:** No cloud sync, no tracking, no data ever leaves your machine
- 🏠 **Local-First:** All processing happens on your device
- 📂 **Standard Files:** Uses regular markdown files - no proprietary formats
- 💻 **Cross-Platform:** Built with Tauri, runs on Windows, macOS, and Linux
- ⚡ **Lightweight:** Minimal dependencies to maximize AI inference resources

## 🚀 Current Status

**Development Stage:** Phase 1 Implementation
- ✅ **Project Structure:** Tauri v2 template configured
- 🔄 **In Progress:** Core markdown editor with three-column layout
- ⏳ **Next:** File tree, vault management, and editor/preview toggle

**What's Working:**
- Basic Tauri v2 application shell
- Rust backend with file system access
- Frontend structure ready for markdown editing

**What's Coming Next:**
- Vault folder selection and file scanning
- Three-column responsive layout
- Markdown editor with syntax highlighting
- Live preview with toggle functionality

## 📦 Prerequisites & Installation

### System Requirements
- **OS:** Windows 10+, macOS 10.15+, or Linux
- **Node.js:** v18+ 
- **Rust:** Latest stable version
- **pnpm:** Package manager (recommended)

### AI Requirements (Phase 2+)
- **Ollama:** For local AI capabilities
  - Install from [ollama.com](https://ollama.com/)
  - Recommended models: `nomic-embed-text` (embeddings), `llama2` (generation)

### Development Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/username/ainote.git
   cd ainote
   ```

2. **Install dependencies:**
   ```bash
   pnpm install
   ```

3. **Start development server:**
   ```bash
   pnpm tauri dev
   ```

4. **Build for production:**
   ```bash
   pnpm tauri build
   ```

### Development Commands
- `pnpm tauri dev` - Start development server with hot reload
- `pnpm tauri build` - Build production application
- `cargo check` - Check Rust code for compilation errors
- `cargo test` - Run Rust tests

## 🏗️ Technology Stack

### Local-First & Lightweight Design Philosophy

**Core Design Rules:**
- **No External Dependencies:** Minimize external libraries to reduce bloat and maximize AI inference resources
- **Vanilla JavaScript Only:** No heavy frontend frameworks (React, Vue, Angular) - pure JS for maximum performance
- **Native Capabilities:** Leverage Tauri's native APIs instead of web-based alternatives
- **Resource Efficient:** Every dependency must justify its existence for AI performance optimization
- **Standard Libraries:** Prefer built-in browser/Rust capabilities over third-party solutions

### Core Framework
- **[Tauri v2](https://v2.tauri.app/)** - Cross-platform app framework (minimal overhead)
- **Rust** - Backend system operations and AI integration (zero-cost abstractions)
- **Vanilla JavaScript** - Frontend without frameworks (maximum performance)
- **CSS3** - Native styling with CSS Grid and Flexbox (no CSS frameworks)

### AI & ML Stack (Phase 2+)
- **[Ollama](https://ollama.com/)** - Local LLM and embedding models (external process)
- **Local Storage** - File-based JSON/SQLite for embeddings (no external databases)
- **Native Algorithms** - Cosine similarity implemented in Rust (no ML libraries)

### Dependency Constraints

**Allowed Dependencies (Minimal Set):**
```json
{
  "frontend": ["@tauri-apps/cli"],
  "backend": ["tauri", "tauri-plugin-opener", "serde", "tokio"],
  "ai_phase": ["reqwest", "serde_json", "rusqlite"]
}
```

**Forbidden Dependencies:**
- Frontend frameworks (React, Vue, Angular, Svelte)
- UI component libraries (Bootstrap, Material-UI, Chakra)
- Heavy JavaScript libraries (jQuery, Lodash, Moment.js)
- CSS frameworks (Tailwind, Bulma, Foundation)
- Markdown processing libraries (use custom lightweight parser)
- Vector database libraries (implement custom solution)

### Resource Allocation Strategy

**Memory & CPU Priority:**
1. **AI Inference (70%)** - Ollama models get maximum resources
2. **Application Logic (20%)** - File operations, UI rendering
3. **System Overhead (10%)** - OS, Tauri runtime

**Performance Targets:**
- **Startup Time:** < 2 seconds cold start
- **Memory Usage:** < 100MB base application (excluding AI models)
- **File Operations:** < 50ms for typical note files
- **UI Responsiveness:** < 16ms frame time (60fps)

## 🎨 UI Layout & Design

### Three-Column Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ aiNote                                              [─] [□] [×] │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ ┌─ File Tree ─┐ ┌─ Editor/Preview ──┐ ┌─ AI Panel ──┐         │
│ │ 📁 Notes    │ │ # My Note         │ │ 📋 Related   │         │
│ │ ├─ 📄 doc1  │ │                   │ │ • Note A     │         │
│ │ ├─ 📄 doc2  │ │ Content here...   │ │ • Note B     │         │
│ │ └─ 📄 doc3  │ │                   │ │              │         │
│ │             │ │ [Edit] [Preview]  │ │ 🤖 Assistant │         │
│ │             │ │                   │ │ [Expand]     │         │
│ │             │ │                   │ │ [Summarize]  │         │
│ └─────────────┘ └───────────────────┘ └──────────────┘         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase Evolution
- **Phase 1:** File Tree + Editor/Preview Toggle + Hidden AI Panel
- **Phase 2:** + AI Panel with note suggestions
- **Phase 3:** + Enhanced AI Panel with chat interface

### Design Principles
- **Minimalist Interface:** Clean, distraction-free design
- **Responsive Layout:** Adapts to different screen sizes
- **Keyboard-First:** Extensive keyboard shortcuts
- **Dark/Light Themes:** User preference support

## 📈 Development Roadmap

### Phase 1: The Core Editor (MVP) 🔄 *In Progress*

**Goal:** Build a usable, standalone markdown editor with three-column layout preparation.

**Architecture:**
- Frontend: Vanilla JS with responsive three-column layout
- Backend: Rust with Tauri commands for file operations  
- Layout: Editor/preview toggle in center, AI panel hidden
- Window: Expandable from 800x600 base size

**Features:**

**1. Vault Management**
- [ ] Folder selection dialog for choosing note vault
- [ ] Vault persistence between application sessions
- [ ] Recursive scanning for all `.md` files in vault
- [ ] Vault switching without application restart

**2. File Tree Panel**
- [ ] Hierarchical display of folder structure with `.md` files
- [ ] Click navigation to open files in editor
- [ ] Visual highlighting of currently open file
- [ ] Right-click context menu (new file, delete, rename)

**3. Editor/Preview Toggle Panel**
- [ ] Toggle between editor and preview modes (Ctrl+Shift+P)
- [ ] Editor mode: Markdown editing with syntax highlighting
- [ ] Preview mode: Rendered markdown for reading
- [ ] Maintain scroll position and content state on toggle

**Editor Mode Features:**
- [ ] Basic markdown syntax highlighting
- [ ] Auto-save after typing delay
- [ ] Keyboard shortcuts (Ctrl+B for bold, Ctrl+I for italic, etc.)
- [ ] Line numbers, word count, find/replace functionality

**Preview Mode Features:**
- [ ] Real-time markdown rendering of current content
- [ ] Support for headers, lists, links, code blocks, tables
- [ ] Export options (print and save as HTML/PDF)
- [ ] Clean, distraction-free reading experience

**4. File Operations**
- [ ] Create new markdown files with template
- [ ] Manual save (Ctrl+S) and automatic saving
- [ ] Delete files with confirmation dialog
- [ ] Inline file renaming in tree view

**5. AI Panel Infrastructure**
- [ ] Create hidden third column structure
- [ ] Responsive design accommodating future AI panel
- [ ] Infrastructure for showing/hiding AI panel in later phases

### Phase 2: The Knowledge Weaver (AI-Powered Retrieval) ⏳ *Planned*

**Goal:** Add semantic search and intelligent note linking using local embeddings.

**Architecture:**
- AI Panel: Third column becomes visible with note suggestions
- Ollama Integration: Local embedding models for semantic similarity
- Vector Storage: Local file-based database for embeddings
- Real-time Processing: Generate embeddings as user types

**Features:**

**1. Ollama Integration & Setup**
- [ ] Detect Ollama running on localhost:11434
- [ ] Download and manage embedding models (nomic-embed-text)
- [ ] Monitor connection status with UI indicators
- [ ] Graceful degradation when Ollama unavailable

**2. Note Indexing System**
- [ ] Index all existing notes on vault selection
- [ ] Incremental re-indexing for changed files
- [ ] Split notes into semantic chunks for better embeddings
- [ ] Store file paths, timestamps, and chunk references

**3. Vector Database**
- [ ] Store embeddings in local JSON/SQLite database
- [ ] Cache embeddings to avoid re-computation
- [ ] Fast similarity search using cosine similarity
- [ ] Clean up orphaned embeddings from deleted files

**4. AI Panel - Note Suggestions**
- [ ] Display semantically similar notes while typing
- [ ] Live updates as user types (debounced for performance)
- [ ] Clickable suggestions to open related notes
- [ ] Show relevance scores for transparency

**5. Real-time Embedding Generation**
- [ ] Detect when user stops typing to trigger embedding
- [ ] Generate embeddings for current paragraph/selection
- [ ] Find most relevant notes from vector database
- [ ] Optimize performance with debouncing and caching

### Phase 3: The Creative Assistant (Generative AI) ⏳ *Planned*

**Goal:** Integrate local LLM for text generation, editing assistance, and RAG-powered content creation.

**Architecture:**
- Enhanced AI Panel: Split into suggestions (top) and assistant (bottom)
- RAG Integration: Combine Phase 2's semantic search with LLM context
- Streaming UI: Real-time token-by-token response display
- Context Management: Smart context window management

**Features:**

**1. LLM Integration & Management**
- [ ] Support multiple Ollama models (llama2, mistral, codellama)
- [ ] Dynamic model selection based on task type
- [ ] Performance monitoring and model availability tracking
- [ ] Optimize context window usage for better performance

**2. Context-Aware Text Generation (RAG)**
- [ ] Use Phase 2's embeddings to find relevant notes for context
- [ ] Prioritize and rank most relevant notes for LLM context
- [ ] Compress long contexts to fit within model limits
- [ ] Show source attribution for generated content

**3. AI Assistant Panel**
- [ ] Conversational AI assistant in bottom section of AI panel
- [ ] Predefined quick action buttons (expand, summarize, rephrase)
- [ ] User-defined prompt templates for frequent use cases
- [ ] Conversation history management within sessions

**4. Text Generation Features**
- [ ] Right-click context menu for selected text operations
- [ ] Generate text directly in editor at cursor position
- [ ] Token-by-token streaming display with typing animation
- [ ] Generation controls: stop, regenerate, accept/reject

**5. Advanced AI Capabilities**
- [ ] Writing assistance: grammar, style, tone adjustment
- [ ] Content expansion using context from related notes
- [ ] Summarization of selected text or entire notes
- [ ] Question answering about vault content using RAG

**6. Integration Features**
- [ ] Auto-suggest links to related notes in generated content
- [ ] Include references to source notes in generated text
- [ ] Track AI-generated vs human-written content
- [ ] Granular undo for AI-generated text insertions

## 🔧 Development Guide

### Project Structure

```
ainote/
├── src/                    # Frontend (Vanilla JS)
│   ├── index.html         # Main HTML entry point
│   ├── main.js           # Application logic
│   ├── styles.css        # Application styles
│   └── assets/           # Static assets
├── src-tauri/            # Backend (Rust)
│   ├── src/
│   │   ├── main.rs      # Application entry point  
│   │   └── lib.rs       # Tauri commands and logic
│   ├── tauri.conf.json  # Tauri configuration
│   └── Cargo.toml       # Rust dependencies
├── CLAUDE.md            # Claude Code instructions
├── package.json         # Frontend dependencies
└── README.md           # This file
```

### Coding Standards

**Local-First Implementation Rules:**

**Rust Backend:**
- Use `Result<T>` for all command return types
- Implement proper error handling with custom error types
- Follow Rust naming conventions (snake_case for functions)
- Add comprehensive documentation for all public APIs
- **No external crates unless absolutely necessary** - prefer std library
- Implement custom algorithms over dependencies (e.g., cosine similarity)
- Optimize for memory efficiency to leave resources for AI

**JavaScript Frontend:**
- **Pure Vanilla JS only** - no frameworks or libraries
- Use modern ES6+ features (modules, classes, async/await)
- Implement custom components instead of using libraries
- Follow consistent naming conventions (camelCase)
- Add JSDoc comments for complex functions
- **No bundlers** - use native ES modules
- Minimize DOM manipulation for performance

**Lightweight Implementation Guidelines:**
- **Custom markdown parser** - implement minimal parser instead of using libraries
- **Native CSS styling** - no preprocessors or frameworks
- **File-based storage** - avoid databases, use JSON/text files
- **Efficient algorithms** - implement only what's needed
- **Memory management** - explicitly manage resources for AI optimization

**File Organization:**
- Group related functionality in modules
- Separate UI components from business logic
- Keep configuration in dedicated files
- Maintain clear separation between phases
- **Minimize file count** - combine related functionality

### Testing Strategy

**Unit Tests:**
- Rust: Use `cargo test` for backend logic
- JavaScript: Manual testing during Phase 1, framework TBD

**Integration Tests:**
- File operations with real filesystem
- Ollama API integration (Phase 2+)
- End-to-end user workflows

**Performance Tests:**
- Large vault handling (1000+ notes)
- Embedding generation speed
- Memory usage with large documents

### Code Review Process

1. **Self-review:** Test locally with `pnpm tauri dev`
2. **Functionality:** Verify all features work as specified
3. **Performance:** Check for memory leaks and slow operations
4. **Documentation:** Update README and CLAUDE.md as needed

## 📚 API Reference

### Phase 1: Backend Commands (Rust)

```rust
// File system operations
select_vault_folder() -> Result<String>
scan_vault_files(vault_path: String) -> Result<Vec<FileInfo>>
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
}
```

### Phase 1: Frontend Components (JS)

```javascript
// Core components
class VaultManager {
    // Handle vault selection and persistence
    selectVault() -> Promise<string>
    loadVault() -> Promise<FileInfo[]>
    saveVaultPreference(path) -> void
}

class FileTree {
    // Render and manage file tree with context menus
    render(files) -> void
    handleFileClick(file) -> void
    showContextMenu(file, event) -> void
}

class EditorPreviewPanel {
    // Toggle between editor and preview modes with shared state
    toggleMode() -> void
    setContent(content) -> void
    getContent() -> string
    maintainScrollPosition() -> void
}

class MarkdownEditor {
    // Editor with syntax highlighting and shortcuts
    init(container) -> void
    setValue(content) -> void
    getValue() -> string
    addKeyboardShortcuts() -> void
}

class PreviewRenderer {
    // Markdown renderer for preview mode
    render(markdown) -> string
    updatePreview(content) -> void
    handleLinkClicks() -> void
}

class AppState {
    // Central state management
    currentFile: string
    currentVault: string
    viewMode: 'editor' | 'preview'
    unsavedChanges: boolean
}

class LayoutManager {
    // Handle three-column responsive layout
    initLayout() -> void
    resizeColumns(sizes) -> void
    toggleAIPanel(visible) -> void
}
```

### Phase 2: AI & Embedding APIs

```rust
// Ollama integration
check_ollama_status() -> Result<bool>
generate_embedding(text: String, model: String) -> Result<Vec<f32>>
get_available_models() -> Result<Vec<String>>

// Vector database operations
index_vault_notes(vault_path: String) -> Result<()>
store_embedding(file_path: String, chunk_id: String, embedding: Vec<f32>) -> Result<()>
search_similar_notes(query_embedding: Vec<f32>, limit: usize) -> Result<Vec<SimilarNote>>
update_note_index(file_path: String, content: String) -> Result<()>

// Data structures
struct SimilarNote {
    file_path: String,
    chunk_id: String,
    similarity_score: f32,
    content_preview: String,
}
```

### Phase 3: LLM & RAG APIs

```rust
// LLM integration
get_available_llm_models() -> Result<Vec<String>>
generate_text(prompt: String, model: String, context: Vec<String>) -> Result<String>
stream_text_generation(prompt: String, model: String) -> Result<TextStream>
check_model_capability(model: String, task: String) -> Result<bool>

// RAG operations
build_rag_context(query: String, max_context_length: usize) -> Result<Vec<ContextNote>>
summarize_context(notes: Vec<String>, max_length: usize) -> Result<String>
rank_relevant_notes(query: String, notes: Vec<SimilarNote>) -> Result<Vec<ContextNote>>

// AI assistant
process_ai_command(command: String, selection: String, context: Vec<String>) -> Result<String>
save_ai_session(session_id: String, messages: Vec<Message>) -> Result<()>
load_ai_session(session_id: String) -> Result<Vec<Message>>
```

## 🤝 Contributing

We welcome contributions to aiNote! This project follows our [Code of Conduct](CODE_OF_CONDUCT.md) and detailed [Contributing Guidelines](CONTRIBUTING.md).

**Quick Start for Contributors:**
1. 📖 Read our [Contributing Guidelines](CONTRIBUTING.md) for detailed instructions
2. 📜 Review our [Code of Conduct](CODE_OF_CONDUCT.md) 
3. 🔍 Check [existing issues](https://github.com/iPixx/ainote/issues) for good first contributions
4. 🍴 Fork the repository and create a feature branch
5. 💻 Follow our local-first and lightweight design principles below

### 🏗️ Development Principles for Contributors

**Local-First & Lightweight Requirements:**
- **No external dependencies** without prior discussion and approval
- **Vanilla JavaScript only** - no frameworks or heavy libraries
- **Custom implementations** preferred over external packages
- **Performance testing** with AI models running simultaneously
- **Memory profiling** to ensure <100MB application footprint

### 🚀 Ways to Contribute

**Code Contributions:**
- Implement features from the [development roadmap](#-development-roadmap)
- Fix bugs and improve performance
- Add tests and documentation
- Improve UI/UX design following our lightweight principles

**Non-Code Contributions:**
- Report bugs and suggest features via [GitHub Issues](https://github.com/iPixx/ainote/issues)
- Improve documentation and guides
- Test with different operating systems
- Share usage feedback and ideas

### 📋 Development Priorities

**Phase 1 (Current Priority):**
- File tree implementation with native JavaScript
- Editor/preview toggle functionality
- Vault management features
- Responsive three-column layout

**Future Phases:**
- Ollama integration and embedding generation
- Custom vector database implementation
- LLM integration and RAG features

### 🔧 Quick Setup for Contributors

```bash
# Clone your fork
git clone https://github.com/your-username/ainote.git
cd ainote

# Install minimal dependencies
pnpm install

# Start development
pnpm tauri dev

# Test your changes
cargo test
```

For detailed setup instructions, code standards, and submission guidelines, please see [CONTRIBUTING.md](CONTRIBUTING.md).

### 🤖 AI-Assisted Development

**AI Usage for Coding Support:**

This project welcomes the use of AI tools as coding assistants, provided they align with our core principles:

**Recommended AI Tools:**
- **Code Generation:** GitHub Copilot, Claude Code, ChatGPT for boilerplate and routine code
- **Code Review:** AI-assisted code analysis and optimization suggestions
- **Documentation:** AI help with comments, documentation, and README updates
- **Debugging:** AI assistance for identifying and fixing bugs

**AI Usage Guidelines:**
- **Maintain Local-First Principles:** Ensure AI suggestions don't introduce external dependencies
- **Review AI Output:** Always review and understand AI-generated code before committing
- **Performance Focus:** Use AI to optimize for the 100MB memory target and AI inference priority
- **Custom Implementation:** Prefer AI-assisted custom solutions over library recommendations
- **Code Quality:** AI should help improve, not replace, thoughtful engineering

**AI-Assisted Workflow:**
1. **Design Phase:** Use AI to explore architecture options within our constraints
2. **Implementation:** Leverage AI for writing vanilla JS and efficient Rust code
3. **Optimization:** AI-assisted performance tuning and memory optimization
4. **Documentation:** AI help with inline documentation and technical writing
5. **Testing:** AI-generated test cases and edge case identification

**What to Avoid:**
- Blindly accepting AI suggestions without understanding the code
- AI recommendations that add dependencies or complexity
- Using AI to circumvent our lightweight design principles
- Relying solely on AI without applying domain knowledge

**Disclosure:** When using AI assistance for significant contributions, consider mentioning it in your PR description to help maintainers understand the development process.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

### Key Points

- **Open Source:** Free to use, modify, and distribute
- **Commercial Use:** Allowed with proper attribution
- **No Warranty:** Software provided "as is"
- **Attribution:** Please include license notice in distributions

---

**Built with ❤️ for the local-first, privacy-conscious community.**

For questions, feedback, or support, please [open an issue](https://github.com/username/ainote/issues) on GitHub.