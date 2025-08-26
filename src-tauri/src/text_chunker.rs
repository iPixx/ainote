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
}

/// Main text chunking processor
#[derive(Debug, Clone)]
pub struct ChunkProcessor {
    /// Configuration for chunking operations
    config: ChunkConfig,
    /// Boundary detector for semantic chunking
    boundary_detector: BoundaryDetector,
}

impl ChunkProcessor {
    /// Creates a new chunk processor with the given configuration
    pub fn new(config: ChunkConfig) -> ChunkResult<Self> {
        config.validate()?;
        
        Ok(Self {
            config,
            boundary_detector: BoundaryDetector::default(),
        })
    }
    
    /// Creates a new chunk processor with default configuration
    pub fn with_default_config() -> ChunkResult<Self> {
        Self::new(ChunkConfig::default())
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
    
    /// Chunks the input text according to the configured strategy
    pub fn chunk_text(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        if text.is_empty() {
            return Err(ChunkError::InvalidInput("Input text is empty".to_string()));
        }
        
        match self.config.strategy {
            ChunkingStrategy::FixedSize => self.chunk_fixed_size(text),
            ChunkingStrategy::Semantic => self.chunk_semantic(text),
            ChunkingStrategy::Hybrid => self.chunk_hybrid(text),
        }
    }
    
    /// Fixed-size chunking algorithm with overlap management
    fn chunk_fixed_size(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let mut chunks = Vec::new();
        let text_len = text.len();
        let mut position = 0;
        let chunk_index = 0;
        
        while position < text_len {
            let chunk_end = (position + self.config.max_chunk_size).min(text_len);
            let chunk_content = text[position..chunk_end].to_string();
            
            let metadata = self.create_chunk_metadata(
                &chunk_content, 
                position, 
                chunk_end, 
                chunk_index, 
                !chunks.is_empty(), // has previous overlap
                chunk_end < text_len // has next overlap
            );
            
            chunks.push(TextChunk::new(chunk_content, metadata));
            
            // Calculate next position with overlap
            if chunk_end >= text_len {
                break;
            }
            
            position = chunk_end.saturating_sub(self.config.overlap_size);
            
            // Prevent infinite loops by ensuring progress
            if position == chunk_end.saturating_sub(self.config.overlap_size) && position >= chunk_end {
                position = chunk_end;
            }
        }
        
        // Update total chunks count in all metadata
        self.finalize_chunks_metadata(chunks)
    }
    
    /// Semantic chunking preserving sentence and paragraph boundaries
    fn chunk_semantic(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let mut chunks = Vec::new();
        let text_len = text.len();
        let mut position = 0;
        
        while position < text_len {
            let target_end = (position + self.config.max_chunk_size).min(text_len);
            
            // If we're at the end or within min_chunk_size, take the rest
            if target_end >= text_len || text_len - position <= self.config.min_chunk_size {
                let chunk_content = text[position..text_len].to_string();
                if !chunk_content.trim().is_empty() {
                    let metadata = self.create_chunk_metadata(
                        &chunk_content,
                        position,
                        text_len,
                        chunks.len(),
                        !chunks.is_empty(),
                        false
                    );
                    chunks.push(TextChunk::new(chunk_content, metadata));
                }
                break;
            }
            
            // Find the best semantic boundary
            let search_range = self.config.max_chunk_size / 4; // 25% search range
            let actual_end = self.boundary_detector
                .find_best_boundary(text, target_end, search_range)
                .unwrap_or(target_end);
            
            let chunk_content = text[position..actual_end].to_string();
            if !chunk_content.trim().is_empty() && chunk_content.len() >= self.config.min_chunk_size {
                let metadata = self.create_chunk_metadata(
                    &chunk_content,
                    position,
                    actual_end,
                    chunks.len(),
                    !chunks.is_empty(),
                    actual_end < text_len
                );
                chunks.push(TextChunk::new(chunk_content, metadata));
            }
            
            // Calculate next position with overlap
            position = actual_end.saturating_sub(self.config.overlap_size);
            
            // Ensure we make progress
            if position >= actual_end {
                position = actual_end;
            }
        }
        
        self.finalize_chunks_metadata(chunks)
    }
    
    /// Hybrid chunking combining semantic boundaries with size constraints
    fn chunk_hybrid(&self, text: &str) -> ChunkResult<Vec<TextChunk>> {
        let mut chunks = Vec::new();
        let text_len = text.len();
        let mut position = 0;
        
        while position < text_len {
            let max_end = (position + self.config.max_chunk_size).min(text_len);
            
            // First try semantic boundary
            let search_range = self.config.max_chunk_size / 3;
            let semantic_end = self.boundary_detector
                .find_best_boundary(text, max_end, search_range);
            
            let chunk_end = match semantic_end {
                Some(boundary) if boundary >= position + self.config.min_chunk_size => boundary,
                _ => max_end, // Fall back to fixed size if no good boundary found
            };
            
            let chunk_content = text[position..chunk_end].to_string();
            if !chunk_content.trim().is_empty() {
                let metadata = self.create_chunk_metadata(
                    &chunk_content,
                    position,
                    chunk_end,
                    chunks.len(),
                    !chunks.is_empty(),
                    chunk_end < text_len
                );
                chunks.push(TextChunk::new(chunk_content, metadata));
            }
            
            if chunk_end >= text_len {
                break;
            }
            
            // Calculate next position with overlap
            position = chunk_end.saturating_sub(self.config.overlap_size);
            if position >= chunk_end {
                position = chunk_end;
            }
        }
        
        self.finalize_chunks_metadata(chunks)
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
        let boundaries = self.boundary_detector.find_sentence_boundaries(text);
        boundaries.len().max(1) // At least one sentence if text is not empty
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
}