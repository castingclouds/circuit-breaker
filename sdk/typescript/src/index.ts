/**
 * Circuit Breaker TypeScript SDK
 *
 * A comprehensive SDK for building and managing workflows using the Circuit Breaker
 * workflow engine. Provides type-safe APIs for workflows, resources, rules engine,
 * functions, LLM integration, and AI agents.
 *
 * @example
 * ```typescript
 * import { CircuitBreakerSDK, createWorkflow } from 'circuit-breaker-sdk';
 *
 * // Create SDK instance
 * const sdk = new CircuitBreakerSDK({
 *   graphqlEndpoint: 'http://localhost:4000/graphql'
 * });
 *
 * // Build a workflow
 * const workflow = createWorkflow('Order Processing')
 *   .addState('pending')
 *   .addState('processing')
 *   .addState('completed')
 *   .addTransition('pending', 'processing', 'start_processing')
 *   .addTransition('processing', 'completed', 'complete_order')
 *   .setInitialState('pending')
 *   .build();
 *
 * // Create the workflow
 * const workflowId = await sdk.workflows.create(workflow);
 *
 * // Create a resource
 * const resource = await sdk.resources.create({
 *   workflowId,
 *   data: { orderId: 'order-123', amount: 99.99 }
 * });
 * ```
 */

// ============================================================================
// Core Exports
// ============================================================================

// Main SDK client
export { CircuitBreakerSDK, createSDK, createSDKAsync } from "./core/client.js";

// Core types
export type {
  SDKConfig,
  LoggingConfig,
  WorkflowDefinition,
  ActivityDefinition,
  Resource,
  ResourceCreateInput,
  ActivityExecuteInput,
  HistoryEvent,
} from "./core/types.js";

// Error classes
export {
  CircuitBreakerError,
  WorkflowError,
  WorkflowNotFoundError,
  WorkflowValidationError,
  InvalidStateTransitionError,
  ResourceError,
  ResourceNotFoundError,
  ResourceStateError,
  ResourceValidationError,
  StateTransitionError,
  ActivityExecutionError,
  RuleError,
  RuleEvaluationError,
  RuleNotFoundError,
  RuleValidationError,
  RuleTimeoutError,
  FunctionError,
  FunctionNotFoundError,
  FunctionValidationError,
  FunctionExecutionError,
  FunctionTimeoutError,
  ContainerError,
  LLMError,
  LLMProviderError,
  LLMProviderNotFoundError,
  LLMModelNotSupportedError,
  LLMRateLimitError,
  LLMQuotaExceededError,
  AgentError,
  AgentNotFoundError,
  AgentConfigurationError,
  NetworkError,
  TimeoutError,
  ConnectionError,
  GraphQLError,
  AuthenticationError,
  AuthorizationError,
  ValidationError,
  SchemaValidationError,
  ErrorFactory,
  ErrorHandler,
  // Error type guards
  isCircuitBreakerError,
  isWorkflowError,
  isResourceError,
  isRuleError,
  isFunctionError,
  isLLMError,
  isAgentError,
  isNetworkError,
  isValidationError,
} from "./core/errors.js";

// ============================================================================
// Workflow Module Exports
// ============================================================================

export {
  WorkflowBuilder,
  BranchBuilder,
  ParallelBuilder,
  LoopBuilder,
  TryCatchBuilder,
  createWorkflow,
  createLinearWorkflow,
  createFromStateMachine,
} from "./workflow/builder.js";

export type {
  WorkflowValidationResult,
  BranchCondition,
  ParallelBranch,
  LoopCondition,
} from "./workflow/builder.js";

// Workflow manager
export { WorkflowManager } from "./workflow/manager.js";
export type {
  WorkflowCreateInput,
  WorkflowUpdateInput,
  WorkflowSearchOptions,
  WorkflowStats,
  WorkflowWithStats,
  WorkflowValidationOptions,
  WorkflowValidationReport,
  WorkflowHealthStatus,
} from "./workflow/manager.js";

// ============================================================================
// Resources Module Exports
// ============================================================================

// Resource manager
export {
  ResourceManager,
  createResourceManager,
  validateResourceDefinition,
} from "./resources/manager.js";

export type {
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
} from "./resources/manager.js";

// Resource builder
export {
  ResourceBuilder,
  ResourceBuilderResult,
  BatchResourceBuilderResult,
  ConditionalTransitionBuilder,
  createResourceBuilder,
  createResourceTemplate,
  createResourceChain,
} from "./resources/builder.js";

export type {
  ResourceBuilderOptions,
  BatchResourceInput,
  ConditionalTransition,
  ResourceTemplate,
  ResourceChain,
} from "./resources/builder.js";

// ============================================================================
// Rules Engine Exports
// ============================================================================

export type {
  RulesConfig,
  Rule,
  RuleType,
  CompositeRule,
  RuleContext,
  RuleEvaluationResult,
  RuleResult,
  RuleValidationResult,
  RuleEvaluator,
} from "./core/types.js";

// Rules engine
export {
  RulesEngine,
  createRulesEngine,
  validateRuleDefinition,
  createSimpleRule,
  createJavaScriptRule,
  createCompositeRule,
  createCustomRule,
} from "./rules/engine.js";

export type {
  RuleCreateInput,
  RuleUpdateInput,
  RuleSearchOptions,
  RuleStats,
  RuleWithStats,
  RuleEvaluationOptions,
  BatchRuleEvaluationInput,
  BatchRuleEvaluationResult,
  RuleHealthStatus,
} from "./rules/engine.js";

// Rule builder
export {
  RuleBuilder,
  SimpleRuleBuilder,
  JavaScriptRuleBuilder,
  CompositeRuleBuilder,
  CustomRuleBuilder,
  RuleBuilderResult,
  BatchRuleBuilder,
  ConditionalGroupBuilder,
  RuleChainBuilder,
  createRuleBuilder,
  createRuleTemplate,
  CommonTemplates,
} from "./rules/builder.js";

export type {
  RuleBuilderOptions,
  RuleTemplate,
  ConditionalRuleGroup,
  RuleChain,
} from "./rules/builder.js";

// ============================================================================
// Function System Exports
// ============================================================================

export type {
  FunctionConfig,
  DockerConfig,
  FunctionDefinition,
  ContainerConfig,
  ContainerMount,
  ResourceLimits,
  EventTrigger,
  EventTriggerType,
  InputMapping,
  FunctionTrigger,
  RetryConfig,
  FunctionChain,
  ChainCondition,
  FunctionResult,
  ExecutionStatus,
  ResourceUsage,
} from "./core/types.js";

// Function system
export {
  FunctionManager,
  createFunctionManager,
  validateFunctionDefinition,
} from "./functions/manager.js";

export type {
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
} from "./functions/manager.js";

// Function builder
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
} from "./functions/builder.js";

export type {
  FunctionBuilderOptions,
  FunctionTemplate,
  FunctionGroup,
  FunctionPipeline,
} from "./functions/builder.js";

// ============================================================================
// LLM Router Exports
// ============================================================================

export type {
  LLMConfig,
  LLMProviderConfig,
  LLMProviderType,
  LoadBalancingConfig,
  FailoverConfig,
  CircuitBreakerConfig,
  RateLimitConfig,
  UsageTrackingConfig,
  ChatCompletionRequest,
  ChatMessage,
  ChatRole,
  ToolCall,
  Tool,
  ChatCompletionResponse,
  Choice,
  Usage,
  ChatCompletionChunk,
  ChoiceDelta,
  MessageDelta,
  ToolCallDelta,
} from "./core/types.js";

// LLM router exports
export { LLMRouter, createLLMRouter, DefaultLLMConfigs } from "./llm/router.js";
export type {
  RoutingStrategy,
  ProviderHealth,
  RoutingInfo,
  LLMRouterStats,
  LLMRouterConfig,
} from "./llm/router.js";

export {
  LLMProvider,
  createOpenAIProvider,
  createAnthropicProvider,
  createOllamaProvider,
  ModelRegistry,
  getProviderForModel,
  isModelSupported,
  getModelsForProvider,
  ModelCapabilities,
  validateProviderConfig,
  DefaultProviderConfigs,
  checkProviderHealth,
  checkAllProviderHealth,
} from "./llm/providers.js";
export type {
  ModelPricing,
  ProviderCapabilities,
  ProviderMetrics,
} from "./llm/providers.js";

export {
  StreamingHandler,
  StreamingSession,
  createStreamingHandler,
  streamFromAsyncGenerator,
  StreamUtils,
} from "./llm/streaming.js";
export type {
  StreamConfig,
  StreamStats,
  StreamEvent,
} from "./llm/streaming.js";

export {
  LLMBuilder,
  MultiProviderBuilder,
  CostOptimizedBuilder,
  PerformanceBuilder,
  createLLMBuilder,
  createMultiProviderBuilder,
  createCostOptimizedBuilder,
  createPerformanceBuilder,
  LLMBuilderTemplates,
} from "./llm/builder.js";
export type {
  LLMBuilderConfig,
  ProviderBuilderConfig,
  HealthCheckBuilderConfig,
  CostTrackingBuilderConfig,
  LoadBalancingBuilderConfig,
  FailoverBuilderConfig,
  LLMBuilderResult,
} from "./llm/builder.js";

// ============================================================================
// Agent System Exports
// ============================================================================

export type {
  AgentDefinition,
  AgentType,
  AgentConfig,
  MemoryConfig,
  StateMachineConfig,
  AgentState,
  StateTransition,
  StateAction,
} from "./core/types.js";

// Agent builder exports
export {
  AgentBuilder,
  ConversationalAgent,
  StateMachineAgent,
  Agent,
  MemoryManager,
  createAgent,
  createConversationalAgent,
  createWorkflowAgent,
  ConversationalAgentBuilder,
  WorkflowAgentBuilder,
  AgentTemplates,
} from "./agents/builder.js";
export type {
  AgentBuilderConfig,
  ConversationalAgentConfig,
  StateMachineAgentConfig,
  MemoryBuilderConfig,
  ToolBuilderConfig,
  ToolImplementation,
  AgentContext,
  AgentBuilderResult,
} from "./agents/builder.js";

// Conversational agent exports
export {
  ConversationalAgent as ConversationalAgentImpl,
  createConversationalAgent as createConversationalAgentDirect,
  ConversationalTemplates,
} from "./agents/conversational.js";
export type {
  ConversationalConfig,
  ConversationTurn,
  ConversationState,
  ConversationMetrics,
} from "./agents/conversational.js";

// State machine agent exports
export {
  StateMachineAgent as StateMachineAgentImpl,
  createStateMachineAgent,
  StateMachineTemplates,
} from "./agents/state-machine.js";
export type {
  StateMachineAgentConfig as StateMachineConfigDetailed,
  StateExecutionContext,
  StateTransitionResult,
  StateMachineMetrics,
  StateMachineSession,
} from "./agents/state-machine.js";

// ============================================================================
// Utility Exports
// ============================================================================

export { GraphQLClient, QueryBuilder } from "./utils/graphql.js";
export type {
  GraphQLRequest,
  GraphQLResponse,
  GraphQLErrorResponse,
  GraphQLClientConfig,
  RequestLog,
} from "./utils/graphql.js";

export {
  Logger,
  createLogger,
  createComponentLogger,
  formatError,
  sanitizeForLogging,
  generateCorrelationId,
  generateRequestId,
  defaultLogger,
  LOG_LEVELS,
  isLogLevel,
  compareLogLevels,
} from "./utils/logger.js";
export type {
  LogLevel,
  LogEntry,
  LoggerConfig,
  LogContext,
  LogStats,
} from "./utils/logger.js";

export type {
  JSONSchema,
  PaginationOptions,
  PaginatedResult,
  FilterOptions,
  DeepPartial,
  RequireFields,
  PartialExcept,
  ArrayElement,
  Awaited,
} from "./core/types.js";

// ============================================================================
// Constants
// ============================================================================

export const SDK_VERSION = "0.1.0";

export const DEFAULT_GRAPHQL_ENDPOINT = "http://localhost:4000/graphql";

export const DEFAULT_TIMEOUT = 30000;

export const SUPPORTED_RULE_TYPES = [
  "simple",
  "composite",
  "custom",
  "javascript",
] as const;

export const SUPPORTED_EXECUTION_STATUSES = [
  "pending",
  "running",
  "success",
  "failure",
  "timeout",
  "cancelled",
] as const;

export const SUPPORTED_LLM_PROVIDERS = [
  "openai",
  "anthropic",
  "ollama",
  "huggingface",
  "cohere",
  "custom",
] as const;

export const SUPPORTED_AGENT_TYPES = [
  "conversational",
  "state_machine",
  "workflow_integrated",
] as const;

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Check if the SDK is compatible with the server version
 */
export function isCompatibleVersion(serverVersion: string): boolean {
  // Simple version compatibility check
  // In a real implementation, this would do semantic version comparison
  const [serverMajor] = serverVersion.split(".");
  const [sdkMajor] = SDK_VERSION.split(".");
  return serverMajor === sdkMajor;
}

/**
 * Get SDK information
 */
export function getSDKInfo(): {
  version: string;
  supportedFeatures: string[];
  defaultEndpoint: string;
} {
  return {
    version: SDK_VERSION,
    supportedFeatures: [
      "workflows",
      "resources",
      "rules",
      "functions",
      "llm",
      "agents",
      "graphql",
      "streaming",
      "validation",
      "error-handling",
    ],
    defaultEndpoint: DEFAULT_GRAPHQL_ENDPOINT,
  };
}

/**
 * Validate SDK configuration
 */
export function validateSDKConfig(config: any): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  if (!config) {
    errors.push("Configuration is required");
    return { valid: false, errors };
  }

  if (!config.graphqlEndpoint) {
    errors.push("GraphQL endpoint is required");
  } else {
    try {
      new URL(config.graphqlEndpoint);
    } catch {
      errors.push("Invalid GraphQL endpoint URL");
    }
  }

  if (
    config.timeout !== undefined &&
    (typeof config.timeout !== "number" || config.timeout <= 0)
  ) {
    errors.push("Timeout must be a positive number");
  }

  if (config.debug !== undefined && typeof config.debug !== "boolean") {
    errors.push("Debug must be a boolean");
  }

  if (config.headers !== undefined && typeof config.headers !== "object") {
    errors.push("Headers must be an object");
  }

  return { valid: errors.length === 0, errors };
}

// ============================================================================
// Re-export Common GraphQL Fragments
// ============================================================================

export {
  WORKFLOW_FRAGMENT,
  RESOURCE_FRAGMENT,
  ACTIVITY_FRAGMENT,
} from "./utils/graphql.js";

// ============================================================================
// Default Export
// ============================================================================

// Export the main SDK class as the default export for convenience
export { CircuitBreakerSDK as default } from "./core/client.js";
