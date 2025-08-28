/**
 * Unit tests for FileTree component
 * 
 * Tests cover:
 * - Component initialization and setup
 * - File tree rendering and hierarchy
 * - Event handling and user interactions
 * - Search and filtering functionality
 * - Performance requirements and virtual scrolling
 * - Accessibility features
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setupTauriMocks } from '../__mocks__/tauri-mocks.js';

// Import dependencies
import FileTree from '../../src/js/components/file-tree.js';
import AppState from '../../src/js/state.js';

describe('FileTree', () => {
  let fileTree;
  let appState;
  let container;
  let tauriMocks;

  beforeEach(() => {
    // Set up Tauri mocks
    tauriMocks = setupTauriMocks();
    
    // Create container element
    container = document.createElement('div');
    document.body.appendChild(container);
    
    // Create AppState instance
    appState = new AppState();
    
    // Create FileTree instance
    fileTree = new FileTree(container, appState);
  });

  afterEach(() => {
    // Clean up
    if (fileTree) {
      fileTree.destroy();
    }
    if (container && container.parentNode) {
      container.parentNode.removeChild(container);
    }
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should throw error for invalid container', () => {
      expect(() => {
        new FileTree(null, appState);
      }).toThrow('FileTree requires a valid DOM container element');

      expect(() => {
        new FileTree('not-an-element', appState);
      }).toThrow('FileTree requires a valid DOM container element');
    });

    it('should throw error for missing AppState', () => {
      expect(() => {
        new FileTree(container, null);
      }).toThrow('FileTree requires an AppState instance');
    });

    it('should initialize with proper default values', () => {
      expect(fileTree.container).toBe(container);
      expect(fileTree.appState).toBe(appState);
      expect(fileTree.files).toEqual([]);
      expect(fileTree.expandedFolders).toBeInstanceOf(Set);
      expect(fileTree.selectedFile).toBeNull();
      expect(fileTree.treeStructure).toBeInstanceOf(Map);
    });

    it('should set up container with proper attributes', () => {
      expect(container.className).toBe(FileTree.CSS_CLASSES.TREE_CONTAINER);
      expect(container.getAttribute('role')).toBe('tree');
      expect(container.getAttribute('aria-label')).toBe('File tree navigation');
    });

    it('should have valid CSS class constants', () => {
      expect(FileTree.CSS_CLASSES).toEqual({
        TREE_CONTAINER: 'file-tree-container',
        TREE_ITEM: 'tree-item',
        TREE_FOLDER: 'tree-folder',
        TREE_FILE: 'tree-file',
        TREE_ICON: 'tree-icon',
        TREE_NAME: 'tree-name',
        TREE_CHILDREN: 'tree-children',
        EXPANDED: 'expanded',
        COLLAPSED: 'collapsed',
        SELECTED: 'selected',
        INDENTED: 'indented'
      });
    });

    it('should have valid event constants', () => {
      expect(FileTree.EVENTS).toEqual({
        FILE_SELECTED: 'file_selected',
        FOLDER_EXPANDED: 'folder_expanded',
        FOLDER_COLLAPSED: 'folder_collapsed',
        TREE_UPDATED: 'tree_updated',
        DRAG_START: 'drag_start',
        DRAG_END: 'drag_end',
        FILE_MOVE_REQUESTED: 'file_move_requested'
      });
    });
  });

  describe('File Tree Rendering', () => {
    const mockFiles = [
      { name: 'root.md', path: '/vault/root.md', is_dir: false },
      { name: 'folder1', path: '/vault/folder1', is_dir: true },
      { name: 'folder2', path: '/vault/folder2', is_dir: true },
      { name: 'nested.md', path: '/vault/folder1/nested.md', is_dir: false },
      { name: 'deep', path: '/vault/folder1/deep', is_dir: true }
    ];

    beforeEach(() => {
      appState.currentVault = '/vault';
    });

    it('should render files with proper hierarchy', () => {
      fileTree.render(mockFiles);

      const treeItems = container.querySelectorAll(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
      expect(treeItems.length).toBeGreaterThan(0);

      // Check that root level items are rendered
      const rootItems = Array.from(treeItems).filter(item => 
        !item.classList.contains(FileTree.CSS_CLASSES.INDENTED)
      );
      expect(rootItems.length).toBeGreaterThan(0);
    });

    it('should handle empty file list gracefully', () => {
      fileTree.render([]);

      const emptyState = container.querySelector('.file-tree-empty-state');
      expect(emptyState).toBeTruthy();
      expect(emptyState.textContent).toContain('No files found in vault');
    });

    it('should handle invalid file input gracefully', () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      
      fileTree.render('not-an-array');

      const emptyState = container.querySelector('.file-tree-empty-state');
      expect(emptyState).toBeTruthy();
      expect(consoleSpy).toHaveBeenCalledWith('FileTree.render: files must be an array');
      
      consoleSpy.mockRestore();
    });

    it('should create tree items with proper attributes', () => {
      fileTree.render(mockFiles);

      const fileItem = container.querySelector(`[data-file-path="/vault/root.md"]`);
      expect(fileItem).toBeTruthy();
      expect(fileItem.getAttribute('role')).toBe('treeitem');
      expect(fileItem.getAttribute('aria-label')).toContain('File: root.md');
      expect(fileItem.getAttribute('tabindex')).toBe('0');
      expect(fileItem.getAttribute('draggable')).toBe('true');
    });

    it('should create folder items with expand/collapse state', () => {
      fileTree.render(mockFiles);

      const folderItem = container.querySelector(`[data-file-path="/vault/folder1"]`);
      expect(folderItem).toBeTruthy();
      expect(folderItem.classList.contains(FileTree.CSS_CLASSES.TREE_FOLDER)).toBe(true);
      expect(folderItem.getAttribute('aria-expanded')).toBe('false');
    });

    it('should apply proper indentation based on depth', () => {
      fileTree.render(mockFiles);
      fileTree.expandFolder('/vault/folder1');

      const nestedItem = container.querySelector(`[data-file-path="/vault/folder1/nested.md"]`);
      if (nestedItem) {
        expect(nestedItem.classList.contains(FileTree.CSS_CLASSES.INDENTED)).toBe(true);
        const paddingLeft = parseInt(nestedItem.style.paddingLeft);
        expect(paddingLeft).toBeGreaterThan(8); // Should have more padding than root level
      }
    });

    it('should emit tree_updated event after rendering', () => {
      const eventSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.TREE_UPDATED, eventSpy);

      fileTree.render(mockFiles);

      expect(eventSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          detail: expect.objectContaining({
            files: mockFiles,
            count: mockFiles.length,
            renderTime: expect.any(Number)
          })
        })
      );
    });
  });

  describe('Folder Expand/Collapse', () => {
    const mockFiles = [
      { name: 'folder1', path: '/vault/folder1', is_dir: true },
      { name: 'nested.md', path: '/vault/folder1/nested.md', is_dir: false }
    ];

    beforeEach(() => {
      appState.currentVault = '/vault';
      fileTree.render(mockFiles);
    });

    it('should expand folder and show children', () => {
      const eventSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.FOLDER_EXPANDED, eventSpy);

      fileTree.expandFolder('/vault/folder1');

      expect(fileTree.expandedFolders.has('/vault/folder1')).toBe(true);
      
      const folderItem = container.querySelector(`[data-file-path="/vault/folder1"]`);
      expect(folderItem.classList.contains(FileTree.CSS_CLASSES.EXPANDED)).toBe(true);
      expect(folderItem.getAttribute('aria-expanded')).toBe('true');
      
      expect(eventSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          detail: { folderPath: '/vault/folder1' }
        })
      );
    });

    it('should collapse folder and hide children', () => {
      fileTree.expandFolder('/vault/folder1'); // First expand it
      
      const eventSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.FOLDER_COLLAPSED, eventSpy);

      fileTree.collapseFolder('/vault/folder1');

      expect(fileTree.expandedFolders.has('/vault/folder1')).toBe(false);
      
      const folderItem = container.querySelector(`[data-file-path="/vault/folder1"]`);
      expect(folderItem.classList.contains(FileTree.CSS_CLASSES.COLLAPSED)).toBe(true);
      expect(folderItem.getAttribute('aria-expanded')).toBe('false');
      
      expect(eventSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          detail: { folderPath: '/vault/folder1' }
        })
      );
    });

    it('should toggle folder state', () => {
      // Initially collapsed
      expect(fileTree.expandedFolders.has('/vault/folder1')).toBe(false);

      fileTree.toggleFolder('/vault/folder1');
      expect(fileTree.expandedFolders.has('/vault/folder1')).toBe(true);

      fileTree.toggleFolder('/vault/folder1');
      expect(fileTree.expandedFolders.has('/vault/folder1')).toBe(false);
    });

    it('should not expand already expanded folder', () => {
      fileTree.expandFolder('/vault/folder1');
      const expandedCount = fileTree.expandedFolders.size;

      fileTree.expandFolder('/vault/folder1'); // Try to expand again
      
      expect(fileTree.expandedFolders.size).toBe(expandedCount);
    });

    it('should not collapse already collapsed folder', () => {
      fileTree.collapseFolder('/vault/folder1'); // Try to collapse when already collapsed
      
      expect(fileTree.expandedFolders.has('/vault/folder1')).toBe(false);
    });
  });

  describe('File Selection', () => {
    const mockFiles = [
      { name: 'file1.md', path: '/vault/file1.md', is_dir: false },
      { name: 'file2.md', path: '/vault/file2.md', is_dir: false }
    ];

    beforeEach(() => {
      appState.currentVault = '/vault';
      fileTree.render(mockFiles);
    });

    it('should select file and update visual state', () => {
      fileTree.selectFile('/vault/file1.md');

      expect(fileTree.selectedFile).toBe('/vault/file1.md');
      
      const selectedItem = container.querySelector(`.${FileTree.CSS_CLASSES.SELECTED}`);
      expect(selectedItem).toBeTruthy();
      expect(selectedItem.getAttribute('data-file-path')).toBe('/vault/file1.md');
      expect(selectedItem.getAttribute('aria-selected')).toBe('true');
    });

    it('should clear previous selection when selecting new file', () => {
      fileTree.selectFile('/vault/file1.md');
      fileTree.selectFile('/vault/file2.md');

      const selectedItems = container.querySelectorAll(`.${FileTree.CSS_CLASSES.SELECTED}`);
      expect(selectedItems.length).toBe(1);
      expect(selectedItems[0].getAttribute('data-file-path')).toBe('/vault/file2.md');
    });

    it('should clear selection when passing null', () => {
      fileTree.selectFile('/vault/file1.md');
      fileTree.selectFile(null);

      expect(fileTree.selectedFile).toBeNull();
      const selectedItems = container.querySelectorAll(`.${FileTree.CSS_CLASSES.SELECTED}`);
      expect(selectedItems.length).toBe(0);
    });

    it('should handle selection of non-existent file gracefully', () => {
      fileTree.selectFile('/vault/nonexistent.md');
      
      expect(fileTree.selectedFile).toBe('/vault/nonexistent.md');
      // No DOM element should be selected since file doesn't exist in tree
      const selectedItems = container.querySelectorAll(`.${FileTree.CSS_CLASSES.SELECTED}`);
      expect(selectedItems.length).toBe(0);
    });
  });

  describe('Event Handling', () => {
    const mockFiles = [
      { name: 'folder1', path: '/vault/folder1', is_dir: true },
      { name: 'file1.md', path: '/vault/file1.md', is_dir: false }
    ];

    beforeEach(() => {
      appState.currentVault = '/vault';
      fileTree.render(mockFiles);
    });

    it('should handle click on file item', () => {
      const eventSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.FILE_SELECTED, eventSpy);

      const fileItem = container.querySelector(`[data-file-path="/vault/file1.md"]`);
      fileItem.click();

      expect(eventSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          detail: { filePath: '/vault/file1.md' }
        })
      );
    });

    it('should handle click on folder item', () => {
      const expandSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.FOLDER_EXPANDED, expandSpy);

      const folderItem = container.querySelector(`[data-file-path="/vault/folder1"]`);
      folderItem.click();

      expect(expandSpy).toHaveBeenCalled();
    });

    it('should handle keyboard navigation', () => {
      const fileItem = container.querySelector(`[data-file-path="/vault/file1.md"]`);
      fileItem.focus();

      // Test Enter key
      const eventSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.FILE_SELECTED, eventSpy);

      const enterEvent = new KeyboardEvent('keydown', { key: 'Enter' });
      fileItem.dispatchEvent(enterEvent);

      expect(eventSpy).toHaveBeenCalled();
    });

    it('should handle space key like Enter', () => {
      const fileItem = container.querySelector(`[data-file-path="/vault/file1.md"]`);
      fileItem.focus();

      const eventSpy = vi.fn();
      container.addEventListener(FileTree.EVENTS.FILE_SELECTED, eventSpy);

      const spaceEvent = new KeyboardEvent('keydown', { key: ' ' });
      fileItem.dispatchEvent(spaceEvent);

      expect(eventSpy).toHaveBeenCalled();
    });
  });

  describe('Search Functionality', () => {
    const mockFiles = [
      { name: 'readme.md', path: '/vault/readme.md', is_dir: false },
      { name: 'notes.md', path: '/vault/notes.md', is_dir: false },
      { name: 'docs', path: '/vault/docs', is_dir: true },
      { name: 'guide.md', path: '/vault/docs/guide.md', is_dir: false }
    ];

    beforeEach(() => {
      appState.currentVault = '/vault';
      fileTree.render(mockFiles);
    });

    it('should activate search mode', () => {
      fileTree.activateSearch();

      expect(fileTree.isSearchActive).toBe(true);
      const searchContainer = container.querySelector('.file-tree-search-container');
      expect(searchContainer.style.display).toBe('flex');
      
      const searchInput = container.querySelector('.file-tree-search-input');
      expect(document.activeElement).toBe(searchInput);
    });

    it('should deactivate search mode', () => {
      fileTree.activateSearch();
      fileTree.deactivateSearch();

      expect(fileTree.isSearchActive).toBe(false);
      const searchContainer = container.querySelector('.file-tree-search-container');
      expect(searchContainer.style.display).toBe('none');
      
      const searchInput = container.querySelector('.file-tree-search-input');
      expect(searchInput.value).toBe('');
    });

    it('should perform fuzzy search on file names', () => {
      const searchResults = fileTree.fuzzySearch(mockFiles, 'read');
      
      expect(searchResults.length).toBeGreaterThan(0);
      expect(searchResults[0].file.name).toBe('readme.md'); // Should match best
      expect(searchResults[0].score).toBeGreaterThan(0);
    });

    it('should calculate fuzzy scores correctly', () => {
      const exactMatchScore = fileTree.calculateFuzzyScore('readme', 'readme');
      const partialMatchScore = fileTree.calculateFuzzyScore('readme', 'read');
      const noMatchScore = fileTree.calculateFuzzyScore('readme', 'xyz');

      expect(exactMatchScore).toBeGreaterThan(partialMatchScore);
      expect(partialMatchScore).toBeGreaterThan(noMatchScore);
      expect(noMatchScore).toBe(0);
    });

    it('should highlight matching text in search results', () => {
      const highlighted = fileTree.highlightText('readme.md', 'read');
      expect(highlighted).toBe('<mark>read</mark>me.md');
    });

    it('should escape regex characters in search term', () => {
      const escaped = fileTree.escapeRegex('test.file');
      expect(escaped).toBe('test\\.file');
    });

    it('should handle empty search term', () => {
      fileTree.performSearch('');
      
      // Should show all files when search is empty
      const treeItems = container.querySelectorAll(`.${FileTree.CSS_CLASSES.TREE_ITEM}`);
      expect(treeItems.length).toBeGreaterThan(0);
    });

    it('should show no results message for non-matching search', () => {
      fileTree.performSearch('nonexistentfile');
      
      const noResults = container.querySelector('.file-tree-no-results');
      expect(noResults).toBeTruthy();
      expect(noResults.textContent).toContain('No files found matching "nonexistentfile"');
    });
  });

  describe('Performance', () => {
    it('should handle large file lists efficiently', () => {
      const largeFileList = Array.from({ length: 1000 }, (_, i) => ({
        name: `file${i}.md`,
        path: `/vault/file${i}.md`,
        is_dir: false
      }));

      const startTime = performance.now();
      fileTree.render(largeFileList);
      const renderTime = performance.now() - startTime;

      // Should complete rendering in reasonable time
      expect(renderTime).toBeLessThan(1000); // 1 second for 1000 files

      // Should enable virtual scrolling for large trees
      expect(fileTree.isVirtualScrolling).toBe(true);
    });

    it('should track performance metrics', () => {
      const mockFiles = [
        { name: 'file1.md', path: '/vault/file1.md', is_dir: false },
        { name: 'file2.md', path: '/vault/file2.md', is_dir: false }
      ];

      fileTree.render(mockFiles);

      expect(fileTree.performanceMetrics.lastRenderTime).toBeGreaterThan(0);
      expect(fileTree.performanceMetrics.renderCount).toBe(1);
      expect(fileTree.performanceMetrics.averageRenderTime).toBeGreaterThan(0);
    });

    it('should check virtual scrolling based on file count', () => {
      // Small file list should not enable virtual scrolling
      const smallFiles = Array.from({ length: 10 }, (_, i) => ({
        name: `file${i}.md`,
        path: `/vault/file${i}.md`,
        is_dir: false
      }));

      fileTree.render(smallFiles);
      expect(fileTree.isVirtualScrolling).toBe(false);

      // Large file list should enable virtual scrolling
      const largeFiles = Array.from({ length: 2000 }, (_, i) => ({
        name: `file${i}.md`,
        path: `/vault/file${i}.md`,
        is_dir: false
      }));

      fileTree.render(largeFiles);
      expect(fileTree.isVirtualScrolling).toBe(true);
    });
  });

  describe('Accessibility', () => {
    const mockFiles = [
      { name: 'folder1', path: '/vault/folder1', is_dir: true },
      { name: 'file1.md', path: '/vault/file1.md', is_dir: false }
    ];

    beforeEach(() => {
      appState.currentVault = '/vault';
      fileTree.render(mockFiles);
    });

    it('should have proper ARIA attributes on container', () => {
      expect(container.getAttribute('role')).toBe('tree');
      expect(container.getAttribute('aria-label')).toBe('File tree navigation');
    });

    it('should have proper ARIA attributes on tree items', () => {
      const fileItem = container.querySelector(`[data-file-path="/vault/file1.md"]`);
      expect(fileItem.getAttribute('role')).toBe('treeitem');
      expect(fileItem.getAttribute('aria-label')).toContain('File: file1.md');
      expect(fileItem.getAttribute('tabindex')).toBe('0');

      const folderItem = container.querySelector(`[data-file-path="/vault/folder1"]`);
      expect(folderItem.getAttribute('role')).toBe('treeitem');
      expect(folderItem.getAttribute('aria-label')).toContain('Folder: folder1');
      expect(folderItem.getAttribute('aria-expanded')).toBe('false');
    });

    it('should update aria-expanded when folder state changes', () => {
      const folderItem = container.querySelector(`[data-file-path="/vault/folder1"]`);
      
      fileTree.expandFolder('/vault/folder1');
      expect(folderItem.getAttribute('aria-expanded')).toBe('true');

      fileTree.collapseFolder('/vault/folder1');
      expect(folderItem.getAttribute('aria-expanded')).toBe('false');
    });

    it('should update aria-selected when file is selected', () => {
      const fileItem = container.querySelector(`[data-file-path="/vault/file1.md"]`);
      
      fileTree.selectFile('/vault/file1.md');
      expect(fileItem.getAttribute('aria-selected')).toBe('true');

      fileTree.selectFile(null);
      expect(fileItem.hasAttribute('aria-selected')).toBe(false);
    });
  });

  describe('Component Lifecycle', () => {
    it('should initialize event listeners on creation', () => {
      expect(fileTree.eventListeners.size).toBeGreaterThan(0);
      expect(fileTree.eventListeners.has('click')).toBe(true);
      expect(fileTree.eventListeners.has('contextmenu')).toBe(true);
      expect(fileTree.eventListeners.has('keydown')).toBe(true);
    });

    it('should clean up resources on destroy', () => {
      const initialSize = fileTree.eventListeners.size;
      expect(initialSize).toBeGreaterThan(0);

      fileTree.destroy();

      expect(fileTree.eventListeners.size).toBe(0);
      expect(fileTree.files).toEqual([]);
      expect(fileTree.expandedFolders.size).toBe(0);
      expect(fileTree.selectedFile).toBeNull();
      expect(fileTree.treeStructure.size).toBe(0);
      expect(container.innerHTML).toBe('');
    });

    it('should handle app state changes', () => {
      const newFiles = [
        { name: 'new.md', path: '/vault/new.md', is_dir: false }
      ];

      // Simulate app state files update
      appState.setFiles(newFiles);

      // FileTree should have received the update through event listeners
      // Note: This tests the integration with AppState events
      expect(fileTree.files).toEqual(newFiles);
    });
  });

  describe('Utilities', () => {
    beforeEach(() => {
      appState.currentVault = '/vault';
    });

    it('should calculate correct parent path', () => {
      expect(fileTree.getParentPath('folder/file.md')).toBe('folder');
      expect(fileTree.getParentPath('file.md')).toBe('');
      expect(fileTree.getParentPath('deep/nested/file.md')).toBe('deep/nested');
    });

    it('should get correct relative path', () => {
      expect(fileTree.getRelativePath('/vault/file.md')).toBe('file.md');
      expect(fileTree.getRelativePath('/vault/folder/file.md')).toBe('folder/file.md');
      expect(fileTree.getRelativePath('/different/path/file.md')).toBe('/different/path/file.md');
    });

    it('should calculate correct depth', () => {
      expect(fileTree.calculateDepth('/vault/file.md')).toBe(0); // Root level
      expect(fileTree.calculateDepth('/vault/folder/file.md')).toBe(1);
      expect(fileTree.calculateDepth('/vault/deep/nested/file.md')).toBe(2);
    });

    it('should get appropriate file icon class', () => {
      expect(fileTree.getFileIconClass('test.md')).toBe('file-md');
      expect(fileTree.getFileIconClass('script.js')).toBe('file-js');
      expect(fileTree.getFileIconClass('style.css')).toBe('file-css');
      expect(fileTree.getFileIconClass('unknown.ext')).toBe('file-default');
    });

    it('should find tree items by path', () => {
      const mockFiles = [
        { name: 'test.md', path: '/vault/test.md', is_dir: false }
      ];
      fileTree.render(mockFiles);

      const item = fileTree.findTreeItem('/vault/test.md');
      expect(item).toBeTruthy();
      expect(item.getAttribute('data-file-path')).toBe('/vault/test.md');

      const nonExistent = fileTree.findTreeItem('/vault/nonexistent.md');
      expect(nonExistent).toBeNull();
    });
  });
});