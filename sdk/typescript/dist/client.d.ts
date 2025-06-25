/**
 * GraphQL-based Circuit Breaker TypeScript SDK Client
 *
 * A clean, minimal client implementation using GraphQL that mirrors the Rust SDK approach.
 * Focuses on GraphQL communication with proper error handling.
 */
import { ClientConfig, PingResponse, ServerInfo } from "./types.js";
export type { ClientConfig } from "./types.js";
import { WorkflowClient } from "./workflows.js";
import { AgentClient } from "./agents.js";
import { FunctionClient } from "./functions.js";
import { ResourceClient } from "./resources.js";
import { RuleClient } from "./rules.js";
import { LLMClient } from "./llm.js";
import { AnalyticsClient } from "./analytics.js";
import { MCPClient } from "./mcp.js";
import { SubscriptionClient } from "./subscriptions.js";
import { NATSClient } from "./nats.js";
export interface ClientBuilderConfig extends Partial<ClientConfig> {
    baseUrl?: string;
}
export declare class Client {
    private readonly config;
    private readonly baseHeaders;
    private readonly graphqlEndpoint;
    constructor(config: Required<ClientConfig>);
    /**
     * Create a new client builder
     */
    static builder(): ClientBuilder;
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
     * Get client configuration
     */
    getConfig(): Required<ClientConfig>;
    /**
     * Make a GraphQL request
     */
    graphqlRequest<T>(query: string, variables?: Record<string, any>, operationName?: string): Promise<T>;
    /**
     * Make a GraphQL query
     */
    query<T>(query: string, variables?: Record<string, any>): Promise<T>;
    /**
     * Make a GraphQL mutation
     */
    mutation<T>(mutation: string, variables?: Record<string, any>): Promise<T>;
    /**
     * Legacy REST request method for backward compatibility
     * This method is maintained for existing code that might use it
     */
    request<T>(_method: "GET" | "POST" | "PUT" | "DELETE", _path: string, _body?: any): Promise<T>;
    private handleHttpError;
    private handleGraphQLErrors;
}
export declare class ClientBuilder {
    private config;
    /**
     * Set the base URL for the server
     */
    baseUrl(url: string): ClientBuilder;
    /**
     * Set the API key for authentication
     */
    apiKey(key: string): ClientBuilder;
    /**
     * Set request timeout in milliseconds
     */
    timeout(ms: number): ClientBuilder;
    /**
     * Set additional headers
     */
    headers(headers: Record<string, string>): ClientBuilder;
    /**
     * Build the client
     */
    build(): Client;
}
//# sourceMappingURL=client.d.ts.map