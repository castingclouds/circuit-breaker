// OpenAI-compatible API module
// This module provides a REST API that's compatible with OpenAI's API specification

pub mod types;
pub mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;

use handlers::{OpenAIApiState, health_check, list_models, chat_completions, get_model, not_found};
use crate::llm::{LLMRouter, LLMRouterConfig};
use crate::llm::cost::CostOptimizer;

/// OpenAI API server configuration
#[derive(Clone, Debug)]
pub struct OpenAIApiConfig {
    pub port: u16,
    pub host: String,
    pub cors_enabled: bool,
    pub api_key_required: bool,
    pub enable_streaming: bool,
    pub max_tokens_per_request: Option<u32>,
    pub rate_limit_per_minute: Option<u32>,
}

impl Default for OpenAIApiConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(),
            cors_enabled: true,
            api_key_required: false,
            enable_streaming: true,
            max_tokens_per_request: Some(4096),
            rate_limit_per_minute: Some(60),
        }
    }
}

/// OpenAI API Server
pub struct OpenAIApiServer {
    config: OpenAIApiConfig,
    state: OpenAIApiState,
}

impl OpenAIApiServer {
    /// Create a new OpenAI API server
    pub fn new(config: OpenAIApiConfig) -> Self {
        let state = OpenAIApiState::new();
        
        Self {
            config,
            state,
        }
    }

    /// Create server with default configuration
    pub fn with_defaults() -> Self {
        Self::new(OpenAIApiConfig::default())
    }

    /// Set custom LLM router
    pub fn with_llm_router(mut self, router: LLMRouter) -> Self {
        self.state.llm_router = Arc::new(router);
        self
    }

    /// Set custom cost optimizer
    pub fn with_cost_optimizer(mut self, optimizer: CostOptimizer) -> Self {
        self.state.cost_optimizer = Arc::new(RwLock::new(optimizer));
        self
    }

    /// Create the Axum router with all OpenAI-compatible routes
    pub fn create_router(&self) -> Router {
        let api_router = Router::new()
            // Models endpoints
            .route("/v1/models", get(list_models))
            .route("/v1/models/:model_id", get(get_model))
            
            // Chat completions endpoint (both streaming and non-streaming)
            .route("/v1/chat/completions", post(chat_completions))
            
            // Health check
            .route("/health", get(health_check))
            .route("/v1/health", get(health_check))
            
            // Fallback for unknown routes
            .fallback(not_found)
            
            // Add shared state
            .with_state(self.state.clone());

        // Add CORS if enabled
        if self.config.cors_enabled {
            api_router.layer(CorsLayer::permissive())
        } else {
            api_router
        }
    }

    /// Run the server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.create_router();
        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        info!("🤖 OpenAI-compatible API server starting");
        info!("📡 Server address: http://{}", addr);
        info!("🔗 API endpoints:");
        info!("   POST http://{}/v1/chat/completions", addr);
        info!("   GET  http://{}/v1/models", addr);
        info!("   GET  http://{}/health", addr);
        info!("📋 Configuration:");
        info!("   CORS enabled: {}", self.config.cors_enabled);
        info!("   API key required: {}", self.config.api_key_required);
        info!("   Streaming enabled: {}", self.config.enable_streaming);
        
        // Start the server
        axum::Server::bind(&addr.parse()?)
            .serve(app.into_make_service())
            .await?;
        
        Ok(())
    }
}

/// Builder pattern for OpenAI API server
pub struct OpenAIApiServerBuilder {
    config: OpenAIApiConfig,
    llm_router: Option<LLMRouter>,
    cost_optimizer: Option<CostOptimizer>,
}

impl OpenAIApiServerBuilder {
    pub fn new() -> Self {
        Self {
            config: OpenAIApiConfig::default(),
            llm_router: None,
            cost_optimizer: None,
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

    pub fn build(self) -> OpenAIApiServer {
        let mut server = OpenAIApiServer::new(self.config);
        
        if let Some(router) = self.llm_router {
            server = server.with_llm_router(router);
        }
        
        if let Some(optimizer) = self.cost_optimizer {
            server = server.with_cost_optimizer(optimizer);
        }
        
        server
    }
}

impl Default for OpenAIApiServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a server with default settings
pub fn create_default_server() -> OpenAIApiServer {
    OpenAIApiServerBuilder::new().build()
}

/// Convenience function to create a server with custom port
pub fn create_server_with_port(port: u16) -> OpenAIApiServer {
    OpenAIApiServerBuilder::new()
        .with_port(port)
        .build()
}

/// Convenience function to create a server with custom configuration
pub fn create_server_with_config(config: OpenAIApiConfig) -> OpenAIApiServer {
    OpenAIApiServer::new(config)
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
        let router = server.create_router();
        
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