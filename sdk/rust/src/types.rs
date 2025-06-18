//! Common types used throughout the Circuit Breaker SDK
//!
//! This module defines the core data types for workflows, agents, functions, and other
//! resources that are used across the SDK.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for workflows
pub type WorkflowId = Uuid;

/// Unique identifier for agents
pub type AgentId = Uuid;

/// Unique identifier for functions
pub type FunctionId = Uuid;

/// Unique identifier for resources
pub type ResourceId = Uuid;

/// Unique identifier for rules
pub type RuleId = Uuid;

/// Unique identifier for executions
pub type ExecutionId = String;

// ============================================================================
// Workflow Types
// ============================================================================

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: Option<WorkflowId>,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub activities: Vec<ActivityDefinition>,
    pub triggers: Vec<TriggerDefinition>,
    pub variables: HashMap<String, serde_json::Value>,
    pub settings: WorkflowSettings,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Activity definition within a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDefinition {
    pub id: String,
    pub name: String,
    pub activity_type: ActivityType,
    pub config: serde_json::Value,
    pub dependencies: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub retry_policy: Option<RetryPolicy>,
}

/// Types of activities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActivityType {
    #[serde(rename = "function")]
    Function { function_id: FunctionId },
    #[serde(rename = "agent")]
    Agent { agent_id: AgentId },
    #[serde(rename = "http")]
    Http { url: String, method: String },
    #[serde(rename = "condition")]
    Condition { expression: String },
    #[serde(rename = "parallel")]
    Parallel { activities: Vec<String> },
    #[serde(rename = "sequence")]
    Sequence { activities: Vec<String> },
}

/// Trigger definition for workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDefinition {
    pub id: String,
    pub name: String,
    pub trigger_type: TriggerType,
    pub config: serde_json::Value,
    pub enabled: bool,
}

/// Types of triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerType {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "schedule")]
    Schedule { cron: String },
    #[serde(rename = "webhook")]
    Webhook { path: String },
    #[serde(rename = "event")]
    Event { event_type: String },
}

/// Workflow settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub max_concurrent_executions: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub retry_policy: Option<RetryPolicy>,
    pub error_handling: ErrorHandling,
}

/// Error handling strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandling {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "continue")]
    Continue,
    #[serde(rename = "retry")]
    Retry,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_conditions: Vec<String>,
}

/// Backoff strategies for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackoffStrategy {
    #[serde(rename = "fixed")]
    Fixed { delay_ms: u64 },
    #[serde(rename = "exponential")]
    Exponential {
        initial_delay_ms: u64,
        multiplier: f64,
        max_delay_ms: Option<u64>,
    },
    #[serde(rename = "linear")]
    Linear {
        initial_delay_ms: u64,
        increment_ms: u64,
    },
}

// ============================================================================
// Agent Types
// ============================================================================

/// Agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: Option<AgentId>,
    pub name: String,
    pub description: Option<String>,
    pub agent_type: AgentType,
    pub config: serde_json::Value,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Types of agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    #[serde(rename = "conversational")]
    Conversational,
    #[serde(rename = "state_machine")]
    StateMachine,
    #[serde(rename = "workflow_integrated")]
    WorkflowIntegrated,
}

// ============================================================================
// Function Types
// ============================================================================

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub id: Option<FunctionId>,
    pub name: String,
    pub description: Option<String>,
    pub runtime: FunctionRuntime,
    pub code: FunctionCode,
    pub entrypoint: String,
    pub environment: HashMap<String, String>,
    pub timeout_ms: u64,
    pub memory_mb: u32,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Supported function runtimes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionRuntime {
    #[serde(rename = "nodejs18")]
    NodeJS18,
    #[serde(rename = "nodejs20")]
    NodeJS20,
    #[serde(rename = "python39")]
    Python39,
    #[serde(rename = "python310")]
    Python310,
    #[serde(rename = "python311")]
    Python311,
    #[serde(rename = "go119")]
    Go119,
    #[serde(rename = "go120")]
    Go120,
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "java11")]
    Java11,
    #[serde(rename = "java17")]
    Java17,
}

/// Function code source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FunctionCode {
    #[serde(rename = "inline")]
    Inline { source: String },
    #[serde(rename = "zip")]
    Zip {
        url: String,
        checksum: Option<String>,
    },
    #[serde(rename = "git")]
    Git {
        repository: String,
        branch: Option<String>,
        path: Option<String>,
    },
    #[serde(rename = "container")]
    Container { image: String, tag: Option<String> },
}

// ============================================================================
// Execution Types
// ============================================================================

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "timeout")]
    Timeout,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub id: ExecutionId,
    pub status: ExecutionStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
}

// ============================================================================
// LLM Types
// ============================================================================

/// LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<TokenUsage>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
}

/// Chat message roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

/// Chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(rename = "promptTokens")]
    pub prompt_tokens: u32,
    #[serde(rename = "completionTokens")]
    pub completion_tokens: u32,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u32,
}

// ============================================================================
// Pagination Types
// ============================================================================

/// Options for paginated requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationOptions {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub has_more: bool,
}

// ============================================================================
// Resource Types
// ============================================================================

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub id: Option<ResourceId>,
    pub name: String,
    pub resource_type: String,
    pub config: serde_json::Value,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================================
// Rule Types
// ============================================================================

/// Rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    pub id: Option<RuleId>,
    pub name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub condition: String,
    pub actions: Vec<RuleAction>,
    pub enabled: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Types of rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    #[serde(rename = "condition")]
    Condition,
    #[serde(rename = "validation")]
    Validation,
    #[serde(rename = "transformation")]
    Transformation,
    #[serde(rename = "filter")]
    Filter,
}

/// Rule action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub action_type: String,
    pub config: serde_json::Value,
}

// ============================================================================
// Common Request/Response Types
// ============================================================================

/// Standard list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub has_more: bool,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: Some(0),
            limit: Some(50),
        }
    }
}

/// Common filter parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filters {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}
