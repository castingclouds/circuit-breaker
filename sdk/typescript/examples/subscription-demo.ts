/**
 * Real-time Subscription Demo
 *
 * This example demonstrates the Circuit Breaker SDK's real-time subscription capabilities,
 * showing how to subscribe to various events including resource updates, workflow events,
 * agent executions, LLM streaming, cost updates, and MCP server status changes.
 */

import { Client } from '../src/client.js';
import {
  subscribeResourceUpdates,
  subscribeWorkflowEvents,
  subscribeLLMStream,
  subscribeCostUpdates,
  type ResourceUpdateEvent,
  type WorkflowEvent,
  type LLMStreamChunk,
  type CostUpdateEvent,
  type AgentExecutionEvent,
  type MCPServerStatusUpdate,
  type MCPSessionEvent,
  type SubscriptionId,
} from '../src/subscriptions.js';

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main(): Promise<void> {
  console.log('🔔 Circuit Breaker Real-time Subscription Demo');
  console.log('===============================================');

  // Initialize the client
  const baseUrl = process.env.CIRCUIT_BREAKER_URL || 'http://localhost:4000';
  const apiKey = process.env.CIRCUIT_BREAKER_API_KEY;

  let client = Client.builder().baseUrl(baseUrl);
  if (apiKey) {
    client = client.apiKey(apiKey);
  }
  const circuitBreakerClient = client.build();

  // Test connection
  try {
    const ping = await circuitBreakerClient.ping();
    console.log(`✅ Connected to Circuit Breaker server: ${ping.message}`);
  } catch (error) {
    console.log(`❌ Failed to connect to server: ${error}`);
    console.log('   Note: This demo shows subscription infrastructure even without a running server');
    console.log(`   In production, ensure the Circuit Breaker server is running at ${baseUrl}`);
  }

  console.log('\n📡 Real-time Subscription Features:');
  console.log('===================================');

  // 1. Resource Updates Subscription
  console.log('\n1. 📊 Resource Updates Subscription');
  console.log('   ---------------------------------');

  const resourceId = 'demo_resource_123';
  console.log(`   Subscribing to updates for resource: ${resourceId}`);

  try {
    const resourceSubId = await circuitBreakerClient
      .subscriptions()
      .resourceUpdates()
      .resourceId(resourceId)
      .subscribe(
        (resource: ResourceUpdateEvent) => {
          console.log('   📦 Resource Update Received:');
          console.log(`      • ID: ${resource.id}`);
          console.log(`      • Workflow: ${resource.workflowId}`);
          console.log(`      • State: ${resource.state}`);
          console.log(`      • Updated: ${resource.updatedAt}`);
        },
        (error) => {
          console.log(`   ❌ Resource subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ Resource subscription completed');
        },
      );

    console.log(`   ✅ Resource subscription active: ${resourceSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Resource subscription setup: ${error}`);
  }

  // 2. Workflow Events Subscription
  console.log('\n2. 🔄 Workflow Events Subscription');
  console.log('   --------------------------------');

  const workflowId = 'demo_workflow_456';
  console.log(`   Subscribing to events for workflow: ${workflowId}`);

  try {
    const workflowSubId = await circuitBreakerClient
      .subscriptions()
      .workflowEvents()
      .workflowId(workflowId)
      .subscribe(
        (event: WorkflowEvent) => {
          console.log('   🔄 Workflow Event Received:');
          console.log(`      • ID: ${event.id}`);
          console.log(`      • Type: ${event.type}`);
          console.log(`      • Message: ${event.message}`);
          console.log(`      • Timestamp: ${event.timestamp}`);
        },
        (error) => {
          console.log(`   ❌ Workflow subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ Workflow subscription completed');
        },
      );

    console.log(`   ✅ Workflow subscription active: ${workflowSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Workflow subscription setup: ${error}`);
  }

  // 3. LLM Streaming Subscription
  console.log('\n3. 🤖 LLM Streaming Subscription');
  console.log('   ------------------------------');

  const requestId = 'llm_request_789';
  console.log(`   Subscribing to LLM stream for request: ${requestId}`);

  try {
    const llmSubId = await circuitBreakerClient
      .subscriptions()
      .llmStream(requestId)
      .subscribe(
        (chunk: LLMStreamChunk) => {
          console.log('   🤖 LLM Chunk Received:');
          console.log(`      • Request ID: ${chunk.id}`);
          console.log(`      • Content: ${chunk.content}`);
          console.log(`      • Finished: ${chunk.finished}`);
          console.log(`      • Timestamp: ${chunk.timestamp}`);
        },
        (error) => {
          console.log(`   ❌ LLM stream subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ LLM stream subscription completed');
        },
      );

    console.log(`   ✅ LLM stream subscription active: ${llmSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  LLM stream subscription setup: ${error}`);
  }

  // 4. Cost Updates Subscription
  console.log('\n4. 💰 Cost Updates Subscription');
  console.log('   -----------------------------');

  const userId = 'demo_user_123';
  console.log(`   Subscribing to cost updates for user: ${userId}`);

  try {
    const costSubId = await circuitBreakerClient
      .subscriptions()
      .costUpdates()
      .userId(userId)
      .subscribe(
        (update: CostUpdateEvent) => {
          console.log('   💰 Cost Update Received:');
          if (update.userId) {
            console.log(`      • User ID: ${update.userId}`);
          }
          if (update.projectId) {
            console.log(`      • Project ID: ${update.projectId}`);
          }
          console.log(`      • Cost: $${update.cost.toFixed(2)}`);
          console.log(`      • Timestamp: ${update.timestamp}`);
        },
        (error) => {
          console.log(`   ❌ Cost updates subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ Cost updates subscription completed');
        },
      );

    console.log(`   ✅ Cost updates subscription active: ${costSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Cost updates subscription setup: ${error}`);
  }

  // 5. Agent Execution Stream
  console.log('\n5. 🤖 Agent Execution Stream');
  console.log('   --------------------------');

  const executionId = 'agent_exec_456';
  console.log(`   Subscribing to agent execution: ${executionId}`);

  try {
    const agentSubId = await circuitBreakerClient
      .subscriptions()
      .agentExecutionStream()
      .executionId(executionId)
      .subscribe(
        (event: AgentExecutionEvent) => {
          console.log('   🤖 Agent Execution Event:');
          console.log(`      • ID: ${event.id}`);
          console.log(`      • Agent: ${event.agentId}`);
          console.log(`      • Status: ${event.status}`);
          console.log(`      • Timestamp: ${event.timestamp}`);
        },
        (error) => {
          console.log(`   ❌ Agent execution subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ Agent execution subscription completed');
        },
      );

    console.log(`   ✅ Agent execution subscription active: ${agentSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Agent execution subscription setup: ${error}`);
  }

  // 6. MCP Server Status Updates
  console.log('\n6. 🔌 MCP Server Status Updates');
  console.log('   -----------------------------');

  const serverId = 'mcp_server_789';
  console.log(`   Subscribing to MCP server status: ${serverId}`);

  try {
    const mcpStatusSubId = await circuitBreakerClient
      .subscriptions()
      .mcpServerStatusUpdates()
      .serverId(serverId)
      .subscribe(
        (update: MCPServerStatusUpdate) => {
          console.log('   🔌 MCP Server Status Update:');
          console.log(`      • Server ID: ${update.serverId}`);
          console.log(`      • Status: ${update.status}`);
          if (update.message) {
            console.log(`      • Message: ${update.message}`);
          }
          console.log(`      • Timestamp: ${update.timestamp}`);
        },
        (error) => {
          console.log(`   ❌ MCP status subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ MCP status subscription completed');
        },
      );

    console.log(`   ✅ MCP status subscription active: ${mcpStatusSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  MCP status subscription setup: ${error}`);
  }

  // 7. MCP Session Events
  console.log('\n7. 📡 MCP Session Events');
  console.log('   ----------------------');

  console.log(`   Subscribing to MCP session events for user: ${userId}`);

  try {
    const mcpSessionSubId = await circuitBreakerClient
      .subscriptions()
      .mcpSessionEvents()
      .userId(userId)
      .subscribe(
        (event: MCPSessionEvent) => {
          console.log('   📡 MCP Session Event:');
          console.log(`      • Session ID: ${event.sessionId}`);
          console.log(`      • Event: ${event.event}`);
          console.log(`      • Timestamp: ${event.timestamp}`);
        },
        (error) => {
          console.log(`   ❌ MCP session subscription error: ${error.message}`);
        },
        () => {
          console.log('   ✅ MCP session subscription completed');
        },
      );

    console.log(`   ✅ MCP session subscription active: ${mcpSessionSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  MCP session subscription setup: ${error}`);
  }

  // 8. Subscription Metrics and Monitoring
  console.log('\n8. 📈 Subscription Metrics');
  console.log('   ------------------------');

  const metrics = circuitBreakerClient.subscriptions().getMetrics();
  console.log('   Current Subscription Status:');
  console.log(`   • Active Subscriptions: ${metrics.activeSubscriptions}`);
  console.log(`   • Messages Received: ${metrics.messagesReceived}`);

  // 9. Convenience Functions Demo
  console.log('\n9. 🛠️  Convenience Functions');
  console.log('   -------------------------');

  console.log('   Using convenience functions for common subscriptions:');

  // Resource updates convenience function
  try {
    const convResourceSubId = await subscribeResourceUpdates(
      circuitBreakerClient,
      'convenience_resource',
      (resource) => {
        console.log(`   📦 Convenience resource update: ${resource.id} -> ${resource.state}`);
      },
    );

    console.log(`   ✅ Convenience resource subscription: ${convResourceSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Convenience resource subscription: ${error}`);
  }

  // Workflow events convenience function
  try {
    const convWorkflowSubId = await subscribeWorkflowEvents(
      circuitBreakerClient,
      'convenience_workflow',
      (event) => {
        console.log(`   🔄 Convenience workflow event: ${event.type} - ${event.message}`);
      },
    );

    console.log(`   ✅ Convenience workflow subscription: ${convWorkflowSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Convenience workflow subscription: ${error}`);
  }

  // LLM stream convenience function
  try {
    const convLLMSubId = await subscribeLLMStream(
      circuitBreakerClient,
      'convenience_llm_request',
      (chunk) => {
        console.log(`   🤖 Convenience LLM chunk: ${chunk.content} (finished: ${chunk.finished})`);
      },
    );

    console.log(`   ✅ Convenience LLM subscription: ${convLLMSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Convenience LLM subscription: ${error}`);
  }

  // Cost updates convenience function
  try {
    const convCostSubId = await subscribeCostUpdates(
      circuitBreakerClient,
      'convenience_user',
      (update) => {
        console.log(`   💰 Convenience cost update: $${update.cost.toFixed(2)}`);
      },
    );

    console.log(`   ✅ Convenience cost subscription: ${convCostSubId.toString()}`);
  } catch (error) {
    console.log(`   ⚠️  Convenience cost subscription: ${error}`);
  }

  // 10. Advanced Subscription Patterns
  console.log('\n10. 🔬 Advanced Subscription Patterns');
  console.log('    ----------------------------------');

  // Multiple subscriptions for the same resource
  console.log('    Setting up multiple subscriptions for comprehensive monitoring:');

  const monitoredResourceId = 'monitored_resource_999';

  // State change monitoring
  try {
    const stateMonitorSubId = await circuitBreakerClient
      .subscriptions()
      .resourceUpdates()
      .resourceId(monitoredResourceId)
      .subscribe((resource) => {
        console.log(`    🔍 State Monitor: ${resource.id} is now in state '${resource.state}'`);
      });

    console.log(`    ✅ State monitor active: ${stateMonitorSubId.toString()}`);
  } catch (error) {
    console.log(`    ⚠️  State monitor setup: ${error}`);
  }

  // Workflow context monitoring
  try {
    const workflowMonitorSubId = await circuitBreakerClient
      .subscriptions()
      .workflowEvents()
      .workflowId('monitored_workflow')
      .subscribe((event) => {
        console.log(`    🔍 Workflow Monitor: ${event.message}`);
      });

    console.log(`    ✅ Workflow monitor active: ${workflowMonitorSubId.toString()}`);
  } catch (error) {
    console.log(`    ⚠️  Workflow monitor setup: ${error}`);
  }

  // 11. Real-time Dashboard Simulation
  console.log('\n11. 📊 Real-time Dashboard Simulation');
  console.log('    -----------------------------------');

  console.log('    Simulating a real-time dashboard with multiple data streams:');

  const dashboardMetrics = circuitBreakerClient.subscriptions().getMetrics();
  console.log('    📊 Dashboard Metrics:');
  console.log(`       • Total Active Streams: ${dashboardMetrics.activeSubscriptions}`);
  console.log(`       • Data Points Received: ${dashboardMetrics.messagesReceived}`);

  // Simulate dashboard updates
  for (let i = 1; i <= 5; i++) {
    await sleep(1000);
    const currentMetrics = circuitBreakerClient.subscriptions().getMetrics();
    console.log(
      `    📊 Dashboard Update #${i}: ${currentMetrics.activeSubscriptions} active streams, ${currentMetrics.messagesReceived} total messages`,
    );
  }

  // 12. TypeScript-specific Features
  console.log('\n12. 🔷 TypeScript-Specific Features');
  console.log('    --------------------------------');

  console.log('    Type-safe subscription handling:');

  // Type-safe event handlers with destructuring
  try {
    const typeSafeSubId = await circuitBreakerClient
      .subscriptions()
      .resourceUpdates()
      .resourceId('typescript_resource')
      .subscribe(
        ({ id, state, workflowId, updatedAt }: ResourceUpdateEvent) => {
          console.log('    🔷 Type-safe resource update:');
          console.log(`       • Resource: ${id}`);
          console.log(`       • State: ${state}`);
          console.log(`       • Workflow: ${workflowId}`);
          console.log(`       • Updated: ${updatedAt}`);
        },
        (error) => {
          // Type-safe error handling
          const errorMessage: string = error.message;
          const subscriptionId: string | undefined = error.subscriptionId?.toString();
          console.log(`    ❌ Type-safe error: ${errorMessage} (sub: ${subscriptionId})`);
        },
        () => {
          console.log('    ✅ Type-safe completion handler called');
        },
      );

    console.log(`    ✅ Type-safe subscription active: ${typeSafeSubId.toString()}`);
  } catch (error) {
    console.log(`    ⚠️  Type-safe subscription setup: ${error}`);
  }

  // Type-safe metrics access
  const typedMetrics = circuitBreakerClient.subscriptions().getMetrics();
  const activeCount: number = typedMetrics.activeSubscriptions;
  const messageCount: number = typedMetrics.messagesReceived;
  const failureCount: number = typedMetrics.connectionFailures;

  console.log('    🔷 Type-safe metrics access:');
  console.log(`       • Active (number): ${activeCount}`);
  console.log(`       • Messages (number): ${messageCount}`);
  console.log(`       • Failures (number): ${failureCount}`);

  // 13. Subscription Lifecycle Management
  console.log('\n13. 🔄 Subscription Lifecycle Management');
  console.log('    --------------------------------------');

  console.log('    Demonstrating subscription lifecycle:');
  console.log('    • All subscriptions are automatically managed');
  console.log('    • Auto-reconnection on connection loss');
  console.log('    • Graceful cleanup on application shutdown');
  console.log('    • Message queuing during disconnections');

  // Set up cleanup on process exit
  process.on('SIGINT', async () => {
    console.log('\n🔄 Cleaning up subscriptions...');
    try {
      await circuitBreakerClient.subscriptions().close();
      console.log('✅ All subscriptions closed gracefully');
      process.exit(0);
    } catch (error) {
      console.error('❌ Error during cleanup:', error);
      process.exit(1);
    }
  });

  // Wait a bit to show subscriptions are active
  console.log('\n⏰ Subscriptions are now active and listening for events...');
  console.log('   In a real application, this would continue running indefinitely.');
  console.log('   Events would be processed as they arrive from the server.');
  console.log('   Press Ctrl+C to gracefully shut down subscriptions.');

  await sleep(2000);

  // Final metrics
  const finalMetrics = circuitBreakerClient.subscriptions().getMetrics();
  console.log('\n📈 Final Subscription Statistics:');
  console.log(`   • Active Subscriptions: ${finalMetrics.activeSubscriptions}`);
  console.log(`   • Total Messages Processed: ${finalMetrics.messagesReceived}`);
  console.log(`   • Connection Failures: ${finalMetrics.connectionFailures}`);
  console.log(`   • Reconnection Attempts: ${finalMetrics.reconnectionAttempts}`);

  console.log('\n🎉 Subscription Demo Complete!');
  console.log('===============================');
  console.log('This demo showcased:');
  console.log('• Resource state change subscriptions');
  console.log('• Workflow event monitoring');
  console.log('• Real-time LLM streaming');
  console.log('• Cost update notifications');
  console.log('• Agent execution tracking');
  console.log('• MCP server status monitoring');
  console.log('• MCP session event handling');
  console.log('• Subscription metrics and monitoring');
  console.log('• Convenience functions for common patterns');
  console.log('• Advanced multi-stream monitoring');
  console.log('• Real-time dashboard simulation');
  console.log('• TypeScript-specific type safety features');
  console.log('• Subscription lifecycle management');
  console.log('\nThe Subscription infrastructure provides:');
  console.log('• Type-safe event handling with full TypeScript support');
  console.log('• Automatic reconnection and error recovery');
  console.log('• Comprehensive monitoring and metrics');
  console.log('• Builder patterns for ergonomic APIs');
  console.log('• Production-ready WebSocket management');
  console.log('• Real-time data streaming capabilities');
  console.log('• Graceful cleanup and resource management');

  // Keep the process running to demonstrate real-time capabilities
  console.log('\n⚡ Keeping process alive for real-time demonstration...');
  console.log('   Press Ctrl+C to exit gracefully');

  // Infinite loop to keep the process running
  // In a real app, this would be your main application logic
  while (true) {
    await sleep(5000);
    const liveMetrics = circuitBreakerClient.subscriptions().getMetrics();
    console.log(
      `📊 Live Metrics: ${liveMetrics.activeSubscriptions} subscriptions, ${liveMetrics.messagesReceived} messages`,
    );
  }
}

// Run the demo
main().catch((error) => {
  console.error('Demo failed:', error);
  process.exit(1);
});
