//! # Global State Management
//!
//! This module manages global static instances that need to be shared across the application.
//! These globals are initialized lazily and use Arc<RwLock<>> for thread-safe access.
//!
//! ## Architecture
//!
//! The globals follow a consistent pattern:
//! - **Lazy Initialization**: Created only when first accessed using `once_cell::sync::Lazy`
//! - **Thread Safety**: Wrapped in `Arc<RwLock<>>` for concurrent read/write access
//! - **Optional Content**: Use `Option<T>` to allow for proper initialization lifecycle
//! - **Helper Functions**: Provide async helper functions for easy access and initialization
//!
//! ## Global Instances
//!
//! ### OLLAMA_CLIENT
//! Manages connection to the Ollama service for AI model interactions.
//!
//! ### EMBEDDING_GENERATOR  
//! Handles embedding generation from text using configured AI models.
//!
//! ### EMBEDDING_CACHE
//! Provides caching layer for generated embeddings to improve performance.
//!
//! ## Usage Patterns
//!
//! ```rust
//! // Direct access (requires manual initialization checking)
//! let client = OLLAMA_CLIENT.read().await;
//!
//! // Helper function access (handles initialization automatically)
//! let cache = get_embedding_cache().await;
//! let generator = get_embedding_generator().await;
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

use crate::ollama_client::{OllamaClient, OllamaConfig};
use crate::embedding_generator::EmbeddingGenerator;  
use crate::embedding_cache::EmbeddingCache;

/// Global Ollama client instance for AI model interactions
/// 
/// This client manages the connection to the Ollama service and handles
/// model operations, health checks, and configuration. It's initialized
/// lazily when first accessed and shared across all command handlers.
pub static OLLAMA_CLIENT: Lazy<Arc<RwLock<Option<OllamaClient>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Global embedding generator instance for text-to-vector conversion
///
/// Handles embedding generation from text using configured AI models.
/// Initialized lazily with default configuration and can be reconfigured
/// at runtime through command handlers.
pub static EMBEDDING_GENERATOR: Lazy<Arc<RwLock<Option<EmbeddingGenerator>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Global embedding cache instance for performance optimization
///
/// Provides caching layer for generated embeddings to avoid redundant
/// computation. Includes TTL support, persistence options, and metrics
/// tracking for cache efficiency monitoring.
pub static EMBEDDING_CACHE: Lazy<Arc<RwLock<Option<EmbeddingCache>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Helper function to get or initialize the embedding cache
///
/// This function uses the double-checked locking pattern to ensure
/// thread-safe lazy initialization. If the cache doesn't exist,
/// it creates a new one with default configuration.
///
/// # Returns
/// 
/// Returns a cloned `EmbeddingCache` instance that can be used
/// for caching operations.
///
/// # Example
///
/// ```rust
/// let cache = get_embedding_cache().await;
/// cache.get("some_text", "model_name").await;
/// ```
pub async fn get_embedding_cache() -> EmbeddingCache {
    let cache_lock = EMBEDDING_CACHE.read().await;
    if let Some(cache) = cache_lock.as_ref() {
        cache.clone()
    } else {
        drop(cache_lock);
        // Initialize cache if not exists
        let mut cache_lock = EMBEDDING_CACHE.write().await;
        
        // Double-check pattern to avoid race conditions
        if let Some(cache) = cache_lock.as_ref() {
            cache.clone()
        } else {
            let cache = EmbeddingCache::new();
            *cache_lock = Some(cache.clone());
            cache
        }
    }
}

/// Helper function to get or initialize the embedding generator
///
/// This function uses the double-checked locking pattern to ensure
/// thread-safe lazy initialization. If the generator doesn't exist,
/// it creates a new one with default configuration.
///
/// # Returns
///
/// Returns a cloned `EmbeddingGenerator` instance that can be used
/// for generating embeddings from text.
///
/// # Example
///
/// ```rust
/// let generator = get_embedding_generator().await;
/// let embeddings = generator.generate_embedding("text", "model").await?;
/// ```
pub async fn get_embedding_generator() -> EmbeddingGenerator {
    let generator_lock = EMBEDDING_GENERATOR.read().await;
    if let Some(generator) = generator_lock.as_ref() {
        generator.clone()
    } else {
        drop(generator_lock);
        // Initialize generator if not exists
        let mut generator_lock = EMBEDDING_GENERATOR.write().await;
        
        // Double-check pattern to avoid race conditions
        if let Some(generator) = generator_lock.as_ref() {
            generator.clone()
        } else {
            // Get Ollama config from global client
            let ollama_config = {
                let client_lock = OLLAMA_CLIENT.read().await;
                if let Some(client) = client_lock.as_ref() {
                    client.get_config().clone()
                } else {
                    OllamaConfig::default()
                }
            };
            
            let generator = EmbeddingGenerator::new(ollama_config);
            *generator_lock = Some(generator.clone());
            generator
        }
    }
}