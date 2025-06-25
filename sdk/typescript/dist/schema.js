/**
 * GraphQL schema loader for TypeScript SDK
 *
 * This module provides access to the GraphQL schema files as the single source of truth,
 * eliminating hardcoded schema strings throughout the codebase.
 */
import { readFileSync } from 'fs';
import { join } from 'path';
/**
 * GraphQL schema files loader
 */
export class Schema {
    constructor() {
        this.files = new Map();
        this.loadSchemaFiles();
    }
    /**
     * Get the singleton schema instance
     */
    static getInstance() {
        if (!Schema.instance) {
            Schema.instance = new Schema();
        }
        return Schema.instance;
    }
    /**
     * Load all schema files from the schema directory
     */
    loadSchemaFiles() {
        const schemaDir = join(__dirname, '../../../schema');
        try {
            // Load all schema files
            const schemaFiles = [
                'agents.graphql',
                'analytics.graphql',
                'llm.graphql',
                'mcp.graphql',
                'rules.graphql',
                'types.graphql',
                'workflow.graphql',
                'subscriptions.graphql',
                'nats.graphql'
            ];
            for (const filename of schemaFiles) {
                const filepath = join(schemaDir, filename);
                const content = readFileSync(filepath, 'utf-8');
                const name = filename.replace('.graphql', '');
                this.files.set(name, content);
            }
        }
        catch (error) {
            console.warn('Failed to load schema files:', error);
            // Fallback - schema files will be empty but won't crash
        }
    }
    /**
     * Get a specific schema file content
     */
    get(name) {
        return this.files.get(name);
    }
    /**
     * Get all schema files
     */
    getAll() {
        return new Map(this.files);
    }
    /**
     * Get the complete schema by combining all files
     */
    getCompleteSchema() {
        let completeSchema = '';
        // Add base types first if available
        const typesSchema = this.get('types');
        if (typesSchema) {
            completeSchema += typesSchema + '\n';
        }
        // Add all other schemas
        for (const [name, content] of this.files) {
            if (name !== 'types') {
                completeSchema += content + '\n';
            }
        }
        return completeSchema;
    }
}
/**
 * Helper class to build GraphQL operations
 */
export class QueryBuilder {
    /**
     * Build a GraphQL query
     */
    static query(name, rootField, fields, params = []) {
        return this.buildOperation('query', name, rootField, fields, params);
    }
    /**
     * Build a GraphQL mutation
     */
    static mutation(name, rootField, fields, params = []) {
        return this.buildOperation('mutation', name, rootField, fields, params);
    }
    /**
     * Build a GraphQL subscription
     */
    static subscription(name, rootField, fields, params = []) {
        return this.buildOperation('subscription', name, rootField, fields, params);
    }
    /**
     * Build a GraphQL operation
     */
    static buildOperation(operationType, name, rootField, fields, params = []) {
        let operation = `${operationType} ${name}`;
        // Add parameters if provided
        if (params.length > 0) {
            const paramDefs = params.map(([name, type]) => `$${name}: ${type}`);
            operation += `(${paramDefs.join(', ')})`;
        }
        operation += ' {\n';
        operation += `  ${rootField}`;
        // Add field selections if provided
        if (fields.length > 0) {
            operation += ' {\n';
            for (const field of fields) {
                operation += `    ${field}\n`;
            }
            operation += '  }';
        }
        operation += '\n}';
        return operation;
    }
}
/**
 * Get the global schema instance
 */
export const schema = Schema.getInstance();
/**
 * Common GraphQL operations that reference the actual schema files
 */
export const operations = {
    // Agent operations
    agents: {
        get: (fields = ['id', 'name', 'description', 'createdAt', 'updatedAt']) => QueryBuilder.query('GetAgent', 'agent(id: $id)', fields, [['id', 'ID!']]),
        list: (fields = ['id', 'name', 'description', 'createdAt', 'updatedAt']) => QueryBuilder.query('ListAgents', 'agents', fields),
        create: (fields = ['id', 'name', 'description', 'createdAt', 'updatedAt']) => QueryBuilder.mutation('CreateAgent', 'createAgent(input: $input)', fields, [['input', 'AgentDefinitionInput!']]),
        delete: () => QueryBuilder.mutation('DeleteAgent', 'deleteAgent(id: $id)', ['success'], [['id', 'ID!']]),
    },
    // LLM operations
    llm: {
        chatCompletion: () => QueryBuilder.mutation('LlmChatCompletion', 'llmChatCompletion(input: $input)', [
            'id',
            'model',
            'choices { index message { role content } finishReason }',
            'usage { promptTokens completionTokens totalTokens }',
        ], [['input', 'LlmChatCompletionInput!']]),
        listProviders: () => QueryBuilder.query('ListProviders', 'llmProviders', [
            'id',
            'providerType',
            'name',
            'baseUrl',
            'healthStatus { isHealthy errorRate averageLatencyMs }',
        ]),
        listModels: () => QueryBuilder.query('ListModels', 'llmModels', ['name', 'provider', 'available']),
    },
    // Analytics operations
    analytics: {
        budgetStatus: () => QueryBuilder.query('BudgetStatus', 'budgetStatus(userId: $userId, projectId: $projectId)', [
            'budgetId',
            'limit',
            'used',
            'percentageUsed',
            'isExhausted',
            'isWarning',
            'remaining',
            'message',
        ], [['userId', 'String'], ['projectId', 'String']]),
        costAnalytics: () => QueryBuilder.query('CostAnalytics', 'costAnalytics(input: $input)', [
            'totalCost',
            'totalTokens',
            'averageCostPerToken',
            'providerBreakdown',
            'modelBreakdown',
            'dailyCosts',
            'periodStart',
            'periodEnd',
        ], [['input', 'CostAnalyticsInput!']]),
        setBudget: () => QueryBuilder.mutation('SetBudget', 'setBudget(input: $input)', [
            'budgetId',
            'limit',
            'used',
            'percentageUsed',
            'isExhausted',
            'isWarning',
            'remaining',
            'message',
        ], [['input', 'BudgetInput!']]),
    },
    // Function operations
    functions: {
        get: () => QueryBuilder.query('GetFunction', 'function(id: $id)', ['id', 'name', 'description', 'runtime', 'entrypoint', 'createdAt', 'updatedAt'], [['id', 'ID!']]),
        list: () => QueryBuilder.query('ListFunctions', 'functions', ['id', 'name', 'description', 'runtime', 'entrypoint', 'createdAt', 'updatedAt']),
        create: () => QueryBuilder.mutation('CreateFunction', 'createFunction(input: $input)', ['id', 'name', 'description', 'runtime', 'entrypoint', 'createdAt', 'updatedAt'], [['input', 'CreateFunctionInput!']]),
        execute: () => QueryBuilder.mutation('ExecuteFunction', 'executeFunction(functionId: $functionId, input: $input)', ['id', 'functionId', 'status', 'input', 'output', 'startedAt', 'completedAt', 'errorMessage'], [['functionId', 'ID!'], ['input', 'JSON!']]),
        delete: () => QueryBuilder.mutation('DeleteFunction', 'deleteFunction(id: $id)', ['success'], [['id', 'ID!']]),
    },
    // MCP operations
    mcp: {
        getServer: () => QueryBuilder.query('GetMCPServer', 'mcpServer(id: $id)', ['id', 'name', 'description', 'type', 'status', 'config', 'capabilities', 'createdAt', 'updatedAt'], [['id', 'ID!']]),
        deleteServer: () => QueryBuilder.mutation('DeleteMCPServer', 'deleteMcpServer(id: $id)', ['success', 'message', 'errorCode', 'data'], [['id', 'ID!']]),
    },
    // Common operations
    common: {
        ping: () => QueryBuilder.query('Ping', 'llmProviders', ['name', 'healthStatus { isHealthy }']),
        info: () => QueryBuilder.query('Info', 'llmProviders', ['name', 'healthStatus { isHealthy }']),
    },
};
//# sourceMappingURL=schema.js.map