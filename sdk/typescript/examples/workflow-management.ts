#!/usr/bin/env tsx
/**
 * Workflow Management Example - Circuit Breaker TypeScript SDK
 *
 * This example demonstrates comprehensive workflow management functionality:
 * - Creating workflows with the WorkflowManager
 * - Searching and filtering workflows
 * - Updating workflow metadata
 * - Validating workflow definitions
 * - Getting workflow statistics and health
 * - Managing workflow lifecycle
 *
 * Run with: npx tsx examples/workflow-management.ts
 */

import {
  CircuitBreakerSDK,
  createWorkflow,
  createLinearWorkflow,
  createFromStateMachine,
  WorkflowManager,
  WorkflowCreateInput,
  WorkflowUpdateInput,
  WorkflowSearchOptions,
  WorkflowValidationOptions,
  CircuitBreakerError,
  WorkflowError,
  WorkflowNotFoundError,
  WorkflowValidationError,
  formatError,
  generateRequestId,
} from "../src/index.js";

// ============================================================================
// Configuration
// ============================================================================

const config = {
  graphqlEndpoint:
    process.env.CIRCUIT_BREAKER_ENDPOINT || "http://localhost:4000/graphql",
  timeout: 30000,
  debug: process.env.NODE_ENV === "development",
  logging: {
    level: "info" as const,
    structured: false,
  },
};

// ============================================================================
// Helper Functions
// ============================================================================

function logSuccess(message: string, data?: any): void {
  console.log(`✅ ${message}`);
  if (data && config.debug) {
    console.log("   ", JSON.stringify(data, null, 2));
  }
}

function logInfo(message: string, data?: any): void {
  console.log(`ℹ️  ${message}`);
  if (data && config.debug) {
    console.log("   ", JSON.stringify(data, null, 2));
  }
}

function logError(message: string, error?: any): void {
  console.log(`❌ ${message}`);
  if (error) {
    if (error instanceof CircuitBreakerError) {
      console.log(`   Error: ${error.message} (${error.code})`);
      if (error.context && config.debug) {
        console.log("   Context:", JSON.stringify(error.context, null, 2));
      }
    } else {
      console.log("   Error:", formatError(error));
    }
  }
}

function logWarning(message: string, data?: any): void {
  console.log(`⚠️  ${message}`);
  if (data && config.debug) {
    console.log("   ", JSON.stringify(data, null, 2));
  }
}

// ============================================================================
// Example Workflow Definitions
// ============================================================================

function createEcommerceWorkflow(): WorkflowCreateInput {
  return {
    name: "E-commerce Order Processing",
    description:
      "Complete e-commerce order processing workflow with payment and fulfillment",
    version: "2.1.0",
    tags: ["ecommerce", "orders", "payment", "fulfillment"],
    states: [
      "cart_created",
      "checkout_started",
      "payment_processing",
      "payment_confirmed",
      "inventory_reserved",
      "order_confirmed",
      "fulfillment_started",
      "shipped",
      "delivered",
      "completed",
      "cancelled",
      "refunded",
    ],
    activities: [
      {
        id: "start_checkout",
        name: "Start Checkout Process",
        fromStates: ["cart_created"],
        toState: "checkout_started",
        conditions: ["data.items.length > 0", "data.customer_id != null"],
        description: "Initiate checkout process for cart",
      },
      {
        id: "process_payment",
        name: "Process Payment",
        fromStates: ["checkout_started"],
        toState: "payment_processing",
        conditions: [
          "data.payment_method != null",
          "data.billing_address != null",
        ],
        description: "Submit payment for processing",
      },
      {
        id: "confirm_payment",
        name: "Confirm Payment",
        fromStates: ["payment_processing"],
        toState: "payment_confirmed",
        conditions: [
          'data.payment_status == "confirmed"',
          "data.transaction_id != null",
        ],
        description: "Confirm successful payment",
      },
      {
        id: "reserve_inventory",
        name: "Reserve Inventory",
        fromStates: ["payment_confirmed"],
        toState: "inventory_reserved",
        conditions: ["data.inventory_available == true"],
        description: "Reserve inventory for confirmed order",
      },
      {
        id: "confirm_order",
        name: "Confirm Order",
        fromStates: ["inventory_reserved"],
        toState: "order_confirmed",
        conditions: [],
        description: "Finalize order confirmation",
      },
      {
        id: "start_fulfillment",
        name: "Start Fulfillment",
        fromStates: ["order_confirmed"],
        toState: "fulfillment_started",
        conditions: ["data.shipping_address != null"],
        description: "Begin order fulfillment process",
      },
      {
        id: "ship_order",
        name: "Ship Order",
        fromStates: ["fulfillment_started"],
        toState: "shipped",
        conditions: ["data.tracking_number != null", "data.carrier != null"],
        description: "Ship order to customer",
      },
      {
        id: "deliver_order",
        name: "Deliver Order",
        fromStates: ["shipped"],
        toState: "delivered",
        conditions: ["data.delivery_confirmed == true"],
        description: "Confirm order delivery",
      },
      {
        id: "complete_order",
        name: "Complete Order",
        fromStates: ["delivered"],
        toState: "completed",
        conditions: [],
        description: "Mark order as completed",
      },
      // Cancellation paths
      {
        id: "cancel_order",
        name: "Cancel Order",
        fromStates: ["cart_created", "checkout_started", "payment_processing"],
        toState: "cancelled",
        conditions: [],
        description: "Cancel order before confirmation",
      },
      {
        id: "refund_order",
        name: "Refund Order",
        fromStates: [
          "payment_confirmed",
          "inventory_reserved",
          "order_confirmed",
          "fulfillment_started",
        ],
        toState: "refunded",
        conditions: ["data.refund_approved == true"],
        description: "Process order refund",
      },
    ],
    initialState: "cart_created",
    metadata: {
      department: "ecommerce",
      owner: "ecommerce-team",
      sla_hours: 48,
      priority: "high",
      version_notes: "Added refund support and improved error handling",
    },
  };
}

function createApprovalWorkflow(): WorkflowCreateInput {
  return {
    name: "Document Approval System",
    description:
      "Multi-level document approval workflow for corporate documents",
    version: "1.3.0",
    tags: ["approval", "documents", "compliance"],
    states: [
      "draft",
      "submitted",
      "manager_review",
      "director_review",
      "legal_review",
      "approved",
      "rejected",
      "revision_required",
    ],
    activities: [
      {
        id: "submit_document",
        name: "Submit for Review",
        fromStates: ["draft"],
        toState: "submitted",
        conditions: [
          "data.document_complete == true",
          "data.submitted_by != null",
        ],
        description: "Submit document for approval process",
      },
      {
        id: "manager_review",
        name: "Manager Review",
        fromStates: ["submitted"],
        toState: "manager_review",
        conditions: ["data.manager_assigned != null"],
        description: "Assign to manager for initial review",
      },
      {
        id: "escalate_to_director",
        name: "Escalate to Director",
        fromStates: ["manager_review"],
        toState: "director_review",
        conditions: ["data.requires_director_approval == true"],
        description: "Escalate to director for high-value approvals",
      },
      {
        id: "legal_review_required",
        name: "Legal Review Required",
        fromStates: ["director_review"],
        toState: "legal_review",
        conditions: ["data.requires_legal_review == true"],
        description: "Send to legal team for compliance review",
      },
      {
        id: "approve_document",
        name: "Approve Document",
        fromStates: ["manager_review", "director_review", "legal_review"],
        toState: "approved",
        conditions: ["data.approval_granted == true"],
        description: "Grant final approval",
      },
      {
        id: "reject_document",
        name: "Reject Document",
        fromStates: ["manager_review", "director_review", "legal_review"],
        toState: "rejected",
        conditions: ["data.rejection_reason != null"],
        description: "Reject document with reason",
      },
      {
        id: "request_revision",
        name: "Request Revision",
        fromStates: ["manager_review", "director_review", "legal_review"],
        toState: "revision_required",
        conditions: ["data.revision_notes != null"],
        description: "Request document revision",
      },
      {
        id: "resubmit_revised",
        name: "Resubmit Revised Document",
        fromStates: ["revision_required"],
        toState: "submitted",
        conditions: ["data.revisions_completed == true"],
        description: "Resubmit after making requested revisions",
      },
    ],
    initialState: "draft",
    metadata: {
      department: "operations",
      owner: "compliance-team",
      sla_hours: 72,
      priority: "medium",
      compliance_required: true,
    },
  };
}

function createITTicketWorkflow(): WorkflowCreateInput {
  return {
    name: "IT Support Ticket System",
    description: "Automated IT support ticket routing and resolution workflow",
    version: "3.0.0",
    tags: ["it-support", "tickets", "automation"],
    states: [
      "created",
      "triaged",
      "assigned",
      "in_progress",
      "pending_user",
      "escalated",
      "resolved",
      "closed",
      "reopened",
    ],
    activities: [
      {
        id: "auto_triage",
        name: "Auto Triage Ticket",
        fromStates: ["created"],
        toState: "triaged",
        conditions: ["data.priority != null", "data.category != null"],
        description:
          "Automatically triage ticket based on priority and category",
      },
      {
        id: "assign_to_tech",
        name: "Assign to Technician",
        fromStates: ["triaged"],
        toState: "assigned",
        conditions: ["data.assigned_to != null"],
        description: "Assign ticket to available technician",
      },
      {
        id: "start_work",
        name: "Start Working on Ticket",
        fromStates: ["assigned"],
        toState: "in_progress",
        conditions: [],
        description: "Technician starts working on the ticket",
      },
      {
        id: "wait_for_user",
        name: "Wait for User Response",
        fromStates: ["in_progress"],
        toState: "pending_user",
        conditions: ["data.user_input_required == true"],
        description: "Waiting for user to provide additional information",
      },
      {
        id: "resume_work",
        name: "Resume Work",
        fromStates: ["pending_user"],
        toState: "in_progress",
        conditions: ["data.user_responded == true"],
        description: "Resume work after receiving user input",
      },
      {
        id: "escalate_ticket",
        name: "Escalate Ticket",
        fromStates: ["in_progress"],
        toState: "escalated",
        conditions: ["data.escalation_required == true"],
        description: "Escalate to senior technician or specialist",
      },
      {
        id: "resolve_ticket",
        name: "Resolve Ticket",
        fromStates: ["in_progress", "escalated"],
        toState: "resolved",
        conditions: ["data.resolution_provided == true"],
        description: "Mark ticket as resolved",
      },
      {
        id: "close_ticket",
        name: "Close Ticket",
        fromStates: ["resolved"],
        toState: "closed",
        conditions: ["data.user_confirmed == true"],
        description: "Close ticket after user confirmation",
      },
      {
        id: "reopen_ticket",
        name: "Reopen Ticket",
        fromStates: ["closed"],
        toState: "reopened",
        conditions: ["data.reopen_reason != null"],
        description: "Reopen closed ticket",
      },
      {
        id: "reassign_reopened",
        name: "Reassign Reopened Ticket",
        fromStates: ["reopened"],
        toState: "assigned",
        conditions: [],
        description: "Reassign reopened ticket for investigation",
      },
    ],
    initialState: "created",
    metadata: {
      department: "it",
      owner: "it-support-team",
      sla_hours: 24,
      priority: "high",
      auto_assignment: true,
      escalation_rules: {
        critical: "2h",
        high: "4h",
        medium: "24h",
        low: "72h",
      },
    },
  };
}

// ============================================================================
// Main Example Function
// ============================================================================

async function runWorkflowManagementExample(): Promise<void> {
  console.log("🚀 Circuit Breaker SDK - Workflow Management Example");
  console.log("====================================================");
  console.log();

  let sdk: CircuitBreakerSDK;

  try {
    // Initialize SDK
    logInfo("Initializing Circuit Breaker SDK...");
    sdk = new CircuitBreakerSDK(config);

    const initResult = await sdk.initialize();
    if (!initResult.success) {
      throw new Error(
        `SDK initialization failed: ${initResult.errors.join(", ")}`,
      );
    }

    logSuccess("SDK initialized successfully");
    console.log();

    // ========================================================================
    // Part 1: Create Multiple Workflows
    // ========================================================================

    logInfo("📋 Creating Multiple Workflows...");

    const workflowDefinitions = [
      createEcommerceWorkflow(),
      createApprovalWorkflow(),
      createITTicketWorkflow(),
    ];

    const createdWorkflowIds: string[] = [];

    for (const workflowDef of workflowDefinitions) {
      try {
        logInfo(`Creating workflow: ${workflowDef.name}`);

        const workflowId = await sdk.workflows.create(workflowDef);
        createdWorkflowIds.push(workflowId);

        logSuccess(`✓ Created: ${workflowDef.name} (${workflowId})`);
        logInfo(
          `  States: ${workflowDef.states.length}, Activities: ${workflowDef.activities.length}`,
        );
        logInfo(`  Tags: ${workflowDef.tags?.join(", ") || "none"}`);
        logInfo(`  Version: ${workflowDef.version}`);
      } catch (error) {
        logError(`Failed to create workflow: ${workflowDef.name}`, error);

        if (error instanceof WorkflowValidationError) {
          console.log("   Validation errors:");
          error.validationErrors.forEach((err) => console.log(`     • ${err}`));
        }
      }
    }

    console.log();
    logSuccess(`Created ${createdWorkflowIds.length} workflows successfully`);
    console.log();

    // ========================================================================
    // Part 2: Search and Filter Workflows
    // ========================================================================

    logInfo("🔍 Searching and Filtering Workflows...");

    // Search by query
    logInfo('Searching for "order" workflows...');
    const orderWorkflows = await sdk.workflows.search("order", {
      limit: 10,
      includeStats: true,
    });

    logSuccess(`Found ${orderWorkflows.length} workflows containing "order"`);
    orderWorkflows.forEach((wf) => {
      console.log(
        `  • ${wf.name} (${wf.id.substring(0, 8)}...) - v${wf.version}`,
      );
    });

    // Filter by tags
    logInfo('Filtering by tags: ["ecommerce", "orders"]...');
    const ecommerceWorkflows = await sdk.workflows.list({
      tags: ["ecommerce", "orders"],
      includeStats: true,
      includeActivities: true,
    });

    logSuccess(`Found ${ecommerceWorkflows.data.length} e-commerce workflows`);
    ecommerceWorkflows.data.forEach((wf) => {
      console.log(`  • ${wf.name} - ${wf.tags?.join(", ")}`);
      if (wf.stats) {
        console.log(
          `    Resources: ${wf.stats.totalResources}, Active: ${wf.stats.activeResources}`,
        );
      }
    });

    // List all workflows with pagination
    logInfo("Listing all workflows with pagination...");
    const allWorkflows = await sdk.workflows.list({
      limit: 5,
      offset: 0,
      sortBy: "createdAt",
      sortOrder: "desc",
      includeStats: false,
    });

    logSuccess(
      `Listed ${allWorkflows.data.length} of ${allWorkflows.total} total workflows`,
    );
    allWorkflows.data.forEach((wf, index) => {
      console.log(
        `  ${index + 1}. ${wf.name} (created: ${new Date(wf.createdAt).toLocaleDateString()})`,
      );
    });

    console.log();

    // ========================================================================
    // Part 3: Get and Update Workflow Details
    // ========================================================================

    logInfo("📝 Getting and Updating Workflow Details...");

    if (createdWorkflowIds.length > 0) {
      const workflowId = createdWorkflowIds[0];

      // Get workflow details
      logInfo(`Getting details for workflow: ${workflowId}`);
      const workflow = await sdk.workflows.get(workflowId, true);

      logSuccess("Retrieved workflow details");
      console.log(`  Name: ${workflow.name}`);
      console.log(`  Version: ${workflow.version}`);
      console.log(`  States: ${workflow.states.length}`);
      console.log(`  Activities: ${workflow.activities.length}`);
      console.log(
        `  Created: ${new Date(workflow.createdAt).toLocaleString()}`,
      );

      if (workflow.stats) {
        console.log(`  Statistics:`);
        console.log(`    Total Resources: ${workflow.stats.totalResources}`);
        console.log(`    Active Resources: ${workflow.stats.activeResources}`);
        console.log(
          `    Completed Resources: ${workflow.stats.completedResources}`,
        );
      }

      // Update workflow metadata
      logInfo("Updating workflow metadata...");
      const updates: WorkflowUpdateInput = {
        version: "2.1.1",
        description:
          workflow.description + " (Updated with monitoring improvements)",
        metadata: {
          ...workflow.metadata,
          last_updated_by: "sdk-example",
          update_reason: "Added monitoring and performance improvements",
          monitoring_enabled: true,
        },
      };

      const updatedWorkflow = await sdk.workflows.update(workflowId, updates);
      logSuccess("Workflow updated successfully");
      console.log(`  New version: ${updatedWorkflow.version}`);
      console.log(`  Updated description: ${updatedWorkflow.description}`);
    }

    console.log();

    // ========================================================================
    // Part 4: Workflow Validation
    // ========================================================================

    logInfo("✅ Validating Workflow Definitions...");

    // Validate a complex workflow
    const complexWorkflow = createWorkflow("Complex Validation Test")
      .addStates(["start", "middle1", "middle2", "middle3", "end", "error"])
      .setInitialState("start")
      .addTransition("start", "middle1", "step1")
      .addTransition("middle1", "middle2", "step2")
      .addTransition("middle2", "middle3", "step3")
      .addTransition("middle3", "end", "finish")
      .addTransition("middle1", "error", "handle_error")
      .addTransition("middle2", "error", "handle_error")
      .addSimpleRule("step1", "input_valid", "==", true)
      .addSimpleRule("step2", "processing_complete", "==", true)
      .addSimpleRule("step3", "final_check", "==", true)
      .setDescription("Complex workflow for validation testing")
      .setVersion("1.0.0")
      .addTags(["test", "validation", "complex"])
      .build();

    const validationOptions: WorkflowValidationOptions = {
      checkReachability: true,
      validateRules: true,
      checkLoops: true,
      validateReferences: true,
      includeWarnings: true,
    };

    logInfo("Validating complex workflow definition...");
    const validationReport = await sdk.workflows.validate(
      complexWorkflow,
      validationOptions,
    );

    logSuccess(`Validation completed - Valid: ${validationReport.valid}`);
    console.log(`  Complexity: ${validationReport.complexity}`);
    console.log(
      `  Estimated execution paths: ${validationReport.estimatedExecutionPaths}`,
    );
    console.log(
      `  Unreachable states: ${validationReport.unreachableStates.length}`,
    );
    console.log(
      `  Terminal states: ${validationReport.terminalStates.join(", ")}`,
    );

    if (validationReport.errors.length > 0) {
      console.log("  Validation errors:");
      validationReport.errors.forEach((error) => console.log(`    • ${error}`));
    }

    if (validationReport.warnings.length > 0) {
      console.log("  Warnings:");
      validationReport.warnings.forEach((warning) =>
        console.log(`    • ${warning}`),
      );
    }

    if (validationReport.suggestions.length > 0) {
      console.log("  Suggestions:");
      validationReport.suggestions.forEach((suggestion) =>
        console.log(`    • ${suggestion}`),
      );
    }

    if (validationReport.potentialBottlenecks.length > 0) {
      console.log("  Potential bottlenecks:");
      validationReport.potentialBottlenecks.forEach((bottleneck) =>
        console.log(`    • ${bottleneck}`),
      );
    }

    console.log();

    // ========================================================================
    // Part 5: Workflow Statistics and Health Monitoring
    // ========================================================================

    logInfo("📊 Monitoring Workflow Statistics and Health...");

    for (const workflowId of createdWorkflowIds.slice(0, 2)) {
      try {
        logInfo(
          `Getting statistics for workflow: ${workflowId.substring(0, 8)}...`,
        );

        // Get detailed statistics
        const stats = await sdk.workflows.getStats(workflowId);

        console.log(`  Total Resources: ${stats.totalResources}`);
        console.log(`  Active Resources: ${stats.activeResources}`);
        console.log(`  Completed Resources: ${stats.completedResources}`);
        console.log(`  Failed Resources: ${stats.failedResources}`);

        if (stats.averageExecutionTime) {
          console.log(
            `  Average Execution Time: ${(stats.averageExecutionTime / 1000).toFixed(2)}s`,
          );
        }

        if (stats.lastExecution) {
          console.log(
            `  Last Execution: ${new Date(stats.lastExecution).toLocaleString()}`,
          );
        }

        // Show state distribution
        if (Object.keys(stats.stateDistribution).length > 0) {
          console.log("  State Distribution:");
          Object.entries(stats.stateDistribution).forEach(([state, count]) => {
            console.log(`    ${state}: ${count}`);
          });
        }

        // Show activity statistics
        if (Object.keys(stats.activityStats).length > 0) {
          console.log("  Top Activities:");
          Object.entries(stats.activityStats)
            .slice(0, 3)
            .forEach(([activity, activityStats]) => {
              console.log(
                `    ${activity}: ${activityStats.executions} executions, ${(activityStats.successRate * 100).toFixed(1)}% success`,
              );
            });
        }

        // Get health status
        const health = await sdk.workflows.getHealth(workflowId);

        console.log(
          `  Health Status: ${health.healthy ? "✅ Healthy" : "❌ Unhealthy"}`,
        );
        if (health.issues.length > 0) {
          console.log("  Health Issues:");
          health.issues.forEach((issue) => console.log(`    • ${issue}`));
        }

        console.log(`  Error Rate: ${(health.errorRate * 100).toFixed(2)}%`);
        console.log(
          `  Avg Execution Time: ${(health.avgExecutionTime / 1000).toFixed(2)}s`,
        );
        console.log();
      } catch (error) {
        if (error instanceof WorkflowNotFoundError) {
          logWarning(`Workflow not found: ${workflowId}`);
        } else {
          logError(
            `Failed to get statistics for workflow: ${workflowId}`,
            error,
          );
        }
      }
    }

    // ========================================================================
    // Part 6: Advanced Workflow Patterns
    // ========================================================================

    logInfo("🔧 Creating Advanced Workflow Patterns...");

    // Create a linear workflow
    logInfo("Creating linear workflow...");
    const linearWorkflow = createLinearWorkflow(
      "Linear Process Example",
      ["initialize", "validate", "process", "complete"],
      ["init", "validate", "process"],
    );

    const linearDef = linearWorkflow
      .setDescription("Simple linear workflow for batch processing")
      .addTags(["linear", "batch", "example"])
      .build();

    const linearWorkflowId = await sdk.workflows.create(linearDef);
    logSuccess(`Created linear workflow: ${linearWorkflowId}`);

    // Create a state machine workflow
    logInfo("Creating state machine workflow...");
    const stateMachineWorkflow = createFromStateMachine({
      name: "State Machine Example",
      states: ["idle", "running", "paused", "stopped", "error"],
      transitions: [
        { from: "idle", to: "running", event: "start" },
        { from: "running", to: "paused", event: "pause" },
        { from: "paused", to: "running", event: "resume" },
        { from: "running", to: "stopped", event: "stop" },
        { from: "paused", to: "stopped", event: "stop" },
        {
          from: "running",
          to: "error",
          event: "error",
          condition: "data.error_occurred == true",
        },
        { from: "error", to: "idle", event: "reset" },
      ],
      initialState: "idle",
    });

    const stateMachineDef = stateMachineWorkflow
      .setDescription("State machine pattern for process control")
      .addTags(["state-machine", "control", "example"])
      .build();

    const stateMachineWorkflowId = await sdk.workflows.create(stateMachineDef);
    logSuccess(`Created state machine workflow: ${stateMachineWorkflowId}`);

    console.log();

    // ========================================================================
    // Part 7: Workflow Manager Health and Statistics
    // ========================================================================

    logInfo("🏥 Checking Workflow Manager Health...");

    const managerHealth = await sdk.workflows.getManagerHealth();
    logSuccess(
      `Workflow Manager Health: ${managerHealth.healthy ? "✅ Healthy" : "❌ Unhealthy"}`,
    );
    console.log(`  Cache Size: ${managerHealth.cacheSize} workflows`);
    if (managerHealth.lastActivity) {
      console.log(
        `  Last Activity: ${managerHealth.lastActivity.toLocaleString()}`,
      );
    }

    const managerStats = sdk.workflows.getStats();
    console.log(`  Cache Hits: ${managerStats.cacheHits}`);
    console.log(`  Cache Misses: ${managerStats.cacheMisses}`);
    console.log(`  Operations Count: ${managerStats.operationsCount}`);

    console.log();

    // ========================================================================
    // Summary and Cleanup
    // ========================================================================

    logSuccess("🎉 Workflow Management Example Completed!");
    console.log();

    console.log("📋 Operations performed:");
    console.log(`  ✅ Created ${createdWorkflowIds.length + 2} workflows`);
    console.log("  ✅ Searched and filtered workflows");
    console.log("  ✅ Updated workflow metadata");
    console.log("  ✅ Validated workflow definitions");
    console.log("  ✅ Monitored workflow statistics and health");
    console.log("  ✅ Created advanced workflow patterns");
    console.log("  ✅ Checked manager health and performance");
    console.log();

    console.log("📊 Created workflow types:");
    console.log("  • E-commerce Order Processing (complex business workflow)");
    console.log("  • Document Approval System (multi-level approval)");
    console.log("  • IT Support Ticket System (automated routing)");
    console.log("  • Linear Process Example (simple sequential)");
    console.log("  • State Machine Example (control flow)");
    console.log();

    console.log("🚀 Key features demonstrated:");
    console.log("  • Comprehensive workflow CRUD operations");
    console.log("  • Advanced search and filtering capabilities");
    console.log("  • Real-time validation with detailed reports");
    console.log("  • Statistics and health monitoring");
    console.log("  • Multiple workflow pattern support");
    console.log("  • Performance monitoring and caching");
    console.log("  • Error handling and recovery");
    console.log();

    console.log("💡 Next steps:");
    console.log("  • Create resources and execute workflows");
    console.log("  • Implement custom rules and validation logic");
    console.log("  • Set up real-time monitoring and alerts");
    console.log("  • Integrate with external systems and APIs");
    console.log("  • Build workflow analytics dashboards");

    // Cleanup: In a real scenario, you might want to delete test workflows
    if (process.env.CLEANUP_TEST_WORKFLOWS === "true") {
      logInfo("🧹 Cleaning up test workflows...");
      for (const workflowId of [
        ...createdWorkflowIds,
        linearWorkflowId,
        stateMachineWorkflowId,
      ]) {
        try {
          const deleted = await sdk.workflows.delete(workflowId);
          if (deleted) {
            logSuccess(`Deleted workflow: ${workflowId.substring(0, 8)}...`);
          }
        } catch (error) {
          logWarning(`Failed to delete workflow: ${workflowId}`, error);
        }
      }
    }
  } catch (error) {
    logError("Example execution failed", error);

    if (error instanceof CircuitBreakerError) {
      console.log();
      console.log("💡 Troubleshooting tips:");

      if (error.code.includes("NETWORK") || error.code.includes("CONNECTION")) {
        console.log(
          "  • Ensure Circuit Breaker server is running on the configured endpoint",
        );
        console.log("  • Check network connectivity and firewall settings");
        console.log("  • Verify the GraphQL endpoint URL is correct");
      }

      if (error.code.includes("VALIDATION")) {
        console.log("  • Review workflow definition for structural issues");
        console.log("  • Check that all required fields are provided");
        console.log("  • Ensure state and activity names are valid");
        console.log(
          "  • Verify that fromStates and toStates reference existing states",
        );
      }

      if (error.code.includes("WORKFLOW")) {
        console.log("  • Check workflow permissions and access rights");
        console.log("  • Verify workflow exists and is accessible");
        console.log("  • Review workflow definition syntax");
      }

      if (error.code.includes("TIMEOUT")) {
        console.log("  • Increase timeout configuration");
        console.log("  • Check server performance and load");
        console.log(
          "  • Consider breaking large operations into smaller chunks",
        );
      }
    }

    process.exit(1);
  } finally {
    // Cleanup SDK resources
    if (sdk!) {
      try {
        await sdk.dispose();
        logInfo("SDK resources disposed successfully");
      } catch (error) {
        logWarning("Error during SDK cleanup", error);
      }
    }
  }
}

// ============================================================================
// Main Execution
// ============================================================================

if (import.meta.url === `file://${process.argv[1]}`) {
  runWorkflowManagementExample().catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
  });
}

export { runWorkflowManagementExample };
