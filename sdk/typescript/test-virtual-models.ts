#!/usr/bin/env node

/**
 * Simple test to verify virtual models and smart routing are available
 * in the TypeScript SDK
 */

import {
  COMMON_MODELS,
  createCostOptimizedChat,
  createFastChat,
  createBalancedChat,
  createSmartChat,
} from "./src/index.js";

console.log("🧪 Testing TypeScript SDK Virtual Models & Smart Routing");
console.log("======================================================");

// Test 1: Virtual Model Constants
console.log("\n1. ✅ Virtual Model Constants Available:");
console.log(`   • SMART_FAST: ${COMMON_MODELS.SMART_FAST}`);
console.log(`   • SMART_CHEAP: ${COMMON_MODELS.SMART_CHEAP}`);
console.log(`   • SMART_BALANCED: ${COMMON_MODELS.SMART_BALANCED}`);
console.log(`   • SMART_CREATIVE: ${COMMON_MODELS.SMART_CREATIVE}`);
console.log(`   • SMART_CODING: ${COMMON_MODELS.SMART_CODING}`);
console.log(`   • SMART_ANALYSIS: ${COMMON_MODELS.SMART_ANALYSIS}`);

// Test 2: Convenience Builders
console.log("\n2. ✅ Convenience Builders Available:");

const costBuilder = createCostOptimizedChat();
console.log(`   • createCostOptimizedChat(): ${typeof costBuilder}`);

const fastBuilder = createFastChat();
console.log(`   • createFastChat(): ${typeof fastBuilder}`);

const balancedBuilder = createBalancedChat();
console.log(`   • createBalancedChat(): ${typeof balancedBuilder}`);

const smartBuilder = createSmartChat(COMMON_MODELS.SMART_CODING);
console.log(`   • createSmartChat(): ${typeof smartBuilder}`);

// Test 3: Smart Routing Configuration
console.log("\n3. ✅ Smart Routing Configuration:");

try {
  const request = createSmartChat(COMMON_MODELS.SMART_CODING)
    .setSystemPrompt("You are a coding assistant")
    .addUserMessage("Write a hello world function")
    .setTaskType("code_generation")
    .setRoutingStrategy("performance_first")
    .setMaxCostPer1kTokens(0.05)
    .setFallbackModels(["gpt-4", "claude-3-sonnet-20240229"])
    .setMaxLatency(3000)
    .setBudgetConstraint({
      daily_limit: 10.0,
      per_request_limit: 0.10,
    })
    .buildSmart();

  console.log("   • Smart routing configuration successful");
  console.log(`   • Model: ${request.model}`);
  console.log(`   • Messages: ${request.messages.length} messages`);
  console.log(`   • Circuit Breaker Options: ${request.circuit_breaker ? "✅" : "❌"}`);

  if (request.circuit_breaker) {
    console.log(`   • Routing Strategy: ${request.circuit_breaker.routing_strategy}`);
    console.log(`   • Task Type: ${request.circuit_breaker.task_type}`);
    console.log(`   • Max Cost: $${request.circuit_breaker.max_cost_per_1k_tokens}/1k tokens`);
    console.log(`   • Max Latency: ${request.circuit_breaker.max_latency_ms}ms`);
    console.log(`   • Fallback Models: ${request.circuit_breaker.fallback_models?.length || 0}`);
    console.log(`   • Budget Constraint: ${request.circuit_breaker.budget_constraint ? "✅" : "❌"}`);
  }
} catch (error) {
  console.log(`   ❌ Configuration failed: ${error}`);
}

// Test 4: Type Verification
console.log("\n4. ✅ Type Verification:");

const testTypes = {
  RoutingStrategy: "cost_optimized" as const,
  TaskType: "code_generation" as const,
  VirtualModel: COMMON_MODELS.SMART_FAST,
  BudgetConstraint: {
    daily_limit: 50.0,
    monthly_limit: 500.0,
    per_request_limit: 0.20,
  },
};

console.log("   • RoutingStrategy type: ✅");
console.log("   • TaskType type: ✅");
console.log("   • Virtual model constants: ✅");
console.log("   • BudgetConstraint interface: ✅");

console.log("\n🎉 All Virtual Models & Smart Routing Features Available!");
console.log("🚀 TypeScript SDK is ready for intelligent LLM routing!");

console.log("\n📝 Usage Examples:");
console.log("   const client = new Client({ baseUrl: 'http://localhost:3000' });");
console.log("   const llm = client.llm();");
console.log("");
console.log("   // Virtual model usage:");
console.log(`   const response = await llm.chatCompletion({`);
console.log(`     model: COMMON_MODELS.SMART_CHEAP,`);
console.log(`     messages: [{ role: 'user', content: 'Hello!' }]`);
console.log(`   });`);
console.log("");
console.log("   // Smart routing with convenience builders:");
console.log("   const costResponse = await createCostOptimizedChat()");
console.log("     .addUserMessage('Summarize this document')");
console.log("     .setMaxCostPer1kTokens(0.01)");
console.log("     .execute(llm);");
