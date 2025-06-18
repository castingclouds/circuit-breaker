const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const fs = require("fs");
const path = require("path");

/**
 * MCP Examples with Real Test Key Loading
 *
 * This file demonstrates MCP server management with OAuth and JWT authentication.
 * For JWT examples, we load actual RSA key pairs from the test_keys directory
 * instead of using hardcoded placeholder keys.
 *
 * Key Files:
 * - test_keys/test.pem: RSA private key (PKCS#8 format)
 * - test_keys/test.pub: RSA public key (X.509 SubjectPublicKeyInfo format)
 *
 * ⚠️  IMPORTANT: These are test keys only - never use in production!
 */

// Configuration constants
const CB_URL = "https://2bc3-76-182-171-196.ngrok-free.app";

// Load test keys from test_keys directory
const testKeysDir = path.join(__dirname, "../../test_keys");
const privateKeyPath = path.join(testKeysDir, "test.pem");
const publicKeyPath = path.join(testKeysDir, "test.pub");

let testPrivateKey, testPublicKey;
try {
  testPrivateKey = fs.readFileSync(privateKeyPath, "utf8");
  testPublicKey = fs.readFileSync(publicKeyPath, "utf8");
  console.log("✅ Successfully loaded test keys from:", testKeysDir);
} catch (error) {
  console.warn("⚠️  Warning: Could not load test keys:", error.message);
  console.warn("Using placeholder keys for examples (not for production!)");
  testPrivateKey =
    "-----BEGIN PRIVATE KEY-----\nPLACEHOLDER_PRIVATE_KEY\n-----END PRIVATE KEY-----";
  testPublicKey =
    "-----BEGIN PUBLIC KEY-----\nPLACEHOLDER_PUBLIC_KEY\n-----END PUBLIC KEY-----";
}

/**
 * Validate that test keys are properly formatted
 */
function validateTestKeys() {
  const privateKeyValid =
    testPrivateKey.includes("-----BEGIN PRIVATE KEY-----") &&
    testPrivateKey.includes("-----END PRIVATE KEY-----") &&
    !testPrivateKey.includes("PLACEHOLDER");

  const publicKeyValid =
    testPublicKey.includes("-----BEGIN PUBLIC KEY-----") &&
    testPublicKey.includes("-----END PUBLIC KEY-----") &&
    !testPublicKey.includes("PLACEHOLDER");

  return {
    valid: privateKeyValid && publicKeyValid,
    privateKeyValid,
    publicKeyValid,
    usingRealKeys: !testPrivateKey.includes("PLACEHOLDER"),
  };
}

// Load the MCP schema
const mcpSchema = loadSchemaSync(path.join(__dirname, "../mcp.graphql"), {
  loaders: [new GraphQLFileLoader()],
});

// Load GraphQL operations
const operationsFile = path.join(__dirname, "../operations/mcp.graphql");
const operations = fs.readFileSync(operationsFile, "utf8");

// Parse operations to extract individual queries/mutations
const operationMap = {};
const operationRegex =
  /(query|mutation|subscription)\s+(\w+)[\s\S]*?(?=(?:query|mutation|subscription)\s+\w+|$)/g;
let match;
while ((match = operationRegex.exec(operations)) !== null) {
  operationMap[match[2]] = match[0].trim();
}

// GraphQL client setup
const endpoint = "http://localhost:4000/graphql";
const client = new GraphQLClient(endpoint);

/**
 * MCP Examples
 * These examples demonstrate how to use the Model Context Protocol server management operations
 * defined in ../mcp.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * Get all MCP servers with filtering
 */
async function getMcpServers(type = null, status = null) {
  const query = operationMap.GetMcpServers;

  try {
    const data = await client.request(query, {
      type,
      status,
      pagination: {
        first: 50,
        after: null,
      },
    });
    console.log("MCP servers:", JSON.stringify(data, null, 2));
    return data.mcpServers;
  } catch (error) {
    console.error("Error fetching MCP servers:", error);
    throw error;
  }
}

/**
 * Get a specific MCP server by ID
 */
async function getMcpServer(serverId) {
  const query = operationMap.GetMcpServer;

  try {
    const data = await client.request(query, { id: serverId });
    console.log("MCP server details:", JSON.stringify(data, null, 2));
    return data.mcpServer;
  } catch (error) {
    console.error("Error fetching MCP server:", error);
    throw error;
  }
}

/**
 * Get MCP servers for a specific tenant
 */
async function getMcpServersByTenant(tenantId) {
  const query = operationMap.GetMcpServersByTenant;

  try {
    const data = await client.request(query, {
      tenantId,
      pagination: {
        first: 20,
        after: null,
      },
    });
    console.log("Tenant MCP servers:", JSON.stringify(data, null, 2));
    return data.mcpServersByTenant;
  } catch (error) {
    console.error("Error fetching tenant MCP servers:", error);
    throw error;
  }
}

/**
 * Get available OAuth providers
 */
async function getMcpOAuthProviders() {
  const query = operationMap.GetMcpOAuthProviders;

  try {
    const data = await client.request(query);
    console.log("OAuth providers:", JSON.stringify(data, null, 2));
    return data.mcpOAuthProviders;
  } catch (error) {
    console.error("Error fetching OAuth providers:", error);
    throw error;
  }
}

/**
 * Get MCP server capabilities
 */
async function getMcpServerCapabilities(serverId) {
  const query = operationMap.GetMcpServerCapabilities;

  try {
    const data = await client.request(query, { serverId });
    console.log("Server capabilities:", JSON.stringify(data, null, 2));
    return data.mcpServerCapabilities;
  } catch (error) {
    console.error("Error fetching server capabilities:", error);
    throw error;
  }
}

/**
 * Get MCP server health status
 */
async function getMcpServerHealth(serverId) {
  const query = operationMap.GetMcpServerHealth;

  try {
    const data = await client.request(query, { serverId });
    console.log("Server health:", JSON.stringify(data, null, 2));
    return data.mcpServerHealth;
  } catch (error) {
    console.error("Error fetching server health:", error);
    throw error;
  }
}

/**
 * Get active MCP sessions
 */
async function getMcpSessions(userId = null, serverId = null) {
  const query = operationMap.GetMcpSessions;

  try {
    const data = await client.request(query, {
      userId,
      serverId,
      pagination: {
        first: 30,
        after: null,
      },
    });
    console.log("MCP sessions:", JSON.stringify(data, null, 2));
    return data.mcpSessions;
  } catch (error) {
    console.error("Error fetching MCP sessions:", error);
    throw error;
  }
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Create GitLab OAuth MCP server (from conversation context)
 */
async function createGitLabMcpServer() {
  const mutation = operationMap.CreateMcpServer;

  const serverInput = {
    name: "GitLab Remote MCP Server",
    description:
      "Multi-tenant MCP server with GitLab OAuth authentication for repository access",
    type: "REMOTE",
    tenantId: "tenant-gitlab-001",
    config: {
      endpoint: CB_URL,
      timeoutSeconds: 30,
      maxConnections: 100,
      ssl: {
        verify: true,
        caCert: null,
        clientCert: null,
        clientKey: null,
      },
      retry: {
        maxAttempts: 3,
        initialDelayMs: 1000,
        maxDelayMs: 30000,
        backoffMultiplier: 2.0,
        jitterFactor: 0.1,
      },
      rateLimit: {
        requestsPerSecond: 50,
        burstSize: 100,
        windowSeconds: 60,
      },
      headers: {
        "User-Agent": "Circuit-Breaker-MCP/1.0",
        Accept: "application/json",
      },
    },
    auth: {
      oauth: {
        providerId: "gitlab",
        clientId:
          "7b0f347f26b4fe62313cd8a627e38193f2b209365ed3398d44fe02e69972a1eb",
        clientSecret:
          "gloas-c2004e0cc0a3f7465c569db45e23a24aca734ce2316af6f903060479857d1226",
        scopes: ["api", "read_user", "read_repository"],
        redirectUri: `${CB_URL}/oauth/callback`,
        additionalParams: {
          response_type: "code",
          approval_prompt: "auto",
        },
        refreshConfig: {
          autoRefresh: true,
          refreshThresholdSeconds: 300,
          maxRefreshAttempts: 3,
          refreshRetryDelaySeconds: 5,
        },
      },
    },
    metadata: {
      provider: "gitlab",
      version: "v1.0",
      region: "us-east-1",
      environment: "production",
    },
    tags: ["gitlab", "oauth", "remote", "git", "repository"],
    enabled: true,
  };

  try {
    const data = await client.request(mutation, { input: serverInput });
    console.log("Created GitLab MCP server:", JSON.stringify(data, null, 2));
    return data.createMcpServer;
  } catch (error) {
    console.error("Error creating GitLab MCP server:", error);
    throw error;
  }
}

/**
 * Create GitHub OAuth MCP server
 */
async function createGitHubMcpServer() {
  const mutation = operationMap.CreateMcpServer;

  const serverInput = {
    name: "GitHub Enterprise MCP Server",
    description: "MCP server for GitHub Enterprise integration with OAuth",
    type: "REMOTE",
    tenantId: "tenant-github-001",
    config: {
      endpoint: "https://github-mcp.example.com",
      timeoutSeconds: 45,
      maxConnections: 150,
      ssl: {
        verify: true,
      },
      retry: {
        maxAttempts: 5,
        initialDelayMs: 500,
        maxDelayMs: 15000,
        backoffMultiplier: 1.5,
        jitterFactor: 0.2,
      },
      rateLimit: {
        requestsPerSecond: 100,
        burstSize: 200,
        windowSeconds: 60,
      },
    },
    auth: {
      oauth: {
        providerId: "github",
        clientId: "Iv1.a629723fb4c09cd0",
        clientSecret: "github_client_secret_example_12345",
        scopes: ["repo", "user", "admin:org"],
        redirectUri: "https://github-mcp.example.com/oauth/callback",
        refreshConfig: {
          autoRefresh: true,
          refreshThresholdSeconds: 600,
          maxRefreshAttempts: 5,
          refreshRetryDelaySeconds: 10,
        },
      },
    },
    metadata: {
      provider: "github",
      enterprise: true,
      version: "v2.1",
    },
    tags: ["github", "oauth", "enterprise", "git"],
    enabled: true,
  };

  try {
    const data = await client.request(mutation, { input: serverInput });
    console.log("Created GitHub MCP server:", JSON.stringify(data, null, 2));
    return data.createMcpServer;
  } catch (error) {
    console.error("Error creating GitHub MCP server:", error);
    throw error;
  }
}

/**
 * Create JWT-based local MCP server
 */
async function createJwtMcpServer() {
  const mutation = operationMap.CreateMcpServer;

  // Validate test keys before using them
  const keyValidation = validateTestKeys();
  if (!keyValidation.valid) {
    console.warn("⚠️  Warning: Invalid test keys detected");
    console.warn("Private key valid:", keyValidation.privateKeyValid);
    console.warn("Public key valid:", keyValidation.publicKeyValid);
    console.warn("Using real keys:", keyValidation.usingRealKeys);
  }

  const serverInput = {
    name: "Local JWT MCP Server",
    description: "Local MCP server with JWT authentication for internal tools",
    type: "LOCAL",
    tenantId: "tenant-internal-001",
    config: {
      endpoint: "http://localhost:8080/mcp",
      timeoutSeconds: 15,
      maxConnections: 50,
      retry: {
        maxAttempts: 2,
        initialDelayMs: 250,
        maxDelayMs: 5000,
        backoffMultiplier: 2.0,
        jitterFactor: 0.05,
      },
      rateLimit: {
        requestsPerSecond: 200,
        burstSize: 300,
        windowSeconds: 30,
      },
    },
    auth: {
      jwt: {
        issuer: "circuit-breaker-mcp",
        audience: "internal-tools",
        publicKey: testPublicKey,
        privateKey: testPrivateKey,
        algorithm: "RS256",
        expirationSeconds: 3600,
        customClaims: {
          tenant: "internal",
          permissions: ["read", "write", "admin"],
        },
      },
    },
    metadata: {
      provider: "internal",
      type: "jwt",
      version: "v1.0",
    },
    tags: ["local", "jwt", "internal", "tools"],
    enabled: true,
  };

  try {
    const data = await client.request(mutation, { input: serverInput });
    console.log("Created JWT MCP server:", JSON.stringify(data, null, 2));
    return data.createMcpServer;
  } catch (error) {
    console.error("Error creating JWT MCP server:", error);
    throw error;
  }
}

/**
 * Update MCP server configuration
 */
async function updateMcpServer(serverId) {
  const mutation = operationMap.UpdateMcpServer;

  const updateInput = {
    name: "Updated GitLab MCP Server",
    description: "Updated multi-tenant MCP server with enhanced GitLab OAuth",
    config: {
      timeoutSeconds: 60,
      maxConnections: 200,
      rateLimit: {
        requestsPerSecond: 75,
        burstSize: 150,
        windowSeconds: 60,
      },
    },
    metadata: {
      provider: "gitlab",
      version: "v1.1",
      region: "us-east-1",
      environment: "production",
      lastUpdated: new Date().toISOString(),
    },
    tags: ["gitlab", "oauth", "remote", "git", "repository", "updated"],
    enabled: true,
  };

  try {
    const data = await client.request(mutation, {
      id: serverId,
      input: updateInput,
    });
    console.log("Updated MCP server:", JSON.stringify(data, null, 2));
    return data.updateMcpServer;
  } catch (error) {
    console.error("Error updating MCP server:", error);
    throw error;
  }
}

/**
 * Configure OAuth for existing server
 */
async function configureMcpOAuth(serverId) {
  const mutation = operationMap.ConfigureMcpOAuth;

  const oauthInput = {
    serverId: serverId,
    config: {
      providerId: "gitlab",
      clientId:
        "7b0f347f26b4fe62313cd8a627e38193f2b209365ed3398d44fe02e69972a1eb",
      clientSecret:
        "gloas-c2004e0cc0a3f7465c569db45e23a24aca734ce2316af6f903060479857d1226",
      scopes: ["api", "read_user", "read_repository", "write_repository"],
      redirectUri: `${CB_URL}/oauth/callback`,
      additionalParams: {
        response_type: "code",
        approval_prompt: "auto",
        access_type: "offline",
      },
      refreshConfig: {
        autoRefresh: true,
        refreshThresholdSeconds: 300,
        maxRefreshAttempts: 3,
        refreshRetryDelaySeconds: 5,
      },
    },
  };

  try {
    const data = await client.request(mutation, { input: oauthInput });
    console.log("Configured OAuth:", JSON.stringify(data, null, 2));
    return data.configureMcpOAuth;
  } catch (error) {
    console.error("Error configuring OAuth:", error);
    throw error;
  }
}

/**
 * Configure JWT for existing server
 */
async function configureMcpJwt(serverId) {
  const mutation = operationMap.ConfigureMcpJwt;

  // Validate test keys before using them
  const keyValidation = validateTestKeys();
  if (!keyValidation.valid) {
    console.warn("⚠️  Warning: Invalid test keys detected");
    console.warn("Private key valid:", keyValidation.privateKeyValid);
    console.warn("Public key valid:", keyValidation.publicKeyValid);
    console.warn("Using real keys:", keyValidation.usingRealKeys);
  }

  const jwtInput = {
    serverId: serverId,
    config: {
      issuer: "circuit-breaker-mcp-v2",
      audience: "internal-services",
      publicKey: testPublicKey,
      privateKey: testPrivateKey,
      algorithm: "RS256",
      expirationSeconds: 7200,
      customClaims: {
        tenant: "multi-tenant",
        permissions: ["read", "write"],
        scope: "mcp:full-access",
      },
    },
  };

  try {
    const data = await client.request(mutation, { input: jwtInput });
    console.log("Configured JWT:", JSON.stringify(data, null, 2));
    return data.configureMcpJwt;
  } catch (error) {
    console.error("Error configuring JWT:", error);
    throw error;
  }
}

/**
 * Initiate OAuth flow
 */
async function initiateMcpOAuth(serverId, userId) {
  const mutation = operationMap.InitiateMcpOAuth;

  const initiateInput = {
    serverId: serverId,
    userId: userId,
    redirectUri: `${CB_URL}/oauth/callback`,
    additionalParams: {
      state: `user_${userId}_server_${serverId}`,
      scope: "api read_user read_repository",
    },
  };

  try {
    const data = await client.request(mutation, { input: initiateInput });
    console.log("OAuth initiation:", JSON.stringify(data, null, 2));
    return data.initiateMcpOAuth;
  } catch (error) {
    console.error("Error initiating OAuth:", error);
    throw error;
  }
}

/**
 * Complete OAuth flow
 */
async function completeMcpOAuth(serverId, userId, authCode, state) {
  const mutation = operationMap.CompleteMcpOAuth;

  const completeInput = {
    serverId: serverId,
    userId: userId,
    code: authCode,
    state: state,
    codeVerifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", // PKCE code verifier
  };

  try {
    const data = await client.request(mutation, { input: completeInput });
    console.log("OAuth completion:", JSON.stringify(data, null, 2));
    return data.completeMcpOAuth;
  } catch (error) {
    console.error("Error completing OAuth:", error);
    throw error;
  }
}

/**
 * Authenticate with JWT
 */
async function authenticateMcpJwt(serverId, userId, jwtToken) {
  const mutation = operationMap.AuthenticateMcpJwt;

  const authInput = {
    serverId: serverId,
    userId: userId,
    token: jwtToken,
    metadata: {
      userAgent: "Circuit-Breaker-Client/1.0",
      ipAddress: "192.168.1.100",
      deviceId: "device-12345",
    },
  };

  try {
    const data = await client.request(mutation, { input: authInput });
    console.log("JWT authentication:", JSON.stringify(data, null, 2));
    return data.authenticateMcpJwt;
  } catch (error) {
    console.error("Error authenticating with JWT:", error);
    throw error;
  }
}

/**
 * Register server capabilities
 */
async function registerMcpCapabilities(serverId) {
  const mutation = operationMap.RegisterMcpCapabilities;

  const capabilitiesInput = {
    serverId: serverId,
    tools: [
      {
        name: "git_clone",
        description: "Clone a Git repository to local filesystem",
        inputSchema: {
          type: "object",
          properties: {
            url: { type: "string", description: "Repository URL" },
            branch: { type: "string", description: "Branch to clone" },
            depth: { type: "integer", description: "Clone depth" },
          },
          required: ["url"],
        },
        category: "git",
        metadata: {
          version: "1.0",
          author: "GitLab MCP Team",
        },
      },
      {
        name: "git_commit",
        description: "Create a new commit with changes",
        inputSchema: {
          type: "object",
          properties: {
            message: { type: "string", description: "Commit message" },
            files: {
              type: "array",
              items: { type: "string" },
              description: "Files to commit",
            },
            author: { type: "object", description: "Author information" },
          },
          required: ["message", "files"],
        },
        category: "git",
        metadata: {
          version: "1.0",
        },
      },
      {
        name: "merge_request_create",
        description: "Create a new merge request",
        inputSchema: {
          type: "object",
          properties: {
            title: { type: "string", description: "MR title" },
            description: { type: "string", description: "MR description" },
            source_branch: { type: "string", description: "Source branch" },
            target_branch: { type: "string", description: "Target branch" },
          },
          required: ["title", "source_branch", "target_branch"],
        },
        category: "gitlab",
        metadata: {
          version: "1.0",
        },
      },
    ],
    resources: [
      {
        uri: "gitlab://projects",
        name: "GitLab Projects",
        description: "Access to GitLab projects and repositories",
        type: "collection",
        mimeType: "application/json",
        metadata: {
          provider: "gitlab",
          version: "v4",
        },
      },
      {
        uri: "gitlab://issues",
        name: "GitLab Issues",
        description: "Access to GitLab issues and tracking",
        type: "collection",
        mimeType: "application/json",
        metadata: {
          provider: "gitlab",
        },
      },
    ],
    prompts: [
      {
        name: "code_review",
        description: "Generate code review comments for merge requests",
        template:
          "Review the following code changes:\n\n{diff}\n\nProvide constructive feedback focusing on:\n- Code quality\n- Security concerns\n- Performance implications\n- Best practices",
        parameters: [
          {
            name: "diff",
            description: "Git diff of the changes",
            type: "string",
            required: true,
          },
          {
            name: "language",
            description: "Programming language",
            type: "string",
            required: false,
            defaultValue: "auto-detect",
          },
        ],
        category: "code-review",
        metadata: {
          version: "1.0",
        },
      },
      {
        name: "commit_message",
        description: "Generate conventional commit messages",
        template:
          "Generate a conventional commit message for the following changes:\n\n{changes}\n\nFormat: type(scope): description\n\nTypes: feat, fix, docs, style, refactor, test, chore",
        parameters: [
          {
            name: "changes",
            description: "Description of changes made",
            type: "string",
            required: true,
          },
        ],
        category: "git",
        metadata: {
          version: "1.0",
        },
      },
    ],
    features: {
      streaming: true,
      fileOperations: true,
      notifications: true,
      batchOperations: false,
      maxRequestSize: 10485760, // 10MB
      contentTypes: [
        "application/json",
        "text/plain",
        "application/octet-stream",
      ],
    },
    protocolVersion: "1.0.0",
  };

  try {
    const data = await client.request(mutation, { input: capabilitiesInput });
    console.log("Registered capabilities:", JSON.stringify(data, null, 2));
    return data.registerMcpCapabilities;
  } catch (error) {
    console.error("Error registering capabilities:", error);
    throw error;
  }
}

/**
 * Test MCP server connection
 */
async function testMcpConnection(serverId) {
  const mutation = operationMap.TestMcpConnection;

  try {
    const data = await client.request(mutation, { serverId });
    console.log("Connection test:", JSON.stringify(data, null, 2));
    return data.testMcpConnection;
  } catch (error) {
    console.error("Error testing connection:", error);
    throw error;
  }
}

/**
 * Refresh session token
 */
async function refreshMcpSession(sessionId) {
  const mutation = operationMap.RefreshMcpSession;

  try {
    const data = await client.request(mutation, { sessionId });
    console.log("Session refreshed:", JSON.stringify(data, null, 2));
    return data.refreshMcpSession;
  } catch (error) {
    console.error("Error refreshing session:", error);
    throw error;
  }
}

/**
 * Revoke session
 */
async function revokeMcpSession(sessionId) {
  const mutation = operationMap.RevokeMcpSession;

  try {
    const data = await client.request(mutation, { sessionId });
    console.log("Session revoked:", JSON.stringify(data, null, 2));
    return data.revokeMcpSession;
  } catch (error) {
    console.error("Error revoking session:", error);
    throw error;
  }
}

/**
 * Toggle server enabled status
 */
async function toggleMcpServer(serverId, enabled) {
  const mutation = operationMap.ToggleMcpServer;

  try {
    const data = await client.request(mutation, { id: serverId, enabled });
    console.log("Server toggled:", JSON.stringify(data, null, 2));
    return data.toggleMcpServer;
  } catch (error) {
    console.error("Error toggling server:", error);
    throw error;
  }
}

/**
 * Delete MCP server
 */
async function deleteMcpServer(serverId) {
  const mutation = operationMap.DeleteMcpServer;

  try {
    const data = await client.request(mutation, { id: serverId });
    console.log("Server deleted:", JSON.stringify(data, null, 2));
    return data.deleteMcpServer;
  } catch (error) {
    console.error("Error deleting server:", error);
    throw error;
  }
}

// ============================================================================
// SUBSCRIPTION EXAMPLES
// ============================================================================

/**
 * Subscribe to server status updates using WebSocket
 * Note: This requires a WebSocket client like graphql-ws
 */
function subscribeToMcpServerStatus(serverId, callback) {
  const subscription = operationMap.McpServerStatusUpdates;

  console.log(`Subscription query for server ${serverId}:`, subscription);

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { serverId } }, callback);
}

/**
 * Subscribe to session events
 */
function subscribeToMcpSessionEvents(userId, serverId, callback) {
  const subscription = operationMap.McpSessionEvents;

  console.log(
    `Subscription query for user ${userId}, server ${serverId}:`,
    subscription,
  );

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { userId, serverId } }, callback);
}

/**
 * Subscribe to capability updates
 */
function subscribeToMcpCapabilityUpdates(serverId, callback) {
  const subscription = operationMap.McpCapabilityUpdates;

  console.log(
    `Subscription query for server capabilities ${serverId}:`,
    subscription,
  );

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { serverId } }, callback);
}

/**
 * Subscribe to authentication events
 */
function subscribeToMcpAuthEvents(tenantId, callback) {
  const subscription = operationMap.McpAuthEvents;

  console.log(
    `Subscription query for auth events, tenant ${tenantId}:`,
    subscription,
  );

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { tenantId } }, callback);
}

// ============================================================================
// COMPLETE MCP EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating MCP server lifecycle with OAuth
 */
async function completeMcpOAuthExample() {
  console.log("\n=== Complete MCP OAuth Example ===\n");

  const userId = "user-dev-123";
  let serverId, sessionId;

  try {
    // 1. Create GitLab MCP server with OAuth
    console.log("1. Creating GitLab MCP server with OAuth...");
    const server = await createGitLabMcpServer();
    serverId = server.id;

    // 2. Register server capabilities
    console.log("\n2. Registering server capabilities...");
    await registerMcpCapabilities(serverId);

    // 3. Test server connection
    console.log("\n3. Testing server connection...");
    await testMcpConnection(serverId);

    // 4. Check server health
    console.log("\n4. Checking server health...");
    await getMcpServerHealth(serverId);

    // 5. Get available OAuth providers
    console.log("\n5. Getting OAuth providers...");
    await getMcpOAuthProviders();

    // 6. Initiate OAuth flow
    console.log("\n6. Initiating OAuth flow...");
    const oauthInit = await initiateMcpOAuth(serverId, userId);
    console.log(`📱 Please visit: ${oauthInit.authorizationUrl}`);

    // 7. Simulate OAuth completion (normally done by redirect)
    console.log("\n7. Simulating OAuth completion...");
    const mockAuthCode = "mock_authorization_code_12345";
    const session = await completeMcpOAuth(
      serverId,
      userId,
      mockAuthCode,
      oauthInit.state,
    );
    sessionId = session.id;

    // 8. Get active sessions
    console.log("\n8. Getting active sessions...");
    await getMcpSessions(userId, serverId);

    // 9. Refresh session token
    console.log("\n9. Refreshing session token...");
    await refreshMcpSession(sessionId);

    // 10. Get server details with full configuration
    console.log("\n10. Getting server details...");
    await getMcpServer(serverId);

    console.log("\n✅ MCP OAuth example completed successfully!");
  } catch (error) {
    console.error("\n❌ MCP OAuth example failed:", error.message);
  } finally {
    // Cleanup: revoke session and disable server
    if (sessionId) {
      console.log("\n🧹 Cleaning up: revoking session...");
      await revokeMcpSession(sessionId);
    }
    if (serverId) {
      console.log("🧹 Cleaning up: disabling server...");
      await toggleMcpServer(serverId, false);
    }
  }
}

/**
 * Complete example demonstrating MCP server lifecycle with JWT
 */
async function completeMcpJwtExample() {
  console.log("\n=== Complete MCP JWT Example ===\n");

  // Display key information for debugging
  displayKeyInfo();

  const userId = "user-internal-456";
  let serverId, sessionId;

  try {
    // 1. Create JWT MCP server
    console.log("1. Creating JWT MCP server...");
    const server = await createJwtMcpServer();
    serverId = server.id;

    // 2. Configure JWT authentication
    console.log("\n2. Configuring JWT authentication...");
    await configureMcpJwt(serverId);

    // 3. Register server capabilities
    console.log("\n3. Registering server capabilities...");
    await registerMcpCapabilities(serverId);

    // 4. Test server connection
    console.log("\n4. Testing server connection...");
    await testMcpConnection(serverId);

    // 5. Generate and authenticate with JWT
    console.log("\n5. Authenticating with JWT...");
    const mockJwtToken =
      "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLWludGVybmFsLTQ1NiIsImlhdCI6MTcwNjcwMDAwMCwiZXhwIjoxNzA2NzAzNjAwfQ.mock_signature";
    const session = await authenticateMcpJwt(serverId, userId, mockJwtToken);
    sessionId = session.id;

    // 6. Get server capabilities
    console.log("\n6. Getting server capabilities...");
    await getMcpServerCapabilities(serverId);

    // 7. Get active sessions
    console.log("\n7. Getting active sessions...");
    await getMcpSessions(userId, serverId);

    // 8. Get server health
    console.log("\n8. Checking server health...");
    await getMcpServerHealth(serverId);

    console.log("\n✅ MCP JWT example completed successfully!");
  } catch (error) {
    console.error("\n❌ MCP JWT example failed:", error.message);
  } finally {
    // Cleanup: revoke session and disable server
    if (sessionId) {
      console.log("\n🧹 Cleaning up: revoking session...");
      await revokeMcpSession(sessionId);
    }
    if (serverId) {
      console.log("🧹 Cleaning up: disabling server...");
      await toggleMcpServer(serverId, false);
    }
  }
}

/**
 * Multi-tenant MCP management example
 */
async function multiTenantMcpExample() {
  console.log("\n=== Multi-Tenant MCP Example ===\n");

  const tenants = [
    "tenant-gitlab-001",
    "tenant-github-001",
    "tenant-internal-001",
  ];
  const serverIds = [];

  try {
    // 1. Create servers for different tenants
    console.log("1. Creating servers for multiple tenants...");

    const gitlabServer = await createGitLabMcpServer();
    serverIds.push(gitlabServer.id);

    const githubServer = await createGitHubMcpServer();
    serverIds.push(githubServer.id);

    const jwtServer = await createJwtMcpServer();
    serverIds.push(jwtServer.id);

    // 2. Get servers by tenant
    for (const tenantId of tenants) {
      console.log(`\n2. Getting servers for tenant: ${tenantId}...`);
      await getMcpServersByTenant(tenantId);
    }

    // 3. List all servers with filtering
    console.log("\n3. Listing all remote servers...");
    await getMcpServers("REMOTE", null);

    console.log("\n4. Listing all active servers...");
    await getMcpServers(null, "ONLINE");

    // 5. Test all server connections
    console.log("\n5. Testing all server connections...");
    for (const serverId of serverIds) {
      await testMcpConnection(serverId);
    }

    console.log("\n✅ Multi-tenant MCP example completed successfully!");
  } catch (error) {
    console.error("\n❌ Multi-tenant MCP example failed:", error.message);
  } finally {
    // Cleanup: disable all servers
    console.log("\n🧹 Cleaning up: disabling all servers...");
    for (const serverId of serverIds) {
      await toggleMcpServer(serverId, false);
    }
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to validate MCP server configuration
 */
function validateMcpServerConfig(config) {
  const required = ["name", "type", "tenantId", "config"];
  const missing = required.filter((field) => !(field in config));
  if (missing.length > 0) {
    throw new Error(
      `Missing required MCP server fields: ${missing.join(", ")}`,
    );
  }

  if (config.type === "REMOTE" && !config.config.endpoint) {
    throw new Error("Remote MCP servers must have an endpoint configured");
  }

  return true;
}

/**
 * Helper function to format server health status
 */
function formatServerHealth(health) {
  const statusIcon =
    health.status === "HEALTHY"
      ? "✅"
      : health.status === "UNHEALTHY"
        ? "❌"
        : "❓";

  return {
    status: `${statusIcon} ${health.status}`,
    lastCheck: new Date(health.lastCheckAt).toLocaleString(),
    responseTime: health.responseTimeMs ? `${health.responseTimeMs}ms` : "N/A",
    uptime: health.uptimeSeconds ? formatUptime(health.uptimeSeconds) : "N/A",
    error: health.error || "None",
    details: {
      connection: health.details.connection === "HEALTHY" ? "✅" : "❌",
      authentication: health.details.authentication === "HEALTHY" ? "✅" : "❌",
      capabilities: health.details.capabilities === "HEALTHY" ? "✅" : "❌",
      resources: health.details.resources === "HEALTHY" ? "✅" : "❌",
      tools: health.details.tools === "HEALTHY" ? "✅" : "❌",
    },
  };
}

/**
 * Helper function to format uptime
 */
function formatUptime(seconds) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  } else if (hours > 0) {
    return `${hours}h ${minutes}m ${secs}s`;
  } else if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  } else {
    return `${secs}s`;
  }
}

/**
 * Helper function to format session info
 */
function formatSessionInfo(session) {
  const statusIcon =
    session.status === "ACTIVE"
      ? "✅"
      : session.status === "EXPIRED"
        ? "⏰"
        : session.status === "REVOKED"
          ? "❌"
          : "❓";

  return {
    id: session.id,
    status: `${statusIcon} ${session.status}`,
    authMethod: session.authMethod,
    server: session.server.name,
    userId: session.userId,
    created: new Date(session.createdAt).toLocaleString(),
    lastActivity: new Date(session.lastActivityAt).toLocaleString(),
    expires: session.expiresAt
      ? new Date(session.expiresAt).toLocaleString()
      : "Never",
    requests: session.requestCount,
    tokenExpires: session.tokenExpiresAt
      ? new Date(session.tokenExpiresAt).toLocaleString()
      : "N/A",
  };
}

/**
 * Helper function to generate OAuth state parameter
 */
function generateOAuthState(userId, serverId, nonce = null) {
  const timestamp = Date.now();
  const randomNonce = nonce || Math.random().toString(36).substring(2, 15);
  return `${userId}_${serverId}_${timestamp}_${randomNonce}`;
}

/**
 * Helper function to validate OAuth state parameter
 */
function validateOAuthState(state, expectedUserId, expectedServerId) {
  const parts = state.split("_");
  if (parts.length !== 4) {
    return { valid: false, reason: "Invalid state format" };
  }

  const [userId, serverId, timestamp, nonce] = parts;

  if (userId !== expectedUserId) {
    return { valid: false, reason: "User ID mismatch" };
  }

  if (serverId !== expectedServerId) {
    return { valid: false, reason: "Server ID mismatch" };
  }

  const stateTimestamp = parseInt(timestamp);
  const currentTime = Date.now();
  const maxAge = 10 * 60 * 1000; // 10 minutes

  if (currentTime - stateTimestamp > maxAge) {
    return { valid: false, reason: "State expired" };
  }

  return { valid: true, userId, serverId, timestamp: stateTimestamp, nonce };
}

/**
 * Helper function to generate PKCE code verifier and challenge
 */
function generatePKCE() {
  const codeVerifier =
    Math.random().toString(36).substring(2, 15) +
    Math.random().toString(36).substring(2, 15) +
    Math.random().toString(36).substring(2, 15);

  // In a real implementation, you'd use crypto to generate SHA256 hash
  const codeChallenge = Buffer.from(codeVerifier)
    .toString("base64")
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=/g, "");

  return {
    codeVerifier,
    codeChallenge,
    codeChallengeMethod: "S256",
  };
}

/**
 * Helper function to format server capabilities summary
 */
function formatCapabilitiesSummary(capabilities) {
  return {
    protocolVersion: capabilities.protocolVersion,
    lastUpdated: new Date(capabilities.lastUpdated).toLocaleString(),
    counts: {
      tools: capabilities.tools.length,
      resources: capabilities.resources.length,
      prompts: capabilities.prompts.length,
    },
    features: {
      streaming: capabilities.features.streaming ? "✅" : "❌",
      fileOperations: capabilities.features.fileOperations ? "✅" : "❌",
      notifications: capabilities.features.notifications ? "✅" : "❌",
      batchOperations: capabilities.features.batchOperations ? "✅" : "❌",
    },
    maxRequestSize: capabilities.features.maxRequestSize
      ? `${(capabilities.features.maxRequestSize / 1024 / 1024).toFixed(1)}MB`
      : "N/A",
    contentTypes: capabilities.features.contentTypes.join(", "),
  };
}

/**
 * Helper function to format server metrics
 */
function formatServerMetrics(server) {
  return {
    id: server.id,
    name: server.name,
    type: server.type,
    status: server.status,
    health: server.health ? formatServerHealth(server.health) : "Not available",
    activeSessions: server.activeSessionsCount,
    enabled: server.enabled ? "✅ Enabled" : "❌ Disabled",
    config: {
      endpoint: server.config.endpoint || "Local",
      timeout: `${server.config.timeoutSeconds}s`,
      maxConnections: server.config.maxConnections,
      rateLimit: server.config.rateLimit
        ? `${server.config.rateLimit.requestsPerSecond}/s`
        : "None",
    },
    authType: server.auth
      ? server.auth.__typename === "McpOAuthConfig"
        ? "OAuth"
        : "JWT"
      : "None",
    lastUpdated: new Date(server.audit.updatedAt).toLocaleString(),
  };
}

/**
 * Helper function to display key information for debugging
 */
function displayKeyInfo() {
  const keyValidation = validateTestKeys();

  console.log("\n🔐 Test Key Information:");
  console.log("Keys directory:", testKeysDir);
  console.log("Private key file:", privateKeyPath);
  console.log("Public key file:", publicKeyPath);
  console.log(
    "Private key valid:",
    keyValidation.privateKeyValid ? "✅" : "❌",
  );
  console.log("Public key valid:", keyValidation.publicKeyValid ? "✅" : "❌");
  console.log("Using real keys:", keyValidation.usingRealKeys ? "✅" : "❌");

  if (keyValidation.usingRealKeys) {
    console.log(
      "Private key preview:",
      testPrivateKey.substring(0, 50) + "...",
    );
    console.log("Public key preview:", testPublicKey.substring(0, 50) + "...");
  } else {
    console.log("⚠️  Using placeholder keys - not suitable for real testing!");
  }

  return keyValidation;
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  getMcpServers,
  getMcpServer,
  getMcpServersByTenant,
  getMcpOAuthProviders,
  getMcpServerCapabilities,
  getMcpServerHealth,
  getMcpSessions,

  // Mutation functions
  createGitLabMcpServer,
  createGitHubMcpServer,
  createJwtMcpServer,
  updateMcpServer,
  configureMcpOAuth,
  configureMcpJwt,
  initiateMcpOAuth,
  completeMcpOAuth,
  authenticateMcpJwt,
  registerMcpCapabilities,
  testMcpConnection,
  refreshMcpSession,
  revokeMcpSession,
  toggleMcpServer,
  deleteMcpServer,

  // Subscription functions
  subscribeToMcpServerStatus,
  subscribeToMcpSessionEvents,
  subscribeToMcpCapabilityUpdates,
  subscribeToMcpAuthEvents,

  // Complete examples
  completeMcpOAuthExample,
  completeMcpJwtExample,
  multiTenantMcpExample,

  // Utilities
  validateMcpServerConfig,
  formatServerHealth,
  formatUptime,
  formatSessionInfo,
  generateOAuthState,
  validateOAuthState,
  generatePKCE,
  formatCapabilitiesSummary,
  formatServerMetrics,

  // Key utilities
  validateTestKeys,
  displayKeyInfo,

  // Test key access
  testPrivateKey,
  testPublicKey,

  // Schema reference
  mcpSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeMcpOAuthExample()
    .then(() => completeMcpJwtExample())
    .then(() => multiTenantMcpExample())
    .catch(console.error)
    .finally(() => process.exit(0));
}
