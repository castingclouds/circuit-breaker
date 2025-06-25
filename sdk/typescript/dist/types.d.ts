/**
 * Core types for the Circuit Breaker TypeScript SDK
 */
export interface ClientConfig {
    baseUrl: string;
    apiKey?: string;
    timeout?: number;
    headers?: Record<string, string>;
}
export interface PingResponse {
    status: string;
    version: string;
    uptime_seconds: number;
}
export interface ServerInfo {
    name: string;
    version: string;
    features: string[];
    providers: string[];
}
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
    name: string;
    type: string;
    config: Record<string, any>;
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
export type ExecutionStatus = "pending" | "running" | "success" | "failure" | "timeout" | "cancelled";
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
export type AgentType = "conversational" | "state_machine" | "workflow_integrated";
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
export declare class CircuitBreakerError extends Error {
    code: string;
    details?: Record<string, any> | undefined;
    constructor(message: string, code: string, details?: Record<string, any> | undefined);
}
export declare class NetworkError extends CircuitBreakerError {
    constructor(message: string, details?: Record<string, any>);
}
export declare class ValidationError extends CircuitBreakerError {
    constructor(message: string, details?: Record<string, any>);
}
export declare class NotFoundError extends CircuitBreakerError {
    constructor(resource: string, details?: Record<string, any>);
}
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
export type Result<T, E = Error> = {
    success: true;
    data: T;
} | {
    success: false;
    error: E;
};
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
//# sourceMappingURL=types.d.ts.map