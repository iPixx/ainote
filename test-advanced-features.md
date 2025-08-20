# Advanced Markdown Features Test

## Tables

| Feature | Status | Performance Target | Actual Result |
|:--------|:------:|:------------------:|-------------:|
| Link Handling | ✅ Complete | < 5ms | 2ms |
| Table Rendering | ✅ Complete | < 20ms | 15ms |
| Image Loading | ✅ Complete | < 100ms | 50ms |
| Export HTML | ✅ Complete | < 200ms | 150ms |

## Links

### Internal Links
- [Another file](./vault/notes/first_test.md)
- [Local link](./README.md)

### External Links
- [Claude Code GitHub](https://github.com/anthropics/claude-code)
- [Anthropic](https://www.anthropic.com)

### Anchor Links
- [Jump to Tables](#tables)
- [Jump to Images](#images)

## Images

![Test Image](https://picsum.photos/400/300)
![Local Image](./src/assets/tauri.svg)
![Missing Image](./missing-image.png)

## Code Blocks

```javascript
// Advanced JavaScript features
class PreviewRenderer {
  async exportToHTML(options = {}) {
    const startTime = performance.now();
    // Implementation here...
    return htmlDocument;
  }
  
  handleLinkClicks() {
    const links = this.elements.content.querySelectorAll('a');
    links.forEach(link => {
      // Handle different link types
    });
  }
}
```

```rust
// Rust example
fn main() {
    println!("Hello from aiNote!");
}
```

## Performance Test

This document should render in under 50ms and handle all advanced features:
- ✅ Table rendering with alignment
- ✅ Link click handling
- ✅ Image loading and error handling
- ✅ Export functionality
- ✅ Performance optimization