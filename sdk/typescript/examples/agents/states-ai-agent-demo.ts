#!/usr/bin/env npx tsx
// States AI Agent Demo - TypeScript Client
// Demonstrates how to configure and use States AI Agents via GraphQL
// Run with: npx tsx examples/typescript/states_ai_agent_demo.ts

import { config } from "dotenv";
import { resolve } from "path";

// Load environment variables from .env file in project root
config({ path: resolve(process.cwd(), "../../.env") });

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface AgentDefinition {
  id: string;
  name: string;
  description: string;
  llmProvider: LLMProvider;
  llmConfig: LLMConfig;
  prompts: AgentPrompts;
  capabilities: string[];
  tools: string[];
}

interface LLMProvider {
  providerType: "openai" | "anthropic" | "google" | "ollama" | "custom";
  apiKey?: string;
  model: string;
  baseUrl?: string;
  endpoint?: string;
  headers?: Record<string, string>;
}

interface LLMConfig {
  temperature: number;
  maxTokens?: number;
  topP?: number;
  frequencyPenalty?: number;
  presencePenalty?: number;
  stopSequences: string[];
}

interface AgentPrompts {
  system: string;
  userTemplate: string;
  contextInstructions?: string;
}

interface StateAgentConfig {
  id: string;
  stateId: string;
  agentId: string;
  llmConfig?: LLMConfig;
  triggerConditions: Rule[];
  inputMapping: Record<string, string>;
  outputMapping: Record<string, string>;
  autoActivity?: string;
  schedule?: StateAgentSchedule;
  retryConfig?: AgentRetryConfig;
  enabled: boolean;
}

interface StateAgentSchedule {
  initialDelaySeconds?: number;
  intervalSeconds?: number;
  maxExecutions?: number;
}

interface AgentRetryConfig {
  maxAttempts: number;
  backoffSeconds: number;
  retryOnErrors: string[];
}

interface Rule {
  id: string;
  condition: RuleCondition;
}

interface RuleCondition {
  type: "field_exists" | "field_equals" | "field_contains";
  field?: string;
  value?: string;
}

interface Resource {
  id: string;
  workflowId: string;
  state: string;
  data: Record<string, any>;
  metadata: Record<string, any>;
}

interface AgentExecution {
  id: string;
  agentId: string;
  resourceId: string;
  stateId: string;
  status:
    | "pending"
    | "running"
    | "completed"
    | "failed"
    | "timeout"
    | "cancelled";
  inputData: any;
  outputData?: any;
  errorMessage?: string;
  startedAt: string;
  completedAt?: string;
  durationMs?: number;
  retryCount: number;
}

interface AgentStreamEvent {
  executionId: string;
  eventType: string;
  content?: string;
  status?: string;
  toolName?: string;
  error?: string;
  timestamp: string;
}

class StatesAIAgentClient {
  private baseUrl: string;
  private wsUrl: string;
  private headers: Record<string, string>;

  constructor(baseUrl?: string) {
    this.baseUrl =
      baseUrl || process.env.GRAPHQL_ENDPOINT || "http://localhost:4000";
    this.wsUrl =
      process.env.GRAPHQL_WS_ENDPOINT || this.baseUrl.replace("http", "ws");
    this.headers = {
      "Content-Type": "application/json",
      "User-Agent": "Circuit-Breaker-States-AI-Client/1.0",
    };
  }

  async request<T = any>(
    query: string,
    variables?: any,
  ): Promise<GraphQLResponse<T>> {
    const response = await fetch(`${this.baseUrl}/graphql`, {
      method: "POST",
      headers: this.headers,
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return (await response.json()) as GraphQLResponse<T>;
  }

  // Agent Management
  async createAgent(
    agent: Omit<AgentDefinition, "id">,
  ): Promise<AgentDefinition> {
    const mutation = `
      mutation CreateAgent($input: AgentDefinitionInput!) {
        createAgent(input: $input) {
          id
          name
          description
          llmProvider {
            providerType
            model
            baseUrl
          }
          llmConfig {
            temperature
            maxTokens
            topP
            frequencyPenalty
            presencePenalty
            stopSequences
          }
          prompts {
            system
            userTemplate
            contextInstructions
          }
          capabilities
          tools
        }
      }
    `;

    const response = await this.request<{ createAgent: AgentDefinition }>(
      mutation,
      {
        input: agent,
      },
    );

    if (response.errors) {
      throw new Error(`Failed to create agent: ${response.errors[0].message}`);
    }

    return response.data!.createAgent;
  }

  async getAgent(agentId: string): Promise<AgentDefinition | null> {
    const query = `
      query GetAgent($id: String!) {
        agent(id: $id) {
          id
          name
          description
          llmProvider {
            providerType
            model
            baseUrl
          }
          llmConfig {
            temperature
            maxTokens
            topP
            frequencyPenalty
            presencePenalty
            stopSequences
          }
          prompts {
            system
            userTemplate
            contextInstructions
          }
          capabilities
          tools
        }
      }
    `;

    const response = await this.request<{ agent: AgentDefinition | null }>(
      query,
      {
        id: agentId,
      },
    );

    return response.data?.agent || null;
  }

  async listAgents(): Promise<AgentDefinition[]> {
    const query = `
      query ListAgents {
        agents {
          id
          name
          description
          capabilities
        }
      }
    `;

    const response = await this.request<{ agents: AgentDefinition[] }>(query);
    return response.data?.agents || [];
  }

  // State Agent Configuration
  async createStateAgentConfig(
    config: Omit<StateAgentConfig, "id">,
  ): Promise<StateAgentConfig> {
    const mutation = `
      mutation CreateStateAgentConfig($config: StateAgentConfigInput!) {
        createStateAgentConfig(config: $config) {
          id
          stateId
          agentId
          llmConfig {
            temperature
            maxTokens
            topP
            frequencyPenalty
            presencePenalty
            stopSequences
          }
          triggerConditions {
            id
            condition {
              type
              field
              value
            }
          }
          inputMapping
          outputMapping
          autoActivity
          schedule {
            initialDelaySeconds
            intervalSeconds
            maxExecutions
          }
          retryConfig {
            maxAttempts
            backoffSeconds
            retryOnErrors
          }
          enabled
        }
      }
    `;

    const response = await this.request<{
      createStateAgentConfig: StateAgentConfig;
    }>(mutation, {
      config: config,
    });

    if (response.errors) {
      throw new Error(
        `Failed to create state agent config: ${response.errors[0].message}`,
      );
    }

    return response.data!.createStateAgentConfig;
  }

  async getStateAgentConfigs(stateId: string): Promise<StateAgentConfig[]> {
    const query = `
      query GetStateAgentConfigs($stateId: String) {
        stateAgentConfigs(stateId: $stateId) {
          id
          stateId
          agentId
          enabled
          triggerConditions {
            id
            condition {
              type
              field
              value
            }
          }
          inputMapping
          outputMapping
          autoActivity
        }
      }
    `;

    const response = await this.request<{
      stateAgentConfigs: StateAgentConfig[];
    }>(query, {
      stateId,
    });

    return response.data?.stateAgentConfigs || [];
  }

  // Resource Operations
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
        }
      }
    `;

    const response = await this.request<{ resource: Resource | null }>(query, {
      id: resourceId,
    });

    return response.data?.resource || null;
  }

  // Agent Execution
  async triggerStateAgents(resourceId: string): Promise<AgentExecution[]> {
    const mutation = `
      mutation TriggerStateAgents($resourceId: String!) {
        triggerStateAgents(resourceId: $resourceId) {
          id
          agentId
          status
          startedAt
        }
      }
    `;

    const response = await this.request<{
      triggerStateAgents: AgentExecution[];
    }>(mutation, {
      resourceId,
    });

    if (response.errors) {
      throw new Error(
        `Failed to trigger state agents: ${response.errors[0].message}`,
      );
    }

    return response.data!.triggerStateAgents;
  }

  async getAgentExecution(executionId: string): Promise<AgentExecution | null> {
    const query = `
      query GetAgentExecution($id: String!) {
        agentExecution(id: $id) {
          id
          agentId
          tokenId
          placeId
          status
          inputData
          outputData
          errorMessage
          startedAt
          completedAt
          durationMs
          retryCount
        }
      }
    `;

    const response = await this.request<{
      agentExecution: AgentExecution | null;
    }>(query, {
      id: executionId,
    });

    return response.data?.agentExecution || null;
  }

  async getResourceExecutions(resourceId: string): Promise<AgentExecution[]> {
    const query = `
      query GetResourceExecutions($resourceId: String!) {
        resourceExecutions(resourceId: $resourceId) {
          id
          agentId
          status
          startedAt
          completedAt
          durationMs
          outputData
        }
      }
    `;

    const response = await this.request<{
      resourceExecutions: AgentExecution[];
    }>(query, {
      resourceId,
    });

    return response.data?.resourceExecutions || [];
  }

  // Streaming
  subscribeToAgentExecution(
    executionId: string,
    onEvent: (event: AgentStreamEvent) => void,
  ): any {
    // Note: WebSocket would be available in browser environment
    // For Node.js, you would need to install 'ws' package
    const WebSocket = require("ws");
    const ws = new WebSocket(`${this.wsUrl}/graphql`, "graphql-ws");

    ws.onopen = () => {
      // Send connection init
      ws.send(
        JSON.stringify({
          type: "connection_init",
        }),
      );

      // Send subscription
      ws.send(
        JSON.stringify({
          id: "1",
          type: "start",
          payload: {
            query: `
            subscription AgentExecutionStream($executionId: String!) {
              agentExecutionStream(executionId: $executionId) {
                executionId
                eventType
                content
                status
                toolName
                error
                timestamp
              }
            }
          `,
            variables: { executionId },
          },
        }),
      );
    };

    ws.onmessage = (event: any) => {
      const message = JSON.parse(event.data);
      if (message.type === "data") {
        onEvent(message.payload.data.agentExecutionStream);
      }
    };

    return ws;
  }
}

// Demo functions
async function runDemo() {
  console.log("🤖 States AI Agent TypeScript Demo");
  console.log("===================================");

  // Check for required environment variables
  const requiredEnvVars = ["ANTHROPIC_API_KEY"];
  const missingVars = requiredEnvVars.filter(
    (varName) => !process.env[varName],
  );

  if (missingVars.length > 0) {
    console.warn(
      `⚠️  Warning: Missing environment variables: ${missingVars.join(", ")}`,
    );
    console.warn(
      "Make sure to copy .env.example to .env and configure your Anthropic API key",
    );
    console.warn("Demo will continue with placeholder keys...\n");
  }

  const client = new StatesAIAgentClient();

  try {
    // Create demo agents
    console.log("\n🔧 Creating demo agents...");

    const classificationAgent = await client.createAgent({
      name: "Content Classification Agent",
      description: "Classifies content into categories",
      // Using Anthropic as default (requires ANTHROPIC_API_KEY in .env)
      llmProvider: {
        providerType: "anthropic",
        model:
          process.env.ANTHROPIC_DEFAULT_MODEL || "claude-3-5-sonnet-20241022",
        apiKey: process.env.ANTHROPIC_API_KEY || "demo-key",
        ...(process.env.ANTHROPIC_BASE_URL && {
          baseUrl: process.env.ANTHROPIC_BASE_URL,
        }),
      },
      // Alternative providers (uncomment to use):
      // OpenAI:
      // llmProvider: {
      //   providerType: 'openai',
      //   model: process.env.OPENAI_DEFAULT_MODEL || 'gpt-4',
      //   apiKey: process.env.OPENAI_API_KEY || 'demo-key',
      //   baseUrl: process.env.OPENAI_BASE_URL,
      // },
      // Google Gemini:
      // llmProvider: {
      //   providerType: 'google',
      //   model: process.env.GOOGLE_DEFAULT_MODEL || 'gemini-pro',
      //   apiKey: process.env.GOOGLE_API_KEY || 'demo-key',
      // },
      // Ollama (local):
      // llmProvider: {
      //   providerType: 'ollama',
      //   model: process.env.OLLAMA_DEFAULT_MODEL || 'llama2',
      //   baseUrl: process.env.OLLAMA_BASE_URL || 'http://localhost:11434',
      // },
      llmConfig: {
        temperature: 0.2, // Lower temperature for consistent classification
        maxTokens: 200,
        topP: 0.9,
        frequencyPenalty: 0.0,
        presencePenalty: 0.0,
        stopSequences: ["CLASSIFICATION COMPLETE"],
      },
      prompts: {
        system:
          "You are a content classification expert. Analyze the provided content and classify it into categories.",
        userTemplate:
          "Please classify this content: {content}\n\nContent type: {content_type}",
        contextInstructions:
          "Focus on the technical depth and intended audience.",
      },
      capabilities: ["content_analysis", "categorization"],
      tools: [],
    });

    console.log(`✅ Created classification agent: ${classificationAgent.id}`);

    const reviewAgent = await client.createAgent({
      name: "Content Review Agent",
      description: "Reviews content for quality and accuracy",
      // Using Anthropic as default (requires ANTHROPIC_API_KEY in .env)
      llmProvider: {
        providerType: "anthropic",
        model:
          process.env.ANTHROPIC_DEFAULT_MODEL || "claude-3-5-sonnet-20241022",
        apiKey: process.env.ANTHROPIC_API_KEY || "demo-key",
        ...(process.env.ANTHROPIC_BASE_URL && {
          baseUrl: process.env.ANTHROPIC_BASE_URL,
        }),
      },
      // Alternative providers (uncomment to use):
      // OpenAI:
      // llmProvider: {
      //   providerType: 'openai',
      //   model: process.env.OPENAI_DEFAULT_MODEL || 'gpt-4',
      //   apiKey: process.env.OPENAI_API_KEY || 'demo-key',
      //   baseUrl: process.env.OPENAI_BASE_URL,
      // },
      // Google Gemini:
      // llmProvider: {
      //   providerType: 'google',
      //   model: process.env.GOOGLE_DEFAULT_MODEL || 'gemini-pro',
      //   apiKey: process.env.GOOGLE_API_KEY || 'demo-key',
      // },
      // Ollama (local):
      // llmProvider: {
      //   providerType: 'ollama',
      //   model: process.env.OLLAMA_DEFAULT_MODEL || 'llama2',
      //   baseUrl: process.env.OLLAMA_BASE_URL || 'http://localhost:11434',
      // },
      llmConfig: {
        temperature: 0.3,
        maxTokens: 500,
        topP: 0.9,
        frequencyPenalty: 0.0,
        presencePenalty: 0.0,
        stopSequences: ["REVIEW COMPLETE"],
      },
      prompts: {
        system:
          "You are a content quality reviewer. Analyze content for accuracy, clarity, and completeness.",
        userTemplate:
          "Please review this {content_type} content:\n\n{content}\n\nClassification: {classification}\nPriority: {priority}",
        contextInstructions: "Focus on technical accuracy and readability.",
      },
      capabilities: ["content_review", "quality_assessment"],
      tools: [],
    });

    console.log(`✅ Created review agent: ${reviewAgent.id}`);

    // Configure State AI Agents
    console.log("\n⚙️  Configuring State AI Agents...");

    const classificationConfig = await client.createStateAgentConfig({
      stateId: "pending_classification",
      agentId: classificationAgent.id,
      llmConfig: {
        temperature: parseFloat(
          process.env.AGENT_CLASSIFICATION_TEMPERATURE || "0.1",
        ), // Very low for consistent classification with Anthropic
        maxTokens: parseInt(
          process.env.AGENT_CLASSIFICATION_MAX_TOKENS || "100",
        ),
        topP: 0.9,
        frequencyPenalty: 0.0,
        presencePenalty: 0.0,
        stopSequences: ["CLASSIFICATION COMPLETE"],
      },
      triggerConditions: [
        {
          id: "has_content",
          condition: {
            type: "field_exists",
            field: "data.content",
          },
        },
        {
          id: "unclassified",
          condition: {
            type: "field_equals",
            field: "metadata.status",
            value: "unclassified",
          },
        },
      ],
      inputMapping: {
        content: "data.content",
        content_type: "metadata.type",
      },
      outputMapping: {
        "data.classification": "category",
        "data.confidence": "confidence_score",
        "metadata.classifier": "agent_id",
        "metadata.classified_at": "timestamp",
      },
      autoActivity: "move_to_categorized",
      schedule: {
        initialDelaySeconds: parseInt(
          process.env.AGENT_INITIAL_DELAY_SECONDS || "2",
        ),
        maxExecutions: 1,
      },
      retryConfig: {
        maxAttempts: parseInt(process.env.AGENT_DEFAULT_MAX_ATTEMPTS || "2"),
        backoffSeconds: parseInt(
          process.env.AGENT_DEFAULT_BACKOFF_SECONDS || "5",
        ),
        retryOnErrors: ["timeout", "rate_limit"],
      },
      enabled: true,
    });

    console.log(`✅ Created classification config: ${classificationConfig.id}`);

    const reviewConfig = await client.createStateAgentConfig({
      stateId: "pending_review",
      agentId: reviewAgent.id,
      triggerConditions: [
        {
          id: "has_content",
          condition: {
            type: "field_exists",
            field: "data.content",
          },
        },
        {
          id: "has_classification",
          condition: {
            type: "field_exists",
            field: "data.classification",
          },
        },
      ],
      inputMapping: {
        content: "data.content",
        content_type: "metadata.type",
        classification: "data.classification",
        priority: "metadata.priority",
      },
      outputMapping: {
        "data.review_result": "assessment",
        "data.review_score": "quality_score",
        "metadata.reviewer": "agent_id",
        "metadata.review_timestamp": "timestamp",
      },
      schedule: {
        initialDelaySeconds: parseInt(
          process.env.AGENT_INITIAL_DELAY_SECONDS || "1",
        ),
        maxExecutions: 1,
      },
      retryConfig: {
        maxAttempts: parseInt(process.env.AGENT_DEFAULT_MAX_ATTEMPTS || "3"),
        backoffSeconds: parseInt(
          process.env.AGENT_DEFAULT_BACKOFF_SECONDS || "10",
        ),
        retryOnErrors: ["timeout", "rate_limit", "network_error"],
      },
      enabled: true,
    });

    console.log(`✅ Created review config: ${reviewConfig.id}`);

    // Create demo resources
    console.log("\n📄 Creating demo resources...");

    const docResource = await client.createResource(
      "document_workflow",
      {
        content:
          "This is a technical document about Rust programming and async/await patterns.",
      },
      {
        type: "document",
        status: "unclassified",
      },
    );

    console.log(`✅ Created document resource: ${docResource.id}`);

    // Trigger state agents
    console.log("\n🚀 Triggering state agents...");
    const executions = await client.triggerStateAgents(docResource.id);
    console.log(`📊 Started ${executions.length} agent executions`);

    // Subscribe to execution streams
    if (executions.length > 0) {
      console.log("\n📡 Subscribing to agent execution streams...");

      executions.forEach((execution) => {
        const ws = client.subscribeToAgentExecution(execution.id, (event) => {
          console.log(
            `🔄 Agent ${execution.agentId} [${event.eventType}]: ${event.content || event.status || event.error}`,
          );
        });

        // Close WebSocket after 30 seconds
        setTimeout(() => {
          ws.close();
        }, 30000);
      });
    }

    // Wait and check execution status
    console.log("\n⏳ Waiting for executions to complete...");
    await new Promise((resolve) => setTimeout(resolve, 5000));

    for (const execution of executions) {
      const updated = await client.getAgentExecution(execution.id);
      if (updated) {
        console.log(
          `📋 Execution ${execution.id}: ${updated.status} (${updated.durationMs}ms)`,
        );
        if (updated.outputData) {
          console.log(
            `   Output: ${JSON.stringify(updated.outputData, null, 2)}`,
          );
        }
      }
    }

    // Check updated resource
    const updatedResource = await client.getResource(docResource.id);
    if (updatedResource) {
      console.log("\n📄 Updated resource data:");
      console.log(
        `   Classification: ${updatedResource.data.classification || "Not set"}`,
      );
      console.log(
        `   Confidence: ${updatedResource.data.confidence || "Not set"}`,
      );
      console.log(
        `   Classifier: ${updatedResource.metadata.classifier || "Not set"}`,
      );
    }

    console.log("\n✨ Demo completed successfully!");
  } catch (error) {
    console.error("❌ Demo failed:", error);
    process.exit(1);
  }
}

// Run the demo
if (import.meta.url === `file://${process.argv[1]}`) {
  runDemo().catch(console.error);
}

export {
  StatesAIAgentClient,
  type AgentDefinition,
  type StateAgentConfig,
  type AgentExecution,
  type AgentStreamEvent,
};
