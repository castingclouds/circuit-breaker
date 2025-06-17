# Circuit Breaker TypeScript SDK Implementation Plan

This document outlines the comprehensive SDK implementation for Circuit Breaker in TypeScript, providing developers with an easy-to-use interface for building workflows using the GraphQL endpoint, OpenAI router, and Functions system.

## SDK Architecture Overview

```
circuit-breaker-sdk/
├── src/
│   ├── core/
│   │   ├── client.ts           # Main SDK client
│   │   ├── types.ts            # Core type definitions
│   │   └── errors.ts           # Error handling
│   ├── workflow/
│   │   ├── manager.ts          # Workflow management
│   │   ├── builder.ts          # Workflow builder pattern
│   │   └── executor.ts         # Workflow execution
│   ├── resources/
│   │   ├── manager.ts          # Resource management
│   │   └── tracker.ts          # Resource state tracking
│   ├── functions/
│   │   ├── manager.ts          # Function system integration
│   │   ├── executor.ts         # Function execution
│   │   └── docker.ts           # Docker container management
│   ├── llm/
│   │   ├── router.ts           # OpenAI-compatible router
│   │   ├── providers.ts        # LLM provider implementations
│   │   └── streaming.ts        # Streaming support
│   ├── agents/
│   │   ├── builder.ts          # AI agent construction
│   │   ├── state-machine.ts    # State machine agents
│   │   └── conversation.ts     # Conversational agents
│   ├── rules/
│   │   ├── engine.ts           # Rules engine for state transitions
│   │   ├── builder.ts          # Rule builder and composition
│   │   ├── evaluator.ts        # Rule evaluation logic
│   │   └── registry.ts         # Global rule registry
│   ├── utils/
│   │   ├── graphql.ts          # GraphQL utilities
│   │   ├── validation.ts       # Input validation
│   │   └── logger.ts           # Logging utilities
│   └── index.ts                # Main exports
├── examples/
│   ├── basic-workflow.ts
│   ├── ai-agent.ts
│   ├── function-chains.ts
│   └── llm-integration.ts
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
├── docs/
│   ├── api-reference.md
│   ├── getting-started.md
│   └── examples.md
├── package.json
├── tsconfig.json
├── README.md
└── CHANGELOG.md
```

## Core Implementation Plan

### 1. Main SDK Client (`src/core/client.ts`)

```typescript
export class CircuitBreakerSDK {
  private graphqlClient: GraphQLClient;
  private llmRouter: LLMRouter;
  private functionManager: FunctionManager;
  
  constructor(config: SDKConfig) {
    this.graphqlClient = new GraphQLClient(config.graphqlEndpoint);
    this.llmRouter = new LLMRouter(config.llmConfig);
    this.functionManager = new FunctionManager(config.functionConfig);
    this.rules = new RulesEngine(config.rulesConfig);
  }
  
  // Workflow management
  workflows: WorkflowManager;
  
  // Resource management
  resources: ResourceManager;
  
  // Function system
  functions: FunctionManager;
  
  // LLM integration
  llm: LLMRouter;
  
  // AI agent builders
  agents: AgentBuilder;
  
  // Rules engine
  rules: RulesEngine;
}
```

### 2. Workflow Builder Pattern (`src/workflow/builder.ts`)

```typescript
export class WorkflowBuilder {
  private workflow: WorkflowDefinition;
  
  constructor(name: string) {
    this.workflow = { name, states: [], activities: [] };
  }
  
  addState(state: string): WorkflowBuilder;
  addTransition(from: string, to: string, activity: string): WorkflowBuilder;
  addRule(activity: string, rule: Rule): WorkflowBuilder;
  addRules(activity: string, rules: Rule[], requireAll?: boolean): WorkflowBuilder;
  addSimpleRule(activity: string, field: string, operator: string, value: any): WorkflowBuilder;
  setInitialState(state: string): WorkflowBuilder;
  
  // Fluent interface for complex workflows
  branch(condition: string): BranchBuilder;
  parallel(): ParallelBuilder;
  loop(condition: string): LoopBuilder;
  
  build(): WorkflowDefinition;
  
  // Rule integration helpers
  withRulesEngine(engine: RulesEngine): WorkflowBuilder;
  validateRules(): Promise<RuleValidationResult>;
}

export class RuleValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}
```

### 3. Function System Integration (`src/functions/manager.ts`)

```typescript
export class FunctionManager {
  async createFunction(definition: FunctionDefinition): Promise<FunctionId>;
  async executeFunction(id: FunctionId, input: any): Promise<FunctionResult>;
  async chainFunctions(chain: FunctionChain[]): Promise<ChainResult>;
  
  // Docker integration
  async deployContainer(config: ContainerConfig): Promise<ContainerId>;
  async executeContainer(id: ContainerId, input: any): Promise<ContainerResult>;
  
  // Event-driven execution
  onWorkflowEvent(event: WorkflowEventType, handler: EventHandler): void;
  onResourceState(state: string, handler: StateHandler): void;
}
```

### 4. LLM Router (`src/llm/router.ts`)

```typescript
export class LLMRouter {
  private providers: Map<string, LLMProvider>;
  
  constructor(config: LLMConfig) {
    this.setupProviders(config);
  }
  
  // OpenAI-compatible interface
  async chat(request: ChatCompletionRequest): Promise<ChatCompletionResponse>;
  async stream(request: ChatCompletionRequest): AsyncIterable<ChatCompletionChunk>;
  
  // Provider management
  addProvider(name: string, provider: LLMProvider): void;
  setDefaultProvider(name: string): void;
  
  // Load balancing and failover
  async routeRequest(request: LLMRequest): Promise<LLMResponse>;
  getProviderHealth(): ProviderHealthStatus[];
}
```

### 5. Rules Engine (`src/rules/engine.ts`)

```typescript
export class RulesEngine {
  private ruleRegistry: Map<string, Rule>;
  private evaluationCache: Map<string, EvaluationResult>;
  
  constructor(config?: RulesConfig) {
    this.ruleRegistry = new Map();
    this.evaluationCache = new Map();
    this.initializeCommonRules();
  }
  
  // Rule management
  registerRule(name: string, rule: Rule): void;
  getRule(name: string): Rule | undefined;
  removeRule(name: string): boolean;
  
  // Rule evaluation
  async canTransition(
    resource: Resource, 
    activity: ActivityDefinition
  ): Promise<boolean>;
  
  async evaluateRules(
    rules: Rule[], 
    context: RuleContext
  ): Promise<RuleEvaluationResult>;
  
  async getAvailableTransitions(
    resource: Resource, 
    workflow: WorkflowDefinition
  ): Promise<ActivityDefinition[]>;
  
  // Rule building helpers
  createRule(name: string): RuleBuilder;
  and(rules: Rule[]): CompositeRule;
  or(rules: Rule[]): CompositeRule;
  not(rule: Rule): NotRule;
  
  // Common predefined rules
  private initializeCommonRules(): void;
}

export class RuleBuilder {
  private rule: Rule;
  
  constructor(name: string) {
    this.rule = { name, type: 'simple', condition: '' };
  }
  
  // Simple conditions
  fieldEquals(field: string, value: any): RuleBuilder;
  fieldGreaterThan(field: string, value: number): RuleBuilder;
  fieldContains(field: string, value: string): RuleBuilder;
  fieldExists(field: string): RuleBuilder;
  
  // Complex conditions
  custom(evaluator: RuleEvaluator): RuleBuilder;
  javascript(expression: string): RuleBuilder;
  
  // Metadata
  description(text: string): RuleBuilder;
  category(category: string): RuleBuilder;
  
  build(): Rule;
}
```

### 6. AI Agent Builder (`src/agents/builder.ts`)

```typescript
export class AgentBuilder {
  private agent: AgentDefinition;
  
  constructor(name: string) {
    this.agent = { name, type: 'conversational' };
  }
  
  // State machine agents
  stateMachine(): StateMachineAgentBuilder;
  
  // Conversational agents
  conversational(): ConversationalAgentBuilder;
  
  // Workflow-integrated agents
  workflowAgent(workflowId: string): WorkflowAgentBuilder;
  
  build(): Agent;
}

export class StateMachineAgentBuilder {
  addState(name: string, prompt: string): StateMachineAgentBuilder;
  addTransition(from: string, to: string, condition: string): StateMachineAgentBuilder;
  setLLMProvider(provider: string): StateMachineAgentBuilder;
  
  build(): StateMachineAgent;
}
```

## Type Definitions (`src/core/types.ts`)

### Core Types

```typescript
export interface SDKConfig {
  graphqlEndpoint: string;
  llmConfig?: LLMConfig;
  functionConfig?: FunctionConfig;
  rulesConfig?: RulesConfig;
  logging?: LoggingConfig;
}

export interface WorkflowDefinition {
  name: string;
  states: string[];
  activities: ActivityDefinition[];
  initialState: string;
  metadata?: Record<string, any>;
}

export interface ActivityDefinition {
  id: string;
  name?: string;
  fromStates: string[];
  toState: string;
  conditions: string[]; // Legacy string-based conditions
  rules?: Rule[];       // New structured rules
  functions?: FunctionTrigger[];
  requiresAllRules?: boolean; // AND vs OR logic for multiple rules
}

export interface Resource {
  id: string;
  workflowId: string;
  state: string;
  data: any;
  metadata: Record<string, any>;
  history: HistoryEvent[];
}
```

### Function System Types

```typescript
export interface FunctionDefinition {
  id: string;
  name: string;
  container: ContainerConfig;
  triggers: EventTrigger[];
  chains: FunctionChain[];
  inputSchema?: JSONSchema;
  outputSchema?: JSONSchema;
}

export interface ContainerConfig {
  image: string;
  command?: string[];
  environment?: Record<string, string>;
  mounts?: ContainerMount[];
  resources?: ResourceLimits;
}

export interface EventTrigger {
  type: 'workflow_event' | 'resource_state' | 'function_completion';
  condition: string;
  inputMapping?: InputMapping;
}
```

### LLM Types

```typescript
export interface LLMConfig {
  providers: LLMProviderConfig[];
  defaultProvider: string;
  loadBalancing?: LoadBalancingConfig;
  failover?: FailoverConfig;
}

export interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}
```

### Rules Engine Types

```typescript
export interface RulesConfig {
  enableCache?: boolean;
  cacheSize?: number;
  customRules?: Record<string, Rule>;
  evaluationTimeout?: number;
}

export interface Rule {
  name: string;
  type: 'simple' | 'composite' | 'custom' | 'javascript';
  condition?: string;
  evaluator?: RuleEvaluator;
  description?: string;
  category?: string;
  metadata?: Record<string, any>;
}

export interface CompositeRule extends Rule {
  type: 'composite';
  operator: 'AND' | 'OR';
  rules: Rule[];
}

export interface NotRule extends Rule {
  type: 'composite';
  operator: 'NOT';
  rule: Rule;
}

export interface RuleContext {
  resource: Resource;
  workflow: WorkflowDefinition;
  activity: ActivityDefinition;
  metadata?: Record<string, any>;
}

export interface RuleEvaluationResult {
  passed: boolean;
  results: RuleResult[];
  errors?: string[];
  evaluationTime: number;
}

export interface RuleResult {
  rule: Rule;
  passed: boolean;
  error?: string;
  context?: any;
}

export type RuleEvaluator = (context: RuleContext) => Promise<boolean> | boolean;
```

## Implementation Phases

### Phase 1: Core SDK Foundation (Week 1-2)
- [ ] Basic GraphQL client with type safety
- [ ] Core error handling and logging
- [ ] Workflow CRUD operations
- [ ] Resource management
- [ ] Unit tests for core functionality

### Phase 2: Workflow Builder & Execution (Week 3-4)
- [ ] Fluent workflow builder API
- [ ] Rules engine integration for state transitions
- [ ] Advanced workflow patterns (branching, parallel, loops)
- [ ] Resource tracking and state management
- [ ] Workflow execution engine with rule evaluation
- [ ] Integration tests

### Phase 3: Function System Integration (Week 5-6)
- [ ] Function definition and management
- [ ] Docker container integration
- [ ] Event-driven function execution
- [ ] Function chaining and composition
- [ ] Container lifecycle management
- [ ] Rule-based function triggering

### Phase 4: LLM Router & AI Integration (Week 7-8)
- [ ] OpenAI-compatible API client
- [ ] Multiple LLM provider support
- [ ] Streaming response handling
- [ ] Load balancing and failover
- [ ] Usage tracking and billing

### Phase 5: AI Agent Framework (Week 9-10)
- [ ] State machine agent builder
- [ ] Conversational agent framework
- [ ] Workflow-integrated agents
- [ ] Agent memory and context management
- [ ] Multi-agent coordination

### Phase 6: Advanced Features (Week 11-12)
- [ ] Real-time subscriptions via GraphQL/WebSocket
- [ ] Batch operations and bulk processing
- [ ] Workflow analytics and monitoring
- [ ] Plugin system for extensibility
- [ ] Performance optimization

## Usage Examples

### Basic Workflow Creation

```typescript
import { CircuitBreakerSDK } from 'circuit-breaker-sdk';

const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql'
});

// Using builder pattern
const workflow = sdk.workflows.builder('Order Processing')
  .addState('pending')
  .addState('processing')
  .addState('completed')
  .addTransition('pending', 'processing', 'start_processing')
  .addTransition('processing', 'completed', 'complete_order')
  .setInitialState('pending')
  .build();

const workflowId = await sdk.workflows.create(workflow);
```

### Rules Engine Integration

```typescript
// Register custom business rules
sdk.rules.registerRule('payment_verified', {
  name: 'payment_verified',
  type: 'simple',
  condition: 'data.payment_status === "verified"',
  description: 'Checks if payment has been verified'
});

sdk.rules.registerRule('inventory_available', {
  name: 'inventory_available',
  type: 'custom',
  evaluator: async (context) => {
    const { resource } = context;
    const items = resource.data.items || [];
    // Custom inventory check logic
    return items.every(item => item.quantity > 0);
  },
  description: 'Validates inventory availability'
});

// Build workflow with rules
const workflow = sdk.workflows.builder('Order Processing')
  .addState('pending')
  .addState('processing')
  .addState('completed')
  .addTransition('pending', 'processing', 'start_processing')
  .addRule('start_processing', sdk.rules.createRule('can_process')
    .fieldEquals('status', 'valid')
    .fieldExists('customer_id')
    .build())
  .addRules('start_processing', [
    sdk.rules.getRule('payment_verified'),
    sdk.rules.getRule('inventory_available')
  ], true) // Require all rules to pass
  .addTransition('processing', 'completed', 'complete_order')
  .setInitialState('pending')
  .withRulesEngine(sdk.rules)
  .build();

// Check if transition is possible
const canStart = await sdk.rules.canTransition(resource, activity);
const availableTransitions = await sdk.rules.getAvailableTransitions(resource, workflow);

// Evaluate specific rules
const evaluation = await sdk.rules.evaluateRules([
  sdk.rules.getRule('payment_verified'),
  sdk.rules.getRule('inventory_available')
], {
  resource,
  workflow,
  activity
});
```

### Function Integration

```typescript
// Create a data processing function
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

// Chain with notification function
await sdk.functions.chain([
  processor,
  'notification-sender'
], {
  condition: 'success',
  delay: '5s'
});
```

### AI Agent Creation

```typescript
// Create a customer service agent
const agent = sdk.agents.conversational('Customer Service Bot')
  .setSystemPrompt('You are a helpful customer service representative')
  .setLLMProvider('openai-gpt4')
  .addWorkflowIntegration('customer-support-workflow')
  .enableMemory(true)
  .build();

// Deploy agent
const agentId = await sdk.agents.deploy(agent);

// Use agent in conversation
const response = await sdk.agents.chat(agentId, {
  message: 'I need help with my order',
  context: { customerId: 'cust_123' }
});
```

### LLM Router Usage

```typescript
// Configure multiple providers
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
    { role: 'user', content: 'Explain quantum computing' }
  ],
  max_tokens: 500
});

// Streaming
for await (const chunk of sdk.llm.stream(request)) {
  process.stdout.write(chunk.choices[0]?.delta?.content || '');
}
```

## Error Handling Strategy

```typescript
export class CircuitBreakerError extends Error {
  constructor(
    message: string,
    public code: string,
    public context?: any
  ) {
    super(message);
    this.name = 'CircuitBreakerError';
  }
}

// Specific error types
export class WorkflowError extends CircuitBreakerError {}
export class ResourceError extends CircuitBreakerError {}
export class FunctionError extends CircuitBreakerError {}
export class LLMError extends CircuitBreakerError {}
export class RuleError extends CircuitBreakerError {}
export class RuleEvaluationError extends CircuitBreakerError {}
```

## Testing Strategy

### Unit Tests
- Core client functionality
- Workflow builder validation
- Rules engine evaluation logic
- Rule composition and validation
- Type safety and serialization
- Error handling

### Integration Tests
- GraphQL API integration
- Function execution
- LLM provider communication
- Agent interactions
- Rules engine with real workflows
- State transition validation

### End-to-End Tests
- Complete workflow scenarios with complex rules
- Multi-component integration
- Performance benchmarks
- Real-world use cases with business logic
- Rule-driven workflow automation

## Documentation Plan

### API Reference
- Comprehensive API documentation
- Type definitions and interfaces
- Method signatures and examples
- Error codes and handling

### Getting Started Guide
- Installation and setup
- Basic concepts and terminology
- Step-by-step tutorials
- Common patterns and best practices

### Advanced Topics
- Custom provider integration
- Plugin development
- Performance tuning
- Scaling considerations

## Distribution Strategy

### NPM Package Structure
```json
{
  "name": "circuit-breaker-sdk",
  "version": "1.0.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": ["dist/", "README.md"],
  "exports": {
    ".": "./dist/index.js",
    "./workflow": "./dist/workflow/index.js",
    "./functions": "./dist/functions/index.js",
    "./llm": "./dist/llm/index.js",
    "./agents": "./dist/agents/index.js"
  }
}
```

### Build Configuration
- TypeScript compilation with strict mode
- Bundle optimization for browser and Node.js
- Tree-shaking support for modular imports
- Source maps for debugging

## Success Metrics

### Developer Experience
- Time to first successful workflow: < 5 minutes
- API learning curve: Intuitive for TypeScript developers
- Documentation completeness: 100% API coverage
- Community adoption: GitHub stars, npm downloads

### Technical Performance
- GraphQL query optimization
- Function execution latency
- LLM response times
- Memory usage and cleanup

### Reliability
- Error handling coverage
- Graceful degradation
- Provider failover success rate
- Integration test coverage > 90%

This implementation plan provides a comprehensive roadmap for building a production-ready TypeScript SDK that makes Circuit Breaker accessible to developers while maintaining the power and flexibility of the underlying system.