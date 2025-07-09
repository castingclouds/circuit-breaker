// Integration tests for Agent WebSocket handlers
use crate::{
    api::agents::{
        middleware::TenantId,
        websocket_handlers::{ws_handler, WebSocketState},
    },
    engine::{AgentEngine, AgentEngineConfig, AgentStorage, InMemoryAgentStorage},
    models::{
        AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId, AgentStreamEvent,
        LLMConfig, LLMProvider,
    },
    CircuitBreakerError, Result,
};

use axum::{
    body::Body,
    extract::{State, WebSocketUpgrade},
    http::{Method, Request, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tower::ServiceExt;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

// Test helpers
async fn setup_test_environment() -> Arc<AgentEngine> {
    // Create in-memory storage
    let storage = Arc::new(InMemoryAgentStorage::new());

    // Add a test agent
    let agent = AgentDefinition {
        id: AgentId::from("test-agent"),
        name: "Test Agent".to_string(),
        description: Some("A test agent for WebSocket testing".to_string()),
        llm_provider: LLMProvider::OpenAI {
            model: "gpt-3.5-turbo".to_string(),
            api_key: "test-key".to_string(),
            organization_id: None,
            base_url: None,
        },
        llm_config: LLMConfig {
            temperature: 0.7,
            max_tokens: Some(1000),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: None,
        },
        system_prompt: "You are a test agent.".to_string(),
        prompts: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    storage.store_agent(&agent).await.unwrap();

    // Create agent engine
    let config = AgentEngineConfig {
        max_concurrent_executions: 10,
        stream_buffer_size: 100,
        connection_timeout: Duration::from_secs(10),
        execution_timeout: Duration::from_secs(30),
        cleanup_interval: Duration::from_secs(60),
    };

    Arc::new(AgentEngine::new(storage, config))
}

// Create a test server with WebSocket handler
fn create_test_app(engine: Arc<AgentEngine>) -> Router {
    let ws_state = WebSocketState::new(engine);

    Router::new()
        .route("/agents/ws", get(ws_handler))
        .with_state((ws_state.get_engine(), ws_state.get_connection_manager()))
        .layer(TraceLayer::new_for_http())
}

// WebSocket client for testing
struct TestWebSocketClient {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl TestWebSocketClient {
    async fn connect(url: &str) -> Self {
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        Self { ws_stream }
    }

    async fn send(&mut self, message: Value) {
        let message_str = serde_json::to_string(&message).expect("Failed to serialize message");
        self.ws_stream
            .send(WsMessage::Text(message_str))
            .await
            .expect("Failed to send message");
    }

    async fn receive(&mut self) -> Value {
        let message = self
            .ws_stream
            .next()
            .await
            .expect("No message received")
            .expect("WebSocket error");
        match message {
            WsMessage::Text(text) => serde_json::from_str(&text).expect("Failed to parse message"),
            _ => panic!("Expected text message"),
        }
    }

    async fn receive_timeout(&mut self, timeout: Duration) -> Option<Value> {
        let timeout_future = time::sleep(timeout);
        tokio::select! {
            _ = timeout_future => None,
            message_opt = self.ws_stream.next() => {
                match message_opt {
                    Some(Ok(WsMessage::Text(text))) => {
                        Some(serde_json::from_str(&text).expect("Failed to parse message"))
                    },
                    _ => None,
                }
            }
        }
    }

    async fn close(mut self) {
        let _ = self.ws_stream.close(None).await;
    }
}

// Setup test server
async fn setup_test_server() -> (String, Arc<AgentEngine>) {
    let engine = setup_test_environment().await;
    let app = create_test_app(engine.clone());

    // Start a local server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Return the WebSocket URL and engine
    (format!("ws://{}/agents/ws", addr), engine)
}

// Test cases
#[tokio::test]
async fn test_websocket_connection() {
    // Setup
    let (ws_url, _) = setup_test_server().await;

    // Connect to WebSocket server
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Should receive auth success message
    let response = client.receive().await;
    assert_eq!(response["type"], "auth_success");
    assert!(response["tenant_id"].is_string());

    client.close().await;
}

#[tokio::test]
async fn test_agent_execution() {
    // Setup
    let (ws_url, _) = setup_test_server().await;

    // Connect to WebSocket server
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Receive auth message
    let _ = client.receive().await;

    // Execute an agent
    client
        .send(json!({
            "type": "execute",
            "agent_id": "test-agent",
            "context": {
                "user_id": "test-user",
                "message": "Hello, agent!"
            },
            "input_mapping": {
                "content": "message"
            }
        }))
        .await;

    // Should receive execution started message
    let response = client.receive().await;
    assert_eq!(response["type"], "execution_started");
    assert_eq!(response["agent_id"], "test-agent");
    let execution_id = response["execution_id"].as_str().unwrap().to_string();

    // Should receive thinking status
    let response = client.receive().await;
    assert_eq!(response["type"], "thinking");
    assert_eq!(response["execution_id"], execution_id);

    // Should eventually receive completion
    let mut received_complete = false;
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if let Some(response) = client.receive_timeout(Duration::from_millis(500)).await {
            if response["type"] == "complete" {
                assert_eq!(response["execution_id"], execution_id);
                assert!(response["response"].is_object());
                received_complete = true;
                break;
            }
        }
    }

    assert!(received_complete, "Did not receive completion message");

    client.close().await;
}

#[tokio::test]
async fn test_subscription() {
    // Setup
    let (ws_url, engine) = setup_test_server().await;

    // Create an execution directly
    let context = json!({
        "tenant_id": "default",
        "test_data": "subscription-test"
    });

    let execution = AgentExecution::new(
        AgentId::from("test-agent"),
        context,
        json!({"message": "Subscription test"}),
    );

    let execution_id = execution.id;
    engine.storage.store_execution(&execution).await.unwrap();

    // Connect to WebSocket server
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Receive auth message
    let _ = client.receive().await;

    // Subscribe to the execution
    client
        .send(json!({
            "type": "subscribe",
            "execution_id": execution_id.to_string()
        }))
        .await;

    // Manually trigger an event for this execution
    let stream_sender = engine.subscribe_to_stream();
    let _ = stream_sender.send(AgentStreamEvent::ThinkingStatus {
        execution_id,
        status: "Test thinking status".to_string(),
        context: Some(json!({"tenant_id": "default"})),
    });

    // Should receive the thinking status
    let mut received_thinking = false;
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if let Some(response) = client.receive_timeout(Duration::from_millis(500)).await {
            if response["type"] == "thinking" {
                assert_eq!(response["execution_id"], execution_id.to_string());
                assert_eq!(response["status"], "Test thinking status");
                received_thinking = true;
                break;
            }
        }
    }

    assert!(received_thinking, "Did not receive thinking status message");

    // Unsubscribe from the execution
    client
        .send(json!({
            "type": "unsubscribe",
            "execution_id": execution_id.to_string()
        }))
        .await;

    // Send another event - should not be received
    let _ = stream_sender.send(AgentStreamEvent::ThinkingStatus {
        execution_id,
        status: "Another thinking status".to_string(),
        context: Some(json!({"tenant_id": "default"})),
    });

    // Should not receive the second thinking status
    let response = client.receive_timeout(Duration::from_secs(1)).await;
    assert!(response.is_none(), "Received message after unsubscribing");

    client.close().await;
}

#[tokio::test]
async fn test_multiple_concurrent_executions() {
    // Setup
    let (ws_url, _) = setup_test_server().await;

    // Connect to WebSocket server
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Receive auth message
    let _ = client.receive().await;

    // Execute first agent
    client
        .send(json!({
            "type": "execute",
            "agent_id": "test-agent",
            "context": {
                "user_id": "test-user",
                "message": "First execution"
            }
        }))
        .await;

    // Receive first execution started
    let response = client.receive().await;
    assert_eq!(response["type"], "execution_started");
    let first_execution_id = response["execution_id"].as_str().unwrap().to_string();

    // Execute second agent
    client
        .send(json!({
            "type": "execute",
            "agent_id": "test-agent",
            "context": {
                "user_id": "test-user",
                "message": "Second execution"
            }
        }))
        .await;

    // Receive second execution started
    let response = client.receive().await;
    assert_eq!(response["type"], "execution_started");
    let second_execution_id = response["execution_id"].as_str().unwrap().to_string();

    // Should be different execution IDs
    assert_ne!(first_execution_id, second_execution_id);

    // Collect messages for a short time
    let mut first_messages = 0;
    let mut second_messages = 0;

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if let Some(response) = client.receive_timeout(Duration::from_millis(500)).await {
            if response["execution_id"] == first_execution_id {
                first_messages += 1;
            } else if response["execution_id"] == second_execution_id {
                second_messages += 1;
            }
        } else {
            break;
        }
    }

    // Should have received messages for both executions
    assert!(
        first_messages > 0,
        "No messages received for first execution"
    );
    assert!(
        second_messages > 0,
        "No messages received for second execution"
    );

    client.close().await;
}

#[tokio::test]
async fn test_error_handling() {
    // Setup
    let (ws_url, _) = setup_test_server().await;

    // Connect to WebSocket server
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Receive auth message
    let _ = client.receive().await;

    // Try to execute a non-existent agent
    client
        .send(json!({
            "type": "execute",
            "agent_id": "non-existent-agent",
            "context": {
                "user_id": "test-user",
                "message": "Hello"
            }
        }))
        .await;

    // Should receive an error message
    let mut received_error = false;
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if let Some(response) = client.receive_timeout(Duration::from_millis(500)).await {
            if response["type"] == "error" {
                assert!(response["error"].as_str().unwrap().contains("not found"));
                received_error = true;
                break;
            }
        }
    }

    assert!(received_error, "Did not receive error message");

    // Try to subscribe to invalid execution ID
    client
        .send(json!({
            "type": "subscribe",
            "execution_id": "not-a-uuid"
        }))
        .await;

    // Should receive an error message
    let response = client.receive().await;
    assert_eq!(response["type"], "error");
    assert!(response["error"]
        .as_str()
        .unwrap()
        .contains("Invalid execution ID"));

    client.close().await;
}

#[tokio::test]
async fn test_ping_pong() {
    // Setup
    let (ws_url, _) = setup_test_server().await;

    // Connect to WebSocket server
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Receive auth message
    let _ = client.receive().await;

    // Send ping
    client
        .send(json!({
            "type": "ping"
        }))
        .await;

    // Should receive pong
    let response = client.receive().await;
    assert_eq!(response["type"], "pong");
    assert!(response["timestamp"].is_string());

    client.close().await;
}

#[tokio::test]
async fn test_tenant_isolation() {
    // Setup
    let (ws_url, engine) = setup_test_server().await;

    // Create executions for different tenants
    let tenant1_context = json!({
        "tenant_id": "tenant1",
        "test_data": "tenant1-test"
    });

    let tenant2_context = json!({
        "tenant_id": "tenant2",
        "test_data": "tenant2-test"
    });

    let tenant1_execution = AgentExecution::new(
        AgentId::from("test-agent"),
        tenant1_context,
        json!({"message": "Tenant 1 test"}),
    );

    let tenant2_execution = AgentExecution::new(
        AgentId::from("test-agent"),
        tenant2_context,
        json!({"message": "Tenant 2 test"}),
    );

    let tenant1_execution_id = tenant1_execution.id;
    let tenant2_execution_id = tenant2_execution.id;

    engine
        .storage
        .store_execution(&tenant1_execution)
        .await
        .unwrap();
    engine
        .storage
        .store_execution(&tenant2_execution)
        .await
        .unwrap();

    // Connect as tenant1
    let mut client = TestWebSocketClient::connect(&ws_url).await;

    // Receive auth message
    let _ = client.receive().await;

    // Try to subscribe to tenant2's execution
    client
        .send(json!({
            "type": "subscribe",
            "execution_id": tenant2_execution_id.to_string()
        }))
        .await;

    // Should receive an error message due to tenant isolation
    let response = client.receive().await;
    assert_eq!(response["type"], "error");
    assert!(response["error"]
        .as_str()
        .unwrap()
        .contains("Access denied"));

    // Subscribe to tenant1's execution (should work)
    client
        .send(json!({
            "type": "subscribe",
            "execution_id": tenant1_execution_id.to_string()
        }))
        .await;

    // Trigger events for both executions
    let stream_sender = engine.subscribe_to_stream();

    let _ = stream_sender.send(AgentStreamEvent::ThinkingStatus {
        execution_id: tenant1_execution_id,
        status: "Tenant 1 thinking".to_string(),
        context: Some(json!({"tenant_id": "tenant1"})),
    });

    let _ = stream_sender.send(AgentStreamEvent::ThinkingStatus {
        execution_id: tenant2_execution_id,
        status: "Tenant 2 thinking".to_string(),
        context: Some(json!({"tenant_id": "tenant2"})),
    });

    // Should only receive tenant1's event
    let response = client.receive().await;
    assert_eq!(response["type"], "thinking");
    assert_eq!(response["execution_id"], tenant1_execution_id.to_string());
    assert_eq!(response["status"], "Tenant 1 thinking");

    // Should not receive tenant2's event
    let response = client.receive_timeout(Duration::from_secs(1)).await;
    assert!(response.is_none(), "Received message from another tenant");

    client.close().await;
}
