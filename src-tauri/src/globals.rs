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
use crate::embedding_queue::EmbeddingQueue;
use crate::vector_db::VectorDatabase;
use crate::suggestion_cache::SuggestionCache;

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

/// Global embedding queue instance for advanced request management
///
/// Provides sophisticated queuing system for embedding requests with
/// priority-based processing, request deduplication, cancellation support,
/// and comprehensive performance monitoring for optimal resource usage.
pub static EMBEDDING_QUEUE: Lazy<Arc<RwLock<Option<EmbeddingQueue>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Global vector database instance for embedding storage and retrieval
///
/// Manages the vector database for storing and querying embeddings.
/// Includes support for incremental updates, similarity search, and
/// comprehensive database operations with transaction safety.
pub static VECTOR_DATABASE: Lazy<Arc<RwLock<Option<VectorDatabase>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Global suggestion cache instance for AI suggestion optimization
///
/// Provides intelligent caching for AI-powered note suggestions with
/// context-aware filtering, cache invalidation, and performance monitoring.
/// Includes support for recent suggestion tracking and cache warming.
pub static SUGGESTION_CACHE: Lazy<Arc<RwLock<Option<SuggestionCache>>>> = 
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

/// Helper function to get or initialize the embedding queue
///
/// This function uses the double-checked locking pattern to ensure
/// thread-safe lazy initialization. If the queue doesn't exist,
/// it creates a new one with default configuration and starts
/// the background processing tasks.
///
/// # Returns
///
/// Returns a cloned `EmbeddingQueue` instance that can be used
/// for queued embedding generation with advanced features.
///
/// # Example
///
/// ```rust
/// let queue = get_embedding_queue().await;
/// let request_id = queue.submit_request("text", "model", priority).await?;
/// let embedding = queue.wait_for_result(request_id).await?;
/// ```
pub async fn get_embedding_queue() -> EmbeddingQueue {
    let queue_lock = EMBEDDING_QUEUE.read().await;
    if let Some(queue) = queue_lock.as_ref() {
        queue.clone()
    } else {
        drop(queue_lock);
        // Initialize queue if not exists
        let mut queue_lock = EMBEDDING_QUEUE.write().await;
        
        // Double-check pattern to avoid race conditions
        if let Some(queue) = queue_lock.as_ref() {
            queue.clone()
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
            
            let queue = EmbeddingQueue::with_default_config(ollama_config);
            
            // Start the queue's background processing tasks
            let (_processor_handle, _cleanup_handle) = queue.start().await;
            // Note: We're not storing the handles. In a production system,
            // you might want to store these globally to manage shutdown
            
            *queue_lock = Some(queue.clone());
            queue
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

/// Helper function to get or initialize the suggestion cache
///
/// This function uses the double-checked locking pattern to ensure
/// thread-safe lazy initialization. If the cache doesn't exist,
/// it creates a new one with default configuration.
///
/// # Returns
/// 
/// Returns a cloned `SuggestionCache` instance that can be used
/// for suggestion caching operations.
///
/// # Example
///
/// ```rust
/// let cache = get_suggestion_cache().await;
/// cache.cache_suggestions("content", "model", &context, suggestions).await;
/// ```
pub async fn get_suggestion_cache() -> SuggestionCache {
    let cache_lock = SUGGESTION_CACHE.read().await;
    if let Some(cache) = cache_lock.as_ref() {
        cache.clone()
    } else {
        drop(cache_lock);
        // Initialize cache if not exists
        let mut cache_lock = SUGGESTION_CACHE.write().await;
        
        // Double-check pattern to avoid race conditions
        if let Some(cache) = cache_lock.as_ref() {
            cache.clone()
        } else {
            let cache = SuggestionCache::new();
            *cache_lock = Some(cache.clone());
            cache
        }
    }
}