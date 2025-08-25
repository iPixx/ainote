//! # aiNote Backend
//! 
//! Local-first, AI-powered markdown note-taking application backend.
//! This module serves as the main entry point and orchestrates the various
//! command modules that handle different aspects of the application.
//!
//! ## Architecture
//!
//! The backend is organized into focused modules following single responsibility:
//! - `commands/`: All Tauri command handlers organized by domain
//! - `globals`: Global state management and singleton instances
//! - `app_setup`: Window initialization and event handling
//! - Core modules: errors, types, performance, validation, etc.
//! - AI modules: ollama_client, embedding_*, similarity_search, vector_db
//!
//! ## Command Organization
//!
//! Commands are organized into domain-specific modules in the `commands/` directory:
//! - File operations: CRUD, auto-save, folder management
//! - Vault operations: scanning, validation, watching
//! - State management: app/window/layout persistence
//! - AI integration: Ollama client, embeddings, search
//! - Performance: benchmarking, regression detection

use tauri::Manager;

// Core module declarations
pub mod commands;           // Tauri command modules organized by domain
pub mod globals;            // Global state management
pub mod app_setup;          // Application setup and window management

// Supporting modules
pub mod performance;
pub mod errors;
pub mod types;
pub mod metadata_cache;
pub mod file_locks;
pub mod validation;

// Core infrastructure modules  
pub mod ollama_client;          // Ollama HTTP client and connection management
pub mod embedding_generator;    // Embedding generation engine
pub mod embedding_cache;        // Embedding cache management
pub mod vector_db;             // Vector database storage and operations
pub mod similarity_search;     // Similarity search algorithms

// Performance and benchmarking modules
pub mod benchmarks;
pub mod performance_baseline;
pub mod regression_detection;

// Legacy standalone command modules (still used in invoke_handler)
// TODO: These should eventually be fully integrated into commands/ modules
pub mod file_operations;        // Legacy: superseded by commands::file_operations
pub mod vault_operations;       // Legacy: superseded by commands::vault_operations  
pub mod state_management;       // Legacy: superseded by commands::state_management
pub mod text_processing;        // Legacy: superseded by commands::text_processing
pub mod search_commands;        // Legacy: contains Tauri commands for basic search
pub mod similarity_search_commands; // Legacy: contains Tauri commands for advanced search

#[cfg(test)]
pub mod ollama_integration_tests;

#[cfg(test)]
pub mod embedding_tests;

// Re-exports for commonly used types
pub use errors::{FileSystemError, FileSystemResult};
pub use types::{AppState, WindowState, LayoutState, FileInfo};

// AI and search infrastructure re-exports
pub use ollama_client::{
    OllamaClient, OllamaConfig, ConnectionStatus, ConnectionState, HealthResponse, OllamaClientError,
    ModelInfo, ModelCompatibility, ModelVerificationResult, DownloadStatus, DownloadProgress, DownloadConfig
};
pub use embedding_generator::{EmbeddingGenerator, EmbeddingError, EmbeddingResult, EmbeddingConfig};
pub use embedding_cache::{EmbeddingCache, CacheError, CacheResult, CacheConfig, CacheMetrics};
pub use similarity_search::{
    SimilaritySearch, SimilarityError, SearchResult, SearchConfig, SearchMetrics, PerformanceConfig,
    EnhancedSearchResult, BenchmarkReport, ConcurrentSearchManager, GlobalSearchMetrics
};

/// Main application entry point and Tauri app configuration.
/// 
/// This function initializes the Tauri application with all necessary plugins,
/// window setup, and command handlers. The commands are organized into focused
/// modules in the `commands/` directory for better maintainability.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Setup window state and event handlers using app_setup module
            app_setup::setup_window_state(&window);
            app_setup::setup_window_events(&window);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // File Operations
            commands::file_operations::read_file,
            commands::file_operations::write_file,
            commands::file_operations::auto_save_file,
            commands::file_operations::create_file,
            commands::file_operations::delete_file,
            commands::file_operations::rename_file,
            commands::file_operations::preview_file,
            commands::file_operations::reveal_in_finder,
            commands::file_operations::get_file_info,
            commands::file_operations::create_folder,
            
            // Vault Operations
            commands::vault_operations::select_vault_folder,
            commands::vault_operations::select_vault,
            commands::vault_operations::validate_vault,
            commands::vault_operations::load_vault,
            commands::vault_operations::scan_vault_files,
            commands::vault_operations::scan_vault_files_chunked,
            commands::vault_operations::watch_vault,
            
            // State Management
            commands::state_management::load_app_state,
            commands::state_management::save_app_state,
            commands::state_management::save_window_state,
            commands::state_management::save_layout_state,
            commands::state_management::save_session_state,
            commands::state_management::save_vault_preferences,
            commands::state_management::get_vault_preferences,
            
            // Text Processing
            commands::text_processing::preprocess_text,
            commands::text_processing::chunk_text,
            commands::text_processing::validate_text,
            commands::text_processing::get_optimal_chunk_size,
            commands::text_processing::benchmark_chunk_sizes,
            commands::text_processing::create_chunking_config,
            commands::text_processing::chunk_text_with_config,
            
            // Ollama Integration
            commands::ollama_commands::check_ollama_status,
            commands::ollama_commands::get_ollama_health,
            commands::ollama_commands::configure_ollama_url,
            commands::ollama_commands::start_ollama_monitoring,
            commands::ollama_commands::get_available_models,
            commands::ollama_commands::verify_model,
            commands::ollama_commands::is_nomic_embed_available,
            commands::ollama_commands::get_model_info,
            commands::ollama_commands::download_model,
            commands::ollama_commands::get_download_progress,
            commands::ollama_commands::get_all_downloads,
            commands::ollama_commands::cancel_download,
            commands::ollama_commands::clear_completed_downloads,
            
            // Embedding Processing
            commands::embedding_commands::generate_embedding,
            commands::embedding_commands::generate_batch_embeddings,
            commands::embedding_commands::update_embedding_generator_config,
            commands::embedding_commands::get_embedding_generator_config,
            commands::embedding_commands::get_embedding_cache_metrics,
            commands::embedding_commands::clear_embedding_cache,
            commands::embedding_commands::get_embedding_cache_size,
            commands::embedding_commands::update_embedding_cache_config,
            commands::embedding_commands::get_embedding_cache_config,
            commands::embedding_commands::cleanup_expired_embeddings,
            commands::embedding_commands::check_embedding_cached,
            
            // Performance & Benchmarking
            commands::performance_commands::run_embedding_benchmarks,
            commands::performance_commands::generate_benchmark_report,
            commands::performance_commands::detect_performance_regressions,
            commands::performance_commands::establish_performance_baseline,
            commands::performance_commands::compare_performance_against_baseline,
            commands::performance_commands::get_baseline_report,
            commands::performance_commands::analyze_performance_regressions,
            
            // Search & Similarity - Basic (caching-focused)
            search_commands::search_similar_notes,
            search_commands::batch_search_similar_notes,
            search_commands::configure_similarity_search,
            search_commands::threshold_search_similar_notes,
            search_commands::get_search_cache_stats,
            search_commands::clear_search_cache,
            search_commands::cleanup_search_cache,
            search_commands::initialize_search_system,
            search_commands::get_search_system_status,
            
            // Search & Similarity - Advanced (performance-optimized)  
            similarity_search_commands::optimized_search_similar_notes,
            similarity_search_commands::optimized_batch_search_similar_notes,
            similarity_search_commands::approximate_search_similar_notes,
            similarity_search_commands::get_search_metrics,
            similarity_search_commands::is_search_high_load,
            similarity_search_commands::get_active_search_count,
            similarity_search_commands::benchmark_search_performance,
            similarity_search_commands::configure_search_performance,
            similarity_search_commands::test_search_functionality
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Basic smoke test to ensure the application can be initialized
    /// 
    /// Most tests have been moved to separate integration test files:
    /// - tests/file_operations_tests.rs: File CRUD operations and vault scanning
    /// - tests/ollama_integration_tests.rs: Ollama client and model management
    /// - Existing integration tests: e2e_ollama_tests.rs, vector_db_integration_tests.rs, etc.
    #[test]
    fn test_module_imports() {
        // Test that all main modules are accessible
        use crate::types::AppState;
        
        // Test that core types can be instantiated
        let _app_state = AppState::default();
        
        // Test passed - all imports work correctly
        assert!(true);
    }
    
    #[test]
    fn test_re_exports() {
        // Test that re-exported types are accessible
        let _fs_error: FileSystemError = FileSystemError::FileNotFound { path: "test".to_string() };
        let _app_state: AppState = AppState::default();
        
        // Verify default state values are reasonable
        assert_eq!(_app_state.window.width, 1200.0);
        assert_eq!(_app_state.window.height, 800.0);
    }
}
