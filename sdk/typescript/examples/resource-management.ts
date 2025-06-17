/**
 * Resource Management Example
 *
 * This example demonstrates comprehensive resource management using the
 * Circuit Breaker TypeScript SDK, including CRUD operations, state transitions,
 * batch operations, and advanced features.
 */

import {
  CircuitBreakerSDK,
  createResourceBuilder,
  createResourceTemplate,
  ResourceManager,
  ResourceBuilder,
  WorkflowDefinition,
} from '../src/index.js';

// Initialize SDK
const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql',
  debug: true,
});

// Sample workflow definition for this example
const sampleWorkflow: WorkflowDefinition = {
  id: 'order-processing',
  name: 'Order Processing Workflow',
  description: 'Complete order processing from creation to fulfillment',
  states: ['pending', 'validated', 'processing', 'shipped', 'delivered', 'cancelled'],
  initialState: 'pending',
  activities: [
    {
      id: 'validate_order',
      name: 'Validate Order',
      type: 'function',
      config: { functionId: 'order-validator' },
    },
    {
      id: 'process_payment',
      name: 'Process Payment',
      type: 'function',
      config: { functionId: 'payment-processor' },
    },
    {
      id: 'prepare_shipment',
      name: 'Prepare Shipment',
      type: 'function',
      config: { functionId: 'shipment-preparer' },
    },
    {
      id: 'ship_order',
      name: 'Ship Order',
      type: 'function',
      config: { functionId: 'shipping-service' },
    },
    {
      id: 'mark_delivered',
      name: 'Mark as Delivered',
      type: 'function',
      config: { functionId: 'delivery-tracker' },
    },
  ],
};

async function basicResourceOperations() {
  console.log('=== Basic Resource Operations ===');

  // Create a resource manager
  const resourceManager = sdk.resources;

  try {
    // 1. Create a new resource
    console.log('Creating a new order resource...');
    const orderResource = await resourceManager.create({
      workflowId: 'order-processing',
      data: {
        orderId: 'ORD-001',
        customerId: 'CUST-123',
        items: [
          { productId: 'PROD-A', quantity: 2, price: 29.99 },
          { productId: 'PROD-B', quantity: 1, price: 49.99 },
        ],
        totalAmount: 109.97,
        shippingAddress: {
          street: '123 Main St',
          city: 'Anytown',
          state: 'CA',
          zip: '12345',
        },
      },
      metadata: {
        source: 'web-app',
        priority: 'normal',
        customerTier: 'gold',
      },
    });

    console.log('✅ Resource created:', {
      id: orderResource.id,
      state: orderResource.state,
      orderId: orderResource.data.orderId,
    });

    // 2. Get the resource
    console.log('\nRetrieving the resource...');
    const retrievedResource = await resourceManager.get(orderResource.id, {
      includeWorkflow: true,
    });
    console.log('✅ Resource retrieved:', {
      id: retrievedResource.id,
      state: retrievedResource.state,
      lastUpdate: retrievedResource.updatedAt,
    });

    // 3. Update the resource
    console.log('\nUpdating resource metadata...');
    const updatedResource = await resourceManager.update(orderResource.id, {
      metadata: {
        ...retrievedResource.metadata,
        priority: 'high',
        notes: 'Customer requested expedited shipping',
      },
    });
    console.log('✅ Resource updated:', {
      priority: updatedResource.metadata.priority,
      notes: updatedResource.metadata.notes,
    });

    // 4. Execute an activity
    console.log('\nExecuting validation activity...');
    const activityResult = await resourceManager.executeActivity({
      resourceId: orderResource.id,
      activityId: 'validate_order',
      data: { validateInventory: true, validatePayment: true },
    });
    console.log('✅ Activity executed:', {
      success: activityResult.success,
      newState: activityResult.resource.state,
      duration: activityResult.duration,
    });

    // 5. Transition state directly
    console.log('\nTransitioning to processing state...');
    const transitionResult = await resourceManager.transitionState({
      resourceId: orderResource.id,
      toState: 'processing',
      data: { processedBy: 'system', timestamp: new Date().toISOString() },
    });
    console.log('✅ State transitioned:', {
      success: transitionResult.success,
      newState: transitionResult.resource.state,
    });

    return orderResource.id;
  } catch (error) {
    console.error('❌ Error in basic operations:', error);
    throw error;
  }
}

async function batchOperations() {
  console.log('\n=== Batch Operations ===');

  const resourceManager = sdk.resources;

  try {
    // Create multiple orders in batch
    console.log('Creating multiple orders in batch...');
    const batchInputs = [
      {
        workflowId: 'order-processing',
        data: {
          orderId: 'ORD-002',
          customerId: 'CUST-456',
          items: [{ productId: 'PROD-C', quantity: 1, price: 19.99 }],
          totalAmount: 19.99,
        },
        metadata: { source: 'mobile-app', priority: 'normal' },
      },
      {
        workflowId: 'order-processing',
        data: {
          orderId: 'ORD-003',
          customerId: 'CUST-789',
          items: [{ productId: 'PROD-D', quantity: 3, price: 39.99 }],
          totalAmount: 119.97,
        },
        metadata: { source: 'api', priority: 'high' },
      },
      {
        workflowId: 'order-processing',
        data: {
          orderId: 'ORD-004',
          customerId: 'CUST-101',
          items: [{ productId: 'PROD-E', quantity: 2, price: 59.99 }],
          totalAmount: 119.98,
        },
        metadata: { source: 'web-app', priority: 'low' },
      },
    ];

    const batchResult = await resourceManager.createBatch(batchInputs, {
      concurrency: 2,
      continueOnError: true,
    });

    console.log('✅ Batch creation completed:', {
      successful: batchResult.successful,
      failed: batchResult.failed,
      total: batchResult.total,
      duration: batchResult.duration,
    });

    // Execute validation on all successful resources
    if (batchResult.successful > 0) {
      console.log('\nExecuting validation on batch resources...');
      const successfulResources = batchResult.results
        .filter(r => r.success)
        .map(r => r.data!);

      const validationInputs = successfulResources.map(resource => ({
        resourceId: resource.id,
        activityId: 'validate_order',
        data: { batchValidation: true },
      }));

      const batchActivityResult = await resourceManager.executeActivityBatch(
        validationInputs,
        { concurrency: 3 }
      );

      console.log('✅ Batch activity execution completed:', {
        successful: batchActivityResult.successful,
        failed: batchActivityResult.failed,
      });
    }
  } catch (error) {
    console.error('❌ Error in batch operations:', error);
    throw error;
  }
}

async function resourceBuilderExample() {
  console.log('\n=== Resource Builder Example ===');

  try {
    // Create a resource builder with default options
    const builder = createResourceBuilder(sdk.resources, {
      validate: true,
      defaultMetadata: {
        environment: 'production',
        version: '1.0',
      },
      defaultWorkflowId: 'order-processing',
    });

    // Register a template for express orders
    const expressOrderTemplate = createResourceTemplate(
      'express-order',
      'order-processing',
      {
        expedited: true,
        processingTime: '2-hours',
        shippingMethod: 'express',
      },
      {
        description: 'Template for express orders',
        defaultMetadata: {
          priority: 'high',
          expedited: true,
        },
        requiredFields: ['orderId', 'customerId', 'totalAmount'],
      }
    );

    builder.registerTemplate(expressOrderTemplate);

    // Create a resource from template
    console.log('Creating express order from template...');
    const expressOrder = await builder.fromTemplate('express-order', {
      orderId: 'EXP-001',
      customerId: 'CUST-VIP',
      totalAmount: 299.99,
      items: [{ productId: 'PROD-PREMIUM', quantity: 1, price: 299.99 }],
    });

    console.log('✅ Express order created:', {
      id: expressOrder.resource.id,
      state: expressOrder.resource.state,
      expedited: expressOrder.resource.data.expedited,
    });

    // Use fluent interface for state transitions
    console.log('\nUsing fluent interface for processing...');
    const processedOrder = await expressOrder
      .executeActivity('validate_order', { fastTrack: true })
      .then(result => result.transitionTo('processing', {
        processedAt: new Date().toISOString(),
        fastTrack: true,
      }));

    console.log('✅ Order processed using fluent interface:', {
      state: processedOrder.resource.state,
      fastTrack: processedOrder.resource.data.fastTrack,
    });

    // Create and execute conditional transition
    console.log('\nCreating conditional transition...');
    const conditionalTransition = builder.createConditionalTransition({
      condition: async (resource) => {
        return resource.metadata.priority === 'high' &&
               resource.data.totalAmount > 200;
      },
      targetState: 'shipped',
      data: { autoShipped: true, reason: 'high-value-express' },
    });

    const transitionResult = await conditionalTransition.execute(processedOrder.resource);
    if (transitionResult) {
      console.log('✅ Conditional transition executed:', {
        newState: transitionResult.resource.state,
        autoShipped: transitionResult.resource.data.autoShipped,
      });
    }

    // Batch creation from template
    console.log('\nCreating batch orders from template...');
    const batchFromTemplate = await builder.createBatchFromTemplate(
      'express-order',
      [
        {
          batchId: 'batch-1',
          data: {
            orderId: 'EXP-002',
            customerId: 'CUST-VIP2',
            totalAmount: 199.99,
          },
        },
        {
          batchId: 'batch-2',
          data: {
            orderId: 'EXP-003',
            customerId: 'CUST-VIP3',
            totalAmount: 399.99,
          },
        },
      ]
    );

    console.log('✅ Batch from template created:', {
      successful: batchFromTemplate.batchResult.successful,
      failed: batchFromTemplate.batchResult.failed,
    });

    // Execute activity on all successful batch resources
    const batchActivityResults = await batchFromTemplate.executeActivityOnAll(
      'validate_order',
      { batchTemplate: true }
    );

    console.log('✅ Batch activity execution on template resources:', {
      processed: batchActivityResults.length,
    });

  } catch (error) {
    console.error('❌ Error in resource builder example:', error);
    throw error;
  }
}

async function searchAndAnalytics() {
  console.log('\n=== Search and Analytics ===');

  const resourceManager = sdk.resources;

  try {
    // Search for resources with various filters
    console.log('Searching for high-priority orders...');
    const highPriorityOrders = await resourceManager.search({
      metadata: { priority: 'high' },
      includeWorkflow: true,
      limit: 10,
    });

    console.log('✅ High priority orders found:', {
      count: highPriorityOrders.items.length,
      totalCount: highPriorityOrders.totalCount,
    });

    // Search for orders in specific states
    console.log('\nSearching for orders in processing states...');
    const processingOrders = await resourceManager.search({
      states: ['validated', 'processing'],
      createdAfter: new Date(Date.now() - 24 * 60 * 60 * 1000), // Last 24 hours
      sortBy: 'createdAt',
      sortDirection: 'desc',
    });

    console.log('✅ Processing orders found:', {
      count: processingOrders.items.length,
      states: processingOrders.items.map(r => r.state),
    });

    // Get resource statistics
    console.log('\nGetting resource statistics...');
    const stats = await resourceManager.getStats('order-processing');
    console.log('✅ Resource statistics:', {
      totalResources: stats.totalResources,
      activeResources: stats.activeResources,
      byState: stats.byState,
      averageAge: stats.averageAge,
    });

    // Get health status
    console.log('\nChecking resource health...');
    const health = await resourceManager.getHealth('order-processing');
    console.log('✅ Resource health:', {
      healthy: health.healthy,
      issues: health.issues.length,
      stuckResources: health.stuckResources,
      errorRate: health.errorRate,
    });

  } catch (error) {
    console.error('❌ Error in search and analytics:', error);
    throw error;
  }
}

async function validationExample() {
  console.log('\n=== Validation Example ===');

  const resourceManager = sdk.resources;

  try {
    // Create a resource for validation
    const testResource = await resourceManager.create({
      workflowId: 'order-processing',
      data: {
        orderId: 'TEST-001',
        customerId: 'TEST-CUSTOMER',
        totalAmount: 0, // Invalid amount
      },
      metadata: {
        test: true,
      },
    });

    // Validate the resource
    console.log('Validating test resource...');
    const validationReport = await resourceManager.validate(testResource.id, {
      validateData: true,
      validateState: true,
      validateMetadata: true,
      includeWarnings: true,
    });

    console.log('✅ Validation report:', {
      valid: validationReport.valid,
      errors: validationReport.errors,
      warnings: validationReport.warnings,
      suggestions: validationReport.suggestions,
    });

    // Clean up test resource
    await resourceManager.delete(testResource.id, { force: true });
    console.log('✅ Test resource cleaned up');

  } catch (error) {
    console.error('❌ Error in validation example:', error);
    throw error;
  }
}

async function runAllExamples() {
  console.log('🚀 Starting Resource Management Examples\n');

  try {
    // Initialize the SDK
    await sdk.initialize();
    console.log('✅ SDK initialized\n');

    // Run all examples
    const orderResourceId = await basicResourceOperations();
    await batchOperations();
    await resourceBuilderExample();
    await searchAndAnalytics();
    await validationExample();

    // Clean up created resources (optional)
    console.log('\n=== Cleanup ===');
    try {
      await sdk.resources.delete(orderResourceId, { force: true });
      console.log('✅ Cleanup completed');
    } catch (cleanupError) {
      console.warn('⚠️ Cleanup warning:', cleanupError);
    }

    console.log('\n🎉 All examples completed successfully!');

  } catch (error) {
    console.error('💥 Example execution failed:', error);
    throw error;
  } finally {
    // Dispose of SDK resources
    await sdk.dispose();
    console.log('👋 SDK disposed');
  }
}

// Export for use in other examples or tests
export {
  basicResourceOperations,
  batchOperations,
  resourceBuilderExample,
  searchAndAnalytics,
  validationExample,
  runAllExamples,
};

// Run examples if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllExamples().catch(console.error);
}
