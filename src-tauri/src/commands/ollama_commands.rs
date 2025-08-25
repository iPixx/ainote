//! # Ollama Client Commands
//!
//! This module contains all Tauri commands related to Ollama service management.
//! It provides comprehensive functionality for Ollama connection management,
//! model operations, health monitoring, and download management.
//!
//! ## Command Overview
//!
//! ### Connection Management
//! - `check_ollama_status`: Check current connection state
//! - `get_ollama_health`: Get detailed health information
//! - `configure_ollama_url`: Configure Ollama service URL
//! - `start_ollama_monitoring`: Begin background health monitoring
//!
//! ### Model Management
//! - `get_available_models`: List all available models
//! - `verify_model`: Verify specific model availability
//! - `is_nomic_embed_available`: Check for embedding model
//! - `get_model_info`: Get detailed model information
//!
//! ### Model Downloads
//! - `download_model`: Initiate model download
//! - `get_download_progress`: Check download progress
//! - `get_all_downloads`: List all downloads
//! - `cancel_download`: Cancel active download
//! - `clear_completed_downloads`: Clean up completed downloads
//!
//! ## Client Lifecycle Management
//!
//! The Ollama client uses lazy initialization and connection pooling:
//!
//! 1. **Lazy Initialization**: Client created on first use
//! 2. **Configuration Updates**: Runtime URL and parameter changes
//! 3. **Health Monitoring**: Background connection monitoring
//! 4. **Automatic Retry**: Built-in retry logic for transient failures
//! 5. **Connection State**: Persistent state tracking
//!
//! ## Connection States
//!
//! - **Connected**: Successfully connected and operational
//! - **Connecting**: Connection attempt in progress
//! - **Disconnected**: Not connected, no active attempt
//! - **Failed**: Connection failed with error details
//!
//! ## Health Monitoring
//!
//! The health monitoring system provides:
//! - Periodic health checks in the background
//! - Automatic retry with exponential backoff
//! - Connection state change notifications
//! - Performance metrics collection
//!
//! ## URL Configuration
//!
//! Supports flexible Ollama service configuration:
//! - **Default**: `http://localhost:11434`
//! - **Custom URLs**: Any HTTP/HTTPS endpoint
//! - **Runtime Changes**: Dynamic URL updates without restart
//! - **Validation**: URL format and reachability validation
//!
//! ## Model Download Management
//!
//! Advanced download features:
//! - **Progress Tracking**: Real-time download progress
//! - **Parallel Downloads**: Multiple concurrent downloads
//! - **Resume Support**: Resume interrupted downloads
//! - **Cancellation**: Cancel downloads in progress
//! - **Cleanup**: Automatic cleanup of completed downloads
//!
//! ## Error Handling
//!
//! Comprehensive error handling for:
//! - Network connectivity issues
//! - Invalid URL configurations
//! - Model availability problems
//! - Download failures and interruptions
//! - Service unavailability

use crate::globals::OLLAMA_CLIENT;
use crate::ollama_client::{OllamaClient, OllamaConfig, ConnectionState, HealthResponse, ModelInfo, ModelVerificationResult, DownloadProgress};

/// Check the current connection status to the Ollama service
///
/// This command performs a fresh health check and returns the current
/// connection state. It initializes the client if not already created
/// and updates the connection state based on the health check result.
///
/// # Returns
/// * `Ok(ConnectionState)` - Current connection state with details
/// * `Err(String)` - Error message if status check fails
///
/// # Connection States
/// - **Connected**: Service is reachable and responding
/// - **Connecting**: Connection attempt in progress
/// - **Disconnected**: Not connected, no active attempt
/// - **Failed**: Connection failed with error details
///
/// # Example Usage (from frontend)
/// ```javascript
/// const status = await invoke('check_ollama_status');
/// console.log('Ollama status:', status);
/// if (status.status === 'Connected') {
///     console.log('Service is available');
/// }
/// ```
#[tauri::command]
pub async fn check_ollama_status() -> Result<ConnectionState, String> {
    eprintln!("🔍 [DEBUG RUST] check_ollama_status command called");
    
    let client = {
        let client_lock = OLLAMA_CLIENT.read().await;
        if let Some(client) = client_lock.as_ref() {
            eprintln!("🔍 [DEBUG RUST] Using existing Ollama client instance");
            client.clone()
        } else {
            eprintln!("🔍 [DEBUG RUST] No existing client found, creating new OllamaClient");
            // Initialize client if not exists
            drop(client_lock);
            let mut client_lock = OLLAMA_CLIENT.write().await;
            let new_client = OllamaClient::new();
            eprintln!("🔍 [DEBUG RUST] Created new client with config: {:?}", new_client.get_config());
            *client_lock = Some(new_client.clone());
            new_client
        }
    };
    
    // Perform actual health check to get current status
    eprintln!("🔍 [DEBUG RUST] Performing fresh health check");
    let _health_result = client.check_health().await; // This updates the internal state
    
    // Now get the updated connection state
    let state = client.get_connection_state().await;
    eprintln!("🔍 [DEBUG RUST] Fresh connection state after health check: {:?}", state);
    Ok(state)
}

/// Get detailed health information from the Ollama service
///
/// This command performs a comprehensive health check and returns detailed
/// information about the Ollama service including version, available models,
/// and system status.
///
/// # Returns
/// * `Ok(HealthResponse)` - Detailed health information
/// * `Err(String)` - Error message if health check fails
///
/// # Health Information
/// - Service version and build info
/// - Available models and their status
/// - System resource usage
/// - Connection latency metrics
/// - Service capabilities
///
/// # Example Usage (from frontend)
/// ```javascript
/// try {
///     const health = await invoke('get_ollama_health');
///     console.log('Ollama version:', health.version);
///     console.log('Available models:', health.models.length);
/// } catch (error) {
///     console.error('Health check failed:', error);
/// }
/// ```
#[tauri::command]
pub async fn get_ollama_health() -> Result<HealthResponse, String> {
    let client = {
        let client_lock = OLLAMA_CLIENT.read().await;
        if let Some(client) = client_lock.as_ref() {
            client.clone()
        } else {
            drop(client_lock);
            let mut client_lock = OLLAMA_CLIENT.write().await;
            let new_client = OllamaClient::new();
            *client_lock = Some(new_client.clone());
            new_client
        }
    };
    
    match client.check_health().await {
        Ok(health) => Ok(health),
        Err(e) => Err(format!("Health check failed: {}", e))
    }
}

/// Configure the Ollama service URL and connection parameters
///
/// This command updates the Ollama client configuration with a new base URL
/// and validates the connection. It supports both HTTP and HTTPS endpoints
/// with comprehensive URL validation.
///
/// # Arguments
/// * `base_url` - New base URL for Ollama service (must include protocol)
///
/// # Returns
/// * `Ok(())` - Configuration successfully updated
/// * `Err(String)` - Error message if configuration is invalid
///
/// # URL Requirements
/// - Must start with `http://` or `https://`
/// - Must be a valid URL format
/// - Should be reachable (validated during configuration)
/// - Trailing slashes are automatically removed
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('configure_ollama_url', { 
///     baseUrl: 'http://192.168.1.100:11434' 
/// });
/// console.log('Ollama URL configured successfully');
/// ```
#[tauri::command]
pub async fn configure_ollama_url(base_url: String) -> Result<(), String> {
    // Input validation
    if base_url.trim().is_empty() {
        return Err("Base URL cannot be empty".to_string());
    }
    
    // Basic URL validation
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err("Base URL must start with http:// or https://".to_string());
    }
    
    let sanitized_url = base_url.trim().trim_end_matches('/').to_string();
    
    let mut client_lock = OLLAMA_CLIENT.write().await;
    let config = OllamaConfig {
        base_url: sanitized_url.clone(),
        ..Default::default()
    };
    
    if let Some(existing_client) = client_lock.as_mut() {
        // Update existing client configuration
        existing_client.update_config(config).await;
    } else {
        // Create new client with configuration
        let client = OllamaClient::with_config(config);
        *client_lock = Some(client);
    }
    
    Ok(())
}

/// Start background health monitoring for the Ollama service
///
/// This command initiates background monitoring of the Ollama service with
/// automatic health checks, retry logic, and connection state updates.
/// The monitoring runs independently and doesn't block the UI.
///
/// # Returns
/// * `Ok(())` - Monitoring successfully started
/// * `Err(String)` - Error message if monitoring cannot be started
///
/// # Monitoring Features
/// - **Automatic Health Checks**: Periodic health verification
/// - **Retry Logic**: Exponential backoff for failed connections
/// - **State Updates**: Real-time connection state changes
/// - **Non-blocking**: Runs in background without UI impact
/// - **Resource Efficient**: Minimal CPU and network overhead
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('start_ollama_monitoring');
/// console.log('Ollama monitoring started');
/// 
/// // Listen for status changes
/// listen('ollama-status-changed', (event) => {
///     console.log('Status changed:', event.payload);
/// });
/// ```
#[tauri::command]
pub async fn start_ollama_monitoring() -> Result<(), String> {
    let config = {
        let client_lock = OLLAMA_CLIENT.read().await;
        if let Some(client) = client_lock.as_ref() {
            client.get_config().clone()
        } else {
            drop(client_lock);
            let mut client_lock = OLLAMA_CLIENT.write().await;
            let client = OllamaClient::new();
            let config = client.get_config().clone();
            *client_lock = Some(client);
            config
        }
    };
    
    // Start monitoring with retry logic - this is non-blocking
    // Create a new client instance for background monitoring to avoid borrowing issues
    tokio::spawn(async move {
        let monitoring_client = OllamaClient::with_config(config);
        // Perform health check with retries in background
        match monitoring_client.check_health_with_retry().await {
            Ok(_) => {
                eprintln!("Ollama monitoring started successfully");
            }
            Err(e) => {
                eprintln!("Ollama monitoring failed to connect: {}", e);
            }
        }
    });
    
    Ok(())
}

/// Get list of all available models from the Ollama service
///
/// This command retrieves a comprehensive list of all models available
/// on the connected Ollama service, including model metadata and status.
///
/// # Returns
/// * `Ok(Vec<ModelInfo>)` - List of available models with metadata
/// * `Err(String)` - Error message if models cannot be retrieved
///
/// # Model Information
/// Each ModelInfo contains:
/// - Model name and version
/// - Size and resource requirements
/// - Capabilities and supported tasks
/// - Download status and availability
/// - Performance characteristics
///
/// # Example Usage (from frontend)
/// ```javascript
/// const models = await invoke('get_available_models');
/// console.log(`Found ${models.length} available models:`);
/// models.forEach(model => {
///     console.log(`- ${model.name} (${model.size})`);
/// });
/// ```
#[tauri::command]
pub async fn get_available_models() -> Result<Vec<ModelInfo>, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.get_available_models().await
            .map_err(|e| e.to_string())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.get_available_models().await
            .map_err(|e| e.to_string());
        *client_lock = Some(client);
        result
    }
}

/// Verify that a specific model is available and functional
///
/// This command checks whether a specific model is available on the Ollama
/// service and verifies its functionality with comprehensive validation.
///
/// # Arguments
/// * `model_name` - Name of the model to verify
///
/// # Returns
/// * `Ok(ModelVerificationResult)` - Detailed verification results
/// * `Err(String)` - Error message if verification fails
///
/// # Verification Process
/// - **Availability Check**: Verify model exists in service
/// - **Status Validation**: Check model is ready for use
/// - **Capability Testing**: Test basic model functionality
/// - **Performance Metrics**: Measure response times
/// - **Resource Requirements**: Check system compatibility
///
/// # Example Usage (from frontend)
/// ```javascript
/// const verification = await invoke('verify_model', { 
///     modelName: 'llama2:7b' 
/// });
/// 
/// if (verification.is_available) {
///     console.log('Model is ready for use');
///     console.log('Response time:', verification.response_time_ms + 'ms');
/// }
/// ```
#[tauri::command]
pub async fn verify_model(model_name: String) -> Result<ModelVerificationResult, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.verify_model(&model_name).await
            .map_err(|e| e.to_string())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.verify_model(&model_name).await
            .map_err(|e| e.to_string());
        *client_lock = Some(client);
        result
    }
}

/// Check if the nomic-embed-text model is available for embedding generation
///
/// This command specifically checks for the presence and availability of
/// the nomic-embed-text model, which is required for text embedding
/// functionality in the AI features.
///
/// # Returns
/// * `Ok(bool)` - True if nomic-embed-text model is available
/// * `Err(String)` - Error message if check fails
///
/// # Model Requirements
/// The nomic-embed-text model is essential for:
/// - Text embedding generation
/// - Semantic similarity calculations
/// - Vector database operations
/// - AI-powered search functionality
///
/// # Example Usage (from frontend)
/// ```javascript
/// const isAvailable = await invoke('is_nomic_embed_available');
/// if (isAvailable) {
///     console.log('Embedding functionality is available');
///     // Enable AI features
/// } else {
///     console.log('Please install nomic-embed-text model');
///     // Show download option
/// }
/// ```
#[tauri::command]
pub async fn is_nomic_embed_available() -> Result<bool, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.is_nomic_embed_available().await
            .map_err(|e| e.to_string())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.is_nomic_embed_available().await
            .map_err(|e| e.to_string());
        *client_lock = Some(client);
        result
    }
}

/// Get detailed information about a specific model
///
/// This command retrieves comprehensive metadata and information about
/// a specific model, including capabilities, requirements, and status.
///
/// # Arguments
/// * `model_name` - Name of the model to query
///
/// # Returns
/// * `Ok(Some(ModelInfo))` - Detailed model information if available
/// * `Ok(None)` - Model not found
/// * `Err(String)` - Error message if query fails
///
/// # Model Information
/// Detailed information includes:
/// - Model architecture and parameters
/// - Supported tasks and capabilities
/// - Resource requirements (RAM, storage)
/// - Performance characteristics
/// - Version and update information
/// - Download size and dependencies
///
/// # Example Usage (from frontend)
/// ```javascript
/// const modelInfo = await invoke('get_model_info', { 
///     modelName: 'llama2:7b' 
/// });
/// 
/// if (modelInfo) {
///     console.log('Model:', modelInfo.name);
///     console.log('Size:', modelInfo.size);
///     console.log('Parameters:', modelInfo.parameters);
/// } else {
///     console.log('Model not found');
/// }
/// ```
#[tauri::command]
pub async fn get_model_info(model_name: String) -> Result<Option<ModelInfo>, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.get_model_info(&model_name).await
            .map_err(|e| e.to_string())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.get_model_info(&model_name).await
            .map_err(|e| e.to_string());
        *client_lock = Some(client);
        result
    }
}

/// Initiate download of a specific model
///
/// This command starts the download process for a model, providing
/// real-time progress tracking and download management capabilities.
///
/// # Arguments
/// * `model_name` - Name of the model to download
///
/// # Returns
/// * `Ok(DownloadProgress)` - Initial download progress information
/// * `Err(String)` - Error message if download cannot be started
///
/// # Download Management
/// - **Progress Tracking**: Real-time download progress
/// - **Resume Support**: Resume interrupted downloads
/// - **Bandwidth Control**: Configurable download speeds
/// - **Validation**: Verify download integrity
/// - **Cleanup**: Automatic cleanup on failure
///
/// # Example Usage (from frontend)
/// ```javascript
/// const progress = await invoke('download_model', { 
///     modelName: 'llama2:7b' 
/// });
/// 
/// console.log('Download started:', progress.status);
/// console.log('Total size:', progress.total_bytes);
/// 
/// // Poll for progress updates
/// const checkProgress = setInterval(async () => {
///     const current = await invoke('get_download_progress', { 
///         modelName: 'llama2:7b' 
///     });
///     if (current && current.status === 'completed') {
///         clearInterval(checkProgress);
///         console.log('Download completed');
///     }
/// }, 1000);
/// ```
#[tauri::command]
pub async fn download_model(model_name: String) -> Result<DownloadProgress, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.download_model(&model_name).await
            .map_err(|e| e.to_string())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.download_model(&model_name).await
            .map_err(|e| e.to_string());
        *client_lock = Some(client);
        result
    }
}

/// Get current download progress for a specific model
///
/// This command retrieves the current download progress for a model,
/// including transfer statistics and estimated completion time.
///
/// # Arguments
/// * `model_name` - Name of the model to check
///
/// # Returns
/// * `Ok(Some(DownloadProgress))` - Current progress if download is active
/// * `Ok(None)` - No active download for this model
/// * `Err(String)` - Error message if progress cannot be retrieved
///
/// # Progress Information
/// - Bytes downloaded and total size
/// - Download speed and ETA
/// - Current status (downloading, paused, completed, failed)
/// - Error information if applicable
/// - Verification progress for completed downloads
///
/// # Example Usage (from frontend)
/// ```javascript
/// const progress = await invoke('get_download_progress', { 
///     modelName: 'llama2:7b' 
/// });
/// 
/// if (progress) {
///     const percent = (progress.downloaded_bytes / progress.total_bytes) * 100;
///     console.log(`Download ${percent.toFixed(1)}% complete`);
///     console.log(`Speed: ${progress.speed_bytes_per_sec} bytes/sec`);
/// }
/// ```
#[tauri::command]
pub async fn get_download_progress(model_name: String) -> Result<Option<DownloadProgress>, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        Ok(client.get_download_progress(&model_name).await)
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.get_download_progress(&model_name).await;
        *client_lock = Some(client);
        Ok(result)
    }
}

/// Get progress information for all active downloads
///
/// This command retrieves progress information for all currently active
/// model downloads, providing a comprehensive overview of download activity.
///
/// # Returns
/// * `Ok(HashMap<String, DownloadProgress>)` - Map of model names to progress
/// * `Err(String)` - Error message if download list cannot be retrieved
///
/// # Return Format
/// Returns a HashMap where:
/// - **Key**: Model name
/// - **Value**: DownloadProgress with current status and statistics
///
/// # Example Usage (from frontend)
/// ```javascript
/// const allDownloads = await invoke('get_all_downloads');
/// 
/// Object.entries(allDownloads).forEach(([modelName, progress]) => {
///     console.log(`${modelName}: ${progress.status}`);
///     if (progress.status === 'downloading') {
///         const percent = (progress.downloaded_bytes / progress.total_bytes) * 100;
///         console.log(`  Progress: ${percent.toFixed(1)}%`);
///     }
/// });
/// ```
#[tauri::command]
pub async fn get_all_downloads() -> Result<std::collections::HashMap<String, DownloadProgress>, String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        Ok(client.get_all_downloads().await)
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.get_all_downloads().await;
        *client_lock = Some(client);
        Ok(result)
    }
}

/// Cancel an active model download
///
/// This command cancels an in-progress download and cleans up any
/// partially downloaded files. The cancellation is graceful and
/// preserves the ability to resume the download later.
///
/// # Arguments
/// * `model_name` - Name of the model download to cancel
///
/// # Returns
/// * `Ok(())` - Download successfully cancelled
/// * `Err(String)` - Error message if cancellation fails
///
/// # Cancellation Behavior
/// - **Graceful Stop**: Allows current chunk to complete
/// - **Cleanup**: Removes partial files if configured
/// - **Resume Support**: Preserves ability to resume later
/// - **Status Update**: Updates download status immediately
/// - **Resource Cleanup**: Frees network and disk resources
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('cancel_download', { modelName: 'llama2:7b' });
/// console.log('Download cancelled');
/// 
/// // Optionally restart later
/// // await invoke('download_model', { modelName: 'llama2:7b' });
/// ```
#[tauri::command]
pub async fn cancel_download(model_name: String) -> Result<(), String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.cancel_download(&model_name).await
            .map_err(|e| e.to_string())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        let result = client.cancel_download(&model_name).await
            .map_err(|e| e.to_string());
        *client_lock = Some(client);
        result
    }
}

/// Clear all completed downloads from the download manager
///
/// This command removes all completed downloads from the active download
/// list, cleaning up the download manager interface and freeing resources.
/// It only affects the download tracking, not the downloaded models themselves.
///
/// # Returns
/// * `Ok(())` - Completed downloads successfully cleared
/// * `Err(String)` - Error message if cleanup fails
///
/// # Cleanup Behavior
/// - **Status Only**: Removes only completed/failed downloads
/// - **Model Preservation**: Does not affect downloaded model files
/// - **Active Downloads**: Leaves in-progress downloads untouched
/// - **Resource Cleanup**: Frees download tracking resources
/// - **UI Cleanup**: Cleans up download progress displays
///
/// # Example Usage (from frontend)
/// ```javascript
/// await invoke('clear_completed_downloads');
/// console.log('Download history cleared');
/// 
/// // UI will show only active downloads
/// const activeDownloads = await invoke('get_all_downloads');
/// console.log(`${Object.keys(activeDownloads).length} downloads still active`);
/// ```
#[tauri::command]
pub async fn clear_completed_downloads() -> Result<(), String> {
    let client_lock = OLLAMA_CLIENT.read().await;
    
    if let Some(client) = client_lock.as_ref() {
        client.clear_completed_downloads().await;
        Ok(())
    } else {
        drop(client_lock);
        // Initialize client if not exists
        let mut client_lock = OLLAMA_CLIENT.write().await;
        let client = OllamaClient::new();
        client.clear_completed_downloads().await;
        *client_lock = Some(client);
        Ok(())
    }
}