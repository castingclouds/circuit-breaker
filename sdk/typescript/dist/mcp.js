/**
 * Model Context Protocol (MCP) Client
 *
 * This module provides functionality for managing MCP servers, handling OAuth/JWT authentication,
 * and managing sessions for the Circuit Breaker workflow automation server.
 */
// ============================================================================
// Types
// ============================================================================
/**
 * MCP Server type enumeration
 */
export var MCPServerType;
(function (MCPServerType) {
    MCPServerType["BUILT_IN"] = "BUILT_IN";
    MCPServerType["CUSTOM"] = "CUSTOM";
    MCPServerType["THIRD_PARTY"] = "THIRD_PARTY";
})(MCPServerType || (MCPServerType = {}));
/**
 * MCP Server status enumeration
 */
export var MCPServerStatus;
(function (MCPServerStatus) {
    MCPServerStatus["ACTIVE"] = "ACTIVE";
    MCPServerStatus["INACTIVE"] = "INACTIVE";
    MCPServerStatus["ERROR"] = "ERROR";
    MCPServerStatus["CONNECTING"] = "CONNECTING";
})(MCPServerStatus || (MCPServerStatus = {}));
// ============================================================================
// MCP Client
// ============================================================================
/**
 * MCP client for Model Context Protocol operations
 */
export class MCPClient {
    constructor(client) {
        this.client = client;
    }
    /**
     * Get MCP servers with optional filtering
     */
    servers() {
        return new MCPServersBuilder(this.client);
    }
    /**
     * Get a specific MCP server by ID
     */
    async getServer(id) {
        const query = `
      query GetMCPServer($id: ID!) {
        mcpServer(id: $id) {
          id
          name
          description
          type
          status
          config
          capabilities
          health
          tenantId
          createdAt
          updatedAt
        }
      }
    `;
        const variables = { id };
        const response = await this.client.graphql(query, variables);
        return response.mcpServer;
    }
    /**
     * Create a new MCP server
     */
    createServer() {
        return new CreateMCPServerBuilder(this.client);
    }
    /**
     * Update an existing MCP server
     */
    updateServer(id) {
        return new UpdateMCPServerBuilder(this.client, id);
    }
    /**
     * Delete an MCP server
     */
    async deleteServer(id) {
        const query = `
      mutation DeleteMCPServer($id: ID!) {
        deleteMcpServer(id: $id) {
          success
          message
          errorCode
          data
        }
      }
    `;
        const variables = { id };
        const response = await this.client.graphql(query, variables);
        return {
            success: response.deleteMcpServer.success,
            message: response.deleteMcpServer.message,
        };
    }
    /**
     * Configure OAuth for an MCP server
     */
    configureOAuth() {
        return new ConfigureOAuthBuilder(this.client);
    }
    /**
     * Configure JWT authentication for an MCP server
     */
    configureJWT() {
        return new ConfigureJWTBuilder(this.client);
    }
    /**
     * Get available OAuth providers
     */
    async getOAuthProviders() {
        const query = `
      query GetMCPOAuthProviders {
        mcpOAuthProviders {
          id
          name
          type
          config
          isEnabled
        }
      }
    `;
        const response = await this.client.graphql(query);
        return response.mcpOAuthProviders;
    }
    /**
     * Get server capabilities
     */
    async getServerCapabilities(serverId) {
        const query = `
      query GetMCPServerCapabilities($serverId: ID!) {
        mcpServerCapabilities(serverId: $serverId) {
          tools
          resources
          prompts
          sampling
        }
      }
    `;
        const variables = { serverId };
        const response = await this.client.graphql(query, variables);
        return response.mcpServerCapabilities;
    }
    /**
     * Get server health status
     */
    async getServerHealth(serverId) {
        const query = `
      query GetMCPServerHealth($serverId: ID!) {
        mcpServerHealth(serverId: $serverId) {
          status
          message
          lastCheck
          responseTime
          details
        }
      }
    `;
        const variables = { serverId };
        const response = await this.client.graphql(query, variables);
        return response.mcpServerHealth;
    }
    /**
     * Initiate OAuth flow
     */
    async initiateOAuth(serverId, userId) {
        const query = `
      mutation InitiateMCPOAuth($input: InitiateMcpOAuthInput!) {
        initiateMcpOAuth(input: $input) {
          authUrl
          state
          codeChallenge
          expiresAt
        }
      }
    `;
        const input = { serverId, userId };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
        return response.initiateMcpOAuth;
    }
    /**
     * Complete OAuth flow
     */
    async completeOAuth(state, code) {
        const query = `
      mutation CompleteMCPOAuth($input: CompleteMcpOAuthInput!) {
        completeMcpOAuth(input: $input) {
          id
          serverId
          userId
          status
          accessToken
          refreshToken
          expiresAt
          createdAt
          updatedAt
        }
      }
    `;
        const input = { state, code };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
        return response.completeMcpOAuth;
    }
}
// ============================================================================
// Builders
// ============================================================================
/**
 * Builder for MCP servers queries
 */
export class MCPServersBuilder {
    constructor(client) {
        this.client = client;
    }
    /**
     * Filter by server type
     */
    type(type) {
        this._type = type;
        return this;
    }
    /**
     * Filter by server status
     */
    status(status) {
        this._status = status;
        return this;
    }
    /**
     * Filter by tenant ID
     */
    tenantId(tenantId) {
        this._tenantId = tenantId;
        return this;
    }
    /**
     * Set pagination parameters
     */
    pagination(pagination) {
        this._pagination = pagination;
        return this;
    }
    /**
     * Execute the query
     */
    async list() {
        const query = this._tenantId
            ? `
          query GetMCPServersByTenant($tenantId: String!, $pagination: PaginationInput) {
            mcpServersByTenant(tenantId: $tenantId, pagination: $pagination) {
              servers {
                id
                name
                description
                type
                status
                config
                capabilities
                health
                tenantId
                createdAt
                updatedAt
              }
              pageInfo {
                hasNextPage
                hasPreviousPage
                startCursor
                endCursor
                totalCount
              }
              totalCount
            }
          }
        `
            : `
          query GetMCPServers($type: McpServerType, $status: McpServerStatus, $pagination: PaginationInput) {
            mcpServers(type: $type, status: $status, pagination: $pagination) {
              servers {
                id
                name
                description
                type
                status
                config
                capabilities
                health
                tenantId
                createdAt
                updatedAt
              }
              pageInfo {
                hasNextPage
                hasPreviousPage
                startCursor
                endCursor
                totalCount
              }
              totalCount
            }
          }
        `;
        const variables = this._tenantId
            ? { tenantId: this._tenantId, pagination: this._pagination }
            : { type: this._type, status: this._status, pagination: this._pagination };
        const response = await this.client.graphql(query, variables);
        return response.mcpServers || response.mcpServersByTenant;
    }
}
/**
 * Builder for creating MCP servers
 */
export class CreateMCPServerBuilder {
    constructor(client) {
        this.client = client;
    }
    /**
     * Set the server name
     */
    name(name) {
        this._name = name;
        return this;
    }
    /**
     * Set the server description
     */
    description(description) {
        this._description = description;
        return this;
    }
    /**
     * Set the server type
     */
    type(type) {
        this._type = type;
        return this;
    }
    /**
     * Set the server configuration
     */
    config(config) {
        this._config = config;
        return this;
    }
    /**
     * Set the tenant ID
     */
    tenantId(tenantId) {
        this._tenantId = tenantId;
        return this;
    }
    /**
     * Execute the create server mutation
     */
    async execute() {
        if (!this._name) {
            throw new Error('name is required');
        }
        if (!this._type) {
            throw new Error('type is required');
        }
        const query = `
      mutation CreateMCPServer($input: CreateMcpServerInput!) {
        createMcpServer(input: $input) {
          id
          name
          description
          type
          status
          config
          capabilities
          health
          tenantId
          createdAt
          updatedAt
        }
      }
    `;
        const input = {
            name: this._name,
            description: this._description,
            type: this._type,
            config: this._config || {},
            tenantId: this._tenantId,
        };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
        return response.createMcpServer;
    }
}
/**
 * Builder for updating MCP servers
 */
export class UpdateMCPServerBuilder {
    constructor(client, id) {
        this.client = client;
        this.id = id;
    }
    /**
     * Set the server name
     */
    name(name) {
        this._name = name;
        return this;
    }
    /**
     * Set the server description
     */
    description(description) {
        this._description = description;
        return this;
    }
    /**
     * Set the server status
     */
    status(status) {
        this._status = status;
        return this;
    }
    /**
     * Set the server configuration
     */
    config(config) {
        this._config = config;
        return this;
    }
    /**
     * Execute the update server mutation
     */
    async execute() {
        const query = `
      mutation UpdateMCPServer($id: ID!, $input: UpdateMcpServerInput!) {
        updateMcpServer(id: $id, input: $input) {
          id
          name
          description
          type
          status
          config
          capabilities
          health
          tenantId
          createdAt
          updatedAt
        }
      }
    `;
        const input = {
            name: this._name,
            description: this._description,
            status: this._status,
            config: this._config,
        };
        const variables = { id: this.id, input };
        const response = await this.client.graphql(query, variables);
        return response.updateMcpServer;
    }
}
/**
 * Builder for configuring OAuth
 */
export class ConfigureOAuthBuilder {
    constructor(client) {
        this.client = client;
    }
    /**
     * Set the server ID
     */
    serverId(serverId) {
        this._serverId = serverId;
        return this;
    }
    /**
     * Set the OAuth provider
     */
    provider(provider) {
        this._provider = provider;
        return this;
    }
    /**
     * Set the client ID
     */
    clientId(clientId) {
        this._clientId = clientId;
        return this;
    }
    /**
     * Set the client secret
     */
    clientSecret(clientSecret) {
        this._clientSecret = clientSecret;
        return this;
    }
    /**
     * Set the OAuth scopes
     */
    scopes(scopes) {
        this._scopes = scopes;
        return this;
    }
    /**
     * Set the redirect URI
     */
    redirectUri(redirectUri) {
        this._redirectUri = redirectUri;
        return this;
    }
    /**
     * Execute the configure OAuth mutation
     */
    async execute() {
        if (!this._serverId) {
            throw new Error('serverId is required');
        }
        if (!this._provider) {
            throw new Error('provider is required');
        }
        if (!this._clientId) {
            throw new Error('clientId is required');
        }
        if (!this._clientSecret) {
            throw new Error('clientSecret is required');
        }
        const query = `
      mutation ConfigureMCPOAuth($input: ConfigureMcpOAuthInput!) {
        configureMcpOAuth(input: $input) {
          id
          serverId
          provider
          clientId
          scopes
          redirectUri
          isEnabled
          createdAt
          updatedAt
        }
      }
    `;
        const input = {
            serverId: this._serverId,
            provider: this._provider,
            clientId: this._clientId,
            clientSecret: this._clientSecret,
            scopes: this._scopes,
            redirectUri: this._redirectUri,
        };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
        return response.configureMcpOAuth;
    }
}
/**
 * Builder for configuring JWT
 */
export class ConfigureJWTBuilder {
    constructor(client) {
        this.client = client;
    }
    /**
     * Set the server ID
     */
    serverId(serverId) {
        this._serverId = serverId;
        return this;
    }
    /**
     * Set the JWT secret key
     */
    secretKey(secretKey) {
        this._secretKey = secretKey;
        return this;
    }
    /**
     * Set the JWT algorithm
     */
    algorithm(algorithm) {
        this._algorithm = algorithm;
        return this;
    }
    /**
     * Set the JWT expiration time (in seconds)
     */
    expiration(expiration) {
        this._expiration = expiration;
        return this;
    }
    /**
     * Execute the configure JWT mutation
     */
    async execute() {
        if (!this._serverId) {
            throw new Error('serverId is required');
        }
        if (!this._secretKey) {
            throw new Error('secretKey is required');
        }
        const query = `
      mutation ConfigureMCPJWT($input: ConfigureMcpJwtInput!) {
        configureMcpJwt(input: $input) {
          id
          serverId
          algorithm
          expiration
          isEnabled
          createdAt
          updatedAt
        }
      }
    `;
        const input = {
            serverId: this._serverId,
            secretKey: this._secretKey,
            algorithm: this._algorithm || 'HS256',
            expiration: this._expiration || 3600, // Default 1 hour
        };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
        return response.configureMcpJwt;
    }
}
// ============================================================================
// Convenience Functions
// ============================================================================
/**
 * Convenience function to create an MCP server
 */
export function createMCPServer(client, name, type) {
    return client.mcp().createServer().name(name).type(type);
}
/**
 * Convenience function to list MCP servers
 */
export function listMCPServers(client) {
    return client.mcp().servers();
}
/**
 * Convenience function to get MCP server health
 */
export async function getMCPServerHealth(client, serverId) {
    return client.mcp().getServerHealth(serverId);
}
/**
 * Convenience function to get a custom MCP server
 */
export async function getCustomMCPServer(client, name, endpoint) {
    return client
        .mcp()
        .createServer()
        .name(name)
        .type(MCPServerType.CUSTOM)
        .config({ endpoint })
        .execute();
}
// ============================================================================
// Usage Examples
// ============================================================================
/**
 * Example usage of the MCP client
 *
 * ```typescript
 * import { Client } from './client.js';
 * import { MCPServerType, createMCPServer } from './mcp.js';
 *
 * const client = Client.builder()
 *   .baseUrl('http://localhost:4000')
 *   .build();
 *
 * // List all MCP servers
 * const servers = await client.mcp().servers().list();
 * console.log(`Found ${servers.servers.length} MCP servers`);
 *
 * // Create a custom MCP server
 * const server = await createMCPServer(client, 'My Server', MCPServerType.CUSTOM)
 *   .config({ endpoint: 'http://localhost:8080' })
 *   .execute();
 *
 * // Configure OAuth
 * const oauthConfig = await client.mcp()
 *   .configureOAuth()
 *   .serverId(server.id)
 *   .provider('google')
 *   .clientId('your-client-id')
 *   .clientSecret('your-client-secret')
 *   .execute();
 *
 * // Get server health
 * const health = await client.mcp().getServerHealth(server.id);
 * console.log(`Server health: ${health.status}`);
 * ```
 */
//# sourceMappingURL=mcp.js.map