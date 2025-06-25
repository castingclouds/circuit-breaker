/**
 * GraphQL schema loader for TypeScript SDK
 *
 * This module provides access to the GraphQL schema files as the single source of truth,
 * eliminating hardcoded schema strings throughout the codebase.
 *
 * Follows the same pattern as the Rust SDK QueryBuilder approach.
 */

import { readFileSync } from "fs";
import { join } from "path";

/**
 * GraphQL schema files loader
 */
export class Schema {
  private files: Map<string, string> = new Map();
  private static instance: Schema;

  constructor() {
    this.loadSchemaFiles();
  }

  /**
   * Get the singleton schema instance
   */
  public static getInstance(): Schema {
    if (!Schema.instance) {
      Schema.instance = new Schema();
    }
    return Schema.instance;
  }

  /**
   * Load all schema files from the schema directory
   */
  private loadSchemaFiles(): void {
    const schemaDir = join(__dirname, "../../../schema");

    try {
      // Load all schema files
      const schemaFiles = [
        "agents.graphql",
        "analytics.graphql",
        "llm.graphql",
        "mcp.graphql",
        "rules.graphql",
        "types.graphql",
        "workflow.graphql",
        "subscriptions.graphql",
        "nats.graphql",
      ];

      for (const filename of schemaFiles) {
        const filepath = join(schemaDir, filename);
        const content = readFileSync(filepath, "utf-8");
        const name = filename.replace(".graphql", "");
        this.files.set(name, content);
      }
    } catch (error) {
      console.warn("Failed to load schema files:", error);
      // Fallback - schema files will be empty but won't crash
    }
  }

  /**
   * Get a specific schema file content
   */
  public get(name: string): string | undefined {
    return this.files.get(name);
  }

  /**
   * Get all schema files
   */
  public getAll(): Map<string, string> {
    return new Map(this.files);
  }

  /**
   * Get the complete schema by combining all files
   */
  public getCompleteSchema(): string {
    let completeSchema = "";

    // Add base types first if available
    const typesSchema = this.get("types");
    if (typesSchema) {
      completeSchema += typesSchema + "\n";
    }

    // Add all other schemas
    for (const [name, content] of this.files) {
      if (name !== "types") {
        completeSchema += content + "\n";
      }
    }

    return completeSchema;
  }
}

/**
 * Helper class to build GraphQL operations (matches Rust SDK pattern)
 */
export class QueryBuilder {
  /**
   * Build a GraphQL query
   */
  public static query(
    name: string,
    rootField: string,
    fields: string[],
  ): string {
    return this.buildOperation("query", name, rootField, fields, []);
  }

  /**
   * Build a GraphQL query with parameters
   */
  public static queryWithParams(
    name: string,
    rootField: string,
    fields: string[],
    params: Array<[string, string]>,
  ): string {
    return this.buildOperation("query", name, rootField, fields, params);
  }

  /**
   * Build a GraphQL mutation
   */
  public static mutation(
    name: string,
    rootField: string,
    fields: string[],
  ): string {
    return this.buildOperation("mutation", name, rootField, fields, []);
  }

  /**
   * Build a GraphQL mutation with parameters
   */
  public static mutationWithParams(
    name: string,
    rootField: string,
    fields: string[],
    params: Array<[string, string]>,
  ): string {
    return this.buildOperation("mutation", name, rootField, fields, params);
  }

  /**
   * Build a GraphQL subscription
   */
  public static subscription(
    name: string,
    rootField: string,
    fields: string[],
  ): string {
    return this.buildOperation("subscription", name, rootField, fields, []);
  }

  /**
   * Build a GraphQL subscription with parameters
   */
  public static subscriptionWithParams(
    name: string,
    rootField: string,
    fields: string[],
    params: Array<[string, string]>,
  ): string {
    return this.buildOperation("subscription", name, rootField, fields, params);
  }

  /**
   * Build a GraphQL operation (internal helper)
   */
  private static buildOperation(
    operationType: string,
    name: string,
    rootField: string,
    fields: string[],
    params: Array<[string, string]>,
  ): string {
    let operation = `${operationType} ${name}`;

    // Add parameters if provided
    if (params.length > 0) {
      const paramDefs = params.map(([name, type]) => `$${name}: ${type}`);
      operation += `(${paramDefs.join(", ")})`;
    }

    operation += " {\n";
    operation += `  ${rootField}`;

    // Add field selections if provided
    if (fields.length > 0) {
      operation += " {\n";
      for (const field of fields) {
        operation += `    ${field}\n`;
      }
      operation += "  }";
    }

    operation += "\n}";
    return operation;
  }
}

/**
 * Get the global schema instance
 */
export const schema = Schema.getInstance();
