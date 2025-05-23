// Core domain models for Circuit Breaker
// These are the generic, language-agnostic data structures

//! # Domain Models Module
//! 
//! This module contains the core domain models for Circuit Breaker. These are
//! **generic, language-agnostic** data structures that form the foundation of
//! the workflow engine.
//! 
//! ## Rust Learning Notes:
//! 
//! ### Module Organization
//! This `mod.rs` file serves as the **module root** for the `models` directory.
//! When you have a directory with a `mod.rs` file, Rust treats the directory
//! as a module, and `mod.rs` acts as the entry point.
//! 
//! ### Module Declarations
//! Each `pub mod` declaration tells Rust to:
//! 1. Look for a `.rs` file with that name in the same directory
//! 2. Include that file's code as a submodule
//! 3. Make it publicly accessible (because of `pub`)
//! 
//! ### Re-exports for Clean APIs
//! The `pub use` statements at the bottom create a clean, flat API.
//! Users can import `use circuit_breaker::models::Token` instead of
//! `use circuit_breaker::models::token::Token`.

// Declares the `place` submodule from `place.rs`
// Contains PlaceId and TransitionId - the basic building blocks of Petri nets
pub mod place;

// Declares the `transition` submodule from `transition.rs`  
// Contains TransitionDefinition - defines how places connect
pub mod transition;

// Declares the `workflow` submodule from `workflow.rs`
// Contains WorkflowDefinition - the complete workflow structure
pub mod workflow;

// Declares the `token` submodule from `token.rs`
// Contains Token - represents workflow execution instances
pub mod token;

// Re-export main types for convenience
// This creates shortcuts so users don't need to know the internal structure

/// Re-export the fundamental Petri net building blocks
/// PlaceId represents states, TransitionId represents actions
pub use place::{PlaceId, TransitionId};

/// Re-export transition definitions
/// TransitionDefinition defines how tokens can move between places
pub use transition::TransitionDefinition;

/// Re-export workflow definitions  
/// WorkflowDefinition contains the complete workflow structure
pub use workflow::WorkflowDefinition;

/// Re-export token types
/// - Token: The main workflow execution instance
/// - HistoryEvent: Records each state transition
/// - TokenMetadata: Key-value metadata storage
pub use token::{Token, HistoryEvent, TokenMetadata}; 