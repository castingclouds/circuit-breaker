// Core domain models for Circuit Breaker
// These are the generic, language-agnostic data structures

//! # Domain Models Module
//!
//! This module contains the core domain models for Circuit Breaker. These are
//! **generic, language-agnostic** data structures that form the foundation of
//! the workflow engine.
//!
//! ## Rust Learning Notes:
//!
//! ### Module Organization
//! This `mod.rs` file serves as the **module root** for the `models` directory.
//! When you have a directory with a `mod.rs` file, Rust treats the directory
//! as a module, and `mod.rs` acts as the entry point.
//!
//! ### Module Declarations
//! Each `pub mod` declaration tells Rust to:
//! 1. Look for a `.rs` file with that name in the same directory
//! 2. Include that file's code as a submodule
//! 3. Make it publicly accessible (because of `pub`)
//!
//! ### Re-exports for Clean APIs
//! The `pub use` statements at the bottom create a clean, flat API.
//! Users can import `use circuit_breaker::models::Token` instead of
//! `use circuit_breaker::models::token::Token`.

// Declares the `state` submodule from `state.rs`
// Contains StateId and ActivityId - the basic building blocks of workflows
pub mod state;

// Declares the `activity` submodule from `activity.rs`
// Contains ActivityDefinition - defines how states connect
pub mod activity;

// Declares the `workflow` submodule from `workflow.rs`
// Contains WorkflowDefinition - the complete workflow structure
pub mod workflow;

// Declares the `resource` submodule from `resource.rs`
// Contains Resource - represents workflow execution instances
pub mod resource;

// Declares the `rule` submodule from `rule.rs`
// Contains Rule and RuleCondition - the rules engine for token gating
pub mod rule;

// Declares the `function` submodule from `function.rs`
// Contains FunctionDefinition and event-driven execution types
pub mod function;

// Declares the `agent` submodule from `agent.rs`
// Contains AgentDefinition and AI agent execution types
pub mod agent;

// Re-export main types for convenience
// This creates shortcuts so users don't need to know the internal structure

/// Re-export the fundamental workflow building blocks
/// StateId represents states, ActivityId represents actions
pub use state::{ActivityId, StateId};

/// Re-export activity definitions
/// ActivityDefinition defines how resources can move between states
pub use activity::ActivityDefinition;

/// Re-export workflow definitions
/// WorkflowDefinition contains the complete workflow structure
pub use workflow::WorkflowDefinition;

/// Re-export resource types
/// - Resource: The main workflow execution instance
/// - HistoryEvent: Records each state transition
/// - ResourceMetadata: Key-value metadata storage
/// - ActivityRecord: NATS-specific activity tracking
pub use resource::{ActivityRecord, HistoryEvent, Resource, ResourceMetadata};

/// Re-export rules engine types
/// - Rule: A single evaluatable condition
/// - RuleCondition: The actual evaluation logic (field checks, logical operations)
/// - RuleEvaluationResult: Detailed results for debugging
pub use rule::{Rule, RuleCondition, RuleEvaluationResult};

/// Re-export function types
/// - FunctionDefinition: Docker-based event-driven functions
/// - FunctionId: Unique identifier for functions
/// - EventTrigger: Defines what events trigger functions
/// - ContainerConfig: Docker container configuration
/// - ContainerMount: File/directory mount configuration
/// - ResourceLimits: Container resource constraints
/// - FunctionExecution: Records function execution results
/// - ExecutionStatus: Function execution state
/// - TriggerEvent: Event payload that triggers functions
/// - EventType: Types of events that can occur
/// - FunctionSchema: JSON Schema for input/output validation
/// - InputMapping: How to map data between functions in chains
/// - ChainCondition: Conditions for triggering function chains
/// - FunctionChain: Function chaining definition
/// - RetryConfig: Retry configuration for failed executions
/// - ChainExecution: Chain execution tracking
/// - ChainStatus: Status of function execution chains
pub use function::{
    BackoffStrategy, ChainCondition, ChainExecution, ChainStatus, ContainerConfig, ContainerMount,
    EventTrigger, EventType, ExecutionStatus, FunctionChain, FunctionDefinition, FunctionExecution,
    FunctionId, FunctionSchema, InputMapping, ResourceLimits, RetryCondition, RetryConfig,
    TriggerEvent,
};

/// Re-export agent types
/// - AgentId: Unique identifier for AI agents
/// - AgentDefinition: Complete agent configuration with LLM settings
/// - LLMProvider: AI provider configuration (OpenAI, Anthropic, etc.)
/// - LLMConfig: LLM generation parameters
/// - AgentPrompts: System and user prompt templates
/// - AgentActivityConfig: Agent execution in workflow activities
/// - StateAgentConfig: DEPRECATED - Agent execution for resources in specific states (will be moved to workflow integration layer)
/// - StateAgentSchedule: DEPRECATED - Scheduling configuration for state agents (will be moved to workflow integration layer)
/// - AgentRetryConfig: Retry configuration for agent failures
/// - AgentExecution: Records of agent execution
/// - AgentExecutionStatus: Status of agent executions
/// - AgentStreamEvent: Real-time streaming events from agent execution
/// - Conversation: Agent conversation records
/// - ConversationMessage: Individual messages in conversations
/// - MessageRole: Role of messages (system, user, assistant, tool)
pub use agent::{
    AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentPrompts, AgentRetryConfig, AgentStreamEvent, Conversation, ConversationMessage, LLMConfig,
    LLMProvider, MessageRole,
};

// Deprecated workflow-specific types (to be moved to integration layer)
#[allow(deprecated)]
pub use agent::{StateAgentConfig, StateAgentSchedule};
