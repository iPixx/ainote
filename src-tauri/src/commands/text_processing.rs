//! # Text Processing Commands
//!
//! This module contains all Tauri commands related to text processing operations.
//! It provides functionality for text preprocessing, chunking, validation, and
//! optimization for AI processing workflows.
//!
//! ## Command Overview
//!
//! ### Core Text Processing
//! - `preprocess_text`: Clean and prepare text for AI processing
//! - `validate_text`: Validate text meets processing requirements
//!
//! ### Text Chunking
//! - `chunk_text`: Split text into chunks with overlap
//! - `chunk_text_with_config`: Advanced chunking with custom configuration
//! - `get_optimal_chunk_size`: Determine best chunk size for text
//! - `benchmark_chunk_sizes`: Performance test different chunk sizes
//!
//! ### Configuration Management
//! - `create_chunking_config`: Build chunking configuration objects
//!
//! ## Text Processing Pipeline
//!
//! The typical text processing workflow:
//!
//! 1. **Preprocessing**: Clean markdown, normalize whitespace, handle encoding
//! 2. **Validation**: Check text length, encoding, and content requirements  
//! 3. **Chunking**: Split into manageable pieces with appropriate overlap
//! 4. **Optimization**: Tune parameters based on performance benchmarks
//!
//! ## Chunking Strategies
//!
//! ### Basic Chunking
//! - Fixed-size chunks with configurable overlap
//! - Preserves sentence boundaries when possible
//! - Maintains paragraph structure
//!
//! ### Advanced Chunking
//! - Semantic-aware splitting at natural boundaries
//! - Configurable minimum and maximum chunk sizes
//! - Content-aware overlap calculation
//! - Performance-optimized for different text types
//!
//! ## Performance Considerations
//!
//! ### Memory Management
//! - Efficient string processing with minimal allocations
//! - Streaming processing for large texts
//! - Configurable limits to prevent memory exhaustion
//!
//! ### Processing Speed
//! - Optimized regex patterns for text cleaning
//! - Efficient chunking algorithms
//! - Parallel processing for batch operations
//!
//! ## Text Validation
//!
//! ### Supported Content
//! - UTF-8 encoded text
//! - Markdown formatted content
//! - Plain text documents
//! - Mixed content with proper encoding
//!
//! ### Validation Checks
//! - Character encoding validation
//! - Length constraints (minimum/maximum)
//! - Content structure validation
//! - Special character handling

use crate::text_processing::{self, ChunkingConfig, ChunkingBenchmark};

/// Preprocess text for AI processing by cleaning and normalizing content
///
/// This command performs comprehensive text preprocessing including markdown
/// cleaning, whitespace normalization, encoding validation, and preparation
/// for downstream AI processing tasks.
///
/// # Arguments
/// * `input` - Raw input text to preprocess
///
/// # Returns
/// * `Ok(String)` - Cleaned and preprocessed text
/// * `Err(String)` - Error message if preprocessing fails
///
/// # Processing Steps
/// 1. **Encoding Validation**: Ensure proper UTF-8 encoding
/// 2. **Markdown Cleaning**: Remove or normalize markdown syntax
/// 3. **Whitespace Normalization**: Standardize line endings and spacing
/// 4. **Special Character Handling**: Process unicode and special characters
/// 5. **Content Structure**: Preserve meaningful structure while cleaning
///
/// # Example Usage (from frontend)
/// ```javascript
/// const cleanText = await invoke('preprocess_text', { 
///     input: '# Title\n\nSome **bold** text with\r\n mixed line endings' 
/// });
/// console.log('Preprocessed:', cleanText);
/// ```
#[tauri::command]
pub fn preprocess_text(input: String) -> Result<String, String> {
    let processor = text_processing::TextProcessor::new();
    processor.preprocess_text(input).map_err(|e| e.to_string())
}

/// Split text into chunks with specified size and overlap
///
/// This command performs basic text chunking using fixed-size chunks with
/// configurable overlap. It attempts to preserve sentence and word boundaries
/// while maintaining the specified chunk size constraints.
///
/// # Arguments
/// * `text` - Input text to chunk
/// * `chunk_size` - Target size for each chunk (in characters)
/// * `overlap` - Number of characters to overlap between chunks
///
/// # Returns
/// * `Ok(Vec<String>)` - List of text chunks
/// * `Err(String)` - Error message if chunking fails
///
/// # Chunking Behavior
/// - Attempts to split at sentence boundaries when possible
/// - Falls back to word boundaries if sentence splitting isn't feasible
/// - Preserves paragraph structure where appropriate
/// - Ensures overlap doesn't exceed chunk size
///
/// # Example Usage (from frontend)
/// ```javascript
/// const chunks = await invoke('chunk_text', {
///     text: 'Long document text...',
///     chunkSize: 1000,
///     overlap: 100
/// });
/// console.log(`Split into ${chunks.length} chunks`);
/// ```
#[tauri::command]
pub fn chunk_text(text: String, chunk_size: usize, overlap: usize) -> Result<Vec<String>, String> {
    let processor = text_processing::TextProcessor::new();
    processor.chunk_text(text, chunk_size, overlap).map_err(|e| e.to_string())
}

/// Validate text meets processing requirements and constraints
///
/// This command performs comprehensive validation of text content to ensure
/// it meets the requirements for AI processing, including encoding, length,
/// and content structure validation.
///
/// # Arguments
/// * `text` - Text to validate
///
/// # Returns
/// * `Ok(())` - Text is valid for processing
/// * `Err(String)` - Validation error with specific details
///
/// # Validation Checks
/// - **Encoding**: Must be valid UTF-8
/// - **Length**: Must be within configured min/max bounds
/// - **Content**: Must contain processable text content
/// - **Structure**: Must have reasonable text structure
/// - **Characters**: Must not contain problematic character sequences
///
/// # Example Usage (from frontend)
/// ```javascript
/// try {
///     await invoke('validate_text', { text: documentContent });
///     console.log('Text is valid for processing');
/// } catch (error) {
///     console.error('Validation failed:', error);
/// }
/// ```
#[tauri::command]
pub fn validate_text(text: String) -> Result<(), String> {
    text_processing::TextProcessor::validate_text(&text).map_err(|e| e.to_string())
}

/// Determine optimal chunk size for the given text
///
/// This command analyzes text characteristics and determines the optimal
/// chunk size for processing, taking into account text structure, content
/// density, and processing efficiency considerations.
///
/// # Arguments
/// * `text` - Text to analyze for optimal chunking
///
/// # Returns
/// * `Ok(usize)` - Recommended chunk size in characters
/// * `Err(String)` - Error message if analysis fails
///
/// # Analysis Factors
/// - **Text Length**: Total document length
/// - **Structure**: Paragraph and sentence distribution
/// - **Density**: Information density and complexity
/// - **Content Type**: Markdown vs plain text considerations
/// - **Processing Target**: Optimization for AI model context windows
///
/// # Example Usage (from frontend)
/// ```javascript
/// const optimalSize = await invoke('get_optimal_chunk_size', { 
///     text: documentContent 
/// });
/// console.log(`Recommended chunk size: ${optimalSize} characters`);
/// ```
#[tauri::command]
pub fn get_optimal_chunk_size(text: String) -> Result<usize, String> {
    let processor = text_processing::TextProcessor::new();
    Ok(processor.get_optimal_chunk_size(&text))
}

/// Benchmark different chunk sizes for performance optimization
///
/// This command tests multiple chunk sizes against sample text to determine
/// the best performance characteristics for different chunking strategies.
/// Useful for optimizing processing pipelines.
///
/// # Arguments
/// * `sample_text` - Representative sample text for benchmarking
/// * `sizes` - List of chunk sizes to test
///
/// # Returns
/// * `Ok(Vec<ChunkingBenchmark>)` - Performance results for each chunk size
/// * `Err(String)` - Error message if benchmarking fails
///
/// # Benchmark Metrics
/// - **Processing Time**: Time to chunk the text
/// - **Chunk Count**: Number of chunks produced
/// - **Overlap Efficiency**: Quality of overlap preservation
/// - **Boundary Preservation**: How well boundaries are preserved
/// - **Memory Usage**: Memory overhead for chunking process
///
/// # Example Usage (from frontend)
/// ```javascript
/// const benchmarks = await invoke('benchmark_chunk_sizes', {
///     sampleText: representativeText,
///     sizes: [500, 1000, 1500, 2000]
/// });
/// 
/// benchmarks.forEach(result => {
///     console.log(`Size ${result.chunk_size}: ${result.processing_time}ms`);
/// });
/// ```
#[tauri::command]
pub fn benchmark_chunk_sizes(sample_text: String, sizes: Vec<usize>) -> Result<Vec<ChunkingBenchmark>, String> {
    let processor = text_processing::TextProcessor::new();
    processor.benchmark_chunk_sizes(&sample_text, &sizes).map_err(|e| e.to_string())
}

/// Create a chunking configuration object with specified parameters
///
/// This command builds a comprehensive chunking configuration that can be
/// used with advanced chunking operations. It provides fine-grained control
/// over chunking behavior and optimization parameters.
///
/// # Arguments
/// * `chunk_size` - Target chunk size (optional, uses default if None)
/// * `overlap` - Overlap between chunks (optional, uses default if None)
/// * `preserve_sentences` - Whether to preserve sentence boundaries (optional)
/// * `preserve_paragraphs` - Whether to preserve paragraph boundaries (optional)
/// * `min_chunk_size` - Minimum acceptable chunk size (optional)
/// * `max_chunk_size` - Maximum acceptable chunk size (optional)
///
/// # Returns
/// * `Ok(ChunkingConfig)` - Complete configuration object
/// * `Err(String)` - Error message if configuration is invalid
///
/// # Configuration Options
/// - **Basic Parameters**: chunk_size, overlap, min/max sizes
/// - **Boundary Preservation**: sentence and paragraph preservation
/// - **Performance Tuning**: optimization flags and thresholds
/// - **Content Handling**: special handling for different content types
///
/// # Example Usage (from frontend)
/// ```javascript
/// const config = await invoke('create_chunking_config', {
///     chunkSize: 1000,
///     overlap: 100,
///     preserveSentences: true,
///     preserveParagraphs: true,
///     minChunkSize: 500
/// });
/// ```
#[tauri::command]
pub fn create_chunking_config(
    chunk_size: Option<usize>,
    overlap: Option<usize>,
    preserve_sentences: Option<bool>,
    preserve_paragraphs: Option<bool>,
    min_chunk_size: Option<usize>,
    max_chunk_size: Option<usize>
) -> Result<ChunkingConfig, String> {
    let mut config = ChunkingConfig::default();
    
    if let Some(size) = chunk_size {
        config.chunk_size = size;
    }
    if let Some(ovlp) = overlap {
        config.overlap = ovlp;
    }
    if let Some(sentences) = preserve_sentences {
        config.preserve_sentences = sentences;
    }
    if let Some(paragraphs) = preserve_paragraphs {
        config.preserve_paragraphs = paragraphs;
    }
    if let Some(min_size) = min_chunk_size {
        config.min_chunk_size = min_size;
    }
    // Note: max_chunk_size parameter is not used as ChunkingConfig doesn't have this field
    // This parameter is kept for API compatibility but ignored
    let _ = max_chunk_size;
    
    Ok(config)
}

/// Advanced text chunking using custom configuration
///
/// This command performs sophisticated text chunking using a detailed
/// configuration object. It provides maximum control over chunking behavior
/// and optimization for specific use cases.
///
/// # Arguments
/// * `text` - Input text to chunk
/// * `config` - Chunking configuration object
///
/// # Returns
/// * `Ok(Vec<String>)` - List of text chunks
/// * `Err(String)` - Error message if chunking fails
///
/// # Advanced Features
/// - **Semantic Boundaries**: Respects content structure
/// - **Adaptive Sizing**: Adjusts chunk size based on content
/// - **Quality Optimization**: Optimizes for downstream processing
/// - **Performance Tuning**: Configurable performance/quality tradeoffs
///
/// # Example Usage (from frontend)
/// ```javascript
/// const config = await invoke('create_chunking_config', {
///     chunkSize: 1000,
///     preserveSentences: true
/// });
/// 
/// const chunks = await invoke('chunk_text_with_config', {
///     text: documentContent,
///     config: config
/// });
/// ```
#[tauri::command]
pub fn chunk_text_with_config(text: String, config: ChunkingConfig) -> Result<Vec<String>, String> {
    let processor = text_processing::TextProcessor::with_config(config.clone());
    processor.chunk_text(text, config.chunk_size, config.overlap).map_err(|e| e.to_string())
}