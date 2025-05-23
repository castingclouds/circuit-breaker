// Circuit Breaker Engine
// This contains the execution engines and API interfaces

//! # Circuit Breaker Engine Module
//! 
//! This module contains the execution engines and API interfaces that power
//! the Circuit Breaker workflow system. The engine is the layer between the
//! domain models and the external world.
//! 
//! ## Architecture Overview
//! 
//! The engine follows a **generic engine** pattern inspired by Dagger:
//! - **Domain Models**: Pure business logic (in `models/`)
//! - **Engine Layer**: Execution and API interfaces (this module)
//! - **Server Layer**: HTTP servers and GraphQL endpoints (in `server/`)
//! 
//! ## Engine Components
//! 
//! ### GraphQL Engine (`graphql` module)
//! - Provides GraphQL schema and resolvers
//! - Translates between GraphQL types and domain models
//! - Enables polyglot architecture (any language can use via GraphQL)
//! - Includes input/output types for mutations and queries
//! 
//! ### Storage Engine (`storage` module)  
//! - Abstracts storage operations
//! - Provides in-memory implementation for development/testing
//! - Can be extended for persistent storage (database, NATS, etc.)
//! - Manages workflow definitions and token state
//! 
//! ### Rules Engine (`rules` module)
//! - Evaluates transition rules against token state
//! - Provides sophisticated condition evaluation for workflow gating
//! - Supports both structured rules and legacy string conditions
//! - Enables complex logical expressions (AND, OR, NOT)
//! 
//! ## Rust Learning Notes:
//! 
//! ### Module Organization Pattern
//! This is a common Rust pattern for organizing large modules:
//! 1. Create a directory with the module name (`engine/`)
//! 2. Add a `mod.rs` file as the module root
//! 3. Declare submodules in `mod.rs`
//! 4. Re-export important types for clean API
//! 
//! ### Re-exports for API Design
//! The `pub use` statements create a clean API by:
//! - Flattening the module hierarchy for users
//! - Hiding internal organization details
//! - Making commonly-used types easily accessible

/// GraphQL engine for API interface
/// 
/// Contains:
/// - GraphQL schema definitions
/// - Resolver implementations  
/// - Input/output type mappings
/// - Schema building functions
pub mod graphql;

/// Storage abstraction layer
/// 
/// Contains:
/// - Storage trait definition
/// - In-memory storage implementation
/// - Storage operations for workflows and tokens
pub mod storage;

/// Rules engine for transition evaluation
/// 
/// Contains:
/// - RulesEngine for evaluating token transitions
/// - Global rule registry and management
/// - Detailed evaluation results and feedback
/// - Legacy condition support
pub mod rules;

// Re-export main engine types for clean API access
// Users can import directly from engine instead of navigating submodules

/// Re-export GraphQL types for external API
/// 
/// These types enable GraphQL integration:
/// - GQL suffix types: GraphQL representations of domain models
/// - Input types: For GraphQL mutations (creating/updating data)
/// - Schema types: For building the complete GraphQL schema
pub use graphql::{
    // GraphQL representations of domain models
    WorkflowGQL,              // Workflow definition for GraphQL responses
    TokenGQL,                 // Token state for GraphQL responses  
    TransitionGQL,            // Transition definition for GraphQL responses
    HistoryEventGQL,          // History event for GraphQL responses
    
    // Input types for GraphQL mutations
    WorkflowDefinitionInput,  // Input for creating workflows
    TokenCreateInput,         // Input for creating tokens
    TransitionFireInput,      // Input for firing transitions
    
    // Schema building and management
    CircuitBreakerSchema,     // Complete GraphQL schema type
    create_schema,            // Function to create schema with default storage
    create_schema_with_storage // Function to create schema with custom storage
};

/// Re-export storage types for persistence layer
/// 
/// These types enable storage abstraction:
/// - WorkflowStorage: Trait defining storage operations
/// - InMemoryStorage: Default in-memory implementation
pub use storage::{WorkflowStorage, InMemoryStorage};

/// Re-export rules engine types for transition evaluation
/// 
/// These types enable sophisticated rule-based workflow control:
/// - RulesEngine: Central engine for evaluating token transitions
/// - WorkflowEvaluationResult: Detailed evaluation results for all transitions
pub use rules::{RulesEngine, WorkflowEvaluationResult}; 