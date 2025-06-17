/**
 * LLM Provider implementations for Circuit Breaker SDK
 *
 * Provides unified interfaces for different LLM providers with standardized:
 * - Authentication handling
 * - Request/response formatting
 * - Error handling and retries
 * - Cost estimation
 * - Model capability detection
 *
 * @example
 * ```typescript
 * const provider = new LLMProvider({
 *   name: 'openai-gpt4',
 *   type: 'openai',
 *   endpoint: 'https://api.openai.com/v1',
 *   apiKey: process.env.OPENAI_API_KEY,
 *   models: ['gpt-4', 'gpt-3.5-turbo']
 * });
 *
 * await provider.initialize();
 * const response = await provider.chatCompletion(request);
 * ```
 */

import fetch from "node-fetch";
import {
  LLMProviderConfig,
  LLMProviderType,
  ChatCompletionRequest,
  ChatCompletionResponse,
  ChatCompletionChunk,
  ChatMessage,
  ChatRole,
  Usage,
} from "../core/types.js";
import {
  LLMError,
  LLMProviderError,
  LLMProviderNotFoundError,
  LLMModelNotSupportedError,
  LLMRateLimitError,
  LLMQuotaExceededError,
  NetworkError,
  TimeoutError,
} from "../core/errors.js";
import { Logger, createComponentLogger } from "../utils/logger.js";

export interface ModelPricing {
  inputTokenPrice: number; // Price per 1K tokens
  outputTokenPrice: number; // Price per 1K tokens
  currency: string;
}

export interface ProviderCapabilities {
  supportsStreaming: boolean;
  supportsFunctionCalling: boolean;
  supportsSystemMessages: boolean;
  maxContextLength: number;
  maxOutputTokens: number;
}

export interface ProviderMetrics {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  averageLatency: number;
  totalCost: number;
  lastRequestTime?: Date;
}

/**
 * Base class for LLM providers
 */
export abstract class BaseLLMProvider {
  public readonly name: string;
  public readonly type: LLMProviderType;
  public readonly endpoint: string;
  public readonly apiKey?: string;
  public readonly models: string[];
  protected logger: Logger;
  protected metrics: ProviderMetrics;

  constructor(config: LLMProviderConfig, logger?: Logger) {
    this.name = config.name;
    this.type = config.type;
    this.endpoint = config.endpoint;
    this.apiKey = config.apiKey;
    this.models = config.models || [];
    this.logger = logger || createComponentLogger(`Provider:${this.name}`);

    this.metrics = {
      totalRequests: 0,
      successfulRequests: 0,
      failedRequests: 0,
      averageLatency: 0,
      totalCost: 0,
    };
  }

  abstract initialize(): Promise<void>;
  abstract chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse>;
  abstract chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk>;
  abstract healthCheck(): Promise<boolean>;
  abstract getModelPricing(model: string): ModelPricing | null;
  abstract getCapabilities(model: string): ProviderCapabilities;

  /**
   * Check if provider supports a specific model
   */
  supportsModel(model: string): boolean {
    return this.models.includes(model) || this.models.length === 0;
  }

  /**
   * Get list of supported models
   */
  getSupportedModels(): string[] {
    return [...this.models];
  }

  /**
   * Estimate cost for a request
   */
  estimateCost(request: ChatCompletionRequest): number {
    const pricing = this.getModelPricing(request.model);
    if (!pricing) return 0;

    // Rough token estimation (4 chars = 1 token)
    const inputTokens = this.estimateTokens(request.messages);
    const outputTokens = request.max_tokens || 150;

    const inputCost = (inputTokens / 1000) * pricing.inputTokenPrice;
    const outputCost = (outputTokens / 1000) * pricing.outputTokenPrice;

    return inputCost + outputCost;
  }

  /**
   * Estimate tokens for messages
   */
  protected estimateTokens(messages: ChatMessage[]): number {
    const totalChars = messages.reduce(
      (sum, msg) => sum + msg.content.length,
      0,
    );
    return Math.ceil(totalChars / 4); // Rough estimation
  }

  /**
   * Update metrics after request
   */
  protected updateMetrics(
    success: boolean,
    latency: number,
    cost: number = 0,
  ): void {
    this.metrics.totalRequests++;
    this.metrics.lastRequestTime = new Date();

    if (success) {
      this.metrics.successfulRequests++;
    } else {
      this.metrics.failedRequests++;
    }

    // Update average latency
    this.metrics.averageLatency =
      (this.metrics.averageLatency * (this.metrics.totalRequests - 1) +
        latency) /
      this.metrics.totalRequests;

    this.metrics.totalCost += cost;
  }

  /**
   * Get provider metrics
   */
  getMetrics(): ProviderMetrics {
    return { ...this.metrics };
  }

  /**
   * Handle HTTP errors from provider APIs
   */
  protected handleHttpError(response: any, body: any): never {
    const status = response.status;
    const message = body?.error?.message || body?.message || "Unknown error";

    switch (status) {
      case 401:
        throw new LLMProviderError(
          this.name,
          `Authentication failed: ${message}`,
        );
      case 403:
        throw new LLMProviderError(this.name, `Permission denied: ${message}`);
      case 429:
        throw new LLMRateLimitError(this.name, message);
      case 500:
      case 502:
      case 503:
      case 504:
        throw new LLMProviderError(this.name, `Server error: ${message}`);
      default:
        throw new LLMProviderError(this.name, `HTTP ${status}: ${message}`);
    }
  }

  /**
   * Clean up provider resources
   */
  async destroy?(): Promise<void> {
    // Override in subclasses if needed
  }
}

/**
 * OpenAI provider implementation
 */
export class OpenAIProvider extends BaseLLMProvider {
  private readonly baseUrl: string;

  constructor(config: LLMProviderConfig, logger?: Logger) {
    super(config, logger);
    this.baseUrl = config.endpoint || "https://api.openai.com/v1";
  }

  async initialize(): Promise<void> {
    if (!this.apiKey) {
      throw new LLMProviderError(
        this.name,
        "API key is required for OpenAI provider",
      );
    }

    // Test the connection
    try {
      await this.healthCheck();
      this.logger.info("OpenAI provider initialized successfully");
    } catch (error) {
      throw new LLMProviderError(
        this.name,
        `Failed to initialize: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: request.model,
          messages: request.messages,
          temperature: request.temperature,
          max_tokens: request.max_tokens,
          top_p: request.top_p,
          frequency_penalty: request.frequency_penalty,
          presence_penalty: request.presence_penalty,
          stop: request.stop,
          stream: false,
          tools: request.tools,
          tool_choice: request.tool_choice,
          user: request.user,
        }),
      });

      const body = await response.json();

      if (!response.ok) {
        this.handleHttpError(response, body);
      }

      const latency = Date.now() - startTime;
      const cost = this.calculateActualCost(body.usage, request.model);
      this.updateMetrics(true, latency, cost);

      return body as ChatCompletionResponse;
    } catch (error) {
      const latency = Date.now() - startTime;
      this.updateMetrics(false, latency);

      if (error instanceof LLMError) {
        throw error;
      }

      throw new LLMProviderError(
        this.name,
        `Chat completion failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async *chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          ...request,
          stream: true,
        }),
      });

      if (!response.ok) {
        const body = await response.json();
        this.handleHttpError(response, body);
      }

      if (!response.body) {
        throw new LLMProviderError(this.name, "No response body for streaming");
      }

      const decoder = new TextDecoder();
      const reader = response.body.getReader();

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value);
          const lines = chunk.split("\n").filter((line) => line.trim() !== "");

          for (const line of lines) {
            if (line.startsWith("data: ")) {
              const data = line.slice(6);
              if (data === "[DONE]") {
                const latency = Date.now() - startTime;
                this.updateMetrics(true, latency);
                return;
              }

              try {
                const parsed = JSON.parse(data);
                yield parsed as ChatCompletionChunk;
              } catch (error) {
                this.logger.warn("Failed to parse streaming chunk", {
                  line,
                  error,
                });
              }
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      const latency = Date.now() - startTime;
      this.updateMetrics(false, latency);

      if (error instanceof LLMError) {
        throw error;
      }

      throw new LLMProviderError(
        this.name,
        `Streaming failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/models`, {
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
        },
      });

      return response.ok;
    } catch {
      return false;
    }
  }

  getModelPricing(model: string): ModelPricing | null {
    const pricing: Record<string, ModelPricing> = {
      "gpt-4": {
        inputTokenPrice: 0.03,
        outputTokenPrice: 0.06,
        currency: "USD",
      },
      "gpt-4-turbo": {
        inputTokenPrice: 0.01,
        outputTokenPrice: 0.03,
        currency: "USD",
      },
      "gpt-3.5-turbo": {
        inputTokenPrice: 0.0015,
        outputTokenPrice: 0.002,
        currency: "USD",
      },
      "gpt-3.5-turbo-16k": {
        inputTokenPrice: 0.003,
        outputTokenPrice: 0.004,
        currency: "USD",
      },
    };

    return pricing[model] || null;
  }

  getCapabilities(model: string): ProviderCapabilities {
    const capabilities: Record<string, ProviderCapabilities> = {
      "gpt-4": {
        supportsStreaming: true,
        supportsFunctionCalling: true,
        supportsSystemMessages: true,
        maxContextLength: 8192,
        maxOutputTokens: 4096,
      },
      "gpt-4-turbo": {
        supportsStreaming: true,
        supportsFunctionCalling: true,
        supportsSystemMessages: true,
        maxContextLength: 128000,
        maxOutputTokens: 4096,
      },
      "gpt-3.5-turbo": {
        supportsStreaming: true,
        supportsFunctionCalling: true,
        supportsSystemMessages: true,
        maxContextLength: 4096,
        maxOutputTokens: 4096,
      },
    };

    return (
      capabilities[model] || {
        supportsStreaming: false,
        supportsFunctionCalling: false,
        supportsSystemMessages: true,
        maxContextLength: 4096,
        maxOutputTokens: 1024,
      }
    );
  }

  private calculateActualCost(usage: Usage, model: string): number {
    const pricing = this.getModelPricing(model);
    if (!pricing || !usage) return 0;

    const inputCost = (usage.prompt_tokens / 1000) * pricing.inputTokenPrice;
    const outputCost =
      (usage.completion_tokens / 1000) * pricing.outputTokenPrice;

    return inputCost + outputCost;
  }
}

/**
 * Anthropic provider implementation
 */
export class AnthropicProvider extends BaseLLMProvider {
  private readonly baseUrl: string;

  constructor(config: LLMProviderConfig, logger?: Logger) {
    super(config, logger);
    this.baseUrl = config.endpoint || "https://api.anthropic.com/v1";
  }

  async initialize(): Promise<void> {
    if (!this.apiKey) {
      throw new LLMProviderError(
        this.name,
        "API key is required for Anthropic provider",
      );
    }

    try {
      await this.healthCheck();
      this.logger.info("Anthropic provider initialized successfully");
    } catch (error) {
      throw new LLMProviderError(
        this.name,
        `Failed to initialize: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    const startTime = Date.now();

    try {
      // Convert OpenAI format to Anthropic format
      const anthropicRequest = this.convertToAnthropicFormat(request);

      const response = await fetch(`${this.baseUrl}/messages`, {
        method: "POST",
        headers: {
          "x-api-key": this.apiKey!,
          "Content-Type": "application/json",
          "anthropic-version": "2023-06-01",
        },
        body: JSON.stringify(anthropicRequest),
      });

      const body = await response.json();

      if (!response.ok) {
        this.handleHttpError(response, body);
      }

      const latency = Date.now() - startTime;
      const openaiResponse = this.convertFromAnthropicFormat(
        body,
        request.model,
      );
      const cost = this.calculateActualCost(
        openaiResponse.usage!,
        request.model,
      );
      this.updateMetrics(true, latency, cost);

      return openaiResponse;
    } catch (error) {
      const latency = Date.now() - startTime;
      this.updateMetrics(false, latency);

      if (error instanceof LLMError) {
        throw error;
      }

      throw new LLMProviderError(
        this.name,
        `Chat completion failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async *chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk> {
    const startTime = Date.now();

    try {
      const anthropicRequest = this.convertToAnthropicFormat(request);
      anthropicRequest.stream = true;

      const response = await fetch(`${this.baseUrl}/messages`, {
        method: "POST",
        headers: {
          "x-api-key": this.apiKey!,
          "Content-Type": "application/json",
          "anthropic-version": "2023-06-01",
        },
        body: JSON.stringify(anthropicRequest),
      });

      if (!response.ok) {
        const body = await response.json();
        this.handleHttpError(response, body);
      }

      if (!response.body) {
        throw new LLMProviderError(this.name, "No response body for streaming");
      }

      const decoder = new TextDecoder();
      const reader = response.body.getReader();

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value);
          const lines = chunk.split("\n").filter((line) => line.trim() !== "");

          for (const line of lines) {
            if (line.startsWith("data: ")) {
              const data = line.slice(6);

              try {
                const parsed = JSON.parse(data);
                const openaiChunk = this.convertAnthropicStreamToOpenAI(
                  parsed,
                  request.model,
                );
                if (openaiChunk) {
                  yield openaiChunk;
                }
              } catch (error) {
                this.logger.warn("Failed to parse streaming chunk", {
                  line,
                  error,
                });
              }
            }
          }
        }

        const latency = Date.now() - startTime;
        this.updateMetrics(true, latency);
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      const latency = Date.now() - startTime;
      this.updateMetrics(false, latency);

      if (error instanceof LLMError) {
        throw error;
      }

      throw new LLMProviderError(
        this.name,
        `Streaming failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async healthCheck(): Promise<boolean> {
    try {
      // Anthropic doesn't have a dedicated health endpoint, so we'll use a minimal request
      const response = await fetch(`${this.baseUrl}/messages`, {
        method: "POST",
        headers: {
          "x-api-key": this.apiKey!,
          "Content-Type": "application/json",
          "anthropic-version": "2023-06-01",
        },
        body: JSON.stringify({
          model: "claude-3-haiku-20240307",
          max_tokens: 1,
          messages: [{ role: "user", content: "hi" }],
        }),
      });

      return response.status !== 401 && response.status !== 403;
    } catch {
      return false;
    }
  }

  getModelPricing(model: string): ModelPricing | null {
    const pricing: Record<string, ModelPricing> = {
      "claude-3-opus-20240229": {
        inputTokenPrice: 0.015,
        outputTokenPrice: 0.075,
        currency: "USD",
      },
      "claude-3-sonnet-20240229": {
        inputTokenPrice: 0.003,
        outputTokenPrice: 0.015,
        currency: "USD",
      },
      "claude-3-haiku-20240307": {
        inputTokenPrice: 0.00025,
        outputTokenPrice: 0.00125,
        currency: "USD",
      },
    };

    return pricing[model] || null;
  }

  getCapabilities(model: string): ProviderCapabilities {
    return {
      supportsStreaming: true,
      supportsFunctionCalling: true,
      supportsSystemMessages: true,
      maxContextLength: 200000,
      maxOutputTokens: 4096,
    };
  }

  private convertToAnthropicFormat(request: ChatCompletionRequest): any {
    const messages = request.messages.map((msg) => ({
      role: msg.role === "assistant" ? "assistant" : "user",
      content: msg.content,
    }));

    return {
      model: request.model,
      max_tokens: request.max_tokens || 1000,
      messages,
      temperature: request.temperature,
      top_p: request.top_p,
      stop_sequences: request.stop,
    };
  }

  private convertFromAnthropicFormat(
    anthropicResponse: any,
    model: string,
  ): ChatCompletionResponse {
    return {
      id: anthropicResponse.id,
      object: "chat.completion",
      created: Math.floor(Date.now() / 1000),
      model,
      choices: [
        {
          index: 0,
          message: {
            role: "assistant",
            content: anthropicResponse.content[0]?.text || "",
          },
          finish_reason:
            anthropicResponse.stop_reason === "end_turn" ? "stop" : "length",
        },
      ],
      usage: {
        prompt_tokens: anthropicResponse.usage?.input_tokens || 0,
        completion_tokens: anthropicResponse.usage?.output_tokens || 0,
        total_tokens:
          (anthropicResponse.usage?.input_tokens || 0) +
          (anthropicResponse.usage?.output_tokens || 0),
      },
    };
  }

  private convertAnthropicStreamToOpenAI(
    chunk: any,
    model: string,
  ): ChatCompletionChunk | null {
    if (chunk.type === "content_block_delta" && chunk.delta?.text) {
      return {
        id: `chatcmpl-${Date.now()}`,
        object: "chat.completion.chunk",
        created: Math.floor(Date.now() / 1000),
        model,
        choices: [
          {
            index: 0,
            delta: {
              role: "assistant",
              content: chunk.delta.text,
            },
            finish_reason: null,
          },
        ],
      };
    }

    if (chunk.type === "message_stop") {
      return {
        id: `chatcmpl-${Date.now()}`,
        object: "chat.completion.chunk",
        created: Math.floor(Date.now() / 1000),
        model,
        choices: [
          {
            index: 0,
            delta: {},
            finish_reason: "stop",
          },
        ],
      };
    }

    return null;
  }

  private calculateActualCost(usage: Usage, model: string): number {
    const pricing = this.getModelPricing(model);
    if (!pricing || !usage) return 0;

    const inputCost = (usage.prompt_tokens / 1000) * pricing.inputTokenPrice;
    const outputCost =
      (usage.completion_tokens / 1000) * pricing.outputTokenPrice;

    return inputCost + outputCost;
  }
}

/**
 * Ollama provider implementation for local models
 */
export class OllamaProvider extends BaseLLMProvider {
  constructor(config: LLMProviderConfig, logger?: Logger) {
    super(config, logger);
  }

  async initialize(): Promise<void> {
    try {
      const isHealthy = await this.healthCheck();
      if (!isHealthy) {
        throw new Error("Ollama server is not responding");
      }
      this.logger.info("Ollama provider initialized successfully");
    } catch (error) {
      throw new LLMProviderError(
        this.name,
        `Failed to initialize: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.endpoint}/api/chat`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: request.model,
          messages: request.messages,
          stream: false,
          options: {
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop,
          },
        }),
      });

      const body = await response.json();

      if (!response.ok) {
        this.handleHttpError(response, body);
      }

      const latency = Date.now() - startTime;
      const openaiResponse = this.convertOllamaToOpenAI(body, request.model);
      this.updateMetrics(true, latency, 0); // Ollama is free

      return openaiResponse;
    } catch (error) {
      const latency = Date.now() - startTime;
      this.updateMetrics(false, latency);

      if (error instanceof LLMError) {
        throw error;
      }

      throw new LLMProviderError(
        this.name,
        `Chat completion failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async *chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.endpoint}/api/chat`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: request.model,
          messages: request.messages,
          stream: true,
          options: {
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop,
          },
        }),
      });

      if (!response.ok) {
        const body = await response.json();
        this.handleHttpError(response, body);
      }

      if (!response.body) {
        throw new LLMProviderError(this.name, "No response body for streaming");
      }

      const decoder = new TextDecoder();
      const reader = response.body.getReader();

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value);
          const lines = chunk.split("\n").filter((line) => line.trim() !== "");

          for (const line of lines) {
            try {
              const parsed = JSON.parse(line);
              const openaiChunk = this.convertOllamaStreamToOpenAI(
                parsed,
                request.model,
              );
              if (openaiChunk) {
                yield openaiChunk;
                if (parsed.done) {
                  const latency = Date.now() - startTime;
                  this.updateMetrics(true, latency, 0);
                  return;
                }
              }
            } catch (error) {
              this.logger.warn("Failed to parse streaming chunk", {
                line,
                error,
              });
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      const latency = Date.now() - startTime;
      this.updateMetrics(false, latency);

      if (error instanceof LLMError) {
        throw error;
      }

      throw new LLMProviderError(
        this.name,
        `Streaming failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.endpoint}/api/tags`);
      return response.ok;
    } catch {
      return false;
    }
  }

  getModelPricing(model: string): ModelPricing | null {
    // Ollama is free for local hosting
    return { inputTokenPrice: 0, outputTokenPrice: 0, currency: "USD" };
  }

  getCapabilities(model: string): ProviderCapabilities {
    return {
      supportsStreaming: true,
      supportsFunctionCalling: false, // Most Ollama models don't support function calling
      supportsSystemMessages: true,
      maxContextLength: 4096, // Varies by model
      maxOutputTokens: 2048,
    };
  }

  private convertOllamaToOpenAI(
    ollamaResponse: any,
    model: string,
  ): ChatCompletionResponse {
    return {
      id: `chatcmpl-${Date.now()}`,
      object: "chat.completion",
      created: Math.floor(Date.now() / 1000),
      model,
      choices: [
        {
          index: 0,
          message: {
            role: "assistant",
            content: ollamaResponse.message?.content || "",
          },
          finish_reason: ollamaResponse.done ? "stop" : "length",
        },
      ],
      usage: {
        prompt_tokens: 0, // Ollama doesn't provide token counts
        completion_tokens: 0,
        total_tokens: 0,
      },
    };
  }

  private convertOllamaStreamToOpenAI(
    chunk: any,
    model: string,
  ): ChatCompletionChunk | null {
    if (chunk.message?.content) {
      return {
        id: `chatcmpl-${Date.now()}`,
        object: "chat.completion.chunk",
        created: Math.floor(Date.now() / 1000),
        model,
        choices: [
          {
            index: 0,
            delta: {
              role: "assistant",
              content: chunk.message.content,
            },
            finish_reason: chunk.done ? "stop" : null,
          },
        ],
      };
    }

    return null;
  }
}

/**
 * LLMProvider factory and manager
 */
export class LLMProvider {
  private provider: BaseLLMProvider;

  constructor(config: LLMProviderConfig, logger?: Logger) {
    switch (config.type) {
      case "openai":
        this.provider = new OpenAIProvider(config, logger);
        break;
      case "anthropic":
        this.provider = new AnthropicProvider(config, logger);
        break;
      case "ollama":
        this.provider = new OllamaProvider(config, logger);
        break;
      default:
        throw new LLMProviderNotFoundError(
          `Provider type '${config.type}' is not supported`,
        );
    }
  }

  get name(): string {
    return this.provider.name;
  }

  get type(): LLMProviderType {
    return this.provider.type;
  }

  async initialize(): Promise<void> {
    return this.provider.initialize();
  }

  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    return this.provider.chatCompletion(request);
  }

  async chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk> {
    return this.provider.chatCompletionStream(request);
  }

  async healthCheck(): Promise<boolean> {
    return this.provider.healthCheck();
  }

  supportsModel(model: string): boolean {
    return this.provider.supportsModel(model);
  }

  getSupportedModels(): string[] {
    return this.provider.getSupportedModels();
  }

  estimateCost(request: ChatCompletionRequest): number {
    return this.provider.estimateCost(request);
  }

  getModelPricing(model: string): ModelPricing | null {
    return this.provider.getModelPricing(model);
  }

  getCapabilities(model: string): ProviderCapabilities {
    return this.provider.getCapabilities(model);
  }

  getMetrics(): ProviderMetrics {
    return this.provider.getMetrics();
  }

  async destroy(): Promise<void> {
    return this.provider.destroy?.();
  }
}

/**
 * Provider factory functions
 */
export function createOpenAIProvider(
  config: Omit<LLMProviderConfig, "type">,
  logger?: Logger,
): LLMProvider {
  return new LLMProvider({ ...config, type: "openai" }, logger);
}

export function createAnthropicProvider(
  config: Omit<LLMProviderConfig, "type">,
  logger?: Logger,
): LLMProvider {
  return new LLMProvider({ ...config, type: "anthropic" }, logger);
}

export function createOllamaProvider(
  config: Omit<LLMProviderConfig, "type">,
  logger?: Logger,
): LLMProvider {
  return new LLMProvider({ ...config, type: "ollama" }, logger);
}

/**
 * Model registry for provider selection
 */
export const ModelRegistry = {
  // OpenAI models
  "gpt-4": "openai",
  "gpt-4-turbo": "openai",
  "gpt-4-turbo-preview": "openai",
  "gpt-3.5-turbo": "openai",
  "gpt-3.5-turbo-16k": "openai",

  // Anthropic models
  "claude-3-opus-20240229": "anthropic",
  "claude-3-sonnet-20240229": "anthropic",
  "claude-3-haiku-20240307": "anthropic",
  "claude-3-opus": "anthropic",
  "claude-3-sonnet": "anthropic",
  "claude-3-haiku": "anthropic",

  // Ollama models (common ones)
  llama2: "ollama",
  "llama2:7b": "ollama",
  "llama2:13b": "ollama",
  "llama2:70b": "ollama",
  mistral: "ollama",
  "mistral:7b": "ollama",
  codellama: "ollama",
  "codellama:7b": "ollama",
  vicuna: "ollama",
  alpaca: "ollama",
};

/**
 * Get the default provider type for a model
 */
export function getProviderForModel(model: string): LLMProviderType | null {
  return (ModelRegistry as any)[model] || null;
}

/**
 * Check if a model is supported by any provider
 */
export function isModelSupported(model: string): boolean {
  return model in ModelRegistry;
}

/**
 * Get all available models for a provider type
 */
export function getModelsForProvider(providerType: LLMProviderType): string[] {
  return Object.entries(ModelRegistry)
    .filter(([, provider]) => provider === providerType)
    .map(([model]) => model);
}

/**
 * Model capabilities database
 */
export const ModelCapabilities = {
  "gpt-4": {
    maxTokens: 8192,
    supportsStreaming: true,
    supportsFunctionCalling: true,
    supportsVision: false,
    reasoning: "advanced",
    speed: "slow",
    cost: "high",
  },
  "gpt-4-turbo": {
    maxTokens: 128000,
    supportsStreaming: true,
    supportsFunctionCalling: true,
    supportsVision: true,
    reasoning: "advanced",
    speed: "medium",
    cost: "medium",
  },
  "gpt-3.5-turbo": {
    maxTokens: 4096,
    supportsStreaming: true,
    supportsFunctionCalling: true,
    supportsVision: false,
    reasoning: "good",
    speed: "fast",
    cost: "low",
  },
  "claude-3-opus": {
    maxTokens: 200000,
    supportsStreaming: true,
    supportsFunctionCalling: true,
    supportsVision: true,
    reasoning: "advanced",
    speed: "slow",
    cost: "high",
  },
  "claude-3-sonnet": {
    maxTokens: 200000,
    supportsStreaming: true,
    supportsFunctionCalling: true,
    supportsVision: true,
    reasoning: "good",
    speed: "medium",
    cost: "medium",
  },
  "claude-3-haiku": {
    maxTokens: 200000,
    supportsStreaming: true,
    supportsFunctionCalling: false,
    supportsVision: true,
    reasoning: "good",
    speed: "fast",
    cost: "low",
  },
};

/**
 * Utility function to validate provider configuration
 */
export function validateProviderConfig(config: LLMProviderConfig): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  if (!config.name || config.name.trim().length === 0) {
    errors.push("Provider name is required");
  }

  if (!config.type) {
    errors.push("Provider type is required");
  }

  if (!config.endpoint || config.endpoint.trim().length === 0) {
    errors.push("Provider endpoint is required");
  } else {
    try {
      new URL(config.endpoint);
    } catch {
      errors.push("Provider endpoint must be a valid URL");
    }
  }

  // API key validation for providers that require it
  if (["openai", "anthropic"].includes(config.type) && !config.apiKey) {
    errors.push(`API key is required for ${config.type} provider`);
  }

  // Model validation
  if (config.models && config.models.length === 0) {
    errors.push("At least one model must be specified");
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Default provider configurations
 */
export const DefaultProviderConfigs = {
  openai: (apiKey: string): LLMProviderConfig => ({
    name: "openai-default",
    type: "openai",
    endpoint: "https://api.openai.com/v1",
    apiKey,
    models: ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
    priority: 1,
  }),

  anthropic: (apiKey: string): LLMProviderConfig => ({
    name: "anthropic-default",
    type: "anthropic",
    endpoint: "https://api.anthropic.com/v1",
    apiKey,
    models: ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
    priority: 2,
  }),

  ollama: (baseUrl: string = "http://localhost:11434"): LLMProviderConfig => ({
    name: "ollama-local",
    type: "ollama",
    endpoint: baseUrl,
    models: ["llama2", "mistral", "codellama"],
    priority: 3,
  }),
};

/**
 * Provider health check utility
 */
export async function checkProviderHealth(
  config: LLMProviderConfig,
  logger?: Logger,
): Promise<boolean> {
  try {
    const provider = new LLMProvider(config, logger);
    await provider.initialize();
    const isHealthy = await provider.healthCheck();
    await provider.destroy();
    return isHealthy;
  } catch {
    return false;
  }
}

/**
 * Bulk provider health check
 */
export async function checkAllProviderHealth(
  configs: LLMProviderConfig[],
  logger?: Logger,
): Promise<Record<string, boolean>> {
  const results: Record<string, boolean> = {};

  const checks = configs.map(async (config) => {
    results[config.name] = await checkProviderHealth(config, logger);
  });

  await Promise.allSettled(checks);
  return results;
}
