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

// Declares the `place` submodule from `place.rs`
// Contains PlaceId and TransitionId - the basic building blocks of Petri nets
pub mod place;

// Declares the `transition` submodule from `transition.rs`  
// Contains TransitionDefinition - defines how places connect
pub mod transition;

// Declares the `workflow` submodule from `workflow.rs`
// Contains WorkflowDefinition - the complete workflow structure
pub mod workflow;

// Declares the `token` submodule from `token.rs`
// Contains Token - represents workflow execution instances
pub mod token;

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

/// Re-export the fundamental Petri net building blocks
/// PlaceId represents states, TransitionId represents actions
pub use place::{PlaceId, TransitionId};

/// Re-export transition definitions
/// TransitionDefinition defines how tokens can move between places
pub use transition::TransitionDefinition;

/// Re-export workflow definitions  
/// WorkflowDefinition contains the complete workflow structure
pub use workflow::WorkflowDefinition;

/// Re-export token types
/// - Token: The main workflow execution instance
/// - HistoryEvent: Records each state transition
/// - TokenMetadata: Key-value metadata storage
pub use token::{Token, HistoryEvent, TokenMetadata};

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
    FunctionDefinition, FunctionId, EventTrigger, ContainerConfig, ContainerMount,
    ResourceLimits, FunctionExecution, ExecutionStatus, TriggerEvent, EventType,
    FunctionSchema, InputMapping, ChainCondition, FunctionChain, RetryConfig,
    BackoffStrategy, RetryCondition, ChainExecution, ChainStatus
};

/// Re-export agent types
/// - AgentId: Unique identifier for AI agents
/// - AgentDefinition: Complete agent configuration with LLM settings
/// - LLMProvider: AI provider configuration (OpenAI, Anthropic, etc.)
/// - LLMConfig: LLM generation parameters
/// - AgentPrompts: System and user prompt templates
/// - AgentTransitionConfig: Agent execution in workflow transitions
/// - PlaceAgentConfig: Agent execution for tokens in specific places
/// - PlaceAgentSchedule: Scheduling configuration for place agents
/// - AgentRetryConfig: Retry configuration for agent failures
/// - AgentExecution: Records of agent execution
/// - AgentExecutionStatus: Status of agent executions
/// - AgentStreamEvent: Real-time streaming events from agent execution
/// - Conversation: Agent conversation records
/// - ConversationMessage: Individual messages in conversations
/// - MessageRole: Role of messages (system, user, assistant, tool)
pub use agent::{
    AgentId, AgentDefinition, LLMProvider, LLMConfig, AgentPrompts,
    AgentTransitionConfig, PlaceAgentConfig, PlaceAgentSchedule, AgentRetryConfig,
    AgentExecution, AgentExecutionStatus, AgentStreamEvent, Conversation,
    ConversationMessage, MessageRole
};