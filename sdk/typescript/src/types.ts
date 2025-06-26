/**
 * Core types for the Circuit Breaker TypeScript SDK
 */

// ============================================================================
// Client Configuration
// ============================================================================

export interface ClientConfig {
  baseUrl: string;
  apiKey?: string;
  timeout?: number;
  headers?: Record<string, string>;
}

// ============================================================================
// Common Types
// ============================================================================

export interface PingResponse {
  status: string;
  version: string;
  uptime_seconds: number;
  endpoints?: {
    graphql: boolean;
    rest: boolean;
    graphqlUrl: string;
    restUrl: string;
  };
  graphql?: boolean;
  rest?: boolean;
  models?: number;
}

export interface ServerInfo {
  name: string;
  version: string;
  features: string[];
  providers: string[];
  endpoints?: {
    graphql: boolean;
    rest: boolean;
  };
}

// ============================================================================
// Workflow Types
// ============================================================================

export interface Workflow {
  id: string;
  name: string;
  states: string[];
  initialState?: string;
  activities?: ActivityDefinition[];
  createdAt?: string;
  updatedAt?: string;
}

export interface WorkflowDefinition {
  states: WorkflowState[];
  transitions: WorkflowTransition[];
  initial_state: string;
}

export interface ActivityDefinition {
  id: string;
  name?: string;
  fromStates: string[];
  toState: string;
  conditions: string[];
  description?: string;
}

export interface WorkflowState {
  name: string;
  type: "normal" | "final";
  actions?: WorkflowAction[];
}

export interface WorkflowTransition {
  from: string;
  to: string;
  event: string;
  conditions?: WorkflowCondition[];
}

export interface WorkflowAction {
  type: string;
  config: Record<string, any>;
}

export interface WorkflowCondition {
  type: string;
  config: Record<string, any>;
}

export interface WorkflowExecution {
  id: string;
  workflow_id: string;
  status: ExecutionStatus;
  current_state: string;
  input: Record<string, any>;
  output?: Record<string, any>;
  error?: string;
  created_at: string;
  updated_at: string;
}

export type ExecutionStatus =
  | "pending"
  | "running"
  | "success"
  | "failure"
  | "timeout"
  | "cancelled";

// ============================================================================
// Agent Types
// ============================================================================

export interface Agent {
  id: string;
  name: string;
  description?: string;
  llmProvider?: LLMProvider;
  llmConfig?: LLMConfig;
  prompts?: AgentPrompts;
  capabilities?: string[];
  tools?: Tool[];
  createdAt?: string;
  updatedAt?: string;
}

export interface LLMProvider {
  name: string;
  healthStatus: {
    isHealthy: boolean;
    lastCheck?: string;
    errorRate?: number;
    averageLatencyMs?: number;
    consecutiveFailures?: number;
    lastError?: string;
  };
}

export interface LLMConfig {
  model?: string;
  temperature?: number;
  maxTokens?: number;
}

export interface AgentPrompts {
  system?: string;
  user?: string;
}

export type AgentType =
  | "conversational"
  | "state_machine"
  | "workflow_integrated";

export interface AgentConfig {
  llm_provider?: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  system_prompt?: string;
  tools?: Tool[];
  memory?: MemoryConfig;
}

export interface MemoryConfig {
  type: "short_term" | "long_term" | "persistent";
  max_entries?: number;
  ttl?: number;
}

export interface Tool {
  name: string;
  description: string;
  parameters: Record<string, any>;
}

// ============================================================================
// Function Types
// ============================================================================

export interface Function {
  id: string;
  name: string;
  description?: string;
  runtime: string;
  code: string;
  entrypoint?: string;
  config?: FunctionConfig;
  createdAt?: string;
  updatedAt?: string;
}

export interface FunctionConfig {
  timeout?: number;
  memory?: number;
  environment?: Record<string, string>;
  docker?: DockerConfig;
}

export interface DockerConfig {
  image: string;
  command?: string[];
  environment?: Record<string, string>;
  mounts?: ContainerMount[];
  resource_limits?: ResourceLimits;
}

export interface ContainerMount {
  source: string;
  target: string;
  readonly?: boolean;
}

export interface ResourceLimits {
  memory?: string;
  cpu?: string;
}

export interface FunctionExecution {
  id: string;
  function_id: string;
  status: ExecutionStatus;
  input: Record<string, any>;
  output?: Record<string, any>;
  error?: string;
  duration?: number;
  created_at: string;
  completed_at?: string;
}

// ============================================================================
// Resource Types
// ============================================================================

export interface Resource {
  id: string;
  workflowId: string;
  state: string;
  data: Record<string, any>;
  createdAt?: string;
  updatedAt?: string;
}

export interface ResourceCreateInput {
  workflow_id: string;
  data: Record<string, any>;
  initial_state?: string;
}

export interface ResourceUpdateInput {
  data?: Record<string, any>;
  state?: string;
}

// ============================================================================
// Rule Types
// ============================================================================

export interface Rule {
  id: string;
  name: string;
  description?: string;
  type: RuleType;
  definition: RuleDefinition;
  created_at: string;
  updated_at: string;
}

export type RuleType = "simple" | "composite" | "javascript" | "custom";

export interface RuleDefinition {
  conditions: RuleCondition[];
  actions: RuleAction[];
  combinator?: "and" | "or";
}

export interface RuleCondition {
  field: string;
  operator: string;
  value: any;
}

export interface RuleAction {
  type: string;
  config: Record<string, any>;
}

export interface RuleEvaluationResult {
  rule_id: string;
  matched: boolean;
  actions_executed: string[];
  context: Record<string, any>;
}

// ============================================================================
// LLM Types
// ============================================================================

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  top_p?: number;
  frequency_penalty?: number;
  presence_penalty?: number;
  stop?: string[];
  user?: string;
  functions?: ChatFunction[];
  function_call?: any;
  circuit_breaker?: CircuitBreakerOptions;
}

export interface ChatCompletionResponse {
  id: string;
  choices: Choice[];
  usage: Usage;
  model: string;
  created: number;
}

export interface Choice {
  index: number;
  message: ChatMessage;
  finish_reason: string;
}

export interface Usage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface SmartCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  circuit_breaker?: CircuitBreakerOptions;
}

export interface CircuitBreakerOptions {
  routing_strategy?: RoutingStrategy;
  max_cost_per_1k_tokens?: number;
  max_latency_ms?: number;
  fallback_models?: string[];
  task_type?: TaskType;
  require_streaming?: boolean;
  budget_constraint?: BudgetConstraint;
}

export type RoutingStrategy =
  | "cost_optimized"
  | "performance_first"
  | "load_balanced"
  | "failover_chain"
  | "model_specific";

export type TaskType =
  | "general_chat"
  | "coding"
  | "analysis"
  | "creative"
  | "reasoning";

export interface BudgetConstraint {
  daily_limit?: number;
  monthly_limit?: number;
  per_request_limit?: number;
}

export interface ChatFunction {
  name: string;
  description?: string;
  parameters: any;
}

export interface ModelInfo {
  id: string;
  object: string;
  created?: number;
  owned_by: string;
  provider?: string;
  context_window?: number;
  supports_streaming?: boolean;
  cost_per_input_token?: number;
  cost_per_output_token?: number;
}

export interface ModelsResponse {
  object: string;
  data: ModelInfo[];
}

export interface EmbeddingResponse {
  object: string;
  data: EmbeddingData[];
  model: string;
  usage: EmbeddingUsage;
}

export interface EmbeddingData {
  object: string;
  embedding: number[];
  index: number;
}

export interface EmbeddingUsage {
  prompt_tokens: number;
  total_tokens: number;
}

// ============================================================================
// Error Types
// ============================================================================

export class CircuitBreakerError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: Record<string, any>,
  ) {
    super(message);
    this.name = "CircuitBreakerError";
  }
}

export class NetworkError extends CircuitBreakerError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, "NETWORK_ERROR", details);
    this.name = "NetworkError";
  }
}

export class ValidationError extends CircuitBreakerError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, "VALIDATION_ERROR", details);
    this.name = "ValidationError";
  }
}

export class NotFoundError extends CircuitBreakerError {
  constructor(resource: string, details?: Record<string, any>) {
    super(`${resource} not found`, "NOT_FOUND", details);
    this.name = "NotFoundError";
  }
}

// ============================================================================
// Utility Types
// ============================================================================

export interface PaginationOptions {
  page?: number;
  limit?: number;
}

export interface PaginatedResult<T> {
  data: T[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    pages: number;
  };
}

export type Result<T, E = Error> =
  | { success: true; data: T }
  | { success: false; error: E };

// ============================================================================
// Builder Input Types
// ============================================================================

export interface WorkflowCreateInput {
  name: string;
  description?: string;
  definition: WorkflowDefinition;
}

export interface AgentCreateInput {
  name: string;
  description?: string;
  type: AgentType;
  config: AgentConfig;
}

export interface FunctionCreateInput {
  name: string;
  description?: string;
  runtime: string;
  code: string;
  config?: FunctionConfig;
}

export interface RuleCreateInput {
  name: string;
  description?: string;
  type: RuleType;
  definition: RuleDefinition;
}
