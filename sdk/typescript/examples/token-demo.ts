#!/usr/bin/env tsx
/**
 * Resource Management Demo - Circuit Breaker TypeScript SDK
 *
 * This example demonstrates comprehensive resource management capabilities:
 * - Creating and managing resources (tokens)
 * - State transitions and lifecycle management
 * - Resource data manipulation and updates
 * - Batch operations and bulk processing
 * - Resource search and filtering
 * - Resource history and auditing
 *
 * Run with: npx tsx examples/token-demo.ts
 */

/// <reference types="node" />

import {
  CircuitBreakerSDK,
  createWorkflow,
  createResourceBuilder,
  createResourceTemplate,
  ResourceManager,
  ResourceBuilder,
  WorkflowDefinition,
  Resource,
  ResourceCreateInput,
  ResourceUpdateInput,
  StateTransitionInput,
  BatchOperationOptions,
  CircuitBreakerError,
  ResourceError,
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
  headers: {
    "User-Agent": "CircuitBreaker-SDK-TokenDemo/0.1.0",
  },
};

// ============================================================================
// Helper Functions
// ============================================================================

function logSuccess(message: string, data?: any): void {
  console.log(`✅ ${message}`);
  if (data && config.debug) {
    console.log(JSON.stringify(data, null, 2));
  }
}

function logInfo(message: string, data?: any): void {
  console.log(`ℹ️  ${message}`);
  if (data && config.debug) {
    console.log(JSON.stringify(data, null, 2));
  }
}

function logError(message: string, error?: any): void {
  console.error(`❌ ${message}`);
  if (error) {
    if (error instanceof CircuitBreakerError) {
      console.error(`   Error: ${formatError(error)}`);
      if (error.context && config.debug) {
        console.error(`   Context: ${JSON.stringify(error.context, null, 2)}`);
      }
    } else {
      console.error(`   ${error.message || error}`);
      if (config.debug && error.stack) {
        console.error(`   Stack: ${error.stack}`);
      }
    }
  }
}

function logWarning(message: string, data?: any): void {
  console.warn(`⚠️  ${message}`);
  if (data && config.debug) {
    console.warn(JSON.stringify(data, null, 2));
  }
}

// ============================================================================
// Workflow Definitions
// ============================================================================

function createContentCreationWorkflow(): WorkflowDefinition {
  return createWorkflow("AI-Powered Content Creation")
    .addState("ideation")
    .addState("drafting")
    .addState("review")
    .addState("revision")
    .addState("approval")
    .addState("published")
    .addState("archived")
    .addTransition("ideation", "drafting", "start_draft")
    .addTransition("drafting", "review", "submit_for_review")
    .addTransition("review", "revision", "request_revision")
    .addTransition("review", "approval", "approve_content")
    .addTransition("revision", "review", "resubmit")
    .addTransition("approval", "published", "publish")
    .addTransition("published", "archived", "archive")
    .setInitialState("ideation")
    .build();
}

function createOrderProcessingWorkflow(): WorkflowDefinition {
  return createWorkflow("E-commerce Order Processing")
    .addState("cart")
    .addState("checkout")
    .addState("payment_pending")
    .addState("payment_confirmed")
    .addState("fulfillment")
    .addState("shipped")
    .addState("delivered")
    .addState("cancelled")
    .addState("refunded")
    .addTransition("cart", "checkout", "proceed_to_checkout")
    .addTransition("checkout", "payment_pending", "submit_payment")
    .addTransition("payment_pending", "payment_confirmed", "confirm_payment")
    .addTransition("payment_pending", "cancelled", "cancel_order")
    .addTransition("payment_confirmed", "fulfillment", "start_fulfillment")
    .addTransition("fulfillment", "shipped", "ship_order")
    .addTransition("shipped", "delivered", "confirm_delivery")
    .addTransition("payment_confirmed", "refunded", "process_refund")
    .setInitialState("cart")
    .build();
}

function createUserOnboardingWorkflow(): WorkflowDefinition {
  return createWorkflow("User Onboarding Process")
    .addState("registration")
    .addState("email_verification")
    .addState("profile_setup")
    .addState("document_upload")
    .addState("verification_pending")
    .addState("verification_complete")
    .addState("onboarding_complete")
    .addState("suspended")
    .addTransition(
      "registration",
      "email_verification",
      "send_verification_email",
    )
    .addTransition("email_verification", "profile_setup", "verify_email")
    .addTransition("profile_setup", "document_upload", "complete_profile")
    .addTransition(
      "document_upload",
      "verification_pending",
      "submit_documents",
    )
    .addTransition(
      "verification_pending",
      "verification_complete",
      "approve_verification",
    )
    .addTransition("verification_pending", "suspended", "reject_verification")
    .addTransition(
      "verification_complete",
      "onboarding_complete",
      "complete_onboarding",
    )
    .setInitialState("registration")
    .build();
}

// ============================================================================
// Sample Data Generators
// ============================================================================

function generateContentResourceData() {
  const topics = [
    "AI",
    "Technology",
    "Business",
    "Health",
    "Science",
    "Travel",
  ];
  const types = ["blog_post", "article", "tutorial", "news"];
  const priorities = ["low", "medium", "high", "urgent"];

  return {
    title: `Sample ${topics[Math.floor(Math.random() * topics.length)]} Content`,
    type: types[Math.floor(Math.random() * types.length)],
    priority: priorities[Math.floor(Math.random() * priorities.length)],
    targetLength: Math.floor(Math.random() * 2000) + 500,
    author: `author-${Math.floor(Math.random() * 10) + 1}`,
    deadline: new Date(
      Date.now() + Math.random() * 30 * 24 * 60 * 60 * 1000,
    ).toISOString(),
    keywords: [
      `keyword${Math.floor(Math.random() * 100)}`,
      `tag${Math.floor(Math.random() * 50)}`,
    ],
    audience: ["general", "technical", "business"][
      Math.floor(Math.random() * 3)
    ],
  };
}

function generateOrderResourceData() {
  const products = [
    "Laptop",
    "Phone",
    "Tablet",
    "Headphones",
    "Keyboard",
    "Mouse",
  ];
  const customers = ["customer1", "customer2", "customer3", "customer4"];

  const items = Array.from(
    { length: Math.floor(Math.random() * 5) + 1 },
    () => ({
      productId: `prod-${Math.floor(Math.random() * 1000)}`,
      name: products[Math.floor(Math.random() * products.length)],
      price: parseFloat((Math.random() * 1000 + 50).toFixed(2)),
      quantity: Math.floor(Math.random() * 5) + 1,
    }),
  );

  const subtotal = items.reduce(
    (sum, item) => sum + item.price * item.quantity,
    0,
  );
  const tax = subtotal * 0.08;
  const shipping = subtotal > 500 ? 0 : 19.99;
  const total = subtotal + tax + shipping;

  return {
    orderId: `ORD-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
    customerId: customers[Math.floor(Math.random() * customers.length)],
    items,
    pricing: {
      subtotal: parseFloat(subtotal.toFixed(2)),
      tax: parseFloat(tax.toFixed(2)),
      shipping: parseFloat(shipping.toFixed(2)),
      total: parseFloat(total.toFixed(2)),
    },
    shippingAddress: {
      street: "123 Main St",
      city: "Anytown",
      state: "CA",
      zipCode: "12345",
      country: "US",
    },
    paymentMethod: "credit_card",
    estimatedDelivery: new Date(
      Date.now() + Math.random() * 10 * 24 * 60 * 60 * 1000,
    ).toISOString(),
  };
}

function generateUserResourceData() {
  const firstNames = ["John", "Jane", "Alice", "Bob", "Charlie", "Diana"];
  const lastNames = ["Smith", "Doe", "Johnson", "Brown", "Davis", "Wilson"];
  const domains = ["example.com", "test.com", "demo.org"];

  const firstName = firstNames[Math.floor(Math.random() * firstNames.length)];
  const lastName = lastNames[Math.floor(Math.random() * lastNames.length)];

  return {
    userId: `user-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
    email: `${firstName!.toLowerCase()}.${lastName!.toLowerCase()}@${domains[Math.floor(Math.random() * domains.length)]}`,
    profile: {
      firstName,
      lastName,
      dateOfBirth: new Date(
        Date.now() - Math.random() * 50 * 365 * 24 * 60 * 60 * 1000,
      )
        .toISOString()
        .split("T")[0],
      phone: `+1${Math.floor(Math.random() * 9000000000) + 1000000000}`,
      address: {
        street: "456 Oak Ave",
        city: "Somewhere",
        state: "NY",
        zipCode: "67890",
        country: "US",
      },
    },
    preferences: {
      language: "en",
      timezone: "America/New_York",
      notifications: {
        email: true,
        sms: Math.random() > 0.5,
        push: true,
      },
    },
    accountType: ["standard", "premium", "business"][
      Math.floor(Math.random() * 3)
    ],
  };
}

// ============================================================================
// Demo Functions
// ============================================================================

async function demonstrateBasicResourceOperations(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🔧 Basic Resource Operations");
  console.log("=".repeat(50));

  // Create content workflow
  const contentWorkflow = createContentCreationWorkflow();
  const contentWorkflowId = await sdk.workflows.create(contentWorkflow);
  logSuccess(`Created content workflow: ${contentWorkflowId}`);

  // Create a single resource
  const contentData = generateContentResourceData();
  const resourceInput: ResourceCreateInput = {
    workflowId: contentWorkflowId,
    data: contentData,
    metadata: {
      createdBy: "demo-system",
      source: "automated",
      priority: contentData.priority,
    },
  };

  logInfo("Creating content resource...");
  const resourceId = await sdk.resources.create(resourceInput);
  logSuccess(`Resource created with ID: ${resourceId}`);

  // Get resource details
  const resource = await sdk.resources.get(resourceId);
  logInfo("Resource details:", {
    id: resource.id,
    state: resource.state,
    title: resource.data.title,
    type: resource.data.type,
    createdAt: resource.createdAt,
  });

  // Update resource data
  const updateInput: ResourceUpdateInput = {
    data: {
      ...resource.data,
      lastModified: new Date().toISOString(),
      wordCount: Math.floor(Math.random() * 1000) + 200,
    },
    metadata: {
      ...resource.metadata,
      lastUpdateBy: "demo-system",
    },
  };

  await sdk.resources.update(resourceId, updateInput);
  logSuccess("Resource updated successfully");

  // Perform state transition
  const transitionInput: StateTransitionInput = {
    resourceId,
    toState: "drafting",
    activityId: "start_draft",
    data: {
      draftStarted: new Date().toISOString(),
      assignedWriter: resource.data.author,
    },
  };

  logInfo("Transitioning resource state...");
  const transitionResult =
    await sdk.resources.executeTransition(transitionInput);
  logSuccess(`Resource transitioned to state: ${transitionResult.newState}`);

  return;
}

async function demonstrateBatchOperations(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n📦 Batch Resource Operations");
  console.log("=".repeat(50));

  // Create order processing workflow
  const orderWorkflow = createOrderProcessingWorkflow();
  const orderWorkflowId = await sdk.workflows.create(orderWorkflow);
  logSuccess(`Created order workflow: ${orderWorkflowId}`);

  // Create multiple resources in batch
  const batchSize = 5;
  const batchResources: ResourceCreateInput[] = Array.from(
    { length: batchSize },
    () => ({
      workflowId: orderWorkflowId,
      data: generateOrderResourceData(),
      metadata: {
        createdBy: "batch-process",
        batchId: generateRequestId(),
      },
    }),
  );

  logInfo(`Creating batch of ${batchSize} order resources...`);
  const batchOptions: BatchOperationOptions = {
    concurrency: 3,
  };

  const batchResult = await sdk.resources.createBatch(
    batchResources,
    batchOptions,
  );
  logSuccess(`Batch operation completed:`, {
    successful: batchResult.successful.length,
    failed: batchResult.failed.length,
    totalProcessed: batchResult.totalProcessed,
  });

  // Batch state transitions
  if (batchResult.successful.length > 0) {
    const transitionInputs = batchResult.successful
      .slice(0, 3)
      .map((resource: any) => ({
        resourceId: resource.id,
        toState: "checkout",
        activityId: "proceed_to_checkout",
        data: {
          checkoutStarted: new Date().toISOString(),
        },
      }));

    logInfo("Performing batch state transitions...");
    const batchTransitionResult = await sdk.resources.executeBatchTransitions(
      transitionInputs,
      batchOptions,
    );
    logSuccess(`Batch transitions completed:`, {
      successful: batchTransitionResult.successful.length,
      failed: batchTransitionResult.failed.length,
    });
  }

  return;
}

async function demonstrateResourceBuilder(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🏗️  Resource Builder Operations");
  console.log("=".repeat(50));

  // Create user onboarding workflow
  const userWorkflow = createUserOnboardingWorkflow();
  const userWorkflowId = await sdk.workflows.create(userWorkflow);
  logSuccess(`Created user workflow: ${userWorkflowId}`);

  // Create resource manually since builder API differs
  const userData = generateUserResourceData();
  const builtResource: ResourceCreateInput = {
    workflowId: userWorkflowId,
    data: userData,
    metadata: {
      registrationSource: "web",
      userAgent: "Mozilla/5.0...",
      ipAddress: "192.168.1.1",
      tags: ["new-user", "web-registration"],
      validation: {
        requireEmail: true,
        validateAge: true,
        checkDuplicates: true,
      },
    },
  };

  const builtResourceId = await sdk.resources.create(builtResource);
  logSuccess(`Resource built and created: ${builtResourceId}`);

  // Create a resource chain manually
  const chainedResources: ResourceCreateInput[] = [
    {
      workflowId: userWorkflowId,
      data: { ...userData, step: "profile_completion" },
      metadata: { chainStep: 1 },
    },
    {
      workflowId: userWorkflowId,
      data: { ...userData, step: "document_verification" },
      metadata: { chainStep: 2 },
    },
    {
      workflowId: userWorkflowId,
      data: { ...userData, step: "final_approval" },
      metadata: { chainStep: 3 },
    },
  ];

  logInfo(
    `Creating resource chain with ${chainedResources.length} resources...`,
  );
  const chainResult = await sdk.resources.createBatch(chainedResources);
  logSuccess(`Resource chain created:`, {
    successful: chainResult.successful.length,
    failed: chainResult.failed.length,
  });

  return;
}

async function demonstrateResourceSearch(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🔍 Resource Search and Filtering");
  console.log("=".repeat(50));

  // Search resources by workflow
  const allWorkflows = await sdk.workflows.list();
  if (allWorkflows.length === 0) {
    logWarning("No workflows available for resource search");
    return;
  }

  const workflowId = allWorkflows[0].id;
  logInfo(`Searching resources for workflow: ${workflowId}`);

  // Basic search
  const basicSearch = await sdk.resources.search({
    workflowId,
    limit: 10,
  });
  logInfo(`Found ${basicSearch.length} resources in workflow`);

  // Advanced search with filters
  const advancedSearch = await sdk.resources.search({
    workflowId,
    states: ["ideation", "drafting", "review"],
    metadata: {
      createdBy: "demo-system",
    },
    dateRange: {
      start: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(), // Last 24 hours
      end: new Date().toISOString(),
    },
    limit: 20,
  });
  logInfo(`Advanced search found ${advancedSearch.length} resources`);

  // Search with pagination
  const paginatedSearch = await sdk.resources.search({
    workflowId,
    limit: 5,
  });
  logInfo(`Paginated search (page 1): ${paginatedSearch.length} resources`);

  if (paginatedSearch.length > 0) {
    const sampleResource = paginatedSearch[0];
    logInfo("Sample resource from search:", {
      id: sampleResource.id,
      state: sampleResource.state,
      createdAt: sampleResource.createdAt,
      dataKeys: Object.keys(sampleResource.data),
    });
  }

  return;
}

async function demonstrateResourceHistory(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n📜 Resource History and Auditing");
  console.log("=".repeat(50));

  // Find resources with history
  const allResources = await sdk.resources.search({ limit: 10 });
  if (allResources.length === 0) {
    logWarning("No resources available for history demonstration");
    return;
  }

  const resourceWithHistory = allResources.find(
    (r: any) => r.history && r.history.length > 0,
  );
  if (!resourceWithHistory) {
    logWarning("No resources with history found");
    return;
  }

  logInfo(`Examining history for resource: ${resourceWithHistory.id}`);

  // Display resource history
  resourceWithHistory.history.forEach((event: any, index: number) => {
    logInfo(`History Event ${index + 1}:`, {
      timestamp: event.timestamp,
      activity: event.activity,
      fromState: event.fromState,
      toState: event.toState,
      hasData: !!event.data,
    });
  });

  // Get detailed resource information
  const detailedResource = await sdk.resources.get(resourceWithHistory.id);
  logSuccess("Resource audit trail:", {
    id: detailedResource.id,
    currentState: detailedResource.state,
    createdAt: detailedResource.createdAt,
    updatedAt: detailedResource.updatedAt,
    totalHistoryEvents: detailedResource.history.length,
    lastActivity:
      detailedResource.history[detailedResource.history.length - 1]?.activity,
  });

  return;
}

async function demonstrateResourceTemplates(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n📋 Resource Templates");
  console.log("=".repeat(50));

  // Get a workflow for templates
  const workflows = await sdk.workflows.list();
  if (workflows.length === 0) {
    logWarning("No workflows available for template demonstration");
    return;
  }

  const workflowId = workflows[0].id;

  // Create resource template manually since template API differs
  logSuccess("Creating user resource template manually");

  // Use template pattern to create resources
  const templateData = generateUserResourceData();
  const resourceFromTemplate: ResourceCreateInput = {
    workflowId,
    data: {
      ...templateData,
      accountType: "standard",
      status: "active",
      preferences: {
        notifications: true,
        marketing: false,
      },
      templateGenerated: true,
    },
    metadata: {
      templateUsed: "standard-user",
      createdBy: "template-system",
      generatedAt: new Date().toISOString(),
      generatorVersion: "1.0.0",
    },
  };

  const templateResourceId = await sdk.resources.create(resourceFromTemplate);
  logSuccess(`Resource created from template: ${templateResourceId}`);

  // Validate resource manually
  const hasRequiredFields =
    resourceFromTemplate.data.email &&
    resourceFromTemplate.data.profile?.firstName &&
    resourceFromTemplate.data.profile?.lastName;

  logInfo("Template validation result:", {
    isValid: hasRequiredFields,
    errors: hasRequiredFields ? [] : ["Missing required fields"],
    warnings: [],
  });

  return;
}

async function demonstrateResourceStatistics(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n📊 Resource Statistics and Analytics");
  console.log("=".repeat(50));

  // Get workflow statistics
  const workflows = await sdk.workflows.list();
  for (const workflow of workflows.slice(0, 3)) {
    const stats = await sdk.resources.getWorkflowStats(workflow.id);
    logInfo(`Workflow: ${workflow.name}`, {
      totalResources: stats.totalResources,
      resourcesByState: stats.resourcesByState,
      averageProcessingTime: stats.averageProcessingTime,
      completionRate: stats.completionRate,
    });
  }

  // Get overall resource statistics
  const overallStats = await sdk.resources.getGlobalStats();
  logSuccess("Global resource statistics:", {
    totalResources: overallStats.totalResources,
    totalWorkflows: overallStats.totalWorkflows,
    activeResources: overallStats.activeResources,
    completedResources: overallStats.completedResources,
    averageResourceLifetime: overallStats.averageResourceLifetime,
  });

  return;
}

// ============================================================================
// Main Demo Function
// ============================================================================

async function runTokenDemo(): Promise<void> {
  console.log("🚀 Starting Resource Management Demo");
  console.log("====================================\n");

  try {
    // Initialize SDK
    logInfo("Initializing Circuit Breaker SDK...");
    const sdk = new CircuitBreakerSDK(config);

    // Test connection
    logInfo("Testing SDK connection...");
    const sdkHealth = await sdk.getHealth();
    const sdkConfig = sdk.getConfig();
    logSuccess("SDK initialized successfully", {
      version: sdk.getVersion(),
      healthy: sdkHealth.healthy,
      endpoint: sdkConfig.graphqlEndpoint,
    });

    // Run demonstrations
    await demonstrateBasicResourceOperations(sdk);
    await demonstrateBatchOperations(sdk);
    await demonstrateResourceBuilder(sdk);
    await demonstrateResourceSearch(sdk);
    await demonstrateResourceHistory(sdk);
    await demonstrateResourceTemplates(sdk);
    await demonstrateResourceStatistics(sdk);

    // Final summary
    logInfo("\n📊 Demo Summary");
    console.log("=".repeat(50));

    const finalStats = await sdk.resources.getGlobalStats();
    logSuccess("Final resource statistics:", {
      totalResources: finalStats.totalResources,
      resourcesCreatedInDemo: finalStats.totalResources, // Assuming clean start
      workflowsCreated: finalStats.totalWorkflows,
    });

    console.log("\n✨ Resource Management Demo completed successfully!");
    console.log("===============================================");
  } catch (error) {
    logError("Resource demo failed", error);
    process.exit(1);
  }
}

// ============================================================================
// Run Demo
// ============================================================================

if (import.meta.url === `file://${process.argv[1]}`) {
  runTokenDemo()
    .then(() => {
      logSuccess("Demo completed successfully");
      process.exit(0);
    })
    .catch((error) => {
      logError("Demo failed", error);
      process.exit(1);
    });
}

export { runTokenDemo };
