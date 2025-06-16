use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{ActivityId, Rule, StateId};

/// Unique identifier for an AI agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

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
            max_tokens: Some(1000),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivityConfig {
    pub agent_id: AgentId,
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
    pub required: bool,
    pub timeout_seconds: Option<u64>,
    pub retry_config: Option<AgentRetryConfig>,
}

/// Scheduling configuration for place agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAgentSchedule {
    pub initial_delay_seconds: Option<u64>,
    pub interval_seconds: Option<u64>,
    pub max_executions: Option<u32>,
}

/// Configuration for running agents on tokens in specific places
/// Configuration for agents that monitor specific states
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

/// Agent execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    pub id: Uuid,
    pub agent_id: AgentId,
    pub resource_id: Uuid,
    pub state_id: StateId,
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
        resource_id: Uuid,
        state_id: StateId,
        input_data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            resource_id,
            state_id,
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
}

/// Stream events for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStreamEvent {
    ContentChunk {
        execution_id: Uuid,
        chunk: String,
        sequence: u32,
    },
    ThinkingStatus {
        execution_id: Uuid,
        status: String,
    },
    ToolCall {
        execution_id: Uuid,
        tool_name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        execution_id: Uuid,
        tool_name: String,
        result: serde_json::Value,
    },
    Completed {
        execution_id: Uuid,
        final_response: serde_json::Value,
        usage: Option<serde_json::Value>,
    },
    Failed {
        execution_id: Uuid,
        error: String,
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
