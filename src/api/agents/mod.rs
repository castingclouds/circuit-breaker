// Agent API endpoints for standalone agent execution
// This module provides REST API endpoints for executing agents without workflow dependencies

pub mod http_handlers;
pub mod middleware;
pub mod nats_storage;
pub mod tenant_isolation;
pub mod tenant_storage;
pub mod websocket_handlers;

use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::{Future, SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::agents::middleware::{
    rate_limit, validate_tenant, RateLimitConfig, RateLimiter, TenantId,
};
use crate::engine::{AgentEngine, AgentEngineConfig, AgentStorage, InMemoryAgentStorage};
use crate::models::{
    AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentStreamEvent, LLMConfig,
};
use crate::{CircuitBreakerError, Result};

// Request and response types

#[derive(Debug, Deserialize)]
pub struct ExecuteAgentRequest {
    pub agent_id: String,
    pub context: serde_json::Value,
    pub input_mapping: Option<HashMap<String, String>>,
    pub output_mapping: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteAgentResponse {
    pub execution_id: String,
    pub agent_id: String,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub context: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct GetExecutionRequest {
    pub execution_id: String,
}

// Handler functions

/// Execute an agent with the provided context (non-streaming)
pub async fn execute_agent(
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
    Json(request): Json<ExecuteAgentRequest>,
) -> impl IntoResponse {
    // Log the tenant ID for this request
    info!("Executing agent for tenant: {}", tenant_id.0);

    // Ensure the request context includes tenant information
    let context = if !request.context.get("tenant_id").is_some() {
        // Add tenant ID to context if not present
        let mut context = request.context.clone();
        if let serde_json::Value::Object(ref mut map) = context {
            map.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.0.clone()),
            );
        }
        context
    } else {
        request.context.clone()
    };

    // Create modified request with tenant-enriched context
    let request = ExecuteAgentRequest {
        agent_id: request.agent_id,
        context,
        input_mapping: request.input_mapping,
        output_mapping: request.output_mapping,
    };
    // Convert request to agent config
    let config = AgentActivityConfig {
        agent_id: AgentId::from(request.agent_id),
        input_mapping: request.input_mapping.unwrap_or_default(),
        output_mapping: request.output_mapping.unwrap_or_default(),
        required: true,
        timeout_seconds: Some(30),
        retry_config: None,
    };

    // Execute the agent
    let context = request.context.clone();
    match engine.execute_agent(&config, request.context).await {
        Ok(execution) => {
            let response = ExecuteAgentResponse {
                execution_id: execution.id.to_string(),
                agent_id: execution.agent_id.to_string(),
                status: execution.status.to_string(),
                output: execution.output_data,
                error: execution.error_message,
                context: execution.context,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let error_msg = e.to_string();
            error!("Agent execution failed: {}", error_msg);
            let response = ExecuteAgentResponse {
                execution_id: Uuid::new_v4().to_string(),
                agent_id: config.agent_id.to_string(),
                status: "failed".to_string(),
                output: None,
                error: Some(error_msg),
                context: context.clone(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Execute an agent with streaming response
pub async fn execute_agent_stream(
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
    Json(request): Json<ExecuteAgentRequest>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    // Log the tenant ID for this streaming request
    info!("Streaming agent execution for tenant: {}", tenant_id.0);

    // Ensure the request context includes tenant information
    let context = if !request.context.get("tenant_id").is_some() {
        // Add tenant ID to context if not present
        let mut context = request.context.clone();
        if let serde_json::Value::Object(ref mut map) = context {
            map.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.0.clone()),
            );
        }
        context
    } else {
        request.context.clone()
    };

    // Create modified request with tenant-enriched context
    let request = ExecuteAgentRequest {
        agent_id: request.agent_id,
        context,
        input_mapping: request.input_mapping,
        output_mapping: request.output_mapping,
    };
    // Create a subscriber to the agent stream
    let mut stream = engine.subscribe_to_stream();

    // Configure the agent
    let config = AgentActivityConfig {
        agent_id: AgentId::from(request.agent_id),
        input_mapping: request.input_mapping.unwrap_or_default(),
        output_mapping: request.output_mapping.unwrap_or_default(),
        required: true,
        timeout_seconds: Some(30),
        retry_config: None,
    };

    // Execute the agent and get the execution ID
    let execution_result = engine.execute_agent(&config, request.context).await;
    let execution_id = match execution_result {
        Ok(execution) => execution.id,
        Err(e) => {
            error!("Agent execution failed: {}", e);
            Uuid::new_v4() // Return a placeholder ID for failed executions
        }
    };

    // Transform the broadcast stream into an SSE stream
    let stream = BroadcastStream::new(stream).filter_map(move |msg| async move {
        match msg {
            Ok(event) => {
                // Filter for events related to this execution
                let id = execution_id;

                match event {
                    AgentStreamEvent::ThinkingStatus {
                        execution_id,
                        status,
                        context,
                    } if execution_id == id => {
                        Some(Ok(Event::default().event("thinking").data(status)))
                    }
                    AgentStreamEvent::ContentChunk {
                        execution_id,
                        chunk,
                        sequence,
                        context,
                    } if execution_id == id => Some(Ok(Event::default()
                        .event("chunk")
                        .data(chunk)
                        .id(sequence.to_string()))),
                    AgentStreamEvent::Completed {
                        execution_id,
                        final_response,
                        usage,
                        context,
                    } if execution_id == id => Some(Ok(Event::default()
                        .event("complete")
                        .data(serde_json::to_string(&final_response).unwrap_or_default()))),
                    AgentStreamEvent::Failed {
                        execution_id,
                        error,
                        context,
                    } if execution_id == id => {
                        Some(Ok(Event::default().event("error").data(error)))
                    }
                    _ => None, // Filter out unrelated events
                }
            }
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// Get information about a specific agent execution
pub async fn get_execution(
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
    Path(execution_id): Path<String>,
) -> impl IntoResponse {
    // Log the tenant ID for this request
    info!(
        "Getting execution {} for tenant: {}",
        execution_id, tenant_id.0
    );
    let uuid = match Uuid::parse_str(&execution_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid execution ID format"
                })),
            )
        }
    };

    match engine.storage().get_execution(&uuid).await {
        Ok(Some(execution)) => {
            let response = ExecuteAgentResponse {
                execution_id: execution.id.to_string(),
                agent_id: execution.agent_id.to_string(),
                status: execution.status.to_string(),
                output: execution.output_data,
                error: execution.error_message,
                context: execution.context,
            };
            (
                StatusCode::OK,
                Json(serde_json::to_value(response).unwrap()),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::to_value(ExecuteAgentResponse {
                    execution_id: execution_id.clone(),
                    agent_id: "unknown".to_string(),
                    status: "not_found".to_string(),
                    output: None,
                    error: Some(format!("Execution with ID {} not found", execution_id)),
                    context: serde_json::json!({}),
                })
                .unwrap(),
            ),
        ),
        Err(e) => {
            error!("Error retrieving execution {}: {}", execution_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::to_value(ExecuteAgentResponse {
                        execution_id: execution_id.clone(),
                        agent_id: "unknown".to_string(),
                        status: "error".to_string(),
                        output: None,
                        error: Some(format!("Failed to retrieve execution: {}", e)),
                        context: serde_json::json!({}),
                    })
                    .unwrap(),
                ),
            )
        }
    }
}

// Route registration

/// Handle WebSocket connections for agent streaming
pub async fn execute_agent_websocket(
    ws: WebSocketUpgrade,
    tenant_id: TenantId,
    State(engine): State<Arc<AgentEngine>>,
) -> impl IntoResponse {
    // Log the tenant ID for this WebSocket connection
    info!("WebSocket connection for tenant: {}", tenant_id.0);

    // Pass tenant ID to the WebSocket handler
    ws.on_upgrade(move |socket| handle_websocket_connection(socket, engine, tenant_id.0))
    // This line is replaced in the edit above
}

/// Process WebSocket connections for agent streaming
async fn handle_websocket_connection(
    mut socket: WebSocket,
    engine: Arc<AgentEngine>,
    tenant_id: String,
) {
    // Create a subscriber to the agent stream
    let mut stream = engine.subscribe_to_stream();

    // First message should be JSON with agent execution request
    let message = match socket.recv().await {
        Some(Ok(Message::Text(text))) => text,
        _ => {
            // Send error message and close
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "error": "Expected JSON execution request"
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };

    // Parse the request
    let mut request: ExecuteAgentRequest = match serde_json::from_str(&message) {
        Ok(req) => req,
        Err(e) => {
            // Send error message and close
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "error": format!("Invalid request format: {}", e)
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };

    // Configure the agent
    let config = AgentActivityConfig {
        agent_id: AgentId::from(request.agent_id),
        input_mapping: request.input_mapping.unwrap_or_default(),
        output_mapping: request.output_mapping.unwrap_or_default(),
        required: true,
        timeout_seconds: Some(30),
        retry_config: None,
    };

    // Ensure the request context includes tenant information
    if !request.context.get("tenant_id").is_some() {
        // Add tenant ID to context if not present
        if let serde_json::Value::Object(ref mut map) = request.context {
            map.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.clone()),
            );
        }
    }

    // Send initial message
    let _ = socket
        .send(Message::Text(
            serde_json::json!({
                "type": "init",
                "message": "Agent execution started",
                "tenant_id": tenant_id
            })
            .to_string(),
        ))
        .await;

    // Clone engine for the async task
    let engine_clone = engine.clone();

    // Spawn the execution in the background
    let execution_id = tokio::spawn(async move {
        let execution_future = engine_clone.execute_agent(&config, request.context);
        match execution_future.await {
            Ok(execution) => execution.id,
            Err(e) => {
                error!("Agent execution failed: {}", e);
                Uuid::new_v4() // Return a placeholder ID for failed executions
            }
        }
    });

    // Process stream events
    let mut recv_task = tokio::spawn(async move {
        // Wait for execution to start and get the ID
        let id = match execution_id.await {
            Ok(id) => id,
            Err(_) => return,
        };

        while let Ok(event) = stream.recv().await {
            match event {
                AgentStreamEvent::ThinkingStatus {
                    execution_id,
                    status,
                    context,
                } if execution_id == id => {
                    let _ = socket
                        .send(Message::Text(
                            serde_json::json!({
                                "type": "thinking",
                                "message": status,
                                "context": context,
                            })
                            .to_string(),
                        ))
                        .await;
                }
                AgentStreamEvent::ContentChunk {
                    execution_id,
                    chunk,
                    sequence,
                    context,
                } if execution_id == id => {
                    let _ = socket
                        .send(Message::Text(
                            serde_json::json!({
                                "type": "chunk",
                                "message": chunk,
                                "sequence": sequence,
                                "context": context,
                            })
                            .to_string(),
                        ))
                        .await;
                }
                AgentStreamEvent::Completed {
                    execution_id,
                    final_response,
                    usage,
                    context,
                } if execution_id == id => {
                    let _ = socket
                        .send(Message::Text(
                            serde_json::json!({
                                "type": "complete",
                                "message": final_response,
                                "usage": usage,
                                "context": context,
                            })
                            .to_string(),
                        ))
                        .await;

                    // Close the connection after completion
                    break;
                }
                AgentStreamEvent::Failed {
                    execution_id,
                    error,
                    context,
                } if execution_id == id => {
                    let _ = socket
                        .send(Message::Text(
                            serde_json::json!({
                                "type": "error",
                                "message": error,
                                "context": context,
                            })
                            .to_string(),
                        ))
                        .await;

                    // Close the connection after error
                    break;
                }
                _ => {} // Ignore other events
            }
        }
    });

    // Wait for the WebSocket to close or the stream to end
    let _result = recv_task.await;
}

// Route registration
pub fn routes(engine: Arc<AgentEngine>) -> Router {
    // Create the base router with agent endpoints
    let base_router = Router::new()
        .route("/api/agents/execute", post(execute_agent))
        .route("/api/agents/stream", post(execute_agent_stream))
        .route("/api/agents/ws", get(execute_agent_websocket))
        .route("/api/agents/executions/:execution_id", get(get_execution))
        .with_state(engine);

    // Apply tenant validation middleware to all routes
    base_router.layer(axum::middleware::from_fn(validate_tenant))
}

/// Builder for the standalone agent API server
pub struct StandaloneAgentApiBuilder {
    engine: Option<Arc<AgentEngine>>,
    storage: Option<Arc<dyn AgentStorage>>,
    config: AgentEngineConfig,
    rate_limit_config: Option<RateLimitConfig>,
    enable_rate_limiting: bool,
}

impl StandaloneAgentApiBuilder {
    /// Create a new standalone agent API builder
    pub fn new() -> Self {
        Self {
            engine: None,
            storage: None,
            config: AgentEngineConfig::default(),
            rate_limit_config: None,
            enable_rate_limiting: false,
        }
    }

    /// Set a custom agent engine configuration
    pub fn with_config(mut self, config: AgentEngineConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable rate limiting with custom configuration
    pub fn with_rate_limiting(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self.enable_rate_limiting = true;
        self
    }

    /// Enable rate limiting with default configuration
    pub fn with_default_rate_limiting(mut self) -> Self {
        self.rate_limit_config = Some(RateLimitConfig::default());
        self.enable_rate_limiting = true;
        self
    }

    /// Enable request queuing with specified queue size
    pub fn with_request_queuing(mut self, queue_size: usize) -> Self {
        if self.rate_limit_config.is_none() {
            let mut config = RateLimitConfig::default();
            config.max_queue_size = queue_size;
            self.rate_limit_config = Some(config);
        } else {
            let mut config = self.rate_limit_config.unwrap();
            config.max_queue_size = queue_size;
            self.rate_limit_config = Some(config);
        }
        self.enable_rate_limiting = true;
        self
    }

    /// Use an existing agent engine
    pub fn with_engine(mut self, engine: Arc<AgentEngine>) -> Self {
        self.engine = Some(engine);
        self
    }

    /// Use an existing storage implementation
    pub fn with_storage(mut self, storage: Arc<dyn AgentStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Use in-memory storage (for testing or simple deployments)
    pub fn with_memory_storage(mut self) -> Self {
        self.storage = Some(Arc::new(InMemoryAgentStorage::default()));
        self
    }

    /// Use NATS storage (for production deployments)
    pub async fn with_nats_storage(mut self, nats_url: &str) -> Result<Self> {
        // This is a placeholder - actual implementation would connect to NATS
        // and create a NATS-backed implementation of AgentStorage
        debug!("Connecting to NATS for agent storage at {}", nats_url);

        // For now, we'll use in-memory storage
        // In a real implementation, this would be replaced with a NATS implementation
        self.storage = Some(Arc::new(InMemoryAgentStorage::default()));

        Ok(self)
    }

    /// Build the agent API router
    pub fn build(self) -> Router {
        // If an engine was provided, use it
        let engine = if let Some(engine) = self.engine {
            engine
        } else {
            // Otherwise, create a new engine from storage and config
            let storage = self.storage.unwrap_or_else(|| {
                debug!("No storage provided, using in-memory storage");
                Arc::new(InMemoryAgentStorage::default())
            });

            Arc::new(AgentEngine::new(storage, self.config))
        };

        let mut router = routes(engine);

        // Add rate limiting if enabled
        if self.enable_rate_limiting {
            let config = self.rate_limit_config.unwrap_or_default();
            let rate_limiter = Arc::new(RateLimiter::new(config));

            // Apply rate limiting middleware to all routes
            router = router.layer(axum::middleware::from_fn_with_state(
                rate_limiter,
                rate_limit,
            ));

            info!("Rate limiting enabled for agent API");
        }

        info!("Tenant validation enabled for agent API");

        router
    }

    /// Build the agent API router with async initialization
    pub async fn build_async(self) -> Router {
        self.build()
    }
}

/// Helper function to add agent API routes to an existing Axum application
pub fn add_routes_to_app(app: Router, engine: Arc<AgentEngine>) -> Router {
    app.merge(routes(engine))
}

/// Helper function to add agent API routes with rate limiting to an existing Axum application
pub fn add_routes_with_rate_limiting(
    app: Router,
    engine: Arc<AgentEngine>,
    config: RateLimitConfig,
) -> Router {
    let rate_limiter = Arc::new(RateLimiter::new(config));
    let agent_routes = routes(engine).layer(axum::middleware::from_fn_with_state(
        rate_limiter,
        rate_limit,
    ));

    app.merge(agent_routes)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod http_handlers_tests;

#[cfg(test)]
mod websocket_tests;

#[cfg(test)]
mod tenant_isolation_tests;

#[cfg(test)]
mod nats_storage_tests;

// Re-export HTTP handlers for convenience
pub use http_handlers::{
    execute_agent as execute_agent_http, get_execution_details, list_agent_executions,
    routes as http_routes, stream_execution_events,
};

// Re-export WebSocket handlers for convenience
pub use websocket_handlers::{routes as websocket_routes, ws_handler, WebSocketState};

// Re-export tenant isolation for convenience
pub use tenant_isolation::{
    RateLimits, ResourceQuotas, TenantAwareAgentEngine, TenantAwareAgentEngineFactory, TenantConfig,
};

// Re-export tenant storage for convenience
pub use tenant_storage::{
    BackupManager, BackupType, MetricsCollector, PartitionStrategy, TenantAgentStorage,
    TenantId as TenantStorageId, TenantStorageConfig, TenantStorageMetrics,
};

// Re-export NATS storage for convenience
pub use nats_storage::{create_tenant_aware_nats_storage, NatsAgentStorage, NatsStorageConfig};
