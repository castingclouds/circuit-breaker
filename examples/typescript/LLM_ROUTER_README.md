# TypeScript LLM Router Demo

This TypeScript implementation demonstrates the same functionality as the Rust `llm_router_demo.rs`, showcasing real Anthropic API integration with SSE streaming and WebSocket GraphQL subscriptions.

## Overview

The TypeScript demo provides a complete example of:
- **SSE (Server-Sent Events)** streaming with Anthropic Claude API
- **WebSocket** subscriptions for GraphQL real-time updates
- Cost tracking and budget management
- Provider health monitoring
- Type-safe GraphQL operations

## Architecture

```
┌─────────────────┐    SSE     ┌──────────────────┐
│   TypeScript    │◄─────────►│  Anthropic API   │
│   Demo Client   │            │  (Claude 3/4)    │
└─────────────────┘            └──────────────────┘
         │
         │ WebSocket (GraphQL)
         ▼
┌─────────────────┐            ┌──────────────────┐
│ Circuit Breaker │◄─────────►│   Other LLM      │
│     Server      │    HTTP    │   Providers      │
│   (Port 4000)   │            │                  │
└─────────────────┘            └──────────────────┘
```

## Key Features

### 1. **SSE Streaming with Anthropic**
- Direct integration with Anthropic's streaming API
- Real-time token-by-token response streaming
- Proper SSE event parsing (`data: ` prefixed lines)
- Handles Anthropic-specific events: `content_block_delta`, `message_stop`

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
2. **Circuit Breaker Server** - Must be running on port 4000
3. **Anthropic API Key** - Set as environment variable

### Quick Setup

```bash
# 1. Navigate to TypeScript examples
cd circuit-breaker/examples/typescript

# 2. Run setup script (installs dependencies)
./setup.sh

# 3. Set your Anthropic API key
export ANTHROPIC_API_KEY=your_key_here

# 4. Start the Circuit Breaker server (in another terminal)
cd ../..
cargo run --bin server

# 5. Run the LLM router demo
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

```bash
# Required
export ANTHROPIC_API_KEY=your_anthropic_api_key_here

# Optional (defaults shown)
export CIRCUIT_BREAKER_URL=http://localhost:4000
export ANTHROPIC_API_URL=https://api.anthropic.com/v1/messages
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

### 3. **Direct Anthropic SSE Streaming**
```typescript
// Real-time streaming with SSE
const response = await fetch('https://api.anthropic.com/v1/messages', {
  method: 'POST',
  headers: {
    'x-api-key': apiKey,
    'anthropic-version': '2023-06-01'
  },
  body: JSON.stringify({
    model: 'claude-3-sonnet-20240229',
    stream: true,  // Enable SSE streaming
    messages: [...]
  })
});

// Process SSE stream
const reader = response.body?.getReader();
while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  
  // Parse SSE events
  const chunk = decoder.decode(value);
  for (const line of chunk.split('\n')) {
    if (line.startsWith('data: ')) {
      const event = JSON.parse(line.slice(6));
      if (event.type === 'content_block_delta') {
        process.stdout.write(event.delta.text);
      }
    }
  }
}
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

### Anthropic SSE Events
```typescript
interface AnthropicStreamEvent {
  type: 'content_block_delta' | 'message_stop' | 'message_start';
  index?: number;
  delta?: {
    type: string;
    text?: string;
  };
  message?: {
    id: string;
    usage?: {
      input_tokens: number;
      output_tokens: number;
    };
  };
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

✅ ANTHROPIC_API_KEY found
✅ Server is running and accessible

📊 1. Checking LLM Providers
----------------------------
✅ Available Providers: {...}

💬 2. Direct Anthropic SSE Streaming
-----------------------------------
🔄 Testing real-time SSE streaming...
📡 Using direct Anthropic streaming API integration
🔄 Real-time SSE streaming response:
   Claude 3: A woodchuck would chuck approximately 700 pounds of wood per day if a woodchuck could chuck wood, according to wildlife biologist Richard Thomas's 1988 calculation.
✅ SSE streaming completed successfully!
   Chunks received: 23
   🎯 This demonstrates working SSE streaming infrastructure

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
   • Real Anthropic Claude API integration with SSE streaming
   • TypeScript implementation matching Rust functionality
   • Actual token counting and cost calculation
   • GraphQL queries and mutations
   • WebSocket streaming infrastructure validation
   • Real-time subscription capabilities

🎉 Circuit Breaker: TypeScript + SSE + WebSocket ready!
📡 Test real-time streaming now: http://localhost:4000
```

## Testing WebSocket Subscriptions

Once the demo completes, you can test live WebSocket subscriptions using GraphiQL:

1. Open http://localhost:4000 in your browser
2. Try these subscription examples:

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

1. **"ANTHROPIC_API_KEY not set"**
   ```bash
   export ANTHROPIC_API_KEY=your_key_here
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

## Performance Characteristics

- **SSE Latency**: ~50-200ms per token (depends on model)
- **WebSocket Latency**: ~1-5ms for local subscriptions
- **Memory Usage**: ~50MB for demo client
- **Concurrent Streams**: Supports multiple simultaneous SSE streams

## Comparison: TypeScript vs Rust

| Feature | TypeScript | Rust |
|---------|------------|------|
| SSE Streaming | ✅ `fetch` + `ReadableStream` | ✅ `reqwest` + `bytes_stream()` |
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