// Agent execution engine for Places AI Agent functionality
// This module handles AI agent execution for workflow tokens

//! # Agent Execution Engine
//!
//! This module provides the core engine for executing AI agents within workflows.
//! It supports both transition-based agent execution and place-based agent execution.
//!
//! ## Features
//!
//! - **Places AI Agent**: Run agents on tokens in specific places
//! - **Transition Agent**: Run agents during workflow transitions
//! - **Real-time Streaming**: Stream agent responses via multiple protocols
//! - **LLM Provider Support**: OpenAI, Anthropic, Google, Ollama, custom APIs
//! - **Retry Logic**: Configurable retry with backoff strategies
//! - **Input/Output Mapping**: Map token data to agent inputs and outputs
//! - **Scheduling**: Support for delayed and periodic agent execution

use futures::StreamExt;
use reqwest;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use uuid::Uuid;

use crate::models::{
    AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentStreamEvent, LLMConfig, LLMProvider, Resource,
};
use crate::{CircuitBreakerError, Result};

/// Storage trait for agent-related data
#[async_trait::async_trait]
pub trait AgentStorage: Send + Sync {
    // Agent definitions
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()>;
    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>>;
    async fn list_agents(&self) -> Result<Vec<AgentDefinition>>;
    async fn delete_agent(&self, id: &AgentId) -> Result<bool>;

    // Agent executions
    async fn store_execution(&self, execution: &AgentExecution) -> Result<()>;
    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>>;
    async fn list_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>>;

    /// List executions matching multiple context criteria (AND logic)
    async fn list_executions_by_context_filters(
        &self,
        filters: &[(&str, &str)],
    ) -> Result<Vec<AgentExecution>>;

    /// List executions by nested context path (e.g., "workflow.resource_id")
    async fn list_executions_by_nested_context(
        &self,
        context_path: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>>;

    /// Count executions matching context criteria
    async fn count_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<usize>;

    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>>;

    // Convenience methods for common query patterns

    /// List executions for a specific resource
    async fn list_executions_for_resource(&self, resource_id: &Uuid)
        -> Result<Vec<AgentExecution>>;

    /// Get executions by status
    async fn list_executions_by_status(
        &self,
        status: &AgentExecutionStatus,
    ) -> Result<Vec<AgentExecution>>;

    /// Get recent executions (limited count)
    async fn list_recent_executions(&self, limit: usize) -> Result<Vec<AgentExecution>>;

    /// Get executions for an agent with specific context
    async fn list_executions_for_agent_with_context(
        &self,
        agent_id: &AgentId,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>>;
}

/// In-memory implementation of AgentStorage for development/testing
#[derive(Debug, Default)]
pub struct InMemoryAgentStorage {
    agents: RwLock<HashMap<AgentId, AgentDefinition>>,
    executions: RwLock<HashMap<Uuid, AgentExecution>>,
}

#[async_trait::async_trait]
impl AgentStorage for InMemoryAgentStorage {
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()> {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id.clone(), agent.clone());
        Ok(())
    }

    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>> {
        let agents = self.agents.read().await;
        Ok(agents.get(id).cloned())
    }

    async fn list_agents(&self) -> Result<Vec<AgentDefinition>> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    async fn delete_agent(&self, id: &AgentId) -> Result<bool> {
        let mut agents = self.agents.write().await;
        Ok(agents.remove(id).is_some())
    }

    async fn store_execution(&self, execution: &AgentExecution) -> Result<()> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.id, execution.clone());
        Ok(())
    }

    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.get(id).cloned())
    }

    async fn list_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| {
                exec.get_context_value(context_key)
                    .and_then(|v| v.as_str())
                    .map(|value| value == context_value)
                    .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn list_executions_by_context_filters(
        &self,
        filters: &[(&str, &str)],
    ) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| {
                filters.iter().all(|(key, expected_value)| {
                    exec.get_context_value(key)
                        .and_then(|v| v.as_str())
                        .map(|value| value == *expected_value)
                        .unwrap_or(false)
                })
            })
            .cloned()
            .collect())
    }

    async fn list_executions_by_nested_context(
        &self,
        context_path: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| {
                exec.get_nested_context_value(context_path)
                    .and_then(|v| v.as_str())
                    .map(|value| value == context_value)
                    .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn count_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<usize> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| {
                exec.get_context_value(context_key)
                    .and_then(|v| v.as_str())
                    .map(|value| value == context_value)
                    .unwrap_or(false)
            })
            .count())
    }

    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| &exec.agent_id == agent_id)
            .cloned()
            .collect())
    }

    async fn list_executions_by_status(
        &self,
        status: &AgentExecutionStatus,
    ) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| &exec.status == status)
            .cloned()
            .collect())
    }

    async fn list_recent_executions(&self, limit: usize) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        let mut sorted_executions: Vec<_> = executions.values().cloned().collect();

        // Sort by started_at in descending order (most recent first)
        sorted_executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        // Take only the requested number
        sorted_executions.truncate(limit);

        Ok(sorted_executions)
    }

    async fn list_executions_for_agent_with_context(
        &self,
        agent_id: &AgentId,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| {
                &exec.agent_id == agent_id
                    && exec
                        .get_context_value(context_key)
                        .and_then(|v| v.as_str())
                        .map(|value| value == context_value)
                        .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn list_executions_for_resource(
        &self,
        resource_id: &Uuid,
    ) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        let resource_id_str = resource_id.to_string();

        Ok(executions
            .values()
            .filter(|exec| {
                // Check for resource_id in various context locations
                exec.get_context_value("resource_id")
                    .and_then(|v| v.as_str())
                    .map(|value| value == resource_id_str)
                    .unwrap_or(false)
                    || exec
                        .get_context_value("workflow")
                        .and_then(|w| w.get("resource_id"))
                        .and_then(|v| v.as_str())
                        .map(|value| value == resource_id_str)
                        .unwrap_or(false)
            })
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    use crate::models::{AgentExecutionStatus, AgentPrompts};
    use serde_json::json;
    use std::collections::HashMap;
    use tokio;

    async fn create_test_storage_with_executions() -> InMemoryAgentStorage {
        let storage = InMemoryAgentStorage::default();

        // Create test agent
        let agent = AgentDefinition {
            id: AgentId::from("test-agent"),
            name: "Test Agent".to_string(),
            description: "Test agent for storage tests".to_string(),
            llm_provider: LLMProvider::OpenAI {
                api_key: "test-key".to_string(),
                model: "gpt-4".to_string(),
                base_url: None,
            },
            llm_config: LLMConfig::default(),
            prompts: AgentPrompts {
                system: "You are a test agent".to_string(),
                user_template: "Test: {input}".to_string(),
                context_instructions: None,
            },
            capabilities: vec!["test".to_string()],
            tools: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        storage.store_agent(&agent).await.unwrap();

        // Create test executions with different contexts
        let executions = vec![
            AgentExecution::new(
                AgentId::from("test-agent"),
                json!({
                    "resource_id": "resource-1",
                    "state_id": "pending",
                    "workflow": {
                        "id": "workflow-1",
                        "version": "v1.0"
                    }
                }),
                json!({"message": "test 1"}),
            ),
            AgentExecution::new(
                AgentId::from("test-agent"),
                json!({
                    "resource_id": "resource-2",
                    "state_id": "completed",
                    "workflow": {
                        "id": "workflow-1",
                        "version": "v1.1"
                    }
                }),
                json!({"message": "test 2"}),
            ),
            AgentExecution::new(
                AgentId::from("test-agent"),
                json!({
                    "resource_id": "resource-1",
                    "state_id": "failed",
                    "workflow": {
                        "id": "workflow-2",
                        "version": "v1.0"
                    }
                }),
                json!({"message": "test 3"}),
            ),
        ];

        for execution in executions {
            storage.store_execution(&execution).await.unwrap();
        }

        storage
    }

    #[tokio::test]
    async fn test_list_executions_by_context() {
        let storage = create_test_storage_with_executions().await;

        // Test filtering by resource_id
        let resource_1_executions = storage
            .list_executions_by_context("resource_id", "resource-1")
            .await
            .unwrap();
        assert_eq!(resource_1_executions.len(), 2);

        let resource_2_executions = storage
            .list_executions_by_context("resource_id", "resource-2")
            .await
            .unwrap();
        assert_eq!(resource_2_executions.len(), 1);

        // Test filtering by state_id
        let pending_executions = storage
            .list_executions_by_context("state_id", "pending")
            .await
            .unwrap();
        assert_eq!(pending_executions.len(), 1);

        // Test non-existent context
        let nonexistent = storage
            .list_executions_by_context("nonexistent_key", "value")
            .await
            .unwrap();
        assert_eq!(nonexistent.len(), 0);
    }

    #[tokio::test]
    async fn test_list_executions_by_context_filters() {
        let storage = create_test_storage_with_executions().await;

        // Test multiple filters (AND logic)
        let filtered = storage
            .list_executions_by_context_filters(&[
                ("resource_id", "resource-1"),
                ("state_id", "pending"),
            ])
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);

        // Test filters that match no executions
        let no_match = storage
            .list_executions_by_context_filters(&[
                ("resource_id", "resource-1"),
                ("state_id", "nonexistent"),
            ])
            .await
            .unwrap();
        assert_eq!(no_match.len(), 0);

        // Test empty filters (should return all)
        let all = storage
            .list_executions_by_context_filters(&[])
            .await
            .unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_executions_by_nested_context() {
        let storage = create_test_storage_with_executions().await;

        // Test nested context access
        let workflow_1_executions = storage
            .list_executions_by_nested_context("workflow.id", "workflow-1")
            .await
            .unwrap();
        assert_eq!(workflow_1_executions.len(), 2);

        let version_v10_executions = storage
            .list_executions_by_nested_context("workflow.version", "v1.0")
            .await
            .unwrap();
        assert_eq!(version_v10_executions.len(), 2);

        // Test non-existent nested path
        let nonexistent = storage
            .list_executions_by_nested_context("workflow.nonexistent", "value")
            .await
            .unwrap();
        assert_eq!(nonexistent.len(), 0);
    }

    #[tokio::test]
    async fn test_count_executions_by_context() {
        let storage = create_test_storage_with_executions().await;

        let resource_1_count = storage
            .count_executions_by_context("resource_id", "resource-1")
            .await
            .unwrap();
        assert_eq!(resource_1_count, 2);

        let pending_count = storage
            .count_executions_by_context("state_id", "pending")
            .await
            .unwrap();
        assert_eq!(pending_count, 1);

        let nonexistent_count = storage
            .count_executions_by_context("nonexistent", "value")
            .await
            .unwrap();
        assert_eq!(nonexistent_count, 0);
    }

    #[tokio::test]
    async fn test_list_executions_by_status() {
        let storage = create_test_storage_with_executions().await;

        // All executions start as Pending
        let pending_executions = storage
            .list_executions_by_status(&AgentExecutionStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending_executions.len(), 3);

        // Test with different statuses
        let completed_executions = storage
            .list_executions_by_status(&AgentExecutionStatus::Completed)
            .await
            .unwrap();
        assert_eq!(completed_executions.len(), 0);
    }

    #[tokio::test]
    async fn test_list_recent_executions() {
        let storage = create_test_storage_with_executions().await;

        // Test limiting results
        let recent_2 = storage.list_recent_executions(2).await.unwrap();
        assert_eq!(recent_2.len(), 2);

        let recent_5 = storage.list_recent_executions(5).await.unwrap();
        assert_eq!(recent_5.len(), 3); // Only 3 total executions

        let recent_0 = storage.list_recent_executions(0).await.unwrap();
        assert_eq!(recent_0.len(), 0);
    }

    #[tokio::test]
    async fn test_list_executions_for_agent_with_context() {
        let storage = create_test_storage_with_executions().await;

        let agent_id = AgentId::from("test-agent");

        // Test agent + context filtering
        let agent_resource_1 = storage
            .list_executions_for_agent_with_context(&agent_id, "resource_id", "resource-1")
            .await
            .unwrap();
        assert_eq!(agent_resource_1.len(), 2);

        let agent_pending = storage
            .list_executions_for_agent_with_context(&agent_id, "state_id", "pending")
            .await
            .unwrap();
        assert_eq!(agent_pending.len(), 1);

        // Test with non-existent agent
        let nonexistent_agent = AgentId::from("nonexistent");
        let no_executions = storage
            .list_executions_for_agent_with_context(&nonexistent_agent, "resource_id", "resource-1")
            .await
            .unwrap();
        assert_eq!(no_executions.len(), 0);
    }

    #[tokio::test]
    async fn test_list_executions_by_resource_id_context() {
        let storage = create_test_storage_with_executions().await;

        // Test with random UUID that shouldn't match anything
        let resource_id = Uuid::new_v4();
        let no_executions = storage
            .list_executions_by_context("resource_id", &resource_id.to_string())
            .await
            .unwrap();
        assert_eq!(no_executions.len(), 0);

        // Create execution with UUID resource_id for testing
        let execution = AgentExecution::new(
            AgentId::from("test-agent"),
            json!({
                "resource_id": resource_id.to_string(),
                "state_id": "test"
            }),
            json!({"message": "uuid test"}),
        );
        storage.store_execution(&execution).await.unwrap();

        let found_executions = storage
            .list_executions_by_context("resource_id", &resource_id.to_string())
            .await
            .unwrap();
        assert_eq!(found_executions.len(), 1);
    }

    // Add tests for context-based Agent Engine methods
    #[cfg(test)]
    mod agent_engine_tests {
        use super::*;

        use serde_json::json;
        use std::collections::HashMap;

        fn create_test_agent_engine() -> AgentEngine {
            let storage = Arc::new(InMemoryAgentStorage::default());
            let llm_router = Arc::new(crate::llm::LLMRouter::new());
            AgentEngine::new(storage, AgentEngineConfig::default(), llm_router)
        }

        #[test]
        fn test_extract_value_from_context() {
            let engine = create_test_agent_engine();

            // Create a test context with nested structure
            let context = json!({
                "resource_id": "res-123",
                "state_id": "state-pending",
                "metadata": {
                    "priority": "high",
                    "tags": ["important", "urgent"]
                },
                "workflow": {
                    "id": "workflow-abc",
                    "version": "1.0.2",
                    "settings": {
                        "timeout": 3600,
                        "retry": true
                    }
                }
            });

            // Test basic extraction
            assert_eq!(
                engine
                    .extract_value_from_context(&context, "resource_id")
                    .unwrap(),
                json!("res-123")
            );

            // Test nested extraction
            assert_eq!(
                engine
                    .extract_value_from_context(&context, "metadata.priority")
                    .unwrap(),
                json!("high")
            );

            // Test deeply nested extraction
            assert_eq!(
                engine
                    .extract_value_from_context(&context, "workflow.settings.timeout")
                    .unwrap(),
                json!(3600)
            );

            // Test non-existent path
            assert_eq!(
                engine
                    .extract_value_from_context(&context, "nonexistent.path")
                    .unwrap(),
                json!(null)
            );
        }

        #[test]
        fn test_map_input_data() {
            let engine = create_test_agent_engine();

            // Create a test context
            let context = json!({
                "resource_id": "res-123",
                "user": {
                    "id": "user-456",
                    "name": "Test User",
                    "preferences": {
                        "theme": "dark"
                    }
                },
                "data": {
                    "title": "Test Document",
                    "content": "Lorem ipsum dolor sit amet"
                }
            });

            // Create mappings
            let mut mappings = HashMap::new();
            mappings.insert("document_id".to_string(), "resource_id".to_string());
            mappings.insert("user_name".to_string(), "user.name".to_string());
            mappings.insert("document_title".to_string(), "data.title".to_string());
            mappings.insert("theme".to_string(), "user.preferences.theme".to_string());

            // Test mapping
            let result = engine.map_input_data(&mappings, &context).unwrap();

            assert_eq!(result["document_id"], json!("res-123"));
            assert_eq!(result["user_name"], json!("Test User"));
            assert_eq!(result["document_title"], json!("Test Document"));
            assert_eq!(result["theme"], json!("dark"));
        }

        #[test]
        fn test_set_value_in_context() {
            let engine = create_test_agent_engine();

            // Create a mutable context
            let mut context = json!({
                "resource_id": "res-123",
                "metadata": {
                    "tags": ["draft"]
                }
            });

            // Test setting a top-level value
            engine
                .set_value_in_context(&mut context, "status", json!("approved"))
                .unwrap();
            assert_eq!(context["status"], json!("approved"));

            // Test setting a nested value
            engine
                .set_value_in_context(&mut context, "metadata.priority", json!("high"))
                .unwrap();
            assert_eq!(context["metadata"]["priority"], json!("high"));

            // Test setting a value in a path that doesn't exist yet (should create intermediate objects)
            engine
                .set_value_in_context(&mut context, "review.comments.main", json!("Looks good!"))
                .unwrap();
            assert_eq!(context["review"]["comments"]["main"], json!("Looks good!"));
        }

        #[test]
        fn test_apply_output_to_context() {
            let engine = create_test_agent_engine();

            // Create a mutable context
            let mut context = json!({
                "resource_id": "res-123",
                "data": {
                    "status": "pending"
                }
            });

            // Create output
            let output = json!({
                "approved": true,
                "review_score": 95,
                "reviewer": {
                    "name": "John Doe",
                    "department": "Quality Assurance"
                }
            });

            // Create mappings
            let mut mappings = HashMap::new();
            mappings.insert("data.status".to_string(), "approved".to_string());
            mappings.insert("metadata.score".to_string(), "review_score".to_string());
            mappings.insert(
                "metadata.reviewer_name".to_string(),
                "reviewer.name".to_string(),
            );

            // Apply output
            engine
                .apply_output_to_context(&mut context, &output, &mappings)
                .unwrap();

            // Verify results
            assert_eq!(context["data"]["status"], json!(true));
            assert_eq!(context["metadata"]["score"], json!(95));
            assert_eq!(context["metadata"]["reviewer_name"], json!("John Doe"));
        }
    }
}

/// Configuration for the agent engine
#[derive(Debug, Clone)]
pub struct AgentEngineConfig {
    pub max_concurrent_executions: usize,
    pub stream_buffer_size: usize,
    pub connection_timeout: Duration,
    pub execution_timeout: Duration,
    pub cleanup_interval: Duration,
}

impl Default for AgentEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 50,
            stream_buffer_size: 1000,
            connection_timeout: Duration::from_secs(30),
            execution_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Main agent execution engine
pub struct AgentEngine {
    storage: Arc<dyn AgentStorage>,
    config: AgentEngineConfig,
    stream_sender: broadcast::Sender<AgentStreamEvent>,
    llm_router: Arc<crate::llm::LLMRouter>,
}

impl AgentEngine {
    pub fn new(
        storage: Arc<dyn AgentStorage>,
        config: AgentEngineConfig,
        llm_router: Arc<crate::llm::LLMRouter>,
    ) -> Self {
        let (stream_sender, _) = broadcast::channel(config.stream_buffer_size);

        // Validate environment variables for LLM providers
        if let Err(e) = Self::validate_environment_variables() {
            warn!("⚠️  Environment validation failed: {}", e);
            warn!("🔧 Agent execution may fail without proper API keys");
        }

        Self {
            storage,
            config,
            stream_sender,
            llm_router,
        }
    }

    /// Get a reference to the storage backend
    pub fn storage(&self) -> &Arc<dyn AgentStorage> {
        &self.storage
    }

    /// Subscribe to agent execution stream events
    pub fn subscribe_to_stream(&self) -> broadcast::Receiver<AgentStreamEvent> {
        self.stream_sender.subscribe()
    }

    /// Execute agent with a generic context
    pub async fn execute_agent(
        &self,
        config: &AgentActivityConfig,
        context: serde_json::Value,
    ) -> Result<AgentExecution> {
        info!(
            "🎯 AgentEngine::execute_agent() called for agent_id: {}",
            config.agent_id
        );
        debug!(
            "🎯 Agent config: timeout={}s, required={}",
            config.timeout_seconds.unwrap_or(0),
            config.required
        );
        debug!(
            "🎯 Context: {}",
            serde_json::to_string(&context).unwrap_or_else(|_| "Invalid JSON".to_string())
        );

        info!("📂 Looking up agent definition for: {}", config.agent_id);
        debug!(
            "🔍 Using storage backend: {}",
            std::any::type_name::<dyn AgentStorage>()
        );
        debug!(
            "🔍 Storage implementation: {:?}",
            std::ptr::addr_of!(*self.storage)
        );
        let agent = self
            .storage
            .get_agent(&config.agent_id)
            .await?
            .ok_or_else(|| {
                error!("❌ Agent not found: {}", config.agent_id);
                error!(
                    "🔍 Storage type when agent not found: {}",
                    std::any::type_name::<dyn AgentStorage>()
                );
                CircuitBreakerError::NotFound(format!("Agent {}", config.agent_id.as_str()))
            })?;

        info!("✅ Agent definition found: {}", agent.name);
        debug!("📋 Agent LLM provider: {:?}", agent.llm_provider);
        debug!("📋 Agent system prompt: {}", agent.prompts.system);

        // Map input using context
        info!("🔄 Mapping input data using context");
        debug!("🔄 Input mapping: {:?}", config.input_mapping);
        let input_data = self.map_input_data(&config.input_mapping, &context)?;
        debug!(
            "🔄 Mapped input data: {}",
            serde_json::to_string(&input_data).unwrap_or_else(|_| "Invalid JSON".to_string())
        );

        info!("📝 Creating new agent execution");
        let mut execution = AgentExecution::new(config.agent_id.clone(), context, input_data);
        info!("📝 Created execution with ID: {}", execution.id);

        info!(
            "⚡ Calling execute_agent_internal() for execution: {}",
            execution.id
        );
        let internal_start = std::time::Instant::now();

        self.execute_agent_internal(
            &agent,
            &mut execution,
            &config.input_mapping,
            &config.output_mapping,
        )
        .await?;

        let internal_duration = internal_start.elapsed();
        info!(
            "✅ execute_agent_internal() completed in {:?} for execution: {}",
            internal_duration, execution.id
        );
        info!("📋 Final execution status: {:?}", execution.status);

        Ok(execution)
    }

    /// Execute agent for an activity (deprecated - use execute_agent instead)
    #[deprecated(
        since = "0.5.0",
        note = "Use execute_agent with a custom context instead"
    )]
    pub async fn execute_activity_agent(
        &self,
        config: &AgentActivityConfig,
        resource: &Resource,
    ) -> Result<AgentExecution> {
        // Create context from resource
        let context = serde_json::json!({
            "resource_id": resource.id,
            "state_id": resource.current_state(),
            "metadata": resource.metadata,
            "workflow_context": {
                "resource_id": resource.id,
                "state_id": resource.current_state()
            }
        });

        self.execute_agent(config, context).await
    }

    /// Internal agent execution logic
    async fn execute_agent_internal(
        &self,
        agent: &AgentDefinition,
        execution: &mut AgentExecution,
        _input_mapping: &HashMap<String, String>,
        _output_mapping: &HashMap<String, String>,
    ) -> Result<()> {
        info!(
            "⚡ execute_agent_internal() starting for execution: {}",
            execution.id
        );
        debug!("⚡ Agent: {} ({})", agent.name, agent.id);
        debug!("⚡ LLM Provider: {:?}", agent.llm_provider);
        debug!("⚡ LLM Config: {:?}", agent.llm_config);

        info!("📝 Starting execution and storing to storage");
        execution.start();

        info!("💾 Storing execution to storage");
        self.storage.store_execution(execution).await?;
        info!("✅ Execution stored successfully");

        // Emit starting event
        info!(
            "📡 Emitting ThinkingStatus event for execution: {}",
            execution.id
        );
        let _ = self.stream_sender.send(AgentStreamEvent::ThinkingStatus {
            execution_id: execution.id,
            status: "Starting agent execution".to_string(),
            context: Some(execution.context.clone()),
        });
        info!("📡 ThinkingStatus event sent");

        // Execute the LLM call (this would integrate with actual LLM providers)
        info!(
            "🤖 About to call LLM provider for execution: {}",
            execution.id
        );
        info!("🤖 LLM Provider type: {:?}", agent.llm_provider);
        debug!(
            "🤖 Input data: {}",
            serde_json::to_string(&execution.input_data)
                .unwrap_or_else(|_| "Invalid JSON".to_string())
        );

        let llm_start = std::time::Instant::now();
        info!("🎯 Calling call_llm_provider_streaming()...");

        // Start streaming LLM call - this now returns immediately and streams in background
        match self
            .call_llm_provider_streaming(
                &agent.llm_provider,
                &agent.llm_config,
                &execution.input_data,
                execution.id,
            )
            .await
        {
            Ok(response) => {
                let llm_duration = llm_start.elapsed();
                info!(
                    "✅ LLM streaming started successfully in {:?}",
                    llm_duration
                );
                debug!(
                    "📋 LLM streaming response: {}",
                    serde_json::to_string(&response).unwrap_or_else(|_| "Invalid JSON".to_string())
                );

                info!("📝 Execution started - streaming in progress");
                execution.complete(response.clone());

                // Don't emit completion event here - it's handled by the background streaming task
                info!("📡 Streaming events will be emitted as chunks arrive");
            }
            Err(e) => {
                let llm_duration = llm_start.elapsed();
                error!(
                    "❌ LLM streaming failed to start after {:?}: {}",
                    llm_duration, e
                );
                error!("❌ LLM error details: {:?}", e);
                error!("❌ Failed execution: {}", execution.id);

                info!("📝 Marking execution as failed");
                execution.fail(e.to_string());

                // Emit failure event
                info!("📡 Emitting Failed event for execution: {}", execution.id);
                let _ = self.stream_sender.send(AgentStreamEvent::Failed {
                    execution_id: execution.id,
                    error: e.to_string(),
                    context: Some(execution.context.clone()),
                });
                info!("📡 Failed event sent");
            }
        }

        self.storage.store_execution(execution).await?;
        Ok(())
    }

    /// Map context data to agent input using the provided mapping
    fn map_input_data(&self, mapping: &HashMap<String, String>, context: &Value) -> Result<Value> {
        let mut input = json!({});

        for (input_key, context_path) in mapping {
            let value = self.extract_value_from_context(context, context_path)?;
            input[input_key] = value;
        }

        Ok(input)
    }

    /// Extract value from context using a dot-notation path like "workflow.id" or "metadata.type"
    fn extract_value_from_context(&self, context: &Value, path: &str) -> Result<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = context;

        for part in parts {
            if let Some(value) = current.get(part) {
                current = value;
            } else {
                return Ok(Value::Null);
            }
        }

        Ok(current.clone())
    }

    /// Route virtual models through Circuit Breaker's intelligent routing (streaming version)
    async fn route_virtual_model_streaming(
        &self,
        virtual_model: &str,
        config: &LLMConfig,
        input: &Value,
        execution_id: uuid::Uuid,
    ) -> Result<Value> {
        info!("🌟 Routing virtual model: {}", virtual_model);

        // Map virtual models to actual providers based on routing strategy
        let (actual_provider, actual_model, routing_reason) = match virtual_model {
            "cb:fastest" => {
                info!("🚀 cb:fastest -> routing to fastest available model");
                ("openai", "o4-mini-2025-04-16", "Optimized for speed")
            }
            "cb:cost-optimal" => {
                info!("💰 cb:cost-optimal -> routing to most cost-effective model");
                ("openai", "o4-mini-2025-04-16", "Optimized for cost")
            }
            "cb:smart-chat" => {
                info!("⚖️ cb:smart-chat -> routing to balanced model");
                (
                    "anthropic",
                    "claude-sonnet-4-20250514",
                    "Balanced performance and cost",
                )
            }
            "cb:creative" => {
                info!("🎨 cb:creative -> routing to creative model");
                (
                    "anthropic",
                    "claude-sonnet-4-20250514",
                    "Optimized for creativity",
                )
            }
            "cb:coding" => {
                info!("💻 cb:coding -> routing to coding-optimized model");
                (
                    "anthropic",
                    "claude-sonnet-4-20250514",
                    "Optimized for code generation",
                )
            }
            _ => {
                return Err(CircuitBreakerError::NotFound(format!(
                    "Unknown virtual model: {}",
                    virtual_model
                )));
            }
        };

        info!(
            "🎯 Virtual model {} routed to {} {} ({})",
            virtual_model, actual_provider, actual_model, routing_reason
        );

        // Create routed provider
        let routed_provider = match actual_provider {
            "openai" => LLMProvider::OpenAI {
                model: actual_model.to_string(),
                api_key: String::new(), // Will be populated from config
                base_url: None,
            },
            "anthropic" => LLMProvider::Anthropic {
                model: actual_model.to_string(),
                api_key: String::new(),
                base_url: None,
            },
            "google" => LLMProvider::Google {
                model: actual_model.to_string(),
                api_key: String::new(),
            },
            "ollama" => LLMProvider::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: actual_model.to_string(),
            },
            _ => {
                return Err(CircuitBreakerError::NotFound(format!(
                    "Unknown provider: {}",
                    actual_provider
                )));
            }
        };

        info!("🔄 Recursively calling LLM provider with streaming configuration");

        // Use streaming call instead of non-streaming
        let mut result = self
            .call_llm_provider_streaming(&routed_provider, config, input, execution_id)
            .await?;

        // Add virtual model metadata to response
        if let Some(obj) = result.as_object_mut() {
            obj.insert("virtual_model".to_string(), json!(virtual_model));
            obj.insert("routed_provider".to_string(), json!(actual_provider));
            obj.insert("routed_model".to_string(), json!(actual_model));
            obj.insert("routing_reason".to_string(), json!(routing_reason));
        }

        info!("✅ Virtual model routing completed for: {}", virtual_model);
        Ok(result)
    }

    async fn route_virtual_model(
        &self,
        virtual_model: &str,
        config: &LLMConfig,
        input: &Value,
    ) -> Result<Value> {
        info!("🌟 Routing virtual model: {}", virtual_model);

        // Map virtual models to actual providers based on routing strategy
        let (actual_provider, actual_model, routing_reason) = match virtual_model {
            "cb:fastest" => {
                info!("🚀 cb:fastest -> routing to fastest available model");
                ("openai", "o4-mini-2025-04-16", "Optimized for speed")
            }
            "cb:cost-optimal" => {
                info!("💰 cb:cost-optimal -> routing to most cost-effective model");
                ("openai", "o4-mini-2025-04-16", "Optimized for cost")
            }
            "cb:smart-chat" => {
                info!("⚖️ cb:smart-chat -> routing to balanced model");
                (
                    "anthropic",
                    "claude-sonnet-4-20250514",
                    "Balanced performance and cost",
                )
            }
            "cb:creative" => {
                info!("🎨 cb:creative -> routing to creative model");
                (
                    "anthropic",
                    "claude-sonnet-4-20250514",
                    "Optimized for creativity",
                )
            }
            "cb:coding" => {
                info!("💻 cb:coding -> routing to coding-optimized model");
                (
                    "anthropic",
                    "claude-sonnet-4-20250514",
                    "Optimized for code generation",
                )
            }
            "cb:analysis" => {
                info!("📊 cb:analysis -> routing to analysis-optimized model");
                ("google", "gemini-2.5-flash", "Optimized for data analysis")
            }
            _ => {
                warn!(
                    "🔍 Unknown virtual model: {} -> falling back to default",
                    virtual_model
                );
                (
                    "openai",
                    "o4-mini-2025-04-16",
                    "Unknown virtual model fallback",
                )
            }
        };

        info!(
            "🎯 Virtual model {} routed to {} {} ({})",
            virtual_model, actual_provider, actual_model, routing_reason
        );

        // Create the appropriate provider configuration
        let routed_provider = match actual_provider {
            "openai" => {
                let api_key =
                    std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "not-set".to_string());
                LLMProvider::OpenAI {
                    model: actual_model.to_string(),
                    api_key,
                    base_url: None,
                }
            }
            "anthropic" => {
                let api_key =
                    std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "not-set".to_string());
                LLMProvider::Anthropic {
                    model: actual_model.to_string(),
                    api_key,
                    base_url: None,
                }
            }
            "google" => {
                let api_key =
                    std::env::var("GOOGLE_API_KEY").unwrap_or_else(|_| "not-set".to_string());
                LLMProvider::Google {
                    model: actual_model.to_string(),
                    api_key,
                }
            }
            _ => {
                error!(
                    "🔴 Unsupported provider in virtual routing: {}",
                    actual_provider
                );
                return Err(CircuitBreakerError::Internal(format!(
                    "Unsupported LLM provider: {} with model: {}",
                    actual_provider, actual_model
                )));
            }
        };

        info!("🔄 Recursively calling LLM provider with routed configuration");

        // Recursively call with the routed provider
        let mut result = self
            .call_llm_provider(&routed_provider, config, input)
            .await?;

        // Add virtual model metadata to response
        if let Some(obj) = result.as_object_mut() {
            obj.insert("virtual_model".to_string(), json!(virtual_model));
            obj.insert("routed_provider".to_string(), json!(actual_provider));
            obj.insert("routed_model".to_string(), json!(actual_model));
            obj.insert("routing_reason".to_string(), json!(routing_reason));
        }

        info!("✅ Virtual model routing completed for: {}", virtual_model);
        Ok(result)
    }

    /// Validate environment variables for LLM providers
    pub fn validate_environment_variables() -> Result<()> {
        info!("🔍 Validating environment variables for LLM providers");

        let openai_key = std::env::var("OPENAI_API_KEY").ok();
        let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let google_key = std::env::var("GOOGLE_API_KEY").ok();

        info!(
            "🔑 OPENAI_API_KEY: {}",
            if openai_key.is_some() {
                "✅ Set"
            } else {
                "❌ Not set"
            }
        );
        info!(
            "🔑 ANTHROPIC_API_KEY: {}",
            if anthropic_key.is_some() {
                "✅ Set"
            } else {
                "❌ Not set"
            }
        );
        info!(
            "🔑 GOOGLE_API_KEY: {}",
            if google_key.is_some() {
                "✅ Set"
            } else {
                "❌ Not set"
            }
        );

        if openai_key.is_none() && anthropic_key.is_none() && google_key.is_none() {
            error!("❌ No LLM API keys found in environment variables!");
            error!("💡 Set at least one of: OPENAI_API_KEY, ANTHROPIC_API_KEY, GOOGLE_API_KEY");
            return Err(CircuitBreakerError::Internal(
                "No LLM API keys found in environment".to_string(),
            ));
        }

        info!("✅ Environment variable validation completed");
        Ok(())
    }

    /// Call the configured LLM provider with streaming support
    fn call_llm_provider_streaming<'a>(
        &'a self,
        provider: &'a LLMProvider,
        config: &'a LLMConfig,
        input: &'a Value,
        execution_id: uuid::Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(async move {
            info!("🤖 call_llm_provider_streaming() starting");
            debug!("🤖 Provider: {:?}", provider);
            debug!("🤖 Config: {:?}", config);
            debug!(
                "🤖 Input: {}",
                serde_json::to_string(input).unwrap_or_else(|_| "Invalid JSON".to_string())
            );

            // Check if this is a virtual model that needs Circuit Breaker routing
            let model_name = match provider {
                LLMProvider::OpenAI { model, .. } => model,
                LLMProvider::Anthropic { model, .. } => model,
                LLMProvider::Google { model, .. } => model,
                LLMProvider::Ollama { model, .. } => model,
                LLMProvider::Custom { model, .. } => model,
            };

            info!("🎯 Model requested: {}", model_name);

            // Check for virtual model routing
            if model_name.starts_with("cb:") {
                info!(
                    "🌟 Virtual model detected: {} - routing through Circuit Breaker",
                    model_name
                );
                return self
                    .route_virtual_model_streaming(model_name, config, input, execution_id)
                    .await;
            }

            info!("🔧 Using direct provider routing for model: {}", model_name);

            // Create LLM request
            let request = crate::llm::LLMRequest {
                id: uuid::Uuid::new_v4(),
                model: model_name.clone(),
                messages: vec![crate::llm::ChatMessage {
                    role: crate::llm::MessageRole::User,
                    content: input
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("No message provided")
                        .to_string(),
                    name: None,
                    function_call: None,
                }],
                temperature: Some(config.temperature.into()),
                max_tokens: config.max_tokens,
                top_p: config.top_p.map(|p| p.into()),
                frequency_penalty: config.frequency_penalty.map(|f| f.into()),
                presence_penalty: config.presence_penalty.map(|p| p.into()),
                stop: if config.stop_sequences.is_empty() {
                    debug!("🚫 Stop sequences empty, setting stop to None");
                    None
                } else {
                    debug!(
                        "🛑 Stop sequences not empty: {:?}, setting stop to Some",
                        config.stop_sequences
                    );
                    Some(config.stop_sequences.clone())
                },
                stream: Some(true),
                function_call: None,
                functions: None,
                user: None,
                metadata: std::collections::HashMap::new(),
            };

            // Start streaming in background task - don't await completion
            let stream_sender = self.stream_sender.clone();
            let llm_router = self.llm_router.clone();
            let model_name_clone = model_name.clone();
            let input_clone = input.clone();

            tokio::spawn(async move {
                debug!(
                    "🎯 About to call stream_chat_completion with stop: {:?}",
                    request.stop
                );
                match llm_router.stream_chat_completion(request).await {
                    Ok(mut stream) => {
                        let mut full_response = String::new();
                        let mut sequence = 0u32;

                        // Process streaming chunks in real-time
                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    if let Some(choice) = chunk.choices.first() {
                                        if !choice.delta.content.is_empty() {
                                            let content = &choice.delta.content;
                                            full_response.push_str(content);
                                            sequence += 1;

                                            // Emit ContentChunk event immediately
                                            let _ = stream_sender.send(
                                                AgentStreamEvent::ContentChunk {
                                                    execution_id,
                                                    chunk: content.clone(),
                                                    sequence,
                                                    context: None,
                                                },
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("❌ Streaming chunk error: {}", e);
                                    let _ = stream_sender.send(AgentStreamEvent::Failed {
                                        execution_id,
                                        error: format!("Streaming error: {}", e),
                                        context: None,
                                    });
                                    return;
                                }
                            }
                        }

                        // Emit completion event
                        let response_json = serde_json::json!({
                            "response": full_response,
                            "model": model_name_clone,
                            "provider": "streaming",
                            "input_processed": input_clone
                        });

                        let _ = stream_sender.send(AgentStreamEvent::Completed {
                            execution_id,
                            final_response: response_json,
                            usage: None,
                            context: None,
                        });
                    }
                    Err(e) => {
                        error!("❌ Streaming API call failed: {}", e);
                        let _ = stream_sender.send(AgentStreamEvent::Failed {
                            execution_id,
                            error: format!("Streaming failed: {}", e),
                            context: None,
                        });
                    }
                }
            });

            // Return immediately - streaming happens in background
            let response_json = serde_json::json!({
                "status": "streaming_started",
                "execution_id": execution_id,
                "model": model_name,
                "provider": "streaming",
                "input_processed": input
            });

            Ok(response_json)
        })
    }

    /// Call the configured LLM provider (non-streaming version)
    fn call_llm_provider<'a>(
        &'a self,
        provider: &'a LLMProvider,
        config: &'a LLMConfig,
        input: &'a Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(async move {
            info!("🤖 call_llm_provider() starting");
            debug!("🤖 Provider: {:?}", provider);
            debug!("🤖 Config: {:?}", config);
            debug!(
                "🤖 Input: {}",
                serde_json::to_string(input).unwrap_or_else(|_| "Invalid JSON".to_string())
            );

            // Check if this is a virtual model that needs Circuit Breaker routing
            let model_name = match provider {
                LLMProvider::OpenAI { model, .. } => model,
                LLMProvider::Anthropic { model, .. } => model,
                LLMProvider::Google { model, .. } => model,
                LLMProvider::Ollama { model, .. } => model,
                LLMProvider::Custom { model, .. } => model,
            };

            info!("🎯 Model requested: {}", model_name);

            // Check for virtual model routing
            if model_name.starts_with("cb:") {
                info!(
                    "🌟 Virtual model detected: {} - routing through Circuit Breaker",
                    model_name
                );
                return self.route_virtual_model(model_name, config, input).await;
            }

            info!("🔧 Using direct provider routing for model: {}", model_name);

            match provider {
                LLMProvider::OpenAI {
                    model,
                    api_key,
                    base_url,
                    ..
                } => {
                    info!("🤖 Calling OpenAI provider with model: {}", model);
                    debug!("🤖 OpenAI base_url: {:?}", base_url);

                    // Check if API key is available from environment
                    let env_api_key = std::env::var("OPENAI_API_KEY").ok();
                    if env_api_key.is_none() {
                        warn!("⚠️  OPENAI_API_KEY not found in environment variables");
                    }

                    info!("⏱️  Starting OpenAI API simulation (500ms delay)");
                    sleep(Duration::from_millis(500)).await;
                    info!("🚀 Making real OpenAI API call with model: {}", model);

                    // Use the LLM router for real API calls
                    let request = crate::llm::LLMRequest {
                        id: uuid::Uuid::new_v4(),
                        model: model.clone(),
                        messages: vec![crate::llm::ChatMessage {
                            role: crate::llm::MessageRole::User,
                            content: input.to_string(),
                            name: None,
                            function_call: None,
                        }],
                        temperature: Some(config.temperature.into()),
                        max_tokens: config.max_tokens,
                        top_p: None,
                        frequency_penalty: None,
                        presence_penalty: None,
                        stop: None,
                        stream: Some(false),
                        functions: None,
                        function_call: None,
                        user: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    match self.llm_router.chat_completion(request).await {
                        Ok(response) => {
                            info!("✅ OpenAI API call completed");
                            Ok(json!({
                                "model": model,
                                "response": response.choices.get(0)
                                    .map(|c| c.message.content.clone())
                                    .unwrap_or_else(|| "No response content".to_string()),
                                "input_processed": input,
                                "temperature": config.temperature,
                                "provider": "openai",
                                "usage": response.usage
                            }))
                        }
                        Err(e) => {
                            error!("❌ OpenAI API call failed: {}", e);
                            Err(CircuitBreakerError::InvalidInput(format!(
                                "OpenAI API error: {}",
                                e
                            )))
                        }
                    }
                }
                LLMProvider::Anthropic {
                    model,
                    api_key,
                    base_url,
                    ..
                } => {
                    info!("🤖 Calling Anthropic provider with model: {}", model);
                    debug!("🤖 Anthropic base_url: {:?}", base_url);

                    // Check if API key is available from environment
                    let env_api_key = std::env::var("ANTHROPIC_API_KEY").ok();
                    if env_api_key.is_none() {
                        warn!("⚠️  ANTHROPIC_API_KEY not found in environment variables");
                    }

                    info!("🚀 Making real Anthropic API call with model: {}", model);

                    // Use the LLM router for real API calls
                    let request = crate::llm::LLMRequest {
                        id: uuid::Uuid::new_v4(),
                        model: model.clone(),
                        messages: vec![crate::llm::ChatMessage {
                            role: crate::llm::MessageRole::User,
                            content: input.to_string(),
                            name: None,
                            function_call: None,
                        }],
                        temperature: Some(config.temperature.into()),
                        max_tokens: config.max_tokens,
                        top_p: None,
                        frequency_penalty: None,
                        presence_penalty: None,
                        stop: None,
                        stream: Some(false),
                        functions: None,
                        function_call: None,
                        user: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    match self.llm_router.chat_completion(request).await {
                        Ok(response) => {
                            info!("✅ Anthropic API call completed");
                            Ok(json!({
                                "model": model,
                                "response": response.choices.get(0)
                                    .map(|c| c.message.content.clone())
                                    .unwrap_or_else(|| "No response content".to_string()),
                                "input_processed": input,
                                "temperature": config.temperature,
                                "provider": "anthropic",
                                "usage": response.usage
                            }))
                        }
                        Err(e) => {
                            error!("❌ Anthropic API call failed: {}", e);
                            Err(CircuitBreakerError::InvalidInput(format!(
                                "Anthropic API error: {}",
                                e
                            )))
                        }
                    }
                }
                LLMProvider::Google { model, api_key, .. } => {
                    info!("🤖 Calling Google provider with model: {}", model);

                    // Check if API key is available from environment
                    let env_api_key = std::env::var("GOOGLE_API_KEY").ok();
                    if env_api_key.is_none() {
                        warn!("⚠️  GOOGLE_API_KEY not found in environment variables");
                    }

                    info!("🚀 Making real Google API call with model: {}", model);

                    // Use the LLM router for real API calls
                    let request = crate::llm::LLMRequest {
                        id: uuid::Uuid::new_v4(),
                        model: model.clone(),
                        messages: vec![crate::llm::ChatMessage {
                            role: crate::llm::MessageRole::User,
                            content: input.to_string(),
                            name: None,
                            function_call: None,
                        }],
                        temperature: Some(config.temperature.into()),
                        max_tokens: config.max_tokens,
                        top_p: None,
                        frequency_penalty: None,
                        presence_penalty: None,
                        stop: None,
                        stream: Some(false),
                        functions: None,
                        function_call: None,
                        user: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    match self.llm_router.chat_completion(request).await {
                        Ok(response) => {
                            info!("✅ Google API call completed");
                            Ok(json!({
                                "model": model,
                                "response": response.choices.get(0)
                                    .map(|c| c.message.content.clone())
                                    .unwrap_or_else(|| "No response content".to_string()),
                                "input_processed": input,
                                "temperature": config.temperature,
                                "provider": "google",
                                "usage": response.usage
                            }))
                        }
                        Err(e) => {
                            error!("❌ Google API call failed: {}", e);
                            Err(CircuitBreakerError::InvalidInput(format!(
                                "Google API error: {}",
                                e
                            )))
                        }
                    }
                }
                LLMProvider::Ollama {
                    model, base_url, ..
                } => {
                    info!("🤖 Calling Ollama provider with model: {}", model);
                    debug!("🤖 Ollama base_url: {}", base_url);

                    info!("🚀 Making real Ollama API call with model: {}", model);

                    // Use the LLM router for real API calls
                    let request = crate::llm::LLMRequest {
                        id: uuid::Uuid::new_v4(),
                        model: model.clone(),
                        messages: vec![crate::llm::ChatMessage {
                            role: crate::llm::MessageRole::User,
                            content: input.to_string(),
                            name: None,
                            function_call: None,
                        }],
                        temperature: Some(config.temperature.into()),
                        max_tokens: config.max_tokens,
                        top_p: None,
                        frequency_penalty: None,
                        presence_penalty: None,
                        stop: None,
                        stream: Some(false),
                        functions: None,
                        function_call: None,
                        user: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    match self.llm_router.chat_completion(request).await {
                        Ok(response) => {
                            info!("✅ Ollama API call completed");
                            Ok(json!({
                                "model": model,
                                "response": response.choices.get(0)
                                    .map(|c| c.message.content.clone())
                                    .unwrap_or_else(|| "No response content".to_string()),
                                "input_processed": input,
                                "temperature": config.temperature,
                                "provider": "ollama",
                                "usage": response.usage
                            }))
                        }
                        Err(e) => {
                            error!("❌ Ollama API call failed: {}", e);
                            Err(CircuitBreakerError::InvalidInput(format!(
                                "Ollama API error: {}",
                                e
                            )))
                        }
                    }
                }
                LLMProvider::Custom {
                    model,
                    endpoint,
                    headers,
                    ..
                } => {
                    info!("🤖 Calling Custom provider with model: {}", model);
                    debug!("🤖 Custom endpoint: {}", endpoint);
                    debug!("🤖 Custom headers: {:?}", headers);

                    info!(
                        "🚀 Making real Custom Provider API call with model: {}",
                        model
                    );

                    // Use the LLM router for real API calls
                    let request = crate::llm::LLMRequest {
                        id: uuid::Uuid::new_v4(),
                        model: model.clone(),
                        messages: vec![crate::llm::ChatMessage {
                            role: crate::llm::MessageRole::User,
                            content: input.to_string(),
                            name: None,
                            function_call: None,
                        }],
                        temperature: Some(config.temperature.into()),
                        max_tokens: config.max_tokens,
                        top_p: None,
                        frequency_penalty: None,
                        presence_penalty: None,
                        stop: None,
                        stream: Some(false),
                        functions: None,
                        function_call: None,
                        user: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    match self.llm_router.chat_completion(request).await {
                        Ok(response) => {
                            info!("✅ Custom Provider API call completed");
                            Ok(json!({
                                "model": model,
                                "endpoint": endpoint,
                                "response": response.choices.get(0)
                                    .map(|c| c.message.content.clone())
                                    .unwrap_or_else(|| "No response content".to_string()),
                                "input_processed": input,
                                "temperature": config.temperature,
                                "provider": "custom",
                                "usage": response.usage
                            }))
                        }
                        Err(e) => {
                            error!("❌ Custom Provider API call failed: {}", e);
                            Err(CircuitBreakerError::InvalidInput(format!(
                                "Custom Provider API error: {}",
                                e
                            )))
                        }
                    }
                }
            }
        })
    }

    /// Apply agent output to context using output mapping
    pub fn apply_output_to_context(
        &self,
        context: &mut Value,
        output: &Value,
        mapping: &HashMap<String, String>,
    ) -> Result<()> {
        for (context_path, output_key) in mapping {
            if let Some(value) = output.get(output_key) {
                self.set_value_in_context(context, context_path, value.clone())?;
            }
        }
        Ok(())
    }

    /// Set value in context using a dot-notation path
    fn set_value_in_context(&self, context: &mut Value, path: &str, value: Value) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err(CircuitBreakerError::InvalidInput(
                "Empty context path".to_string(),
            ));
        }

        // Handle single-level path
        if parts.len() == 1 {
            if let Value::Object(ref mut map) = context {
                map.insert(parts[0].to_string(), value);
                return Ok(());
            } else {
                return Err(CircuitBreakerError::InvalidInput(
                    "Context is not an object".to_string(),
                ));
            }
        }

        // Handle multi-level path
        let mut current = context;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                if let Value::Object(ref mut map) = current {
                    map.insert(part.to_string(), value);
                    return Ok(());
                } else {
                    return Err(CircuitBreakerError::InvalidInput(format!(
                        "Cannot set value at path '{}': parent is not an object",
                        path
                    )));
                }
            } else {
                // Navigate to next level, creating objects as needed
                if let Value::Object(ref mut map) = current {
                    if !map.contains_key(*part) {
                        map.insert(part.to_string(), json!({}));
                    }

                    let next = map.get_mut(*part).unwrap();
                    if !next.is_object() {
                        *next = json!({});
                    }
                    current = next;
                } else {
                    return Err(CircuitBreakerError::InvalidInput(format!(
                        "Cannot navigate to path '{}': parent is not an object",
                        path
                    )));
                }
            }
        }

        Ok(())
    }

    /// Make actual Anthropic API call
    async fn call_anthropic_api(
        &self,
        model: &str,
        api_key: &str,
        base_url: Option<&str>,
        config: &LLMConfig,
        input: &Value,
    ) -> Result<Value> {
        let client = reqwest::Client::new();

        // Extract content from input
        let content = if let Some(content_str) = input.get("content").and_then(|v| v.as_str()) {
            content_str
        } else {
            return Err(CircuitBreakerError::InvalidInput(
                "No content found in input".to_string(),
            ));
        };

        // Prepare the request body according to Anthropic's API format
        let request_body = json!({
            "model": model,
            "max_tokens": config.max_tokens.unwrap_or(1000),
            "temperature": config.temperature,
            "messages": [{
                "role": "user",
                "content": content
            }]
        });

        // Construct the API endpoint URL
        let api_url = base_url
            .unwrap_or("https://api.anthropic.com/v1")
            .trim_end_matches('/');
        let messages_url = format!("{}/messages", api_url);

        // Make the API call
        let response = client
            .post(&messages_url)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                CircuitBreakerError::InvalidInput(format!("Anthropic API request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CircuitBreakerError::InvalidInput(format!(
                "Anthropic API error {}: {}",
                status, error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            CircuitBreakerError::InvalidInput(format!("Failed to parse Anthropic response: {}", e))
        })?;

        // Extract the content from Anthropic's response format
        let anthropic_response = response_json
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .and_then(|text| text.as_str())
            .unwrap_or("No response from Anthropic");

        // Return structured response
        Ok(json!({
            "model": model,
            "response": anthropic_response,
            "input_processed": input,
            "temperature": config.temperature,
            "provider": "anthropic",
            "raw_response": response_json
        }))
    }

    /// Get agent execution statistics
    pub async fn get_execution_stats(&self, agent_id: &AgentId) -> Result<ExecutionStats> {
        let executions = self.storage.list_executions_for_agent(agent_id).await?;

        let total = executions.len();
        let completed = executions
            .iter()
            .filter(|e| e.status == AgentExecutionStatus::Completed)
            .count();
        let failed = executions
            .iter()
            .filter(|e| e.status == AgentExecutionStatus::Failed)
            .count();
        let running = executions
            .iter()
            .filter(|e| e.status == AgentExecutionStatus::Running)
            .count();

        let avg_duration = if completed > 0 {
            let total_duration: u64 = executions
                .iter()
                .filter(|e| e.status == AgentExecutionStatus::Completed)
                .filter_map(|e| e.duration_ms)
                .sum();
            Some(total_duration / completed as u64)
        } else {
            None
        };

        Ok(ExecutionStats {
            total,
            completed,
            failed,
            running,
            avg_duration_ms: avg_duration,
        })
    }
}

/// Agent execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub running: usize,
    pub avg_duration_ms: Option<u64>,
}
