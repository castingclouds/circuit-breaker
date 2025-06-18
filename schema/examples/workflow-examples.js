const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const { addResolversToSchema } = require("@graphql-tools/schema");
const fs = require("fs");
const path = require("path");

// Load the workflow schema
const workflowSchema = loadSchemaSync(
  path.join(__dirname, "../workflow.graphql"),
  {
    loaders: [new GraphQLFileLoader()],
  },
);

// Load GraphQL operations
const operationsFile = path.join(__dirname, "../operations/workflow.graphql");
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
 * Workflow Examples
 * These examples demonstrate how to use the workflow management operations
 * defined in ../workflow.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * Get a specific workflow with all state details
 */
async function getWorkflow(workflowId) {
  const query = operationMap.GetWorkflow;

  try {
    const data = await client.request(query, { workflowId });
    console.log("Workflow details:", JSON.stringify(data, null, 2));
    return data.workflow;
  } catch (error) {
    console.error("Error fetching workflow:", error);
    throw error;
  }
}

/**
 * List all workflows
 */
async function listWorkflows() {
  const query = operationMap.ListWorkflows;

  try {
    const data = await client.request(query);
    console.log("All workflows:", JSON.stringify(data, null, 2));
    return data.workflows;
  } catch (error) {
    console.error("Error listing workflows:", error);
    throw error;
  }
}

/**
 * Get a specific resource with state information
 */
async function getResource(resourceId) {
  const query = operationMap.GetResource;

  try {
    const data = await client.request(query, { resourceId });
    console.log("Resource details:", JSON.stringify(data, null, 2));
    return data.resource;
  } catch (error) {
    console.error("Error fetching resource:", error);
    throw error;
  }
}

/**
 * Get available activities for a resource
 */
async function getAvailableActivities(resourceId) {
  const query = operationMap.GetAvailableActivities;

  try {
    const data = await client.request(query, { resourceId });
    console.log("Available activities:", JSON.stringify(data, null, 2));
    return data.availableActivities;
  } catch (error) {
    console.error("Error fetching available activities:", error);
    throw error;
  }
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Create a document processing workflow
 */
async function createDocumentProcessingWorkflow() {
  const mutation = operationMap.CreateWorkflow;

  const workflowInput = {
    name: "Document Processing Workflow",
    description: "Process and review documents",
    states: [
      {
        id: "submitted",
        name: "Submitted",
        workflowId: "doc-processing",
        stateType: "INITIAL",
        isInitial: true,
        description: "Document has been submitted",
        config: {
          timeoutSeconds: 86400,
          notifications: {
            enabled: true,
            channels: ["email", "slack"],
          },
        },
      },
      {
        id: "processing",
        name: "Processing",
        workflowId: "doc-processing",
        stateType: "NORMAL",
        description: "Document is being processed",
        config: {
          timeoutSeconds: 3600,
          autoTransition: {
            enabled: true,
            delaySeconds: 300,
            targetState: "approved",
            conditions: ["processing_complete"],
          },
        },
      },
      {
        id: "approved",
        name: "Approved",
        workflowId: "doc-processing",
        stateType: "TERMINAL",
        isTerminal: true,
        description: "Document has been approved",
      },
    ],
    activities: [
      {
        id: "start_processing",
        name: "Start Processing",
        fromStates: ["submitted"],
        toState: "processing",
        conditions: ["document_uploaded"],
        description: "Begin document processing",
      },
      {
        id: "approve",
        name: "Approve Document",
        fromStates: ["processing"],
        toState: "approved",
        conditions: ["processing_complete"],
        description: "Approve the document",
      },
    ],
    initialState: "submitted",
  };

  try {
    const data = await client.request(mutation, { input: workflowInput });
    console.log("Created workflow:", JSON.stringify(data, null, 2));
    return data.createWorkflow;
  } catch (error) {
    console.error("Error creating workflow:", error);
    throw error;
  }
}

/**
 * Create a new resource in a workflow
 */
async function createDocumentResource(workflowId) {
  const mutation = operationMap.CreateResource;

  const resourceInput = {
    workflowId: workflowId,
    data: {
      documentName: "Annual Report 2024.pdf",
      documentType: "annual_report",
      submittedBy: "john.doe@company.com",
      priority: "high",
    },
    metadata: {
      source: "web_upload",
      fileSize: 2048576,
      submissionDate: "2024-01-15T10:30:00Z",
    },
  };

  try {
    const data = await client.request(mutation, { input: resourceInput });
    console.log("Created resource:", JSON.stringify(data, null, 2));
    return data.createResource;
  } catch (error) {
    console.error("Error creating resource:", error);
    throw error;
  }
}

/**
 * Execute an activity to transition a resource
 */
async function executeActivity(resourceId, activityId, activityData = {}) {
  const mutation = operationMap.ExecuteActivity;

  const executeInput = {
    resourceId: resourceId,
    activityId: activityId,
    data: activityData,
  };

  try {
    const data = await client.request(mutation, { input: executeInput });
    console.log("Activity executed:", JSON.stringify(data, null, 2));
    return data.executeActivity;
  } catch (error) {
    console.error("Error executing activity:", error);
    throw error;
  }
}

// ============================================================================
// SUBSCRIPTION EXAMPLES
// ============================================================================

/**
 * Subscribe to resource updates using WebSocket
 * Note: This requires a WebSocket client like graphql-ws
 */
function subscribeToResourceUpdates(resourceId, callback) {
  const subscription = operationMap.ResourceUpdates;

  // This would require WebSocket setup
  console.log(`Subscription query for resource ${resourceId}:`, subscription);

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { resourceId } }, callback);
}

// ============================================================================
// COMPLETE WORKFLOW EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating a full workflow lifecycle
 */
async function completeWorkflowExample() {
  console.log("\n=== Complete Workflow Example ===\n");

  try {
    // 1. Create a workflow
    console.log("1. Creating workflow...");
    const workflow = await createDocumentProcessingWorkflow();
    const workflowId = workflow.id;

    // 2. Create a resource in the workflow
    console.log("\n2. Creating resource...");
    const resource = await createDocumentResource(workflowId);
    const resourceId = resource.id;

    // 3. Check available activities
    console.log("\n3. Checking available activities...");
    const activities = await getAvailableActivities(resourceId);

    // 4. Execute an activity if available
    if (activities.length > 0) {
      console.log("\n4. Executing activity...");
      const activityId = activities[0].id;
      await executeActivity(resourceId, activityId, {
        processingAgent: "doc-processor-v2",
        priority: "high",
      });
    }

    // 5. Get updated resource state
    console.log("\n5. Getting updated resource state...");
    await getResource(resourceId);

    console.log("\n✅ Workflow example completed successfully!");
  } catch (error) {
    console.error("\n❌ Workflow example failed:", error.message);
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to validate GraphQL response
 */
function validateResponse(response, expectedFields) {
  const missing = expectedFields.filter((field) => !(field in response));
  if (missing.length > 0) {
    throw new Error(`Missing required fields: ${missing.join(", ")}`);
  }
  return true;
}

/**
 * Helper function to format timestamps
 */
function formatTimestamp(timestamp) {
  return new Date(timestamp).toLocaleString();
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  getWorkflow,
  listWorkflows,
  getResource,
  getAvailableActivities,

  // Mutation functions
  createDocumentProcessingWorkflow,
  createDocumentResource,
  executeActivity,

  // Subscription functions
  subscribeToResourceUpdates,

  // Complete examples
  completeWorkflowExample,

  // Utilities
  validateResponse,
  formatTimestamp,

  // Schema reference
  workflowSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeWorkflowExample()
    .catch(console.error)
    .finally(() => process.exit(0));
}
