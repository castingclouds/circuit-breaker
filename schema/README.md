# Circuit Breaker GraphQL Schema Export

This directory contains the exported GraphQL schemas from the Circuit Breaker LLM Router service, organized for client SDK generation and API documentation.

## Overview

The Circuit Breaker GraphQL API provides comprehensive operations for:
- **Workflow Management**: Create and manage workflows, resources, and state transitions
- **Agent Management**: Configure and execute AI agents with LLM integrations
- **LLM Provider Management**: Manage multiple LLM providers and chat completions
- **Cost & Budget Analytics**: Track usage costs and manage budget limits
- **Rules Engine**: Create and evaluate conditional business rules
- **Real-time Subscriptions**: Stream updates for workflows, agents, and system events
- **NATS Integration**: Enhanced event streaming and distributed operations
- **MCP Server Management**: Multi-tenant Model Context Protocol servers with OAuth and JWT authentication

## Schema Files

### Core API Schemas

| File | Description | Operations |
|------|-------------|------------|
| [`workflow.graphql`](workflow.graphql) | Workflow management operations and types | 5 queries, 3 mutations, 2 subscriptions |
| [`agents.graphql`](agents.graphql) | Agent definitions, executions, and configurations | 5 queries, 3 mutations, 1 subscription |
| [`llm.graphql`](llm.graphql) | LLM provider management and chat completion operations | 2 queries, 2 mutations, 1 subscription |
| [`analytics.graphql`](analytics.graphql) | Cost tracking, budget management, and analytics | 2 queries, 1 mutation, 1 subscription |
| [`rules.graphql`](rules.graphql) | Rules engine operations and evaluation | 3 queries, 4 mutations |
| [`nats.graphql`](nats.graphql) | NATS-enhanced operations and event streaming | 3 queries, 2 mutations |
| [`mcp.graphql`](mcp.graphql) | Model Context Protocol server management with OAuth/JWT | 8 queries, 12 mutations, 4 subscriptions |
| [`subscriptions.graphql`](subscriptions.graphql) | Real-time subscription operations | 8 subscriptions |
| [`types.graphql`](types.graphql) | Shared types, scalars, and input objects | Common types, enums, interfaces |

### Supporting Files

| File | Description |
|------|-------------|
| [`introspection.json`](introspection.json) | Full GraphQL introspection data from server |
| [`schemas.md`](schemas.md) | Export progress tracking document |
| [`validate.sh`](validate.sh) | Schema validation script |
| [`README.md`](README.md) | This documentation file |

## Quick Start

### 1. Validate Schemas Against Running Server

```bash
# Ensure the Circuit Breaker server is running on localhost:4000
cd circuit-breaker/schema
./validate.sh
```

### 2. Generate Client SDK

```bash
# Example using GraphQL Code Generator
npm install -g @graphql-codegen/cli
graphql-codegen --config codegen.yml
```

### 3. Explore the API

```bash
# Access GraphQL Playground
open http://localhost:4000/graphql

# Or use curl for testing
curl -X POST -H "Content-Type: application/json" \
  -d '{"query":"{ workflows { id name states } }"}' \
  http://localhost:4000/graphql
```

## Schema Structure

### Root Operations

```graphql
type Query {
  # Workflow operations
  workflow(id: String!): WorkflowGQL
  workflows: [WorkflowGQL!]!
  
  # Agent operations
  agents: [AgentDefinitionGQL!]!
  agent(id: String!): AgentDefinitionGQL
  
  # LLM operations
  llmProviders: [LlmProviderGQL!]!
  
  # Analytics operations
  budgetStatus(userId: String, projectId: String): BudgetStatusGQL!
  
  # Rules operations
  rules(tags: [String!]): [RuleGQL!]!
  
  # NATS operations
  natsResource(id: String!): NatsResourceGQL
  
  # MCP operations
  mcpServers(type: McpServerType, status: McpServerStatus): McpServerConnection!
  mcpServer(id: ID!): McpServer
  mcpServersByTenant(tenantId: String!): McpServerConnection!
  mcpOAuthProviders: [McpOAuthProvider!]!
  mcpServerCapabilities(serverId: ID!): McpServerCapabilities
  mcpServerHealth(serverId: ID!): McpServerHealth!
  mcpSessions(userId: String, serverId: ID): McpSessionConnection!
}

type Mutation {
  # Workflow mutations
  createWorkflow(input: WorkflowDefinitionInput!): WorkflowGQL!
  createResource(input: ResourceCreateInput!): ResourceGQL!
  executeActivity(input: ActivityExecuteInput!): ResourceGQL!
  
  # Agent mutations
  createAgent(input: AgentDefinitionInput!): AgentDefinitionGQL!
  triggerStateAgents(input: TriggerStateAgentsInput!): [AgentExecutionGQL!]!
  
  # LLM mutations
  llmChatCompletion(input: LlmChatCompletionInput!): LlmResponseGQL!
  configureLlmProvider(input: LlmProviderConfigInput!): LlmProviderGQL!
  
  # Analytics mutations
  setBudget(input: BudgetInput!): BudgetStatusGQL!
  
  # Rules mutations
  createRule(input: RuleInput!): RuleGQL!
  evaluateRule(input: RuleEvaluationInput!): RuleEvaluationResultGQL!
  
  # MCP mutations
  createMcpServer(input: CreateMcpServerInput!): McpServer!
  updateMcpServer(id: ID!, input: UpdateMcpServerInput!): McpServer!
  deleteMcpServer(id: ID!): ApiResponse!
  configureMcpOAuth(input: ConfigureMcpOAuthInput!): McpOAuthConfig!
  configureMcpJwt(input: ConfigureMcpJwtInput!): McpJwtConfig!
  initiateMcpOAuth(input: InitiateMcpOAuthInput!): McpOAuthInitiation!
  completeMcpOAuth(input: CompleteMcpOAuthInput!): McpSession!
  authenticateMcpJwt(input: AuthenticateMcpJwtInput!): McpSession!
  refreshMcpSession(sessionId: ID!): McpSession!
  revokeMcpSession(sessionId: ID!): ApiResponse!
  registerMcpCapabilities(input: RegisterMcpCapabilitiesInput!): McpServerCapabilities!
  toggleMcpServer(id: ID!, enabled: Boolean!): McpServer!
  testMcpConnection(serverId: ID!): McpConnectionTest!
}

type Subscription {
  # Real-time updates
  resourceUpdates(resourceId: String!): ResourceGQL!
  workflowEvents(workflowId: String!): WorkflowEventGQL!
  agentExecutionStream(executionId: String!): AgentExecutionEventGQL!
  llmStream(requestId: String!): LlmStreamEventGQL!
  costUpdates(userId: String): CostUpdateEventGQL!
  mcpServerStatusUpdates(serverId: ID): McpServerStatusEvent!
  mcpSessionEvents(userId: String, serverId: ID): McpSessionEvent!
  mcpCapabilityUpdates(serverId: ID!): McpServerCapabilities!
  mcpAuthEvents(tenantId: String): McpAuthEvent!
}
```

### Key Types

#### Workflow Types
- `WorkflowGQL`: Complete workflow definition with states and activities
- `ResourceGQL`: Resource/token with current state and transition history
- `ActivityGQL`: Activity definition for state transitions

#### Agent Types
- `AgentDefinitionGQL`: AI agent configuration with LLM settings
- `AgentExecutionGQL`: Agent execution results and status
- `StateAgentConfigGQL`: State-specific agent configurations

#### LLM Types
- `LlmProviderGQL`: LLM provider configuration and health status
- `LlmResponseGQL`: Chat completion response with usage metrics
- `TokenUsageGQL`: Token consumption and cost information

#### Analytics Types
- `BudgetStatusGQL`: Budget limits and usage tracking
- `CostAnalyticsGQL`: Cost breakdown and analytics data

#### MCP Types
- `McpServer`: MCP server instance with configuration and health status
- `McpSession`: Active MCP sessions with authentication and token management
- `McpOAuthConfig`: OAuth configuration for GitLab, GitHub, and other providers
- `McpJwtConfig`: JWT authentication configuration and validation
- `McpServerCapabilities`: Available tools, resources, and prompts
- `McpServerHealth`: Health monitoring and connection status

## Example Operations

### Create a Workflow

```graphql
mutation CreateWorkflow {
  createWorkflow(input: {
    name: "Document Processing"
    states: ["submitted", "processing", "reviewed", "completed"]
    initialState: "submitted"
    activities: [
      {
        id: "start_processing"
        fromStates: ["submitted"]
        toState: "processing"
        conditions: []
      }
    ]
  }) {
    id
    name
    states
    activities {
      id
      fromStates
      toState
    }
  }
}
```

### Send LLM Chat Request

```graphql
mutation SendChatRequest {
  llmChatCompletion(input: {
    model: "gpt-4"
    messages: [
      {
        role: "user"
        content: "Explain quantum computing in simple terms"
      }
    ]
    temperature: 0.7
    maxTokens: 500
  }) {
    id
    choices {
      message {
        role
        content
      }
    }
    usage {
      totalTokens
      estimatedCost
    }
    routingInfo {
      selectedProvider
      latencyMs
    }
  }
}
```

### Subscribe to Resource Updates

```graphql
subscription ResourceUpdates {
  resourceUpdates(resourceId: "resource-123") {
    id
    state
    updatedAt
    history {
      timestamp
      fromState
      toState
      activity
    }
  }
}
```

### Create MCP Server with OAuth

```graphql
mutation CreateMcpServer {
  createMcpServer(input: {
    name: "GitLab MCP Server"
    description: "Multi-tenant MCP server with GitLab OAuth"
    type: REMOTE
    tenantId: "tenant-123"
    config: {
      endpoint: "https://mcp.example.com"
      timeoutSeconds: 30
      maxConnections: 100
    }
    auth: {
      oauth: {
        providerId: "gitlab"
        clientId: "7b0f347f26b4fe62313cd8a627e38193f2b209365ed3398d44fe02e69972a1eb"
        clientSecret: "gloas-c2004e0cc0a3f7465c569db45e23a24aca734ce2316af6f903060479857d1226"
        scopes: ["api"]
        redirectUri: "https://2bc3-76-182-171-196.ngrok-free.app/oauth/callback"
      }
    }
    tags: ["gitlab", "oauth", "remote"]
  }) {
    id
    name
    type
    status
    auth {
      ... on McpOAuthConfig {
        provider {
          name
          type
        }
        scopes
        redirectUri
      }
    }
    health {
      status
      responseTimeMs
    }
  }
}
```

### Initiate OAuth Flow

```graphql
mutation InitiateOAuthFlow {
  initiateMcpOAuth(input: {
    serverId: "mcp-server-123"
    userId: "user-456"
    redirectUri: "https://2bc3-76-182-171-196.ngrok-free.app/oauth/callback"
  }) {
    authorizationUrl
    state
    provider {
      name
      type
    }
    expiresAt
  }
}
```

### Complete OAuth and Create Session

```graphql
mutation CompleteOAuthFlow {
  completeMcpOAuth(input: {
    serverId: "mcp-server-123"
    userId: "user-456"
    code: "authorization-code-from-provider"
    state: "csrf-state-parameter"
  }) {
    id
    status
    authMethod
    tokenExpiresAt
    server {
      name
      type
    }
    createdAt
  }
}
```

### Subscribe to MCP Events

```graphql
subscription McpSessionEvents {
  mcpSessionEvents(userId: "user-456") {
    type
    session {
      id
      status
      server {
        name
        type
      }
    }
    timestamp
    success
  }
}
```

## Development Workflow

### 1. Schema Export Process

1. **Server Running**: Ensure Circuit Breaker server is running on localhost:4000
2. **Introspection**: Export full schema using introspection query
3. **Schema Files**: Organize schema into domain-specific files
4. **Validation**: Run validation script to verify all operations
5. **Documentation**: Update documentation and examples

### 2. Client SDK Generation

```bash
# Install dependencies
npm install @graphql-codegen/cli @graphql-codegen/typescript

# Create codegen configuration
cat > codegen.yml << EOF
overwrite: true
schema: "http://localhost:4000/graphql"
documents: "queries/**/*.graphql"
generates:
  src/generated/graphql.ts:
    plugins:
      - "typescript"
      - "typescript-operations"
      - "typescript-graphql-request"
EOF

# Generate TypeScript types
graphql-codegen
```

### 3. Testing Queries

```bash
# Test workflow operations
curl -X POST -H "Content-Type: application/json" \
  -d '{"query":"{ workflows { id name } }"}' \
  http://localhost:4000/graphql

# Test agent operations  
curl -X POST -H "Content-Type: application/json" \
  -d '{"query":"{ agents { id name capabilities } }"}' \
  http://localhost:4000/graphql
```

## Integration Examples

### JavaScript/TypeScript Client

```typescript
import { GraphQLClient } from 'graphql-request';

const client = new GraphQLClient('http://localhost:4000/graphql');

// Create workflow
const workflow = await client.request(`
  mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
    createWorkflow(input: $input) {
      id
      name
      states
    }
  }
`, {
  input: {
    name: "My Workflow",
    states: ["start", "end"],
    initialState: "start",
    activities: []
  }
});

// Subscribe to updates
const subscription = client.request(`
  subscription {
    resourceUpdates(resourceId: "123") {
      id
      state
    }
  }
`);
```

### Python Client

```python
from gql import gql, Client
from gql.transport.requests import RequestsHTTPTransport

transport = RequestsHTTPTransport(url="http://localhost:4000/graphql")
client = Client(transport=transport, fetch_schema_from_transport=True)

# Query workflows
query = gql("""
  query GetWorkflows {
    workflows {
      id
      name
      states
    }
  }
""")

result = client.execute(query)
print(result)
```

## Maintenance

### Updating Schemas

1. Make changes to the running GraphQL server
2. Re-export introspection data: `curl -X POST -H "Content-Type: application/json" -d '{"query":"..."}' http://localhost:4000/graphql > introspection.json`
3. Update individual schema files as needed
4. Run validation script: `./validate.sh`
5. Update documentation and examples

### Version Management

- Schema files are versioned with the main Circuit Breaker codebase
- Breaking changes should be documented in CHANGELOG.md
- Consider using schema versioning for major API changes

## Support

- **Documentation**: See individual schema files for detailed field documentation
- **Examples**: Check the `examples/` directory for complete operation examples
- **Issues**: Report schema-related issues in the main Circuit Breaker repository
- **API Playground**: Access interactive schema explorer at http://localhost:4000/graphql

## License

This schema export is part of the Circuit Breaker project and follows the same licensing terms.