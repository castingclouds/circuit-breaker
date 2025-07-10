// Agent HTTP API handlers
// This module provides REST API endpoints for agent execution with a clean HTTP interface

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::agents::middleware::{rate_limit, validate_tenant, RateLimiter, TenantId};
use crate::api::agents::websocket_handlers::{ws_handler, ConnectionManager, WebSocketState};
use crate::engine::AgentEngine;
use crate::models::{
    AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentStreamEvent, LLMConfig,
};
use crate::{CircuitBreakerError, Result};

// Request and response types

#[derive(Debug, Deserialize)]
pub struct ExecuteAgentRequest {
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
    pub created_at: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AgentExecutionSummary {
    pub execution_id: String,
    pub agent_id: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub has_error: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListExecutionsQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListExecutionsResponse {
    pub executions: Vec<AgentExecutionSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

// Handler functions

/// Execute an agent with the provided context (non-streaming)
/// POST /agents/{agent_id}/execute
pub async fn execute_agent(
    Path(agent_id): Path<String>,
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
    Json(request): Json<ExecuteAgentRequest>,
) -> impl IntoResponse {
    // Log the request with details
    info!(
        "🚀 POST /agents/{}/execute called for tenant {}",
        agent_id, tenant_id.0
    );
    debug!(
        "📋 Request context: {}",
        serde_json::to_string(&request.context).unwrap_or_else(|_| "Invalid JSON".to_string())
    );
    debug!("📋 Input mapping: {:?}", request.input_mapping);
    debug!("📋 Output mapping: {:?}", request.output_mapping);

    // Ensure the request context includes tenant information
    let context = ensure_tenant_in_context(request.context, &tenant_id.0);
    debug!(
        "📋 Enhanced context with tenant: {}",
        serde_json::to_string(&context).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    // Convert request to agent config
    let config = AgentActivityConfig {
        agent_id: AgentId::from(agent_id.clone()),
        input_mapping: request.input_mapping.unwrap_or_default(),
        output_mapping: request.output_mapping.unwrap_or_default(),
        required: true,
        timeout_seconds: Some(300), // 5 minutes default timeout
        retry_config: None,
    };

    info!("⚙️  Agent config created - calling engine.execute_agent()");
    debug!(
        "⚙️  Agent config: agent_id={}, timeout={}s",
        config.agent_id,
        config.timeout_seconds.unwrap_or(0)
    );

    // Execute the agent
    let execution_start = std::time::Instant::now();
    info!(
        "🎯 Starting agent execution for agent_id: {}",
        config.agent_id
    );

    match engine.execute_agent(&config, context).await {
        Ok(execution) => {
            let execution_duration = execution_start.elapsed();
            info!(
                "✅ Agent execution completed successfully in {:?}",
                execution_duration
            );
            info!("📋 Execution ID: {}", execution.id);
            info!("📋 Execution status: {:?}", execution.status);

            if let Some(ref output) = execution.output_data {
                debug!(
                    "📋 Execution output: {}",
                    serde_json::to_string(output).unwrap_or_else(|_| "Invalid JSON".to_string())
                );
            } else {
                debug!("📋 No execution output");
            }

            if let Some(ref error) = execution.error_message {
                warn!("⚠️  Execution completed with error: {}", error);
            }

            let response = ExecuteAgentResponse {
                execution_id: execution.id.to_string(),
                agent_id: execution.agent_id.to_string(),
                status: execution.status.to_string(),
                output: execution.output_data,
                error: execution.error_message,
                created_at: execution.started_at.to_rfc3339(),
                context: execution.context,
            };

            info!(
                "📤 Sending successful response for execution {}",
                execution.id
            );
            (
                StatusCode::OK,
                Json(serde_json::to_value(response).unwrap_or_else(
                    |_| serde_json::json!({"error": "Failed to serialize response"}),
                )),
            )
        }
        Err(e) => {
            let execution_duration = execution_start.elapsed();
            let error_msg = e.to_string();
            error!(
                "❌ Agent execution failed after {:?}: {}",
                execution_duration, error_msg
            );
            error!("🔍 Error details: {:?}", e);
            error!("🔍 Failed agent_id: {}", agent_id);
            error!("🔍 Failed tenant: {}", tenant_id.0);

            // Log the error type for debugging
            match &e {
                crate::CircuitBreakerError::Internal(message) => {
                    error!("🌐 Internal error: {}", message);
                }
                crate::CircuitBreakerError::Storage(storage_err) => {
                    error!("💾 Storage error: {}", storage_err);
                }
                crate::CircuitBreakerError::NotFound(message) => {
                    error!("🤖 Resource not found: {}", message);
                }
                crate::CircuitBreakerError::RateLimited(message) => {
                    error!("⏰ Rate limited: {}", message);
                }
                _ => {
                    error!("🔍 Other error type: {:?}", e);
                }
            }

            let error_response = serde_json::json!({
                "error": error_msg,
                "agent_id": agent_id,
                "tenant_id": tenant_id.0,
                "execution_duration_ms": execution_duration.as_millis(),
                "error_type": format!("{:?}", e).split('(').next().unwrap_or("Unknown"),
            });

            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
        }
    }
}

/// List executions for a specific agent
/// GET /agents/{agent_id}/executions
pub async fn list_agent_executions(
    Path(agent_id): Path<String>,
    Query(query): Query<ListExecutionsQuery>,
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
) -> impl IntoResponse {
    info!(
        "📋 GET /agents/{}/executions called for tenant {}",
        agent_id, tenant_id.0
    );

    // Extract query parameters with defaults
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let agent_id_parsed = AgentId::from(agent_id.clone());

    // Get executions for this agent
    let executions = match engine
        .storage()
        .list_executions_for_agent(&agent_id_parsed)
        .await
    {
        Ok(executions) => {
            // Filter by tenant_id in context
            let tenant_executions: Vec<AgentExecution> = executions
                .into_iter()
                .filter(|exec| {
                    // Extract tenant_id from context and check if it matches
                    if let Some(exec_tenant) =
                        exec.context.get("tenant_id").and_then(|t| t.as_str())
                    {
                        exec_tenant == tenant_id.0
                    } else {
                        false // No tenant ID means not accessible
                    }
                })
                .collect();

            // Filter by status if provided
            let status_filtered = if let Some(status_str) = &query.status {
                tenant_executions
                    .into_iter()
                    .filter(|exec| {
                        exec.status.to_string().to_lowercase() == status_str.to_lowercase()
                    })
                    .collect()
            } else {
                tenant_executions
            };

            status_filtered
        }
        Err(e) => {
            error!("Error listing executions: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to list executions: {}", e)
                })),
            );
        }
    };

    // Calculate pagination
    let total = executions.len();
    let paginated = executions
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|exec| AgentExecutionSummary {
            execution_id: exec.id.to_string(),
            agent_id: exec.agent_id.to_string(),
            status: exec.status.to_string(),
            created_at: exec.started_at.to_rfc3339(),
            completed_at: exec.completed_at.map(|dt| dt.to_rfc3339()),
            has_error: exec.error_message.is_some(),
        })
        .collect();

    // Create response
    let response = ListExecutionsResponse {
        executions: paginated,
        total,
        page: offset / limit,
        page_size: limit,
    };

    (
        StatusCode::OK,
        Json(
            serde_json::to_value(response)
                .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize response"})),
        ),
    )
}

/// Get details of a specific execution
/// GET /agents/{agent_id}/executions/{execution_id}
pub async fn get_execution_details(
    Path((agent_id, execution_id)): Path<(String, String)>,
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
) -> impl IntoResponse {
    info!(
        "🔍 GET /agents/{}/executions/{} called for tenant {}",
        agent_id, execution_id, tenant_id.0
    );

    // Parse execution ID
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

    // Get execution details
    match engine.storage().get_execution(&uuid).await {
        Ok(Some(execution)) => {
            // Verify agent ID matches
            if execution.agent_id.to_string() != agent_id {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": "Execution not found for this agent"
                    })),
                );
            }

            // Verify tenant ID matches
            if let Some(exec_tenant) = execution.context.get("tenant_id").and_then(|t| t.as_str()) {
                if exec_tenant != tenant_id.0 {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(serde_json::json!({
                            "error": "Access denied to this execution"
                        })),
                    );
                }
            } else {
                // No tenant ID in context is a security issue
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({
                        "error": "Access denied to this execution"
                    })),
                );
            }

            // Return execution details
            let response = ExecuteAgentResponse {
                execution_id: execution.id.to_string(),
                agent_id: execution.agent_id.to_string(),
                status: execution.status.to_string(),
                output: execution.output_data,
                error: execution.error_message,
                created_at: execution.started_at.to_rfc3339(),
                context: execution.context,
            };

            (
                StatusCode::OK,
                Json(serde_json::to_value(response).unwrap_or_else(
                    |_| serde_json::json!({"error": "Failed to serialize response"}),
                )),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Execution with ID {} not found", execution_id)
            })),
        ),
        Err(e) => {
            error!("Error retrieving execution {}: {}", execution_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to retrieve execution: {}", e)
                })),
            )
        }
    }
}

/// Execute an agent with streaming response
/// POST /agents/{agent_id}/execute/stream
pub async fn execute_agent_stream(
    Path(agent_id): Path<String>,
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
    Json(request): Json<ExecuteAgentRequest>,
) -> impl IntoResponse {
    info!(
        "🌊 Starting streaming execution for agent_id: {}, tenant: {}",
        agent_id, tenant_id.0
    );

    // Ensure tenant ID is in context
    let enhanced_context = ensure_tenant_in_context(request.context, &tenant_id.0);

    // Create agent config
    let config = AgentActivityConfig {
        agent_id: AgentId::from(agent_id.clone()),
        input_mapping: request.input_mapping.unwrap_or_default(),
        output_mapping: request.output_mapping.unwrap_or_default(),
        required: true,
        timeout_seconds: Some(300),
        retry_config: None,
    };

    // Subscribe to stream before starting execution
    let stream = engine.subscribe_to_stream();

    // Start the agent execution and get the execution ID
    let execution_result = engine.execute_agent(&config, enhanced_context).await;
    let execution_id = match execution_result {
        Ok(execution) => {
            info!("🌊 Streaming execution started: {}", execution.id);
            execution.id
        }
        Err(e) => {
            error!("🌊 Streaming execution failed: {}", e);
            // Return error response
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
                .unwrap()
                .into_response();
        }
    };

    // Create manual SSE response with proper headers and termination
    let (mut sender, body) = axum::body::Body::channel();

    tokio::spawn(async move {
        let mut stream = BroadcastStream::new(stream);

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(event) => {
                    // Filter events for this specific execution ID
                    let event_execution_id = match &event {
                        AgentStreamEvent::ThinkingStatus { execution_id, .. } => *execution_id,
                        AgentStreamEvent::ContentChunk { execution_id, .. } => *execution_id,
                        AgentStreamEvent::Completed { execution_id, .. } => *execution_id,
                        AgentStreamEvent::Failed { execution_id, .. } => *execution_id,
                        AgentStreamEvent::ToolCall { execution_id, .. } => *execution_id,
                        AgentStreamEvent::ToolResult { execution_id, .. } => *execution_id,
                    };

                    // Only process events for our execution
                    if event_execution_id != execution_id {
                        continue;
                    }

                    let sse_data = match &event {
                        AgentStreamEvent::ThinkingStatus { status, .. } => {
                            format!("event: thinking\ndata: {}\n\n", status)
                        }
                        AgentStreamEvent::ContentChunk {
                            chunk, sequence, ..
                        } => {
                            format!("event: chunk\nid: {}\ndata: {}\n\n", sequence, chunk)
                        }
                        AgentStreamEvent::Completed { final_response, .. } => {
                            let response_data =
                                serde_json::to_string(&final_response).unwrap_or_default();
                            format!("event: complete\ndata: {}\n\n", response_data)
                        }
                        AgentStreamEvent::Failed { error, .. } => {
                            format!("event: error\ndata: {}\n\n", error)
                        }
                        AgentStreamEvent::ToolCall {
                            tool_name,
                            arguments,
                            ..
                        } => {
                            let tool_data = serde_json::json!({"tool_name": tool_name, "tool_input": arguments});
                            format!("event: tool_call\ndata: {}\n\n", tool_data.to_string())
                        }
                        AgentStreamEvent::ToolResult {
                            tool_name, result, ..
                        } => {
                            let tool_data =
                                serde_json::json!({"tool_name": tool_name, "tool_result": result});
                            format!("event: tool_result\ndata: {}\n\n", tool_data.to_string())
                        }
                    };

                    if sender.send_data(sse_data.into()).await.is_err() {
                        break;
                    }

                    // Terminate stream after completion or error
                    match &event {
                        AgentStreamEvent::Completed { .. } | AgentStreamEvent::Failed { .. } => {
                            // Send termination signal
                            let _ = sender.send_data("data: [DONE]\n\n".into()).await;
                            break;
                        }
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .body(body)
        .map_err(|e| {
            error!("Failed to create SSE response: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
                .unwrap()
        })
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
                .unwrap()
        })
        .into_response()
}

/// Stream events for a specific execution
/// GET /agents/{agent_id}/executions/{execution_id}/stream
pub async fn stream_execution_events(
    Path((agent_id, execution_id)): Path<(String, String)>,
    State(engine): State<Arc<AgentEngine>>,
    tenant_id: TenantId,
) -> impl IntoResponse {
    info!(
        "🌊 Streaming handler called for agent_id: {}, execution_id: {}, tenant: {}",
        agent_id, execution_id, tenant_id.0
    );

    // Parse execution ID
    let execution_id_uuid = match Uuid::parse_str(&execution_id) {
        Ok(id) => id,
        Err(_) => {
            // Return an immediate error response for invalid UUID
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::empty())
                .unwrap()
                .into_response();
        }
    };

    // Skip storage validation for streaming - just stream directly
    // The execution_id and agent_id are validated by the URL pattern

    // Create a subscriber to the agent stream
    let stream = engine.subscribe_to_stream();

    // Transform the broadcast stream into an SSE stream
    let stream = BroadcastStream::new(stream).filter_map(move |msg| async move {
        match msg {
            Ok(event) => {
                // Only include events for the requested execution ID
                match event {
                    AgentStreamEvent::ThinkingStatus {
                        execution_id,
                        status,
                        context,
                    } if execution_id == execution_id_uuid => {
                        Some(Ok::<axum::response::sse::Event, axum::Error>(
                            Event::default().event("thinking").data(status),
                        ))
                    }
                    AgentStreamEvent::ContentChunk {
                        execution_id,
                        chunk,
                        sequence,
                        context,
                    } if execution_id == execution_id_uuid => Some(Ok(Event::default()
                        .event("chunk")
                        .data(chunk)
                        .id(sequence.to_string()))),
                    AgentStreamEvent::Completed {
                        execution_id,
                        final_response,
                        usage,
                        context,
                    } if execution_id == execution_id_uuid => Some(Ok(Event::default()
                        .event("complete")
                        .data(serde_json::to_string(&final_response).unwrap_or_default()))),
                    AgentStreamEvent::Failed {
                        execution_id,
                        error,
                        context,
                    } if execution_id == execution_id_uuid => {
                        Some(Ok(Event::default().event("error").data(error)))
                    }
                    _ => None, // Filter out unrelated events
                }
            }
            Err(_) => None,
        }
    });

    // Return the SSE stream
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

// Helper functions

/// Ensure tenant ID is included in the context
fn ensure_tenant_in_context(context: serde_json::Value, tenant_id: &str) -> serde_json::Value {
    // Check if tenant_id is already in the context
    if context.get("tenant_id").is_some() {
        return context;
    }

    // Add tenant ID to context
    let mut context_obj = context.as_object().cloned().unwrap_or_default();
    context_obj.insert(
        "tenant_id".to_string(),
        serde_json::Value::String(tenant_id.to_string()),
    );

    serde_json::Value::Object(context_obj)
}

// Router configuration

/// Create a router with all agent HTTP endpoints
pub fn routes(engine: Arc<AgentEngine>) -> Router {
    // Create WebSocket state
    let websocket_state = WebSocketState::new(engine.clone());
    let websocket_router_state = (engine.clone(), websocket_state.get_connection_manager());

    Router::new()
        .route("/test", get(|| async { "Test route works!" }))
        .route("/agents/:agent_id/execute", post(execute_agent))
        .route(
            "/agents/:agent_id/execute/stream",
            post(execute_agent_stream),
        )
        .route("/agents/:agent_id/executions", get(list_agent_executions))
        .route(
            "/agents/:agent_id/executions/:execution_id",
            get(get_execution_details),
        )
        .route(
            "/agents/:agent_id/executions/:execution_id/stream",
            get(stream_execution_events),
        )
        .with_state(engine)
        .layer(axum::middleware::from_fn(validate_tenant))
        .merge(
            Router::new()
                .route("/agents/ws", get(ws_handler))
                .with_state(websocket_router_state)
                .layer(axum::middleware::from_fn(validate_tenant)),
        )
}

/// Add agent HTTP routes to an existing router
pub fn add_routes_to_app(app: Router, engine: Arc<AgentEngine>) -> Router {
    app.merge(routes(engine))
}

/// Add agent HTTP routes with rate limiting to an existing router
pub fn add_routes_with_rate_limiting(
    app: Router,
    engine: Arc<AgentEngine>,
    rate_limiter: Arc<RateLimiter>,
) -> Router {
    let agent_routes = routes(engine).layer(axum::middleware::from_fn_with_state(
        rate_limiter,
        rate_limit,
    ));

    app.merge(agent_routes)
}
