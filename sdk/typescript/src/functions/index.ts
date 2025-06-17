/**
 * Functions module for Circuit Breaker TypeScript SDK
 *
 * This module provides comprehensive function management functionality including
 * Docker-based function execution, container lifecycle management, function
 * orchestration, and advanced execution features.
 */

// Core function manager
export {
  FunctionManager,
  createFunctionManager,
  validateFunctionDefinition,
  // Types
  FunctionCreateInput,
  FunctionUpdateInput,
  FunctionSearchOptions,
  FunctionExecuteInput,
  FunctionStats,
  FunctionWithStats,
  ContainerStatus,
  BatchExecutionInput,
  BatchExecutionResult,
  FunctionHealthStatus,
  ContainerLogOptions,
} from "./manager.js";

// Function builder and utilities
export {
  FunctionBuilder,
  FunctionBuilderInstance,
  FunctionBuilderResult,
  BatchFunctionBuilder,
  FunctionGroupBuilder,
  FunctionPipelineBuilder,
  createFunctionBuilder,
  createFunctionTemplate,
  CommonTemplates,
  // Types
  FunctionBuilderOptions,
  FunctionTemplate,
  FunctionGroup,
  FunctionPipeline,
} from "./builder.js";

// Re-export core types that are commonly used with functions
export type {
  FunctionDefinition,
  FunctionConfig,
  ContainerConfig,
  ResourceLimits,
  EventTrigger,
  FunctionChain,
  FunctionResult,
  ExecutionStatus,
  ResourceUsage,
  RetryConfig,
  ContainerMount,
  EventTriggerType,
  InputMapping,
  ChainCondition,
} from "../core/types.js";

// Re-export relevant error types
export {
  FunctionError,
  FunctionNotFoundError,
  FunctionValidationError,
  FunctionExecutionError,
  FunctionTimeoutError,
  ContainerError,
} from "../core/errors.js";
