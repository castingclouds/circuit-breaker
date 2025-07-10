// GraphQL API for the Circuit Breaker engine
// This provides a GraphQL interface for defining and executing State Managed Workflows

use async_graphql::{Context, Enum, InputObject, Object, Schema, SimpleObject, Subscription, ID};
use chrono::Utc;
use serde_json;
use tracing::debug;
use uuid::Uuid;

use crate::engine::rules::StoredRule;
use crate::engine::storage::WorkflowStorage;
use crate::engine::{AgentEngine, AgentStorage};
use crate::models::{
    ActivityDefinition, ActivityId, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentPrompts, AgentRetryConfig, HistoryEvent, LLMConfig, LLMProvider, Resource,
    ResourceMetadata, Rule, RuleCondition, StateAgentConfig, StateAgentSchedule, StateId,
    WorkflowDefinition,
};

// GraphQL types - these are the API representations of our domain models

#[derive(SimpleObject, Debug, Clone)]
pub struct WorkflowGQL {
    pub id: ID,
    pub name: String,
    pub states: Vec<String>,
    pub activities: Vec<ActivityGQL>,
    pub initial_state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ActivityGQL {
    pub id: String,
    pub name: Option<String>,
    pub from_states: Vec<String>,
    pub to_state: String,
    pub conditions: Vec<String>,
    pub description: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ResourceGQL {
    pub id: ID,
    pub workflow_id: String,
    pub state: String,
    pub data: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub history: Vec<HistoryEventGQL>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct HistoryEventGQL {
    pub timestamp: String,
    pub activity: String,
    pub from_state: String,
    pub to_state: String,
    pub data: Option<serde_json::Value>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ActivityRecordGQL {
    pub from_state: String,
    pub to_state: String,
    pub activity_id: String,
    pub timestamp: String,
    pub triggered_by: Option<String>,
    pub nats_sequence: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct NATSResourceGQL {
    pub id: ID,
    pub workflow_id: String,
    pub state: String,
    pub data: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub history: Vec<HistoryEventGQL>,
    pub nats_sequence: Option<String>,
    pub nats_timestamp: Option<String>,
    pub nats_subject: Option<String>,
    pub activity_history: Vec<ActivityRecordGQL>,
}

// Agent-related GraphQL types
#[derive(SimpleObject, Debug, Clone)]
pub struct AgentDefinitionGQL {
    pub id: String,
    pub name: String,
    pub description: String,
    pub llm_provider: AgentLLMProviderGQL,
    pub llm_config: LLMConfigGQL,
    pub prompts: AgentPromptsGQL,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct AgentLLMProviderGQL {
    pub provider_type: String,
    pub model: String,
    pub base_url: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMConfigGQL {
    pub temperature: f64,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop_sequences: Vec<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct AgentPromptsGQL {
    pub system: String,
    pub user_template: String,
    pub context_instructions: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct StateAgentConfigGQL {
    pub id: String,
    pub state_id: String,
    pub agent_id: String,
    pub llm_config: Option<LLMConfigGQL>,
    pub input_mapping: serde_json::Value,
    pub output_mapping: serde_json::Value,
    pub auto_activity: Option<String>,
    pub schedule: Option<StateAgentScheduleGQL>,
    pub retry_config: Option<AgentRetryConfigGQL>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct StateAgentScheduleGQL {
    pub initial_delay_seconds: Option<i32>,
    pub interval_seconds: Option<i32>,
    pub max_executions: Option<i32>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct AgentRetryConfigGQL {
    pub max_attempts: i32,
    pub backoff_seconds: i32,
    pub retry_on_errors: Vec<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct AgentExecutionGQL {
    pub id: String,
    pub agent_id: String,
    pub resource_id: String,
    pub state_id: String,
    pub status: AgentExecutionStatusGQL,
    pub input_data: serde_json::Value,
    pub output_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i32>,
    pub retry_count: i32,
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionStatusGQL {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct CampaignGQL {
    pub id: ID,
    pub name: String,
    pub workflow_id: String,
    pub agents: Vec<AgentGQL>,
    pub communication_pattern: CommunicationPattern,
    pub status: CampaignStatus,
    pub created_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct AgentGQL {
    pub id: ID,
    pub name: String,
    pub agent_type: String,
    pub configuration: serde_json::Value,
    pub status: AgentStatus,
}

// LLM Router GraphQL Types
#[derive(SimpleObject, Debug, Clone)]
pub struct LLMProviderGQL {
    pub id: ID,
    pub provider_type: String,
    pub name: String,
    pub base_url: String,
    pub models: Vec<LLMModelGQL>,
    pub health_status: LLMProviderHealthGQL,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMModelGQL {
    pub id: String,
    pub name: String,
    pub max_tokens: i32,
    pub context_window: i32,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub capabilities: Vec<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMProviderHealthGQL {
    pub is_healthy: bool,
    pub last_check: String,
    pub error_rate: f64,
    pub average_latency_ms: i32,
    pub consecutive_failures: i32,
    pub last_error: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMRequestGQL {
    pub id: ID,
    pub model: String,
    pub messages: Vec<ChatMessageGQL>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub stream: bool,
    pub user: Option<String>,
    pub project_id: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ChatMessageGQL {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMResponseGQL {
    pub id: String,
    pub model: String,
    pub choices: Vec<LLMChoiceGQL>,
    pub usage: TokenUsageGQL,
    pub provider: String,
    pub routing_info: RoutingInfoGQL,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMChoiceGQL {
    pub index: i32,
    pub message: ChatMessageGQL,
    pub finish_reason: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct TokenUsageGQL {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub estimated_cost: f64,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct RoutingInfoGQL {
    pub selected_provider: String,
    pub routing_strategy: String,
    pub latency_ms: i32,
    pub retry_count: i32,
    pub fallback_used: bool,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct CostInfoGQL {
    pub request_id: ID,
    pub provider: String,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cost_usd: f64,
    pub timestamp: String,
    pub user_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct BudgetStatusGQL {
    pub budget_id: String,
    pub limit: f64,
    pub used: f64,
    pub percentage_used: f64,
    pub is_exhausted: bool,
    pub is_warning: bool,
    pub remaining: f64,
    pub message: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct CostAnalyticsGQL {
    pub total_cost: f64,
    pub total_tokens: i32,
    pub average_cost_per_token: f64,
    pub provider_breakdown: serde_json::Value,
    pub model_breakdown: serde_json::Value,
    pub daily_costs: serde_json::Value,
    pub period_start: String,
    pub period_end: String,
}

// Input types for mutations
#[derive(InputObject, Debug)]
pub struct WorkflowDefinitionInput {
    pub name: String,
    pub states: Vec<String>,
    pub activities: Vec<ActivityDefinitionInput>,
    pub initial_state: String,
    pub description: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct ActivityDefinitionInput {
    pub id: String,
    pub name: Option<String>,
    pub from_states: Vec<String>,
    pub to_state: String,
    pub conditions: Vec<String>,
    pub description: Option<String>,
}

// LLM Router Input Types
#[derive(InputObject, Debug)]
pub struct LLMChatCompletionInput {
    pub model: String,
    pub messages: Vec<ChatMessageInput>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub user: Option<String>,
    pub project_id: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct ChatMessageInput {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct LLMProviderConfigInput {
    pub provider_type: String,
    pub name: String,
    pub base_url: String,
    pub api_key_id: String,
    pub models: Vec<LLMModelInput>,
}

#[derive(InputObject, Debug)]
pub struct LLMModelInput {
    pub id: String,
    pub name: String,
    pub max_tokens: i32,
    pub context_window: i32,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub capabilities: Vec<String>,
}

#[derive(InputObject, Debug)]
pub struct BudgetInput {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub limit: f64,
    pub period: String,
    pub warning_threshold: f64,
}

#[derive(InputObject, Debug)]
pub struct CostAnalyticsInput {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub start_date: String,
    pub end_date: String,
}

#[derive(InputObject, Debug)]
pub struct ResourceCreateInput {
    pub workflow_id: String,
    pub initial_state: Option<String>,
    pub data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(InputObject, Debug)]
pub struct ActivityExecuteInput {
    pub resource_id: String,
    pub activity_id: String,
    pub data: Option<serde_json::Value>,
}

#[derive(InputObject, Debug)]
pub struct CampaignCreateInput {
    pub name: String,
    pub workflow_id: String,
    pub agents: Vec<AgentCreateInput>,
    pub communication_pattern: CommunicationPattern,
}

#[derive(InputObject, Debug)]
pub struct AgentCreateInput {
    pub name: String,
    pub agent_type: String,
    pub configuration: serde_json::Value,
}

// Agent-related input types
#[derive(InputObject, Debug)]
pub struct AgentDefinitionInput {
    pub name: String,
    pub description: String,
    pub llm_provider: AgentLLMProviderInput,
    pub llm_config: LLMConfigInput,
    pub prompts: AgentPromptsInput,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
}

#[derive(InputObject, Debug)]
pub struct AgentLLMProviderInput {
    pub provider_type: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct LLMConfigInput {
    pub temperature: f64,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop_sequences: Vec<String>,
}

#[derive(InputObject, Debug)]
pub struct AgentPromptsInput {
    pub system: String,
    pub user_template: String,
    pub context_instructions: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct StateAgentConfigInput {
    pub state_id: String,
    pub agent_id: String,
    pub llm_config: Option<LLMConfigInput>,
    pub input_mapping: serde_json::Value,
    pub output_mapping: serde_json::Value,
    pub auto_activity: Option<String>,
    pub schedule: Option<StateAgentScheduleInput>,
    pub retry_config: Option<AgentRetryConfigInput>,
    pub enabled: bool,
}

#[derive(InputObject, Debug)]
pub struct StateAgentScheduleInput {
    pub initial_delay_seconds: Option<i32>,
    pub interval_seconds: Option<i32>,
    pub max_executions: Option<i32>,
}

#[derive(InputObject, Debug)]
pub struct AgentRetryConfigInput {
    pub max_attempts: i32,
    pub backoff_seconds: i32,
    pub retry_on_errors: Vec<String>,
}

#[derive(InputObject, Debug)]
pub struct TriggerStateAgentsInput {
    pub resource_id: String,
}

// Rule types for GraphQL
#[derive(SimpleObject, Debug, Clone)]
pub struct RuleGQL {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: RuleConditionGQL,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct RuleConditionGQL {
    pub condition_type: String,
    pub field: Option<String>,
    pub value: Option<serde_json::Value>,
    pub substring: Option<String>,
    pub rules: Option<Vec<RuleGQL>>,
    pub rule: Option<Box<RuleGQL>>,
    pub script: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct RuleEvaluationResultGQL {
    pub rule_id: String,
    pub passed: bool,
    pub reason: String,
    pub details: Option<serde_json::Value>,
    pub sub_results: Vec<RuleEvaluationResultGQL>,
}

// Rule input types
#[derive(InputObject, Debug)]
pub struct RuleInput {
    pub name: String,
    pub description: String,
    pub condition: RuleConditionInput,
    pub tags: Option<Vec<String>>,
}

#[derive(InputObject, Debug)]
pub struct RuleConditionInput {
    pub condition_type: String,
    pub field: Option<String>,
    pub value: Option<serde_json::Value>,
    pub substring: Option<String>,
    pub rules: Option<Vec<RuleConditionInput>>,
    pub rule: Option<Box<RuleConditionInput>>,
    pub script: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct RuleEvaluationInput {
    pub rule_id: String,
    pub data: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

// NATS-specific input types
#[derive(InputObject, Debug)]
pub struct CreateWorkflowInstanceInput {
    pub workflow_id: String,
    pub initial_data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub triggered_by: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct ExecuteActivityWithNATSInput {
    pub resource_id: String,
    pub activity_id: String,
    pub new_state: String,
    pub triggered_by: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(InputObject, Debug)]
pub struct ResourcesInStateInput {
    pub workflow_id: String,
    pub state_id: String,
}

// Enums
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicationPattern {
    Serial,
    Parallel,
    Hybrid,
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Running,
    Waiting,
    Completed,
    Failed,
}

// Conversion functions from domain models to GraphQL types
impl From<&WorkflowDefinition> for WorkflowGQL {
    fn from(workflow: &WorkflowDefinition) -> Self {
        WorkflowGQL {
            id: ID(workflow.id.clone()),
            name: workflow.name.clone(),
            states: workflow
                .states
                .iter()
                .map(|s| s.as_str().to_string())
                .collect(),
            activities: workflow.activities.iter().map(|a| a.into()).collect(),
            initial_state: workflow.initial_state.as_str().to_string(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

// Agent type conversions
impl From<&AgentDefinition> for AgentDefinitionGQL {
    fn from(agent: &AgentDefinition) -> Self {
        AgentDefinitionGQL {
            id: agent.id.as_str().to_string(),
            name: agent.name.clone(),
            description: agent.description.clone(),
            llm_provider: AgentLLMProviderGQL::from(&agent.llm_provider),
            llm_config: LLMConfigGQL::from(&agent.llm_config),
            prompts: AgentPromptsGQL::from(&agent.prompts),
            capabilities: agent.capabilities.clone(),
            tools: agent.tools.clone(),
            created_at: agent.created_at.to_rfc3339(),
            updated_at: agent.updated_at.to_rfc3339(),
        }
    }
}

impl From<&LLMProvider> for AgentLLMProviderGQL {
    fn from(provider: &LLMProvider) -> Self {
        match provider {
            LLMProvider::OpenAI {
                model, base_url, ..
            } => AgentLLMProviderGQL {
                provider_type: "openai".to_string(),
                model: model.clone(),
                base_url: base_url.clone(),
            },
            LLMProvider::Anthropic { model, .. } => AgentLLMProviderGQL {
                provider_type: "anthropic".to_string(),
                model: model.clone(),
                base_url: None,
            },
            LLMProvider::Google { model, .. } => AgentLLMProviderGQL {
                provider_type: "google".to_string(),
                model: model.clone(),
                base_url: None,
            },
            LLMProvider::Ollama {
                model, base_url, ..
            } => AgentLLMProviderGQL {
                provider_type: "ollama".to_string(),
                model: model.clone(),
                base_url: Some(base_url.clone()),
            },
            LLMProvider::Custom {
                model, endpoint, ..
            } => AgentLLMProviderGQL {
                provider_type: "custom".to_string(),
                model: model.clone(),
                base_url: Some(endpoint.clone()),
            },
        }
    }
}

impl From<&LLMConfig> for LLMConfigGQL {
    fn from(config: &LLMConfig) -> Self {
        LLMConfigGQL {
            temperature: config.temperature as f64,
            max_tokens: config.max_tokens.map(|t| t as i32),
            top_p: config.top_p.map(|p| p as f64),
            frequency_penalty: config.frequency_penalty.map(|p| p as f64),
            presence_penalty: config.presence_penalty.map(|p| p as f64),
            stop_sequences: config.stop_sequences.clone(),
        }
    }
}

impl From<&AgentPrompts> for AgentPromptsGQL {
    fn from(prompts: &AgentPrompts) -> Self {
        AgentPromptsGQL {
            system: prompts.system.clone(),
            user_template: prompts.user_template.clone(),
            context_instructions: prompts.context_instructions.clone(),
        }
    }
}

impl From<&StateAgentConfig> for StateAgentConfigGQL {
    fn from(config: &StateAgentConfig) -> Self {
        StateAgentConfigGQL {
            id: config.id.to_string(),
            state_id: config.state_id.as_str().to_string(),
            agent_id: config.agent_id.as_str().to_string(),
            llm_config: config.llm_config.as_ref().map(LLMConfigGQL::from),
            input_mapping: serde_json::to_value(&config.input_mapping).unwrap_or_default(),
            output_mapping: serde_json::to_value(&config.output_mapping).unwrap_or_default(),
            auto_activity: config
                .auto_activity
                .as_ref()
                .map(|t| t.as_str().to_string()),
            schedule: config.schedule.as_ref().map(StateAgentScheduleGQL::from),
            retry_config: config.retry_config.as_ref().map(AgentRetryConfigGQL::from),
            enabled: config.enabled,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}

impl From<&StateAgentSchedule> for StateAgentScheduleGQL {
    fn from(schedule: &StateAgentSchedule) -> Self {
        StateAgentScheduleGQL {
            initial_delay_seconds: schedule.initial_delay_seconds.map(|d| d as i32),
            interval_seconds: schedule.interval_seconds.map(|i| i as i32),
            max_executions: schedule.max_executions.map(|e| e as i32),
        }
    }
}

impl From<&AgentRetryConfig> for AgentRetryConfigGQL {
    fn from(config: &AgentRetryConfig) -> Self {
        AgentRetryConfigGQL {
            max_attempts: config.max_attempts as i32,
            backoff_seconds: config.backoff_seconds as i32,
            retry_on_errors: config.retry_on_errors.clone(),
        }
    }
}

impl From<&AgentExecution> for AgentExecutionGQL {
    fn from(execution: &AgentExecution) -> Self {
        AgentExecutionGQL {
            id: execution.id.to_string(),
            agent_id: execution.agent_id.as_str().to_string(),
            resource_id: execution
                .get_context_value("resource_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            state_id: execution
                .get_context_value("state_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            status: AgentExecutionStatusGQL::from(&execution.status),
            input_data: execution.input_data.clone(),
            output_data: execution.output_data.clone(),
            error_message: execution.error_message.clone(),
            started_at: execution.started_at.to_rfc3339(),
            completed_at: execution.completed_at.map(|t| t.to_rfc3339()),
            duration_ms: execution.duration_ms.map(|d| d as i32),
            retry_count: execution.retry_count as i32,
        }
    }
}

impl From<&AgentExecutionStatus> for AgentExecutionStatusGQL {
    fn from(status: &AgentExecutionStatus) -> Self {
        match status {
            AgentExecutionStatus::Pending => AgentExecutionStatusGQL::Pending,
            AgentExecutionStatus::Running => AgentExecutionStatusGQL::Running,
            AgentExecutionStatus::Completed => AgentExecutionStatusGQL::Completed,
            AgentExecutionStatus::Failed => AgentExecutionStatusGQL::Failed,
            AgentExecutionStatus::Timeout => AgentExecutionStatusGQL::Timeout,
            AgentExecutionStatus::Cancelled => AgentExecutionStatusGQL::Cancelled,
        }
    }
}

impl From<&ActivityDefinition> for ActivityGQL {
    fn from(activity: &ActivityDefinition) -> Self {
        ActivityGQL {
            id: activity.id.as_str().to_string(),
            name: None,
            from_states: activity
                .from_states
                .iter()
                .map(|s| s.as_str().to_string())
                .collect(),
            to_state: activity.to_state.as_str().to_string(),
            conditions: activity.conditions.clone(),
            description: None,
        }
    }
}

impl From<&Resource> for ResourceGQL {
    fn from(resource: &Resource) -> Self {
        ResourceGQL {
            id: ID(resource.id.to_string()),
            workflow_id: resource.workflow_id.clone(),
            state: resource.state.as_str().to_string(),
            data: resource.data.clone(),
            metadata: serde_json::to_value(&resource.metadata).unwrap_or_default(),
            created_at: resource.created_at.to_rfc3339(),
            updated_at: resource.updated_at.to_rfc3339(),
            history: resource.history.iter().map(|h| h.into()).collect(),
        }
    }
}

impl From<&HistoryEvent> for HistoryEventGQL {
    fn from(event: &HistoryEvent) -> Self {
        HistoryEventGQL {
            timestamp: event.timestamp.to_rfc3339(),
            activity: event.activity.as_str().to_string(),
            from_state: event.from.as_str().to_string(),
            to_state: event.to.as_str().to_string(),
            data: event.data.clone(),
        }
    }
}

impl From<&crate::models::ActivityRecord> for ActivityRecordGQL {
    fn from(record: &crate::models::ActivityRecord) -> Self {
        ActivityRecordGQL {
            from_state: record.from_state.as_str().to_string(),
            to_state: record.to_state.as_str().to_string(),
            activity_id: record.activity_id.as_str().to_string(),
            timestamp: record.timestamp.to_rfc3339(),
            triggered_by: record.triggered_by.clone(),
            nats_sequence: record.nats_sequence.map(|s| s.to_string()),
            metadata: record.metadata.clone(),
        }
    }
}

impl From<&Resource> for NATSResourceGQL {
    fn from(resource: &Resource) -> Self {
        NATSResourceGQL {
            id: ID(resource.id.to_string()),
            workflow_id: resource.workflow_id.clone(),
            state: resource.state.as_str().to_string(),
            data: resource.data.clone(),
            metadata: serde_json::to_value(&resource.metadata).unwrap_or_default(),
            created_at: resource.created_at.to_rfc3339(),
            updated_at: resource.updated_at.to_rfc3339(),
            history: resource.history.iter().map(|h| h.into()).collect(),
            nats_sequence: resource.nats_sequence.map(|s| s.to_string()),
            nats_timestamp: resource.nats_timestamp.map(|ts| ts.to_rfc3339()),
            nats_subject: resource.nats_subject.clone(),
            activity_history: resource.activity_history.iter().map(|r| r.into()).collect(),
        }
    }
}

// Rule conversion implementations
impl From<&StoredRule> for RuleGQL {
    fn from(rule: &StoredRule) -> Self {
        RuleGQL {
            id: rule.id.clone(),
            name: rule.name.clone(),
            description: rule.description.clone(),
            condition: RuleConditionGQL::from(&rule.condition),
            version: rule.version,
            created_at: rule.created_at.to_rfc3339(),
            updated_at: rule.updated_at.to_rfc3339(),
            created_by: rule.created_by.clone(),
            tags: rule.tags.clone(),
        }
    }
}

impl From<&RuleCondition> for RuleConditionGQL {
    fn from(condition: &RuleCondition) -> Self {
        match condition {
            RuleCondition::FieldExists { field } => RuleConditionGQL {
                condition_type: "FieldExists".to_string(),
                field: Some(field.clone()),
                value: None,
                substring: None,
                rules: None,
                rule: None,
                script: None,
            },
            RuleCondition::FieldEquals { field, value } => RuleConditionGQL {
                condition_type: "FieldEquals".to_string(),
                field: Some(field.clone()),
                value: Some(value.clone()),
                substring: None,
                rules: None,
                rule: None,
                script: None,
            },
            RuleCondition::FieldGreaterThan { field, value } => RuleConditionGQL {
                condition_type: "FieldGreaterThan".to_string(),
                field: Some(field.clone()),
                value: Some(serde_json::Value::Number(
                    serde_json::Number::from_f64(*value).unwrap(),
                )),
                substring: None,
                rules: None,
                rule: None,
                script: None,
            },
            RuleCondition::FieldLessThan { field, value } => RuleConditionGQL {
                condition_type: "FieldLessThan".to_string(),
                field: Some(field.clone()),
                value: Some(serde_json::Value::Number(
                    serde_json::Number::from_f64(*value).unwrap(),
                )),
                substring: None,
                rules: None,
                rule: None,
                script: None,
            },
            RuleCondition::FieldContains { field, substring } => RuleConditionGQL {
                condition_type: "FieldContains".to_string(),
                field: Some(field.clone()),
                value: None,
                substring: Some(substring.clone()),
                rules: None,
                rule: None,
                script: None,
            },
            RuleCondition::And { rules: _rules } => RuleConditionGQL {
                condition_type: "And".to_string(),
                field: None,
                value: None,
                substring: None,
                rules: None, // Nested rules not fully supported yet
                rule: None,
                script: None,
            },
            RuleCondition::Or { rules: _rules } => RuleConditionGQL {
                condition_type: "Or".to_string(),
                field: None,
                value: None,
                substring: None,
                rules: None, // Nested rules not fully supported yet
                rule: None,
                script: None,
            },
            RuleCondition::Not { rule } => RuleConditionGQL {
                condition_type: "Not".to_string(),
                field: None,
                value: None,
                substring: None,
                rules: None,
                rule: None, // Nested rules not fully supported in StoredRule context
                script: None,
            },
            RuleCondition::Expression { script } => RuleConditionGQL {
                condition_type: "Expression".to_string(),
                field: None,
                value: None,
                substring: None,
                rules: None,
                rule: None,
                script: Some(script.clone()),
            },
        }
    }
}

impl From<RuleInput> for Rule {
    fn from(input: RuleInput) -> Self {
        Rule {
            id: uuid::Uuid::new_v4().to_string(),
            description: input.description,
            condition: RuleCondition::from(input.condition),
        }
    }
}

impl From<RuleConditionInput> for RuleCondition {
    fn from(input: RuleConditionInput) -> Self {
        match input.condition_type.as_str() {
            "FieldExists" => RuleCondition::FieldExists {
                field: input.field.unwrap_or_default(),
            },
            "FieldEquals" => RuleCondition::FieldEquals {
                field: input.field.unwrap_or_default(),
                value: input.value.unwrap_or(serde_json::Value::Null),
            },
            "FieldGreaterThan" => RuleCondition::FieldGreaterThan {
                field: input.field.unwrap_or_default(),
                value: input.value.and_then(|v| v.as_f64()).unwrap_or(0.0),
            },
            "FieldLessThan" => RuleCondition::FieldLessThan {
                field: input.field.unwrap_or_default(),
                value: input.value.and_then(|v| v.as_f64()).unwrap_or(0.0),
            },
            "FieldContains" => RuleCondition::FieldContains {
                field: input.field.unwrap_or_default(),
                substring: input.substring.unwrap_or_default(),
            },
            "And" => RuleCondition::And {
                rules: input
                    .rules
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| Rule {
                        id: uuid::Uuid::new_v4().to_string(),
                        description: "Nested rule".to_string(),
                        condition: RuleCondition::from(r),
                    })
                    .collect(),
            },
            "Or" => RuleCondition::Or {
                rules: input
                    .rules
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| Rule {
                        id: uuid::Uuid::new_v4().to_string(),
                        description: "Nested rule".to_string(),
                        condition: RuleCondition::from(r),
                    })
                    .collect(),
            },
            "Not" => RuleCondition::Not {
                rule: Box::new(Rule {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: "Negated rule".to_string(),
                    condition: input.rule.map(|r| RuleCondition::from(*r)).unwrap_or(
                        RuleCondition::Expression {
                            script: "true".to_string(),
                        },
                    ),
                }),
            },
            "Expression" => RuleCondition::Expression {
                script: input.script.unwrap_or_default(),
            },
            _ => RuleCondition::Expression {
                script: "false".to_string(),
            },
        }
    }
}

// GraphQL Query root
pub struct Query;

#[Object]
impl Query {
    /// Get a workflow definition by ID
    async fn workflow(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<WorkflowGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        match storage.get_workflow(&id).await {
            Ok(Some(workflow)) => Ok(Some(WorkflowGQL::from(&workflow))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get workflow: {}",
                e
            ))),
        }
    }

    /// List all workflow definitions
    async fn workflows(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<WorkflowGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        match storage.list_workflows().await {
            Ok(workflows) => Ok(workflows.iter().map(WorkflowGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to list workflows: {}",
                e
            ))),
        }
    }

    /// Get a resource by ID
    async fn resource(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<ResourceGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        let resource_id = id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

        match storage.get_resource(&resource_id).await {
            Ok(Some(resource)) => Ok(Some(ResourceGQL::from(&resource))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get resource: {}",
                e
            ))),
        }
    }

    /// List resources, optionally filtered by workflow
    async fn resources(
        &self,
        ctx: &Context<'_>,
        workflow_id: Option<String>,
    ) -> async_graphql::Result<Vec<ResourceGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        match storage.list_resources(workflow_id.as_deref()).await {
            Ok(resources) => Ok(resources.iter().map(ResourceGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to list resources: {}",
                e
            ))),
        }
    }

    /// Get available activities for a resource
    async fn available_activities(
        &self,
        ctx: &Context<'_>,
        resource_id: String,
    ) -> async_graphql::Result<Vec<ActivityGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        let resource_uuid = resource_id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

        // Get the resource with retry logic for timing issues
        let mut resource = None;
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(100 * (2_u64.pow(attempt))))
                    .await;
            }

            match storage.get_resource(&resource_uuid).await {
                Ok(Some(found_resource)) => {
                    resource = Some(found_resource);
                    break;
                }
                Ok(None) => {
                    if attempt == 2 {
                        return Err(async_graphql::Error::new(
                            "Resource not found after retries",
                        ));
                    }
                    continue;
                }
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to get resource: {}",
                        e
                    )));
                }
            }
        }

        let resource = resource.unwrap();

        let workflow = storage
            .get_workflow(&resource.workflow_id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;

        let current_state = StateId::from(resource.current_state());
        let available = workflow.available_activities(&current_state);

        Ok(available.iter().map(|a| ActivityGQL::from(*a)).collect())
    }

    /// Get an agent by ID
    async fn agent(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<AgentDefinitionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let agent_id = AgentId::from(id);

        match agent_storage.get_agent(&agent_id).await {
            Ok(Some(agent)) => Ok(Some(AgentDefinitionGQL::from(&agent))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get agent: {}",
                e
            ))),
        }
    }

    /// List all agents
    async fn agents(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<AgentDefinitionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;

        match agent_storage.list_agents().await {
            Ok(agents) => Ok(agents.iter().map(AgentDefinitionGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to list agents: {}",
                e
            ))),
        }
    }

    /// Get state agent configurations for a specific state
    /// TEMPORARILY DISABLED: StateAgentConfig methods removed from AgentStorage trait
    /// Will be moved to workflow integration layer in Phase 2
    async fn state_agent_configs(
        &self,
        _ctx: &Context<'_>,
        _state_id: String,
    ) -> async_graphql::Result<Vec<StateAgentConfigGQL>> {
        // TODO: Re-implement in workflow integration layer
        Err(async_graphql::Error::new(
            "StateAgentConfig functionality temporarily disabled during standalone agent refactoring"
        ))
    }

    /// Get agent execution by ID
    async fn agent_execution(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<AgentExecutionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let execution_id = id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid execution ID format"))?;

        match agent_storage.get_execution(&execution_id).await {
            Ok(Some(execution)) => Ok(Some(AgentExecutionGQL::from(&execution))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get agent execution: {}",
                e
            ))),
        }
    }

    /// Get agent executions for a resource
    async fn resource_executions(
        &self,
        ctx: &Context<'_>,
        resource_id: String,
    ) -> async_graphql::Result<Vec<AgentExecutionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let resource_uuid = resource_id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

        match agent_storage
            .list_executions_for_resource(&resource_uuid)
            .await
        {
            Ok(executions) => Ok(executions.iter().map(AgentExecutionGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get resource executions: {}",
                e
            ))),
        }
    }

    /// NATS-specific queries for enhanced token operations

    /// Get resource with NATS metadata by ID
    async fn nats_resource(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<NATSResourceGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        let resource_id = id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

        match storage.get_resource(&resource_id).await {
            Ok(Some(resource)) => Ok(Some(NATSResourceGQL::from(&resource))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get NATS resource: {}",
                e
            ))),
        }
    }

    /// Get resources currently in a specific state (NATS-specific)
    async fn resources_in_state(
        &self,
        ctx: &Context<'_>,
        workflow_id: String,
        state_id: String,
    ) -> async_graphql::Result<Vec<NATSResourceGQL>> {
        // Try to get NATS storage for more efficient state-based queries
        if let Ok(nats_storage) =
            ctx.data::<std::sync::Arc<crate::engine::nats_storage::NATSStorage>>()
        {
            match nats_storage
                .get_resources_in_state(&workflow_id, &state_id)
                .await
            {
                Ok(resources) => Ok(resources.iter().map(NATSResourceGQL::from).collect()),
                Err(e) => Err(async_graphql::Error::new(format!(
                    "Failed to get resources in state: {}",
                    e
                ))),
            }
        } else {
            // Fallback to regular storage with filtering
            let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
            match storage.list_resources(Some(&workflow_id)).await {
                Ok(resources) => {
                    let filtered: Vec<NATSResourceGQL> = resources
                        .iter()
                        .filter(|resource| resource.state.as_str() == state_id)
                        .map(NATSResourceGQL::from)
                        .collect();
                    Ok(filtered)
                }
                Err(e) => Err(async_graphql::Error::new(format!(
                    "Failed to get resources in state: {}",
                    e
                ))),
            }
        }
    }

    /// Find resource by ID with workflow context (more efficient for NATS)
    async fn find_resource(
        &self,
        ctx: &Context<'_>,
        workflow_id: String,
        resource_id: String,
    ) -> async_graphql::Result<Option<NATSResourceGQL>> {
        let resource_uuid = resource_id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

        // Try NATS storage first for more efficient lookup
        if let Ok(nats_storage) =
            ctx.data::<std::sync::Arc<crate::engine::nats_storage::NATSStorage>>()
        {
            match nats_storage
                .get_resource_from_workflow(&resource_uuid, &workflow_id)
                .await
            {
                Ok(Some(resource)) => Ok(Some(NATSResourceGQL::from(&resource))),
                Ok(None) => Ok(None),
                Err(e) => Err(async_graphql::Error::new(format!(
                    "Failed to find resource: {}",
                    e
                ))),
            }
        } else {
            // Fallback to regular storage
            let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
            match storage.get_resource(&resource_uuid).await {
                Ok(Some(resource)) => {
                    if resource.workflow_id == workflow_id {
                        Ok(Some(NATSResourceGQL::from(&resource)))
                    } else {
                        Ok(None)
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => Err(async_graphql::Error::new(format!(
                    "Failed to find resource: {}",
                    e
                ))),
            }
        }
    }

    /// List all configured LLM providers
    async fn llm_providers(
        &self,
        _ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<LLMProviderGQL>> {
        // Create router and get providers
        let router = crate::llm::router::LLMRouter::new().await.map_err(|e| {
            async_graphql::Error::new(format!("Failed to initialize router: {}", e))
        })?;

        let providers = router.get_providers().await;
        let health_status = router.get_health_status().await;

        Ok(providers
            .into_iter()
            .map(|provider| {
                let health = health_status
                    .get(&provider.provider_type)
                    .cloned()
                    .unwrap_or_default();

                LLMProviderGQL {
                    id: ID(provider.id.to_string()),
                    provider_type: provider.provider_type.to_string(),
                    name: provider.name,
                    base_url: provider.base_url,
                    models: provider
                        .models
                        .into_iter()
                        .map(|model| LLMModelGQL {
                            id: model.id,
                            name: model.name,
                            max_tokens: model.max_tokens as i32,
                            context_window: model.context_window as i32,
                            cost_per_input_token: model.cost_per_input_token,
                            cost_per_output_token: model.cost_per_output_token,
                            supports_streaming: model.supports_streaming,
                            supports_function_calling: model.supports_function_calling,
                            capabilities: model
                                .capabilities
                                .into_iter()
                                .map(|c| format!("{:?}", c))
                                .collect(),
                        })
                        .collect(),
                    health_status: LLMProviderHealthGQL {
                        is_healthy: health.is_healthy,
                        last_check: health.last_check.to_rfc3339(),
                        error_rate: 0.0,
                        average_latency_ms: 0,
                        consecutive_failures: health.consecutive_failures as i32,
                        last_error: health.last_error,
                    },
                    created_at: provider.created_at.to_rfc3339(),
                    updated_at: provider.updated_at.to_rfc3339(),
                }
            })
            .collect())
    }

    /// Get LLM provider by ID
    async fn llm_provider(
        &self,
        _ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<LLMProviderGQL>> {
        // Mock implementation
        if id == "openai" {
            Ok(Some(LLMProviderGQL {
                id: ID("openai".to_string()),
                provider_type: "openai".to_string(),
                name: "OpenAI".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                models: vec![],
                health_status: LLMProviderHealthGQL {
                    is_healthy: true,
                    last_check: chrono::Utc::now().to_rfc3339(),
                    error_rate: 0.01,
                    average_latency_ms: 800,
                    consecutive_failures: 0,
                    last_error: None,
                },
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get budget status for user or project
    async fn budget_status(
        &self,
        _ctx: &Context<'_>,
        user_id: Option<String>,
        project_id: Option<String>,
    ) -> async_graphql::Result<BudgetStatusGQL> {
        // Mock implementation
        Ok(BudgetStatusGQL {
            budget_id: if let Some(pid) = project_id {
                format!("project:{}", pid)
            } else {
                format!("user:{}", user_id.unwrap_or_else(|| "default".to_string()))
            },
            limit: 100.0,
            used: 25.50,
            percentage_used: 0.255,
            is_exhausted: false,
            is_warning: false,
            remaining: 74.50,
            message: "Budget healthy: $25.50 of $100.00 used".to_string(),
        })
    }

    /// Get cost analytics for a time period
    async fn cost_analytics(
        &self,
        _ctx: &Context<'_>,
        input: CostAnalyticsInput,
    ) -> async_graphql::Result<CostAnalyticsGQL> {
        // Mock implementation
        Ok(CostAnalyticsGQL {
            total_cost: 125.75,
            total_tokens: 50000,
            average_cost_per_token: 0.002515,
            provider_breakdown: serde_json::json!({
                "openai": 75.25,
                "anthropic": 50.50
            }),
            model_breakdown: serde_json::json!({
                "gpt-4": 75.25,
                "claude-3-opus": 50.50
            }),
            daily_costs: serde_json::json!({
                "2024-01-01": 25.50,
                "2024-01-02": 30.25,
                "2024-01-03": 70.00
            }),
            period_start: input.start_date,
            period_end: input.end_date,
        })
    }

    /// Get a rule by ID
    async fn rule(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<RuleGQL>> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        match rule_storage.get_rule(&id).await {
            Ok(Some(stored_rule)) => Ok(Some(RuleGQL::from(&stored_rule))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get rule: {}",
                e
            ))),
        }
    }

    /// List all rules, optionally filtered by tags
    async fn rules(
        &self,
        ctx: &Context<'_>,
        tags: Option<Vec<String>>,
    ) -> async_graphql::Result<Vec<RuleGQL>> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        match rule_storage.list_rules(tags).await {
            Ok(stored_rules) => {
                let rules: Vec<RuleGQL> = stored_rules
                    .iter()
                    .map(|stored_rule| RuleGQL::from(stored_rule))
                    .collect();
                Ok(rules)
            }
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to list rules: {}",
                e
            ))),
        }
    }

    /// Get rules for a specific workflow
    async fn workflow_rules(
        &self,
        ctx: &Context<'_>,
        workflow_id: String,
    ) -> async_graphql::Result<Vec<RuleGQL>> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        match rule_storage.get_workflow_rules(&workflow_id).await {
            Ok(stored_rules) => {
                let rules: Vec<RuleGQL> = stored_rules
                    .iter()
                    .map(|stored_rule| RuleGQL::from(stored_rule))
                    .collect();
                Ok(rules)
            }
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to get workflow rules: {}",
                e
            ))),
        }
    }
}

// GraphQL Mutation root
pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new workflow definition
    async fn create_workflow(
        &self,
        ctx: &Context<'_>,
        input: WorkflowDefinitionInput,
    ) -> async_graphql::Result<WorkflowGQL> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;

        // Convert input to internal types
        let workflow_id = Uuid::new_v4().to_string();
        let states: Vec<StateId> = input.states.into_iter().map(StateId::from).collect();
        let activities: Vec<ActivityDefinition> = input
            .activities
            .into_iter()
            .map(|a| ActivityDefinition {
                id: ActivityId::from(a.id.as_str()),
                from_states: a.from_states.into_iter().map(StateId::from).collect(),
                to_state: StateId::from(a.to_state),
                conditions: a.conditions,
                rules: vec![], // Start with empty rules - can be added later via GraphQL
            })
            .collect();

        let workflow = WorkflowDefinition {
            id: workflow_id,
            name: input.name,
            states,
            activities,
            initial_state: StateId::from(input.initial_state),
        };

        // Validate workflow before storing
        workflow
            .validate()
            .map_err(|e| async_graphql::Error::new(format!("Invalid workflow: {}", e)))?;

        let created = storage
            .create_workflow(workflow)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store workflow: {}", e)))?;

        Ok(WorkflowGQL::from(&created))
    }

    /// Create a new token in a workflow
    /// Create a new resource
    async fn create_resource(
        &self,
        ctx: &Context<'_>,
        input: ResourceCreateInput,
    ) -> async_graphql::Result<ResourceGQL> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;

        // Get workflow to determine initial place
        let workflow = storage
            .get_workflow(&input.workflow_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new(format!("Workflow not found: {}", input.workflow_id))
            })?;

        let initial_state = input
            .initial_state
            .map(StateId::from)
            .unwrap_or_else(|| workflow.initial_state.clone());

        let mut resource = Resource::new(&input.workflow_id, initial_state);

        // Set data if provided
        if let Some(data) = input.data {
            resource.data = data;
        }

        // Set metadata if provided
        if let Some(metadata) = input.metadata {
            if let Some(metadata_obj) = metadata.as_object() {
                for (key, value) in metadata_obj {
                    resource.set_metadata(key, value.clone());
                }
            }
        }

        let created = storage
            .create_resource(resource)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store resource: {}", e)))?;

        Ok(ResourceGQL::from(&created))
    }

    /// Execute an activity - automatically uses NATS-aware execution when available
    async fn execute_activity(
        &self,
        ctx: &Context<'_>,
        input: ActivityExecuteInput,
    ) -> async_graphql::Result<ResourceGQL> {
        // Check if NATS storage is available and use it for proper state persistence
        if let Ok(nats_storage) =
            ctx.data::<std::sync::Arc<crate::engine::nats_storage::NATSStorage>>()
        {
            let resource_id = input
                .resource_id
                .parse::<Uuid>()
                .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

            // Get the resource with retry logic for timing issues
            let mut resource = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        100 * (2_u64.pow(attempt)),
                    ))
                    .await;
                }

                match nats_storage.get_resource(&resource_id).await {
                    Ok(Some(found_resource)) => {
                        resource = Some(found_resource);
                        break;
                    }
                    Ok(None) => {
                        if attempt == 2 {
                            return Err(async_graphql::Error::new(
                                "Resource not found after retries",
                            ));
                        }
                        continue;
                    }
                    Err(e) => {
                        return Err(async_graphql::Error::new(format!(
                            "Failed to get resource: {}",
                            e
                        )));
                    }
                }
            }

            let resource = resource.unwrap();

            let workflow = nats_storage
                .get_workflow(&resource.workflow_id)
                .await?
                .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;

            let activity_id = ActivityId::from(input.activity_id.clone());
            let current_state = StateId::from(resource.current_state());

            // Check if activity is valid
            let target_state = workflow
                .can_execute_activity(&current_state, &activity_id)
                .ok_or_else(|| async_graphql::Error::new("Invalid activity"))?;

            // Use NATS-aware execution for proper state persistence
            let executed_resource = nats_storage
                .execute_activity_with_nats(
                    resource,
                    target_state.clone(),
                    activity_id,
                    Some("graphql-api".to_string()),
                )
                .await
                .map_err(|e| {
                    async_graphql::Error::new(format!("Failed to execute activity: {}", e))
                })?;

            Ok(ResourceGQL::from(&executed_resource))
        } else {
            // Fallback to generic storage (should not happen with NATS configured)
            let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;

            let resource_id = input
                .resource_id
                .parse::<Uuid>()
                .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

            // Get the resource with retry logic for timing issues
            let mut resource = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        100 * (2_u64.pow(attempt)),
                    ))
                    .await;
                }

                match storage.get_resource(&resource_id).await {
                    Ok(Some(found_resource)) => {
                        resource = Some(found_resource);
                        break;
                    }
                    Ok(None) => {
                        if attempt == 2 {
                            return Err(async_graphql::Error::new(
                                "Resource not found after retries",
                            ));
                        }
                        continue;
                    }
                    Err(e) => {
                        return Err(async_graphql::Error::new(format!(
                            "Failed to get resource: {}",
                            e
                        )));
                    }
                }
            }

            let mut resource = resource.unwrap();

            let workflow = storage
                .get_workflow(&resource.workflow_id)
                .await?
                .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;

            let activity_id = ActivityId::from(input.activity_id);
            let current_state = StateId::from(resource.current_state());

            // Check if activity is valid
            let target_state = workflow
                .can_execute_activity(&current_state, &activity_id)
                .ok_or_else(|| async_graphql::Error::new("Invalid activity"))?;

            // Update with any provided data before executing activity
            if let Some(data) = input.data {
                resource.data = data;
            }

            // Execute the activity
            resource.execute_activity(target_state.clone(), activity_id);

            // Store the updated resource
            let updated = storage.update_resource(resource).await.map_err(|e| {
                async_graphql::Error::new(format!("Failed to update resource: {}", e))
            })?;

            Ok(ResourceGQL::from(&updated))
        }
    }

    /// Create a new agent
    async fn create_agent(
        &self,
        ctx: &Context<'_>,
        input: AgentDefinitionInput,
    ) -> async_graphql::Result<AgentDefinitionGQL> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;

        debug!(
            "🔍 GraphQL using storage backend: {}",
            std::any::type_name::<dyn AgentStorage>()
        );
        debug!(
            "🔍 GraphQL storage implementation: {:?}",
            std::ptr::addr_of!(**agent_storage)
        );

        // Convert input to internal types
        let agent_id = AgentId::from(format!("agent_{}", Uuid::new_v4()));

        let llm_provider = match input.llm_provider.provider_type.as_str() {
            "openai" => LLMProvider::OpenAI {
                api_key: input.llm_provider.api_key.unwrap_or_else(|| {
                    std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "not-set".to_string())
                }),
                model: input.llm_provider.model,
                base_url: input.llm_provider.base_url,
            },
            "anthropic" => LLMProvider::Anthropic {
                api_key: input.llm_provider.api_key.unwrap_or_else(|| {
                    std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "not-set".to_string())
                }),
                model: input.llm_provider.model,
                base_url: input.llm_provider.base_url,
            },
            "google" => LLMProvider::Google {
                api_key: input.llm_provider.api_key.unwrap_or_else(|| {
                    std::env::var("GOOGLE_API_KEY").unwrap_or_else(|_| "not-set".to_string())
                }),
                model: input.llm_provider.model,
            },
            "ollama" => LLMProvider::Ollama {
                base_url: input
                    .llm_provider
                    .base_url
                    .unwrap_or_else(|| "http://localhost:11434".to_string()),
                model: input.llm_provider.model,
            },
            "custom" => LLMProvider::Custom {
                endpoint: input.llm_provider.base_url.unwrap_or_default(),
                headers: std::collections::HashMap::new(),
                model: input.llm_provider.model,
            },
            _ => return Err(async_graphql::Error::new("Invalid LLM provider type")),
        };

        let llm_config = LLMConfig {
            temperature: input.llm_config.temperature as f32,
            max_tokens: input.llm_config.max_tokens.map(|t| t as u32),
            top_p: input.llm_config.top_p.map(|p| p as f32),
            frequency_penalty: input.llm_config.frequency_penalty.map(|p| p as f32),
            presence_penalty: input.llm_config.presence_penalty.map(|p| p as f32),
            stop_sequences: input.llm_config.stop_sequences,
        };

        let prompts = AgentPrompts {
            system: input.prompts.system,
            user_template: input.prompts.user_template,
            context_instructions: input.prompts.context_instructions,
        };

        let now = chrono::Utc::now();
        let agent = AgentDefinition {
            id: agent_id,
            name: input.name,
            description: input.description,
            llm_provider,
            llm_config,
            prompts,
            capabilities: input.capabilities,
            tools: input.tools,
            created_at: now,
            updated_at: now,
        };

        agent_storage
            .store_agent(&agent)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store agent: {}", e)))?;

        Ok(AgentDefinitionGQL::from(&agent))
    }

    /// Create state agent configuration
    /// TEMPORARILY DISABLED: StateAgentConfig methods removed from AgentStorage trait
    /// Will be moved to workflow integration layer in Phase 2
    async fn create_state_agent_config(
        &self,
        _ctx: &Context<'_>,
        _input: StateAgentConfigInput,
    ) -> async_graphql::Result<StateAgentConfigGQL> {
        // TODO: Re-implement in workflow integration layer
        Err(async_graphql::Error::new(
            "StateAgentConfig functionality temporarily disabled during standalone agent refactoring"
        ))
    }

    /// Trigger state agents for a resource
    /// TEMPORARILY DISABLED: execute_state_agents method removed from AgentEngine
    /// Will be moved to workflow integration layer in Phase 2
    async fn trigger_state_agents(
        &self,
        _ctx: &Context<'_>,
        _input: TriggerStateAgentsInput,
    ) -> async_graphql::Result<Vec<AgentExecutionGQL>> {
        // TODO: Re-implement in workflow integration layer
        Err(async_graphql::Error::new(
            "State agent triggering temporarily disabled during standalone agent refactoring",
        ))
    }

    /// NATS-specific mutations for enhanced token operations

    /// Create a workflow instance with NATS event tracking
    async fn create_workflow_instance(
        &self,
        ctx: &Context<'_>,
        input: CreateWorkflowInstanceInput,
    ) -> async_graphql::Result<NATSResourceGQL> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;

        // Get the workflow definition to find initial place
        let workflow = storage
            .get_workflow(&input.workflow_id)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to get workflow: {}", e)))?
            .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;

        // Create new resource
        let mut resource = Resource::new(&input.workflow_id, workflow.initial_state.clone());

        // Set initial data if provided
        if let Some(data) = input.initial_data {
            resource.data = data;
        }

        // Set metadata if provided
        if let Some(metadata) = input.metadata {
            if let serde_json::Value::Object(map) = metadata {
                for (key, value) in map {
                    resource.set_metadata(key, value);
                }
            }
        }

        // Try to use NATS storage for enhanced functionality
        if let Ok(nats_storage) =
            ctx.data::<std::sync::Arc<crate::engine::nats_storage::NATSStorage>>()
        {
            let created_resource = nats_storage
                .create_resource_with_event(resource, input.triggered_by)
                .await
                .map_err(|e| {
                    async_graphql::Error::new(format!("Failed to create NATS resource: {}", e))
                })?;
            Ok(NATSResourceGQL::from(&created_resource))
        } else {
            // Fallback to regular storage
            let created_resource = storage.create_resource(resource).await.map_err(|e| {
                async_graphql::Error::new(format!("Failed to create resource: {}", e))
            })?;
            Ok(NATSResourceGQL::from(&created_resource))
        }
    }

    /// Transition token with NATS event publishing
    /// Execute activity with NATS
    async fn execute_activity_with_nats(
        &self,
        ctx: &Context<'_>,
        input: ExecuteActivityWithNATSInput,
    ) -> async_graphql::Result<NATSResourceGQL> {
        // Parse resource ID
        let resource_id = input
            .resource_id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid resource ID format"))?;

        // Try to use NATS storage directly first for consistent behavior
        if let Ok(nats_storage) =
            ctx.data::<std::sync::Arc<crate::engine::nats_storage::NATSStorage>>()
        {
            // Get resource directly from NATS storage with retry logic
            let mut resource = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        100 * (2_u64.pow(attempt)),
                    ))
                    .await;
                }

                match nats_storage.get_resource(&resource_id).await {
                    Ok(Some(found_resource)) => {
                        resource = Some(found_resource);
                        break;
                    }
                    Ok(None) => {
                        if attempt == 2 {
                            return Err(async_graphql::Error::new(
                                "Resource not found after retries",
                            ));
                        }
                        continue;
                    }
                    Err(e) => {
                        return Err(async_graphql::Error::new(format!(
                            "Failed to get resource: {}",
                            e
                        )));
                    }
                }
            }

            let mut resource = resource.unwrap();

            // Get the workflow to validate activity
            let workflow = nats_storage
                .get_workflow(&resource.workflow_id)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to get workflow: {}", e)))?
                .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;

            let activity_id = ActivityId::from(input.activity_id);
            let new_state = StateId::from(input.new_state);
            let current_state = resource.state.clone();

            // Validate activity
            if !workflow
                .can_execute_activity(&current_state, &activity_id)
                .map(|s| *s == new_state)
                .unwrap_or(false)
            {
                return Err(async_graphql::Error::new("Invalid activity"));
            }

            // Update resource data if provided
            if let Some(data) = input.data {
                resource.data = data;
            }

            let executed_resource = nats_storage
                .execute_activity_with_nats(resource, new_state, activity_id, input.triggered_by)
                .await
                .map_err(|e| {
                    async_graphql::Error::new(format!("Failed to execute NATS activity: {}", e))
                })?;
            Ok(NATSResourceGQL::from(&executed_resource))
        } else {
            // Fallback to wrapper storage
            let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;

            // Get the resource with retry logic for timing issues
            let mut resource = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        100 * (2_u64.pow(attempt)),
                    ))
                    .await;
                }

                match storage.get_resource(&resource_id).await {
                    Ok(Some(found_resource)) => {
                        resource = Some(found_resource);
                        break;
                    }
                    Ok(None) => {
                        if attempt == 2 {
                            return Err(async_graphql::Error::new(
                                "Resource not found after retries",
                            ));
                        }
                        continue;
                    }
                    Err(e) => {
                        return Err(async_graphql::Error::new(format!(
                            "Failed to get resource: {}",
                            e
                        )));
                    }
                }
            }

            let mut resource = resource.unwrap();

            // Get the workflow to validate activity
            let workflow = storage
                .get_workflow(&resource.workflow_id)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to get workflow: {}", e)))?
                .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;

            let activity_id = ActivityId::from(input.activity_id);
            let new_state = StateId::from(input.new_state);
            let current_state = resource.state.clone();

            // Validate activity
            if !workflow
                .can_execute_activity(&current_state, &activity_id)
                .map(|s| *s == new_state)
                .unwrap_or(false)
            {
                return Err(async_graphql::Error::new("Invalid activity"));
            }

            // Update resource data if provided
            if let Some(data) = input.data {
                resource.data = data;
            }

            // Regular activity execution
            resource.execute_activity(new_state, activity_id);
            let updated_resource = storage.update_resource(resource).await.map_err(|e| {
                async_graphql::Error::new(format!("Failed to update resource: {}", e))
            })?;
            Ok(NATSResourceGQL::from(&updated_resource))
        }
    }

    /// Send LLM chat completion request
    async fn llm_chat_completion(
        &self,
        _ctx: &Context<'_>,
        input: LLMChatCompletionInput,
    ) -> async_graphql::Result<LLMResponseGQL> {
        // Create router
        let router = crate::llm::router::LLMRouter::new().await.map_err(|e| {
            async_graphql::Error::new(format!("Failed to initialize router: {}", e))
        })?;

        // Convert GraphQL input to LLM request
        let llm_request = crate::llm::LLMRequest {
            id: uuid::Uuid::new_v4(),
            model: input.model,
            messages: input
                .messages
                .into_iter()
                .map(|msg| crate::llm::ChatMessage {
                    role: match msg.role.as_str() {
                        "system" => crate::llm::MessageRole::System,
                        "user" => crate::llm::MessageRole::User,
                        "assistant" => crate::llm::MessageRole::Assistant,
                        "function" => crate::llm::MessageRole::Function,
                        _ => crate::llm::MessageRole::User,
                    },
                    content: msg.content,
                    name: msg.name,
                    function_call: None,
                })
                .collect(),
            temperature: input.temperature.map(|t| t as f64),
            max_tokens: input.max_tokens.map(|t| t as u32),
            top_p: input.top_p.map(|p| p as f64),
            frequency_penalty: input.frequency_penalty.map(|p| p as f64),
            presence_penalty: input.presence_penalty.map(|p| p as f64),
            stop: input.stop,
            stream: Some(input.stream.unwrap_or(false)),
            functions: None,
            function_call: None,
            user: input.user,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                if let Some(project_id) = input.project_id {
                    meta.insert(
                        "project_id".to_string(),
                        serde_json::Value::String(project_id),
                    );
                }
                meta
            },
        };

        // Make the actual LLM request
        let response = router
            .chat_completion(llm_request)
            .await
            .map_err(|e| async_graphql::Error::new(format!("LLM request failed: {}", e)))?;

        // Convert response back to GraphQL format
        Ok(LLMResponseGQL {
            id: response.id,
            model: response.model,
            choices: response
                .choices
                .into_iter()
                .map(|choice| LLMChoiceGQL {
                    index: choice.index as i32,
                    message: ChatMessageGQL {
                        role: match choice.message.role {
                            crate::llm::MessageRole::System => "system".to_string(),
                            crate::llm::MessageRole::User => "user".to_string(),
                            crate::llm::MessageRole::Assistant => "assistant".to_string(),
                            crate::llm::MessageRole::Function => "function".to_string(),
                        },
                        content: choice.message.content,
                        name: choice.message.name,
                    },
                    finish_reason: choice.finish_reason,
                })
                .collect(),
            usage: TokenUsageGQL {
                prompt_tokens: response.usage.prompt_tokens as i32,
                completion_tokens: response.usage.completion_tokens as i32,
                total_tokens: response.usage.total_tokens as i32,
                estimated_cost: response.usage.estimated_cost,
            },
            provider: response.provider.to_string(),
            routing_info: RoutingInfoGQL {
                selected_provider: response.routing_info.selected_provider.to_string(),
                routing_strategy: format!("{:?}", response.routing_info.routing_strategy),
                latency_ms: response.routing_info.latency_ms as i32,
                retry_count: response.routing_info.retry_count as i32,
                fallback_used: response.routing_info.fallback_used,
            },
        })
    }

    /// Configure LLM provider
    async fn configure_llm_provider(
        &self,
        _ctx: &Context<'_>,
        input: LLMProviderConfigInput,
    ) -> async_graphql::Result<LLMProviderGQL> {
        // Mock implementation - in real implementation this would store provider config
        Ok(LLMProviderGQL {
            id: ID(uuid::Uuid::new_v4().to_string()),
            provider_type: input.provider_type.clone(),
            name: input.name.clone(),
            base_url: input.base_url.clone(),
            models: input
                .models
                .into_iter()
                .map(|model| LLMModelGQL {
                    id: model.id,
                    name: model.name,
                    max_tokens: model.max_tokens,
                    context_window: model.context_window,
                    cost_per_input_token: model.cost_per_input_token,
                    cost_per_output_token: model.cost_per_output_token,
                    supports_streaming: model.supports_streaming,
                    supports_function_calling: model.supports_function_calling,
                    capabilities: model.capabilities,
                })
                .collect(),
            health_status: LLMProviderHealthGQL {
                is_healthy: true,
                last_check: chrono::Utc::now().to_rfc3339(),
                error_rate: 0.0,
                average_latency_ms: 0,
                consecutive_failures: 0,
                last_error: None,
            },
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Set budget limits
    async fn set_budget(
        &self,
        _ctx: &Context<'_>,
        input: BudgetInput,
    ) -> async_graphql::Result<BudgetStatusGQL> {
        // Mock implementation - in real implementation this would store budget config
        let budget_id = if let Some(project_id) = input.project_id {
            format!("project:{}", project_id)
        } else if let Some(user_id) = input.user_id {
            format!("user:{}", user_id)
        } else {
            "default".to_string()
        };

        Ok(BudgetStatusGQL {
            budget_id,
            limit: input.limit,
            used: 0.0,
            percentage_used: 0.0,
            is_exhausted: false,
            is_warning: false,
            remaining: input.limit,
            message: format!("Budget set to ${:.2}", input.limit),
        })
    }

    /// Create a new rule
    async fn create_rule(
        &self,
        ctx: &Context<'_>,
        input: RuleInput,
    ) -> async_graphql::Result<RuleGQL> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        let stored_rule = crate::engine::rules::StoredRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            description: input.description,
            condition: RuleCondition::from(input.condition),
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: None, // TODO: Get from auth context
            tags: input.tags.unwrap_or_default(),
            workflow_id: None,
        };

        match rule_storage.create_rule(stored_rule).await {
            Ok(created_rule) => Ok(RuleGQL::from(&created_rule)),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to create rule: {}",
                e
            ))),
        }
    }

    /// Update an existing rule
    async fn update_rule(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: RuleInput,
    ) -> async_graphql::Result<RuleGQL> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        let stored_rule = crate::engine::rules::StoredRule {
            id: id.clone(),
            name: input.name,
            description: input.description,
            condition: RuleCondition::from(input.condition),
            version: 1,                     // Will be incremented in update_rule
            created_at: chrono::Utc::now(), // Will be preserved in update_rule
            updated_at: chrono::Utc::now(),
            created_by: None,
            tags: input.tags.unwrap_or_default(),
            workflow_id: None,
        };

        match rule_storage.update_rule(&id, stored_rule).await {
            Ok(updated_rule) => Ok(RuleGQL::from(&updated_rule)),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to update rule: {}",
                e
            ))),
        }
    }

    /// Delete a rule
    async fn delete_rule(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<bool> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        match rule_storage.delete_rule(&id).await {
            Ok(deleted) => Ok(deleted),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to delete rule: {}",
                e
            ))),
        }
    }

    /// Evaluate a rule against data
    async fn evaluate_rule(
        &self,
        ctx: &Context<'_>,
        input: RuleEvaluationInput,
    ) -> async_graphql::Result<RuleEvaluationResultGQL> {
        let rule_storage = ctx.data::<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>()?;

        match rule_storage.get_rule(&input.rule_id).await {
            Ok(Some(stored_rule)) => {
                let rule: Rule = stored_rule.into();

                // Combine data and metadata for evaluation
                let mut combined_data = input.data;
                if let Some(metadata) = input.metadata {
                    if let serde_json::Value::Object(metadata_obj) = metadata {
                        if let serde_json::Value::Object(ref mut data_obj) = combined_data {
                            data_obj.extend(metadata_obj);
                        }
                    }
                }

                let metadata = ResourceMetadata::new();
                let result = rule.evaluate_detailed(&metadata, &combined_data);

                Ok(RuleEvaluationResultGQL {
                    rule_id: result.rule_id,
                    passed: result.passed,
                    reason: result.explanation,
                    details: Some(serde_json::to_value(&result.sub_results).unwrap_or_default()),
                    sub_results: vec![], // TODO: Convert sub_results properly
                })
            }
            Ok(None) => Err(async_graphql::Error::new(format!(
                "Rule not found: {}",
                input.rule_id
            ))),
            Err(e) => Err(async_graphql::Error::new(format!(
                "Failed to evaluate rule: {}",
                e
            ))),
        }
    }
}

// GraphQL Subscription root (for real-time updates)
pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to resource state changes
    async fn resource_updates(
        &self,
        _resource_id: String,
    ) -> impl futures::Stream<Item = ResourceGQL> {
        // TODO: Implement real-time resource updates using NATS streams
        futures::stream::empty()
    }

    /// Subscribe to workflow events
    async fn workflow_events(&self, _workflow_id: String) -> impl futures::Stream<Item = String> {
        // TODO: Implement real-time workflow events
        futures::stream::empty()
    }

    /// Subscribe to agent execution stream events
    async fn agent_execution_stream(
        &self,
        ctx: &Context<'_>,
        execution_id: String,
    ) -> async_graphql::Result<impl futures::Stream<Item = String>> {
        let _agent_engine = ctx.data::<AgentEngine>()?;
        let _execution_uuid = execution_id
            .parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid execution ID format"))?;

        // TODO: Implement real-time agent execution streaming
        // For now, return empty stream
        Ok(futures::stream::empty())
    }

    /// Subscribe to LLM response stream for real-time streaming
    async fn llm_stream(
        &self,
        _ctx: &Context<'_>,
        _request_id: String,
    ) -> async_graphql::Result<impl futures::Stream<Item = String>> {
        // Create router and real LLM request
        let router = crate::llm::router::LLMRouter::new().await.map_err(|e| {
            async_graphql::Error::new(format!("Failed to initialize router: {}", e))
        })?;

        let llm_request = crate::llm::LLMRequest {
            id: uuid::Uuid::new_v4(),
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![crate::llm::ChatMessage {
                role: crate::llm::MessageRole::User,
                content: "How much wood would a woodchuck chuck if a woodchuck could chuck wood?"
                    .to_string(),
                name: None,
                function_call: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(150),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(true),
            functions: None,
            function_call: None,
            user: None,
            metadata: std::collections::HashMap::new(),
        };

        // Get the real streaming response
        let stream = router
            .stream_chat_completion(llm_request)
            .await
            .map_err(|e| {
                async_graphql::Error::new(format!("LLM streaming request failed: {}", e))
            })?;

        // Convert the stream to JSON strings for WebSocket
        use futures::StreamExt;
        let json_stream = stream.map(|chunk_result| match chunk_result {
            Ok(chunk) => serde_json::to_string(&serde_json::json!({
                "type": "chunk",
                "data": {
                    "id": chunk.id,
                    "model": chunk.model,
                    "choices": chunk.choices.iter().map(|choice| {
                        serde_json::json!({
                            "index": choice.index,
                            "delta": {
                                "role": match choice.delta.role {
                                    crate::llm::MessageRole::Assistant => "assistant",
                                    crate::llm::MessageRole::User => "user",
                                    crate::llm::MessageRole::System => "system",
                                    crate::llm::MessageRole::Function => "function",
                                },
                                "content": choice.delta.content
                            },
                            "finish_reason": choice.finish_reason
                        })
                    }).collect::<Vec<_>>()
                }
            }))
            .unwrap_or_else(|_| {
                r#"{"type":"error","error":"JSON serialization failed"}"#.to_string()
            }),
            Err(e) => serde_json::to_string(&serde_json::json!({
                "type": "error",
                "error": e.to_string()
            }))
            .unwrap_or_else(|_| r#"{"type":"error","error":"Unknown error"}"#.to_string()),
        });

        Ok(json_stream)
    }

    /// Subscribe to cost updates for real-time budget monitoring
    async fn cost_updates(
        &self,
        ctx: &Context<'_>,
        _user_id: Option<String>,
    ) -> async_graphql::Result<impl futures::Stream<Item = String>> {
        // Mock cost update stream - in production this would track real cost changes
        let mock_updates = vec![
            r#"{"type":"cost_update","user_id":"user123","cost_delta":0.025,"total_cost":15.50,"timestamp":"2024-01-15T10:30:00Z"}"#,
            r#"{"type":"budget_warning","user_id":"user123","percentage_used":0.85,"message":"Budget warning: 85% of daily budget used"}"#,
        ];

        let stream =
            futures::stream::iter(mock_updates.into_iter().map(|update| update.to_string()));

        Ok(stream)
    }
}

// Schema type alias
pub type CircuitBreakerSchema = Schema<Query, Mutation, Subscription>;

/// Create the GraphQL schema
pub fn create_schema() -> CircuitBreakerSchema {
    Schema::build(Query, Mutation, Subscription).finish()
}

/// Create schema with storage backend
pub fn create_schema_with_storage(storage: Box<dyn WorkflowStorage>) -> CircuitBreakerSchema {
    Schema::build(Query, Mutation, Subscription)
        .data(storage)
        .finish()
}

/// Create schema with workflow storage, agent storage, and agent engine
pub fn create_schema_with_agents(
    workflow_storage: Box<dyn WorkflowStorage>,
    agent_storage: std::sync::Arc<dyn AgentStorage>,
    agent_engine: AgentEngine,
) -> CircuitBreakerSchema {
    Schema::build(Query, Mutation, Subscription)
        .data(workflow_storage)
        .data(agent_storage)
        .data(agent_engine)
        .finish()
}

/// Create schema with NATS storage backend
/// This provides enhanced GraphQL functionality with NATS-specific resolvers
pub fn create_schema_with_nats(
    nats_storage: std::sync::Arc<crate::engine::nats_storage::NATSStorage>,
) -> CircuitBreakerSchema {
    // Use NATS storage as the primary WorkflowStorage implementation
    let storage_boxed: Box<dyn WorkflowStorage> = Box::new(
        crate::engine::nats_storage::NATSStorageWrapper::new(nats_storage.clone()),
    );

    Schema::build(Query, Mutation, Subscription)
        .data(storage_boxed)
        .data(nats_storage)
        .finish()
}

/// Create schema with NATS storage, agent storage, and agent engine
/// This provides the full Circuit Breaker functionality with NATS streaming
pub fn create_schema_with_nats_and_agents(
    nats_storage: std::sync::Arc<crate::engine::nats_storage::NATSStorage>,
    agent_storage: std::sync::Arc<dyn AgentStorage>,
    agent_engine: AgentEngine,
) -> CircuitBreakerSchema {
    // Use NATS storage as the primary WorkflowStorage implementation
    let storage_boxed: Box<dyn WorkflowStorage> = Box::new(
        crate::engine::nats_storage::NATSStorageWrapper::new(nats_storage.clone()),
    );

    Schema::build(Query, Mutation, Subscription)
        .data(storage_boxed)
        .data(nats_storage)
        .data(agent_storage)
        .data(agent_engine)
        .finish()
}

/// Create GraphQL schema with NATS storage, agents, and rule storage
pub fn create_schema_with_full_storage(
    nats_storage: std::sync::Arc<crate::engine::nats_storage::NATSStorage>,
    agent_storage: std::sync::Arc<dyn AgentStorage>,
    agent_engine: std::sync::Arc<AgentEngine>,
    rule_storage: std::sync::Arc<dyn crate::engine::rules::RuleStorage>,
) -> CircuitBreakerSchema {
    // Use NATS storage as the primary WorkflowStorage implementation
    let storage_boxed: Box<dyn WorkflowStorage> = Box::new(
        crate::engine::nats_storage::NATSStorageWrapper::new(nats_storage.clone()),
    );

    Schema::build(Query, Mutation, Subscription)
        .data(storage_boxed)
        .data(nats_storage)
        .data(agent_storage)
        .data(agent_engine)
        .data(rule_storage)
        .finish()
}
