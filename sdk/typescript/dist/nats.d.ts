/**
 * NATS Event Streaming Client
 *
 * Provides enhanced resource operations with NATS event streaming capabilities.
 * This client mirrors the functionality of the Rust NATS client implementation.
 */
import { Client } from "./client.js";
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
/**
 * NATS Event Streaming Client
 *
 * Provides enhanced resource operations with NATS event streaming capabilities.
 * This client offers specialized operations for resources that need event tracking
 * and state management through NATS messaging.
 */
export declare class NATSClient {
    private client;
    constructor(client: Client);
    /**
     * Get resource with NATS metadata by ID
     *
     * @param id - Resource ID
     * @returns Promise resolving to NATS resource or null if not found
     */
    getResource(id: string): Promise<NATSResource | null>;
    /**
     * Get resources currently in a specific state (NATS-specific)
     *
     * @param workflowId - Workflow ID
     * @param stateId - State ID to filter by
     * @returns Promise resolving to array of NATS resources
     */
    resourcesInState(workflowId: string, stateId: string): Promise<NATSResource[]>;
    /**
     * Find resource by ID with workflow context (more efficient for NATS)
     *
     * @param workflowId - Workflow ID for context
     * @param resourceId - Resource ID to find
     * @returns Promise resolving to NATS resource or null if not found
     */
    findResource(workflowId: string, resourceId: string): Promise<NATSResource | null>;
    /**
     * Create a workflow instance with NATS event tracking
     *
     * @param workflowId - Workflow ID
     * @returns CreateWorkflowInstanceBuilder
     */
    createWorkflowInstance(workflowId?: string): CreateWorkflowInstanceBuilder;
    /**
     * Execute activity with NATS event publishing
     *
     * @param resourceId - Resource ID
     * @param activityName - Activity name
     * @returns ExecuteActivityWithNATSBuilder
     */
    executeActivityWithNats(resourceId?: string, activityName?: string): ExecuteActivityWithNATSBuilder;
    /**
     * Map GraphQL NATS resource to SDK type
     */
    private mapNATSResource;
}
/**
 * Builder for creating workflow instances with NATS event tracking
 */
export declare class CreateWorkflowInstanceBuilder {
    private client;
    private workflowId?;
    private initialData?;
    private initialState?;
    private metadata?;
    private enableNatsEvents?;
    constructor(client: Client, workflowId?: string);
    /**
     * Set workflow ID
     */
    setWorkflowId(workflowId: string): this;
    /**
     * Set initial data for the workflow instance
     */
    setInitialData(data: Record<string, any>): this;
    /**
     * Set initial state for the workflow instance
     */
    setInitialState(state: string): this;
    /**
     * Set metadata for the workflow instance
     */
    setMetadata(metadata: Record<string, any>): this;
    /**
     * Enable or disable NATS event tracking
     */
    setEnableNatsEvents(enabled: boolean): this;
    /**
     * Execute the workflow instance creation
     */
    execute(): Promise<NATSResource>;
    /**
     * Map GraphQL NATS resource to SDK type
     */
    private mapNATSResource;
}
/**
 * Builder for executing activities with NATS event publishing
 */
export declare class ExecuteActivityWithNATSBuilder {
    private client;
    private resourceId?;
    private activityName?;
    private inputData?;
    private natsSubject?;
    private natsHeaders?;
    constructor(client: Client, resourceId?: string, activityName?: string);
    /**
     * Set resource ID
     */
    setResourceId(resourceId: string): this;
    /**
     * Set activity name
     */
    setActivityName(activityName: string): this;
    /**
     * Set input data for the activity
     */
    setInputData(data: Record<string, any>): this;
    /**
     * Set NATS subject for event publishing
     */
    setNatsSubject(subject: string): this;
    /**
     * Set NATS headers
     */
    setNatsHeaders(headers: Record<string, string>): this;
    /**
     * Add a NATS header
     */
    addNatsHeader(key: string, value: string): this;
    /**
     * Execute the activity with NATS event publishing
     */
    execute(): Promise<NATSResource>;
    /**
     * Map GraphQL NATS resource to SDK type
     */
    private mapNATSResource;
}
/**
 * Create a workflow instance with NATS event tracking
 *
 * @param client - Circuit Breaker client
 * @param workflowId - Workflow ID
 * @returns CreateWorkflowInstanceBuilder
 */
export declare function createWorkflowInstance(client: Client, workflowId: string): CreateWorkflowInstanceBuilder;
/**
 * Execute activity with NATS event publishing
 *
 * @param client - Circuit Breaker client
 * @param resourceId - Resource ID
 * @param activityName - Activity name
 * @returns ExecuteActivityWithNATSBuilder
 */
export declare function executeActivityWithNats(client: Client, resourceId: string, activityName: string): ExecuteActivityWithNATSBuilder;
/**
 * Get NATS resource by ID
 *
 * @param client - Circuit Breaker client
 * @param resourceId - Resource ID
 * @returns Promise resolving to NATS resource or null
 */
export declare function getNatsResource(client: Client, resourceId: string): Promise<NATSResource | null>;
/**
 * Get resources in a specific state
 *
 * @param client - Circuit Breaker client
 * @param workflowId - Workflow ID
 * @param stateId - State ID
 * @returns Promise resolving to array of NATS resources
 */
export declare function getResourcesInState(client: Client, workflowId: string, stateId: string): Promise<NATSResource[]>;
//# sourceMappingURL=nats.d.ts.map