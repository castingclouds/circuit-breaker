#!/usr/bin/env npx tsx
// Simplified States AI Agent Demo - TypeScript Client
// Demonstrates basic workflow operations with the current GraphQL schema
// Run with: npx tsx states_ai_agent_demo_simple.ts

import { config } from "dotenv";
import { resolve } from "path";

// Load environment variables from .env file in project root
config({ path: resolve(process.cwd(), "../../.env") });

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface Workflow {
  id: string;
  name: string;
  states: string[];
  activities: Activity[];
}

interface Activity {
  id: string;
  fromStates: string[];
  toState: string;
  conditions: string[];
}

interface Resource {
  id: string;
  workflowId: string;
  state: string;
  data: Record<string, any>;
  metadata: Record<string, any>;
  history: HistoryEvent[];
}

interface HistoryEvent {
  activity: string;
  fromState: string;
  toState: string;
  timestamp: string;
}

class SimpleStatesAIClient {
  private baseUrl: string;
  private headers: Record<string, string>;

  constructor(baseUrl?: string) {
    this.baseUrl =
      baseUrl || process.env.GRAPHQL_ENDPOINT || "http://localhost:4000";
    this.headers = {
      "Content-Type": "application/json",
      "User-Agent": "Circuit-Breaker-Simple-Client/1.0",
    };
  }

  async request<T = any>(
    query: string,
    variables?: any,
  ): Promise<GraphQLResponse<T>> {
    try {
      const url = this.baseUrl.endsWith("/graphql")
        ? this.baseUrl
        : `${this.baseUrl}/graphql`;
      console.log(`🌐 Making GraphQL request to: ${url}`);
      console.log(`📤 Query: ${query.substring(0, 100)}...`);

      const response = await fetch(url, {
        method: "POST",
        headers: this.headers,
        body: JSON.stringify({ query, variables }),
      });

      console.log(
        `📥 Response status: ${response.status} ${response.statusText}`,
      );

      if (!response.ok) {
        const errorText = await response.text();
        console.error(`❌ HTTP error details: ${errorText}`);
        throw new Error(
          `HTTP error! status: ${response.status} - ${errorText}`,
        );
      }

      const result = (await response.json()) as GraphQLResponse<T>;

      if (result.errors) {
        console.error("❌ GraphQL errors:", result.errors);
      } else {
        console.log("✅ GraphQL request successful");
      }

      return result;
    } catch (error) {
      console.error("❌ GraphQL request failed:", error);
      throw error;
    }
  }

  async createWorkflow(name: string): Promise<Workflow> {
    const mutation = `
      mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
        createWorkflow(input: $input) {
          id
          name
          states
          activities {
            id
            fromStates
            toState
            conditions
          }
        }
      }
    `;

    const input = {
      name,
      initialState: "pending_classification",
      states: [
        "pending_classification",
        "classified",
        "pending_review",
        "reviewed",
        "published",
      ],
      activities: [
        {
          id: "classify",
          fromStates: ["pending_classification"],
          toState: "classified",
          conditions: [],
        },
        {
          id: "review",
          fromStates: ["classified"],
          toState: "pending_review",
          conditions: [],
        },
        {
          id: "approve",
          fromStates: ["pending_review"],
          toState: "reviewed",
          conditions: [],
        },
        {
          id: "publish",
          fromStates: ["reviewed"],
          toState: "published",
          conditions: [],
        },
      ],
    };

    const response = await this.request<{ createWorkflow: Workflow }>(
      mutation,
      { input },
    );

    if (response.errors) {
      throw new Error(
        `Failed to create workflow: ${response.errors[0].message}`,
      );
    }

    return response.data!.createWorkflow;
  }

  async createResource(
    workflowId: string,
    data: Record<string, any>,
    metadata: Record<string, any> = {},
  ): Promise<Resource> {
    const mutation = `
      mutation CreateResource($input: ResourceCreateInput!) {
        createResource(input: $input) {
          id
          workflowId
          state
          data
          metadata
          history {
            activity
            fromState
            toState
            timestamp
          }
        }
      }
    `;

    const response = await this.request<{ createResource: Resource }>(
      mutation,
      {
        input: { workflowId, data, metadata },
      },
    );

    if (response.errors) {
      throw new Error(
        `Failed to create resource: ${response.errors[0].message}`,
      );
    }

    return response.data!.createResource;
  }

  async getResource(resourceId: string): Promise<Resource | null> {
    const query = `
      query GetResource($id: String!) {
        resource(id: $id) {
          id
          workflowId
          state
          data
          metadata
          history {
            activity
            fromState
            toState
            timestamp
          }
        }
      }
    `;

    const response = await this.request<{ resource: Resource | null }>(query, {
      id: resourceId,
    });
    return response.data?.resource || null;
  }

  async executeActivity(
    resourceId: string,
    activityId: string,
  ): Promise<Resource> {
    const mutation = `
      mutation ExecuteActivity($input: ActivityExecuteInput!) {
        executeActivity(input: $input) {
          id
          workflowId
          state
          data
          metadata
          history {
            activity
            fromState
            toState
            timestamp
          }
        }
      }
    `;

    const response = await this.request<{ executeActivity: Resource }>(
      mutation,
      {
        input: { resourceId, activityId },
      },
    );

    if (response.errors) {
      throw new Error(
        `Failed to execute activity: ${response.errors[0].message}`,
      );
    }

    return response.data!.executeActivity;
  }

  async listWorkflows(): Promise<Workflow[]> {
    const query = `
      query ListWorkflows {
        workflows {
          id
          name
          states
          activities {
            id
            fromStates
            toState
          }
        }
      }
    `;

    const response = await this.request<{ workflows: Workflow[] }>(query);
    return response.data?.workflows || [];
  }
}

// Demo functions
async function runSimpleDemo() {
  console.log("🚀 Simple States AI Agent Demo (TypeScript)");
  console.log("===========================================");

  // Check for API key
  if (
    !process.env.ANTHROPIC_API_KEY ||
    process.env.ANTHROPIC_API_KEY === "your_anthropic_api_key_here"
  ) {
    console.warn("⚠️  Note: ANTHROPIC_API_KEY not configured in .env");
    console.warn("Agent functionality will use placeholder responses");
    console.warn("Configure your API key for real agent execution\n");
  } else {
    console.log("✅ Anthropic API key configured for agent execution\n");
  }

  const client = new SimpleStatesAIClient();

  try {
    // Test connectivity
    console.log("🔍 Testing GraphQL server connectivity...");
    console.log(`   Server URL: ${client["baseUrl"]}`);

    const workflows = await client.listWorkflows();
    console.log(`✅ Connected! Found ${workflows.length} existing workflows\n`);

    // Test Anthropic agent creation
    console.log("🤖 Testing Anthropic agent creation...");
    const testAgentResult = await client.request(
      `
      mutation CreateAgent($input: AgentDefinitionInput!) {
        createAgent(input: $input) {
          id
          name
          description
          llmProvider {
            providerType
            model
          }
        }
      }
    `,
      {
        input: {
          name: "Simple Test Agent",
          description: "Test agent for Anthropic integration",
          llmProvider: {
            providerType: "anthropic",
            model:
              process.env.ANTHROPIC_DEFAULT_MODEL || "claude-3-sonnet-20240229",
            apiKey: process.env.ANTHROPIC_API_KEY || "demo-key",
            ...(process.env.ANTHROPIC_BASE_URL && {
              baseUrl: process.env.ANTHROPIC_BASE_URL,
            }),
          },
          llmConfig: {
            temperature: 0.7,
            maxTokens: 100,
            topP: 0.9,
            frequencyPenalty: 0.0,
            presencePenalty: 0.0,
            stopSequences: [],
          },
          prompts: {
            system: "You are a helpful assistant.",
            userTemplate: "Please respond to: {input}",
            contextInstructions: "Be concise and helpful.",
          },
          capabilities: ["text_generation"],
          tools: [],
        },
      },
    );

    if (testAgentResult.errors) {
      console.error(
        "❌ Failed to create test agent:",
        testAgentResult.errors[0].message,
      );
      console.log("Continuing with workflow demo...\n");
    } else {
      console.log(
        `✅ Created test agent: ${testAgentResult.data.createAgent.id}`,
      );
      console.log(
        `   Provider: ${testAgentResult.data.createAgent.llmProvider.providerType}`,
      );
      console.log(
        `   Model: ${testAgentResult.data.createAgent.llmProvider.model}\n`,
      );
    }

    // Create a demo workflow for AI agent processing
    console.log("📋 Creating AI-enabled document workflow...");
    const workflow = await client.createWorkflow(
      "AI-Enabled Document Processing",
    );
    console.log(`✅ Created workflow: ${workflow.id}`);
    console.log(`   States: ${workflow.states.join(" → ")}\n`);

    // Create a document resource that would trigger AI agents
    console.log("📄 Creating document resource for AI processing...");
    const documentResource = await client.createResource(
      workflow.id,
      {
        content:
          "This is a technical document about Rust programming and async/await patterns.",
        type: "technical_document",
      },
      {
        status: "unclassified",
        priority: "high",
        author: "demo_user",
      },
    );
    console.log(`✅ Created resource: ${documentResource.id}`);
    console.log(`   Current state: ${documentResource.state}`);
    console.log(
      `   Content preview: "${documentResource.data.content.substring(0, 50)}..."`,
    );

    // Simulate AI agent processing by executing activities
    console.log("\n🤖 Simulating AI agent workflow...");

    console.log("   1. Classification Agent would process the document...");
    console.log(
      "      (In full implementation: AI analyzes content and classifies it)",
    );
    await new Promise((resolve) => setTimeout(resolve, 1000)); // Simulate processing

    const classifiedResource = await client.executeActivity(
      documentResource.id,
      "classify",
    );
    console.log(`   ✅ Resource moved to: ${classifiedResource.state}`);

    console.log("   2. Moving to review stage...");
    const reviewResource = await client.executeActivity(
      classifiedResource.id,
      "review",
    );
    console.log(`   ✅ Resource moved to: ${reviewResource.state}`);

    console.log("   3. Review Agent would analyze quality...");
    console.log(
      "      (In full implementation: AI reviews content quality and accuracy)",
    );
    await new Promise((resolve) => setTimeout(resolve, 1000)); // Simulate processing

    const approvedResource = await client.executeActivity(
      reviewResource.id,
      "approve",
    );
    console.log(`   ✅ Resource moved to: ${approvedResource.state}`);

    // Show final resource state
    console.log("\n📊 Final resource state:");
    const finalResource = await client.getResource(approvedResource.id);
    if (finalResource) {
      console.log(`   ID: ${finalResource.id}`);
      console.log(`   Current state: ${finalResource.state}`);
      console.log(`   Workflow activities: ${finalResource.history.length}`);
      console.log("   History:");
      finalResource.history.forEach((event, index) => {
        console.log(
          `     ${index + 1}. ${event.fromState} → ${event.toState} (${event.activity})`,
        );
      });
    }

    console.log("\n🎯 What this demonstrates:");
    console.log("   • Basic workflow operations via GraphQL");
    console.log("   • Resource state management and activities");
    console.log("   • States where AI agents would be triggered");
    console.log("   • Document processing pipeline structure");

    console.log("\n📝 Next steps for full AI integration:");
    console.log("   • Implement GraphQL resolvers for agent operations");
    console.log("   • Add States AI Agent configurations");
    console.log("   • Enable real-time agent execution and streaming");
    console.log("   • Connect with Anthropic Claude for content processing");

    console.log("\n✨ Demo completed successfully!");
  } catch (error) {
    console.error("❌ Demo failed:", error);
    if (error instanceof Error) {
      console.error("Error details:", error.message);
    }
    process.exit(1);
  }
}

// Run the demo
if (import.meta.url === `file://${process.argv[1]}`) {
  runSimpleDemo().catch(console.error);
}

export {
  SimpleStatesAIClient,
  type Workflow,
  type Resource,
  type Activity,
  type HistoryEvent,
};
