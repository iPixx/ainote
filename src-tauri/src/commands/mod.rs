//! # Tauri Command Modules
//!
//! This module organizes all Tauri commands into focused, single-responsibility modules.
//! Each module handles a specific domain of functionality to improve maintainability,
//! testing, and code organization.
//!
//! ## Architecture Overview
//!
//! The command modules follow a consistent pattern:
//! - **Single Responsibility**: Each module handles one domain (files, vault, state, etc.)
//! - **Error Handling**: All commands return `Result<T, String>` for consistent error propagation
//! - **Documentation**: Comprehensive rustdoc comments for all public functions
//! - **Testing**: Each module includes unit tests for command logic
//!
//! ## Module Organization
//!
//! ### Core File Operations
//! - `file_operations`: File CRUD operations (create, read, write, delete, rename)
//! - `vault_operations`: Vault scanning, validation, and folder management
//!
//! ### Application State
//! - `state_management`: App state, window state, and layout persistence
//!
//! ### AI Integration
//! - `ollama_commands`: Ollama client management and model operations
//! - `embedding_commands`: Embedding generation, caching, and configuration
//! - `search_commands`: Similarity search and vector operations
//!
//! ### Performance & Monitoring
//! - `performance_commands`: Benchmarking, baseline management, regression detection
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::commands::{
//!     file_operations::read_file,
//!     vault_operations::load_vault,
//!     state_management::save_app_state,
//! };
//!
//! // Commands are registered in lib.rs invoke_handler!() macro
//! // and can be called from the frontend via tauri.invoke()
//! ```
//!
//! ## Integration with Main Application
//!
//! All commands are re-exported from this module and registered in `lib.rs`:
//!
//! ```rust
//! use crate::commands::*;
//!
//! tauri::Builder::default()
//!     .invoke_handler(tauri::generate_handler![
//!         // All command functions are registered here
//!     ])
//! ```

// Command modules - Phase 2 Complete
// All command modules have been extracted from the monolithic lib.rs

// File Operations Module
// Handles: CRUD operations, file preview, auto-save, folder creation
pub mod file_operations;

// Vault Operations Module
// Handles: vault scanning, validation, folder selection, and file watching
pub mod vault_operations;

// State Management Module
// Handles: application state, window state, layout preferences, and session data
pub mod state_management;

// Text Processing Module
// Handles: text preprocessing, chunking, validation, and optimization
pub mod text_processing;

// Ollama Commands Module
// Handles: Ollama client management, health checks, model operations, and monitoring
pub mod ollama_commands;

// Embedding Commands Module
// Handles: embedding generation, batch processing, caching, and configuration
pub mod embedding_commands;

// Performance Commands Module
// Handles: benchmarking, baseline management, and regression detection
pub mod performance_commands;

// Search Commands Module
// Handles: similarity search operations and vector database queries
pub mod search_commands;

// Incremental Commands Module
// Handles: incremental update system, file change monitoring, and automatic embedding updates
pub mod incremental_commands;

// Maintenance Commands Module
// Handles: maintenance operations, orphaned embedding cleanup, index compaction, and storage optimization
pub mod maintenance_commands;

// Rebuilding Commands Module
// Handles: index rebuilding operations, health checks, corruption detection, and recovery systems
pub mod rebuilding_commands;

// Monitoring Commands Module
// Handles: performance monitoring, metrics collection, real-time monitoring, and alerting
pub mod monitoring_commands;

// Indexing Commands Module  
// Handles: automated vault indexing pipeline, progress tracking, cancellation, and file monitoring integration
pub mod indexing_commands;

// Optimization Commands Module
// Handles: automatic optimization scheduling, trigger configuration, manual optimization execution, and status monitoring
pub mod optimization_commands;

// Re-export all command functions for easy access in lib.rs
pub use file_operations::*;
pub use vault_operations::*;
pub use state_management::*;
pub use text_processing::*;
pub use ollama_commands::*;
pub use embedding_commands::*;
pub use performance_commands::*;
pub use search_commands::*;
pub use incremental_commands::*;
pub use maintenance_commands::*;
pub use rebuilding_commands::*;
pub use monitoring_commands::*;
pub use indexing_commands::*;
pub use optimization_commands::*;