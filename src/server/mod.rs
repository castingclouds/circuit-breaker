// Circuit Breaker Server Implementations
// This contains various server types that can expose the workflow engine

//! # Circuit Breaker Server Module
//! 
//! This module contains server implementations that expose the Circuit Breaker
//! workflow engine to external clients. The server layer sits on top of the
//! engine layer and provides network-accessible APIs.
//! 
//! ## Server Architecture
//! 
//! The server follows a **layered architecture**:
//! ```text
//! Client (Any Language) 
//!        ↓ HTTP/GraphQL
//! Server Layer (this module) ← HTTP servers, GraphQL endpoints
//!        ↓ Function calls  
//! Engine Layer ← GraphQL schema, storage abstraction
//!        ↓ Function calls
//! Domain Layer ← Pure business logic, Petri nets
//! ```
//! 
//! ## Polyglot Architecture
//! 
//! The server enables **polyglot workflows**:
//! - **Any language** can define workflows via GraphQL
//! - **TypeScript, Python, Go, etc.** can all use the same Rust backend
//! - **GraphQL** provides language-agnostic API
//! - **No language lock-in** - switch clients without changing backend
//! 
//! ## Server Types
//! 
//! ### GraphQL Server (`graphql` module)
//! - HTTP server with GraphQL endpoint  
//! - Built on Axum web framework
//! - Provides GraphiQL interface for development
//! - Handles CORS for browser access
//! - Integrates with any storage backend
//! 
//! ### Future Server Types
//! - **gRPC Server**: For high-performance scenarios
//! - **WebSocket Server**: For real-time workflow updates
//! - **REST API Server**: For simple HTTP integration
//! 
//! ## Rust Learning Notes:
//! 
//! This module demonstrates:
//! - Web server architecture patterns
//! - Async HTTP handling
//! - Integration between web frameworks and business logic
//! - Configuration and builder patterns

/// GraphQL HTTP server implementation
/// 
/// Contains:
/// - Axum-based HTTP server
/// - GraphQL endpoint configuration
/// - CORS and middleware setup
/// - Builder pattern for server configuration
pub mod graphql;

// Re-export main server types for easy access
// This allows users to import server types directly from the server module

/// Re-export GraphQL server types
/// 
/// These types enable HTTP server setup:
/// - GraphQLServer: The main server instance
/// - GraphQLServerConfig: Configuration options
/// - GraphQLServerBuilder: Builder pattern for easy setup
pub use graphql::{GraphQLServer, GraphQLServerConfig, GraphQLServerBuilder}; 