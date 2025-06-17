/**
 * Workflow Manager for Circuit Breaker TypeScript SDK
 *
 * This file provides comprehensive workflow management functionality including
 * CRUD operations, validation, search, and workflow lifecycle management
 * through the GraphQL API.
 */

import {
  WorkflowDefinition,
  ActivityDefinition,
  PaginationOptions,
  PaginatedResult,
  FilterOptions,
  DeepPartial,
} from '../core/types.js';
import {
  WorkflowError,
  WorkflowNotFoundError,
  WorkflowValidationError,
  GraphQLError,
  ValidationError,
  NetworkError,
  ErrorHandler,
} from '../core/errors.js';
import { GraphQLClient, QueryBuilder } from '../utils/graphql.js';
import { Logger, LogContext, sanitizeForLogging } from '../utils/logger.js';

// ============================================================================
// Types
// ============================================================================

export interface WorkflowCreateInput {
  name: string;
  states: string[];
  activities: ActivityDefinition[];
  initialState: string;
  description?: string;
  version?: string;
  tags?: string[];
  metadata?: Record<string, any>;
}

export interface WorkflowUpdateInput {
  name?: string;
  description?: string;
  version?: string;
  tags?: string[];
  metadata?: Record<string, any>;
  // Note: States and activities updates require special handling
}

export interface WorkflowSearchOptions extends PaginationOptions, FilterOptions {
  /** Search in workflow names and descriptions */
  query?: string;

  /** Filter by workflow tags */
  tags?: string[];

  /** Filter by workflow version */
  version?: string;

  /** Filter by creation date range */
  createdAfter?: Date;
  createdBefore?: Date;

  /** Filter by last update date range */
  updatedAfter?: Date;
  updatedBefore?: Date;

  /** Include workflow statistics */
  includeStats?: boolean;

  /** Include activity details */
  includeActivities?: boolean;
}

export interface WorkflowStats {
  /** Total number of resources created for this workflow */
  totalResources: number;

  /** Number of active resources (not in terminal states) */
  activeResources: number;

  /** Number of completed resources */
  completedResources: number;

  /** Number of failed resources */
  failedResources: number;

  /** Average execution time for completed workflows */
  averageExecutionTime?: number;

  /** Most common current states */
  stateDistribution: Record<string, number>;

  /** Activity execution statistics */
  activityStats: Record<string, {
    executions: number;
    successRate: number;
    averageExecutionTime: number;
  }>;

  /** Last execution timestamp */
  lastExecution?: Date;
}

export interface WorkflowWithStats extends WorkflowDefinition {
  id: string;
  createdAt: string;
  updatedAt: string;
  stats?: WorkflowStats;
}

export interface WorkflowValidationOptions {
  /** Check for unreachable states */
  checkReachability?: boolean;

  /** Validate rule syntax */
  validateRules?: boolean;

  /** Check for potential infinite loops */
  checkLoops?: boolean;

  /** Validate activity references */
  validateReferences?: boolean;

  /** Include performance warnings */
  includeWarnings?: boolean;
}

export interface WorkflowValidationReport {
  valid: boolean;
  errors: string[];
  warnings: string[];
  suggestions: string[];
  stateCount: number;
  activityCount: number;
  ruleCount: number;
  complexity: 'low' | 'medium' | 'high' | 'very_high';
  estimatedExecutionPaths: number;
  potentialBottlenecks: string[];
  unreachableStates: string[];
  terminalStates: string[];
}

export interface WorkflowHealthStatus {
  healthy: boolean;
  issues: string[];
  lastCheck: Date;
  resourceCount: number;
  errorRate: number;
  avgExecutionTime: number;
}

// ============================================================================
// Workflow Manager
// ============================================================================

export class WorkflowManager {
  private readonly graphqlClient: GraphQLClient;
  private readonly logger: Logger;
  private readonly cache = new Map<string, WorkflowWithStats>();
  private readonly cacheTimeout = 5 * 60 * 1000; // 5 minutes
  private readonly maxCacheSize = 100;

  constructor(graphqlClient: GraphQLClient, logger: Logger) {
    this.graphqlClient = graphqlClient;
    this.logger = logger.child({ component: 'WorkflowManager' });
  }

  // ============================================================================
  // CRUD Operations
  // ============================================================================

  /**
   * Create a new workflow
   */
  async create(workflow: WorkflowCreateInput, context?: LogContext): Promise<string> {
    const requestId = context?.requestId || this.generateRequestId();

    this.logger.info('Creating workflow', {
      workflowName: workflow.name,
      stateCount: workflow.states.length,
      activityCount: workflow.activities.length,
      requestId,
    });

    try {
      // Pre-validate workflow structure
      this.validateWorkflowInput(workflow);

      const mutation = `
        mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
          createWorkflow(input: $input) {
            id
            name
            states
            activities {
              id
              name
              fromStates
              toState
              conditions
              description
              metadata
            }
            initialState
            description
            version
            tags
            metadata
            createdAt
            updatedAt
          }
        }
      `;

      const variables = {
        input: {
          name: workflow.name,
          states: workflow.states,
          activities: workflow.activities,
          initialState: workflow.initialState,
          description: workflow.description,
          version: workflow.version,
          tags: workflow.tags,
          metadata: workflow.metadata,
        },
      };

      const result = await this.graphqlClient.request<{
        createWorkflow: WorkflowWithStats;
      }>(mutation, variables, undefined, { 'X-Request-ID': requestId });

      const createdWorkflow = result.createWorkflow;

      // Cache the created workflow
      this.setCacheEntry(createdWorkflow.id, createdWorkflow);

      this.logger.info('Workflow created successfully', {
        workflowId: createdWorkflow.id,
        workflowName: createdWorkflow.name,
        requestId,
      });

      return createdWorkflow.id;

    } catch (error) {
      const handled = ErrorHandler.handle(error, requestId);

      this.logger.error('Failed to create workflow', {
        workflowName: workflow.name,
        error: handled.message,
        requestId,
      });

      if (error instanceof GraphQLError) {
        // Check for specific GraphQL validation errors
        const validationErrors = error.graphqlErrors
          .filter(e => e.extensions?.code === 'VALIDATION_ERROR')
          .map(e => e.message);

        if (validationErrors.length > 0) {
          throw new WorkflowValidationError(validationErrors, workflow, requestId);
        }
      }

      throw new WorkflowError(
        `Failed to create workflow: ${handled.message}`,
        'WORKFLOW_CREATE_ERROR',
        { workflow: sanitizeForLogging(workflow) },
        requestId
      );
    }
  }

  /**
   * Get a workflow by ID
   */
  async get(workflowId: string, includeStats: boolean = false, context?: LogContext): Promise<WorkflowWithStats> {
    const requestId = context?.requestId || this.generateRequestId();

    // Check cache first
    const cached = this.getCacheEntry(workflowId);
    if (cached && (!includeStats || cached.stats)) {
      this.logger.debug('Retrieved workflow from cache', { workflowId, requestId });
      return cached;
    }

    this.logger.debug('Fetching workflow', { workflowId, includeStats, requestId });

    try {
      const query = `
        query GetWorkflow($id: String!, $includeStats: Boolean = false) {
          workflow(id: $id) {
            id
            name
            states
            activities {
              id
              name
              fromStates
              toState
              conditions
              description
              metadata
            }
            initialState
            description
            version
            tags
            metadata
            createdAt
            updatedAt
            stats @include(if: $includeStats) {
              totalResources
              activeResources
              completedResources
              failedResources
              averageExecutionTime
              stateDistribution
              activityStats
              lastExecution
            }
          }
        }
      `;

      const result = await this.graphqlClient.request<{
        workflow: WorkflowWithStats | null;
      }>(query, { id: workflowId, includeStats }, undefined, { 'X-Request-ID': requestId });

      if (!result.workflow) {
        throw new WorkflowNotFoundError(workflowId, requestId);
      }

      // Cache the result
      this.setCacheEntry(workflowId, result.workflow);

      this.logger.debug('Workflow retrieved successfully', { workflowId, requestId });

      return result.workflow;

    } catch (error) {
      if (error instanceof WorkflowNotFoundError) {
        throw error;
      }

      const handled = ErrorHandler.handle(error, requestId);

      this.logger.error('Failed to get workflow', {
        workflowId,
        error: handled.message,
        requestId,
      });

      throw new WorkflowError(
        `Failed to get workflow: ${handled.message}`,
        'WORKFLOW_GET_ERROR',
        { workflowId },
        requestId
      );
    }
  }

  /**
   * Update a workflow
   */
  async update(
    workflowId: string,
    updates: WorkflowUpdateInput,
    context?: LogContext
  ): Promise<WorkflowWithStats> {
    const requestId = context?.requestId || this.generateRequestId();

    this.logger.info('Updating workflow', {
      workflowId,
      updates: sanitizeForLogging(updates),
      requestId,
    });

    try {
      const mutation = `
        mutation UpdateWorkflow($id: String!, $input: WorkflowUpdateInput!) {
          updateWorkflow(id: $id, input: $input) {
            id
            name
            states
            activities {
              id
              name
              fromStates
              toState
              conditions
              description
              metadata
            }
            initialState
            description
            version
            tags
            metadata
            createdAt
            updatedAt
          }
        }
      `;

      const result = await this.graphqlClient.request<{
        updateWorkflow: WorkflowWithStats;
      }>(mutation, { id: workflowId, input: updates }, undefined, { 'X-Request-ID': requestId });

      // Update cache
      this.setCacheEntry(workflowId, result.updateWorkflow);

      this.logger.info('Workflow updated successfully', { workflowId, requestId });

      return result.updateWorkflow;

    } catch (error) {
      const handled = ErrorHandler.handle(error, requestId);

      this.logger.error('Failed to update workflow', {
        workflowId,
        error: handled.message,
        requestId,
      });

      if (error instanceof GraphQLError) {
        const notFoundError = error.graphqlErrors.find(
          e => e.extensions?.code === 'NOT_FOUND'
        );
        if (notFoundError) {
          throw new WorkflowNotFoundError(workflowId, requestId);
        }
      }

      throw new WorkflowError(
        `Failed to update workflow: ${handled.message}`,
        'WORKFLOW_UPDATE_ERROR',
        { workflowId, updates },
        requestId
      );
    }
  }

  /**
   * Delete a workflow
   */
  async delete(workflowId: string, context?: LogContext): Promise<boolean> {
    const requestId = context?.requestId || this.generateRequestId();

    this.logger.info('Deleting workflow', { workflowId, requestId });

    try {
      const mutation = `
        mutation DeleteWorkflow($id: String!) {
          deleteWorkflow(id: $id)
        }
      `;

      const result = await this.graphqlClient.request<{
        deleteWorkflow: boolean;
      }>(mutation, { id: workflowId }, undefined, { 'X-Request-ID': requestId });

      if (result.deleteWorkflow) {
        // Remove from cache
        this.cache.delete(workflowId);

        this.logger.info('Workflow deleted successfully', { workflowId, requestId });
      }

      return result.deleteWorkflow;

    } catch (error) {
      const handled = ErrorHandler.handle(error, requestId);

      this.logger.error('Failed to delete workflow', {
        workflowId,
        error: handled.message,
        requestId,
      });

      if (error instanceof GraphQLError) {
        const notFoundError = error.graphqlErrors.find(
          e => e.extensions?.code === 'NOT_FOUND'
        );
        if (notFoundError) {
          throw new WorkflowNotFoundError(workflowId, requestId);
        }
      }

      throw new WorkflowError(
        `Failed to delete workflow: ${handled.message}`,
        'WORKFLOW_DELETE_ERROR',
        { workflowId },
        requestId
      );
    }
  }

  // ============================================================================
  // Search and List Operations
  // ============================================================================

  /**
   * List workflows with filtering and pagination
   */
  async list(options: WorkflowSearchOptions = {}, context?: LogContext): Promise<PaginatedResult<WorkflowWithStats>> {
    const requestId = context?.requestId || this.generateRequestId();

    this.logger.debug('Listing workflows', {
      options: sanitizeForLogging(options),
      requestId,
    });

    try {
      const query = `
        query ListWorkflows(
          $offset: Int,
          $limit: Int,
          $sortBy: String,
          $sortOrder: SortOrder,
          $filters: WorkflowFilters,
          $query: String,
          $includeStats: Boolean = false,
          $includeActivities: Boolean = true
        ) {
          workflows(
            offset: $offset,
            limit: $limit,
            sortBy: $sortBy,
            sortOrder: $sortOrder,
            filters: $filters,
            query: $query
          ) {
            data {
              id
              name
              states
              activities @include(if: $includeActivities) {
                id
                name
                fromStates
                toState
                conditions
                description
              }
              initialState
              description
              version
              tags
              metadata
              createdAt
              updatedAt
              stats @include(if: $includeStats) {
                totalResources
                activeResources
                completedResources
                failedResources
                averageExecutionTime
                stateDistribution
                lastExecution
              }
            }
            total
            offset
            limit
            hasMore
          }
        }
      `;

      const variables = {
        offset: options.offset || 0,
        limit: options.limit || 50,
        sortBy: options.sortBy || 'createdAt',
        sortOrder: options.sortOrder?.toUpperCase() || 'DESC',
        query: options.query,
        includeStats: options.includeStats || false,
        includeActivities: options.includeActivities !== false,
        filters: this.buildFilters(options),
      };

      const result = await this.graphqlClient.request<{
        workflows: PaginatedResult<WorkflowWithStats>;
      }>(query, variables, undefined, { 'X-Request-ID': requestId });

      // Cache workflows
      result.workflows.data.forEach(workflow => {
        this.setCacheEntry(workflow.id, workflow);
      });

      this.logger.debug('Workflows listed successfully', {
        count: result.workflows.data.length,
        total: result.workflows.total,
        requestId,
      });

      return result.workflows;

    } catch (error) {
      const handled = ErrorHandler.handle(error, requestId);

      this.logger.error('Failed to list workflows', {
        error: handled.message,
        requestId,
      });

      throw new WorkflowError(
        `Failed to list workflows: ${handled.message}`,
        'WORKFLOW_LIST_ERROR',
        { options },
        requestId
      );
    }
  }

  /**
   * Search workflows by name, description, or tags
   */
  async search(query: string, options: WorkflowSearchOptions = {}, context?: LogContext): Promise<WorkflowWithStats[]> {
    return this.list({ ...options, query }, context).then(result => result.data);
  }

  // ============================================================================
  // Validation Operations
  // ============================================================================

  /**
   * Validate a workflow definition
   */
  async validate(
    workflow: WorkflowDefinition,
    options: WorkflowValidationOptions = {},
    context?: LogContext
  ): Promise<WorkflowValidationReport> {
    const requestId = context?.requestId || this.generateRequestId();

    this.logger.debug('Validating workflow', {
      workflowName: workflow.name,
      options,
      requestId,
    });

    try {
      const mutation = `
        mutation ValidateWorkflow($input: WorkflowDefinitionInput!, $options: ValidationOptions) {
          validateWorkflow(input: $input, options: $options) {
            valid
            errors
            warnings
            suggestions
            stateCount
            activityCount
            ruleCount
            complexity
            estimatedExecutionPaths
            potentialBottlenecks
            unreachableStates
            terminalStates
          }
        }
      `;

      const result = await this.graphqlClient.request<{
        validateWorkflow: WorkflowValidationReport;
      }>(mutation, {
        input: workflow,
        options
      }, undefined, { 'X-Request-ID': requestId });

      this.logger.debug('Workflow validation completed', {
        workflowName: workflow.name,
        valid: result.validateWorkflow.valid,
        errorCount: result.validateWorkflow.errors.length,
        warningCount: result.validateWorkflow.warnings.length,
        requestId,
      });

      return result.validateWorkflow;

    } catch (error) {
      const handled = ErrorHandler.handle(error, requestId);

      this.logger.error('Failed to validate workflow', {
        workflowName: workflow.name,
        error: handled.message,
        requestId,
      });

      throw new WorkflowError(
        `Failed to validate workflow: ${handled.message}`,
        'WORKFLOW_VALIDATION_ERROR',
        { workflow: sanitizeForLogging(workflow) },
        requestId
      );
    }
  }

  // ============================================================================
  // Statistics and Monitoring
  // ============================================================================

  /**
   * Get workflow statistics
   */
  async getStats(workflowId: string, context?: LogContext): Promise<WorkflowStats> {
    const requestId = context?.requestId || this.generateRequestId();

    try {
      const query = `
        query GetWorkflowStats($id: String!) {
          workflowStats(id: $id) {
            totalResources
            activeResources
            completedResources
            failedResources
            averageExecutionTime
            stateDistribution
            activityStats
            lastExecution
          }
        }
      `;

      const result = await this.graphqlClient.request<{
        workflowStats: WorkflowStats;
      }>(query, { id: workflowId }, undefined, { 'X-Request-ID': requestId });

      return result.workflowStats;

    } catch (error) {
      const handled = ErrorHandler.handle(error, requestId);

      if (error instanceof GraphQLError) {
        const notFoundError = error.graphqlErrors.find(
          e => e.extensions?.code === 'NOT_FOUND'
        );
        if (notFoundError) {
          throw new WorkflowNotFoundError(workflowId, requestId);
        }
      }

      throw new WorkflowError(
        `Failed to get workflow stats: ${handled.message}`,
        'WORKFLOW_STATS_ERROR',
        { workflowId },
        requestId
      );
    }
  }

  /**
   * Get workflow health status
   */
  async getHealth(workflowId: string, context?: LogContext): Promise<WorkflowHealthStatus> {
    const requestId = context?.requestId || this.generateRequestId();

    try {
      const stats = await this.getStats(workflowId, context);

      const issues: string[] = [];

      // Check for potential issues
      if (stats.totalResources > 0) {
        const errorRate = stats.failedResources / stats.totalResources;
        if (errorRate > 0.1) {
          issues.push(`High error rate: ${(errorRate * 100).toFixed(1)}%`);
        }

        if (stats.activeResources / stats.totalResources > 0.8) {
          issues.push('Many resources are still active (potential bottleneck)');
        }
      }

      if (stats.averageExecutionTime && stats.averageExecutionTime > 60000) {
        issues.push('Long average execution time');
      }

      return {
        healthy: issues.length === 0,
        issues,
        lastCheck: new Date(),
        resourceCount: stats.totalResources,
        errorRate: stats.totalResources > 0 ? stats.failedResources / stats.totalResources : 0,
        avgExecutionTime: stats.averageExecutionTime || 0,
      };

    } catch (error) {
      if (error instanceof WorkflowNotFoundError) {
        throw error;
      }

      const handled = ErrorHandler.handle(error, requestId);

      throw new WorkflowError(
        `Failed to get workflow health: ${handled.message}`,
        'WORKFLOW_HEALTH_ERROR',
        { workflowId },
        requestId
      );
    }
  }

  // ============================================================================
  // Manager Lifecycle
  // ============================================================================

  /**
   * Initialize the workflow manager
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing WorkflowManager');

    try {
      // Test connectivity with a simple query
      const healthQuery = `
        query WorkflowManagerHealth {
          __schema {
            queryType {
              name
            }
          }
        }
      `;

      await this.graphqlClient.request(healthQuery);

      this.logger.info('WorkflowManager initialized successfully');
    } catch (error) {
      this.logger.error('Failed to initialize WorkflowManager', { error });
      throw error;
    }
  }

  /**
   * Get manager health status
   */
  async getManagerHealth(): Promise<{ healthy: boolean; cacheSize: number; lastActivity?: Date }> {
    try {
      // Test basic connectivity
      const testQuery = `query { __typename }`;
      await this.graphqlClient.request(testQuery);

      return {
        healthy: true,
        cacheSize: this.cache.size,
        lastActivity: new Date(),
      };
    } catch {
      return {
        healthy: false,
        cacheSize: this.cache.size,
      };
    }
  }

  /**
   * Get manager statistics
   */
  getStats(): {
    cacheHits: number;
    cacheMisses: number;
    cacheSize: number;
    operationsCount: number;
  } {
    // In a real implementation, these would be tracked
    return {
      cacheHits: 0,
      cacheMisses: 0,
      cacheSize: this.cache.size,
      operationsCount: 0,
    };
  }

  /**
   * Reset manager state
   */
  async reset(): Promise<void> {
    this.cache.clear();
    this.logger.info('WorkflowManager state reset');
  }

  /**
   * Dispose of manager resources
   */
  async dispose(): Promise<void> {
    this.cache.clear();
    this.logger.info('WorkflowManager disposed');
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private validateWorkflowInput(workflow: WorkflowCreateInput): void {
    if (!workflow.name?.trim()) {
      throw new ValidationError('name', workflow.name, 'Workflow name is required');
    }

    if (!workflow.states || workflow.states.length === 0) {
      throw new ValidationError('states', workflow.states, 'At least one state is required');
    }

    if (!workflow.initialState?.trim()) {
      throw new ValidationError('initialState', workflow.initialState, 'Initial state is required');
    }

    if (!workflow.states.includes(workflow.initialState)) {
      throw new ValidationError(
        'initialState',
        workflow.initialState,
        'Initial state must be one of the defined states'
      );
    }

    // Validate activities
    const activityIds = new Set<string>();
    for (const activity of workflow.activities) {
      if (!activity.id?.trim()) {
        throw new ValidationError('activity.id', activity.id, 'Activity ID is required');
      }

      if (activityIds.has(activity.id)) {
        throw new ValidationError('activity.id', activity.id, 'Duplicate activity ID');
      }
      activityIds.add(activity.id);

      // Validate from/to states
      for (const fromState of activity.fromStates) {
        if (!workflow.states.includes(fromState)) {
          throw new ValidationError(
            'activity.fromStates',
            fromState,
            `Unknown from state in activity ${activity.id}`
          );
        }
      }

      if (!workflow.states.includes(activity.toState)) {
        throw new ValidationError(
          'activity.toState',
          activity.toState,
          `Unknown to state in activity ${activity.id}`
        );
      }
    }
  }

  private buildFilters(options: WorkflowSearchOptions): any {
    const filters: any = {};

    if (options.tags && options.tags.length > 0) {
      filters.tags = options.tags;
    }

    if (options.version) {
      filters.version = options.version;
    }

    if (options.createdAfter || options.createdBefore) {
      filters.createdAt = {};
      if (options.createdAfter) {
        filters.createdAt.gte = options.createdAfter.toISOString();
      }
      if (options.createdBefore) {
        filters.createdAt.lte = options.createdBefore.toISOString();
      }
    }

    if (options.updatedAfter || options.updatedBefore) {
      filters.updatedAt = {};
      if (options.updatedAfter) {
        filters.updatedAt.gte = options.updatedAfter.toISOString();
      }
      if (options.updatedBefore) {
        filters.updatedAt.lte = options.updatedBefore.toISOString();
      }
    }

    return Object.keys(filters).length > 0 ? filters : undefined;
  }

  private getCacheEntry(workflowId: string): WorkflowWithStats | undefined {
    return this.cache.get(workflowId);
  }

  private setCacheEntry(workflowId: string, workflow: WorkflowWithStats): void {
    // Implement cache size limit
    if (this.cache.size >= this.maxCacheSize) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }

    this.cache.set(workflowId, {
      ...workflow,
      // Add cache timestamp for TTL
      _cacheTimestamp: Date.now(),
    } as any);
  }

  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Create a workflow manager instance
 */
export function createWorkflowManager(graphqlClient: GraphQLClient, logger: Logger): WorkflowManager {
  return new WorkflowManager(graphqlClient, logger);
}

/**
 * Validate workflow definition locally (basic validation)
 */
export function validateWorkflowDefinition(workflow: WorkflowDefinition): {
  valid: boolean;
  errors: string[];
  warnings: string[];
} {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Basic structural validation
  if (!workflow.name?.trim()) {
    errors.push('Workflow name is required');
  }

  if (!workflow.states || workflow.states.length === 0) {
    errors.push('At least one state is required');
  }

  if (!workflow.initialState?.trim()) {
    errors.push('Initial state is required');
  } else if (workflow.states && !workflow.states.includes(workflow.initialState)) {
    errors.push('Initial state must be one of the defined states');
  }

  // Check for duplicate states
  const uniqueStates = new Set(workflow.states);
  if (uniqueStates.size !== workflow.states.length) {
    errors.push('Duplicate states found');
  }

  // Check activities
  const activityIds = new Set<string>();
  for (const activity of workflow.activities) {
    if (!activity.id?.trim()) {
      errors.push(`Activity ID is required`);
      continue;
    }

    if (activityIds.has(activity.id)) {
      errors.push(`Duplicate activity ID: ${activity.id}`);
    }
    activityIds.add(activity.id);

    // Check state references
    for (const fromState of activity.fromStates) {
      if (!workflow.states.includes(fromState)) {
        errors.push(`Activity ${activity.id} references unknown from state: ${fromState}`);
      }
    }

    if (!workflow.states.includes(activity.toState)) {
      errors.push(`Activity ${activity.id} references unknown to state: ${activity.toState}`);
    }

    // Check for self-transitions
    if (activity.fromStates.includes(activity.toState)) {
      warnings.push(`Activity ${activity.id} creates a self-transition`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}
