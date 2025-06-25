/**
 * Functions API client for Circuit Breaker TypeScript SDK
 * Note: Functions are handled through the activity system in GraphQL
 */

import {
  Function,
  FunctionCreateInput,
  ExecutionStatus,
  PaginationOptions,
} from "./types";
import type { Client } from "./client";
import { QueryBuilder } from "./schema";

export class FunctionClient {
  constructor(private _client: Client) {}

  /**
   * Create a new function
   * Note: This is currently handled through the activity system
   */
  async create(_input: FunctionCreateInput): Promise<Function> {
    throw new Error(
      "Function creation is not directly supported in the current GraphQL schema. " +
        "Functions are managed through the activity system. " +
        "Use workflow activities to define function-like behavior.",
    );
  }

  /**
   * Get a function by ID
   */
  async get(_id: string): Promise<Function> {
    throw new Error(
      "Direct function retrieval is not supported. " +
        "Functions are managed through workflow activities.",
    );
  }

  /**
   * List all functions
   */
  async list(options?: PaginationOptions): Promise<Function[]> {
    // Return available activities as a proxy for functions
    const query = QueryBuilder.queryWithParams(
      "GetAvailableActivities",
      "availableActivities(resourceId: $resourceId)",
      ["id", "name", "fromStates", "toState", "conditions", "description"],
      [["resourceId", "ID!"]],
    );

    // This would need a resource ID, so we return empty for now
    console.warn(
      "Function listing is not directly supported. Use workflow activities instead.",
    );
    return [];
  }

  /**
   * Update a function
   */
  async update(
    _id: string,
    _updates: Partial<FunctionCreateInput>,
  ): Promise<Function> {
    throw new Error(
      "Function updates are not supported. " +
        "Update workflow activities instead.",
    );
  }

  /**
   * Delete a function
   */
  async delete(_id: string): Promise<boolean> {
    throw new Error(
      "Function deletion is not supported. " +
        "Remove activities from workflows instead.",
    );
  }

  /**
   * Execute a function
   * This maps to executing an activity on a resource
   */
  async execute(
    _id: string,
    _input: Record<string, any>,
  ): Promise<FunctionExecution> {
    throw new Error(
      "Direct function execution is not supported. " +
        "Use resource.executeActivity() to execute function-like activities.",
    );
  }

  /**
   * Get function execution status
   */
  async getExecution(_executionId: string): Promise<FunctionExecution> {
    throw new Error(
      "Function execution tracking is not supported. " +
        "Use workflow execution tracking instead.",
    );
  }
}

// ============================================================================
// Builder Pattern for Function Creation (Legacy Support)
// ============================================================================

export class FunctionBuilder {
  private func: Partial<FunctionCreateInput> = {};

  /**
   * Set function name
   */
  setName(name: string): FunctionBuilder {
    this.func.name = name;
    return this;
  }

  /**
   * Set function description
   */
  setDescription(description: string): FunctionBuilder {
    this.func.description = description;
    return this;
  }

  /**
   * Set runtime
   */
  setRuntime(runtime: string): FunctionBuilder {
    this.func.runtime = runtime;
    return this;
  }

  /**
   * Set function code
   */
  setCode(code: string): FunctionBuilder {
    this.func.code = code;
    return this;
  }

  /**
   * Set timeout
   */
  setTimeout(timeout: number): FunctionBuilder {
    if (!this.func.config) this.func.config = {};
    this.func.config.timeout = timeout;
    return this;
  }

  /**
   * Set memory limit
   */
  setMemory(memory: number): FunctionBuilder {
    if (!this.func.config) this.func.config = {};
    this.func.config.memory = memory;
    return this;
  }

  /**
   * Add environment variable
   */
  addEnvironmentVariable(key: string, value: string): FunctionBuilder {
    if (!this.func.config) this.func.config = {};
    if (!this.func.config.environment) this.func.config.environment = {};
    this.func.config.environment[key] = value;
    return this;
  }

  /**
   * Build the function definition
   */
  build(): FunctionCreateInput {
    if (!this.func.name) {
      throw new Error("Function name is required");
    }

    if (!this.func.runtime) {
      throw new Error("Function runtime is required");
    }

    if (!this.func.code) {
      throw new Error("Function code is required");
    }

    console.warn(
      "Function creation through builder is deprecated. " +
        "Use workflow activities for function-like behavior.",
    );

    return this.func as FunctionCreateInput;
  }
}

/**
 * Create a new function builder
 */
export function createFunction(name: string): FunctionBuilder {
  return new FunctionBuilder().setName(name);
}
