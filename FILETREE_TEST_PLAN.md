# 🧪 **FileTree Advanced Features Test Plan**

This document provides a comprehensive test checklist to validate all the implemented advanced FileTree features for GitHub issue #32.

## 🚀 **Setup Tests**

### Initial Setup

- [x] **App Launch**: Start `pnpm tauri dev` successfully
- [ ] **FileTree Loads**: File tree component initializes without console errors
- [x] **Search Button**: 🔍 button appears in file tree header
- [ ] **CSS Loaded**: All new styles are applied (no broken layouts)

---

## 🔍 **Search & Filter Tests**

### Search Interface

- [ ] **Button Activation**: Click 🔍 button → search input appears
- [ ] **Keyboard Activation**: Focus file tree, press `Ctrl/Cmd+F` → search activates
- [ ] **Input Focus**: Search input gets focus automatically
- [ ] **Close Button**: × button closes search interface

### Search Functionality

- [ ] **Basic Search**: Type filename → file appears in results
- [ ] **Fuzzy Search**: Type partial characters → relevant files found
- [ ] **Real-time Results**: Search updates as you type (300ms debounce)
- [ ] **No Results**: Search for non-existent file → "No files found" message
- [ ] **Search Highlighting**: Matching text highlighted in results
- [ ] **Path Display**: Search results show file paths
- [ ] **Case Insensitive**: Search works regardless of case

### Search Navigation

- [ ] **Arrow Down**: In search input, press ↓ → focus moves to first result
- [ ] **Escape**: Press Escape → search closes, focus returns to tree
- [ ] **Clear Search**: Clear input → normal tree view returns

---

## ⌨️ **Keyboard Navigation Tests**

### Basic Navigation

- [ ] **Arrow Up/Down**: Navigate between visible files
- [ ] **Arrow Left**: On expanded folder → collapses folder
- [ ] **Arrow Right**: On collapsed folder → expands folder
- [ ] **Arrow Right**: On expanded folder → moves to first child
- [ ] **Enter**: On file → opens file, on folder → toggles expand/collapse
- [ ] **Space**: Same behavior as Enter

### Advanced Navigation

- [ ] **Home**: Jumps to first item in tree
- [ ] **End**: Jumps to last visible item
- [ ] **Page Up**: Moves up ~10 items quickly
- [ ] **Page Down**: Moves down ~10 items quickly
- [ ] **Tab Navigation**: Tree items are properly focusable

### Focus Management

- [ ] **Visual Focus**: Focused item clearly highlighted
- [ ] **Focus Persistence**: Focus maintained during folder expand/collapse
- [ ] **Scroll Into View**: Focused item scrolls into view automatically

---

## 🖱️ **Drag & Drop Tests**

### Drag Operations

- [ ] **Drag Start**: Click and drag file → dragging visual feedback appears
- [ ] **Drag Image**: Custom drag image shows during drag
- [ ] **Drop Target**: Hover over folder → folder highlights as drop target
- [ ] **Invalid Drop**: Try to drop on file → cursor shows "not allowed"
- [ ] **Self Drop**: Try to drop folder on itself → prevented

### Drop Operations

- [ ] **Successful Move**: Drop file in folder → file moves, tree updates
- [ ] **Conflict Dialog**: Drop file where name exists → confirmation dialog
- [ ] **Folder Move**: Drag folder to another folder → entire folder moves
- [ ] **Nested Drop**: Drop file in deeply nested folder → works correctly
- [ ] **Current File Update**: Move currently open file → editor updates path

### Error Handling

- [ ] **Move Failure**: Simulate move error → error notification shown
- [ ] **Missing File**: Try to move deleted file → graceful error handling
- [ ] **Permission Error**: Move to protected folder → proper error message

---

## ⚡ **Performance Tests**

### Virtual Scrolling (needs 1000+ files)

- [ ] **Activation**: With 1000+ files → virtual scrolling auto-enables
- [ ] **Smooth Scroll**: Scrolling remains smooth at 60fps
- [ ] **Memory Usage**: Check DevTools → memory usage < 100MB
- [ ] **Console Logging**: Check console → performance metrics logged

### Large Vault Tests

- [ ] **Initial Render**: Large vault loads in < 100ms (check console)
- [ ] **Search Performance**: Search 1000+ files → results in reasonable time
- [ ] **Folder Expansion**: Expand large folder → "Load more" appears if > 100 children
- [ ] **Load More**: Click "Load more" → remaining items batch-loaded

### Lazy Loading

- [ ] **Folder Loading**: Expand folder → loading indicator appears briefly
- [ ] **Large Folders**: Folders with 100+ items → partial loading with "Load more"
- [ ] **Batch Loading**: "Load more" loads items in batches smoothly
- [ ] **Memory Cleanup**: Collapsed folders → children cleaned up from memory

---

## 🎨 **Visual & UX Tests**

### Loading States

- [ ] **Search Loading**: Search shows activity during processing
- [ ] **Folder Loading**: Folder icons show loading state during expansion
- [ ] **File Move Loading**: Drag operations show loading feedback
- [ ] **Global Loading**: File operations show loading notifications

### Error States

- [ ] **Error Display**: Errors show with ⚠️ icon and message
- [ ] **Error Dismiss**: Click × on error → error disappears
- [ ] **Auto Dismiss**: Errors auto-dismiss after 10 seconds
- [ ] **Folder Errors**: Folder load errors → folder marked with error state

### Responsive Design

- [ ] **Mobile View**: Features work on mobile/narrow screens
- [ ] **Search Mobile**: Search interface adapts to small screens
- [ ] **Drag Mobile**: Touch drag & drop works on touch devices
- [ ] **Path Hiding**: File paths hidden on mobile to save space

---

## 🎯 **Integration Tests**

### App State Integration

- [ ] **File Selection**: Tree selection updates app state correctly
- [ ] **Current File**: Currently open file highlighted in tree
- [ ] **Vault Changes**: Changing vault updates tree correctly
- [ ] **File Changes**: External file changes reflected in tree

### Backend Integration

- [ ] **File Moves**: Drag & drop calls backend correctly
- [ ] **Error Handling**: Backend errors properly surfaced to UI
- [ ] **File Watching**: File system changes update tree (if implemented)
- [ ] **Permissions**: Permission errors handled gracefully

### Event System

- [ ] **File Selection Events**: Tree fires selection events correctly
- [ ] **Drag Events**: Drag start/end events fired appropriately
- [ ] **Search Events**: Search state changes communicated
- [ ] **Performance Events**: Performance metrics events working

---

## 🔧 **Edge Cases & Stress Tests**

### Edge Cases

- [ ] **Empty Vault**: No files → proper empty state shown
- [ ] **Single File**: Vault with one file → navigation works
- [ ] **Deep Nesting**: Very deep folder structure → scrolling/navigation works
- [ ] **Long Names**: Very long file names → proper truncation/display
- [ ] **Special Characters**: Files with Unicode/special characters → handled correctly

### Stress Tests

- [ ] **Rapid Search**: Type very fast → debouncing works, no errors
- [ ] **Multiple Drags**: Perform many drag operations quickly → stable
- [ ] **Rapid Navigation**: Navigate quickly with arrow keys → smooth, no lag
- [ ] **Large Search**: Search in vault with 10,000+ files → reasonable performance

### Browser Compatibility

- [ ] **Chrome**: All features work in Chrome
- [ ] **Firefox**: All features work in Firefox
- [ ] **Safari**: All features work in Safari (if on macOS)
- [ ] **Accessibility**: Screen reader compatibility (basic test)

---

## 📊 **Performance Benchmarks**

### Target Metrics (check console logs)

- [ ] **Initial Render**: < 100ms for 1000 files
- [ ] **File Selection**: < 16ms response time
- [ ] **Search Response**: < 300ms for search results
- [ ] **Memory Usage**: < 100MB total application memory
- [ ] **Scroll Performance**: 60fps scrolling maintained

### Console Validation

- [ ] **No Errors**: No JavaScript errors in console
- [ ] **Performance Logs**: Performance metrics logged for large operations
- [ ] **Memory Logs**: Memory usage logged for large trees
- [ ] **Event Logs**: All events properly logged (in debug mode)

---

## 🚀 **Acceptance Test Scenarios**

### User Scenario 1: Finding Files

1. Open large vault (100+ files)
2. Click search button
3. Search for specific file
4. Navigate to file with arrow keys
5. Press Enter to open file
6. ✅ File opens in editor

### User Scenario 2: Organizing Files

1. Select a markdown file
2. Drag it to a different folder
3. Confirm move operation
4. Verify file appears in new location
5. ✅ File structure updated

### User Scenario 3: Keyboard Workflow

1. Focus file tree with tab
2. Navigate with arrow keys
3. Use Ctrl+F to search
4. Use Escape to exit search
5. Navigate to file and press Enter
6. ✅ Entirely keyboard-driven workflow

---

## 📋 **Testing Guidelines**

### 🎯 Priority Testing Order:

1. **Core Features** (Search, Navigation, Drag & Drop)
2. **Performance** (Virtual scrolling, Memory usage)
3. **Integration** (Backend communication, State management)
4. **Edge Cases** (Error handling, Large vaults)
5. **Polish** (Visual feedback, Accessibility)

### 📝 Test with different vault sizes:

- **Small**: < 50 files
- **Medium**: 100-500 files
- **Large**: 1000+ files
- **Huge**: 5000+ files (if available)

### 🛠️ Testing Setup:

1. Start development server: `pnpm tauri dev`
2. Open Developer Tools (F12) to monitor console and performance
3. Select or create test vaults of different sizes
4. Have sample markdown files ready for testing

### ⚠️ Known Limitations:

- Virtual scrolling requires 1000+ files to activate
- Drag & drop uses existing `rename_file` backend command
- Performance metrics logged to console in debug mode
- Mobile drag & drop may have platform-specific behavior

### 🐛 Bug Reporting:

When reporting issues, please include:

- Browser and OS version
- Console errors (if any)
- Steps to reproduce
- Expected vs actual behavior
- Vault size and file count

---

## ✅ **Completion Criteria**

The FileTree advanced features implementation is considered complete when:

- [ ] All core functionality tests pass
- [ ] Performance benchmarks are met
- [ ] No critical bugs in common workflows
- [ ] Integration with existing app functionality works
- [ ] User experience is smooth and intuitive

**Test Status**: ⏳ Ready for testing  
**Last Updated**: 2025-08-18  
**GitHub Issue**: #32 - Frontend: Add advanced navigation and performance features
