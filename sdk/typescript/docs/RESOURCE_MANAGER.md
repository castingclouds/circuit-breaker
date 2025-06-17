# ResourceManager Documentation

## Overview

The ResourceManager is a core component of the Circuit Breaker TypeScript SDK that provides comprehensive resource lifecycle management, state transitions, and advanced operations for workflow-based applications.

## Features

- **Complete CRUD Operations**: Create, read, update, and delete resources
- **State Management**: Execute activities and manage state transitions
- **Batch Operations**: Process multiple resources efficiently
- **Advanced Search**: Query resources with flexible filtering options
- **Validation**: Comprehensive resource validation and health monitoring
- **Caching**: Built-in caching for improved performance
- **Error Handling**: Robust error handling with detailed context
- **Analytics**: Resource statistics and health monitoring

## Installation

The ResourceManager is included in the Circuit Breaker SDK:

```typescript
import { CircuitBreakerSDK, ResourceManager } from 'circuit-breaker-sdk';

const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql'
});

const resourceManager = sdk.resources;
```

## Basic Usage

### Creating Resources

```typescript
// Create a simple resource
const resource = await resourceManager.create({
  workflowId: 'order-processing',
  data: {
    orderId: 'ORD-001',
    customerId: 'CUST-123',
    totalAmount: 99.99
  },
  metadata: {
    source: 'web-app',
    priority: 'normal'
  }
});

console.log('Created resource:', resource.id);
```

### Retrieving Resources

```typescript
// Get a resource by ID
const resource = await resourceManager.get('resource-id', {
  includeWorkflow: true,
  useCache: true
});

// Search for resources
const results = await resourceManager.search({
  workflowId: 'order-processing',
  state: 'pending',
  limit: 10
});

console.log('Found', results.totalCount, 'resources');
```

### Updating Resources

```typescript
// Update resource data and metadata
const updatedResource = await resourceManager.update('resource-id', {
  data: { status: 'updated' },
  metadata: { lastModified: new Date().toISOString() }
});
```

### State Management

```typescript
// Execute an activity
const activityResult = await resourceManager.executeActivity({
  resourceId: 'resource-id',
  activityId: 'validate_order',
  data: { validateInventory: true }
});

// Direct state transition
const transitionResult = await resourceManager.transitionState({
  resourceId: 'resource-id',
  toState: 'processing',
  data: { processedAt: new Date().toISOString() }
});
```

## Advanced Features

### Batch Operations

```typescript
// Create multiple resources at once
const batchInputs = [
  {
    workflowId: 'order-processing',
    data: { orderId: 'ORD-002', amount: 50 }
  },
  {
    workflowId: 'order-processing', 
    data: { orderId: 'ORD-003', amount: 75 }
  }
];

const batchResult = await resourceManager.createBatch(batchInputs, {
  concurrency: 3,
  continueOnError: true
});

console.log(`Created ${batchResult.successful} resources`);

// Execute activities on multiple resources
const activityInputs = batchResult.results
  .filter(r => r.success)
  .map(r => ({
    resourceId: r.data.id,
    activityId: 'validate_order'
  }));

const batchActivityResult = await resourceManager.executeActivityBatch(
  activityInputs,
  { concurrency: 2 }
);
```

### Advanced Search and Filtering

```typescript
// Complex search with multiple filters
const searchResults = await resourceManager.search({
  query: 'urgent order',
  workflowId: 'order-processing',
  states: ['pending', 'processing'],
  metadata: { priority: 'high' },
  createdAfter: new Date(Date.now() - 24 * 60 * 60 * 1000), // Last 24 hours
  sortBy: 'createdAt',
  sortDirection: 'desc',
  limit: 50,
  includeHistory: true,
  includeWorkflow: true
});

// Process results
searchResults.data.forEach(resource => {
  console.log(`Resource ${resource.id} in state ${resource.state}`);
  if (resource.history) {
    console.log(`Last transition: ${resource.history[0]?.timestamp}`);
  }
});
```

### Resource Validation

```typescript
// Validate a resource
const validationReport = await resourceManager.validate('resource-id', {
  validateData: true,
  validateState: true,
  validateMetadata: true,
  includeWarnings: true
});

if (!validationReport.valid) {
  console.error('Validation errors:', validationReport.errors);
}

if (validationReport.warnings.length > 0) {
  console.warn('Validation warnings:', validationReport.warnings);
}
```

### Analytics and Monitoring

```typescript
// Get resource statistics
const stats = await resourceManager.getStats('order-processing');
console.log('Total resources:', stats.totalResources);
console.log('Active resources:', stats.activeResources);
console.log('Resources by state:', stats.byState);

// Monitor resource health
const health = await resourceManager.getHealth('order-processing');
if (!health.healthy) {
  console.error('Resource health issues:', health.issues);
  console.log('Stuck resources:', health.stuckResources);
  console.log('Error rate:', health.errorRate);
}
```

## ResourceBuilder

The ResourceBuilder provides a fluent interface for creating and managing resources with advanced features.

### Basic Builder Usage

```typescript
import { createResourceBuilder, createResourceTemplate } from 'circuit-breaker-sdk';

// Create a builder with default options
const builder = createResourceBuilder(resourceManager, {
  validate: true,
  defaultMetadata: {
    environment: 'production',
    version: '1.0'
  },
  defaultWorkflowId: 'order-processing'
});

// Create a resource using builder
const result = await builder.create({
  workflowId: 'order-processing',
  data: { orderId: 'ORD-001', amount: 99.99 }
});

// Use fluent interface for operations
const processedOrder = await result
  .executeActivity('validate_order')
  .then(r => r.transitionTo('processing'))
  .then(r => r.update({ status: 'processed' }));
```

### Templates

```typescript
// Create a template for common resource patterns
const orderTemplate = createResourceTemplate(
  'standard-order',
  'order-processing',
  {
    // Default data structure
    priority: 'normal',
    processingTime: '4-hours',
    notifications: true
  },
  {
    description: 'Standard order template',
    defaultMetadata: {
      type: 'order',
      version: '1.0'
    },
    requiredFields: ['orderId', 'customerId', 'totalAmount']
  }
);

// Register template with builder
builder.registerTemplate(orderTemplate);

// Create resources from template
const orderFromTemplate = await builder.fromTemplate('standard-order', {
  orderId: 'ORD-002',
  customerId: 'CUST-456',
  totalAmount: 149.99,
  items: [{ product: 'Widget', quantity: 2 }]
});
```

### Batch Operations with Builder

```typescript
// Create multiple resources from template
const batchResult = await builder.createBatchFromTemplate(
  'standard-order',
  [
    {
      batchId: 'batch-1',
      data: {
        orderId: 'ORD-003',
        customerId: 'CUST-789',
        totalAmount: 199.99
      }
    },
    {
      batchId: 'batch-2', 
      data: {
        orderId: 'ORD-004',
        customerId: 'CUST-101',
        totalAmount: 299.99
      }
    }
  ]
);

// Execute activities on all successful resources
const activityResults = await batchResult.executeActivityOnAll(
  'validate_order',
  { batchValidation: true }
);

// Transition all to processing state
const transitionResults = await batchResult.transitionAllTo(
  'processing',
  { batchProcessed: true }
);
```

### Conditional Operations

```typescript
// Create conditional transition
const conditionalTransition = builder.createConditionalTransition({
  condition: async (resource) => {
    return resource.metadata.priority === 'high' && 
           resource.data.totalAmount > 200;
  },
  targetState: 'expedited',
  data: { expedited: true },
  metadata: { reason: 'high-value-order' }
});

// Execute on a resource
const transitionResult = await conditionalTransition.execute(resource);
if (transitionResult) {
  console.log('Condition met, resource expedited');
}

// Execute on multiple resources
const resources = await resourceManager.search({ 
  state: 'validated',
  limit: 100 
});
const expeditedResources = await conditionalTransition.executeOnResources(
  resources.data
);
```

## API Reference

### ResourceManager

#### Constructor
```typescript
new ResourceManager(graphqlClient: GraphQLClient, logger: Logger)
```

#### Methods

##### create(input, options?)
Creates a new resource.

**Parameters:**
- `input: ResourceCreateInput` - Resource creation data
- `options?: { validate?: boolean }` - Creation options

**Returns:** `Promise<Resource>`

##### get(resourceId, options?)
Retrieves a resource by ID.

**Parameters:**
- `resourceId: string` - Resource identifier
- `options?: { includeWorkflow?: boolean, useCache?: boolean }` - Retrieval options

**Returns:** `Promise<Resource>`

##### update(resourceId, input, options?)
Updates an existing resource.

**Parameters:**
- `resourceId: string` - Resource identifier
- `input: ResourceUpdateInput` - Update data
- `options?: { validate?: boolean }` - Update options

**Returns:** `Promise<Resource>`

##### delete(resourceId, options?)
Deletes a resource.

**Parameters:**
- `resourceId: string` - Resource identifier
- `options?: { force?: boolean }` - Deletion options

**Returns:** `Promise<boolean>`

##### search(options?)
Searches for resources with filtering.

**Parameters:**
- `options?: ResourceSearchOptions` - Search and filter options

**Returns:** `Promise<PaginatedResult<ResourceWithWorkflow>>`

##### executeActivity(input)
Executes an activity on a resource.

**Parameters:**
- `input: ActivityExecuteInput` - Activity execution data

**Returns:** `Promise<ActivityExecutionResult>`

##### transitionState(input)
Transitions a resource to a new state.

**Parameters:**
- `input: StateTransitionInput` - State transition data

**Returns:** `Promise<StateTransitionResult>`

##### createBatch(inputs, options?)
Creates multiple resources in batch.

**Parameters:**
- `inputs: ResourceCreateInput[]` - Array of resource creation data
- `options?: BatchOperationOptions` - Batch operation options

**Returns:** `Promise<BatchOperationResult<Resource>>`

##### executeActivityBatch(inputs, options?)
Executes activities on multiple resources in batch.

**Parameters:**
- `inputs: ActivityExecuteInput[]` - Array of activity execution data
- `options?: BatchOperationOptions` - Batch operation options

**Returns:** `Promise<BatchOperationResult<ActivityExecutionResult>>`

##### getStats(workflowId?)
Gets resource statistics.

**Parameters:**
- `workflowId?: string` - Optional workflow filter

**Returns:** `Promise<ResourceStats>`

##### validate(resourceId, options?)
Validates a resource.

**Parameters:**
- `resourceId: string` - Resource identifier
- `options?: ResourceValidationOptions` - Validation options

**Returns:** `Promise<ResourceValidationReport>`

##### getHealth(workflowId?)
Gets resource health status.

**Parameters:**
- `workflowId?: string` - Optional workflow filter

**Returns:** `Promise<ResourceHealthStatus>`

### Types

#### ResourceCreateInput
```typescript
interface ResourceCreateInput {
  workflowId: string;
  initialState?: string;
  data: any;
  metadata?: Record<string, any>;
}
```

#### ResourceUpdateInput
```typescript
interface ResourceUpdateInput {
  data?: any;
  metadata?: Record<string, any>;
}
```

#### ResourceSearchOptions
```typescript
interface ResourceSearchOptions extends PaginationOptions {
  query?: string;
  workflowId?: string;
  state?: string;
  states?: string[];
  createdAfter?: Date;
  createdBefore?: Date;
  updatedAfter?: Date;
  updatedBefore?: Date;
  includeHistory?: boolean;
  includeWorkflow?: boolean;
  metadata?: Record<string, any>;
  dataFields?: Record<string, any>;
  sortBy?: string;
  sortDirection?: "asc" | "desc";
}
```

#### StateTransitionInput
```typescript
interface StateTransitionInput {
  resourceId: string;
  toState: string;
  activityId?: string;
  data?: any;
  metadata?: Record<string, any>;
  validate?: boolean;
}
```

#### ActivityExecuteInput
```typescript
interface ActivityExecuteInput {
  resourceId: string;
  activityId: string;
  data?: any;
  metadata?: Record<string, any>;
}
```

#### BatchOperationOptions
```typescript
interface BatchOperationOptions {
  concurrency?: number;
  continueOnError?: boolean;
  operationTimeout?: number;
}
```

## Error Handling

The ResourceManager provides comprehensive error handling with specific error types:

```typescript
import { 
  ResourceNotFoundError,
  ResourceValidationError,
  StateTransitionError,
  ActivityExecutionError 
} from 'circuit-breaker-sdk';

try {
  const resource = await resourceManager.get('invalid-id');
} catch (error) {
  if (error instanceof ResourceNotFoundError) {
    console.error('Resource not found:', error.message);
  } else if (error instanceof ResourceValidationError) {
    console.error('Validation error:', error.message);
  } else {
    console.error('Unexpected error:', error);
  }
}
```

## Best Practices

### Resource Lifecycle Management

1. **Always validate resources** when creating them in production environments
2. **Use appropriate metadata** to track resource context and ownership
3. **Handle state transitions carefully** and validate transition rules
4. **Monitor resource health** regularly in production systems

### Performance Optimization

1. **Use caching** for frequently accessed resources
2. **Implement batch operations** for bulk resource operations
3. **Limit search results** with appropriate pagination
4. **Use selective field inclusion** to reduce payload size

### Error Handling

1. **Implement retry logic** for transient failures
2. **Log resource operations** with appropriate context
3. **Handle partial failures** in batch operations gracefully
4. **Monitor error rates** and set up alerting

### Security Considerations

1. **Validate resource data** before persistence
2. **Implement access controls** for resource operations
3. **Sanitize search inputs** to prevent injection attacks
4. **Audit resource changes** for compliance requirements

## Examples

### E-commerce Order Processing

```typescript
// Complete order processing workflow
async function processOrder(orderData: any) {
  try {
    // Create order resource
    const order = await resourceManager.create({
      workflowId: 'order-processing',
      data: orderData,
      metadata: {
        source: 'web',
        customerTier: orderData.customerTier,
        priority: orderData.expedited ? 'high' : 'normal'
      }
    });

    // Validate order
    const validationResult = await resourceManager.executeActivity({
      resourceId: order.id,
      activityId: 'validate_order',
      data: { 
        checkInventory: true,
        validatePayment: true 
      }
    });

    if (!validationResult.success) {
      throw new Error('Order validation failed');
    }

    // Process payment
    const paymentResult = await resourceManager.executeActivity({
      resourceId: order.id,
      activityId: 'process_payment',
      data: { paymentMethod: orderData.paymentMethod }
    });

    if (paymentResult.success) {
      // Transition to fulfillment
      await resourceManager.transitionState({
        resourceId: order.id,
        toState: 'fulfillment',
        data: { 
          paymentId: paymentResult.output.paymentId,
          processedAt: new Date().toISOString()
        }
      });
    }

    return order;
  } catch (error) {
    console.error('Order processing failed:', error);
    throw error;
  }
}
```

### Bulk Data Processing

```typescript
// Process large datasets efficiently
async function processBulkData(dataItems: any[]) {
  const builder = createResourceBuilder(resourceManager, {
    defaultWorkflowId: 'data-processing',
    defaultMetadata: { 
      batchId: generateBatchId(),
      processedAt: new Date().toISOString()
    }
  });

  // Create resources in batches
  const batchSize = 50;
  const results = [];

  for (let i = 0; i < dataItems.length; i += batchSize) {
    const batch = dataItems.slice(i, i + batchSize);
    const batchInputs = batch.map((item, index) => ({
      batchId: `item-${i + index}`,
      input: {
        workflowId: 'data-processing',
        data: item,
        metadata: { itemIndex: i + index }
      }
    }));

    const batchResult = await builder.createBatch(batchInputs);
    
    // Process successful items
    if (batchResult.getSuccessful().length > 0) {
      await batchResult.executeActivityOnAll('process_data');
    }

    // Handle failures
    const failed = batchResult.getFailed();
    if (failed.length > 0) {
      console.warn(`${failed.length} items failed in batch ${i / batchSize + 1}`);
      // Implement retry logic or dead letter queue
    }

    results.push(batchResult);
  }

  return results;
}
```

## Integration with Other SDK Components

The ResourceManager integrates seamlessly with other SDK components:

### Workflow Integration
```typescript
// Resource follows workflow state machine
const workflow = await sdk.workflows.get('order-processing');
const validStates = workflow.states;

// Ensure resource state is valid for workflow
const resource = await resourceManager.get('resource-id');
if (!validStates.includes(resource.state)) {
  console.error('Invalid resource state for workflow');
}
```

### Rules Engine Integration
```typescript
// Use rules to determine resource transitions
const ruleResult = await sdk.rules.evaluate('order-priority-rule', {
  resource: resource,
  context: { currentTime: new Date() }
});

if (ruleResult.matches) {
  await resourceManager.transitionState({
    resourceId: resource.id,
    toState: ruleResult.targetState,
    data: ruleResult.data
  });
}
```

### Function Integration
```typescript
// Execute functions as part of resource activities
const functionResult = await sdk.functions.execute('order-validator', {
  resourceData: resource.data,
  validationRules: ['inventory', 'payment', 'shipping']
});

if (functionResult.success) {
  await resourceManager.update(resource.id, {
    data: { ...resource.data, validation: functionResult.output }
  });
}
```

## Troubleshooting

### Common Issues

1. **Resource Not Found Errors**
   - Check resource ID spelling
   - Verify resource hasn't been deleted
   - Ensure proper permissions

2. **State Transition Failures**
   - Validate target state exists in workflow
   - Check transition rules and conditions
   - Verify activity prerequisites

3. **Performance Issues**
   - Enable caching for frequently accessed resources
   - Use batch operations for bulk processing
   - Implement proper pagination for large result sets

4. **Validation Errors**
   - Check required fields in resource data
   - Validate data types and formats
   - Ensure metadata structure compliance

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
// Monitor resource health periodically
setInterval(async () => {
  const health = await resourceManager.getHealth();
  if (!health.healthy) {
    console.error('Resource health issues detected:', health.issues);
    // Trigger alerts or corrective actions
  }
}, 60000); // Check every minute
```

## Contributing

The ResourceManager is part of the Circuit Breaker SDK. To contribute:

1. Follow the existing code patterns and TypeScript conventions
2. Add comprehensive tests for new features
3. Update documentation for API changes
4. Ensure backward compatibility when possible

For more information, see the main SDK documentation and contribution guidelines.