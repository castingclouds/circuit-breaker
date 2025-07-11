use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{ActivityId, Rule, StateId};

/// Unique identifier for an AI agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AgentId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl From<String> for AgentId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMProvider {
    OpenAI {
        api_key: String,
        model: String,
        base_url: Option<String>,
    },
    Anthropic {
        api_key: String,
        model: String,
        base_url: Option<String>,
    },
    Google {
        api_key: String,
        model: String,
    },
    Ollama {
        base_url: String,
        model: String,
    },
    Custom {
        endpoint: String,
        headers: HashMap<String, String>,
        model: String,
    },
}

/// LLM generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: Some(4000),
            top_p: Some(0.9),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: vec![],
        }
    }
}

/// Agent prompt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPrompts {
    pub system: String,
    pub user_template: String,
    pub context_instructions: Option<String>,
}

/// Agent definition with LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub llm_provider: LLMProvider,
    pub llm_config: LLMConfig,
    pub prompts: AgentPrompts,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Retry configuration for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRetryConfig {
    pub max_attempts: u32,
    pub backoff_seconds: u64,
    pub retry_on_errors: Vec<String>,
}

impl Default for AgentRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_seconds: 10,
            retry_on_errors: vec![
                "timeout".to_string(),
                "rate_limit".to_string(),
                "network_error".to_string(),
            ],
        }
    }
}

/// Configuration for agent execution in activities
///
/// NOTE: This struct is already generic and workflow-independent, using HashMap<String, String>
/// for input/output mappings. This is the preferred approach for standalone agent architecture.
/// The mappings can work with any context structure, not just workflow Resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivityConfig {
    pub agent_id: AgentId,
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
    pub required: bool,
    pub timeout_seconds: Option<u64>,
    pub retry_config: Option<AgentRetryConfig>,
}

// =============================================================================
// WORKFLOW-SPECIFIC TYPES (TO BE MOVED TO INTEGRATION LAYER)
// =============================================================================
//
// The following types are tightly coupled to the Petri net workflow engine
// and will be moved to the workflow integration bridge layer in Phase 2.
// They are kept here temporarily to maintain compilation during refactoring.
//
// DEPRECATION NOTICE: These types will be moved to src/integration/workflow_bridge.rs
// in Phase 2 of the standalone agent refactoring.
//
// =============================================================================

/// Scheduling configuration for place agents
///
/// DEPRECATED: This will be moved to the workflow integration layer
#[deprecated(note = "Will be moved to workflow integration layer in Phase 2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAgentSchedule {
    pub initial_delay_seconds: Option<u64>,
    pub interval_seconds: Option<u64>,
    pub max_executions: Option<u32>,
}

/// Configuration for running agents on tokens in specific places
/// Configuration for agents that monitor specific states
///
/// DEPRECATED: This workflow-specific type will be moved to the workflow integration layer
#[deprecated(note = "Will be moved to workflow integration layer in Phase 2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAgentConfig {
    pub id: Uuid,
    pub state_id: StateId,
    pub agent_id: AgentId,
    pub llm_config: Option<LLMConfig>,
    pub trigger_conditions: Vec<Rule>,
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
    pub auto_activity: Option<ActivityId>,
    pub schedule: Option<StateAgentSchedule>,
    pub retry_config: Option<AgentRetryConfig>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DEPRECATED: This impl will be moved to the workflow integration layer
#[allow(deprecated)]
impl StateAgentConfig {
    pub fn new(state_id: StateId, agent_id: AgentId) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            state_id,
            agent_id,
            llm_config: None,
            trigger_conditions: vec![],
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
            auto_activity: None,
            schedule: None,
            retry_config: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Agent execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

impl std::fmt::Display for AgentExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentExecutionStatus::Pending => write!(f, "pending"),
            AgentExecutionStatus::Running => write!(f, "running"),
            AgentExecutionStatus::Completed => write!(f, "completed"),
            AgentExecutionStatus::Failed => write!(f, "failed"),
            AgentExecutionStatus::Timeout => write!(f, "timeout"),
            AgentExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Agent execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    pub id: Uuid,
    pub agent_id: AgentId,
    pub context: serde_json::Value,
    pub config_id: Option<Uuid>,
    pub status: AgentExecutionStatus,
    pub input_data: serde_json::Value,
    pub output_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub retry_count: u32,
}

impl AgentExecution {
    pub fn new(
        agent_id: AgentId,
        context: serde_json::Value,
        input_data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            context,
            config_id: None,
            status: AgentExecutionStatus::Pending,
            input_data,
            output_data: None,
            error_message: None,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            retry_count: 0,
        }
    }

    pub fn start(&mut self) {
        self.status = AgentExecutionStatus::Running;
        self.started_at = Utc::now();
    }

    pub fn complete(&mut self, output: serde_json::Value) {
        self.status = AgentExecutionStatus::Completed;
        self.output_data = Some(output);
        let now = Utc::now();
        self.completed_at = Some(now);
        self.duration_ms = Some((now - self.started_at).num_milliseconds() as u64);
    }

    pub fn fail(&mut self, error: String) {
        self.status = AgentExecutionStatus::Failed;
        self.error_message = Some(error);
        let now = Utc::now();
        self.completed_at = Some(now);
        self.duration_ms = Some((now - self.started_at).num_milliseconds() as u64);
    }

    /// Get a value from the execution context
    pub fn get_context_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    /// Set a value in the execution context
    pub fn set_context_value(&mut self, key: String, value: serde_json::Value) {
        if let serde_json::Value::Object(ref mut map) = self.context {
            map.insert(key, value);
        }
    }

    /// Get a nested value from the execution context using dot notation
    pub fn get_nested_context_value(&self, path: &str) -> Option<&serde_json::Value> {
        let keys: Vec<&str> = path.split('.').collect();
        let mut current = &self.context;

        for key in keys {
            current = current.get(key)?;
        }

        Some(current)
    }

    /// Set a nested value in the execution context using dot notation
    pub fn set_nested_context_value(&mut self, path: &str, value: serde_json::Value) {
        let keys: Vec<&str> = path.split('.').collect();
        if keys.is_empty() {
            return;
        }

        // Ensure context is an object
        if !self.context.is_object() {
            self.context = serde_json::json!({});
        }

        let mut current = &mut self.context;
        let last_key = keys.last().unwrap();

        // Navigate to the parent of the final key
        for key in keys.iter().take(keys.len() - 1) {
            if let serde_json::Value::Object(ref mut map) = current {
                let entry = map
                    .entry(key.to_string())
                    .or_insert_with(|| serde_json::json!({}));
                if !entry.is_object() {
                    *entry = serde_json::json!({});
                }
                current = entry;
            }
        }

        // Set the final value
        if let serde_json::Value::Object(ref mut map) = current {
            map.insert(last_key.to_string(), value);
        }
    }

    /// Create a new execution with a default empty context
    pub fn new_with_empty_context(agent_id: AgentId, input_data: serde_json::Value) -> Self {
        Self::new(agent_id, serde_json::json!({}), input_data)
    }
}

/// Stream events for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStreamEvent {
    ContentChunk {
        execution_id: Uuid,
        chunk: String,
        sequence: u32,
        context: Option<serde_json::Value>,
    },
    ThinkingStatus {
        execution_id: Uuid,
        status: String,
        context: Option<serde_json::Value>,
    },
    ToolCall {
        execution_id: Uuid,
        tool_name: String,
        arguments: serde_json::Value,
        context: Option<serde_json::Value>,
    },
    ToolResult {
        execution_id: Uuid,
        tool_name: String,
        result: serde_json::Value,
        context: Option<serde_json::Value>,
    },
    Completed {
        execution_id: Uuid,
        final_response: serde_json::Value,
        usage: Option<serde_json::Value>,
        context: Option<serde_json::Value>,
    },
    Failed {
        execution_id: Uuid,
        error: String,
        context: Option<serde_json::Value>,
    },
}

/// Conversation record for agent interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub agent_id: AgentId,
    pub token_id: Uuid,
    pub messages: Vec<ConversationMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Individual message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// Message role in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_agent_execution_context_creation() {
        let agent_id = AgentId::from("test-agent");
        let context = json!({
            "resource_id": "123",
            "state_id": "pending",
            "user_id": "user123"
        });
        let input_data = json!({"message": "test input"});

        let execution = AgentExecution::new(agent_id, context.clone(), input_data.clone());

        assert_eq!(execution.agent_id.as_str(), "test-agent");
        assert_eq!(execution.context, context);
        assert_eq!(execution.input_data, input_data);
        assert_eq!(execution.status, AgentExecutionStatus::Pending);
    }

    #[test]
    fn test_context_value_operations() {
        let agent_id = AgentId::from("test-agent");
        let context = json!({
            "resource_id": "123",
            "state_id": "pending"
        });
        let input_data = json!({});

        let mut execution = AgentExecution::new(agent_id, context, input_data);

        // Test getting existing values
        assert_eq!(
            execution.get_context_value("resource_id"),
            Some(&json!("123"))
        );
        assert_eq!(
            execution.get_context_value("state_id"),
            Some(&json!("pending"))
        );
        assert_eq!(execution.get_context_value("nonexistent"), None);

        // Test setting new values
        execution.set_context_value("user_id".to_string(), json!("user456"));
        assert_eq!(
            execution.get_context_value("user_id"),
            Some(&json!("user456"))
        );
    }

    #[test]
    fn test_nested_context_operations() {
        let agent_id = AgentId::from("test-agent");
        let context = json!({
            "workflow": {
                "resource_id": "123",
                "state_id": "pending"
            }
        });
        let input_data = json!({});

        let mut execution = AgentExecution::new(agent_id, context, input_data);

        // Test getting nested values
        assert_eq!(
            execution.get_nested_context_value("workflow.resource_id"),
            Some(&json!("123"))
        );
        assert_eq!(
            execution.get_nested_context_value("workflow.state_id"),
            Some(&json!("pending"))
        );
        assert_eq!(
            execution.get_nested_context_value("workflow.nonexistent"),
            None
        );

        // Test setting nested values
        execution.set_nested_context_value("workflow.user_id", json!("user789"));
        assert_eq!(
            execution.get_nested_context_value("workflow.user_id"),
            Some(&json!("user789"))
        );

        // Test creating deep nested structure
        execution.set_nested_context_value("deep.nested.value", json!("deep_value"));
        assert_eq!(
            execution.get_nested_context_value("deep.nested.value"),
            Some(&json!("deep_value"))
        );
    }

    #[test]
    fn test_new_with_empty_context() {
        let agent_id = AgentId::from("test-agent");
        let input_data = json!({"message": "test"});

        let execution = AgentExecution::new_with_empty_context(agent_id, input_data.clone());

        assert_eq!(execution.agent_id.as_str(), "test-agent");
        assert_eq!(execution.context, json!({}));
        assert_eq!(execution.input_data, input_data);
        assert_eq!(execution.status, AgentExecutionStatus::Pending);
    }

    #[test]
    fn test_execution_status_transitions() {
        let agent_id = AgentId::from("test-agent");
        let context = json!({});
        let input_data = json!({});

        let mut execution = AgentExecution::new(agent_id, context, input_data);

        // Test starting execution
        execution.start();
        assert_eq!(execution.status, AgentExecutionStatus::Running);

        // Test completing execution
        let output = json!({"result": "success"});
        execution.complete(output.clone());
        assert_eq!(execution.status, AgentExecutionStatus::Completed);
        assert_eq!(execution.output_data, Some(output));
        assert!(execution.completed_at.is_some());
        assert!(execution.duration_ms.is_some());
    }

    #[test]
    fn test_execution_failure() {
        let agent_id = AgentId::from("test-agent");
        let context = json!({});
        let input_data = json!({});

        let mut execution = AgentExecution::new(agent_id, context, input_data);

        execution.start();
        execution.fail("Test error".to_string());

        assert_eq!(execution.status, AgentExecutionStatus::Failed);
        assert_eq!(execution.error_message, Some("Test error".to_string()));
        assert!(execution.completed_at.is_some());
        assert!(execution.duration_ms.is_some());
    }
}
