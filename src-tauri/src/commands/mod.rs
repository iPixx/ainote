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

// Command modules will be created in Phase 2 of the refactoring
// This module structure is prepared for future command extraction

// File Operations Module (Phase 2)
// Will handle: CRUD operations, file preview, auto-save
// pub mod file_operations;

// Vault Operations Module (Phase 2)
// Will handle: vault scanning, validation, folder selection, and file watching
// pub mod vault_operations;

// State Management Module (Phase 2)
// Will handle: application state, window state, layout preferences, and session data
// pub mod state_management;

// Ollama Commands Module (Phase 2)
// Will handle: Ollama client management, health checks, model operations, and monitoring
// pub mod ollama_commands;

// Embedding Commands Module (Phase 2)
// Will handle: embedding generation, batch processing, caching, and configuration
// pub mod embedding_commands;

// Search Commands Module (Phase 2)
// Will handle: similarity search operations and vector database queries
// pub mod search_commands;

// Performance Commands Module (Phase 2)
// Will handle: benchmarking, baseline management, and regression detection
// pub mod performance_commands;

// Re-export all command functions for easy access in lib.rs
// Note: Actual re-exports will be added as modules are implemented in Phase 2