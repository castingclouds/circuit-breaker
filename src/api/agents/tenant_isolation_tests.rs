// Integration tests for tenant isolation functionality
use crate::{
    api::agents::tenant_isolation::{
        RateLimits, ResourceQuotas, TenantAwareAgentEngine, TenantAwareAgentEngineFactory,
        TenantConfig, TenantModelConfig, TenantUsageStats,
    },
    engine::{AgentEngine, AgentEngineConfig, AgentStorage, InMemoryAgentStorage},
    models::{
        AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
        LLMConfig, LLMProvider,
    },
    CircuitBreakerError, Result,
};

use serde_json::json;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time;

// Test helpers
async fn setup_test_environment() -> (Arc<AgentEngine>, TenantAwareAgentEngineFactory) {
    // Create in-memory storage
    let storage = Arc::new(InMemoryAgentStorage::new());

    // Add a test agent
    let agent = AgentDefinition {
        id: AgentId::from("test-agent"),
        name: "Test Agent".to_string(),
        description: Some("A test agent for tenant isolation testing".to_string()),
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

    let engine = Arc::new(AgentEngine::new(storage, config));

    // Create factory
    let factory = TenantAwareAgentEngineFactory::new(engine.clone());

    // Add test tenant configurations
    let tenant1_config = TenantConfig {
        tenant_id: "tenant1".to_string(),
        name: "Tenant 1".to_string(),
        active: true,
        quotas: ResourceQuotas {
            max_agents: 5,
            max_executions: 50,
            max_tokens_per_execution: 2000,
            max_storage_bytes: 1024 * 1024,
            max_execution_history_days: 10,
        },
        rate_limits: RateLimits {
            requests_per_minute: 10, // Set low for testing
            executions_per_hour: 50,
            tokens_per_day: 50000,
        },
        max_concurrent_executions: 5,
        default_model_config: Some(TenantModelConfig {
            default_model: "gpt-3.5-turbo".to_string(),
            default_temperature: 0.5,
            default_max_tokens: 1000,
        }),
        allowed_models: Some(vec!["gpt-3.5-turbo".to_string(), "gpt-4".to_string()]),
        metadata: json!({"plan": "premium"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let tenant2_config = TenantConfig {
        tenant_id: "tenant2".to_string(),
        name: "Tenant 2".to_string(),
        active: true,
        quotas: ResourceQuotas {
            max_agents: 3,
            max_executions: 30,
            max_tokens_per_execution: 1000,
            max_storage_bytes: 1024 * 512,
            max_execution_history_days: 5,
        },
        rate_limits: RateLimits {
            requests_per_minute: 5, // Set low for testing
            executions_per_hour: 20,
            tokens_per_day: 10000,
        },
        max_concurrent_executions: 2,
        default_model_config: Some(TenantModelConfig {
            default_model: "gpt-3.5-turbo".to_string(),
            default_temperature: 0.7,
            default_max_tokens: 500,
        }),
        allowed_models: Some(vec!["gpt-3.5-turbo".to_string()]),
        metadata: json!({"plan": "basic"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    factory.add_tenant_config(tenant1_config).await.unwrap();
    factory.add_tenant_config(tenant2_config).await.unwrap();

    (engine, factory)
}

// Integration tests
#[tokio::test]
async fn test_tenant_isolation_basic() {
    let (_, factory) = setup_test_environment().await;

    // Get tenant engines
    let tenant1_engine = factory.get_engine("tenant1").await.unwrap();
    let tenant2_engine = factory.get_engine("tenant2").await.unwrap();

    // Create an execution for tenant1
    let tenant1_context = json!({
        "message": "Hello from tenant1",
        "user_id": "user1"
    });

    let config = AgentActivityConfig {
        agent_id: AgentId::from("test-agent"),
        input_mapping: HashMap::new(),
        output_mapping: HashMap::new(),
    };

    let tenant1_execution = tenant1_engine
        .execute_agent(&config, tenant1_context)
        .await
        .unwrap();

    // Create an execution for tenant2
    let tenant2_context = json!({
        "message": "Hello from tenant2",
        "user_id": "user2"
    });

    let tenant2_execution = tenant2_engine
        .execute_agent(&config, tenant2_context)
        .await
        .unwrap();

    // Verify tenant IDs were set correctly in context
    assert_eq!(
        tenant1_execution
            .context
            .get("tenant_id")
            .and_then(|v| v.as_str()),
        Some("tenant1")
    );
    assert_eq!(
        tenant2_execution
            .context
            .get("tenant_id")
            .and_then(|v| v.as_str()),
        Some("tenant2")
    );

    // Verify tenant1 can only see its own executions
    let tenant1_executions = tenant1_engine
        .storage
        .list_executions_by_context("tenant_id", "tenant1")
        .await
        .unwrap();
    assert_eq!(tenant1_executions.len(), 1);
    assert_eq!(tenant1_executions[0].id, tenant1_execution.id);

    // Verify tenant2 can only see its own executions
    let tenant2_executions = tenant2_engine
        .storage
        .list_executions_by_context("tenant_id", "tenant2")
        .await
        .unwrap();
    assert_eq!(tenant2_executions.len(), 1);
    assert_eq!(tenant2_executions[0].id, tenant2_execution.id);

    // Verify tenant1 cannot access tenant2's execution
    let result = tenant1_engine
        .storage
        .get_execution(&tenant2_execution.id)
        .await;
    assert!(result.is_err());

    // Verify tenant2 cannot access tenant1's execution
    let result = tenant2_engine
        .storage
        .get_execution(&tenant1_execution.id)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tenant_rate_limiting() {
    let (_, factory) = setup_test_environment().await;

    // Get tenant engine with low rate limit (10 requests per minute)
    let tenant1_engine = factory.get_engine("tenant1").await.unwrap();

    let config = AgentActivityConfig {
        agent_id: AgentId::from("test-agent"),
        input_mapping: HashMap::new(),
        output_mapping: HashMap::new(),
    };

    // Execute requests up to the limit
    for i in 0..10 {
        let context = json!({
            "message": format!("Request {}", i),
            "user_id": "user1"
        });

        let result = tenant1_engine.execute_agent(&config, context).await;
        assert!(result.is_ok(), "Request {} should succeed", i);
    }

    // The next request should hit the rate limit
    let context = json!({
        "message": "This should be rate limited",
        "user_id": "user1"
    });

    let result = tenant1_engine.execute_agent(&config, context).await;
    assert!(result.is_err(), "Request should be rate limited");
    match result {
        Err(CircuitBreakerError::RateLimited(_)) => {
            // Expected error
        }
        _ => panic!("Expected RateLimited error"),
    }

    // But tenant2 should still be able to make requests
    let tenant2_engine = factory.get_engine("tenant2").await.unwrap();
    let context = json!({
        "message": "Tenant2 request",
        "user_id": "user2"
    });

    let result = tenant2_engine.execute_agent(&config, context).await;
    assert!(
        result.is_ok(),
        "Tenant2 should not be affected by tenant1's rate limit"
    );
}

#[tokio::test]
async fn test_tenant_model_configuration() {
    let (_, factory) = setup_test_environment().await;

    // Get tenant engines with different model configurations
    let tenant1_engine = factory.get_engine("tenant1").await.unwrap();
    let tenant2_engine = factory.get_engine("tenant2").await.unwrap();

    let config = AgentActivityConfig {
        agent_id: AgentId::from("test-agent"),
        input_mapping: HashMap::new(),
        output_mapping: HashMap::new(),
    };

    // Execute with empty context - should apply tenant defaults
    let tenant1_context = json!({
        "message": "Apply defaults"
    });

    let tenant1_execution = tenant1_engine
        .execute_agent(&config, tenant1_context)
        .await
        .unwrap();

    let tenant2_context = json!({
        "message": "Apply defaults"
    });

    let tenant2_execution = tenant2_engine
        .execute_agent(&config, tenant2_context)
        .await
        .unwrap();

    // Verify tenant1's model config
    if let Some(llm_config) = tenant1_execution.context.get("llm_config") {
        assert_eq!(
            llm_config.get("temperature").and_then(|t| t.as_f64()),
            Some(0.5)
        );
        assert_eq!(
            llm_config.get("max_tokens").and_then(|t| t.as_u64()),
            Some(1000)
        );
    } else {
        panic!("Expected llm_config in tenant1 context");
    }

    // Verify tenant2's model config
    if let Some(llm_config) = tenant2_execution.context.get("llm_config") {
        assert_eq!(
            llm_config.get("temperature").and_then(|t| t.as_f64()),
            Some(0.7)
        );
        assert_eq!(
            llm_config.get("max_tokens").and_then(|t| t.as_u64()),
            Some(500)
        );
    } else {
        panic!("Expected llm_config in tenant2 context");
    }

    // Override with custom model config
    let tenant1_context_override = json!({
        "message": "Override defaults",
        "llm_config": {
            "model": "gpt-4",
            "temperature": 0.9
        }
    });

    let tenant1_execution_override = tenant1_engine
        .execute_agent(&config, tenant1_context_override)
        .await
        .unwrap();

    // Verify custom config was preserved
    if let Some(llm_config) = tenant1_execution_override.context.get("llm_config") {
        assert_eq!(
            llm_config.get("model").and_then(|m| m.as_str()),
            Some("gpt-4")
        );
        assert_eq!(
            llm_config.get("temperature").and_then(|t| t.as_f64()),
            Some(0.9)
        );
    } else {
        panic!("Expected llm_config in tenant1 override context");
    }
}

#[tokio::test]
async fn test_tenant_concurrent_executions() {
    let (_, factory) = setup_test_environment().await;

    // Get tenant2 engine with max_concurrent_executions = 2
    let tenant2_engine = factory.get_engine("tenant2").await.unwrap();

    let config = AgentActivityConfig {
        agent_id: AgentId::from("test-agent"),
        input_mapping: HashMap::new(),
        output_mapping: HashMap::new(),
    };

    // Start two long-running executions (they won't actually run long in tests, but we're testing the concurrent limit)
    let mut handles = vec![];
    for i in 0..2 {
        let engine = tenant2_engine.clone();
        let handle = tokio::spawn(async move {
            let context = json!({
                "message": format!("Long running execution {}", i),
                "user_id": "user2"
            });

            engine.execute_agent(&config, context).await
        });
        handles.push(handle);
    }

    // Wait a moment for the executions to start
    time::sleep(Duration::from_millis(100)).await;

    // The third execution should fail due to concurrent limit
    let context = json!({
        "message": "This should exceed concurrent limit",
        "user_id": "user2"
    });

    let result = tenant2_engine.execute_agent(&config, context).await;
    assert!(result.is_err(), "Expected concurrent execution limit error");
    match result {
        Err(CircuitBreakerError::TooManyRequests(_)) => {
            // Expected error
        }
        _ => panic!("Expected TooManyRequests error"),
    }

    // But tenant1 with higher limit should still work
    let tenant1_engine = factory.get_engine("tenant1").await.unwrap();
    let context = json!({
        "message": "Tenant1 concurrent execution",
        "user_id": "user1"
    });

    let result = tenant1_engine.execute_agent(&config, context).await;
    assert!(
        result.is_ok(),
        "Tenant1 should not be affected by tenant2's concurrent limit"
    );

    // Wait for all handles to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn test_tenant_usage_stats() {
    let (_, factory) = setup_test_environment().await;

    // Get tenant engines
    let tenant1_engine = factory.get_engine("tenant1").await.unwrap();
    let tenant2_engine = factory.get_engine("tenant2").await.unwrap();

    let config = AgentActivityConfig {
        agent_id: AgentId::from("test-agent"),
        input_mapping: HashMap::new(),
        output_mapping: HashMap::new(),
    };

    // Create executions for tenant1
    for i in 0..3 {
        let context = json!({
            "message": format!("Tenant1 execution {}", i),
            "usage": {
                "total_tokens": 100
            }
        });

        tenant1_engine
            .execute_agent(&config, context)
            .await
            .unwrap();
    }

    // Create executions for tenant2
    for i in 0..2 {
        let context = json!({
            "message": format!("Tenant2 execution {}", i),
            "usage": {
                "total_tokens": 50
            }
        });

        tenant2_engine
            .execute_agent(&config, context)
            .await
            .unwrap();
    }

    // Get usage stats
    let tenant1_stats = tenant1_engine.get_usage_stats().await;
    let tenant2_stats = tenant2_engine.get_usage_stats().await;

    // Verify tenant1 stats
    assert_eq!(tenant1_stats.total_executions, 3);
    assert!(tenant1_stats.total_tokens >= 300); // At least 3 * 100

    // Verify tenant2 stats
    assert_eq!(tenant2_stats.total_executions, 2);
    assert!(tenant2_stats.total_tokens >= 100); // At least 2 * 50

    // Verify stats are isolated
    assert_ne!(
        tenant1_stats.total_executions,
        tenant2_stats.total_executions
    );
    assert_ne!(tenant1_stats.total_tokens, tenant2_stats.total_tokens);
}

#[tokio::test]
async fn test_tenant_factory_management() {
    let (_, factory) = setup_test_environment().await;

    // List tenant IDs
    let tenant_ids = factory.list_tenant_ids().await;
    assert!(tenant_ids.contains(&"tenant1".to_string()));
    assert!(tenant_ids.contains(&"tenant2".to_string()));

    // Add a new tenant
    let new_tenant_config = TenantConfig {
        tenant_id: "tenant3".to_string(),
        name: "Tenant 3".to_string(),
        active: true,
        quotas: ResourceQuotas::default(),
        rate_limits: RateLimits::default(),
        max_concurrent_executions: 5,
        default_model_config: None,
        allowed_models: None,
        metadata: json!({}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    factory.add_tenant_config(new_tenant_config).await.unwrap();

    // Verify new tenant was added
    let tenant_ids = factory.list_tenant_ids().await;
    assert!(tenant_ids.contains(&"tenant3".to_string()));

    // Get tenant config
    let config = factory.get_tenant_config("tenant3").await.unwrap();
    assert_eq!(config.name, "Tenant 3");

    // Remove tenant
    let removed = factory.remove_tenant_config("tenant3").await;
    assert!(removed);

    // Verify tenant was removed
    let tenant_ids = factory.list_tenant_ids().await;
    assert!(!tenant_ids.contains(&"tenant3".to_string()));
}
