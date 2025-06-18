const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const fs = require("fs");
const path = require("path");

// Load the LLM schema
const llmSchema = loadSchemaSync(
  path.join(__dirname, "../llm.graphql"),
  {
    loaders: [new GraphQLFileLoader()],
  },
);

// Load GraphQL operations
const operationsFile = path.join(
  __dirname,
  "../operations/llm.graphql",
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
 * LLM Examples
 * These examples demonstrate how to use the LLM provider management operations
 * defined in ../llm.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * List all configured LLM providers
 */
async function listLLMProviders() {
  const query = operationMap.ListLLMProviders;

  try {
    const data = await client.request(query);
    console.log("All LLM providers:", JSON.stringify(data, null, 2));
    return data.llmProviders;
  } catch (error) {
    console.error("Error listing LLM providers:", error);
    throw error;
  }
}

/**
 * Get specific LLM provider details
 */
async function getLLMProvider(providerId) {
  const query = operationMap.GetLLMProvider;

  try {
    const data = await client.request(query, { providerId });
    console.log("LLM provider details:", JSON.stringify(data, null, 2));
    return data.llmProvider;
  } catch (error) {
    console.error("Error fetching LLM provider:", error);
    throw error;
  }
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Send a chat completion request
 */
async function sendChatCompletion(userMessage, model = "gpt-4") {
  const mutation = operationMap.LLMChatCompletion;

  const chatInput = {
    model: model,
    messages: [
      {
        role: "system",
        content: "You are a helpful assistant that provides concise and accurate answers.",
      },
      {
        role: "user",
        content: userMessage,
      },
    ],
    temperature: 0.7,
    maxTokens: 500,
    topP: 0.9,
    frequencyPenalty: 0.0,
    presencePenalty: 0.0,
    stop: ["END", "STOP"],
    stream: false,
    user: "example-user",
    projectId: "example-project",
  };

  try {
    const data = await client.request(mutation, { input: chatInput });
    console.log("Chat completion response:", JSON.stringify(data, null, 2));
    return data.llmChatCompletion;
  } catch (error) {
    console.error("Error sending chat completion:", error);
    throw error;
  }
}

/**
 * Send a streaming chat request
 */
async function sendStreamingChat(userMessage, model = "gpt-3.5-turbo") {
  const mutation = operationMap.LLMChatCompletion;

  const chatInput = {
    model: model,
    messages: [
      {
        role: "user",
        content: userMessage,
      },
    ],
    temperature: 0.8,
    maxTokens: 1000,
    stream: true,
    user: "example-user",
  };

  try {
    const data = await client.request(mutation, { input: chatInput });
    console.log("Streaming chat response:", JSON.stringify(data, null, 2));
    return data.llmChatCompletion;
  } catch (error) {
    console.error("Error sending streaming chat:", error);
    throw error;
  }
}

/**
 * Configure OpenAI provider
 */
async function configureOpenAIProvider() {
  const mutation = operationMap.ConfigureLLMProvider;

  const providerInput = {
    providerType: "openai",
    name: "OpenAI GPT Models",
    baseUrl: "https://api.openai.com/v1",
    apiKeyId: "openai-key-1",
    models: [
      {
        id: "gpt-4",
        name: "GPT-4",
        maxTokens: 8192,
        contextWindow: 8192,
        costPerInputToken: 0.00003,
        costPerOutputToken: 0.00006,
        supportsStreaming: true,
        supportsFunctionCalling: true,
        capabilities: ["text-generation", "code-generation", "analysis"],
      },
      {
        id: "gpt-3.5-turbo",
        name: "GPT-3.5 Turbo",
        maxTokens: 4096,
        contextWindow: 4096,
        costPerInputToken: 0.0000015,
        costPerOutputToken: 0.000002,
        supportsStreaming: true,
        supportsFunctionCalling: true,
        capabilities: ["text-generation", "conversation"],
      },
    ],
  };

  try {
    const data = await client.request(mutation, { input: providerInput });
    console.log("Configured OpenAI provider:", JSON.stringify(data, null, 2));
    return data.configureLlmProvider;
  } catch (error) {
    console.error("Error configuring OpenAI provider:", error);
    throw error;
  }
}

/**
 * Configure Anthropic provider
 */
async function configureAnthropicProvider() {
  const mutation = operationMap.ConfigureLLMProvider;

  const providerInput = {
    providerType: "anthropic",
    name: "Anthropic Claude Models",
    baseUrl: "https://api.anthropic.com/v1",
    apiKeyId: "anthropic-key-1",
    models: [
      {
        id: "claude-3-opus",
        name: "Claude 3 Opus",
        maxTokens: 4096,
        contextWindow: 200000,
        costPerInputToken: 0.000015,
        costPerOutputToken: 0.000075,
        supportsStreaming: true,
        supportsFunctionCalling: false,
        capabilities: ["text-generation", "analysis", "reasoning"],
      },
      {
        id: "claude-3-sonnet",
        name: "Claude 3 Sonnet",
        maxTokens: 4096,
        contextWindow: 200000,
        costPerInputToken: 0.000003,
        costPerOutputToken: 0.000015,
        supportsStreaming: true,
        supportsFunctionCalling: false,
        capabilities: ["text-generation", "conversation"],
      },
    ],
  };

  try {
    const data = await client.request(mutation, { input: providerInput });
    console.log("Configured Anthropic provider:", JSON.stringify(data, null, 2));
    return data.configureLlmProvider;
  } catch (error) {
    console.error("Error configuring Anthropic provider:", error);
    throw error;
  }
}

/**
 * Configure local Ollama provider
 */
async function configureOllamaProvider() {
  const mutation = operationMap.ConfigureLLMProvider;

  const providerInput = {
    providerType: "ollama",
    name: "Local Ollama Instance",
    baseUrl: "http://localhost:11434",
    apiKeyId: "none",
    models: [
      {
        id: "llama2:7b",
        name: "Llama 2 7B",
        maxTokens: 2048,
        contextWindow: 4096,
        costPerInputToken: 0.0,
        costPerOutputToken: 0.0,
        supportsStreaming: true,
        supportsFunctionCalling: false,
        capabilities: ["text-generation", "conversation"],
      },
      {
        id: "codellama:13b",
        name: "Code Llama 13B",
        maxTokens: 2048,
        contextWindow: 4096,
        costPerInputToken: 0.0,
        costPerOutputToken: 0.0,
        supportsStreaming: true,
        supportsFunctionCalling: false,
        capabilities: ["code-generation", "code-analysis"],
      },
    ],
  };

  try {
    const data = await client.request(mutation, { input: providerInput });
    console.log("Configured Ollama provider:", JSON.stringify(data, null, 2));
    return data.configureLlmProvider;
  } catch (error) {
    console.error("Error configuring Ollama provider:", error);
    throw error;
  }
}

/**
 * Multi-turn conversation example
 */
async function multiTurnConversation() {
  const mutation = operationMap.LLMChatCompletion;

  const chatInput = {
    model: "gpt-4",
    messages: [
      {
        role: "system",
        content: "You are a technical advisor helping with software architecture decisions.",
      },
      {
        role: "user",
        content: "I'm building a microservices application. What are the key considerations for service communication?",
      },
      {
        role: "assistant",
        content: "Key considerations for microservice communication include: 1) Choosing between synchronous (REST, gRPC) vs asynchronous (message queues) communication, 2) Implementing proper error handling and circuit breakers, 3) Managing data consistency across services, and 4) Monitoring and observability for distributed systems.",
      },
      {
        role: "user",
        content: "How do I implement circuit breakers effectively?",
      },
    ],
    temperature: 0.3,
    maxTokens: 800,
    user: "developer-123",
    projectId: "architecture-project",
  };

  try {
    const data = await client.request(mutation, { input: chatInput });
    console.log("Multi-turn conversation response:", JSON.stringify(data, null, 2));
    return data.llmChatCompletion;
  } catch (error) {
    console.error("Error in multi-turn conversation:", error);
    throw error;
  }
}

// ============================================================================
// SUBSCRIPTION EXAMPLES
// ============================================================================

/**
 * Subscribe to LLM streaming response using WebSocket
 * Note: This requires a WebSocket client like graphql-ws
 */
function subscribeToLLMStream(requestId, callback) {
  const subscription = operationMap.LLMStream;

  // This would require WebSocket setup
  console.log(`Subscription query for request ${requestId}:`, subscription);

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { requestId } }, callback);
}

// ============================================================================
// COMPLETE LLM EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating LLM provider lifecycle
 */
async function completeLLMExample() {
  console.log("\n=== Complete LLM Example ===\n");

  try {
    // 1. Configure providers
    console.log("1. Configuring OpenAI provider...");
    await configureOpenAIProvider();

    console.log("\n2. Configuring Anthropic provider...");
    await configureAnthropicProvider();

    // 3. List all providers
    console.log("\n3. Listing all LLM providers...");
    const providers = await listLLMProviders();

    // 4. Send chat completion
    console.log("\n4. Sending chat completion...");
    await sendChatCompletion("Explain quantum computing in simple terms");

    // 5. Multi-turn conversation
    console.log("\n5. Multi-turn conversation about circuit breakers...");
    await multiTurnConversation();

    // 6. Streaming example
    console.log("\n6. Streaming chat example...");
    await sendStreamingChat("Write a short story about a robot learning to paint");

    console.log("\n✅ LLM example completed successfully!");

  } catch (error) {
    console.error("\n❌ LLM example failed:", error.message);
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to calculate token cost
 */
function calculateTokenCost(usage, model) {
  const costs = {
    "gpt-4": { input: 0.00003, output: 0.00006 },
    "gpt-3.5-turbo": { input: 0.0000015, output: 0.000002 },
    "claude-3-opus": { input: 0.000015, output: 0.000075 },
    "claude-3-sonnet": { input: 0.000003, output: 0.000015 },
  };

  const modelCosts = costs[model] || { input: 0, output: 0 };
  const inputCost = usage.promptTokens * modelCosts.input;
  const outputCost = usage.completionTokens * modelCosts.output;

  return {
    inputCost,
    outputCost,
    totalCost: inputCost + outputCost,
    breakdown: {
      promptTokens: usage.promptTokens,
      completionTokens: usage.completionTokens,
      totalTokens: usage.totalTokens,
    },
  };
}

/**
 * Helper function to format provider health status
 */
function formatHealthStatus(healthStatus) {
  return {
    status: healthStatus.isHealthy ? "✅ Healthy" : "❌ Unhealthy",
    lastCheck: new Date(healthStatus.lastCheck).toLocaleString(),
    errorRate: `${(healthStatus.errorRate * 100).toFixed(2)}%`,
    avgLatency: `${healthStatus.averageLatencyMs}ms`,
    failures: healthStatus.consecutiveFailures,
    lastError: healthStatus.lastError || "None",
  };
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  listLLMProviders,
  getLLMProvider,

  // Mutation functions
  sendChatCompletion,
  sendStreamingChat,
  configureOpenAIProvider,
  configureAnthropicProvider,
  configureOllamaProvider,
  multiTurnConversation,

  // Subscription functions
  subscribeToLLMStream,

  // Complete examples
  completeLLMExample,

  // Utilities
  calculateTokenCost,
  formatHealthStatus,

  // Schema reference
  llmSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeLLMExample()
    .catch(console.error)
    .finally(() => process.exit(0));
}
