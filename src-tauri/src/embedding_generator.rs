use std::time::{Duration, Instant};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use thiserror::Error;
use tokio::sync::RwLock;

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
    #[allow(dead_code)]  // This field might be used in future
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]  // This field might be used in future
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
    #[allow(dead_code)]  // This field might be used in future
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
    /// Connection warmup on startup
    pub warmup_connections: bool,
    /// Keep-alive duration for connections (seconds)
    pub keep_alive_seconds: u64,
    /// TCP no-delay optimization
    pub tcp_nodelay: bool,
    /// Adaptive timeout based on network conditions
    pub adaptive_timeout: bool,
    /// Connection health check interval (seconds)
    pub health_check_interval_seconds: u64,
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
            warmup_connections: true, // Warm up connections on startup
            keep_alive_seconds: 60, // Keep connections alive for 60 seconds
            tcp_nodelay: true, // Enable TCP no-delay for lower latency
            adaptive_timeout: true, // Adapt timeout based on network conditions
            health_check_interval_seconds: 30, // Health check every 30 seconds
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
    /// Network performance metrics for adaptive timeout
    network_metrics: Arc<RwLock<NetworkMetrics>>,
}

/// Network performance tracking for adaptive optimization
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    /// Moving average of request latencies (ms)
    avg_latency_ms: f64,
    /// Number of successful requests
    success_count: u64,
    /// Number of failed requests
    failure_count: u64,
    /// Last health check timestamp
    last_health_check: Instant,
    /// Connection pool efficiency (0.0-1.0)
    pool_efficiency: f64,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            avg_latency_ms: 0.0,
            success_count: 0,
            failure_count: 0,
            last_health_check: Instant::now(),
            pool_efficiency: 1.0,
        }
    }
}

impl NetworkMetrics {
    /// Update metrics with a new request timing
    fn update_request_timing(&mut self, latency_ms: f64, success: bool) {
        if success {
            self.success_count += 1;
            // Exponential moving average
            let alpha = 0.2;
            if self.avg_latency_ms == 0.0 {
                self.avg_latency_ms = latency_ms;
            } else {
                self.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_latency_ms;
            }
        } else {
            self.failure_count += 1;
        }
    }

    /// Calculate adaptive timeout based on current network performance
    fn calculate_adaptive_timeout(&self, base_timeout_ms: u64) -> u64 {
        if self.success_count == 0 {
            return base_timeout_ms;
        }

        let failure_rate = self.failure_count as f64 / (self.success_count + self.failure_count) as f64;
        let latency_multiplier = if self.avg_latency_ms > 1000.0 { 2.0 } else { 1.5 };
        let failure_multiplier = 1.0 + failure_rate;

        let adaptive_timeout = (base_timeout_ms as f64 * latency_multiplier * failure_multiplier) as u64;
        
        // Cap at reasonable limits
        adaptive_timeout.min(base_timeout_ms * 3).max(base_timeout_ms / 2)
    }
}

impl EmbeddingGenerator {
    /// Create a new embedding generator with default configuration
    pub fn new(ollama_config: OllamaConfig) -> Self {
        let embedding_config = EmbeddingConfig::default();
        Self::with_config(ollama_config, embedding_config)
    }
    
    /// Create a new embedding generator with custom configuration
    pub fn with_config(ollama_config: OllamaConfig, embedding_config: EmbeddingConfig) -> Self {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_millis(embedding_config.timeout_ms))
            .pool_max_idle_per_host(embedding_config.connection_pool_size)
            .pool_idle_timeout(Duration::from_secs(embedding_config.keep_alive_seconds))
            .tcp_keepalive(Duration::from_secs(embedding_config.keep_alive_seconds))
            .tcp_nodelay(embedding_config.tcp_nodelay);

        // Enable HTTP/2 for better connection reuse
        client_builder = client_builder.http2_prior_knowledge();

        let client = client_builder
            .build()
            .unwrap_or_else(|_| Client::new());

        let network_metrics = Arc::new(RwLock::new(NetworkMetrics::default()));
        
        let generator = Self {
            client,
            ollama_config,
            embedding_config: embedding_config.clone(),
            text_processor: TextProcessor::new(),
            network_metrics: network_metrics.clone(),
        };

        // Start connection warmup if enabled
        if embedding_config.warmup_connections {
            let warmup_generator = generator.clone();
            tokio::spawn(async move {
                warmup_generator.warmup_connections().await;
            });
        }

        // Start periodic health checks if enabled
        if embedding_config.health_check_interval_seconds > 0 {
            let health_generator = generator.clone();
            tokio::spawn(async move {
                health_generator.periodic_health_checks().await;
            });
        }

        generator
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
    
    /// Generate a single embedding via HTTP request with adaptive timeout
    async fn generate_single_embedding_request(&self, text: &str, model: &str) -> EmbeddingResult<Vec<f32>> {
        let request_start = Instant::now();
        
        let request = EmbeddingRequest {
            model: model.to_string(),
            prompt: text.to_string(),
            options: None, // Use default options for now
        };
        
        let url = format!("{}/api/embeddings", self.ollama_config.base_url);
        
        // Use adaptive timeout if enabled
        let timeout_ms = if self.embedding_config.adaptive_timeout {
            let metrics = self.network_metrics.read().await;
            metrics.calculate_adaptive_timeout(self.embedding_config.timeout_ms)
        } else {
            self.embedding_config.timeout_ms
        };

        let response = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.client
                .post(&url)
                .json(&request)
                .send()
        ).await
        .map_err(|_| EmbeddingError::Timeout { duration_ms: timeout_ms })?
        .map_err(EmbeddingError::from)?;

        let request_latency = request_start.elapsed().as_millis() as f64;
        
        if !response.status().is_success() {
            // Update metrics with failure
            {
                let mut metrics = self.network_metrics.write().await;
                metrics.update_request_timing(request_latency, false);
            }
            
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
            // Update metrics with failure
            {
                let mut metrics = self.network_metrics.write().await;
                metrics.update_request_timing(request_latency, false);
            }
            
            return Err(EmbeddingError::InvalidResponse {
                reason: "Empty embedding vector returned".to_string(),
            });
        }

        // Update metrics with success
        {
            let mut metrics = self.network_metrics.write().await;
            metrics.update_request_timing(request_latency, true);
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
        let mut client_builder = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .pool_max_idle_per_host(config.connection_pool_size)
            .pool_idle_timeout(Duration::from_secs(config.keep_alive_seconds))
            .tcp_keepalive(Duration::from_secs(config.keep_alive_seconds))
            .tcp_nodelay(config.tcp_nodelay);

        // Enable HTTP/2 for better connection reuse
        client_builder = client_builder.http2_prior_knowledge();

        self.client = client_builder
            .build()
            .unwrap_or_else(|_| Client::new());
            
        self.embedding_config = config;
    }

    /// Warm up connections to Ollama service for better initial performance
    async fn warmup_connections(&self) {
        eprintln!("🔥 Warming up connections to Ollama service...");
        
        // Send a small test request to establish connections
        let test_text = "Connection warmup test";
        let test_model = "nomic-embed-text"; // Common embedding model
        
        for i in 0..self.embedding_config.connection_pool_size.min(3) {
            let warmup_start = Instant::now();
            match self.generate_single_embedding_request(test_text, test_model).await {
                Ok(_) => {
                    let warmup_time = warmup_start.elapsed();
                    eprintln!("✅ Connection {} warmed up in {:?}", i + 1, warmup_time);
                }
                Err(e) => {
                    eprintln!("⚠️ Connection {} warmup failed: {}", i + 1, e);
                    // Continue with other connections
                }
            }
            
            // Small delay between warmup requests
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Periodic health checks to monitor connection quality
    async fn periodic_health_checks(&self) {
        let mut interval = tokio::time::interval(
            Duration::from_secs(self.embedding_config.health_check_interval_seconds)
        );
        
        loop {
            interval.tick().await;
            
            let health_check_start = Instant::now();
            let url = format!("{}/api/tags", self.ollama_config.base_url);
            
            match self.client.get(&url).send().await {
                Ok(response) => {
                    let latency = health_check_start.elapsed().as_millis() as f64;
                    let mut metrics = self.network_metrics.write().await;
                    metrics.last_health_check = Instant::now();
                    
                    if response.status().is_success() {
                        metrics.pool_efficiency = 1.0;
                        eprintln!("💚 Health check passed ({:.1}ms latency)", latency);
                    } else {
                        metrics.pool_efficiency = 0.7; // Degraded but functional
                        eprintln!("⚠️ Health check degraded: HTTP {}", response.status());
                    }
                }
                Err(e) => {
                    let mut metrics = self.network_metrics.write().await;
                    metrics.pool_efficiency = 0.3; // Significant issues
                    eprintln!("❌ Health check failed: {}", e);
                }
            }
        }
    }

    /// Get current network performance metrics
    pub async fn get_network_metrics(&self) -> NetworkMetrics {
        self.network_metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_config() -> OllamaConfig {
        OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            timeout_ms: 5000,
            max_retries: 1,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 1000,
            ..OllamaConfig::default()
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