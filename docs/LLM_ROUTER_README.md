# TypeScript LLM Router Demo

This TypeScript implementation demonstrates the same functionality as the Rust version, showcasing Circuit Breaker's OpenAI-compatible API routing to Anthropic with SSE streaming and WebSocket GraphQL subscriptions.

## Overview

The TypeScript demo provides a complete example of:
- **Circuit Breaker OpenAI-compatible API** routing to Anthropic providers
- **SSE (Server-Sent Events)** streaming through the router
- **WebSocket** subscriptions for GraphQL real-time updates
- Cost tracking and budget management
- Provider health monitoring
- Type-safe GraphQL operations

## Architecture

```
┌─────────────────┐    HTTP     ┌─────────────────┐    SSE      ┌──────────────────┐
│   TypeScript               │───────►│ Circuit Breaker             │ ──────►│  Anthropic API               │
│   Demo Client              │             │ OpenAI API                  │             │  (Claude 3/4)                │
│   (No API Key)             │             │ (Has all keys)              │             │  (Server has key)            │
└─────────────────┘             └─────────────────┘             └──────────────────┘
         │                                           │
         │ WebSocket (GraphQL)                       │ HTTP
         ▼                                           ▼
┌─────────────────┐            ┌──────────────────┐
│ Circuit Breaker            │            │   Other LLM                   │
│ GraphQL Server             │            │   Providers                   │
│   (Port 4000)              │            │   (OpenAI,vLLM,Ollama etc.)   │
└─────────────────┘            └──────────────────┘
```

## Key Features

### 1. **Streaming via Circuit Breaker Router**
- OpenAI-compatible API on port 3000
- Server-side API key management (clients don't need provider keys)
- Routes to Anthropic (and other providers) based on model selection
- Real-time token-by-token response streaming
- Unified interface for all LLM providers

### 2. **WebSocket GraphQL Subscriptions**
- Real-time updates for multiple subscribers
- GraphQL subscription validation
- Support for multiple streaming channels:
  - `llmStream` - LLM responses
  - `tokenUpdates` - Workflow tokens
  - `costUpdates` - Cost monitoring
  - `agentExecutionStream` - AI agent execution
  - `workflowEvents` - Workflow state changes

### 3. **Type Safety**
- Full TypeScript interfaces for all API responses
- GraphQL response typing
- Anthropic API event type definitions
- Compile-time error checking

## Setup Instructions

### Prerequisites

1. **Node.js 18+** - Download from [nodejs.org](https://nodejs.org/)
2. **Circuit Breaker Server** - Must be running on ports 3000 (OpenAI API) and 4000 (GraphQL)
3. **Server Configuration** - Anthropic API key must be configured server-side

### Quick Setup

```bash
# 1. Navigate to TypeScript examples
cd circuit-breaker/examples/typescript

# 2. Run setup script (installs dependencies)
./setup.sh

# 3. Configure server with API key (in another terminal)
cd ../..
export ANTHROPIC_API_KEY=your_key_here
cargo run --bin server

# 4. Run the LLM router demo (no API key needed)
npm run demo:llm
```

### Manual Setup

```bash
# Install dependencies
npm install

# Verify TypeScript installation
npx tsc --version

# Run the demo
npx tsx llm_router_demo.ts
```

## Environment Variables

### Server-Side (Required)
```bash
# API keys are managed server-side
export ANTHROPIC_API_KEY=your_anthropic_api_key_here
```

### Client-Side (Optional)
```bash
# Optional client configuration (defaults shown)
export CIRCUIT_BREAKER_GRAPHQL_URL=http://localhost:4000
export CIRCUIT_BREAKER_OPENAI_URL=http://localhost:3000
```

## Demo Walkthrough

The demo performs the following operations:

### 1. **Server Connectivity Check**
```typescript
// Tests if Circuit Breaker server is running
const healthResponse = await fetch('http://localhost:4000/health');
```

### 2. **LLM Provider Query**
```typescript
// GraphQL query to get available providers
const query = `
  query {
    llmProviders {
      id
      providerType
      name
      models {
        supportsStreaming
        costPerInputToken
        costPerOutputToken
      }
    }
  }
`;
```

### 2. **Streaming via Circuit Breaker OpenAI API**
```typescript
// Real-time streaming through Circuit Breaker router
const response = await fetch('http://localhost:3000/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json'
    // No Authorization header - server manages API keys
  },
  body: JSON.stringify({
    model: 'claude-3-sonnet-20240229',  // Router selects Anthropic
    stream: true,  // Enable SSE streaming
    messages: [...]
  })
});

// Process OpenAI-compatible SSE stream
const reader = response.body?.getReader();
while (true) {
  const { done, value } = await reader.read();
  if (done) break;

  // Parse OpenAI-compatible SSE events
  const chunk = decoder.decode(value);
  for (const line of chunk.split('\n')) {
    if (line.startsWith('data: ')) {
      const event = JSON.parse(line.slice(6));
      if (event.choices?.[0]?.delta?.content) {
        process.stdout.write(event.choices[0].delta.content);
      }
    }
  }
}
```

### 3. **OpenAI-Compatible API Usage**
```typescript
// Use Circuit Breaker's OpenAI-compatible endpoint
const response = await fetch('http://localhost:3000/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json'
    // No API key needed - server handles authentication
  },
  body: JSON.stringify({
    model: 'claude-3-sonnet-20240229',  // Router selects Anthropic provider
    messages: [{ role: 'user', content: 'Hello!' }],
    stream: false
  })
});
```

### 4. **Budget and Cost Management**
```typescript
// Check budget status
const budgetQuery = `
  query {
    budgetStatus(userId: "demo-user", projectId: "demo-project") {
      limit
      used
      remaining
      isExhausted
    }
  }
`;

// Set budget limits
const setBudgetMutation = `
  mutation($input: BudgetInput!) {
    setBudget(input: $input) {
      budgetId
      limit
      message
    }
  }
`;
```

### 5. **WebSocket Subscription Validation**
```typescript
// Test WebSocket GraphQL subscriptions
const client = createClient({
  url: 'ws://localhost:4000/ws',
  webSocketImpl: WebSocket,
  connectionParams: {
    'Sec-WebSocket-Protocol': 'graphql-ws'
  }
});

// Example subscription
const subscription = `
  subscription {
    llmStream(requestId: "live-demo") {
      id
      content
      tokens
      cost
      timestamp
    }
  }
`;
```

## API Response Types

### OpenAI-Compatible Stream Events
```typescript
interface OpenAIStreamEvent {
  id: string;
  object: 'chat.completion.chunk';
  created: number;
  model: string;
  choices: Array<{
    index: number;
    delta: {
      content?: string;
      role?: string;
    };
    finish_reason?: string | null;
  }>;
}
```

### GraphQL Responses
```typescript
interface LLMProvider {
  id: string;
  providerType: string;
  name: string;
  baseUrl: string;
  healthStatus: {
    isHealthy: boolean;
    errorRate: number;
    averageLatencyMs: number;
  };
  models: Array<{
    id: string;
    name: string;
    costPerInputToken: number;
    costPerOutputToken: number;
    supportsStreaming: boolean;
  }>;
}

interface BudgetStatus {
  budgetId: string;
  limit: number;
  used: number;
  remaining: number;
  isExhausted: boolean;
  message: string;
}
```

## Expected Output

```
🤖 Circuit Breaker LLM Router Demo - TypeScript Integration
===========================================================

ℹ️  API keys are managed server-side by Circuit Breaker
💡 Client does not need to provide API keys - router handles authentication
✅ Server is running and accessible

📊 1. Checking LLM Providers
----------------------------
✅ Available Providers: {...}

2. Circuit Breaker OpenAI API Streaming
---------------------------------------
🔄 Testing real-time SSE streaming...
📡 Using Circuit Breaker OpenAI-compatible API with Anthropic routing
🔄 Real-time SSE streaming response:
   Claude 3: A woodchuck would chuck approximately 700 pounds of wood per day if a woodchuck could chuck wood, according to wildlife biologist Richard Thomas's 1988 calculation.
✅ SSE streaming completed successfully!
   Chunks received: 23
   🎯 This demonstrates Circuit Breaker router with OpenAI-compatible streaming

💰 3. Checking Budget Status
---------------------------
✅ Budget Status:
   Limit: $50
   Used: $0.42
   Remaining: $49.58
   Status: Budget is healthy

📈 4. Getting Cost Analytics
---------------------------
✅ Cost Analytics:
   Total Cost: $0.42
   Total Tokens: 1247
   Avg Cost/Token: $0.000000337

⚙️  5. Configuring New Provider
------------------------------
✅ Provider Configured:
   Provider: Anthropic Claude
   Type: anthropic
   Base URL: https://api.anthropic.com

💵 6. Setting Budget Limits
--------------------------
✅ Budget Set:
   Budget ID: budget-123
   Daily Limit: $50
   Status: Budget successfully configured

🔄 7. WebSocket Streaming Implementation Validation
--------------------------------------------------
✅ GraphQL Subscription type found: Subscription
📋 Available WebSocket subscription fields:
   ✅ llmStream - Real-time LLM response streaming
   ✅ tokenUpdates - Workflow token state streaming
   ✅ costUpdates - Real-time cost monitoring
   ✅ agentExecutionStream - AI agent execution streaming
   ✅ workflowEvents - Workflow state change streaming

📡 8. Testing WebSocket GraphQL Subscriptions
--------------------------------------------
✅ WebSocket GraphQL subscriptions infrastructure ready
💡 Test live subscriptions at: http://localhost:4000 (GraphiQL)

🎯 9. Integration Analysis
-------------------------
✅ What We Just Demonstrated:
   • Circuit Breaker OpenAI-compatible API routing to Anthropic
   • TypeScript implementation matching Rust functionality
   • Unified interface for multiple LLM providers through port 3000
   • Actual token counting and cost calculation
   • Claude 3: ~$0.000003/input token, ~$0.000015/output token
   • GraphQL queries and mutations on port 4000
   • WebSocket streaming infrastructure validation
   • Real-time subscription capabilities

🎉 Circuit Breaker: TypeScript + SSE + WebSocket ready!
📡 Test real-time streaming now: http://localhost:4000
```

## Testing the Complete System

Once the demo completes, you can test both the OpenAI-compatible API and WebSocket subscriptions:

### OpenAI-Compatible API (Port 3000)
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-sonnet-20240229",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### WebSocket Subscriptions (Port 4000)
Open http://localhost:4000 in your browser and try these subscription examples:

```graphql
# LLM Streaming
subscription {
  llmStream(requestId: "test-123") {
    id
    content
    tokens
    cost
    timestamp
  }
}

# Cost Monitoring
subscription {
  costUpdates(userId: "demo-user") {
    totalCost
    dailySpend
    timestamp
  }
}

# Token Updates
subscription {
  tokenUpdates(tokenId: "token-456") {
    id
    place
    data
    timestamp
  }
}
```

## Troubleshooting

### Common Issues

1. **"API authentication failed"**
   ```bash
   # Set API key server-side before starting server
   export ANTHROPIC_API_KEY=your_key_here
   cargo run --bin server
   ```

2. **"Cannot connect to server"**
   ```bash
   # Start the Circuit Breaker server
   cargo run --bin server
   ```

3. **WebSocket connection fails**
   - Ensure server is running with WebSocket support
   - Check firewall/proxy settings

4. **TypeScript compilation errors**
   ```bash
   npm run type-check
   ```

### Debug Mode

Enable debug logging:
```bash
DEBUG=* npm run demo:llm
```

### Testing Both Endpoints

Test the complete system:
```bash
# Test OpenAI-compatible API
curl http://localhost:3000/v1/models

# Test GraphQL API
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ __typename }"}'
```

## Performance Characteristics

- **SSE Latency**: ~50-200ms per token (depends on model)
- **WebSocket Latency**: ~1-5ms for local subscriptions
- **Memory Usage**: ~50MB for demo client
- **Concurrent Streams**: Supports multiple simultaneous SSE streams

## Comparison: TypeScript vs Rust

| Feature | TypeScript | Rust |
|---------|------------|------|
| API Integration | ✅ Circuit Breaker OpenAI API | ✅ Circuit Breaker LLM Router |
| SSE Streaming | ✅ `fetch` + OpenAI format | ✅ `reqwest` + Anthropic format |
| WebSocket | ✅ `ws` + `graphql-ws` | ✅ `tokio-tungstenite` + `async-graphql` |
| Type Safety | ✅ TypeScript interfaces | ✅ Rust structs + `serde` |
| Performance | Good (V8 JIT) | Excellent (native) |
| Memory Usage | ~50MB | ~5MB |
| Startup Time | ~200ms | ~50ms |
| Error Handling | Promises/async-await | `Result<T, E>` |

## Next Steps

1. **Add More Providers**: Extend to OpenAI, Google, Cohere
2. **Error Recovery**: Implement retry logic and fallbacks
3. **Caching**: Add response caching for cost optimization
4. **Monitoring**: Add metrics and observability
5. **UI Integration**: Build React/Vue components using this client

## Related Examples

- `graphql_client.ts` - Basic GraphQL operations
- `places_ai_agent_demo.ts` - AI agent workflows
- `basic_workflow.ts` - Workflow execution patterns

## License

This TypeScript demo is part of the Circuit Breaker project and follows the same license terms.
