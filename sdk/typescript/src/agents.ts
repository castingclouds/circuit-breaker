/**
 * Agents API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */

import {
  Agent,
  AgentCreateInput,
  AgentType,
  ChatMessage,
  PaginationOptions,
  // PaginatedResult,
} from "./types.js";
import type { Client } from "./client.js";

export class AgentClient {
  constructor(private client: Client) {}

  /**
   * Create a new agent
   */
  async create(input: AgentCreateInput): Promise<Agent> {
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
          createdAt
          updatedAt
        }
      }
    `;

    const variables = {
      input: {
        name: input.name,
        description: input.description || "",
        llmProvider: {
          providerType: input.config.llm_provider || "openai",
          model: input.config.model || "gpt-3.5-turbo",
          apiKey: process.env.OPENAI_API_KEY || "",
          baseUrl: null,
        },
        llmConfig: {
          temperature: input.config.temperature || 0.7,
          maxTokens: input.config.max_tokens || 1000,
          topP: null,
          frequencyPenalty: null,
          presencePenalty: null,
          stopSequences: [],
        },
        prompts: {
          system: input.config.system_prompt || "You are a helpful assistant.",
          userTemplate: "{message}",
          contextInstructions: null,
        },
        capabilities: ["chat", "completion"],
        tools: input.config.tools?.map((tool) => tool.name) || [],
      },
    };

    const result = await this.client.mutation<{ createAgent: Agent }>(
      mutation,
      variables,
    );
    return result.createAgent;
  }

  /**
   * Get an agent by ID
   */
  async get(id: string): Promise<Agent> {
    const query = `
      query GetAgent($id: ID!) {
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
          createdAt
          updatedAt
        }
      }
    `;

    const result = await this.client.query<{ agent: Agent }>(query, { id });
    return result.agent;
  }

  /**
   * List all agents
   */
  async list(_options?: PaginationOptions): Promise<Agent[]> {
    const query = `
      query GetAgents {
        agents {
          id
          name
          description
          llmProvider {
            name
            healthStatus {
              isHealthy
              lastCheck
              errorRate
              averageLatencyMs
              consecutiveFailures
              lastError
            }
          }
          llmConfig {
            model
            temperature
            maxTokens
          }
          prompts {
            system
            user
          }
          capabilities
          tools {
            name
            description
            parameters
          }
          createdAt
          updatedAt
        }
      }
    `;

    const result = await this.client.query<{ agents: Agent[] }>(query);
    return result.agents;
  }

  /**
   * Update an agent
   */
  async update(id: string, updates: Partial<AgentCreateInput>): Promise<Agent> {
    const mutation = `
      mutation UpdateAgent($id: ID!, $input: AgentDefinitionInput!) {
        updateAgent(id: $id, input: $input) {
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
          createdAt
          updatedAt
        }
      }
    `;

    const variables = {
      id,
      input: {
        name: updates.name,
        description: updates.description || "",
        llmProvider: {
          name: updates.config?.llm_provider || "openai",
        },
        llmConfig: {
          model: updates.config?.model || "gpt-3.5-turbo",
          temperature: updates.config?.temperature || 0.7,
          maxTokens: updates.config?.max_tokens || 1000,
        },
        prompts: {
          system: updates.config?.system_prompt || "",
          user: "",
        },
        capabilities: ["chat", "completion"],
        tools:
          updates.config?.tools?.map((tool) => ({
            name: tool.name,
            description: tool.description,
            parameters: tool.parameters,
          })) || [],
      },
    };

    const result = await this.client.mutation<{ updateAgent: Agent }>(
      mutation,
      variables,
    );
    return result.updateAgent;
  }

  /**
   * Delete an agent
   */
  async delete(id: string): Promise<boolean> {
    const mutation = `
      mutation DeleteAgent($id: ID!) {
        deleteAgent(id: $id) {
          success
        }
      }
    `;

    const result = await this.client.mutation<{
      deleteAgent: { success: boolean };
    }>(mutation, { id });
    return result.deleteAgent.success;
  }

  /**
   * Chat with an agent
   */
  async chat(id: string, messages: ChatMessage[]): Promise<any> {
    // First get the agent to retrieve its configuration
    const agent = await this.get(id);

    const mutation = `
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

    // Create messages including the agent's system prompt
    const agentMessages = [
      {
        role: "system",
        content: agent.prompts?.system || "You are a helpful assistant.",
      },
      ...messages.map((msg) => ({
        role: msg.role,
        content: msg.content,
      })),
    ];

    const variables = {
      input: {
        model: agent.llmProvider?.model || "gpt-3.5-turbo",
        messages: agentMessages,
        temperature: agent.llmConfig?.temperature || 0.7,
        maxTokens: agent.llmConfig?.maxTokens || 1000,
      },
    };

    const result = await this.client.mutation<{ llmChatCompletion: any }>(
      mutation,
      variables,
    );

    return result.llmChatCompletion;
  }

  /**
   * Execute an agent with input
   */
  async execute(id: string, input: Record<string, any>): Promise<any> {
    const mutation = `
      mutation ExecuteAgent($agentId: ID!, $input: JSON!) {
        executeAgent(agentId: $agentId, input: $input) {
          id
          status
          output
          error
          created_at
          completed_at
        }
      }
    `;

    const variables = {
      agentId: id,
      input,
    };

    const result = await this.client.mutation<{ executeAgent: any }>(
      mutation,
      variables,
    );
    return result.executeAgent;
  }
}

// ============================================================================
// Builder Pattern for Agent Creation
// ============================================================================

export class AgentBuilder {
  private agent: Partial<AgentCreateInput> = {
    config: {},
  };

  /**
   * Set agent name
   */
  setName(name: string): AgentBuilder {
    this.agent.name = name;
    return this;
  }

  /**
   * Set agent description
   */
  setDescription(description: string): AgentBuilder {
    this.agent.description = description;
    return this;
  }

  /**
   * Set agent type
   */
  setType(type: AgentType): AgentBuilder {
    this.agent.type = type;
    return this;
  }

  /**
   * Set LLM provider
   */
  setLLMProvider(provider: string): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    this.agent.config.llm_provider = provider;
    return this;
  }

  /**
   * Set LLM model
   */
  setModel(model: string): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    this.agent.config.model = model;
    return this;
  }

  /**
   * Set temperature
   */
  setTemperature(temperature: number): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    this.agent.config.temperature = temperature;
    return this;
  }

  /**
   * Set max tokens
   */
  setMaxTokens(maxTokens: number): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    this.agent.config.max_tokens = maxTokens;
    return this;
  }

  /**
   * Set system prompt
   */
  setSystemPrompt(prompt: string): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    this.agent.config.system_prompt = prompt;
    return this;
  }

  /**
   * Add a tool
   */
  addTool(
    name: string,
    description: string,
    parameters: Record<string, any>,
  ): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    if (!this.agent.config.tools) this.agent.config.tools = [];

    this.agent.config.tools.push({
      name,
      description,
      parameters,
    });
    return this;
  }

  /**
   * Set memory configuration
   */
  setMemory(
    type: "short_term" | "long_term" | "persistent",
    options?: { max_entries?: number; ttl?: number },
  ): AgentBuilder {
    if (!this.agent.config) this.agent.config = {};
    this.agent.config.memory = {
      type,
      ...(options?.max_entries !== undefined && {
        max_entries: options.max_entries,
      }),
      ...(options?.ttl !== undefined && { ttl: options.ttl }),
    };
    return this;
  }

  /**
   * Build the agent definition
   */
  build(): AgentCreateInput {
    if (!this.agent.name) {
      throw new Error("Agent name is required");
    }

    if (!this.agent.type) {
      this.agent.type = "conversational";
    }

    if (!this.agent.config) {
      this.agent.config = {};
    }

    return this.agent as AgentCreateInput;
  }
}

/**
 * Create a new agent builder
 */
export function createAgent(name: string): AgentBuilder {
  return new AgentBuilder().setName(name);
}
