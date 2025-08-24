use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use thiserror::Error;

use crate::ollama_client::{OllamaConfig, OllamaClientError};
use crate::text_processing::{TextProcessor, TextProcessingError};

/// Errors that can occur during embedding generation
#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("API error: {status_code} - {message}")]
    Api { status_code: u16, message: String },
    
    #[error("Invalid model: {model} - {reason}")]
    InvalidModel { model: String, reason: String },
    
    #[error("Text processing error: {0}")]
    TextProcessing(#[from] TextProcessingError),
    
    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },
    
    #[error("Empty text provided")]
    EmptyText,
    
    #[error("Invalid embedding response: {reason}")]
    InvalidResponse { reason: String },
    
    #[error("Ollama client error: {0}")]
    OllamaClient(#[from] OllamaClientError),
    
    #[error("Connection pool error: {message}")]
    ConnectionPool { message: String },
}

pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

/// Request payload for Ollama embedding API
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<EmbeddingOptions>,
}

/// Optional parameters for embedding generation
#[derive(Debug, Serialize)]
struct EmbeddingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

/// Response from Ollama embedding API
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
}

/// Batch embedding request for multiple texts
#[derive(Debug, Serialize)]
struct BatchEmbeddingRequest {
    model: String,
    prompts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<EmbeddingOptions>,
}

/// Response for batch embedding requests
#[derive(Debug, Deserialize)]
struct BatchEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
}

/// Configuration for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// HTTP client timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retries for failed requests
    pub max_retries: usize,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Whether to preprocess text before embedding
    pub preprocess_text: bool,
    /// Maximum text length for single embedding
    pub max_text_length: usize,
    /// Batch size for multiple embedding requests
    pub batch_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000, // 30 seconds for embedding generation
            max_retries: 3,
            connection_pool_size: 10,
            preprocess_text: true,
            max_text_length: 8192, // 8KB max text length
            batch_size: 10, // Process up to 10 texts at once
        }
    }
}

/// Core embedding generator with HTTP client and connection pooling
#[derive(Clone)]
pub struct EmbeddingGenerator {
    client: Client,
    ollama_config: OllamaConfig,
    embedding_config: EmbeddingConfig,
    text_processor: TextProcessor,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator with default configuration
    pub fn new(ollama_config: OllamaConfig) -> Self {
        let embedding_config = EmbeddingConfig::default();
        Self::with_config(ollama_config, embedding_config)
    }
    
    /// Create a new embedding generator with custom configuration
    pub fn with_config(ollama_config: OllamaConfig, embedding_config: EmbeddingConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(embedding_config.timeout_ms))
            .pool_max_idle_per_host(embedding_config.connection_pool_size)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            client,
            ollama_config,
            embedding_config,
            text_processor: TextProcessor::new(),
        }
    }
    
    /// Generate embedding for a single text
    pub async fn generate_embedding(&self, text: String, model: String) -> EmbeddingResult<Vec<f32>> {
        let start_time = Instant::now();
        
        // Validate input
        if text.trim().is_empty() {
            return Err(EmbeddingError::EmptyText);
        }
        
        if text.len() > self.embedding_config.max_text_length {
            return Err(EmbeddingError::TextProcessing(
                TextProcessingError::TextTooLong {
                    length: text.len(),
                    max_length: self.embedding_config.max_text_length,
                }
            ));
        }
        
        // Preprocess text if enabled
        let processed_text = if self.embedding_config.preprocess_text {
            self.text_processor.preprocess_text(text)?
        } else {
            text
        };
        
        // Validate model name
        self.validate_model_name(&model)?;
        
        // Generate embedding with retries
        let mut last_error = None;
        for attempt in 0..=self.embedding_config.max_retries {
            match self.generate_single_embedding_request(&processed_text, &model).await {
                Ok(embedding) => {
                    let duration = start_time.elapsed();
                    eprintln!("✅ Generated embedding for {} characters in {:?} (attempt {})", 
                              processed_text.len(), duration, attempt + 1);
                    return Ok(embedding);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.embedding_config.max_retries {
                        // Exponential backoff
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempt as u32)));
                        eprintln!("⚠️ Embedding attempt {} failed, retrying in {:?}...", attempt + 1, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| EmbeddingError::InvalidResponse {
            reason: "Unknown error during embedding generation".to_string()
        }))
    }
    
    /// Generate embeddings for multiple texts in batch
    pub async fn generate_batch_embeddings(
        &self, 
        texts: Vec<String>, 
        model: String
    ) -> EmbeddingResult<Vec<Vec<f32>>> {
        let start_time = Instant::now();
        
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        
        // Validate model name
        self.validate_model_name(&model)?;
        
        // Process texts in batches
        let mut all_embeddings = Vec::new();
        let batch_size = self.embedding_config.batch_size;
        
        for (batch_idx, batch_texts) in texts.chunks(batch_size).enumerate() {
            eprintln!("🔄 Processing batch {} of {} (size: {})", 
                      batch_idx + 1, 
                      texts.len().div_ceil(batch_size), 
                      batch_texts.len());
            
            // Preprocess texts in parallel if enabled
            let processed_batch: Result<Vec<String>, TextProcessingError> = if self.embedding_config.preprocess_text {
                batch_texts.iter()
                    .map(|text| self.text_processor.preprocess_text(text.clone()))
                    .collect()
            } else {
                Ok(batch_texts.to_vec())
            };
            
            let processed_batch = processed_batch?;
            
            // Validate batch texts
            for (i, text) in processed_batch.iter().enumerate() {
                if text.trim().is_empty() {
                    eprintln!("⚠️ Skipping empty text at index {}", batch_idx * batch_size + i);
                    continue;
                }
                
                if text.len() > self.embedding_config.max_text_length {
                    return Err(EmbeddingError::TextProcessing(
                        TextProcessingError::TextTooLong {
                            length: text.len(),
                            max_length: self.embedding_config.max_text_length,
                        }
                    ));
                }
            }
            
            // Try batch request first, fallback to individual requests
            match self.generate_batch_embedding_request(&processed_batch, &model).await {
                Ok(batch_embeddings) => {
                    all_embeddings.extend(batch_embeddings);
                }
                Err(e) => {
                    eprintln!("⚠️ Batch request failed, falling back to individual requests: {}", e);
                    
                    // Fallback: Generate embeddings individually
                    for text in processed_batch {
                        if text.trim().is_empty() {
                            all_embeddings.push(Vec::new()); // Empty embedding for empty text
                            continue;
                        }
                        
                        let embedding = self.generate_single_embedding_request(&text, &model).await?;
                        all_embeddings.push(embedding);
                    }
                }
            }
        }
        
        let duration = start_time.elapsed();
        eprintln!("✅ Generated {} embeddings in {:?}", all_embeddings.len(), duration);
        
        Ok(all_embeddings)
    }
    
    /// Generate a single embedding via HTTP request
    async fn generate_single_embedding_request(&self, text: &str, model: &str) -> EmbeddingResult<Vec<f32>> {
        let request = EmbeddingRequest {
            model: model.to_string(),
            prompt: text.to_string(),
            options: None, // Use default options for now
        };
        
        let url = format!("{}/api/embeddings", self.ollama_config.base_url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(EmbeddingError::Api {
                status_code,
                message: error_text,
            });
        }
        
        let embedding_response: EmbeddingResponse = response.json().await?;
        
        // Validate embedding
        if embedding_response.embedding.is_empty() {
            return Err(EmbeddingError::InvalidResponse {
                reason: "Empty embedding vector returned".to_string(),
            });
        }
        
        Ok(embedding_response.embedding)
    }
    
    /// Generate multiple embeddings via batch HTTP request
    async fn generate_batch_embedding_request(&self, texts: &[String], model: &str) -> EmbeddingResult<Vec<Vec<f32>>> {
        // Note: Ollama API might not support batch requests yet, so this will try and fallback gracefully
        let request = BatchEmbeddingRequest {
            model: model.to_string(),
            prompts: texts.to_vec(),
            options: None,
        };
        
        let url = format!("{}/api/embeddings/batch", self.ollama_config.base_url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // If batch endpoint doesn't exist, return a specific error for fallback
            if status_code == 404 {
                return Err(EmbeddingError::Api {
                    status_code,
                    message: "Batch embedding endpoint not available, using individual requests".to_string(),
                });
            }
            
            return Err(EmbeddingError::Api {
                status_code,
                message: error_text,
            });
        }
        
        let batch_response: BatchEmbeddingResponse = response.json().await?;
        
        // Validate batch response
        if batch_response.embeddings.len() != texts.len() {
            return Err(EmbeddingError::InvalidResponse {
                reason: format!(
                    "Batch response length mismatch: expected {}, got {}",
                    texts.len(),
                    batch_response.embeddings.len()
                ),
            });
        }
        
        for (i, embedding) in batch_response.embeddings.iter().enumerate() {
            if embedding.is_empty() {
                return Err(EmbeddingError::InvalidResponse {
                    reason: format!("Empty embedding vector at index {}", i),
                });
            }
        }
        
        Ok(batch_response.embeddings)
    }
    
    /// Validate that the model name is appropriate for embedding generation
    fn validate_model_name(&self, model: &str) -> EmbeddingResult<()> {
        if model.trim().is_empty() {
            return Err(EmbeddingError::InvalidModel {
                model: model.to_string(),
                reason: "Model name cannot be empty".to_string(),
            });
        }
        
        // Check for known embedding models
        let known_embedding_models = [
            "nomic-embed-text",
            "mxbai-embed-large",
            "all-minilm",
            "sentence-transformers",
        ];
        
        let is_likely_embedding_model = known_embedding_models.iter()
            .any(|&known_model| model.contains(known_model)) || 
            model.contains("embed");
            
        if !is_likely_embedding_model {
            eprintln!("⚠️ Warning: '{}' doesn't appear to be an embedding model", model);
            // Don't error, just warn, as there might be custom embedding models
        }
        
        Ok(())
    }
    
    /// Update the Ollama configuration
    pub fn update_ollama_config(&mut self, config: OllamaConfig) {
        self.ollama_config = config;
    }
    
    /// Get the current embedding configuration
    pub fn get_embedding_config(&self) -> &EmbeddingConfig {
        &self.embedding_config
    }
    
    /// Update the embedding configuration
    pub fn update_embedding_config(&mut self, config: EmbeddingConfig) {
        // Recreate HTTP client with new timeout and pool settings
        self.client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .pool_max_idle_per_host(config.connection_pool_size)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
            
        self.embedding_config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    fn create_test_config() -> OllamaConfig {
        OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            timeout_ms: 5000,
            max_retries: 1,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 1000,
        }
    }
    
    #[tokio::test]
    async fn test_embedding_generator_creation() {
        let config = create_test_config();
        let generator = EmbeddingGenerator::new(config.clone());
        assert_eq!(generator.ollama_config.base_url, config.base_url);
    }
    
    #[tokio::test]
    async fn test_empty_text_validation() {
        let config = create_test_config();
        let generator = EmbeddingGenerator::new(config);
        
        let result = generator.generate_embedding("".to_string(), "test-model".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingError::EmptyText => {},
            _ => panic!("Expected EmptyText error"),
        }
    }
    
    #[tokio::test]
    async fn test_text_too_long_validation() {
        let config = create_test_config();
        let mut embedding_config = EmbeddingConfig::default();
        embedding_config.max_text_length = 100;
        
        let generator = EmbeddingGenerator::with_config(config, embedding_config);
        
        let long_text = "a".repeat(200);
        let result = generator.generate_embedding(long_text, "test-model".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingError::TextProcessing(TextProcessingError::TextTooLong { .. }) => {},
            e => panic!("Expected TextTooLong error, got: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn test_model_validation() {
        let config = create_test_config();
        let generator = EmbeddingGenerator::new(config);
        
        let result = generator.validate_model_name("");
        assert!(result.is_err());
        
        let result = generator.validate_model_name("valid-model");
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_batch_embedding_empty_list() {
        let config = create_test_config();
        let generator = EmbeddingGenerator::new(config);
        
        let result = generator.generate_batch_embeddings(vec![], "test-model".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
    
    #[test]
    fn test_embedding_config_defaults() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.timeout_ms, 30_000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.connection_pool_size, 10);
        assert_eq!(config.preprocess_text, true);
        assert_eq!(config.max_text_length, 8192);
        assert_eq!(config.batch_size, 10);
    }
}