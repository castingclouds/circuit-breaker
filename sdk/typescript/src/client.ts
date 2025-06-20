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

// Re-export types that are needed by index.ts
export type { ClientConfig } from "./types.js";
import { WorkflowClient } from "./workflows.js";
import { AgentClient } from "./agents.js";
import { FunctionClient } from "./functions.js";
import { ResourceClient } from "./resources.js";
import { RuleClient } from "./rules.js";
import { LLMClient } from "./llm.js";

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

  constructor(config: Required<ClientConfig>) {
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
  static builder(): ClientBuilder {
    return new ClientBuilder();
  }

  // ============================================================================
  // Health and Info Methods
  // ============================================================================

  /**
   * Test connection to the server
   */
  async ping(): Promise<PingResponse> {
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

    const result = await this.graphqlRequest<{
      llmProviders: Array<{
        name: string;
        healthStatus: { isHealthy: boolean };
      }>;
    }>(query);

    // Convert GraphQL response to expected ping format
    const _healthyProviders = result.llmProviders.filter(
      (p) => p.healthStatus.isHealthy,
    );

    return {
      status: "ok",
      version: "1.0.0", // Could be retrieved from server if available
      uptime_seconds: 0, // Could be retrieved from server if available
    };
  }

  /**
   * Get server information
   */
  async info(): Promise<ServerInfo> {
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

    const result = await this.graphqlRequest<{
      llmProviders: Array<{
        name: string;
        healthStatus: { isHealthy: boolean };
      }>;
    }>(query);

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

  // ============================================================================
  // GraphQL Request Methods
  // ============================================================================

  /**
   * Make a GraphQL request
   */
  async graphqlRequest<T>(
    query: string,
    variables?: Record<string, any>,
    operationName?: string,
  ): Promise<T> {
    const request: GraphQLRequest = {
      query,
      variables: variables || {},
      operationName,
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
   * Legacy REST request method for backward compatibility
   * This method is maintained for existing code that might use it
   */
  async request<T>(
    _method: "GET" | "POST" | "PUT" | "DELETE",
    _path: string,
    _body?: any,
  ): Promise<T> {
    // This is a fallback that converts some REST calls to GraphQL
    // For full functionality, use the GraphQL methods directly

    if (_path === "/health") {
      return this.ping() as Promise<T>;
    }

    if (_path === "/info") {
      return this.info() as Promise<T>;
    }

    throw new ValidationError(
      `REST endpoint ${_method} ${_path} not supported. Use GraphQL methods instead.`,
    );
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
    const _mainError = errors[0];
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
