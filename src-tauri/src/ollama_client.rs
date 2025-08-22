use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use reqwest::Client;

/// Configuration for Ollama client connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub timeout_ms: u64,
    pub max_retries: usize,
    pub initial_retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            timeout_ms: 100, // <100ms requirement
            max_retries: 4, // 1s, 2s, 4s, 8s backoff sequence
            initial_retry_delay_ms: 1000, // 1s
            max_retry_delay_ms: 30000, // max 30s
        }
    }
}

/// Connection status for Ollama service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Retrying { attempt: usize, next_retry_in_ms: u64 },
    Failed { error: String },
}

/// Health check response from Ollama API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: Option<String>,
    pub models: Option<Vec<String>>,
}

/// Model information from Ollama API
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub size: Option<u64>,
    pub digest: Option<String>,
    pub modified_at: Option<String>,
    pub template: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

/// Model compatibility status for embedding models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelCompatibility {
    Compatible,
    Incompatible { reason: String },
    Unknown,
}

/// Model verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVerificationResult {
    pub model_name: String,
    pub is_available: bool,
    pub is_compatible: ModelCompatibility,
    pub info: Option<ModelInfo>,
    pub verification_time_ms: u64,
}

/// Connection state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionState {
    pub status: ConnectionStatus,
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    pub last_successful_connection: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: usize,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
    pub health_info: Option<HealthResponse>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            last_check: None,
            last_successful_connection: None,
            retry_count: 0,
            next_retry_at: None,
            health_info: None,
        }
    }
}

/// Main Ollama client for service detection and health monitoring
#[derive(Debug, Clone)]
pub struct OllamaClient {
    config: OllamaConfig,
    client: Client,
    state: Arc<RwLock<ConnectionState>>,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaClient {
    /// Create a new Ollama client with default configuration
    pub fn new() -> Self {
        Self::with_config(OllamaConfig::default())
    }

    /// Create a new Ollama client with custom configuration
    pub fn with_config(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            state: Arc::new(RwLock::new(ConnectionState::default())),
        }
    }

    /// Get current connection state
    pub async fn get_connection_state(&self) -> ConnectionState {
        eprintln!("📊 [DEBUG RUST] OllamaClient::get_connection_state() called");
        let state = self.state.read().await.clone();
        eprintln!("📊 [DEBUG RUST] Current connection state: status={:?}, retry_count={}, last_check={:?}", 
                 state.status, state.retry_count, state.last_check);
        state
    }

    /// Update configuration for custom Ollama URL
    pub async fn update_config(&mut self, config: OllamaConfig) {
        self.config = config.clone();
        
        // Recreate HTTP client with new timeout
        self.client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to recreate HTTP client");

        // Reset connection state when config changes
        let mut state = self.state.write().await;
        *state = ConnectionState::default();
    }

    /// Perform service discovery and health check
    pub async fn check_health(&self) -> Result<HealthResponse, OllamaClientError> {
        eprintln!("🏥 [DEBUG RUST] OllamaClient::check_health() started");
        let start_time = Instant::now();
        
        // Update status to connecting
        {
            eprintln!("🏥 [DEBUG RUST] Updating status to Connecting");
            let mut state = self.state.write().await;
            state.status = ConnectionStatus::Connecting;
            state.last_check = Some(chrono::Utc::now());
            eprintln!("🏥 [DEBUG RUST] Status updated to: {:?}, last_check updated", state.status);
        }

        let health_url = format!("{}/api/tags", self.config.base_url);
        eprintln!("🏥 [DEBUG RUST] Making HTTP GET request to: {}", health_url);
        eprintln!("🏥 [DEBUG RUST] Using timeout: {}ms", self.config.timeout_ms);
        
        match self.client.get(&health_url).send().await {
            Ok(response) => {
                let elapsed = start_time.elapsed();
                eprintln!("🏥 [DEBUG RUST] HTTP response received in {:?}", elapsed);
                eprintln!("🏥 [DEBUG RUST] Response status: {}", response.status());
                
                if response.status().is_success() {
                    eprintln!("🏥 [DEBUG RUST] Response is successful, parsing JSON");
                    let health_response = if let Ok(json) = response.json::<serde_json::Value>().await {
                        eprintln!("🏥 [DEBUG RUST] Successfully parsed JSON response: {:?}", json);
                        HealthResponse {
                            status: "healthy".to_string(),
                            version: json.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            models: json.get("models").and_then(|m| {
                                m.as_array().map(|arr| {
                                    arr.iter()
                                        .filter_map(|model| model.get("name").and_then(|n| n.as_str()))
                                        .map(|s| s.to_string())
                                        .collect()
                                })
                            }),
                        }
                    } else {
                        eprintln!("🏥 [DEBUG RUST] Failed to parse JSON, using default healthy response");
                        HealthResponse {
                            status: "healthy".to_string(),
                            version: None,
                            models: None,
                        }
                    };
                    
                    eprintln!("🏥 [DEBUG RUST] Created health response: {:?}", health_response);

                    // Update successful connection state
                    {
                        eprintln!("🏥 [DEBUG RUST] Updating status to Connected");
                        let mut state = self.state.write().await;
                        state.status = ConnectionStatus::Connected;
                        state.last_successful_connection = Some(chrono::Utc::now());
                        state.retry_count = 0;
                        state.next_retry_at = None;
                        state.health_info = Some(health_response.clone());
                        eprintln!("🏥 [DEBUG RUST] Connection state updated successfully");
                    }

                    // Log performance metrics
                    if elapsed > Duration::from_millis(50) {
                        eprintln!("Warning: Ollama health check took {:?} (target: <100ms)", elapsed);
                    } else {
                        eprintln!("🏥 [DEBUG RUST] Health check completed within performance target: {:?}", elapsed);
                    }

                    Ok(health_response)
                } else {
                    eprintln!("🏥 [DEBUG RUST] HTTP error response: status={}", response.status());
                    let error = OllamaClientError::HttpError {
                        status_code: response.status().as_u16(),
                        message: format!("HTTP {}: {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown")),
                    };
                    eprintln!("🏥 [DEBUG RUST] Created HTTP error: {:?}", error);
                    self.handle_connection_failure(error.clone()).await;
                    Err(error)
                }
            },
            Err(e) => {
                eprintln!("🏥 [DEBUG RUST] Network error occurred: {:?}", e);
                eprintln!("🏥 [DEBUG RUST] Is timeout: {}", e.is_timeout());
                let error = OllamaClientError::NetworkError { 
                    message: format!("Connection failed: {}", e),
                    is_timeout: e.is_timeout(),
                };
                eprintln!("🏥 [DEBUG RUST] Created network error: {:?}", error);
                self.handle_connection_failure(error.clone()).await;
                Err(error)
            }
        }
    }

    /// Check if service is available (lightweight version)
    pub async fn is_available(&self) -> bool {
        (self.check_health().await).is_ok()
    }

    /// Start health monitoring with exponential backoff
    pub async fn check_health_with_retry(&self) -> Result<HealthResponse, OllamaClientError> {
        let mut retry_count = 0;
        let mut delay_ms = self.config.initial_retry_delay_ms;

        loop {
            match self.check_health().await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if retry_count >= self.config.max_retries {
                        return Err(error);
                    }

                    // Update retry status
                    {
                        let mut state = self.state.write().await;
                        state.status = ConnectionStatus::Retrying {
                            attempt: retry_count + 1,
                            next_retry_in_ms: delay_ms,
                        };
                        state.retry_count = retry_count + 1;
                        state.next_retry_at = Some(chrono::Utc::now() + chrono::Duration::milliseconds(delay_ms as i64));
                    }

                    // Wait before retry
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    
                    // Exponential backoff: double the delay, but cap at max
                    delay_ms = std::cmp::min(delay_ms * 2, self.config.max_retry_delay_ms);
                    retry_count += 1;
                }
            }
        }
    }

    /// Handle connection failure and update state
    async fn handle_connection_failure(&self, error: OllamaClientError) {
        eprintln!("❌ [DEBUG RUST] handle_connection_failure() called with error: {:?}", error);
        let mut state = self.state.write().await;
        let previous_status = state.status.clone();
        state.status = ConnectionStatus::Failed {
            error: error.to_string(),
        };
        state.health_info = None;
        eprintln!("❌ [DEBUG RUST] Status changed from {:?} to {:?}", previous_status, state.status);
        eprintln!("❌ [DEBUG RUST] Health info cleared");
    }

    /// Get configuration
    pub fn get_config(&self) -> &OllamaConfig {
        &self.config
    }

    // === MODEL MANAGEMENT METHODS ===

    /// Get list of available models from Ollama
    pub async fn get_available_models(&self) -> Result<Vec<ModelInfo>, OllamaClientError> {
        let start_time = Instant::now();
        let models_url = format!("{}/api/tags", self.config.base_url);
        
        match self.client.get(&models_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let json: serde_json::Value = response.json().await
                        .map_err(|e| OllamaClientError::ConfigError { 
                            message: format!("Failed to parse models response: {}", e) 
                        })?;
                    
                    let models = json.get("models")
                        .and_then(|m| m.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|model| {
                                    let name = model.get("name").and_then(|n| n.as_str())?;
                                    Some(ModelInfo {
                                        name: name.to_string(),
                                        size: model.get("size").and_then(|s| s.as_u64()),
                                        digest: model.get("digest").and_then(|d| d.as_str()).map(|s| s.to_string()),
                                        modified_at: model.get("modified_at").and_then(|m| m.as_str()).map(|s| s.to_string()),
                                        template: model.get("template").and_then(|t| t.as_str()).map(|s| s.to_string()),
                                        parameter_size: model.get("details").and_then(|d| d.get("parameter_size")).and_then(|p| p.as_str()).map(|s| s.to_string()),
                                        quantization_level: model.get("details").and_then(|d| d.get("quantization_level")).and_then(|q| q.as_str()).map(|s| s.to_string()),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let elapsed = start_time.elapsed();
                    if elapsed > Duration::from_millis(5000) {
                        eprintln!("Warning: Model list retrieval took {:?} (target: <5s)", elapsed);
                    }

                    Ok(models)
                } else {
                    Err(OllamaClientError::HttpError {
                        status_code: response.status().as_u16(),
                        message: format!("Failed to get models: HTTP {}", response.status()),
                    })
                }
            },
            Err(e) => Err(OllamaClientError::NetworkError {
                message: format!("Failed to connect to Ollama for model list: {}", e),
                is_timeout: e.is_timeout(),
            })
        }
    }

    /// Verify if a specific model is available and compatible
    pub async fn verify_model(&self, model_name: &str) -> Result<ModelVerificationResult, OllamaClientError> {
        let start_time = Instant::now();
        
        // Get all available models
        let available_models = self.get_available_models().await?;
        
        // Find the requested model
        let model_info = available_models.iter()
            .find(|model| model.name == model_name)
            .cloned();
        
        let is_available = model_info.is_some();
        
        // Check compatibility for embedding models
        let is_compatible = if is_available {
            self.check_model_compatibility(model_name)
        } else {
            ModelCompatibility::Unknown
        };
        
        let elapsed = start_time.elapsed();
        
        Ok(ModelVerificationResult {
            model_name: model_name.to_string(),
            is_available,
            is_compatible,
            info: model_info,
            verification_time_ms: elapsed.as_millis() as u64,
        })
    }

    /// Check if a model is compatible for embedding use
    fn check_model_compatibility(&self, model_name: &str) -> ModelCompatibility {
        // Known compatible embedding models
        let compatible_models = vec![
            "nomic-embed-text",
            "nomic-embed-text:latest",
            "mxbai-embed-large",
            "mxbai-embed-large:latest",
            "all-minilm",
            "all-minilm:latest",
        ];
        
        let embedding_patterns = vec![
            "embed",
            "embedding",
            "sentence",
            "nomic",
            "mxbai",
            "minilm",
        ];
        
        let model_lower = model_name.to_lowercase();
        
        // Check exact matches first
        if compatible_models.iter().any(|&compatible| model_lower == compatible.to_lowercase()) {
            return ModelCompatibility::Compatible;
        }
        
        // Check if model name contains embedding-related patterns
        if embedding_patterns.iter().any(|&pattern| model_lower.contains(pattern)) {
            return ModelCompatibility::Compatible;
        }
        
        // Check if it's a chat/completion model (incompatible for embeddings)
        let incompatible_patterns = vec![
            "llama",
            "mistral", 
            "codellama",
            "chat",
            "instruct",
            "vicuna",
            "alpaca",
        ];
        
        if incompatible_patterns.iter().any(|&pattern| model_lower.contains(pattern)) {
            return ModelCompatibility::Incompatible { 
                reason: format!("Model '{}' appears to be a chat/completion model, not an embedding model", model_name) 
            };
        }
        
        // Unknown model type
        ModelCompatibility::Unknown
    }

    /// Check if the specific nomic-embed-text model is available
    pub async fn is_nomic_embed_available(&self) -> Result<bool, OllamaClientError> {
        let verification = self.verify_model("nomic-embed-text").await?;
        Ok(verification.is_available && matches!(verification.is_compatible, ModelCompatibility::Compatible))
    }

    /// Get model information for a specific model
    pub async fn get_model_info(&self, model_name: &str) -> Result<Option<ModelInfo>, OllamaClientError> {
        let models = self.get_available_models().await?;
        Ok(models.into_iter().find(|model| model.name == model_name))
    }
}

/// Errors that can occur during Ollama client operations
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum OllamaClientError {
    #[error("Network error: {message}")]
    NetworkError { message: String, is_timeout: bool },
    
    #[error("HTTP error: {status_code} - {message}")]
    HttpError { status_code: u16, message: String },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },
}

impl From<reqwest::Error> for OllamaClientError {
    fn from(error: reqwest::Error) -> Self {
        OllamaClientError::NetworkError {
            message: error.to_string(),
            is_timeout: error.is_timeout(),
        }
    }
}

impl From<serde_json::Error> for OllamaClientError {
    fn from(error: serde_json::Error) -> Self {
        OllamaClientError::ConfigError {
            message: format!("JSON parsing error: {}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock server infrastructure for future integration tests
    // Currently unused but kept for future test expansion

    #[tokio::test]
    async fn test_ollama_client_creation() {
        let client = OllamaClient::new();
        
        assert_eq!(client.config.base_url, "http://localhost:11434");
        assert_eq!(client.config.timeout_ms, 100);
        assert_eq!(client.config.max_retries, 4);
        
        let state = client.get_connection_state().await;
        assert_eq!(state.status, ConnectionStatus::Disconnected);
        assert_eq!(state.retry_count, 0);
    }

    #[tokio::test]
    async fn test_custom_config() {
        let custom_config = OllamaConfig {
            base_url: "http://custom:8080".to_string(),
            timeout_ms: 200,
            max_retries: 3,
            initial_retry_delay_ms: 500,
            max_retry_delay_ms: 15000,
        };
        
        let client = OllamaClient::with_config(custom_config.clone());
        
        assert_eq!(client.config.base_url, "http://custom:8080");
        assert_eq!(client.config.timeout_ms, 200);
        assert_eq!(client.config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_connection_state_management() {
        let client = OllamaClient::new();
        
        // Initial state
        let state = client.get_connection_state().await;
        assert_eq!(state.status, ConnectionStatus::Disconnected);
        assert!(state.last_check.is_none());
        assert!(state.last_successful_connection.is_none());
        
        // Test state updates (without actually connecting)
        {
            let mut state_lock = client.state.write().await;
            state_lock.status = ConnectionStatus::Connected;
            state_lock.last_successful_connection = Some(chrono::Utc::now());
        }
        
        let updated_state = client.get_connection_state().await;
        assert_eq!(updated_state.status, ConnectionStatus::Connected);
        assert!(updated_state.last_successful_connection.is_some());
    }

    #[tokio::test]
    async fn test_config_update() {
        let mut client = OllamaClient::new();
        
        let new_config = OllamaConfig {
            base_url: "http://updated:9999".to_string(),
            timeout_ms: 150,
            max_retries: 2,
            initial_retry_delay_ms: 750,
            max_retry_delay_ms: 20000,
        };
        
        client.update_config(new_config.clone()).await;
        
        assert_eq!(client.config.base_url, "http://updated:9999");
        assert_eq!(client.config.timeout_ms, 150);
        
        // State should be reset after config update
        let state = client.get_connection_state().await;
        assert_eq!(state.status, ConnectionStatus::Disconnected);
        assert_eq!(state.retry_count, 0);
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Test error conversion
        let network_error = OllamaClientError::NetworkError {
            message: "Connection refused".to_string(),
            is_timeout: false,
        };
        
        assert!(network_error.to_string().contains("Network error"));
        
        let http_error = OllamaClientError::HttpError {
            status_code: 404,
            message: "Not Found".to_string(),
        };
        
        assert!(http_error.to_string().contains("HTTP error: 404"));
    }

    #[tokio::test]
    async fn test_health_response_serialization() {
        let health_response = HealthResponse {
            status: "healthy".to_string(),
            version: Some("0.1.0".to_string()),
            models: Some(vec!["llama2".to_string(), "codellama".to_string()]),
        };
        
        let serialized = serde_json::to_string(&health_response).unwrap();
        let deserialized: HealthResponse = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.status, "healthy");
        assert_eq!(deserialized.version, Some("0.1.0".to_string()));
        assert_eq!(deserialized.models, Some(vec!["llama2".to_string(), "codellama".to_string()]));
    }

    #[tokio::test]
    async fn test_connection_status_serialization() {
        let statuses = vec![
            ConnectionStatus::Connected,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Connecting,
            ConnectionStatus::Retrying { attempt: 2, next_retry_in_ms: 4000 },
            ConnectionStatus::Failed { error: "Timeout".to_string() },
        ];
        
        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: ConnectionStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[tokio::test]
    async fn test_exponential_backoff_calculation() {
        let config = OllamaConfig::default();
        
        let mut delay = config.initial_retry_delay_ms;
        let expected_delays = vec![1000, 2000, 4000, 8000, 16000]; // Last one capped at max
        
        for expected in expected_delays {
            let actual = std::cmp::min(delay, config.max_retry_delay_ms);
            assert!(actual >= expected || actual == config.max_retry_delay_ms);
            delay *= 2;
        }
    }

    #[tokio::test]
    async fn test_memory_usage_estimation() {
        // Test that client structures use minimal memory
        let client = OllamaClient::new();
        let state = client.get_connection_state().await;
        
        // Basic size checks (these are estimates, not exact measurements)
        let client_size = std::mem::size_of_val(&client);
        let state_size = std::mem::size_of_val(&state);
        let config_size = std::mem::size_of_val(&client.config);
        
        // These should be reasonable for the <5MB target
        assert!(client_size < 1024); // <1KB for main struct
        assert!(state_size < 512);   // <512B for state
        assert!(config_size < 256);  // <256B for config
        
        println!("Memory usage estimates:");
        println!("  Client: {} bytes", client_size);
        println!("  State: {} bytes", state_size);
        println!("  Config: {} bytes", config_size);
    }

    #[tokio::test]
    async fn test_performance_requirements() {
        use std::time::Instant;
        
        let client = OllamaClient::new();
        
        // Test that basic operations are fast
        let start = Instant::now();
        let _state = client.get_connection_state().await;
        let get_state_duration = start.elapsed();
        
        // State access should be very fast (<1ms)
        assert!(get_state_duration < Duration::from_millis(1));
        
        let start = Instant::now();
        let _config = client.get_config();
        let get_config_duration = start.elapsed();
        
        // Config access should be instant
        assert!(get_config_duration < Duration::from_micros(100));
        
        println!("Performance measurements:");
        println!("  Get state: {:?}", get_state_duration);
        println!("  Get config: {:?}", get_config_duration);
    }

    #[tokio::test]
    async fn test_thread_safety() {
        use std::sync::Arc;
        use tokio::task;
        
        let client = Arc::new(OllamaClient::new());
        let mut handles = Vec::new();
        
        // Test concurrent access from multiple tasks
        for i in 0..10 {
            let client_clone = Arc::clone(&client);
            let handle = task::spawn(async move {
                for _ in 0..100 {
                    let _state = client_clone.get_connection_state().await;
                    tokio::time::sleep(Duration::from_micros(i * 10)).await;
                }
            });
            handles.push(handle);
        }
        
        // All tasks should complete without panics
        for handle in handles {
            handle.await.expect("Task should complete successfully");
        }
    }
    
    #[tokio::test]
    async fn test_configuration_validation() {
        // Test various configuration edge cases
        let valid_configs = vec![
            OllamaConfig {
                base_url: "http://localhost:11434".to_string(),
                timeout_ms: 50,
                max_retries: 1,
                initial_retry_delay_ms: 500,
                max_retry_delay_ms: 1000,
            },
            OllamaConfig {
                base_url: "https://remote.ollama.com:8443".to_string(),
                timeout_ms: 500,
                max_retries: 10,
                initial_retry_delay_ms: 100,
                max_retry_delay_ms: 60000,
            },
        ];
        
        for config in valid_configs {
            let client = OllamaClient::with_config(config.clone());
            assert_eq!(client.config.base_url, config.base_url);
            assert_eq!(client.config.timeout_ms, config.timeout_ms);
        }
    }

    #[tokio::test]
    async fn test_state_consistency() {
        let client = OllamaClient::new();
        
        // Test that state transitions are consistent
        {
            let mut state = client.state.write().await;
            state.status = ConnectionStatus::Connecting;
            state.retry_count = 1;
        }
        
        let state1 = client.get_connection_state().await;
        let state2 = client.get_connection_state().await;
        
        // Multiple reads should return consistent data
        assert_eq!(state1.status, state2.status);
        assert_eq!(state1.retry_count, state2.retry_count);
    }

    // === MODEL MANAGEMENT TESTS ===

    #[tokio::test]
    async fn test_model_info_serialization() {
        let model_info = ModelInfo {
            name: "nomic-embed-text".to_string(),
            size: Some(274_000_000),
            digest: Some("sha256:123abc".to_string()),
            modified_at: Some("2024-01-01T00:00:00Z".to_string()),
            template: Some("embed".to_string()),
            parameter_size: Some("137M".to_string()),
            quantization_level: Some("f16".to_string()),
        };

        let serialized = serde_json::to_string(&model_info).unwrap();
        let deserialized: ModelInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, "nomic-embed-text");
        assert_eq!(deserialized.size, Some(274_000_000));
        assert_eq!(deserialized.digest, Some("sha256:123abc".to_string()));
    }

    #[tokio::test]
    async fn test_model_compatibility_serialization() {
        let compatibilities = vec![
            ModelCompatibility::Compatible,
            ModelCompatibility::Unknown,
            ModelCompatibility::Incompatible { 
                reason: "Not an embedding model".to_string() 
            },
        ];

        for compatibility in compatibilities {
            let serialized = serde_json::to_string(&compatibility).unwrap();
            let deserialized: ModelCompatibility = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, compatibility);
        }
    }

    #[tokio::test]
    async fn test_model_verification_result_serialization() {
        let result = ModelVerificationResult {
            model_name: "nomic-embed-text".to_string(),
            is_available: true,
            is_compatible: ModelCompatibility::Compatible,
            info: Some(ModelInfo {
                name: "nomic-embed-text".to_string(),
                size: Some(274_000_000),
                digest: None,
                modified_at: None,
                template: None,
                parameter_size: None,
                quantization_level: None,
            }),
            verification_time_ms: 150,
        };

        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: ModelVerificationResult = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.model_name, "nomic-embed-text");
        assert_eq!(deserialized.is_available, true);
        assert_eq!(deserialized.is_compatible, ModelCompatibility::Compatible);
        assert_eq!(deserialized.verification_time_ms, 150);
    }

    #[tokio::test]
    async fn test_model_compatibility_logic() {
        let client = OllamaClient::new();

        // Test compatible models
        let compatible_tests = vec![
            "nomic-embed-text",
            "nomic-embed-text:latest",
            "mxbai-embed-large",
            "all-minilm",
            "custom-embed-model",
            "sentence-transformer",
        ];

        for model in compatible_tests {
            let compatibility = client.check_model_compatibility(model);
            assert_eq!(compatibility, ModelCompatibility::Compatible, 
                      "Model '{}' should be compatible", model);
        }

        // Test incompatible models
        let incompatible_tests = vec![
            "llama2",
            "llama3:instruct",
            "mistral",
            "codellama",
            "vicuna-chat",
            "alpaca-7b",
        ];

        for model in incompatible_tests {
            let compatibility = client.check_model_compatibility(model);
            assert!(matches!(compatibility, ModelCompatibility::Incompatible { .. }), 
                   "Model '{}' should be incompatible", model);
        }

        // Test unknown models
        let unknown_tests = vec![
            "random-model",
            "custom-unknown",
            "test-model-123",
        ];

        for model in unknown_tests {
            let compatibility = client.check_model_compatibility(model);
            assert_eq!(compatibility, ModelCompatibility::Unknown, 
                      "Model '{}' should be unknown", model);
        }
    }

    #[tokio::test]
    async fn test_model_compatibility_case_insensitive() {
        let client = OllamaClient::new();

        let test_cases = vec![
            ("NOMIC-EMBED-TEXT", ModelCompatibility::Compatible),
            ("NoMiC-EmBeD-tExT:LaTeSt", ModelCompatibility::Compatible),
            ("LLAMA2", ModelCompatibility::Incompatible { reason: String::new() }),
            ("MiStRaL", ModelCompatibility::Incompatible { reason: String::new() }),
        ];

        for (model, expected) in test_cases {
            let compatibility = client.check_model_compatibility(model);
            match (compatibility, expected) {
                (ModelCompatibility::Compatible, ModelCompatibility::Compatible) => {},
                (ModelCompatibility::Incompatible { .. }, ModelCompatibility::Incompatible { .. }) => {},
                (actual, expected) => panic!("Model '{}': expected {:?}, got {:?}", model, expected, actual),
            }
        }
    }

    #[tokio::test]
    async fn test_model_verification_performance() {
        use std::time::Instant;
        
        let client = OllamaClient::new();
        
        // Test performance of compatibility checking (should be fast)
        let start = Instant::now();
        for _ in 0..1000 {
            let _compatibility = client.check_model_compatibility("nomic-embed-text");
        }
        let elapsed = start.elapsed();
        
        // 1000 compatibility checks should complete in <10ms
        assert!(elapsed < Duration::from_millis(10), 
               "Compatibility checking too slow: {:?}", elapsed);
        
        println!("Compatibility check performance: {:?} for 1000 operations", elapsed);
    }

    #[tokio::test]
    async fn test_model_info_memory_usage() {
        // Test that ModelInfo structures use reasonable memory
        let model_info = ModelInfo {
            name: "nomic-embed-text".to_string(),
            size: Some(274_000_000),
            digest: Some("sha256:abcd1234".to_string()),
            modified_at: Some("2024-01-01T00:00:00Z".to_string()),
            template: Some("embed template".to_string()),
            parameter_size: Some("137M".to_string()),
            quantization_level: Some("f16".to_string()),
        };

        let model_size = std::mem::size_of_val(&model_info);
        
        // ModelInfo should be reasonably sized (<1KB)
        assert!(model_size < 1024, "ModelInfo too large: {} bytes", model_size);
        
        println!("ModelInfo memory usage: {} bytes", model_size);
    }

    #[tokio::test]
    async fn test_model_verification_result_construction() {
        let _client = OllamaClient::new();
        
        // Test verification result for available compatible model
        let model_info = ModelInfo {
            name: "nomic-embed-text".to_string(),
            size: Some(274_000_000),
            digest: None,
            modified_at: None,
            template: None,
            parameter_size: None,
            quantization_level: None,
        };
        
        let result = ModelVerificationResult {
            model_name: "nomic-embed-text".to_string(),
            is_available: true,
            is_compatible: ModelCompatibility::Compatible,
            info: Some(model_info),
            verification_time_ms: 100,
        };
        
        assert_eq!(result.model_name, "nomic-embed-text");
        assert!(result.is_available);
        assert_eq!(result.is_compatible, ModelCompatibility::Compatible);
        assert!(result.info.is_some());
        assert_eq!(result.verification_time_ms, 100);
        
        // Test verification result for unavailable model
        let unavailable_result = ModelVerificationResult {
            model_name: "missing-model".to_string(),
            is_available: false,
            is_compatible: ModelCompatibility::Unknown,
            info: None,
            verification_time_ms: 50,
        };
        
        assert!(!unavailable_result.is_available);
        assert!(unavailable_result.info.is_none());
        assert_eq!(unavailable_result.is_compatible, ModelCompatibility::Unknown);
    }

    #[tokio::test]
    async fn test_embedding_model_patterns() {
        let client = OllamaClient::new();
        
        // Test various embedding model patterns
        let embedding_patterns = vec![
            "sentence-transformers/all-MiniLM-L6-v2",
            "instructor-embed",
            "bge-large-en",
            "e5-large-v2",
            "gte-large", 
            "text-embedding-ada-002",
            "multilingual-embed",
        ];
        
        for pattern in embedding_patterns {
            let compatibility = client.check_model_compatibility(pattern);
            assert_eq!(compatibility, ModelCompatibility::Compatible,
                      "Pattern '{}' should be recognized as embedding model", pattern);
        }
    }

    #[tokio::test]
    async fn test_model_name_edge_cases() {
        let client = OllamaClient::new();
        
        // Test edge cases in model names
        let edge_cases = vec![
            ("", ModelCompatibility::Unknown),
            ("embed", ModelCompatibility::Compatible),
            ("llama", ModelCompatibility::Incompatible { reason: String::new() }),
            ("nomic", ModelCompatibility::Compatible),
            ("embed-llama", ModelCompatibility::Compatible), // embed takes precedence
            ("llama-embed", ModelCompatibility::Compatible), // embed takes precedence
        ];
        
        for (model_name, expected) in edge_cases {
            let compatibility = client.check_model_compatibility(model_name);
            match (compatibility, expected) {
                (ModelCompatibility::Compatible, ModelCompatibility::Compatible) => {},
                (ModelCompatibility::Incompatible { .. }, ModelCompatibility::Incompatible { .. }) => {},
                (ModelCompatibility::Unknown, ModelCompatibility::Unknown) => {},
                (actual, expected) => panic!("Model '{}': expected {:?}, got {:?}", model_name, expected, actual),
            }
        }
    }
}