/**
 * Resource Manager for Circuit Breaker TypeScript SDK
 *
 * This file provides comprehensive resource management functionality including
 * CRUD operations, state transitions, activity execution, and resource lifecycle
 * management through the GraphQL API.
 */

import {
  Resource,
  ResourceCreateInput,
  ActivityExecuteInput,
  HistoryEvent,
  WorkflowDefinition,
  PaginationOptions,
  PaginatedResult,
} from "../core/types.js";
import {
  ResourceNotFoundError,
  ResourceValidationError,
  StateTransitionError,
  ActivityExecutionError,
  ErrorHandler,
} from "../core/errors.js";
import { GraphQLClient } from "../utils/graphql.js";
import { Logger, sanitizeForLogging } from "../utils/logger.js";

// ============================================================================
// Logger Context Type
// ============================================================================

interface LogContext {
  operation?: string;
  resourceId?: string;
  workflowId?: string;
  requestId?: string;
  component?: string;
  userId?: string;
  correlationId?: string;
}

// ============================================================================
// Types
// ============================================================================

export interface ResourceUpdateInput {
  data?: any;
  metadata?: Record<string, any>;
  // Note: State updates should use executeActivity or transitionState methods
}

export interface ResourceSearchOptions extends PaginationOptions {
  /** Search in resource data and metadata */
  query?: string;

  /** Filter by workflow ID */
  workflowId?: string;

  /** Filter by current state */
  state?: string;
  states?: string[];

  /** Filter by creation date range */
  createdAfter?: Date;
  createdBefore?: Date;

  /** Filter by last update date range */
  updatedAfter?: Date;
  updatedBefore?: Date;

  /** Include resource history */
  includeHistory?: boolean;

  /** Include workflow definition */
  includeWorkflow?: boolean;

  /** Filter by metadata fields */
  metadata?: Record<string, any>;

  /** Filter by data fields (simple equality) */
  dataFields?: Record<string, any>;

  /** Sort field */
  sortBy?: string;

  /** Sort direction */
  sortDirection?: "asc" | "desc";
}

export interface ResourceStats {
  /** Total number of resources */
  totalResources: number;

  /** Resources by state */
  byState: Record<string, number>;

  /** Resources by workflow */
  byWorkflow: Record<string, number>;

  /** Active resources (not in terminal states) */
  activeResources: number;

  /** Completed resources */
  completedResources: number;

  /** Failed resources */
  failedResources: number;

  /** Average resource age */
  averageAge: number;

  /** Recent activity count (last 24h) */
  recentActivity: number;

  /** Most active workflows */
  mostActiveWorkflows: { workflowId: string; count: number }[];
}

export interface ResourceWithWorkflow extends Resource {
  workflow?: WorkflowDefinition;
}

export interface StateTransitionInput {
  /** Resource ID to transition */
  resourceId: string;

  /** Target state */
  toState: string;

  /** Optional activity that triggered the transition */
  activityId?: string;

  /** Optional data to include with the transition */
  data?: any;

  /** Optional metadata for the transition */
  metadata?: Record<string, any>;

  /** Whether to validate the transition is allowed */
  validate?: boolean;
}

export interface StateTransitionResult {
  /** Whether the transition was successful */
  success: boolean;

  /** Updated resource */
  resource: Resource;

  /** New history event */
  historyEvent: HistoryEvent;

  /** Any validation warnings */
  warnings?: string[];
}

export interface ActivityExecutionResult {
  /** Whether the execution was successful */
  success: boolean;

  /** Updated resource */
  resource: Resource;

  /** Execution output data */
  output?: any;

  /** New history event */
  historyEvent: HistoryEvent;

  /** Execution duration in milliseconds */
  duration: number;

  /** Any warnings or messages */
  messages?: string[];
}

export interface BatchOperationOptions {
  /** Maximum number of operations to process concurrently */
  concurrency?: number;

  /** Whether to continue on individual failures */
  continueOnError?: boolean;

  /** Timeout for each individual operation */
  operationTimeout?: number;
}

export interface BatchOperationResult<T> {
  /** Number of successful operations */
  successful: number;

  /** Number of failed operations */
  failed: number;

  /** Total operations attempted */
  total: number;

  /** Individual results */
  results: { success: boolean; data?: T; error?: Error }[];

  /** Overall operation duration */
  duration: number;
}

export interface ResourceValidationOptions {
  /** Validate resource data against workflow schema */
  validateData?: boolean;

  /** Check if current state is valid for workflow */
  validateState?: boolean;

  /** Validate metadata structure */
  validateMetadata?: boolean;

  /** Check for data consistency */
  checkConsistency?: boolean;

  /** Include performance warnings */
  includeWarnings?: boolean;
}

export interface ResourceValidationReport {
  valid: boolean;
  errors: string[];
  warnings: string[];
  suggestions: string[];
  dataValid: boolean;
  stateValid: boolean;
  metadataValid: boolean;
  consistencyIssues: string[];
}

export interface ResourceHealthStatus {
  healthy: boolean;
  issues: string[];
  lastCheck: Date;
  stuckResources: number;
  errorRate: number;
  avgProcessingTime: number;
  oldestResource?: Date;
}

// ============================================================================
// Resource Manager
// ============================================================================

export class ResourceManager {
  private readonly graphqlClient: GraphQLClient;
  private readonly logger: Logger;
  private readonly cache = new Map<string, Resource>();
  private readonly workflowCache = new Map<string, WorkflowDefinition>();
  private readonly cacheTimeout = 5 * 60 * 1000; // 5 minutes
  private readonly maxCacheSize = 500;

  constructor(graphqlClient: GraphQLClient, logger: Logger) {
    this.graphqlClient = graphqlClient;
    this.logger = logger;
  }

  // ============================================================================
  // Core CRUD Operations
  // ============================================================================

  /**
   * Create a new resource
   */
  async create(
    input: ResourceCreateInput,
    options: { validate?: boolean } = {},
  ): Promise<Resource> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_create",
      workflowId: input.workflowId,
      requestId,
    };

    this.logger.info("Creating new resource", context);

    try {
      // Validate input
      await this.validateResourceInput(input);

      // Get workflow definition for validation
      const workflow = await this.getWorkflow(input.workflowId);

      if (options.validate !== false) {
        await this.validateResourceForWorkflow(input, workflow);
      }

      // Determine initial state
      const initialState = input.initialState || workflow.initialState;
      if (!workflow.states.includes(initialState)) {
        throw new ResourceValidationError(
          `Initial state '${initialState}' not found in workflow states`,
          "INVALID_INITIAL_STATE",
          { availableStates: workflow.states, requestedState: initialState },
          requestId,
        );
      }

      // Create resource via GraphQL
      const mutation = `
        mutation CreateResource($input: ResourceCreateInput!) {
          createResource(input: $input) {
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
              metadata
            }
          }
        }
      `;

      const variables = {
        input: {
          workflowId: input.workflowId,
          initialState,
          data: input.data,
          metadata: input.metadata || {},
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const resource = response.createResource;

      // Cache the resource
      this.setCacheEntry(resource.id, resource);

      this.logger.info("Resource created successfully", {
        ...context,
        resourceId: resource.id,
        state: resource.state,
      });

      return resource;
    } catch (error) {
      this.logger.error("Failed to create resource", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get a resource by ID
   */
  async get(
    resourceId: string,
    options: { includeWorkflow?: boolean; useCache?: boolean } = {},
  ): Promise<Resource> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_get",
      resourceId,
      requestId,
    };

    this.logger.debug("Getting resource", context);

    try {
      // Check cache first
      if (options.useCache !== false) {
        const cached = this.getCacheEntry(resourceId);
        if (cached) {
          this.logger.debug("Resource found in cache", context);
          return cached;
        }
      }

      // Fetch from API
      const workflowFields = options.includeWorkflow
        ? `workflow { id, name, states, initialState, activities { id, name, config } }`
        : "";

      const query = `
        query GetResource($id: ID!) {
          resource(id: $id) {
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
              metadata
            }
            ${workflowFields}
          }
        }
      `;

      const variables = { id: resourceId };

      const response = await this.graphqlClient.request(query, variables);

      if (!response.resource) {
        throw new ResourceNotFoundError(resourceId, requestId);
      }

      const resource = response.resource;

      // Cache the resource
      this.setCacheEntry(resourceId, resource);

      this.logger.debug("Resource retrieved successfully", {
        ...context,
        workflowId: resource.workflowId,
        state: resource.state,
      });

      return resource;
    } catch (error) {
      this.logger.error("Failed to get resource", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Update a resource
   */
  async update(
    resourceId: string,
    input: ResourceUpdateInput,
    options: { validate?: boolean } = {},
  ): Promise<Resource> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_update",
      resourceId,
      requestId,
    };

    this.logger.info("Updating resource", context);

    try {
      // Get existing resource
      const existingResource = await this.get(resourceId, { useCache: false });

      if (options.validate !== false) {
        await this.validateResourceUpdate(existingResource, input);
      }

      // Update via GraphQL
      const mutation = `
        mutation UpdateResource($id: ID!, $input: ResourceUpdateInput!) {
          updateResource(id: $id, input: $input) {
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
              metadata
            }
          }
        }
      `;

      const variables = {
        id: resourceId,
        input: {
          data: input.data,
          metadata: input.metadata,
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const resource = response.updateResource;

      // Update cache
      this.setCacheEntry(resourceId, resource);

      this.logger.info("Resource updated successfully", {
        ...context,
        workflowId: resource.workflowId,
        state: resource.state,
      });

      return resource;
    } catch (error) {
      this.logger.error("Failed to update resource", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Delete a resource
   */
  async delete(
    resourceId: string,
    options: { force?: boolean } = {},
  ): Promise<boolean> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_delete",
      resourceId,
      requestId,
    };

    this.logger.info("Deleting resource", context);

    try {
      // Get resource first to check state
      if (!options.force) {
        const resource = await this.get(resourceId, { useCache: false });
        const workflow = await this.getWorkflow(resource.workflowId);

        // Check if resource is in a terminal state
        const terminalStates = workflow.activities
          .filter((a) => a.name && a.name.includes("end"))
          .map((a) => a.name);

        if (!terminalStates.includes(resource.state)) {
          throw new ResourceValidationError(
            "Cannot delete resource that is not in a terminal state. Use force=true to override.",
            "RESOURCE_NOT_TERMINAL",
            { currentState: resource.state, terminalStates },
            requestId,
          );
        }
      }

      // Delete via GraphQL
      const mutation = `
        mutation DeleteResource($id: ID!, $force: Boolean) {
          deleteResource(id: $id, force: $force) {
            success
          }
        }
      `;

      const variables = { id: resourceId, force: options.force };

      const response = await this.graphqlClient.request(mutation, variables);

      if (response.deleteResource.success) {
        // Remove from cache
        this.cache.delete(resourceId);
        this.logger.info("Resource deleted successfully", context);
        return true;
      }

      return false;
    } catch (error) {
      this.logger.error("Failed to delete resource", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Search and List Operations
  // ============================================================================

  /**
   * List resources with pagination and filtering
   */
  async list(
    options: ResourceSearchOptions = {},
  ): Promise<PaginatedResult<ResourceWithWorkflow>> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_list",
      requestId,
    };

    this.logger.debug("Listing resources", {
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

      // Build query fields
      const historyFields = options.includeHistory
        ? `history { timestamp, activity, fromState, toState, data, metadata }`
        : "";

      const workflowFields = options.includeWorkflow
        ? `workflow { id, name, states, initialState, activities { id, name, config } }`
        : "";

      const query = `
        query ListResources($limit: Int, $offset: Int, $filters: ResourceFilters, $sortBy: String, $sortDirection: String) {
          resources(limit: $limit, offset: $offset, filters: $filters, sortBy: $sortBy, sortDirection: $sortDirection) {
            items {
              id
              workflowId
              state
              data
              metadata
              createdAt
              updatedAt
              ${historyFields}
              ${workflowFields}
            }
            totalCount
            hasMore
          }
        }
      `;

      const response = await this.graphqlClient.request(query, args);
      const result = response.resources;

      this.logger.debug("Resources listed successfully", {
        ...context,
        count: result.items.length,
        totalCount: result.totalCount,
      });

      return {
        items: result.items,
        totalCount: result.totalCount,
        hasMore: result.hasMore,
        limit: options.limit || 50,
        offset: options.offset || 0,
      };
    } catch (error) {
      this.logger.error("Failed to list resources", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Search resources (alias for list with search-specific defaults)
   */
  async search(
    options: ResourceSearchOptions,
  ): Promise<PaginatedResult<ResourceWithWorkflow>> {
    return this.list({
      ...options,
      includeWorkflow: options.includeWorkflow !== false,
    });
  }

  // ============================================================================
  // State Management
  // ============================================================================

  /**
   * Execute an activity on a resource
   */
  async executeActivity(
    input: ActivityExecuteInput,
  ): Promise<ActivityExecutionResult> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "activity_execute",
      resourceId: input.resourceId,
      requestId,
    };

    this.logger.info("Executing activity", context);

    const startTime = Date.now();

    try {
      // Get resource and workflow
      const resource = await this.get(input.resourceId, { useCache: false });
      const workflow = await this.getWorkflow(resource.workflowId);

      // Find activity
      const activity = workflow.activities.find(
        (a) => a.id === input.activityId,
      );
      if (!activity) {
        throw new ActivityExecutionError(
          `Activity '${input.activityId}' not found in workflow`,
          "ACTIVITY_NOT_FOUND",
          { activityId: input.activityId, workflowId: resource.workflowId },
          requestId,
        );
      }

      // Execute activity via GraphQL
      const mutation = `
        mutation ExecuteActivity($resourceId: ID!, $activityId: ID!, $data: JSON, $metadata: JSON) {
          executeActivity(resourceId: $resourceId, activityId: $activityId, data: $data, metadata: $metadata) {
            success
            output
            duration
            messages
            resource {
              id
              workflowId
              state
              data
              metadata
              createdAt
              updatedAt
            }
            historyEvent {
              timestamp
              activity
              fromState
              toState
              data
              metadata
            }
          }
        }
      `;

      const variables = {
        resourceId: input.resourceId,
        activityId: input.activityId,
        data: input.data,
        metadata: input.metadata,
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const result = response.executeActivity;

      // Update cache
      this.setCacheEntry(input.resourceId, result.resource);

      const duration = Date.now() - startTime;

      this.logger.info("Activity executed successfully", {
        ...context,
        success: result.success,
        duration: duration,
        newState: result.resource.state,
      });

      return {
        success: result.success,
        resource: result.resource,
        output: result.output,
        historyEvent: result.historyEvent,
        duration: duration,
        messages: result.messages,
      };
    } catch (error) {
      this.logger.error("Failed to execute activity", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Transition a resource to a new state
   */
  async transitionState(
    input: StateTransitionInput,
  ): Promise<StateTransitionResult> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "state_transition",
      resourceId: input.resourceId,
      requestId,
    };

    this.logger.info("Transitioning resource state", context);

    try {
      // Get resource and workflow for validation
      const resource = await this.get(input.resourceId, { useCache: false });
      const workflow = await this.getWorkflow(resource.workflowId);

      if (input.validate !== false) {
        // Validate target state exists
        if (!workflow.states.includes(input.toState)) {
          throw new StateTransitionError(
            `Target state '${input.toState}' not found in workflow states`,
            "INVALID_TARGET_STATE",
            {
              targetState: input.toState,
              availableStates: workflow.states,
              currentState: resource.state,
            },
            requestId,
          );
        }

        // Additional validation logic could go here
        // (e.g., checking if transition is allowed based on workflow rules)
      }

      // Perform transition via GraphQL
      const mutation = `
        mutation TransitionResourceState($resourceId: ID!, $toState: String!, $activityId: ID, $data: JSON, $metadata: JSON) {
          transitionResourceState(resourceId: $resourceId, toState: $toState, activityId: $activityId, data: $data, metadata: $metadata) {
            success
            warnings
            resource {
              id
              workflowId
              state
              data
              metadata
              createdAt
              updatedAt
            }
            historyEvent {
              timestamp
              activity
              fromState
              toState
              data
              metadata
            }
          }
        }
      `;

      const variables = {
        resourceId: input.resourceId,
        toState: input.toState,
        activityId: input.activityId,
        data: input.data,
        metadata: input.metadata,
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const result = response.transitionResourceState;

      // Update cache
      this.setCacheEntry(input.resourceId, result.resource);

      this.logger.info("State transition completed", {
        ...context,
        success: result.success,
        fromState: resource.state,
        toState: result.resource.state,
      });

      return {
        success: result.success,
        resource: result.resource,
        historyEvent: result.historyEvent,
        warnings: result.warnings,
      };
    } catch (error) {
      this.logger.error("Failed to transition state", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Batch Operations
  // ============================================================================

  /**
   * Create multiple resources in batch
   */
  async createBatch(
    inputs: ResourceCreateInput[],
    options: BatchOperationOptions = {},
  ): Promise<BatchOperationResult<Resource>> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_create_batch",
      requestId,
    };

    this.logger.info("Creating resources in batch", context);

    const startTime = Date.now();
    const results: { success: boolean; data?: Resource; error?: Error }[] = [];
    const concurrency = options.concurrency || 5;
    const continueOnError = options.continueOnError !== false;

    try {
      // Process in chunks
      for (let i = 0; i < inputs.length; i += concurrency) {
        const chunk = inputs.slice(i, i + concurrency);
        const chunkPromises = chunk.map(async (input) => {
          try {
            const resource = await this.create(input, { validate: true });
            return { success: true, data: resource };
          } catch (error) {
            if (!continueOnError) {
              throw error;
            }
            return { success: false, error: error as Error };
          }
        });

        const chunkResults = await Promise.all(chunkPromises);
        results.push(...chunkResults);
      }

      const successful = results.filter((r) => r.success).length;
      const failed = results.filter((r) => !r.success).length;
      const duration = Date.now() - startTime;

      this.logger.info("Batch resource creation completed", {
        ...context,
        successful,
        failed,
        duration,
      });

      return {
        successful,
        failed,
        total: inputs.length,
        results,
        duration,
      };
    } catch (error) {
      this.logger.error("Batch resource creation failed", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Execute activity on multiple resources in batch
   */
  async executeActivityBatch(
    inputs: ActivityExecuteInput[],
    options: BatchOperationOptions = {},
  ): Promise<BatchOperationResult<ActivityExecutionResult>> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "activity_execute_batch",
      requestId,
    };

    this.logger.info("Executing activities in batch", context);

    const startTime = Date.now();
    const results: {
      success: boolean;
      data?: ActivityExecutionResult;
      error?: Error;
    }[] = [];
    const concurrency = options.concurrency || 3;
    const continueOnError = options.continueOnError !== false;

    try {
      // Process in chunks
      for (let i = 0; i < inputs.length; i += concurrency) {
        const chunk = inputs.slice(i, i + concurrency);
        const chunkPromises = chunk.map(async (input) => {
          try {
            const result = await this.executeActivity(input);
            return { success: true, data: result };
          } catch (error) {
            if (!continueOnError) {
              throw error;
            }
            return { success: false, error: error as Error };
          }
        });

        const chunkResults = await Promise.all(chunkPromises);
        results.push(...chunkResults);
      }

      const successful = results.filter((r) => r.success).length;
      const failed = results.filter((r) => !r.success).length;
      const duration = Date.now() - startTime;

      this.logger.info("Batch activity execution completed", {
        ...context,
        successful,
        failed,
        duration,
      });

      return {
        successful,
        failed,
        total: inputs.length,
        results,
        duration,
      };
    } catch (error) {
      this.logger.error("Batch activity execution failed", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Analytics and Monitoring
  // ============================================================================

  /**
   * Get resource statistics
   */
  async getStats(workflowId?: string): Promise<ResourceStats> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_stats",
      requestId,
      ...(workflowId ? { workflowId } : {}),
    };

    this.logger.debug("Getting resource statistics", context);

    try {
      const args: any = {};
      if (workflowId) {
        args.workflowId = workflowId;
      }

      const query = `
        query ResourceStats($workflowId: ID) {
          resourceStats(workflowId: $workflowId) {
            totalResources
            byState
            byWorkflow
            activeResources
            completedResources
            failedResources
            averageAge
            recentActivity
            mostActiveWorkflows {
              workflowId
              count
            }
          }
        }
      `;

      const response = await this.graphqlClient.request(query, args);
      const stats = response.resourceStats;

      this.logger.debug("Resource statistics retrieved", {
        ...context,
        totalResources: stats.totalResources,
        activeResources: stats.activeResources,
      });

      return stats;
    } catch (error) {
      this.logger.error("Failed to get resource statistics", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Validate a resource
   */
  async validate(
    resourceId: string,
    options: ResourceValidationOptions = {},
  ): Promise<ResourceValidationReport> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_validate",
      resourceId,
      requestId,
    };

    this.logger.debug("Validating resource", context);

    try {
      const resource = await this.get(resourceId, { includeWorkflow: true });
      const workflow = await this.getWorkflow(resource.workflowId);

      const report: ResourceValidationReport = {
        valid: true,
        errors: [],
        warnings: [],
        suggestions: [],
        dataValid: true,
        stateValid: true,
        metadataValid: true,
        consistencyIssues: [],
      };

      // Validate state
      if (options.validateState !== false) {
        if (!workflow.states.includes(resource.state)) {
          report.errors.push(`Invalid state '${resource.state}' for workflow`);
          report.stateValid = false;
          report.valid = false;
        }
      }

      // Additional validation logic would go here...
      // This is a simplified version

      this.logger.debug("Resource validation completed", {
        ...context,
        valid: report.valid,
        errors: report.errors.length,
        warnings: report.warnings.length,
      });

      return report;
    } catch (error) {
      this.logger.error("Failed to validate resource", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get resource health status
   */
  async getHealth(workflowId?: string): Promise<ResourceHealthStatus> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "resource_health",
      requestId,
      ...(workflowId ? { workflowId } : {}),
    };

    this.logger.debug("Getting resource health status", context);

    try {
      const args: any = {};
      if (workflowId) {
        args.workflowId = workflowId;
      }

      const query = `
        query ResourceHealth($workflowId: ID) {
          resourceHealth(workflowId: $workflowId) {
            healthy
            issues
            lastCheck
            stuckResources
            errorRate
            avgProcessingTime
            oldestResource
          }
        }
      `;

      const response = await this.graphqlClient.request(query, args);
      const health = response.resourceHealth;

      this.logger.debug("Resource health status retrieved", {
        ...context,
        healthy: health.healthy,
        issues: health.issues.length,
      });

      return {
        ...health,
        lastCheck: new Date(health.lastCheck),
        oldestResource: health.oldestResource
          ? new Date(health.oldestResource)
          : undefined,
      };
    } catch (error) {
      this.logger.error("Failed to get resource health", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Manager Lifecycle
  // ============================================================================

  /**
   * Initialize the resource manager
   */
  async initialize(): Promise<void> {
    const context: LogContext = { operation: "resource_manager_init" };
    this.logger.info("Initializing ResourceManager", context);

    try {
      // Perform any initialization tasks
      // This could include warming up caches, validating connections, etc.

      this.logger.info("ResourceManager initialized successfully", context);
    } catch (error) {
      this.logger.error("Failed to initialize ResourceManager", context, error);
      throw error;
    }
  }

  /**
   * Get manager health status
   */
  async getManagerHealth(): Promise<{
    healthy: boolean;
    cacheSize: number;
    lastActivity?: Date;
  }> {
    return {
      healthy: true,
      cacheSize: this.cache.size,
      lastActivity: new Date(),
    };
  }

  /**
   * Get manager statistics
   */
  getManagerStats(): {
    cacheSize: number;
    maxCacheSize: number;
    cacheHitRate: number;
  } {
    return {
      cacheSize: this.cache.size,
      maxCacheSize: this.maxCacheSize,
      cacheHitRate: 0, // This would need proper tracking
    };
  }

  /**
   * Reset the manager (clear caches, etc.)
   */
  async reset(): Promise<void> {
    this.cache.clear();
    this.workflowCache.clear();
  }

  /**
   * Dispose of the manager and clean up resources
   */
  async dispose(): Promise<void> {
    this.cache.clear();
    this.workflowCache.clear();
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  /**
   * Validate resource input
   */
  private async validateResourceInput(
    input: ResourceCreateInput,
  ): Promise<void> {
    if (!input.workflowId) {
      throw new ResourceValidationError(
        "Workflow ID is required",
        "MISSING_WORKFLOW_ID",
        { input },
      );
    }

    if (input.data === undefined || input.data === null) {
      throw new ResourceValidationError(
        "Resource data is required",
        "MISSING_RESOURCE_DATA",
        { input },
      );
    }

    // Additional validation logic can be added here
  }

  /**
   * Validate resource for workflow compatibility
   */
  private async validateResourceForWorkflow(
    input: ResourceCreateInput,
    workflow: WorkflowDefinition,
  ): Promise<void> {
    // Validate initial state if provided
    if (input.initialState && !workflow.states.includes(input.initialState)) {
      throw new ResourceValidationError(
        `Initial state '${input.initialState}' not found in workflow states`,
        "INVALID_INITIAL_STATE",
        {
          providedState: input.initialState,
          availableStates: workflow.states,
        },
      );
    }

    // Additional workflow-specific validation can be added here
  }

  /**
   * Validate resource update
   */
  private async validateResourceUpdate(
    resource: Resource,
    input: ResourceUpdateInput,
  ): Promise<void> {
    // Basic validation - can be extended
    if (input.data !== undefined && input.data === null) {
      throw new ResourceValidationError(
        "Resource data cannot be null",
        "INVALID_RESOURCE_DATA",
        { resourceId: resource.id },
      );
    }
  }

  /**
   * Get workflow definition (with caching)
   */
  private async getWorkflow(workflowId: string): Promise<WorkflowDefinition> {
    // Check cache first
    const cached = this.workflowCache.get(workflowId);
    if (cached) {
      return cached;
    }

    // Fetch from API
    const query = `
      query GetWorkflow($id: ID!) {
        workflow(id: $id) {
          name
          states
          initialState
          activities {
            id
            name
            config
          }
        }
      }
    `;

    const variables = { id: workflowId };

    const response = await this.graphqlClient.request(query, variables);

    if (!response.workflow) {
      throw new ResourceValidationError(
        `Workflow not found: ${workflowId}`,
        "WORKFLOW_NOT_FOUND",
        { workflowId },
      );
    }

    const workflow = { id: workflowId, ...response.workflow };

    // Cache the workflow
    this.workflowCache.set(workflowId!, workflow);

    // Clean cache if it gets too large
    if (this.workflowCache.size > this.maxCacheSize) {
      const firstKey = this.workflowCache.keys().next().value;
      if (firstKey) {
        this.workflowCache.delete(firstKey);
      }
    }

    return workflow;
  }

  /**
   * Build filters for resource queries
   */
  private buildFilters(options: ResourceSearchOptions): Record<string, any> {
    const filters: Record<string, any> = {};

    if (options.query) {
      filters.search = options.query;
    }

    if (options.workflowId) {
      filters.workflowId = options.workflowId;
    }

    if (options.state) {
      filters.state = options.state;
    }

    if (options.states && options.states.length > 0) {
      filters.states = options.states;
    }

    if (options.createdAfter) {
      filters.createdAfter = options.createdAfter.toISOString();
    }

    if (options.createdBefore) {
      filters.createdBefore = options.createdBefore.toISOString();
    }

    if (options.updatedAfter) {
      filters.updatedAfter = options.updatedAfter.toISOString();
    }

    if (options.updatedBefore) {
      filters.updatedBefore = options.updatedBefore.toISOString();
    }

    if (options.metadata) {
      filters.metadata = options.metadata;
    }

    if (options.dataFields) {
      filters.dataFields = options.dataFields;
    }

    return filters;
  }

  /**
   * Get resource from cache
   */
  private getCacheEntry(resourceId: string): Resource | undefined {
    return this.cache.get(resourceId);
  }

  /**
   * Set resource in cache
   */
  private setCacheEntry(resourceId: string, resource: Resource): void {
    this.cache.set(resourceId, resource);

    // Clean cache if it gets too large
    if (this.cache.size > this.maxCacheSize) {
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }

    // Set timeout to clear cache entry
    setTimeout(() => {
      this.cache.delete(resourceId);
    }, this.cacheTimeout);
  }

  /**
   * Generate a unique request ID for tracking
   */
  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new ResourceManager instance
 */
export function createResourceManager(
  graphqlClient: GraphQLClient,
  logger: Logger,
): ResourceManager {
  return new ResourceManager(graphqlClient, logger);
}

/**
 * Validate a resource definition
 */
export function validateResourceDefinition(
  resource: Resource,
  workflow: WorkflowDefinition,
): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  // Validate state
  if (!workflow.states.includes(resource.state)) {
    errors.push(`Invalid state '${resource.state}' for workflow`);
  }

  // Validate workflow ID
  if (workflow.name && resource.workflowId !== workflow.name) {
    errors.push("Resource workflow ID does not match provided workflow");
  }

  // Validate required fields
  if (!resource.id) {
    errors.push("Resource ID is required");
  }

  if (!resource.data) {
    errors.push("Resource data is required");
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}
