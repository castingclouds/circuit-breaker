#!/usr/bin/env node

/**
 * Circuit Breaker TypeScript SDK - Smart Routing Demo
 *
 * This example demonstrates the Circuit Breaker smart routing features including:
 * - Virtual models with intelligent routing
 * - Cost optimization and performance tuning
 * - Task-specific routing
 * - Budget constraints and fallback models
 * - Streaming with smart selection
 */

import {
  Client,
  COMMON_MODELS,
  createCostOptimizedChat,
  createFastChat,
  createBalancedChat,
  createSmartChat,
  SmartCompletionRequest,
  CircuitBreakerOptions,
  ChatMessage,
  BudgetConstraint,
} from "../src/index.js";

async function main() {
  console.log("🤖 Circuit Breaker TypeScript SDK - Smart Routing Demo");
  console.log("======================================================");
  console.log("🎯 Showcasing Virtual Models & Intelligent Routing");

  // Initialize the client for LLM API (OpenAI-compatible REST API on port 3000)
  const llmBaseUrl =
    process.env.CIRCUIT_BREAKER_LLM_URL || "http://localhost:3000";
  const apiKey = process.env.CIRCUIT_BREAKER_API_KEY;

  const client = new Client({
    baseUrl: llmBaseUrl,
    apiKey,
    timeout: 30000,
  });

  console.log(`🔗 Connected to Circuit Breaker LLM router at: ${llmBaseUrl}`);
  console.log("📡 All LLM calls will be routed through the Circuit Breaker");

  // Test connection to the LLM API
  try {
    // Test with a simple model list request instead of ping
    const llm = client.llm();
    await llm.listModels();
    console.log(`✅ Circuit Breaker LLM API: connected`);
  } catch (error) {
    console.log(`❌ Failed to connect to LLM API: ${error}`);
    console.log(
      "   Make sure the Circuit Breaker server is running on port 3000",
    );
    console.log(
      "   The LLM API uses REST endpoints (/v1/chat/completions, /v1/models)",
    );
    return;
  }

  const llm = client.llm();

  // ============================================================================
  // 1. List Available Models
  // ============================================================================
  console.log("\n1. 📋 Available Models");
  console.log("   ------------------");

  try {
    const models = await llm.listModels();
    console.log("   Available models through Circuit Breaker router:");
    models.forEach((model) => {
      console.log(`   • ${model.id} (${model.owned_by})`);
    });
  } catch (error) {
    console.log(`   ⚠️  Could not fetch models: ${error}`);
    console.log("   Using predefined virtual models...");
  }

  // ============================================================================
  // 2. Virtual Model Demonstration
  // ============================================================================
  console.log("\n2. 🎯 Virtual Model Smart Routing");
  console.log("   --------------------------------");

  const virtualModels = [
    ["💰 Cost-Optimized", COMMON_MODELS.SMART_CHEAP],
    ["⚡ Performance-First", COMMON_MODELS.SMART_FAST],
    ["⚖️  Balanced", COMMON_MODELS.SMART_BALANCED],
    ["🎨 Creative", COMMON_MODELS.SMART_CREATIVE],
    ["💻 Coding", COMMON_MODELS.SMART_CODING],
    ["📊 Analysis", COMMON_MODELS.SMART_ANALYSIS],
  ];

  for (const [name, virtualModel] of virtualModels) {
    console.log(`   Testing ${name}`);
    const startTime = Date.now();

    try {
      const response = await llm.chatCompletion({
        model: virtualModel,
        messages: [
          {
            role: "user",
            content:
              "Explain circuit breaker pattern in software in one sentence.",
          },
        ],
        max_tokens: 100,
      });

      const duration = Date.now() - startTime;
      const content = response.choices[0]?.message?.content || "No response";
      console.log(`   ✅ ${name} (${duration}ms): ${content}`);
    } catch (error) {
      console.log(`   ❌ ${name} failed: ${error}`);
    }
  }

  // ============================================================================
  // 3. Smart Completion with Circuit Breaker Options
  // ============================================================================
  console.log("\n3. 🧠 Smart Completion with Routing Options");
  console.log("   ------------------------------------------");

  const smartRequest: SmartCompletionRequest = {
    model: COMMON_MODELS.SMART_CHEAP,
    messages: [
      {
        role: "system",
        content: "You are a cost-conscious AI assistant.",
      },
      {
        role: "user",
        content: "Write a short explanation of microservices architecture.",
      },
    ],
    temperature: 0.7,
    max_tokens: 150,
    stream: false,
    circuit_breaker: {
      routing_strategy: "cost_optimized",
      max_cost_per_1k_tokens: 0.01,
      task_type: "general",
      fallback_models: ["o4-mini-2025-04-16", "claude-sonnet-4-20250514"],
      max_latency_ms: 5000,
      require_streaming: false,
    },
  };

  try {
    const response = await llm.smartCompletion(smartRequest);
    const content = response.choices[0]?.message?.content || "No response";
    console.log(`   🧠 Smart routed response: ${content}`);
    if (response.usage) {
      console.log(
        `   💰 Cost-optimized tokens: ${response.usage.total_tokens} total`,
      );
    }
  } catch (error) {
    console.log(`   ⚠️  Smart completion failed: ${error}`);
  }

  // ============================================================================
  // 4. Task-Specific Optimization
  // ============================================================================
  console.log("\n4. 🎯 Task-Specific Smart Routing");
  console.log("   --------------------------------");

  // Code generation task
  try {
    const codeResponse = await createSmartChat(COMMON_MODELS.SMART_CODING)
      .setSystemPrompt("You are an expert programmer.")
      .addUserMessage(
        "Write a TypeScript function to sort an array using quicksort",
      )
      .setTaskType("coding")
      .setRoutingStrategy("performance_first")
      .setMaxCostPer1kTokens(0.05)
      .execute(llm);

    const codeContent =
      codeResponse.choices[0]?.message?.content || "No response";
    console.log(`   💻 Code generation: ${codeContent}`);
  } catch (error) {
    console.log(`   ⚠️  Code generation failed: ${error}`);
  }

  // Creative writing task
  try {
    const creativeResponse = await createSmartChat(COMMON_MODELS.SMART_CREATIVE)
      .addUserMessage("Write a haiku about distributed systems")
      .setTaskType("creative")
      .setTemperature(0.9)
      .execute(llm);

    const creativeContent =
      creativeResponse.choices[0]?.message?.content || "No response";
    console.log(`   🎨 Creative writing: ${creativeContent}`);
  } catch (error) {
    console.log(`   ⚠️  Creative writing failed: ${error}`);
  }

  // ============================================================================
  // 5. Convenience Builder Functions
  // ============================================================================
  console.log("\n5. 🛠️  Convenience Builder Functions");
  console.log("   ----------------------------------");

  // Cost-optimized builder
  try {
    const costResponse = await createCostOptimizedChat()
      .addUserMessage("Summarize the benefits of serverless computing")
      .setMaxCostPer1kTokens(0.005)
      .execute(llm);

    const costContent =
      costResponse.choices[0]?.message?.content || "No response";
    console.log(`   💰 Cost-optimized: ${costContent}`);
  } catch (error) {
    console.log(`   ⚠️  Cost-optimized failed: ${error}`);
  }

  // Performance-first builder
  try {
    const fastResponse = await createFastChat()
      .addUserMessage("Quick: What is Docker?")
      .execute(llm);

    const fastContent =
      fastResponse.choices[0]?.message?.content || "No response";
    console.log(`   ⚡ Performance-first: ${fastContent}`);
  } catch (error) {
    console.log(`   ⚠️  Performance-first failed: ${error}`);
  }

  // Balanced approach
  try {
    const balancedResponse = await createBalancedChat()
      .addUserMessage(
        "Explain the trade-offs between monolithic and microservices architectures",
      )
      .setFallbackModels(["gpt-4", "claude-3-sonnet-20240229"])
      .execute(llm);

    const balancedContent =
      balancedResponse.choices[0]?.message?.content || "No response";
    console.log(`   ⚖️  Balanced: ${balancedContent}`);
  } catch (error) {
    console.log(`   ⚠️  Balanced failed: ${error}`);
  }

  // ============================================================================
  // 6. Test Virtual Models (Non-Streaming First)
  // ============================================================================
  console.log("\n6. 🧪 Testing Virtual Models (Non-Streaming)");
  console.log("   -------------------------------------------");

  try {
    console.log("   Testing cb:creative virtual model...");
    const virtualResponse = await llm.chatCompletion({
      model: COMMON_MODELS.SMART_CREATIVE,
      messages: [{ role: "user", content: "Say 'Hello from virtual model!'" }],
      max_tokens: 20,
    });
    const virtualContent =
      virtualResponse.choices[0]?.message?.content || "No response";
    console.log(`   ✅ Virtual model works: ${virtualContent}`);
  } catch (error) {
    console.log(`   ❌ Virtual model failed: ${error}`);
  }

  // ============================================================================
  // 7. Smart Streaming with Virtual Models
  // ============================================================================
  console.log("\n7. 🌊 Smart Streaming with Virtual Models");
  console.log("   ----------------------------------------");

  try {
    const streamBuilder = createSmartChat(COMMON_MODELS.SMART_FAST)
      .addUserMessage(
        "Write a short story about a circuit breaker in distributed systems.",
      )
      .setTemperature(0.8)
      .setMaxTokens(300)
      .setStream(true)
      .setCircuitBreakerOptions({
        routing_strategy: "performance_first",
        task_type: "general_chat",
        require_streaming: true,
        max_latency_ms: 3000,
        fallback_models: ["gpt-4", "claude-3-opus-20240229"],
      });

    const streamingRequest = streamBuilder.build();
    console.log("   🌊 Starting streaming response...");

    process.stdout.write("   📝 Story: ");

    await llm.streamChatCompletion(
      streamingRequest,
      (chunk) => {
        if (chunk.choices[0]?.delta?.content) {
          process.stdout.write(chunk.choices[0].delta.content);
        }
      },
      (error) => {
        console.log(`\n   ❌ Streaming error: ${error}`);
      },
    );

    console.log("\n   ✅ Streaming completed");
  } catch (error) {
    console.log(`   ⚠️  Streaming failed: ${error}`);
  }

  // ============================================================================
  // 8. Budget-Constrained Routing
  // ============================================================================
  console.log("\n8. 💰 Budget-Constrained Smart Routing");
  console.log("   -------------------------------------");

  const budgetConstraint: BudgetConstraint = {
    daily_limit: 10.0,
    monthly_limit: 100.0,
    per_request_limit: 0.05,
  };

  try {
    const budgetResponse = await createSmartChat(COMMON_MODELS.SMART_BALANCED)
      .setSystemPrompt("You are a budget-conscious technical writer.")
      .addUserMessage("Explain Circuit Breaker pattern benefits in 2 sentences")
      .setCircuitBreakerOptions({
        routing_strategy: "cost_optimized",
        max_cost_per_1k_tokens: 0.01,
        task_type: "general",
        budget_constraint: budgetConstraint,
        fallback_models: ["o4-mini-2025-04-16", "claude-sonnet-4-20250514"],
        max_latency_ms: 4000,
        require_streaming: false,
      })
      .execute(llm);

    const budgetContent =
      budgetResponse.choices[0]?.message?.content || "No response";
    console.log(`   💰 Budget-aware response: ${budgetContent}`);
    console.log(`   📋 Response ID: ${budgetResponse.id}`);
    console.log(`   🤖 Model selected: ${budgetResponse.model}`);
    if (budgetResponse.usage) {
      console.log(`   📊 Tokens used: ${budgetResponse.usage.total_tokens}`);
    }
  } catch (error) {
    console.log(`   ⚠️  Budget-constrained request failed: ${error}`);
  }

  // ============================================================================
  // 9. Advanced Routing with All Features
  // ============================================================================
  console.log("\n9. 🔧 Advanced Routing Configuration");
  console.log("   ----------------------------------");

  const advancedOptions: CircuitBreakerOptions = {
    routing_strategy: "cost_optimized",
    max_cost_per_1k_tokens: 0.02,
    max_latency_ms: 3000,
    task_type: "analysis",
    fallback_models: ["o4-mini-2025-04-16", "claude-sonnet-4-20250514"],
    require_streaming: false,
    budget_constraint: {
      daily_limit: 50.0,
      per_request_limit: 0.1,
    },
  };

  try {
    const advancedResponse = await createSmartChat(COMMON_MODELS.SMART_ANALYSIS)
      .setSystemPrompt("You are a data analysis expert.")
      .addUserMessage(
        "Analyze the trade-offs between different database types for a high-traffic web application",
      )
      .setTemperature(0.3)
      .setMaxTokens(200)
      .setCircuitBreakerOptions(advancedOptions)
      .executeSmart(llm);

    const advancedContent =
      advancedResponse.choices[0]?.message?.content || "No response";
    console.log(`   🔧 Advanced analysis: ${advancedContent}`);
    if (advancedResponse.usage) {
      console.log(
        `   📈 Analysis tokens: ${advancedResponse.usage.total_tokens}`,
      );
    }
  } catch (error) {
    console.log(`   ⚠️  Advanced routing failed: ${error}`);
  }

  // ============================================================================
  // Summary
  // ============================================================================
  console.log("\n✨ Circuit Breaker Smart Router Demo Complete!");
  console.log("===============================================");
  console.log("🎯 Key Features Demonstrated:");
  console.log(
    "   • 🎯 Virtual Models (smart-fast, smart-cheap, smart-balanced, etc.)",
  );
  console.log("   • 🧠 Smart Routing with cost/performance optimization");
  console.log("   • 🎨 Task-specific routing (code, creative, analysis)");
  console.log("   • 💰 Budget-constrained routing with cost limits");
  console.log("   • ⚡ Performance-first vs cost-optimized strategies");
  console.log("   • 🌊 Smart streaming with fallback models");
  console.log("   • 🛠️  Convenience builders for common patterns");
  console.log("   • 🔄 Automatic failover and load balancing");
  console.log("\n🚀 Circuit Breaker goes beyond simple API routing!");
  console.log("   It provides intelligent model selection, cost optimization,");
  console.log(
    "   and task-aware routing while maintaining OpenAI compatibility!",
  );
}

// Run the demo
main().catch((error) => {
  console.error("Demo failed:", error);
  process.exit(1);
});
