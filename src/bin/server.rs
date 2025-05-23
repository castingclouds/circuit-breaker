// Circuit Breaker - Main GraphQL Server
// The production server for State Managed Workflows
// Run with: cargo run --bin server

//! # Circuit Breaker Main Server Binary
//! 
//! This is the main executable that starts the Circuit Breaker HTTP server.
//! It demonstrates how all the pieces come together to create a running
//! workflow engine that clients can connect to via GraphQL.
//! 
//! ## What This Server Provides
//! 
//! - **GraphQL API**: Complete workflow management via GraphQL
//! - **GraphiQL Interface**: Interactive GraphQL explorer at http://localhost:4000
//! - **Default Workflows**: Pre-loaded example workflows for testing
//! - **In-Memory Storage**: Simple storage for development (no database needed)
//! - **CORS Support**: Allows browser-based clients to connect
//! 
//! ## Architecture Demonstration
//! 
//! This binary shows the complete Circuit Breaker architecture:
//! ```text
//! main() function
//!   ↓ builds
//! GraphQLServerBuilder 
//!   ↓ creates  
//! HTTP Server (Axum)
//!   ↓ serves
//! GraphQL Schema
//!   ↓ resolves via
//! Storage Layer (InMemoryStorage)
//!   ↓ operates on
//! Domain Models (Workflows, Tokens, Places, Transitions)
//! ```
//! 
//! ## Usage Examples
//! 
//! Once running, you can:
//! - Visit http://localhost:4000 for GraphiQL interface
//! - Send GraphQL queries from any language
//! - Create workflows, tokens, and fire transitions
//! - Explore the default workflows provided
//! 
//! ## Rust Learning Notes:
//! 
//! This file demonstrates several important Rust concepts:
//! - Binary crate vs library crate organization
//! - Async main functions with tokio
//! - Builder pattern for configuration
//! - Error handling with ? operator and Box<dyn Error>
//! - External crate integration (tracing, tokio)

use circuit_breaker::GraphQLServerBuilder; // Import from our library crate
use tracing_subscriber;                     // Logging framework

/// Main entry point for the Circuit Breaker server
/// 
/// ## Rust Learning Notes:
/// 
/// ### Async Main Function
/// `#[tokio::main]` is a macro that transforms the async main function into
/// a synchronous main that sets up the tokio async runtime. This allows us
/// to use `.await` in the main function.
/// 
/// Without this macro, we would need to write:
/// ```rust
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     tokio::runtime::Runtime::new()?.block_on(async {
///         // async code here
///     })
/// }
/// ```
/// 
/// ### Error Handling with Box<dyn Error>
/// `Box<dyn std::error::Error>` is a common pattern for main functions.
/// It can hold any error type that implements the Error trait, making it
/// flexible for different kinds of errors that might occur.
/// 
/// ### The ? Operator
/// The `?` operator is used for error propagation:
/// - If the operation succeeds, extract the value and continue
/// - If the operation fails, return the error immediately
/// - Much cleaner than explicit match statements for error handling
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging for the application
    // This sets up tracing/logging that will show debug info, errors, etc.
    // In production, you might configure different log levels or outputs
    tracing_subscriber::fmt::init();

    // Print startup banner - helps identify server startup in logs
    println!("🚀 Starting Circuit Breaker Server...");
    println!("=====================================");

    // Build and start the production server
    // 
    // ## Rust Learning Notes:
    // 
    // ### Builder Pattern
    // The builder pattern is common in Rust for complex object construction.
    // It allows:
    // - Step-by-step configuration
    // - Optional parameters with sensible defaults
    // - Type-safe configuration (compile-time checks)
    // - Fluent API (method chaining)
    // 
    // ### Method Chaining
    // Each method returns `self` so you can chain calls:
    // - `.with_port(4000)` sets the port and returns the builder
    // - `.build_and_run()` consumes the builder and starts the server
    // 
    // ### Async Operations
    // `.build_and_run().await?` does several things:
    // - `.build_and_run()` returns a Future
    // - `.await` waits for the Future to complete
    // - `?` propagates any errors that occur
    GraphQLServerBuilder::new()     // Create a new server builder
        .with_port(4000)            // Configure to listen on port 4000
        .build_and_run()            // Build the server and start running it
        .await?;                    // Wait for the server to start (or fail)

    // If we reach here, the server started successfully
    // In practice, the server runs indefinitely, so this line rarely executes
    Ok(())
} 