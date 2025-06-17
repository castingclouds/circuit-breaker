/**
 * LLM Builder for Circuit Breaker SDK
 *
 * Provides a fluent API for configuring LLM routers and providers with:
 * - Simple provider configuration
 * - Routing strategy selection
 * - Health monitoring setup
 * - Cost optimization configuration
 * - Template-based configurations
 *
 * @example
 * ```typescript
 * const router = new LLMBuilder()
 *   .addOpenAI({
 *     apiKey: process.env.OPENAI_API_KEY,
 *     models: ['gpt-4', 'gpt-3.5-turbo']
 *   })
 *   .addAnthropic({
 *     apiKey: process.env.ANTHROPIC_API_KEY,
 *     models: ['claude-3-sonnet']
 *   })
 *   .setRoutingStrategy('cost-optimized')
 *   .enableHealthChecks({ interval: 60 })
 *   .enableCostTracking()
 *   .build();
 * ```
 */

import {
  LLMConfig,
  LLMProviderConfig,
  LLMProviderType,
  LoadBalancingConfig,
  FailoverConfig,
  CircuitBreakerConfig,
  RateLimitConfig,
  UsageTrackingConfig,
} from '../core/types.js';
import { Logger, createComponentLogger } from '../utils/logger.js';
import { LLMRouter, LLMRouterConfig, RoutingStrategy } from './router.js';
import { validateProviderConfig, DefaultProviderConfigs } from './providers.js';
import { StreamConfig } from './streaming.js';

export interface LLMBuilderConfig {
  /** Default provider name */
  defaultProvider?: string;

  /** Global timeout for all requests */
  timeout?: number;

  /** Maximum retries for failed requests */
  maxRetries?: number;

  /** Enable debug logging */
  debug?: boolean;

  /** Custom logger instance */
  logger?: Logger;
}

export interface ProviderBuilderConfig {
  name?: string;
  endpoint?: string;
  apiKey?: string;
  models?: string[];
  priority?: number;
  timeout?: number;
  maxRetries?: number;
  rateLimit?: {
    requestsPerMinute?: number;
    tokensPerMinute?: number;
    concurrent?: number;
  };
}

export interface HealthCheckBuilderConfig {
  enabled?: boolean;
  interval?: number; // seconds
  timeout?: number; // milliseconds
  retries?: number;
}

export interface CostTrackingBuilderConfig {
  enabled?: boolean;
  budgetLimit?: number; // USD
  alertThreshold?: number; // percentage of budget
  trackPerUser?: boolean;
  trackPerProject?: boolean;
}

export interface LoadBalancingBuilderConfig {
  strategy?: 'round-robin' | 'weighted' | 'least-connections' | 'response-time';
  weights?: Record<string, number>;
  stickySession?: boolean;
}

export interface FailoverBuilderConfig {
  enabled?: boolean;
  maxRetries?: number;
  backoffStrategy?: 'exponential' | 'linear' | 'fixed';
  baseDelay?: number; // milliseconds
}

/**
 * Result of building an LLM configuration
 */
export interface LLMBuilderResult {
  router: LLMRouter;
  config: LLMRouterConfig;
  providers: LLMProviderConfig[];
  validation: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  };
}

/**
 * Builder for LLM Router configuration
 */
export class LLMBuilder {
  private config: LLMBuilderConfig;
  private providers: LLMProviderConfig[] = [];
  private routingStrategy: RoutingStrategy = 'cost-optimized';
  private healthCheckConfig?: HealthCheckBuilderConfig;
  private costTrackingConfig?: CostTrackingBuilderConfig;
  private loadBalancingConfig?: LoadBalancingBuilderConfig;
  private failoverConfig?: FailoverBuilderConfig;
  private streamConfig?: StreamConfig;
  private logger: Logger;

  constructor(config: LLMBuilderConfig = {}) {
    this.config = {
      timeout: 30000,
      maxRetries: 3,
      debug: false,
      ...config,
    };

    this.logger = config.logger || createComponentLogger('LLMBuilder');
  }

  /**
   * Add OpenAI provider
   */
  addOpenAI(config: ProviderBuilderConfig): this {
    const providerConfig: LLMProviderConfig = {
      name: config.name || 'openai-provider',
      type: 'openai',
      endpoint: config.endpoint || 'https://api.openai.com/v1',
      apiKey: config.apiKey || process.env.OPENAI_API_KEY,
      models: config.models || ['gpt-4', 'gpt-3.5-turbo'],
      priority: config.priority || 1,
      timeout: config.timeout,
      maxRetries: config.maxRetries,
      rateLimit: config.rateLimit,
    };

    this.providers.push(providerConfig);
    return this;
  }

  /**
   * Add Anthropic provider
   */
  addAnthropic(config: ProviderBuilderConfig): this {
    const providerConfig: LLMProviderConfig = {
      name: config.name || 'anthropic-provider',
      type: 'anthropic',
      endpoint: config.endpoint || 'https://api.anthropic.com/v1',
      apiKey: config.apiKey || process.env.ANTHROPIC_API_KEY,
      models: config.models || ['claude-3-sonnet', 'claude-3-haiku'],
      priority: config.priority || 2,
      timeout: config.timeout,
      maxRetries: config.maxRetries,
      rateLimit: config.rateLimit,
    };

    this.providers.push(providerConfig);
    return this;
  }

  /**
   * Add Ollama provider
   */
  addOllama(config: ProviderBuilderConfig): this {
    const providerConfig: LLMProviderConfig = {
      name: config.name || 'ollama-provider',
      type: 'ollama',
      endpoint: config.endpoint || 'http://localhost:11434',
      models: config.models || ['llama2', 'mistral'],
      priority: config.priority || 3,
      timeout: config.timeout,
      maxRetries: config.maxRetries,
      rateLimit: config.rateLimit,
    };

    this.providers.push(providerConfig);
    return this;
  }

  /**
   * Add custom provider
   */
  addProvider(config: LLMProviderConfig): this {
    this.providers.push(config);
    return this;
  }

  /**
   * Set routing strategy
   */
  setRoutingStrategy(strategy: RoutingStrategy): this {
    this.routingStrategy = strategy;
    return this;
  }

  /**
   * Set default provider
   */
  setDefaultProvider(providerName: string): this {
    this.config.defaultProvider = providerName;
    return this;
  }

  /**
   * Enable health checks
   */
  enableHealthChecks(config: HealthCheckBuilderConfig = {}): this {
    this.healthCheckConfig = {
      enabled: true,
      interval: 60,
      timeout: 10000,
      retries: 3,
      ...config,
    };
    return this;
  }

  /**
   * Disable health checks
   */
  disableHealthChecks(): this {
    this.healthCheckConfig = { enabled: false };
    return this;
  }

  /**
   * Enable cost tracking
   */
  enableCostTracking(config: CostTrackingBuilderConfig = {}): this {
    this.costTrackingConfig = {
      enabled: true,
      trackPerUser: false,
      trackPerProject: false,
      ...config,
    };
    return this;
  }

  /**
   * Configure load balancing
   */
  setLoadBalancing(config: LoadBalancingBuilderConfig): this {
    this.loadBalancingConfig = config;
    return this;
  }

  /**
   * Configure failover
   */
  setFailover(config: FailoverBuilderConfig): this {
    this.failoverConfig = {
      enabled: true,
      maxRetries: 3,
      backoffStrategy: 'exponential',
      baseDelay: 1000,
      ...config,
    };
    return this;
  }

  /**
   * Configure streaming
   */
  setStreaming(config: StreamConfig): this {
    this.streamConfig = config;
    return this;
  }

  /**
   * Set global timeout
   */
  setTimeout(timeout: number): this {
    this.config.timeout = timeout;
    return this;
  }

  /**
   * Set max retries
   */
  setMaxRetries(retries: number): this {
    this.config.maxRetries = retries;
    return this;
  }

  /**
   * Enable debug mode
   */
  enableDebug(): this {
    this.config.debug = true;
    return this;
  }

  /**
   * Validate configuration
   */
  validate(): { isValid: boolean; errors: string[]; warnings: string[] } {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Check if at least one provider is configured
    if (this.providers.length === 0) {
      errors.push('At least one provider must be configured');
    }

    // Validate each provider
    for (const provider of this.providers) {
      const validation = validateProviderConfig(provider);
      if (!validation.valid) {
        errors.push(...validation.errors.map(err => `Provider ${provider.name}: ${err}`));
      }
    }

    // Check default provider exists
    if (this.config.defaultProvider) {
      const defaultExists = this.providers.some(p => p.name === this.config.defaultProvider);
      if (!defaultExists) {
        errors.push(`Default provider '${this.config.defaultProvider}' not found`);
      }
    }

    // Check for duplicate provider names
    const providerNames = this.providers.map(p => p.name);
    const duplicates = providerNames.filter((name, index) => providerNames.indexOf(name) !== index);
    if (duplicates.length > 0) {
      errors.push(`Duplicate provider names: ${duplicates.join(', ')}`);
    }

    // Warnings
    if (this.providers.length === 1) {
      warnings.push('Only one provider configured - no failover available');
    }

    if (!this.healthCheckConfig?.enabled) {
      warnings.push('Health checks disabled - automatic failover may not work properly');
    }

    return { isValid: errors.length === 0, errors, warnings };
  }

  /**
   * Build the LLM router
   */
  async build(): Promise<LLMBuilderResult> {
    // Validate configuration
    const validation = this.validate();
    if (!validation.isValid) {
      throw new Error(`Invalid configuration: ${validation.errors.join(', ')}`);
    }

    // Build router configuration
    const routerConfig: LLMRouterConfig = {
      providers: this.providers,
      defaultProvider: this.config.defaultProvider || this.providers[0]?.name,
      routingStrategy: this.routingStrategy,
      timeout: this.config.timeout,
      maxRetries: this.config.maxRetries,
      costTracking: this.costTrackingConfig?.enabled,
      debug: this.config.debug,
      healthCheck: this.healthCheckConfig,
      loadBalancing: this.loadBalancingConfig,
      failover: this.failoverConfig,
    };

    // Create router
    const router = new LLMRouter(routerConfig, this.logger);

    // Wait for initialization
    await router.initializeProviders();

    return {
      router,
      config: routerConfig,
      providers: this.providers,
      validation,
    };
  }

  /**
   * Build configuration only (without creating router)
   */
  buildConfig(): LLMRouterConfig {
    const validation = this.validate();
    if (!validation.isValid) {
      throw new Error(`Invalid configuration: ${validation.errors.join(', ')}`);
    }

    return {
      providers: this.providers,
      defaultProvider: this.config.defaultProvider || this.providers[0]?.name,
      routingStrategy: this.routingStrategy,
      timeout: this.config.timeout,
      maxRetries: this.config.maxRetries,
      costTracking: this.costTrackingConfig?.enabled,
      debug: this.config.debug,
      healthCheck: this.healthCheckConfig,
      loadBalancing: this.loadBalancingConfig,
      failover: this.failoverConfig,
    };
  }

  /**
   * Clone the builder with current configuration
   */
  clone(): LLMBuilder {
    const cloned = new LLMBuilder(this.config);
    cloned.providers = [...this.providers];
    cloned.routingStrategy = this.routingStrategy;
    cloned.healthCheckConfig = this.healthCheckConfig ? { ...this.healthCheckConfig } : undefined;
    cloned.costTrackingConfig = this.costTrackingConfig ? { ...this.costTrackingConfig } : undefined;
    cloned.loadBalancingConfig = this.loadBalancingConfig ? { ...this.loadBalancingConfig } : undefined;
    cloned.failoverConfig = this.failoverConfig ? { ...this.failoverConfig } : undefined;
    cloned.streamConfig = this.streamConfig ? { ...this.streamConfig } : undefined;
    return cloned;
  }

  /**
   * Get current configuration as JSON
   */
  toJSON(): any {
    return {
      config: this.config,
      providers: this.providers,
      routingStrategy: this.routingStrategy,
      healthCheck: this.healthCheckConfig,
      costTracking: this.costTrackingConfig,
      loadBalancing: this.loadBalancingConfig,
      failover: this.failoverConfig,
      streaming: this.streamConfig,
    };
  }

  /**
   * Load configuration from JSON
   */
  static fromJSON(json: any): LLMBuilder {
    const builder = new LLMBuilder(json.config);

    if (json.providers) {
      builder.providers = json.providers;
    }

    if (json.routingStrategy) {
      builder.routingStrategy = json.routingStrategy;
    }

    if (json.healthCheck) {
      builder.healthCheckConfig = json.healthCheck;
    }

    if (json.costTracking) {
      builder.costTrackingConfig = json.costTracking;
    }

    if (json.loadBalancing) {
      builder.loadBalancingConfig = json.loadBalancing;
    }

    if (json.failover) {
      builder.failoverConfig = json.failover;
    }

    if (json.streaming) {
      builder.streamConfig = json.streaming;
    }

    return builder;
  }
}

/**
 * Specialized builders for common use cases
 */
export class MultiProviderBuilder extends LLMBuilder {
  constructor(apiKeys: { openai?: string; anthropic?: string; ollama?: string }) {
    super();

    if (apiKeys.openai) {
      this.addOpenAI({ apiKey: apiKeys.openai });
    }

    if (apiKeys.anthropic) {
      this.addAnthropic({ apiKey: apiKeys.anthropic });
    }

    if (apiKeys.ollama) {
      this.addOllama({ endpoint: apiKeys.ollama });
    }

    this.setRoutingStrategy('load-balanced')
      .enableHealthChecks()
      .enableCostTracking()
      .setFailover({ enabled: true });
  }
}

export class CostOptimizedBuilder extends LLMBuilder {
  constructor(apiKeys: { openai?: string; anthropic?: string }) {
    super();

    // Add providers in cost-optimized order
    if (apiKeys.openai) {
      this.addOpenAI({
        apiKey: apiKeys.openai,
        models: ['gpt-3.5-turbo', 'gpt-4'],
        priority: 1,
      });
    }

    if (apiKeys.anthropic) {
      this.addAnthropic({
        apiKey: apiKeys.anthropic,
        models: ['claude-3-haiku', 'claude-3-sonnet'],
        priority: 2,
      });
    }

    this.setRoutingStrategy('cost-optimized')
      .enableCostTracking({
        budgetLimit: 100,
        alertThreshold: 80,
      });
  }
}

export class PerformanceBuilder extends LLMBuilder {
  constructor(apiKeys: { openai?: string; anthropic?: string }) {
    super();

    // Add providers optimized for performance
    if (apiKeys.openai) {
      this.addOpenAI({
        apiKey: apiKeys.openai,
        models: ['gpt-4-turbo', 'gpt-3.5-turbo'],
        priority: 1,
      });
    }

    if (apiKeys.anthropic) {
      this.addAnthropic({
        apiKey: apiKeys.anthropic,
        models: ['claude-3-haiku', 'claude-3-sonnet'],
        priority: 2,
      });
    }

    this.setRoutingStrategy('performance-first')
      .enableHealthChecks({ interval: 30 })
      .setTimeout(10000);
  }
}

/**
 * Factory functions for creating builders
 */
export function createLLMBuilder(config?: LLMBuilderConfig): LLMBuilder {
  return new LLMBuilder(config);
}

export function createMultiProviderBuilder(apiKeys: {
  openai?: string;
  anthropic?: string;
  ollama?: string;
}): MultiProviderBuilder {
  return new MultiProviderBuilder(apiKeys);
}

export function createCostOptimizedBuilder(apiKeys: {
  openai?: string;
  anthropic?: string;
}): CostOptimizedBuilder {
  return new CostOptimizedBuilder(apiKeys);
}

export function createPerformanceBuilder(apiKeys: {
  openai?: string;
  anthropic?: string;
}): PerformanceBuilder {
  return new PerformanceBuilder(apiKeys);
}

/**
 * Template configurations
 */
export const LLMBuilderTemplates = {
  /**
   * Development template with local and cloud providers
   */
  development: (apiKeys: Record<string, string>): LLMBuilder => {
    return createLLMBuilder()
      .addOllama({ endpoint: 'http://localhost:11434' })
      .addOpenAI({ apiKey: apiKeys.openai })
      .setRoutingStrategy('failover-chain')
      .enableHealthChecks()
      .enableDebug();
  },

  /**
   * Production template with redundancy and monitoring
   */
  production: (apiKeys: Record<string, string>): LLMBuilder => {
    return createLLMBuilder()
      .addOpenAI({ apiKey: apiKeys.openai, priority: 1 })
      .addAnthropic({ apiKey: apiKeys.anthropic, priority: 2 })
      .setRoutingStrategy('load-balanced')
      .enableHealthChecks({ interval: 30 })
      .enableCostTracking({ budgetLimit: 1000 })
      .setFailover({ enabled: true, maxRetries: 3 });
  },

  /**
   * High-volume template with cost optimization
   */
  highVolume: (apiKeys: Record<string, string>): LLMBuilder => {
    return createLLMBuilder()
      .addOpenAI({
        apiKey: apiKeys.openai,
        models: ['gpt-3.5-turbo'],
        rateLimit: { requestsPerMinute: 3000 },
      })
      .addAnthropic({
        apiKey: apiKeys.anthropic,
        models: ['claude-3-haiku'],
        rateLimit: { requestsPerMinute: 1000 },
      })
      .setRoutingStrategy('cost-optimized')
      .enableCostTracking({ budgetLimit: 5000 })
      .setLoadBalancing({ strategy: 'weighted' });
  },

  /**
   * Testing template with mock providers
   */
  testing: (): LLMBuilder => {
    return createLLMBuilder()
      .addProvider({
        name: 'mock-provider',
        type: 'custom',
        endpoint: 'http://localhost:3001/mock',
        models: ['mock-model'],
      })
      .setRoutingStrategy('model-specific')
      .disableHealthChecks()
      .enableDebug();
  },
};
