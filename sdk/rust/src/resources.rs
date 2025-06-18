//! Resources module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for managing resources like databases,
//! APIs, and other external systems.

use crate::{types::*, Client, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client for resource operations
#[derive(Debug, Clone)]
pub struct ResourceClient {
    client: Client,
}

impl ResourceClient {
    /// Create a new resource client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a new resource
    pub fn create(&self) -> ResourceBuilder {
        ResourceBuilder::new(self.client.clone())
    }

    /// Get a resource by ID
    pub async fn get(&self, id: ResourceId) -> Result<Resource> {
        let query = r#"
            query GetResource($id: ID!) {
                resource(id: $id) {
                    id
                    name
                    type
                    config
                    tags
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: ResourceId,
        }

        #[derive(Deserialize)]
        struct Response {
            resource: ResourceData,
        }

        let response: Response = self.client.graphql(query, Variables { id }).await?;

        Ok(Resource {
            client: self.client.clone(),
            data: response.resource,
        })
    }

    /// Create a resource from workflow ID
    pub async fn create_from_workflow(&self, workflow_id: WorkflowId) -> Result<ResourceBuilder> {
        Ok(ResourceBuilder::new(self.client.clone()).set_workflow_id(workflow_id))
    }

    /// List resources with optional filters
    /// List all resources
    pub async fn list(&self) -> Result<Vec<Resource>> {
        let query = r#"
            query ListResources {
                resources {
                    id
                    name
                    type
                    config
                    tags
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Deserialize)]
        struct Response {
            resources: Vec<ResourceData>,
        }

        let response: Response = self.client.graphql(query, ()).await?;

        Ok(response
            .resources
            .into_iter()
            .map(|data| Resource {
                client: self.client.clone(),
                data,
            })
            .collect())
    }

    /// Execute an activity on a resource
    pub async fn execute_activity(
        &self,
        resource_id: impl Into<String>,
        activity_id: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Result<Resource> {
        let mutation = r#"
            mutation ExecuteActivity($input: ActivityExecuteInput!) {
                executeActivity(input: $input) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            input: ActivityExecuteInput,
        }

        #[derive(Serialize)]
        struct ActivityExecuteInput {
            #[serde(rename = "resourceId")]
            resource_id: String,
            #[serde(rename = "activityId")]
            activity_id: String,
            data: Option<serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "executeActivity")]
            execute_activity: ResourceGQLData,
        }

        #[derive(Deserialize)]
        struct ResourceGQLData {
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
        }

        let response: Response = self
            .client
            .graphql(
                mutation,
                Variables {
                    input: ActivityExecuteInput {
                        resource_id: resource_id.into(),
                        activity_id: activity_id.into(),
                        data,
                    },
                },
            )
            .await?;

        // Convert ResourceGQLData to ResourceData for compatibility
        let resource_data = ResourceData {
            id: ResourceId::parse_str(&response.execute_activity.id)
                .unwrap_or_else(|_| ResourceId::new_v4()),
            name: "Resource".to_string(), // Default name
            resource_type: "workflow_resource".to_string(), // Default type
            config: response.execute_activity.data,
            tags: Vec::new(),
            state: Some(response.execute_activity.state),
            created_at: chrono::DateTime::parse_from_rfc3339(&response.execute_activity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&response.execute_activity.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        };

        Ok(Resource {
            client: self.client.clone(),
            data: resource_data,
        })
    }

    /// Get resource execution history
    pub async fn get_history(
        &self,
        resource_id: impl Into<String>,
        options: Option<PaginationOptions>,
    ) -> Result<PaginatedResult<ResourceExecution>> {
        let query = r#"
            query GetResource($id: ID!) {
                resource(id: $id) {
                    id
                    history {
                        timestamp
                        activity
                        fromState
                        toState
                        data
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
            resource: ResourceWithHistory,
        }

        #[derive(Deserialize)]
        struct ResourceWithHistory {
            id: String,
            history: Vec<HistoryEvent>,
        }

        #[derive(Deserialize)]
        struct HistoryEvent {
            timestamp: String,
            activity: String,
            #[serde(rename = "fromState")]
            from_state: String,
            #[serde(rename = "toState")]
            to_state: String,
            data: Option<serde_json::Value>,
        }

        let response: Response = self
            .client
            .graphql(
                query,
                Variables {
                    id: resource_id.into(),
                },
            )
            .await?;

        // Convert history events to resource executions
        let executions: Vec<ResourceExecution> = response
            .resource
            .history
            .into_iter()
            .enumerate()
            .map(|(index, event)| ResourceExecution {
                id: format!("history_{}", index),
                activity: event.activity,
                from_state: event.from_state,
                to_state: event.to_state,
                timestamp: chrono::DateTime::parse_from_rfc3339(&event.timestamp)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                data: event.data,
                metadata: None, // Not provided in history events
            })
            .collect();

        let limit = options
            .as_ref()
            .and_then(|o| o.limit)
            .unwrap_or(executions.len());
        let limited_executions = executions.into_iter().take(limit).collect::<Vec<_>>();

        Ok(PaginatedResult {
            data: limited_executions,
            total: limit,
            has_more: false,
        })
    }
}

/// Builder for creating resources
pub struct ResourceBuilder {
    client: Client,
    name: Option<String>,
    resource_type: Option<String>,
    config: serde_json::Value,
    tags: Vec<String>,
    workflow_id: Option<WorkflowId>,
    data: HashMap<String, serde_json::Value>,
    initial_state: Option<String>,
}

impl ResourceBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            name: None,
            resource_type: None,
            config: serde_json::Value::Null,
            tags: Vec::new(),
            workflow_id: None,
            data: HashMap::new(),
            initial_state: None,
        }
    }

    /// Set the resource name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the resource type
    pub fn resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Set the resource configuration
    pub fn config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    /// Add a tag
    /// Add a tag to the resource
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the workflow ID for this resource
    pub fn set_workflow_id(mut self, workflow_id: WorkflowId) -> Self {
        self.workflow_id = Some(workflow_id);
        self
    }

    /// Add data to the resource
    pub fn add_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Set the initial state for this resource
    pub fn set_initial_state(mut self, state: impl Into<String>) -> Self {
        self.initial_state = Some(state.into());
        self
    }

    /// Add multiple tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Build and create the resource
    pub async fn build(self) -> Result<Resource> {
        let workflow_id = self.workflow_id.ok_or_else(|| crate::Error::Validation {
            message: "Workflow ID is required".to_string(),
        })?;

        let mutation = r#"
            mutation CreateResource($input: ResourceCreateInput!) {
                createResource(input: $input) {
                    id
                    workflowId
                    state
                    data
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            input: CreateResourceInput,
        }

        #[derive(Serialize)]
        struct CreateResourceInput {
            #[serde(rename = "workflowId")]
            workflow_id: WorkflowId,
            #[serde(rename = "initialState")]
            initial_state: Option<String>,
            data: Option<serde_json::Value>,
            metadata: Option<serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "createResource")]
            create_resource: SimpleResourceData,
        }

        #[derive(Deserialize)]
        struct SimpleResourceData {
            id: ResourceId,
            #[serde(rename = "workflowId")]
            workflow_id: WorkflowId,
            state: String,
            data: Option<serde_json::Value>,
            #[serde(rename = "createdAt")]
            created_at: String,
            #[serde(rename = "updatedAt")]
            updated_at: String,
        }

        // Convert data HashMap to JSON value
        let data_json = if self.data.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&self.data)?)
        };

        let response: Response = self
            .client
            .graphql(
                mutation,
                Variables {
                    input: CreateResourceInput {
                        workflow_id,
                        initial_state: self.initial_state,
                        data: data_json,
                        metadata: Some(serde_json::json!({})),
                    },
                },
            )
            .await?;

        // Convert to full ResourceData
        let resource_data = ResourceData {
            id: response.create_resource.id,
            name: self.name.unwrap_or_else(|| "Default Resource".to_string()),
            resource_type: self.resource_type.unwrap_or_else(|| "workflow".to_string()),
            config: serde_json::json!({}),
            tags: self.tags,
            state: Some(response.create_resource.state),
            created_at: chrono::DateTime::parse_from_rfc3339(&response.create_resource.created_at)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&response.create_resource.updated_at)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .with_timezone(&chrono::Utc),
        };

        Ok(Resource {
            client: self.client,
            data: resource_data,
        })
    }
}

/// A resource instance
#[derive(Debug, Clone)]
pub struct Resource {
    client: Client,
    data: ResourceData,
}

impl Resource {
    /// Get the resource ID
    pub fn id(&self) -> ResourceId {
        self.data.id
    }

    /// Get the resource name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// Get the resource type
    pub fn resource_type(&self) -> &str {
        &self.data.resource_type
    }

    /// Get the resource tags
    pub fn tags(&self) -> &[String] {
        &self.data.tags
    }

    /// Get the resource configuration
    pub fn config(&self) -> &serde_json::Value {
        &self.data.config
    }

    /// Get the resource state
    pub fn state(&self) -> Option<&str> {
        self.data.state.as_deref()
    }

    /// Test the resource connection
    pub async fn test_connection(&self) -> Result<bool> {
        let mutation = r#"
            mutation TestResource($id: ID!) {
                testResource(id: $id) {
                    success
                    message
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: ResourceId,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "testResource")]
            test_resource: TestResult,
        }

        #[derive(Deserialize)]
        struct TestResult {
            success: bool,
            message: Option<String>,
        }

        let response: Response = self
            .client
            .graphql(mutation, Variables { id: self.data.id })
            .await?;

        Ok(response.test_resource.success)
    }

    /// Delete the resource
    pub async fn delete(self) -> Result<()> {
        let mutation = r#"
            mutation DeleteResource($id: ID!) {
                deleteResource(id: $id) {
                    success
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: ResourceId,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "deleteResource")]
            delete_resource: DeleteResult,
        }

        #[derive(Deserialize)]
        struct DeleteResult {
            success: bool,
        }

        let _response: Response = self
            .client
            .graphql(mutation, Variables { id: self.data.id })
            .await?;

        Ok(())
    }
}

// Internal data structures
#[derive(Debug, Clone, Deserialize)]
struct ResourceData {
    id: ResourceId,
    name: String,
    resource_type: String,
    config: serde_json::Value,
    tags: Vec<String>,
    state: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Resource execution history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceExecution {
    pub id: String,
    pub activity: String,
    pub from_state: String,
    pub to_state: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Convenience function to create a resource builder from workflow ID
pub fn create_resource(workflow_id: WorkflowId) -> ResourceBuilderStandalone {
    ResourceBuilderStandalone::new(workflow_id)
}

/// Standalone resource builder that can be used without a client initially
pub struct ResourceBuilderStandalone {
    workflow_id: WorkflowId,
    data: HashMap<String, serde_json::Value>,
    initial_state: Option<String>,
}

impl ResourceBuilderStandalone {
    fn new(workflow_id: WorkflowId) -> Self {
        Self {
            workflow_id,
            data: HashMap::new(),
            initial_state: None,
        }
    }

    /// Add data to the resource
    pub fn add_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Set the initial state
    pub fn set_initial_state(mut self, state: impl Into<String>) -> Self {
        self.initial_state = Some(state.into());
        self
    }

    /// Build the resource definition
    pub fn build(self) -> ResourceDefinition {
        ResourceDefinition {
            workflow_id: self.workflow_id,
            data: self.data,
            initial_state: self.initial_state,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceDefinition {
    pub workflow_id: WorkflowId,
    pub data: HashMap<String, serde_json::Value>,
    pub initial_state: Option<String>,
}
