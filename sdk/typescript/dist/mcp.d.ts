/**
 * Model Context Protocol (MCP) Client
 *
 * This module provides functionality for managing MCP servers, handling OAuth/JWT authentication,
 * and managing sessions for the Circuit Breaker workflow automation server.
 */
import { Client } from './client.js';
/**
 * MCP Server type enumeration
 */
export declare enum MCPServerType {
    BUILT_IN = "BUILT_IN",
    CUSTOM = "CUSTOM",
    THIRD_PARTY = "THIRD_PARTY"
}
/**
 * MCP Server status enumeration
 */
export declare enum MCPServerStatus {
    ACTIVE = "ACTIVE",
    INACTIVE = "INACTIVE",
    ERROR = "ERROR",
    CONNECTING = "CONNECTING"
}
/**
 * MCP Server information
 */
export interface MCPServer {
    /** Server identifier */
    id: string;
    /** Server name */
    name: string;
    /** Server description */
    description?: string;
    /** Server type */
    type: MCPServerType;
    /** Server status */
    status: MCPServerStatus;
    /** Server configuration */
    config: Record<string, any>;
    /** Server capabilities */
    capabilities?: Record<string, any>;
    /** Server health information */
    health?: Record<string, any>;
    /** Tenant identifier */
    tenantId?: string;
    /** Creation timestamp */
    createdAt: string;
    /** Update timestamp */
    updatedAt: string;
}
/**
 * MCP Server connection wrapper
 */
export interface MCPServerConnection {
    /** List of servers */
    servers: MCPServer[];
    /** Pagination information */
    pageInfo: {
        hasNextPage: boolean;
        hasPreviousPage: boolean;
        startCursor?: string;
        endCursor?: string;
        totalCount?: number;
    };
    /** Total count */
    totalCount: number;
}
/**
 * OAuth provider information
 */
export interface MCPOAuthProvider {
    /** Provider identifier */
    id: string;
    /** Provider name */
    name: string;
    /** Provider type */
    type: string;
    /** Provider configuration */
    config: Record<string, any>;
    /** Whether provider is enabled */
    isEnabled: boolean;
}
/**
 * Server capabilities
 */
export interface MCPServerCapabilities {
    /** Available tools */
    tools?: string[];
    /** Available resources */
    resources?: string[];
    /** Available prompts */
    prompts?: string[];
    /** Sampling configuration */
    sampling?: Record<string, any>;
}
/**
 * Server health status
 */
export interface MCPServerHealth {
    /** Health status */
    status: string;
    /** Status message */
    message?: string;
    /** Last check timestamp */
    lastCheck?: string;
    /** Response time in milliseconds */
    responseTime?: number;
    /** Additional health details */
    details?: Record<string, any>;
}
/**
 * OAuth initiation response
 */
export interface MCPOAuthInitiation {
    /** Authorization URL */
    authUrl: string;
    /** OAuth state parameter */
    state: string;
    /** PKCE code challenge */
    codeChallenge?: string;
    /** Expiration timestamp */
    expiresAt: string;
}
/**
 * MCP Session
 */
export interface MCPSession {
    /** Session identifier */
    id: string;
    /** Server identifier */
    serverId: string;
    /** User identifier */
    userId: string;
    /** Session status */
    status: string;
    /** Access token */
    accessToken?: string;
    /** Refresh token */
    refreshToken?: string;
    /** Token expiration */
    expiresAt?: string;
    /** Creation timestamp */
    createdAt: string;
    /** Update timestamp */
    updatedAt: string;
}
/**
 * OAuth configuration
 */
export interface MCPOAuthConfig {
    /** Configuration identifier */
    id: string;
    /** Server identifier */
    serverId: string;
    /** OAuth provider */
    provider: string;
    /** Client identifier */
    clientId: string;
    /** OAuth scopes */
    scopes?: string[];
    /** Redirect URI */
    redirectUri?: string;
    /** Whether configuration is enabled */
    isEnabled: boolean;
    /** Creation timestamp */
    createdAt: string;
    /** Update timestamp */
    updatedAt: string;
}
/**
 * JWT configuration
 */
export interface MCPJWTConfig {
    /** Configuration identifier */
    id: string;
    /** Server identifier */
    serverId: string;
    /** JWT algorithm */
    algorithm: string;
    /** Token expiration time (seconds) */
    expiration: number;
    /** Whether configuration is enabled */
    isEnabled: boolean;
    /** Creation timestamp */
    createdAt: string;
    /** Update timestamp */
    updatedAt: string;
}
/**
 * Input for creating MCP servers
 */
export interface CreateMCPServerInput {
    /** Server name */
    name: string;
    /** Server description */
    description?: string;
    /** Server type */
    type: MCPServerType;
    /** Server configuration */
    config: Record<string, any>;
    /** Tenant identifier */
    tenantId?: string;
}
/**
 * Input for updating MCP servers
 */
export interface UpdateMCPServerInput {
    /** Server name */
    name?: string;
    /** Server description */
    description?: string;
    /** Server status */
    status?: MCPServerStatus;
    /** Server configuration */
    config?: Record<string, any>;
}
/**
 * Input for configuring OAuth
 */
export interface ConfigureOAuthInput {
    /** Server identifier */
    serverId: string;
    /** OAuth provider */
    provider: string;
    /** Client identifier */
    clientId: string;
    /** Client secret */
    clientSecret: string;
    /** OAuth scopes */
    scopes?: string[];
    /** Redirect URI */
    redirectUri?: string;
}
/**
 * Input for configuring JWT
 */
export interface ConfigureJWTInput {
    /** Server identifier */
    serverId: string;
    /** JWT secret key */
    secretKey: string;
    /** JWT algorithm */
    algorithm?: string;
    /** Token expiration time (seconds) */
    expiration?: number;
}
/**
 * Pagination input parameters
 */
export interface PaginationInput {
    /** Number of items to return */
    first?: number;
    /** Cursor to start after */
    after?: string;
    /** Number of items to return from the end */
    last?: number;
    /** Cursor to start before */
    before?: string;
}
/**
 * MCP client for Model Context Protocol operations
 */
export declare class MCPClient {
    private client;
    constructor(client: Client);
    /**
     * Get MCP servers with optional filtering
     */
    servers(): MCPServersBuilder;
    /**
     * Get a specific MCP server by ID
     */
    getServer(id: string): Promise<MCPServer | null>;
    /**
     * Create a new MCP server
     */
    createServer(): CreateMCPServerBuilder;
    /**
     * Update an existing MCP server
     */
    updateServer(id: string): UpdateMCPServerBuilder;
    /**
     * Delete an MCP server
     */
    deleteServer(id: string): Promise<{
        success: boolean;
        message: string;
    }>;
    /**
     * Configure OAuth for an MCP server
     */
    configureOAuth(): ConfigureOAuthBuilder;
    /**
     * Configure JWT authentication for an MCP server
     */
    configureJWT(): ConfigureJWTBuilder;
    /**
     * Get available OAuth providers
     */
    getOAuthProviders(): Promise<MCPOAuthProvider[]>;
    /**
     * Get server capabilities
     */
    getServerCapabilities(serverId: string): Promise<MCPServerCapabilities | null>;
    /**
     * Get server health status
     */
    getServerHealth(serverId: string): Promise<MCPServerHealth>;
    /**
     * Initiate OAuth flow
     */
    initiateOAuth(serverId: string, userId: string): Promise<MCPOAuthInitiation>;
    /**
     * Complete OAuth flow
     */
    completeOAuth(state: string, code: string): Promise<MCPSession>;
}
/**
 * Builder for MCP servers queries
 */
export declare class MCPServersBuilder {
    private client;
    private _type?;
    private _status?;
    private _tenantId?;
    private _pagination?;
    constructor(client: Client);
    /**
     * Filter by server type
     */
    type(type: MCPServerType): this;
    /**
     * Filter by server status
     */
    status(status: MCPServerStatus): this;
    /**
     * Filter by tenant ID
     */
    tenantId(tenantId: string): this;
    /**
     * Set pagination parameters
     */
    pagination(pagination: PaginationInput): this;
    /**
     * Execute the query
     */
    list(): Promise<MCPServerConnection>;
}
/**
 * Builder for creating MCP servers
 */
export declare class CreateMCPServerBuilder {
    private client;
    private _name?;
    private _description?;
    private _type?;
    private _config?;
    private _tenantId?;
    constructor(client: Client);
    /**
     * Set the server name
     */
    name(name: string): this;
    /**
     * Set the server description
     */
    description(description: string): this;
    /**
     * Set the server type
     */
    type(type: MCPServerType): this;
    /**
     * Set the server configuration
     */
    config(config: Record<string, any>): this;
    /**
     * Set the tenant ID
     */
    tenantId(tenantId: string): this;
    /**
     * Execute the create server mutation
     */
    execute(): Promise<MCPServer>;
}
/**
 * Builder for updating MCP servers
 */
export declare class UpdateMCPServerBuilder {
    private client;
    private id;
    private _name?;
    private _description?;
    private _status?;
    private _config?;
    constructor(client: Client, id: string);
    /**
     * Set the server name
     */
    name(name: string): this;
    /**
     * Set the server description
     */
    description(description: string): this;
    /**
     * Set the server status
     */
    status(status: MCPServerStatus): this;
    /**
     * Set the server configuration
     */
    config(config: Record<string, any>): this;
    /**
     * Execute the update server mutation
     */
    execute(): Promise<MCPServer>;
}
/**
 * Builder for configuring OAuth
 */
export declare class ConfigureOAuthBuilder {
    private client;
    private _serverId?;
    private _provider?;
    private _clientId?;
    private _clientSecret?;
    private _scopes?;
    private _redirectUri?;
    constructor(client: Client);
    /**
     * Set the server ID
     */
    serverId(serverId: string): this;
    /**
     * Set the OAuth provider
     */
    provider(provider: string): this;
    /**
     * Set the client ID
     */
    clientId(clientId: string): this;
    /**
     * Set the client secret
     */
    clientSecret(clientSecret: string): this;
    /**
     * Set the OAuth scopes
     */
    scopes(scopes: string[]): this;
    /**
     * Set the redirect URI
     */
    redirectUri(redirectUri: string): this;
    /**
     * Execute the configure OAuth mutation
     */
    execute(): Promise<MCPOAuthConfig>;
}
/**
 * Builder for configuring JWT
 */
export declare class ConfigureJWTBuilder {
    private client;
    private _serverId?;
    private _secretKey?;
    private _algorithm?;
    private _expiration?;
    constructor(client: Client);
    /**
     * Set the server ID
     */
    serverId(serverId: string): this;
    /**
     * Set the JWT secret key
     */
    secretKey(secretKey: string): this;
    /**
     * Set the JWT algorithm
     */
    algorithm(algorithm: string): this;
    /**
     * Set the JWT expiration time (in seconds)
     */
    expiration(expiration: number): this;
    /**
     * Execute the configure JWT mutation
     */
    execute(): Promise<MCPJWTConfig>;
}
/**
 * Convenience function to create an MCP server
 */
export declare function createMCPServer(client: Client, name: string, type: MCPServerType): CreateMCPServerBuilder;
/**
 * Convenience function to list MCP servers
 */
export declare function listMCPServers(client: Client): MCPServersBuilder;
/**
 * Convenience function to get MCP server health
 */
export declare function getMCPServerHealth(client: Client, serverId: string): Promise<MCPServerHealth>;
/**
 * Convenience function to get a custom MCP server
 */
export declare function getCustomMCPServer(client: Client, name: string, endpoint: string): Promise<MCPServer>;
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
//# sourceMappingURL=mcp.d.ts.map