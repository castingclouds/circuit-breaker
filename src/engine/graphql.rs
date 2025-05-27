// GraphQL API for the Circuit Breaker engine
// This provides a GraphQL interface for defining and executing State Managed Workflows

use async_graphql::{Context, Object, Schema, Subscription, InputObject, SimpleObject, Enum, ID};
use chrono::Utc;
use serde_json;
use uuid::Uuid;

use crate::models::{Token, PlaceId, TransitionId, WorkflowDefinition, TransitionDefinition, HistoryEvent,
    AgentId, AgentDefinition, LLMProvider, LLMConfig, AgentPrompts, PlaceAgentConfig, 
    PlaceAgentSchedule, AgentRetryConfig, AgentExecution, AgentExecutionStatus, AgentStreamEvent};
use crate::engine::storage::WorkflowStorage;
use crate::engine::{TokenEvents, EventBus, AgentStorage, AgentEngine};

// GraphQL types - these are the API representations of our domain models

#[derive(SimpleObject, Debug, Clone)]
pub struct WorkflowGQL {
    pub id: ID,
    pub name: String,
    pub places: Vec<String>,
    pub transitions: Vec<TransitionGQL>,
    pub initial_place: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct TransitionGQL {
    pub id: String,
    pub name: Option<String>,
    pub from_places: Vec<String>,
    pub to_place: String,
    pub conditions: Vec<String>,
    pub description: Option<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct TokenGQL {
    pub id: ID,
    pub workflow_id: String,
    pub place: String,
    pub data: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub history: Vec<HistoryEventGQL>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct HistoryEventGQL {
    pub timestamp: String,
    pub transition: String,
    pub from_place: String,
    pub to_place: String,
    pub data: Option<serde_json::Value>,
}

// Agent-related GraphQL types
#[derive(SimpleObject, Debug, Clone)]
pub struct AgentDefinitionGQL {
    pub id: String,
    pub name: String,
    pub description: String,
    pub llm_provider: LLMProviderGQL,
    pub llm_config: LLMConfigGQL,
    pub prompts: AgentPromptsGQL,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LLMProviderGQL {
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
pub struct PlaceAgentConfigGQL {
    pub id: String,
    pub place_id: String,
    pub agent_id: String,
    pub llm_config: Option<LLMConfigGQL>,
    pub input_mapping: serde_json::Value,
    pub output_mapping: serde_json::Value,
    pub auto_transition: Option<String>,
    pub schedule: Option<PlaceAgentScheduleGQL>,
    pub retry_config: Option<AgentRetryConfigGQL>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct PlaceAgentScheduleGQL {
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
    pub token_id: String,
    pub place_id: String,
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

// Input types for mutations
#[derive(InputObject, Debug)]
pub struct WorkflowDefinitionInput {
    pub name: String,
    pub places: Vec<String>,
    pub transitions: Vec<TransitionDefinitionInput>,
    pub initial_place: String,
    pub description: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct TransitionDefinitionInput {
    pub id: String,
    pub name: Option<String>,
    pub from_places: Vec<String>,
    pub to_place: String,
    pub conditions: Vec<String>,
    pub description: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct TokenCreateInput {
    pub workflow_id: String,
    pub initial_place: Option<String>,
    pub data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(InputObject, Debug)]
pub struct TransitionFireInput {
    pub token_id: String,
    pub transition_id: String,
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
    pub llm_provider: LLMProviderInput,
    pub llm_config: LLMConfigInput,
    pub prompts: AgentPromptsInput,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
}

#[derive(InputObject, Debug)]
pub struct LLMProviderInput {
    pub provider_type: String,
    pub model: String,
    pub api_key: String,
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
pub struct PlaceAgentConfigInput {
    pub place_id: String,
    pub agent_id: String,
    pub llm_config: Option<LLMConfigInput>,
    pub input_mapping: serde_json::Value,
    pub output_mapping: serde_json::Value,
    pub auto_transition: Option<String>,
    pub schedule: Option<PlaceAgentScheduleInput>,
    pub retry_config: Option<AgentRetryConfigInput>,
    pub enabled: bool,
}

#[derive(InputObject, Debug)]
pub struct PlaceAgentScheduleInput {
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
pub struct TriggerPlaceAgentsInput {
    pub token_id: String,
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
            places: workflow.places.iter().map(|s| s.as_str().to_string()).collect(),
            transitions: workflow.transitions.iter().map(|t| t.into()).collect(),
            initial_place: workflow.initial_place.as_str().to_string(),
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
            llm_provider: LLMProviderGQL::from(&agent.llm_provider),
            llm_config: LLMConfigGQL::from(&agent.llm_config),
            prompts: AgentPromptsGQL::from(&agent.prompts),
            capabilities: agent.capabilities.clone(),
            tools: agent.tools.clone(),
            created_at: agent.created_at.to_rfc3339(),
            updated_at: agent.updated_at.to_rfc3339(),
        }
    }
}

impl From<&LLMProvider> for LLMProviderGQL {
    fn from(provider: &LLMProvider) -> Self {
        match provider {
            LLMProvider::OpenAI { model, base_url, .. } => LLMProviderGQL {
                provider_type: "openai".to_string(),
                model: model.clone(),
                base_url: base_url.clone(),
            },
            LLMProvider::Anthropic { model, .. } => LLMProviderGQL {
                provider_type: "anthropic".to_string(),
                model: model.clone(),
                base_url: None,
            },
            LLMProvider::Google { model, .. } => LLMProviderGQL {
                provider_type: "google".to_string(),
                model: model.clone(),
                base_url: None,
            },
            LLMProvider::Ollama { model, base_url } => LLMProviderGQL {
                provider_type: "ollama".to_string(),
                model: model.clone(),
                base_url: Some(base_url.clone()),
            },
            LLMProvider::Custom { model, endpoint, .. } => LLMProviderGQL {
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

impl From<&PlaceAgentConfig> for PlaceAgentConfigGQL {
    fn from(config: &PlaceAgentConfig) -> Self {
        PlaceAgentConfigGQL {
            id: config.id.to_string(),
            place_id: config.place_id.as_str().to_string(),
            agent_id: config.agent_id.as_str().to_string(),
            llm_config: config.llm_config.as_ref().map(LLMConfigGQL::from),
            input_mapping: serde_json::to_value(&config.input_mapping).unwrap_or_default(),
            output_mapping: serde_json::to_value(&config.output_mapping).unwrap_or_default(),
            auto_transition: config.auto_transition.as_ref().map(|t| t.as_str().to_string()),
            schedule: config.schedule.as_ref().map(PlaceAgentScheduleGQL::from),
            retry_config: config.retry_config.as_ref().map(AgentRetryConfigGQL::from),
            enabled: config.enabled,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}

impl From<&PlaceAgentSchedule> for PlaceAgentScheduleGQL {
    fn from(schedule: &PlaceAgentSchedule) -> Self {
        PlaceAgentScheduleGQL {
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
            token_id: execution.token_id.to_string(),
            place_id: execution.place_id.as_str().to_string(),
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

impl From<&TransitionDefinition> for TransitionGQL {
    fn from(transition: &TransitionDefinition) -> Self {
        TransitionGQL {
            id: transition.id.as_str().to_string(),
            name: None,
            from_places: transition.from_places.iter().map(|s| s.as_str().to_string()).collect(),
            to_place: transition.to_place.as_str().to_string(),
            conditions: transition.conditions.clone(),
            description: None,
        }
    }
}

impl From<&Token> for TokenGQL {
    fn from(token: &Token) -> Self {
        TokenGQL {
            id: ID(token.id.to_string()),
            workflow_id: token.workflow_id.clone(),
            place: token.place.as_str().to_string(),
            data: token.data.clone(),
            metadata: serde_json::to_value(&token.metadata).unwrap_or_default(),
            created_at: token.created_at.to_rfc3339(),
            updated_at: token.updated_at.to_rfc3339(),
            history: token.history.iter().map(|h| h.into()).collect(),
        }
    }
}

impl From<&HistoryEvent> for HistoryEventGQL {
    fn from(event: &HistoryEvent) -> Self {
        HistoryEventGQL {
            timestamp: event.timestamp.to_rfc3339(),
            transition: event.transition.as_str().to_string(),
            from_place: event.from.as_str().to_string(),
            to_place: event.to.as_str().to_string(),
            data: event.data.clone(),
        }
    }
}

// GraphQL Query root
pub struct Query;

#[Object]
impl Query {
    /// Get a workflow definition by ID
    async fn workflow(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<WorkflowGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        match storage.get_workflow(&id).await {
            Ok(Some(workflow)) => Ok(Some(WorkflowGQL::from(&workflow))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get workflow: {}", e))),
        }
    }

    /// List all workflow definitions
    async fn workflows(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<WorkflowGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        match storage.list_workflows().await {
            Ok(workflows) => Ok(workflows.iter().map(WorkflowGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to list workflows: {}", e))),
        }
    }

    /// Get a token by ID
    async fn token(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<TokenGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        let token_id = id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid token ID format"))?;
        
        match storage.get_token(&token_id).await {
            Ok(Some(token)) => Ok(Some(TokenGQL::from(&token))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get token: {}", e))),
        }
    }

    /// List tokens, optionally filtered by workflow
    async fn tokens(&self, ctx: &Context<'_>, workflow_id: Option<String>) -> async_graphql::Result<Vec<TokenGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        match storage.list_tokens(workflow_id.as_deref()).await {
            Ok(tokens) => Ok(tokens.iter().map(TokenGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to list tokens: {}", e))),
        }
    }

    /// Get available transitions for a token
    async fn available_transitions(
        &self, 
        ctx: &Context<'_>, 
        token_id: String
    ) -> async_graphql::Result<Vec<TransitionGQL>> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        let token_uuid = token_id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid token ID format"))?;
        
        let token = storage.get_token(&token_uuid).await?
            .ok_or_else(|| async_graphql::Error::new("Token not found"))?;
        
        let workflow = storage.get_workflow(&token.workflow_id).await?
            .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;
        
        let current_place = PlaceId::from(token.current_place());
        let available = workflow.available_transitions(&current_place);
        
        Ok(available.iter().map(|t| TransitionGQL::from(*t)).collect())
    }

    /// Get an agent by ID
    async fn agent(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<AgentDefinitionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let agent_id = AgentId::from(id);
        
        match agent_storage.get_agent(&agent_id).await {
            Ok(Some(agent)) => Ok(Some(AgentDefinitionGQL::from(&agent))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get agent: {}", e))),
        }
    }

    /// List all agents
    async fn agents(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<AgentDefinitionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        
        match agent_storage.list_agents().await {
            Ok(agents) => Ok(agents.iter().map(AgentDefinitionGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to list agents: {}", e))),
        }
    }

    /// Get place agent configurations for a specific place
    async fn place_agent_configs(&self, ctx: &Context<'_>, place_id: String) -> async_graphql::Result<Vec<PlaceAgentConfigGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let place = PlaceId::from(place_id);
        
        match agent_storage.get_place_agent_configs(&place).await {
            Ok(configs) => Ok(configs.iter().map(PlaceAgentConfigGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get place agent configs: {}", e))),
        }
    }

    /// Get agent execution by ID
    async fn agent_execution(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<AgentExecutionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let execution_id = id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid execution ID format"))?;
        
        match agent_storage.get_execution(&execution_id).await {
            Ok(Some(execution)) => Ok(Some(AgentExecutionGQL::from(&execution))),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get agent execution: {}", e))),
        }
    }

    /// Get agent executions for a token
    async fn token_executions(&self, ctx: &Context<'_>, token_id: String) -> async_graphql::Result<Vec<AgentExecutionGQL>> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        let token_uuid = token_id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid token ID format"))?;
        
        match agent_storage.list_executions_for_token(&token_uuid).await {
            Ok(executions) => Ok(executions.iter().map(AgentExecutionGQL::from).collect()),
            Err(e) => Err(async_graphql::Error::new(format!("Failed to get token executions: {}", e))),
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
        let workflow_id = format!("workflow_{}", Uuid::new_v4());
        let places: Vec<PlaceId> = input.places.into_iter().map(PlaceId::from).collect();
        let transitions: Vec<TransitionDefinition> = input.transitions
            .into_iter()
            .map(|t| TransitionDefinition {
                id: TransitionId::from(t.id.as_str()),
                from_places: t.from_places.into_iter().map(PlaceId::from).collect(),
                to_place: PlaceId::from(t.to_place),
                conditions: t.conditions,
                rules: vec![], // Start with empty rules - can be added later via GraphQL
            })
            .collect();
        
        let workflow = WorkflowDefinition {
            id: workflow_id,
            name: input.name,
            places,
            transitions,
            initial_place: PlaceId::from(input.initial_place),
        };
        
        // Validate workflow before storing
        workflow.validate()
            .map_err(|e| async_graphql::Error::new(format!("Invalid workflow: {}", e)))?;
        
        let created = storage.create_workflow(workflow).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store workflow: {}", e)))?;
        
        Ok(WorkflowGQL::from(&created))
    }

    /// Create a new token in a workflow
    async fn create_token(
        &self,
        ctx: &Context<'_>,
        input: TokenCreateInput,
    ) -> async_graphql::Result<TokenGQL> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        
        // Get workflow to determine initial place
        let workflow = storage.get_workflow(&input.workflow_id).await?
            .ok_or_else(|| async_graphql::Error::new(format!("Workflow not found: {}", input.workflow_id)))?;
        
        let initial_place = input.initial_place
            .map(PlaceId::from)
            .unwrap_or_else(|| workflow.initial_place.clone());
        
        let mut token = Token::new(&input.workflow_id, initial_place);
        
        // Set data if provided
        if let Some(data) = input.data {
            token.data = data;
        }
        
        // Set metadata if provided
        if let Some(metadata) = input.metadata {
            if let Some(metadata_obj) = metadata.as_object() {
                for (key, value) in metadata_obj {
                    token.set_metadata(key, value.clone());
                }
            }
        }
        
        let created = storage.create_token(token).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store token: {}", e)))?;
        
        Ok(TokenGQL::from(&created))
    }

    /// Fire a transition on a token
    async fn fire_transition(
        &self,
        ctx: &Context<'_>,
        input: TransitionFireInput,
    ) -> async_graphql::Result<TokenGQL> {
        let storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        // TODO: Get EventBus from context once it's properly integrated
        // For now, we'll create a temporary one and suggest architectural improvement
        let event_bus = EventBus::new();
        
        let token_id = input.token_id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid token ID format"))?;
        
        let mut token = storage.get_token(&token_id).await?
            .ok_or_else(|| async_graphql::Error::new("Token not found"))?;
        
        let workflow = storage.get_workflow(&token.workflow_id).await?
            .ok_or_else(|| async_graphql::Error::new("Workflow not found"))?;
        
        let transition_id = TransitionId::from(input.transition_id);
        let current_place = PlaceId::from(token.current_place());
        
        // Check if transition is valid
        let target_place = workflow.can_transition(&current_place, &transition_id)
            .ok_or_else(|| async_graphql::Error::new("Invalid transition"))?;
        
        // Update with any provided data before transition
        if let Some(data) = input.data {
            token.data = data;
        }
        
        // Fire the transition using the combined method that emits events
        token.transition_to_with_events(target_place.clone(), transition_id, &event_bus).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to transition token: {}", e)))?;
        
        // Store the updated token
        let updated = storage.update_token(token).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to update token: {}", e)))?;
        
        Ok(TokenGQL::from(&updated))
    }

    /// Create a new agent
    async fn create_agent(
        &self,
        ctx: &Context<'_>,
        input: AgentDefinitionInput,
    ) -> async_graphql::Result<AgentDefinitionGQL> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        
        // Convert input to internal types
        let agent_id = AgentId::from(format!("agent_{}", Uuid::new_v4()));
        
        let llm_provider = match input.llm_provider.provider_type.as_str() {
            "openai" => LLMProvider::OpenAI {
                api_key: input.llm_provider.api_key,
                model: input.llm_provider.model,
                base_url: input.llm_provider.base_url,
            },
            "anthropic" => LLMProvider::Anthropic {
                api_key: input.llm_provider.api_key,
                model: input.llm_provider.model,
            },
            "google" => LLMProvider::Google {
                api_key: input.llm_provider.api_key,
                model: input.llm_provider.model,
            },
            "ollama" => LLMProvider::Ollama {
                base_url: input.llm_provider.base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
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
        
        agent_storage.store_agent(&agent).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store agent: {}", e)))?;
        
        Ok(AgentDefinitionGQL::from(&agent))
    }

    /// Create a place agent configuration
    async fn create_place_agent_config(
        &self,
        ctx: &Context<'_>,
        input: PlaceAgentConfigInput,
    ) -> async_graphql::Result<PlaceAgentConfigGQL> {
        let agent_storage = ctx.data::<std::sync::Arc<dyn AgentStorage>>()?;
        
        // Verify agent exists
        let agent_id = AgentId::from(input.agent_id);
        agent_storage.get_agent(&agent_id).await?
            .ok_or_else(|| async_graphql::Error::new("Agent not found"))?;
        
        let place_id = PlaceId::from(input.place_id);
        
        // Convert input mappings from JSON
        let input_mapping: std::collections::HashMap<String, String> = 
            serde_json::from_value(input.input_mapping)
                .map_err(|e| async_graphql::Error::new(format!("Invalid input mapping: {}", e)))?;
        
        let output_mapping: std::collections::HashMap<String, String> = 
            serde_json::from_value(input.output_mapping)
                .map_err(|e| async_graphql::Error::new(format!("Invalid output mapping: {}", e)))?;
        
        let llm_config = input.llm_config.map(|config| LLMConfig {
            temperature: config.temperature as f32,
            max_tokens: config.max_tokens.map(|t| t as u32),
            top_p: config.top_p.map(|p| p as f32),
            frequency_penalty: config.frequency_penalty.map(|p| p as f32),
            presence_penalty: config.presence_penalty.map(|p| p as f32),
            stop_sequences: config.stop_sequences,
        });
        
        let schedule = input.schedule.map(|sched| PlaceAgentSchedule {
            initial_delay_seconds: sched.initial_delay_seconds.map(|d| d as u64),
            interval_seconds: sched.interval_seconds.map(|i| i as u64),
            max_executions: sched.max_executions.map(|e| e as u32),
        });
        
        let retry_config = input.retry_config.map(|retry| AgentRetryConfig {
            max_attempts: retry.max_attempts as u32,
            backoff_seconds: retry.backoff_seconds as u64,
            retry_on_errors: retry.retry_on_errors,
        });
        
        let now = chrono::Utc::now();
        let config = PlaceAgentConfig {
            id: Uuid::new_v4(),
            place_id,
            agent_id,
            llm_config,
            trigger_conditions: vec![], // TODO: Add trigger conditions input
            input_mapping,
            output_mapping,
            auto_transition: input.auto_transition.map(TransitionId::from),
            schedule,
            retry_config,
            enabled: input.enabled,
            created_at: now,
            updated_at: now,
        };
        
        agent_storage.store_place_agent_config(&config).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to store place agent config: {}", e)))?;
        
        Ok(PlaceAgentConfigGQL::from(&config))
    }

    /// Trigger place agents for a token
    async fn trigger_place_agents(
        &self,
        ctx: &Context<'_>,
        input: TriggerPlaceAgentsInput,
    ) -> async_graphql::Result<Vec<AgentExecutionGQL>> {
        let workflow_storage = ctx.data::<Box<dyn WorkflowStorage>>()?;
        let agent_engine = ctx.data::<AgentEngine>()?;
        
        let token_id = input.token_id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid token ID format"))?;
        
        let token = workflow_storage.get_token(&token_id).await?
            .ok_or_else(|| async_graphql::Error::new("Token not found"))?;
        
        let executions = agent_engine.execute_place_agents(&token).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to execute place agents: {}", e)))?;
        
        Ok(executions.iter().map(AgentExecutionGQL::from).collect())
    }
}

// GraphQL Subscription root (for real-time updates)
pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to token state changes
    async fn token_updates(&self, _token_id: String) -> impl futures::Stream<Item = TokenGQL> {
        // TODO: Implement real-time token updates using NATS streams
        futures::stream::empty()
    }

    /// Subscribe to workflow events
    async fn workflow_events(&self, _workflow_id: String) -> impl futures::Stream<Item = String> {
        // TODO: Implement real-time workflow events
        futures::stream::empty()
    }

    /// Subscribe to agent execution stream events
    async fn agent_execution_stream(&self, ctx: &Context<'_>, execution_id: String) -> async_graphql::Result<impl futures::Stream<Item = String>> {
        let _agent_engine = ctx.data::<AgentEngine>()?;
        let _execution_uuid = execution_id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid execution ID format"))?;
        
        // TODO: Implement real-time agent execution streaming
        // For now, return empty stream
        Ok(futures::stream::empty())
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