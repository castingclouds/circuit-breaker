/**
 * NATS Event Streaming Client
 *
 * Provides enhanced resource operations with NATS event streaming capabilities.
 * This client mirrors the functionality of the Rust NATS client implementation.
 */
import { CircuitBreakerError } from "./types.js";
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
    constructor(client) {
        this.client = client;
    }
    /**
     * Get resource with NATS metadata by ID
     *
     * @param id - Resource ID
     * @returns Promise resolving to NATS resource or null if not found
     */
    async getResource(id) {
        const query = `
      query GetNATSResource($id: String!) {
        natsResource(id: $id) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            id
            event
            data
            timestamp
            source
          }
        }
      }
    `;
        const variables = { id };
        const response = await this.client.graphqlRequest(query, variables);
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
    async resourcesInState(workflowId, stateId) {
        const query = `
      query GetResourcesInState($workflowId: String!, $stateId: String!) {
        resourcesInState(workflowId: $workflowId, stateId: $stateId) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            id
            event
            data
            timestamp
            source
          }
        }
      }
    `;
        const variables = { workflowId, stateId };
        const response = await this.client.graphqlRequest(query, variables);
        return response.resourcesInState.map((resource) => this.mapNATSResource(resource));
    }
    /**
     * Find resource by ID with workflow context (more efficient for NATS)
     *
     * @param workflowId - Workflow ID for context
     * @param resourceId - Resource ID to find
     * @returns Promise resolving to NATS resource or null if not found
     */
    async findResource(workflowId, resourceId) {
        const query = `
      query FindResource($workflowId: String!, $resourceId: String!) {
        findResource(workflowId: $workflowId, resourceId: $resourceId) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            id
            event
            data
            timestamp
            source
          }
        }
      }
    `;
        const variables = { workflowId, resourceId };
        const response = await this.client.graphqlRequest(query, variables);
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
    createWorkflowInstance(workflowId) {
        return new CreateWorkflowInstanceBuilder(this.client, workflowId);
    }
    /**
     * Execute activity with NATS event publishing
     *
     * @param resourceId - Resource ID
     * @param activityName - Activity name
     * @returns ExecuteActivityWithNATSBuilder
     */
    executeActivityWithNats(resourceId, activityName) {
        return new ExecuteActivityWithNATSBuilder(this.client, resourceId, activityName);
    }
    /**
     * Map GraphQL NATS resource to SDK type
     */
    mapNATSResource(resource) {
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
    constructor(client, workflowId) {
        this.client = client;
        this.workflowId = workflowId;
        this.enableNatsEvents = true; // Default to enabled
    }
    /**
     * Set workflow ID
     */
    setWorkflowId(workflowId) {
        this.workflowId = workflowId;
        return this;
    }
    /**
     * Set initial data for the workflow instance
     */
    setInitialData(data) {
        this.initialData = data;
        return this;
    }
    /**
     * Set initial state for the workflow instance
     */
    setInitialState(state) {
        this.initialState = state;
        return this;
    }
    /**
     * Set metadata for the workflow instance
     */
    setMetadata(metadata) {
        this.metadata = metadata;
        return this;
    }
    /**
     * Enable or disable NATS event tracking
     */
    setEnableNatsEvents(enabled) {
        this.enableNatsEvents = enabled;
        return this;
    }
    /**
     * Execute the workflow instance creation
     */
    async execute() {
        if (!this.workflowId) {
            throw new CircuitBreakerError("Workflow ID is required", "VALIDATION_ERROR");
        }
        const query = `
      mutation CreateWorkflowInstance($input: CreateWorkflowInstanceInput!) {
        createWorkflowInstance(input: $input) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            id
            event
            data
            timestamp
            source
          }
        }
      }
    `;
        const input = {
            workflowId: this.workflowId,
            ...(this.initialData !== undefined && { initialData: this.initialData }),
            ...(this.initialState !== undefined && {
                initialState: this.initialState,
            }),
            ...(this.metadata !== undefined && { metadata: this.metadata }),
            ...(this.enableNatsEvents !== undefined && {
                enableNatsEvents: this.enableNatsEvents,
            }),
        };
        const response = await this.client.graphqlRequest(query, { input });
        return this.mapNATSResource(response.createWorkflowInstance);
    }
    /**
     * Map GraphQL NATS resource to SDK type
     */
    mapNATSResource(resource) {
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
    constructor(client, resourceId, activityName) {
        this.client = client;
        this.resourceId = resourceId;
        this.activityName = activityName;
    }
    /**
     * Set resource ID
     */
    setResourceId(resourceId) {
        this.resourceId = resourceId;
        return this;
    }
    /**
     * Set activity name
     */
    setActivityName(activityName) {
        this.activityName = activityName;
        return this;
    }
    /**
     * Set input data for the activity
     */
    setInputData(data) {
        this.inputData = data;
        return this;
    }
    /**
     * Set NATS subject for event publishing
     */
    setNatsSubject(subject) {
        this.natsSubject = subject;
        return this;
    }
    /**
     * Set NATS headers
     */
    setNatsHeaders(headers) {
        this.natsHeaders = headers;
        return this;
    }
    /**
     * Add a NATS header
     */
    addNatsHeader(key, value) {
        if (!this.natsHeaders) {
            this.natsHeaders = {};
        }
        this.natsHeaders[key] = value;
        return this;
    }
    /**
     * Execute the activity with NATS event publishing
     */
    async execute() {
        if (!this.resourceId) {
            throw new CircuitBreakerError("Resource ID is required", "VALIDATION_ERROR");
        }
        if (!this.activityName) {
            throw new CircuitBreakerError("Activity name is required", "VALIDATION_ERROR");
        }
        const query = `
      mutation ExecuteActivityWithNATS($input: ExecuteActivityWithNATSInput!) {
        executeActivityWithNats(input: $input) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            id
            event
            data
            timestamp
            source
          }
        }
      }
    `;
        const input = {
            resourceId: this.resourceId,
            activityName: this.activityName,
            ...(this.inputData !== undefined && { inputData: this.inputData }),
            ...(this.natsSubject !== undefined && { natsSubject: this.natsSubject }),
            ...(this.natsHeaders !== undefined && { natsHeaders: this.natsHeaders }),
        };
        const response = await this.client.graphqlRequest(query, { input });
        return this.mapNATSResource(response.executeActivityWithNats);
    }
    /**
     * Map GraphQL NATS resource to SDK type
     */
    mapNATSResource(resource) {
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
export function createWorkflowInstance(client, workflowId) {
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
export function executeActivityWithNats(client, resourceId, activityName) {
    return client.nats().executeActivityWithNats(resourceId, activityName);
}
/**
 * Get NATS resource by ID
 *
 * @param client - Circuit Breaker client
 * @param resourceId - Resource ID
 * @returns Promise resolving to NATS resource or null
 */
export async function getNatsResource(client, resourceId) {
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
export async function getResourcesInState(client, workflowId, stateId) {
    return client.nats().resourcesInState(workflowId, stateId);
}
//# sourceMappingURL=nats.js.map