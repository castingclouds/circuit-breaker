// API module for Circuit Breaker
// This module provides multiple API interfaces:
// - OpenAI-compatible REST API
// - MCP (Model Context Protocol) server

pub mod agents;
pub mod handlers;
pub mod mcp_auth;
pub mod mcp_oauth_setup;
pub mod mcp_server;
pub mod mcp_storage;
pub mod mcp_types;
pub mod oauth;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::llm::cost::CostOptimizer;
use crate::llm::LLMRouter;
use handlers::{chat_completions, get_model, health_check, list_models, not_found, OpenAIApiState};
use mcp_oauth_setup::setup_oauth_providers;
use mcp_server::MCPServerManager;
use tracing::warn;

/// API server configuration
#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub port: u16,
    pub host: String,
    pub cors_enabled: bool,
    pub api_key_required: bool,
    pub enable_streaming: bool,
    pub max_tokens_per_request: Option<u32>,
    pub rate_limit_per_minute: Option<u32>,
    pub enable_openai_api: bool,
    pub enable_mcp_server: bool,
}

/// OpenAI API server configuration (for backward compatibility)
pub type OpenAIApiConfig = ApiConfig;

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(),
            cors_enabled: true,
            api_key_required: false,
            enable_streaming: true,
            max_tokens_per_request: Some(4096),
            rate_limit_per_minute: Some(60),
            enable_openai_api: true,
            enable_mcp_server: true,
        }
    }
}

/// Circuit Breaker API Server
pub struct CircuitBreakerApiServer {
    config: ApiConfig,
    openai_state: OpenAIApiState,
    mcp_manager: MCPServerManager,
}

/// OpenAI API Server (for backward compatibility)
pub type OpenAIApiServer = CircuitBreakerApiServer;

impl CircuitBreakerApiServer {
    /// Create a new Circuit Breaker API server
    pub fn new(config: ApiConfig) -> Self {
        let openai_state = OpenAIApiState::new();
        let mcp_manager = MCPServerManager::new();

        Self {
            config,
            openai_state,
            mcp_manager,
        }
    }

    /// Create a new Circuit Breaker API server with NATS storage
    pub async fn with_nats_storage(
        config: ApiConfig,
        nats_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let openai_state = OpenAIApiState::new();
        let mcp_manager = MCPServerManager::with_nats_storage(nats_url)
            .await
            .map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;

        Ok(Self {
            config,
            openai_state,
            mcp_manager,
        })
    }

    /// Initialize OAuth providers for remote MCP
    pub async fn setup_oauth(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.config.enable_mcp_server {
            match setup_oauth_providers(&self.mcp_manager).await {
                Ok(config) => {
                    if config.enabled && config.has_providers() {
                        info!("✅ OAuth providers configured for remote MCP server");
                    }
                }
                Err(e) => {
                    warn!("⚠️  Failed to setup OAuth providers: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Create server with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ApiConfig::default())
    }

    /// Set custom LLM router
    pub fn with_llm_router(mut self, router: LLMRouter) -> Self {
        self.openai_state.llm_router = Arc::new(router);
        self
    }

    /// Set custom cost optimizer
    pub fn with_cost_optimizer(mut self, optimizer: CostOptimizer) -> Self {
        self.openai_state.cost_optimizer = Arc::new(RwLock::new(optimizer));
        self
    }

    /// Create the Axum router with all API routes
    pub fn create_router(&self) -> Router {
        let mut app = Router::new();

        // Add OpenAI-compatible API routes if enabled
        if self.config.enable_openai_api {
            let openai_router = Router::new()
                // Models endpoints
                .route("/v1/models", get(list_models))
                .route("/v1/models/:model_id", get(get_model))
                // Chat completions endpoint (both streaming and non-streaming)
                .route("/v1/chat/completions", post(chat_completions))
                // Embeddings endpoint
                .route("/v1/embeddings", post(handlers::embeddings))
                // Health check
                .route("/health", get(health_check))
                .route("/v1/health", get(health_check))
                // Add OpenAI state
                .with_state(self.openai_state.clone());

            app = app.merge(openai_router);
        }

        // Add MCP server routes if enabled
        if self.config.enable_mcp_server {
            let mcp_server =
                mcp_server::CircuitBreakerMCPServer::with_manager(self.mcp_manager.clone());
            let mcp_router = mcp_server.create_router();
            app = app.merge(mcp_router);

            // Remote MCP functionality is built into the main MCP router
        }

        // Add fallback for unknown routes
        app = app.fallback(not_found);

        // Add CORS if enabled
        if self.config.cors_enabled {
            app.layer(CorsLayer::permissive())
        } else {
            app
        }
    }

    /// Run the server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Setup OAuth providers before starting the server
        self.setup_oauth().await?;

        let app = self.create_router();
        let addr = format!("{}:{}", self.config.host, self.config.port);

        info!("🚀 Circuit Breaker API server starting");
        info!("📡 Server address: http://{}", addr);
        info!("🔗 Available APIs:");

        if self.config.enable_openai_api {
            info!("   OpenAI-compatible API:");
            info!("     POST http://{}/v1/chat/completions", addr);
            info!("     GET  http://{}/v1/models", addr);
            info!("     GET  http://{}/health", addr);
        }

        if self.config.enable_mcp_server {
            info!("   MCP (Model Context Protocol) server:");
            info!("     POST http://{}/mcp", addr);
            info!("     WS   http://{}/mcp/ws", addr);
            info!("     GET  http://{}/mcp/info", addr);
            info!("   Remote MCP instances (OAuth per instance):");
            info!("     GET  http://{}/mcp/{{instance_id}}", addr);
            info!("     POST http://{}/mcp/{{instance_id}}", addr);
        }

        info!("📋 Configuration:");
        info!("   CORS enabled: {}", self.config.cors_enabled);
        info!("   API key required: {}", self.config.api_key_required);
        info!("   Streaming enabled: {}", self.config.enable_streaming);
        info!("   OpenAI API enabled: {}", self.config.enable_openai_api);
        info!("   MCP server enabled: {}", self.config.enable_mcp_server);

        // Start the server
        axum::Server::bind(&addr.parse()?)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }
}

/// Builder pattern for Circuit Breaker API server
pub struct CircuitBreakerApiServerBuilder {
    config: ApiConfig,
    llm_router: Option<LLMRouter>,
    cost_optimizer: Option<CostOptimizer>,
    nats_url: Option<String>,
}

/// OpenAI API server builder (for backward compatibility)
pub type OpenAIApiServerBuilder = CircuitBreakerApiServerBuilder;

impl CircuitBreakerApiServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ApiConfig::default(),
            llm_router: None,
            cost_optimizer: None,
            nats_url: None,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn with_host(mut self, host: String) -> Self {
        self.config.host = host;
        self
    }

    pub fn with_cors(mut self, enabled: bool) -> Self {
        self.config.cors_enabled = enabled;
        self
    }

    pub fn with_api_key_required(mut self, required: bool) -> Self {
        self.config.api_key_required = required;
        self
    }

    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.config.enable_streaming = enabled;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.max_tokens_per_request = Some(max_tokens);
        self
    }

    pub fn with_rate_limit(mut self, requests_per_minute: u32) -> Self {
        self.config.rate_limit_per_minute = Some(requests_per_minute);
        self
    }

    pub fn with_llm_router(mut self, router: LLMRouter) -> Self {
        self.llm_router = Some(router);
        self
    }

    pub fn with_cost_optimizer(mut self, optimizer: CostOptimizer) -> Self {
        self.cost_optimizer = Some(optimizer);
        self
    }

    pub fn with_nats_storage(mut self, nats_url: String) -> Self {
        self.nats_url = Some(nats_url);
        self
    }

    pub fn with_openai_api(mut self, enabled: bool) -> Self {
        self.config.enable_openai_api = enabled;
        self
    }

    pub fn with_mcp_server(mut self, enabled: bool) -> Self {
        self.config.enable_mcp_server = enabled;
        self
    }

    pub async fn build_async(self) -> CircuitBreakerApiServer {
        let mut server = if let Some(nats_url) = self.nats_url {
            CircuitBreakerApiServer::with_nats_storage(self.config, &nats_url)
                .await
                .expect("Failed to create server with NATS storage")
        } else {
            CircuitBreakerApiServer::new(self.config)
        };

        if let Some(router) = self.llm_router {
            server = server.with_llm_router(router);
            // Refresh models after setting the router
            server.openai_state.refresh_models().await;
        }

        if let Some(optimizer) = self.cost_optimizer {
            server = server.with_cost_optimizer(optimizer);
        }

        server
    }

    pub fn build(self) -> CircuitBreakerApiServer {
        let mut server = if self.nats_url.is_some() {
            panic!("Cannot use NATS storage with synchronous build - use build_async() instead");
        } else {
            CircuitBreakerApiServer::new(self.config)
        };

        if let Some(router) = self.llm_router {
            server = server.with_llm_router(router);
        }

        if let Some(optimizer) = self.cost_optimizer {
            server = server.with_cost_optimizer(optimizer);
        }

        server
    }
}

impl Default for CircuitBreakerApiServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a server with default settings
pub fn create_default_server() -> CircuitBreakerApiServer {
    CircuitBreakerApiServerBuilder::new().build()
}

/// Convenience function to create a server with custom port
pub fn create_server_with_port(port: u16) -> CircuitBreakerApiServer {
    CircuitBreakerApiServerBuilder::new()
        .with_port(port)
        .build()
}

/// Convenience function to create a server with custom configuration
pub fn create_server_with_config(config: ApiConfig) -> CircuitBreakerApiServer {
    CircuitBreakerApiServer::new(config)
}

/// Convenience function to create an OpenAI-only server
pub fn create_openai_only_server() -> CircuitBreakerApiServer {
    CircuitBreakerApiServerBuilder::new()
        .with_openai_api(true)
        .with_mcp_server(false)
        .build()
}

/// Convenience function to create an MCP-only server
pub fn create_mcp_only_server() -> CircuitBreakerApiServer {
    CircuitBreakerApiServerBuilder::new()
        .with_openai_api(false)
        .with_mcp_server(true)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_server_creation() {
        let server = create_default_server();
        assert_eq!(server.config.port, 3000);
        assert!(server.config.cors_enabled);
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let server = OpenAIApiServerBuilder::new()
            .with_port(8080)
            .with_cors(false)
            .with_api_key_required(true)
            .build();

        assert_eq!(server.config.port, 8080);
        assert!(!server.config.cors_enabled);
        assert!(server.config.api_key_required);
    }

    #[tokio::test]
    async fn test_router_creation() {
        let server = create_default_server();
        let _router = server.create_router();

        // Test that the router was created successfully
        // In a real test, you might want to test specific routes
        assert!(true); // Placeholder for now
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let server = create_default_server();
        let app = server.create_router();

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
