/**
 * Error handling system for Circuit Breaker TypeScript SDK
 *
 * This file provides a comprehensive error hierarchy and handling utilities
 * for all SDK operations including workflows, resources, rules, functions, and LLM operations.
 */

// ============================================================================
// Base Error Classes
// ============================================================================

/**
 * Base error class for all Circuit Breaker SDK errors
 */
export class CircuitBreakerError extends Error {
  public readonly code: string;
  public readonly context?: any;
  public readonly timestamp: Date;
  public readonly requestId?: string;

  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message);
    this.name = "CircuitBreakerError";
    this.code = code;
    this.context = context;
    this.timestamp = new Date();
    this.requestId = requestId;

    // Ensure proper prototype chain for instanceof checks
    Object.setPrototypeOf(this, CircuitBreakerError.prototype);

    // Capture stack trace if available
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, CircuitBreakerError);
    }
  }

  /**
   * Convert error to JSON for logging/serialization
   */
  toJSON(): Record<string, any> {
    return {
      name: this.name,
      message: this.message,
      code: this.code,
      context: this.context,
      timestamp: this.timestamp.toISOString(),
      requestId: this.requestId,
      stack: this.stack,
    };
  }

  /**
   * Check if error is retryable
   */
  isRetryable(): boolean {
    return (
      this.code.startsWith("NETWORK_") ||
      this.code.startsWith("TIMEOUT_") ||
      this.code === "RATE_LIMITED"
    );
  }

  /**
   * Get error severity level
   */
  getSeverity(): "low" | "medium" | "high" | "critical" {
    if (this.code.startsWith("VALIDATION_")) return "low";
    if (this.code.startsWith("NOT_FOUND_")) return "medium";
    if (this.code.startsWith("UNAUTHORIZED_")) return "high";
    if (this.code.startsWith("INTERNAL_")) return "critical";
    return "medium";
  }
}

// ============================================================================
// Workflow-Related Errors
// ============================================================================

export class WorkflowError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "WorkflowError";
    Object.setPrototypeOf(this, WorkflowError.prototype);
  }
}

export class WorkflowNotFoundError extends WorkflowError {
  constructor(workflowId: string, requestId?: string) {
    super(
      `Workflow not found: ${workflowId}`,
      "WORKFLOW_NOT_FOUND",
      { workflowId },
      requestId,
    );
    this.name = "WorkflowNotFoundError";
    Object.setPrototypeOf(this, WorkflowNotFoundError.prototype);
  }
}

export class WorkflowValidationError extends WorkflowError {
  public readonly validationErrors: string[];

  constructor(validationErrors: string[], context?: any, requestId?: string) {
    const message = `Workflow validation failed: ${validationErrors.join(", ")}`;
    super(message, "WORKFLOW_VALIDATION_ERROR", context, requestId);
    this.name = "WorkflowValidationError";
    this.validationErrors = validationErrors;
    Object.setPrototypeOf(this, WorkflowValidationError.prototype);
  }
}

export class InvalidStateTransitionError extends WorkflowError {
  constructor(
    fromState: string,
    toState: string,
    activityId: string,
    requestId?: string,
  ) {
    super(
      `Invalid state transition from '${fromState}' to '${toState}' via activity '${activityId}'`,
      "INVALID_STATE_TRANSITION",
      { fromState, toState, activityId },
      requestId,
    );
    this.name = "InvalidStateTransitionError";
    Object.setPrototypeOf(this, InvalidStateTransitionError.prototype);
  }
}

// ============================================================================
// Resource-Related Errors
// ============================================================================

export class ResourceError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "ResourceError";
    Object.setPrototypeOf(this, ResourceError.prototype);
  }
}

export class ResourceNotFoundError extends ResourceError {
  constructor(resourceId: string, requestId?: string) {
    super(
      `Resource not found: ${resourceId}`,
      "RESOURCE_NOT_FOUND",
      { resourceId },
      requestId,
    );
    this.name = "ResourceNotFoundError";
    Object.setPrototypeOf(this, ResourceNotFoundError.prototype);
  }
}

export class ResourceStateError extends ResourceError {
  constructor(
    resourceId: string,
    expectedState: string,
    actualState: string,
    requestId?: string,
  ) {
    super(
      `Resource ${resourceId} is in state '${actualState}', expected '${expectedState}'`,
      "RESOURCE_INVALID_STATE",
      { resourceId, expectedState, actualState },
      requestId,
    );
    this.name = "ResourceStateError";
    Object.setPrototypeOf(this, ResourceStateError.prototype);
  }
}

export class ResourceValidationError extends ResourceError {
  public readonly validationErrors?: string[];

  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "ResourceValidationError";
    Object.setPrototypeOf(this, ResourceValidationError.prototype);
  }
}

export class StateTransitionError extends ResourceError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "StateTransitionError";
    Object.setPrototypeOf(this, StateTransitionError.prototype);
  }
}

export class ActivityExecutionError extends ResourceError {
  public readonly activityId: string;
  public readonly executionId?: string;

  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "ActivityExecutionError";
    this.activityId = context?.activityId || "";
    this.executionId = context?.executionId;
    Object.setPrototypeOf(this, ActivityExecutionError.prototype);
  }
}

// ============================================================================
// Rules Engine Errors
// ============================================================================

export class RuleError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "RuleError";
    Object.setPrototypeOf(this, RuleError.prototype);
  }
}

export class RuleEvaluationError extends RuleError {
  public readonly ruleName: string;
  public readonly evaluationErrors: string[];

  constructor(
    ruleName: string,
    evaluationErrors: string[],
    context?: any,
    requestId?: string,
  ) {
    const message = `Rule evaluation failed for '${ruleName}': ${evaluationErrors.join(", ")}`;
    super(message, "RULE_EVALUATION_ERROR", context, requestId);
    this.name = "RuleEvaluationError";
    this.ruleName = ruleName;
    this.evaluationErrors = evaluationErrors;
    Object.setPrototypeOf(this, RuleEvaluationError.prototype);
  }
}

export class RuleNotFoundError extends RuleError {
  constructor(ruleName: string, requestId?: string) {
    super(
      `Rule not found: ${ruleName}`,
      "RULE_NOT_FOUND",
      { ruleName },
      requestId,
    );
    this.name = "RuleNotFoundError";
    Object.setPrototypeOf(this, RuleNotFoundError.prototype);
  }
}

export class RuleValidationError extends RuleError {
  public readonly validationErrors: string[];

  constructor(validationErrors: string[], context?: any, requestId?: string) {
    const message = `Rule validation failed: ${validationErrors.join(", ")}`;
    super(message, "RULE_VALIDATION_ERROR", context, requestId);
    this.name = "RuleValidationError";
    this.validationErrors = validationErrors;
    Object.setPrototypeOf(this, RuleValidationError.prototype);
  }
}

export class RuleTimeoutError extends RuleError {
  constructor(ruleName: string, timeout: number, requestId?: string) {
    super(
      `Rule evaluation timeout for '${ruleName}' after ${timeout}ms`,
      "RULE_EVALUATION_TIMEOUT",
      { ruleName, timeout },
      requestId,
    );
    this.name = "RuleTimeoutError";
    Object.setPrototypeOf(this, RuleTimeoutError.prototype);
  }
}

// ============================================================================
// Function System Errors
// ============================================================================

export class FunctionError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "FunctionError";
    Object.setPrototypeOf(this, FunctionError.prototype);
  }
}

export class FunctionNotFoundError extends FunctionError {
  constructor(functionId: string, requestId?: string) {
    super(
      `Function not found: ${functionId}`,
      "FUNCTION_NOT_FOUND",
      { functionId },
      requestId,
    );
    this.name = "FunctionNotFoundError";
    Object.setPrototypeOf(this, FunctionNotFoundError.prototype);
  }
}

export class FunctionExecutionError extends FunctionError {
  public readonly functionId: string;
  public readonly executionId?: string;
  public readonly exitCode?: number;

  constructor(
    functionId: string,
    error: string,
    executionId?: string,
    exitCode?: number,
    requestId?: string,
  ) {
    super(
      `Function execution failed for '${functionId}': ${error}`,
      "FUNCTION_EXECUTION_ERROR",
      { functionId, executionId, exitCode, error },
      requestId,
    );
    this.name = "FunctionExecutionError";
    this.functionId = functionId;
    this.executionId = executionId;
    this.exitCode = exitCode;
    Object.setPrototypeOf(this, FunctionExecutionError.prototype);
  }
}

export class FunctionTimeoutError extends FunctionError {
  constructor(
    functionId: string,
    timeout: number,
    executionId?: string,
    requestId?: string,
  ) {
    super(
      `Function execution timeout for '${functionId}' after ${timeout}ms`,
      "FUNCTION_EXECUTION_TIMEOUT",
      { functionId, timeout, executionId },
      requestId,
    );
    this.name = "FunctionTimeoutError";
    Object.setPrototypeOf(this, FunctionTimeoutError.prototype);
  }
}

export class FunctionValidationError extends FunctionError {
  public readonly validationErrors: string[];

  constructor(validationErrors: string[], context?: any, requestId?: string) {
    const message = `Function validation failed: ${validationErrors.join(", ")}`;
    super(message, "FUNCTION_VALIDATION_ERROR", context, requestId);
    this.name = "FunctionValidationError";
    this.validationErrors = validationErrors;
    Object.setPrototypeOf(this, FunctionValidationError.prototype);
  }
}

export class ContainerError extends FunctionError {
  public readonly containerId?: string;
  public readonly image: string;

  constructor(
    image: string,
    error: string,
    containerId?: string,
    requestId?: string,
  ) {
    super(
      `Container error for image '${image}': ${error}`,
      "CONTAINER_ERROR",
      { image, containerId, error },
      requestId,
    );
    this.name = "ContainerError";
    this.image = image;
    this.containerId = containerId;
    Object.setPrototypeOf(this, ContainerError.prototype);
  }
}

// ============================================================================
// LLM-Related Errors
// ============================================================================

export class LLMError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "LLMError";
    Object.setPrototypeOf(this, LLMError.prototype);
  }
}

export class LLMProviderError extends LLMError {
  public readonly provider: string;
  public readonly model?: string;

  constructor(
    provider: string,
    error: string,
    model?: string,
    requestId?: string,
  ) {
    super(
      `LLM provider error from '${provider}': ${error}`,
      "LLM_PROVIDER_ERROR",
      { provider, model, error },
      requestId,
    );
    this.name = "LLMProviderError";
    this.provider = provider;
    this.model = model;
    Object.setPrototypeOf(this, LLMProviderError.prototype);
  }
}

export class LLMProviderNotFoundError extends LLMError {
  constructor(provider: string, requestId?: string) {
    super(
      `LLM provider not found: ${provider}`,
      "LLM_PROVIDER_NOT_FOUND",
      { provider },
      requestId,
    );
    this.name = "LLMProviderNotFoundError";
    Object.setPrototypeOf(this, LLMProviderNotFoundError.prototype);
  }
}

export class LLMModelNotSupportedError extends LLMError {
  constructor(model: string, provider: string, requestId?: string) {
    super(
      `Model '${model}' not supported by provider '${provider}'`,
      "LLM_MODEL_NOT_SUPPORTED",
      { model, provider },
      requestId,
    );
    this.name = "LLMModelNotSupportedError";
    Object.setPrototypeOf(this, LLMModelNotSupportedError.prototype);
  }
}

export class LLMRateLimitError extends LLMError {
  public readonly retryAfter?: number;

  constructor(provider: string, retryAfter?: number, requestId?: string) {
    super(
      `Rate limit exceeded for provider '${provider}'`,
      "LLM_RATE_LIMITED",
      { provider, retryAfter },
      requestId,
    );
    this.name = "LLMRateLimitError";
    this.retryAfter = retryAfter;
    Object.setPrototypeOf(this, LLMRateLimitError.prototype);
  }
}

export class LLMQuotaExceededError extends LLMError {
  constructor(provider: string, quotaType: string, requestId?: string) {
    super(
      `Quota exceeded for provider '${provider}': ${quotaType}`,
      "LLM_QUOTA_EXCEEDED",
      { provider, quotaType },
      requestId,
    );
    this.name = "LLMQuotaExceededError";
    Object.setPrototypeOf(this, LLMQuotaExceededError.prototype);
  }
}

// ============================================================================
// Agent-Related Errors
// ============================================================================

export class AgentError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "AgentError";
    Object.setPrototypeOf(this, AgentError.prototype);
  }
}

export class AgentNotFoundError extends AgentError {
  constructor(agentId: string, requestId?: string) {
    super(
      `Agent not found: ${agentId}`,
      "AGENT_NOT_FOUND",
      { agentId },
      requestId,
    );
    this.name = "AgentNotFoundError";
    Object.setPrototypeOf(this, AgentNotFoundError.prototype);
  }
}

export class AgentConfigurationError extends AgentError {
  constructor(agentId: string, error: string, requestId?: string) {
    super(
      `Agent configuration error for '${agentId}': ${error}`,
      "AGENT_CONFIGURATION_ERROR",
      { agentId, error },
      requestId,
    );
    this.name = "AgentConfigurationError";
    Object.setPrototypeOf(this, AgentConfigurationError.prototype);
  }
}

// ============================================================================
// Network and API Errors
// ============================================================================

export class NetworkError extends CircuitBreakerError {
  constructor(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ) {
    super(message, code, context, requestId);
    this.name = "NetworkError";
    Object.setPrototypeOf(this, NetworkError.prototype);
  }
}

export class TimeoutError extends NetworkError {
  public readonly timeoutMs: number;

  constructor(operation: string, timeoutMs: number, requestId?: string) {
    super(
      `Operation '${operation}' timed out after ${timeoutMs}ms`,
      "NETWORK_TIMEOUT",
      { operation, timeoutMs },
      requestId,
    );
    this.name = "TimeoutError";
    this.timeoutMs = timeoutMs;
    Object.setPrototypeOf(this, TimeoutError.prototype);
  }
}

export class ConnectionError extends NetworkError {
  constructor(endpoint: string, error: string, requestId?: string) {
    super(
      `Connection failed to '${endpoint}': ${error}`,
      "NETWORK_CONNECTION_ERROR",
      { endpoint, error },
      requestId,
    );
    this.name = "ConnectionError";
    Object.setPrototypeOf(this, ConnectionError.prototype);
  }
}

export class GraphQLError extends NetworkError {
  public readonly query?: string;
  public readonly variables?: any;
  public readonly graphqlErrors: any[];

  constructor(
    graphqlErrors: any[],
    query?: string,
    variables?: any,
    requestId?: string,
  ) {
    const message = `GraphQL error: ${graphqlErrors.map((e) => e.message).join(", ")}`;
    super(
      message,
      "GRAPHQL_ERROR",
      { query, variables, graphqlErrors },
      requestId,
    );
    this.name = "GraphQLError";
    this.query = query;
    this.variables = variables;
    this.graphqlErrors = graphqlErrors;
    Object.setPrototypeOf(this, GraphQLError.prototype);
  }
}

// ============================================================================
// Authentication and Authorization Errors
// ============================================================================

export class AuthenticationError extends CircuitBreakerError {
  constructor(message: string, context?: any, requestId?: string) {
    super(message, "AUTHENTICATION_ERROR", context, requestId);
    this.name = "AuthenticationError";
    Object.setPrototypeOf(this, AuthenticationError.prototype);
  }
}

export class AuthorizationError extends CircuitBreakerError {
  constructor(resource: string, action: string, requestId?: string) {
    super(
      `Not authorized to perform '${action}' on '${resource}'`,
      "AUTHORIZATION_ERROR",
      { resource, action },
      requestId,
    );
    this.name = "AuthorizationError";
    Object.setPrototypeOf(this, AuthorizationError.prototype);
  }
}

// ============================================================================
// Validation Errors
// ============================================================================

export class ValidationError extends CircuitBreakerError {
  public readonly field?: string;
  public readonly value?: any;
  public readonly constraint?: string;

  constructor(
    field: string,
    value: any,
    constraint: string,
    requestId?: string,
  ) {
    super(
      `Validation error for field '${field}': ${constraint}`,
      "VALIDATION_ERROR",
      { field, value, constraint },
      requestId,
    );
    this.name = "ValidationError";
    this.field = field;
    this.value = value;
    this.constraint = constraint;
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

export class SchemaValidationError extends ValidationError {
  public readonly schema: any;
  public readonly validationErrors: any[];

  constructor(schema: any, validationErrors: any[], requestId?: string) {
    const message = `Schema validation failed: ${validationErrors.map((e) => e.message || e).join(", ")}`;
    super("schema", null, message, requestId);
    this.name = "SchemaValidationError";
    this.schema = schema;
    this.validationErrors = validationErrors;
    Object.setPrototypeOf(this, SchemaValidationError.prototype);
  }
}

// ============================================================================
// Error Utilities
// ============================================================================

/**
 * Error factory for creating typed errors
 */
export class ErrorFactory {
  static workflow(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): WorkflowError {
    return new WorkflowError(message, code, context, requestId);
  }

  static resource(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): ResourceError {
    return new ResourceError(message, code, context, requestId);
  }

  static rule(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): RuleError {
    return new RuleError(message, code, context, requestId);
  }

  static function(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): FunctionError {
    return new FunctionError(message, code, context, requestId);
  }

  static llm(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): LLMError {
    return new LLMError(message, code, context, requestId);
  }

  static agent(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): AgentError {
    return new AgentError(message, code, context, requestId);
  }

  static network(
    message: string,
    code: string,
    context?: any,
    requestId?: string,
  ): NetworkError {
    return new NetworkError(message, code, context, requestId);
  }

  static validation(
    field: string,
    value: any,
    constraint: string,
    requestId?: string,
  ): ValidationError {
    return new ValidationError(field, value, constraint, requestId);
  }
}

/**
 * Error handler utility class
 */
export class ErrorHandler {
  /**
   * Handle and categorize unknown errors
   */
  static handle(error: unknown, requestId?: string): CircuitBreakerError {
    if (error instanceof CircuitBreakerError) {
      return error;
    }

    if (error instanceof Error) {
      return new CircuitBreakerError(
        error.message,
        "UNKNOWN_ERROR",
        { originalError: error.name, stack: error.stack },
        requestId,
      );
    }

    return new CircuitBreakerError(
      `Unknown error: ${String(error)}`,
      "UNKNOWN_ERROR",
      { originalError: error },
      requestId,
    );
  }

  /**
   * Check if error should be retried
   */
  static shouldRetry(
    error: CircuitBreakerError,
    attempt: number,
    maxAttempts: number,
  ): boolean {
    if (attempt >= maxAttempts) return false;
    return error.isRetryable();
  }

  /**
   * Get retry delay based on error type and attempt number
   */
  static getRetryDelay(error: CircuitBreakerError, attempt: number): number {
    const baseDelay = 1000; // 1 second
    const maxDelay = 30000; // 30 seconds

    if (error instanceof LLMRateLimitError && error.retryAfter) {
      return error.retryAfter * 1000;
    }

    // Exponential backoff with jitter
    const delay = Math.min(baseDelay * Math.pow(2, attempt), maxDelay);
    const jitter = Math.random() * 0.1 * delay;
    return delay + jitter;
  }

  /**
   * Log error with appropriate level
   */
  static log(
    error: CircuitBreakerError,
    logger?: (level: string, message: string, meta?: any) => void,
  ): void {
    const level = error.getSeverity();
    const message = `${error.name}: ${error.message}`;
    const meta = error.toJSON();

    if (logger) {
      logger(level, message, meta);
    } else {
      // Default console logging
      switch (level) {
        case "critical":
          console.error(`[CRITICAL] ${message}`, meta);
          break;
        case "high":
          console.error(`[ERROR] ${message}`, meta);
          break;
        case "medium":
          console.warn(`[WARN] ${message}`, meta);
          break;
        case "low":
          console.info(`[INFO] ${message}`, meta);
          break;
      }
    }
  }
}

/**
 * Type guard utilities
 */
export const isCircuitBreakerError = (
  error: unknown,
): error is CircuitBreakerError => {
  return error instanceof CircuitBreakerError;
};

export const isWorkflowError = (error: unknown): error is WorkflowError => {
  return error instanceof WorkflowError;
};

export const isResourceError = (error: unknown): error is ResourceError => {
  return error instanceof ResourceError;
};

export const isRuleError = (error: unknown): error is RuleError => {
  return error instanceof RuleError;
};

export const isFunctionError = (error: unknown): error is FunctionError => {
  return error instanceof FunctionError;
};

export const isLLMError = (error: unknown): error is LLMError => {
  return error instanceof LLMError;
};

export const isAgentError = (error: unknown): error is AgentError => {
  return error instanceof AgentError;
};

export const isNetworkError = (error: unknown): error is NetworkError => {
  return error instanceof NetworkError;
};

export const isValidationError = (error: unknown): error is ValidationError => {
  return error instanceof ValidationError;
};
