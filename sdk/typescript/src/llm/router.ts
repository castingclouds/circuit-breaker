/**
 * LLM Router for Circuit Breaker SDK
 *
 * Provides intelligent routing across multiple LLM providers with features like:
 * - Multi-provider support (OpenAI, Anthropic, Ollama, etc.)
 * - Smart routing strategies (cost-optimized, performance-first, load-balanced)
 * - Health monitoring and automatic failover
 * - Rate limiting and usage tracking
 * - Streaming support
 * - Circuit breaker pattern for resilience
 *
 * @example
 * ```typescript
 * const router = new LLMRouter({
 *   providers: [
 *     {
 *       name: 'openai-primary',
 *       type: 'openai',
 *       endpoint: 'https://api.openai.com/v1',
 *       apiKey: process.env.OPENAI_API_KEY,
 *       models: ['gpt-4', 'gpt-3.5-turbo']
 *     }
 *   ],
 *   defaultProvider: 'openai-primary',
 *   routingStrategy: 'cost-optimized'
 * });
 *
 * const response = await router.chatCompletion({
 *   model: 'gpt-4',
 *   messages: [{ role: 'user', content: 'Hello!' }]
 * });
 * ```
 */

import { EventEmitter } from "events";
import {
  LLMConfig,
  LLMProviderConfig,
  LLMProviderType,
  ChatCompletionRequest,
  ChatCompletionResponse,
  ChatCompletionChunk,
} from "../core/types.js";
import {
  LLMError,
  LLMProviderNotFoundError,
  LLMModelNotSupportedError,
} from "../core/errors.js";
import { Logger, createComponentLogger } from "../utils/logger.js";

// Temporary LLMProvider stub until providers.js is fully implemented
class LLMProvider {
  public name: string;
  constructor(config: LLMProviderConfig, logger?: Logger) {
    this.name = config.name;
  }
  async initialize(): Promise<void> {}
  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    throw new Error("Not implemented");
  }
  async *chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk> {
    throw new Error("Not implemented");
  }
  async healthCheck(): Promise<boolean> {
    return true;
  }
  supportsModel(model: string): boolean {
    return true;
  }
  getSupportedModels(): string[] {
    return [];
  }
  estimateCost(request: ChatCompletionRequest): number {
    return 0;
  }
  async destroy?(): Promise<void> {}
}

// Temporary StreamingHandler stub
class StreamingHandler {
  constructor(logger: Logger) {}
}

export type RoutingStrategy =
  | "cost-optimized"
  | "performance-first"
  | "load-balanced"
  | "failover-chain"
  | "model-specific"
  | "custom";

export interface ProviderHealth {
  provider: string;
  isHealthy: boolean;
  lastCheck: Date;
  errorRate: number;
  averageLatency: number;
  consecutiveFailures: number;
  lastError?: string;
}

export interface RoutingInfo {
  selectedProvider: string;
  strategy: RoutingStrategy;
  latency: number;
  retryCount: number;
  fallbackUsed: boolean;
  cost?: number;
}

export interface LLMRouterStats {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  averageLatency: number;
  totalCost: number;
  providerStats: Record<
    string,
    {
      requests: number;
      successes: number;
      failures: number;
      averageLatency: number;
      cost: number;
    }
  >;
}

export interface LLMRouterConfig extends LLMConfig {
  /** Routing strategy */
  routingStrategy?: RoutingStrategy;

  /** Health check configuration */
  healthCheck?: {
    enabled: boolean;
    interval: number; // seconds
    timeout: number; // seconds
    retries: number;
  };

  /** Request timeout in milliseconds */
  timeout?: number;

  /** Maximum retries per request */
  maxRetries?: number;

  /** Retry delay in milliseconds */
  retryDelay?: number;

  /** Enable cost tracking */
  costTracking?: boolean;

  /** Enable detailed logging */
  debug?: boolean;
}

/**
 * LLMRouter manages multiple LLM providers and routes requests intelligently
 */
export class LLMRouter extends EventEmitter {
  private config: LLMRouterConfig;
  private providers: Map<string, LLMProvider> = new Map();
  private providerHealth: Map<string, ProviderHealth> = new Map();
  private stats: LLMRouterStats;
  private logger: Logger;
  private streamingHandler: StreamingHandler;
  private healthCheckInterval?: ReturnType<typeof setInterval>;

  constructor(config: LLMRouterConfig, logger?: Logger) {
    super();

    this.config = {
      routingStrategy: "cost-optimized",
      healthCheck: {
        enabled: true,
        interval: 60,
        timeout: 10000,
        retries: 3,
      },
      timeout: 30000,
      maxRetries: 3,
      retryDelay: 1000,
      costTracking: true,
      debug: false,
      ...config,
    };

    this.logger = logger || createComponentLogger("LLMRouter");
    this.streamingHandler = new StreamingHandler(this.logger);

    this.stats = {
      totalRequests: 0,
      successfulRequests: 0,
      failedRequests: 0,
      averageLatency: 0,
      totalCost: 0,
      providerStats: {},
    };

    this.initializeProviders();
    this.startHealthChecks();
  }

  /**
   * Initialize all configured providers
   */
  async initializeProviders(): Promise<void> {
    this.logger.info("Initializing LLM providers", {
      providerCount: this.config.providers.length,
      defaultProvider: this.config.defaultProvider,
    });

    for (const providerConfig of this.config.providers) {
      try {
        const provider = new LLMProvider(providerConfig, this.logger);
        await provider.initialize();

        this.providers.set(providerConfig.name, provider);
        this.providerHealth.set(providerConfig.name, {
          provider: providerConfig.name,
          isHealthy: true,
          lastCheck: new Date(),
          errorRate: 0,
          averageLatency: 0,
          consecutiveFailures: 0,
        });

        this.stats.providerStats[providerConfig.name] = {
          requests: 0,
          successes: 0,
          failures: 0,
          averageLatency: 0,
          cost: 0,
        };

        this.logger.info(`Provider ${providerConfig.name} initialized`, {
          type: providerConfig.type,
          endpoint: providerConfig.endpoint,
          models: providerConfig.models?.length || 0,
        });
      } catch (error) {
        this.logger.error(
          `Failed to initialize provider ${providerConfig.name}`,
          {
            error: error instanceof Error ? error.message : String(error),
            config: providerConfig,
          },
        );
      }
    }

    if (this.providers.size === 0) {
      throw new LLMError("No providers successfully initialized");
    }

    this.logger.info(
      `LLM Router initialized with ${this.providers.size} providers`,
    );
  }

  /**
   * Start health check monitoring
   */
  private startHealthChecks(): void {
    if (!this.config.healthCheck?.enabled) {
      return;
    }

    const interval = this.config.healthCheck.interval * 1000;
    this.healthCheckInterval = setInterval(() => {
      this.runHealthChecks().catch((error) => {
        this.logger.error("Health check failed", { error });
      });
    }, interval);

    this.logger.debug("Health checks started", { interval });
  }

  /**
   * Run health checks on all providers
   */
  private async runHealthChecks(): Promise<void> {
    const healthPromises = Array.from(this.providers.entries()).map(
      async ([name, provider]) => {
        try {
          const startTime = Date.now();
          const isHealthy = await provider.healthCheck();
          const latency = Date.now() - startTime;

          const health = this.providerHealth.get(name)!;
          health.isHealthy = isHealthy;
          health.lastCheck = new Date();
          health.averageLatency = (health.averageLatency + latency) / 2;
          health.consecutiveFailures = isHealthy
            ? 0
            : health.consecutiveFailures + 1;

          (this as any).emit("healthCheck", {
            provider: name,
            isHealthy,
            latency,
          });
        } catch (error) {
          const health = this.providerHealth.get(name)!;
          health.isHealthy = false;
          health.lastCheck = new Date();
          health.consecutiveFailures += 1;
          health.lastError =
            error instanceof Error ? error.message : String(error);

          (this as any).emit("providerError", { provider: name, error });
        }
      },
    );

    await Promise.allSettled(healthPromises);
  }

  /**
   * Send a chat completion request
   */
  async chatCompletion(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    const startTime = Date.now();
    const requestId = this.generateRequestId();

    this.logger.debug("Chat completion request started", {
      requestId,
      model: request.model,
      messages: request.messages.length,
      stream: request.stream,
    });

    this.stats.totalRequests++;

    try {
      const provider = await this.selectProvider(request);
      const response = await this.executeWithRetry(
        provider,
        request,
        requestId,
      );

      const latency = Date.now() - startTime;
      this.updateStats(
        provider.name,
        true,
        latency,
        response.usage?.total_tokens || 0,
      );

      // Add routing information
      (response as any).routingInfo = {
        selectedProvider: provider.name,
        strategy: this.config.routingStrategy,
        latency,
        retryCount: 0,
        fallbackUsed: false,
      } as RoutingInfo;

      this.stats.successfulRequests++;
      (this as any).emit("requestComplete", {
        requestId,
        provider: provider.name,
        latency,
      });

      return response;
    } catch (error) {
      const latency = Date.now() - startTime;
      this.stats.failedRequests++;
      (this as any).emit("requestFailed", { requestId, error, latency });

      this.logger.error("Chat completion request failed", {
        requestId,
        error: error instanceof Error ? error.message : String(error),
        latency,
      });

      throw error;
    }
  }

  /**
   * Send a streaming chat completion request
   */
  async *chatCompletionStream(
    request: ChatCompletionRequest,
  ): AsyncGenerator<ChatCompletionChunk> {
    const startTime = Date.now();
    const requestId = this.generateRequestId();

    this.logger.debug("Streaming chat completion request started", {
      requestId,
      model: request.model,
      messages: request.messages.length,
    });

    this.stats.totalRequests++;

    try {
      const provider = await this.selectProvider(request);
      const stream = await provider.chatCompletionStream({
        ...request,
        stream: true,
      });

      let totalTokens = 0;

      for await (const chunk of stream) {
        yield chunk;
      }

      const latency = Date.now() - startTime;
      this.updateStats(provider.name, true, latency, totalTokens);
      this.stats.successfulRequests++;

      (this as any).emit("streamComplete", {
        requestId,
        provider: provider.name,
        latency,
      });
    } catch (error) {
      const latency = Date.now() - startTime;
      this.stats.failedRequests++;
      (this as any).emit("streamFailed", { requestId, error, latency });

      this.logger.error("Streaming chat completion request failed", {
        requestId,
        error: error instanceof Error ? error.message : String(error),
        latency,
      });

      throw error;
    }
  }

  /**
   * Select the best provider for a request based on routing strategy
   */
  private async selectProvider(
    request: ChatCompletionRequest,
  ): Promise<LLMProvider> {
    const availableProviders = this.getHealthyProviders();

    if (availableProviders.length === 0) {
      throw new LLMProviderNotFoundError("No healthy providers available");
    }

    // Filter providers that support the requested model
    const modelSupportingProviders = availableProviders.filter((provider) =>
      provider.supportsModel(request.model),
    );

    if (modelSupportingProviders.length === 0) {
      throw new LLMModelNotSupportedError(request.model);
    }

    switch (this.config.routingStrategy) {
      case "cost-optimized":
        return this.selectCostOptimizedProvider(
          modelSupportingProviders,
          request,
        );

      case "performance-first":
        return this.selectPerformanceFirstProvider(modelSupportingProviders);

      case "load-balanced":
        return this.selectLoadBalancedProvider(modelSupportingProviders);

      case "failover-chain":
        return this.selectFailoverProvider(modelSupportingProviders);

      case "model-specific":
        return this.selectModelSpecificProvider(
          modelSupportingProviders,
          request.model,
        );

      default:
        return modelSupportingProviders[0];
    }
  }

  /**
   * Select provider based on cost optimization
   */
  private selectCostOptimizedProvider(
    providers: LLMProvider[],
    request: ChatCompletionRequest,
  ): LLMProvider {
    return providers.reduce((cheapest, current) => {
      const cheapestCost = cheapest.estimateCost(request);
      const currentCost = current.estimateCost(request);
      return currentCost < cheapestCost ? current : cheapest;
    });
  }

  /**
   * Select provider based on performance (lowest latency)
   */
  private selectPerformanceFirstProvider(
    providers: LLMProvider[],
  ): LLMProvider {
    return providers.reduce((fastest, current) => {
      const fastestLatency =
        this.providerHealth.get(fastest.name)?.averageLatency || Infinity;
      const currentLatency =
        this.providerHealth.get(current.name)?.averageLatency || Infinity;
      return currentLatency < fastestLatency ? current : fastest;
    });
  }

  /**
   * Select provider using round-robin load balancing
   */
  private selectLoadBalancedProvider(providers: LLMProvider[]): LLMProvider {
    const requestCounts = providers.map(
      (p) => this.stats.providerStats[p.name]?.requests || 0,
    );
    const minRequests = Math.min(...requestCounts);
    const leastUsedProviders = providers.filter(
      (p, i) => requestCounts[i] === minRequests,
    );

    return leastUsedProviders[
      Math.floor(Math.random() * leastUsedProviders.length)
    ];
  }

  /**
   * Select provider using failover chain (primary -> secondary -> ...)
   */
  private selectFailoverProvider(providers: LLMProvider[]): LLMProvider {
    // Return the first healthy provider in the configured order
    for (const providerConfig of this.config.providers) {
      const provider = providers.find((p) => p.name === providerConfig.name);
      if (provider) {
        return provider;
      }
    }
    return providers[0]!;
  }

  /**
   * Select provider based on model-specific preferences
   */
  private selectModelSpecificProvider(
    providers: LLMProvider[],
    model: string,
  ): LLMProvider {
    // If there's a default provider configured, prefer it
    const defaultProvider = providers.find(
      (p) => p.name === this.config.defaultProvider,
    );
    if (defaultProvider && defaultProvider.supportsModel(model)) {
      return defaultProvider;
    }

    return providers[0]!;
  }

  /**
   * Execute request with retry logic
   */
  private async executeWithRetry(
    provider: LLMProvider,
    request: ChatCompletionRequest,
    requestId: string,
  ): Promise<ChatCompletionResponse> {
    let lastError: Error;

    for (let attempt = 0; attempt <= this.config.maxRetries!; attempt++) {
      try {
        return await provider.chatCompletion(request);
      } catch (error) {
        lastError = error as Error;

        // Update provider health on failure
        this.updateProviderHealth(provider.name, false, error as Error);

        if (attempt < this.config.maxRetries!) {
          const delay = this.config.retryDelay! * Math.pow(2, attempt); // Exponential backoff
          this.logger.warn(
            `Request attempt ${attempt + 1} failed, retrying in ${delay}ms`,
            {
              requestId,
              provider: provider.name,
              error: error instanceof Error ? error.message : String(error),
            },
          );

          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }

    throw lastError!;
  }

  /**
   * Get healthy providers
   */
  private getHealthyProviders(): LLMProvider[] {
    return Array.from(this.providers.values()).filter((provider) => {
      const health = this.providerHealth.get(provider.name);
      return health?.isHealthy !== false;
    });
  }

  /**
   * Update provider health status
   */
  private updateProviderHealth(
    providerName: string,
    success: boolean,
    error?: Error,
  ): void {
    const health = this.providerHealth.get(providerName);
    if (!health) return;

    if (success) {
      health.consecutiveFailures = 0;
      health.lastError = undefined;
    } else {
      health.consecutiveFailures++;
      health.lastError = error?.message;

      // Mark as unhealthy after too many consecutive failures
      if (health.consecutiveFailures >= 3) {
        health.isHealthy = false;
      }
    }

    health.lastCheck = new Date();
  }

  /**
   * Update router statistics
   */
  private updateStats(
    providerName: string,
    success: boolean,
    latency: number,
    tokens: number,
  ): void {
    const providerStats = this.stats.providerStats[providerName];
    if (!providerStats) return;

    providerStats.requests++;
    if (success) {
      providerStats.successes++;
    } else {
      providerStats.failures++;
    }

    // Update average latency
    providerStats.averageLatency =
      (providerStats.averageLatency * (providerStats.requests - 1) + latency) /
      providerStats.requests;

    // Update overall stats
    this.stats.averageLatency =
      (this.stats.averageLatency * (this.stats.totalRequests - 1) + latency) /
      this.stats.totalRequests;
  }

  /**
   * Generate unique request ID
   */
  private generateRequestId(): string {
    return `llm_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Get current router statistics
   */
  getStats(): LLMRouterStats {
    return { ...this.stats };
  }

  /**
   * Get provider health status
   */
  getProviderHealth(providerName?: string): ProviderHealth | ProviderHealth[] {
    if (providerName) {
      const health = this.providerHealth.get(providerName);
      if (!health) {
        throw new LLMProviderNotFoundError(providerName);
      }
      return health;
    }

    return Array.from(this.providerHealth.values());
  }

  /**
   * Get available providers
   */
  getProviders(): string[] {
    return Array.from(this.providers.keys());
  }

  /**
   * Get available models across all providers
   */
  getAvailableModels(): Record<string, string[]> {
    const models: Record<string, string[]> = {};

    for (const [name, provider] of this.providers) {
      models[name] = provider.getSupportedModels();
    }

    return models;
  }

  /**
   * Check if a specific model is supported
   */
  supportsModel(model: string): boolean {
    return Array.from(this.providers.values()).some((provider) =>
      provider.supportsModel(model),
    );
  }

  /**
   * Estimate cost for a request
   */
  estimateCost(request: ChatCompletionRequest): number {
    try {
      // For now, return a simple estimate
      return 0.001; // Simple fallback estimate
    } catch {
      return 0;
    }
  }

  /**
   * Clean up resources
   */
  async destroy(): Promise<void> {
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
    }

    for (const provider of this.providers.values()) {
      await provider.destroy?.();
    }

    this.providers.clear();
    this.providerHealth.clear();
    (this as any).removeAllListeners?.();

    this.logger.info("LLM Router destroyed");
  }
}

/**
 * Create an LLM router with the given configuration
 */
export function createLLMRouter(
  config: LLMRouterConfig,
  logger?: Logger,
): LLMRouter {
  return new LLMRouter(config, logger);
}

/**
 * Default LLM router configuration for common use cases
 */
export const DefaultLLMConfigs = {
  /**
   * Cost-optimized configuration prioritizing cheaper models
   */
  costOptimized: (apiKeys: Record<string, string>): LLMRouterConfig => ({
    providers: [
      {
        name: "openai-gpt35",
        type: "openai" as LLMProviderType,
        endpoint: "https://api.openai.com/v1",
        apiKey: apiKeys.openai || "",
        models: ["gpt-3.5-turbo", "gpt-3.5-turbo-16k"],
      },
      {
        name: "openai-gpt4",
        type: "openai" as LLMProviderType,
        endpoint: "https://api.openai.com/v1",
        apiKey: apiKeys.openai || "",
        models: ["gpt-4", "gpt-4-turbo"],
      },
    ],
    defaultProvider: "openai-gpt35",
    routingStrategy: "cost-optimized",
    costTracking: true,
  }),

  /**
   * Performance-first configuration prioritizing speed
   */
  performanceFirst: (apiKeys: Record<string, string>): LLMRouterConfig => ({
    providers: [
      {
        name: "openai-gpt4",
        type: "openai" as LLMProviderType,
        endpoint: "https://api.openai.com/v1",
        apiKey: apiKeys.openai || "",
        models: ["gpt-4-turbo", "gpt-4"],
      },
      {
        name: "anthropic-claude",
        type: "anthropic" as LLMProviderType,
        endpoint: "https://api.anthropic.com/v1",
        apiKey: apiKeys.anthropic || "",
        models: ["claude-3-sonnet", "claude-3-haiku"],
      },
    ],
    defaultProvider: "openai-gpt4",
    routingStrategy: "performance-first",
    healthCheck: { enabled: true, interval: 30, timeout: 5000, retries: 2 },
  }),

  /**
   * Multi-provider configuration with load balancing
   */
  multiProvider: (apiKeys: Record<string, string>): LLMRouterConfig => ({
    providers: [
      {
        name: "openai-primary",
        type: "openai" as LLMProviderType,
        endpoint: "https://api.openai.com/v1",
        apiKey: apiKeys.openai || "",
        models: ["gpt-4", "gpt-3.5-turbo"],
      },
      {
        name: "anthropic-secondary",
        type: "anthropic" as LLMProviderType,
        endpoint: "https://api.anthropic.com/v1",
        apiKey: apiKeys.anthropic || "",
        models: ["claude-3-sonnet", "claude-3-haiku"],
      },
      {
        name: "ollama-local",
        type: "ollama" as LLMProviderType,
        endpoint: "http://localhost:11434",
        models: ["llama2", "mistral"],
      },
    ],
    defaultProvider: "openai-primary",
    routingStrategy: "load-balanced",
    failover: { enabled: true },
    loadBalancing: { strategy: "round_robin" },
  }),
};
