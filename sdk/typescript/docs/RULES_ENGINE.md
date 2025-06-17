# RulesEngine Documentation

## Overview

The RulesEngine is a powerful component of the Circuit Breaker TypeScript SDK that provides comprehensive rule management functionality including rule evaluation, validation, CRUD operations, and support for various rule types (simple, composite, custom, javascript).

## Features

- **Multiple Rule Types**: Support for simple, JavaScript, composite, and custom rules
- **Complete CRUD Operations**: Create, read, update, and delete rules
- **Rule Evaluation**: Evaluate individual rules or rule sets with context
- **Composition**: Create complex composite rules with AND, OR, NOT operators
- **Custom Evaluators**: Define custom rule logic with TypeScript/JavaScript functions
- **Batch Operations**: Process multiple rules efficiently
- **Advanced Search**: Query rules with flexible filtering options
- **Validation**: Comprehensive rule validation and syntax checking
- **Caching**: Built-in caching for improved performance
- **Templates**: Reusable rule templates with parameters
- **Conditional Groups**: Group rules with enable/disable conditions
- **Rule Chains**: Sequential rule execution with context transformation
- **Analytics**: Rule statistics and health monitoring

## Installation

The RulesEngine is included in the Circuit Breaker SDK:

```typescript
import { CircuitBreakerSDK, RulesEngine, createRuleBuilder } from 'circuit-breaker-sdk';

const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql'
});

const rulesEngine = sdk.rules;
```

## Basic Usage

### Creating Rules

#### Simple Rules

```typescript
// Create a simple string-based rule
const stateRule = await rulesEngine.create({
  name: 'check-pending-state',
  type: 'simple',
  condition: "resource.state == 'pending'",
  description: 'Check if resource is in pending state',
  category: 'validation',
  priority: 10,
  enabled: true,
});
```

#### JavaScript Rules

```typescript
// Create a JavaScript rule with complex logic
const complexRule = await rulesEngine.create({
  name: 'complex-validation',
  type: 'javascript',
  condition: `
    const { resource } = context;
    const isValidCustomer = resource.data.customerId && resource.data.customerId.length > 0;
    const isValidAmount = resource.data.totalAmount > 0;
    const hasItems = resource.data.items && resource.data.items.length > 0;
    return isValidCustomer && isValidAmount && hasItems;
  `,
  description: 'Complex order validation',
  category: 'business-logic',
  enabled: true,
});
```

#### Custom Rules

```typescript
// Create a custom rule with evaluator function
const customRule = await rulesEngine.create({
  name: 'priority-customer',
  type: 'custom',
  description: 'Check if customer has priority status',
  category: 'customer',
  evaluator: async (context) => {
    const tier = context.resource.metadata.customerTier;
    const priority = context.resource.data.priority;
    return tier === 'gold' || tier === 'platinum' || priority === 'high';
  },
  enabled: true,
});
```

#### Composite Rules

```typescript
// Create composite rule combining multiple rules
const compositeRule = await rulesEngine.create({
  name: 'valid-premium-order',
  type: 'composite',
  operator: 'AND',
  rules: [
    { name: 'check-pending-state' },
    { name: 'priority-customer' },
    { name: 'complex-validation' }
  ],
  description: 'Valid premium order check',
  category: 'composite',
  enabled: true,
});
```

### Evaluating Rules

#### Single Rule Evaluation

```typescript
// Define evaluation context
const context = {
  resource: {
    id: 'resource-001',
    workflowId: 'order-processing',
    state: 'pending',
    data: {
      orderId: 'ORD-123',
      customerId: 'CUST-456',
      totalAmount: 299.99,
      priority: 'high'
    },
    metadata: {
      customerTier: 'gold',
      region: 'us-west'
    },
    history: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  activity: {
    id: 'validate_order',
    name: 'Validate Order',
    config: {}
  },
  timestamp: new Date(),
};

// Evaluate a single rule
const result = await rulesEngine.evaluateRule('check-pending-state', context);
console.log('Rule passed:', result.passed);
console.log('Evaluation time:', result.evaluationTime);
```

#### Multiple Rule Evaluation

```typescript
// Evaluate multiple rules
const ruleNames = ['check-pending-state', 'priority-customer', 'complex-validation'];
const multiResult = await rulesEngine.evaluateRules(ruleNames, context);

console.log('All rules passed:', multiResult.passed);
console.log('Rules evaluated:', multiResult.rulesEvaluated);
console.log('Rules passed:', multiResult.rulesPassed);
```

#### Batch Rule Evaluation

```typescript
// Evaluate rules for multiple contexts
const batchInputs = [
  {
    rules: ['check-pending-state', 'priority-customer'],
    context: context1,
    options: { timeout: 5000 }
  },
  {
    rules: ['complex-validation'],
    context: context2,
    options: { stopOnFailure: true }
  }
];

const batchResult = await rulesEngine.evaluateBatch(batchInputs);
console.log('Batch success:', batchResult.success);
console.log('Successful evaluations:', batchResult.successful);
```

## RuleBuilder - Fluent Interface

The RuleBuilder provides a fluent interface for creating and managing rules:

### Basic Builder Usage

```typescript
import { createRuleBuilder } from 'circuit-breaker-sdk';

const builder = createRuleBuilder(rulesEngine, {
  validate: true,
  defaultCategory: 'auto-generated',
  autoEnable: true,
  defaultMetadata: {
    generatedBy: 'rule-builder',
    version: '1.0'
  }
});

// Create rules using fluent interface
const rule = await builder
  .simple('fluent-validation')
  .condition("resource.data.orderId.startsWith('ORD-')")
  .description('Validate order ID format')
  .category('format-validation')
  .priority(100)
  .metadata({ pattern: 'ORD-*' })
  .create();
```

### Rule Types with Builder

#### Simple Rules

```typescript
const simpleRule = await builder
  .simple('amount-check')
  .condition("resource.data.totalAmount > 100")
  .description('Check minimum order amount')
  .category('validation')
  .create();
```

#### JavaScript Rules

```typescript
const jsRule = await builder
  .javascript('date-range-check')
  .condition(`
    const orderDate = new Date(context.resource.data.orderDate);
    const now = new Date();
    const daysDiff = (now - orderDate) / (1000 * 60 * 60 * 24);
    return daysDiff <= 30;
  `)
  .description('Check if order is within 30 days')
  .create();
```

#### Composite Rules

```typescript
const compositeRule = await builder
  .composite('comprehensive-validation')
  .operator('AND')
  .addRule(simpleRule)
  .addRule(jsRule)
  .addRuleByName('existing-rule-name')
  .description('Comprehensive order validation')
  .create();
```

#### Custom Rules

```typescript
const customRule = await builder
  .custom('business-logic-check')
  .evaluator(async (context) => {
    // Custom business logic
    const customer = await fetchCustomerData(context.resource.data.customerId);
    return customer.isActive && customer.creditScore > 600;
  })
  .description('Business logic validation')
  .create();
```

### Templates

```typescript
// Register a template
const stateTemplate = createRuleTemplate('state-check', 'simple', {
  condition: "resource.state == '{{state}}'",
  description: 'Check if resource is in specific state',
  parameters: ['state'],
  category: 'state-validation',
});

builder.registerTemplate(stateTemplate);

// Create rule from template
const templateRule = await builder.fromTemplate(
  'state-check',
  'check-processing-state',
  { state: 'processing' }
).create();
```

### Batch Operations

```typescript
// Create multiple rules with common properties
const batchRules = await builder.createBatch([
  {
    name: 'validation-rule-1',
    type: 'simple',
    condition: "resource.metadata.source == 'web-app'",
  },
  {
    name: 'validation-rule-2',
    type: 'javascript',
    condition: 'context.resource.data.items.length > 0',
  }
], {
  category: 'batch-validation',
  description: 'Batch created validation rules',
  enabled: true,
}).create();
```

## Advanced Features

### Conditional Groups

```typescript
// Create conditional rule groups
const conditionalGroup = builder
  .conditionalGroup('business-hours-validation')
  .addRules([rule1, rule2, rule3])
  .enableWhen(async (context) => {
    const hour = new Date().getHours();
    return hour >= 9 && hour <= 17; // Business hours only
  })
  .metadata({ type: 'time-conditional' })
  .register();

// Evaluate conditional group
const groupResult = await builder.evaluateConditionalGroup(
  'business-hours-validation',
  context
);
```

### Rule Chains

```typescript
// Create rule chains with sequential execution
const chain = builder
  .chain('validation-pipeline')
  .addRules([validationRule1, validationRule2, validationRule3])
  .stopOnFailure(true)
  .transformContext((context, ruleIndex) => {
    return {
      ...context,
      metadata: {
        ...context.metadata,
        validationStep: ruleIndex + 1,
      },
    };
  })
  .register();

// Execute rule chain
const chainResult = await builder.executeChain('validation-pipeline', context);
```

### Testing Rules

```typescript
// Test rules without saving them
const testResult = await builder
  .simple('test-rule')
  .condition("resource.data.totalAmount > 100")
  .build()
  .test(context);

console.log('Test passed:', testResult.passed);
console.log('Test errors:', testResult.errors);
```

### Cloning Rules

```typescript
// Clone existing rule with modifications
const clonedRule = await builder.clone(
  'existing-rule-name',
  'new-rule-name',
  {
    description: 'Modified version of existing rule',
    priority: 50,
    metadata: { clonedAt: new Date().toISOString() }
  }
);
```

## Rule Management

### Searching and Listing

```typescript
// Search for rules with filters
const searchResults = await rulesEngine.search({
  query: 'validation',
  type: 'javascript',
  category: 'business-logic',
  enabled: true,
  minPriority: 10,
  maxPriority: 100,
  includeStats: true,
  limit: 20,
  sortBy: 'priority',
  sortDirection: 'desc'
});

console.log('Found rules:', searchResults.items.length);
```

### Updating Rules

```typescript
// Update existing rule
const updatedRule = await rulesEngine.update('rule-name', {
  description: 'Updated description',
  priority: 20,
  enabled: false,
  metadata: { lastModified: new Date().toISOString() }
});
```

### Deleting Rules

```typescript
// Delete rule (checks for dependencies)
await rulesEngine.delete('rule-name');

// Force delete (ignores dependencies)
await rulesEngine.delete('rule-name', { force: true });
```

## Analytics and Monitoring

### Rule Statistics

```typescript
// Get comprehensive rule statistics
const stats = await rulesEngine.getStats();
console.log('Total rules:', stats.totalRules);
console.log('Enabled rules:', stats.enabled);
console.log('Rules by type:', stats.byType);
console.log('Rules by category:', stats.byCategory);
console.log('Average evaluation time:', stats.averageEvaluationTime);
console.log('Most evaluated rules:', stats.mostEvaluated);
```

### Health Monitoring

```typescript
// Check rule engine health
const health = await rulesEngine.getHealth();
console.log('Engine healthy:', health.healthy);
console.log('Health issues:', health.issues);
console.log('Failing rules:', health.failingRules);
console.log('Error rate:', health.errorRate);
console.log('Cache efficiency:', health.cachingEfficiency);
```

### Rule Validation

```typescript
// Validate rule definition
const validationResult = await rulesEngine.validateRule({
  name: 'test-rule',
  type: 'javascript',
  condition: 'context.resource.data.amount > 0',
  description: 'Test rule validation'
}, { deep: true });

console.log('Rule valid:', validationResult.valid);
console.log('Validation errors:', validationResult.errors);
console.log('Validation warnings:', validationResult.warnings);
```

## API Reference

### RulesEngine

#### Constructor
```typescript
new RulesEngine(graphqlClient: GraphQLClient, logger: Logger, config?: RulesConfig)
```

#### Methods

##### create(input, options?)
Creates a new rule.

**Parameters:**
- `input: RuleCreateInput` - Rule creation data
- `options?: { validate?: boolean }` - Creation options

**Returns:** `Promise<Rule>`

##### get(ruleName, options?)
Retrieves a rule by name.

**Parameters:**
- `ruleName: string` - Rule name
- `options?: { useCache?: boolean }` - Retrieval options

**Returns:** `Promise<Rule>`

##### update(ruleName, input, options?)
Updates an existing rule.

**Parameters:**
- `ruleName: string` - Rule name
- `input: RuleUpdateInput` - Update data
- `options?: { validate?: boolean }` - Update options

**Returns:** `Promise<Rule>`

##### delete(ruleName, options?)
Deletes a rule.

**Parameters:**
- `ruleName: string` - Rule name
- `options?: { force?: boolean }` - Deletion options

**Returns:** `Promise<boolean>`

##### evaluateRule(ruleName, context, options?)
Evaluates a single rule.

**Parameters:**
- `ruleName: string` - Rule name
- `context: RuleContext` - Evaluation context
- `options?: RuleEvaluationOptions` - Evaluation options

**Returns:** `Promise<RuleEvaluationResult>`

##### evaluateRules(ruleNames, context, options?)
Evaluates multiple rules.

**Parameters:**
- `ruleNames: string[]` - Array of rule names
- `context: RuleContext` - Evaluation context
- `options?: RuleEvaluationOptions` - Evaluation options

**Returns:** `Promise<RuleEvaluationResult>`

##### search(options?)
Searches for rules with filtering.

**Parameters:**
- `options?: RuleSearchOptions` - Search and filter options

**Returns:** `Promise<PaginatedResult<RuleWithStats>>`

##### validateRule(rule, options?)
Validates a rule definition.

**Parameters:**
- `rule: Rule | RuleCreateInput` - Rule to validate
- `options?: { deep?: boolean }` - Validation options

**Returns:** `Promise<RuleValidationResult>`

### Types

#### RuleCreateInput
```typescript
interface RuleCreateInput {
  name: string;
  type: RuleType;
  condition?: string;
  evaluator?: RuleEvaluator;
  description?: string;
  category?: string;
  metadata?: Record<string, any>;
  priority?: number;
  enabled?: boolean;
  // For composite rules
  operator?: 'AND' | 'OR' | 'NOT';
  rules?: Rule[];
}
```

#### RuleContext
```typescript
interface RuleContext {
  resource: Resource;
  workflow?: WorkflowDefinition;
  activity: ActivityDefinition;
  metadata?: Record<string, any>;
  timestamp: Date;
}
```

#### RuleEvaluationResult
```typescript
interface RuleEvaluationResult {
  passed: boolean;
  results: RuleResult[];
  errors: string[];
  evaluationTime: number;
  rulesEvaluated: number;
  rulesPassed: number;
}
```

#### RuleEvaluationOptions
```typescript
interface RuleEvaluationOptions {
  timeout?: number;
  stopOnFailure?: boolean;
  includeDetails?: boolean;
  customEvaluators?: Record<string, RuleEvaluator>;
  useCache?: boolean;
}
```

## Rule Types

### Simple Rules
- String-based conditions with basic pattern matching
- Suitable for simple comparisons and checks
- Example: `"resource.state == 'pending'"`

### JavaScript Rules
- Full JavaScript expressions with access to context
- Support for complex logic and calculations
- Example: `"context.resource.data.amount > 100 && context.resource.metadata.tier === 'gold'"`

### Composite Rules
- Combine multiple rules with logical operators (AND, OR, NOT)
- Support for nested rule hierarchies
- Efficient short-circuit evaluation

### Custom Rules
- User-defined evaluator functions
- Full access to TypeScript/JavaScript capabilities
- Support for async operations and external API calls

## Error Handling

The RulesEngine provides comprehensive error handling:

```typescript
import { 
  RuleNotFoundError,
  RuleValidationError,
  RuleEvaluationError,
  RuleTimeoutError 
} from 'circuit-breaker-sdk';

try {
  const result = await rulesEngine.evaluateRule('invalid-rule', context);
} catch (error) {
  if (error instanceof RuleNotFoundError) {
    console.error('Rule not found:', error.message);
  } else if (error instanceof RuleEvaluationError) {
    console.error('Rule evaluation failed:', error.message);
    console.error('Rule name:', error.ruleName);
    console.error('Evaluation errors:', error.evaluationErrors);
  } else if (error instanceof RuleTimeoutError) {
    console.error('Rule evaluation timeout:', error.message);
  } else {
    console.error('Unexpected error:', error);
  }
}
```

## Best Practices

### Rule Design

1. **Keep rules focused** - Each rule should have a single responsibility
2. **Use descriptive names** - Rule names should clearly indicate their purpose
3. **Add comprehensive descriptions** - Help other developers understand rule logic
4. **Organize with categories** - Group related rules for better management
5. **Set appropriate priorities** - Use priorities to control evaluation order

### Performance Optimization

1. **Use caching** for frequently evaluated rules
2. **Implement timeouts** for custom evaluators
3. **Optimize JavaScript conditions** for better execution speed
4. **Use composite rules** to leverage short-circuit evaluation
5. **Monitor rule performance** with built-in analytics

### Error Handling

1. **Implement comprehensive validation** before rule creation
2. **Use try-catch blocks** for rule evaluation
3. **Handle timeouts gracefully** in custom evaluators
4. **Log rule failures** with appropriate context
5. **Monitor error rates** and set up alerting

### Security Considerations

1. **Validate JavaScript conditions** before execution
2. **Sanitize user inputs** in rule conditions
3. **Implement access controls** for rule management
4. **Audit rule changes** for compliance requirements
5. **Use safe evaluation contexts** for custom rules

## Examples

### E-commerce Order Validation

```typescript
// Complete order validation rule set
async function createOrderValidationRules(rulesEngine: RulesEngine) {
  const builder = createRuleBuilder(rulesEngine);

  // Basic validation rules
  const basicRules = await builder.createBatch([
    {
      name: 'has-customer-id',
      type: 'simple',
      condition: "resource.data.customerId != null",
    },
    {
      name: 'positive-amount',
      type: 'javascript',
      condition: 'context.resource.data.totalAmount > 0',
    },
    {
      name: 'has-items',
      type: 'simple',
      condition: "resource.data.items.length > 0",
    }
  ], {
    category: 'basic-validation',
    description: 'Basic order validation rules',
  }).create();

  // Business logic rules
  const businessRule = await builder
    .custom('credit-check')
    .evaluator(async (context) => {
      const customerId = context.resource.data.customerId;
      const amount = context.resource.data.totalAmount;
      
      // Simulate credit check
      const creditScore = await getCreditScore(customerId);
      const maxAmount = getMaxOrderAmount(creditScore);
      
      return amount <= maxAmount;
    })
    .description('Customer credit validation')
    .category('business-logic')
    .create();

  // Composite validation rule
  const orderValidation = await builder
    .composite('complete-order-validation')
    .operator('AND')
    .addRules(basicRules)
    .addRule(businessRule)
    .description('Complete order validation')
    .create();

  return orderValidation;
}
```

### Dynamic Rule Configuration

```typescript
// Configure rules based on business hours and region
async function createDynamicRules(rulesEngine: RulesEngine) {
  const builder = createRuleBuilder(rulesEngine);

  // Business hours group
  const businessHoursGroup = builder
    .conditionalGroup('business-hours-rules')
    .addRules([
      await builder.simple('priority-processing')
        .condition("resource.data.priority == 'high'")
        .create(),
      await builder.simple('express-shipping')
        .condition("resource.data.shipping == 'express'")
        .create()
    ])
    .enableWhen(async (context) => {
      const hour = new Date().getHours();
      return hour >= 9 && hour <= 17;
    })
    .register();

  // Region-specific chain
  const regionChain = builder
    .chain('region-validation')
    .addRules([
      await builder.javascript('region-check')
        .condition('context.resource.metadata.region != null')
        .create(),
      await builder.javascript('shipping-validation')
        .condition(`
          const region = context.resource.metadata.region;
          const shipping = context.resource.data.shipping;
          return isShippingAvailable(region, shipping);
        `)
        .create()
    ])
    .stopOnFailure(true)
    .register();

  return { businessHoursGroup, regionChain };
}
```

## Integration with Other SDK Components

### Workflow Integration
```typescript
// Use rules to control workflow transitions
const transitionRule = await rulesEngine.create({
  name: 'can-transition-to-processing',
  type: 'composite',
  operator: 'AND',
  rules: [
    { name: 'has-customer-id' },
    { name: 'positive-amount' },
    { name: 'credit-check' }
  ],
  description: 'Validate transition to processing state'
});

// Evaluate before state transition
const canTransition = await rulesEngine.evaluateRule(
  'can-transition-to-processing',
  context
);

if (canTransition.passed) {
  await resourceManager.transitionState({
    resourceId: resource.id,
    toState: 'processing'
  });
}
```

### Resource Integration
```typescript
// Validate resources using rules
const resource = await resourceManager.get('resource-id');
const validationContext: RuleContext = {
  resource,
  activity: { id: 'validation', name: 'Resource Validation', config: {} },
  timestamp: new Date()
};

const validationResult = await rulesEngine.evaluateRules([
  'basic-validation',
  'business-rules',
  'compliance-check'
], validationContext);

if (!validationResult.passed) {
  throw new Error(`Resource validation failed: ${validationResult.errors.join(', ')}`);
}
```

## Troubleshooting

### Common Issues

1. **Rule Not Found Errors**
   - Check rule name spelling
   - Verify rule was created successfully
   - Ensure rule is enabled

2. **Evaluation Failures**
   - Validate JavaScript syntax in conditions
   - Check context structure matches expectations
   - Verify custom evaluators handle errors properly

3. **Performance Issues**
   - Enable caching for frequently used rules
   - Optimize JavaScript conditions
   - Use composite rules for complex logic
   - Monitor evaluation times

4. **Timeout Errors**
   - Increase timeout values for slow evaluators
   - Optimize custom evaluator performance
   - Use async patterns in custom evaluators

### Debug Mode

Enable debug logging for detailed operation traces:

```typescript
const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql',
  debug: true,
  logging: {
    level: 'debug',
    enableConsole: true
  }
});
```

### Health Monitoring

Set up regular health checks:

```typescript
// Monitor rule engine health periodically
setInterval(async () => {
  const health = await rulesEngine.getHealth();
  if (!health.healthy) {
    console.error('Rule engine health issues detected:', health.issues);
    // Trigger alerts or corrective actions
  }
}, 60000); // Check every minute
```

## Contributing

The RulesEngine is part of the Circuit Breaker SDK. To contribute:

1. Follow the existing code patterns and TypeScript conventions
2. Add comprehensive tests for new rule types and features
3. Update documentation for API changes
4. Ensure backward compatibility when possible

For more information, see the main SDK documentation and contribution guidelines.