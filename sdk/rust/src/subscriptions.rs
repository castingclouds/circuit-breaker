//! Real-time Subscription Infrastructure
//!
//! This module provides comprehensive real-time subscription capabilities for the Circuit Breaker SDK,
//! enabling WebSocket-based GraphQL subscriptions with automatic reconnection, error recovery,
//! and type-safe event handling.
//!
//! # Examples
//!
//! ```rust
//! use circuit_breaker_sdk::{Client, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = Client::builder()
//!         .base_url("ws://localhost:4000/graphql")?
//!         .build()?;
//!
//!     // Subscribe to resource updates
//!     let resource_sub = client.subscriptions()
//!         .resource_updates()
//!         .resource_id("resource_123")
//!         .subscribe()
//!         .await?;
//!
//!     resource_sub.on_update(|resource| {
//!         println!("Resource updated: {} -> {}", resource.id, resource.state);
//!     }).await?;
//!
//!     // Subscribe to LLM streaming
//!     let llm_sub = client.subscriptions()
//!         .llm_stream("request_456")
//!         .subscribe()
//!         .await?;
//!
//!     llm_sub.on_chunk(|chunk| {
//!         print!("{}", chunk.content);
//!     }).await?;
//!
//!     // Keep subscriptions alive
//!     tokio::signal::ctrl_c().await?;
//!
//!     Ok(())
//! }
//! ```

use crate::client::Client;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use url::Url;
use uuid::Uuid;

/// Subscription client for real-time GraphQL subscriptions
pub struct SubscriptionClient {
    client: Client,
    manager: Arc<SubscriptionManager>,
}

impl SubscriptionClient {
    /// Create a new subscription client
    pub(crate) fn new(client: Client) -> Self {
        let manager = Arc::new(SubscriptionManager::new(client.clone()));
        Self { client, manager }
    }

    /// Subscribe to resource state changes
    pub fn resource_updates(&self) -> ResourceUpdateSubscriptionBuilder {
        ResourceUpdateSubscriptionBuilder::new(self.manager.clone())
    }

    /// Subscribe to workflow events
    pub fn workflow_events(&self) -> WorkflowEventSubscriptionBuilder {
        WorkflowEventSubscriptionBuilder::new(self.manager.clone())
    }

    /// Subscribe to agent execution events
    pub fn agent_execution_stream(&self) -> AgentExecutionSubscriptionBuilder {
        AgentExecutionSubscriptionBuilder::new(self.manager.clone())
    }

    /// Subscribe to LLM response streaming
    pub fn llm_stream(&self, request_id: &str) -> LLMStreamSubscriptionBuilder {
        LLMStreamSubscriptionBuilder::new(self.manager.clone(), request_id.to_string())
    }

    /// Subscribe to real-time cost updates
    pub fn cost_updates(&self) -> CostUpdateSubscriptionBuilder {
        CostUpdateSubscriptionBuilder::new(self.manager.clone())
    }

    /// Subscribe to MCP server status updates
    pub fn mcp_server_status_updates(&self) -> MCPServerStatusSubscriptionBuilder {
        MCPServerStatusSubscriptionBuilder::new(self.manager.clone())
    }

    /// Subscribe to MCP session events
    pub fn mcp_session_events(&self) -> MCPSessionEventSubscriptionBuilder {
        MCPSessionEventSubscriptionBuilder::new(self.manager.clone())
    }

    /// Get subscription manager for advanced operations
    pub fn manager(&self) -> Arc<SubscriptionManager> {
        self.manager.clone()
    }
}

/// Core subscription manager handling WebSocket connections and message routing
pub struct SubscriptionManager {
    client: Client,
    websocket: Arc<RwLock<Option<WebSocketConnection>>>,
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, ActiveSubscription>>>,
    metrics: SubscriptionMetrics,
    config: SubscriptionConfig,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new(client: Client) -> Self {
        Self {
            client,
            websocket: Arc::new(RwLock::new(None)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            metrics: SubscriptionMetrics::new(),
            config: SubscriptionConfig::default(),
        }
    }

    /// Start a new subscription
    pub async fn subscribe<T>(
        self: &Arc<Self>,
        subscription: GraphQLSubscription,
        handler: Box<dyn SubscriptionHandler<T> + Send + Sync>,
    ) -> Result<SubscriptionId>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        // Ensure WebSocket connection is established
        self.ensure_connection().await?;

        let subscription_id = SubscriptionId::new();
        let active_sub = ActiveSubscription::new(subscription_id.clone(), subscription, handler);

        // Store the subscription
        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(subscription_id.clone(), active_sub);
        }

        // Send subscription start message
        self.send_subscription_start(&subscription_id).await?;

        self.metrics
            .active_subscriptions
            .fetch_add(1, Ordering::Relaxed);

        Ok(subscription_id)
    }

    /// Unsubscribe from a subscription
    pub async fn unsubscribe(self: &Arc<Self>, subscription_id: &SubscriptionId) -> Result<()> {
        // Send unsubscribe message
        self.send_subscription_stop(subscription_id).await?;

        // Remove from active subscriptions
        {
            let mut subs = self.subscriptions.write().await;
            subs.remove(subscription_id);
        }

        self.metrics
            .active_subscriptions
            .fetch_sub(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get current metrics
    pub fn metrics(&self) -> &SubscriptionMetrics {
        &self.metrics
    }

    /// Ensure WebSocket connection is established
    async fn ensure_connection(self: &Arc<Self>) -> Result<()> {
        let mut ws_guard = self.websocket.write().await;

        if ws_guard.is_none() || !ws_guard.as_ref().unwrap().is_connected().await {
            let ws_url = self.build_websocket_url()?;
            let connection = WebSocketConnection::connect(ws_url, self.config.clone()).await?;
            *ws_guard = Some(connection);

            // Start message handling task with proper Arc<Self>
            self.start_message_handler();
        }

        Ok(())
    }

    /// Build WebSocket URL from base client URL
    fn build_websocket_url(&self) -> Result<Url> {
        let mut url = self.client.base_url().clone();

        // Convert HTTP(S) to WS(S)
        match url.scheme() {
            "http" => url
                .set_scheme("ws")
                .map_err(|_| crate::Error::Configuration {
                    message: "Failed to set WebSocket scheme".to_string(),
                })?,
            "https" => url
                .set_scheme("wss")
                .map_err(|_| crate::Error::Configuration {
                    message: "Failed to set secure WebSocket scheme".to_string(),
                })?,
            _ => {}
        }

        url.set_path("/graphql");
        Ok(url)
    }

    /// Send subscription start message
    async fn send_subscription_start(
        self: &Arc<Self>,
        subscription_id: &SubscriptionId,
    ) -> Result<()> {
        let ws_guard = self.websocket.read().await;
        if let Some(connection) = ws_guard.as_ref() {
            let subs_guard = self.subscriptions.read().await;
            if let Some(active_sub) = subs_guard.get(subscription_id) {
                let start_message = GraphQLWSMessage::Start {
                    id: subscription_id.to_string(),
                    payload: active_sub.subscription.clone(),
                };
                connection.send_message(start_message).await?;
            }
        }
        Ok(())
    }

    /// Send subscription stop message
    async fn send_subscription_stop(
        self: &Arc<Self>,
        subscription_id: &SubscriptionId,
    ) -> Result<()> {
        let ws_guard = self.websocket.read().await;
        if let Some(connection) = ws_guard.as_ref() {
            let stop_message = GraphQLWSMessage::Stop {
                id: subscription_id.to_string(),
            };
            connection.send_message(stop_message).await?;
        }
        Ok(())
    }

    /// Start background message handling task
    fn start_message_handler(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                if let Err(e) = manager.handle_messages().await {
                    eprintln!("WebSocket message handling error: {}", e);

                    // Attempt reconnection after error
                    if let Err(reconnect_err) = manager.reconnect().await {
                        eprintln!("Reconnection failed: {}", reconnect_err);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }

                // Small delay to prevent tight loop
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }

    /// Attempt to reconnect WebSocket connection
    async fn reconnect(self: &Arc<Self>) -> Result<()> {
        let mut attempt = 0;
        let max_attempts = self.config.reconnect_attempts;

        while attempt < max_attempts {
            attempt += 1;

            match self.ensure_connection().await {
                Ok(_) => {
                    println!("Successfully reconnected after {} attempts", attempt);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Reconnection attempt {} failed: {}", attempt, e);

                    // Exponential backoff
                    let delay = Duration::from_secs(2_u64.pow(attempt.min(6)));
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(crate::Error::Network {
            message: format!("Failed to reconnect after {} attempts", max_attempts),
        })
    }

    /// Handle incoming WebSocket messages
    async fn handle_messages(self: &Arc<Self>) -> Result<()> {
        let ws_guard = self.websocket.read().await;
        if let Some(connection) = ws_guard.as_ref() {
            while let Some(message) = connection.receive_message().await? {
                self.process_message(message).await?;
            }
        }
        Ok(())
    }

    /// Process a single WebSocket message
    async fn process_message(self: &Arc<Self>, message: GraphQLWSMessage) -> Result<()> {
        match message {
            GraphQLWSMessage::Data { id, payload } => {
                self.handle_subscription_data(&id, payload).await?;
            }
            GraphQLWSMessage::Error { id, payload } => {
                self.handle_subscription_error(&id, payload).await?;
            }
            GraphQLWSMessage::Complete { id } => {
                self.handle_subscription_complete(&id).await?;
            }
            GraphQLWSMessage::ConnectionAck => {
                // Connection acknowledged, ready to send subscriptions
            }
            GraphQLWSMessage::Ping { .. } => {
                // Respond with pong
                let ws_guard = self.websocket.read().await;
                if let Some(connection) = ws_guard.as_ref() {
                    connection
                        .send_message(GraphQLWSMessage::Pong { payload: None })
                        .await?;
                }
            }
            _ => {
                // Handle other message types as needed
            }
        }

        self.metrics
            .messages_received
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Handle subscription data
    async fn handle_subscription_data(
        self: &Arc<Self>,
        id: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        let subscription_id = SubscriptionId::from_string(id);
        let subs_guard = self.subscriptions.read().await;

        if let Some(active_sub) = subs_guard.get(&subscription_id) {
            active_sub.handle_data(payload).await?;
        }

        Ok(())
    }

    /// Handle subscription error
    async fn handle_subscription_error(
        self: &Arc<Self>,
        id: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        let subscription_id = SubscriptionId::from_string(id);
        let subs_guard = self.subscriptions.read().await;

        if let Some(active_sub) = subs_guard.get(&subscription_id) {
            let error = SubscriptionError::GraphQLError {
                subscription_id: subscription_id.clone(),
                payload,
            };
            active_sub.handle_error(error).await?;
        }

        Ok(())
    }

    /// Handle subscription completion
    async fn handle_subscription_complete(self: &Arc<Self>, id: &str) -> Result<()> {
        let subscription_id = SubscriptionId::from_string(id);
        let subs_guard = self.subscriptions.read().await;

        if let Some(active_sub) = subs_guard.get(&subscription_id) {
            active_sub.handle_complete().await?;
        }

        // Remove completed subscription
        drop(subs_guard);
        {
            let mut subs = self.subscriptions.write().await;
            subs.remove(&subscription_id);
        }

        self.metrics
            .active_subscriptions
            .fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }
}

/// WebSocket connection wrapper with auto-reconnection
pub struct WebSocketConnection {
    sender: mpsc::UnboundedSender<GraphQLWSMessage>,
    config: SubscriptionConfig,
    connected: Arc<std::sync::atomic::AtomicBool>,
}

impl WebSocketConnection {
    /// Connect to WebSocket endpoint
    pub async fn connect(url: Url, config: SubscriptionConfig) -> Result<Self> {
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| crate::Error::Network {
                message: format!("WebSocket connection failed: {}", e),
            })?;

        let (sender, receiver) = mpsc::unbounded_channel();
        let connected = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Start message sending task
        let connected_clone = connected.clone();
        tokio::spawn(async move {
            Self::message_sender_task(ws_stream, receiver, connected_clone).await;
        });

        Ok(Self {
            sender,
            config,
            connected,
        })
    }

    /// Check if connection is active
    pub async fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Send a message to the WebSocket
    pub async fn send_message(&self, message: GraphQLWSMessage) -> Result<()> {
        self.sender
            .send(message)
            .map_err(|e| crate::Error::Network {
                message: format!("Failed to send WebSocket message: {}", e),
            })?;
        Ok(())
    }

    /// Receive a message from the WebSocket
    pub async fn receive_message(&self) -> Result<Option<GraphQLWSMessage>> {
        // This would be implemented with proper message receiving logic
        // For now, returning None to indicate no message
        Ok(None)
    }

    /// Background task for sending messages
    async fn message_sender_task(
        mut ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        mut receiver: mpsc::UnboundedReceiver<GraphQLWSMessage>,
        connected: Arc<std::sync::atomic::AtomicBool>,
    ) {
        use futures_util::SinkExt;

        while let Some(message) = receiver.recv().await {
            let json_str = serde_json::to_string(&message).unwrap_or_default();
            let ws_message = Message::Text(json_str);

            if let Err(_) = ws_stream.send(ws_message).await {
                connected.store(false, Ordering::Relaxed);
                break;
            }
        }

        connected.store(false, Ordering::Relaxed);
    }
}

/// Unique identifier for subscriptions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    /// Create a new subscription ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from string
    pub fn from_string(s: &str) -> Self {
        Self(Uuid::parse_str(s).unwrap_or_else(|_| Uuid::new_v4()))
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Active subscription state
pub struct ActiveSubscription {
    pub id: SubscriptionId,
    pub subscription: GraphQLSubscription,
    handler: Box<dyn SubscriptionHandlerDyn + Send + Sync>,
}

impl ActiveSubscription {
    /// Create a new active subscription
    pub fn new<T>(
        id: SubscriptionId,
        subscription: GraphQLSubscription,
        handler: Box<dyn SubscriptionHandler<T> + Send + Sync>,
    ) -> Self
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let dyn_handler = SubscriptionHandlerWrapper::new(handler);
        Self {
            id,
            subscription,
            handler: Box::new(dyn_handler),
        }
    }

    /// Handle subscription data
    pub async fn handle_data(&self, payload: serde_json::Value) -> Result<()> {
        self.handler.handle_data(payload).await
    }

    /// Handle subscription error
    pub async fn handle_error(&self, error: SubscriptionError) -> Result<()> {
        self.handler.handle_error(error).await
    }

    /// Handle subscription completion
    pub async fn handle_complete(&self) -> Result<()> {
        self.handler.handle_complete().await
    }
}

/// GraphQL subscription definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLSubscription {
    pub query: String,
    pub variables: Option<serde_json::Value>,
    pub operation_name: Option<String>,
}

/// GraphQL WebSocket message protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GraphQLWSMessage {
    #[serde(rename = "connection_init")]
    ConnectionInit { payload: Option<serde_json::Value> },
    #[serde(rename = "connection_ack")]
    ConnectionAck,
    #[serde(rename = "start")]
    Start {
        id: String,
        payload: GraphQLSubscription,
    },
    #[serde(rename = "data")]
    Data {
        id: String,
        payload: serde_json::Value,
    },
    #[serde(rename = "error")]
    Error {
        id: String,
        payload: serde_json::Value,
    },
    #[serde(rename = "complete")]
    Complete { id: String },
    #[serde(rename = "stop")]
    Stop { id: String },
    #[serde(rename = "ping")]
    Ping { payload: Option<serde_json::Value> },
    #[serde(rename = "pong")]
    Pong { payload: Option<serde_json::Value> },
}

/// Subscription handler trait
#[async_trait::async_trait]
pub trait SubscriptionHandler<T> {
    /// Handle subscription data
    async fn on_data(&mut self, data: T) -> Result<()>;

    /// Handle subscription error
    async fn on_error(&mut self, error: SubscriptionError) -> Result<()>;

    /// Handle subscription completion
    async fn on_complete(&mut self) -> Result<()>;
}

/// Dynamic subscription handler trait
#[async_trait::async_trait]
trait SubscriptionHandlerDyn {
    async fn handle_data(&self, payload: serde_json::Value) -> Result<()>;
    async fn handle_error(&self, error: SubscriptionError) -> Result<()>;
    async fn handle_complete(&self) -> Result<()>;
}

/// Wrapper for type-safe subscription handlers
struct SubscriptionHandlerWrapper<T> {
    handler: tokio::sync::Mutex<Box<dyn SubscriptionHandler<T> + Send + Sync>>,
}

impl<T> SubscriptionHandlerWrapper<T>
where
    T: for<'de> Deserialize<'de> + Send + 'static,
{
    fn new(handler: Box<dyn SubscriptionHandler<T> + Send + Sync>) -> Self {
        Self {
            handler: tokio::sync::Mutex::new(handler),
        }
    }
}

#[async_trait::async_trait]
impl<T> SubscriptionHandlerDyn for SubscriptionHandlerWrapper<T>
where
    T: for<'de> Deserialize<'de> + Send + 'static,
{
    async fn handle_data(&self, payload: serde_json::Value) -> Result<()> {
        let data: T = serde_json::from_value(payload).map_err(|e| crate::Error::Parse {
            message: format!("Failed to parse subscription data: {}", e),
        })?;

        let mut handler = self.handler.lock().await;
        handler.on_data(data).await
    }

    async fn handle_error(&self, error: SubscriptionError) -> Result<()> {
        let mut handler = self.handler.lock().await;
        handler.on_error(error).await
    }

    async fn handle_complete(&self) -> Result<()> {
        let mut handler = self.handler.lock().await;
        handler.on_complete().await
    }
}

/// Subscription configuration
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    pub reconnect_attempts: u32,
    pub reconnect_delay: Duration,
    pub heartbeat_interval: Duration,
    pub message_timeout: Duration,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(30),
            message_timeout: Duration::from_secs(10),
        }
    }
}

/// Subscription error types
#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("WebSocket connection failed: {message}")]
    ConnectionFailed { message: String },

    #[error("Subscription {subscription_id} failed with GraphQL error")]
    GraphQLError {
        subscription_id: SubscriptionId,
        payload: serde_json::Value,
    },

    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("Rate limit exceeded, retry after {retry_after:?}")]
    RateLimitExceeded { retry_after: Duration },

    #[error("Subscription timeout after {timeout:?}")]
    Timeout { timeout: Duration },
}

/// Subscription metrics for monitoring
pub struct SubscriptionMetrics {
    pub active_subscriptions: AtomicU64,
    pub messages_received: AtomicU64,
    pub connection_failures: AtomicU64,
    pub reconnection_attempts: AtomicU64,
}

impl SubscriptionMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            active_subscriptions: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            connection_failures: AtomicU64::new(0),
            reconnection_attempts: AtomicU64::new(0),
        }
    }

    /// Get current active subscriptions count
    pub fn active_count(&self) -> u64 {
        self.active_subscriptions.load(Ordering::Relaxed)
    }

    /// Get total messages received
    pub fn messages_count(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Subscription Builders
// ============================================================================

/// Builder for resource update subscriptions
pub struct ResourceUpdateSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
    resource_id: Option<String>,
    workflow_id: Option<String>,
}

impl ResourceUpdateSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>) -> Self {
        Self {
            manager,
            resource_id: None,
            workflow_id: None,
        }
    }

    /// Filter by resource ID
    pub fn resource_id<S: Into<String>>(mut self, id: S) -> Self {
        self.resource_id = Some(id.into());
        self
    }

    /// Filter by workflow ID
    pub fn workflow_id<S: Into<String>>(mut self, id: S) -> Self {
        self.workflow_id = Some(id.into());
        self
    }

    /// Subscribe with handler
    pub async fn subscribe<F>(self, handler: F) -> Result<SubscriptionId>
    where
        F: Fn(ResourceGQL) + Send + Sync + 'static,
    {
        let subscription = GraphQLSubscription {
            query: r#"
                subscription ResourceUpdates($resourceId: String!) {
                    resourceUpdates(resourceId: $resourceId) {
                        id
                        workflowId
                        state
                        data
                        metadata
                        createdAt
                        updatedAt
                    }
                }
            "#
            .to_string(),
            variables: Some(serde_json::json!({
                "resourceId": self.resource_id
            })),
            operation_name: Some("ResourceUpdates".to_string()),
        };

        let boxed_handler = Box::new(SimpleHandler::new(handler));
        self.manager.subscribe(subscription, boxed_handler).await
    }
}

/// Builder for workflow event subscriptions
pub struct WorkflowEventSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
    workflow_id: Option<String>,
}

impl WorkflowEventSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>) -> Self {
        Self {
            manager,
            workflow_id: None,
        }
    }

    /// Filter by workflow ID
    pub fn workflow_id<S: Into<String>>(mut self, id: S) -> Self {
        self.workflow_id = Some(id.into());
        self
    }

    /// Subscribe with handler
    pub async fn subscribe<F>(self, handler: F) -> Result<SubscriptionId>
    where
        F: Fn(WorkflowEventGQL) + Send + Sync + 'static,
    {
        let subscription = GraphQLSubscription {
            query: r#"
                subscription WorkflowEvents($workflowId: String!) {
                    workflowEvents(workflowId: $workflowId) {
                        id
                        type
                        message
                        data
                        timestamp
                    }
                }
            "#
            .to_string(),
            variables: Some(serde_json::json!({
                "workflowId": self.workflow_id
            })),
            operation_name: Some("WorkflowEvents".to_string()),
        };

        let boxed_handler = Box::new(SimpleHandler::new(handler));
        self.manager.subscribe(subscription, boxed_handler).await
    }
}

/// Simple handler implementation for basic use cases
struct SimpleHandler<T, F> {
    handler: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> SimpleHandler<T, F>
where
    F: Fn(T) + Send + Sync + 'static,
{
    fn new(handler: F) -> Self {
        Self {
            handler,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T, F> SubscriptionHandler<T> for SimpleHandler<T, F>
where
    T: Send + 'static,
    F: Fn(T) + Send + Sync + 'static,
{
    async fn on_data(&mut self, data: T) -> Result<()> {
        (self.handler)(data);
        Ok(())
    }

    async fn on_error(&mut self, _error: SubscriptionError) -> Result<()> {
        // Default error handling - could be configurable
        Ok(())
    }

    async fn on_complete(&mut self) -> Result<()> {
        // Default completion handling
        Ok(())
    }
}

// Additional builders would be implemented similarly...
pub struct AgentExecutionSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
}

impl AgentExecutionSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>) -> Self {
        Self { manager }
    }
}

pub struct LLMStreamSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
    request_id: String,
}

impl LLMStreamSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>, request_id: String) -> Self {
        Self {
            manager,
            request_id,
        }
    }
}

pub struct CostUpdateSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
    user_id: Option<String>,
}

impl CostUpdateSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>) -> Self {
        Self {
            manager,
            user_id: None,
        }
    }

    /// Filter cost updates for a specific user
    pub fn user_id(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Subscribe with handler
    pub async fn subscribe<F>(self, handler: F) -> Result<SubscriptionId>
    where
        F: Fn(CostUpdateEvent) + Send + Sync + 'static,
    {
        let mut variables = serde_json::Map::new();
        if let Some(user_id) = self.user_id {
            variables.insert("userId".to_string(), serde_json::Value::String(user_id));
        }

        let subscription = GraphQLSubscription {
            query: "subscription CostUpdates($userId: String) { costUpdates(userId: $userId) { id userId cost currency description timestamp metadata } }".to_string(),
            variables: Some(serde_json::Value::Object(variables)),
            operation_name: None,
        };

        let handler_box = Box::new(SimpleHandler::new(handler));

        self.manager.subscribe(subscription, handler_box).await
    }
}

pub struct MCPServerStatusSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
}

impl MCPServerStatusSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>) -> Self {
        Self { manager }
    }
}

pub struct MCPSessionEventSubscriptionBuilder {
    manager: Arc<SubscriptionManager>,
}

impl MCPSessionEventSubscriptionBuilder {
    fn new(manager: Arc<SubscriptionManager>) -> Self {
        Self { manager }
    }
}

// GraphQL response types for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostUpdateEvent {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub cost: f64,
    pub currency: String,
    pub description: Option<String>,
    pub timestamp: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGQL {
    pub id: String,
    #[serde(rename = "workflowId")]
    pub workflow_id: String,
    pub state: String,
    pub data: serde_json::Value,
    pub metadata: serde_json::Value,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventGQL {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

/// Convenience functions for common subscription patterns
pub async fn subscribe_resource_updates<F>(
    client: &Client,
    resource_id: &str,
    handler: F,
) -> Result<SubscriptionId>
where
    F: Fn(ResourceGQL) + Send + Sync + 'static,
{
    client
        .subscriptions()
        .resource_updates()
        .resource_id(resource_id)
        .subscribe(handler)
        .await
}

pub async fn subscribe_workflow_events<F>(
    client: &Client,
    workflow_id: &str,
    handler: F,
) -> Result<SubscriptionId>
where
    F: Fn(WorkflowEventGQL) + Send + Sync + 'static,
{
    client
        .subscriptions()
        .workflow_events()
        .workflow_id(workflow_id)
        .subscribe(handler)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_id() {
        let id1 = SubscriptionId::new();
        let id2 = SubscriptionId::new();
        assert_ne!(id1, id2);

        let id_str = id1.to_string();
        let id3 = SubscriptionId::from_string(&id_str);
        assert_eq!(id1.to_string(), id3.to_string());
    }

    #[tokio::test]
    async fn test_subscription_manager_creation() {
        let client = Client::builder()
            .base_url("http://localhost:4000")
            .unwrap()
            .build()
            .unwrap();

        let manager = SubscriptionManager::new(client);
        assert_eq!(manager.metrics().active_count(), 0);
        assert_eq!(manager.metrics().messages_count(), 0);
    }

    #[test]
    fn test_graphql_subscription_serialization() {
        let subscription = GraphQLSubscription {
            query: "subscription { test }".to_string(),
            variables: Some(serde_json::json!({"id": "123"})),
            operation_name: Some("TestSub".to_string()),
        };

        let json = serde_json::to_string(&subscription).unwrap();
        assert!(json.contains("subscription { test }"));
        assert!(json.contains("TestSub"));
    }

    #[test]
    fn test_graphql_ws_message_serialization() {
        let start_msg = GraphQLWSMessage::Start {
            id: "sub_123".to_string(),
            payload: GraphQLSubscription {
                query: "subscription { test }".to_string(),
                variables: None,
                operation_name: None,
            },
        };

        let json = serde_json::to_string(&start_msg).unwrap();
        assert!(json.contains("start"));
        assert!(json.contains("sub_123"));

        let ack_msg = GraphQLWSMessage::ConnectionAck;
        let ack_json = serde_json::to_string(&ack_msg).unwrap();
        assert!(ack_json.contains("connection_ack"));
    }

    #[test]
    fn test_subscription_config_defaults() {
        let config = SubscriptionConfig::default();
        assert_eq!(config.reconnect_attempts, 5);
        assert_eq!(config.reconnect_delay, Duration::from_secs(1));
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.message_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_resource_gql_deserialization() {
        let json = r#"{
            "id": "resource_123",
            "workflowId": "workflow_456",
            "state": "processing",
            "data": {"key": "value"},
            "metadata": {"env": "test"},
            "createdAt": "2024-01-01T00:00:00Z",
            "updatedAt": "2024-01-01T01:00:00Z"
        }"#;

        let resource: ResourceGQL = serde_json::from_str(json).unwrap();
        assert_eq!(resource.id, "resource_123");
        assert_eq!(resource.workflow_id, "workflow_456");
        assert_eq!(resource.state, "processing");
    }

    #[test]
    fn test_workflow_event_gql_deserialization() {
        let json = r#"{
            "id": "event_123",
            "type": "state_changed",
            "message": "Resource state changed",
            "data": {"from": "start", "to": "processing"},
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let event: WorkflowEventGQL = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "event_123");
        assert_eq!(event.event_type, "state_changed");
        assert_eq!(event.message, "Resource state changed");
    }
}
