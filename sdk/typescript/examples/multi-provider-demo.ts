/**
 * Multi-Provider LLM Demo - TypeScript Edition
 *
 * This demo showcases the Circuit Breaker LLM Router's multi-provider capabilities,
 * demonstrating OpenAI, Anthropic, and Google Gemini integration with cost tracking,
 * streaming, and smart routing features.
 *
 * Architecture Note:
 * The Circuit Breaker server normalizes all provider responses to OpenAI-compatible
 * format, enabling a single client interface regardless of the underlying provider.
 * This works seamlessly with virtual models that can route to any provider.
 *
 * Prerequisites:
 * 1. Circuit Breaker server running on ports 3000 (OpenAI API) and 4000 (GraphQL)
 * 2. API keys configured in server's .env file
 * 3. Run with: npx tsx multi-provider-demo.ts
 */

import * as readline from "readline";
import { Client, COMMON_MODELS } from "../src/index.js";

// Types and interfaces
interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string }>;
}

interface LLMProvider {
  id: string;
  providerType: "openai" | "anthropic" | "google";
  name: string;
  baseUrl: string;
  models: LLMModel[];
  healthStatus: {
    isHealthy: boolean;
    errorRate: number;
    averageLatencyMs: number;
    lastCheck?: string;
  };
}

interface LLMModel {
  id: string;
  name: string;
  maxTokens?: number;
  contextWindow?: number;
  costPerInputToken: number;
  costPerOutputToken: number;
  supportsStreaming: boolean;
  supportsFunctionCalling: boolean;
  capabilities?: string[];
}

interface ChatCompletionRequest {
  model: string;
  messages: Array<{
    role: "system" | "user" | "assistant";
    content: string;
  }>;
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  circuit_breaker?: {
    routing_strategy?: string;
    max_cost_per_1k_tokens?: number;
    task_type?: string;
  };
}

interface ChatCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message: {
      role: string;
      content: string;
    };
    finish_reason: string;
  }>;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  system_fingerprint?: string;
}

interface ProviderComparison {
  provider: string;
  model: string;
  response: string;
  tokens: number;
  estimatedCost: number;
  latencyMs: number;
  success: boolean;
}

interface StreamingChunk {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Array<{
    index: number;
    delta: {
      role?: string;
      content?: string;
    };
    finish_reason?: string;
  }>;
  provider?: string;
}

// SSE Parser for handling different provider streaming formats
class SSEParser {
  private buffer: string = "";

  parseChunk(chunk: Uint8Array): Array<{ data: string }> {
    const text = new TextDecoder().decode(chunk);
    this.buffer += text;

    const events: Array<{ data: string }> = [];

    while (true) {
      const doubleNewlineIndex = this.buffer.indexOf("\n\n");
      if (doubleNewlineIndex === -1) break;

      const eventBlock = this.buffer.slice(0, doubleNewlineIndex);
      this.buffer = this.buffer.slice(doubleNewlineIndex + 2);

      for (const line of eventBlock.split("\n")) {
        if (line.startsWith("data: ")) {
          const data = line.slice(6).trim();
          if (data && data !== "[DONE]") {
            events.push({ data });
          }
        }
      }
    }

    return events;
  }

  hasRemainingData(): boolean {
    return this.buffer.trim().length > 0;
  }

  flushRemaining(): Array<{ data: string }> {
    const events: Array<{ data: string }> = [];
    if (this.buffer.trim()) {
      // Try to parse remaining buffer as potential data
      for (const line of this.buffer.split("\n")) {
        if (line.startsWith("data: ")) {
          const data = line.slice(6).trim();
          if (data && data !== "[DONE]") {
            events.push({ data });
          }
        }
      }
    }
    this.buffer = "";
    return events;
  }
}

// Unified SSE parser - server normalizes all provider responses to OpenAI format
class UniversalSSEParser {
  static parseEvent(
    event: { data: string },
    requestId: string,
    model: string,
  ): StreamingChunk | null {
    if (!event.data.trim() || event.data.trim() === "[DONE]") {
      return null;
    }

    try {
      const chunk = JSON.parse(event.data);

      if (chunk.choices && chunk.choices[0]?.delta?.content) {
        return {
          id: chunk.id,
          object: chunk.object,
          created: chunk.created,
          model: chunk.model,
          choices: [
            {
              index: chunk.choices[0].index,
              delta: {
                role: chunk.choices[0].delta.role || "assistant",
                content: chunk.choices[0].delta.content,
              },
              finish_reason: chunk.choices[0].finish_reason,
            },
          ],
          provider: "openai",
        };
      }

      return null;
    } catch (e) {
      console.error("Failed to parse OpenAI stream chunk:", e);
      return null;
    }
  }
}

class MultiProviderDemo {
  private client: Client;
  private readonly graphqlUrl = "http://localhost:4000/graphql";
  private readonly openaiApiUrl = "http://localhost:8081/v1/chat/completions";

  constructor() {
    // Initialize Circuit Breaker SDK client
    let clientBuilder = Client.builder()
      .baseUrl(process.env.CIRCUIT_BREAKER_URL || "http://localhost:4000")
      .timeout(30000);

    if (process.env.CIRCUIT_BREAKER_API_KEY) {
      clientBuilder = clientBuilder.apiKey(process.env.CIRCUIT_BREAKER_API_KEY);
    }

    this.client = clientBuilder.build();
  }

  async main(): Promise<void> {
    console.log(
      "🤖 Circuit Breaker Multi-Provider LLM Demo - TypeScript Edition",
    );
    console.log(
      "==================================================================",
    );
    console.log();

    console.log("🔑 Multi-Provider Architecture:");
    console.log("   📊 OpenAI: GPT-4, GPT-3.5, o4 models");
    console.log("   🧠 Anthropic: Claude 3 Haiku, Sonnet, Opus");
    console.log("   🔍 Google: Gemini Pro, Flash, Vision models");
    console.log("   🎯 Smart Routing: Auto-select optimal provider");
    console.log();

    try {
      // Test server connectivity
      await this.testServerConnectivity();

      await this.waitForEnter("Ready to explore multi-provider capabilities?");

      // 1. List and analyze all providers
      await this.listProviders();

      await this.waitForEnter(
        "Providers analyzed! Ready to test provider-specific models?",
      );

      // 2. Test each provider individually
      await this.testIndividualProviders();

      await this.waitForEnter(
        "Individual tests complete! Ready for cost comparison?",
      );

      // 3. Cost comparison across providers
      await this.compareCosts();

      await this.waitForEnter(
        "Cost analysis done! Ready to test streaming across providers?",
      );

      // 4. Streaming comparison
      await this.testStreamingAcrossProviders();

      await this.waitForEnter(
        "Streaming tests complete! Ready for smart routing demo?",
      );

      // 5. Smart routing demonstration
      await this.demonstrateSmartRouting();

      await this.waitForEnter(
        "Smart routing demo complete! Ready for advanced features?",
      );

      // 6. Routing behavior analysis
      await this.analyzeRoutingBehavior();

      await this.waitForEnter(
        "Routing analysis complete! Ready for advanced features?",
      );

      // 7. Advanced features
      await this.testAdvancedFeatures();

      // 8. Final summary
      await this.printSummary();
    } catch (error) {
      console.error("❌ Demo failed:", error);
      process.exit(1);
    }
  }

  private async testServerConnectivity(): Promise<void> {
    console.log("🔗 Testing server connectivity...");

    try {
      // Test Circuit Breaker connection
      const ping = await this.client.ping();
      console.log(`✅ Circuit Breaker server v${ping.version} connected`);

      // Test OpenAI API endpoint
      const openaiResponse = await fetch("http://localhost:8081/v1/models");
      if (openaiResponse.ok) {
        console.log("✅ OpenAI API endpoint accessible");
      } else {
        throw new Error("OpenAI API endpoint not accessible");
      }
    } catch (error) {
      console.log("❌ Server not responding. Please start the server first:");
      console.log("   cargo run --bin server");
      throw error;
    }
  }

  private async listProviders(): Promise<LLMProvider[]> {
    console.log("\n📊 1. Provider Discovery & Analysis");
    console.log("===================================");

    const query = `
      query {
        llmProviders {
          id
          providerType
          name
          baseUrl
          models {
            id
            name
            maxTokens
            contextWindow
            costPerInputToken
            costPerOutputToken
            supportsStreaming
            supportsFunctionCalling
            capabilities
          }
          healthStatus {
            isHealthy
            errorRate
            averageLatencyMs
            lastCheck
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{ llmProviders: LLMProvider[] }>(
      query,
    );
    const providers = response.llmProviders;

    console.log(`✅ Found ${providers.length} providers configured:`);
    console.log();

    for (const provider of providers) {
      const status = provider.healthStatus.isHealthy
        ? "🟢 Healthy"
        : "🔴 Unhealthy";
      console.log(
        `🏢 ${provider.name} (${provider.providerType.toUpperCase()})`,
      );
      console.log(`   Status: ${status}`);
      console.log(`   Base URL: ${provider.baseUrl}`);
      console.log(`   Models: ${provider.models.length}`);

      // Show top 3 models with cost info
      const topModels = provider.models.slice(0, 3);
      for (const model of topModels) {
        const inputCost = (model.costPerInputToken * 1000).toFixed(4);
        const outputCost = (model.costPerOutputToken * 1000).toFixed(4);
        console.log(
          `     • ${model.name}: $${inputCost}/$${outputCost} per 1K tokens`,
        );
      }

      if (provider.models.length > 3) {
        console.log(`     ... and ${provider.models.length - 3} more models`);
      }
      console.log();
    }

    return providers;
  }

  private async testIndividualProviders(): Promise<void> {
    console.log("\n🧪 2. Individual Provider Testing");
    console.log("=================================");

    // Test specific models from each provider
    const testCases = [
      {
        provider: "OpenAI",
        model: "o4-mini-2025-04-16",
        prompt:
          "Explain what makes you unique as an AI assistant in one sentence.",
      },
      {
        provider: "Anthropic",
        model: "claude-3-haiku-20240307",
        prompt:
          "Explain what makes you unique as an AI assistant in one sentence.",
      },
      {
        provider: "Google",
        model: "gemini-1.5-flash",
        prompt: "Say hello and introduce yourself briefly.",
      },
    ];

    for (const testCase of testCases) {
      console.log(`\n🔧 Testing ${testCase.provider} (${testCase.model}):`);

      try {
        const startTime = Date.now();
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: testCase.model,
            messages: [{ role: "user", content: testCase.prompt }],
            max_tokens: testCase.model.includes("gemini") ? 1000 : 100,
          } as ChatCompletionRequest),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          const latency = Date.now() - startTime;

          console.log(`   ✅ Success (${latency}ms)`);
          console.log(
            `   💬 Response: "${result.choices[0].message.content.substring(0, 80)}..."`,
          );
          console.log(
            `   📊 Tokens: ${result.usage.total_tokens} (${result.usage.prompt_tokens} + ${result.usage.completion_tokens})`,
          );
        } else {
          const error = await response.text();
          console.log(`   ❌ Failed: ${error.substring(0, 100)}...`);
        }
      } catch (error) {
        console.log(`   ❌ Error: ${error}`);
      }
    }
  }

  private async compareCosts(): Promise<void> {
    console.log("\n💰 3. Cost Comparison Analysis");
    console.log("==============================");

    const models = [
      {
        name: "o4-mini-2025-04-16",
        prompt: "Write a haiku about artificial intelligence.",
      },
      {
        name: "claude-3-haiku-20240307",
        prompt: "Write a haiku about artificial intelligence.",
      },
      {
        name: "gemini-1.5-flash",
        prompt: "Write a short haiku.",
      },
    ];

    const results: ProviderComparison[] = [];

    for (const model of models) {
      console.log(`\n💸 Testing cost for ${model.name}:`);

      try {
        const startTime = Date.now();
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: model.name,
            messages: [{ role: "user", content: model.prompt }],
            max_tokens: model.name.includes("gemini") ? 1000 : 50,
          } as ChatCompletionRequest),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          const latency = Date.now() - startTime;

          // Estimate cost (simplified calculation)
          const estimatedCost = this.estimateCost(model.name, result.usage);

          results.push({
            provider: this.getProviderFromModel(model.name),
            model: model.name,
            response: result.choices[0].message.content,
            tokens: result.usage.total_tokens,
            estimatedCost,
            latencyMs: latency,
            success: true,
          });

          console.log(`   ✅ Success`);
          console.log(`   📝 Response: "${result.choices[0].message.content}"`);
          console.log(`   💰 Estimated cost: $${estimatedCost.toFixed(6)}`);
          console.log(`   ⏱️  Latency: ${latency}ms`);
        } else {
          results.push({
            provider: this.getProviderFromModel(model.name),
            model: model.name,
            response: "Failed",
            tokens: 0,
            estimatedCost: 0,
            latencyMs: 0,
            success: false,
          });
          console.log(`   ❌ Failed`);
        }
      } catch (error) {
        console.log(`   ❌ Error: ${error}`);
      }
    }

    // Print cost comparison table
    console.log("\n📈 Cost Comparison Summary:");
    console.log(
      "┌─────────────┬──────────────────┬─────────────┬───────────┬───────────┐",
    );
    console.log(
      "│ Provider    │ Model            │ Cost ($)    │ Tokens    │ Latency   │",
    );
    console.log(
      "├─────────────┼──────────────────┼─────────────┼───────────┼───────────┤",
    );

    for (const result of results.filter((r) => r.success)) {
      const provider = result.provider.padEnd(11);
      const model = result.model.substring(0, 16).padEnd(16);
      const cost = result.estimatedCost.toFixed(6).padStart(11);
      const tokens = result.tokens.toString().padStart(9);
      const latency = `${result.latencyMs}ms`.padStart(9);
      console.log(
        `│ ${provider} │ ${model} │ ${cost} │ ${tokens} │ ${latency} │`,
      );
    }
    console.log(
      "└─────────────┴──────────────────┴─────────────┴───────────┴───────────┘",
    );

    // Find most cost-effective
    const successfulResults = results.filter((r) => r.success);
    if (successfulResults.length > 0) {
      const cheapest = successfulResults.reduce((a, b) =>
        a.estimatedCost < b.estimatedCost ? a : b,
      );
      console.log(
        `\n🏆 Most Cost-Effective: ${cheapest.model} (${cheapest.provider}) - $${cheapest.estimatedCost.toFixed(6)}`,
      );
    }
  }

  private async testStreamingAcrossProviders(): Promise<void> {
    console.log("\n🌊 4. Multi-Provider Real-Time Streaming Test");
    console.log("==============================================");

    // Test with multiple models to show streaming across ALL providers
    const streamingModels = [
      {
        name: "OpenAI GPT-4",
        model: "o4-mini-2025-04-16",
        prompt: "Write a short story about a robot learning to paint",
        provider: "openai",
      },
      {
        name: "Anthropic Claude",
        model: "claude-3-haiku-20240307",
        prompt: "Write a short story about a robot learning to paint",
        provider: "anthropic",
      },
      {
        name: "Google Gemini",
        model: "gemini-1.5-flash",
        prompt: "Write a short story about a robot learning to paint",
        provider: "google",
      },
    ];

    for (const testModel of streamingModels) {
      console.log(`\n🌊 ${testModel.name} - "${testModel.prompt}"`);
      console.log("Response:");

      try {
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Accept: "text/event-stream",
          },
          body: JSON.stringify({
            model: testModel.model,
            messages: [{ role: "user", content: testModel.prompt }],
            max_tokens: testModel.model.includes("gemini") ? 5000 : 300,
            temperature: 0.7,
            stream: true,
            metadata: { provider: testModel.provider },
          } as ChatCompletionRequest),
        });

        if (response.ok && response.body) {
          // Use SSE parser for proper streaming
          const parser = new SSEParser();
          const reader = response.body.getReader();

          try {
            while (true) {
              const { done, value } = await reader.read();
              if (done) break;

              const events = parser.parseChunk(value);

              for (const event of events) {
                let chunk: StreamingChunk | null = null;

                // Server normalizes all responses to OpenAI format
                chunk = UniversalSSEParser.parseEvent(
                  event,
                  `req-${Date.now()}`,
                  testModel.model,
                );

                if (chunk && chunk.choices[0]?.delta?.content) {
                  const content = chunk.choices[0].delta.content;
                  process.stdout.write(content);
                }
              }
            }

            // Process any remaining buffer
            const remainingEvents = parser.flushRemaining();
            for (const event of remainingEvents) {
              let chunk: StreamingChunk | null = null;

              // Server normalizes all responses to OpenAI format
              chunk = UniversalSSEParser.parseEvent(
                event,
                `req-${Date.now()}`,
                testModel.model,
              );

              if (chunk && chunk.choices[0]?.delta?.content) {
                const content = chunk.choices[0].delta.content;
                process.stdout.write(content);
              }
            }
          } finally {
            reader.releaseLock();
          }

          console.log("\n");
        } else {
          console.log(
            `❌ ${testModel.provider} streaming failed: ${response.status} ${response.statusText}`,
          );
        }
      } catch (error) {
        console.log(`❌ ${testModel.provider} streaming failed: ${error}`);
      }
    }
  }

  private async demonstrateSmartRouting(): Promise<void> {
    console.log("\n🧠 5. Smart Routing Demonstration");
    console.log("=================================");

    // First, let's check provider availability and costs
    console.log("\n📊 Provider Analysis for Routing:");
    try {
      const query = `
        query {
          llmProviders {
            id
            name
            providerType
            healthStatus {
              isHealthy
              errorRate
              averageLatencyMs
            }
            models {
              id
              name
              costPerInputToken
              costPerOutputToken
              supportsStreaming
              supportsFunctionCalling
            }
          }
        }
      `;

      const response = await this.graphqlRequest<{
        llmProviders: LLMProvider[];
      }>(query);

      for (const provider of response.llmProviders) {
        const status = provider.healthStatus.isHealthy ? "🟢" : "🔴";
        console.log(`   ${status} ${provider.name} (${provider.providerType})`);
        console.log(
          `      Health: ${provider.healthStatus.isHealthy ? "Healthy" : "Unhealthy"}`,
        );
        console.log(`      Error Rate: ${provider.healthStatus.errorRate}%`);
        console.log(
          `      Avg Latency: ${provider.healthStatus.averageLatencyMs}ms`,
        );

        // Show cheapest model
        const cheapestModel = provider.models.reduce((prev, curr) =>
          prev.costPerInputToken < curr.costPerInputToken ? prev : curr,
        );
        console.log(
          `      Cheapest Model: ${cheapestModel.name} ($${(cheapestModel.costPerInputToken * 1000).toFixed(4)}/1K tokens)`,
        );
      }
    } catch (error) {
      console.log(`   ⚠️  Could not fetch provider analysis: ${error}`);
    }

    const routingScenarios = [
      {
        name: "Cost-Optimized Task",
        prompt: "What is 2+2?",
        config: {
          routing_strategy: "cost_optimized",
          max_cost_per_1k_tokens: 0.001,
        },
      },
      {
        name: "Performance-First Task",
        prompt: "Solve this quickly: What day comes after Monday?",
        config: { routing_strategy: "performance_first" },
      },
      {
        name: "Coding Task",
        prompt: "Write a simple function to reverse a string in Python.",
        config: { routing_strategy: "task_specific", task_type: "coding" },
      },
      {
        name: "Creative Task",
        prompt: "Write a creative short story about a robot learning to paint.",
        config: { routing_strategy: "task_specific", task_type: "creative" },
      },
    ];

    // Test virtual models with debug info
    console.log("\n🎯 Testing Virtual Models:");
    const virtualModels = [
      "auto",
      "cb:cost-optimal",
      "cb:fastest",
      "cb:coding",
    ];

    for (const model of virtualModels) {
      console.log(`\n   Testing virtual model: ${model}`);
      try {
        const requestBody = {
          model,
          messages: [
            { role: "user", content: "Hello! What provider are you?" },
          ],
          max_tokens: 200,
          metadata: {
            debug: true,
            trace_routing: true,
          },
        } as ChatCompletionRequest;

        console.log(
          `   📤 Request: ${JSON.stringify(requestBody, null, 2).substring(0, 200)}...`,
        );

        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(requestBody),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          console.log(`   ✅ ${model} → Routed to: ${result.model}`);
          console.log(
            `   🏷️  System fingerprint: ${result.system_fingerprint || "N/A"}`,
          );
          console.log(
            `   💰 Token usage: ${result.usage?.total_tokens || "N/A"} tokens`,
          );
          console.log(
            `   💬 Response: "${result.choices[0].message.content.substring(0, 60)}..."`,
          );

          // Log response headers for debugging
          const debugHeaders = response.headers.get("x-routing-debug");
          if (debugHeaders) {
            console.log(`   🔍 Routing Debug: ${debugHeaders}`);
          }
        } else {
          const errorText = await response.text();
          console.log(
            `   ❌ ${model} failed: ${response.status} ${response.statusText}`,
          );
          console.log(`   📋 Error details: ${errorText.substring(0, 200)}...`);
        }
      } catch (error) {
        console.log(`   ❌ ${model} error: ${error}`);
      }
    }

    // Test smart routing with circuit breaker config and debug logging
    console.log("\n🎛️  Testing Smart Routing Strategies:");

    for (const scenario of routingScenarios) {
      console.log(`\n   🎯 ${scenario.name}:`);
      console.log(
        `   📋 Strategy: ${JSON.stringify(scenario.config, null, 2)}`,
      );

      try {
        const requestBody = {
          model: "auto",
          messages: [{ role: "user", content: scenario.prompt }],
          max_tokens: 200,
          circuit_breaker: {
            ...scenario.config,
            debug: true,
            trace_routing: true,
          },
          metadata: {
            debug: true,
            scenario: scenario.name,
          },
        } as ChatCompletionRequest;

        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Debug-Routing": "true",
          },
          body: JSON.stringify(requestBody),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          console.log(`   ✅ Strategy: ${scenario.config.routing_strategy}`);
          console.log(`   🎯 Selected: ${result.model || "auto-routed"}`);
          console.log(
            `   💰 Cost estimate: $${this.estimateCost(result.model, result.usage).toFixed(6)}`,
          );
          console.log(
            `   ⏱️  Response time: ${result.created ? new Date(result.created * 1000).toLocaleTimeString() : "N/A"}`,
          );
          console.log(
            `   💬 Preview: "${result.choices[0].message.content.substring(0, 80)}..."`,
          );

          // Check for routing debug info
          const routingInfo = response.headers.get("x-circuit-breaker-routing");
          if (routingInfo) {
            console.log(`   🔍 Routing Info: ${routingInfo}`);
          }

          // Analyze why this provider might have been selected
          const provider = this.getProviderFromModel(result.model);
          console.log(`   📊 Analysis: Selected ${provider} - likely due to:`);

          if (scenario.config.routing_strategy === "cost_optimized") {
            console.log(`      • Cost optimization (model: ${result.model})`);
          } else if (scenario.config.routing_strategy === "performance_first") {
            console.log(`      • Performance priority (lowest latency)`);
          } else if (scenario.config.routing_strategy === "task_specific") {
            console.log(
              `      • Task-specific optimization (${scenario.config.task_type})`,
            );
          }
        } else {
          const errorText = await response.text();
          console.log(
            `   ❌ Failed: ${response.status} ${response.statusText}`,
          );
          console.log(`   📋 Error: ${errorText.substring(0, 200)}...`);
        }
      } catch (error) {
        console.log(`   ❌ Error: ${error}`);
      }
    }

    // Test explicit provider forcing to understand the issue
    console.log("\n🔬 Provider Forcing Test (to debug routing bias):");
    const explicitModels = [
      { name: "OpenAI Direct", model: "o4-mini-2025-04-16" },
      { name: "Anthropic Direct", model: "claude-3-haiku-20240307" },
      { name: "Google Direct", model: "gemini-1.5-flash" },
    ];

    for (const test of explicitModels) {
      console.log(`\n   Testing explicit ${test.name}:`);
      try {
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: test.model,
            messages: [{ role: "user", content: "Hello from " + test.name }],
            max_tokens: 50,
          }),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          console.log(`   ✅ ${test.name}: Success`);
          console.log(`   🎯 Actual model: ${result.model}`);
          console.log(
            `   💰 Cost: $${this.estimateCost(result.model, result.usage).toFixed(6)}`,
          );
        } else {
          const errorText = await response.text();
          console.log(`   ❌ ${test.name}: Failed - ${response.status}`);
          console.log(`   📋 ${errorText.substring(0, 100)}...`);
        }
      } catch (error) {
        console.log(`   ❌ ${test.name}: Error - ${error}`);
      }
    }
  }

  private async analyzeRoutingBehavior(): Promise<void> {
    console.log("\n🔍 6. Routing Behavior Analysis");
    console.log("===============================");

    // Test routing consistency
    console.log("\n🔁 Testing routing consistency (10 identical requests):");
    const testPrompt = "What is 2+2?";
    const routingResults: Record<string, number> = {};

    for (let i = 0; i < 10; i++) {
      try {
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: "auto",
            messages: [{ role: "user", content: testPrompt }],
            max_tokens: 50,
            circuit_breaker: {
              routing_strategy: "cost_optimized",
            },
          }),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          const provider = this.getProviderFromModel(result.model);
          routingResults[provider] = (routingResults[provider] || 0) + 1;
        }
      } catch (error) {
        console.log(`   Request ${i + 1} failed: ${error}`);
      }
    }

    console.log("\n📊 Routing Distribution:");
    for (const [provider, count] of Object.entries(routingResults)) {
      const percentage = ((count / 10) * 100).toFixed(1);
      console.log(`   ${provider}: ${count}/10 requests (${percentage}%)`);
    }

    // Analyze potential reasons for Anthropic bias
    console.log("\n🧐 Potential Reasons for Anthropic Preference:");
    console.log("   1. Cost Optimization:");
    console.log("      • Claude Haiku: ~$0.00025/1K input tokens");
    console.log(
      "      • GPT-4o-mini: ~$0.003/1K input tokens (12x more expensive)",
    );
    console.log("      • Gemini Flash: ~$0.000075/1K input tokens (cheapest)");
    console.log();
    console.log("   2. Health Status:");
    console.log("      • Anthropic provider may have better health metrics");
    console.log("      • Lower error rates or faster response times");
    console.log();
    console.log("   3. Model Capabilities:");
    console.log(
      "      • Task-specific routing may favor Claude for certain tasks",
    );
    console.log("      • Context window or capability matching");

    // Test with different routing strategies
    console.log("\n🎯 Testing Different Routing Strategies:");
    const strategies = [
      {
        name: "Cost Optimized",
        config: { routing_strategy: "cost_optimized" },
      },
      {
        name: "Performance First",
        config: { routing_strategy: "performance_first" },
      },
      { name: "No Strategy", config: {} },
    ];

    for (const strategy of strategies) {
      console.log(`\n   Testing ${strategy.name}:`);
      try {
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: "auto",
            messages: [{ role: "user", content: "Simple test question" }],
            max_tokens: 30,
            circuit_breaker: strategy.config,
          }),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          const provider = this.getProviderFromModel(result.model);
          const cost = this.estimateCost(result.model, result.usage);
          console.log(`      → ${provider} (${result.model})`);
          console.log(`      → Cost: $${cost.toFixed(6)}`);
        }
      } catch (error) {
        console.log(`      → Failed: ${error}`);
      }
    }

    // Recommendations
    console.log("\n💡 Recommendations:");
    if (routingResults["Anthropic"] > 7) {
      console.log("   🎯 Heavy Anthropic bias detected. Consider:");
      console.log("      • Check if OpenAI provider is healthy");
      console.log("      • Verify OpenAI API keys are configured");
      console.log("      • Test explicit OpenAI model calls");
      console.log("      • Review cost optimization settings");
    }
    console.log("   📊 To force specific providers:");
    console.log(
      "      • Use explicit model names (e.g., 'o4-mini-2025-04-16')",
    );
    console.log("      • Configure routing_strategy: 'performance_first'");
    console.log("      • Set max_cost_per_1k_tokens higher for premium models");
  }

  private async testAdvancedFeatures(): Promise<void> {
    console.log("\n🚀 7. Advanced Features");
    console.log("=======================");

    // Test function calling (if supported)
    console.log("\n🔧 Function Calling Test:");
    try {
      const response = await fetch(this.openaiApiUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: "o4-mini-2025-04-16",
          messages: [
            {
              role: "user",
              content:
                "What is the weather like today? Use the get_weather function.",
            },
          ],
          max_tokens: 200,
          functions: [
            {
              name: "get_weather",
              description: "Get current weather for a location",
              parameters: {
                type: "object",
                properties: {
                  location: { type: "string", description: "City name" },
                },
                required: ["location"],
              },
            },
          ],
        }),
      });

      if (response.ok) {
        const result = await response.json();
        console.log("   ✅ Function calling capability detected");
        console.log(
          `   💬 Response type: ${result.choices[0].message.content ? "text" : "function_call"}`,
        );
      } else {
        console.log(
          "   ℹ️  Function calling test skipped (model may not support it)",
        );
      }
    } catch (error) {
      console.log(`   ❌ Function calling test failed: ${error}`);
    }

    // Test with different temperatures
    console.log("\n🌡️  Temperature Variation Test:");
    const temperatures = [0.0, 0.5, 1.0];
    const creativityPrompt =
      "Complete this story starter: 'The last person on Earth sat alone in a room...'";

    for (const temp of temperatures) {
      console.log(`\n   Testing temperature ${temp}:`);
      try {
        const response = await fetch(this.openaiApiUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: "o4-mini-2025-04-16",
            messages: [{ role: "user", content: creativityPrompt }],
            temperature: temp,
            max_tokens: 100,
          } as ChatCompletionRequest),
        });

        if (response.ok) {
          const result: ChatCompletionResponse = await response.json();
          console.log(
            `   ✅ Temperature ${temp}: "${result.choices[0].message.content.substring(0, 60)}..."`,
          );
        }
      } catch (error) {
        console.log(`   ❌ Temperature ${temp} failed: ${error}`);
      }
    }

    // Test Circuit Breaker SDK LLM client integration
    console.log("\n🔧 Circuit Breaker SDK Integration:");
    try {
      const llmClient = this.client.llm();

      // Test direct LLM usage through SDK
      const response = await llmClient.chat(
        "o4-mini-2025-04-16",
        "Explain the benefits of multi-provider LLM routing in 2 sentences.",
        {
          temperature: 0.3,
          maxTokens: 100,
        },
      );
      console.log(`   ✅ SDK LLM response: ${response}`);
    } catch (error) {
      console.log(`   ⚠️  SDK LLM integration skipped: ${error}`);
    }
  }

  private async printSummary(): Promise<void> {
    console.log("\n📋 8. Demo Summary");
    console.log("==================");

    console.log("✅ Multi-Provider Integration Completed:");
    console.log("   🏢 Provider Discovery: All configured providers detected");
    console.log("   🧪 Individual Testing: Provider-specific model validation");
    console.log("   💰 Cost Analysis: Comparative pricing across providers");
    console.log(
      "   🌊 Streaming Support: Real-time response capabilities verified",
    );
    console.log(
      "   🧠 Smart Routing: Virtual models and strategy-based routing",
    );
    console.log(
      "   🚀 Advanced Features: Function calling and parameter testing",
    );
    console.log();

    console.log("🎯 Key Benefits Demonstrated:");
    console.log("   • Unified API across multiple LLM providers");
    console.log("   • Automatic cost optimization and provider selection");
    console.log("   • Real-time streaming interface with normalized responses");
    console.log("   • Smart routing based on task requirements");
    console.log("   • Transparent cost tracking and comparison");
    console.log();

    console.log("🛠️  Next Steps:");
    console.log("   • Integrate Circuit Breaker into your application");
    console.log("   • Configure provider preferences and cost limits");
    console.log("   • Set up monitoring and analytics");
    console.log("   • Explore GraphQL subscriptions for real-time updates");
    console.log();

    console.log("🌐 Resources:");
    console.log("   • GraphiQL Interface: http://localhost:4000");
    console.log("   • OpenAI API Endpoint: http://localhost:8081");
    console.log("   • Documentation: Check the docs/ directory");
    console.log();

    console.log("🎉 Multi-Provider Demo Complete!");
    console.log(
      "   Thank you for exploring Circuit Breaker's multi-provider capabilities!",
    );
  }

  // Helper methods
  private async graphqlRequest<T>(query: string, variables?: any): Promise<T> {
    const response = await fetch(this.graphqlUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`GraphQL request failed: ${response.statusText}`);
    }

    const result: GraphQLResponse<T> = await response.json();

    if (result.errors) {
      throw new Error(
        `GraphQL errors: ${result.errors.map((e) => e.message).join(", ")}`,
      );
    }

    return result.data!;
  }

  private async waitForEnter(message: string): Promise<void> {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    return new Promise((resolve) => {
      rl.question(`\n🎤 ${message}\n   Press Enter to continue...`, () => {
        rl.close();
        resolve();
      });
    });
  }

  private estimateCost(
    model: string,
    usage: { prompt_tokens: number; completion_tokens: number },
  ): number {
    // Simplified cost estimation based on known rates
    const rates: Record<string, { input: number; output: number }> = {
      "o4-mini-2025-04-16": { input: 0.003, output: 0.012 },
      "gpt-4o-mini": { input: 0.00015, output: 0.0006 },
      "gpt-4": { input: 0.03, output: 0.06 },
      "claude-3-haiku-20240307": { input: 0.00025, output: 0.00125 },
      "claude-3-sonnet-20240229": { input: 0.003, output: 0.015 },
      "gemini-1.5-flash": { input: 0.000075, output: 0.0003 },
      "gemini-1.5-pro": { input: 0.00125, output: 0.005 },
    };

    const rate = rates[model] || { input: 0.001, output: 0.002 };
    return (
      (usage.prompt_tokens * rate.input +
        usage.completion_tokens * rate.output) /
      1000
    );
  }

  private getProviderFromModel(model: string): string {
    if (
      model.startsWith("gpt-") ||
      model.startsWith("o4-") ||
      model.includes("o4-")
    )
      return "OpenAI";
    if (model.startsWith("claude-")) return "Anthropic";
    if (model.startsWith("gemini-")) return "Google";
    return "Unknown";
  }
}

// Main execution
async function main(): Promise<void> {
  const demo = new MultiProviderDemo();
  await demo.main();
}

// Handle graceful shutdown
process.on("SIGINT", () => {
  console.log("\n👋 Shutting down gracefully...");
  process.exit(0);
});

process.on("unhandledRejection", (reason, promise) => {
  console.error("Unhandled Rejection at:", promise, "reason:", reason);
  process.exit(1);
});

// Run the demo
if (require.main === module) {
  main().catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
  });
}

export { main, MultiProviderDemo };
