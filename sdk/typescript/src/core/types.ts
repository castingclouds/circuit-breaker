/**
 * Core type definitions for Circuit Breaker TypeScript SDK
 *
 * This file contains all the fundamental types used throughout the SDK,
 * including workflow definitions, resources, activities, and configuration.
 */

// ============================================================================
// SDK Configuration Types
// ============================================================================

export interface SDKConfig {
  /** GraphQL endpoint for Circuit Breaker server */
  graphqlEndpoint: string;

  /** Optional LLM router configuration */
  llmConfig?: LLMConfig;

  /** Optional function system configuration */
  functionConfig?: FunctionConfig;

  /** Optional rules engine configuration */
  rulesConfig?: RulesConfig;

  /** Optional logging configuration */
  logging?: LoggingConfig;

  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;

  /** Additional headers for GraphQL requests */
  headers?: Record<string, string>;

  /** Enable request/response logging */
  debug?: boolean;
}

export interface LoggingConfig {
  /** Log level: 'debug', 'info', 'warn', 'error' */
  level: "debug" | "info" | "warn" | "error";

  /** Enable structured logging */
  structured?: boolean;

  /** Custom logger function */
  logger?: (level: string, message: string, meta?: any) => void;
}

// ============================================================================
// Workflow Types
// ============================================================================

export interface WorkflowDefinition {
  /** Unique workflow name */
  name: string;

  /** List of possible states */
  states: string[];

  /** List of activities (transitions between states) */
  activities: ActivityDefinition[];

  /** Initial state when resources are created */
  initialState: string;

  /** Optional workflow metadata */
  metadata?: Record<string, any>;

  /** Optional workflow description */
  description?: string;

  /** Optional workflow version */
  version?: string;

  /** Optional workflow tags */
  tags?: string[];
}

export interface ActivityDefinition {
  /** Unique activity identifier */
  id: string;

  /** Optional human-readable activity name */
  name?: string;

  /** States from which this activity can be triggered */
  fromStates: string[];

  /** Target state after activity completion */
  toState: string;

  /** Legacy string-based conditions (for backward compatibility) */
  conditions: string[];

  /** New structured rules for state transitions */
  rules?: Rule[];

  /** Function triggers associated with this activity */
  functions?: FunctionTrigger[];

  /** Whether all rules must pass (true) or any rule (false) */
  requiresAllRules?: boolean;

  /** Optional activity description */
  description?: string;

  /** Optional activity metadata */
  metadata?: Record<string, any>;
}

// ============================================================================
// Resource Types
// ============================================================================

export interface Resource {
  /** Unique resource identifier */
  id: string;

  /** Workflow this resource belongs to */
  workflowId: string;

  /** Current state of the resource */
  state: string;

  /** Resource data payload */
  data: any;

  /** Resource metadata */
  metadata: Record<string, any>;

  /** State transition history */
  history: HistoryEvent[];

  /** Resource creation timestamp */
  createdAt: string;

  /** Resource last update timestamp */
  updatedAt: string;
}

export interface HistoryEvent {
  /** Event timestamp */
  timestamp: string;

  /** Activity that caused the transition */
  activity: string;

  /** Previous state */
  fromState: string;

  /** New state */
  toState: string;

  /** Optional data associated with the transition */
  data?: any;

  /** Optional metadata for the event */
  metadata?: Record<string, any>;
}

export interface ResourceCreateInput {
  /** Workflow ID for the new resource */
  workflowId: string;

  /** Initial state (defaults to workflow's initialState) */
  initialState?: string;

  /** Initial resource data */
  data: any;

  /** Initial resource metadata */
  metadata?: Record<string, any>;
}

export interface ActivityExecuteInput {
  /** Resource ID to execute activity on */
  resourceId: string;

  /** Activity ID to execute */
  activityId: string;

  /** Optional data to include with the transition */
  data?: any;

  /** Optional metadata for the transition */
  metadata?: Record<string, any>;
}

// ============================================================================
// Rules Engine Types
// ============================================================================

export interface RulesConfig {
  /** Enable evaluation result caching */
  enableCache?: boolean;

  /** Maximum cache size */
  cacheSize?: number;

  /** Predefined custom rules */
  customRules?: Record<string, Rule>;

  /** Rule evaluation timeout in milliseconds */
  evaluationTimeout?: number;

  /** Enable strict mode (fail on unknown fields) */
  strictMode?: boolean;
}

export interface Rule {
  /** Unique rule name */
  name: string;

  /** Rule type */
  type: RuleType;

  /** Rule condition (for simple/javascript rules) */
  condition?: string;

  /** Custom evaluator function */
  evaluator?: RuleEvaluator;

  /** Human-readable description */
  description?: string;

  /** Rule category for organization */
  category?: string;

  /** Rule metadata */
  metadata?: Record<string, any>;

  /** Rule priority (higher executes first) */
  priority?: number;

  /** Whether rule is enabled */
  enabled?: boolean;
}

export type RuleType = "simple" | "composite" | "custom" | "javascript";

export interface CompositeRule extends Rule {
  type: "composite";

  /** Logical operator */
  operator: "AND" | "OR" | "NOT";

  /** Child rules */
  rules: Rule[];
}

export interface RuleContext {
  /** Current resource */
  resource: Resource;

  /** Current workflow definition */
  workflow?: WorkflowDefinition;

  /** Current activity being evaluated */
  activity: ActivityDefinition;

  /** Additional context metadata */
  metadata?: Record<string, any>;

  /** Request timestamp */
  timestamp: Date;
}

export interface RuleEvaluationResult {
  /** Whether all rules passed */
  passed: boolean;

  /** Individual rule results */
  results: RuleResult[];

  /** Evaluation errors */
  errors: string[];

  /** Total evaluation time in milliseconds */
  evaluationTime: number;

  /** Number of rules evaluated */
  rulesEvaluated: number;

  /** Number of rules that passed */
  rulesPassed: number;
}

export interface RuleResult {
  /** Rule that was evaluated */
  rule: Rule;

  /** Whether the rule passed */
  passed: boolean;

  /** Error message if rule failed */
  error?: string;

  /** Additional context from evaluation */
  context?: any;

  /** Rule execution time */
  executionTime?: number;
}

export interface RuleValidationResult {
  /** Whether validation passed */
  valid: boolean;

  /** Validation errors */
  errors: string[];

  /** Validation warnings */
  warnings: string[];

  /** Validated rule count */
  ruleCount: number;
}

export type RuleEvaluator = (
  context: RuleContext,
) => Promise<boolean> | boolean;

// ============================================================================
// Function System Types
// ============================================================================

export interface FunctionConfig {
  /** Docker configuration */
  dockerConfig?: DockerConfig;

  /** Function execution timeout */
  executionTimeout?: number;

  /** Maximum concurrent function executions */
  maxConcurrency?: number;

  /** Function registry endpoint */
  registryEndpoint?: string;

  /** Enable function caching */
  enableCaching?: boolean;
}

export interface DockerConfig {
  /** Docker host URL */
  host?: string;

  /** Docker socket path */
  socketPath?: string;

  /** Docker registry credentials */
  registry?: {
    username: string;
    password: string;
    serveraddress?: string;
  };

  /** Default resource limits */
  defaultLimits?: ResourceLimits;
}

export interface FunctionDefinition {
  /** Unique function identifier */
  id: string;

  /** Human-readable function name */
  name: string;

  /** Container configuration */
  container: ContainerConfig;

  /** Event triggers */
  triggers: EventTrigger[];

  /** Function chains */
  chains: FunctionChain[];

  /** Input JSON schema */
  inputSchema?: JSONSchema;

  /** Output JSON schema */
  outputSchema?: JSONSchema;

  /** Function description */
  description?: string;

  /** Function tags */
  tags?: string[];

  /** Function metadata */
  metadata?: Record<string, any>;

  /** Function version */
  version?: string;

  /** Whether function is enabled */
  enabled?: boolean;
}

export interface ContainerConfig {
  /** Docker image name */
  image: string;

  /** Container command */
  command?: string[];

  /** Environment variables */
  environment?: Record<string, string>;

  /** Volume mounts */
  mounts?: ContainerMount[];

  /** Resource limits */
  resources?: ResourceLimits;

  /** Working directory */
  workingDir?: string;

  /** Network mode */
  networkMode?: string;

  /** Container labels */
  labels?: Record<string, string>;
}

export interface ContainerMount {
  /** Host path */
  source: string;

  /** Container path */
  target: string;

  /** Mount type */
  type: "bind" | "volume" | "tmpfs";

  /** Mount options */
  options?: string[];

  /** Read-only mount */
  readonly?: boolean;
}

export interface ResourceLimits {
  /** CPU limit (number of cores) */
  cpu?: number;

  /** Memory limit in bytes */
  memory?: number;

  /** GPU count */
  gpu?: number;

  /** Disk space limit in bytes */
  disk?: number;

  /** Network bandwidth limit */
  network?: number;
}

export interface EventTrigger {
  /** Trigger type */
  type: EventTriggerType;

  /** Trigger condition */
  condition: string;

  /** Input mapping strategy */
  inputMapping?: InputMapping;

  /** Trigger filters */
  filters?: Record<string, any>;

  /** Trigger description */
  description?: string;

  /** Whether trigger is enabled */
  enabled?: boolean;
}

export type EventTriggerType =
  | "workflow_event"
  | "resource_state"
  | "function_completion"
  | "schedule"
  | "webhook";

export type InputMapping =
  | "full_data"
  | "metadata_only"
  | "custom"
  | { fields: Record<string, string> };

export interface FunctionTrigger {
  /** Function ID to trigger */
  functionId: string;

  /** Trigger condition */
  condition?: string;

  /** Input mapping */
  inputMapping?: InputMapping;

  /** Execution delay */
  delay?: number;

  /** Retry configuration */
  retry?: RetryConfig;
}

export interface RetryConfig {
  /** Maximum retry attempts */
  maxAttempts: number;

  /** Initial delay between retries (ms) */
  initialDelay: number;

  /** Backoff multiplier */
  backoffMultiplier?: number;

  /** Maximum delay between retries (ms) */
  maxDelay?: number;

  /** Retry on specific error types */
  retryOn?: string[];
}

export interface FunctionChain {
  /** Target function ID */
  targetFunction: string;

  /** Chain condition */
  condition: ChainCondition;

  /** Input mapping for chained function */
  inputMapping: InputMapping;

  /** Execution delay */
  delay?: number;

  /** Chain description */
  description?: string;
}

export type ChainCondition = "always" | "success" | "failure" | "custom";

export interface FunctionResult {
  /** Function execution ID */
  executionId: string;

  /** Function ID */
  functionId: string;

  /** Execution status */
  status: ExecutionStatus;

  /** Function output */
  output: any;

  /** Execution logs */
  logs: string[];

  /** Execution duration in milliseconds */
  executionTime: number;

  /** Start timestamp */
  startedAt: string;

  /** Completion timestamp */
  completedAt?: string;

  /** Error information */
  error?: string;

  /** Resource usage */
  resourceUsage?: ResourceUsage;
}

export type ExecutionStatus =
  | "pending"
  | "running"
  | "success"
  | "failure"
  | "timeout"
  | "cancelled";

export interface ResourceUsage {
  /** CPU usage percentage */
  cpu: number;

  /** Memory usage in bytes */
  memory: number;

  /** Network I/O in bytes */
  network: {
    bytesIn: number;
    bytesOut: number;
  };

  /** Disk I/O in bytes */
  disk: {
    bytesRead: number;
    bytesWritten: number;
  };
}

// ============================================================================
// LLM Types
// ============================================================================

export interface LLMConfig {
  /** Available LLM providers */
  providers: LLMProviderConfig[];

  /** Default provider name */
  defaultProvider: string;

  /** Load balancing configuration */
  loadBalancing?: LoadBalancingConfig;

  /** Failover configuration */
  failover?: FailoverConfig;

  /** Rate limiting configuration */
  rateLimiting?: RateLimitConfig;

  /** Usage tracking configuration */
  usageTracking?: UsageTrackingConfig;
}

export interface LLMProviderConfig {
  /** Provider name */
  name: string;

  /** Provider type */
  type: LLMProviderType;

  /** API endpoint */
  endpoint: string;

  /** API key */
  apiKey?: string;

  /** Provider-specific configuration */
  config?: Record<string, any>;

  /** Supported models */
  models?: string[];

  /** Provider weight for load balancing */
  weight?: number;

  /** Whether provider is enabled */
  enabled?: boolean;

  /** Provider priority for routing */
  priority?: number;

  /** Request timeout in milliseconds */
  timeout?: number;

  /** Maximum retry attempts */
  maxRetries?: number;

  /** Rate limiting configuration */
  rateLimit?: {
    requestsPerMinute?: number;
    tokensPerMinute?: number;
    concurrent?: number;
  };
}

export type LLMProviderType =
  | "openai"
  | "anthropic"
  | "ollama"
  | "huggingface"
  | "cohere"
  | "custom";

export interface LoadBalancingConfig {
  /** Load balancing strategy */
  strategy: "round_robin" | "weighted" | "least_connections" | "random";

  /** Health check interval in seconds */
  healthCheckInterval?: number;

  /** Request timeout in milliseconds */
  timeout?: number;
}

export interface FailoverConfig {
  /** Enable automatic failover */
  enabled: boolean;

  /** Maximum failover attempts */
  maxAttempts?: number;

  /** Maximum retries */
  maxRetries?: number;

  /** Backoff strategy */
  backoffStrategy?: "exponential" | "linear" | "fixed";

  /** Base delay in milliseconds */
  baseDelay?: number;

  /** Failover timeout in milliseconds */
  timeout?: number;

  /** Circuit breaker configuration */
  circuitBreaker?: CircuitBreakerConfig;
}

export interface CircuitBreakerConfig {
  /** Failure threshold */
  failureThreshold: number;

  /** Recovery timeout in milliseconds */
  recoveryTimeout: number;

  /** Monitoring window in milliseconds */
  monitoringWindow: number;
}

export interface RateLimitConfig {
  /** Requests per minute */
  requestsPerMinute: number;

  /** Tokens per minute */
  tokensPerMinute?: number;

  /** Rate limit strategy */
  strategy: "sliding_window" | "fixed_window" | "token_bucket";
}

export interface UsageTrackingConfig {
  /** Enable usage tracking */
  enabled: boolean;

  /** Tracking granularity */
  granularity: "request" | "token" | "both";

  /** Storage backend for usage data */
  storage?: "memory" | "redis" | "database";
}

export interface ChatCompletionRequest {
  /** Model identifier */
  model: string;

  /** Conversation messages */
  messages: ChatMessage[];

  /** Response randomness (0-2) */
  temperature?: number;

  /** Maximum tokens to generate */
  max_tokens?: number;

  /** Enable streaming response */
  stream?: boolean;

  /** Stop sequences */
  stop?: string[];

  /** Presence penalty (-2 to 2) */
  presence_penalty?: number;

  /** Frequency penalty (-2 to 2) */
  frequency_penalty?: number;

  /** Top-p nucleus sampling */
  top_p?: number;

  /** Number of completions to generate */
  n?: number;

  /** Function/tool definitions */
  tools?: Tool[];

  /** Function calling mode */
  tool_choice?:
    | "none"
    | "auto"
    | { type: "function"; function: { name: string } };

  /** User identifier for tracking */
  user?: string;

  /** Additional provider-specific parameters */
  extra?: Record<string, any>;
}

export interface ChatMessage {
  /** Message role */
  role: ChatRole;

  /** Message content */
  content: string;

  /** Optional message name */
  name?: string;

  /** Function/tool calls (for assistant messages) */
  tool_calls?: ToolCall[];

  /** Function call result (for function messages) */
  tool_call_id?: string;
}

export type ChatRole = "system" | "user" | "assistant" | "tool" | "function";

export interface ToolCall {
  /** Tool call ID */
  id: string;

  /** Tool type */
  type: "function";

  /** Function call details */
  function: {
    name: string;
    arguments: string;
  };
}

export interface Tool {
  /** Tool type */
  type: "function";

  /** Function definition */
  function: {
    name: string;
    description?: string;
    parameters?: JSONSchema;
  };
}

export interface ChatCompletionResponse {
  /** Response ID */
  id: string;

  /** Object type */
  object: string;

  /** Creation timestamp */
  created: number;

  /** Model used */
  model: string;

  /** Response choices */
  choices: Choice[];

  /** Token usage statistics */
  usage?: Usage;

  /** System fingerprint */
  system_fingerprint?: string;
}

export interface Choice {
  /** Choice index */
  index: number;

  /** Generated message */
  message: ChatMessage;

  /** Finish reason */
  finish_reason: "stop" | "length" | "tool_calls" | "content_filter" | null;

  /** Log probabilities */
  logprobs?: any;
}

export interface Usage {
  /** Prompt tokens */
  prompt_tokens: number;

  /** Completion tokens */
  completion_tokens: number;

  /** Total tokens */
  total_tokens: number;
}

export interface ChatCompletionChunk {
  /** Chunk ID */
  id: string;

  /** Object type */
  object: string;

  /** Creation timestamp */
  created: number;

  /** Model used */
  model: string;

  /** Delta choices */
  choices: ChoiceDelta[];

  /** System fingerprint */
  system_fingerprint?: string;
}

export interface ChoiceDelta {
  /** Choice index */
  index: number;

  /** Message delta */
  delta: MessageDelta;

  /** Finish reason */
  finish_reason?: "stop" | "length" | "tool_calls" | "content_filter" | null;
}

export interface MessageDelta {
  /** Role (only in first chunk) */
  role?: ChatRole;

  /** Content delta */
  content?: string;

  /** Tool calls delta */
  tool_calls?: ToolCallDelta[];
}

export interface ToolCallDelta {
  /** Tool call index */
  index: number;

  /** Tool call ID */
  id?: string;

  /** Tool type */
  type?: "function";

  /** Function call delta */
  function?: {
    name?: string;
    arguments?: string;
  };
}

// ============================================================================
// Agent Types
// ============================================================================

export interface AgentDefinition {
  /** Agent name */
  name: string;

  /** Agent type */
  type: AgentType;

  /** Agent configuration */
  config: AgentConfig;

  /** Agent description */
  description?: string;

  /** Agent version */
  version?: string;

  /** Agent tags */
  tags?: string[];

  /** Agent metadata */
  metadata?: Record<string, any>;
}

export type AgentType =
  | "conversational"
  | "state_machine"
  | "workflow_integrated";

export interface AgentConfig {
  /** LLM provider to use */
  llmProvider?: string;

  /** System prompt */
  systemPrompt?: string;

  /** Memory configuration */
  memory?: MemoryConfig;

  /** Tool/function access */
  tools?: Tool[];

  /** State machine configuration (for state machine agents) */
  stateMachine?: StateMachineConfig;

  /** Workflow integrations */
  workflowIntegrations?: string[];

  /** Agent-specific settings */
  settings?: Record<string, any>;
}

export interface MemoryConfig {
  /** Enable memory */
  enabled: boolean;

  /** Memory type */
  type: "short_term" | "long_term" | "both";

  /** Memory size limit */
  maxSize?: number;

  /** Memory persistence */
  persistent?: boolean;

  /** Memory backend */
  backend?: "memory" | "redis" | "database";
}

export interface StateMachineConfig {
  /** Agent states */
  states: AgentState[];

  /** State transitions */
  transitions: StateTransition[];

  /** Initial state */
  initialState: string;
}

export interface AgentState {
  /** State name */
  name: string;

  /** State prompt */
  prompt: string;

  /** State actions */
  actions: StateAction[];

  /** State metadata */
  metadata?: Record<string, any>;
}

export interface StateTransition {
  /** Source state */
  fromState: string;

  /** Target state */
  toState: string;

  /** Transition condition */
  condition: string;

  /** Transition actions */
  actions?: StateAction[];
}

export interface StateAction {
  /** Action type */
  type: "function_call" | "workflow_trigger" | "state_change" | "custom";

  /** Action configuration */
  config: Record<string, any>;

  /** Action condition */
  condition?: string;
}

// ============================================================================
// Utility Types
// ============================================================================

export interface JSONSchema {
  type?: string;
  properties?: Record<string, JSONSchema>;
  required?: string[];
  items?: JSONSchema;
  additionalProperties?: boolean | JSONSchema;
  enum?: any[];
  const?: any;
  default?: any;
  description?: string;
  examples?: any[];
  format?: string;
  pattern?: string;
  minimum?: number;
  maximum?: number;
  minLength?: number;
  maxLength?: number;
  minItems?: number;
  maxItems?: number;
  uniqueItems?: boolean;
  multipleOf?: number;
  exclusiveMinimum?: number;
  exclusiveMaximum?: number;
  minProperties?: number;
  maxProperties?: number;
  allOf?: JSONSchema[];
  anyOf?: JSONSchema[];
  oneOf?: JSONSchema[];
  not?: JSONSchema;
  if?: JSONSchema;
  then?: JSONSchema;
  else?: JSONSchema;
  [key: string]: any;
}

export interface PaginationOptions {
  /** Page offset */
  offset?: number;

  /** Page size */
  limit?: number;

  /** Sort field */
  sortBy?: string;

  /** Sort direction */
  sortOrder?: "asc" | "desc";
}

export interface PaginatedResult<T> {
  /** Result data */
  data: T[];

  /** Total count */
  total: number;

  /** Current offset */
  offset: number;

  /** Page size */
  limit: number;

  /** Whether there are more results */
  hasMore: boolean;
}

export interface FilterOptions {
  /** Field filters */
  filters?: Record<string, any>;

  /** Search query */
  search?: string;

  /** Date range filter */
  dateRange?: {
    from?: Date;
    to?: Date;
  };

  /** Include archived/deleted items */
  includeArchived?: boolean;
}

// ============================================================================
// Type Utilities
// ============================================================================

/** Make all properties optional recursively */
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

/** Make specific properties required */
export type RequireFields<T, K extends keyof T> = T & Required<Pick<T, K>>;

/** Omit properties and make the rest optional */
export type PartialExcept<T, K extends keyof T> = Partial<T> & Pick<T, K>;

/** Extract the type of array elements */
export type ArrayElement<T> = T extends (infer U)[] ? U : never;

/** Extract the resolved type of a Promise */
export type Awaited<T> = T extends Promise<infer U> ? U : T;
