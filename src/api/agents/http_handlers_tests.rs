// Integration tests for Agent HTTP handlers
use crate::{
    api::agents::{
        http_handlers::{
            execute_agent, get_execution_details, list_agent_executions, stream_execution_events,
            ExecuteAgentRequest,
        },
        middleware::TenantId,
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
    extract::{Path, Query, State},
    http::{Method, Request, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use futures::{Stream, StreamExt};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::broadcast;
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
        description: Some("A test agent for API testing".to_string()),
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

// Create a test server with our API routes
fn create_test_app(engine: Arc<AgentEngine>) -> Router {
    Router::new()
        .route("/agents/:agent_id/execute", post(execute_agent))
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
        .layer(TraceLayer::new_for_http())
}

// Create test executions for a specific tenant
async fn create_test_executions(
    engine: Arc<AgentEngine>,
    tenant_id: &str,
    count: usize,
) -> Vec<Uuid> {
    let mut execution_ids = Vec::with_capacity(count);

    for i in 0..count {
        let context = json!({
            "tenant_id": tenant_id,
            "test_data": format!("execution-{}", i),
            "custom": {
                "value": i
            }
        });

        let mut execution = AgentExecution::new(
            AgentId::from("test-agent"),
            context,
            json!({"message": format!("Test execution {}", i)}),
        );

        // Randomly set some as completed
        if i % 3 == 0 {
            execution.complete(json!({"result": format!("Test result {}", i)}));
        } else if i % 5 == 0 {
            execution.fail(format!("Test error {}", i));
        }

        engine.storage.store_execution(&execution).await.unwrap();
        execution_ids.push(execution.id);
    }

    execution_ids
}

// Extract JSON body from response
async fn json_body(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_execute_agent_endpoint() {
    // Setup
    let engine = setup_test_environment().await;
    let app = create_test_app(engine);

    // Create request data
    let request_data = json!({
        "context": {
            "user_id": "test-user",
            "message": "Hello, agent!"
        },
        "input_mapping": {
            "content": "message"
        }
    });

    // Create request with tenant header
    let request = Request::builder()
        .method(Method::POST)
        .uri("/agents/test-agent/execute")
        .header("Content-Type", "application/json")
        .header("x-tenant-id", "tenant1")
        .body(Body::from(request_data.to_string()))
        .unwrap();

    // Send the request
    let response = app.oneshot(request).await.unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    // Extract and verify response body
    let body = json_body(response).await;
    assert_eq!(body["agent_id"], "test-agent");
    assert_eq!(body["status"], "completed");
    assert!(body["execution_id"].is_string());
    assert!(body["context"]["tenant_id"].is_string());
    assert_eq!(body["context"]["tenant_id"], "tenant1");
}

#[tokio::test]
async fn test_list_agent_executions_endpoint() {
    // Setup
    let engine = setup_test_environment().await;

    // Create test executions for different tenants
    let tenant1_executions = create_test_executions(engine.clone(), "tenant1", 5).await;
    let tenant2_executions = create_test_executions(engine.clone(), "tenant2", 3).await;

    let app = create_test_app(engine);

    // Test listing executions for tenant1
    let request = Request::builder()
        .method(Method::GET)
        .uri("/agents/test-agent/executions")
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    // Extract and verify response body
    let body = json_body(response).await;
    assert_eq!(body["total"], 5);
    assert_eq!(body["executions"].as_array().unwrap().len(), 5);

    // Test listing executions for tenant2
    let request = Request::builder()
        .method(Method::GET)
        .uri("/agents/test-agent/executions")
        .header("x-tenant-id", "tenant2")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = json_body(response).await;

    // Verify tenant isolation
    assert_eq!(body["total"], 3);
    assert_eq!(body["executions"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_get_execution_details_endpoint() {
    // Setup
    let engine = setup_test_environment().await;

    // Create test executions for different tenants
    let tenant1_executions = create_test_executions(engine.clone(), "tenant1", 2).await;
    let tenant2_executions = create_test_executions(engine.clone(), "tenant2", 2).await;

    let app = create_test_app(engine);

    // Test accessing tenant1's execution
    let execution_id = tenant1_executions[0];
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/agents/test-agent/executions/{}", execution_id))
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    // Extract and verify response body
    let body = json_body(response).await;
    assert_eq!(body["execution_id"], execution_id.to_string());
    assert_eq!(body["agent_id"], "test-agent");

    // Test tenant isolation - tenant2 trying to access tenant1's execution
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/agents/test-agent/executions/{}", execution_id))
        .header("x-tenant-id", "tenant2")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify forbidden access
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_stream_execution_events_endpoint() {
    // Setup
    let engine = setup_test_environment().await;

    // Create a test execution
    let context = json!({
        "tenant_id": "tenant1",
        "test_data": "streaming-test"
    });

    let execution = AgentExecution::new(
        AgentId::from("test-agent"),
        context,
        json!({"message": "Streaming test"}),
    );

    let execution_id = execution.id;
    engine.storage.store_execution(&execution).await.unwrap();

    let app = create_test_app(engine.clone());

    // This is a basic test setup - in a real test, we would need to mock the SSE stream
    // or use a real SSE client to verify the streaming functionality

    // Test accessing the stream endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/agents/test-agent/executions/{}/stream",
            execution_id
        ))
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify response - just check that we get a 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Test tenant isolation
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/agents/test-agent/executions/{}/stream",
            execution_id
        ))
        .header("x-tenant-id", "tenant2")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify forbidden access
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_invalid_agent_id() {
    // Setup
    let engine = setup_test_environment().await;
    let app = create_test_app(engine);

    // Try to execute a non-existent agent
    let request_data = json!({
        "context": {
            "user_id": "test-user",
            "message": "Hello, agent!"
        }
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/agents/non-existent-agent/execute")
        .header("Content-Type", "application/json")
        .header("x-tenant-id", "tenant1")
        .body(Body::from(request_data.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify error response
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = json_body(response).await;
    assert!(body["error"].is_string());
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_invalid_execution_id() {
    // Setup
    let engine = setup_test_environment().await;
    let app = create_test_app(engine);

    // Try to get details of a non-existent execution
    let invalid_id = Uuid::new_v4();
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/agents/test-agent/executions/{}", invalid_id))
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify error response
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_malformed_execution_id() {
    // Setup
    let engine = setup_test_environment().await;
    let app = create_test_app(engine);

    // Try to get details with a malformed UUID
    let request = Request::builder()
        .method(Method::GET)
        .uri("/agents/test-agent/executions/not-a-uuid")
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify error response
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_missing_tenant_id() {
    // Setup
    let engine = setup_test_environment().await;
    let app = create_test_app(engine);

    // Create request without tenant header
    let request_data = json!({
        "context": {
            "user_id": "test-user",
            "message": "Hello, agent!"
        }
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/agents/test-agent/execute")
        .header("Content-Type", "application/json")
        // No tenant ID header
        .body(Body::from(request_data.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // In our implementation, missing tenant ID will use "default"
    assert_eq!(response.status(), StatusCode::OK);

    let body = json_body(response).await;
    assert_eq!(body["context"]["tenant_id"], "default");
}

#[tokio::test]
async fn test_pagination_and_filtering() {
    // Setup
    let engine = setup_test_environment().await;

    // Create many test executions
    let tenant1_executions = create_test_executions(engine.clone(), "tenant1", 25).await;

    let app = create_test_app(engine);

    // Test pagination - page 1 with limit 10
    let request = Request::builder()
        .method(Method::GET)
        .uri("/agents/test-agent/executions?limit=10&offset=0")
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = json_body(response).await;

    assert_eq!(body["total"], 25);
    assert_eq!(body["executions"].as_array().unwrap().len(), 10);
    assert_eq!(body["page_size"], 10);

    // Test pagination - page 2
    let request = Request::builder()
        .method(Method::GET)
        .uri("/agents/test-agent/executions?limit=10&offset=10")
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = json_body(response).await;

    assert_eq!(body["total"], 25);
    assert_eq!(body["executions"].as_array().unwrap().len(), 10);

    // Test filtering by status
    let request = Request::builder()
        .method(Method::GET)
        .uri("/agents/test-agent/executions?status=failed")
        .header("x-tenant-id", "tenant1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = json_body(response).await;

    // We should have some failed executions (those with i % 5 == 0)
    assert!(body["total"].as_u64().unwrap() > 0);
    assert!(body["total"].as_u64().unwrap() < 25);
}
