// Comprehensive test suite for the embedding generation system - CORRECTED VERSION
// This module tests all aspects of embedding functionality

#![cfg(test)]

use crate::text_processing::{TextProcessor, TextProcessingError};
use crate::embedding_generator::{EmbeddingGenerator, EmbeddingError, EmbeddingConfig};
use crate::embedding_cache::{EmbeddingCache, CacheConfig};
use crate::ollama_client::OllamaConfig;

use serde_json::json;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use wiremock::{
    Mock, MockServer, ResponseTemplate, 
    matchers::{method, path}
};

// =============================================================================
// TEST UTILITIES AND FIXTURES
// =============================================================================

/// Mock Ollama server for embedding API testing
pub struct MockEmbeddingServer {
    server: MockServer,
    base_url: String,
}

impl MockEmbeddingServer {
    /// Create a new mock embedding server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let base_url = server.uri();
        
        Self { server, base_url }
    }

    /// Get the base URL of the mock server
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Setup successful embedding response
    pub async fn setup_successful_embedding(&self, expected_model: &str) {
        let embedding_vector: Vec<f32> = (0..384).map(|i| i as f32 * 0.01).collect();
        
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&json!({
                        "embedding": embedding_vector,
                        "model": expected_model
                    }))
                    .insert_header("content-type", "application/json")
            )
            .expect(1..)
            .mount(&self.server)
            .await;
    }

    /// Setup network timeout simulation
    pub async fn setup_timeout_response(&self) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(60)) // Force timeout
            )
            .expect(1..)
            .mount(&self.server)
            .await;
    }

    /// Setup API error response
    pub async fn setup_error_response(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(&json!({
                        "error": error_message
                    }))
            )
            .expect(1..)
            .mount(&self.server)
            .await;
    }
}

/// Generate sample text of specified size for testing
pub fn generate_sample_text(size_chars: usize) -> String {
    let base_text = "This is a sample sentence for testing embedding generation. ";
    let repeat_count = (size_chars / base_text.len()) + 1;
    base_text.repeat(repeat_count).chars().take(size_chars).collect()
}

/// Generate markdown sample text with various elements
pub fn generate_markdown_sample() -> String {
    r#"# Test Document

This is a **test document** with various *markdown* elements for preprocessing tests.

## Features

- Lists with items
- [Links](https://example.com) to external sites
- `inline code` snippets

```rust
fn test_function() {
    println!("Hello from code block");
}
```

> This is a blockquote

| Table | Header |
|-------|--------|
| Cell1 | Value1 |

The document ends here with a final sentence."#.to_string()
}

/// Create test Ollama configuration
pub fn create_test_ollama_config(base_url: &str) -> OllamaConfig {
    OllamaConfig {
        base_url: base_url.to_string(),
        timeout_ms: 5_000,
        max_retries: 1,
        initial_retry_delay_ms: 100,
        max_retry_delay_ms: 500,
    }
}

/// Create test embedding configuration
pub fn create_test_embedding_config() -> EmbeddingConfig {
    EmbeddingConfig {
        timeout_ms: 5_000,
        max_retries: 1,
        connection_pool_size: 5,
        preprocess_text: true,
        max_text_length: 4096,
        batch_size: 5,
    }
}

/// Create test cache configuration with temporary directory
pub fn create_test_cache_config() -> (CacheConfig, TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let cache_file_path = temp_dir.path().join("test_embedding_cache.json");
    
    let config = CacheConfig {
        max_entries: 100,
        ttl_seconds: 300,
        persist_to_disk: false, // Disable for tests to avoid filesystem issues
        cache_file_path: Some(cache_file_path.to_string_lossy().to_string()),
        enable_metrics: true,
    };
    
    (config, temp_dir)
}

// =============================================================================
// TEXT PROCESSING UNIT TESTS
// =============================================================================

#[cfg(test)]
mod text_processing_tests {
    use super::*;

    #[test]
    fn test_markdown_preprocessing_comprehensive() {
        let processor = TextProcessor::new();
        let markdown_input = generate_markdown_sample();
        
        let result = processor.preprocess_text(markdown_input).unwrap();
        
        // Verify markdown syntax is removed
        assert!(!result.contains("#"), "Headers should be removed");
        assert!(!result.contains("**"), "Bold markers should be removed");
        assert!(!result.contains("`"), "Code markers should be removed");
        assert!(!result.contains("!["), "Image syntax should be removed");
        assert!(!result.contains("["), "Link brackets should be removed");
        assert!(!result.contains("|"), "Table syntax should be removed");
        
        // Verify content is preserved
        assert!(result.contains("Test Document"));
        assert!(result.contains("test document"));
        assert!(result.contains("Features"));
    }

    #[test]
    fn test_text_chunking_basic() {
        let processor = TextProcessor::new();
        let text = "This is a sample text. It has multiple sentences.".to_string();
        
        let chunks = processor.chunk_text(text, 30, 5).unwrap();
        assert!(!chunks.is_empty(), "Should produce chunks");
        
        for chunk in &chunks {
            assert!(chunk.len() <= 40, "Chunks should not be too large"); // Allow some boundary flexibility
            assert!(!chunk.trim().is_empty(), "Chunks should not be empty");
        }
    }

    #[test]
    fn test_text_validation_edge_cases() {
        // Empty text
        assert!(matches!(
            TextProcessor::validate_text(""),
            Err(TextProcessingError::EmptyText)
        ));
        
        // Whitespace only
        assert!(matches!(
            TextProcessor::validate_text("   \n\t   "),
            Err(TextProcessingError::EmptyText)
        ));
        
        // Valid unicode text
        let unicode_text = "Unicode: 中文 العربية русский 🎉";
        assert!(TextProcessor::validate_text(unicode_text).is_ok());
    }
}

// =============================================================================
// EMBEDDING GENERATOR UNIT TESTS  
// =============================================================================

#[cfg(test)]
mod embedding_generator_tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_generator_creation() {
        let ollama_config = create_test_ollama_config("http://localhost:11434");
        let embedding_config = create_test_embedding_config();
        
        let generator = EmbeddingGenerator::with_config(ollama_config, embedding_config.clone());
        
        // Verify configuration
        let retrieved_config = generator.get_embedding_config();
        assert_eq!(retrieved_config.timeout_ms, embedding_config.timeout_ms);
        assert_eq!(retrieved_config.max_retries, embedding_config.max_retries);
        assert_eq!(retrieved_config.batch_size, embedding_config.batch_size);
    }

    #[tokio::test]
    async fn test_embedding_input_validation() {
        let mock_server = MockEmbeddingServer::new().await;
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let generator = EmbeddingGenerator::new(ollama_config);
        
        // Test empty text validation
        let result = generator.generate_embedding("".to_string(), "test-model".to_string()).await;
        assert!(matches!(result, Err(EmbeddingError::EmptyText)));
        
        // Test whitespace-only text validation
        let result = generator.generate_embedding("   \n\t   ".to_string(), "test-model".to_string()).await;
        assert!(matches!(result, Err(EmbeddingError::EmptyText)));
    }

    #[tokio::test]
    async fn test_successful_embedding_generation() {
        let mock_server = MockEmbeddingServer::new().await;
        mock_server.setup_successful_embedding("nomic-embed-text").await;
        
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let result = generator.generate_embedding(
            "This is a test sentence.".to_string(), 
            "nomic-embed-text".to_string()
        ).await;
        
        assert!(result.is_ok(), "Embedding generation should succeed");
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 384, "Should return 384-dimensional embedding");
        
        // Verify embedding values are reasonable
        assert!(embedding.iter().all(|&val| val.is_finite()), "All values should be finite");
    }

    #[tokio::test]
    async fn test_model_validation() {
        let mock_server = MockEmbeddingServer::new().await;
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let generator = EmbeddingGenerator::new(ollama_config);
        
        // Test empty model validation
        let result = generator.generate_embedding("test text".to_string(), "".to_string()).await;
        assert!(matches!(result, Err(EmbeddingError::InvalidModel { .. })));
    }
}

// =============================================================================
// EMBEDDING CACHE UNIT TESTS
// =============================================================================

#[cfg(test)]
mod embedding_cache_tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let (config, _temp_dir) = create_test_cache_config();
        let cache = EmbeddingCache::with_config(config);
        
        // Test basic set/get operations
        let text = "test text for caching";
        let model = "test-model";
        let embedding = vec![1.0, 2.0, 3.0, 4.0];
        
        // Set embedding in cache
        let result = cache.set(text, model, embedding.clone()).await;
        assert!(result.is_ok(), "Should successfully cache embedding");
        
        // Get embedding from cache
        let cached_result = cache.get(text, model).await.unwrap();
        assert!(cached_result.is_some(), "Should retrieve cached embedding");
        assert_eq!(cached_result.unwrap(), embedding, "Cached embedding should match original");
    }

    #[tokio::test]
    async fn test_cache_miss_scenarios() {
        let (config, _temp_dir) = create_test_cache_config();
        let cache = EmbeddingCache::with_config(config);
        
        // Test cache miss for non-existent text
        let result = cache.get("non-existent text", "test-model").await.unwrap();
        assert!(result.is_none(), "Should return None for cache miss");
    }

    #[tokio::test]
    async fn test_cache_metrics() {
        let (config, _temp_dir) = create_test_cache_config();
        let cache = EmbeddingCache::with_config(config);
        
        let embedding = vec![1.0, 2.0, 3.0];
        let model = "test-model";
        
        // Perform cache operations to generate metrics
        cache.set("text1", model, embedding.clone()).await.unwrap();
        cache.get("text1", model).await.unwrap(); // Hit
        cache.get("missing", model).await.unwrap(); // Miss
        
        let metrics = cache.get_metrics().await;
        
        // Use the correct field names from the actual CacheMetrics struct
        assert_eq!(metrics.hits, 1, "Should track cache hits");
        assert_eq!(metrics.misses, 1, "Should track cache misses");
        assert_eq!(metrics.insertions, 1, "Should track insertions");
    }
}

// =============================================================================
// INTEGRATION TESTS WITH MOCKED OLLAMA API
// =============================================================================

#[cfg(test)]
mod integration_tests_mocked {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_embedding_flow() {
        let mock_server = MockEmbeddingServer::new().await;
        mock_server.setup_successful_embedding("nomic-embed-text").await;
        
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let embedding_config = create_test_embedding_config();
        let (cache_config, _temp_dir) = create_test_cache_config();
        
        // Create components
        let generator = EmbeddingGenerator::with_config(ollama_config, embedding_config);
        let cache = EmbeddingCache::with_config(cache_config);
        
        // Test complete flow
        let text = "# Test Document\n\nThis is a **test**.";
        let model = "nomic-embed-text";
        
        // Generate embedding
        let embedding1 = generator.generate_embedding(text.to_string(), model.to_string()).await.unwrap();
        cache.set(text, model, embedding1.clone()).await.unwrap();
        
        // Retrieve from cache
        let cached_embedding = cache.get(text, model).await.unwrap();
        assert!(cached_embedding.is_some(), "Should retrieve from cache");
        assert_eq!(cached_embedding.unwrap(), embedding1, "Cached embedding should match original");
        
        // Verify metrics
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.hits, 1, "Should record cache hit");
    }

    #[tokio::test]
    async fn test_network_error_recovery() {
        let mock_server = MockEmbeddingServer::new().await;
        
        // Setup server errors followed by success
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&mock_server.server)
            .await;
            
        mock_server.setup_successful_embedding("test-model").await;
        
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let mut embedding_config = create_test_embedding_config();
        embedding_config.max_retries = 2;
        let generator = EmbeddingGenerator::with_config(ollama_config, embedding_config);
        
        let result = generator.generate_embedding(
            "Test error recovery.".to_string(),
            "test-model".to_string()
        ).await;
        
        assert!(result.is_ok(), "Should succeed after retry");
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 384, "Should return valid embedding");
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let mock_server = MockEmbeddingServer::new().await;
        mock_server.setup_error_response(400, "Invalid model").await;
        
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let result = generator.generate_embedding(
            "Test API error.".to_string(),
            "invalid-model".to_string()
        ).await;
        
        assert!(result.is_err(), "Should fail with API error");
        
        match result.unwrap_err() {
            EmbeddingError::Api { status_code, message } => {
                assert_eq!(status_code, 400, "Should preserve status code");
                assert!(message.contains("Invalid model"), "Should preserve error message");
            }
            other => panic!("Expected API error, got: {:?}", other),
        }
    }
}

// =============================================================================
// PERFORMANCE AND VALIDATION TESTS
// =============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_text_processing_performance() {
        let processor = TextProcessor::new();
        
        let test_sizes = vec![1_000, 10_000];
        
        for char_count in test_sizes {
            let sample_text = generate_sample_text(char_count);
            
            let start_time = Instant::now();
            let _processed = processor.preprocess_text(sample_text).unwrap();
            let duration = start_time.elapsed();
            
            println!("Processed {} chars in {:?}", char_count, duration);
            assert!(duration < Duration::from_millis(1000), 
                   "Processing should be under 1 second");
        }
    }

    #[tokio::test]
    async fn test_embedding_generation_performance() {
        let mock_server = MockEmbeddingServer::new().await;
        mock_server.setup_successful_embedding("test-model").await;
        
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let generator = EmbeddingGenerator::new(ollama_config);
        
        let text = generate_sample_text(2000);
        
        let start_time = Instant::now();
        let _embedding = generator.generate_embedding(text, "test-model".to_string()).await.unwrap();
        let duration = start_time.elapsed();
        
        println!("Generated embedding in {:?}", duration);
        assert!(duration < Duration::from_secs(5), 
               "Embedding generation should be reasonable");
    }

    #[tokio::test]
    async fn test_cache_performance() {
        let (mut config, _temp_dir) = create_test_cache_config();
        config.max_entries = 1000;
        let cache = EmbeddingCache::with_config(config);
        
        let embedding = vec![0.1; 384];
        let model = "perf-test-model";
        let entry_count = 100;
        
        // Benchmark cache operations
        let start_time = Instant::now();
        for i in 0..entry_count {
            let text = format!("performance_test_text_{}", i);
            cache.set(&text, model, embedding.clone()).await.unwrap();
        }
        let insert_duration = start_time.elapsed();
        
        let start_time = Instant::now();
        for i in 0..entry_count {
            let text = format!("performance_test_text_{}", i);
            let _ = cache.get(&text, model).await.unwrap();
        }
        let lookup_duration = start_time.elapsed();
        
        println!("Cache insertions: {:?}, lookups: {:?}", insert_duration, lookup_duration);
        
        assert!(insert_duration < Duration::from_secs(5), "Insertions should be fast");
        assert!(lookup_duration < Duration::from_secs(2), "Lookups should be fast");
    }
}

// =============================================================================
// END-TO-END VALIDATION TESTS
// =============================================================================

#[cfg(test)]
mod end_to_end_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_system_validation() {
        let mock_server = MockEmbeddingServer::new().await;
        mock_server.setup_successful_embedding("nomic-embed-text").await;
        
        // Setup all components
        let ollama_config = create_test_ollama_config(mock_server.base_url());
        let embedding_config = create_test_embedding_config();
        let (cache_config, _temp_dir) = create_test_cache_config();
        
        let text_processor = TextProcessor::new();
        let generator = EmbeddingGenerator::with_config(ollama_config, embedding_config);
        let cache = EmbeddingCache::with_config(cache_config);
        
        println!("\n=== Complete System Validation ===");
        
        let test_text = "This is a comprehensive test document.";
        
        // Step 1: Text preprocessing
        let processed_text = text_processor.preprocess_text(test_text.to_string()).unwrap();
        println!("Preprocessing: {} → {} chars", test_text.len(), processed_text.len());
        
        // Step 2: Embedding generation
        let embedding = generator.generate_embedding(processed_text.clone(), "nomic-embed-text".to_string()).await.unwrap();
        cache.set(&processed_text, "nomic-embed-text", embedding.clone()).await.unwrap();
        
        // Step 3: Validation
        assert!(!embedding.is_empty(), "Embedding should not be empty");
        assert_eq!(embedding.len(), 384, "Should have correct dimensionality");
        assert!(embedding.iter().all(|&v| v.is_finite()), "All values should be finite");
        
        // Step 4: Cache validation
        let cached = cache.get(&processed_text, "nomic-embed-text").await.unwrap();
        assert!(cached.is_some(), "Should be cached");
        assert_eq!(cached.unwrap(), embedding, "Cached should match original");
        
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.hits, 1, "Should have cache hit");
        assert_eq!(metrics.insertions, 1, "Should have insertion");
        
        println!("✅ Complete system validation passed");
    }
}