//! NATS-Enhanced Operations Client
//!
//! This module provides functionality for NATS event streaming and enhanced workflow operations
//! for the Circuit Breaker workflow automation server.
//!
//! # Examples
//!
//! ```rust
//! use circuit_breaker_sdk::{Client, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = Client::builder()
//!         .base_url("http://localhost:4000")?
//!         .build()?;
//!
//!     // Get resource with NATS metadata
//!     let resource = client.nats()
//!         .get_resource("resource_123")
//!         .await?;
//!
//!     if let Some(nats_resource) = resource {
//!         println!("Resource state: {}", nats_resource.state);
//!         println!("Event history: {} events", nats_resource.history.len());
//!     }
//!
//!     // Get resources in a specific state
//!     let resources = client.nats()
//!         .resources_in_state("workflow_456", "processing")
//!         .await?;
//!
//!     println!("Found {} resources in processing state", resources.len());
//!
//!     // Create workflow instance with NATS tracking
//!     let instance = client.nats()
//!         .create_workflow_instance()
//!         .workflow_id("workflow_456")
//!         .initial_data(serde_json::json!({"key": "value"}))
//!         .execute()
//!         .await?;
//!
//!     println!("Created workflow instance: {}", instance.id);
//!
//!     Ok(())
//! }
//! ```

use crate::client::Client;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// NATS client for enhanced workflow operations with event streaming
pub struct NATSClient {
    client: Client,
}

impl NATSClient {
    /// Create a new NATS client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get resource with NATS metadata by ID
    pub async fn get_resource(&self, id: &str) -> Result<Option<NATSResource>> {
        let query = r#"
            query GetNATSResource($id: String!) {
                natsResource(id: $id) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                    updatedAt
                    history {
                        id
                        event
                        data
                        timestamp
                        source
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "natsResource")]
            nats_resource: Option<NATSResourceGQL>,
        }

        let variables = Variables { id: id.to_string() };
        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.nats_resource.map(|r| r.into()))
    }

    /// Get resources currently in a specific state (NATS-specific)
    pub async fn resources_in_state(
        &self,
        workflow_id: &str,
        state_id: &str,
    ) -> Result<Vec<NATSResource>> {
        let query = r#"
            query GetResourcesInState($workflowId: String!, $stateId: String!) {
                resourcesInState(workflowId: $workflowId, stateId: $stateId) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                    updatedAt
                    history {
                        id
                        event
                        data
                        timestamp
                        source
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "workflowId")]
            workflow_id: String,
            #[serde(rename = "stateId")]
            state_id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "resourcesInState")]
            resources_in_state: Vec<NATSResourceGQL>,
        }

        let variables = Variables {
            workflow_id: workflow_id.to_string(),
            state_id: state_id.to_string(),
        };

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response
            .resources_in_state
            .into_iter()
            .map(|r| r.into())
            .collect())
    }

    /// Find resource by ID with workflow context (more efficient for NATS)
    pub async fn find_resource(
        &self,
        workflow_id: &str,
        resource_id: &str,
    ) -> Result<Option<NATSResource>> {
        let query = r#"
            query FindResource($workflowId: String!, $resourceId: String!) {
                findResource(workflowId: $workflowId, resourceId: $resourceId) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                    updatedAt
                    history {
                        id
                        event
                        data
                        timestamp
                        source
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "workflowId")]
            workflow_id: String,
            #[serde(rename = "resourceId")]
            resource_id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "findResource")]
            find_resource: Option<NATSResourceGQL>,
        }

        let variables = Variables {
            workflow_id: workflow_id.to_string(),
            resource_id: resource_id.to_string(),
        };

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.find_resource.map(|r| r.into()))
    }

    /// Create a workflow instance with NATS event tracking
    pub fn create_workflow_instance(&self) -> CreateWorkflowInstanceBuilder {
        CreateWorkflowInstanceBuilder::new(self.client.clone())
    }

    /// Execute activity with NATS event publishing
    pub fn execute_activity_with_nats(&self) -> ExecuteActivityWithNATSBuilder {
        ExecuteActivityWithNATSBuilder::new(self.client.clone())
    }
}

/// Resource with NATS-specific metadata and event tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NATSResource {
    /// Unique resource identifier
    pub id: String,
    /// ID of the workflow this resource belongs to
    pub workflow_id: String,
    /// Current state of the resource
    pub state: String,
    /// Resource data payload
    pub data: serde_json::Value,
    /// Resource metadata
    pub metadata: serde_json::Value,
    /// Timestamp when resource was created
    pub created_at: String,
    /// Timestamp when resource was last updated
    pub updated_at: String,
    /// Historical state transitions
    pub history: Vec<HistoryEvent>,
}

/// Historical event entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEvent {
    /// Event identifier
    pub id: String,
    /// Event type
    pub event: String,
    /// Event data
    pub data: serde_json::Value,
    /// Event timestamp
    pub timestamp: String,
    /// Event source
    pub source: Option<String>,
}

/// Input for creating workflow instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowInstanceInput {
    /// Workflow definition ID
    pub workflow_id: String,
    /// Initial data for the instance
    pub initial_data: Option<serde_json::Value>,
    /// Initial state (optional, will use workflow default)
    pub initial_state: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
    /// Enable NATS event publishing
    pub enable_nats_events: Option<bool>,
}

/// Input for executing activities with NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteActivityWithNATSInput {
    /// Resource ID to execute activity on
    pub resource_id: String,
    /// Activity name to execute
    pub activity_name: String,
    /// Activity input data
    pub input_data: Option<serde_json::Value>,
    /// NATS subject for publishing events
    pub nats_subject: Option<String>,
    /// Additional NATS headers
    pub nats_headers: Option<HashMap<String, String>>,
}

// GraphQL response types
#[derive(Deserialize)]
struct NATSResourceGQL {
    id: String,
    #[serde(rename = "workflowId")]
    workflow_id: String,
    state: String,
    data: serde_json::Value,
    metadata: serde_json::Value,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    history: Vec<HistoryEventGQL>,
}

impl From<NATSResourceGQL> for NATSResource {
    fn from(gql: NATSResourceGQL) -> Self {
        Self {
            id: gql.id,
            workflow_id: gql.workflow_id,
            state: gql.state,
            data: gql.data,
            metadata: gql.metadata,
            created_at: gql.created_at,
            updated_at: gql.updated_at,
            history: gql.history.into_iter().map(|h| h.into()).collect(),
        }
    }
}

#[derive(Deserialize)]
struct HistoryEventGQL {
    id: String,
    event: String,
    data: serde_json::Value,
    timestamp: String,
    source: Option<String>,
}

impl From<HistoryEventGQL> for HistoryEvent {
    fn from(gql: HistoryEventGQL) -> Self {
        Self {
            id: gql.id,
            event: gql.event,
            data: gql.data,
            timestamp: gql.timestamp,
            source: gql.source,
        }
    }
}

/// Builder for creating workflow instances
pub struct CreateWorkflowInstanceBuilder {
    client: Client,
    workflow_id: Option<String>,
    initial_data: Option<serde_json::Value>,
    initial_state: Option<String>,
    metadata: Option<serde_json::Value>,
    enable_nats_events: Option<bool>,
}

impl CreateWorkflowInstanceBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            workflow_id: None,
            initial_data: None,
            initial_state: None,
            metadata: None,
            enable_nats_events: None,
        }
    }

    /// Set the workflow ID
    pub fn workflow_id<S: Into<String>>(mut self, workflow_id: S) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// Set initial data for the instance
    pub fn initial_data(mut self, data: serde_json::Value) -> Self {
        self.initial_data = Some(data);
        self
    }

    /// Set initial state
    pub fn initial_state<S: Into<String>>(mut self, state: S) -> Self {
        self.initial_state = Some(state.into());
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Enable NATS event publishing
    pub fn enable_nats_events(mut self, enable: bool) -> Self {
        self.enable_nats_events = Some(enable);
        self
    }

    /// Execute the create workflow instance mutation
    pub async fn execute(self) -> Result<NATSResource> {
        let query = r#"
            mutation CreateWorkflowInstance($input: CreateWorkflowInstanceInput!) {
                createWorkflowInstance(input: $input) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                    updatedAt
                    history {
                        id
                        event
                        data
                        timestamp
                        source
                    }
                }
            }
        "#;

        let input = CreateWorkflowInstanceInput {
            workflow_id: self.workflow_id.ok_or_else(|| crate::Error::Validation {
                message: "workflow_id is required".to_string(),
            })?,
            initial_data: self.initial_data,
            initial_state: self.initial_state,
            metadata: self.metadata,
            enable_nats_events: self.enable_nats_events,
        };

        #[derive(Serialize)]
        struct Variables {
            input: CreateWorkflowInstanceInput,
        }

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "createWorkflowInstance")]
            create_workflow_instance: NATSResourceGQL,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.create_workflow_instance.into())
    }
}

/// Builder for executing activities with NATS
pub struct ExecuteActivityWithNATSBuilder {
    client: Client,
    resource_id: Option<String>,
    activity_name: Option<String>,
    input_data: Option<serde_json::Value>,
    nats_subject: Option<String>,
    nats_headers: Option<HashMap<String, String>>,
}

impl ExecuteActivityWithNATSBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            resource_id: None,
            activity_name: None,
            input_data: None,
            nats_subject: None,
            nats_headers: None,
        }
    }

    /// Set the resource ID
    pub fn resource_id<S: Into<String>>(mut self, resource_id: S) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    /// Set the activity name
    pub fn activity_name<S: Into<String>>(mut self, activity_name: S) -> Self {
        self.activity_name = Some(activity_name.into());
        self
    }

    /// Set activity input data
    pub fn input_data(mut self, data: serde_json::Value) -> Self {
        self.input_data = Some(data);
        self
    }

    /// Set NATS subject for event publishing
    pub fn nats_subject<S: Into<String>>(mut self, subject: S) -> Self {
        self.nats_subject = Some(subject.into());
        self
    }

    /// Set NATS headers
    pub fn nats_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.nats_headers = Some(headers);
        self
    }

    /// Add a NATS header
    pub fn nats_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        if self.nats_headers.is_none() {
            self.nats_headers = Some(HashMap::new());
        }
        if let Some(ref mut headers) = self.nats_headers {
            headers.insert(key.into(), value.into());
        }
        self
    }

    /// Execute the activity with NATS mutation
    pub async fn execute(self) -> Result<NATSResource> {
        let query = r#"
            mutation ExecuteActivityWithNATS($input: ExecuteActivityWithNatsInput!) {
                executeActivityWithNats(input: $input) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                    updatedAt
                    history {
                        id
                        event
                        data
                        timestamp
                        source
                    }
                }
            }
        "#;

        let input = ExecuteActivityWithNATSInput {
            resource_id: self.resource_id.ok_or_else(|| crate::Error::Validation {
                message: "resource_id is required".to_string(),
            })?,
            activity_name: self.activity_name.ok_or_else(|| crate::Error::Validation {
                message: "activity_name is required".to_string(),
            })?,
            input_data: self.input_data,
            nats_subject: self.nats_subject,
            nats_headers: self.nats_headers,
        };

        #[derive(Serialize)]
        struct Variables {
            input: ExecuteActivityWithNATSInput,
        }

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "executeActivityWithNats")]
            execute_activity_with_nats: NATSResourceGQL,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.execute_activity_with_nats.into())
    }
}

/// Convenience functions
pub fn create_workflow_instance(
    client: &Client,
    workflow_id: &str,
) -> CreateWorkflowInstanceBuilder {
    client
        .nats()
        .create_workflow_instance()
        .workflow_id(workflow_id)
        .enable_nats_events(true)
}

pub fn execute_activity_with_nats(
    client: &Client,
    resource_id: &str,
    activity_name: &str,
) -> ExecuteActivityWithNATSBuilder {
    client
        .nats()
        .execute_activity_with_nats()
        .resource_id(resource_id)
        .activity_name(activity_name)
}

pub async fn get_nats_resource(client: &Client, resource_id: &str) -> Result<Option<NATSResource>> {
    client.nats().get_resource(resource_id).await
}

pub async fn get_resources_in_state(
    client: &Client,
    workflow_id: &str,
    state_id: &str,
) -> Result<Vec<NATSResource>> {
    client
        .nats()
        .resources_in_state(workflow_id, state_id)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nats_resource_serialization() {
        let resource = NATSResource {
            id: "resource_123".to_string(),
            workflow_id: "workflow_456".to_string(),
            state: "processing".to_string(),
            data: serde_json::json!({"key": "value"}),
            metadata: serde_json::json!({"env": "test"}),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T01:00:00Z".to_string(),
            history: vec![],
        };

        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("resource_123"));
        assert!(json.contains("workflow_456"));
        assert!(json.contains("processing"));
    }

    #[test]
    fn test_history_event_serialization() {
        let event = HistoryEvent {
            id: "event_123".to_string(),
            event: "state_changed".to_string(),
            data: serde_json::json!({"from": "start", "to": "processing"}),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            source: Some("nats".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("event_123"));
        assert!(json.contains("state_changed"));
        assert!(json.contains("nats"));
    }

    #[test]
    fn test_workflow_instance_input_validation() {
        let input = CreateWorkflowInstanceInput {
            workflow_id: "workflow_123".to_string(),
            initial_data: Some(serde_json::json!({"key": "value"})),
            initial_state: Some("start".to_string()),
            metadata: None,
            enable_nats_events: Some(true),
        };

        assert_eq!(input.workflow_id, "workflow_123");
        assert!(input.enable_nats_events.unwrap());
    }
}
