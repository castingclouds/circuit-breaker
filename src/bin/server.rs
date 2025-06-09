// Circuit Breaker - Unified Server
// Provides both GraphQL and OpenAI-compatible REST API endpoints
// Run with: cargo run --bin server

//! # Circuit Breaker Unified Server Binary
//! 
//! This is the main executable that starts the Circuit Breaker HTTP server with both
//! GraphQL and OpenAI-compatible REST API endpoints. It demonstrates how all the pieces
//! come together to create a running workflow engine and LLM router.
//! 
//! ## What This Server Provides
//! 
//! ### GraphQL API (Port 4000)
//! - **GraphQL API**: Complete workflow management via GraphQL
//! - **GraphiQL Interface**: Interactive GraphQL explorer at http://localhost:4000
//! - **Default Workflows**: Pre-loaded example workflows for testing
//! - **Agent Management**: AI agent creation and execution
//! - **WebSocket Subscriptions**: Real-time workflow updates
//! 
//! ### OpenAI-Compatible REST API (Port 3000)
//! - **Chat Completions**: POST /v1/chat/completions (OpenAI compatible)
//! - **Streaming Support**: Server-Sent Events for real-time responses
//! - **Model Management**: GET /v1/models (list available models)
//! - **Provider Routing**: Intelligent routing across multiple LLM providers
//! - **Cost Optimization**: Built-in cost tracking and optimization
//! - **BYOK Model**: Bring Your Own Key for all providers
//! 
//! ## Architecture Demonstration
//! 
//! This binary shows the complete Circuit Breaker architecture:
//! ```text
//! main() function
//!   ↓ spawns
//! GraphQL Server (Port 4000) + OpenAI API Server (Port 3000)
//!   ↓ both use
//! Shared LLM Router + Storage Layer
//!   ↓ operates on
//! Domain Models (Workflows, Tokens, Agents) + LLM Providers
//! ```
//! 
//! ## Usage Examples
//! 
//! ### GraphQL API (http://localhost:4000)
//! - Visit http://localhost:4000 for GraphiQL interface
//! - Send GraphQL queries from any language
//! - Create workflows, tokens, and fire transitions
//! - Manage AI agents and executions
//! 
//! ### OpenAI API (http://localhost:3000)
//! ```bash
//! # Chat completion
//! curl -X POST http://localhost:3000/v1/chat/completions \
//!   -H 'Content-Type: application/json' \
//!   -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello!"}]}'
//! 
//! # Streaming chat completion
//! curl -X POST http://localhost:3000/v1/chat/completions \
//!   -H 'Content-Type: application/json' \
//!   -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello!"}], "stream": true}'
//! 
//! # List models
//! curl http://localhost:3000/v1/models
//! ```
//! 
//! ## Rust Learning Notes:
//! 
//! This file demonstrates several important Rust concepts:
//! - Binary crate vs library crate organization
//! - Async main functions with tokio
//! - Builder pattern for configuration
//! - Error handling with ? operator and Box<dyn Error>
//! - Concurrent server management with tokio::spawn
//! - External crate integration (tracing, tokio, axum)

use circuit_breaker::{
    GraphQLServerBuilder,
    OpenAIApiServerBuilder, OpenAIApiConfig,
    llm::{LLMRouter, cost::CostOptimizer},
};
use tracing_subscriber::{EnvFilter};
use tracing::{info, error, warn};
use dotenv::dotenv;
use std::env;
use async_nats;
use tokio;

/// Configuration from environment variables
struct ServerConfig {
    // GraphQL Server Config
    graphql_port: u16,
    graphql_host: String,
    
    // OpenAI API Server Config
    openai_port: u16,
    openai_host: String,
    openai_cors_enabled: bool,
    openai_api_key_required: bool,
    openai_enable_streaming: bool,
    
    // Shared Config
    log_level: String,
    environment: String,
    storage_type: String,
    nats_url: String,
    
    // LLM Provider Keys
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    google_api_key: Option<String>,
    ollama_base_url: Option<String>,
}

impl ServerConfig {
    fn from_env() -> Self {
        Self {
            // GraphQL Server
            graphql_port: env::var("GRAPHQL_PORT")
                .unwrap_or_else(|_| "4000".to_string())
                .parse()
                .unwrap_or(4000),
            graphql_host: env::var("GRAPHQL_HOST").unwrap_or_else(|_| "localhost".to_string()),
            
            // OpenAI API Server
            openai_port: env::var("OPENAI_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            openai_host: env::var("OPENAI_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            openai_cors_enabled: env::var("OPENAI_CORS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            openai_api_key_required: env::var("OPENAI_API_KEY_REQUIRED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            openai_enable_streaming: env::var("OPENAI_ENABLE_STREAMING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            
            // Shared
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            storage_type: env::var("STORAGE_BACKEND").unwrap_or_else(|_| "memory".to_string()),
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            
            // LLM Provider Keys
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            google_api_key: env::var("GOOGLE_API_KEY").ok(),
            ollama_base_url: env::var("OLLAMA_BASE_URL").ok(),
        }
    }
}

/// Main entry point for the Circuit Breaker unified server
/// 
/// ## Rust Learning Notes:
/// 
/// ### Async Main Function
/// `#[tokio::main]` is a macro that transforms the async main function into
/// a synchronous main that sets up the tokio async runtime. This allows us
/// to use `.await` in the main function.
/// 
/// ### Concurrent Server Management
/// This function demonstrates running multiple servers concurrently using
/// tokio::spawn to create separate async tasks for each server.
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
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load environment variables from .env file
    // This will load variables for API keys, configuration, etc.
    // In production, these would typically be set by the deployment system
    if let Err(e) = dotenv() {
        // Only warn if .env file is missing - it's optional
        eprintln!("Warning: Could not load .env file: {}", e);
        eprintln!("Environment variables must be set manually or via system configuration");
    }

    // Load configuration from environment
    let config = ServerConfig::from_env();

    // Initialize structured logging for the application
    init_logging(&config.log_level)?;

    // Print startup banner - helps identify server startup in logs
    info!("🚀 Starting Circuit Breaker Unified Server...");
    info!("=============================================");
    
    // Log environment configuration
    info!("Environment: {}", config.environment);
    info!("Log Level: {}", config.log_level);
    info!("GraphQL Server: {}:{}", config.graphql_host, config.graphql_port);
    info!("OpenAI API Server: {}:{}", config.openai_host, config.openai_port);
    
    // Log storage configuration
    info!("Storage: {}", config.storage_type);
    if config.storage_type == "nats" {
        info!("NATS URL: {}", config.nats_url);
    }
    
    // Log agent provider configuration (without exposing API keys)
    if config.openai_api_key.is_some() {
        info!("✅ OpenAI API key configured");
    } else {
        warn!("⚠️  No OpenAI API key found in OPENAI_API_KEY");
    }
    if config.anthropic_api_key.is_some() {
        info!("✅ Anthropic API key configured");
    } else {
        warn!("⚠️  No Anthropic API key found in ANTHROPIC_API_KEY");
    }
    if config.google_api_key.is_some() {
        info!("✅ Google API key configured");
    } else {
        warn!("⚠️  No Google API key found in GOOGLE_API_KEY");
    }
    if config.ollama_base_url.is_some() {
        info!("✅ Ollama configuration found");
    } else {
        warn!("⚠️  No Ollama configuration found in OLLAMA_BASE_URL");
    }

    // Create shared LLM infrastructure
    info!("🔧 Initializing shared LLM infrastructure...");
    
    // Create LLM router with configured API keys
    let llm_router = LLMRouter::new_with_keys(
        config.openai_api_key.clone(),
        config.anthropic_api_key.clone(),
        config.google_api_key.clone(),
    ).await.map_err(|e| {
        error!("Failed to create LLM router: {}", e);
        e
    })?;
    
    // Create cost optimizer with dependencies
    let usage_tracker = std::sync::Arc::new(circuit_breaker::llm::cost::InMemoryUsageTracker::new());
    let budget_manager = std::sync::Arc::new(circuit_breaker::llm::cost::BudgetManager::new(usage_tracker));
    let cost_analyzer = std::sync::Arc::new(circuit_breaker::llm::cost::CostAnalyzer::new());
    let cost_optimizer = CostOptimizer::new(budget_manager, cost_analyzer);
    
    info!("✅ Shared LLM infrastructure initialized");

    // Build GraphQL server
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
    
    let mut graphql_builder = GraphQLServerBuilder::new()
        .with_port(config.graphql_port)
        .with_agents();

    // Configure storage backend based on environment variable
    match config.storage_type.as_str() {
        "nats" => {
            info!("🔧 Initializing NATS storage backend...");
            info!("📡 Testing NATS connection to: {}", config.nats_url);
            
            // Test basic NATS connectivity first
            match async_nats::connect(&config.nats_url).await {
                Ok(client) => {
                    info!("✅ Successfully connected to NATS server");
                    
                    // Test JetStream availability
                    let _jetstream = async_nats::jetstream::new(client);
                    info!("✅ JetStream context created successfully");
                    info!("📊 NATS connection ready for workflow storage");
                },
                Err(e) => {
                    error!("❌ Failed to connect to NATS server at {}: {}", config.nats_url, e);
                    error!("💡 Make sure NATS server is running:");
                    error!("   nats-server --jetstream");
                    error!("   Or using Docker: docker run -p 4222:4222 nats:alpine --jetstream");
                    return Err(e.into());
                }
            }
            
            // Now configure the storage backend
            info!("🔧 Configuring NATS storage backend...");
            graphql_builder = graphql_builder.with_nats(&config.nats_url).await
                .map_err(|e| {
                    error!("❌ Failed to initialize NATS storage: {}", e);
                    format!("Failed to initialize NATS storage: {}", e)
                })?;
            info!("✅ NATS storage backend successfully configured");
            info!("🎯 Circuit Breaker will use NATS JetStream for persistent storage");
        },
        "memory" | _ => {
            info!("🔧 Configuring in-memory storage backend");
            info!("⚠️  Note: Data will not persist between server restarts");
            info!("✅ In-memory storage backend configured");
        }
    }

    // Build OpenAI API server
    let openai_config = OpenAIApiConfig {
        port: config.openai_port,
        host: config.openai_host.clone(),
        cors_enabled: config.openai_cors_enabled,
        api_key_required: config.openai_api_key_required,
        enable_streaming: config.openai_enable_streaming,
        max_tokens_per_request: Some(4096),
        rate_limit_per_minute: Some(60),
    };

    let openai_server = OpenAIApiServerBuilder::new()
        .with_port(config.openai_port)
        .with_host(config.openai_host.clone())
        .with_cors(config.openai_cors_enabled)
        .with_api_key_required(config.openai_api_key_required)
        .with_streaming(config.openai_enable_streaming)
        .with_llm_router(llm_router)
        .with_cost_optimizer(cost_optimizer)
        .build();

    // Print server information
    info!("");
    info!("🎯 Servers Starting:");
    info!("📊 GraphQL Server: http://{}:{}", config.graphql_host, config.graphql_port);
    info!("   - GraphiQL Interface: http://{}:{}", config.graphql_host, config.graphql_port);
    info!("   - GraphQL Endpoint: http://{}:{}/graphql", config.graphql_host, config.graphql_port);
    info!("   - WebSocket: ws://{}:{}/ws", config.graphql_host, config.graphql_port);
    info!("");
    info!("🤖 OpenAI API Server: http://{}:{}", config.openai_host, config.openai_port);
    info!("   - Chat Completions: POST http://{}:{}/v1/chat/completions", config.openai_host, config.openai_port);
    info!("   - Models: GET http://{}:{}/v1/models", config.openai_host, config.openai_port);
    info!("   - Health: GET http://{}:{}/health", config.openai_host, config.openai_port);
    info!("");
    info!("📖 Example Usage:");
    info!("   # OpenAI API");
    info!("   curl -X POST http://{}:{}/v1/chat/completions \\", config.openai_host, config.openai_port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{\"model\": \"gpt-4\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello!\"}}]}}'");
    info!("");
    info!("   # Streaming");
    info!("   curl -X POST http://{}:{}/v1/chat/completions \\", config.openai_host, config.openai_port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{\"model\": \"gpt-4\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello!\"}}], \"stream\": true}}'");
    info!("");

    // Start both servers concurrently
    // 
    // ## Rust Learning Notes:
    // 
    // ### Concurrent Task Management
    // We use tokio::spawn to run both servers concurrently in separate tasks.
    // This allows both servers to run simultaneously without blocking each other.
    // 
    // ### tokio::select!
    // The select! macro waits for the first of multiple async operations to complete.
    // In this case, we wait for either server to exit (which shouldn't happen in normal operation).
    
    info!("🚀 Starting both servers...");
    
    let graphql_handle = tokio::spawn(async move {
        if let Err(e) = graphql_builder.build_and_run().await {
            error!("❌ GraphQL server error: {}", e);
            Err(format!("GraphQL server error: {}", e))
        } else {
            Ok(())
        }
    });

    let openai_handle = tokio::spawn(async move {
        if let Err(e) = openai_server.run().await {
            error!("❌ OpenAI server error: {}", e);
            Err(format!("OpenAI server error: {}", e))
        } else {
            Ok(())
        }
    });

    // Wait for either server to exit (which shouldn't happen in normal operation)
    tokio::select! {
        result = graphql_handle => {
            match result {
                Ok(Ok(())) => info!("✅ GraphQL server shutdown gracefully"),
                Ok(Err(e)) => error!("❌ GraphQL server error: {}", e),
                Err(e) => error!("❌ GraphQL server task error: {}", e),
            }
        }
        result = openai_handle => {
            match result {
                Ok(Ok(())) => info!("✅ OpenAI API server shutdown gracefully"),
                Ok(Err(e)) => error!("❌ OpenAI API server error: {}", e),
                Err(e) => error!("❌ OpenAI API server task error: {}", e),
            }
        }
    }

    info!("🏁 Circuit Breaker Unified Server shutdown complete");
    Ok(())
}

/// Initialize logging based on configuration
fn init_logging(log_level: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();
    
    Ok(())
}