/**
 * Functions API client for Circuit Breaker TypeScript SDK
 * Note: Functions are handled through the activity system in GraphQL
 */
export class FunctionClient {
    constructor(_client) {
        this._client = _client;
    }
    /**
     * Create a new function
     * Note: This is currently handled through the activity system
     */
    async create(_input) {
        throw new Error("Function creation is not directly supported in the current GraphQL schema. " +
            "Functions are managed through the activity system. " +
            "Use workflow activities to define function-like behavior.");
    }
    /**
     * Get a function by ID
     */
    async get(_id) {
        throw new Error("Direct function retrieval is not supported. " +
            "Functions are managed through workflow activities.");
    }
    /**
     * List all functions
     */
    async list(options) {
        // Return available activities as a proxy for functions
        const query = `
      query GetAvailableActivities($resourceId: ID!) {
        availableActivities(resourceId: $resourceId) {
          id
          name
          fromStates
          toState
          conditions
          description
        }
      }
    `;
        // This would need a resource ID, so we return empty for now
        console.warn("Function listing is not directly supported. Use workflow activities instead.");
        return [];
    }
    /**
     * Update a function
     */
    async update(_id, _updates) {
        throw new Error("Function updates are not supported. " +
            "Update workflow activities instead.");
    }
    /**
     * Delete a function
     */
    async delete(_id) {
        throw new Error("Function deletion is not supported. " +
            "Remove activities from workflows instead.");
    }
    /**
     * Execute a function
     * This maps to executing an activity on a resource
     */
    async execute(_id, _input) {
        throw new Error("Direct function execution is not supported. " +
            "Use resource.executeActivity() to execute function-like activities.");
    }
    /**
     * Get function execution status
     */
    async getExecution(_executionId) {
        throw new Error("Function execution tracking is not supported. " +
            "Use workflow execution tracking instead.");
    }
}
// ============================================================================
// Builder Pattern for Function Creation (Legacy Support)
// ============================================================================
export class FunctionBuilder {
    constructor() {
        this.func = {};
    }
    /**
     * Set function name
     */
    setName(name) {
        this.func.name = name;
        return this;
    }
    /**
     * Set function description
     */
    setDescription(description) {
        this.func.description = description;
        return this;
    }
    /**
     * Set runtime
     */
    setRuntime(runtime) {
        this.func.runtime = runtime;
        return this;
    }
    /**
     * Set function code
     */
    setCode(code) {
        this.func.code = code;
        return this;
    }
    /**
     * Set timeout
     */
    setTimeout(timeout) {
        if (!this.func.config)
            this.func.config = {};
        this.func.config.timeout = timeout;
        return this;
    }
    /**
     * Set memory limit
     */
    setMemory(memory) {
        if (!this.func.config)
            this.func.config = {};
        this.func.config.memory = memory;
        return this;
    }
    /**
     * Add environment variable
     */
    addEnvironmentVariable(key, value) {
        if (!this.func.config)
            this.func.config = {};
        if (!this.func.config.environment)
            this.func.config.environment = {};
        this.func.config.environment[key] = value;
        return this;
    }
    /**
     * Build the function definition
     */
    build() {
        if (!this.func.name) {
            throw new Error("Function name is required");
        }
        if (!this.func.runtime) {
            throw new Error("Function runtime is required");
        }
        if (!this.func.code) {
            throw new Error("Function code is required");
        }
        console.warn("Function creation through builder is deprecated. " +
            "Use workflow activities for function-like behavior.");
        return this.func;
    }
}
/**
 * Create a new function builder
 */
export function createFunction(name) {
    return new FunctionBuilder().setName(name);
}
//# sourceMappingURL=functions.js.map