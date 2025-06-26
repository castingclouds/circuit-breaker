/**
 * Basic usage example for Circuit Breaker TypeScript SDK
 *
 * This example demonstrates the core functionality of the simplified SDK,
 * showing how to create workflows, resources, functions, and agents.
 */

import {
  Client,
  createWorkflow,
  createResource,
  createAgent,
  createChat,
  COMMON_MODELS,
} from "../src/index.js";
import { RuleBuilder, evaluateRule } from "../src/rules.js";

async function main() {
  console.log("🔧 Circuit Breaker TypeScript SDK Example");
  console.log("=========================================");

  // ============================================================================
  // 1. Create Client
  // ============================================================================

  console.log("\n1. Creating client...");

  let clientBuilder = Client.builder()
    .baseUrl(process.env.CIRCUIT_BREAKER_URL || "http://localhost:4000")
    .timeout(30000);

  if (process.env.CIRCUIT_BREAKER_API_KEY) {
    clientBuilder = clientBuilder.apiKey(process.env.CIRCUIT_BREAKER_API_KEY);
  }

  const client = clientBuilder.build();

  // Alternative: Use the convenience SDK class
  // const sdk = createSDK({
  //   baseUrl: 'http://localhost:3000',
  //   apiKey: 'your-api-key'
  // });

  try {
    // Test connection
    const ping = await client.ping();
    console.log(`✅ Connected to Circuit Breaker v${ping.version}`);

    const info = await client.info();
    console.log(
      `📊 Server: ${info.name}, Features: ${info.features.join(", ")}`,
    );
  } catch (error) {
    console.error("❌ Failed to connect:", error);
    return;
  }

  // ============================================================================
  // 2. Create Workflow
  // ============================================================================

  console.log("\n2. Creating workflow...");

  try {
    // Using the builder pattern
    const workflowDefinition = createWorkflow("Order Processing Example")
      .setDescription("A simple order processing workflow")
      .addState("pending", "normal")
      .addState("validating", "normal")
      .addState("processing", "normal")
      .addState("completed", "final")
      .addState("cancelled", "final")
      .addTransition("pending", "validating", "validate")
      .addTransition("validating", "processing", "approve")
      .addTransition("validating", "cancelled", "reject")
      .addTransition("processing", "completed", "complete")
      .addTransition("processing", "cancelled", "cancel")
      .setInitialState("pending")
      .build();

    const workflow = await client.workflows().create(workflowDefinition);
    console.log(`✅ Created workflow: ${workflow.name} (${workflow.id})`);

    // ============================================================================
    // 3. Create Resource
    // ============================================================================

    console.log("\n3. Creating resource...");

    const resourceDefinition = createResource(workflow.id)
      .addData("orderId", "ORD-2024-001")
      .addData("customerId", "CUST-123")
      .addData("amount", 299.99)
      .addData("items", [
        { id: "ITEM-1", name: "Product A", price: 199.99 },
        { id: "ITEM-2", name: "Product B", price: 100.0 },
      ])
      .setInitialState("pending")
      .build();

    const resource = await client.resources().create(resourceDefinition);
    console.log(
      `✅ Created resource: ${resource.id} in state '${resource.state}'`,
    );

    // ============================================================================
    // 4. Create Function (Skipped - handled via activities)
    // ============================================================================

    console.log(
      "\n4. Function creation (skipped - functions are handled via workflow activities)",
    );

    // ============================================================================
    // 5. Server-Side Rule Storage & Evaluation
    // ============================================================================

    console.log("\n5. Testing server-side rule storage...");

    try {
      // Create a rule on the server
      const createRuleMutation = `
        mutation CreateRule($input: RuleInput!) {
          createRule(input: $input) {
            id
            name
            description
            version
            createdAt
            updatedAt
            tags
          }
        }
      `;

      const ruleInput = {
        name: "High Value Order Rule",
        description: "Detects orders over $1000 for approval workflow",
        condition: {
          conditionType: "FieldGreaterThan",
          field: "amount",
          value: 1000,
        },
        tags: ["e-commerce", "validation", "high-value"],
      };

      const createResult = (await client.mutation(createRuleMutation, {
        input: ruleInput,
      })) as any;

      if (createResult && createResult.createRule) {
        const createdRule = createResult.createRule;
        console.log(
          `✅ Created server rule: ${createdRule.name} (${createdRule.id})`,
        );

        // Test rule evaluation on the server
        const evaluateRuleMutation = `
          mutation EvaluateRule($input: RuleEvaluationInput!) {
            evaluateRule(input: $input) {
              ruleId
              passed
              reason
              details
            }
          }
        `;

        const evaluationInput = {
          ruleId: createdRule.id,
          data: {
            orderId: "ORD-2024-001",
            customerId: "CUST-123",
            amount: 1500.0,
          },
          metadata: {
            source: "api",
            timestamp: new Date().toISOString(),
          },
        };

        const evalResult = (await client.mutation(evaluateRuleMutation, {
          input: evaluationInput,
        })) as any;

        if (evalResult && evalResult.evaluateRule) {
          const evaluation = evalResult.evaluateRule;
          const status = evaluation.passed ? "✅" : "❌";
          console.log(`${status} Server rule evaluation: ${evaluation.reason}`);
        }

        // List all rules
        const listRulesQuery = `
          query ListRules {
            rules {
              id
              name
              description
              tags
              version
              createdAt
              updatedAt
            }
          }
        `;

        const listResult = (await client.query(listRulesQuery)) as any;
        if (listResult && listResult.rules) {
          console.log(`📋 Found ${listResult.rules.length} rules on server:`);
          listResult.rules.forEach((rule: any) => {
            console.log(
              `  - ${rule.name} (v${rule.version}): ${rule.description}`,
            );
            console.log(`    Tags: ${rule.tags.join(", ")}`);
            console.log(
              `    Created: ${new Date(rule.createdAt).toLocaleString()}`,
            );
          });
        }
      }
    } catch (error) {
      console.log(`⚠️ Server-side rules not available: ${error}`);

      // Fallback to client-side rule evaluation
      console.log("\n📋 Falling back to client-side rule evaluation...");

      const highValueRule = RuleBuilder.fieldGreaterThan(
        "high_value",
        "High value order",
        "amount",
        1000,
      );

      const resourceData = {
        orderId: "ORD-2024-001",
        customerId: "CUST-123",
        amount: 299.99,
      };

      const result = evaluateRule(highValueRule, resourceData);
      const status = result.passed ? "✅" : "❌";
      console.log(`${status} Client-side evaluation: ${result.reason}`);
    }

    // ============================================================================
    // 6. Create Agent (if LLM is available)
    // ============================================================================

    console.log("\n6. Creating agent...");

    try {
      const agentDefinition = createAgent("Order Support Agent")
        .setDescription("AI agent for customer order support")
        .setType("conversational")
        .setLLMProvider("openai")
        .setModel(COMMON_MODELS.GPT_O4_MINI)
        .setTemperature(0.7)
        .setMaxTokens(1000)
        .setSystemPrompt(
          `You are a helpful customer service agent for an e-commerce platform.
          Help customers with their orders, returns, and general inquiries.
          Be polite, professional, and concise.`,
        )
        .addTool("lookup_order", "Look up order details by order ID", {
          orderId: { type: "string", description: "The order ID to lookup" },
        })
        .setMemory("short_term", { max_entries: 10 })
        .build();

      const agent = await client.agents().create(agentDefinition);
      console.log(`✅ Created agent: ${agent.name} (${agent.id})`);

      // Chat with the agent
      const chatResponse = await client.agents().chat(agent.id, [
        {
          role: "user",
          content: "Hi, I have a question about my order ORD-2024-001",
        },
      ]);
      console.log(
        `🤖 Agent response:`,
        chatResponse.choices[0]?.message?.content,
      );
    } catch (error) {
      console.log(`⚠️  Agent creation skipped (LLM not available): ${error}`);
    }

    // ============================================================================
    // 7. Multi-Provider LLM Usage (if available)
    // ============================================================================

    console.log("\n7. Multi-Provider LLM capabilities...");

    try {
      const llmClient = client.llm();

      // Test different providers
      console.log("\n🧪 Testing multiple providers:");

      const testModels = [
        { name: "OpenAI GPT-4", model: COMMON_MODELS.GPT_O4_MINI },
        { name: "Claude Sonnet", model: COMMON_MODELS.CLAUDE_4_SONNET },
        { name: "Gemini Pro", model: COMMON_MODELS.GEMINI_PRO },
      ];

      for (const { name, model } of testModels) {
        try {
          const startTime = Date.now();
          const response = await llmClient.chat(
            model,
            "Explain workflow automation benefits in one sentence.",
            {
              temperature: 0.3,
              maxTokens: 1000,
            },
          );
          const latency = Date.now() - startTime;
          console.log(`✅ ${name} (${latency}ms): ${response}`);
        } catch (error) {
          console.log(`⚠️  ${name} unavailable: ${error}`);
        }
      }

      // Test virtual models for smart routing
      console.log("\n🎯 Testing smart routing with virtual models:");

      const virtualModels = [
        { name: "Auto-Route", model: "auto" },
        { name: "Cost-Optimal", model: "cb:cost-optimal" },
        { name: "Fastest", model: "cb:fastest" },
      ];

      for (const { name, model } of virtualModels) {
        try {
          const response = await llmClient.chat(
            model,
            "What is the capital of France?",
            { maxTokens: 1000 },
          );
          console.log(`🎯 ${name}: ${response}`);
        } catch (error) {
          console.log(`⚠️  ${name} routing failed: ${error}`);
        }
      }

      // Test streaming across providers
      console.log("\n🌊 Testing multi-provider streaming:");
      try {
        // Direct OpenAI API call for streaming demonstration
        const streamingResponse = await fetch(
          "http://localhost:3000/v1/chat/completions",
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              model: COMMON_MODELS.GPT_O4_MINI,
              messages: [
                { role: "user", content: "Write a short haiku about AI" },
              ],
              stream: true,
              max_tokens: 1000,
            }),
          },
        );

        if (streamingResponse.ok && streamingResponse.body) {
          console.log("📝 Streaming response:");
          const reader = streamingResponse.body.getReader();
          const decoder = new TextDecoder();

          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const chunk = decoder.decode(value);
            const lines = chunk
              .split("\n")
              .filter((line) => line.trim() && line.includes("data:"));

            for (const line of lines) {
              if (line.includes("[DONE]")) break;
              try {
                const data = JSON.parse(line.replace("data: ", ""));
                const content = data.choices?.[0]?.delta?.content;
                if (content) process.stdout.write(content);
              } catch (e) {
                // Skip malformed JSON
              }
            }
          }
          console.log("\n✅ Streaming complete");
        } else {
          console.log("⚠️  Streaming not available");
        }
      } catch (error) {
        console.log(`⚠️  Streaming test failed: ${error}`);
      }

      // Cost comparison demonstration
      console.log("\n💰 Provider cost comparison:");
      const costTestPrompt = "Hello world";

      const costTests = [
        {
          provider: "OpenAI",
          model: COMMON_MODELS.GPT_O4_MINI,
          estimatedCost: 0.003,
        },
        {
          provider: "Anthropic",
          model: COMMON_MODELS.CLAUDE_4_SONNET,
          estimatedCost: 0.000025,
        },
        {
          provider: "Google",
          model: COMMON_MODELS.GEMINI_PRO,
          estimatedCost: 0.0000375,
        },
      ];

      console.log("Provider    | Model              | Est. Cost/1K tokens");
      console.log("------------|--------------------|-----------------");

      for (const test of costTests) {
        const providerPad = test.provider.padEnd(11);
        const modelPad = test.model.substring(0, 18).padEnd(18);
        const costStr = `$${test.estimatedCost.toFixed(6)}`;
        console.log(`${providerPad}| ${modelPad} | ${costStr}`);
      }

      // Using chat builder with different providers
      console.log("\n🔧 Chat builder with provider selection:");
      const chatBuilder = createChat(COMMON_MODELS.CLAUDE_4_SONNET)
        .setSystemPrompt(
          "You are a helpful assistant specializing in workflow automation.",
        )
        .addUserMessage("List 3 key benefits of workflow automation")
        .setTemperature(0.2)
        .setMaxTokens(150);

      const chatResult = await chatBuilder.execute(llmClient);
      console.log(
        `📝 Claude response:`,
        chatResult.choices[0]?.message?.content,
      );

      // Direct GraphQL llmChatCompletion example (matching Rust SDK approach)
      console.log("\n🔧 Direct GraphQL llmChatCompletion example:");
      const llmChatMutation = `
        mutation LlmChatCompletion($input: LlmchatCompletionInput!) {
          llmChatCompletion(input: $input) {
            id
            model
            choices {
              index
              message {
                role
                content
              }
              finishReason
            }
            usage {
              promptTokens
              completionTokens
              totalTokens
            }
          }
        }
      `;

      const llmInput = {
        model: COMMON_MODELS.GPT_O4_MINI,
        messages: [
          {
            role: "system",
            content:
              "You are a helpful assistant specializing in workflow automation.",
          },
          {
            role: "user",
            content:
              "What are the main advantages of using Circuit Breaker for LLM routing?",
          },
        ],
        temperature: 0.5,
        maxTokens: 1000,
      };

      try {
        const llmResult = (await client.mutation(llmChatMutation, {
          input: llmInput,
        })) as any;

        if (llmResult && llmResult.llmChatCompletion) {
          const response = llmResult.llmChatCompletion;
          console.log(
            `🚀 Direct GraphQL (${response.model}): ${response.choices[0]?.message?.content}`,
          );
          console.log(
            `   📊 Token usage: ${response.usage.totalTokens} total (${response.usage.promptTokens} + ${response.usage.completionTokens})`,
          );
        }
      } catch (graphqlError) {
        console.log(
          `⚠️  Direct GraphQL llmChatCompletion failed: ${graphqlError}`,
        );
      }
    } catch (error) {
      console.log(`⚠️  Multi-provider LLM testing skipped: ${error}`);

      // Fallback to basic LLM usage
      try {
        const llmClient = client.llm();
        const response = await llmClient.chat(
          COMMON_MODELS.GPT_O4_MINI,
          "Explain the benefits of workflow automation in 2 sentences.",
          {
            temperature: 0.3,
            maxTokens: 1000,
          },
        );
        console.log(`🤖 Fallback LLM response: ${response}`);
      } catch (fallbackError) {
        console.log(`⚠️  All LLM usage skipped: ${fallbackError}`);
      }
    }

    // ============================================================================
    // 8. Execute Workflow (Create Workflow Instance)
    // ============================================================================

    console.log("\n8. Creating workflow instance...");

    const workflowInstance = await client.workflows().execute(workflow.id, {
      initialData: {
        orderId: "ORD-2024-001",
        customerId: "CUST-123",
        amount: 299.99,
      },
      metadata: {
        source: "api",
        timestamp: new Date().toISOString(),
      },
    });
    console.log(`✅ Created workflow instance: ${workflowInstance.id}`);
    console.log(`📊 Current state: ${workflowInstance.state}`);

    // Check instance status
    setTimeout(async () => {
      try {
        const updatedInstance = await client
          .workflows()
          .getExecution(workflowInstance.id);
        console.log(`📊 Instance update - State: ${updatedInstance.state}`);
      } catch (error) {
        console.log(`⚠️  Could not fetch instance status: ${error}`);
      }
    }, 2000);

    // ============================================================================
    // 9. Resource Operations
    // ============================================================================

    console.log("\n9. Performing resource operations...");

    // Execute activity on resource using the workflow instance
    try {
      const activityResult = await client
        .resources()
        .executeActivity(workflowInstance.id, "activity_0", {
          data: {
            timestamp: new Date().toISOString(),
            performedBy: "sdk-user",
            notes: "Executed validate activity",
          },
        });
      console.log(
        `✅ Activity executed, resource state: ${activityResult.state}`,
      );

      // Get resource history
      const history = await client
        .resources()
        .getHistory(activityResult.id, { limit: 5 });
      console.log(`📜 Resource history (${history.data.length} events):`);
      history.data.forEach((event) => {
        console.log(
          `  - ${event.activity}: ${event.fromState} → ${event.toState} at ${new Date(event.timestamp).toLocaleTimeString()}`,
        );
      });
    } catch (error) {
      console.log(`⚠️  Activity execution failed: ${error}`);
    }

    console.log("\n✨ Example completed successfully!");
    console.log("\n🚀 For comprehensive multi-provider LLM testing, run:");
    console.log("   npx tsx examples/multi-provider-demo.ts");
    console.log("\n📖 This demonstrates:");
    console.log("   • Provider discovery and health monitoring");
    console.log("   • Cost optimization across providers");
    console.log("   • Real-time streaming from multiple providers");
    console.log("   • Smart routing and virtual models");
    console.log("   • Advanced features like function calling");
  } catch (error) {
    console.error("\n❌ Example failed:", error);

    if (error instanceof Error) {
      console.error("Error details:", {
        name: error.name,
        message: error.message,
        stack: error.stack?.split("\n").slice(0, 3).join("\n"),
      });
    }
  }
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

// Run the example
if (require.main === module) {
  main().catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
  });
}

export { main };
