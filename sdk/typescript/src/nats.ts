/**
 * NATS Event Streaming Client
 *
 * Provides enhanced resource operations with NATS event streaming capabilities.
 * This client mirrors the functionality of the Rust NATS client implementation.
 */

import { Client } from "./client.js";
import { CircuitBreakerError } from "./types.js";
import { QueryBuilder } from "./schema";

// ============================================================================
// Types
// ============================================================================

/**
 * NATS-enhanced resource with event history
 */
export interface NATSResource {
  id: string;
  workflowId: string;
  state: string;
  data: Record<string, any>;
  metadata: Record<string, any>;
  createdAt: string;
  updatedAt: string;
  history: HistoryEvent[];
}

/**
 * Event history entry for NATS resources
 */
export interface HistoryEvent {
  id: string;
  event: string;
  data: Record<string, any>;
  timestamp: string;
  source: string;
}

/**
 * Input for creating workflow instances with NATS event tracking
 */
export interface CreateWorkflowInstanceInput {
  workflowId: string;
  initialData?: Record<string, any> | undefined;
  initialState?: string | undefined;
  metadata?: Record<string, any> | undefined;
  enableNatsEvents?: boolean | undefined;
}

/**
 * Input for executing activities with NATS event publishing
 */
export interface ExecuteActivityWithNATSInput {
  resourceId: string;
  activityName: string;
  inputData?: Record<string, any> | undefined;
  natsSubject?: string | undefined;
  natsHeaders?: Record<string, string> | undefined;
}

// ============================================================================
// Internal GraphQL Types
// ============================================================================

interface NATSResourceGQL {
  id: string;
  workflowId: string;
  state: string;
  data: Record<string, any>;
  metadata: Record<string, any>;
  createdAt: string;
  updatedAt: string;
  history: HistoryEventGQL[];
}

interface HistoryEventGQL {
  id: string;
  event: string;
  data: Record<string, any>;
  timestamp: string;
  source: string;
}

// ============================================================================
// NATS Client
// ============================================================================

/**
 * NATS Event Streaming Client
 *
 * Provides enhanced resource operations with NATS event streaming capabilities.
 * This client offers specialized operations for resources that need event tracking
 * and state management through NATS messaging.
 */
export class NATSClient {
  constructor(private client: Client) {}

  /**
   * Get resource with NATS metadata by ID
   *
   * @param id - Resource ID
   * @returns Promise resolving to NATS resource or null if not found
   */
  async getResource(id: string): Promise<NATSResource | null> {
    const query = QueryBuilder.queryWithParams(
      "GetNATSResource",
      "natsResource(id: $id)",
      [
        "id",
        "workflowId",
        "state",
        "data",
        "metadata",
        "createdAt",
        "updatedAt",
        "history { timestamp activity fromState toState data }",
      ],
      [["id", "String!"]],
    );

    const variables = { id };
    const response = await this.client.graphqlRequest<{
      natsResource: NATSResourceGQL | null;
    }>(query, variables);

    return response.natsResource
      ? this.mapNATSResource(response.natsResource)
      : null;
  }

  /**
   * Get resources currently in a specific state (NATS-specific)
   *
   * @param workflowId - Workflow ID
   * @param stateId - State ID to filter by
   * @returns Promise resolving to array of NATS resources
   */
  async resourcesInState(
    workflowId: string,
    stateId: string,
  ): Promise<NATSResource[]> {
    const query = QueryBuilder.queryWithParams(
      "GetResourcesInState",
      "resourcesInState(workflowId: $workflowId, stateId: $stateId)",
      [
        "id",
        "workflowId",
        "state",
        "data",
        "metadata",
        "createdAt",
        "updatedAt",
        "history { timestamp activity fromState toState data }",
      ],
      [
        ["workflowId", "String!"],
        ["stateId", "String!"],
      ],
    );

    const variables = { workflowId, stateId };
    const response = await this.client.graphqlRequest<{
      resourcesInState: NATSResourceGQL[];
    }>(query, variables);

    return response.resourcesInState.map((resource) =>
      this.mapNATSResource(resource),
    );
  }

  /**
   * Find resource by ID with workflow context (more efficient for NATS)
   *
   * @param workflowId - Workflow ID for context
   * @param resourceId - Resource ID to find
   * @returns Promise resolving to NATS resource or null if not found
   */
  async findResource(
    workflowId: string,
    resourceId: string,
  ): Promise<NATSResource | null> {
    const query = QueryBuilder.queryWithParams(
      "FindResource",
      "findResource(workflowId: $workflowId, resourceId: $resourceId)",
      [
        "id",
        "workflowId",
        "state",
        "data",
        "metadata",
        "createdAt",
        "updatedAt",
        "history { timestamp activity fromState toState data }",
      ],
      [
        ["workflowId", "String!"],
        ["resourceId", "String!"],
      ],
    );

    const variables = { workflowId, resourceId };
    const response = await this.client.graphqlRequest<{
      findResource: NATSResourceGQL | null;
    }>(query, variables);

    return response.findResource
      ? this.mapNATSResource(response.findResource)
      : null;
  }

  /**
   * Create a workflow instance with NATS event tracking
   *
   * @param workflowId - Workflow ID
   * @returns CreateWorkflowInstanceBuilder
   */
  createWorkflowInstance(workflowId?: string): CreateWorkflowInstanceBuilder {
    return new CreateWorkflowInstanceBuilder(this.client, workflowId);
  }

  /**
   * Execute activity with NATS event publishing
   *
   * @param resourceId - Resource ID
   * @param activityName - Activity name
   * @returns ExecuteActivityWithNATSBuilder
   */
  executeActivityWithNats(
    resourceId?: string,
    activityName?: string,
  ): ExecuteActivityWithNATSBuilder {
    return new ExecuteActivityWithNATSBuilder(
      this.client,
      resourceId,
      activityName,
    );
  }

  /**
   * Map GraphQL NATS resource to SDK type
   */
  private mapNATSResource(resource: NATSResourceGQL): NATSResource {
    return {
      id: resource.id,
      workflowId: resource.workflowId,
      state: resource.state,
      data: resource.data,
      metadata: resource.metadata,
      createdAt: resource.createdAt,
      updatedAt: resource.updatedAt,
      history: resource.history.map((event) => ({
        id: event.id,
        event: event.event,
        data: event.data,
        timestamp: event.timestamp,
        source: event.source,
      })),
    };
  }
}

// ============================================================================
// Builders
// ============================================================================

/**
 * Builder for creating workflow instances with NATS event tracking
 */
export class CreateWorkflowInstanceBuilder {
  private workflowId?: string | undefined;
  private initialData?: Record<string, any>;
  private initialState?: string;
  private metadata?: Record<string, any>;
  private enableNatsEvents?: boolean;

  constructor(
    private client: Client,
    workflowId?: string,
  ) {
    this.workflowId = workflowId;
    this.enableNatsEvents = true; // Default to enabled
  }

  /**
   * Set workflow ID
   */
  setWorkflowId(workflowId: string): this {
    this.workflowId = workflowId;
    return this;
  }

  /**
   * Set initial data for the workflow instance
   */
  setInitialData(data: Record<string, any>): this {
    this.initialData = data;
    return this;
  }

  /**
   * Set initial state for the workflow instance
   */
  setInitialState(state: string): this {
    this.initialState = state;
    return this;
  }

  /**
   * Set metadata for the workflow instance
   */
  setMetadata(metadata: Record<string, any>): this {
    this.metadata = metadata;
    return this;
  }

  /**
   * Enable or disable NATS event tracking
   */
  setEnableNatsEvents(enabled: boolean): this {
    this.enableNatsEvents = enabled;
    return this;
  }

  /**
   * Execute the workflow instance creation
   */
  async execute(): Promise<NATSResource> {
    if (!this.workflowId) {
      throw new CircuitBreakerError(
        "Workflow ID is required",
        "VALIDATION_ERROR",
      );
    }

    const query = QueryBuilder.mutationWithParams(
      "CreateWorkflowInstance",
      "createWorkflowInstance(input: $input)",
      [
        "id",
        "workflowId",
        "state",
        "data",
        "metadata",
        "createdAt",
        "updatedAt",
        "history { timestamp activity fromState toState data }",
      ],
      [["input", "CreateWorkflowInstanceInput!"]],
    );

    const input: CreateWorkflowInstanceInput = {
      workflowId: this.workflowId!,
      ...(this.initialData !== undefined && { initialData: this.initialData }),
      ...(this.initialState !== undefined && {
        initialState: this.initialState,
      }),
      ...(this.metadata !== undefined && { metadata: this.metadata }),
      ...(this.enableNatsEvents !== undefined && {
        enableNatsEvents: this.enableNatsEvents,
      }),
    };

    const response = await this.client.graphqlRequest<{
      createWorkflowInstance: NATSResourceGQL;
    }>(query, { input });

    return this.mapNATSResource(response.createWorkflowInstance);
  }

  /**
   * Map GraphQL NATS resource to SDK type
   */
  private mapNATSResource(resource: NATSResourceGQL): NATSResource {
    return {
      id: resource.id,
      workflowId: resource.workflowId,
      state: resource.state,
      data: resource.data,
      metadata: resource.metadata,
      createdAt: resource.createdAt,
      updatedAt: resource.updatedAt,
      history: resource.history.map((event) => ({
        id: event.id,
        event: event.event,
        data: event.data,
        timestamp: event.timestamp,
        source: event.source,
      })),
    };
  }
}

/**
 * Builder for executing activities with NATS event publishing
 */
export class ExecuteActivityWithNATSBuilder {
  private resourceId?: string | undefined;
  private activityName?: string | undefined;
  private inputData?: Record<string, any>;
  private natsSubject?: string;
  private natsHeaders?: Record<string, string>;

  constructor(
    private client: Client,
    resourceId?: string,
    activityName?: string,
  ) {
    this.resourceId = resourceId;
    this.activityName = activityName;
  }

  /**
   * Set resource ID
   */
  setResourceId(resourceId: string): this {
    this.resourceId = resourceId;
    return this;
  }

  /**
   * Set activity name
   */
  setActivityName(activityName: string): this {
    this.activityName = activityName;
    return this;
  }

  /**
   * Set input data for the activity
   */
  setInputData(data: Record<string, any>): this {
    this.inputData = data;
    return this;
  }

  /**
   * Set NATS subject for event publishing
   */
  setNatsSubject(subject: string): this {
    this.natsSubject = subject;
    return this;
  }

  /**
   * Set NATS headers
   */
  setNatsHeaders(headers: Record<string, string>): this {
    this.natsHeaders = headers;
    return this;
  }

  /**
   * Add a NATS header
   */
  addNatsHeader(key: string, value: string): this {
    if (!this.natsHeaders) {
      this.natsHeaders = {};
    }
    this.natsHeaders[key] = value;
    return this;
  }

  /**
   * Execute the activity with NATS event publishing
   */
  async execute(): Promise<NATSResource> {
    if (!this.resourceId) {
      throw new CircuitBreakerError(
        "Resource ID is required",
        "VALIDATION_ERROR",
      );
    }
    if (!this.activityName) {
      throw new CircuitBreakerError(
        "Activity name is required",
        "VALIDATION_ERROR",
      );
    }

    const query = QueryBuilder.mutationWithParams(
      "ExecuteActivityWithNATS",
      "executeActivityWithNats(input: $input)",
      [
        "id",
        "workflowId",
        "state",
        "data",
        "metadata",
        "createdAt",
        "updatedAt",
        "history { timestamp activity fromState toState data }",
      ],
      [["input", "ExecuteActivityWithNATSInput!"]],
    );

    const input: ExecuteActivityWithNATSInput = {
      resourceId: this.resourceId!,
      activityName: this.activityName!,
      ...(this.inputData !== undefined && { inputData: this.inputData }),
      ...(this.natsSubject !== undefined && { natsSubject: this.natsSubject }),
      ...(this.natsHeaders !== undefined && { natsHeaders: this.natsHeaders }),
    };

    const response = await this.client.graphqlRequest<{
      executeActivityWithNats: NATSResourceGQL;
    }>(query, { input });

    return this.mapNATSResource(response.executeActivityWithNats);
  }

  /**
   * Map GraphQL NATS resource to SDK type
   */
  private mapNATSResource(resource: NATSResourceGQL): NATSResource {
    return {
      id: resource.id,
      workflowId: resource.workflowId,
      state: resource.state,
      data: resource.data,
      metadata: resource.metadata,
      createdAt: resource.createdAt,
      updatedAt: resource.updatedAt,
      history: resource.history.map((event) => ({
        id: event.id,
        event: event.event,
        data: event.data,
        timestamp: event.timestamp,
        source: event.source,
      })),
    };
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Create a workflow instance with NATS event tracking
 *
 * @param client - Circuit Breaker client
 * @param workflowId - Workflow ID
 * @returns CreateWorkflowInstanceBuilder
 */
export function createWorkflowInstance(
  client: Client,
  workflowId: string,
): CreateWorkflowInstanceBuilder {
  return client
    .nats()
    .createWorkflowInstance(workflowId)
    .setEnableNatsEvents(true);
}

/**
 * Execute activity with NATS event publishing
 *
 * @param client - Circuit Breaker client
 * @param resourceId - Resource ID
 * @param activityName - Activity name
 * @returns ExecuteActivityWithNATSBuilder
 */
export function executeActivityWithNats(
  client: Client,
  resourceId: string,
  activityName: string,
): ExecuteActivityWithNATSBuilder {
  return client.nats().executeActivityWithNats(resourceId, activityName);
}

/**
 * Get NATS resource by ID
 *
 * @param client - Circuit Breaker client
 * @param resourceId - Resource ID
 * @returns Promise resolving to NATS resource or null
 */
export async function getNatsResource(
  client: Client,
  resourceId: string,
): Promise<NATSResource | null> {
  return client.nats().getResource(resourceId);
}

/**
 * Get resources in a specific state
 *
 * @param client - Circuit Breaker client
 * @param workflowId - Workflow ID
 * @param stateId - State ID
 * @returns Promise resolving to array of NATS resources
 */
export async function getResourcesInState(
  client: Client,
  workflowId: string,
  stateId: string,
): Promise<NATSResource[]> {
  return client.nats().resourcesInState(workflowId, stateId);
}
