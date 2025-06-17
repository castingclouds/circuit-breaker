#!/usr/bin/env tsx
/**
 * Basic Workflow Example - Circuit Breaker TypeScript SDK
 *
 * This example demonstrates the core functionality of the Circuit Breaker SDK:
 * - Creating and configuring the SDK client
 * - Building workflows with the fluent API
 * - Adding rules and conditions
 * - Creating and managing resources
 * - Executing state transitions
 * - Monitoring workflow progress
 *
 * Run with: npx tsx examples/basic-workflow.ts
 */

import {
  CircuitBreakerSDK,
  createWorkflow,
  createSDK,
  WorkflowValidationError,
  ResourceError,
  CircuitBreakerError,
  formatError,
  generateRequestId,
} from '../src/index.js';

// ============================================================================
// Configuration
// ============================================================================

const config = {
  graphqlEndpoint: process.env.CIRCUIT_BREAKER_ENDPOINT || 'http://localhost:4000/graphql',
  timeout: 30000,
  debug: process.env.NODE_ENV === 'development',
  logging: {
    level: 'info' as const,
    structured: false,
  },
  headers: {
    'User-Agent': 'CircuitBreaker-SDK-Example/0.1.0',
  },
};

// ============================================================================
// Helper Functions
// ============================================================================

function logSuccess(message: string, data?: any): void {
  console.log(`✅ ${message}`);
  if (data && config.debug) {
    console.log('   Data:', JSON.stringify(data, null, 2));
  }
}

function logInfo(message: string, data?: any): void {
  console.log(`ℹ️  ${message}`);
  if (data && config.debug) {
    console.log('   Data:', JSON.stringify(data, null, 2));
  }
}

function logError(message: string, error?: any): void {
  console.log(`❌ ${message}`);
  if (error) {
    if (error instanceof CircuitBreakerError) {
      console.log(`   Error: ${error.message} (${error.code})`);
      if (error.context && config.debug) {
        console.log('   Context:', JSON.stringify(error.context, null, 2));
      }
    } else {
      console.log('   Error:', formatError(error));
    }
  }
}

function logWarning(message: string, data?: any): void {
  console.log(`⚠️  ${message}`);
  if (data && config.debug) {
    console.log('   Data:', JSON.stringify(data, null, 2));
  }
}

// ============================================================================
// Example Workflow Definitions
// ============================================================================

/**
 * Create an order processing workflow
 */
function createOrderProcessingWorkflow() {
  return createWorkflow('Order Processing System')
    .setDescription('Complete order processing workflow with validation and fulfillment')
    .setVersion('1.0.0')
    .addTags(['ecommerce', 'orders', 'fulfillment'])

    // Define workflow states
    .addStates([
      'pending',           // Initial state
      'validated',         // Order validation complete
      'payment_processing', // Payment being processed
      'payment_confirmed',  // Payment successful
      'fulfillment',       // Order being fulfilled
      'shipped',           // Order shipped
      'delivered',         // Order delivered
      'completed',         // Order completed successfully
      'cancelled',         // Order cancelled
      'failed'             // Order failed
    ])

    .setInitialState('pending')

    // Add transitions with validation rules
    .addTransition('pending', 'validated', 'validate_order', {
      name: 'Validate Order',
      description: 'Validate order data and inventory availability',
    })

    .addTransition('validated', 'payment_processing', 'process_payment', {
      name: 'Process Payment',
      description: 'Initiate payment processing',
    })

    .addTransition('payment_processing', 'payment_confirmed', 'confirm_payment', {
      name: 'Confirm Payment',
      description: 'Confirm payment was successful',
    })

    .addTransition('payment_processing', 'failed', 'payment_failed', {
      name: 'Payment Failed',
      description: 'Handle payment failure',
    })

    .addTransition('payment_confirmed', 'fulfillment', 'start_fulfillment', {
      name: 'Start Fulfillment',
      description: 'Begin order fulfillment process',
    })

    .addTransition('fulfillment', 'shipped', 'ship_order', {
      name: 'Ship Order',
      description: 'Ship the order to customer',
    })

    .addTransition('shipped', 'delivered', 'mark_delivered', {
      name: 'Mark Delivered',
      description: 'Mark order as delivered',
    })

    .addTransition('delivered', 'completed', 'complete_order', {
      name: 'Complete Order',
      description: 'Finalize order completion',
    })

    // Cancellation paths
    .addTransition('pending', 'cancelled', 'cancel_order', {
      name: 'Cancel Order',
      description: 'Cancel order before processing',
    })

    .addTransition('validated', 'cancelled', 'cancel_validated_order', {
      name: 'Cancel Validated Order',
      description: 'Cancel order after validation',
    })

    // Add business rules
    .addSimpleRule('validate_order', 'items', 'exists')
    .addSimpleRule('validate_order', 'customer_id', 'exists')
    .addSimpleRule('validate_order', 'total_amount', '>', 0)

    .addSimpleRule('process_payment', 'payment_method', 'exists')
    .addSimpleRule('process_payment', 'billing_address', 'exists')

    .addSimpleRule('confirm_payment', 'payment_status', '==', 'confirmed')
    .addSimpleRule('confirm_payment', 'transaction_id', 'exists')

    .addSimpleRule('start_fulfillment', 'inventory_reserved', '==', true)
    .addSimpleRule('start_fulfillment', 'shipping_address', 'exists')

    .addSimpleRule('ship_order', 'tracking_number', 'exists')
    .addSimpleRule('ship_order', 'carrier', 'exists')

    // Add metadata
    .addMetadata('department', 'ecommerce')
    .addMetadata('owner', 'fulfillment-team')
    .addMetadata('sla_hours', 24)
    .addMetadata('priority', 'high');
}

/**
 * Create a simple approval workflow
 */
function createApprovalWorkflow() {
  return createWorkflow('Document Approval Process')
    .setDescription('Simple document approval workflow with review and approval steps')
    .setVersion('1.0.0')

    .addStates(['submitted', 'under_review', 'approved', 'rejected', 'revision_needed'])
    .setInitialState('submitted')

    .addTransition('submitted', 'under_review', 'start_review')
    .addTransition('under_review', 'approved', 'approve')
    .addTransition('under_review', 'rejected', 'reject')
    .addTransition('under_review', 'revision_needed', 'request_revision')
    .addTransition('revision_needed', 'submitted', 'resubmit')

    // Add simple rules
    .addSimpleRule('start_review', 'document_type', 'exists')
    .addSimpleRule('start_review', 'submitted_by', 'exists')
    .addSimpleRule('approve', 'reviewer_approval', '==', true)
    .addSimpleRule('reject', 'rejection_reason', 'exists');
}

// ============================================================================
// Main Example Function
// ============================================================================

async function runBasicWorkflowExample(): Promise<void> {
  console.log('🚀 Circuit Breaker TypeScript SDK - Basic Workflow Example');
  console.log('==========================================================');
  console.log();

  let sdk: CircuitBreakerSDK;

  try {
    // Initialize SDK
    logInfo('Initializing Circuit Breaker SDK...');
    sdk = createSDK(config.graphqlEndpoint, config);

    // Initialize and check health
    const initResult = await sdk.initialize();
    if (!initResult.success) {
      throw new Error(`SDK initialization failed: ${initResult.errors.join(', ')}`);
    }

    logSuccess('SDK initialized successfully');
    if (initResult.warnings.length > 0) {
      initResult.warnings.forEach(warning => logWarning(warning));
    }

    // Display component status
    logInfo('Component Status:', initResult.components);
    console.log();

    // ========================================================================
    // Part 1: Create Order Processing Workflow
    // ========================================================================

    logInfo('📋 Creating Order Processing Workflow...');

    const orderWorkflow = createOrderProcessingWorkflow();

    // Validate workflow structure
    const validation = orderWorkflow.validate();
    if (!validation.valid) {
      throw new WorkflowValidationError(validation.errors);
    }

    logSuccess(`Workflow validation passed`);
    logInfo(`States: ${validation.stateCount}, Activities: ${validation.activityCount}, Rules: ${validation.ruleCount}`);

    if (validation.warnings.length > 0) {
      validation.warnings.forEach(warning => logWarning(warning));
    }

    // Build and create workflow
    const workflowDefinition = orderWorkflow.build();
    // Note: In a real implementation, this would call sdk.workflows.create()
    // const orderWorkflowId = await sdk.workflows.create(workflowDefinition);
    const orderWorkflowId = 'workflow_' + generateRequestId(); // Simulated

    logSuccess(`Created workflow: ${workflowDefinition.name} (${orderWorkflowId})`);
    console.log();

    // ========================================================================
    // Part 2: Create Order Resources
    // ========================================================================

    logInfo('📦 Creating Order Resources...');

    const orderData = {
      orderId: 'ORD-2024-001',
      customerId: 'CUST-12345',
      items: [
        {
          sku: 'LAPTOP-001',
          name: 'Gaming Laptop',
          quantity: 1,
          price: 1299.99,
          category: 'electronics'
        },
        {
          sku: 'MOUSE-001',
          name: 'Wireless Gaming Mouse',
          quantity: 1,
          price: 79.99,
          category: 'accessories'
        }
      ],
      total_amount: 1379.98,
      currency: 'USD',
      payment_method: {
        type: 'credit_card',
        last_four: '4242',
        brand: 'visa'
      },
      billing_address: {
        street: '123 Main St',
        city: 'San Francisco',
        state: 'CA',
        zip: '94105',
        country: 'US'
      },
      shipping_address: {
        street: '456 Oak Ave',
        city: 'Palo Alto',
        state: 'CA',
        zip: '94301',
        country: 'US'
      }
    };

    // Note: In a real implementation, this would call sdk.resources.create()
    // const orderResource = await sdk.resources.create({
    //   workflowId: orderWorkflowId,
    //   data: orderData,
    //   metadata: {
    //     created_by: 'customer_portal',
    //     source: 'web',
    //     priority: 'standard'
    //   }
    // });

    // Simulated resource creation
    const orderResource = {
      id: 'resource_' + generateRequestId(),
      workflowId: orderWorkflowId,
      state: 'pending',
      data: orderData,
      metadata: {
        created_by: 'customer_portal',
        source: 'web',
        priority: 'standard'
      },
      history: [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    logSuccess(`Created order resource: ${orderResource.id}`);
    logInfo(`Order ID: ${orderData.orderId}`);
    logInfo(`Customer: ${orderData.customerId}`);
    logInfo(`Total: $${orderData.total_amount} ${orderData.currency}`);
    logInfo(`Items: ${orderData.items.length} items`);
    logInfo(`Current state: ${orderResource.state}`);
    console.log();

    // ========================================================================
    // Part 3: Execute Order Processing Workflow
    // ========================================================================

    logInfo('⚡ Executing Order Processing Workflow...');

    const activities = [
      {
        id: 'validate_order',
        name: 'Validate Order',
        data: {
          validation_timestamp: new Date().toISOString(),
          inventory_check: true,
          customer_verified: true,
          fraud_check: 'passed'
        }
      },
      {
        id: 'process_payment',
        name: 'Process Payment',
        data: {
          payment_processor: 'stripe',
          processing_timestamp: new Date().toISOString(),
          attempt_number: 1
        }
      },
      {
        id: 'confirm_payment',
        name: 'Confirm Payment',
        data: {
          payment_status: 'confirmed',
          transaction_id: 'txn_' + generateRequestId(),
          confirmation_timestamp: new Date().toISOString(),
          amount_charged: orderData.total_amount
        }
      },
      {
        id: 'start_fulfillment',
        name: 'Start Fulfillment',
        data: {
          inventory_reserved: true,
          fulfillment_center: 'FC-SF-01',
          estimated_ship_date: new Date(Date.now() + 86400000).toISOString(), // Tomorrow
          warehouse_location: 'San Francisco, CA'
        }
      },
      {
        id: 'ship_order',
        name: 'Ship Order',
        data: {
          tracking_number: 'TRK-' + generateRequestId().substring(0, 10).toUpperCase(),
          carrier: 'UPS',
          shipping_method: 'ground',
          estimated_delivery: new Date(Date.now() + 3 * 86400000).toISOString(), // 3 days
          ship_timestamp: new Date().toISOString()
        }
      },
      {
        id: 'mark_delivered',
        name: 'Mark Delivered',
        data: {
          delivery_timestamp: new Date(Date.now() + 1000).toISOString(), // Simulate quick delivery
          delivery_confirmation: 'signature',
          delivered_to: 'Front door'
        }
      },
      {
        id: 'complete_order',
        name: 'Complete Order',
        data: {
          completion_timestamp: new Date().toISOString(),
          customer_satisfaction_survey_sent: true,
          invoice_generated: true,
          order_completion_status: 'successful'
        }
      }
    ];

    let currentResource = orderResource;
    const expectedStates = [
      'pending',
      'validated',
      'payment_processing',
      'payment_confirmed',
      'fulfillment',
      'shipped',
      'delivered',
      'completed'
    ];

    for (let i = 0; i < activities.length; i++) {
      const activity = activities[i];
      const expectedNextState = expectedStates[i + 1];

      logInfo(`Executing: ${activity.name} (${activity.id})`);

      // Simulate small delay
      await new Promise(resolve => setTimeout(resolve, 200));

      try {
        // Note: In a real implementation, this would call sdk.resources.executeActivity()
        // const result = await sdk.resources.executeActivity({
        //   resourceId: currentResource.id,
        //   activityId: activity.id,
        //   data: activity.data
        // });

        // Simulate activity execution
        const historyEvent = {
          timestamp: new Date().toISOString(),
          activity: activity.id,
          fromState: currentResource.state,
          toState: expectedNextState || currentResource.state,
          data: activity.data
        };

        currentResource = {
          ...currentResource,
          state: expectedNextState || currentResource.state,
          data: { ...currentResource.data, ...activity.data },
          history: [...currentResource.history, historyEvent],
          updatedAt: new Date().toISOString()
        };

        logSuccess(`✓ Activity completed: ${currentResource.state}`);

        // Log relevant activity data
        if (activity.data.transaction_id) {
          logInfo(`  Transaction ID: ${activity.data.transaction_id}`);
        }
        if (activity.data.tracking_number) {
          logInfo(`  Tracking Number: ${activity.data.tracking_number}`);
        }
        if (activity.data.estimated_delivery) {
          logInfo(`  Estimated Delivery: ${new Date(activity.data.estimated_delivery).toLocaleDateString()}`);
        }

      } catch (error) {
        logError(`Activity execution failed: ${activity.name}`, error);

        if (error instanceof ResourceError) {
          logError('Resource error details:', {
            resourceId: currentResource.id,
            currentState: currentResource.state,
            activityId: activity.id
          });
        }

        // In a real scenario, you might want to handle different error types
        // and potentially trigger error handling workflows
        break;
      }
    }

    console.log();

    // ========================================================================
    // Part 4: Display Final Results
    // ========================================================================

    logSuccess('🎉 Order Processing Workflow Completed!');
    console.log();

    logInfo('Final Order Status:');
    console.log(`  Resource ID: ${currentResource.id}`);
    console.log(`  Order ID: ${currentResource.data.orderId}`);
    console.log(`  Final State: ${currentResource.state}`);
    console.log(`  Customer: ${currentResource.data.customerId}`);
    console.log(`  Total Amount: $${currentResource.data.total_amount}`);
    if (currentResource.data.tracking_number) {
      console.log(`  Tracking Number: ${currentResource.data.tracking_number}`);
    }
    if (currentResource.data.transaction_id) {
      console.log(`  Transaction ID: ${currentResource.data.transaction_id}`);
    }
    console.log();

    logInfo('Order Processing History:');
    currentResource.history.forEach((event, index) => {
      const timestamp = new Date(event.timestamp).toLocaleTimeString();
      console.log(`  ${index + 1}. ${event.fromState} → ${event.toState} via ${event.activity} (${timestamp})`);

      // Show key data for important transitions
      if (event.data.transaction_id) {
        console.log(`     💳 Payment: ${event.data.transaction_id}`);
      }
      if (event.data.tracking_number) {
        console.log(`     📦 Tracking: ${event.data.tracking_number}`);
      }
      if (event.data.estimated_delivery) {
        console.log(`     🚚 Delivery: ${new Date(event.data.estimated_delivery).toLocaleDateString()}`);
      }
    });

    console.log();

    // ========================================================================
    // Part 5: Create Simple Approval Workflow Example
    // ========================================================================

    logInfo('📄 Creating Simple Approval Workflow...');

    const approvalWorkflow = createApprovalWorkflow();
    const approvalDefinition = approvalWorkflow.build();

    // Note: In a real implementation, this would create the workflow
    const approvalWorkflowId = 'workflow_' + generateRequestId();

    logSuccess(`Created approval workflow: ${approvalDefinition.name} (${approvalWorkflowId})`);

    // Create a document for approval
    const documentData = {
      documentId: 'DOC-2024-001',
      title: 'Q4 Marketing Budget Proposal',
      document_type: 'budget_proposal',
      submitted_by: 'marketing_team',
      department: 'marketing',
      amount_requested: 250000,
      description: 'Budget proposal for Q4 marketing campaigns',
      attachments: ['budget_breakdown.xlsx', 'campaign_overview.pdf'],
      urgency: 'high'
    };

    const documentResource = {
      id: 'resource_' + generateRequestId(),
      workflowId: approvalWorkflowId,
      state: 'submitted',
      data: documentData,
      metadata: {
        created_by: 'document_portal',
        department: 'marketing',
        requires_approval: true
      },
      history: [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    logSuccess(`Created document resource: ${documentResource.id}`);
    logInfo(`Document: ${documentData.title}`);
    logInfo(`Submitted by: ${documentData.submitted_by}`);
    logInfo(`Amount: $${documentData.amount_requested.toLocaleString()}`);
    console.log();

    // Simulate approval process
    logInfo('⚡ Executing Approval Process...');

    const approvalActivities = [
      {
        id: 'start_review',
        name: 'Start Review',
        expectedState: 'under_review',
        data: {
          reviewer_assigned: 'finance_director',
          review_started: new Date().toISOString(),
          estimated_completion: new Date(Date.now() + 86400000).toISOString() // 1 day
        }
      },
      {
        id: 'approve',
        name: 'Approve Document',
        expectedState: 'approved',
        data: {
          reviewer_approval: true,
          approved_by: 'finance_director',
          approval_timestamp: new Date().toISOString(),
          approved_amount: documentData.amount_requested,
          approval_notes: 'Budget allocation approved for Q4 marketing initiatives'
        }
      }
    ];

    let currentDocumentResource = documentResource;

    for (const activity of approvalActivities) {
      logInfo(`Executing: ${activity.name} (${activity.id})`);

      await new Promise(resolve => setTimeout(resolve, 100));

      const historyEvent = {
        timestamp: new Date().toISOString(),
        activity: activity.id,
        fromState: currentDocumentResource.state,
        toState: activity.expectedState,
        data: activity.data
      };

      currentDocumentResource = {
        ...currentDocumentResource,
        state: activity.expectedState,
        data: { ...currentDocumentResource.data, ...activity.data },
        history: [...currentDocumentResource.history, historyEvent],
        updatedAt: new Date().toISOString()
      };

      logSuccess(`✓ Activity completed: ${currentDocumentResource.state}`);

      if (activity.data.approved_by) {
        logInfo(`  Approved by: ${activity.data.approved_by}`);
      }
      if (activity.data.approval_notes) {
        logInfo(`  Notes: ${activity.data.approval_notes}`);
      }
    }

    console.log();
    logSuccess('🎉 Document Approval Completed!');
    console.log();

    logInfo('Final Document Status:');
    console.log(`  Document: ${currentDocumentResource.data.title}`);
    console.log(`  Status: ${currentDocumentResource.state}`);
    console.log(`  Approved Amount: $${currentDocumentResource.data.approved_amount?.toLocaleString() || 'N/A'}`);
    console.log(`  Approved By: ${currentDocumentResource.data.approved_by || 'N/A'}`);
    console.log();

    // ========================================================================
    // Part 6: SDK Statistics and Summary
    // ========================================================================

    logInfo('📊 SDK Usage Statistics:');
    const stats = sdk.getStats();
    console.log(`  GraphQL Requests: ${stats.requests.total}`);
    console.log(`  Successful: ${stats.requests.successful}`);
    console.log(`  Failed: ${stats.requests.failed}`);
    console.log(`  Average Response Time: ${stats.requests.averageResponseTime.toFixed(2)}ms`);
    console.log();

    const health = await sdk.getHealth();
    logInfo('System Health:');
    console.log(`  Overall Status: ${health.healthy ? '✅ Healthy' : '❌ Unhealthy'}`);
    console.log(`  SDK Version: ${health.version}`);
    console.log(`  Uptime: ${(health.uptime / 1000).toFixed(2)} seconds`);

    Object.entries(health.components).forEach(([component, status]) => {
      const statusIcon = status.status === 'healthy' ? '✅' :
                        status.status === 'degraded' ? '⚠️' : '❌';
      console.log(`  ${component}: ${statusIcon} ${status.status}`);
    });

    console.log();

    // ========================================================================
    // Summary
    // ========================================================================

    logSuccess('🎉 Basic Workflow Example Completed Successfully!');
    console.log();

    console.log('📋 What was demonstrated:');
    console.log('  ✅ SDK initialization and health checking');
    console.log('  ✅ Fluent workflow builder API');
    console.log('  ✅ Workflow validation and rule definition');
    console.log('  ✅ Resource creation and management');
    console.log('  ✅ Activity execution and state transitions');
    console.log('  ✅ Comprehensive error handling');
    console.log('  ✅ Workflow history and audit trails');
    console.log('  ✅ Multiple workflow patterns');
    console.log('  ✅ SDK statistics and monitoring');
    console.log();

    console.log('🚀 Next steps:');
    console.log('  • Try the rules engine example for advanced rule evaluation');
    console.log('  • Explore function integration for serverless execution');
    console.log('  • Test LLM integration for AI-powered workflows');
    console.log('  • Build AI agents with conversational interfaces');
    console.log('  • Implement real-time workflow monitoring');

  } catch (error) {
    logError('Example execution failed', error);

    if (error instanceof CircuitBreakerError) {
      console.log();
      console.log('💡 Troubleshooting tips:');

      if (error.code.includes('NETWORK') || error.code.includes('CONNECTION')) {
        console.log('  • Ensure Circuit Breaker server is running on the configured endpoint');
        console.log('  • Check network connectivity and firewall settings');
        console.log('  • Verify the GraphQL endpoint URL is correct');
      }

      if (error.code.includes('VALIDATION')) {
        console.log('  • Review workflow definition for structural issues');
        console.log('  • Check that all required fields are provided');
        console.log('  • Ensure state and activity names are valid');
      }

      if (error.code.includes('TIMEOUT')) {
        console.log('  • Increase timeout configuration');
        console.log('  • Check server performance and load');
        console.log('  • Consider breaking large operations into smaller chunks');
      }
    }

    process.exit(1);
  } finally {
    // Cleanup
    if (sdk!) {
      try {
        await sdk.dispose();
        logInfo('SDK resources disposed successfully');
      } catch (error) {
        logWarning('Error during SDK cleanup', error);
      }
    }
  }
}

// ============================================================================
// Main Execution
// ============================================================================

if (import.meta.url === `file://${process.argv[1]}`) {
  runBasicWorkflowExample().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

export { runBasicWorkflowExample };
