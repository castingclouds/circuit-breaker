/**
 * Circuit Breaker TypeScript SDK
 *
 * A simple, clean TypeScript client for the Circuit Breaker workflow engine.
 * Mirrors the Rust SDK approach with minimal abstractions and direct API access.
 */
export { Client, ClientBuilder } from "./client.js";
export type { ClientBuilderConfig } from "./client.js";
export type { ClientConfig } from "./types.js";
export { WorkflowClient, WorkflowBuilder, createWorkflow, } from "./workflows.js";
export { AgentClient, AgentBuilder, createAgent } from "./agents.js";
export { FunctionClient, FunctionBuilder, createFunction, } from "./functions.js";
export { ResourceClient, ResourceBuilder, createResource, } from "./resources.js";
export { RuleClient, RuleBuilder, LegacyRuleBuilder, ClientRuleEngine, createRule, createLegacyRule, createRuleEngine, evaluateRule, CommonRules, } from "./rules.js";
export { LLMClient, ChatBuilder, Conversation, createChat, createConversation, quickChat, COMMON_MODELS, } from "./llm.js";
export { AnalyticsClient, BudgetStatusBuilder, CostAnalyticsBuilder, SetBudgetBuilder, costAnalytics, budgetStatus, setBudget, getUserBudgetStatus, getProjectBudgetStatus, getUserMonthlyCostAnalytics, setUserMonthlyBudget, } from "./analytics.js";
export { MCPClient, MCPServersBuilder, CreateMCPServerBuilder, UpdateMCPServerBuilder, ConfigureOAuthBuilder, ConfigureJWTBuilder, MCPServerType, MCPServerStatus, createMCPServer, listMCPServers, getMCPServerHealth, getCustomMCPServer, } from "./mcp.js";
export { SubscriptionClient, SubscriptionManager, SubscriptionId, SubscriptionError, SubscriptionMetrics, WebSocketConnection, ResourceUpdateSubscriptionBuilder, WorkflowEventSubscriptionBuilder, AgentExecutionSubscriptionBuilder, LLMStreamSubscriptionBuilder, CostUpdateSubscriptionBuilder, MCPServerStatusSubscriptionBuilder, MCPSessionEventSubscriptionBuilder, subscribeResourceUpdates, subscribeWorkflowEvents, subscribeLLMStream, subscribeCostUpdates, } from "./subscriptions.js";
export { NATSClient, CreateWorkflowInstanceBuilder, ExecuteActivityWithNATSBuilder, createWorkflowInstance, executeActivityWithNats, getNatsResource, getResourcesInState, } from "./nats.js";
export type { PingResponse, ServerInfo, PaginationOptions, PaginatedResult, Result, ExecutionStatus, Workflow, WorkflowDefinition, WorkflowState, WorkflowTransition, WorkflowAction, WorkflowCondition, WorkflowExecution, WorkflowCreateInput, Agent, AgentType, AgentConfig, MemoryConfig, Tool, AgentCreateInput, Function, FunctionConfig, DockerConfig, ContainerMount, ResourceLimits, FunctionExecution, FunctionCreateInput, Resource, ResourceCreateInput, ResourceUpdateInput, ChatMessage, ChatCompletionRequest, ChatCompletionResponse, Choice, Usage, } from "./types.js";
export type { MCPServer, MCPServerConnection, MCPOAuthProvider, MCPServerCapabilities, MCPServerHealth, MCPOAuthInitiation, MCPSession, MCPOAuthConfig, MCPJWTConfig, CreateMCPServerInput, UpdateMCPServerInput, ConfigureOAuthInput, ConfigureJWTInput, PaginationInput, } from "./mcp.js";
export type { GraphQLSubscription, GraphQLWSMessage, SubscriptionHandler, SubscriptionConfig, ResourceUpdateEvent, WorkflowEvent, AgentExecutionEvent, LLMStreamChunk, CostUpdateEvent, MCPServerStatusUpdate, MCPSessionEvent, } from "./subscriptions.js";
export type { NATSResource, HistoryEvent, CreateWorkflowInstanceInput, ExecuteActivityWithNATSInput, } from "./nats.js";
export type { Rule, RuleCondition, RuleEvaluationResult, RuleCreateInput, LegacyRule, } from "./rules.js";
export { CircuitBreakerError, NetworkError, ValidationError, NotFoundError, } from "./types.js";
export type { LLMModel, ChatCompletionChunk, ChoiceDelta, MessageDelta, TokenCount, ProviderHealth, } from "./llm.js";
export declare const SDK_VERSION = "0.1.0";
export declare const DEFAULT_BASE_URL = "http://localhost:3000";
export declare const DEFAULT_TIMEOUT = 30000;
import { Client } from "./client.js";
import type { ClientConfig, PingResponse, ServerInfo } from "./types.js";
import type { WorkflowClient } from "./workflows.js";
import type { AgentClient } from "./agents.js";
import type { FunctionClient } from "./functions.js";
import type { ResourceClient } from "./resources.js";
import type { RuleClient } from "./rules.js";
import type { LLMClient } from "./llm.js";
import type { AnalyticsClient } from "./analytics.js";
import type { MCPClient } from "./mcp.js";
import type { SubscriptionClient } from "./subscriptions.js";
import type { NATSClient } from "./nats.js";
/**
 * Main SDK class that provides access to all API clients
 */
export declare class CircuitBreakerSDK {
    private client;
    constructor(config: ClientConfig);
    /**
     * Test connection to the server
     */
    ping(): Promise<PingResponse>;
    /**
     * Get server information
     */
    info(): Promise<ServerInfo>;
    /**
     * Access workflows API
     */
    workflows(): WorkflowClient;
    /**
     * Access agents API
     */
    agents(): AgentClient;
    /**
     * Access functions API
     */
    functions(): FunctionClient;
    /**
     * Access resources API
     */
    resources(): ResourceClient;
    /**
     * Access rules API
     */
    rules(): RuleClient;
    /**
     * Access LLM API
     */
    llm(): LLMClient;
    /**
     * Access analytics and budget management API
     */
    analytics(): AnalyticsClient;
    /**
     * Access MCP (Model Context Protocol) API
     */
    mcp(): MCPClient;
    /**
     * Access real-time subscription API
     */
    subscriptions(): SubscriptionClient;
    /**
     * Access NATS-enhanced operations API
     */
    nats(): NATSClient;
    /**
     * Get the underlying client
     */
    getClient(): Client;
}
/**
 * Create a new SDK instance
 */
export declare function createSDK(config: ClientConfig): CircuitBreakerSDK;
/**
 * Create a simple SDK instance with just a base URL
 */
export declare function createSimpleSDK(baseUrl: string, apiKey?: string): CircuitBreakerSDK;
export default CircuitBreakerSDK;
//# sourceMappingURL=index.d.ts.map