/**
 * Resources API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */

import {
  Resource,
  ResourceCreateInput,
  ResourceUpdateInput,
  PaginationOptions,
} from "./types.js";
import type { Client } from "./client.js";
import { QueryBuilder } from "./schema";

export class ResourceClient {
  constructor(private client: Client) {}

  /**
   * Create a new resource
   */
  async create(input: ResourceCreateInput): Promise<Resource> {
    const mutation = QueryBuilder.mutationWithParams(
      "CreateResource",
      "createResource(input: $input)",
      ["id", "workflowId", "state", "data", "createdAt", "updatedAt"],
      [["input", "ResourceCreateInput!"]],
    );

    const variables = {
      input: {
        workflowId: input.workflow_id,
        initialState: input.initial_state || "pending",
        data: input.data,
        metadata: {},
      },
    };

    const result = await this.client.mutation<{ createResource: Resource }>(
      mutation,
      variables,
    );
    return result.createResource;
  }

  /**
   * Get a resource by ID
   */
  async get(id: string): Promise<Resource> {
    const query = QueryBuilder.queryWithParams(
      "GetResource",
      "resource(id: $id)",
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

    const result = await this.client.query<{ resource: Resource }>(query, {
      id,
    });
    return result.resource;
  }

  /**
   * List all resources
   */
  async list(_options?: PaginationOptions): Promise<Resource[]> {
    const query = QueryBuilder.query("GetResources", "resources", [
      "id",
      "workflowId",
      "state",
      "data",
      "createdAt",
      "updatedAt",
    ]);

    const result = await this.client.query<{ resources: Resource[] }>(query);
    return result.resources;
  }

  /**
   * Update a resource
   */
  async update(id: string, updates: ResourceUpdateInput): Promise<Resource> {
    const mutation = QueryBuilder.mutationWithParams(
      "UpdateResource",
      "updateResource(id: $id, input: $input)",
      ["id", "workflowId", "state", "data", "createdAt", "updatedAt"],
      [
        ["id", "ID!"],
        ["input", "ResourceCreateInput!"],
      ],
    );

    const variables = {
      id,
      input: {
        data: updates.data,
        state: updates.state,
      },
    };

    const result = await this.client.mutation<{ updateResource: Resource }>(
      mutation,
      variables,
    );
    return result.updateResource;
  }

  /**
   * Delete a resource
   */
  async delete(id: string): Promise<boolean> {
    const mutation = QueryBuilder.mutationWithParams(
      "DeleteResource",
      "deleteResource(id: $id)",
      ["success"],
      [["id", "ID!"]],
    );

    const result = await this.client.mutation<{
      deleteResource: { success: boolean };
    }>(mutation, { id });
    return result.deleteResource.success;
  }

  /**
   * Transition a resource to a new state
   */
  async transition(
    id: string,
    newState: string,
    event: string,
  ): Promise<Resource> {
    const mutation = QueryBuilder.mutationWithParams(
      "TransitionResource",
      "transitionResource(id: $id, state: $state, event: $event)",
      ["id", "workflowId", "state", "data", "createdAt", "updatedAt"],
      [
        ["id", "ID!"],
        ["state", "String!"],
        ["event", "String!"],
      ],
    );

    const variables = {
      id,
      state: newState,
      event,
    };

    const result = await this.client.mutation<{
      transitionResource: Resource;
    }>(mutation, variables);
    return result.transitionResource;
  }

  /**
   * Execute an activity on a resource
   */
  async executeActivity(
    id: string,
    activityId: string,
    options: { strict?: boolean; data?: any } = {},
  ): Promise<Resource> {
    const mutation = QueryBuilder.mutationWithParams(
      "ExecuteActivity",
      "executeActivity(input: $input)",
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
      [["input", "ActivityExecuteInput!"]],
    );

    const variables = {
      input: {
        resourceId: id,
        activityId,
        data: options.data || {},
      },
    };

    const result = await this.client.mutation<{
      executeActivity: Resource;
    }>(mutation, variables);
    return result.executeActivity;
  }

  /**
   * Get resource history
   */
  async getHistory(
    id: string,
    _options: { limit?: number } = {},
  ): Promise<{ data: Array<any> }> {
    const query = QueryBuilder.queryWithParams(
      "GetResourceHistory",
      "resource(id: $id)",
      ["history { timestamp activity fromState toState data }"],
      [["id", "String!"]],
    );

    const result = await this.client.query<{
      resource: {
        history: Array<{
          timestamp: string;
          activity: string;
          fromState: string;
          toState: string;
          data?: any;
        }>;
      };
    }>(query, { id });

    return { data: result.resource.history };
  }
}

// ============================================================================
// Builder Pattern for Resource Creation
// ============================================================================

export class ResourceBuilder {
  private resource: Partial<ResourceCreateInput> = {
    data: {},
  };

  /**
   * Set workflow ID
   */
  setWorkflowId(workflowId: string): ResourceBuilder {
    this.resource.workflow_id = workflowId;
    return this;
  }

  /**
   * Add data field
   */
  addData(key: string, value: any): ResourceBuilder {
    if (!this.resource.data) {
      this.resource.data = {};
    }
    this.resource.data[key] = value;
    return this;
  }

  /**
   * Set initial state
   */
  setInitialState(state: string): ResourceBuilder {
    this.resource.initial_state = state;
    return this;
  }

  /**
   * Build the resource definition
   */
  build(): ResourceCreateInput {
    if (!this.resource.workflow_id) {
      throw new Error("Workflow ID is required");
    }

    if (!this.resource.data || Object.keys(this.resource.data).length === 0) {
      throw new Error("Resource must have at least one data field");
    }

    return this.resource as ResourceCreateInput;
  }
}

/**
 * Create a new resource builder
 */
export function createResource(workflowId: string): ResourceBuilder {
  return new ResourceBuilder().setWorkflowId(workflowId);
}
