/**
 * Resources API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */
export class ResourceClient {
    constructor(client) {
        this.client = client;
    }
    /**
     * Create a new resource
     */
    async create(input) {
        const mutation = `
      mutation CreateResource($input: ResourceCreateInput!) {
        createResource(input: $input) {
          id
          workflowId
          state
          data
          createdAt
          updatedAt
        }
      }
    `;
        const variables = {
            input: {
                workflowId: input.workflow_id,
                initialState: input.initial_state || "pending",
                data: input.data,
                metadata: {},
            },
        };
        const result = await this.client.mutation(mutation, variables);
        return result.createResource;
    }
    /**
     * Get a resource by ID
     */
    async get(id) {
        const query = `
      query GetResource($id: String!) {
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
          }
        }
      }
    `;
        const result = await this.client.query(query, {
            id,
        });
        return result.resource;
    }
    /**
     * List all resources
     */
    async list(_options) {
        const query = `
      query GetResources {
        resources {
          id
          workflowId
          state
          data
          createdAt
          updatedAt
        }
      }
    `;
        const result = await this.client.query(query);
        return result.resources;
    }
    /**
     * Update a resource
     */
    async update(id, updates) {
        const mutation = `
      mutation UpdateResource($id: ID!, $input: ResourceCreateInput!) {
        updateResource(id: $id, input: $input) {
          id
          workflowId
          state
          data
          createdAt
          updatedAt
        }
      }
    `;
        const variables = {
            id,
            input: {
                data: updates.data,
                state: updates.state,
            },
        };
        const result = await this.client.mutation(mutation, variables);
        return result.updateResource;
    }
    /**
     * Delete a resource
     */
    async delete(id) {
        const mutation = `
      mutation DeleteResource($id: ID!) {
        deleteResource(id: $id) {
          success
        }
      }
    `;
        const result = await this.client.mutation(mutation, { id });
        return result.deleteResource.success;
    }
    /**
     * Transition a resource to a new state
     */
    async transition(id, newState, event) {
        const mutation = `
      mutation TransitionResource($id: ID!, $state: String!, $event: String!) {
        transitionResource(id: $id, state: $state, event: $event) {
          id
          workflowId
          state
          data
          createdAt
          updatedAt
        }
      }
    `;
        const variables = {
            id,
            state: newState,
            event,
        };
        const result = await this.client.mutation(mutation, variables);
        return result.transitionResource;
    }
    /**
     * Execute an activity on a resource
     */
    async executeActivity(id, activityId, options = {}) {
        const mutation = `
      mutation ExecuteActivity($input: ActivityExecuteInput!) {
        executeActivity(input: $input) {
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
          }
        }
      }
    `;
        const variables = {
            input: {
                resourceId: id,
                activityId,
                data: options.data || {},
            },
        };
        const result = await this.client.mutation(mutation, variables);
        return result.executeActivity;
    }
    /**
     * Get resource history
     */
    async getHistory(id, _options = {}) {
        const query = `
      query GetResource($id: String!) {
        resource(id: $id) {
          history {
            timestamp
            activity
            fromState
            toState
            data
          }
        }
      }
    `;
        const result = await this.client.query(query, { id });
        return { data: result.resource.history };
    }
}
// ============================================================================
// Builder Pattern for Resource Creation
// ============================================================================
export class ResourceBuilder {
    constructor() {
        this.resource = {
            data: {},
        };
    }
    /**
     * Set workflow ID
     */
    setWorkflowId(workflowId) {
        this.resource.workflow_id = workflowId;
        return this;
    }
    /**
     * Add data field
     */
    addData(key, value) {
        if (!this.resource.data) {
            this.resource.data = {};
        }
        this.resource.data[key] = value;
        return this;
    }
    /**
     * Set initial state
     */
    setInitialState(state) {
        this.resource.initial_state = state;
        return this;
    }
    /**
     * Build the resource definition
     */
    build() {
        if (!this.resource.workflow_id) {
            throw new Error("Workflow ID is required");
        }
        if (!this.resource.data || Object.keys(this.resource.data).length === 0) {
            throw new Error("Resource must have at least one data field");
        }
        return this.resource;
    }
}
/**
 * Create a new resource builder
 */
export function createResource(workflowId) {
    return new ResourceBuilder().setWorkflowId(workflowId);
}
//# sourceMappingURL=resources.js.map