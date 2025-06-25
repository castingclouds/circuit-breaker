#!/usr/bin/env node

/**
 * Simple connection test for Circuit Breaker TypeScript SDK
 * Tests both GraphQL and REST/SSE endpoints
 */

import { Client, COMMON_MODELS } from "./src/index.js";

async function testConnection() {
  console.log("🔧 Circuit Breaker SDK Connection Test");
  console.log("=====================================");

  // Test OpenAI-compatible REST API (port 3000)
  console.log("\n1. Testing OpenAI-compatible REST API (port 3000)");
  console.log("   ------------------------------------------------");

  const restClient = new Client({
    baseUrl: "http://localhost:3000",
    timeout: 10000,
  });

  try {
    // Test health endpoint
    console.log("   Testing /health endpoint...");
    const healthResponse = await fetch("http://localhost:3000/health");
    if (healthResponse.ok) {
      const healthData = await healthResponse.json();
      console.log(`   ✅ Health check: ${healthData.status}`);
    } else {
      console.log(`   ❌ Health check failed: ${healthResponse.status}`);
    }

    // Test models endpoint
    console.log("   Testing /v1/models endpoint...");
    const llm = restClient.llm();
    const models = await llm.listModels();
    console.log(`   ✅ Found ${models.length} models`);
    console.log("   Available models:");
    models.slice(0, 5).forEach(model => {
      console.log(`     • ${model.id} (${model.owned_by})`);
    });
    if (models.length > 5) {
      console.log(`     ... and ${models.length - 5} more`);
    }

    // Test virtual models
    console.log("   Testing virtual models...");
    const virtualModels = [
      COMMON_MODELS.SMART_FAST,
      COMMON_MODELS.SMART_CHEAP,
      COMMON_MODELS.SMART_CODING,
    ];

    for (const virtualModel of virtualModels) {
      try {
        const response = await llm.chatCompletion({
          model: virtualModel,
          messages: [{ role: "user", content: "Say 'Hello from Circuit Breaker!'" }],
          max_tokens: 20,
        });
        const content = response.choices[0]?.message?.content || "";
        console.log(`   ✅ ${virtualModel}: ${content.trim()}`);
      } catch (error) {
        console.log(`   ❌ ${virtualModel}: ${error}`);
      }
    }

  } catch (error) {
    console.log(`   ❌ REST API test failed: ${error}`);
    console.log("   Make sure the Circuit Breaker server is running on port 3000");
  }

  // Test GraphQL API (port 4000)
  console.log("\n2. Testing GraphQL API (port 4000)");
  console.log("   ---------------------------------");

  const graphqlClient = new Client({
    baseUrl: "http://localhost:4000",
    timeout: 10000,
  });

  try {
    // Test GraphQL endpoint with a simple query
    console.log("   Testing GraphQL endpoint...");
    const graphqlResponse = await fetch("http://localhost:4000/graphql", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        query: `query { __typename }`,
      }),
    });

    if (graphqlResponse.ok) {
      const data = await graphqlResponse.json();
      if (data.data) {
        console.log("   ✅ GraphQL endpoint responding");
      } else {
        console.log("   ❌ GraphQL endpoint returned errors:", data.errors);
      }
    } else {
      console.log(`   ❌ GraphQL endpoint failed: ${graphqlResponse.status}`);
    }

    // Test some GraphQL operations
    console.log("   Testing workflow operations...");
    try {
      const workflows = graphqlClient.workflows();
      const workflowList = await workflows.list();
      console.log(`   ✅ Found ${workflowList.length} workflows`);
    } catch (error) {
      console.log(`   ❌ Workflow operations failed: ${error}`);
    }

  } catch (error) {
    console.log(`   ❌ GraphQL API test failed: ${error}`);
    console.log("   Make sure the Circuit Breaker server is running on port 4000");
  }

  // Summary
  console.log("\n📋 Connection Test Summary");
  console.log("   ========================");
  console.log("   🔗 Port 3000: OpenAI-compatible REST API for LLM conversations");
  console.log("   🔗 Port 4000: GraphQL API for workflows, agents, rules, etc.");
  console.log("\n💡 Usage:");
  console.log("   - Use port 3000 for all LLM/chat operations");
  console.log("   - Use port 4000 for workflow management and business logic");
  console.log("   - Both APIs can be used simultaneously by different clients");
}

// Run the test
testConnection().catch((error) => {
  console.error("Connection test failed:", error);
  process.exit(1);
});
