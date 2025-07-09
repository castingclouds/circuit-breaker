// Tests for the agent API endpoints
use crate::{
    api::agents::{
        execute_agent, execute_agent_stream, get_execution, middleware::TenantId,
        ExecuteAgentRequest,
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
    extract::State,
    http::{Request, StatusCode},
    response::Response,
};
use futures::StreamExt;
use hyper::body::to_bytes;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tower::{Service, ServiceExt};
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
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
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

#[tokio::test]
async fn test_execute_agent_success() {
    // Setup
    let engine = setup_test_environment().await;

    // Create request
    let request = ExecuteAgentRequest {
        agent_id: "test-agent".to_string(),
        context: json!({
            "user_context": {
                "user_id": "test-user",
                "session_id": "test-session"
            },
            "custom_context": {
                "test_value": "hello world"
            }
        }),
        input_mapping: Some(HashMap::from([(
            "message".to_string(),
            "custom_context.test_value".to_string(),
        )])),
        output_mapping: Some(HashMap::new()),
    };

    // Execute request
    let response = execute_agent(State(engine), axum::Json(request))
        .await
        .into_response();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    // Extract body
    let body_bytes = to_bytes(response.into_body()).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Validate response structure
    assert!(body.get("execution_id").is_some());
    assert_eq!(
        body.get("agent_id").unwrap().as_str().unwrap(),
        "test-agent"
    );
    assert!(body.get("context").is_some());
}

#[tokio::test]
async fn test_execute_agent_invalid_agent() {
    // Setup
    let engine = setup_test_environment().await;

    // Create request with invalid agent ID
    let request = ExecuteAgentRequest {
        agent_id: "non-existent-agent".to_string(),
        context: json!({}),
        input_mapping: None,
        output_mapping: None,
    };

    // Execute request
    let response = execute_agent(State(engine), axum::Json(request))
        .await
        .into_response();

    // Assert
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Extract body
    let body_bytes = to_bytes(response.into_body()).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Validate response has error
    assert!(body.get("error").is_some());
}

#[tokio::test]
async fn test_get_execution() {
    // Setup
    let engine = setup_test_environment().await;

    // Create an execution to retrieve
    let execution = AgentExecution::new(
        AgentId::from("test-agent"),
        json!({
            "test_context": "value"
        }),
        json!({
            "input": "test input"
        }),
    );

    engine.storage.store_execution(&execution).await.unwrap();
    let execution_id = execution.id.to_string();

    // Execute request
    let response = get_execution(State(engine), axum::extract::Path(execution_id.clone()))
        .await
        .into_response();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    // Extract body
    let body_bytes = to_bytes(response.into_body()).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Validate response
    assert_eq!(
        body.get("execution_id").unwrap().as_str().unwrap(),
        execution_id
    );
    assert_eq!(
        body.get("agent_id").unwrap().as_str().unwrap(),
        "test-agent"
    );
}

#[tokio::test]
async fn test_get_execution_not_found() {
    // Setup
    let engine = setup_test_environment().await;

    // Use a random UUID that should not exist
    let random_id = Uuid::new_v4().to_string();

    // Execute request
    let response = get_execution(State(engine), axum::extract::Path(random_id))
        .await
        .into_response();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Note: Testing the streaming endpoint requires more complex setup with a proper
// HTTP client that can handle SSE events. This would typically be done with
// integration tests rather than unit tests.

#[tokio::test]
async fn test_tenant_isolation() {
    // Setup
    let engine = setup_test_environment().await;

    // Create requests for different tenants
    let tenant1_request = ExecuteAgentRequest {
        agent_id: "test-agent".to_string(),
        context: json!({
            "user_context": {
                "user_id": "user-tenant1",
                "session_id": "session-tenant1"
            },
            "custom_context": {
                "test_value": "tenant1 data"
            }
        }),
        input_mapping: Some(HashMap::from([(
            "message".to_string(),
            "custom_context.test_value".to_string(),
        )])),
        output_mapping: Some(HashMap::new()),
    };

    let tenant2_request = ExecuteAgentRequest {
        agent_id: "test-agent".to_string(),
        context: json!({
            "user_context": {
                "user_id": "user-tenant2",
                "session_id": "session-tenant2"
            },
            "custom_context": {
                "test_value": "tenant2 data"
            }
        }),
        input_mapping: Some(HashMap::from([(
            "message".to_string(),
            "custom_context.test_value".to_string(),
        )])),
        output_mapping: Some(HashMap::new()),
    };

    // Create tenant IDs
    let tenant1 = TenantId("tenant1".to_string());
    let tenant2 = TenantId("tenant2".to_string());

    // Execute requests with different tenant IDs
    let response1 = execute_agent(State(engine.clone()), tenant1, axum::Json(tenant1_request))
        .await
        .into_response();

    let response2 = execute_agent(State(engine.clone()), tenant2, axum::Json(tenant2_request))
        .await
        .into_response();

    // Assert both responses were successful
    assert_eq!(response1.status(), StatusCode::OK);
    assert_eq!(response2.status(), StatusCode::OK);

    // Extract bodies
    let body_bytes1 = to_bytes(response1.into_body()).await.unwrap();
    let body1: serde_json::Value = serde_json::from_slice(&body_bytes1).unwrap();

    let body_bytes2 = to_bytes(response2.into_body()).await.unwrap();
    let body2: serde_json::Value = serde_json::from_slice(&body_bytes2).unwrap();

    // Verify tenant IDs were added to context
    let context1 = body1.get("context").unwrap();
    let context2 = body2.get("context").unwrap();

    assert_eq!(
        context1.get("tenant_id").unwrap().as_str().unwrap(),
        "tenant1"
    );
    assert_eq!(
        context2.get("tenant_id").unwrap().as_str().unwrap(),
        "tenant2"
    );

    // Verify executions can be retrieved with tenant ID
    let execution_id1 = body1.get("execution_id").unwrap().as_str().unwrap();
    let response = get_execution(
        State(engine.clone()),
        tenant1,
        axum::extract::Path(execution_id1.to_string()),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
}
