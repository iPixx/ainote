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
}