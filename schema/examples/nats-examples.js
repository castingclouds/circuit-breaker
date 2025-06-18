const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const fs = require("fs");
const path = require("path");

// Load the NATS schema
const natsSchema = loadSchemaSync(
  path.join(__dirname, "../nats.graphql"),
  {
    loaders: [new GraphQLFileLoader()],
  },
);

// Load GraphQL operations
const operationsFile = path.join(
  __dirname,
  "../operations/nats.graphql",
);
const operations = fs.readFileSync(operationsFile, "utf8");

// Parse operations to extract individual queries/mutations
const operationMap = {};
const operationRegex =
  /(query|mutation|subscription)\s+(\w+)[\s\S]*?(?=(?:query|mutation|subscription)\s+\w+|$)/g;
let match;
while ((match = operationRegex.exec(operations)) !== null) {
  operationMap[match[2]] = match[0].trim();
}

// GraphQL client setup
const endpoint = "http://localhost:4000/graphql";
const client = new GraphQLClient(endpoint);

/**
 * NATS Examples
 * These examples demonstrate how to use the NATS-enhanced operations
 * defined in ../nats.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * Get resource with NATS metadata by ID
 */
async function getNatsResource(resourceId) {
  const query = operationMap.GetNatsResource;

  try {
    const data = await client.request(query, { id: resourceId });
    console.log("NATS resource details:", JSON.stringify(data, null, 2));
    return data.natsResource;
  } catch (error) {
    console.error("Error fetching NATS resource:", error);
    throw error;
  }
}

/**
 * Get resources currently in a specific state
 */
async function getResourcesInState(workflowId, stateId) {
  const query = operationMap.GetResourcesInState;

  try {
    const data = await client.request(query, { workflowId, stateId });
    console.log("Resources in state:", JSON.stringify(data, null, 2));
    return data.resourcesInState;
  } catch (error) {
    console.error("Error fetching resources in state:", error);
    throw error;
  }
}

/**
 * Find resource by ID with workflow context (more efficient for NATS)
 */
async function findResource(workflowId, resourceId) {
  const query = operationMap.FindResource;

  try {
    const data = await client.request(query, { workflowId, resourceId });
    console.log("Found resource:", JSON.stringify(data, null, 2));
    return data.findResource;
  } catch (error) {
    console.error("Error finding resource:", error);
    throw error;
  }
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Create a workflow instance with NATS event tracking
 */
async function createWorkflowInstance(workflowId, initialData = {}, metadata = {}, triggeredBy = "system") {
  const mutation = operationMap.CreateWorkflowInstance;

  const instanceInput = {
    workflowId: workflowId,
    initialData: initialData,
    metadata: {
      ...metadata,
      natsTracking: true,
      createdBy: triggeredBy,
      timestamp: new Date().toISOString(),
    },
    triggeredBy: triggeredBy,
  };

  try {
    const data = await client.request(mutation, { input: instanceInput });
    console.log("Created workflow instance:", JSON.stringify(data, null, 2));
    return data.createWorkflowInstance;
  } catch (error) {
    console.error("Error creating workflow instance:", error);
    throw error;
  }
}

/**
 * Create a document processing workflow instance
 */
async function createDocumentWorkflowInstance(workflowId) {
  const initialData = {
    document: {
      name: "Q4-Financial-Report.pdf",
      type: "financial_report",
      size: 3145728, // 3MB
      uploadedAt: new Date().toISOString(),
    },
    submitter: {
      userId: "user-finance-001",
      department: "Finance",
      email: "finance@company.com",
    },
    priority: "high",
    deadline: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(), // 7 days from now
  };

  const metadata = {
    source: "finance_portal",
    category: "quarterly_report",
    compliance: ["SOX", "GAAP"],
    requiredApprovals: ["CFO", "External_Auditor"],
  };

  return await createWorkflowInstance(workflowId, initialData, metadata, "finance-portal");
}

/**
 * Execute activity with NATS event publishing
 */
async function executeActivityWithNats(resourceId, activityId, newState, triggeredBy = "system", data = {}) {
  const mutation = operationMap.ExecuteActivityWithNats;

  const executeInput = {
    resourceId: resourceId,
    activityId: activityId,
    newState: newState,
    triggeredBy: triggeredBy,
    data: {
      ...data,
      executedAt: new Date().toISOString(),
      natsEnabled: true,
    },
  };

  try {
    const data = await client.request(mutation, { input: executeInput });
    console.log("Executed activity with NATS:", JSON.stringify(data, null, 2));
    return data.executeActivityWithNats;
  } catch (error) {
    console.error("Error executing activity with NATS:", error);
    throw error;
  }
}

/**
 * Process document through review workflow
 */
async function processDocumentReview(resourceId) {
  const processData = {
    reviewer: "ai-document-analyzer",
    analysisResults: {
      completeness: 0.95,
      accuracy: 0.87,
      complianceScore: 0.92,
      flaggedIssues: [
        "Missing executive summary section",
        "Footnote 12 requires clarification"
      ],
    },
    processingTime: "00:02:34",
    confidence: 0.89,
  };

  return await executeActivityWithNats(
    resourceId,
    "start_review",
    "under_review",
    "ai-analyzer-v2",
    processData
  );
}

/**
 * Approve document after review
 */
async function approveDocument(resourceId, approverUserId) {
  const approvalData = {
    approver: approverUserId,
    approvalType: "final",
    comments: "Document meets all compliance requirements",
    signatureTimestamp: new Date().toISOString(),
    approvalCode: `APPR-${Date.now()}`,
  };

  return await executeActivityWithNats(
    resourceId,
    "approve_document",
    "approved",
    approverUserId,
    approvalData
  );
}

// ============================================================================
// NATS EVENT TRACKING EXAMPLES
// ============================================================================

/**
 * Monitor workflow progress using NATS sequence tracking
 */
async function monitorWorkflowProgress(resourceId) {
  console.log("\n=== Monitoring Workflow Progress ===");

  try {
    const resource = await getNatsResource(resourceId);

    if (!resource) {
      console.log("Resource not found");
      return;
    }

    console.log("\n📊 Current Status:");
    console.log(`  Resource ID: ${resource.id}`);
    console.log(`  Current State: ${resource.state}`);
    console.log(`  NATS Sequence: ${resource.natsSequence}`);
    console.log(`  NATS Subject: ${resource.natsSubject}`);
    console.log(`  Last Updated: ${new Date(resource.updatedAt).toLocaleString()}`);

    console.log("\n📈 Activity History:");
    resource.activityHistory.forEach((activity, index) => {
      console.log(`  ${index + 1}. ${activity.fromState} → ${activity.toState}`);
      console.log(`     Activity: ${activity.activityId}`);
      console.log(`     Triggered by: ${activity.triggeredBy}`);
      console.log(`     NATS Seq: ${activity.natsSequence}`);
      console.log(`     Time: ${new Date(activity.timestamp).toLocaleString()}`);
      console.log("");
    });

    return resource;
  } catch (error) {
    console.error("Error monitoring workflow progress:", error);
    throw error;
  }
}

/**
 * Get all resources in processing state
 */
async function getProcessingResources(workflowId) {
  console.log("\n=== Resources Currently Processing ===");

  try {
    const resources = await getResourcesInState(workflowId, "processing");

    console.log(`Found ${resources.length} resources in processing state:`);

    resources.forEach((resource, index) => {
      console.log(`\n${index + 1}. Resource ${resource.id}:`);
      console.log(`   NATS Sequence: ${resource.natsSequence}`);
      console.log(`   Started: ${new Date(resource.createdAt).toLocaleString()}`);
      console.log(`   Processing Time: ${getProcessingDuration(resource.createdAt)}`);

      if (resource.activityHistory.length > 0) {
        const lastActivity = resource.activityHistory[resource.activityHistory.length - 1];
        console.log(`   Last Activity: ${lastActivity.activityId}`);
        console.log(`   Triggered by: ${lastActivity.triggeredBy}`);
      }
    });

    return resources;
  } catch (error) {
    console.error("Error getting processing resources:", error);
    throw error;
  }
}

// ============================================================================
// COMPLETE NATS EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating NATS-enhanced workflow lifecycle
 */
async function completeNatsExample() {
  console.log("\n=== Complete NATS Example ===\n");

  const workflowId = "document-workflow-nats";

  try {
    // 1. Create a workflow instance with NATS tracking
    console.log("1. Creating document workflow instance with NATS tracking...");
    const instance = await createDocumentWorkflowInstance(workflowId);
    const resourceId = instance.id;

    // 2. Monitor initial state
    console.log("\n2. Monitoring initial workflow state...");
    await monitorWorkflowProgress(resourceId);

    // 3. Start document processing
    console.log("\n3. Starting document processing...");
    await processDocumentReview(resourceId);

    // 4. Monitor processing state
    console.log("\n4. Monitoring processing state...");
    await monitorWorkflowProgress(resourceId);

    // 5. Check all processing resources
    console.log("\n5. Checking all processing resources...");
    await getProcessingResources(workflowId);

    // 6. Approve document
    console.log("\n6. Approving document...");
    await approveDocument(resourceId, "cfo-user-001");

    // 7. Final state monitoring
    console.log("\n7. Final workflow state monitoring...");
    await monitorWorkflowProgress(resourceId);

    // 8. Demonstrate efficient resource finding
    console.log("\n8. Demonstrating efficient resource finding...");
    const foundResource = await findResource(workflowId, resourceId);
    console.log(`✅ Efficiently found resource: ${foundResource.id}`);

    console.log("\n✅ NATS example completed successfully!");

  } catch (error) {
    console.error("\n❌ NATS example failed:", error.message);
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to calculate processing duration
 */
function getProcessingDuration(startTime) {
  const start = new Date(startTime);
  const now = new Date();
  const diffMs = now - start;

  const hours = Math.floor(diffMs / (1000 * 60 * 60));
  const minutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = Math.floor((diffMs % (1000 * 60)) / 1000);

  return `${hours}h ${minutes}m ${seconds}s`;
}

/**
 * Helper function to format NATS event information
 */
function formatNatsEvent(resource) {
  return {
    resourceId: resource.id,
    currentState: resource.state,
    natsInfo: {
      sequence: resource.natsSequence,
      subject: resource.natsSubject,
      timestamp: resource.natsTimestamp,
    },
    eventCount: resource.activityHistory.length,
    lastActivity: resource.activityHistory.length > 0
      ? resource.activityHistory[resource.activityHistory.length - 1].activityId
      : "none",
  };
}

/**
 * Helper function to analyze workflow performance using NATS data
 */
function analyzeWorkflowPerformance(resource) {
  if (!resource.activityHistory || resource.activityHistory.length === 0) {
    return { message: "No activity history available" };
  }

  const activities = resource.activityHistory;
  const startTime = new Date(activities[0].timestamp);
  const endTime = new Date(activities[activities.length - 1].timestamp);
  const totalDuration = endTime - startTime;

  const stateTransitions = activities.map(activity => ({
    from: activity.fromState,
    to: activity.toState,
    duration: activity.timestamp,
    triggeredBy: activity.triggeredBy,
  }));

  return {
    totalDuration: getProcessingDuration(startTime),
    totalSteps: activities.length,
    averageStepTime: Math.round(totalDuration / activities.length / 1000), // seconds
    stateTransitions: stateTransitions,
    natsEvents: activities.map(a => a.natsSequence).filter(Boolean),
    performance: totalDuration < 300000 ? "fast" : totalDuration < 3600000 ? "normal" : "slow", // < 5min, < 1hr
  };
}

/**
 * Helper function to validate NATS event sequence
 */
function validateNatsSequence(resource) {
  const sequences = resource.activityHistory
    .map(activity => parseInt(activity.natsSequence))
    .filter(seq => !isNaN(seq))
    .sort((a, b) => a - b);

  const isSequential = sequences.every((seq, index) =>
    index === 0 || seq === sequences[index - 1] + 1
  );

  return {
    isValid: isSequential,
    sequenceCount: sequences.length,
    firstSequence: sequences[0],
    lastSequence: sequences[sequences.length - 1],
    gaps: isSequential ? [] : findSequenceGaps(sequences),
  };
}

/**
 * Helper function to find gaps in NATS sequence
 */
function findSequenceGaps(sequences) {
  const gaps = [];
  for (let i = 1; i < sequences.length; i++) {
    if (sequences[i] !== sequences[i - 1] + 1) {
      gaps.push({
        before: sequences[i - 1],
        after: sequences[i],
        missing: sequences[i] - sequences[i - 1] - 1,
      });
    }
  }
  return gaps;
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  getNatsResource,
  getResourcesInState,
  findResource,

  // Mutation functions
  createWorkflowInstance,
  createDocumentWorkflowInstance,
  executeActivityWithNats,
  processDocumentReview,
  approveDocument,

  // Monitoring functions
  monitorWorkflowProgress,
  getProcessingResources,

  // Complete examples
  completeNatsExample,

  // Utilities
  getProcessingDuration,
  formatNatsEvent,
  analyzeWorkflowPerformance,
  validateNatsSequence,
  findSequenceGaps,

  // Schema reference
  natsSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeNatsExample()
    .catch(console.error)
    .finally(() => process.exit(0));
}
