/**
 * Workflows API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */
import { Workflow, WorkflowCreateInput, WorkflowExecution, PaginationOptions } from "./types.js";
import type { Client } from "./client.js";
export declare class WorkflowClient {
    private client;
    constructor(client: Client);
    /**
     * Create a new workflow
     */
    create(input: WorkflowCreateInput): Promise<Workflow>;
    /**
     * Get a workflow by ID
     */
    get(id: string): Promise<Workflow>;
    /**
     * List all workflows
     */
    list(_options?: PaginationOptions): Promise<Workflow[]>;
    /**
     * Update a workflow
     */
    update(id: string, updates: Partial<WorkflowCreateInput>): Promise<Workflow>;
    /**
     * Delete a workflow
     */
    delete(id: string): Promise<boolean>;
    /**
     * Execute a workflow by creating a workflow instance
     */
    execute(id: string, input: Record<string, any>): Promise<any>;
    /**
     * Get resource (workflow instance) status
     */
    getExecution(resourceId: string): Promise<any>;
    /**
     * List workflow executions
     */
    listExecutions(workflowId?: string): Promise<WorkflowExecution[]>;
    /**
     * Cancel a workflow execution
     */
    cancelExecution(executionId: string): Promise<boolean>;
}
export declare class WorkflowBuilder {
    private workflow;
    /**
     * Set workflow name
     */
    setName(name: string): WorkflowBuilder;
    /**
     * Set workflow description
     */
    setDescription(description: string): WorkflowBuilder;
    /**
     * Add a state to the workflow
     */
    addState(name: string, type?: "normal" | "final"): WorkflowBuilder;
    /**
     * Add a transition between states
     */
    addTransition(from: string, to: string, event: string): WorkflowBuilder;
    /**
     * Set the initial state
     */
    setInitialState(state: string): WorkflowBuilder;
    /**
     * Build the workflow definition
     */
    build(): WorkflowCreateInput;
}
/**
 * Create a new workflow builder
 */
export declare function createWorkflow(name: string): WorkflowBuilder;
//# sourceMappingURL=workflows.d.ts.map