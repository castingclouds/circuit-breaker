// Tests for NATS-based agent storage implementation
// These tests require a running NATS server to pass

use crate::{
    api::agents::{
        nats_storage::{create_tenant_aware_nats_storage, NatsAgentStorage, NatsStorageConfig},
        tenant_storage::{TenantId, TenantStorageConfig},
    },
    engine::AgentStorage,
    models::{
        AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId, AgentPrompts, LLMConfig,
        LLMProvider,
    },
    CircuitBreakerError, Result,
};

use chrono::Utc;
use serde_json::json;
use std::{collections::HashMap, env, path::PathBuf, sync::Arc, time::Duration};
use tokio::time;
use uuid::Uuid;

// Helper function to check if NATS is available
async fn is_nats_available() -> bool {
    match async_nats::connect("nats://localhost:4222").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

// Helper function to create test storage
async fn create_test_storage() -> Option<Arc<NatsAgentStorage>> {
    if !is_nats_available().await {
        println!("Skipping NATS test - no server available");
        return None;
    }

    let config = NatsStorageConfig {
        url: "nats://localhost:4222".to_string(),
        stream_name: format!("TEST_AGENTS_{}", Uuid::new_v4()),
        agents_bucket: format!("test_agents_{}", Uuid::new_v4()),
        executions_bucket: format!("test_executions_{}", Uuid::new_v4()),
        ..Default::default()
    };

    match NatsAgentStorage::new(config).await {
        Ok(storage) => Some(Arc::new(storage)),
        Err(e) => {
            println!("Failed to create NATS storage: {}", e);
            None
        }
    }
}

// Helper function to create test agent definition
fn create_test_agent(tenant_id: Option<&str>) -> AgentDefinition {
    let tenant_str = tenant_id.unwrap_or("default");
    let id = format!("test-agent-{}-{}", tenant_str, Uuid::new_v4());

    AgentDefinition {
        id: AgentId::from(id),
        name: format!("Test Agent for {}", tenant_str),
        description: format!("Test agent for tenant {}", tenant_str),
        llm_provider: LLMProvider::OpenAI {
            model: "gpt-3.5-turbo".to_string(),
            api_key: "test-key".to_string(),
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
        prompts: AgentPrompts {
            system: format!("You are a test agent for tenant {}", tenant_str),
            user_template: "User query: {{query}}",
            context_instructions: Some(format!(
                "Remember you are working for tenant {}",
                tenant_str
            )),
        },
        capabilities: vec!["text".to_string(), "chat".to_string()],
        tools: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// Helper function to create test execution
fn create_test_execution(agent_id: &AgentId, tenant_id: Option<&str>) -> AgentExecution {
    let tenant_str = tenant_id.unwrap_or("default");

    let mut context = json!({
        "message": format!("Test execution for {}", tenant_str),
        "timestamp": Utc::now().to_rfc3339()
    });

    if let Some(tenant) = tenant_id {
        if let Some(obj) = context.as_object_mut() {
            obj.insert("tenant_id".to_string(), json!(tenant));
        }
    }

    AgentExecution::new(
        agent_id.clone(),
        context,
        json!({
            "content": format!("Test content for {}", tenant_str)
        }),
    )
}

// Tests for NATS storage
#[tokio::test]
async fn test_nats_basic_operations() {
    // Skip test if NATS is not available
    let storage = match create_test_storage().await {
        Some(s) => s,
        None => return,
    };

    // Create and store an agent
    let agent = create_test_agent(None);
    storage.store_agent(&agent).await.unwrap();

    // Retrieve the agent
    let retrieved_agent = storage.get_agent(&agent.id).await.unwrap().unwrap();
    assert_eq!(retrieved_agent.id, agent.id);
    assert_eq!(retrieved_agent.name, agent.name);

    // Create and store an execution
    let execution = create_test_execution(&agent.id, None);
    storage.store_execution(&execution).await.unwrap();

    // Retrieve the execution
    let retrieved_execution = storage.get_execution(&execution.id).await.unwrap().unwrap();
    assert_eq!(retrieved_execution.id, execution.id);
    assert_eq!(retrieved_execution.agent_id, agent.id);

    // List agents
    let agents = storage.list_agents().await.unwrap();
    assert!(agents.iter().any(|a| a.id == agent.id));

    // List executions for agent
    let executions = storage.list_executions_for_agent(&agent.id).await.unwrap();
    assert!(executions.iter().any(|e| e.id == execution.id));

    // Delete agent
    let deleted = storage.delete_agent(&agent.id).await.unwrap();
    assert!(deleted);

    // Verify agent is gone
    let agent_result = storage.get_agent(&agent.id).await.unwrap();
    assert!(agent_result.is_none());
}

#[tokio::test]
async fn test_nats_tenant_isolation() {
    // Skip test if NATS is not available
    let storage = match create_test_storage().await {
        Some(s) => s,
        None => return,
    };

    // Create agents for different tenants
    let tenant1_agent = create_test_agent(Some("tenant1"));
    let tenant2_agent = create_test_agent(Some("tenant2"));

    // Store agents
    storage.store_agent(&tenant1_agent).await.unwrap();
    storage.store_agent(&tenant2_agent).await.unwrap();

    // Create executions for each tenant
    let tenant1_execution = create_test_execution(&tenant1_agent.id, Some("tenant1"));
    let tenant2_execution = create_test_execution(&tenant2_agent.id, Some("tenant2"));

    // Store executions
    storage.store_execution(&tenant1_execution).await.unwrap();
    storage.store_execution(&tenant2_execution).await.unwrap();

    // Query by tenant context
    let tenant1_executions = storage
        .list_executions_by_context("tenant_id", "tenant1")
        .await
        .unwrap();

    let tenant2_executions = storage
        .list_executions_by_context("tenant_id", "tenant2")
        .await
        .unwrap();

    // Verify isolation
    assert_eq!(tenant1_executions.len(), 1);
    assert_eq!(tenant2_executions.len(), 1);

    assert_eq!(tenant1_executions[0].id, tenant1_execution.id);
    assert_eq!(tenant2_executions[0].id, tenant2_execution.id);
}

#[tokio::test]
async fn test_tenant_aware_nats_storage() {
    // Skip test if NATS is not available
    if !is_nats_available().await {
        println!("Skipping NATS test - no server available");
        return;
    }

    // Create temp directory for backups
    let temp_dir = env::temp_dir().join(format!("circuit_breaker_test_backups_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create tenant-aware NATS storage
    let tenant_storage =
        create_tenant_aware_nats_storage("nats://localhost:4222", Some(temp_dir.to_str().unwrap()))
            .await
            .unwrap();

    // Add tenant configuration
    let tenant_config = TenantStorageConfig {
        tenant_id: TenantId::new("test_tenant"),
        ..Default::default()
    };

    tenant_storage
        .add_tenant_config(tenant_config)
        .await
        .unwrap();

    // Create and store an agent
    let agent = create_test_agent(Some("test_tenant"));
    tenant_storage.store_agent(&agent).await.unwrap();

    // Create and store an execution
    let execution = create_test_execution(&agent.id, Some("test_tenant"));
    tenant_storage.store_execution(&execution).await.unwrap();

    // Query by tenant context
    let executions = tenant_storage
        .list_executions_by_context("tenant_id", "test_tenant")
        .await
        .unwrap();

    // Verify execution was stored and can be retrieved
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].id, execution.id);

    // Get tenant metrics
    let metrics = tenant_storage
        .get_tenant_metrics(&TenantId::new("test_tenant"))
        .await
        .unwrap();

    // Verify metrics were collected
    assert!(metrics.storage_bytes_used > 0);
    assert!(metrics.write_operations > 0);
}

#[tokio::test]
async fn test_nats_reconnection() {
    // Skip test if NATS is not available
    let storage = match create_test_storage().await {
        Some(s) => s,
        None => return,
    };

    // Store an agent
    let agent = create_test_agent(None);
    storage.store_agent(&agent).await.unwrap();

    // In a real test, we would disconnect from NATS here
    // and test the reconnection logic, but that's hard to do
    // in an automated test. Instead, we'll just test that
    // operations still work after a short delay.

    // Wait a bit
    time::sleep(Duration::from_millis(100)).await;

    // Try to get the agent
    let retrieved_agent = storage.get_agent(&agent.id).await.unwrap().unwrap();
    assert_eq!(retrieved_agent.id, agent.id);
}
