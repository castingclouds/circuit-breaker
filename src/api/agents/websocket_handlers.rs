// WebSocket streaming support for Agent API
// This module provides WebSocket handlers for real-time agent execution streaming

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    sync::{broadcast, mpsc, RwLock as TokioRwLock},
    task::JoinHandle,
    time,
};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::agents::middleware::TenantId;
use crate::engine::AgentEngine;
use crate::models::{
    AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentStreamEvent, LLMConfig, LLMProvider,
};
use crate::{CircuitBreakerError, Result};

// Message types for WebSocket communication

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "auth")]
    Authenticate { token: String },

    #[serde(rename = "subscribe")]
    Subscribe { execution_id: String },

    #[serde(rename = "unsubscribe")]
    Unsubscribe { execution_id: String },

    #[serde(rename = "execute")]
    ExecuteAgent {
        agent_id: String,
        context: serde_json::Value,
        input_mapping: Option<HashMap<String, String>>,
        output_mapping: Option<HashMap<String, String>>,
    },

    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "auth_success")]
    AuthSuccess { tenant_id: String },

    #[serde(rename = "auth_failure")]
    AuthFailure { error: String },

    #[serde(rename = "execution_started")]
    ExecutionStarted {
        execution_id: String,
        agent_id: String,
        timestamp: String,
    },

    #[serde(rename = "thinking")]
    Thinking {
        execution_id: String,
        status: String,
        timestamp: String,
    },

    #[serde(rename = "chunk")]
    ContentChunk {
        execution_id: String,
        chunk: String,
        sequence: u32,
        timestamp: String,
    },

    #[serde(rename = "complete")]
    Complete {
        execution_id: String,
        response: serde_json::Value,
        usage: Option<serde_json::Value>,
        timestamp: String,
    },

    #[serde(rename = "error")]
    Error {
        execution_id: Option<String>,
        error: String,
        timestamp: String,
    },

    #[serde(rename = "pong")]
    Pong { timestamp: String },
}

// Connection management

/// Represents a single WebSocket connection with associated state
struct Connection {
    tenant_id: String,
    subscriptions: HashSet<Uuid>,
    last_activity: Instant,
    tx: mpsc::Sender<ServerMessage>,
}

impl Connection {
    fn new(tenant_id: String, tx: mpsc::Sender<ServerMessage>) -> Self {
        Self {
            tenant_id,
            subscriptions: HashSet::new(),
            last_activity: Instant::now(),
            tx,
        }
    }

    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    fn is_stale(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    async fn send_message(&self, message: ServerMessage) -> Result<()> {
        self.tx.send(message).await.map_err(|e| {
            CircuitBreakerError::Internal(format!("Failed to send WebSocket message: {}", e))
        })
    }
}

/// Manages all active WebSocket connections
#[derive(Clone)]
struct ConnectionManager {
    connections: Arc<TokioRwLock<HashMap<Uuid, Connection>>>,
    timeout: Duration,
}

impl ConnectionManager {
    fn new(timeout: Duration) -> Self {
        Self {
            connections: Arc::new(TokioRwLock::new(HashMap::new())),
            timeout,
        }
    }

    async fn add_connection(&self, tenant_id: String, tx: mpsc::Sender<ServerMessage>) -> Uuid {
        let connection_id = Uuid::new_v4();
        let connection = Connection::new(tenant_id, tx);

        let mut connections = self.connections.write().await;
        connections.insert(connection_id, connection);

        connection_id
    }

    async fn remove_connection(&self, connection_id: &Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
    }

    async fn update_activity(&self, connection_id: &Uuid) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(connection_id) {
            connection.update_activity();
            Ok(())
        } else {
            Err(CircuitBreakerError::NotFound(format!(
                "Connection {} not found",
                connection_id
            )))
        }
    }

    async fn subscribe(&self, connection_id: &Uuid, execution_id: Uuid) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(connection_id) {
            connection.subscriptions.insert(execution_id);
            Ok(())
        } else {
            Err(CircuitBreakerError::NotFound(format!(
                "Connection {} not found",
                connection_id
            )))
        }
    }

    async fn unsubscribe(&self, connection_id: &Uuid, execution_id: Uuid) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(connection_id) {
            connection.subscriptions.remove(&execution_id);
            Ok(())
        } else {
            Err(CircuitBreakerError::NotFound(format!(
                "Connection {} not found",
                connection_id
            )))
        }
    }

    async fn get_tenant_id(&self, connection_id: &Uuid) -> Option<String> {
        let connections = self.connections.read().await;

        connections
            .get(connection_id)
            .map(|conn| conn.tenant_id.clone())
    }

    async fn is_subscribed(&self, connection_id: &Uuid, execution_id: &Uuid) -> bool {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            connection.subscriptions.contains(execution_id)
        } else {
            false
        }
    }

    async fn send_to_connection(&self, connection_id: &Uuid, message: ServerMessage) -> Result<()> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            connection.send_message(message).await
        } else {
            Err(CircuitBreakerError::NotFound(format!(
                "Connection {} not found",
                connection_id
            )))
        }
    }

    async fn send_to_subscribers(&self, execution_id: &Uuid, message: ServerMessage) -> Result<()> {
        let connections = self.connections.read().await;

        for (conn_id, connection) in connections.iter() {
            if connection.subscriptions.contains(execution_id) {
                // Clone the message for each subscriber
                let message_clone = serde_json::to_value(&message)
                    .and_then(|v| serde_json::from_value(v))
                    .map_err(|e| {
                        CircuitBreakerError::Internal(format!("Failed to clone message: {}", e))
                    })?;

                // Send in a way that doesn't block other subscribers if one connection is slow
                let _ = connection.send_message(message_clone).await;
            }
        }

        Ok(())
    }

    // Background task to clean up stale connections
    fn start_cleanup_task(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;
                self.cleanup_stale_connections().await;
            }
        })
    }

    async fn cleanup_stale_connections(&self) {
        let mut to_remove = Vec::new();

        // Find stale connections
        {
            let connections = self.connections.read().await;

            for (id, connection) in connections.iter() {
                if connection.is_stale(self.timeout) {
                    to_remove.push(*id);
                }
            }
        }

        // Remove stale connections
        if !to_remove.is_empty() {
            let mut connections = self.connections.write().await;

            for id in to_remove.iter() {
                connections.remove(id);
                info!("Removed stale WebSocket connection: {}", id);
            }

            info!("Cleaned up {} stale connections", to_remove.len());
        }
    }
}

// WebSocket handler for agent streaming

/// Handler function for WebSocket upgrade requests
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State((engine, connection_manager)): State<(Arc<AgentEngine>, ConnectionManager)>,
    tenant_id: TenantId,
) -> impl IntoResponse {
    // Upgrade the connection to WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, engine, connection_manager, tenant_id.0))
}

/// Process an individual WebSocket connection
async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    engine: Arc<AgentEngine>,
    connection_manager: ConnectionManager,
    tenant_id: String,
) {
    // Split the socket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a channel for sending messages to the WebSocket
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Register the connection
    let connection_id = connection_manager
        .add_connection(tenant_id.clone(), tx)
        .await;
    info!("New WebSocket connection established: {}", connection_id);

    // Send initial auth success message
    let auth_message = ServerMessage::AuthSuccess {
        tenant_id: tenant_id.clone(),
    };

    if let Err(e) = connection_manager
        .send_to_connection(&connection_id, auth_message)
        .await
    {
        error!("Failed to send auth message: {}", e);
        return;
    }

    // Spawn a task to forward messages from the channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let json = serde_json::to_string(&message).unwrap_or_else(|e| {
                error!("Failed to serialize message: {}", e);
                String::from(r#"{"type":"error","error":"Failed to serialize message"}"#)
            });

            if let Err(e) = ws_sender.send(axum::extract::ws::Message::Text(json)).await {
                error!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
    });

    // Subscribe to the agent stream
    let mut stream = engine.subscribe_to_stream();

    // Spawn a task to forward events from the agent stream to subscribers
    let cm = connection_manager.clone();
    let conn_id = connection_id;
    let mut stream_task = tokio::spawn(async move {
        let mut stream = BroadcastStream::new(stream);

        while let Some(Ok(event)) = stream.next().await {
            match event {
                AgentStreamEvent::ThinkingStatus {
                    execution_id,
                    status,
                    context,
                } => {
                    // Check if this connection is subscribed to this execution
                    if !cm.is_subscribed(&conn_id, &execution_id).await {
                        continue;
                    }

                    // Forward to the client
                    let message = ServerMessage::Thinking {
                        execution_id: execution_id.to_string(),
                        status,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = cm.send_to_connection(&conn_id, message).await;
                }
                AgentStreamEvent::ContentChunk {
                    execution_id,
                    chunk,
                    sequence,
                    context,
                } => {
                    if !cm.is_subscribed(&conn_id, &execution_id).await {
                        continue;
                    }

                    let message = ServerMessage::ContentChunk {
                        execution_id: execution_id.to_string(),
                        chunk,
                        sequence,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = cm.send_to_connection(&conn_id, message).await;
                }
                AgentStreamEvent::Completed {
                    execution_id,
                    final_response,
                    usage,
                    context,
                } => {
                    if !cm.is_subscribed(&conn_id, &execution_id).await {
                        continue;
                    }

                    let message = ServerMessage::Complete {
                        execution_id: execution_id.to_string(),
                        response: final_response,
                        usage,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = cm.send_to_connection(&conn_id, message).await;
                }
                AgentStreamEvent::Failed {
                    execution_id,
                    error,
                    context,
                } => {
                    if !cm.is_subscribed(&conn_id, &execution_id).await {
                        continue;
                    }

                    let message = ServerMessage::Error {
                        execution_id: Some(execution_id.to_string()),
                        error,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = cm.send_to_connection(&conn_id, message).await;
                }
                _ => {
                    // Ignore other event types
                }
            }
        }
    });

    // Process incoming WebSocket messages
    while let Some(Ok(ws_message)) = ws_receiver.next().await {
        // Update activity timestamp
        let _ = connection_manager.update_activity(&connection_id).await;

        match ws_message {
            axum::extract::ws::Message::Text(text) => {
                // Parse the client message
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_message) => {
                        process_client_message(
                            client_message,
                            &connection_id,
                            &connection_manager,
                            &engine,
                            &tenant_id,
                        )
                        .await;
                    }
                    Err(e) => {
                        // Send error for invalid message format
                        let error_message = ServerMessage::Error {
                            execution_id: None,
                            error: format!("Invalid message format: {}", e),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };

                        let _ = connection_manager
                            .send_to_connection(&connection_id, error_message)
                            .await;
                    }
                }
            }
            axum::extract::ws::Message::Ping(data) => {
                // Respond to ping with pong
                if let Err(e) = ws_sender.send(axum::extract::ws::Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
            axum::extract::ws::Message::Close(_) => {
                info!("WebSocket connection closed by client: {}", connection_id);
                break;
            }
            _ => {
                // Ignore other message types
            }
        }
    }

    // Clean up the connection
    connection_manager.remove_connection(&connection_id).await;
    info!("WebSocket connection terminated: {}", connection_id);

    // Abort the tasks
    send_task.abort();
    stream_task.abort();
}

/// Process a client message
async fn process_client_message(
    message: ClientMessage,
    connection_id: &Uuid,
    connection_manager: &ConnectionManager,
    engine: &Arc<AgentEngine>,
    tenant_id: &str,
) {
    match message {
        ClientMessage::Ping => {
            // Respond with a pong
            let pong = ServerMessage::Pong {
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let _ = connection_manager
                .send_to_connection(connection_id, pong)
                .await;
        }
        ClientMessage::Subscribe { execution_id } => {
            // Parse the execution ID
            match Uuid::parse_str(&execution_id) {
                Ok(execution_uuid) => {
                    // Verify the execution exists and belongs to this tenant
                    match engine.storage.get_execution(&execution_uuid).await {
                        Ok(Some(execution)) => {
                            // Check tenant ID in context
                            if let Some(exec_tenant) =
                                execution.context.get("tenant_id").and_then(|t| t.as_str())
                            {
                                if exec_tenant != tenant_id {
                                    let error = ServerMessage::Error {
                                        execution_id: Some(execution_id),
                                        error: "Access denied to this execution".to_string(),
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                    };

                                    let _ = connection_manager
                                        .send_to_connection(connection_id, error)
                                        .await;
                                    return;
                                }
                            }

                            // Subscribe to the execution
                            if let Err(e) = connection_manager
                                .subscribe(connection_id, execution_uuid)
                                .await
                            {
                                error!("Failed to subscribe to execution: {}", e);
                                let error = ServerMessage::Error {
                                    execution_id: Some(execution_id),
                                    error: format!("Failed to subscribe: {}", e),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                };

                                let _ = connection_manager
                                    .send_to_connection(connection_id, error)
                                    .await;
                            }
                        }
                        Ok(None) => {
                            let error = ServerMessage::Error {
                                execution_id: Some(execution_id),
                                error: "Execution not found".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };

                            let _ = connection_manager
                                .send_to_connection(connection_id, error)
                                .await;
                        }
                        Err(e) => {
                            let error = ServerMessage::Error {
                                execution_id: Some(execution_id),
                                error: format!("Failed to retrieve execution: {}", e),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };

                            let _ = connection_manager
                                .send_to_connection(connection_id, error)
                                .await;
                        }
                    }
                }
                Err(_) => {
                    let error = ServerMessage::Error {
                        execution_id: Some(execution_id),
                        error: "Invalid execution ID format".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = connection_manager
                        .send_to_connection(connection_id, error)
                        .await;
                }
            }
        }
        ClientMessage::Unsubscribe { execution_id } => {
            // Parse the execution ID
            match Uuid::parse_str(&execution_id) {
                Ok(execution_uuid) => {
                    // Unsubscribe from the execution
                    if let Err(e) = connection_manager
                        .unsubscribe(connection_id, execution_uuid)
                        .await
                    {
                        error!("Failed to unsubscribe from execution: {}", e);
                    }
                }
                Err(_) => {
                    let error = ServerMessage::Error {
                        execution_id: Some(execution_id),
                        error: "Invalid execution ID format".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let _ = connection_manager
                        .send_to_connection(connection_id, error)
                        .await;
                }
            }
        }
        ClientMessage::ExecuteAgent {
            agent_id,
            context,
            input_mapping,
            output_mapping,
        } => {
            // Ensure context has tenant ID
            let mut context_obj = context.as_object().cloned().unwrap_or_default();
            context_obj.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.to_string()),
            );
            let context = serde_json::Value::Object(context_obj);

            // Create agent config
            let config = AgentActivityConfig {
                agent_id: AgentId::from(agent_id.clone()),
                input_mapping: input_mapping.unwrap_or_default(),
                output_mapping: output_mapping.unwrap_or_default(),
            };

            // Execute the agent
            let conn_id = *connection_id;
            let cm = connection_manager.clone();
            let engine_clone = engine.clone();

            tokio::spawn(async move {
                match engine_clone.execute_agent(&config, context).await {
                    Ok(execution) => {
                        let execution_id = execution.id;

                        // Auto-subscribe to this execution
                        if let Err(e) = cm.subscribe(&conn_id, execution_id).await {
                            error!("Failed to auto-subscribe to execution: {}", e);
                        }

                        // Send execution started event
                        let started = ServerMessage::ExecutionStarted {
                            execution_id: execution_id.to_string(),
                            agent_id,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };

                        let _ = cm.send_to_connection(&conn_id, started).await;
                    }
                    Err(e) => {
                        // Send error message
                        let error = ServerMessage::Error {
                            execution_id: None,
                            error: format!("Failed to execute agent: {}", e),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };

                        let _ = cm.send_to_connection(&conn_id, error).await;
                    }
                }
            });
        }
        ClientMessage::Authenticate { token } => {
            // In a real implementation, this would validate the token
            // For now, we just acknowledge it
            info!("Authentication attempt with token: {}", token);

            // We already sent the auth success message at connection time
        }
    }
}

// Shared state for WebSocket connections

#[derive(Clone)]
pub struct WebSocketState {
    engine: Arc<AgentEngine>,
    connection_manager: ConnectionManager,
}

impl WebSocketState {
    pub fn new(engine: Arc<AgentEngine>) -> Self {
        let connection_manager = ConnectionManager::new(Duration::from_secs(300)); // 5 minute timeout

        // Start the cleanup task
        connection_manager.clone().start_cleanup_task();

        Self {
            engine,
            connection_manager,
        }
    }

    pub fn get_engine(&self) -> Arc<AgentEngine> {
        self.engine.clone()
    }

    pub fn get_connection_manager(&self) -> ConnectionManager {
        self.connection_manager.clone()
    }
}

// Router configuration

pub fn routes(state: WebSocketState) -> Router {
    Router::new()
        .route("/agents/ws", get(ws_handler))
        .with_state((state.get_engine(), state.get_connection_manager()))
}

/// Add WebSocket routes to an existing router
pub fn add_routes_to_app(app: Router, state: WebSocketState) -> Router {
    app.merge(routes(state))
}
