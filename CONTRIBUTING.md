# Contributing to aiNote

First off, thank you for considering contributing to aiNote! It's people like you that make open source such a great community. We welcome contributions of all kinds, from reporting bugs to writing code and documentation.

aiNote is a **local-first, AI-powered note-taking application** with strict lightweight design principles. Following these guidelines helps ensure we maintain our core mission of maximizing resources for AI inference while delivering a private, efficient user experience.

## 🎯 Project Mission & Principles

Before contributing, please understand our core principles:

- **🔐 100% Private & Local-First:** No cloud dependencies, all processing happens locally
- **⚡ Lightweight by Design:** Minimal dependencies to maximize AI inference resources
- **🏠 Resource Efficient:** Target <100MB memory footprint (excluding AI models)
- **🧠 AI-Optimized:** Reserve 70% of system resources for Ollama AI inference
- **📂 Standard Files:** Use regular markdown files, no proprietary formats

## 📜 Code of Conduct

This project and everyone participating in it is governed by the [aiNote Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior.

## 🚀 Development Environment Setup

### Prerequisites

- **Node.js** v18+ and **pnpm** (required)
- **Rust** latest stable version (required)
- **Ollama** (for Phase 2+ AI features)

### Initial Setup

```bash
# Fork and clone the repository
git clone https://github.com/your-username/ainote.git
cd ainote

# Install minimal dependencies
pnpm install

# Start development server
pnpm tauri dev

# Run tests
cargo test

# Check Rust code
cargo check
```

### Development Commands

- `pnpm tauri dev` - Start development with hot reload
- `pnpm tauri build` - Build production application
- `cargo check` - Verify Rust code compiles
- `cargo test` - Run Rust unit tests
- `cargo clippy` - Rust linting and suggestions

## 🏗️ Technical Guidelines

### Local-First Development Rules

**✅ ALLOWED:**
- Vanilla JavaScript (ES6+ modules, classes, async/await)
- Native browser APIs (DOM, File API, IndexedDB)
- Tauri's built-in APIs for file system operations
- Standard Rust library functions
- Custom algorithm implementations

**❌ FORBIDDEN:**
- Frontend frameworks (React, Vue, Angular, Svelte)
- UI libraries (Bootstrap, Material-UI, Chakra, Tailwind)
- Heavy JavaScript libraries (jQuery, Lodash, Moment.js)
- External markdown parsers (marked, markdown-it)
- Vector database libraries (use custom implementation)
- CSS preprocessors or frameworks

### Dependency Policy

**Before adding ANY dependency:**
1. **Justify the need** - Why can't this be implemented natively?
2. **Performance impact** - How much memory/CPU will this use?
3. **Local-first compliance** - Does this require internet access?
4. **Maintainer approval** - Get explicit approval before adding

### Performance Requirements

**Memory Targets:**
- Base application: <100MB
- File operations: <50ms for typical notes
- UI responsiveness: <16ms frame time (60fps)

**Testing Requirements:**
- Test with Ollama running (simulates AI resource usage)
- Profile memory usage during development
- Verify performance with large vaults (1000+ notes)

## 📝 How Can I Contribute?

### Reporting Bugs

Bugs are tracked as [GitHub issues](https://github.com/iPixx/ainote/issues). Before creating a bug report, please check existing issues.

**Bug Report Template:**
```markdown
**Bug Description:**
A clear description of what the bug is.

**Steps to Reproduce:**
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

**Expected Behavior:**
What you expected to happen.

**Actual Behavior:**
What actually happened.

**Environment:**
- OS: [e.g. Windows 11, macOS 13, Ubuntu 22.04]
- aiNote Version: [e.g. 0.1.0]
- Ollama Version: [if applicable]
- Available Memory: [e.g. 8GB, 16GB]

**Additional Context:**
- Screenshots or GIFs if applicable
- Console error messages
- Performance impact observed
```

### Suggesting Enhancements

Enhancement suggestions are tracked as [GitHub issues](https://github.com/iPixx/ainote/issues).

**Enhancement Template:**
```markdown
**Feature Description:**
What feature would you like to see?

**Problem Statement:**
What problem does this solve? Why is this enhancement needed?

**Proposed Solution:**
Describe your proposed solution in detail.

**Local-First Compliance:**
How does this maintain our local-first principles?

**Performance Impact:**
Estimated memory/CPU impact of this feature.

**Phase Alignment:**
Which development phase does this belong to? (1, 2, or 3)

**Alternatives Considered:**
What alternatives have you considered?
```

### Your First Code Contribution

**Good First Issues:**
Look for issues labeled `good first issue` - these are small, well-defined tasks perfect for getting familiar with the codebase.

**Phase 1 Priority Areas:**
- File tree implementation (vanilla JS)
- Editor/preview toggle functionality
- Vault management features
- Responsive layout improvements
- Performance optimizations

### Development Workflow

**Phase-Based Development:**
- **Phase 1 (Current):** Core markdown editor functionality
- **Phase 2 (Planned):** AI-powered note suggestions via Ollama
- **Phase 3 (Planned):** LLM integration for text generation

**Feature Alignment:**
Ensure your contribution aligns with the current phase. Check the [Development Roadmap](README.md#-development-roadmap) before starting.

## 🔄 Pull Request Process

### Fork-and-Pull Workflow

1. **Fork** the repository to your GitHub account
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/your-username/ainote.git
   cd ainote
   ```

3. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/bug-description
   ```

4. **Make your changes** following our guidelines
5. **Test thoroughly:**
   ```bash
   # Run Rust tests
   cargo test
   
   # Check compilation
   cargo check
   
   # Test with development server
   pnpm tauri dev
   
   # Performance test (if applicable)
   # Test with Ollama running to simulate AI resource usage
   ```

6. **Commit with conventional commits:**
   ```bash
   git commit -m "feat: add file tree navigation"
   git commit -m "fix: resolve memory leak in editor"
   git commit -m "docs: update API documentation"
   ```

7. **Push to your fork:**
   ```bash
   git push origin feature/your-feature-name
   ```

8. **Open a Pull Request** with detailed description

### Pull Request Requirements

**PR Description Template:**
```markdown
## Summary
Brief description of changes made.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Local-First Compliance
- [ ] No external dependencies added
- [ ] Uses vanilla JavaScript only
- [ ] No cloud services or internet requirements
- [ ] Custom implementations preferred over libraries

## Performance Impact
- [ ] Memory usage tested and within <100MB target
- [ ] Performance tested with Ollama running
- [ ] No significant performance degradation
- [ ] File operations remain <50ms

## Testing
- [ ] Rust tests pass (`cargo test`)
- [ ] Manual testing completed
- [ ] Tested on [specify OS]
- [ ] Performance profiled (if applicable)

## Phase Alignment
- [ ] Aligns with current development phase
- [ ] Does not introduce premature phase features
- [ ] Follows roadmap priorities

## Additional Notes
Any additional information, screenshots, or context.
```

### Code Review Process

**Review Criteria:**
1. **Local-first compliance** - No external dependencies
2. **Performance impact** - Memory and CPU efficiency
3. **Code quality** - Clean, documented, efficient code
4. **Phase alignment** - Fits current development stage
5. **Testing coverage** - Adequate testing completed

**Review Timeline:**
- We aim to review PRs within 48-72 hours
- Complex features may require multiple review rounds
- Performance-critical changes get additional scrutiny

## 🧪 Testing Guidelines

### Unit Testing
```bash
# Run all Rust tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Performance Testing
```bash
# Start Ollama (if available) to simulate AI resource usage
ollama serve

# Run development server and monitor resource usage
pnpm tauri dev

# Test with large vault (create 100+ markdown files)
# Monitor memory usage and responsiveness
```

### Manual Testing Checklist
- [ ] Application starts in <2 seconds
- [ ] Memory usage stays <100MB during normal use
- [ ] File operations complete in <50ms
- [ ] UI remains responsive during file operations
- [ ] Works correctly with large vaults (100+ files)
- [ ] No console errors or warnings

## 🤖 AI-Assisted Development

We welcome the use of AI tools for development assistance:

**Recommended AI Tools:**
- GitHub Copilot, Claude Code, ChatGPT for code generation
- AI-assisted code review and optimization
- Documentation and comment generation

**AI Usage Guidelines:**
- **Review all AI-generated code** before committing
- **Ensure AI suggestions follow our local-first principles**
- **Prefer AI-assisted custom implementations** over library recommendations
- **Mention AI assistance** in PR descriptions for significant contributions

**AI Workflow:**
1. Use AI to explore implementation options within our constraints
2. Generate boilerplate and routine code with AI assistance
3. Optimize performance with AI suggestions
4. Create documentation and tests with AI help
5. Always review and understand the generated code

## 🏆 Recognition

Contributors who make significant improvements will be:
- Added to the project's contributors list
- Mentioned in release notes
- Invited to participate in project direction discussions

## 📞 Getting Help

**Questions or Need Guidance?**
- Open a [GitHub Discussion](https://github.com/iPixx/ainote/discussions)
- Comment on relevant issues
- Check existing documentation and issues first

**Response Time:**
- Issues: 24-48 hours
- Pull requests: 48-72 hours
- Discussions: 24-48 hours

Thank you for contributing to aiNote and helping build the future of local-first, AI-powered note-taking! 🚀