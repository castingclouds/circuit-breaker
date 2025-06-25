/**
 * Circuit Breaker TypeScript SDK
 *
 * A simple, clean TypeScript client for the Circuit Breaker workflow engine.
 * Mirrors the Rust SDK approach with minimal abstractions and direct API access.
 */
// ============================================================================
// Core Client
// ============================================================================
export { Client, ClientBuilder } from "./client.js";
// ============================================================================
// API Clients
// ============================================================================
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
// ============================================================================
// Error Classes
// ============================================================================
export { CircuitBreakerError, NetworkError, ValidationError, NotFoundError, } from "./types.js";
// ============================================================================
// Constants
// ============================================================================
export const SDK_VERSION = "0.1.0";
export const DEFAULT_BASE_URL = "http://localhost:3000";
export const DEFAULT_TIMEOUT = 30000;
// ============================================================================
// Main SDK Class (Convenience)
// ============================================================================
// ============================================================================
// Import required classes for SDK class
// ============================================================================
import { Client } from "./client.js";
/**
 * Main SDK class that provides access to all API clients
 */
export class CircuitBreakerSDK {
    constructor(config) {
        this.client = Client.builder()
            .baseUrl(config.baseUrl)
            .timeout(config.timeout || DEFAULT_TIMEOUT)
            .build();
        if (config.apiKey) {
            // Rebuild with API key
            this.client = Client.builder()
                .baseUrl(config.baseUrl)
                .apiKey(config.apiKey)
                .timeout(config.timeout || DEFAULT_TIMEOUT)
                .build();
        }
        // Add custom headers if provided
        if (config.headers) {
            let builder = Client.builder()
                .baseUrl(config.baseUrl)
                .timeout(config.timeout || DEFAULT_TIMEOUT);
            if (config.apiKey) {
                builder = builder.apiKey(config.apiKey);
            }
            Object.entries(config.headers).forEach(([key, value]) => {
                const headers = { [key]: value };
                builder = builder.headers(headers);
            });
            this.client = builder.build();
        }
    }
    /**
     * Test connection to the server
     */
    async ping() {
        return this.client.ping();
    }
    /**
     * Get server information
     */
    async info() {
        return this.client.info();
    }
    /**
     * Access workflows API
     */
    workflows() {
        return this.client.workflows();
    }
    /**
     * Access agents API
     */
    agents() {
        return this.client.agents();
    }
    /**
     * Access functions API
     */
    functions() {
        return this.client.functions();
    }
    /**
     * Access resources API
     */
    resources() {
        return this.client.resources();
    }
    /**
     * Access rules API
     */
    rules() {
        return this.client.rules();
    }
    /**
     * Access LLM API
     */
    llm() {
        return this.client.llm();
    }
    /**
     * Access analytics and budget management API
     */
    analytics() {
        return this.client.analytics();
    }
    /**
     * Access MCP (Model Context Protocol) API
     */
    mcp() {
        return this.client.mcp();
    }
    /**
     * Access real-time subscription API
     */
    subscriptions() {
        return this.client.subscriptions();
    }
    /**
     * Access NATS-enhanced operations API
     */
    nats() {
        return this.client.nats();
    }
    /**
     * Get the underlying client
     */
    getClient() {
        return this.client;
    }
}
// ============================================================================
// Convenience Factory Functions
// ============================================================================
/**
 * Create a new SDK instance
 */
export function createSDK(config) {
    return new CircuitBreakerSDK(config);
}
/**
 * Create a simple SDK instance with just a base URL
 */
export function createSimpleSDK(baseUrl, apiKey) {
    const config = {
        baseUrl,
        ...(apiKey && { apiKey }),
    };
    return new CircuitBreakerSDK(config);
}
// ============================================================================
// Default Export
// ============================================================================
export default CircuitBreakerSDK;
//# sourceMappingURL=index.js.map