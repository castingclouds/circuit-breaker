# LLM Router Documentation

The Circuit Breaker LLM Router provides intelligent routing across multiple LLM providers with advanced features like cost optimization, health monitoring, and automatic failover.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Configuration](#configuration)
- [Routing Strategies](#routing-strategies)
- [Provider Support](#provider-support)
- [Streaming](#streaming)
- [Health Monitoring](#health-monitoring)
- [Cost Management](#cost-management)
- [Error Handling](#error-handling)
- [Examples](#examples)
- [API Reference](#api-reference)

## Quick Start

### Basic Setup

```typescript
import { createLLMBuilder } from 'circuit-breaker-sdk';

// Simple single provider setup
const router = await createLLMBuilder()
  .addOpenAI({
    apiKey: process.env.OPENAI_API_KEY,
    models: ['gpt-4', 'gpt-3.5-turbo']
  })
  .build();

// Send a chat completion request
const response = await router.router.chatCompletion({
  model: 'gpt-3.5-turbo',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'Hello!' }
  ]
});

console.log(response.choices[0].message.content);
```

### Multi-Provider Setup

```typescript
import { createMultiProviderBuilder } from 'circuit-breaker-sdk';

const router = await createMultiProviderBuilder({
  openai: process.env.OPENAI_API_KEY,
  anthropic: process.env.ANTHROPIC_API_KEY,
  ollama: 'http://localhost:11434'
}).build();

// The router will automatically select the best provider
const response = await router.router.chatCompletion({
  model: 'gpt-4',
  messages: [{ role: 'user', content: 'Explain quantum computing.' }]
});
```

## Core Concepts

### Router Architecture

The LLM Router consists of several key components:

1. **Router Core**: Manages provider selection and request routing
2. **Providers**: Individual LLM provider implementations
3. **Health Monitor**: Tracks provider availability and performance
4. **Cost Tracker**: Monitors usage and costs
5. **Streaming Handler**: Manages real-time response streaming

### Provider Abstraction

All providers implement a unified interface, allowing seamless switching between:

- **OpenAI** (GPT-3.5, GPT-4)
- **Anthropic** (Claude-3 family)
- **Ollama** (Local models)
- **Custom** providers

### Request Flow

1. Request received by router
2. Provider selection based on routing strategy
3. Health check validation
4. Request execution with retry logic
5. Response processing and metrics collection

## Configuration

### Basic Configuration

```typescript
import { LLMBuilder } from 'circuit-breaker-sdk';

const router = await new LLMBuilder({
  timeout: 30000,        // Global timeout
  maxRetries: 3,         // Max retry attempts
  debug: true           // Enable debug logging
})
  .addOpenAI({
    name: 'openai-primary',
    apiKey: process.env.OPENAI_API_KEY,
    models: ['gpt-4', 'gpt-3.5-turbo'],
    priority: 1
  })
  .setRoutingStrategy('cost-optimized')
  .enableHealthChecks()
  .build();
```

### Provider Configuration

```typescript
.addProvider({
  name: 'custom-provider',
  type: 'openai',
  endpoint: 'https://api.openai.com/v1',
  apiKey: 'your-api-key',
  models: ['gpt-4'],
  priority: 1,
  timeout: 20000,
  maxRetries: 2,
  rateLimit: {
    requestsPerMinute: 100,
    tokensPerMinute: 50000,
    concurrent: 10
  }
})
```

### Advanced Configuration

```typescript
const router = await createLLMBuilder()
  .addOpenAI({ /* config */ })
  .addAnthropic({ /* config */ })
  .setRoutingStrategy('performance-first')
  .enableHealthChecks({
    interval: 30,          // Check every 30 seconds
    timeout: 5000,         // 5 second timeout
    retries: 2             // 2 retry attempts
  })
  .enableCostTracking({
    budgetLimit: 100,      // $100 budget limit
    alertThreshold: 80,    // Alert at 80%
    trackPerUser: true     // Per-user tracking
  })
  .setFailover({
    enabled: true,
    maxRetries: 3,
    backoffStrategy: 'exponential'
  })
  .build();
```

## Routing Strategies

### Cost-Optimized

Routes requests to the cheapest available provider for the requested model.

```typescript
.setRoutingStrategy('cost-optimized')
```

**Use cases:**
- High-volume applications
- Budget-conscious deployments
- Development environments

### Performance-First

Routes requests to the provider with the lowest latency.

```typescript
.setRoutingStrategy('performance-first')
```

**Use cases:**
- Real-time applications
- Interactive chat interfaces
- Low-latency requirements

### Load-Balanced

Distributes requests evenly across available providers.

```typescript
.setRoutingStrategy('load-balanced')
.setLoadBalancing({
  strategy: 'round-robin',
  weights: { 'provider-1': 70, 'provider-2': 30 }
})
```

**Use cases:**
- High-availability systems
- Rate limit distribution
- Load distribution

### Failover-Chain

Uses providers in priority order, falling back on failure.

```typescript
.setRoutingStrategy('failover-chain')
```

**Use cases:**
- Primary/backup configurations
- Reliability-critical applications
- Simple fallback scenarios

### Model-Specific

Routes based on model availability and provider capabilities.

```typescript
.setRoutingStrategy('model-specific')
```

**Use cases:**
- Model-specific optimizations
- Provider-specific features
- Custom routing logic

## Provider Support

### OpenAI

```typescript
.addOpenAI({
  apiKey: process.env.OPENAI_API_KEY,
  models: ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo'],
  endpoint: 'https://api.openai.com/v1'  // Optional
})
```

**Supported Models:**
- `gpt-4`: Advanced reasoning, 8K context
- `gpt-4-turbo`: Latest GPT-4, 128K context
- `gpt-3.5-turbo`: Fast and efficient, 4K context

**Features:**
- Function calling
- Streaming responses
- Vision capabilities (GPT-4V)

### Anthropic

```typescript
.addAnthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
  models: ['claude-3-opus', 'claude-3-sonnet', 'claude-3-haiku'],
  endpoint: 'https://api.anthropic.com/v1'  // Optional
})
```

**Supported Models:**
- `claude-3-opus`: Highest capability, best for complex tasks
- `claude-3-sonnet`: Balanced performance and cost
- `claude-3-haiku`: Fastest and most cost-effective

**Features:**
- 200K context window
- Streaming responses
- Vision capabilities

### Ollama (Local)

```typescript
.addOllama({
  endpoint: 'http://localhost:11434',
  models: ['llama2', 'mistral', 'codellama']
})
```

**Supported Models:**
- `llama2`: Meta's open-source model
- `mistral`: Efficient 7B parameter model
- `codellama`: Code-specialized model
- Custom models available through Ollama

**Features:**
- Local inference (no API costs)
- Privacy-preserving
- Custom model support

## Streaming

### Basic Streaming

```typescript
const stream = router.chatCompletionStream({
  model: 'gpt-3.5-turbo',
  messages: [{ role: 'user', content: 'Write a story...' }],
  stream: true
});

for await (const chunk of stream) {
  const content = chunk.choices[0]?.delta?.content || '';
  process.stdout.write(content);
}
```

### Advanced Streaming with Handler

```typescript
import { StreamingHandler } from 'circuit-breaker-sdk';

const handler = new StreamingHandler();
const session = handler.createStream({
  bufferSize: 10,
  chunkTimeout: 5000,
  autoReconnect: true
});

session.on('chunk', (chunk) => {
  console.log('Received:', chunk.choices[0]?.delta?.content);
});

session.on('complete', (response) => {
  console.log('Stream completed:', response);
});

session.on('error', (error) => {
  console.error('Stream error:', error);
});

// Process server-sent events
await handler.processSSEStream(response, session);
```

### Stream Configuration

```typescript
.setStreaming({
  bufferSize: 10,           // Chunk buffer size
  chunkTimeout: 5000,       // Individual chunk timeout
  streamTimeout: 30000,     // Overall stream timeout
  autoReconnect: true,      // Auto-reconnect on failure
  maxReconnectAttempts: 3,  // Max reconnection attempts
  validateChunks: true      // Enable chunk validation
})
```

## Health Monitoring

### Health Check Configuration

```typescript
.enableHealthChecks({
  enabled: true,
  interval: 60,        // Check every 60 seconds
  timeout: 10000,      // 10 second timeout
  retries: 3           // 3 retry attempts
})
```

### Monitoring Events

```typescript
router.on('healthCheck', (data) => {
  console.log(`Health check: ${data.provider} - ${data.isHealthy}`);
});

router.on('providerError', (data) => {
  console.log(`Provider error: ${data.provider} - ${data.error}`);
});
```

### Health Status API

```typescript
// Get all provider health
const allHealth = router.getProviderHealth();

// Get specific provider health
const openaiHealth = router.getProviderHealth('openai-provider');

console.log({
  provider: openaiHealth.provider,
  isHealthy: openaiHealth.isHealthy,
  lastCheck: openaiHealth.lastCheck,
  averageLatency: openaiHealth.averageLatency,
  consecutiveFailures: openaiHealth.consecutiveFailures
});
```

## Cost Management

### Cost Tracking

```typescript
.enableCostTracking({
  enabled: true,
  budgetLimit: 100,        // $100 monthly budget
  alertThreshold: 80,      // Alert at 80% usage
  trackPerUser: true,      // Track costs per user
  trackPerProject: true    // Track costs per project
})
```

### Cost Estimation

```typescript
// Estimate cost before making request
const estimatedCost = router.estimateCost({
  model: 'gpt-4',
  messages: [{ role: 'user', content: 'Long message...' }],
  max_tokens: 1000
});

console.log(`Estimated cost: $${estimatedCost.toFixed(4)}`);
```

### Cost Analytics

```typescript
// Get overall statistics
const stats = router.getStats();
console.log({
  totalRequests: stats.totalRequests,
  totalCost: stats.totalCost,
  averageCostPerRequest: stats.totalCost / stats.totalRequests
});

// Get per-provider costs
Object.entries(stats.providerStats).forEach(([provider, stats]) => {
  console.log(`${provider}: $${stats.cost.toFixed(4)} (${stats.requests} requests)`);
});
```

## Error Handling

### Error Types

```typescript
import {
  LLMError,
  LLMProviderError,
  LLMProviderNotFoundError,
  LLMModelNotSupportedError,
  LLMRateLimitError,
  LLMQuotaExceededError
} from 'circuit-breaker-sdk';

try {
  const response = await router.chatCompletion(request);
} catch (error) {
  if (error instanceof LLMRateLimitError) {
    console.log('Rate limit exceeded, retrying...');
    // Implement retry logic
  } else if (error instanceof LLMProviderNotFoundError) {
    console.log('Provider not available, using fallback...');
    // Implement fallback logic
  } else if (error instanceof LLMModelNotSupportedError) {
    console.log('Model not supported, trying alternative...');
    // Try different model
  }
}
```

### Retry Configuration

```typescript
.setFailover({
  enabled: true,
  maxRetries: 3,
  backoffStrategy: 'exponential',  // exponential, linear, fixed
  baseDelay: 1000                  // Base delay in milliseconds
})
```

### Error Events

```typescript
router.on('requestFailed', (data) => {
  console.log(`Request failed: ${data.error} (attempt ${data.retryCount})`);
});

router.on('requestComplete', (data) => {
  console.log(`Request completed: ${data.provider} (${data.latency}ms)`);
});
```

## Examples

### Example 1: Basic Usage

```typescript
import { createLLMBuilder } from 'circuit-breaker-sdk';

async function basicExample() {
  const router = await createLLMBuilder()
    .addOpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-3.5-turbo']
    })
    .build();

  const response = await router.router.chatCompletion({
    model: 'gpt-3.5-turbo',
    messages: [{ role: 'user', content: 'Hello!' }]
  });

  console.log(response.choices[0].message.content);
  await router.router.destroy();
}
```

### Example 2: Multi-Provider with Failover

```typescript
import { createMultiProviderBuilder } from 'circuit-breaker-sdk';

async function multiProviderExample() {
  const router = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
    anthropic: process.env.ANTHROPIC_API_KEY
  }).build();

  const response = await router.router.chatCompletion({
    model: 'gpt-4',
    messages: [{ role: 'user', content: 'Explain quantum computing.' }]
  });

  console.log('Provider used:', response.routingInfo?.selectedProvider);
  console.log('Response:', response.choices[0].message.content);
  
  await router.router.destroy();
}
```

### Example 3: Cost-Optimized Setup

```typescript
import { createCostOptimizedBuilder } from 'circuit-breaker-sdk';

async function costOptimizedExample() {
  const router = await createCostOptimizedBuilder({
    openai: process.env.OPENAI_API_KEY,
    anthropic: process.env.ANTHROPIC_API_KEY
  }).build();

  const request = {
    model: 'gpt-3.5-turbo',
    messages: [{ role: 'user', content: 'Summarize AI trends.' }]
  };

  const estimatedCost = router.router.estimateCost(request);
  console.log(`Estimated cost: $${estimatedCost.toFixed(4)}`);

  const response = await router.router.chatCompletion(request);
  console.log(response.choices[0].message.content);

  const stats = router.router.getStats();
  console.log(`Actual cost: $${stats.totalCost.toFixed(4)}`);
  
  await router.router.destroy();
}
```

### Example 4: Streaming

```typescript
async function streamingExample() {
  const router = await createLLMBuilder()
    .addOpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-3.5-turbo']
    })
    .build();

  const stream = router.router.chatCompletionStream({
    model: 'gpt-3.5-turbo',
    messages: [{ role: 'user', content: 'Write a detailed story...' }],
    stream: true
  });

  console.log('Streaming response:');
  for await (const chunk of stream) {
    const content = chunk.choices[0]?.delta?.content || '';
    process.stdout.write(content);
  }
  console.log('\nStream completed!');
  
  await router.router.destroy();
}
```

### Example 5: Advanced Configuration

```typescript
async function advancedExample() {
  const router = await createLLMBuilder({
    timeout: 15000,
    maxRetries: 2,
    debug: true
  })
    .addOpenAI({
      name: 'openai-primary',
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-4', 'gpt-3.5-turbo'],
      priority: 1,
      rateLimit: {
        requestsPerMinute: 100,
        tokensPerMinute: 50000
      }
    })
    .addAnthropic({
      name: 'anthropic-secondary',
      apiKey: process.env.ANTHROPIC_API_KEY,
      models: ['claude-3-sonnet'],
      priority: 2
    })
    .setRoutingStrategy('performance-first')
    .enableHealthChecks({
      interval: 30,
      timeout: 5000,
      retries: 2
    })
    .enableCostTracking({
      budgetLimit: 50,
      alertThreshold: 80
    })
    .setFailover({
      enabled: true,
      maxRetries: 3,
      backoffStrategy: 'exponential'
    })
    .build();

  // Set up monitoring
  router.router.on('requestComplete', (data) => {
    console.log(`✅ ${data.provider}: ${data.latency}ms`);
  });

  router.router.on('healthCheck', (data) => {
    console.log(`🏥 ${data.provider}: ${data.isHealthy ? 'healthy' : 'unhealthy'}`);
  });

  const response = await router.router.chatCompletion({
    model: 'gpt-4',
    messages: [{ role: 'user', content: 'Explain machine learning.' }]
  });

  console.log('Response:', response.choices[0].message.content);
  
  // Get health status
  const health = router.router.getProviderHealth();
  console.log('Provider health:', health);
  
  // Get statistics
  const stats = router.router.getStats();
  console.log('Router stats:', {
    totalRequests: stats.totalRequests,
    successRate: (stats.successfulRequests / stats.totalRequests) * 100,
    averageLatency: stats.averageLatency,
    totalCost: stats.totalCost
  });
  
  await router.router.destroy();
}
```

## API Reference

### LLMRouter

#### Methods

- `chatCompletion(request: ChatCompletionRequest): Promise<ChatCompletionResponse>`
- `chatCompletionStream(request: ChatCompletionRequest): AsyncGenerator<ChatCompletionChunk>`
- `getStats(): LLMRouterStats`
- `getProviderHealth(providerName?: string): ProviderHealth | ProviderHealth[]`
- `getProviders(): string[]`
- `getAvailableModels(): Record<string, string[]>`
- `supportsModel(model: string): boolean`
- `estimateCost(request: ChatCompletionRequest): number`
- `destroy(): Promise<void>`

#### Events

- `requestComplete`: Emitted when a request completes successfully
- `requestFailed`: Emitted when a request fails
- `healthCheck`: Emitted after health checks
- `providerError`: Emitted when a provider encounters an error

### LLMBuilder

#### Methods

- `addOpenAI(config: ProviderBuilderConfig): LLMBuilder`
- `addAnthropic(config: ProviderBuilderConfig): LLMBuilder`
- `addOllama(config: ProviderBuilderConfig): LLMBuilder`
- `addProvider(config: LLMProviderConfig): LLMBuilder`
- `setRoutingStrategy(strategy: RoutingStrategy): LLMBuilder`
- `setDefaultProvider(providerName: string): LLMBuilder`
- `enableHealthChecks(config?: HealthCheckBuilderConfig): LLMBuilder`
- `enableCostTracking(config?: CostTrackingBuilderConfig): LLMBuilder`
- `setFailover(config: FailoverBuilderConfig): LLMBuilder`
- `setTimeout(timeout: number): LLMBuilder`
- `setMaxRetries(retries: number): LLMBuilder`
- `validate(): ValidationResult`
- `build(): Promise<LLMBuilderResult>`

### Factory Functions

- `createLLMBuilder(config?: LLMBuilderConfig): LLMBuilder`
- `createMultiProviderBuilder(apiKeys: object): MultiProviderBuilder`
- `createCostOptimizedBuilder(apiKeys: object): CostOptimizedBuilder`
- `createPerformanceBuilder(apiKeys: object): PerformanceBuilder`

## Best Practices

### 1. Provider Configuration

- Always configure multiple providers for reliability
- Set appropriate rate limits to avoid hitting API limits
- Use priority ordering for failover scenarios

### 2. Error Handling

- Implement comprehensive error handling for all error types
- Use appropriate retry strategies with exponential backoff
- Monitor provider health and adjust routing accordingly

### 3. Cost Management

- Enable cost tracking for production deployments
- Set budget limits and monitoring alerts
- Use cost-optimized routing for high-volume applications

### 4. Performance Optimization

- Use streaming for long-form content generation
- Enable health checks for automatic failover
- Monitor latency and adjust provider selection

### 5. Security

- Store API keys securely (environment variables, key management services)
- Use provider-specific endpoints when available
- Implement request logging and audit trails

### 6. Monitoring

- Set up comprehensive logging and monitoring
- Track key metrics (latency, cost, success rate)
- Implement alerting for critical failures

## Troubleshooting

### Common Issues

#### Provider Not Available
```
Error: LLM provider not found: provider-name
```
**Solution:** Check provider configuration and API key validity

#### Model Not Supported
```
Error: Model not supported: model-name
```
**Solution:** Verify model is in provider's model list

#### Rate Limit Exceeded
```
Error: Rate limit exceeded for provider: provider-name
```
**Solution:** Configure rate limits or implement backoff

#### Authentication Failed
```
Error: Authentication failed: Invalid API key
```
**Solution:** Verify API key is correct and has proper permissions

### Debug Mode

Enable debug mode to get detailed logging:

```typescript
const router = await createLLMBuilder({ debug: true })
  .addOpenAI({ /* config */ })
  .build();
```

### Health Checks

Monitor provider health to identify issues:

```typescript
const health = router.getProviderHealth();
health.forEach(h => {
  if (!h.isHealthy) {
    console.log(`Provider ${h.provider} is unhealthy: ${h.lastError}`);
  }
});
```

## Migration Guide

### From Direct Provider APIs

If you're currently using provider APIs directly:

```typescript
// Before: Direct OpenAI usage
import OpenAI from 'openai';
const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
const response = await openai.chat.completions.create({
  model: 'gpt-3.5-turbo',
  messages: [{ role: 'user', content: 'Hello' }]
});

// After: Circuit Breaker LLM Router
import { createLLMBuilder } from 'circuit-breaker-sdk';
const router = await createLLMBuilder()
  .addOpenAI({ apiKey: process.env.OPENAI_API_KEY })
  .build();
const response = await router.router.chatCompletion({
  model: 'gpt-3.5-turbo',
  messages: [{ role: 'user', content: 'Hello' }]
});
```

### Benefits of Migration

1. **Multi-provider support** with automatic failover
2. **Cost optimization** and tracking
3. **Health monitoring** and reliability
4. **Unified interface** across providers
5. **Advanced routing** strategies

## Changelog

### v1.0.0
- Initial release with OpenAI, Anthropic, and Ollama support
- Basic routing strategies implementation
- Health monitoring and cost tracking
- Streaming support
- Comprehensive error handling

### Roadmap

- Additional provider support (Cohere, HuggingFace, etc.)
- Advanced caching mechanisms
- Load balancing improvements
- Enhanced analytics and reporting
- GraphQL integration for configuration management