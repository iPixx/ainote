use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use reqwest::Client;
// StreamExt is used for processing download streams
use futures::StreamExt;

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

/// Download status for model downloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Downloading { 
        progress_percent: f64,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        speed_bytes_per_sec: Option<u64>,
    },
    Completed {
        total_bytes: u64,
        download_time_ms: u64,
    },
    Failed {
        error: String,
        retry_count: usize,
    },
    Cancelled,
}

/// Download progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_name: String,
    pub status: DownloadStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
}

/// Download configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub progress_update_interval_ms: u64,
    pub timeout_ms: u64,
    pub chunk_size: usize,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 2000, // 2 seconds
            progress_update_interval_ms: 500, // 500ms as required
            timeout_ms: 300000, // 5 minutes for large downloads
            chunk_size: 8192, // 8KB chunks for memory efficiency
        }
    }
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
    download_state: Arc<RwLock<std::collections::HashMap<String, DownloadProgress>>>,
    download_config: DownloadConfig,
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
            download_state: Arc::new(RwLock::new(std::collections::HashMap::new())),
            download_config: DownloadConfig::default(),
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

    // === MODEL DOWNLOAD METHODS ===

    /// Download a model from Ollama with progress tracking
    pub async fn download_model(&self, model_name: &str) -> Result<DownloadProgress, OllamaClientError> {
        let _start_time = Instant::now();
        
        // Initialize download progress
        let mut progress = DownloadProgress {
            model_name: model_name.to_string(),
            status: DownloadStatus::Queued,
            started_at: Some(chrono::Utc::now()),
            completed_at: None,
            estimated_completion: None,
        };

        // Store initial progress state
        {
            let mut download_state = self.download_state.write().await;
            download_state.insert(model_name.to_string(), progress.clone());
        }

        // Check if model already exists
        if let Ok(verification) = self.verify_model(model_name).await {
            if verification.is_available {
                progress.status = DownloadStatus::Completed {
                    total_bytes: 0, // Model already exists
                    download_time_ms: 0,
                };
                progress.completed_at = Some(chrono::Utc::now());
                
                let mut download_state = self.download_state.write().await;
                download_state.insert(model_name.to_string(), progress.clone());
                
                return Ok(progress);
            }
        }

        let download_url = format!("{}/api/pull", self.config.base_url);
        let request_body = serde_json::json!({
            "name": model_name,
            "stream": true
        });

        // Start download with retry logic
        let mut retry_count = 0;
        let max_retries = self.download_config.max_retries;

        while retry_count <= max_retries {
            match self.perform_download(&download_url, &request_body, model_name).await {
                Ok(final_progress) => {
                    let mut download_state = self.download_state.write().await;
                    download_state.insert(model_name.to_string(), final_progress.clone());
                    return Ok(final_progress);
                }
                Err(e) => {
                    retry_count += 1;
                    
                    if retry_count > max_retries {
                        // Final failure - update state and return error
                        progress.status = DownloadStatus::Failed {
                            error: e.to_string(),
                            retry_count,
                        };
                        
                        let mut download_state = self.download_state.write().await;
                        download_state.insert(model_name.to_string(), progress.clone());
                        
                        return Err(e);
                    }
                    
                    // Wait before retry
                    tokio::time::sleep(Duration::from_millis(self.download_config.retry_delay_ms)).await;
                    
                    // Update progress with retry info
                    progress.status = DownloadStatus::Failed {
                        error: format!("Retry {}/{}: {}", retry_count, max_retries, e),
                        retry_count,
                    };
                    
                    let mut download_state = self.download_state.write().await;
                    download_state.insert(model_name.to_string(), progress.clone());
                }
            }
        }

        // This should never be reached due to the logic above, but just in case
        Err(OllamaClientError::DownloadError {
            message: format!("Download failed after {} retries", max_retries),
        })
    }

    /// Perform the actual download with streaming progress updates
    async fn perform_download(
        &self,
        url: &str,
        body: &serde_json::Value,
        model_name: &str,
    ) -> Result<DownloadProgress, OllamaClientError> {
        let start_time = Instant::now();
        let last_update = Instant::now();
        
        let response = self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| OllamaClientError::NetworkError {
                message: format!("Failed to start download: {}", e),
                is_timeout: e.is_timeout(),
            })?;

        if !response.status().is_success() {
            return Err(OllamaClientError::HttpError {
                status_code: response.status().as_u16(),
                message: format!("Download request failed: HTTP {}", response.status()),
            });
        }

        let mut stream = response.bytes_stream();
        let mut downloaded_bytes = 0u64;
        let mut total_bytes: Option<u64> = None;
        let mut last_progress_update = Instant::now();

        // Process streaming response
        while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
            let chunk = chunk_result.map_err(|e| OllamaClientError::NetworkError {
                message: format!("Stream error during download: {}", e),
                is_timeout: false,
            })?;

            // Parse JSON response for Ollama pull API
            if let Ok(text) = std::str::from_utf8(&chunk) {
                for line in text.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(line) {
                        // Extract progress information from Ollama response
                        if let Some(status) = json_response.get("status").and_then(|s| s.as_str()) {
                            if status.contains("downloading") || status.contains("pulling") {
                                // Update progress from Ollama's response
                                if let (Some(completed), Some(total)) = (
                                    json_response.get("completed").and_then(|c| c.as_u64()),
                                    json_response.get("total").and_then(|t| t.as_u64()),
                                ) {
                                    downloaded_bytes = completed;
                                    total_bytes = Some(total);
                                }
                            } else if status.contains("verifying") {
                                // Model download complete, now verifying
                                downloaded_bytes = total_bytes.unwrap_or(downloaded_bytes);
                            } else if status.contains("success") || status.contains("complete") {
                                // Download completed successfully
                                let elapsed = start_time.elapsed();
                                
                                let final_progress = DownloadProgress {
                                    model_name: model_name.to_string(),
                                    status: DownloadStatus::Completed {
                                        total_bytes: total_bytes.unwrap_or(downloaded_bytes),
                                        download_time_ms: elapsed.as_millis() as u64,
                                    },
                                    started_at: Some(chrono::Utc::now() - chrono::Duration::milliseconds(elapsed.as_millis() as i64)),
                                    completed_at: Some(chrono::Utc::now()),
                                    estimated_completion: None,
                                };
                                
                                return Ok(final_progress);
                            }
                        }
                        
                        // Check for error status
                        if let Some(error) = json_response.get("error").and_then(|e| e.as_str()) {
                            return Err(OllamaClientError::DownloadError {
                                message: format!("Ollama download error: {}", error),
                            });
                        }
                    }
                }
            }

            // Update progress every 500ms as required
            if last_progress_update.elapsed() >= Duration::from_millis(self.download_config.progress_update_interval_ms) {
                let progress_percent = if let Some(total) = total_bytes {
                    if total > 0 {
                        (downloaded_bytes as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                // Calculate download speed
                let elapsed_secs = last_update.elapsed().as_secs_f64();
                let speed_bytes_per_sec = if elapsed_secs > 0.0 {
                    Some((downloaded_bytes as f64 / start_time.elapsed().as_secs_f64()) as u64)
                } else {
                    None
                };

                // Estimate completion time
                let estimated_completion = if let (Some(speed), Some(total)) = (speed_bytes_per_sec, total_bytes) {
                    if speed > 0 && total > downloaded_bytes {
                        let remaining_bytes = total - downloaded_bytes;
                        let eta_seconds = remaining_bytes as f64 / speed as f64;
                        Some(chrono::Utc::now() + chrono::Duration::seconds(eta_seconds as i64))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let progress = DownloadProgress {
                    model_name: model_name.to_string(),
                    status: DownloadStatus::Downloading {
                        progress_percent,
                        downloaded_bytes,
                        total_bytes,
                        speed_bytes_per_sec,
                    },
                    started_at: Some(chrono::Utc::now() - chrono::Duration::milliseconds(start_time.elapsed().as_millis() as i64)),
                    completed_at: None,
                    estimated_completion,
                };

                // Update download state
                {
                    let mut download_state = self.download_state.write().await;
                    download_state.insert(model_name.to_string(), progress);
                }

                last_progress_update = Instant::now();
            }
        }

        // If we reach here, the stream ended without a success message
        Err(OllamaClientError::DownloadError {
            message: "Download stream ended unexpectedly".to_string(),
        })
    }

    /// Get current download progress for a specific model
    pub async fn get_download_progress(&self, model_name: &str) -> Option<DownloadProgress> {
        let download_state = self.download_state.read().await;
        download_state.get(model_name).cloned()
    }

    /// Get all current downloads
    pub async fn get_all_downloads(&self) -> std::collections::HashMap<String, DownloadProgress> {
        let download_state = self.download_state.read().await;
        download_state.clone()
    }

    /// Cancel a download in progress
    pub async fn cancel_download(&self, model_name: &str) -> Result<(), OllamaClientError> {
        let mut download_state = self.download_state.write().await;
        
        if let Some(mut progress) = download_state.get(model_name).cloned() {
            if matches!(progress.status, DownloadStatus::Downloading { .. } | DownloadStatus::Queued) {
                progress.status = DownloadStatus::Cancelled;
                progress.completed_at = Some(chrono::Utc::now());
                download_state.insert(model_name.to_string(), progress);
                
                // Note: Actual cancellation of HTTP request would require more complex state management
                // For now, we just mark it as cancelled in our tracking
                Ok(())
            } else {
                Err(OllamaClientError::DownloadError {
                    message: format!("Cannot cancel download for '{}': not in progress", model_name),
                })
            }
        } else {
            Err(OllamaClientError::DownloadError {
                message: format!("No download found for model '{}'", model_name),
            })
        }
    }

    /// Clear completed downloads from tracking
    pub async fn clear_completed_downloads(&self) {
        let mut download_state = self.download_state.write().await;
        download_state.retain(|_, progress| {
            !matches!(
                progress.status,
                DownloadStatus::Completed { .. } | DownloadStatus::Failed { .. } | DownloadStatus::Cancelled
            )
        });
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
    
    #[error("Download error: {message}")]
    DownloadError { message: String },
    
    #[error("Disk space error: {message}")]
    DiskSpaceError { message: String },
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

    // === DOWNLOAD FUNCTIONALITY TESTS ===

    #[tokio::test]
    async fn test_download_status_serialization() {
        let statuses = vec![
            DownloadStatus::Queued,
            DownloadStatus::Downloading {
                progress_percent: 45.5,
                downloaded_bytes: 1024 * 1024,
                total_bytes: Some(2 * 1024 * 1024),
                speed_bytes_per_sec: Some(512 * 1024),
            },
            DownloadStatus::Completed {
                total_bytes: 2 * 1024 * 1024,
                download_time_ms: 5000,
            },
            DownloadStatus::Failed {
                error: "Network timeout".to_string(),
                retry_count: 2,
            },
            DownloadStatus::Cancelled,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: DownloadStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[tokio::test]
    async fn test_download_progress_serialization() {
        let progress = DownloadProgress {
            model_name: "nomic-embed-text".to_string(),
            status: DownloadStatus::Downloading {
                progress_percent: 75.0,
                downloaded_bytes: 1536 * 1024,
                total_bytes: Some(2 * 1024 * 1024),
                speed_bytes_per_sec: Some(256 * 1024),
            },
            started_at: Some(chrono::Utc::now()),
            completed_at: None,
            estimated_completion: Some(chrono::Utc::now() + chrono::Duration::seconds(30)),
        };

        let serialized = serde_json::to_string(&progress).unwrap();
        let deserialized: DownloadProgress = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.model_name, "nomic-embed-text");
        assert!(matches!(deserialized.status, DownloadStatus::Downloading { .. }));
        assert!(deserialized.started_at.is_some());
        assert!(deserialized.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_download_config_default() {
        let config = DownloadConfig::default();
        
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 2000);
        assert_eq!(config.progress_update_interval_ms, 500);
        assert_eq!(config.timeout_ms, 300000);
        assert_eq!(config.chunk_size, 8192);
    }

    #[tokio::test]
    async fn test_download_state_management() {
        let client = OllamaClient::new();
        
        // Initially no downloads should exist
        let all_downloads = client.get_all_downloads().await;
        assert!(all_downloads.is_empty());
        
        // Test get_download_progress for non-existent model
        let progress = client.get_download_progress("non-existent-model").await;
        assert!(progress.is_none());
    }

    #[tokio::test]
    async fn test_cancel_non_existent_download() {
        let client = OllamaClient::new();
        
        let result = client.cancel_download("non-existent-model").await;
        assert!(result.is_err());
        
        if let Err(error) = result {
            assert!(matches!(error, OllamaClientError::DownloadError { .. }));
            assert!(error.to_string().contains("No download found"));
        }
    }

    #[tokio::test]
    async fn test_clear_completed_downloads() {
        let client = OllamaClient::new();
        
        // Add a mock completed download to the state
        {
            let mut download_state = client.download_state.write().await;
            download_state.insert("test-model".to_string(), DownloadProgress {
                model_name: "test-model".to_string(),
                status: DownloadStatus::Completed {
                    total_bytes: 1024,
                    download_time_ms: 1000,
                },
                started_at: Some(chrono::Utc::now()),
                completed_at: Some(chrono::Utc::now()),
                estimated_completion: None,
            });
        }
        
        // Verify it exists
        let progress = client.get_download_progress("test-model").await;
        assert!(progress.is_some());
        
        // Clear completed downloads
        client.clear_completed_downloads().await;
        
        // Verify it's been cleared
        let progress = client.get_download_progress("test-model").await;
        assert!(progress.is_none());
    }

    #[tokio::test]
    async fn test_download_error_types() {
        // Test different error types are correctly created
        let network_error = OllamaClientError::NetworkError {
            message: "Connection refused".to_string(),
            is_timeout: false,
        };
        assert!(network_error.to_string().contains("Network error"));

        let download_error = OllamaClientError::DownloadError {
            message: "Download failed".to_string(),
        };
        assert!(download_error.to_string().contains("Download error"));

        let disk_error = OllamaClientError::DiskSpaceError {
            message: "Insufficient space".to_string(),
        };
        assert!(disk_error.to_string().contains("Disk space error"));
    }

    #[tokio::test]
    async fn test_download_progress_calculation() {
        // Test progress percentage calculation
        let downloaded = 1024u64;
        let total = 2048u64;
        let expected_percent = (downloaded as f64 / total as f64) * 100.0;
        
        assert_eq!(expected_percent, 50.0);
        
        // Test edge case: zero total
        let zero_total = 0u64;
        let percent_zero = if zero_total > 0 {
            (downloaded as f64 / zero_total as f64) * 100.0
        } else {
            0.0
        };
        assert_eq!(percent_zero, 0.0);
    }

    #[tokio::test]
    async fn test_speed_calculation() {
        use std::time::Duration;
        
        let bytes_downloaded = 1024u64;
        let elapsed_seconds = 2.0;
        let expected_speed = (bytes_downloaded as f64 / elapsed_seconds) as u64;
        
        assert_eq!(expected_speed, 512); // 512 bytes per second
        
        // Test edge case: zero time elapsed
        let zero_elapsed = 0.0;
        let speed_zero = if zero_elapsed > 0.0 {
            Some((bytes_downloaded as f64 / zero_elapsed) as u64)
        } else {
            None
        };
        assert_eq!(speed_zero, None);
    }

    #[tokio::test]
    async fn test_eta_calculation() {
        let downloaded = 1024u64;
        let total = 2048u64;
        let speed = 512u64; // bytes per second
        
        let remaining = total - downloaded;
        let eta_seconds = remaining as f64 / speed as f64;
        
        assert_eq!(eta_seconds, 2.0); // Should take 2 more seconds
        
        // Test edge case: no speed data
        let eta_no_speed = if speed > 0 && total > downloaded {
            Some(remaining as f64 / speed as f64)
        } else {
            None
        };
        assert!(eta_no_speed.is_some());
    }

    #[tokio::test]
    async fn test_download_config_serialization() {
        let config = DownloadConfig {
            max_retries: 5,
            retry_delay_ms: 3000,
            progress_update_interval_ms: 250,
            timeout_ms: 600000,
            chunk_size: 16384,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: DownloadConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.max_retries, 5);
        assert_eq!(deserialized.retry_delay_ms, 3000);
        assert_eq!(deserialized.progress_update_interval_ms, 250);
        assert_eq!(deserialized.timeout_ms, 600000);
        assert_eq!(deserialized.chunk_size, 16384);
    }

    #[tokio::test]
    async fn test_download_memory_usage() {
        // Test that download structures use reasonable memory
        let progress = DownloadProgress {
            model_name: "nomic-embed-text".to_string(),
            status: DownloadStatus::Downloading {
                progress_percent: 50.0,
                downloaded_bytes: 1024 * 1024,
                total_bytes: Some(2 * 1024 * 1024),
                speed_bytes_per_sec: Some(512 * 1024),
            },
            started_at: Some(chrono::Utc::now()),
            completed_at: None,
            estimated_completion: Some(chrono::Utc::now()),
        };

        let progress_size = std::mem::size_of_val(&progress);
        
        // DownloadProgress should be reasonably sized (<2KB)
        assert!(progress_size < 2048, "DownloadProgress too large: {} bytes", progress_size);
        
        let config = DownloadConfig::default();
        let config_size = std::mem::size_of_val(&config);
        
        // DownloadConfig should be small (<256B)
        assert!(config_size < 256, "DownloadConfig too large: {} bytes", config_size);
        
        println!("Download structure memory usage:");
        println!("  DownloadProgress: {} bytes", progress_size);
        println!("  DownloadConfig: {} bytes", config_size);
    }

    #[tokio::test]
    async fn test_concurrent_download_access() {
        use std::sync::Arc;
        use tokio::task;
        
        let client = Arc::new(OllamaClient::new());
        let mut handles = Vec::new();
        
        // Test concurrent access to download state
        for i in 0..10 {
            let client_clone = Arc::clone(&client);
            let handle = task::spawn(async move {
                let model_name = format!("test-model-{}", i);
                
                // Test various download operations concurrently
                let _progress = client_clone.get_download_progress(&model_name).await;
                let _all_downloads = client_clone.get_all_downloads().await;
                client_clone.clear_completed_downloads().await;
                
                i // Return task identifier
            });
            handles.push(handle);
        }
        
        // All concurrent tasks should complete without panics
        for handle in handles {
            let task_id = handle.await.expect("Concurrent task should complete");
            assert!(task_id < 10);
        }
    }
}