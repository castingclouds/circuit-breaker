//! Circuit Breaker Rust SDK
//!
//! A client library for interacting with the Circuit Breaker workflow automation server.
//! This SDK provides ergonomic Rust APIs for managing workflows, agents, functions, and resources.
//!
//! # Quick Start
//!
//! ```rust
//! use circuit_breaker_sdk::{Client, ClientBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client
//!     let client = Client::builder()
//!         .base_url("https://api.circuit-breaker.dev")?
//!         .api_key("your-api-key".to_string())
//!         .build()?;
//!
//!     // Test connection
//!     let info = client.ping().await?;
//!     println!("Connected to Circuit Breaker v{}", info.version);
//!
//!     // Create a workflow
//!     let workflow = client.workflows()
//!         .create()
//!         .name("My Workflow")
//!         .description("Example workflow")
//!         .build()
//!         .await?;
//!
//!     // Execute the workflow
//!     let execution = workflow.execute().await?;
//!     println!("Workflow executed: {}", execution.id);
//!
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod analytics;
pub mod client;
pub mod functions;
pub mod llm;
pub mod mcp;
pub mod nats;
pub mod resources;
pub mod rules;
pub mod schema;
pub mod subscriptions;
pub mod types;
pub mod workflows;

// Re-export main client types
pub use client::{Client, ClientBuilder, ClientConfig};
pub use types::*;

// Re-export commonly used types from each module
pub use agents::{Agent, AgentBuilder};
pub use analytics::{AnalyticsClient, BudgetStatus, CostAnalytics};
pub use functions::{Function, FunctionBuilder, FunctionExecution};
pub use llm::{
    common_models, BudgetConstraint, ChatBuilder, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, ChatRole, CircuitBreakerOptions, LLMClient, RoutingStrategy,
    SmartCompletionRequest, TaskType,
};
pub use mcp::{MCPClient, MCPServer, MCPServerStatus, MCPServerType};
pub use nats::{HistoryEvent, NATSClient, NATSResource};
pub use resources::{Resource, ResourceBuilder};
pub use rules::{Rule, RuleBuilder, RuleEvaluator};
pub use subscriptions::{SubscriptionClient, SubscriptionId, SubscriptionManager};
pub use workflows::{Workflow, WorkflowBuilder, WorkflowExecution};

// Re-export convenience builders
pub use agents::create_agent;
pub use analytics::{budget_status, cost_analytics, set_budget};
pub use llm::{
    create_balanced_chat, create_chat, create_cost_optimized_chat, create_fast_chat,
    create_smart_chat,
};
pub use mcp::{create_mcp_server, list_mcp_servers};
pub use nats::{
    create_workflow_instance, execute_activity_with_nats, get_nats_resource, get_resources_in_state,
};
pub use resources::create_resource;
pub use subscriptions::{subscribe_resource_updates, subscribe_workflow_events};
pub use workflows::create_workflow;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default API base URL
pub const DEFAULT_BASE_URL: &str = "http://localhost:3000";

/// Result type used throughout the SDK
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the SDK
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Server error ({status}): {message}")]
    Server { status: u16, message: String },

    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("Authentication error: {message}")]
    Auth { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Timeout: operation took longer than {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("LLM error: {message}")]
    LLM { message: String },

    #[error("Stream error: {message}")]
    Stream { message: String },
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Error::Timeout { timeout_ms: 30000 }
        } else if error.is_connect() {
            Error::Network {
                message: format!("Connection failed: {}", error),
            }
        } else {
            Error::Network {
                message: error.to_string(),
            }
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Error::Configuration {
            message: format!("Invalid URL: {}", error),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Parse {
            message: format!("JSON parse error: {}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "http://localhost:3000");
    }
}
