# Circuit Breaker TypeScript SDK Examples

This directory contains comprehensive examples demonstrating the Circuit Breaker TypeScript SDK capabilities.

## Prerequisites

1. **Circuit Breaker Server**: Ensure the Circuit Breaker server is running
   ```bash
   cargo run --bin server
   ```

2. **Environment Variables** (optional):
   ```bash
   export CIRCUIT_BREAKER_URL="http://localhost:4000"
   export CIRCUIT_BREAKER_API_KEY="your-api-key"  # if authentication is enabled
   ```

3. **Dependencies**: Install the required packages
   ```bash
   npm install
   ```

## Examples Overview

### 1. Basic Usage (`basic-usage.ts`)

Demonstrates core SDK functionality including:

- ✅ **Client Creation**: Basic connection and authentication
- ✅ **Workflow Management**: Creating workflows with states and transitions
- ✅ **Resource Operations**: Creating and managing workflow resources
- ✅ **Rule Engine**: Server-side rule storage and evaluation with client-side fallback
- ✅ **Agent Creation**: AI agents with LLM integration
- ✅ **Multi-Provider LLM**: Testing OpenAI, Anthropic, and Google models
- ✅ **Smart Routing**: Virtual models and cost-optimized routing
- ✅ **Streaming**: Real-time response streaming
- ✅ **Workflow Execution**: Running workflow instances
- ✅ **Activity Management**: Executing activities and tracking history

**Run the example:**
```bash
npm run example:basic
# or directly:
npx tsx examples/basic-usage.ts
```

### 2. Multi-Provider LLM Demo (`multi-provider-demo.ts`)

Comprehensive demonstration of Circuit Breaker's multi-provider LLM capabilities:

- 🏢 **Provider Discovery**: Automatic detection of configured LLM providers
- 🧪 **Individual Testing**: Provider-specific model validation and testing
- 💰 **Cost Analysis**: Real-time cost comparison across providers
- 🌊 **Streaming Support**: Live streaming responses from multiple providers
- 🧠 **Smart Routing**: Virtual models and strategy-based provider selection
- 🎯 **Task-Specific Routing**: Automatic provider selection based on task type
- 🔧 **Advanced Features**: Function calling, temperature testing, and more
- 📊 **Performance Metrics**: Latency and throughput comparison

**Run the demo:**
```bash
npm run example:multi-provider
# or directly:
npx tsx examples/multi-provider-demo.ts
```

## Multi-Provider Configuration

The multi-provider demo showcases Circuit Breaker's ability to work with multiple LLM providers simultaneously:

### Supported Providers

| Provider | Models | Features |
|----------|--------|----------|
| **OpenAI** | GPT-4, GPT-4o-mini, GPT-3.5 | Function calling, streaming, embeddings |
| **Anthropic** | Claude 3 (Haiku, Sonnet, Opus) | Long context, safety, reasoning |
| **Google** | Gemini 1.5 (Flash, Pro) | Multimodal, fast inference, large context |

### Virtual Models

Circuit Breaker provides virtual models that automatically route to the best provider:

- `auto` - Automatic routing based on request characteristics
- `cb:cost-optimal` - Routes to the most cost-effective provider
- `cb:fastest` - Routes to the fastest responding provider
- `cb:coding` - Optimized for code generation tasks
- `cb:creative` - Optimized for creative writing tasks

### Smart Routing Strategies

```typescript
// Cost-optimized routing
{
  model: "auto",
  circuit_breaker: {
    routing_strategy: "cost_optimized",
    max_cost_per_1k_tokens: 0.001
  }
}

// Performance-first routing
{
  model: "auto",
  circuit_breaker: {
    routing_strategy: "performance_first"
  }
}

// Task-specific routing
{
  model: "auto",
  circuit_breaker: {
    routing_strategy: "task_specific",
    task_type: "coding" // or "creative", "analytical", etc.
  }
}
```

## Key Features Demonstrated

### 🔄 Workflow Engine
- State machine definition and execution
- Resource lifecycle management
- Activity-based transitions
- History tracking and auditing

### 🤖 AI Integration
- Multi-provider LLM support
- Agent creation and management
- Chat interfaces and streaming
- Cost optimization and routing

### 📋 Rule Engine
- Server-side rule storage
- Real-time rule evaluation
- Client-side fallback capabilities
- Dynamic rule management

### 🔍 Monitoring & Analytics
- Provider health monitoring
- Cost tracking and analysis
- Performance metrics
- Error handling and fallbacks

## Running Examples

### Option 1: NPM Scripts
```bash
# Run basic usage example
npm run example:basic

# Run multi-provider demo
npm run example:multi-provider

# Run all examples
npm run examples
```

### Option 2: Direct Execution
```bash
# Basic usage
npx tsx examples/basic-usage.ts

# Multi-provider demo
npx tsx examples/multi-provider-demo.ts
```

### Option 3: Node.js
```bash
# Compile TypeScript first
npm run build

# Run compiled examples
node dist/examples/basic-usage.js
node dist/examples/multi-provider-demo.js
```

## Troubleshooting

### Common Issues

1. **Server Not Running**
   ```
   ❌ Failed to connect: fetch failed
   ```
   **Solution**: Start the Circuit Breaker server:
   ```bash
   cargo run --bin server
   ```

2. **API Key Required**
   ```
   ❌ Authentication failed
   ```
   **Solution**: Set your API key:
   ```bash
   export CIRCUIT_BREAKER_API_KEY="your-api-key"
   ```

3. **LLM Provider Unavailable**
   ```
   ⚠️ Agent creation skipped (LLM not available)
   ```
   **Solution**: Configure LLM provider API keys in the server's `.env` file

4. **Port Conflicts**
   ```
   ❌ Server not responding
   ```
   **Solution**: Check if ports 3000 (OpenAI API) and 4000 (GraphQL) are available

### Debug Mode

Enable verbose logging by setting the debug environment variable:

```bash
export DEBUG=circuit-breaker:*
npm run example:basic
```

### Debugging Smart Routing

If you notice that smart routing consistently selects one provider (e.g., always Anthropic):

1. **Check Provider Health**:
   ```bash
   curl http://localhost:4000/graphql \
     -H "Content-Type: application/json" \
     -d '{"query": "{ llmProviders { name healthStatus { isHealthy errorRate averageLatencyMs } } }"}'
   ```

2. **Test Explicit Model Selection**:
   ```bash
   # Test OpenAI directly
   curl http://localhost:3000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"model": "o4-mini-2025-04-16", "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 10}'
   
   # Test Anthropic directly
   curl http://localhost:3000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"model": "claude-3-haiku-20240307", "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 10}'
   ```

3. **Force Different Routing Strategies**:
   ```bash
   # Force performance-first routing
   curl http://localhost:3000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"model": "auto", "messages": [{"role": "user", "content": "Hello"}], "circuit_breaker": {"routing_strategy": "performance_first"}}'
   ```

4. **Common Routing Issues**:
   - **Cost Optimization**: Claude Haiku (~$0.00025/1K) vs GPT-4o-mini (~$0.003/1K) - 12x cheaper
   - **Health Status**: Check if OpenAI provider shows as unhealthy
   - **API Keys**: Verify all provider API keys are configured in server `.env`
   - **Rate Limits**: One provider may be hitting rate limits

## Integration Guide

To integrate these examples into your own application:

1. **Install the SDK**:
   ```bash
   npm install circuit-breaker-sdk
   ```

2. **Import the client**:
   ```typescript
   import { Client, createWorkflow, createAgent } from 'circuit-breaker-sdk';
   ```

3. **Initialize the client**:
   ```typescript
   const client = Client.builder()
     .baseUrl('http://localhost:4000')
     .apiKey(process.env.CIRCUIT_BREAKER_API_KEY)
     .build();
   ```

4. **Use the patterns from the examples** to build your workflow automation and AI integration.

## Additional Resources

- 📖 [SDK Documentation](../README.md)
- 🌐 [GraphiQL Interface](http://localhost:4000) (when server is running)
- 🔗 [OpenAI API Endpoint](http://localhost:3000) (when server is running)
- 🏗️ [Circuit Breaker GitHub Repository](https://github.com/circuit-breaker/circuit-breaker)

## Contributing

Found an issue or want to add more examples? Please:

1. Fork the repository
2. Create a feature branch
3. Add your example with proper documentation
4. Submit a pull request

---

**Happy automating with Circuit Breaker! 🚀**