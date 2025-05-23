// GraphQL API for the Circuit Breaker engine
// This provides a GraphQL interface for defining and executing State Managed Workflows

use async_graphql::{Context, Object, Schema, Subscription, InputObject, SimpleObject, Enum, ID};
use chrono::Utc;
use serde_json;
use uuid::Uuid;

use crate::models::{Token, PlaceId, TransitionId, WorkflowDefinition, TransitionDefinition, HistoryEvent};
use crate::engine::storage::WorkflowStorage;

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
        
        // Fire the transition
        token.transition_to(target_place.clone(), transition_id);
        
        // Update with any provided data
        if let Some(data) = input.data {
            token.data = data;
        }
        
        // Store the updated token
        let updated = storage.update_token(token).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to update token: {}", e)))?;
        
        Ok(TokenGQL::from(&updated))
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