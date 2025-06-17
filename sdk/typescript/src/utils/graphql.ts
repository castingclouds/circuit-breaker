/**
 * GraphQL client utility for Circuit Breaker TypeScript SDK
 *
 * This file provides a robust GraphQL client with error handling,
 * request/response logging, timeout support, and type-safe operations.
 */

import { GraphQLError, NetworkError, TimeoutError, ConnectionError } from '../core/errors.js';

// ============================================================================
// Types
// ============================================================================

export interface GraphQLRequest {
  query: string;
  variables?: Record<string, any>;
  operationName?: string;
}

export interface GraphQLResponse<T = any> {
  data?: T;
  errors?: GraphQLErrorResponse[];
  extensions?: Record<string, any>;
}

export interface GraphQLErrorResponse {
  message: string;
  locations?: Array<{
    line: number;
    column: number;
  }>;
  path?: Array<string | number>;
  extensions?: Record<string, any>;
}

export interface GraphQLClientConfig {
  endpoint: string;
  headers?: Record<string, string>;
  timeout?: number;
  debug?: boolean;
  retryAttempts?: number;
  retryDelay?: number;
  logger?: (level: string, message: string, meta?: any) => void;
}

export interface RequestLog {
  id: string;
  timestamp: Date;
  query: string;
  variables?: Record<string, any>;
  operationName?: string;
  response?: GraphQLResponse;
  error?: Error;
  duration: number;
  status: 'success' | 'error' | 'timeout';
}

// ============================================================================
// GraphQL Client
// ============================================================================

export class GraphQLClient {
  private readonly endpoint: string;
  private readonly defaultHeaders: Record<string, string>;
  private readonly timeout: number;
  private readonly debug: boolean;
  private readonly retryAttempts: number;
  private readonly retryDelay: number;
  private readonly logger?: (level: string, message: string, meta?: any) => void;
  private readonly requestLog: RequestLog[] = [];

  constructor(config: GraphQLClientConfig) {
    this.endpoint = config.endpoint;
    this.defaultHeaders = {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
      ...config.headers,
    };
    this.timeout = config.timeout ?? 30000;
    this.debug = config.debug ?? false;
    this.retryAttempts = config.retryAttempts ?? 3;
    this.retryDelay = config.retryDelay ?? 1000;
    this.logger = config.logger;
  }

  /**
   * Execute a GraphQL request
   */
  async request<T = any>(
    query: string,
    variables?: Record<string, any>,
    operationName?: string,
    headers?: Record<string, string>
  ): Promise<T> {
    const requestId = this.generateRequestId();
    const startTime = Date.now();

    if (this.debug) {
      this.log('debug', 'GraphQL request started', {
        requestId,
        operationName,
        query: this.sanitizeQuery(query),
        variables: this.sanitizeVariables(variables),
      });
    }

    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= this.retryAttempts; attempt++) {
      try {
        const response = await this.executeRequest({
          query,
          variables,
          operationName,
        }, headers, requestId);

        const duration = Date.now() - startTime;

        // Log successful request
        const logEntry: RequestLog = {
          id: requestId,
          timestamp: new Date(),
          query,
          variables,
          operationName,
          response,
          duration,
          status: 'success',
        };
        this.requestLog.push(logEntry);

        if (this.debug) {
          this.log('debug', 'GraphQL request completed', {
            requestId,
            duration,
            dataPresent: !!response.data,
            errorsCount: response.errors?.length || 0,
          });
        }

        // Handle GraphQL errors
        if (response.errors && response.errors.length > 0) {
          throw new GraphQLError(
            response.errors,
            query,
            variables,
            requestId
          );
        }

        if (!response.data) {
          throw new GraphQLError(
            [{ message: 'No data returned from GraphQL query' }],
            query,
            variables,
            requestId
          );
        }

        return response.data;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        const duration = Date.now() - startTime;
        const logEntry: RequestLog = {
          id: requestId,
          timestamp: new Date(),
          query,
          variables,
          operationName,
          error: lastError,
          duration,
          status: error instanceof TimeoutError ? 'timeout' : 'error',
        };
        this.requestLog.push(logEntry);

        if (this.debug) {
          this.log('error', 'GraphQL request failed', {
            requestId,
            attempt: attempt + 1,
            error: lastError.message,
            duration,
          });
        }

        // Don't retry on certain error types
        if (
          error instanceof GraphQLError ||
          error instanceof TimeoutError ||
          attempt === this.retryAttempts
        ) {
          throw error;
        }

        // Wait before retry
        if (attempt < this.retryAttempts) {
          await this.sleep(this.retryDelay * Math.pow(2, attempt));
        }
      }
    }

    throw lastError || new Error('Request failed after all retry attempts');
  }

  /**
   * Execute batch GraphQL requests
   */
  async batchRequest<T = any>(
    requests: GraphQLRequest[],
    headers?: Record<string, string>
  ): Promise<T[]> {
    const promises = requests.map(request =>
      this.request<T>(
        request.query,
        request.variables,
        request.operationName,
        headers
      )
    );

    return Promise.all(promises);
  }

  /**
   * Get request log for debugging
   */
  getRequestLog(): RequestLog[] {
    return [...this.requestLog];
  }

  /**
   * Clear request log
   */
  clearRequestLog(): void {
    this.requestLog.length = 0;
  }

  /**
   * Check if GraphQL endpoint is reachable
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(this.endpoint, {
        method: 'POST',
        headers: this.defaultHeaders,
        body: JSON.stringify({
          query: '{ __schema { queryType { name } } }',
        }),
        signal: AbortSignal.timeout(5000),
      });

      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * Get GraphQL schema introspection
   */
  async introspectSchema(): Promise<any> {
    const introspectionQuery = `
      query IntrospectionQuery {
        __schema {
          queryType { name }
          mutationType { name }
          subscriptionType { name }
          types {
            ...FullType
          }
          directives {
            name
            description
            locations
            args {
              ...InputValue
            }
          }
        }
      }

      fragment FullType on __Type {
        kind
        name
        description
        fields(includeDeprecated: true) {
          name
          description
          args {
            ...InputValue
          }
          type {
            ...TypeRef
          }
          isDeprecated
          deprecationReason
        }
        inputFields {
          ...InputValue
        }
        interfaces {
          ...TypeRef
        }
        enumValues(includeDeprecated: true) {
          name
          description
          isDeprecated
          deprecationReason
        }
        possibleTypes {
          ...TypeRef
        }
      }

      fragment InputValue on __InputValue {
        name
        description
        type { ...TypeRef }
        defaultValue
      }

      fragment TypeRef on __Type {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                    }
                  }
                }
              }
            }
          }
        }
      }
    `;

    return this.request(introspectionQuery);
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async executeRequest(
    request: GraphQLRequest,
    headers?: Record<string, string>,
    requestId?: string
  ): Promise<GraphQLResponse> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(this.endpoint, {
        method: 'POST',
        headers: {
          ...this.defaultHeaders,
          ...headers,
        },
        body: JSON.stringify({
          query: request.query,
          variables: request.variables,
          operationName: request.operationName,
        }),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        if (response.status === 408 || response.status === 504) {
          throw new TimeoutError('GraphQL request', this.timeout, requestId);
        }

        const errorText = await response.text();
        throw new ConnectionError(
          this.endpoint,
          `HTTP ${response.status}: ${errorText}`,
          requestId
        );
      }

      const contentType = response.headers.get('content-type');
      if (!contentType?.includes('application/json')) {
        throw new NetworkError(
          'Invalid response content type',
          'INVALID_CONTENT_TYPE',
          { contentType },
          requestId
        );
      }

      const result = await response.json();
      return result as GraphQLResponse;
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof DOMException && error.name === 'AbortError') {
        throw new TimeoutError('GraphQL request', this.timeout, requestId);
      }

      if (error instanceof TypeError && error.message.includes('fetch')) {
        throw new ConnectionError(
          this.endpoint,
          'Network connection failed',
          requestId
        );
      }

      throw error;
    }
  }

  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private sanitizeQuery(query: string): string {
    // Remove extra whitespace and normalize for logging
    return query.replace(/\s+/g, ' ').trim();
  }

  private sanitizeVariables(variables?: Record<string, any>): Record<string, any> | undefined {
    if (!variables) return undefined;

    // Remove sensitive data from variables for logging
    const sanitized = { ...variables };
    const sensitiveKeys = ['password', 'token', 'apiKey', 'secret', 'authorization'];

    for (const key of Object.keys(sanitized)) {
      if (sensitiveKeys.some(sensitive => key.toLowerCase().includes(sensitive))) {
        sanitized[key] = '[REDACTED]';
      }
    }

    return sanitized;
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  private log(level: string, message: string, meta?: any): void {
    if (this.logger) {
      this.logger(level, message, meta);
    } else {
      console.log(`[${level.toUpperCase()}] ${message}`, meta);
    }
  }
}

// ============================================================================
// Query Builder Utilities
// ============================================================================

export class QueryBuilder {
  private fragments: string[] = [];
  private variables: Record<string, any> = {};

  /**
   * Add a GraphQL fragment
   */
  addFragment(name: string, type: string, fields: string): QueryBuilder {
    this.fragments.push(`fragment ${name} on ${type} { ${fields} }`);
    return this;
  }

  /**
   * Add variables
   */
  addVariables(variables: Record<string, any>): QueryBuilder {
    this.variables = { ...this.variables, ...variables };
    return this;
  }

  /**
   * Build a query string
   */
  buildQuery(
    operationName: string,
    variableDefinitions: string,
    selection: string
  ): string {
    const fragmentsStr = this.fragments.length > 0
      ? '\n\n' + this.fragments.join('\n\n')
      : '';

    return `query ${operationName}${variableDefinitions ? `(${variableDefinitions})` : ''} {
  ${selection}
}${fragmentsStr}`;
  }

  /**
   * Build a mutation string
   */
  buildMutation(
    operationName: string,
    variableDefinitions: string,
    selection: string
  ): string {
    const fragmentsStr = this.fragments.length > 0
      ? '\n\n' + this.fragments.join('\n\n')
      : '';

    return `mutation ${operationName}${variableDefinitions ? `(${variableDefinitions})` : ''} {
  ${selection}
}${fragmentsStr}`;
  }

  /**
   * Get variables
   */
  getVariables(): Record<string, any> {
    return { ...this.variables };
  }

  /**
   * Reset builder
   */
  reset(): QueryBuilder {
    this.fragments = [];
    this.variables = {};
    return this;
  }
}

// ============================================================================
// Common GraphQL Queries
// ============================================================================

export const WORKFLOW_FRAGMENT = `
  fragment WorkflowInfo on Workflow {
    id
    name
    states
    activities {
      id
      name
      fromStates
      toState
      conditions
    }
    initialState
    createdAt
    updatedAt
  }
`;

export const RESOURCE_FRAGMENT = `
  fragment ResourceInfo on Resource {
    id
    workflowId
    state
    data
    metadata
    createdAt
    updatedAt
    history {
      timestamp
      activity
      fromState
      toState
      data
    }
  }
`;

export const ACTIVITY_FRAGMENT = `
  fragment ActivityInfo on Activity {
    id
    name
    fromStates
    toState
    conditions
    description
  }
`;
