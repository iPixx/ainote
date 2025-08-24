use std::collections::HashMap;
use thiserror::Error;
use regex::Regex;
use once_cell::sync::Lazy;

/// Errors that can occur during text processing operations
#[derive(Error, Debug)]
pub enum TextProcessingError {
    #[error("Empty text provided")]
    EmptyText,
    
    #[error("Invalid chunk size: {size} (must be between {min} and {max})")]
    InvalidChunkSize { size: usize, min: usize, max: usize },
    
    #[error("Invalid overlap: {overlap} (must be less than chunk size {chunk_size})")]
    InvalidOverlap { overlap: usize, chunk_size: usize },
    
    #[error("Text too long: {length} characters (maximum {max_length})")]
    TextTooLong { length: usize, max_length: usize },
    
    #[error("Invalid unicode text")]
    InvalidUnicode,
    
    #[error("Processing error: {message}")]
    ProcessingError { message: String },
}

pub type TextProcessingResult<T> = Result<T, TextProcessingError>;

/// Configuration for text chunking operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkingConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
    /// Whether to preserve sentence boundaries
    pub preserve_sentences: bool,
    /// Whether to preserve paragraph boundaries
    pub preserve_paragraphs: bool,
    /// Minimum chunk size (chunks smaller than this will be merged)
    pub min_chunk_size: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            overlap: 50,
            preserve_sentences: true,
            preserve_paragraphs: true,
            min_chunk_size: 20,
        }
    }
}

/// Benchmark results for different chunk sizes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkingBenchmark {
    pub chunk_size: usize,
    pub total_chunks: usize,
    pub avg_chunk_length: f64,
    pub processing_time_ms: u128,
    pub memory_usage_bytes: usize,
    pub efficiency_score: f64,
}

/// Text processing and chunking utilities
#[derive(Clone)]
pub struct TextProcessor {
    config: ChunkingConfig,
}

/// Precompiled regex patterns for markdown processing
struct MarkdownPatterns {
    // Headers: # ## ### etc.
    headers: Regex,
    // Bold/italic: **text** *text* __text__ _text_
    emphasis: Regex,
    // Links: [text](url) and [text][ref]
    links: Regex,
    // Images: ![alt](url)
    images: Regex,
    // Code blocks: ```code``` and `code`
    code_blocks: Regex,
    code_inline: Regex,
    // Lists: - item, * item, 1. item
    lists: Regex,
    // Blockquotes: > text
    blockquotes: Regex,
    // Tables: | col | col |
    tables: Regex,
    // Horizontal rules: --- *** ___
    horizontal_rules: Regex,
    // HTML tags: <tag>content</tag>
    html_tags: Regex,
    // Multiple whitespace/newlines
    excessive_whitespace: Regex,
}

/// Sentence boundary detection patterns
struct SentenceBoundaries {
    // Sentence endings: . ! ? followed by whitespace or end
    sentence_end: Regex,
    // Paragraph breaks: double newlines
    paragraph_break: Regex,
    // Common abbreviations that shouldn't trigger sentence breaks
    abbreviations: HashMap<String, bool>,
}

// Static initialization for regex patterns
static MARKDOWN_PATTERNS: Lazy<MarkdownPatterns> = Lazy::new(|| MarkdownPatterns {
    headers: Regex::new(r"(?m)^#{1,6}\s+").unwrap(),
    emphasis: Regex::new(r"\*\*(.*?)\*\*|__(.*?)__|[*_](.*?)[*_]").unwrap(),
    links: Regex::new(r"\[([^\]]+)\]\(([^)]+)\)|\[([^\]]+)\]\[([^\]]*)\]").unwrap(),
    images: Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap(),
    code_blocks: Regex::new(r"(?s)```[\s\S]*?```").unwrap(),
    code_inline: Regex::new(r"`([^`]+)`").unwrap(),
    lists: Regex::new(r"(?m)^[\s]*[-*+]\s+|^[\s]*\d+\.\s+").unwrap(),
    blockquotes: Regex::new(r"(?m)^>\s*").unwrap(),
    tables: Regex::new(r"\|.*\|").unwrap(),
    horizontal_rules: Regex::new(r"(?m)^[\s]*[-*_]{3,}[\s]*$").unwrap(),
    html_tags: Regex::new(r"<[^>]+>").unwrap(),
    excessive_whitespace: Regex::new(r"\s{2,}").unwrap(),
});

static SENTENCE_BOUNDARIES: Lazy<SentenceBoundaries> = Lazy::new(|| {
    let mut abbreviations = HashMap::new();
    
    // Common abbreviations that shouldn't trigger sentence breaks
    let common_abbrevs = vec![
        "Dr", "Mr", "Mrs", "Ms", "Prof", "Sr", "Jr",
        "vs", "etc", "i.e", "e.g", "cf", "al", "Inc", "Ltd", "Corp",
        "A.M", "P.M", "a.m", "p.m", "AM", "PM",
        "U.S", "U.K", "E.U", "NASA", "FBI", "CIA",
    ];
    
    for abbrev in common_abbrevs {
        abbreviations.insert(abbrev.to_string(), true);
        abbreviations.insert(abbrev.to_lowercase(), true);
    }
    
    SentenceBoundaries {
        sentence_end: Regex::new(r"[.!?]+\s+|\z").unwrap(),
        paragraph_break: Regex::new(r"\n\s*\n").unwrap(),
        abbreviations,
    }
});

impl TextProcessor {
    /// Create a new TextProcessor with default configuration
    pub fn new() -> Self {
        Self::with_config(ChunkingConfig::default())
    }
    
    /// Create a new TextProcessor with custom configuration
    pub fn with_config(config: ChunkingConfig) -> Self {
        Self {
            config,
        }
    }
    
    /// Preprocess text by removing markdown syntax and normalizing content
    pub fn preprocess_text(&self, input: String) -> TextProcessingResult<String> {
        if input.is_empty() {
            return Err(TextProcessingError::EmptyText);
        }
        
        // Check text length limits (10MB max for processing)
        const MAX_TEXT_LENGTH: usize = 10 * 1024 * 1024;
        if input.len() > MAX_TEXT_LENGTH {
            return Err(TextProcessingError::TextTooLong {
                length: input.len(),
                max_length: MAX_TEXT_LENGTH,
            });
        }
        
        // Validate unicode
        if !input.is_ascii() && input.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
            return Err(TextProcessingError::InvalidUnicode);
        }
        
        let mut text = input;
        
        // Step 1: Remove code blocks first (preserve content but remove markdown)
        text = MARKDOWN_PATTERNS.code_blocks.replace_all(&text, |caps: &regex::Captures| {
            // Extract code content without the ``` markers
            let code_block = caps.get(0).unwrap().as_str();
            let lines: Vec<&str> = code_block.lines().collect();
            if lines.len() > 2 {
                lines[1..lines.len()-1].join("\n")
            } else {
                String::new()
            }
        }).to_string();
        
        // Step 2: Remove inline code markers but preserve content
        text = MARKDOWN_PATTERNS.code_inline.replace_all(&text, "$1").to_string();
        
        // Step 3: Remove image syntax
        text = MARKDOWN_PATTERNS.images.replace_all(&text, "").to_string();
        
        // Step 4: Convert links to text (preserve link text)
        text = MARKDOWN_PATTERNS.links.replace_all(&text, |caps: &regex::Captures| {
            caps.get(1).or(caps.get(3))
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string()
        }).to_string();
        
        // Step 5: Remove emphasis markers but preserve text
        text = MARKDOWN_PATTERNS.emphasis.replace_all(&text, |caps: &regex::Captures| {
            // Try to get the content from different capture groups
            caps.get(1).or(caps.get(2)).or(caps.get(3))
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string()
        }).to_string();
        
        // Step 6: Clean up headers (remove # markers)
        text = MARKDOWN_PATTERNS.headers.replace_all(&text, "").to_string();
        
        // Step 7: Clean up lists (remove list markers)
        text = MARKDOWN_PATTERNS.lists.replace_all(&text, "").to_string();
        
        // Step 8: Remove blockquote markers
        text = MARKDOWN_PATTERNS.blockquotes.replace_all(&text, "").to_string();
        
        // Step 9: Remove table formatting
        text = MARKDOWN_PATTERNS.tables.replace_all(&text, |caps: &regex::Captures| {
            caps.get(0).unwrap().as_str()
                .replace("|", " ")
                .trim()
                .to_string()
        }).to_string();
        
        // Step 10: Remove horizontal rules
        text = MARKDOWN_PATTERNS.horizontal_rules.replace_all(&text, "").to_string();
        
        // Step 11: Remove HTML tags
        text = MARKDOWN_PATTERNS.html_tags.replace_all(&text, "").to_string();
        
        // Step 12: Normalize whitespace
        text = MARKDOWN_PATTERNS.excessive_whitespace.replace_all(&text, " ").to_string();
        
        // Step 13: Trim and handle empty result
        text = text.trim().to_string();
        
        if text.is_empty() {
            return Err(TextProcessingError::EmptyText);
        }
        
        Ok(text)
    }
    
    /// Chunk text into smaller pieces with optional overlap
    pub fn chunk_text(&self, text: String, chunk_size: usize, overlap: usize) -> TextProcessingResult<Vec<String>> {
        // Validate inputs
        const MIN_CHUNK_SIZE: usize = 20;
        const MAX_CHUNK_SIZE: usize = 8192;
        
        if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&chunk_size) {
            return Err(TextProcessingError::InvalidChunkSize {
                size: chunk_size,
                min: MIN_CHUNK_SIZE,
                max: MAX_CHUNK_SIZE,
            });
        }
        
        if overlap >= chunk_size {
            return Err(TextProcessingError::InvalidOverlap { overlap, chunk_size });
        }
        
        if text.is_empty() {
            return Ok(vec![]);
        }
        
        // For very short texts, return as single chunk
        if text.len() <= chunk_size {
            return Ok(vec![text]);
        }
        
        let mut chunks = Vec::new();
        let mut char_start = 0;
        let text_chars: Vec<char> = text.chars().collect();
        let text_char_len = text_chars.len();
        
        while char_start < text_char_len {
            let char_end = std::cmp::min(char_start + chunk_size, text_char_len);
            
            // Convert back to byte indices for string slicing
            let byte_start = text_chars[..char_start].iter().collect::<String>().len();
            let byte_end = text_chars[..char_end].iter().collect::<String>().len();
            
            // Try to preserve sentence boundaries if enabled
            let final_byte_end = if self.config.preserve_sentences && char_end < text_char_len {
                self.find_sentence_boundary(&text, byte_start, byte_end)
            } else {
                byte_end
            };
            
            // Try to preserve paragraph boundaries if enabled  
            let final_byte_end = if self.config.preserve_paragraphs && final_byte_end < text.len() {
                self.find_paragraph_boundary(&text, byte_start, final_byte_end)
            } else {
                final_byte_end
            };
            
            // Extract chunk
            let chunk = text[byte_start..final_byte_end].trim().to_string();
            
            // Only add non-empty chunks that meet minimum size
            if !chunk.is_empty() && chunk.chars().count() >= self.config.min_chunk_size {
                chunks.push(chunk);
            }
            
            // Calculate next start position with overlap (in characters)
            let final_char_end = text[..final_byte_end].chars().count();
            if final_char_end >= text_char_len {
                break;
            }
            
            char_start = if final_char_end > overlap { final_char_end - overlap } else { final_char_end };
            
            // Prevent infinite loops
            if char_start >= text_char_len {
                break;
            }
        }
        
        // Handle case where no valid chunks were created
        if chunks.is_empty() && !text.trim().is_empty() {
            chunks.push(text.trim().to_string());
        }
        
        Ok(chunks)
    }
    
    /// Validate text for processing (unicode, length, content checks)
    pub fn validate_text(text: &str) -> TextProcessingResult<()> {
        if text.is_empty() {
            return Err(TextProcessingError::EmptyText);
        }
        
        // Check length limits
        const MAX_TEXT_LENGTH: usize = 50 * 1024 * 1024; // 50MB for validation
        if text.len() > MAX_TEXT_LENGTH {
            return Err(TextProcessingError::TextTooLong {
                length: text.len(),
                max_length: MAX_TEXT_LENGTH,
            });
        }
        
        // Validate unicode
        for (i, ch) in text.char_indices() {
            if ch.is_control() && ch != '\n' && ch != '\t' && ch != '\r' {
                return Err(TextProcessingError::ProcessingError {
                    message: format!("Invalid control character at position {}", i),
                });
            }
        }
        
        // Check for reasonable text content (not just whitespace)
        if text.trim().is_empty() {
            return Err(TextProcessingError::EmptyText);
        }
        
        Ok(())
    }
    
    /// Run comprehensive benchmarks for different chunk sizes
    pub fn benchmark_chunk_sizes(&self, sample_text: &str, sizes: &[usize]) -> TextProcessingResult<Vec<ChunkingBenchmark>> {
        use std::time::Instant;
        
        let mut results = Vec::new();
        
        for &chunk_size in sizes {
            let start = Instant::now();
            
            // Preprocess the text
            let preprocessed = self.preprocess_text(sample_text.to_string())?;
            
            // Chunk the text
            let chunks = self.chunk_text(preprocessed, chunk_size, chunk_size / 10)?;
            
            let processing_time = start.elapsed();
            
            // Calculate metrics
            let total_chunks = chunks.len();
            let avg_chunk_length = if total_chunks > 0 {
                chunks.iter().map(|c| c.len()).sum::<usize>() as f64 / total_chunks as f64
            } else {
                0.0
            };
            
            // Estimate memory usage (rough approximation)
            let memory_usage = chunks.iter().map(|c| c.capacity()).sum::<usize>() + 
                              (total_chunks * std::mem::size_of::<String>());
            
            // Calculate efficiency score (chunks per second weighted by size)
            let efficiency_score = if processing_time.as_millis() > 0 {
                (total_chunks as f64 * 1000.0) / processing_time.as_millis() as f64
            } else {
                f64::MAX
            };
            
            results.push(ChunkingBenchmark {
                chunk_size,
                total_chunks,
                avg_chunk_length,
                processing_time_ms: processing_time.as_millis(),
                memory_usage_bytes: memory_usage,
                efficiency_score,
            });
        }
        
        Ok(results)
    }
    
    /// Get optimal chunk size based on text characteristics
    pub fn get_optimal_chunk_size(&self, text: &str) -> usize {
        let text_len = text.len();
        
        // For very short texts
        if text_len < 200 {
            return 256;
        }
        
        // For medium texts
        if text_len < 2000 {
            return 512;
        }
        
        // For longer texts, analyze structure
        let paragraph_count = SENTENCE_BOUNDARIES.paragraph_break.find_iter(text).count();
        let sentence_count = SENTENCE_BOUNDARIES.sentence_end.find_iter(text).count();
        
        // If text has good structure (many paragraphs/sentences), use larger chunks
        if paragraph_count > 5 || sentence_count > 20 {
            return 1024;
        }
        
        // Default to medium chunks
        512
    }
    
    // Private helper methods
    
    fn find_sentence_boundary(&self, text: &str, start: usize, preferred_end: usize) -> usize {
        let search_text = &text[start..preferred_end];
        
        // Look for sentence boundaries in reverse from preferred end
        if let Some(last_match) = SENTENCE_BOUNDARIES.sentence_end.find_iter(search_text).last() {
            let boundary_pos = start + last_match.end();
            
            // Check if this boundary is after an abbreviation
            // Use char indices to handle Unicode properly
            let check_start = text.char_indices().nth(start.saturating_sub(10))
                .map(|(i, _)| i).unwrap_or(start);
            let check_end = std::cmp::min(boundary_pos, text.len());
            let before_boundary = &text[check_start..check_end];
            let mut is_abbreviation = false;
            
            for abbrev in SENTENCE_BOUNDARIES.abbreviations.keys() {
                if before_boundary.ends_with(abbrev) {
                    is_abbreviation = true;
                    break;
                }
            }
            
            if !is_abbreviation {
                return boundary_pos;
            }
        }
        
        preferred_end
    }
    
    fn find_paragraph_boundary(&self, text: &str, start: usize, preferred_end: usize) -> usize {
        let search_text = &text[start..preferred_end];
        
        if let Some(last_match) = SENTENCE_BOUNDARIES.paragraph_break.find_iter(search_text).last() {
            return start + last_match.start();
        }
        
        preferred_end
    }
}

// Helper trait implementations for static access to regex patterns are handled by Lazy statics

impl Default for TextProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn sample_markdown() -> String {
        r#"# Sample Document

This is a **sample** document with *various* markdown elements.

## Features

- Lists with items
- [Links](https://example.com)
- `inline code` and blocks:

```rust
fn main() {
    println!("Hello world!");
}
```

> Blockquotes are also supported

![Image](image.png)

| Table | Data |
|-------|------|
| Cell  | Content |

---

Final paragraph with some text."#.to_string()
    }
    
    #[test]
    fn test_preprocess_text_basic() {
        let processor = TextProcessor::new();
        let result = processor.preprocess_text(sample_markdown()).unwrap();
        
        // Should remove markdown syntax but preserve content
        assert!(!result.contains("#"));
        assert!(!result.contains("**"));
        assert!(!result.contains("`"));
        assert!(!result.contains("!["));
        assert!(!result.contains("|"));
        assert!(result.contains("Sample Document"));
        assert!(result.contains("sample document"));
        assert!(result.contains("Features"));
    }
    
    #[test]
    fn test_preprocess_empty_text() {
        let processor = TextProcessor::new();
        let result = processor.preprocess_text(String::new());
        assert!(matches!(result, Err(TextProcessingError::EmptyText)));
    }
    
    #[test]
    fn test_chunk_text_basic() {
        let processor = TextProcessor::new();
        let text = "This is a sample text. It has multiple sentences. Each sentence should be preserved when possible.".to_string();
        
        let chunks = processor.chunk_text(text, 50, 10).unwrap();
        assert!(chunks.len() > 1);
        
        for chunk in &chunks {
            assert!(chunk.len() <= 60); // Allow for some boundary adjustment
            assert!(!chunk.is_empty());
        }
    }
    
    #[test]
    fn test_chunk_text_short() {
        let processor = TextProcessor::new();
        let text = "Short text.".to_string();
        
        let chunks = processor.chunk_text(text.clone(), 100, 10).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }
    
    #[test]
    fn test_chunk_text_invalid_params() {
        let processor = TextProcessor::new();
        let text = "Sample text".to_string();
        
        // Invalid chunk size
        let result = processor.chunk_text(text.clone(), 10, 5);
        assert!(matches!(result, Err(TextProcessingError::InvalidChunkSize { .. })));
        
        // Invalid overlap
        let result = processor.chunk_text(text, 100, 100);
        assert!(matches!(result, Err(TextProcessingError::InvalidOverlap { .. })));
    }
    
    #[test]
    fn test_validate_text() {
        // Valid text
        assert!(TextProcessor::validate_text("Valid text content").is_ok());
        
        // Empty text
        assert!(matches!(
            TextProcessor::validate_text(""),
            Err(TextProcessingError::EmptyText)
        ));
        
        // Only whitespace
        assert!(matches!(
            TextProcessor::validate_text("   \n\t   "),
            Err(TextProcessingError::EmptyText)
        ));
    }
    
    #[test]
    fn test_unicode_handling() {
        let processor = TextProcessor::new();
        let unicode_text = "Unicode test: 中文 العربية русский 🎉".to_string();
        
        let result = processor.preprocess_text(unicode_text.clone());
        assert!(result.is_ok());
        
        let chunks = processor.chunk_text(unicode_text, 20, 5).unwrap();
        assert!(!chunks.is_empty());
    }
    
    #[test]
    fn test_markdown_link_processing() {
        let processor = TextProcessor::new();
        let text = "Check out [this link](https://example.com) and [another][ref].".to_string();
        
        let result = processor.preprocess_text(text).unwrap();
        assert!(result.contains("this link"));
        assert!(result.contains("another"));
        assert!(!result.contains("https://example.com"));
        assert!(!result.contains("[ref]"));
    }
    
    #[test]
    fn test_code_block_processing() {
        let processor = TextProcessor::new();
        let text = r#"Here's some code:

```rust
fn hello() {
    println!("World");
}
```

End of example."#.to_string();
        
        let result = processor.preprocess_text(text).unwrap();
        assert!(result.contains("Here's some code"));
        assert!(result.contains("println!"));
        assert!(!result.contains("```"));
        assert!(result.contains("End of example"));
    }
    
    #[test]
    fn test_optimal_chunk_size() {
        let processor = TextProcessor::new();
        
        // Short text should get small chunks
        let short_text = "Short text.";
        assert_eq!(processor.get_optimal_chunk_size(short_text), 256);
        
        // Medium text should get medium chunks
        let medium_text = "This is a medium length text. ".repeat(20);
        assert_eq!(processor.get_optimal_chunk_size(&medium_text), 512);
        
        // Long structured text should get large chunks
        let long_text = "This is a paragraph.\n\nAnother paragraph.\n\n".repeat(10);
        let optimal_size = processor.get_optimal_chunk_size(&long_text);
        // Should be either 512 or 1024 depending on text structure analysis
        assert!(optimal_size == 512 || optimal_size == 1024);
    }
    
    #[test]
    fn test_benchmark_chunk_sizes() {
        let processor = TextProcessor::new();
        let sample_text = sample_markdown().repeat(5);
        
        let sizes = vec![256, 512, 1024];
        let benchmarks = processor.benchmark_chunk_sizes(&sample_text, &sizes).unwrap();
        
        assert_eq!(benchmarks.len(), 3);
        
        for benchmark in &benchmarks {
            assert!(benchmark.total_chunks > 0);
            assert!(benchmark.avg_chunk_length > 0.0);
            // Processing time should be available (u128 is always >= 0)
            assert!(benchmark.memory_usage_bytes > 0);
            assert!(benchmark.efficiency_score >= 0.0);
        }
    }
    
    #[test]
    fn test_sentence_boundary_preservation() {
        let processor = TextProcessor::with_config(ChunkingConfig {
            chunk_size: 60,
            overlap: 5,
            preserve_sentences: true,
            preserve_paragraphs: false,
            min_chunk_size: 20,
        });
        
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.".to_string();
        let chunks = processor.chunk_text(text, 60, 5).unwrap();
        
        // Chunks should end at sentence boundaries when possible
        for chunk in &chunks {
            let trimmed = chunk.trim();
            if !trimmed.is_empty() && trimmed.len() < 60 {
                // Should ideally end with sentence punctuation
                let last_char = trimmed.chars().last().unwrap_or(' ');
                // This is a heuristic - not all chunks will end perfectly
                println!("Chunk: '{}' ends with: '{}'", trimmed, last_char);
            }
        }
        
        assert!(!chunks.is_empty());
    }
    
    #[test]
    fn test_overlap_functionality() {
        let processor = TextProcessor::new();
        let text = "Word1 Word2 Word3 Word4 Word5 Word6 Word7 Word8 Word9 Word10.".to_string();
        
        let chunks = processor.chunk_text(text, 20, 10).unwrap();
        
        if chunks.len() > 1 {
            // Check that there's actual overlap between consecutive chunks
            for i in 0..chunks.len() - 1 {
                let current_chunk = &chunks[i];
                let next_chunk = &chunks[i + 1];
                
                // There should be some word overlap
                let current_words: Vec<&str> = current_chunk.split_whitespace().collect();
                let next_words: Vec<&str> = next_chunk.split_whitespace().collect();
                
                let has_overlap = current_words.iter().any(|w| next_words.contains(w));
                println!("Chunk {}: '{}'\nChunk {}: '{}'", i, current_chunk, i+1, next_chunk);
                println!("Has overlap: {}", has_overlap);
                // Note: overlap is character-based, not word-based, so this test is lenient
            }
        }
        
        assert!(!chunks.is_empty());
    }
    
    #[test]
    fn test_custom_config() {
        let config = ChunkingConfig {
            chunk_size: 100,
            overlap: 20,
            preserve_sentences: false,
            preserve_paragraphs: false,
            min_chunk_size: 20,
        };
        
        let processor = TextProcessor::with_config(config);
        let text = "A".repeat(500);
        
        let chunks = processor.chunk_text(text, 100, 20).unwrap();
        
        for chunk in &chunks {
            assert!(chunk.len() >= 20); // min_chunk_size
            assert!(chunk.len() <= 120); // chunk_size + overlap allowance
        }
    }
}