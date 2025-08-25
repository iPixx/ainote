//! # Search Commands
//!
//! This module contains all Tauri commands related to similarity search and
//! vector-based note discovery. It provides both basic caching-focused search
//! operations and advanced performance-optimized search functionality.
//!
//! ## Command Overview
//!
//! ### Basic Search Operations (Caching-Focused)
//! - `search_similar_notes`: Find similar notes using vector similarity
//! - `batch_search_similar_notes`: Search multiple queries in batch
//! - `configure_similarity_search`: Configure search parameters
//! - `threshold_search_similar_notes`: Search with custom similarity thresholds
//!
//! ### Cache Management
//! - `get_search_cache_stats`: Get search cache performance metrics
//! - `clear_search_cache`: Clear all cached search results
//! - `cleanup_search_cache`: Remove expired cache entries
//!
//! ### System Management
//! - `initialize_search_system`: Initialize the search engine
//! - `get_search_system_status`: Get current system status
//!
//! ### Advanced Search Operations (Performance-Optimized)
//! - `optimized_search_similar_notes`: High-performance similarity search
//! - `optimized_batch_search_similar_notes`: Optimized batch searching
//! - `approximate_search_similar_notes`: Fast approximate search
//!
//! ### Performance Monitoring
//! - `get_search_metrics`: Get comprehensive performance metrics
//! - `is_search_high_load`: Check if system is under high load
//! - `get_active_search_count`: Get number of active searches
//! - `benchmark_search_performance`: Run search performance benchmarks
//! - `configure_search_performance`: Configure performance parameters
//! - `test_search_functionality`: Test search system functionality
//!
//! ## Search Architecture
//!
//! The search system is built on a multi-layered architecture:
//!
//! ### Vector Database Layer
//! - **Storage**: Efficient vector storage and retrieval
//! - **Indexing**: Optimized vector indexing for fast search
//! - **Persistence**: Durable storage of vectors and metadata
//! - **Compression**: Space-efficient vector compression
//!
//! ### Embedding Integration
//! - **Text Processing**: Integration with text preprocessing pipeline
//! - **Embedding Generation**: Seamless embedding creation for queries
//! - **Model Management**: Support for multiple embedding models
//! - **Caching**: Aggressive caching of embeddings and results
//!
//! ### Similarity Algorithms
//! - **Cosine Similarity**: Standard cosine similarity for text vectors
//! - **Euclidean Distance**: Alternative distance metrics
//! - **Approximate Search**: Fast approximate nearest neighbor search
//! - **Hybrid Ranking**: Combination of similarity and relevance scoring
//!
//! ## Performance Optimization
//!
//! ### Caching Strategy
//! - **Query Caching**: Cache search results for repeated queries
//! - **Embedding Caching**: Reuse embeddings across searches
//! - **Result Ranking**: Cache ranked result sets
//! - **Metadata Caching**: Cache file and note metadata
//!
//! ### Search Optimization
//! - **Index Optimization**: Hierarchical and approximate indexes
//! - **Batch Processing**: Efficient batch query processing
//! - **Parallel Search**: Multi-threaded search execution
//! - **Load Balancing**: Dynamic load distribution
//!
//! ### Memory Management
//! - **Efficient Storage**: Memory-optimized vector storage
//! - **Garbage Collection**: Automatic cleanup of unused data
//! - **Memory Pooling**: Reuse of allocated memory structures
//! - **Streaming Results**: Memory-efficient result streaming
//!
//! ## Search Configuration
//!
//! ### Basic Parameters
//! - **Similarity Threshold**: Minimum similarity for results
//! - **Result Limit**: Maximum number of results to return
//! - **Search Scope**: Files, folders, or content-based filtering
//! - **Ranking Method**: How to rank and sort results
//!
//! ### Advanced Configuration
//! - **Index Parameters**: Vector index configuration
//! - **Performance Tuning**: CPU, memory, and I/O optimization
//! - **Approximation Settings**: Speed vs accuracy trade-offs
//! - **Concurrent Limits**: Maximum concurrent search operations
//!
//! ## Error Handling
//!
//! Comprehensive error handling for:
//! - Vector database corruption or unavailability
//! - Embedding generation failures
//! - Search index corruption
//! - Memory and resource constraints
//! - Network connectivity issues with embedding services

// Re-export existing search command modules
// The actual implementations are in the separate search_commands and similarity_search_commands modules

// Basic search operations (caching-focused)
pub use crate::search_commands::{
    search_similar_notes,
    batch_search_similar_notes, 
    configure_similarity_search,
    threshold_search_similar_notes,
    get_search_cache_stats,
    clear_search_cache,
    cleanup_search_cache,
    initialize_search_system,
    get_search_system_status
};

// Advanced performance-optimized search operations
pub use crate::similarity_search_commands::{
    optimized_search_similar_notes,
    optimized_batch_search_similar_notes,
    approximate_search_similar_notes,
    get_search_metrics,
    is_search_high_load,
    get_active_search_count,
    benchmark_search_performance,
    configure_search_performance,
    test_search_functionality
};

// Re-export types and configurations
pub use crate::search_commands::{
    SimilaritySearchConfig,
    SimilaritySearchResult, 
    BatchSearchRequest,
    BatchSearchResult,
    SearchCommandError,
    initialize_search_engine,
    get_search_engine_stats
};

pub use crate::similarity_search::{
    ConcurrentSearchManager,
    GlobalSearchMetrics
};