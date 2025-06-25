/**
 * GraphQL-based Circuit Breaker TypeScript SDK Client
 *
 * A clean, minimal client implementation using GraphQL that mirrors the Rust SDK approach.
 * Focuses on GraphQL communication with proper error handling.
 */
import { CircuitBreakerError, NetworkError, ValidationError, NotFoundError, } from "./types.js";
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
const DEFAULT_CONFIG = {
    baseUrl: "http://localhost:4000",
    apiKey: "",
    timeout: 30000,
    headers: {},
};
// ============================================================================
// Main Client Class
// ============================================================================
export class Client {
    constructor(config) {
        this.config = config;
        this.graphqlEndpoint = `${config.baseUrl}/graphql`;
        // Prepare base headers
        this.baseHeaders = {
            "Content-Type": "application/json",
            "User-Agent": `circuit-breaker-sdk-typescript/0.1.0`,
            ...config.headers,
        };
        if (config.apiKey) {
            this.baseHeaders["Authorization"] = `Bearer ${config.apiKey}`;
        }
    }
    // ============================================================================
    // Static Factory Methods
    // ============================================================================
    /**
     * Create a new client builder
     */
    static builder() {
        return new ClientBuilder();
    }
    // ============================================================================
    // Health and Info Methods
    // ============================================================================
    /**
     * Test connection to the server
     */
    async ping() {
        const query = `
      query {
        llmProviders {
          name
          healthStatus {
            isHealthy
          }
        }
      }
    `;
        const result = await this.graphqlRequest(query);
        // Convert GraphQL response to expected ping format
        const _healthyProviders = result.llmProviders.filter((p) => p.healthStatus.isHealthy);
        return {
            status: "ok",
            version: "1.0.0", // Could be retrieved from server if available
            uptime_seconds: 0, // Could be retrieved from server if available
        };
    }
    /**
     * Get server information
     */
    async info() {
        const query = `
      query {
        llmProviders {
          name
          healthStatus {
            isHealthy
          }
        }
      }
    `;
        const result = await this.graphqlRequest(query);
        return {
            name: "Circuit Breaker GraphQL Server",
            version: "1.0.0",
            features: ["workflows", "agents", "functions", "llm", "rules"],
            providers: result.llmProviders.map((p) => p.name),
        };
    }
    // ============================================================================
    // API Client Access
    // ============================================================================
    /**
     * Access workflows API
     */
    workflows() {
        return new WorkflowClient(this);
    }
    /**
     * Access agents API
     */
    agents() {
        return new AgentClient(this);
    }
    /**
     * Access functions API
     */
    functions() {
        return new FunctionClient(this);
    }
    /**
     * Access resources API
     */
    resources() {
        return new ResourceClient(this);
    }
    /**
     * Access rules API
     */
    rules() {
        return new RuleClient(this);
    }
    /**
     * Access LLM API
     */
    llm() {
        return new LLMClient(this);
    }
    /**
     * Access analytics and budget management API
     */
    analytics() {
        return new AnalyticsClient(this);
    }
    /**
     * Access MCP (Model Context Protocol) API
     */
    mcp() {
        return new MCPClient(this);
    }
    /**
     * Access real-time subscription API
     */
    subscriptions() {
        return new SubscriptionClient(this);
    }
    /**
     * Access NATS-enhanced operations API
     */
    nats() {
        return new NATSClient(this);
    }
    /**
     * Get client configuration
     */
    getConfig() {
        return this.config;
    }
    // ============================================================================
    // GraphQL Request Methods
    // ============================================================================
    /**
     * Make a GraphQL request
     */
    async graphqlRequest(query, variables, operationName) {
        const request = {
            query,
            variables: variables || {},
            operationName,
        };
        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);
            const response = await fetch(this.graphqlEndpoint, {
                method: "POST",
                headers: this.baseHeaders,
                body: JSON.stringify(request),
                signal: controller.signal,
            });
            clearTimeout(timeoutId);
            if (!response.ok) {
                throw await this.handleHttpError(response);
            }
            const result = await response.json();
            if (result.errors && result.errors.length > 0) {
                throw this.handleGraphQLErrors(result.errors);
            }
            if (!result.data) {
                throw new NetworkError("No data returned from GraphQL query");
            }
            return result.data;
        }
        catch (error) {
            if (error instanceof CircuitBreakerError) {
                throw error;
            }
            if (error instanceof TypeError && error.message.includes("fetch")) {
                throw new NetworkError(`Network error: ${error.message}`, error);
            }
            if (error instanceof DOMException && error.name === "AbortError") {
                throw new NetworkError("Request timeout", error);
            }
            throw new NetworkError(`Request failed: ${error}`, error);
        }
    }
    /**
     * Make a GraphQL query
     */
    async query(query, variables) {
        return this.graphqlRequest(query, variables);
    }
    /**
     * Make a GraphQL mutation
     */
    async mutation(mutation, variables) {
        return this.graphqlRequest(mutation, variables);
    }
    // ============================================================================
    // Backward Compatibility - REST-like interface
    // ============================================================================
    /**
     * Legacy REST request method for backward compatibility
     * This method is maintained for existing code that might use it
     */
    async request(_method, _path, _body) {
        // This is a fallback that converts some REST calls to GraphQL
        // For full functionality, use the GraphQL methods directly
        if (_path === "/health") {
            return this.ping();
        }
        if (_path === "/info") {
            return this.info();
        }
        throw new ValidationError(`REST endpoint ${_method} ${_path} not supported. Use GraphQL methods instead.`);
    }
    // ============================================================================
    // Error Handling
    // ============================================================================
    async handleHttpError(response) {
        const contentType = response.headers.get("content-type");
        let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
        let errorBody = null;
        try {
            if (contentType?.includes("application/json")) {
                errorBody = await response.json();
                errorMessage = errorBody.message || errorMessage;
            }
            else {
                const text = await response.text();
                if (text) {
                    errorMessage = text;
                }
            }
        }
        catch {
            // Ignore JSON parsing errors, use default message
        }
        switch (response.status) {
            case 400:
                return new ValidationError(errorMessage, errorBody);
            case 404:
                return new NotFoundError(errorMessage, errorBody);
            case 401:
            case 403:
                return new ValidationError(`Authentication error: ${errorMessage}`, errorBody);
            case 429:
                return new NetworkError(`Rate limited: ${errorMessage}`, errorBody);
            case 500:
            case 502:
            case 503:
            case 504:
                return new NetworkError(`Server error: ${errorMessage}`, errorBody);
            default:
                return new NetworkError(errorMessage, errorBody);
        }
    }
    handleGraphQLErrors(errors) {
        const _mainError = errors[0];
        const message = errors.map((e) => e.message).join(", ");
        // Determine error type based on message content
        if (message.includes("Unknown field") ||
            message.includes("Invalid value")) {
            return new ValidationError(`GraphQL validation error: ${message}`, {
                errors,
            });
        }
        if (message.includes("Not found") || message.includes("does not exist")) {
            return new NotFoundError(`GraphQL error: ${message}`, { errors });
        }
        return new NetworkError(`GraphQL error: ${message}`, { errors });
    }
}
// ============================================================================
// Client Builder
// ============================================================================
export class ClientBuilder {
    constructor() {
        this.config = {};
    }
    /**
     * Set the base URL for the server
     */
    baseUrl(url) {
        this.config.baseUrl = url;
        return this;
    }
    /**
     * Set the API key for authentication
     */
    apiKey(key) {
        this.config.apiKey = key;
        return this;
    }
    /**
     * Set request timeout in milliseconds
     */
    timeout(ms) {
        this.config.timeout = ms;
        return this;
    }
    /**
     * Set additional headers
     */
    headers(headers) {
        this.config.headers = { ...this.config.headers, ...headers };
        return this;
    }
    /**
     * Build the client
     */
    build() {
        const finalConfig = {
            ...DEFAULT_CONFIG,
            ...this.config,
        };
        return new Client(finalConfig);
    }
}
//# sourceMappingURL=client.js.map