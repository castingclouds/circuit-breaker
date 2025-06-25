/**
 * LLM API client for Circuit Breaker TypeScript SDK
 *
 * This client only communicates with the Circuit Breaker router,
 * which presents an OpenAI-compatible API and handles all provider routing internally.
 */

import {
  ChatMessage,
  ChatCompletionRequest,
  ChatCompletionResponse,
  SmartCompletionRequest,
  CircuitBreakerOptions,
  RoutingStrategy,
  TaskType,
  BudgetConstraint,
  ModelInfo,
  ModelsResponse,
} from "./types.js";
import type { Client } from "./client.js";
import { streamChatCompletionFromRouter } from "./sse";

export class LLMClient {
  constructor(private client: Client) {}

  /**
   * Send a smart completion request with Circuit Breaker routing
   */
  async smartCompletion(
    request: SmartCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    const chatRequest: ChatCompletionRequest = {
      model: request.model,
      messages: request.messages,
      ...(request.temperature !== undefined && {
        temperature: request.temperature,
      }),
      ...(request.max_tokens !== undefined && {
        max_tokens: request.max_tokens,
      }),
      ...(request.stream !== undefined && { stream: request.stream }),
      ...(request.circuit_breaker && {
        circuit_breaker: request.circuit_breaker,
      }),
    };
    return this.chatCompletion(chatRequest);
  }

  /**
   * Send a chat completion request to the Circuit Breaker router
   */
  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    const config = this.client.getConfig();

    const response = await fetch(`${config.baseUrl}/v1/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(config.apiKey && { Authorization: `Bearer ${config.apiKey}` }),
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(
        `Circuit Breaker router error: ${response.status} ${response.statusText}`,
      );
    }

    return response.json();
  }

  /**
   * Simple chat method for single message
   */
  async chat(
    model: string,
    message: string,
    options?: {
      systemPrompt?: string;
      temperature?: number;
      maxTokens?: number;
    },
  ): Promise<string> {
    const messages: ChatMessage[] = [];

    if (options?.systemPrompt) {
      messages.push({ role: "system", content: options.systemPrompt });
    }

    messages.push({ role: "user", content: message });

    const request: ChatCompletionRequest = {
      model,
      messages,
      ...(options?.temperature !== undefined && {
        temperature: options.temperature,
      }),
      ...(options?.maxTokens !== undefined && {
        max_tokens: options.maxTokens,
      }),
    };

    const response = await this.chatCompletion(request);
    return response.choices[0]?.message?.content || "";
  }

  /**
   * List available models from the Circuit Breaker router
   */
  async listModels(): Promise<ModelInfo[]> {
    const config = this.client.getConfig();

    const response = await fetch(`${config.baseUrl}/v1/models`, {
      headers: {
        ...(config.apiKey && { Authorization: `Bearer ${config.apiKey}` }),
      },
    });

    if (!response.ok) {
      throw new Error(
        `Failed to list models: ${response.status} ${response.statusText}`,
      );
    }

    const modelsResponse: ModelsResponse = await response.json();
    return modelsResponse.data;
  }

  /**
   * Get model details
   */
  async getModel(modelId: string): Promise<ModelInfo> {
    const config = this.client.getConfig();

    const response = await fetch(`${config.baseUrl}/v1/models/${modelId}`, {
      headers: {
        ...(config.apiKey && { Authorization: `Bearer ${config.apiKey}` }),
      },
    });

    if (!response.ok) {
      throw new Error(
        `Failed to get model: ${response.status} ${response.statusText}`,
      );
    }

    return response.json();
  }

  /**
   * Stream chat completion using Circuit Breaker router's SSE endpoint (callback style)
   */
  async streamChatCompletion(
    request: ChatCompletionRequest & { stream: true },
    onChunk: (chunk: ChatCompletionChunk) => void,
    onError?: (error: Error) => void,
  ): Promise<void> {
    const config = this.client.getConfig();

    try {
      // Use the helper function to stream from router
      const streamGenerator = streamChatCompletionFromRouter(
        config.baseUrl,
        request,
        {
          headers: config.apiKey
            ? { Authorization: `Bearer ${config.apiKey}` }
            : {},
          timeout: 30000, // 30 second timeout
        },
      );

      for await (const streamingChunk of streamGenerator) {
        // Convert to ChatCompletionChunk format
        const chunk: ChatCompletionChunk = {
          id: streamingChunk.id,
          choices: streamingChunk.choices.map((choice) => ({
            index: choice.index,
            delta: {
              ...(choice.delta?.content && { content: choice.delta.content }),
              ...(choice.delta?.role && {
                role: choice.delta.role as "assistant",
              }),
            },
            ...(choice.finish_reason && {
              finish_reason: choice.finish_reason,
            }),
          })),
          model: streamingChunk.model,
          created: streamingChunk.created,
        };

        onChunk(chunk);
      }
    } catch (error) {
      if (onError) {
        onError(error instanceof Error ? error : new Error(String(error)));
      } else {
        throw error;
      }
    }
  }

  /**
   * Stream chat completion using Circuit Breaker router's SSE endpoint (async iterator style)
   */
  async *streamChatCompletionIterator(
    request: ChatCompletionRequest & { stream: true },
  ): AsyncGenerator<ChatCompletionChunk, void, unknown> {
    const config = this.client.getConfig();

    // Use the helper function to stream from router
    const streamGenerator = streamChatCompletionFromRouter(
      config.baseUrl,
      request,
      {
        headers: config.apiKey
          ? { Authorization: `Bearer ${config.apiKey}` }
          : {},
        timeout: 30000, // 30 second timeout
      },
    );

    for await (const streamingChunk of streamGenerator) {
      // Convert to ChatCompletionChunk format
      const chunk: ChatCompletionChunk = {
        id: streamingChunk.id,
        choices: streamingChunk.choices.map((choice) => ({
          index: choice.index,
          delta: {
            ...(choice.delta?.content && { content: choice.delta.content }),
            ...(choice.delta?.role && {
              role: choice.delta.role as "assistant",
            }),
          },
          ...(choice.finish_reason && {
            finish_reason: choice.finish_reason,
          }),
        })),
        model: streamingChunk.model,
        created: streamingChunk.created,
      };

      yield chunk;
    }
  }

  /**
   * Stream chat completion (convenience method that returns async iterator)
   * This method is used by the demo and provides a more convenient API
   */
  async *stream(
    request: ChatCompletionRequest & { stream: true },
  ): AsyncGenerator<ChatCompletionChunk, void, unknown> {
    yield* this.streamChatCompletionIterator(request);
  }

  /**
   * Get embeddings from the Circuit Breaker router
   */
  async embeddings(
    model: string,
    input: string | string[],
    options?: {
      dimensions?: number;
      encoding_format?: "float" | "base64";
    },
  ): Promise<EmbeddingResponse> {
    const config = this.client.getConfig();

    const response = await fetch(`${config.baseUrl}/v1/embeddings`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(config.apiKey && { Authorization: `Bearer ${config.apiKey}` }),
      },
      body: JSON.stringify({
        model,
        input,
        ...options,
      }),
    });

    if (!response.ok) {
      throw new Error(
        `Circuit Breaker router error: ${response.status} ${response.statusText}`,
      );
    }

    return response.json();
  }

  /**
   * Count tokens in text (if supported by the router)
   */
  async countTokens(_model: string, text: string): Promise<TokenCount> {
    // This would use a Circuit Breaker-specific endpoint if available
    // For now, provide a simple approximation
    const tokenCount = Math.ceil(text.length / 4); // Rough approximation
    return {
      tokens: tokenCount,
      estimated: true,
    };
  }

  /**
   * Get provider health status from Circuit Breaker router
   */
  async getProviderHealth(): Promise<ProviderHealth[]> {
    const config = this.client.getConfig();

    // This assumes the router exposes health information
    // The exact endpoint would depend on the router's API
    try {
      const response = await fetch(`${config.baseUrl}/health`, {
        headers: {
          ...(config.apiKey && { Authorization: `Bearer ${config.apiKey}` }),
        },
      });

      if (!response.ok) {
        throw new Error(`Health check failed: ${response.status}`);
      }

      const health = await response.json();

      // Transform router health format to expected format
      return [
        {
          provider: "circuit-breaker",
          status: health.status === "ok" ? "healthy" : "unhealthy",
          response_time_ms: health.response_time,
          last_check: new Date().toISOString(),
        },
      ];
    } catch (error) {
      return [
        {
          provider: "circuit-breaker",
          status: "unhealthy",
          last_check: new Date().toISOString(),
        },
      ];
    }
  }
}

/**
 * LLM Model information
 */
export interface LLMModel {
  id: string;
  name: string;
  provider: string;
  max_tokens: number;
  supports_streaming: boolean;
  supports_functions: boolean;
  cost_per_1k_tokens?: {
    input: number;
    output: number;
  };
}

/**
 * Chat completion chunk for streaming
 */
export interface ChatCompletionChunk {
  id: string;
  choices: ChoiceDelta[];
  model: string;
  created: number;
}

export interface ChoiceDelta {
  index: number;
  delta: MessageDelta;
  finish_reason?: string;
}

export interface MessageDelta {
  role?: "assistant";
  content?: string;
}

/**
 * Embedding response
 */
export interface EmbeddingResponse {
  object: "list";
  data: EmbeddingData[];
  model: string;
  usage: {
    prompt_tokens: number;
    total_tokens: number;
  };
}

export interface EmbeddingData {
  object: "embedding";
  index: number;
  embedding: number[];
}

/**
 * Token count response
 */
export interface TokenCount {
  tokens: number;
  estimated: boolean;
}

/**
 * Provider health information
 */
export interface ProviderHealth {
  provider: string;
  status: "healthy" | "degraded" | "unhealthy";
  response_time_ms?: number;
  error_rate?: number;
  last_check: string;
}

/**
 * Simple chat builder for multi-turn conversations
 */
export class ChatBuilder {
  private model: string;
  private messages: ChatMessage[] = [];
  private temperature?: number;
  private maxTokens?: number;
  private stream?: boolean;
  private circuitBreakerOptions?: CircuitBreakerOptions;

  constructor(model: string) {
    this.model = model;
  }

  /**
   * Set system prompt
   */
  setSystemPrompt(prompt: string): ChatBuilder {
    // Remove existing system message if any
    this.messages = this.messages.filter((m) => m.role !== "system");
    this.messages.unshift({ role: "system", content: prompt });
    return this;
  }

  /**
   * Add user message
   */
  addUserMessage(content: string): ChatBuilder {
    this.messages.push({ role: "user", content });
    return this;
  }

  /**
   * Add assistant message
   */
  addAssistantMessage(content: string): ChatBuilder {
    this.messages.push({ role: "assistant", content });
    return this;
  }

  /**
   * Set temperature
   */
  setTemperature(temperature: number): ChatBuilder {
    if (temperature < 0 || temperature > 2) {
      throw new Error("Temperature must be between 0 and 2");
    }
    this.temperature = temperature;
    return this;
  }

  /**
   * Set max tokens
   */
  setMaxTokens(maxTokens: number): ChatBuilder {
    if (maxTokens <= 0) {
      throw new Error("Max tokens must be greater than 0");
    }
    this.maxTokens = maxTokens;
    return this;
  }

  /**
   * Enable streaming
   */
  setStream(stream: boolean): ChatBuilder {
    this.stream = stream;
    return this;
  }

  /**
   * Set Circuit Breaker routing options
   */
  setCircuitBreakerOptions(options: CircuitBreakerOptions): ChatBuilder {
    this.circuitBreakerOptions = options;
    return this;
  }

  /**
   * Set routing strategy
   */
  setRoutingStrategy(strategy: RoutingStrategy): ChatBuilder {
    if (!this.circuitBreakerOptions) {
      this.circuitBreakerOptions = {};
    }
    this.circuitBreakerOptions.routing_strategy = strategy;
    return this;
  }

  /**
   * Set maximum cost per 1k tokens
   */
  setMaxCostPer1kTokens(maxCost: number): ChatBuilder {
    if (!this.circuitBreakerOptions) {
      this.circuitBreakerOptions = {};
    }
    this.circuitBreakerOptions.max_cost_per_1k_tokens = maxCost;
    return this;
  }

  /**
   * Set task type for optimized routing
   */
  setTaskType(taskType: TaskType): ChatBuilder {
    if (!this.circuitBreakerOptions) {
      this.circuitBreakerOptions = {};
    }
    this.circuitBreakerOptions.task_type = taskType;
    return this;
  }

  /**
   * Set fallback models
   */
  setFallbackModels(models: string[]): ChatBuilder {
    if (!this.circuitBreakerOptions) {
      this.circuitBreakerOptions = {};
    }
    this.circuitBreakerOptions.fallback_models = models;
    return this;
  }

  /**
   * Set maximum latency constraint
   */
  setMaxLatency(maxLatencyMs: number): ChatBuilder {
    if (!this.circuitBreakerOptions) {
      this.circuitBreakerOptions = {};
    }
    this.circuitBreakerOptions.max_latency_ms = maxLatencyMs;
    return this;
  }

  /**
   * Set budget constraints
   */
  setBudgetConstraint(budget: BudgetConstraint): ChatBuilder {
    if (!this.circuitBreakerOptions) {
      this.circuitBreakerOptions = {};
    }
    this.circuitBreakerOptions.budget_constraint = budget;
    return this;
  }

  /**
   * Build the chat completion request
   */
  build(): ChatCompletionRequest {
    if (this.messages.length === 0) {
      throw new Error("At least one message is required");
    }

    return {
      model: this.model,
      messages: this.messages,
      ...(this.temperature !== undefined && { temperature: this.temperature }),
      ...(this.maxTokens !== undefined && { max_tokens: this.maxTokens }),
      ...(this.stream !== undefined && { stream: this.stream }),
      ...(this.circuitBreakerOptions && {
        circuit_breaker: this.circuitBreakerOptions,
      }),
    };
  }

  /**
   * Build as smart completion request
   */
  buildSmart(): SmartCompletionRequest {
    if (this.messages.length === 0) {
      throw new Error("At least one message is required");
    }

    return {
      model: this.model,
      messages: this.messages,
      ...(this.temperature !== undefined && { temperature: this.temperature }),
      ...(this.maxTokens !== undefined && { max_tokens: this.maxTokens }),
      ...(this.stream !== undefined && { stream: this.stream }),
      ...(this.circuitBreakerOptions && {
        circuit_breaker: this.circuitBreakerOptions,
      }),
    };
  }

  /**
   * Execute the chat completion
   */
  async execute(client: LLMClient): Promise<ChatCompletionResponse> {
    const request = this.build();
    return client.chatCompletion(request);
  }

  /**
   * Execute as smart completion
   */
  async executeSmart(client: LLMClient): Promise<ChatCompletionResponse> {
    const request = this.buildSmart();
    return client.smartCompletion(request);
  }

  /**
   * Execute and return just the content
   */
  async getResponse(client: LLMClient): Promise<string> {
    const response = await this.execute(client);
    return response.choices[0]?.message?.content || "";
  }
}

/**
 * Create a new chat builder
 */
export function createChat(model: string): ChatBuilder {
  return new ChatBuilder(model);
}

/**
 * Create a smart chat builder with virtual model
 */
export function createSmartChat(virtualModel: string): ChatBuilder {
  return new ChatBuilder(virtualModel);
}

/**
 * Create a cost-optimized chat builder
 */
export function createCostOptimizedChat(): ChatBuilder {
  return new ChatBuilder(COMMON_MODELS.SMART_CHEAP).setRoutingStrategy(
    "cost_optimized",
  );
}

/**
 * Create a performance-optimized chat builder
 */
export function createFastChat(): ChatBuilder {
  return new ChatBuilder(COMMON_MODELS.SMART_FAST).setRoutingStrategy(
    "performance_first",
  );
}

/**
 * Create a balanced chat builder
 */
export function createBalancedChat(): ChatBuilder {
  return new ChatBuilder(COMMON_MODELS.SMART_BALANCED).setRoutingStrategy(
    "load_balanced",
  );
}

/**
 * Quick chat function for simple use cases
 */
export async function quickChat(
  client: LLMClient,
  model: string,
  prompt: string,
  options?: {
    systemPrompt?: string;
    temperature?: number;
    maxTokens?: number;
  },
): Promise<string> {
  return client.chat(model, prompt, options);
}

/**
 * Common model constants for Circuit Breaker router
 * These are examples - actual available models depend on router configuration
 */
export const COMMON_MODELS = {
  // Virtual Models (Circuit Breaker Smart Routing)
  SMART_FAST: "cb:fastest",
  SMART_CHEAP: "cb:cost-optimal",
  SMART_BALANCED: "cb:smart-chat",
  SMART_CREATIVE: "cb:creative",
  SMART_CODING: "cb:coding",
  SMART_ANALYSIS: "cb:analysis",

  // Direct Provider Models
  // OpenAI models
  GPT_3_5_TURBO: "gpt-3.5-turbo",
  GPT_4: "gpt-4",
  GPT_4_TURBO: "gpt-4-turbo-preview",
  GPT_4O_MINI: "gpt-4o-mini",

  // Anthropic models
  CLAUDE_3_HAIKU: "claude-3-haiku-20240307",
  CLAUDE_3_SONNET: "claude-3-sonnet-20240229",
  CLAUDE_3_OPUS: "claude-3-opus-20240229",

  // Google models
  GEMINI_FLASH: "gemini-1.5-flash",
  GEMINI_PRO: "gemini-1.5-pro",

  // Local models (via Ollama)
  LLAMA_2_7B: "llama2:7b",
  LLAMA_2_13B: "llama2:13b",
  LLAMA_3_1_8B: "llama3.1:8b",
  QWEN_CODER_3B: "qwen2.5-coder:3b",
  QWEN_CODER_7B: "qwen2.5-coder:7b",
  CODELLAMA_7B: "codellama:7b",

  // Embedding models
  NOMIC_EMBED: "nomic-embed-text:latest",
  ALL_MINILM: "all-minilm:l6-v2",
} as const;

/**
 * Create a conversation with context management
 */
export class Conversation {
  private client: LLMClient;
  private model: string;
  private messages: ChatMessage[] = [];
  private systemPrompt?: string;
  private maxContextLength: number = 4000; // tokens
  private temperature?: number;
  private maxTokens?: number;

  constructor(
    client: LLMClient,
    model: string,
    options?: {
      systemPrompt?: string;
      maxContextLength?: number;
      temperature?: number;
      maxTokens?: number;
    },
  ) {
    this.client = client;
    this.model = model;
    this.systemPrompt = options?.systemPrompt || undefined;
    this.maxContextLength = options?.maxContextLength || 4000;
    this.temperature = options?.temperature || undefined;
    this.maxTokens = options?.maxTokens || undefined;

    if (this.systemPrompt) {
      this.messages.push({ role: "system", content: this.systemPrompt });
    }
  }

  /**
   * Send a message and get response
   */
  async send(message: string): Promise<string> {
    this.messages.push({ role: "user", content: message });

    // Truncate context if needed (simple implementation)
    await this.truncateContext();

    const request: ChatCompletionRequest = {
      model: this.model,
      messages: this.messages,
      ...(this.temperature !== undefined && { temperature: this.temperature }),
      ...(this.maxTokens !== undefined && { max_tokens: this.maxTokens }),
    };

    const response = await this.client.chatCompletion(request);
    const assistantMessage = response.choices[0]?.message?.content || "";

    this.messages.push({ role: "assistant", content: assistantMessage });

    return assistantMessage;
  }

  /**
   * Send a message with streaming response
   */
  async sendStream(
    message: string,
    onChunk: (content: string) => void,
  ): Promise<string> {
    this.messages.push({ role: "user", content: message });
    await this.truncateContext();

    let fullResponse = "";

    await this.client.streamChatCompletion(
      {
        model: this.model,
        messages: this.messages,
        stream: true,
        ...(this.temperature !== undefined && {
          temperature: this.temperature,
        }),
        ...(this.maxTokens !== undefined && { max_tokens: this.maxTokens }),
      },
      (chunk) => {
        const content = chunk.choices[0]?.delta?.content || "";
        if (content) {
          fullResponse += content;
          onChunk(content);
        }
      },
    );

    this.messages.push({ role: "assistant", content: fullResponse });
    return fullResponse;
  }

  /**
   * Get conversation history
   */
  getHistory(): ChatMessage[] {
    return [...this.messages];
  }

  /**
   * Clear conversation history (keep system prompt)
   */
  clear(): void {
    this.messages = this.systemPrompt
      ? [{ role: "system", content: this.systemPrompt }]
      : [];
  }

  /**
   * Simple context truncation (remove oldest messages)
   */
  private async truncateContext(): Promise<void> {
    // This is a simplified implementation
    // In practice, you'd want to count tokens properly
    const maxMessages = Math.floor(this.maxContextLength / 100); // rough estimate

    if (this.messages.length > maxMessages) {
      const systemMessages = this.messages.filter((m) => m.role === "system");
      const otherMessages = this.messages
        .filter((m) => m.role !== "system")
        .slice(-maxMessages + systemMessages.length);
      this.messages = [...systemMessages, ...otherMessages];
    }
  }
}

/**
 * Create a new conversation
 */
export function createConversation(
  client: LLMClient,
  model: string,
  options?: {
    systemPrompt?: string;
    maxContextLength?: number;
    temperature?: number;
    maxTokens?: number;
  },
): Conversation {
  return new Conversation(client, model, options);
}
