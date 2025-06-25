# Circuit Breaker TypeScript SDK

A simple, clean TypeScript client for the Circuit Breaker workflow engine. This SDK mirrors the Rust SDK approach with minimal abstractions and direct API access.

## Installation

```bash
npm install circuit-breaker-sdk
```

## Quick Start

```typescript
import { Client, createSDK } from 'circuit-breaker-sdk';

// Create a client
const client = Client.builder()
  .baseUrl('https://api.circuit-breaker.dev')
  .apiKey('your-api-key')
  .build();

// Test connection
const info = await client.ping();
console.log('Connected to Circuit Breaker v' + info.version);

// Or use the convenience SDK class
const sdk = createSDK({
  baseUrl: 'https://api.circuit-breaker.dev',
  apiKey: 'your-api-key'
});

// Create a workflow
const workflow = await sdk.workflows().create({
  name: 'My Workflow',
  description: 'Example workflow',
  definition: {
    states: [
      { name: 'pending', type: 'normal' },
      { name: 'completed', type: 'final' }
    ],
    transitions: [
      { from: 'pending', to: 'completed', event: 'complete' }
    ],
    initial_state: 'pending'
  }
});

// Execute the workflow
const execution = await sdk.workflows().execute(workflow.id);
console.log('Workflow executed:', execution.id);
```

## Core Concepts

### Client

The `Client` class is the core HTTP client that handles communication with the Circuit Breaker server:

```typescript
const client = Client.builder()
  .baseUrl('http://localhost:3000')
  .apiKey('your-api-key')
  .timeout(30000)
  .header('X-Custom-Header', 'value')
  .build();
```

### API Clients

Each domain has its own client accessible through the main client:

- **Workflows**: `client.workflows()`
- **Agents**: `client.agents()`
- **Functions**: `client.functions()`
- **Resources**: `client.resources()`
- **Rules**: `client.rules()`
- **LLM**: `client.llm()`

## API Reference

### Workflows

```typescript
const workflowClient = client.workflows();

// Create workflow
const workflow = await workflowClient.create({
  name: 'Order Processing',
  definition: {
    states: [
      { name: 'pending', type: 'normal' },
      { name: 'processing', type: 'normal' },
      { name: 'completed', type: 'final' }
    ],
    transitions: [
      { from: 'pending', to: 'processing', event: 'start' },
      { from: 'processing', to: 'completed', event: 'finish' }
    ],
    initial_state: 'pending'
  }
});

// Execute workflow
const execution = await workflowClient.execute(workflow.id, { orderId: '123' });

// Get execution status
const status = await workflowClient.getExecution(execution.id);
```

### Workflow Builder

```typescript
import { createWorkflow } from 'circuit-breaker-sdk';

const workflow = createWorkflow('Order Processing')
  .setDescription('Process customer orders')
  .addState('pending')
  .addState('processing')
  .addState('completed', 'final')
  .addTransition('pending', 'processing', 'start')
  .addTransition('processing', 'completed', 'finish')
  .setInitialState('pending')
  .build();

const created = await client.workflows().create(workflow);
```

### Agents

```typescript
const agentClient = client.agents();

// Create agent
const agent = await agentClient.create({
  name: 'Customer Support',
  type: 'conversational',
  config: {
    llm_provider: 'openai',
    model: 'gpt-4',
    temperature: 0.7,
    system_prompt: 'You are a helpful customer support agent.'
  }
});

// Chat with agent
const response = await agentClient.chat(agent.id, [
  { role: 'user', content: 'How can I return an item?' }
]);
```

### Agent Builder

```typescript
import { createAgent } from 'circuit-breaker-sdk';

const agent = createAgent('Support Bot')
  .setType('conversational')
  .setLLMProvider('openai')
  .setModel('gpt-4')
  .setTemperature(0.7)
  .setSystemPrompt('You are a helpful assistant.')
  .addTool('search', 'Search knowledge base', { query: 'string' })
  .build();

const created = await client.agents().create(agent);
```

### Functions

```typescript
const functionClient = client.functions();

// Create JavaScript function
const func = await functionClient.create({
  name: 'calculate-tax',
  runtime: 'node18',
  code: `
    exports.handler = async (input) => {
      const { amount, rate } = input;
      return { tax: amount * rate };
    };
  `
});

// Execute function
const result = await functionClient.execute(func.id, { amount: 100, rate: 0.08 });
```

### Function Helpers

```typescript
import { createJavaScriptFunction, createPythonFunction, createDockerFunction } from 'circuit-breaker-sdk';

// JavaScript function
const jsFunc = createJavaScriptFunction(
  'validate-email',
  'exports.handler = (input) => ({ valid: input.email.includes("@") });'
);

// Python function
const pyFunc = createPythonFunction(
  'data-processing',
  'def handler(input): return {"result": input["data"] * 2}'
);

// Docker function
const dockerFunc = createDockerFunction(
  'image-processor',
  'python:3.9-slim',
  { command: ['python', 'process.py'] }
);
```

### Resources

```typescript
const resourceClient = client.resources();

// Create resource
const resource = await resourceClient.create({
  workflow_id: workflow.id,
  data: { orderId: 'ORD-123', amount: 99.99 }
});

// Transition resource state
const updated = await resourceClient.transition(resource.id, 'processing');

// Execute activity
const result = await resourceClient.executeActivity(resource.id, 'validate-order');
```

### Rules

```typescript
const ruleClient = client.rules();

// Create rule
const rule = await ruleClient.create({
  name: 'High Value Order',
  type: 'simple',
  definition: {
    conditions: [
      { field: 'amount', operator: 'greater_than', value: 1000 }
    ],
    actions: [
      { type: 'webhook', config: { url: 'https://api.example.com/alerts' } }
    ]
  }
});

// Evaluate rule
const result = await ruleClient.evaluate(rule.id, { amount: 1500 });
```

### Rule Builder

```typescript
import { createRule } from 'circuit-breaker-sdk';

const rule = createRule('Discount Eligibility')
  .greaterThan('amount', 100)
  .equals('customer_type', 'premium')
  .setCombinator('and')
  .webhook('https://api.example.com/discount')
  .build();

const created = await client.rules().create(rule);
```

### LLM

```typescript
const llmClient = client.llm();

// Simple chat
const response = await llmClient.chat('gpt-4', 'Hello, how are you?');

// Full chat completion
const completion = await llmClient.chatCompletion({
  model: 'gpt-4',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'Explain quantum computing' }
  ],
  temperature: 0.7,
  max_tokens: 500
});

// Chat builder
import { createChat } from 'circuit-breaker-sdk';

const chat = createChat('gpt-4')
  .setSystemPrompt('You are a coding assistant.')
  .addUserMessage('Write a Python function to sort a list')
  .setTemperature(0.2);

const response = await chat.execute(llmClient);
```

### SSE Streaming

The SDK provides comprehensive Server-Sent Events (SSE) streaming for real-time LLM interactions:

```typescript
import { 
  SSEParser, 
  responseToSSEStream, 
  parseProviderEvent, 
  Anthropic, 
  OpenAI, 
  Google 
} from 'circuit-breaker-sdk';

// Real-time streaming chat completion
await llmClient.streamChatCompletion(
  {
    model: 'claude-3-haiku-20240307',
    messages: [{ role: 'user', content: 'Write a poem about AI' }],
    temperature: 0.7,
    stream: true
  },
  (chunk) => {
    // Handle each streaming chunk
    const content = chunk.choices[0]?.delta?.content;
    if (content) {
      process.stdout.write(content);
    }
    
    if (chunk.choices[0]?.finish_reason) {
      console.log('\n✅ Stream completed');
    }
  },
  (error) => {
    console.error('Streaming error:', error);
  }
);
```

#### Manual SSE Parsing

```typescript
// Parse SSE events manually
const parser = new SSEParser();
const events = parser.parseChunk('data: {"text": "hello"}\n\n');

// Convert HTTP response to SSE stream
const response = await fetch('/api/stream');
for await (const event of responseToSSEStream(response)) {
  console.log('SSE Event:', event.data);
}

// Provider-specific parsing
const anthropicChunk = Anthropic.eventToChunk(event, 'req-123', 'claude-3');
const openaiChunk = OpenAI.eventToChunk(event, 'req-123', 'gpt-4');
const googleChunk = Google.eventToChunk(event, 'req-123', 'gemini-pro');

// Auto-detect provider format
const chunk = parseProviderEvent(event, 'req-123', 'model-name');
```

#### SSE Error Handling

```typescript
import { SSEError, SSEParseError, SSEStreamError } from 'circuit-breaker-sdk';

try {
  // SSE operations
} catch (error) {
  if (error instanceof SSEStreamError) {
    console.log('Stream error:', error.statusCode, error.message);
  } else if (error instanceof SSEParseError) {
    console.log('Parse error:', error.rawData);
  } else if (error instanceof SSEError) {
    console.log('SSE error:', error.provider, error.message);
  }
}
```

### Conversations

```typescript
import { createConversation } from 'circuit-breaker-sdk';

const conversation = createConversation(llmClient, 'gpt-4', {
  systemPrompt: 'You are a helpful assistant.',
  maxContextLength: 4000
});

const response1 = await conversation.send('What is TypeScript?');
const response2 = await conversation.send('How is it different from JavaScript?');

// Get conversation history
const history = conversation.getHistory();
```

## Error Handling

The SDK provides typed error classes:

```typescript
import { CircuitBreakerError, NetworkError, ValidationError, NotFoundError } from 'circuit-breaker-sdk';

try {
  const workflow = await client.workflows().get('invalid-id');
} catch (error) {
  if (error instanceof NotFoundError) {
    console.log('Workflow not found');
  } else if (error instanceof NetworkError) {
    console.log('Network issue:', error.message);
  } else if (error instanceof ValidationError) {
    console.log('Invalid request:', error.message);
  } else if (error instanceof CircuitBreakerError) {
    console.log('Circuit Breaker error:', error.code);
  }
}
```

## Configuration

### Client Configuration

```typescript
interface ClientConfig {
  baseUrl: string;
  apiKey?: string;
  timeout?: number;
  headers?: Record<string, string>;
}
```

### Environment Variables

You can also configure the SDK using environment variables:

```bash
CIRCUIT_BREAKER_BASE_URL=http://localhost:3000
CIRCUIT_BREAKER_API_KEY=your-api-key
CIRCUIT_BREAKER_TIMEOUT=30000
```

```typescript
const client = Client.builder()
  .baseUrl(process.env.CIRCUIT_BREAKER_BASE_URL || 'http://localhost:3000')
  .apiKey(process.env.CIRCUIT_BREAKER_API_KEY)
  .timeout(parseInt(process.env.CIRCUIT_BREAKER_TIMEOUT || '30000'))
  .build();
```

## Examples

### Complete Workflow Example

```typescript
import { Client, createWorkflow, createResource } from 'circuit-breaker-sdk';

async function main() {
  const client = Client.builder()
    .baseUrl('http://localhost:3000')
    .build();

  // Create workflow
  const workflowDef = createWorkflow('Order Processing')
    .addState('pending')
    .addState('processing')
    .addState('completed', 'final')
    .addTransition('pending', 'processing', 'start')
    .addTransition('processing', 'completed', 'finish')
    .setInitialState('pending')
    .build();

  const workflow = await client.workflows().create(workflowDef);

  // Create resource
  const resourceDef = createResource(workflow.id)
    .addData('orderId', 'ORD-123')
    .addData('amount', 99.99)
    .build();

  const resource = await client.resources().create(resourceDef);

  // Execute workflow
  const execution = await client.workflows().execute(workflow.id, {
    resourceId: resource.id
  });

  console.log('Workflow execution started:', execution.id);
}

main().catch(console.error);
```

## TypeScript Support

The SDK is built with TypeScript and provides full type safety:

```typescript
import type { Workflow, WorkflowExecution, ExecutionStatus } from 'circuit-breaker-sdk';

const workflow: Workflow = await client.workflows().get('workflow-id');
const execution: WorkflowExecution = await client.workflows().execute(workflow.id);
const status: ExecutionStatus = execution.status;
```

## Comparison with Other SDKs

This TypeScript SDK is designed to be simple and focused, similar to the Rust SDK. Unlike more complex SDK implementations, it:

- **Minimal abstractions**: Direct API access without unnecessary layers
- **Builder patterns**: Simple builders for constructing objects
- **No lazy loading**: All functionality is available upfront
- **Focused on client concerns**: No server-side features like health monitoring
- **Type-safe**: Full TypeScript support with proper error handling

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit pull requests to our GitHub repository.

## License

MIT License - see LICENSE file for details.