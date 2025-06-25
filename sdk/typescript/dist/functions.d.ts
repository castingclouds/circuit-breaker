/**
 * Functions API client for Circuit Breaker TypeScript SDK
 * Note: Functions are handled through the activity system in GraphQL
 */
import { Function, FunctionCreateInput, FunctionExecution, PaginationOptions } from "./types.js";
import type { Client } from "./client.js";
export declare class FunctionClient {
    private _client;
    constructor(_client: Client);
    /**
     * Create a new function
     * Note: This is currently handled through the activity system
     */
    create(_input: FunctionCreateInput): Promise<Function>;
    /**
     * Get a function by ID
     */
    get(_id: string): Promise<Function>;
    /**
     * List all functions
     */
    list(options?: PaginationOptions): Promise<Function[]>;
    /**
     * Update a function
     */
    update(_id: string, _updates: Partial<FunctionCreateInput>): Promise<Function>;
    /**
     * Delete a function
     */
    delete(_id: string): Promise<boolean>;
    /**
     * Execute a function
     * This maps to executing an activity on a resource
     */
    execute(_id: string, _input: Record<string, any>): Promise<FunctionExecution>;
    /**
     * Get function execution status
     */
    getExecution(_executionId: string): Promise<FunctionExecution>;
}
export declare class FunctionBuilder {
    private func;
    /**
     * Set function name
     */
    setName(name: string): FunctionBuilder;
    /**
     * Set function description
     */
    setDescription(description: string): FunctionBuilder;
    /**
     * Set runtime
     */
    setRuntime(runtime: string): FunctionBuilder;
    /**
     * Set function code
     */
    setCode(code: string): FunctionBuilder;
    /**
     * Set timeout
     */
    setTimeout(timeout: number): FunctionBuilder;
    /**
     * Set memory limit
     */
    setMemory(memory: number): FunctionBuilder;
    /**
     * Add environment variable
     */
    addEnvironmentVariable(key: string, value: string): FunctionBuilder;
    /**
     * Build the function definition
     */
    build(): FunctionCreateInput;
}
/**
 * Create a new function builder
 */
export declare function createFunction(name: string): FunctionBuilder;
//# sourceMappingURL=functions.d.ts.map