/**
 * GraphQL-based Circuit Breaker TypeScript SDK Client
 *
 * A clean, minimal client implementation using GraphQL that mirrors the Rust SDK approach.
 * Focuses on GraphQL communication with proper error handling.
 */

import {
  ClientConfig,
  PingResponse,
  ServerInfo,
  CircuitBreakerError,
  NetworkError,
  ValidationError,
  NotFoundError,
} from "./types.js";
import { QueryBuilder } from "./schema";

// Re-export types that are needed by index.ts
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

// ============================================================================
// GraphQL Types
// ============================================================================

interface GraphQLResponse<T> {
  data?: T;
  errors?: Array<{
    message: string;
    locations?: Array<{ line: number; column: number }>;
    path?: Array<string | number>;
  }>;
}

interface GraphQLRequest {
  query: string;
  variables?: Record<string, any>;
  operationName?: string;
}

interface RestRequest {
  method: string;
  path: string;
  body?: any;
  headers?: Record<string, string>;
}

interface EndpointHealth {
  graphql: boolean;
  rest: boolean;
  graphqlUrl: string;
  restUrl: string;
}

// ============================================================================
// Client Configuration
// ============================================================================

export interface ClientBuilderConfig extends Partial<ClientConfig> {
  baseUrl?: string;
}

const DEFAULT_CONFIG: Required<ClientConfig> = {
  baseUrl: "http://localhost:4000",
  apiKey: "",
  timeout: 30000,
  headers: {},
};

// ============================================================================
// Main Client Class
// ============================================================================

export class Client {
  private readonly config: Required<ClientConfig>;
  private readonly baseHeaders: Record<string, string>;
  private readonly graphqlEndpoint: string;
  private readonly restEndpoint: string;
  private endpointHealth: EndpointHealth | null = null;

  constructor(config: Required<ClientConfig>) {
    this.config = config;

    // Smart endpoint detection - check if base URL includes port
    const baseUrl = new URL(config.baseUrl);

    if (baseUrl.port === "3000" || baseUrl.pathname.includes("v1")) {
      // REST endpoint specified
      this.restEndpoint = config.baseUrl;
      this.graphqlEndpoint = `${baseUrl.protocol}//${baseUrl.hostname}:4000/graphql`;
    } else if (
      baseUrl.port === "4000" ||
      baseUrl.pathname.includes("graphql")
    ) {
      // GraphQL endpoint specified
      this.graphqlEndpoint = config.baseUrl.endsWith("/graphql")
        ? config.baseUrl
        : `${config.baseUrl}/graphql`;
      this.restEndpoint = `${baseUrl.protocol}//${baseUrl.hostname}:3000`;
    } else {
      // Default ports
      this.graphqlEndpoint = `${baseUrl.protocol}//${baseUrl.hostname}:4000/graphql`;
      this.restEndpoint = `${baseUrl.protocol}//${baseUrl.hostname}:3000`;
    }

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
  static builder(): ClientBuilder {
    return new ClientBuilder();
  }

  // ============================================================================
  // Health and Info Methods
  // ============================================================================

  /**
   * Test connection to both REST and GraphQL endpoints
   */
  async ping(): Promise<PingResponse> {
    // First, check endpoint health
    await this.checkEndpointHealth();

    if (!this.endpointHealth?.graphql && !this.endpointHealth?.rest) {
      throw new NetworkError(
        "Neither GraphQL nor REST endpoints are available",
      );
    }

    try {
      let healthData: any = {};
      let status = "partial";

      // Try GraphQL endpoint first
      if (this.endpointHealth.graphql) {
        try {
          const graphqlResponse = await fetch(this.graphqlEndpoint, {
            method: "POST",
            headers: this.baseHeaders,
            body: JSON.stringify({
              query: `query { __schema { types { name } } }`,
            }),
            signal: AbortSignal.timeout(this.config.timeout),
          });

          if (graphqlResponse.ok) {
            status = "ok";
            healthData.graphql = true;
          }
        } catch (error) {
          console.warn("GraphQL endpoint check failed:", error);
        }
      }

      // Try REST endpoint
      if (this.endpointHealth.rest) {
        try {
          const restResponse = await fetch(`${this.restEndpoint}/v1/models`, {
            method: "GET",
            headers: this.baseHeaders,
            signal: AbortSignal.timeout(this.config.timeout),
          });

          if (restResponse.ok) {
            const modelsData = await restResponse.json();
            healthData.rest = true;
            healthData.models = modelsData.data?.length || 0;
            if (status === "partial") status = "ok";
          }
        } catch (error) {
          console.warn("REST endpoint check failed:", error);
        }
      }

      return {
        status,
        version: "1.0.0",
        uptime_seconds: Date.now() / 1000,
        endpoints: {
          graphql: this.endpointHealth.graphql,
          rest: this.endpointHealth.rest,
          graphqlUrl: this.endpointHealth.graphqlUrl,
          restUrl: this.endpointHealth.restUrl,
        },
        ...healthData,
      };
    } catch (error) {
      if (error instanceof NetworkError) {
        throw error;
      }
      throw new NetworkError(`Health check failed: ${error}`);
    }
  }

  /**
   * Check health of both endpoints
   */
  private async checkEndpointHealth(): Promise<EndpointHealth> {
    if (this.endpointHealth) {
      return this.endpointHealth;
    }

    const health: EndpointHealth = {
      graphql: false,
      rest: false,
      graphqlUrl: this.graphqlEndpoint,
      restUrl: this.restEndpoint,
    };

    // Check GraphQL endpoint
    try {
      const graphqlResponse = await fetch(this.graphqlEndpoint, {
        method: "GET",
        signal: AbortSignal.timeout(5000),
      });
      health.graphql =
        graphqlResponse.status === 405 || graphqlResponse.status === 400; // GraphQL typically returns 405 for GET
    } catch {
      health.graphql = false;
    }

    // Check REST endpoint
    try {
      const restResponse = await fetch(`${this.restEndpoint}/v1/models`, {
        method: "GET",
        signal: AbortSignal.timeout(5000),
      });
      health.rest = restResponse.ok;
    } catch {
      health.rest = false;
    }

    this.endpointHealth = health;
    return health;
  }

  /**
   * Get server information from both endpoints
   */
  async info(): Promise<ServerInfo> {
    await this.checkEndpointHealth();

    const info: ServerInfo = {
      name: "Circuit Breaker AI Workflow Engine",
      version: "1.0.0",
      features: [],
      providers: [],
      endpoints: {
        graphql: this.endpointHealth!.graphql,
        rest: this.endpointHealth!.rest,
      },
    };

    // Get GraphQL schema info if available
    if (this.endpointHealth!.graphql) {
      try {
        const schemaQuery = `
          query IntrospectionQuery {
            __schema {
              types {
                name
                kind
              }
            }
          }
        `;

        const schemaResult = await this.graphqlRequest<any>(schemaQuery);
        const types = schemaResult.__schema?.types || [];

        // Determine features from schema types
        const features = new Set<string>();
        if (types.some((t: any) => t.name.includes("Workflow")))
          features.add("workflows");
        if (types.some((t: any) => t.name.includes("Agent")))
          features.add("agents");
        if (types.some((t: any) => t.name.includes("Rule")))
          features.add("rules");
        if (
          types.some(
            (t: any) => t.name.includes("Llm") || t.name.includes("LLM"),
          )
        )
          features.add("llm");
        if (
          types.some(
            (t: any) => t.name.includes("Mcp") || t.name.includes("MCP"),
          )
        )
          features.add("mcp");
        if (types.some((t: any) => t.name.includes("Analytics")))
          features.add("analytics");

        info.features.push(...Array.from(features));
      } catch (error) {
        console.warn("Failed to get GraphQL schema info:", error);
      }
    }

    // Get REST API info if available
    if (this.endpointHealth!.rest) {
      try {
        const modelsResponse = await fetch(`${this.restEndpoint}/v1/models`, {
          method: "GET",
          headers: this.baseHeaders,
          signal: AbortSignal.timeout(this.config.timeout),
        });

        if (modelsResponse.ok) {
          const modelsData = await modelsResponse.json();
          const providers = [
            ...new Set(
              modelsData.data?.map((model: any) => model.provider) || [],
            ),
          ];
          info.providers = providers;
          info.features.push(
            "llm-routing",
            "smart-routing",
            "virtual-models",
            "streaming",
          );
        }
      } catch (error) {
        console.warn("Failed to get REST API info:", error);
      }
    }

    return info;
  }

  // ============================================================================
  // API Client Access
  // ============================================================================

  /**
   * Access workflows API
   */
  workflows(): WorkflowClient {
    return new WorkflowClient(this);
  }

  /**
   * Access agents API
   */
  agents(): AgentClient {
    return new AgentClient(this);
  }

  /**
   * Access functions API
   */
  functions(): FunctionClient {
    return new FunctionClient(this);
  }

  /**
   * Access resources API
   */
  resources(): ResourceClient {
    return new ResourceClient(this);
  }

  /**
   * Access rules API
   */
  rules(): RuleClient {
    return new RuleClient(this);
  }

  /**
   * Access LLM API
   */
  llm(): LLMClient {
    return new LLMClient(this);
  }

  /**
   * Access analytics and budget management API
   */
  analytics(): AnalyticsClient {
    return new AnalyticsClient(this);
  }

  /**
   * Access MCP (Model Context Protocol) API
   */
  mcp(): MCPClient {
    return new MCPClient(this);
  }

  /**
   * Access real-time subscription API
   */
  subscriptions(): SubscriptionClient {
    return new SubscriptionClient(this);
  }

  /**
   * Access NATS-enhanced operations API
   */
  nats(): NATSClient {
    return new NATSClient(this);
  }

  /**
   * Get client configuration
   */
  getConfig(): Required<ClientConfig> {
    return this.config;
  }

  // ============================================================================
  // GraphQL Request Methods
  // ============================================================================

  /**
   * Make a GraphQL request with automatic endpoint validation
   */
  async graphqlRequest<T>(
    query: string,
    variables?: Record<string, any>,
    operationName?: string,
  ): Promise<T> {
    // Ensure GraphQL endpoint is available
    await this.checkEndpointHealth();

    if (!this.endpointHealth!.graphql) {
      throw new NetworkError("GraphQL endpoint is not available");
    }

    const request: GraphQLRequest = {
      query,
      variables: variables || {},
      ...(operationName && { operationName }),
    };

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(
        () => controller.abort(),
        this.config.timeout,
      );

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

      const result: GraphQLResponse<T> = await response.json();

      if (result.errors && result.errors.length > 0) {
        throw this.handleGraphQLErrors(result.errors);
      }

      if (!result.data) {
        throw new NetworkError("No data returned from GraphQL query");
      }

      return result.data;
    } catch (error) {
      if (error instanceof CircuitBreakerError) {
        throw error;
      }

      if (error instanceof TypeError && error.message.includes("fetch")) {
        throw new NetworkError(`Network error: ${error.message}`, error);
      }

      if (error instanceof DOMException && error.name === "AbortError") {
        throw new NetworkError("Request timeout", error);
      }

      throw new NetworkError(
        `Request failed: ${error}`,
        error as Record<string, any>,
      );
    }
  }

  /**
   * Make a REST API request with automatic endpoint validation
   */
  async restRequest<T>(
    method: string,
    path: string,
    body?: any,
    headers?: Record<string, string>,
  ): Promise<T> {
    // Ensure REST endpoint is available
    await this.checkEndpointHealth();

    if (!this.endpointHealth!.rest) {
      throw new NetworkError("REST endpoint is not available");
    }

    const url = path.startsWith("/")
      ? `${this.restEndpoint}${path}`
      : `${this.restEndpoint}/${path}`;
    const requestHeaders = { ...this.baseHeaders, ...headers };

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(
        () => controller.abort(),
        this.config.timeout,
      );

      const requestInit: RequestInit = {
        method,
        headers: requestHeaders,
        signal: controller.signal,
      };

      if (
        body &&
        (method === "POST" || method === "PUT" || method === "PATCH")
      ) {
        requestInit.body = JSON.stringify(body);
      }

      const response = await fetch(url, requestInit);
      clearTimeout(timeoutId);

      if (!response.ok) {
        throw await this.handleHttpError(response);
      }

      // Handle streaming responses
      if (
        response.headers.get("content-type")?.includes("text/plain") ||
        response.headers.get("content-type")?.includes("text/event-stream")
      ) {
        return response as unknown as T;
      }

      return await response.json();
    } catch (error) {
      if (error instanceof CircuitBreakerError) {
        throw error;
      }

      if (error instanceof TypeError && error.message.includes("fetch")) {
        throw new NetworkError(`Network error: ${error.message}`, error);
      }

      if (error instanceof DOMException && error.name === "AbortError") {
        throw new NetworkError("Request timeout", error);
      }

      throw new NetworkError(
        `REST request failed: ${error}`,
        error as Record<string, any>,
      );
    }
  }

  /**
   * Make a GraphQL query
   */
  async query<T>(query: string, variables?: Record<string, any>): Promise<T> {
    return this.graphqlRequest<T>(query, variables);
  }

  /**
   * Make a GraphQL mutation
   */
  async mutation<T>(
    mutation: string,
    variables?: Record<string, any>,
  ): Promise<T> {
    return this.graphqlRequest<T>(mutation, variables);
  }

  // ============================================================================
  // Backward Compatibility - REST-like interface
  // ============================================================================

  /**
   * Smart request method that automatically routes to the appropriate endpoint
   */
  async request<T>(
    method: "GET" | "POST" | "PUT" | "DELETE",
    path: string,
    body?: any,
  ): Promise<T> {
    // Handle special endpoints
    if (path === "/health" || path === "health") {
      return this.ping() as Promise<T>;
    }

    if (path === "/info" || path === "info") {
      return this.info() as Promise<T>;
    }

    // Route OpenAI-compatible endpoints to REST
    if (path.startsWith("/v1/") || path.startsWith("v1/")) {
      return this.restRequest<T>(method, path, body);
    }

    // Route GraphQL queries
    if (path === "/graphql" || path === "graphql") {
      if (method !== "POST") {
        throw new ValidationError(
          "GraphQL endpoint only supports POST requests",
        );
      }
      return this.graphqlRequest<T>(
        body.query,
        body.variables,
        body.operationName,
      );
    }

    // Default to REST for other paths
    return this.restRequest<T>(method, path, body);
  }

  /**
   * Get the appropriate endpoint URL for a given operation
   */
  getEndpointUrl(operation: "graphql" | "rest"): string {
    return operation === "graphql" ? this.graphqlEndpoint : this.restEndpoint;
  }

  /**
   * Check if a specific endpoint is available
   */
  async isEndpointAvailable(endpoint: "graphql" | "rest"): Promise<boolean> {
    await this.checkEndpointHealth();
    return endpoint === "graphql"
      ? this.endpointHealth!.graphql
      : this.endpointHealth!.rest;
  }

  // ============================================================================
  // Error Handling
  // ============================================================================

  private async handleHttpError(
    response: Response,
  ): Promise<CircuitBreakerError> {
    const contentType = response.headers.get("content-type");
    let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
    let errorBody = null;

    try {
      if (contentType?.includes("application/json")) {
        errorBody = await response.json();
        errorMessage = errorBody.message || errorMessage;
      } else {
        const text = await response.text();
        if (text) {
          errorMessage = text;
        }
      }
    } catch {
      // Ignore JSON parsing errors, use default message
    }

    switch (response.status) {
      case 400:
        return new ValidationError(errorMessage, errorBody);
      case 404:
        return new NotFoundError(errorMessage, errorBody);
      case 401:
      case 403:
        return new ValidationError(
          `Authentication error: ${errorMessage}`,
          errorBody,
        );
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

  private handleGraphQLErrors(
    errors: Array<{ message: string; path?: Array<string | number> }>,
  ): CircuitBreakerError {
    const message = errors.map((e) => e.message).join(", ");

    // Determine error type based on message content
    if (
      message.includes("Unknown field") ||
      message.includes("Invalid value")
    ) {
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
  private config: Partial<ClientConfig> = {};

  /**
   * Set the base URL for the server
   */
  baseUrl(url: string): ClientBuilder {
    this.config.baseUrl = url;
    return this;
  }

  /**
   * Set the API key for authentication
   */
  apiKey(key: string): ClientBuilder {
    this.config.apiKey = key;
    return this;
  }

  /**
   * Set request timeout in milliseconds
   */
  timeout(ms: number): ClientBuilder {
    this.config.timeout = ms;
    return this;
  }

  /**
   * Set additional headers
   */
  headers(headers: Record<string, string>): ClientBuilder {
    this.config.headers = { ...this.config.headers, ...headers };
    return this;
  }

  /**
   * Build the client
   */
  build(): Client {
    const finalConfig: Required<ClientConfig> = {
      ...DEFAULT_CONFIG,
      ...this.config,
    };

    return new Client(finalConfig);
  }
}
