/**
 * Model Context Protocol (MCP) Client
 *
 * This module provides functionality for managing MCP servers, handling OAuth/JWT authentication,
 * and managing sessions for the Circuit Breaker workflow automation server.
 */

import { Client } from './client.js';

// ============================================================================
// Types
// ============================================================================

/**
 * MCP Server type enumeration
 */
export enum MCPServerType {
  BUILT_IN = 'BUILT_IN',
  CUSTOM = 'CUSTOM',
  THIRD_PARTY = 'THIRD_PARTY',
}

/**
 * MCP Server status enumeration
 */
export enum MCPServerStatus {
  ACTIVE = 'ACTIVE',
  INACTIVE = 'INACTIVE',
  ERROR = 'ERROR',
  CONNECTING = 'CONNECTING',
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

// ============================================================================
// MCP Client
// ============================================================================

/**
 * MCP client for Model Context Protocol operations
 */
export class MCPClient {
  constructor(private client: Client) {}

  /**
   * Get MCP servers with optional filtering
   */
  servers(): MCPServersBuilder {
    return new MCPServersBuilder(this.client);
  }

  /**
   * Get a specific MCP server by ID
   */
  async getServer(id: string): Promise<MCPServer | null> {
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

    const response = await this.client.graphql<{
      mcpServer: MCPServer | null;
    }>(query, variables);

    return response.mcpServer;
  }

  /**
   * Create a new MCP server
   */
  createServer(): CreateMCPServerBuilder {
    return new CreateMCPServerBuilder(this.client);
  }

  /**
   * Update an existing MCP server
   */
  updateServer(id: string): UpdateMCPServerBuilder {
    return new UpdateMCPServerBuilder(this.client, id);
  }

  /**
   * Delete an MCP server
   */
  async deleteServer(id: string): Promise<{ success: boolean; message: string }> {
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

    const response = await this.client.graphql<{
      deleteMcpServer: {
        success: boolean;
        message: string;
        errorCode?: string;
        data?: any;
      };
    }>(query, variables);

    return {
      success: response.deleteMcpServer.success,
      message: response.deleteMcpServer.message,
    };
  }

  /**
   * Configure OAuth for an MCP server
   */
  configureOAuth(): ConfigureOAuthBuilder {
    return new ConfigureOAuthBuilder(this.client);
  }

  /**
   * Configure JWT authentication for an MCP server
   */
  configureJWT(): ConfigureJWTBuilder {
    return new ConfigureJWTBuilder(this.client);
  }

  /**
   * Get available OAuth providers
   */
  async getOAuthProviders(): Promise<MCPOAuthProvider[]> {
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

    const response = await this.client.graphql<{
      mcpOAuthProviders: MCPOAuthProvider[];
    }>(query);

    return response.mcpOAuthProviders;
  }

  /**
   * Get server capabilities
   */
  async getServerCapabilities(serverId: string): Promise<MCPServerCapabilities | null> {
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

    const response = await this.client.graphql<{
      mcpServerCapabilities: MCPServerCapabilities | null;
    }>(query, variables);

    return response.mcpServerCapabilities;
  }

  /**
   * Get server health status
   */
  async getServerHealth(serverId: string): Promise<MCPServerHealth> {
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

    const response = await this.client.graphql<{
      mcpServerHealth: MCPServerHealth;
    }>(query, variables);

    return response.mcpServerHealth;
  }

  /**
   * Initiate OAuth flow
   */
  async initiateOAuth(serverId: string, userId: string): Promise<MCPOAuthInitiation> {
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

    const response = await this.client.graphql<{
      initiateMcpOAuth: MCPOAuthInitiation;
    }>(query, variables);

    return response.initiateMcpOAuth;
  }

  /**
   * Complete OAuth flow
   */
  async completeOAuth(state: string, code: string): Promise<MCPSession> {
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

    const response = await this.client.graphql<{
      completeMcpOAuth: MCPSession;
    }>(query, variables);

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
  private _type?: MCPServerType;
  private _status?: MCPServerStatus;
  private _tenantId?: string;
  private _pagination?: PaginationInput;

  constructor(private client: Client) {}

  /**
   * Filter by server type
   */
  type(type: MCPServerType): this {
    this._type = type;
    return this;
  }

  /**
   * Filter by server status
   */
  status(status: MCPServerStatus): this {
    this._status = status;
    return this;
  }

  /**
   * Filter by tenant ID
   */
  tenantId(tenantId: string): this {
    this._tenantId = tenantId;
    return this;
  }

  /**
   * Set pagination parameters
   */
  pagination(pagination: PaginationInput): this {
    this._pagination = pagination;
    return this;
  }

  /**
   * Execute the query
   */
  async list(): Promise<MCPServerConnection> {
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

    const response = await this.client.graphql<{
      mcpServers?: MCPServerConnection;
      mcpServersByTenant?: MCPServerConnection;
    }>(query, variables);

    return response.mcpServers || response.mcpServersByTenant!;
  }
}

/**
 * Builder for creating MCP servers
 */
export class CreateMCPServerBuilder {
  private _name?: string;
  private _description?: string;
  private _type?: MCPServerType;
  private _config?: Record<string, any>;
  private _tenantId?: string;

  constructor(private client: Client) {}

  /**
   * Set the server name
   */
  name(name: string): this {
    this._name = name;
    return this;
  }

  /**
   * Set the server description
   */
  description(description: string): this {
    this._description = description;
    return this;
  }

  /**
   * Set the server type
   */
  type(type: MCPServerType): this {
    this._type = type;
    return this;
  }

  /**
   * Set the server configuration
   */
  config(config: Record<string, any>): this {
    this._config = config;
    return this;
  }

  /**
   * Set the tenant ID
   */
  tenantId(tenantId: string): this {
    this._tenantId = tenantId;
    return this;
  }

  /**
   * Execute the create server mutation
   */
  async execute(): Promise<MCPServer> {
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

    const input: CreateMCPServerInput = {
      name: this._name,
      description: this._description,
      type: this._type,
      config: this._config || {},
      tenantId: this._tenantId,
    };

    const variables = { input };

    const response = await this.client.graphql<{
      createMcpServer: MCPServer;
    }>(query, variables);

    return response.createMcpServer;
  }
}

/**
 * Builder for updating MCP servers
 */
export class UpdateMCPServerBuilder {
  private _name?: string;
  private _description?: string;
  private _status?: MCPServerStatus;
  private _config?: Record<string, any>;

  constructor(private client: Client, private id: string) {}

  /**
   * Set the server name
   */
  name(name: string): this {
    this._name = name;
    return this;
  }

  /**
   * Set the server description
   */
  description(description: string): this {
    this._description = description;
    return this;
  }

  /**
   * Set the server status
   */
  status(status: MCPServerStatus): this {
    this._status = status;
    return this;
  }

  /**
   * Set the server configuration
   */
  config(config: Record<string, any>): this {
    this._config = config;
    return this;
  }

  /**
   * Execute the update server mutation
   */
  async execute(): Promise<MCPServer> {
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

    const input: UpdateMCPServerInput = {
      name: this._name,
      description: this._description,
      status: this._status,
      config: this._config,
    };

    const variables = { id: this.id, input };

    const response = await this.client.graphql<{
      updateMcpServer: MCPServer;
    }>(query, variables);

    return response.updateMcpServer;
  }
}

/**
 * Builder for configuring OAuth
 */
export class ConfigureOAuthBuilder {
  private _serverId?: string;
  private _provider?: string;
  private _clientId?: string;
  private _clientSecret?: string;
  private _scopes?: string[];
  private _redirectUri?: string;

  constructor(private client: Client) {}

  /**
   * Set the server ID
   */
  serverId(serverId: string): this {
    this._serverId = serverId;
    return this;
  }

  /**
   * Set the OAuth provider
   */
  provider(provider: string): this {
    this._provider = provider;
    return this;
  }

  /**
   * Set the client ID
   */
  clientId(clientId: string): this {
    this._clientId = clientId;
    return this;
  }

  /**
   * Set the client secret
   */
  clientSecret(clientSecret: string): this {
    this._clientSecret = clientSecret;
    return this;
  }

  /**
   * Set the OAuth scopes
   */
  scopes(scopes: string[]): this {
    this._scopes = scopes;
    return this;
  }

  /**
   * Set the redirect URI
   */
  redirectUri(redirectUri: string): this {
    this._redirectUri = redirectUri;
    return this;
  }

  /**
   * Execute the configure OAuth mutation
   */
  async execute(): Promise<MCPOAuthConfig> {
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

    const input: ConfigureOAuthInput = {
      serverId: this._serverId,
      provider: this._provider,
      clientId: this._clientId,
      clientSecret: this._clientSecret,
      scopes: this._scopes,
      redirectUri: this._redirectUri,
    };

    const variables = { input };

    const response = await this.client.graphql<{
      configureMcpOAuth: MCPOAuthConfig;
    }>(query, variables);

    return response.configureMcpOAuth;
  }
}

/**
 * Builder for configuring JWT
 */
export class ConfigureJWTBuilder {
  private _serverId?: string;
  private _secretKey?: string;
  private _algorithm?: string;
  private _expiration?: number;

  constructor(private client: Client) {}

  /**
   * Set the server ID
   */
  serverId(serverId: string): this {
    this._serverId = serverId;
    return this;
  }

  /**
   * Set the JWT secret key
   */
  secretKey(secretKey: string): this {
    this._secretKey = secretKey;
    return this;
  }

  /**
   * Set the JWT algorithm
   */
  algorithm(algorithm: string): this {
    this._algorithm = algorithm;
    return this;
  }

  /**
   * Set the JWT expiration time (in seconds)
   */
  expiration(expiration: number): this {
    this._expiration = expiration;
    return this;
  }

  /**
   * Execute the configure JWT mutation
   */
  async execute(): Promise<MCPJWTConfig> {
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

    const input: ConfigureJWTInput = {
      serverId: this._serverId,
      secretKey: this._secretKey,
      algorithm: this._algorithm || 'HS256',
      expiration: this._expiration || 3600, // Default 1 hour
    };

    const variables = { input };

    const response = await this.client.graphql<{
      configureMcpJwt: MCPJWTConfig;
    }>(query, variables);

    return response.configureMcpJwt;
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Convenience function to create an MCP server
 */
export function createMCPServer(
  client: Client,
  name: string,
  type: MCPServerType,
): CreateMCPServerBuilder {
  return client.mcp().createServer().name(name).type(type);
}

/**
 * Convenience function to list MCP servers
 */
export function listMCPServers(client: Client): MCPServersBuilder {
  return client.mcp().servers();
}

/**
 * Convenience function to get MCP server health
 */
export async function getMCPServerHealth(
  client: Client,
  serverId: string,
): Promise<MCPServerHealth> {
  return client.mcp().getServerHealth(serverId);
}

/**
 * Convenience function to get a custom MCP server
 */
export async function getCustomMCPServer(
  client: Client,
  name: string,
  endpoint: string,
): Promise<MCPServer> {
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
