/**
 * Resources API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */
import { Resource, ResourceCreateInput, ResourceUpdateInput, PaginationOptions } from "./types.js";
import type { Client } from "./client.js";
export declare class ResourceClient {
    private client;
    constructor(client: Client);
    /**
     * Create a new resource
     */
    create(input: ResourceCreateInput): Promise<Resource>;
    /**
     * Get a resource by ID
     */
    get(id: string): Promise<Resource>;
    /**
     * List all resources
     */
    list(_options?: PaginationOptions): Promise<Resource[]>;
    /**
     * Update a resource
     */
    update(id: string, updates: ResourceUpdateInput): Promise<Resource>;
    /**
     * Delete a resource
     */
    delete(id: string): Promise<boolean>;
    /**
     * Transition a resource to a new state
     */
    transition(id: string, newState: string, event: string): Promise<Resource>;
    /**
     * Execute an activity on a resource
     */
    executeActivity(id: string, activityId: string, options?: {
        strict?: boolean;
        data?: any;
    }): Promise<Resource>;
    /**
     * Get resource history
     */
    getHistory(id: string, _options?: {
        limit?: number;
    }): Promise<{
        data: Array<any>;
    }>;
}
export declare class ResourceBuilder {
    private resource;
    /**
     * Set workflow ID
     */
    setWorkflowId(workflowId: string): ResourceBuilder;
    /**
     * Add data field
     */
    addData(key: string, value: any): ResourceBuilder;
    /**
     * Set initial state
     */
    setInitialState(state: string): ResourceBuilder;
    /**
     * Build the resource definition
     */
    build(): ResourceCreateInput;
}
/**
 * Create a new resource builder
 */
export declare function createResource(workflowId: string): ResourceBuilder;
//# sourceMappingURL=resources.d.ts.map