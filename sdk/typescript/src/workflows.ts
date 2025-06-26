/**
 * Workflows API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */

import {
  Workflow,
  WorkflowCreateInput,
  WorkflowExecution,
  ExecutionStatus,
  PaginationOptions,
  PaginatedResult,
} from "./types.js";
import type { Client } from "./client.js";
import { QueryBuilder } from "./schema";

export class WorkflowClient {
  constructor(private client: Client) {}

  /**
   * Create a new workflow
   */
  async create(input: WorkflowCreateInput): Promise<Workflow> {
    const mutation = QueryBuilder.mutationWithParams(
      "CreateWorkflow",
      "createWorkflow(input: $input)",
      [
        "id",
        "name",
        "states",
        "initialState",
        "activities { id name fromStates toState conditions description }",
        "createdAt",
        "updatedAt",
      ],
      [["input", "WorkflowDefinitionInput!"]],
    );

    const variables = {
      input: {
        name: input.name,
        description: input.description || "",
        states: input.definition.states.map((s) => s.name),
        initialState: input.definition.initial_state,
        activities: input.definition.transitions.map((t, index) => ({
          id: `activity_${index}`,
          name: `${t.from} to ${t.to}`,
          fromStates: [t.from],
          toState: t.to,
          conditions: [],
          description: `Transition from ${t.from} to ${t.to} on ${t.event}`,
        })),
      },
    };

    const result = await this.client.mutation<{ createWorkflow: Workflow }>(
      mutation,
      variables,
    );
    return result.createWorkflow;
  }

  /**
   * Get a workflow by ID
   */
  async get(id: string): Promise<Workflow> {
    const query = QueryBuilder.queryWithParams(
      "GetWorkflow",
      "workflow(id: $id)",
      [
        "id",
        "name",
        "states",
        "initialState",
        "activities { id name fromStates toState conditions description }",
        "createdAt",
        "updatedAt",
      ],
      [["id", "ID!"]],
    );

    const result = await this.client.query<{ workflow: Workflow }>(query, {
      id,
    });
    return result.workflow;
  }

  /**
   * List all workflows
   */
  async list(_options?: PaginationOptions): Promise<Workflow[]> {
    const query = QueryBuilder.query("GetWorkflows", "workflows", [
      "id",
      "name",
      "states",
      "initialState",
      "activities { id name fromStates toState conditions description }",
      "createdAt",
      "updatedAt",
    ]);

    const result = await this.client.query<{ workflows: Workflow[] }>(query);
    return result.workflows;
  }

  /**
   * Update a workflow
   */
  async update(
    id: string,
    updates: Partial<WorkflowCreateInput>,
  ): Promise<Workflow> {
    const mutation = QueryBuilder.mutationWithParams(
      "UpdateWorkflow",
      "updateWorkflow(id: $id, input: $input)",
      [
        "id",
        "name",
        "states",
        "initialState",
        "activities { id name fromStates toState conditions description }",
        "createdAt",
        "updatedAt",
      ],
      [
        ["id", "ID!"],
        ["input", "WorkflowDefinitionInput!"],
      ],
    );

    const variables = {
      id,
      input: {
        name: updates.name,
        description: updates.description || "",
        states: updates.definition?.states.map((s) => s.name) || [],
        initialState: updates.definition?.initial_state || "",
        activities:
          updates.definition?.transitions.map((t, index) => ({
            id: `activity_${index}`,
            name: `${t.from} to ${t.to}`,
            fromStates: [t.from],
            toState: t.to,
            conditions: [],
            description: `Transition from ${t.from} to ${t.to} on ${t.event}`,
          })) || [],
      },
    };

    const result = await this.client.mutation<{ updateWorkflow: Workflow }>(
      mutation,
      variables,
    );
    return result.updateWorkflow;
  }

  /**
   * Delete a workflow
   */
  async delete(id: string): Promise<boolean> {
    const mutation = QueryBuilder.mutationWithParams(
      "DeleteWorkflow",
      "deleteWorkflow(id: $id)",
      ["success"],
      [["id", "ID!"]],
    );

    const result = await this.client.mutation<{
      deleteWorkflow: { success: boolean };
    }>(mutation, { id });
    return result.deleteWorkflow.success;
  }

  /**
   * Execute a workflow by creating a workflow instance
   */
  async execute(id: string, input: Record<string, any>): Promise<any> {
    const mutation = QueryBuilder.mutationWithParams(
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

    const variables = {
      input: {
        workflowId: id,
        initialData: input.initialData || input.data || {},
        metadata: input.metadata || {},
        triggeredBy: input.triggeredBy || "sdk",
      },
    };

    const result = await this.client.mutation<{
      createWorkflowInstance: any;
    }>(mutation, variables);
    return result.createWorkflowInstance;
  }

  /**
   * Get resource (workflow instance) status
   */
  async getExecution(resourceId: string): Promise<any> {
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

    const result = await this.client.query<{
      resource: any;
    }>(query, { id: resourceId });
    return result.resource;
  }

  /**
   * List workflow executions
   */
  async listExecutions(workflowId?: string): Promise<WorkflowExecution[]> {
    const query = QueryBuilder.queryWithParams(
      "GetWorkflowExecutions",
      "workflowExecutions(workflowId: $workflowId)",
      [
        "id",
        "workflow_id",
        "status",
        "current_state",
        "input",
        "output",
        "error",
        "started_at",
        "completed_at",
      ],
      [["workflowId", "ID"]],
    );

    const result = await this.client.query<{
      workflowExecutions: WorkflowExecution[];
    }>(query, { workflowId });
    return result.workflowExecutions;
  }

  /**
   * Cancel a workflow execution
   */
  async cancelExecution(executionId: string): Promise<boolean> {
    const mutation = QueryBuilder.mutationWithParams(
      "CancelWorkflowExecution",
      "cancelExecution(id: $id)",
      ["success"],
      [["id", "ID!"]],
    );

    const result = await this.client.mutation<{
      cancelExecution: { success: boolean };
    }>(mutation, { id: executionId });
    return result.cancelExecution.success;
  }
}

// ============================================================================
// Builder Pattern for Workflow Creation
// ============================================================================

export class WorkflowBuilder {
  private workflow: Partial<WorkflowCreateInput> = {
    definition: {
      states: [],
      transitions: [],
      initial_state: "",
    },
  };

  /**
   * Set workflow name
   */
  setName(name: string): WorkflowBuilder {
    this.workflow.name = name;
    return this;
  }

  /**
   * Set workflow description
   */
  setDescription(description: string): WorkflowBuilder {
    this.workflow.description = description;
    return this;
  }

  /**
   * Add a state to the workflow
   */
  addState(name: string, type: "normal" | "final" = "normal"): WorkflowBuilder {
    if (!this.workflow.definition) {
      this.workflow.definition = {
        states: [],
        transitions: [],
        initial_state: "",
      };
    }

    this.workflow.definition.states.push({ name, type });
    return this;
  }

  /**
   * Add a transition between states
   */
  addTransition(from: string, to: string, event: string): WorkflowBuilder {
    if (!this.workflow.definition) {
      this.workflow.definition = {
        states: [],
        transitions: [],
        initial_state: "",
      };
    }

    this.workflow.definition.transitions.push({ from, to, event });
    return this;
  }

  /**
   * Set the initial state
   */
  setInitialState(state: string): WorkflowBuilder {
    if (!this.workflow.definition) {
      this.workflow.definition = {
        states: [],
        transitions: [],
        initial_state: "",
      };
    }

    this.workflow.definition.initial_state = state;
    return this;
  }

  /**
   * Build the workflow definition
   */
  build(): WorkflowCreateInput {
    if (!this.workflow.name) {
      throw new Error("Workflow name is required");
    }

    if (
      !this.workflow.definition ||
      this.workflow.definition.states.length === 0
    ) {
      throw new Error("Workflow must have at least one state");
    }

    if (!this.workflow.definition.initial_state) {
      throw new Error("Workflow must have an initial state");
    }

    return this.workflow as WorkflowCreateInput;
  }
}

/**
 * Create a new workflow builder
 */
export function createWorkflow(name: string): WorkflowBuilder {
  return new WorkflowBuilder().setName(name);
}
