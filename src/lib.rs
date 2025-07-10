// Circuit Breaker - Rust Edition
// A distributed, polyglot workflow engine powered by Petri Nets and GraphQL
// Inspired by Dagger's generic engine architecture

//! # Circuit Breaker Library
//!
//! This is the main library crate for Circuit Breaker, a workflow engine that uses
//! Petri Net theory to manage state transitions. This file serves as the **library root**
//! and defines the public API that external crates can use.
//!
//! ## Core Components
//!
//! ### Domain Models
//! - [`Token`]: The main workflow execution token that moves through places
//! - [`WorkflowDefinition`]: Defines the structure and transitions of a workflow
//! - [`TransitionDefinition`]: Defines how tokens move between places
//! - [`PlaceId`] / [`TransitionId`]: Unique identifiers for workflow elements
//!
//! ### Rules Engine
//!
//! #### [`RulesEngine`] - Central Transition Evaluation Engine
//!
//! The **authoritative** engine for evaluating whether tokens can fire transitions.
//! Provides **complete evaluation** including all condition types:
//!
//! - **Place Compatibility**: Is token in the correct place?
//! - **Structured Rules**: Do all `rules` in the transition pass?
//! - **Legacy Conditions**: Do all string-based `conditions` pass?
//!
//! **Key Features:**
//! - Global rule registry for reusable business logic
//! - Complex logical expressions (AND, OR, NOT) with arbitrary nesting
//! - Detailed evaluation feedback for debugging and user interfaces
//! - Backwards compatibility with legacy string-based conditions
//!
//! **Usage Example:**
//! ```rust
//! use circuit_breaker::{RulesEngine, Token, WorkflowDefinition};
//!
//! // Create engine with common predefined rules
//! let mut engine = RulesEngine::with_common_rules();
//!
//! // Authoritative evaluation (includes ALL condition types)
//! // Note: You would need actual token, transition, and workflow instances
//! // let can_fire = engine.can_transition(&token, &transition);
//! // let available = engine.available_transitions(&token, &workflow);
//! // let detailed = engine.evaluate_all_transitions(&token, &workflow);
//! ```
//!
//! **⚠️ Important:** Always use `RulesEngine` methods for production evaluation.
//! Direct `TransitionDefinition` methods only evaluate structured rules,
//! not legacy string-based conditions.
//!
//! #### [`WorkflowEvaluationResult`] - Comprehensive Evaluation Results
//!
//! Detailed results for all transitions in a workflow, essential for debugging
//! and building user interfaces that show transition requirements.
//!
//! **Contains:**
//! - Workflow and token identification
//! - Current token place
//! - Per-transition evaluation results with explanations
//! - Available vs blocked transition counts
//!
//! **Use Cases:**
//! - **Debugging**: Understand why specific transitions fail
//! - **User Feedback**: Show users what conditions are missing
//! - **UI Development**: Build interfaces showing transition requirements
//! - **Workflow Analysis**: Optimize workflow logic and identify bottlenecks
//!
//! ### GraphQL Engine
//! Provides a language-agnostic API for polyglot workflow management.
//!
//! ### Storage Layer
//! Abstracts persistence with pluggable storage backends.
//!
//! ## Rust Learning Notes:
//!
//! ### Module System
//! Rust organizes code into modules. Each `mod` declaration tells Rust to include
//! code from either a `.rs` file or a directory with a `mod.rs` file.
//!
//! ### Public vs Private
//! - `pub mod` makes modules accessible to external crates
//! - `mod` (without pub) makes modules only accessible within this crate
//!
//! ### Re-exports
//! `pub use` statements create shortcuts so users don't need to know the internal
//! module structure. Instead of `use circuit_breaker::models::token::Token`,
//! users can write `use circuit_breaker::Token`.

// Core domain models (language-agnostic)
// The `pub` keyword makes this module accessible to external crates
pub mod models;

// Engine implementations (GraphQL, etc.)
// This contains the execution engines and APIs
pub mod engine;

// Server implementations
// This contains HTTP server and GraphQL server setup
pub mod server;

// LLM Router and AI infrastructure
// This contains the OpenRouter alternative implementation with BYOK model
pub mod llm;

// OpenAI-compatible API
// This contains REST API endpoints that are compatible with OpenAI's API specification
pub mod api;

// TODO: Implement these modules as we build them
// These are commented out because the modules don't exist yet
// pub mod rules;
// pub mod campaign;
// pub mod agents;

// Re-export core domain types for easy access
// This creates a "flat" API - users can import directly from the crate root
// instead of navigating the module hierarchy
pub use models::{
    ActivityDefinition, // Defines how states connect
    ActivityId,         // Represents state activities
    ActivityRecord,     // NATS-specific activity tracking
    HistoryEvent,       // Records state transition history
    Resource,           // The main workflow execution resource
    ResourceMetadata,   // Key-value metadata storage
    StateId,            // Represents workflow states
    WorkflowDefinition, // Defines the workflow structure
};

// Re-export engine types for convenience
// These are the GraphQL and storage implementations
pub use engine::{
    graphql::{
        create_schema,
        create_schema_with_nats,
        create_schema_with_nats_and_agents,
        create_schema_with_storage,
        ActivityExecuteInput,
        ActivityGQL,
        ActivityRecordGQL,
        // Schema creation functions
        CircuitBreakerSchema,
        // NATS-specific input types
        CreateWorkflowInstanceInput,
        ExecuteActivityWithNATSInput,
        HistoryEventGQL,
        // NATS-specific GraphQL types
        NATSResourceGQL,
        ResourceCreateInput,
        ResourceGQL,
        ResourcesInStateInput,
        // Input types for GraphQL mutations
        WorkflowDefinitionInput,
        // GraphQL types for the API
        WorkflowGQL,
    },
    nats_storage::{NATSStorage, NATSStorageConfig, NATSStorageWrapper, WorkflowStreamManager}, // NATS storage implementation
    rules::{RulesEngine, WorkflowEvaluationResult}, // Rules engine for transition evaluation
    storage::{InMemoryStorage, WorkflowStorage},    // Storage abstraction and implementation
};

// Re-export server types for convenience
pub use server::graphql::GraphQLServerBuilder;

// Re-export API types for convenience
pub use api::{
    create_default_server, create_server_with_config, create_server_with_port,
    types::{
        ChatCompletionRequest, ChatCompletionResponse, ChatCompletionStreamResponse, ChatMessage,
        ChatRole, ErrorResponse, Model, ModelsResponse,
    },
    OpenAIApiConfig, OpenAIApiServer, OpenAIApiServerBuilder,
};

// Core error types
// Using the `thiserror` crate to make error handling easier
use thiserror::Error;

/// Custom error types for Circuit Breaker operations
///
/// ## Rust Learning Notes:
///
/// ### Error Handling in Rust
/// Rust doesn't have exceptions. Instead, it uses `Result<T, E>` types where:
/// - `Ok(value)` represents success
/// - `Err(error)` represents failure
///
/// ### The `thiserror` Crate
/// This crate provides macros to make error types easier to write:
/// - `#[derive(Error)]` implements the `std::error::Error` trait
/// - `#[error("...")]` provides human-readable error messages
/// - `{field}` in error messages allows string interpolation
/// - `#[from]` enables automatic conversion from other error types
#[derive(Error, Debug)]
pub enum CircuitBreakerError {
    /// Error when trying to perform an invalid state transition
    /// The `#[error(...)]` attribute defines the error message format
    /// `{from}`, `{to}`, `{transition}` will be replaced with actual values
    #[error("Invalid state transition from {from} to {to} via {transition}")]
    InvalidTransition {
        from: String,       // Source state
        to: String,         // Target state
        transition: String, // Transition that was attempted
    },

    /// Error when business rule validation fails
    #[error("Rule validation failed: {rule}")]
    RuleValidationFailed { rule: String },

    /// Error when a token cannot be found
    #[error("Token not found: {id}")]
    TokenNotFound { id: String },

    /// Error when a workflow definition cannot be found
    #[error("Workflow not found: {id}")]
    WorkflowNotFound { id: String },

    /// Error when a resource cannot be found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Error when invalid input is provided
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Storage-related errors
    /// Using anyhow::Error for flexible error handling with NATS and other storage backends
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),

    /// JSON serialization/deserialization errors
    /// Also uses `#[from]` for automatic conversion
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// GraphQL-specific errors
    #[error("GraphQL error: {0}")]
    GraphQL(String),

    /// Forbidden access error
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Rate limiting error
    #[error("Rate limited: {0}")]
    RateLimited(String),

    /// Too many requests error
    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    /// Quota exceeded error
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for CircuitBreakerError {
    fn from(err: std::io::Error) -> Self {
        CircuitBreakerError::Internal(err.to_string())
    }
}

/// Type alias for Results that use our custom error type
///
/// ## Rust Learning Notes:
///
/// ### Type Aliases
/// This creates a shorthand for a commonly-used type. Instead of writing
/// `std::result::Result<Token, CircuitBreakerError>` everywhere, we can
/// just write `Result<Token>`.
///
/// ### Generic Type Parameters
/// The `<T>` makes this alias work with any type - `Result<Token>`,
/// `Result<Workflow>`, etc.
pub type Result<T> = std::result::Result<T, CircuitBreakerError>;
