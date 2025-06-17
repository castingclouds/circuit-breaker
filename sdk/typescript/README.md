# Circuit Breaker TypeScript SDK

A comprehensive TypeScript SDK for building and managing workflows using the Circuit Breaker workflow engine. This SDK provides type-safe APIs for workflows, resources, rules engine, functions, LLM integration, and AI agents.

[![npm version](https://badge.fury.io/js/circuit-breaker-sdk.svg)](https://badge.fury.io/js/circuit-breaker-sdk)
[![TypeScript](https://img.shields.io/badge/%3C%2F%3E-TypeScript-%230074c1.svg)](http://www.typescriptlang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🔄 **Fluent Workflow Builder** - Intuitive API for creating complex workflows
- 📊 **Resource Management** - Track and manage workflow execution state
- 🎯 **Rules Engine** - Powerful rule evaluation for state transitions
- 🔧 **Function System** - Docker-based serverless function execution
- 🤖 **LLM Integration** - Multi-provider AI routing with intelligent failover
- 🧠 **AI Agents** - Conversational and state machine agents with memory
- 🎯 **Agent Builder** - Fluent API for creating sophisticated AI agents
- 🌐 **GraphQL API** - Type-safe communication with Circuit Breaker server
- 📝 **Full TypeScript Support** - Complete type safety and IntelliSense
- 🚨 **Error Handling** - Comprehensive error types and handling
- 📊 **Monitoring** - Built-in logging, metrics, and health checks

## Installation

```bash
npm install circuit-breaker-sdk
```

Or with yarn:

```bash
yarn add circuit-breaker-sdk
```

## Quick Start

```typescript
import { CircuitBreakerSDK, createWorkflow } from 'circuit-breaker-sdk';

// Create SDK instance
const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql'
});

// Build a workflow
const workflow = createWorkflow('Order Processing')
  .addState('pending')
  .addState('processing')
  .addState('completed')
  .addTransition('pending', 'processing', 'start_processing')
  .addTransition('processing', 'completed', 'complete_order')
  .setInitialState('pending')
  .build();

// Create the workflow
const workflowId = await sdk.workflows.create(workflow);

// Create a resource
const resource = await sdk.resources.create({
  workflowId,
  data: { orderId: 'order-123', amount: 99.99 }
});

// Execute a state transition
const result = await sdk.resources.executeActivity({
  resourceId: resource.id,
  activityId: 'start_processing',
  data: { processedBy: 'system' }
});

console.log(`Order is now in state: ${result.state}`);
```

## Core Concepts

### Workflows

Workflows define the structure and flow of your business processes using states and activities (transitions).

```typescript
const workflow = createWorkflow('Document Approval')
  .addStates(['submitted', 'under_review', 'approved', 'rejected'])
  .setInitialState('submitted')
  .addTransition('submitted', 'under_review', 'start_review')
  .addTransition('under_review', 'approved', 'approve')
  .addTransition('under_review', 'rejected', 'reject')
  .build();
```

### Resources

Resources are instances of workflows that track the current state and data.

```typescript
const resource = await sdk.resources.create({
  workflowId: 'workflow-123',
  data: {
    documentId: 'doc-456',
    title: 'Budget Proposal',
    submittedBy: 'john.doe'
  },
  metadata: {
    priority: 'high',
    department: 'finance'
  }
});
```

### Rules Engine

Add business logic to control state transitions:

```typescript
// Simple field-based rules
const workflow = createWorkflow('Order Processing')
  .addTransition('pending', 'processing', 'start_processing')
  .addSimpleRule('start_processing', 'payment_verified', '==', true)
  .addSimpleRule('start_processing', 'inventory_available', '>', 0);

// Custom rule evaluators
sdk.rules.registerRule('business_hours', {
  name: 'business_hours',
  type: 'custom',
  evaluator: async (context) => {
    const hour = new Date().getHours();
    return hour >= 9 && hour <= 17;
  },
  description: 'Check if current time is within business hours'
});
```

## Advanced Features

### Function System

Execute containerized functions as part of your workflows:

```typescript
const processor = await sdk.functions.createFunction({
  id: 'order-processor',
  name: 'Order Data Processor',
  container: {
    image: 'node:18-alpine',
    command: ['node', 'process-order.js']
  },
  triggers: [{
    type: 'resource_state',
    condition: 'state == "processing"',
    inputMapping: 'full_data'
  }],
  inputSchema: {
    type: 'object',
    properties: {
      orderId: { type: 'string' },
      items: { type: 'array' }
    }
  }
});
```

### LLM Integration

Integrate with multiple LLM providers:

```typescript
// Configure providers
await sdk.llm.addProvider('openai', {
  apiKey: process.env.OPENAI_API_KEY,
  baseURL: 'https://api.openai.com/v1'
});

await sdk.llm.addProvider('claude', {
  apiKey: process.env.ANTHROPIC_API_KEY,
  baseURL: 'https://api.anthropic.com'
});

// Use with automatic failover
const completion = await sdk.llm.chat({
  model: 'gpt-4',
  messages: [
    { role: 'user', content: 'Analyze this order data...' }
  ],
  max_tokens: 500
});
```

### AI Agents

Build conversational and state machine agents:

```typescript
// Conversational agent
const agent = sdk.agentBuilder('Customer Service Bot')
  .conversational()
  .setSystemPrompt('You are a helpful customer service representative')
  .setLLMProvider('openai-gpt4')
  .addWorkflowIntegration('customer-support-workflow')
  .enableMemory(true)
  .build();

// State machine agent
const stateMachineAgent = sdk.agentBuilder('Order Assistant')
  .stateMachine()
  .addState('greeting', 'Welcome! How can I help you today?')
  .addState('collecting_info', 'Please provide your order details.')
  .addState('processing', 'Processing your request...')
  .addTransition('greeting', 'collecting_info', 'user_responds')
  .addTransition('collecting_info', 'processing', 'info_complete')
  .build();
```

### Advanced Workflow Patterns

Create complex workflow patterns:

```typescript
// Branching workflow
const workflow = createWorkflow('Order Routing')
  .addState('received')
  .addState('express_processing')
  .addState('standard_processing')
  .addState('completed')
  .branch('received', [
    { condition: 'data.priority == "high"', targetState: 'express_processing', activityId: 'route_express' },
    { condition: 'data.amount > 1000', targetState: 'express_processing', activityId: 'route_high_value' }
  ])
  .otherwise('standard_processing', 'route_standard')
  .addTransition('express_processing', 'completed', 'complete_express')
  .addTransition('standard_processing', 'completed', 'complete_standard')
  .build();

// Parallel execution
const parallelWorkflow = createWorkflow('Order Fulfillment')
  .parallel('order_confirmed', [
    {
      name: 'inventory',
      states: ['reserve_inventory', 'inventory_confirmed'],
      activities: [/* inventory activities */],
      joinState: 'inventory_ready'
    },
    {
      name: 'shipping',
      states: ['calculate_shipping', 'shipping_confirmed'],
      activities: [/* shipping activities */],
      joinState: 'shipping_ready'
    }
  ])
  .joinAt('ready_to_ship')
  .build();
```

## Configuration

### SDK Configuration

```typescript
const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql',
  timeout: 30000,
  debug: true,
  headers: {
    'Authorization': 'Bearer your-token',
    'User-Agent': 'MyApp/1.0.0'
  },
  logging: {
    level: 'info',
    structured: true,
    logger: (level, message, meta) => {
      console.log(`[${level}] ${message}`, meta);
    }
  },
  rulesConfig: {
    enableCache: true,
    cacheSize: 1000,
    evaluationTimeout: 5000
  }
});
```

### Environment Variables

```bash
# Required
CIRCUIT_BREAKER_ENDPOINT=http://localhost:4000/graphql

# Optional
CIRCUIT_BREAKER_TIMEOUT=30000
CIRCUIT_BREAKER_DEBUG=true

# For LLM integration
OPENAI_API_KEY=your-openai-key
ANTHROPIC_API_KEY=your-anthropic-key
```

## Error Handling

The SDK provides comprehensive error handling with specific error types:

```typescript
import {
  CircuitBreakerError,
  WorkflowError,
  ResourceError,
  RuleError,
  FunctionError,
  LLMError,
  NetworkError
} from 'circuit-breaker-sdk';

try {
  await sdk.resources.executeActivity({
    resourceId: 'invalid-id',
    activityId: 'some-activity'
  });
} catch (error) {
  if (error instanceof ResourceError) {
    console.log('Resource error:', error.message);
    console.log('Error code:', error.code);
    console.log('Context:', error.context);
  } else if (error instanceof NetworkError) {
    console.log('Network error:', error.message);
    if (error.isRetryable()) {
      // Retry logic
    }
  } else {
    console.log('Unknown error:', error);
  }
}
```

## Monitoring and Observability

### Health Checks

```typescript
const health = await sdk.getHealth();
console.log('System health:', health.healthy);
console.log('Components:', health.components);
```

### Statistics

```typescript
const stats = sdk.getStats();
console.log('Total requests:', stats.requests.total);
console.log('Success rate:', stats.requests.successful / stats.requests.total);
console.log('Average response time:', stats.requests.averageResponseTime);
```

### Logging

```typescript
import { createLogger } from 'circuit-breaker-sdk';

const logger = createLogger({
  level: 'debug',
  structured: true,
  component: 'my-service'
});

logger.info('Processing order', { orderId: 'order-123' });
logger.error('Order failed', { error: 'Payment declined' });
```

## Examples

Check out the `examples/` directory for comprehensive examples:

- `basic-workflow.ts` - Complete workflow creation and execution
- `rules-demo.ts` - Advanced rules engine usage
- `function-chains.ts` - Function system integration
- `llm-integration.ts` - LLM provider setup and usage
- `ai-agent.ts` - Building conversational AI agents

Run examples:

```bash
npm run example:basic
npm run example:rules
npm run example:functions
npm run example:llm
npm run example:agents
```

## API Reference

### Core Classes

- **`CircuitBreakerSDK`** - Main SDK client
- **`WorkflowBuilder`** - Fluent workflow construction
- **`WorkflowManager`** - Workflow CRUD operations
- **`ResourceManager`** - Resource lifecycle management
- **`RulesEngine`** - Rule evaluation and management
- **`FunctionManager`** - Function system integration
- **`LLMRouter`** - Multi-provider LLM routing
- **`AgentBuilder`** - AI agent construction

### Utility Classes

- **`GraphQLClient`** - Type-safe GraphQL communication
- **`Logger`** - Structured logging
- **`ErrorHandler`** - Error management utilities

For complete API documentation, visit: [https://docs.circuit-breaker.dev/sdk/typescript](https://docs.circuit-breaker.dev/sdk/typescript)

## TypeScript Support

This SDK is built with TypeScript and provides complete type safety:

```typescript
import type {
  WorkflowDefinition,
  ActivityDefinition,
  Resource,
  Rule,
  FunctionDefinition,
  ChatCompletionRequest
} from 'circuit-breaker-sdk';

// All types are fully typed and validated
const workflow: WorkflowDefinition = {
  name: 'My Workflow',
  states: ['start', 'end'],
  activities: [],
  initialState: 'start'
};
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/circuit-breaker/sdk.git
cd sdk/typescript

# Install dependencies
npm install

# Build the SDK
npm run build

# Run tests
npm test

# Run examples
npm run example:basic
```

### Testing

```bash
# Unit tests
npm run test

# Integration tests
npm run test:integration

# End-to-end tests
npm run test:e2e

# Test coverage
npm run test:coverage
```

## Roadmap

- [ ] **Real-time Subscriptions** - GraphQL subscriptions for live updates
- [ ] **Batch Operations** - Bulk workflow and resource operations
- [ ] **Workflow Analytics** - Built-in metrics and analytics
- [ ] **Plugin System** - Extensible plugin architecture
- [ ] **Visual Workflow Editor** - Browser-based workflow designer
- [ ] **Workflow Templates** - Pre-built workflow patterns
- [ ] **Advanced Scheduling** - Cron-based and time-triggered workflows
- [ ] **Multi-tenant Support** - Workspace and tenant isolation

## Support

- **Documentation**: [https://docs.circuit-breaker.dev](https://docs.circuit-breaker.dev)
- **Examples**: [examples/](examples/)
- **Issues**: [GitHub Issues](https://github.com/circuit-breaker/sdk/issues)
- **Discussions**: [GitHub Discussions](https://github.com/circuit-breaker/sdk/discussions)
- **Discord**: [Join our community](https://discord.gg/circuit-breaker)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and migration guides.

---

Built with ❤️ by the Circuit Breaker team.