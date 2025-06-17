/**
 * Resources module for Circuit Breaker TypeScript SDK
 *
 * This module provides comprehensive resource management functionality including
 * CRUD operations, state transitions, batch operations, and fluent builders.
 */

// Core resource manager
export {
  ResourceManager,
  createResourceManager,
  validateResourceDefinition,
  // Types
  ResourceUpdateInput,
  ResourceSearchOptions,
  ResourceStats,
  ResourceWithWorkflow,
  StateTransitionInput,
  StateTransitionResult,
  ActivityExecutionResult,
  BatchOperationOptions,
  BatchOperationResult,
  ResourceValidationOptions,
  ResourceValidationReport,
  ResourceHealthStatus,
} from "./manager.js";

// Resource builder and utilities
export {
  ResourceBuilder,
  ResourceBuilderResult,
  BatchResourceBuilderResult,
  ConditionalTransitionBuilder,
  createResourceBuilder,
  createResourceTemplate,
  createResourceChain,
  // Types
  ResourceBuilderOptions,
  BatchResourceInput,
  ConditionalTransition,
  ResourceTemplate,
  ResourceChain,
} from "./builder.js";

// Re-export core types that are commonly used with resources
export type {
  Resource,
  ResourceCreateInput,
  ActivityExecuteInput,
  HistoryEvent,
} from "../core/types.js";

// Re-export relevant error types
export {
  ResourceError,
  ResourceNotFoundError,
  ResourceStateError,
  ResourceValidationError,
  StateTransitionError,
  ActivityExecutionError,
} from "../core/errors.js";
