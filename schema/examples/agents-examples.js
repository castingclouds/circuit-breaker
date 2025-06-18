const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const fs = require("fs");
const path = require("path");

// Load the agents schema
const agentsSchema = loadSchemaSync(
  path.join(__dirname, "../agents.graphql"),
  {
    loaders: [new GraphQLFileLoader()],
  },
);

// Load GraphQL operations
const operationsFile = path.join(
  __dirname,
  "../operations/agents.graphql",
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
 * Agent Examples
 * These examples demonstrate how to use the agent management operations
 * defined in ../agents.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * Get a specific agent with full configuration
 */
async function getAgent(agentId) {
  const query = operationMap.GetAgent;

  try {
    const data = await client.request(query, { agentId });
    console.log("Agent details:", JSON.stringify(data, null, 2));
    return data.agent;
  } catch (error) {
    console.error("Error fetching agent:", error);
    throw error;
  }
}

/**
 * List all agents
 */
async function listAgents() {
  const query = operationMap.ListAgents;

  try {
    const data = await client.request(query);
    console.log("All agents:", JSON.stringify(data, null, 2));
    return data.agents;
  } catch (error) {
    console.error("Error listing agents:", error);
    throw error;
  }
}

/**
 * Get state agent configurations for a specific state
 */
async function getStateAgentConfigs(stateId) {
  const query = operationMap.GetStateAgentConfigs;

  try {
    const data = await client.request(query, { stateId });
    console.log("State agent configs:", JSON.stringify(data, null, 2));
    return data.stateAgentConfigs;
  } catch (error) {
    console.error("Error fetching state agent configs:", error);
    throw error;
  }
}

/**
 * Get agent execution details
 */
async function getAgentExecution(executionId) {
  const query = operationMap.GetAgentExecution;

  try {
    const data = await client.request(query, { executionId });
    console.log("Agent execution details:", JSON.stringify(data, null, 2));
    return data.agentExecution;
  } catch (error) {
    console.error("Error fetching agent execution:", error);
    throw error;
  }
}

/**
 * Get all executions for a resource
 */
async function getResourceExecutions(resourceId) {
  const query = operationMap.GetResourceExecutions;

  try {
    const data = await client.request(query, { resourceId });
    console.log("Resource executions:", JSON.stringify(data, null, 2));
    return data.resourceExecutions;
  } catch (error) {
    console.error("Error fetching resource executions:", error);
    throw error;
  }
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Create a document analysis agent
 */
async function createDocumentAnalysisAgent() {
  const mutation = operationMap.CreateAgent;

  const agentInput = {
    name: "Document Analyzer",
    description: "AI agent that analyzes and extracts key information from documents",
    llmProvider: {
      providerType: "openai",
      model: "gpt-4",
      apiKey: process.env.OPENAI_API_KEY || "sk-test-key",
      baseUrl: "https://api.openai.com/v1",
    },
    llmConfig: {
      temperature: 0.2,
      maxTokens: 2000,
      topP: 0.9,
      frequencyPenalty: 0.0,
      presencePenalty: 0.0,
      stopSequences: ["END_ANALYSIS"],
    },
    prompts: {
      system: "You are a document analysis expert. Analyze documents and extract key information including entities, themes, and actionable items.",
      userTemplate: "Please analyze the following document:\n\n{document_content}\n\nExtract:\n1. Key entities (people, organizations, dates)\n2. Main themes\n3. Action items\n4. Risk factors\n\nFormat your response as structured JSON.",
      contextInstructions: "Consider the document type and context when analyzing. Be thorough but concise.",
    },
    capabilities: [
      "document_analysis",
      "entity_extraction",
      "theme_identification",
      "risk_assessment",
    ],
    tools: [
      "text_parser",
      "entity_recognizer",
      "sentiment_analyzer",
    ],
  };

  try {
    const data = await client.request(mutation, { input: agentInput });
    console.log("Created document analysis agent:", JSON.stringify(data, null, 2));
    return data.createAgent;
  } catch (error) {
    console.error("Error creating agent:", error);
    throw error;
  }
}

/**
 * Create a customer service agent
 */
async function createCustomerServiceAgent() {
  const mutation = operationMap.CreateAgent;

  const agentInput = {
    name: "Customer Service Assistant",
    description: "AI agent for handling customer inquiries and support requests",
    llmProvider: {
      providerType: "anthropic",
      model: "claude-3-sonnet",
      apiKey: process.env.ANTHROPIC_API_KEY || "sk-ant-test-key",
    },
    llmConfig: {
      temperature: 0.7,
      maxTokens: 1000,
      stopSequences: ["CONVERSATION_END"],
    },
    prompts: {
      system: "You are a helpful customer service representative. Be polite, professional, and solution-oriented.",
      userTemplate: "Customer inquiry: {customer_message}\n\nCustomer context: {customer_history}\n\nPlease provide a helpful response.",
      contextInstructions: "Always prioritize customer satisfaction while following company policies.",
    },
    capabilities: [
      "customer_support",
      "inquiry_handling",
      "problem_solving",
      "escalation_management",
    ],
    tools: [
      "knowledge_base",
      "ticket_system",
      "customer_database",
    ],
  };

  try {
    const data = await client.request(mutation, { input: agentInput });
    console.log("Created customer service agent:", JSON.stringify(data, null, 2));
    return data.createAgent;
  } catch (error) {
    console.error("Error creating agent:", error);
    throw error;
  }
}

/**
 * Create state agent configuration
 */
async function createStateAgentConfig(agentId, stateId) {
  const mutation = operationMap.CreateStateAgentConfig;

  const configInput = {
    stateId: stateId,
    agentId: agentId,
    llmConfig: {
      temperature: 0.1,
      maxTokens: 1500,
      stopSequences: ["REVIEW_COMPLETE"],
    },
    inputMapping: {
      document_content: "data.document.content",
      document_type: "data.document.type",
      priority: "metadata.priority",
    },
    outputMapping: {
      review_result: "data.review",
      confidence_score: "data.confidence",
      recommendations: "data.recommendations",
    },
    autoActivity: "complete_review",
    schedule: {
      initialDelaySeconds: 30,
      intervalSeconds: 300,
      maxExecutions: 5,
    },
    retryConfig: {
      maxAttempts: 3,
      backoffSeconds: 60,
      retryOnErrors: ["timeout", "rate_limit", "temporary_failure"],
    },
    enabled: true,
  };

  try {
    const data = await client.request(mutation, { input: configInput });
    console.log("Created state agent config:", JSON.stringify(data, null, 2));
    return data.createStateAgentConfig;
  } catch (error) {
    console.error("Error creating state agent config:", error);
    throw error;
  }
}

/**
 * Trigger state agents for a resource
 */
async function triggerStateAgents(resourceId) {
  const mutation = operationMap.TriggerStateAgents;

  const triggerInput = {
    resourceId: resourceId,
  };

  try {
    const data = await client.request(mutation, { input: triggerInput });
    console.log("Triggered state agents:", JSON.stringify(data, null, 2));
    return data.triggerStateAgents;
  } catch (error) {
    console.error("Error triggering state agents:", error);
    throw error;
  }
}

// ============================================================================
// SUBSCRIPTION EXAMPLES
// ============================================================================

/**
 * Subscribe to agent execution events using WebSocket
 * Note: This requires a WebSocket client like graphql-ws
 */
function subscribeToAgentExecution(executionId, callback) {
  const subscription = operationMap.AgentExecutionStream;

  // This would require WebSocket setup
  console.log(`Subscription query for execution ${executionId}:`, subscription);

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { executionId } }, callback);
}

// ============================================================================
// COMPLETE AGENT EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating agent lifecycle
 */
async function completeAgentExample() {
  console.log("\n=== Complete Agent Example ===\n");

  try {
    // 1. Create a document analysis agent
    console.log("1. Creating document analysis agent...");
    const agent = await createDocumentAnalysisAgent();
    const agentId = agent.id;

    // 2. Create a state agent configuration
    console.log("\n2. Creating state agent configuration...");
    const stateConfig = await createStateAgentConfig(agentId, "document_review");

    // 3. List all agents
    console.log("\n3. Listing all agents...");
    await listAgents();

    // 4. Get agent details
    console.log("\n4. Getting agent details...");
    await getAgent(agentId);

    // 5. Get state agent configs
    console.log("\n5. Getting state agent configs...");
    await getStateAgentConfigs("document_review");

    console.log("\n✅ Agent example completed successfully!");

  } catch (error) {
    console.error("\n❌ Agent example failed:", error.message);
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to validate agent configuration
 */
function validateAgentConfig(config) {
  const required = ['name', 'description', 'llmProvider', 'llmConfig', 'prompts'];
  const missing = required.filter(field => !(field in config));
  if (missing.length > 0) {
    throw new Error(`Missing required fields: ${missing.join(', ')}`);
  }
  return true;
}

/**
 * Helper function to format execution status
 */
function formatExecutionStatus(execution) {
  return {
    id: execution.id,
    status: execution.status,
    duration: execution.durationMs ? `${execution.durationMs}ms` : 'N/A',
    retries: execution.retryCount,
    completed: execution.completedAt ? new Date(execution.completedAt).toLocaleString() : 'N/A'
  };
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  getAgent,
  listAgents,
  getStateAgentConfigs,
  getAgentExecution,
  getResourceExecutions,

  // Mutation functions
  createDocumentAnalysisAgent,
  createCustomerServiceAgent,
  createStateAgentConfig,
  triggerStateAgents,

  // Subscription functions
  subscribeToAgentExecution,

  // Complete examples
  completeAgentExample,

  // Utilities
  validateAgentConfig,
  formatExecutionStatus,

  // Schema reference
  agentsSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeAgentExample()
    .catch(console.error)
    .finally(() => process.exit(0));
}
