/**
 * Function Manager for Circuit Breaker TypeScript SDK
 *
 * This file provides comprehensive function management functionality including
 * Docker-based function execution, container lifecycle management, function
 * orchestration, and advanced execution features.
 */

import {
  FunctionDefinition,
  FunctionConfig,
  ContainerConfig,
  ResourceLimits,
  EventTrigger,
  FunctionChain,
  FunctionResult,
  ExecutionStatus,
  ResourceUsage,
  RetryConfig,
  PaginationOptions,
  PaginatedResult,
} from "../core/types.js";
import {
  FunctionError,
  FunctionNotFoundError,
  FunctionExecutionError,
  FunctionTimeoutError,
  ContainerError,
  ErrorHandler,
} from "../core/errors.js";
import { GraphQLClient } from "../utils/graphql.js";
import { Logger, sanitizeForLogging } from "../utils/logger.js";

// ============================================================================
// Logger Context Type
// ============================================================================

interface LogContext {
  operation?: string;
  functionId?: string;
  executionId?: string;
  containerId?: string;
  requestId?: string;
  component?: string;
  userId?: string;
  correlationId?: string;
}

// ============================================================================
// Extended Types
// ============================================================================

export interface FunctionCreateInput {
  name: string;
  container: ContainerConfig;
  triggers?: EventTrigger[];
  chains?: FunctionChain[];
  description?: string;
  tags?: string[];
  metadata?: Record<string, any>;
  version?: string;
  enabled?: boolean;
  inputSchema?: any;
  outputSchema?: any;
}

export interface FunctionUpdateInput {
  name?: string;
  container?: Partial<ContainerConfig>;
  triggers?: EventTrigger[];
  chains?: FunctionChain[];
  description?: string;
  tags?: string[];
  metadata?: Record<string, any>;
  version?: string;
  enabled?: boolean;
  inputSchema?: any;
  outputSchema?: any;
}

export interface FunctionSearchOptions extends PaginationOptions {
  /** Search in function names and descriptions */
  query?: string;

  /** Filter by tags */
  tags?: string[];

  /** Filter by enabled status */
  enabled?: boolean;

  /** Filter by image name */
  image?: string;

  /** Filter by creation date range */
  createdAfter?: Date;
  createdBefore?: Date;

  /** Include execution statistics */
  includeStats?: boolean;

  /** Include container status */
  includeContainerStatus?: boolean;

  /** Sort field */
  sortBy?: string;

  /** Sort direction */
  sortDirection?: "asc" | "desc";
}

export interface FunctionExecuteInput {
  /** Function ID to execute */
  functionId: string;

  /** Input data for the function */
  input?: any;

  /** Execution timeout in milliseconds */
  timeout?: number;

  /** Environment variables override */
  environment?: Record<string, string>;

  /** Resource limits override */
  resourceLimits?: ResourceLimits;

  /** Execution metadata */
  metadata?: Record<string, any>;

  /** Whether to wait for completion */
  wait?: boolean;

  /** Retry configuration */
  retry?: RetryConfig;
}

export interface FunctionStats {
  /** Total number of functions */
  totalFunctions: number;

  /** Functions by status */
  byStatus: Record<string, number>;

  /** Functions by image */
  byImage: Record<string, number>;

  /** Enabled vs disabled functions */
  enabled: number;
  disabled: number;

  /** Total executions */
  totalExecutions: number;

  /** Recent executions (last 24h) */
  recentExecutions: number;

  /** Average execution time */
  averageExecutionTime: number;

  /** Success rate percentage */
  successRate: number;

  /** Most executed functions */
  mostExecuted: { function: FunctionDefinition; executions: number }[];

  /** Resource usage statistics */
  resourceUsage: {
    totalCpu: number;
    totalMemory: number;
    totalStorage: number;
  };
}

export interface FunctionWithStats extends FunctionDefinition {
  createdAt: string;
  updatedAt: string;
  stats?: {
    executions: number;
    successRate: number;
    averageExecutionTime: number;
    lastExecution?: Date;
    resourceUsage?: ResourceUsage;
  };
  containerStatus?: ContainerStatus;
}

export interface ContainerStatus {
  status: "running" | "stopped" | "error" | "unknown";
  containerId?: string;
  startedAt?: Date;
  resourceUsage?: ResourceUsage;
  logs?: string[];
  error?: string;
}

export interface BatchExecutionInput {
  /** Function executions to perform */
  executions: FunctionExecuteInput[];

  /** Batch execution options */
  options?: {
    /** Maximum concurrent executions */
    concurrency?: number;
    /** Whether to continue on individual failures */
    continueOnError?: boolean;
    /** Overall batch timeout */
    batchTimeout?: number;
  };
}

export interface BatchExecutionResult {
  /** Overall success status */
  success: boolean;

  /** Individual execution results */
  results: {
    success: boolean;
    result?: FunctionResult;
    error?: Error;
    executionInput: FunctionExecuteInput;
  }[];

  /** Number of successful executions */
  successful: number;

  /** Number of failed executions */
  failed: number;

  /** Total batch execution time */
  totalTime: number;
}

export interface FunctionHealthStatus {
  healthy: boolean;
  issues: string[];
  lastCheck: Date;
  failingFunctions: number;
  containerErrors: number;
  resourceUtilization: {
    cpu: number;
    memory: number;
    storage: number;
  };
  averageResponseTime: number;
}

export interface ContainerLogOptions {
  /** Number of log lines to retrieve */
  lines?: number;
  /** Retrieve logs since timestamp */
  since?: Date;
  /** Follow log stream */
  follow?: boolean;
  /** Include timestamps */
  timestamps?: boolean;
}

// ============================================================================
// Function Manager
// ============================================================================

export class FunctionManager {
  private readonly graphqlClient: GraphQLClient;
  private readonly logger: Logger;
  private readonly config: FunctionConfig;
  private readonly cache = new Map<string, FunctionWithStats>();
  private readonly executionCache = new Map<string, FunctionResult>();
  private readonly containerCache = new Map<string, ContainerStatus>();
  private readonly cacheTimeout = 5 * 60 * 1000; // 5 minutes
  private readonly maxCacheSize = 500;

  constructor(
    graphqlClient: GraphQLClient,
    logger: Logger,
    config: FunctionConfig = {},
  ) {
    this.graphqlClient = graphqlClient;
    this.logger = logger;
    this.config = {
      executionTimeout: 300000, // 5 minutes
      maxConcurrency: 10,
      enableCaching: true,
      ...config,
    };
  }

  // ============================================================================
  // Core CRUD Operations
  // ============================================================================

  /**
   * Create a new function
   */
  async create(
    input: FunctionCreateInput,
    options: { validate?: boolean; build?: boolean } = {},
  ): Promise<FunctionDefinition> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_create",
      functionId: input.name,
      requestId,
    };

    this.logger.info("Creating new function", context);

    try {
      // Validate input
      if (options.validate !== false) {
        await this.validateFunctionInput(input);
      }

      // Create function via GraphQL
      const mutation = `
        mutation CreateFunction($input: FunctionCreateInput!) {
          createFunction(input: $input) {
            id
            name
            container {
              image
              command
              environment
              resources {
                cpu
                memory
                gpu
                disk
              }
              mounts {
                source
                target
                type
                readonly
              }
              workingDir
              networkMode
              labels
            }
            triggers {
              type
              condition
              inputMapping
              filters
              enabled
            }
            chains {
              targetFunction
              condition
              inputMapping
              delay
              description
            }
            description
            tags
            metadata
            version
            enabled
            inputSchema
            outputSchema
            createdAt
            updatedAt
          }
        }
      `;

      const variables = {
        input: {
          name: input.name,
          container: input.container,
          triggers: input.triggers || [],
          chains: input.chains || [],
          description: input.description,
          tags: input.tags || [],
          metadata: input.metadata || {},
          version: input.version || "1.0.0",
          enabled: input.enabled !== false,
          inputSchema: input.inputSchema,
          outputSchema: input.outputSchema,
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const functionDef = response.createFunction;

      // Cache the function
      this.setFunctionCache(functionDef.id, functionDef);

      // Build container image if requested
      if (options.build) {
        await this.buildContainer(functionDef.id);
      }

      this.logger.info("Function created successfully", {
        ...context,
        functionId: functionDef.id,
        image: functionDef.container.image,
        enabled: functionDef.enabled,
      });

      return functionDef;
    } catch (error) {
      this.logger.error("Failed to create function", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get a function by ID
   */
  async get(
    functionId: string,
    options: {
      includeStats?: boolean;
      includeContainerStatus?: boolean;
      useCache?: boolean;
    } = {},
  ): Promise<FunctionWithStats> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_get",
      functionId,
      requestId,
    };

    this.logger.debug("Getting function", context);

    try {
      // Check cache first
      if (options.useCache !== false) {
        const cached = this.getFunctionCache(functionId);
        if (cached) {
          this.logger.debug("Function found in cache", context);
          return cached;
        }
      }

      // Fetch from API
      const statsFields = options.includeStats
        ? `stats { executions, successRate, averageExecutionTime, lastExecution, resourceUsage { cpu, memory, network { bytesIn, bytesOut }, disk { bytesRead, bytesWritten } } }`
        : "";

      const containerFields = options.includeContainerStatus
        ? `containerStatus { status, containerId, startedAt, resourceUsage { cpu, memory, network { bytesIn, bytesOut }, disk { bytesRead, bytesWritten } }, error }`
        : "";

      const query = `
        query GetFunction($id: ID!) {
          function(id: $id) {
            id
            name
            container {
              image
              command
              environment
              resources {
                cpu
                memory
                gpu
                disk
              }
              mounts {
                source
                target
                type
                readonly
              }
              workingDir
              networkMode
              labels
            }
            triggers {
              type
              condition
              inputMapping
              filters
              enabled
            }
            chains {
              targetFunction
              condition
              inputMapping
              delay
              description
            }
            description
            tags
            metadata
            version
            enabled
            inputSchema
            outputSchema
            createdAt
            updatedAt
            ${statsFields}
            ${containerFields}
          }
        }
      `;

      const variables = { id: functionId };
      const response = await this.graphqlClient.request(query, variables);

      if (!response.function) {
        throw new FunctionNotFoundError(functionId, requestId);
      }

      const functionDef = response.function;

      // Cache the function
      this.setFunctionCache(functionId, functionDef);

      this.logger.debug("Function retrieved successfully", {
        ...context,
        name: functionDef.name,
        image: functionDef.container.image,
      });

      return functionDef;
    } catch (error) {
      this.logger.error("Failed to get function", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Update a function
   */
  async update(
    functionId: string,
    input: FunctionUpdateInput,
    options: { validate?: boolean; rebuild?: boolean } = {},
  ): Promise<FunctionDefinition> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_update",
      functionId,
      requestId,
    };

    this.logger.info("Updating function", context);

    try {
      // Get existing function
      const existingFunction = await this.get(functionId, { useCache: false });

      if (options.validate !== false) {
        await this.validateFunctionUpdate(existingFunction, input);
      }

      // Update via GraphQL
      const mutation = `
        mutation UpdateFunction($id: ID!, $input: FunctionUpdateInput!) {
          updateFunction(id: $id, input: $input) {
            id
            name
            container {
              image
              command
              environment
              resources {
                cpu
                memory
                gpu
                disk
              }
              mounts {
                source
                target
                type
                readonly
              }
              workingDir
              networkMode
              labels
            }
            triggers {
              type
              condition
              inputMapping
              filters
              enabled
            }
            chains {
              targetFunction
              condition
              inputMapping
              delay
              description
            }
            description
            tags
            metadata
            version
            enabled
            inputSchema
            outputSchema
            createdAt
            updatedAt
          }
        }
      `;

      const variables = {
        id: functionId,
        input: {
          name: input.name,
          container: input.container,
          triggers: input.triggers,
          chains: input.chains,
          description: input.description,
          tags: input.tags,
          metadata: input.metadata,
          version: input.version,
          enabled: input.enabled,
          inputSchema: input.inputSchema,
          outputSchema: input.outputSchema,
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const functionDef = response.updateFunction;

      // Update cache
      this.setFunctionCache(functionId, functionDef);

      // Rebuild container if requested and image changed
      if (options.rebuild && input.container?.image) {
        await this.buildContainer(functionId);
      }

      this.logger.info("Function updated successfully", {
        ...context,
        name: functionDef.name,
        version: functionDef.version,
      });

      return functionDef;
    } catch (error) {
      this.logger.error("Failed to update function", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Delete a function
   */
  async delete(
    functionId: string,
    options: { force?: boolean; removeContainer?: boolean } = {},
  ): Promise<boolean> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_delete",
      functionId,
      requestId,
    };

    this.logger.info("Deleting function", context);

    try {
      // Stop and remove container if requested
      if (options.removeContainer) {
        try {
          await this.stopContainer(functionId);
          await this.removeContainer(functionId);
        } catch (error) {
          this.logger.warn(
            "Failed to remove container during function deletion",
            {
              ...context,
              error,
            },
          );
          if (!options.force) {
            throw error;
          }
        }
      }

      // Delete via GraphQL
      const mutation = `
        mutation DeleteFunction($id: ID!, $force: Boolean) {
          deleteFunction(id: $id, force: $force) {
            success
          }
        }
      `;

      const variables = { id: functionId, force: options.force };
      const response = await this.graphqlClient.request(mutation, variables);

      if (response.deleteFunction.success) {
        // Remove from caches
        this.cache.delete(functionId);
        this.containerCache.delete(functionId);

        // Clear execution cache for this function
        for (const [key, value] of this.executionCache.entries()) {
          if (value.functionId === functionId) {
            this.executionCache.delete(key);
          }
        }

        this.logger.info("Function deleted successfully", context);
        return true;
      }

      return false;
    } catch (error) {
      this.logger.error("Failed to delete function", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Search and List Operations
  // ============================================================================

  /**
   * List functions with pagination and filtering
   */
  async list(
    options: FunctionSearchOptions = {},
  ): Promise<PaginatedResult<FunctionWithStats>> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_list",
      requestId,
    };

    this.logger.debug("Listing functions", {
      ...context,
      options: sanitizeForLogging(options),
    });

    try {
      const args: any = {
        limit: options.limit || 50,
        offset: options.offset || 0,
      };

      // Add filters
      const filters = this.buildFilters(options);
      if (Object.keys(filters).length > 0) {
        args.filters = filters;
      }

      // Add sorting
      if (options.sortBy) {
        args.sortBy = options.sortBy;
        args.sortDirection = options.sortDirection || "asc";
      }

      const statsFields = options.includeStats
        ? `stats { executions, successRate, averageExecutionTime, lastExecution }`
        : "";

      const containerFields = options.includeContainerStatus
        ? `containerStatus { status, containerId, startedAt, error }`
        : "";

      const query = `
        query ListFunctions($limit: Int, $offset: Int, $filters: FunctionFilters, $sortBy: String, $sortDirection: String) {
          functions(limit: $limit, offset: $offset, filters: $filters, sortBy: $sortBy, sortDirection: $sortDirection) {
            items {
              id
              name
              container {
                image
                command
                environment
                resources {
                  cpu
                  memory
                  gpu
                  disk
                }
              }
              triggers {
                type
                condition
                enabled
              }
              description
              tags
              metadata
              version
              enabled
              createdAt
              updatedAt
              ${statsFields}
              ${containerFields}
            }
            totalCount
            hasMore
          }
        }
      `;

      const response = await this.graphqlClient.request(query, args);
      const result = response.functions;

      this.logger.debug("Functions listed successfully", {
        ...context,
        count: result.items.length,
        totalCount: result.totalCount,
      });

      return {
        data: result.items,
        totalCount: result.totalCount,
        hasMore: result.hasMore,
        limit: options.limit || 50,
        offset: options.offset || 0,
      };
    } catch (error) {
      this.logger.error("Failed to list functions", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Search functions (alias for list with search-specific defaults)
   */
  async search(
    options: FunctionSearchOptions,
  ): Promise<PaginatedResult<FunctionWithStats>> {
    return this.list({
      ...options,
      includeStats: options.includeStats !== false,
    });
  }

  // ============================================================================
  // Function Execution
  // ============================================================================

  /**
   * Execute a function
   */
  async execute(input: FunctionExecuteInput): Promise<FunctionResult> {
    const requestId = this.generateRequestId();
    const executionId = this.generateExecutionId();
    const context: LogContext = {
      operation: "function_execute",
      functionId: input.functionId,
      executionId,
      requestId,
    };

    this.logger.info("Executing function", context);

    const startTime = Date.now();

    try {
      // Get function definition
      const functionDef = await this.get(input.functionId);

      if (!functionDef.enabled) {
        throw new FunctionExecutionError(
          input.functionId,
          "Function is disabled",
          executionId,
          undefined,
          requestId,
        );
      }

      // Execute function via GraphQL
      const mutation = `
        mutation ExecuteFunction($input: FunctionExecuteInput!) {
          executeFunction(input: $input) {
            executionId
            functionId
            status
            output
            logs
            executionTime
            startedAt
            completedAt
            error
            resourceUsage {
              cpu
              memory
              network {
                bytesIn
                bytesOut
              }
              disk {
                bytesRead
                bytesWritten
              }
            }
          }
        }
      `;

      const variables = {
        input: {
          functionId: input.functionId,
          input: input.input,
          timeout: input.timeout || this.config.executionTimeout,
          environment: input.environment,
          resourceLimits: input.resourceLimits,
          metadata: {
            ...input.metadata,
            executionId,
            requestId,
          },
          wait: input.wait !== false,
          retry: input.retry,
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const result = response.executeFunction;

      // Cache the execution result
      this.setExecutionCache(executionId, result);

      // Execute function chains if successful
      if (result.status === "success" && functionDef.chains?.length) {
        await this.executeChains(functionDef.chains, result);
      }

      const duration = Date.now() - startTime;

      this.logger.info("Function executed successfully", {
        ...context,
        status: result.status,
        executionTime: result.executionTime,
        totalTime: duration,
      });

      return result;
    } catch (error) {
      this.logger.error("Failed to execute function", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Execute multiple functions in batch
   */
  async executeBatch(
    input: BatchExecutionInput,
  ): Promise<BatchExecutionResult> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_execute_batch",
      requestId,
    };

    this.logger.info("Executing functions in batch", {
      ...context,
      count: input.executions.length,
    });

    const startTime = Date.now();
    const results: {
      success: boolean;
      result?: FunctionResult;
      error?: Error;
      executionInput: FunctionExecuteInput;
    }[] = [];
    const concurrency = input.options?.concurrency || 3;
    const continueOnError = input.options?.continueOnError !== false;

    try {
      // Process in chunks
      for (let i = 0; i < input.executions.length; i += concurrency) {
        const chunk = input.executions.slice(i, i + concurrency);
        const chunkPromises = chunk.map(async (execution) => {
          try {
            const result = await this.execute(execution);
            return { success: true, result, executionInput: execution };
          } catch (error) {
            if (!continueOnError) {
              throw error;
            }
            return {
              success: false,
              error: error as Error,
              executionInput: execution,
            };
          }
        });

        const chunkResults = await Promise.all(chunkPromises);
        results.push(...chunkResults);
      }

      const successful = results.filter((r) => r.success).length;
      const failed = results.filter((r) => !r.success).length;
      const totalTime = Date.now() - startTime;

      this.logger.info("Batch function execution completed", {
        ...context,
        successful,
        failed,
        totalTime,
      });

      return {
        success: failed === 0,
        results,
        successful,
        failed,
        totalTime,
      };
    } catch (error) {
      this.logger.error("Batch function execution failed", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get function execution result
   */
  async getExecution(executionId: string): Promise<FunctionResult> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_get_execution",
      executionId,
      requestId,
    };

    this.logger.debug("Getting function execution", context);

    try {
      // Check cache first
      const cached = this.getExecutionCache(executionId);
      if (cached) {
        return cached;
      }

      // Fetch from API
      const query = `
        query GetExecution($id: ID!) {
          execution(id: $id) {
            executionId
            functionId
            status
            output
            logs
            executionTime
            startedAt
            completedAt
            error
            resourceUsage {
              cpu
              memory
              network {
                bytesIn
                bytesOut
              }
              disk {
                bytesRead
                bytesWritten
              }
            }
          }
        }
      `;

      const response = await this.graphqlClient.request(query, {
        id: executionId,
      });

      if (!response.execution) {
        throw new FunctionExecutionError(
          "unknown",
          `Execution not found: ${executionId}`,
          executionId,
          undefined,
          requestId,
        );
      }

      const result = response.execution;
      this.setExecutionCache(executionId, result);

      return result;
    } catch (error) {
      this.logger.error("Failed to get function execution", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Cancel a running function execution
   */
  async cancelExecution(executionId: string): Promise<boolean> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_cancel_execution",
      executionId,
      requestId,
    };

    this.logger.info("Cancelling function execution", context);

    try {
      const mutation = `
        mutation CancelExecution($id: ID!) {
          cancelExecution(id: $id) {
            success
          }
        }
      `;

      const response = await this.graphqlClient.request(mutation, {
        id: executionId,
      });

      if (response.cancelExecution.success) {
        // Remove from cache to force refresh
        this.executionCache.delete(executionId);
        this.logger.info("Function execution cancelled successfully", context);
        return true;
      }

      return false;
    } catch (error) {
      this.logger.error("Failed to cancel function execution", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Container Management
  // ============================================================================

  /**
   * Build container image for function
   */
  async buildContainer(functionId: string): Promise<string> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "container_build",
      functionId,
      requestId,
    };

    this.logger.info("Building container image", context);

    try {
      const mutation = `
        mutation BuildContainer($functionId: ID!) {
          buildContainer(functionId: $functionId) {
            buildId
            status
            imageId
            logs
          }
        }
      `;

      const response = await this.graphqlClient.request(mutation, {
        functionId,
      });
      const buildResult = response.buildContainer;

      this.logger.info("Container build initiated", {
        ...context,
        buildId: buildResult.buildId,
        status: buildResult.status,
      });

      return buildResult.buildId;
    } catch (error) {
      this.logger.error("Failed to build container", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Start container for function
   */
  async startContainer(functionId: string): Promise<string> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "container_start",
      functionId,
      requestId,
    };

    this.logger.info("Starting container", context);

    try {
      const mutation = `
        mutation StartContainer($functionId: ID!) {
          startContainer(functionId: $functionId) {
            containerId
            status
          }
        }
      `;

      const response = await this.graphqlClient.request(mutation, {
        functionId,
      });
      const result = response.startContainer;

      // Update container cache
      this.setContainerCache(functionId, {
        status: "running",
        containerId: result.containerId,
        startedAt: new Date(),
      });

      this.logger.info("Container started successfully", {
        ...context,
        containerId: result.containerId,
      });

      return result.containerId;
    } catch (error) {
      this.logger.error("Failed to start container", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Stop container for function
   */
  async stopContainer(functionId: string): Promise<boolean> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "container_stop",
      functionId,
      requestId,
    };

    this.logger.info("Stopping container", context);

    try {
      const mutation = `
        mutation StopContainer($functionId: ID!) {
          stopContainer(functionId: $functionId) {
            success
          }
        }
      `;

      const response = await this.graphqlClient.request(mutation, {
        functionId,
      });

      if (response.stopContainer.success) {
        // Update container cache
        this.setContainerCache(functionId, {
          status: "stopped",
        });

        this.logger.info("Container stopped successfully", context);
        return true;
      }

      return false;
    } catch (error) {
      this.logger.error("Failed to stop container", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Remove container for function
   */
  async removeContainer(functionId: string): Promise<boolean> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "container_remove",
      functionId,
      requestId,
    };

    this.logger.info("Removing container", context);

    try {
      const mutation = `
        mutation RemoveContainer($functionId: ID!) {
          removeContainer(functionId: $functionId) {
            success
          }
        }
      `;

      const response = await this.graphqlClient.request(mutation, {
        functionId,
      });

      if (response.removeContainer.success) {
        // Remove from container cache
        this.containerCache.delete(functionId);

        this.logger.info("Container removed successfully", context);
        return true;
      }

      return false;
    } catch (error) {
      this.logger.error("Failed to remove container", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get container logs
   */
  async getContainerLogs(
    functionId: string,
    options: ContainerLogOptions = {},
  ): Promise<string[]> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "container_logs",
      functionId,
      requestId,
    };

    this.logger.debug("Getting container logs", context);

    try {
      const query = `
        query GetContainerLogs($functionId: ID!, $options: ContainerLogOptions) {
          containerLogs(functionId: $functionId, options: $options) {
            logs
          }
        }
      `;

      const response = await this.graphqlClient.request(query, {
        functionId,
        options: {
          lines: options.lines || 100,
          since: options.since?.toISOString(),
          follow: options.follow || false,
          timestamps: options.timestamps || false,
        },
      });

      return response.containerLogs.logs;
    } catch (error) {
      this.logger.error("Failed to get container logs", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get container status
   */
  async getContainerStatus(functionId: string): Promise<ContainerStatus> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "container_status",
      functionId,
      requestId,
    };

    this.logger.debug("Getting container status", context);

    try {
      // Check cache first
      const cached = this.getContainerCache(functionId);
      if (cached) {
        return cached;
      }

      const query = `
        query GetContainerStatus($functionId: ID!) {
          containerStatus(functionId: $functionId) {
            status
            containerId
            startedAt
            resourceUsage {
              cpu
              memory
              network {
                bytesIn
                bytesOut
              }
              disk {
                bytesRead
                bytesWritten
              }
            }
            error
          }
        }
      `;

      const response = await this.graphqlClient.request(query, { functionId });
      const status = response.containerStatus;

      // Convert dates
      if (status.startedAt) {
        status.startedAt = new Date(status.startedAt);
      }

      // Cache the status
      this.setContainerCache(functionId, status);

      return status;
    } catch (error) {
      this.logger.error("Failed to get container status", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Analytics and Monitoring
  // ============================================================================

  /**
   * Get function statistics
   */
  async getStats(): Promise<FunctionStats> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_stats",
      requestId,
    };

    this.logger.debug("Getting function statistics", context);

    try {
      const query = `
        query FunctionStats {
          functionStats {
            totalFunctions
            byStatus
            byImage
            enabled
            disabled
            totalExecutions
            recentExecutions
            averageExecutionTime
            successRate
            mostExecuted {
              function {
                id
                name
                container {
                  image
                }
              }
              executions
            }
            resourceUsage {
              totalCpu
              totalMemory
              totalStorage
            }
          }
        }
      `;

      const response = await this.graphqlClient.request(query);
      const stats = response.functionStats;

      this.logger.debug("Function statistics retrieved", {
        ...context,
        totalFunctions: stats.totalFunctions,
        totalExecutions: stats.totalExecutions,
      });

      return stats;
    } catch (error) {
      this.logger.error("Failed to get function statistics", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get function health status
   */
  async getHealth(): Promise<FunctionHealthStatus> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "function_health",
      requestId,
    };

    this.logger.debug("Getting function health status", context);

    try {
      const query = `
        query FunctionHealth {
          functionHealth {
            healthy
            issues
            lastCheck
            failingFunctions
            containerErrors
            resourceUtilization {
              cpu
              memory
              storage
            }
            averageResponseTime
          }
        }
      `;

      const response = await this.graphqlClient.request(query);
      const health = response.functionHealth;

      this.logger.debug("Function health status retrieved", {
        ...context,
        healthy: health.healthy,
        issues: health.issues.length,
      });

      return {
        ...health,
        lastCheck: new Date(health.lastCheck),
      };
    } catch (error) {
      this.logger.error("Failed to get function health", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Function Chains
  // ============================================================================

  /**
   * Execute function chains
   */
  private async executeChains(
    chains: FunctionChain[],
    triggerResult: FunctionResult,
  ): Promise<void> {
    const context: LogContext = {
      operation: "function_chains",
      functionId: triggerResult.functionId,
      executionId: triggerResult.executionId,
    };

    this.logger.debug("Executing function chains", {
      ...context,
      chainCount: chains.length,
    });

    for (const chain of chains) {
      try {
        // Check chain condition
        let shouldExecute = false;
        switch (chain.condition) {
          case "always":
            shouldExecute = true;
            break;
          case "success":
            shouldExecute = triggerResult.status === "success";
            break;
          case "failure":
            shouldExecute = triggerResult.status === "failure";
            break;
          case "custom":
            // Custom condition evaluation would go here
            shouldExecute = true;
            break;
        }

        if (!shouldExecute) {
          continue;
        }

        // Map input data
        let chainInput: any;
        switch (chain.inputMapping) {
          case "full_data":
            chainInput = triggerResult.output;
            break;
          case "metadata_only":
            chainInput = { executionId: triggerResult.executionId };
            break;
          default:
            chainInput = triggerResult.output;
        }

        // Add delay if specified
        if (chain.delay) {
          await new Promise((resolve) => setTimeout(resolve, chain.delay));
        }

        // Execute chained function
        await this.execute({
          functionId: chain.targetFunction,
          input: chainInput,
          metadata: {
            chainedFrom: triggerResult.functionId,
            chainedExecutionId: triggerResult.executionId,
          },
        });
      } catch (error) {
        this.logger.warn("Chain execution failed", {
          ...context,
          targetFunction: chain.targetFunction,
          error,
        });
        // Continue with other chains
      }
    }
  }

  // ============================================================================
  // Manager Lifecycle
  // ============================================================================

  /**
   * Initialize the function manager
   */
  async initialize(): Promise<void> {
    const context: LogContext = { operation: "function_manager_init" };
    this.logger.info("Initializing FunctionManager", context);

    try {
      // Perform initialization tasks
      this.logger.info("FunctionManager initialized successfully", context);
    } catch (error) {
      this.logger.error("Failed to initialize FunctionManager", context, error);
      throw error;
    }
  }

  /**
   * Reset the manager (clear caches, etc.)
   */
  async reset(): Promise<void> {
    this.cache.clear();
    this.executionCache.clear();
    this.containerCache.clear();
  }

  /**
   * Dispose of the manager and clean up resources
   */
  async dispose(): Promise<void> {
    this.cache.clear();
    this.executionCache.clear();
    this.containerCache.clear();
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  /**
   * Validate function input
   */
  private async validateFunctionInput(
    input: FunctionCreateInput,
  ): Promise<void> {
    if (!input.name) {
      throw new FunctionError(
        "Function name is required",
        "MISSING_FUNCTION_NAME",
        { input },
      );
    }

    if (!input.container?.image) {
      throw new FunctionError(
        "Container image is required",
        "MISSING_CONTAINER_IMAGE",
        { input },
      );
    }
  }

  /**
   * Validate function update
   */
  private async validateFunctionUpdate(
    existingFunction: FunctionDefinition,
    input: FunctionUpdateInput,
  ): Promise<void> {
    if (input.name && input.name !== existingFunction.name) {
      try {
        await this.get(input.name, { useCache: false });
        throw new FunctionError(
          `Function with name '${input.name}' already exists`,
          "FUNCTION_NAME_EXISTS",
          { input },
        );
      } catch (error) {
        if (!(error instanceof FunctionNotFoundError)) {
          throw error;
        }
      }
    }
  }

  /**
   * Build filters for function queries
   */
  private buildFilters(options: FunctionSearchOptions): Record<string, any> {
    const filters: Record<string, any> = {};

    if (options.query) {
      filters.search = options.query;
    }

    if (options.tags && options.tags.length > 0) {
      filters.tags = options.tags;
    }

    if (options.enabled !== undefined) {
      filters.enabled = options.enabled;
    }

    if (options.image) {
      filters.image = options.image;
    }

    if (options.createdAfter) {
      filters.createdAfter = options.createdAfter.toISOString();
    }

    if (options.createdBefore) {
      filters.createdBefore = options.createdBefore.toISOString();
    }

    return filters;
  }

  /**
   * Get function from cache
   */
  private getFunctionCache(functionId: string): FunctionWithStats | undefined {
    return this.cache.get(functionId);
  }

  /**
   * Set function in cache
   */
  private setFunctionCache(
    functionId: string,
    functionDef: FunctionWithStats,
  ): void {
    this.cache.set(functionId, functionDef);

    if (this.cache.size > this.maxCacheSize) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }
  }

  /**
   * Get execution from cache
   */
  private getExecutionCache(executionId: string): FunctionResult | undefined {
    return this.executionCache.get(executionId);
  }

  /**
   * Set execution in cache
   */
  private setExecutionCache(executionId: string, result: FunctionResult): void {
    this.executionCache.set(executionId, result);

    if (this.executionCache.size > this.maxCacheSize) {
      const firstKey = this.executionCache.keys().next().value;
      if (firstKey) {
        this.executionCache.delete(firstKey);
      }
    }

    setTimeout(() => {
      this.executionCache.delete(executionId);
    }, this.cacheTimeout);
  }

  /**
   * Get container status from cache
   */
  private getContainerCache(functionId: string): ContainerStatus | undefined {
    return this.containerCache.get(functionId);
  }

  /**
   * Set container status in cache
   */
  private setContainerCache(functionId: string, status: ContainerStatus): void {
    this.containerCache.set(functionId, status);

    setTimeout(() => {
      this.containerCache.delete(functionId);
    }, this.cacheTimeout);
  }

  /**
   * Generate a unique request ID for tracking
   */
  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Generate a unique execution ID for tracking
   */
  private generateExecutionId(): string {
    return `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new FunctionManager instance
 */
export function createFunctionManager(
  graphqlClient: GraphQLClient,
  logger: Logger,
  config?: FunctionConfig,
): FunctionManager {
  return new FunctionManager(graphqlClient, logger, config);
}

/**
 * Validate a function definition
 */
export function validateFunctionDefinition(functionDef: FunctionDefinition): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  if (!functionDef.name) {
    errors.push("Function name is required");
  }

  if (!functionDef.container?.image) {
    errors.push("Container image is required");
  }

  if (functionDef.container) {
    const container = functionDef.container;

    if (container.resources) {
      if (container.resources.cpu && container.resources.cpu <= 0) {
        errors.push("CPU limit must be positive");
      }
      if (container.resources.memory && container.resources.memory <= 0) {
        errors.push("Memory limit must be positive");
      }
    }

    if (container.mounts) {
      for (const mount of container.mounts) {
        if (!mount.source || !mount.target) {
          errors.push("Mount source and target are required");
        }
      }
    }
  }

  if (functionDef.triggers) {
    for (const trigger of functionDef.triggers) {
      if (!trigger.type || !trigger.condition) {
        errors.push("Trigger type and condition are required");
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}
