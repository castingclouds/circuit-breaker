/**
 * GraphQL schema loader for TypeScript SDK
 *
 * This module provides access to the GraphQL schema files as the single source of truth,
 * eliminating hardcoded schema strings throughout the codebase.
 */
/**
 * GraphQL schema files loader
 */
export declare class Schema {
    private files;
    private static instance;
    constructor();
    /**
     * Get the singleton schema instance
     */
    static getInstance(): Schema;
    /**
     * Load all schema files from the schema directory
     */
    private loadSchemaFiles;
    /**
     * Get a specific schema file content
     */
    get(name: string): string | undefined;
    /**
     * Get all schema files
     */
    getAll(): Map<string, string>;
    /**
     * Get the complete schema by combining all files
     */
    getCompleteSchema(): string;
}
/**
 * Helper class to build GraphQL operations
 */
export declare class QueryBuilder {
    /**
     * Build a GraphQL query
     */
    static query(name: string, rootField: string, fields: string[], params?: Array<[string, string]>): string;
    /**
     * Build a GraphQL mutation
     */
    static mutation(name: string, rootField: string, fields: string[], params?: Array<[string, string]>): string;
    /**
     * Build a GraphQL subscription
     */
    static subscription(name: string, rootField: string, fields: string[], params?: Array<[string, string]>): string;
    /**
     * Build a GraphQL operation
     */
    private static buildOperation;
}
/**
 * Get the global schema instance
 */
export declare const schema: Schema;
/**
 * Common GraphQL operations that reference the actual schema files
 */
export declare const operations: {
    agents: {
        get: (fields?: string[]) => string;
        list: (fields?: string[]) => string;
        create: (fields?: string[]) => string;
        delete: () => string;
    };
    llm: {
        chatCompletion: () => string;
        listProviders: () => string;
        listModels: () => string;
    };
    analytics: {
        budgetStatus: () => string;
        costAnalytics: () => string;
        setBudget: () => string;
    };
    functions: {
        get: () => string;
        list: () => string;
        create: () => string;
        execute: () => string;
        delete: () => string;
    };
    mcp: {
        getServer: () => string;
        deleteServer: () => string;
    };
    common: {
        ping: () => string;
        info: () => string;
    };
};
//# sourceMappingURL=schema.d.ts.map