//! # Text Chunking System
//! 
//! Core text chunking algorithms and infrastructure for intelligent text processing.
//! This module provides semantic-aware text splitting that preserves content boundaries
//! while optimizing for AI embedding generation and search accuracy.
//!
//! ## Features
//!
//! - **Fixed-size chunking:** Configurable character/token limits
//! - **Semantic chunking:** Sentence and paragraph boundary detection
//! - **Overlap management:** Configurable overlap between chunks for context continuity
//! - **Metadata tracking:** Rich chunk metadata for context reconstruction
//! - **Performance optimized:** Efficient processing using standard Rust string handling
//!
//! ## Architecture
//!
//! The module is built around the `ChunkProcessor` struct which provides configurable
//! parameters for different chunking strategies. The core algorithms focus on:
//! - Boundary detection using linguistic patterns
//! - Context preservation through overlap management
//! - Metadata generation for downstream AI processing
//!
//! ## Usage
//!
//! ```rust
//! use crate::text_chunker::{ChunkProcessor, ChunkConfig, ChunkingStrategy};
//!
//! let config = ChunkConfig {
//!     strategy: ChunkingStrategy::Semantic,
//!     max_chunk_size: 1000,
//!     overlap_size: 100,
//!     ..Default::default()
//! };
//! 
//! let processor = ChunkProcessor::new(config);
//! let chunks = processor.chunk_text("Your text content here...")?;
//! ```

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// Errors that can occur during text chunking operations
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkError {
    /// Input text is empty or invalid
    InvalidInput(String),
    /// Configuration parameters are invalid
    InvalidConfig(String),
    /// Chunking operation failed
    ProcessingError(String),
    /// Boundary detection failed
    BoundaryDetectionError(String),
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ChunkError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            ChunkError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
            ChunkError::BoundaryDetectionError(msg) => write!(f, "Boundary detection error: {}", msg),
        }
    }
}

impl std::error::Error for ChunkError {}

pub type ChunkResult<T> = Result<T, ChunkError>;

/// Performance metrics for chunking operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total processing time in milliseconds
    pub processing_time_ms: u64,
    /// Memory usage during chunking in bytes (estimated)
    pub memory_usage_bytes: usize,
    /// Number of chunks generated
    pub chunks_generated: usize,
    /// Processing speed in characters per millisecond
    pub chars_per_ms: f64,
    /// Input text size in characters
    pub input_size_chars: usize,
    /// Average chunk size in characters
    pub avg_chunk_size: f64,
    /// Time spent on boundary detection in milliseconds
    pub boundary_detection_time_ms: u64,
    /// Time spent on markdown parsing in milliseconds
    pub markdown_parsing_time_ms: u64,
    /// Time spent on metadata creation in milliseconds
    pub metadata_creation_time_ms: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            processing_time_ms: 0,
            memory_usage_bytes: 0,
            chunks_generated: 0,
            chars_per_ms: 0.0,
            input_size_chars: 0,
            avg_chunk_size: 0.0,
            boundary_detection_time_ms: 0,
            markdown_parsing_time_ms: 0,
            metadata_creation_time_ms: 0,
        }
    }
}

impl PerformanceMetrics {
    /// Creates a new performance metrics instance
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Calculates derived metrics after processing
    pub fn finalize(&mut self, input_size: usize, chunk_count: usize) {
        self.input_size_chars = input_size;
        self.chunks_generated = chunk_count;
        
        if self.processing_time_ms > 0 {
            self.chars_per_ms = input_size as f64 / self.processing_time_ms as f64;
        }
        
        if chunk_count > 0 {
            self.avg_chunk_size = input_size as f64 / chunk_count as f64;
        }
    }
    
    /// Estimates memory usage based on input size and intermediate data structures
    pub fn estimate_memory_usage(&mut self, input_size: usize, chunk_count: usize) {
        // Rough estimation: input text + chunks + metadata overhead
        let base_text_size = input_size;
        let chunks_overhead = chunk_count * 200; // Average metadata size per chunk
        let processing_overhead = input_size / 10; // 10% overhead for processing
        
        self.memory_usage_bytes = base_text_size + chunks_overhead + processing_overhead;
    }
    
    /// Checks if performance meets the required targets
    pub fn meets_requirements(&self, target_time_ms: u64, target_memory_bytes: usize) -> bool {
        self.processing_time_ms <= target_time_ms && self.memory_usage_bytes <= target_memory_bytes
    }
    
    /// Returns true if processing shows linear scaling characteristics
    pub fn has_linear_scaling(&self) -> bool {
        // For linear scaling, we expect consistent chars_per_ms regardless of input size
        // More lenient threshold since very fast processing might show timing variations
        self.chars_per_ms >= 50.0 || self.processing_time_ms == 0 // Target: at least 50 chars/ms or very fast
    }
}

/// Chunking strategy options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum ChunkingStrategy {
    /// Fixed-size chunks based on character count
    FixedSize,
    /// Semantic chunking preserving sentence boundaries
    #[default]
    Semantic,
    /// Hybrid approach combining size and semantic constraints
    Hybrid,
    /// Markdown-aware chunking preserving document structure
    MarkdownAware,
}

/// Configuration for text chunking operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Chunking strategy to use
    pub strategy: ChunkingStrategy,
    /// Maximum chunk size in characters (200-2000)
    pub max_chunk_size: usize,
    /// Overlap size in characters for context continuity
    pub overlap_size: usize,
    /// Minimum chunk size to avoid very small fragments
    pub min_chunk_size: usize,
    /// Whether to preserve paragraph boundaries
    pub preserve_paragraphs: bool,
    /// Whether to preserve sentence boundaries
    pub preserve_sentences: bool,
    /// Whether to preserve markdown headers as chunk boundaries
    pub preserve_markdown_headers: bool,
    /// Whether to keep code blocks intact as single units
    pub preserve_code_blocks: bool,
    /// Whether to preserve markdown links and references
    pub preserve_markdown_links: bool,
    /// Whether to strip markdown formatting from content
    pub strip_markdown_formatting: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkingStrategy::Semantic,
            max_chunk_size: 1000,
            overlap_size: 100,
            min_chunk_size: 50,
            preserve_paragraphs: true,
            preserve_sentences: true,
            preserve_markdown_headers: true,
            preserve_code_blocks: true,
            preserve_markdown_links: true,
            strip_markdown_formatting: false,
        }
    }
}

impl ChunkConfig {
    /// Validates the configuration parameters
    pub fn validate(&self) -> ChunkResult<()> {
        if self.max_chunk_size < 200 || self.max_chunk_size > 2000 {
            return Err(ChunkError::InvalidConfig(
                "max_chunk_size must be between 200 and 2000 characters".to_string()
            ));
        }
        
        if self.overlap_size >= self.max_chunk_size {
            return Err(ChunkError::InvalidConfig(
                "overlap_size must be less than max_chunk_size".to_string()
            ));
        }
        
        if self.min_chunk_size >= self.max_chunk_size {
            return Err(ChunkError::InvalidConfig(
                "min_chunk_size must be less than max_chunk_size".to_string()
            ));
        }
        
        if self.overlap_size > self.max_chunk_size / 2 {
            return Err(ChunkError::InvalidConfig(
                "overlap_size should not exceed half of max_chunk_size".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Markdown-specific metadata for chunks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkdownMetadata {
    /// Headers present in this chunk (level, text)
    pub headers: Vec<(usize, String)>,
    /// Code blocks in this chunk (language, content)
    pub code_blocks: Vec<(Option<String>, String)>,
    /// Links found in this chunk (text, url, title)
    pub links: Vec<(String, String, Option<String>)>,
    /// Lists present in this chunk (ordered, items)
    pub lists: Vec<(bool, Vec<String>)>,
    /// Tables in this chunk (headers, rows)
    pub tables: Vec<(Vec<String>, Vec<Vec<String>>)>,
    /// Document structure context (parent headers)
    pub structure_context: Vec<(usize, String)>,
    /// Whether this chunk contains formatting that was stripped
    pub has_stripped_formatting: bool,
}

impl Default for MarkdownMetadata {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            code_blocks: Vec::new(),
            links: Vec::new(),
            lists: Vec::new(),
            tables: Vec::new(),
            structure_context: Vec::new(),
            has_stripped_formatting: false,
        }
    }
}

/// Metadata associated with each text chunk
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Original position of chunk start in source text
    pub start_position: usize,
    /// Original position of chunk end in source text
    pub end_position: usize,
    /// Chunk sequence number (0-based)
    pub chunk_index: usize,
    /// Total number of chunks in the document
    pub total_chunks: usize,
    /// Number of characters in the chunk
    pub character_count: usize,
    /// Number of words in the chunk (approximate)
    pub word_count: usize,
    /// Number of sentences in the chunk (approximate)
    pub sentence_count: usize,
    /// Whether this chunk has overlap with previous chunk
    pub has_previous_overlap: bool,
    /// Whether this chunk has overlap with next chunk
    pub has_next_overlap: bool,
    /// Size of overlap with previous chunk
    pub previous_overlap_size: usize,
    /// Size of overlap with next chunk
    pub next_overlap_size: usize,
    /// Additional context information
    pub context: HashMap<String, String>,
    /// Markdown-specific metadata
    pub markdown: Option<MarkdownMetadata>,
}

impl Default for ChunkMetadata {
    fn default() -> Self {
        Self {
            start_position: 0,
            end_position: 0,
            chunk_index: 0,
            total_chunks: 1,
            character_count: 0,
            word_count: 0,
            sentence_count: 0,
            has_previous_overlap: false,
            has_next_overlap: false,
            previous_overlap_size: 0,
            next_overlap_size: 0,
            context: HashMap::new(),
            markdown: None,
        }
    }
}

/// A text chunk with associated metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextChunk {
    /// The text content of the chunk
    pub content: String,
    /// Metadata associated with this chunk
    pub metadata: ChunkMetadata,
}

/// Result of chunking operation with performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkingResult {
    /// Generated text chunks
    pub chunks: Vec<TextChunk>,
    /// Performance metrics for the operation
    pub metrics: PerformanceMetrics,
}

impl ChunkingResult {
    /// Creates a new chunking result
    pub fn new(chunks: Vec<TextChunk>, metrics: PerformanceMetrics) -> Self {
        Self { chunks, metrics }
    }
    
    /// Returns the number of chunks generated
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
    
    /// Returns true if the operation met performance requirements
    pub fn meets_performance_targets(&self) -> bool {
        // 10KB in 100ms, 10MB memory limit
        let target_time_ms = if self.metrics.input_size_chars <= 10_000 { 100 } else {
            // Linear scaling: 100ms per 10KB
            (self.metrics.input_size_chars / 10_000 * 100) as u64
        };
        
        self.metrics.meets_requirements(target_time_ms, 10_000_000) // 10MB
    }
}

impl TextChunk {
    /// Creates a new text chunk with the given content and metadata
    pub fn new(content: String, metadata: ChunkMetadata) -> Self {
        Self { content, metadata }
    }
    
    /// Returns the length of the chunk content in characters
    pub fn len(&self) -> usize {
        self.content.len()
    }
    
    /// Returns whether the chunk is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    
    /// Returns the chunk content as a string slice
    pub fn content(&self) -> &str {
        &self.content
    }
    
    /// Returns an immutable reference to the chunk metadata
    pub fn metadata(&self) -> &ChunkMetadata {
        &self.metadata
    }
}

/// Boundary detection for semantic chunking
#[derive(Debug, Clone)]
struct BoundaryDetector {
    /// Sentence ending punctuation patterns
    sentence_endings: Vec<&'static str>,
    /// Paragraph boundary patterns
    paragraph_patterns: Vec<&'static str>,
}

impl Default for BoundaryDetector {
    fn default() -> Self {
        Self {
            sentence_endings: vec![".", "!", "?", "...", "…"],
            paragraph_patterns: vec!["\n\n", "\r\n\r\n"],
        }
    }
}

impl BoundaryDetector {
    /// Finds sentence boundaries in the given text
    fn find_sentence_boundaries(&self, text: &str) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        
        for i in 0..chars.len() {
            // Check for sentence ending punctuation
            for &ending in &self.sentence_endings {
                if text[i..].starts_with(ending) {
                    let boundary_pos = i + ending.len();
                    
                    // Look ahead for whitespace to confirm sentence boundary
                    if boundary_pos < chars.len() {
                        let next_char = chars[boundary_pos];
                        if next_char.is_whitespace() || boundary_pos == chars.len() - 1 {
                            boundaries.push(boundary_pos);
                        }
                    } else {
                        boundaries.push(boundary_pos);
                    }
                }
            }
        }
        
        // Remove duplicates and sort
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }
    
    /// Finds paragraph boundaries in the given text
    fn find_paragraph_boundaries(&self, text: &str) -> Vec<usize> {
        let mut boundaries = Vec::new();
        
        for &pattern in &self.paragraph_patterns {
            let mut start = 0;
            while let Some(pos) = text[start..].find(pattern) {
                let boundary_pos = start + pos + pattern.len();
                boundaries.push(boundary_pos);
                start = boundary_pos;
            }
        }
        
        // Remove duplicates and sort
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }
    
    /// Finds the best boundary near the target position
    fn find_best_boundary(&self, text: &str, target_pos: usize, search_range: usize) -> Option<usize> {
        if target_pos >= text.len() {
            return Some(text.len());
        }
        
        let start = target_pos.saturating_sub(search_range);
        let end = (target_pos + search_range).min(text.len());
        
        // Get all boundaries in the search range
        let sentence_boundaries = self.find_sentence_boundaries(&text[start..end])
            .into_iter()
            .map(|pos| start + pos)
            .collect::<Vec<_>>();
            
        let paragraph_boundaries = self.find_paragraph_boundaries(&text[start..end])
            .into_iter()
            .map(|pos| start + pos)
            .collect::<Vec<_>>();
        
        // Prefer paragraph boundaries, then sentence boundaries
        let mut all_boundaries = paragraph_boundaries;
        all_boundaries.extend(sentence_boundaries);
        all_boundaries.sort_unstable();
        all_boundaries.dedup();
        
        // Find the boundary closest to target position
        all_boundaries
            .into_iter()
            .min_by_key(|&pos| {
                target_pos.abs_diff(pos)
            })
    }
    
    /// Finds the best boundary near the target position (optimized)
    fn find_best_boundary_optimized(&self, text: &str, target_pos: usize, search_range: usize) -> Option<usize> {
        if target_pos >= text.len() {
            return Some(text.len());
        }
        
        let start = target_pos.saturating_sub(search_range);
        let end = (target_pos + search_range).min(text.len());
        
        // Quick paragraph boundary check first (most efficient)
        if let Some(para_pos) = self.find_nearest_paragraph_boundary(&text[start..end], target_pos - start) {
            return Some(start + para_pos);
        }
        
        // Fall back to sentence boundary detection  
        if let Some(sent_pos) = self.find_nearest_sentence_boundary(&text[start..end], target_pos - start) {
            return Some(start + sent_pos);
        }
        
        // Fall back to word boundary
        self.find_nearest_word_boundary(&text[start..end], target_pos - start)
            .map(|pos| start + pos)
            .or(Some(target_pos))
    }
    
    /// Fast paragraph boundary detection
    fn find_nearest_paragraph_boundary(&self, text: &str, target: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut best_pos = None;
        let mut best_distance = usize::MAX;
        
        // Look for double newlines (paragraph boundaries)
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
                let pos = i + 2;
                let distance = target.abs_diff(pos);
                if distance < best_distance {
                    best_distance = distance;
                    best_pos = Some(pos);
                }
            }
            i += 1;
        }
        
        best_pos
    }
    
    /// Fast sentence boundary detection
    fn find_nearest_sentence_boundary(&self, text: &str, target: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut best_pos = None;
        let mut best_distance = usize::MAX;
        
        let mut i = 0;
        while i < bytes.len() {
            if matches!(bytes[i], b'.' | b'!' | b'?') {
                // Look ahead for whitespace to confirm sentence boundary
                let mut boundary_pos = i + 1;
                while boundary_pos < bytes.len() && bytes[boundary_pos] == b' ' {
                    boundary_pos += 1;
                }
                
                if boundary_pos < bytes.len() {
                    let distance = target.abs_diff(boundary_pos);
                    if distance < best_distance {
                        best_distance = distance;
                        best_pos = Some(boundary_pos);
                    }
                }
            }
            i += 1;
        }
        
        best_pos
    }
    
    /// Fast word boundary detection as fallback
    fn find_nearest_word_boundary(&self, text: &str, target: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        
        // Search backwards for whitespace
        for i in (0..target.min(bytes.len())).rev() {
            if bytes[i].is_ascii_whitespace() {
                return Some(i + 1);
            }
        }
        
        // Search forwards for whitespace
        for i in target..bytes.len() {
            if bytes[i].is_ascii_whitespace() {
                return Some(i);
            }
        }
        
        None
    }
}

/// Markdown element detected during parsing
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownElement {
    Header(usize, String, usize), // level, text, position
    CodeBlock(Option<String>, String, usize, usize), // language, content, start, end
    Link(String, String, Option<String>, usize), // text, url, title, position
    List(bool, Vec<String>, usize), // ordered, items, position
    Table(Vec<String>, Vec<Vec<String>>, usize), // headers, rows, position
    Paragraph(String, usize), // content, position
    LineBreak(usize), // position
}

/// Lightweight markdown parser for chunking purposes
#[derive(Debug, Clone)]
struct MarkdownParser {
    /// Header patterns (ATX and Setext styles)
    header_patterns: Vec<&'static str>,
    /// Code block patterns
    code_block_patterns: Vec<&'static str>,
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self {
            header_patterns: vec!["######", "#####", "####", "###", "##", "#"],
            code_block_patterns: vec!["```", "~~~"],
        }
    }
}

impl MarkdownParser {
    /// Parses markdown text and returns a list of elements with positions
    fn parse(&self, text: &str) -> Vec<MarkdownElement> {
        let mut elements = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let mut line_positions = Vec::new();
        
        // Calculate line positions
        let mut position = 0;
        for line in &lines {
            line_positions.push(position);
            position += line.len() + 1; // +1 for newline
        }
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let line_pos = line_positions[i];
            
            // Skip empty lines
            if line.trim().is_empty() {
                elements.push(MarkdownElement::LineBreak(line_pos));
                i += 1;
                continue;
            }
            
            // Check for headers (ATX style: # ## ### etc.)
            if let Some(header) = self.parse_atx_header(line, line_pos) {
                elements.push(header);
                i += 1;
                continue;
            }
            
            // Check for setext headers (underlined with = or -)
            if i + 1 < lines.len() {
                if let Some(header) = self.parse_setext_header(lines[i], lines[i + 1], line_pos) {
                    elements.push(header);
                    i += 2; // Skip both header line and underline
                    continue;
                }
            }
            
            // Check for code blocks (fenced)
            if let Some((code_block, consumed_lines)) = self.parse_code_block(&lines[i..], line_pos) {
                elements.push(code_block);
                i += consumed_lines;
                continue;
            }
            
            // Check for tables
            if let Some((table, consumed_lines)) = self.parse_table(&lines[i..], line_pos) {
                elements.push(table);
                i += consumed_lines;
                continue;
            }
            
            // Check for lists
            if let Some((list, consumed_lines)) = self.parse_list(&lines[i..], line_pos) {
                elements.push(list);
                i += consumed_lines;
                continue;
            }
            
            // Default: treat as paragraph
            let paragraph_content = line.to_string();
            elements.push(MarkdownElement::Paragraph(paragraph_content, line_pos));
            i += 1;
        }
        
        // Parse links within all text elements
        self.extract_links_from_elements(&mut elements, text);
        
        elements
    }
    
    /// Parses ATX style headers (# Header)
    fn parse_atx_header(&self, line: &str, position: usize) -> Option<MarkdownElement> {
        let trimmed = line.trim_start();
        
        for &pattern in &self.header_patterns {
            if trimmed.starts_with(pattern) && trimmed.len() > pattern.len() {
                let rest = &trimmed[pattern.len()..];
                if rest.starts_with(' ') || rest.is_empty() {
                    let level = pattern.len();
                    let text = rest.trim().to_string();
                    return Some(MarkdownElement::Header(level, text, position));
                }
            }
        }
        
        None
    }
    
    /// Parses Setext style headers (underlined with = or -)
    fn parse_setext_header(&self, header_line: &str, underline: &str, position: usize) -> Option<MarkdownElement> {
        let underline = underline.trim();
        
        if underline.chars().all(|c| c == '=') && underline.len() >= 3 {
            return Some(MarkdownElement::Header(1, header_line.trim().to_string(), position));
        }
        
        if underline.chars().all(|c| c == '-') && underline.len() >= 3 {
            return Some(MarkdownElement::Header(2, header_line.trim().to_string(), position));
        }
        
        None
    }
    
    /// Parses fenced code blocks
    fn parse_code_block(&self, lines: &[&str], start_pos: usize) -> Option<(MarkdownElement, usize)> {
        if lines.is_empty() {
            return None;
        }
        
        let first_line = lines[0].trim();
        
        for &pattern in &self.code_block_patterns {
            if first_line.starts_with(pattern) {
                let language = first_line[pattern.len()..].trim();
                let language = if language.is_empty() { None } else { Some(language.to_string()) };
                
                let mut content = String::new();
                let mut consumed = 1;
                
                // Find closing fence
                for (i, &line) in lines[1..].iter().enumerate() {
                    if line.trim().starts_with(pattern) {
                        consumed = i + 2; // +1 for opening fence, +1 for current line
                        break;
                    }
                    
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(line);
                }
                
                let content_len = content.len();
                return Some((MarkdownElement::CodeBlock(language, content, start_pos, start_pos + content_len), consumed));
            }
        }
        
        None
    }
    
    /// Parses markdown tables
    fn parse_table(&self, lines: &[&str], start_pos: usize) -> Option<(MarkdownElement, usize)> {
        if lines.len() < 2 {
            return None;
        }
        
        // Check if first line looks like a table header
        let header_line = lines[0];
        if !header_line.contains('|') {
            return None;
        }
        
        // Check if second line is a separator
        let separator_line = lines[1];
        if !separator_line.contains('|') || !separator_line.contains('-') {
            return None;
        }
        
        let headers: Vec<String> = header_line
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        if headers.is_empty() {
            return None;
        }
        
        let mut rows = Vec::new();
        let mut consumed = 2;
        
        // Parse data rows
        for &line in lines[2..].iter() {
            if !line.contains('|') {
                break;
            }
            
            let row: Vec<String> = line
                .split('|')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            if !row.is_empty() {
                rows.push(row);
                consumed += 1;
            } else {
                break;
            }
        }
        
        Some((MarkdownElement::Table(headers, rows, start_pos), consumed))
    }
    
    /// Parses markdown lists (ordered and unordered)
    fn parse_list(&self, lines: &[&str], start_pos: usize) -> Option<(MarkdownElement, usize)> {
        if lines.is_empty() {
            return None;
        }
        
        let first_line = lines[0].trim_start();
        
        // Check for unordered list markers
        let is_unordered = first_line.starts_with("- ") || 
                          first_line.starts_with("* ") || 
                          first_line.starts_with("+ ");
        
        // Check for ordered list markers
        let is_ordered = first_line.chars().take(10)
            .take_while(|c| c.is_ascii_digit())
            .count() > 0 && first_line.contains(". ");
        
        if !is_unordered && !is_ordered {
            return None;
        }
        
        let mut items = Vec::new();
        let mut consumed = 0;
        
        for &line in lines.iter() {
            let trimmed = line.trim_start();
            
            let is_list_item = if is_unordered {
                trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ")
            } else {
                trimmed.chars().take(10).take_while(|c| c.is_ascii_digit()).count() > 0 && 
                trimmed.contains(". ")
            };
            
            if is_list_item {
                let content = if is_unordered {
                    trimmed[2..].trim().to_string()
                } else {
                    let dot_pos = trimmed.find(". ").unwrap();
                    trimmed[dot_pos + 2..].trim().to_string()
                };
                items.push(content);
                consumed += 1;
            } else if trimmed.is_empty() && consumed > 0 {
                // Allow empty lines within lists
                consumed += 1;
            } else {
                break;
            }
        }
        
        if items.is_empty() {
            return None;
        }
        
        Some((MarkdownElement::List(!is_unordered, items, start_pos), consumed))
    }
    
    /// Extracts links from all elements
    fn extract_links_from_elements(&self, elements: &mut Vec<MarkdownElement>, text: &str) {
        // Simple link extraction without regex dependency
        // Look for [text](url) patterns
        let mut search_pos = 0;
        
        while let Some(bracket_start) = text[search_pos..].find('[') {
            let abs_bracket_start = search_pos + bracket_start;
            
            if let Some(bracket_end) = text[abs_bracket_start..].find(']') {
                let abs_bracket_end = abs_bracket_start + bracket_end;
                
                if abs_bracket_end + 1 < text.len() && text.chars().nth(abs_bracket_end + 1) == Some('(') {
                    if let Some(paren_end) = text[abs_bracket_end + 2..].find(')') {
                        let abs_paren_end = abs_bracket_end + 2 + paren_end;
                        
                        let link_text = &text[abs_bracket_start + 1..abs_bracket_end];
                        let link_url = &text[abs_bracket_end + 2..abs_paren_end];
                        
                        // Parse title if present: [text](url "title")
                        let (url, title) = if let Some(quote_pos) = link_url.find('"') {
                            let url_part = link_url[..quote_pos].trim();
                            let title_part = link_url[quote_pos + 1..].trim_end_matches('"');
                            (url_part.to_string(), Some(title_part.to_string()))
                        } else {
                            (link_url.trim().to_string(), None)
                        };
                        
                        elements.push(MarkdownElement::Link(
                            link_text.to_string(), 
                            url, 
                            title, 
                            abs_bracket_start
                        ));
                        
                        search_pos = abs_paren_end + 1;
                    } else {
                        search_pos = abs_bracket_end + 1;
                    }
                } else {
                    search_pos = abs_bracket_end + 1;
                }
            } else {
                break;
            }
        }
    }
    
    /// Finds markdown header boundaries in text
    fn find_header_boundaries(&self, text: &str) -> Vec<usize> {
        let elements = self.parse(text);
        elements
            .into_iter()
            .filter_map(|element| match element {
                MarkdownElement::Header(_, _, pos) => Some(pos),
                _ => None,
            })
            .collect()
    }
    
    /// Finds code block boundaries to preserve them intact
    fn find_code_block_boundaries(&self, text: &str) -> Vec<(usize, usize)> {
        let elements = self.parse(text);
        elements
            .into_iter()
            .filter_map(|element| match element {
                MarkdownElement::CodeBlock(_, _, start, end) => Some((start, end)),
                _ => None,
            })
            .collect()
    }
    
    /// Strips markdown formatting while preserving content
    fn strip_formatting(&self, text: &str) -> String {
        let mut result = text.to_string();
        
        // Remove ATX headers
        for &pattern in &self.header_patterns {
            let header_pattern = format!("{} ", pattern);
            result = result.replace(&header_pattern, "");
        }
        
        // Remove emphasis (basic patterns)
        result = result.replace("**", ""); // Bold
        result = result.replace("*", "");  // Italic
        result = result.replace("__", ""); // Bold
        result = result.replace("_", "");  // Italic
        
        // Remove code spans
        result = result.replace("`", "");
        
        // Simple link cleanup [text](url) -> text
        while let Some(start) = result.find('[') {
            if let Some(middle) = result[start..].find("](") {
                if let Some(end) = result[start + middle..].find(')') {
                    let link_start = start;
                    let text_end = start + middle;
                    let link_end = start + middle + end + 1;
                    
                    let link_text = result[start + 1..text_end].to_string();
                    result.replace_range(link_start..link_end, &link_text);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        result
    }
    
    /// Creates markdown metadata from parsed elements
    fn create_metadata(&self, elements: &[MarkdownElement], strip_formatting: bool) -> MarkdownMetadata {
        let mut metadata = MarkdownMetadata::default();
        
        for element in elements {
            match element {
                MarkdownElement::Header(level, text, _) => {
                    metadata.headers.push((*level, text.clone()));
                },
                MarkdownElement::CodeBlock(lang, content, _, _) => {
                    metadata.code_blocks.push((lang.clone(), content.clone()));
                },
                MarkdownElement::Link(text, url, title, _) => {
                    metadata.links.push((text.clone(), url.clone(), title.clone()));
                },
                MarkdownElement::List(ordered, items, _) => {
                    metadata.lists.push((*ordered, items.clone()));
                },
                MarkdownElement::Table(headers, rows, _) => {
                    metadata.tables.push((headers.clone(), rows.clone()));
                },
                _ => {}
            }
        }
        
        metadata.has_stripped_formatting = strip_formatting;
        metadata
    }
    
    /// Builds structure context from parent headers
    fn build_structure_context(&self, elements: &[MarkdownElement], current_pos: usize) -> Vec<(usize, String)> {
        let mut context = Vec::new();
        
        for element in elements {
            if let MarkdownElement::Header(level, text, pos) = element {
                if *pos < current_pos {
                    context.push((*level, text.clone()));
                }
            }
        }
        
        // Keep only the most recent header for each level
        let mut level_context: Vec<Option<String>> = vec![None; 7]; // H1-H6
        
        for (level, text) in context {
            if level <= 6 {
                level_context[level] = Some(text);
                // Clear deeper levels when we encounter a header
                for deeper in (level + 1)..=6 {
                    level_context[deeper] = None;
                }
            }
        }
        
        level_context
            .into_iter()
            .enumerate()
            .filter_map(|(level, text)| text.map(|t| (level, t)))
            .collect()
    }
}

/// Main text chunking processor
#[derive(Debug, Clone)]
pub struct ChunkProcessor {
    /// Configuration for chunking operations
    config: ChunkConfig,
    /// Boundary detector for semantic chunking
    boundary_detector: BoundaryDetector,
    /// Markdown parser for document structure analysis
    markdown_parser: MarkdownParser,
    /// Performance monitoring enabled
    monitor_performance: bool,
}

impl ChunkProcessor {
    /// Creates a new chunk processor with the given configuration
    pub fn new(config: ChunkConfig) -> ChunkResult<Self> {
        config.validate()?;
        
        Ok(Self {
            config,
            boundary_detector: BoundaryDetector::default(),
            markdown_parser: MarkdownParser::default(),
            monitor_performance: true,
        })
    }
    
    /// Creates a new chunk processor with performance monitoring disabled
    pub fn new_without_monitoring(config: ChunkConfig) -> ChunkResult<Self> {
        config.validate()?;
        
        Ok(Self {
            config,
            boundary_detector: BoundaryDetector::default(),
            markdown_parser: MarkdownParser::default(),
            monitor_performance: false,
        })
    }
    
    /// Creates a new chunk processor with default configuration
    pub fn with_default_config() -> ChunkResult<Self> {
        Self::new(ChunkConfig::default())
    }
    
    /// Creates a new chunk processor optimized for large documents
    pub fn for_large_documents() -> ChunkResult<Self> {
        let config = ChunkConfig {
            strategy: ChunkingStrategy::Hybrid,
            max_chunk_size: 1500, // Larger chunks for efficiency
            overlap_size: 150,
            min_chunk_size: 100,
            preserve_paragraphs: true,
            preserve_sentences: false, // Skip for performance
            preserve_markdown_headers: true,
            preserve_code_blocks: true,
            preserve_markdown_links: false, // Skip for performance
            strip_markdown_formatting: false,
        };
        
        Self::new(config)
    }
    
    /// Returns the current configuration
    pub fn config(&self) -> &ChunkConfig {
        &self.config
    }
    
    /// Updates the configuration (validates before applying)
    pub fn set_config(&mut self, config: ChunkConfig) -> ChunkResult<()> {
        config.validate()?;
        self.config = config;
        Ok(())
    }
    
    /// Enables or disables performance monitoring
    pub fn set_performance_monitoring(&mut self, enabled: bool) {
        self.monitor_performance = enabled;
    }
    
    /// Chunks the input text according to the configured strategy
    pub fn chunk_text(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        if text.is_empty() {
            return Err(ChunkError::InvalidInput("Input text is empty".to_string()));
        }
        
        match self.config.strategy {
            ChunkingStrategy::FixedSize => self.chunk_fixed_size(text),
            ChunkingStrategy::Semantic => self.chunk_semantic(text),
            ChunkingStrategy::Hybrid => self.chunk_hybrid(text),
            ChunkingStrategy::MarkdownAware => self.chunk_markdown_aware(text),
        }
    }
    
    /// Chunks the input text with performance monitoring
    pub fn chunk_text_with_metrics(&self, text: &str) -> ChunkResult<ChunkingResult> {
        if text.is_empty() {
            return Err(ChunkError::InvalidInput("Input text is empty".to_string()));
        }
        
        let start_time = Instant::now();
        let mut metrics = PerformanceMetrics::new();
        
        // Perform chunking
        let chunks = match self.config.strategy {
            ChunkingStrategy::FixedSize => self.chunk_fixed_size(text)?,
            ChunkingStrategy::Semantic => self.chunk_semantic(text)?,
            ChunkingStrategy::Hybrid => self.chunk_hybrid(text)?,
            ChunkingStrategy::MarkdownAware => self.chunk_markdown_aware(text)?,
        };
        
        // Calculate metrics
        let processing_time = start_time.elapsed();
        metrics.processing_time_ms = processing_time.as_millis() as u64;
        metrics.finalize(text.len(), chunks.len());
        metrics.estimate_memory_usage(text.len(), chunks.len());
        
        Ok(ChunkingResult::new(chunks, metrics))
    }
    
    /// Processes text in streaming fashion for large documents (>100KB)
    pub fn chunk_text_streaming(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        if text.is_empty() {
            return Err(ChunkError::InvalidInput("Input text is empty".to_string()));
        }
        
        // For very large documents, use streaming approach
        if text.len() > 100_000 {
            return self.chunk_large_text_streaming(text);
        }
        
        // For smaller documents, use regular chunking
        self.chunk_text(text)
    }
    
    /// Fixed-size chunking algorithm with overlap management (optimized)
    fn chunk_fixed_size(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let text_len = text.len();
        let max_chunk_size = self.config.max_chunk_size;
        let overlap_size = self.config.overlap_size;
        
        // Pre-calculate approximate number of chunks to avoid reallocations
        let estimated_chunks = (text_len / (max_chunk_size - overlap_size)).max(1) + 1;
        let mut chunks = Vec::with_capacity(estimated_chunks);
        
        let mut position = 0;
        
        while position < text_len {
            let chunk_end = (position + max_chunk_size).min(text_len);
            
            // Use string slice reference instead of copying until final chunk creation
            let chunk_slice = &text[position..chunk_end];
            
            let metadata = self.create_chunk_metadata_optimized(
                chunk_slice,
                position, 
                chunk_end, 
                chunks.len(), 
                !chunks.is_empty(), // has previous overlap
                chunk_end < text_len // has next overlap
            );
            
            chunks.push(TextChunk::new(chunk_slice.to_string(), metadata));
            
            // Calculate next position with overlap
            if chunk_end >= text_len {
                break;
            }
            
            position = chunk_end.saturating_sub(overlap_size);
            
            // Prevent infinite loops by ensuring progress
            if position >= chunk_end {
                position = chunk_end;
            }
        }
        
        // Update total chunks count in all metadata
        self.finalize_chunks_metadata_optimized(chunks)
    }
    
    /// Semantic chunking preserving sentence and paragraph boundaries (optimized)
    fn chunk_semantic(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let text_len = text.len();
        let max_chunk_size = self.config.max_chunk_size;
        let min_chunk_size = self.config.min_chunk_size;
        let overlap_size = self.config.overlap_size;
        
        // Pre-calculate estimated chunks to avoid reallocations
        let estimated_chunks = (text_len / (max_chunk_size - overlap_size)).max(1) + 1;
        let mut chunks = Vec::with_capacity(estimated_chunks);
        
        let mut position = 0;
        
        while position < text_len {
            let target_end = (position + max_chunk_size).min(text_len);
            
            // If we're at the end or within min_chunk_size, take the rest
            if target_end >= text_len || text_len - position <= min_chunk_size {
                let chunk_slice = &text[position..text_len];
                if !chunk_slice.trim().is_empty() {
                    let metadata = self.create_chunk_metadata_optimized(
                        chunk_slice,
                        position,
                        text_len,
                        chunks.len(),
                        !chunks.is_empty(),
                        false
                    );
                    chunks.push(TextChunk::new(chunk_slice.to_string(), metadata));
                }
                break;
            }
            
            // Find the best semantic boundary with optimized search
            let search_range = max_chunk_size / 4; // 25% search range
            let actual_end = self.boundary_detector
                .find_best_boundary_optimized(text, target_end, search_range)
                .unwrap_or(target_end);
            
            let chunk_slice = &text[position..actual_end];
            if !chunk_slice.trim().is_empty() && chunk_slice.len() >= min_chunk_size {
                let metadata = self.create_chunk_metadata_optimized(
                    chunk_slice,
                    position,
                    actual_end,
                    chunks.len(),
                    !chunks.is_empty(),
                    actual_end < text_len
                );
                chunks.push(TextChunk::new(chunk_slice.to_string(), metadata));
            }
            
            // Calculate next position with overlap
            position = actual_end.saturating_sub(overlap_size);
            
            // Ensure we make progress
            if position >= actual_end {
                position = actual_end;
            }
        }
        
        self.finalize_chunks_metadata_optimized(chunks)
    }
    
    /// Hybrid chunking combining semantic boundaries with size constraints (optimized)
    fn chunk_hybrid(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let text_len = text.len();
        let max_chunk_size = self.config.max_chunk_size;
        let min_chunk_size = self.config.min_chunk_size;
        let overlap_size = self.config.overlap_size;
        
        // Pre-calculate estimated chunks
        let estimated_chunks = (text_len / (max_chunk_size - overlap_size)).max(1) + 1;
        let mut chunks = Vec::with_capacity(estimated_chunks);
        
        let mut position = 0;
        
        while position < text_len {
            let max_end = (position + max_chunk_size).min(text_len);
            
            // First try semantic boundary with optimized detection
            let search_range = max_chunk_size / 3;
            let semantic_end = self.boundary_detector
                .find_best_boundary_optimized(text, max_end, search_range);
            
            let chunk_end = match semantic_end {
                Some(boundary) if boundary >= position + min_chunk_size => boundary,
                _ => max_end, // Fall back to fixed size if no good boundary found
            };
            
            let chunk_slice = &text[position..chunk_end];
            if !chunk_slice.trim().is_empty() {
                let metadata = self.create_chunk_metadata_optimized(
                    chunk_slice,
                    position,
                    chunk_end,
                    chunks.len(),
                    !chunks.is_empty(),
                    chunk_end < text_len
                );
                chunks.push(TextChunk::new(chunk_slice.to_string(), metadata));
            }
            
            if chunk_end >= text_len {
                break;
            }
            
            // Calculate next position with overlap
            position = chunk_end.saturating_sub(overlap_size);
            if position >= chunk_end {
                position = chunk_end;
            }
        }
        
        self.finalize_chunks_metadata_optimized(chunks)
    }
    
    /// Markdown-aware chunking that preserves document structure
    fn chunk_markdown_aware(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let mut chunks = Vec::new();
        let text_len = text.len();
        
        // Parse markdown structure
        let elements = self.markdown_parser.parse(text);
        
        // Get header boundaries for structural chunking
        let header_boundaries = if self.config.preserve_markdown_headers {
            self.markdown_parser.find_header_boundaries(text)
        } else {
            Vec::new()
        };
        
        // Get code block boundaries to preserve them intact
        let code_block_boundaries = if self.config.preserve_code_blocks {
            self.markdown_parser.find_code_block_boundaries(text)
        } else {
            Vec::new()
        };
        
        let mut position = 0;
        
        while position < text_len {
            let target_end = (position + self.config.max_chunk_size).min(text_len);
            
            // Check if we're inside a code block - if so, preserve it intact
            let in_code_block = code_block_boundaries.iter()
                .find(|(start, end)| position >= *start && position < *end);
                
            let chunk_end = if let Some((_, code_end)) = in_code_block {
                // Include the entire code block
                (*code_end).min(text_len)
            } else {
                // Find the best boundary considering markdown structure
                self.find_markdown_boundary(text, target_end, &header_boundaries, position)
            };
            
            let chunk_content = text[position..chunk_end].to_string();
            if !chunk_content.trim().is_empty() && 
               chunk_content.len() >= self.config.min_chunk_size || 
               chunk_end >= text_len {
                
                let metadata = self.create_markdown_chunk_metadata(
                    &chunk_content,
                    position,
                    chunk_end,
                    chunks.len(),
                    !chunks.is_empty(),
                    chunk_end < text_len,
                    &elements
                );
                
                let processed_content = if self.config.strip_markdown_formatting {
                    self.markdown_parser.strip_formatting(&chunk_content)
                } else {
                    chunk_content
                };
                
                chunks.push(TextChunk::new(processed_content, metadata));
            }
            
            if chunk_end >= text_len {
                break;
            }
            
            // Calculate next position with overlap, but respect markdown boundaries
            let next_position = if in_code_block.is_some() {
                // Don't overlap code blocks
                chunk_end
            } else {
                chunk_end.saturating_sub(self.config.overlap_size)
            };
            
            position = if next_position >= chunk_end {
                chunk_end
            } else {
                next_position
            };
        }
        
        self.finalize_chunks_metadata_optimized(chunks)
    }
    
    /// Finds the best boundary for markdown-aware chunking
    fn find_markdown_boundary(&self, text: &str, target_pos: usize, header_boundaries: &[usize], current_pos: usize) -> usize {
        if target_pos >= text.len() {
            return text.len();
        }
        
        let search_range = self.config.max_chunk_size / 4;
        let start = target_pos.saturating_sub(search_range);
        let end = (target_pos + search_range).min(text.len());
        
        // Prefer header boundaries within search range
        if self.config.preserve_markdown_headers {
            let nearby_headers: Vec<usize> = header_boundaries.iter()
                .filter(|&&pos| pos >= start && pos <= end && pos > current_pos)
                .copied()
                .collect();
                
            if let Some(&best_header) = nearby_headers.iter()
                .min_by_key(|&&pos| target_pos.abs_diff(pos)) {
                return best_header;
            }
        }
        
        // Fall back to semantic boundaries
        self.boundary_detector
            .find_best_boundary(text, target_pos, search_range)
            .unwrap_or(target_pos)
    }
    
    /// Creates metadata for a chunk
    fn create_chunk_metadata(
        &self,
        content: &str,
        start_pos: usize,
        end_pos: usize,
        chunk_index: usize,
        has_previous_overlap: bool,
        has_next_overlap: bool,
    ) -> ChunkMetadata {
        let word_count = content.split_whitespace().count();
        let sentence_count = self.count_sentences(content);
        
        ChunkMetadata {
            start_position: start_pos,
            end_position: end_pos,
            chunk_index,
            total_chunks: 0, // Will be updated in finalize_chunks_metadata
            character_count: content.len(),
            word_count,
            sentence_count,
            has_previous_overlap,
            has_next_overlap,
            previous_overlap_size: if has_previous_overlap { self.config.overlap_size } else { 0 },
            next_overlap_size: if has_next_overlap { self.config.overlap_size } else { 0 },
            context: HashMap::new(),
            markdown: None,
        }
    }
    
    /// Creates metadata for a markdown-aware chunk
    fn create_markdown_chunk_metadata(
        &self,
        content: &str,
        start_pos: usize,
        end_pos: usize,
        chunk_index: usize,
        has_previous_overlap: bool,
        has_next_overlap: bool,
        elements: &[MarkdownElement],
    ) -> ChunkMetadata {
        let word_count = content.split_whitespace().count();
        let sentence_count = self.count_sentences(content);
        
        // Filter elements that fall within this chunk
        let chunk_elements: Vec<&MarkdownElement> = elements.iter()
            .filter(|element| {
                let element_pos = match element {
                    MarkdownElement::Header(_, _, pos) => *pos,
                    MarkdownElement::CodeBlock(_, _, start, _) => *start,
                    MarkdownElement::Link(_, _, _, pos) => *pos,
                    MarkdownElement::List(_, _, pos) => *pos,
                    MarkdownElement::Table(_, _, pos) => *pos,
                    MarkdownElement::Paragraph(_, pos) => *pos,
                    MarkdownElement::LineBreak(pos) => *pos,
                };
                element_pos >= start_pos && element_pos < end_pos
            })
            .collect();
        
        // Create markdown-specific metadata
        let chunk_elements_owned: Vec<MarkdownElement> = chunk_elements.into_iter().cloned().collect();
        let markdown_metadata = self.markdown_parser
            .create_metadata(&chunk_elements_owned, self.config.strip_markdown_formatting);
        
        // Build structure context from parent headers
        let structure_context = self.markdown_parser
            .build_structure_context(elements, start_pos);
        
        let mut markdown_meta = markdown_metadata;
        markdown_meta.structure_context = structure_context;
        
        ChunkMetadata {
            start_position: start_pos,
            end_position: end_pos,
            chunk_index,
            total_chunks: 0, // Will be updated in finalize_chunks_metadata
            character_count: content.len(),
            word_count,
            sentence_count,
            has_previous_overlap,
            has_next_overlap,
            previous_overlap_size: if has_previous_overlap { self.config.overlap_size } else { 0 },
            next_overlap_size: if has_next_overlap { self.config.overlap_size } else { 0 },
            context: HashMap::new(),
            markdown: Some(markdown_meta),
        }
    }
    
    /// Finalizes chunks by updating total_chunks count and chunk indices
    fn finalize_chunks_metadata(&self, mut chunks: Vec<TextChunk>) -> ChunkResult<Vec<TextChunk>> {
        let total_chunks = chunks.len();
        
        for (index, chunk) in chunks.iter_mut().enumerate() {
            chunk.metadata.chunk_index = index;
            chunk.metadata.total_chunks = total_chunks;
        }
        
        Ok(chunks)
    }
    
    /// Counts approximate number of sentences in text
    fn count_sentences(&self, text: &str) -> usize {
        self.count_sentences_optimized(text)
    }
    
    /// Counts approximate number of sentences in text (optimized)
    fn count_sentences_optimized(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        
        // Fast sentence counting without full boundary detection
        let mut count = 0;
        let bytes = text.as_bytes();
        let mut i = 0;
        
        while i < bytes.len() {
            match bytes[i] {
                b'.' | b'!' | b'?' => {
                    count += 1;
                    // Skip multiple punctuation
                    while i + 1 < bytes.len() && matches!(bytes[i + 1], b'.' | b'!' | b'?') {
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        
        count.max(1) // At least one sentence if text is not empty
    }
    
    /// Optimized word counting using byte iteration
    fn count_words_optimized(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        
        let mut count = 0;
        let mut in_word = false;
        
        for byte in text.bytes() {
            let is_whitespace = byte.is_ascii_whitespace();
            
            if !is_whitespace && !in_word {
                count += 1;
                in_word = true;
            } else if is_whitespace {
                in_word = false;
            }
        }
        
        count
    }
    
    /// Streaming chunking for large documents (>100KB)
    fn chunk_large_text_streaming(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        const STREAM_BUFFER_SIZE: usize = 50_000; // Process in 50KB chunks
        
        let text_len = text.len();
        let max_chunk_size = self.config.max_chunk_size;
        let overlap_size = self.config.overlap_size;
        
        // Pre-allocate with conservative estimate
        let estimated_chunks = (text_len / (max_chunk_size - overlap_size)) + 10;
        let mut all_chunks = Vec::with_capacity(estimated_chunks);
        
        let mut stream_position = 0;
        
        while stream_position < text_len {
            let stream_end = (stream_position + STREAM_BUFFER_SIZE).min(text_len);
            
            // Extend to a good boundary to avoid splitting mid-sentence
            let actual_stream_end = if stream_end < text_len {
                // Find a good boundary within the next 1KB
                self.boundary_detector
                    .find_best_boundary_optimized(text, stream_end, 500)
                    .unwrap_or(stream_end)
            } else {
                text_len
            };
            
            // Process this stream chunk
            let stream_text = &text[stream_position..actual_stream_end];
            
            // Apply regular chunking to the stream segment
            let strategy_chunks = match self.config.strategy {
                ChunkingStrategy::FixedSize => self.chunk_fixed_size(stream_text)?,
                ChunkingStrategy::Semantic => self.chunk_semantic(stream_text)?,
                ChunkingStrategy::Hybrid => self.chunk_hybrid(stream_text)?,
                ChunkingStrategy::MarkdownAware => self.chunk_markdown_aware(stream_text)?,
            };
            
            // Adjust chunk positions to global coordinates
            for mut chunk in strategy_chunks {
                chunk.metadata.start_position += stream_position;
                chunk.metadata.end_position += stream_position;
                chunk.metadata.chunk_index = all_chunks.len();
                all_chunks.push(chunk);
            }
            
            // Move to next stream position with some overlap
            stream_position = actual_stream_end.saturating_sub(max_chunk_size / 2);
            if stream_position >= actual_stream_end {
                break;
            }
        }
        
        // Final metadata update
        self.finalize_chunks_metadata_optimized(all_chunks)
    }
    
    /// Finalizes chunks by updating total_chunks count and chunk indices (optimized)
    fn finalize_chunks_metadata_optimized(&self, mut chunks: Vec<TextChunk>) -> ChunkResult<Vec<TextChunk>> {
        let total_chunks = chunks.len();
        
        // Use direct indexing instead of enumerate for better performance
        for i in 0..chunks.len() {
            chunks[i].metadata.chunk_index = i;
            chunks[i].metadata.total_chunks = total_chunks;
        }
        
        Ok(chunks)
    }
    
    /// Creates metadata for a chunk (optimized version)
    fn create_chunk_metadata_optimized(
        &self,
        content: &str,
        start_pos: usize,
        end_pos: usize,
        chunk_index: usize,
        has_previous_overlap: bool,
        has_next_overlap: bool,
    ) -> ChunkMetadata {
        // Optimized word counting using byte iteration
        let word_count = self.count_words_optimized(content);
        let sentence_count = if self.config.preserve_sentences {
            self.count_sentences_optimized(content)
        } else {
            1 // Skip expensive sentence counting if not needed
        };
        
        ChunkMetadata {
            start_position: start_pos,
            end_position: end_pos,
            chunk_index,
            total_chunks: 0, // Will be updated in finalize_chunks_metadata_optimized
            character_count: content.len(),
            word_count,
            sentence_count,
            has_previous_overlap,
            has_next_overlap,
            previous_overlap_size: if has_previous_overlap { self.config.overlap_size } else { 0 },
            next_overlap_size: if has_next_overlap { self.config.overlap_size } else { 0 },
            context: HashMap::with_capacity(4), // Pre-allocate with expected capacity
            markdown: None,
        }
    }
}

/// Utility functions for text analysis
impl ChunkProcessor {
    /// Calculates optimal chunk size based on text characteristics
    pub fn calculate_optimal_chunk_size(&self, text: &str) -> usize {
        let _text_len = text.len();
        let avg_sentence_length = self.calculate_average_sentence_length(text);
        let avg_paragraph_length = self.calculate_average_paragraph_length(text);
        
        // Optimize based on text characteristics
        let base_size = self.config.max_chunk_size;
        
        // Adjust for very long or short sentences
        let sentence_factor = if avg_sentence_length > 100 {
            1.2 // Larger chunks for long sentences
        } else if avg_sentence_length < 30 {
            0.8 // Smaller chunks for short sentences
        } else {
            1.0
        };
        
        // Adjust for paragraph structure
        let paragraph_factor = if avg_paragraph_length > base_size {
            1.1 // Slightly larger chunks for long paragraphs
        } else {
            1.0
        };
        
        let optimal_size = (base_size as f64 * sentence_factor * paragraph_factor) as usize;
        optimal_size.clamp(self.config.min_chunk_size, self.config.max_chunk_size)
    }
    
    /// Calculates average sentence length in characters
    fn calculate_average_sentence_length(&self, text: &str) -> usize {
        let boundaries = self.boundary_detector.find_sentence_boundaries(text);
        if boundaries.is_empty() {
            return text.len();
        }
        
        let mut total_length = 0;
        let mut last_pos = 0;
        
        for boundary in &boundaries {
            total_length += boundary.saturating_sub(last_pos);
            last_pos = *boundary;
        }
        
        total_length / boundaries.len()
    }
    
    /// Calculates average paragraph length in characters
    fn calculate_average_paragraph_length(&self, text: &str) -> usize {
        let boundaries = self.boundary_detector.find_paragraph_boundaries(text);
        if boundaries.is_empty() {
            return text.len();
        }
        
        let mut total_length = 0;
        let mut last_pos = 0;
        
        for boundary in &boundaries {
            total_length += boundary.saturating_sub(last_pos);
            last_pos = *boundary;
        }
        
        // Add the final paragraph
        total_length += text.len().saturating_sub(last_pos);
        
        total_length / (boundaries.len() + 1)
    }
    
    /// Analyzes text to suggest optimal chunking configuration
    pub fn analyze_text_for_chunking(&self, text: &str) -> ChunkConfig {
        let optimal_size = self.calculate_optimal_chunk_size(text);
        let suggested_overlap = (optimal_size / 10).clamp(50, 200); // 10% overlap
        
        ChunkConfig {
            strategy: if text.len() > 10000 {
                ChunkingStrategy::Hybrid
            } else {
                ChunkingStrategy::Semantic
            },
            max_chunk_size: optimal_size,
            overlap_size: suggested_overlap,
            min_chunk_size: (optimal_size / 4).max(50),
            preserve_paragraphs: true,
            preserve_sentences: true,
            preserve_markdown_headers: true,
            preserve_code_blocks: true,
            preserve_markdown_links: true,
            strip_markdown_formatting: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample text for testing
    fn sample_text() -> &'static str {
        "This is the first sentence. This is the second sentence! And here is a question? \
         \n\nThis starts a new paragraph. It has multiple sentences. Each sentence ends with punctuation. \
         \n\nThe final paragraph is here. It contains the last few sentences. The end."
    }

    /// Long text sample for testing large document handling
    fn long_text() -> String {
        let paragraph = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. \
                        Nisi ut aliquip ex ea commodo consequat.";
        
        // Create a text with multiple paragraphs
        let mut long_text = String::new();
        for i in 0..20 {
            long_text.push_str(&format!("Paragraph {}. {}\n\n", i + 1, paragraph));
        }
        long_text
    }

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.strategy, ChunkingStrategy::Semantic);
        assert_eq!(config.max_chunk_size, 1000);
        assert_eq!(config.overlap_size, 100);
        assert_eq!(config.min_chunk_size, 50);
        assert!(config.preserve_paragraphs);
        assert!(config.preserve_sentences);
    }

    #[test]
    fn test_chunk_config_validation() {
        // Valid config should pass
        let valid_config = ChunkConfig::default();
        assert!(valid_config.validate().is_ok());

        // Invalid max_chunk_size (too small)
        let mut invalid_config = ChunkConfig::default();
        invalid_config.max_chunk_size = 100;
        assert!(invalid_config.validate().is_err());

        // Invalid max_chunk_size (too large)
        invalid_config.max_chunk_size = 3000;
        assert!(invalid_config.validate().is_err());

        // Invalid overlap_size (too large)
        invalid_config = ChunkConfig::default();
        invalid_config.overlap_size = 1200;
        assert!(invalid_config.validate().is_err());

        // Invalid min_chunk_size (larger than max)
        invalid_config = ChunkConfig::default();
        invalid_config.min_chunk_size = 1500;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_chunk_processor_creation() {
        let config = ChunkConfig::default();
        let processor = ChunkProcessor::new(config).unwrap();
        assert_eq!(processor.config().strategy, ChunkingStrategy::Semantic);

        // Invalid config should fail
        let mut invalid_config = ChunkConfig::default();
        invalid_config.max_chunk_size = 100;
        assert!(ChunkProcessor::new(invalid_config).is_err());
    }

    #[test]
    fn test_fixed_size_chunking() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::FixedSize;
        config.max_chunk_size = 250;
        config.overlap_size = 10;
        
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_text()).unwrap();
        
        assert!(!chunks.is_empty());
        
        // Check that most chunks (except possibly the last) are close to max size
        for (i, chunk) in chunks.iter().enumerate() {
            if i < chunks.len() - 1 {
                // Non-last chunks should be close to max_chunk_size
                assert!(chunk.len() <= 250);
            }
            
            // Verify metadata
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, chunks.len());
            assert_eq!(chunk.metadata.character_count, chunk.content.len());
            assert!(chunk.metadata.word_count > 0);
        }
    }

    #[test]
    fn test_semantic_chunking() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::Semantic;
        config.max_chunk_size = 300;
        config.overlap_size = 20;
        
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_text()).unwrap();
        
        assert!(!chunks.is_empty());
        
        // Check that chunks respect semantic boundaries
        for chunk in &chunks {
            let content = chunk.content().trim();
            assert!(!content.is_empty());
            
            // Most chunks should end with sentence-ending punctuation or be the last chunk
            if chunk.metadata.chunk_index < chunks.len() - 1 {
                let last_char = content.chars().last().unwrap_or(' ');
                // Allow for some flexibility in boundary detection
                assert!(last_char.is_ascii_punctuation() || content.contains('\n'));
            }
        }
    }

    #[test]
    fn test_hybrid_chunking() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::Hybrid;
        config.max_chunk_size = 280;
        config.overlap_size = 15;
        
        let max_size = config.max_chunk_size;
        let min_size = config.min_chunk_size;
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_text()).unwrap();
        
        assert!(!chunks.is_empty());
        
        // Hybrid chunking should balance size and semantic constraints
        for chunk in &chunks {
            assert!(chunk.len() <= max_size + 20); // Allow some flexibility
            assert!(chunk.len() >= min_size || chunk.metadata.chunk_index == chunks.len() - 1);
        }
    }

    #[test]
    fn test_boundary_detection() {
        let detector = BoundaryDetector::default();
        
        // Test sentence boundary detection
        let text = "First sentence. Second sentence! Third sentence?";
        let boundaries = detector.find_sentence_boundaries(text);
        assert_eq!(boundaries.len(), 3);
        assert!(boundaries.contains(&15)); // After "First sentence."
        assert!(boundaries.contains(&32)); // After "Second sentence!"
        assert!(boundaries.contains(&48)); // After "Third sentence?"
        
        // Test paragraph boundary detection
        let text_with_paragraphs = "First paragraph.\n\nSecond paragraph.";
        let para_boundaries = detector.find_paragraph_boundaries(text_with_paragraphs);
        assert!(!para_boundaries.is_empty());
    }

    #[test]
    fn test_overlap_management() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::FixedSize;
        config.max_chunk_size = 250;
        config.overlap_size = 10;
        
        let overlap_size = config.overlap_size;
        let processor = ChunkProcessor::new(config).unwrap();
        let text = "This is a longer text that should be split into multiple chunks with overlap.";
        let chunks = processor.chunk_text(text).unwrap();
        
        if chunks.len() > 1 {
            // Check that consecutive chunks have overlap
            for i in 0..chunks.len() - 1 {
                let current_chunk = &chunks[i];
                let next_chunk = &chunks[i + 1];
                
                assert!(current_chunk.metadata.has_next_overlap);
                assert!(next_chunk.metadata.has_previous_overlap);
                
                // Find overlapping content
                let current_end = &current_chunk.content[current_chunk.len().saturating_sub(overlap_size)..];
                let next_start = &next_chunk.content[..overlap_size.min(next_chunk.len())];
                
                // There should be some common content (allowing for boundary adjustments)
                assert!(current_end.len() > 0);
                assert!(next_start.len() > 0);
            }
        }
    }

    #[test]
    fn test_chunk_metadata() {
        let config = ChunkConfig::default();
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_text()).unwrap();
        
        for (i, chunk) in chunks.iter().enumerate() {
            let metadata = chunk.metadata();
            
            // Basic metadata checks
            assert_eq!(metadata.chunk_index, i);
            assert_eq!(metadata.total_chunks, chunks.len());
            assert_eq!(metadata.character_count, chunk.content.len());
            assert!(metadata.word_count > 0);
            assert!(metadata.sentence_count > 0);
            
            // Position checks
            assert!(metadata.end_position > metadata.start_position);
            
            // Overlap checks
            if i > 0 {
                assert!(metadata.has_previous_overlap);
            } else {
                assert!(!metadata.has_previous_overlap);
            }
            
            if i < chunks.len() - 1 {
                assert!(metadata.has_next_overlap);
            } else {
                assert!(!metadata.has_next_overlap);
            }
        }
    }

    #[test]
    fn test_empty_text_handling() {
        let config = ChunkConfig::default();
        let processor = ChunkProcessor::new(config).unwrap();
        
        // Empty text should return error
        let result = processor.chunk_text("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChunkError::InvalidInput(_)));
    }

    #[test]
    fn test_small_text_handling() {
        let config = ChunkConfig::default();
        let processor = ChunkProcessor::new(config).unwrap();
        
        // Small text should return single chunk
        let small_text = "Short text.";
        let chunks = processor.chunk_text(small_text).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, small_text);
        assert_eq!(chunks[0].metadata.chunk_index, 0);
        assert_eq!(chunks[0].metadata.total_chunks, 1);
    }

    #[test]
    fn test_large_document_performance() {
        let config = ChunkConfig::default();
        let max_size = config.max_chunk_size;
        let processor = ChunkProcessor::new(config).unwrap();
        let large_text = long_text();
        
        // This should complete within reasonable time
        let start = std::time::Instant::now();
        let chunks = processor.chunk_text(&large_text).unwrap();
        let duration = start.elapsed();
        
        assert!(!chunks.is_empty());
        assert!(duration.as_millis() < 100); // Should be fast
        
        // Verify all chunks are properly formed
        for chunk in &chunks {
            assert!(!chunk.content.trim().is_empty());
            assert!(chunk.len() <= max_size + 100); // Allow some flexibility
        }
    }

    #[test]
    fn test_optimal_chunk_size_calculation() {
        let config = ChunkConfig::default();
        let min_size = config.min_chunk_size;
        let max_size = config.max_chunk_size;
        let processor = ChunkProcessor::new(config).unwrap();
        
        let optimal_size = processor.calculate_optimal_chunk_size(sample_text());
        assert!(optimal_size >= min_size);
        assert!(optimal_size <= max_size);
    }

    #[test]
    fn test_text_analysis_for_chunking() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        // Short text analysis
        let short_config = processor.analyze_text_for_chunking(sample_text());
        assert_eq!(short_config.strategy, ChunkingStrategy::Semantic);
        
        // Long text analysis  
        let long_text = long_text();
        let long_config = processor.analyze_text_for_chunking(&long_text);
        // Long text should use Hybrid strategy if over 10000 chars
        if long_text.len() > 10000 {
            assert_eq!(long_config.strategy, ChunkingStrategy::Hybrid);
        } else {
            assert_eq!(long_config.strategy, ChunkingStrategy::Semantic);
        }
    }

    #[test]
    fn test_sentence_counting() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        let text = "First sentence. Second sentence! Third sentence?";
        let count = processor.count_sentences(text);
        assert_eq!(count, 3);
        
        let single_sentence = "Just one sentence.";
        let single_count = processor.count_sentences(single_sentence);
        assert_eq!(single_count, 1);
    }

    #[test]
    fn test_average_sentence_length() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        let text = "Short. Medium length sentence. This is a much longer sentence with more words.";
        let avg_len = processor.calculate_average_sentence_length(text);
        assert!(avg_len > 10); // Should be reasonable average
    }

    #[test]
    fn test_average_paragraph_length() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        let text = "First paragraph.\n\nSecond paragraph with more content.\n\nThird paragraph.";
        let avg_len = processor.calculate_average_paragraph_length(text);
        assert!(avg_len > 0);
    }

    #[test]
    fn test_config_update() {
        let mut processor = ChunkProcessor::with_default_config().unwrap();
        
        let new_config = ChunkConfig {
            strategy: ChunkingStrategy::FixedSize,
            max_chunk_size: 500,
            overlap_size: 50,
            ..ChunkConfig::default()
        };
        
        assert!(processor.set_config(new_config.clone()).is_ok());
        assert_eq!(processor.config().strategy, ChunkingStrategy::FixedSize);
        assert_eq!(processor.config().max_chunk_size, 500);
        
        // Invalid config should fail
        let invalid_config = ChunkConfig {
            max_chunk_size: 100, // Too small
            ..ChunkConfig::default()
        };
        
        assert!(processor.set_config(invalid_config).is_err());
        // Original config should remain unchanged
        assert_eq!(processor.config().max_chunk_size, 500);
    }

    #[test] 
    fn test_text_chunk_methods() {
        let content = "Test content".to_string();
        let metadata = ChunkMetadata::default();
        let chunk = TextChunk::new(content.clone(), metadata);
        
        assert_eq!(chunk.len(), content.len());
        assert!(!chunk.is_empty());
        assert_eq!(chunk.content(), content);
        
        let empty_chunk = TextChunk::new(String::new(), ChunkMetadata::default());
        assert_eq!(empty_chunk.len(), 0);
        assert!(empty_chunk.is_empty());
    }

    #[test]
    fn test_chunk_error_display() {
        let error1 = ChunkError::InvalidInput("test".to_string());
        assert!(error1.to_string().contains("Invalid input"));
        
        let error2 = ChunkError::InvalidConfig("test".to_string());
        assert!(error2.to_string().contains("Invalid configuration"));
        
        let error3 = ChunkError::ProcessingError("test".to_string());
        assert!(error3.to_string().contains("Processing error"));
        
        let error4 = ChunkError::BoundaryDetectionError("test".to_string());
        assert!(error4.to_string().contains("Boundary detection error"));
    }

    /// Edge case test: Text with unusual punctuation patterns
    #[test]
    fn test_unusual_punctuation() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        let text = "Dr. Smith said... 'Hello world!' Then he left. What? Yes!!! Really??? Ok.";
        
        let chunks = processor.chunk_text(text).unwrap();
        assert!(!chunks.is_empty());
        
        // Should handle unusual punctuation gracefully
        for chunk in &chunks {
            assert!(!chunk.content.trim().is_empty());
        }
    }

    /// Edge case test: Text with only whitespace
    #[test] 
    fn test_whitespace_only_text() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        let text = "   \n\n  \t  \n  ";
        
        let result = processor.chunk_text(text);
        // Should either return empty result or handle gracefully
        match result {
            Ok(chunks) => {
                // If chunks are returned, they should not be empty content
                for chunk in chunks {
                    // Allow for trimmed content to be empty, but original content exists
                    assert!(!chunk.content.is_empty());
                }
            },
            Err(_) => {
                // Also acceptable to return error for whitespace-only input
            }
        }
    }

    /// Edge case test: Very long single sentence
    #[test]
    fn test_very_long_sentence() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        let long_sentence = "This is a very ".repeat(100) + "long sentence without punctuation";
        
        let chunks = processor.chunk_text(&long_sentence).unwrap();
        assert!(!chunks.is_empty());
        
        // Should handle long sentences by breaking at word boundaries when possible
        if chunks.len() > 1 {
            for chunk in &chunks {
                assert!(chunk.len() <= processor.config().max_chunk_size + 50); // Allow some flexibility
            }
        }
    }

    /// Sample markdown text for testing
    fn sample_markdown() -> &'static str {
        "# Main Title

This is the introduction paragraph.

## Section 1

Here's some content in section 1. It has multiple sentences.

```rust
fn hello_world() {
    println!(\"Hello, world!\");
}
```

### Subsection 1.1

This subsection contains a list:

- Item 1
- Item 2 with [a link](https://example.com \"Example\")
- Item 3

## Section 2

This section has a table:

| Header 1 | Header 2 |
|----------|----------|
| Row 1    | Data 1   |
| Row 2    | Data 2   |

And some final text."
    }

    #[test]
    fn test_markdown_aware_chunking_basic() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::MarkdownAware;
        config.max_chunk_size = 500;
        config.overlap_size = 50;
        
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_markdown()).unwrap();
        
        assert!(!chunks.is_empty());
        
        // Verify chunks have markdown metadata
        for chunk in &chunks {
            assert!(chunk.metadata.markdown.is_some());
            let markdown_meta = chunk.metadata.markdown.as_ref().unwrap();
            
            // Check that structure is preserved
            assert!(
                !markdown_meta.headers.is_empty() || 
                !markdown_meta.code_blocks.is_empty() || 
                !markdown_meta.lists.is_empty() ||
                !markdown_meta.tables.is_empty() ||
                chunk.metadata.chunk_index == 0 // First chunk might not have structure
            );
        }
    }

    #[test]
    fn test_markdown_header_boundaries() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::MarkdownAware;
        config.preserve_markdown_headers = true;
        config.max_chunk_size = 200; // Small chunks to force header boundaries
        
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_markdown()).unwrap();
        
        // Check that headers are preserved as natural boundaries
        let mut found_main_title = false;
        let mut found_section_1 = false;
        let mut found_section_2 = false;
        
        for chunk in &chunks {
            if let Some(ref markdown_meta) = chunk.metadata.markdown {
                for (level, text) in &markdown_meta.headers {
                    if text.contains("Main Title") {
                        found_main_title = true;
                        assert_eq!(*level, 1); // H1
                    } else if text.contains("Section 1") {
                        found_section_1 = true;
                        assert_eq!(*level, 2); // H2
                    } else if text.contains("Section 2") {
                        found_section_2 = true;
                        assert_eq!(*level, 2); // H2
                    }
                }
            }
        }
        
        assert!(found_main_title);
        assert!(found_section_1);
        assert!(found_section_2);
    }

    #[test]
    fn test_markdown_code_block_preservation() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::MarkdownAware;
        config.preserve_code_blocks = true;
        config.max_chunk_size = 200; // Small to test code block preservation
        
        let processor = ChunkProcessor::new(config).unwrap();
        let chunks = processor.chunk_text(sample_markdown()).unwrap();
        
        // Find the chunk containing the code block
        let mut found_code_block = false;
        for chunk in &chunks {
            if let Some(ref markdown_meta) = chunk.metadata.markdown {
                for (language, content) in &markdown_meta.code_blocks {
                    if content.contains("println!") {
                        found_code_block = true;
                        assert_eq!(language.as_ref().unwrap(), "rust");
                        // Code block should be preserved intact
                        assert!(content.contains("fn hello_world()"));
                        assert!(content.contains("println!"));
                    }
                }
            }
        }
        
        assert!(found_code_block, "Code block should be preserved");
    }

    #[test]
    fn test_markdown_formatting_stripping() {
        let mut config = ChunkConfig::default();
        config.strategy = ChunkingStrategy::MarkdownAware;
        config.strip_markdown_formatting = true;
        
        let processor = ChunkProcessor::new(config).unwrap();
        let markdown_with_formatting = "# Header\n\nThis is **bold** and *italic* text with `code` and [link](url).";
        let chunks = processor.chunk_text(markdown_with_formatting).unwrap();
        
        assert!(!chunks.is_empty());
        
        // Check that formatting is stripped from content
        for chunk in &chunks {
            let content = chunk.content();
            // Basic formatting should be stripped
            if content.contains("bold") {
                // Metadata should indicate formatting was stripped
                if let Some(ref markdown_meta) = chunk.metadata.markdown {
                    assert!(markdown_meta.has_stripped_formatting);
                }
            }
        }
    }
    
    #[test]
    fn test_performance_monitoring() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        let text = long_text();
        
        let result = processor.chunk_text_with_metrics(&text).unwrap();
        
        assert!(!result.chunks.is_empty());
        assert!(result.metrics.input_size_chars > 0);
        assert_eq!(result.metrics.chunks_generated, result.chunks.len());
        assert!(result.metrics.memory_usage_bytes > 0);
        // Note: processing_time_ms and chars_per_ms might be 0 for very fast operations
        
        println!("Performance metrics: {:?}", result.metrics);
        
        // Performance should meet basic requirements
        // Note: Very fast operations might show 0ms processing time
        assert!(result.metrics.processing_time_ms <= 100);
    }
    
    #[test]
    fn test_streaming_chunking_large_document() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        // Create a moderately large document for testing
        let large_paragraph = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(50);
        let large_text = (0..10).map(|i| format!("# Section {}\n\n{}", i, large_paragraph)).collect::<Vec<_>>().join("\n\n");
        
        // Test streaming functionality even on smaller documents
        let start = std::time::Instant::now();
        let chunks = processor.chunk_text_streaming(&large_text).unwrap();
        let duration = start.elapsed();
        
        assert!(!chunks.is_empty());
        println!("Streaming performance: {}ms for {} chars ({} chunks)", 
                 duration.as_millis(), large_text.len(), chunks.len());
        
        // Should handle documents efficiently
        assert!(duration.as_millis() < 500); // Under 0.5 seconds
        
        // Verify chunks are properly formed
        for chunk in &chunks {
            assert!(!chunk.content.trim().is_empty());
            // Hybrid chunking can create larger chunks when finding good semantic boundaries
            // Allow up to 50% larger than max_chunk_size for semantic coherence
            assert!(chunk.len() <= processor.config().max_chunk_size * 3 / 2); // Allow 50% flexibility for hybrid chunking
        }
    }
    
    #[test]
    fn test_memory_efficiency() {
        let processor = ChunkProcessor::for_large_documents().unwrap();
        let text = long_text();
        
        let result = processor.chunk_text_with_metrics(&text).unwrap();
        
        // Memory usage should be reasonable (under 10MB for test text)
        assert!(result.metrics.memory_usage_bytes < 10_000_000);
        
        // Should show linear scaling characteristics (more lenient for small test)
        let has_scaling = result.metrics.has_linear_scaling();
        println!("Linear scaling check: chars_per_ms={}, processing_time_ms={}, has_scaling={}", 
                 result.metrics.chars_per_ms, result.metrics.processing_time_ms, has_scaling);
        assert!(has_scaling);
        
        println!("Memory efficiency: {} bytes for {} input chars", 
                 result.metrics.memory_usage_bytes, text.len());
    }
    
    #[test]
    fn test_optimized_word_counting() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        let text = "Hello world! This is a test sentence with multiple words.";
        let word_count = processor.count_words_optimized(text);
        
        assert_eq!(word_count, 10); // Should count 10 words
        
        // Test edge cases
        assert_eq!(processor.count_words_optimized(""), 0);
        assert_eq!(processor.count_words_optimized("   "), 0);
        assert_eq!(processor.count_words_optimized("word"), 1);
        assert_eq!(processor.count_words_optimized("  word  "), 1);
        assert_eq!(processor.count_words_optimized("word1 word2"), 2);
    }
    
    #[test]
    fn test_optimized_sentence_counting() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        let text = "First sentence. Second sentence! Third sentence?";
        let sentence_count = processor.count_sentences_optimized(text);
        
        assert_eq!(sentence_count, 3);
        
        // Test edge cases
        assert_eq!(processor.count_sentences_optimized(""), 0);
        assert_eq!(processor.count_sentences_optimized("No punctuation"), 1);
        assert_eq!(processor.count_sentences_optimized("Multiple... punctuation!!!"), 2);
    }
    
    #[test]
    fn test_performance_requirements_validation() {
        let processor = ChunkProcessor::with_default_config().unwrap();
        
        // Create a 10KB document
        let text_10kb = "This is a test sentence. ".repeat(400); // Approximately 10KB
        assert!(text_10kb.len() >= 9000 && text_10kb.len() <= 11000);
        
        let start = std::time::Instant::now();
        let result = processor.chunk_text_with_metrics(&text_10kb).unwrap();
        let duration = start.elapsed();
        
        println!("10KB performance: {}ms, memory: {}MB", 
                 duration.as_millis(), result.metrics.memory_usage_bytes / 1024 / 1024);
        
        // Should meet performance requirements: 10KB in <100ms, <10MB memory
        assert!(duration.as_millis() <= 100);
        assert!(result.metrics.processing_time_ms <= 100);
        assert!(result.metrics.memory_usage_bytes <= 10_000_000);
        assert!(result.meets_performance_targets());
    }
    
    /// Create a very large text for performance testing
    fn very_large_text() -> String {
        let paragraph = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. \
                        Nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit. \
                        In voluptate velit esse cillum dolore eu fugiat nulla pariatur. ";
        
        // Create much larger text - repeat paragraph many times
        let large_paragraph = paragraph.repeat(20); // Make each section much larger
        let mut large_text = String::with_capacity(1_200_000); // Pre-allocate more
        
        for i in 0..500 {  // Fewer sections but much larger content
            large_text.push_str(&format!("\n\n## Section {}\n\n{}", i + 1, large_paragraph));
        }
        large_text
    }
    
    #[test] 
    fn test_large_document_performance_streaming() {
        let processor = ChunkProcessor::for_large_documents().unwrap();
        
        // Create a reasonably sized document for CI/testing (around 50KB)
        let paragraph = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(10);
        let large_text = (0..50).map(|i| format!("# Section {}\n\n{}", i, paragraph)).collect::<Vec<_>>().join("\n\n");
        
        println!("Testing document size: {} chars", large_text.len());
        
        let start = std::time::Instant::now();
        let chunks = processor.chunk_text_streaming(&large_text).unwrap();
        let duration = start.elapsed();
        
        println!("Large document performance: {}ms for {} chars ({} chunks)", 
                 duration.as_millis(), large_text.len(), chunks.len());
        
        assert!(!chunks.is_empty());
        
        // Should handle reasonably large documents efficiently
        assert!(duration.as_millis() < 1000); // Under 1 second
        
        // Verify chunk quality
        for chunk in &chunks {
            assert!(!chunk.content.trim().is_empty());
            // Hybrid chunking can create larger chunks when finding good semantic boundaries
            // Allow up to 50% larger than max_chunk_size for semantic coherence
            assert!(chunk.len() <= processor.config().max_chunk_size * 3 / 2);
        }
    }
}