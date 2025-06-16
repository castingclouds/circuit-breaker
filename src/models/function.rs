// Function domain models with chaining support

//! # Function Models with Chaining Support
//!
//! This module defines event-driven functions that can be chained together:
//! - Functions have well-defined input/output schemas using JSON Schema
//! - Function outputs can trigger other functions based on conditions
//! - Function results are available to the rules engine
//! - Supports complex data flow between functions

use super::{ActivityId, ResourceMetadata, Rule, StateId};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for functions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionId(String);

impl FunctionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for FunctionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for FunctionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// JSON Schema definition for function inputs/outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSchema {
    /// JSON Schema for validation
    pub schema: serde_json::Value,
    /// Human-readable description
    pub description: Option<String>,
    /// Example data matching this schema
    pub examples: Vec<serde_json::Value>,
}

/// Event types that can trigger functions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Token was created in a specific place
    TokenCreated { place: Option<StateId> },
    /// Token transitioned from one place to another
    TokenTransitioned {
        from: Option<StateId>,
        to: Option<StateId>,
        transition: Option<ActivityId>,
    },
    /// Token was updated (metadata or data changed)
    TokenUpdated { place: Option<StateId> },
    /// Token completed in a specific place
    TokenCompleted { place: Option<StateId> },
    /// Workflow was created
    WorkflowCreated,
    /// Function completed execution (for chaining)
    FunctionCompleted {
        function_id: FunctionId,
        success: bool,
    },
    /// Custom event with arbitrary data
    Custom { event_name: String },
}

/// Input mapping strategies for function chaining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputMapping {
    /// Pass the entire function output as input to next function
    FullOutput,
    /// Map specific output fields to input fields
    FieldMapping(HashMap<String, String>), // output_field -> input_field
    /// Use a template to transform the data
    Template(serde_json::Value), // JSON template with placeholders
    /// Merge function output with token data
    MergedData,
    /// Custom transformation script (future)
    Script(String),
}

/// Conditions for triggering function chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainCondition {
    /// Always trigger the next function
    Always,
    /// Only trigger if the function succeeded (exit code 0)
    OnSuccess,
    /// Only trigger if the function failed (non-zero exit code)
    OnFailure,
    /// Use a rule to determine if chain should trigger
    ConditionalRule(Rule),
    /// Custom condition script (future)
    Script(String),
}

/// Defines when and how a function should be triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    pub id: String,
    pub event_type: EventType,
    pub workflow_id: Option<String>, // Optional: only trigger for specific workflows
    pub conditions: Vec<String>,     // Optional: additional conditions using rules engine
    pub description: Option<String>,
    /// How to extract/transform data for function input
    pub input_mapping: InputMapping,
}

/// Function chaining definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChain {
    /// The function to trigger when this chain condition is met
    pub target_function: FunctionId,
    /// Condition that determines if this chain should trigger
    pub condition: ChainCondition,
    /// How to map this function's output to the target function's input
    pub input_mapping: InputMapping,
    /// Optional delay before triggering the next function
    pub delay: Option<Duration>,
    /// Description of this chain relationship
    pub description: Option<String>,
}

/// Docker container mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMount {
    /// Path on host or reference to stored content
    pub source: String,
    /// Path inside container
    pub target: String,
    /// Mount as readonly
    pub readonly: bool,
}

/// Resource limits for container execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Memory limit in MB
    pub memory_mb: Option<u64>,
    /// CPU limit (number of cores)
    pub cpu_cores: Option<f64>,
    /// Execution timeout in seconds
    pub timeout_seconds: Option<u64>,
}

/// Retry configuration for failed function executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before retry
    pub retry_delay: Duration,
    /// Backoff strategy for subsequent retries
    pub backoff_strategy: BackoffStrategy,
    /// Conditions that warrant a retry
    pub retry_conditions: Vec<RetryCondition>,
}

/// Backoff strategy for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Exponential backoff with multiplier
    Exponential { multiplier: f64 },
    /// Linear increase in delay
    Linear { increment: Duration },
}

/// Conditions that warrant retrying a failed function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    /// Retry on specific exit codes
    ExitCode(Vec<i32>),
    /// Retry on timeout
    Timeout,
    /// Retry on container startup failures
    ContainerFailure,
    /// Retry on network errors
    NetworkError,
    /// Custom condition based on output
    OutputPattern(String),
}

/// Docker container configuration similar to Dagger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Base image to use (e.g., "node:18-alpine", "python:3.11-slim")
    pub image: String,
    /// Working directory inside container
    pub working_dir: Option<String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Secret environment variables (handled securely)
    pub secret_vars: HashMap<String, String>, // Key is env var name, value is secret reference
    /// Commands to run before the main function
    pub setup_commands: Vec<Vec<String>>, // Each Vec<String> is a command with args
    /// Main command to execute the function
    pub exec_command: Vec<String>,
    /// Files to mount into the container
    pub mounts: Vec<ContainerMount>,
    /// Resource limits
    pub resources: Option<ResourceLimits>,
    /// Network settings
    pub network_mode: Option<String>,
    /// Exposed ports
    pub exposed_ports: Vec<u16>,
}

/// Function execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Starting,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
    Retrying,
}

/// A function definition that can be triggered by events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub id: FunctionId,
    pub name: String,
    pub description: Option<String>,
    pub triggers: Vec<EventTrigger>,
    pub container: ContainerConfig,
    pub input_schema: Option<FunctionSchema>,
    pub output_schema: Option<FunctionSchema>,
    pub chains: Vec<FunctionChain>,
    pub retry_config: Option<RetryConfig>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub enabled: bool,
    pub tags: Vec<String>,
    pub version: String,
}

/// Represents a function execution instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecution {
    pub id: Uuid,
    pub function_id: FunctionId,
    pub trigger_event: String,         // JSON-serialized event data
    pub input_data: serde_json::Value, // Processed input for the function
    pub status: ExecutionStatus,
    pub container_id: Option<String>, // Docker container ID
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub output_data: Option<serde_json::Value>, // Validated function output
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub parent_execution_id: Option<Uuid>, // If this was triggered by another function
    pub chain_position: u32,               // Position in the execution chain
    pub created_at: DateTime<Utc>,
}

/// Event payload that triggered a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    pub id: Uuid,
    pub event_type: EventType,
    pub workflow_id: String,
    pub token_id: Option<Uuid>,
    pub data: serde_json::Value, // Event-specific data
    pub metadata: ResourceMetadata,
    pub timestamp: DateTime<Utc>,
}

/// Chain execution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainExecution {
    pub id: Uuid,
    pub root_execution_id: Uuid, // The first execution that started this chain
    pub executions: Vec<Uuid>,   // All executions in this chain
    pub status: ChainStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Status of a function execution chain
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainStatus {
    Running,
    Completed,
    Failed,
    PartiallyCompleted,
}

// Implementation methods

impl FunctionDefinition {
    /// Create a new function definition
    pub fn new(
        id: impl Into<FunctionId>,
        name: impl Into<String>,
        container: ContainerConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            triggers: Vec::new(),
            container,
            input_schema: None,
            output_schema: None,
            chains: Vec::new(),
            retry_config: None,
            created_at: now,
            updated_at: now,
            enabled: true,
            tags: Vec::new(),
            version: "1.0.0".to_string(),
        }
    }

    /// Add an event trigger to this function
    pub fn add_trigger(&mut self, trigger: EventTrigger) {
        self.triggers.push(trigger);
        self.updated_at = Utc::now();
    }

    /// Add a function chain
    pub fn add_chain(&mut self, chain: FunctionChain) {
        self.chains.push(chain);
        self.updated_at = Utc::now();
    }

    /// Set input schema
    pub fn with_input_schema(mut self, schema: FunctionSchema) -> Self {
        self.input_schema = Some(schema);
        self.updated_at = Utc::now();
        self
    }

    /// Set output schema
    pub fn with_output_schema(mut self, schema: FunctionSchema) -> Self {
        self.output_schema = Some(schema);
        self.updated_at = Utc::now();
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = Some(retry_config);
        self.updated_at = Utc::now();
        self
    }

    /// Check if this function should be triggered by an event
    pub fn matches_event(&self, event: &TriggerEvent) -> bool {
        if !self.enabled {
            return false;
        }

        self.triggers.iter().any(|trigger| {
            // Check workflow ID filter if specified
            if let Some(workflow_filter) = &trigger.workflow_id {
                if *workflow_filter != event.workflow_id {
                    return false;
                }
            }

            // Check event type match
            self.event_type_matches(&trigger.event_type, &event.event_type, event)
        })
    }

    /// Check if event types match with optional filters
    pub fn event_type_matches(
        &self,
        trigger_type: &EventType,
        event_type: &EventType,
        _event: &TriggerEvent,
    ) -> bool {
        match (trigger_type, event_type) {
            (
                EventType::TokenCreated {
                    place: filter_place,
                },
                EventType::TokenCreated { place: event_place },
            ) => filter_place.is_none() || filter_place == event_place,
            (
                EventType::TokenTransitioned {
                    from: filter_from,
                    to: filter_to,
                    transition: filter_transition,
                },
                EventType::TokenTransitioned {
                    from: event_from,
                    to: event_to,
                    transition: event_transition,
                },
            ) => {
                (filter_from.is_none() || filter_from == event_from)
                    && (filter_to.is_none() || filter_to == event_to)
                    && (filter_transition.is_none() || filter_transition == event_transition)
            }
            (
                EventType::TokenUpdated {
                    place: filter_place,
                },
                EventType::TokenUpdated { place: event_place },
            ) => filter_place.is_none() || filter_place == event_place,
            (
                EventType::TokenCompleted {
                    place: filter_place,
                },
                EventType::TokenCompleted { place: event_place },
            ) => filter_place.is_none() || filter_place == event_place,
            (EventType::WorkflowCreated, EventType::WorkflowCreated) => true,
            (
                EventType::Custom {
                    event_name: filter_name,
                },
                EventType::Custom {
                    event_name: event_name,
                },
            ) => filter_name == event_name,
            (
                EventType::FunctionCompleted {
                    function_id: filter_id,
                    success: filter_success,
                },
                EventType::FunctionCompleted {
                    function_id: event_id,
                    success: event_success,
                },
            ) => filter_id == event_id && filter_success == event_success,
            _ => false,
        }
    }

    /// Validate input data against schema
    pub fn validate_input(&self, data: &serde_json::Value) -> Result<(), String> {
        if let Some(schema) = &self.input_schema {
            // TODO: Implement JSON Schema validation
            // For now, just check if it's valid JSON
            if data.is_null()
                && !schema
                    .schema
                    .get("required")
                    .unwrap_or(&serde_json::Value::Null)
                    .is_null()
            {
                return Err("Input data is required".to_string());
            }
        }
        Ok(())
    }

    /// Validate output data against schema
    pub fn validate_output(&self, data: &serde_json::Value) -> Result<(), String> {
        if let Some(_schema) = &self.output_schema {
            // TODO: Implement JSON Schema validation
            // For now, just check if it's valid JSON
            if data.is_null() {
                return Err("Output data cannot be null".to_string());
            }
        }
        Ok(())
    }
}

impl FunctionExecution {
    /// Create a new function execution
    pub fn new(function_id: FunctionId, trigger_event: TriggerEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            function_id,
            trigger_event: serde_json::to_string(&trigger_event).unwrap_or_default(),
            input_data: serde_json::Value::Null,
            status: ExecutionStatus::Pending,
            container_id: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            output_data: None,
            error_message: None,
            retry_count: 0,
            next_retry_at: None,
            parent_execution_id: None,
            chain_position: 0,
            created_at: Utc::now(),
        }
    }

    /// Create a chained execution (triggered by another function)
    pub fn new_chained(
        function_id: FunctionId,
        parent_execution_id: Uuid,
        chain_position: u32,
        input_data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            function_id,
            trigger_event: "{}".to_string(), // Empty for chained executions
            input_data,
            status: ExecutionStatus::Pending,
            container_id: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            output_data: None,
            error_message: None,
            retry_count: 0,
            next_retry_at: None,
            parent_execution_id: Some(parent_execution_id),
            chain_position,
            created_at: Utc::now(),
        }
    }

    /// Mark execution as started
    pub fn start(&mut self, container_id: Option<String>) {
        self.status = ExecutionStatus::Running;
        self.container_id = container_id;
        self.started_at = Some(Utc::now());
    }

    /// Mark execution as completed
    pub fn complete(&mut self, exit_code: i32, stdout: Option<String>, stderr: Option<String>) {
        self.status = if exit_code == 0 {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };
        self.exit_code = Some(exit_code);
        self.stdout = stdout;
        self.stderr = stderr;
        self.completed_at = Some(Utc::now());
    }

    /// Mark execution as failed
    pub fn fail(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.error_message = Some(error);
        self.completed_at = Some(Utc::now());
    }

    /// Mark execution for retry
    pub fn schedule_retry(&mut self, delay: Duration) {
        self.status = ExecutionStatus::Retrying;
        self.retry_count += 1;
        self.next_retry_at = Some(Utc::now() + delay);
    }

    /// Get execution duration if completed
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    /// Check if execution succeeded
    pub fn succeeded(&self) -> bool {
        self.status == ExecutionStatus::Completed && self.exit_code == Some(0)
    }

    /// Check if execution failed
    pub fn failed(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Failed | ExecutionStatus::Timeout
        )
    }
}

impl ContainerConfig {
    /// Create a simple container config with just an image
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            working_dir: None,
            env_vars: HashMap::new(),
            secret_vars: HashMap::new(),
            setup_commands: Vec::new(),
            exec_command: Vec::new(),
            mounts: Vec::new(),
            resources: None,
            network_mode: None,
            exposed_ports: Vec::new(),
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add environment variable
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Add secret environment variable
    pub fn with_secret_var(
        mut self,
        key: impl Into<String>,
        secret_ref: impl Into<String>,
    ) -> Self {
        self.secret_vars.insert(key.into(), secret_ref.into());
        self
    }

    /// Add setup command
    pub fn with_setup_command(mut self, command: Vec<String>) -> Self {
        self.setup_commands.push(command);
        self
    }

    /// Set main execution command
    pub fn with_exec(mut self, command: Vec<String>) -> Self {
        self.exec_command = command;
        self
    }

    /// Add file mount
    pub fn with_mount(mut self, mount: ContainerMount) -> Self {
        self.mounts.push(mount);
        self
    }

    /// Set resource limits
    pub fn with_resources(mut self, resources: ResourceLimits) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set network mode
    pub fn with_network_mode(mut self, mode: impl Into<String>) -> Self {
        self.network_mode = Some(mode.into());
        self
    }

    /// Add exposed port
    pub fn with_exposed_port(mut self, port: u16) -> Self {
        self.exposed_ports.push(port);
        self
    }
}

impl EventTrigger {
    /// Create a new event trigger
    pub fn new(id: impl Into<String>, event_type: EventType) -> Self {
        Self {
            id: id.into(),
            event_type,
            workflow_id: None,
            conditions: Vec::new(),
            description: None,
            input_mapping: InputMapping::FullOutput,
        }
    }

    /// Create trigger for token created
    pub fn on_token_created(id: impl Into<String>, place: Option<StateId>) -> Self {
        Self::new(id, EventType::TokenCreated { place })
    }

    /// Create trigger for token transition
    pub fn on_token_transitioned(
        id: impl Into<String>,
        from: Option<StateId>,
        to: Option<StateId>,
        transition: Option<ActivityId>,
    ) -> Self {
        Self::new(
            id,
            EventType::TokenTransitioned {
                from,
                to,
                transition,
            },
        )
    }

    /// Create trigger for token update
    pub fn on_token_updated(id: impl Into<String>, place: Option<StateId>) -> Self {
        Self::new(id, EventType::TokenUpdated { place })
    }

    /// Create trigger for function completion
    pub fn on_function_completed(
        id: impl Into<String>,
        function_id: FunctionId,
        success: bool,
    ) -> Self {
        Self::new(
            id,
            EventType::FunctionCompleted {
                function_id,
                success,
            },
        )
    }

    /// Limit trigger to specific workflow
    pub fn for_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// Add condition using rules engine
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.conditions.push(condition.into());
        self
    }

    /// Set input mapping strategy
    pub fn with_input_mapping(mut self, mapping: InputMapping) -> Self {
        self.input_mapping = mapping;
        self
    }
}

impl TriggerEvent {
    /// Create a token created event
    pub fn token_created(
        workflow_id: impl Into<String>,
        token_id: Uuid,
        place: StateId,
        data: serde_json::Value,
        metadata: ResourceMetadata,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: EventType::TokenCreated { place: Some(place) },
            workflow_id: workflow_id.into(),
            token_id: Some(token_id),
            data,
            metadata,
            timestamp: Utc::now(),
        }
    }

    /// Create a token transitioned event
    pub fn token_transitioned(
        workflow_id: impl Into<String>,
        token_id: Uuid,
        from: StateId,
        to: StateId,
        transition: ActivityId,
        data: serde_json::Value,
        metadata: ResourceMetadata,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: EventType::TokenTransitioned {
                from: Some(from),
                to: Some(to),
                transition: Some(transition),
            },
            workflow_id: workflow_id.into(),
            token_id: Some(token_id),
            data,
            metadata,
            timestamp: Utc::now(),
        }
    }

    /// Create a function completed event
    pub fn function_completed(
        workflow_id: impl Into<String>,
        function_id: FunctionId,
        success: bool,
        output_data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: EventType::FunctionCompleted {
                function_id,
                success,
            },
            workflow_id: workflow_id.into(),
            token_id: None,
            data: output_data,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }
}

impl FunctionSchema {
    /// Create a simple schema from a JSON value
    pub fn new(schema: serde_json::Value) -> Self {
        Self {
            schema,
            description: None,
            examples: Vec::new(),
        }
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add example
    pub fn with_example(mut self, example: serde_json::Value) -> Self {
        self.examples.push(example);
        self
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            retry_delay: Duration::seconds(30),
            backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
            retry_conditions: vec![
                RetryCondition::ContainerFailure,
                RetryCondition::NetworkError,
                RetryCondition::Timeout,
            ],
        }
    }
}

impl ChainExecution {
    /// Create a new chain execution tracker
    pub fn new(root_execution_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            root_execution_id,
            executions: vec![root_execution_id],
            status: ChainStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Add an execution to this chain
    pub fn add_execution(&mut self, execution_id: Uuid) {
        self.executions.push(execution_id);
    }

    /// Mark chain as completed
    pub fn complete(&mut self, status: ChainStatus) {
        self.status = status;
        self.completed_at = Some(Utc::now());
    }
}
