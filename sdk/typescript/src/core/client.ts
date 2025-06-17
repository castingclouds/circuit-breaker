/**
 * Main SDK Client for Circuit Breaker TypeScript SDK
 *
 * This file contains the primary CircuitBreakerSDK class that serves as the
 * entry point for all SDK functionality including workflows, resources,
 * rules engine, functions, LLM router, and AI agents.
 */

import { SDKConfig } from "./types.js";
import { CircuitBreakerError, ErrorHandler } from "./errors.js";
import { GraphQLClient } from "../utils/graphql.js";
import { WorkflowManager } from "../workflow/manager.js";
import { Logger } from "../utils/logger.js";

// ============================================================================
// Types
// ============================================================================

export interface SDKInitializationResult {
  success: boolean;
  errors: string[];
  warnings: string[];
  components: {
    graphql: boolean;
    workflows: boolean;
    resources: boolean;
    rules: boolean;
    functions?: boolean;
    llm?: boolean;
    agents?: boolean;
  };
}

export interface SDKHealthStatus {
  healthy: boolean;
  components: Record<
    string,
    {
      status: "healthy" | "degraded" | "unhealthy";
      message?: string;
      lastCheck: Date;
    }
  >;
  version: string;
  uptime: number;
}

// ============================================================================
// Main SDK Client
// ============================================================================

export class CircuitBreakerSDK {
  private readonly config: SDKConfig;
  private readonly logger: Logger;
  private readonly graphqlClient: GraphQLClient;
  private readonly startTime: Date;

  // Core managers
  private readonly workflowManager: WorkflowManager;

  // Optional managers (loaded on demand)
  private _resourceManager?: any; // Will be loaded from resources module
  private _rulesEngine?: any; // Will be loaded from rules module
  private _functionManager?: any; // Will be loaded from functions module
  private _llmRouter?: any; // Will be loaded from llm module
  private _agentBuilder?: any; // Will be loaded from agents module

  constructor(config: SDKConfig) {
    this.config = this.validateAndNormalizeConfig(config);
    this.startTime = new Date();

    // Initialize logger
    this.logger = new Logger(this.config.logging);

    // Initialize GraphQL client
    this.graphqlClient = new GraphQLClient({
      endpoint: this.config.graphqlEndpoint,
      headers: this.config.headers,
      timeout: this.config.timeout,
      debug: this.config.debug,
      logger: this.logger.log.bind(this.logger),
    });

    // Initialize core managers
    this.workflowManager = new WorkflowManager(this.graphqlClient, this.logger);

    this.logger.info("CircuitBreakerSDK initialized", {
      endpoint: this.config.graphqlEndpoint,
      version: this.getVersion(),
    });
  }

  // ============================================================================
  // Static Factory Methods
  // ============================================================================

  /**
   * Create SDK instance with automatic configuration
   */
  static async create(config: SDKConfig): Promise<CircuitBreakerSDK> {
    const sdk = new CircuitBreakerSDK(config);
    const initResult = await sdk.initialize();

    if (!initResult.success) {
      throw new CircuitBreakerError(
        `SDK initialization failed: ${initResult.errors.join(", ")}`,
        "SDK_INITIALIZATION_ERROR",
        { initResult },
      );
    }

    return sdk;
  }

  /**
   * Create SDK instance with minimal configuration
   */
  static createSimple(graphqlEndpoint: string): CircuitBreakerSDK {
    return new CircuitBreakerSDK({ graphqlEndpoint });
  }

  // ============================================================================
  // Core API Access
  // ============================================================================

  /**
   * Access workflow management functionality
   */
  get workflows(): WorkflowManager {
    return this.workflowManager;
  }

  /**
   * Access resource management functionality (lazy loaded)
   */
  get resources(): any {
    if (!this._resourceManager) {
      try {
        // Dynamic import to avoid loading resources module unless needed
        const { ResourceManager } = require("../resources/manager.js");
        this._resourceManager = new ResourceManager(
          this.graphqlClient,
          this.logger,
        );
      } catch (error) {
        throw new CircuitBreakerError(
          "Resource manager not available. Install @circuit-breaker/resources package.",
          "RESOURCES_NOT_AVAILABLE",
          { error },
        );
      }
    }
    return this._resourceManager;
  }

  /**
   * Access rules engine functionality (lazy loaded)
   */
  get rules(): any {
    if (!this._rulesEngine) {
      try {
        // Dynamic import to avoid loading rules module unless needed
        const { RulesEngine } = require("../rules/engine.js");
        this._rulesEngine = new RulesEngine(
          this.config.rulesConfig,
          this.logger,
        );
      } catch (error) {
        throw new CircuitBreakerError(
          "Rules engine not available. Install @circuit-breaker/rules package.",
          "RULES_NOT_AVAILABLE",
          { error },
        );
      }
    }
    return this._rulesEngine;
  }

  /**
   * Access function system functionality (lazy loaded)
   */
  get functions(): any {
    if (!this._functionManager) {
      try {
        // Dynamic import to avoid loading functions module unless needed
        const { FunctionManager } = require("../functions/manager.js");
        this._functionManager = new FunctionManager(
          this.config.functionConfig,
          this.graphqlClient,
          this.logger,
        );
      } catch (error) {
        throw new CircuitBreakerError(
          "Function system not available. Install @circuit-breaker/functions package.",
          "FUNCTIONS_NOT_AVAILABLE",
          { error },
        );
      }
    }
    return this._functionManager;
  }

  /**
   * Access LLM router functionality (lazy loaded)
   */
  get llm(): any {
    if (!this._llmRouter) {
      if (!this.config.llmConfig) {
        throw new CircuitBreakerError(
          "LLM configuration not provided. Please provide llmConfig in SDK configuration.",
          "LLM_CONFIG_MISSING",
          {},
        );
      }

      try {
        const { LLMRouter } = require("../llm/router.js");
        this._llmRouter = new LLMRouter(this.config.llmConfig, this.logger);
      } catch (error) {
        throw new CircuitBreakerError(
          "Failed to initialize LLM router.",
          "LLM_INIT_FAILED",
          { error },
        );
      }
    }
    return this._llmRouter;
  }

  /**
   * Create a new agent builder (lazy loaded)
   */
  agentBuilder(name: string): any {
    try {
      // Dynamic import to avoid loading agents module unless needed
      const { AgentBuilder } = require("../agents/builder.js");
      return new AgentBuilder(name, this);
    } catch (error) {
      throw new CircuitBreakerError(
        "Agent system not available. Install @circuit-breaker/agents package.",
        "AGENTS_NOT_AVAILABLE",
        { error },
      );
    }
  }

  // ============================================================================
  // SDK Management
  // ============================================================================

  /**
   * Initialize SDK and check component health
   */
  async initialize(): Promise<SDKInitializationResult> {
    const result: SDKInitializationResult = {
      success: true,
      errors: [],
      warnings: [],
      components: {
        graphql: false,
        workflows: false,
        resources: false,
        rules: false,
      },
    };

    try {
      // Test GraphQL connection
      const isHealthy = await this.graphqlClient.healthCheck();
      result.components.graphql = isHealthy;

      if (!isHealthy) {
        result.errors.push("GraphQL endpoint is not reachable");
        result.success = false;
      }

      // Initialize workflow manager
      try {
        await this.workflowManager.initialize();
        result.components.workflows = true;
      } catch (error) {
        result.errors.push(`Workflow manager initialization failed: ${error}`);
        result.success = false;
      }

      // Check optional components
      try {
        this.resources;
        result.components.resources = true;
      } catch {
        result.warnings.push("Resource manager not available");
      }

      try {
        this.rules;
        result.components.rules = true;
      } catch {
        result.warnings.push("Rules engine not available");
      }

      // Check optional components
      try {
        this.functions;
        result.components.functions = true;
      } catch {
        result.warnings.push("Function system not available");
      }

      try {
        this.llm;
        result.components.llm = true;
      } catch {
        result.warnings.push("LLM router not available");
      }

      try {
        this.agentBuilder("test");
        result.components.agents = true;
      } catch {
        result.warnings.push("Agent system not available");
      }
    } catch (error) {
      result.errors.push(`SDK initialization error: ${error}`);
      result.success = false;
    }

    this.logger.info("SDK initialization completed", result);
    return result;
  }

  /**
   * Get SDK health status
   */
  async getHealth(): Promise<SDKHealthStatus> {
    const health: SDKHealthStatus = {
      healthy: true,
      components: {},
      version: this.getVersion(),
      uptime: Date.now() - this.startTime.getTime(),
    };

    // Check GraphQL health
    try {
      const isHealthy = await this.graphqlClient.healthCheck();
      health.components.graphql = {
        status: isHealthy ? "healthy" : "unhealthy",
        message: isHealthy ? undefined : "Endpoint not reachable",
        lastCheck: new Date(),
      };
      if (!isHealthy) health.healthy = false;
    } catch (error) {
      health.components.graphql = {
        status: "unhealthy",
        message: `Health check failed: ${error}`,
        lastCheck: new Date(),
      };
      health.healthy = false;
    }

    // Check workflow manager health
    try {
      const workflowHealth = await this.workflowManager.getHealth();
      health.components.workflows = {
        status: workflowHealth.healthy ? "healthy" : "degraded",
        lastCheck: new Date(),
      };
    } catch (error) {
      health.components.workflows = {
        status: "unhealthy",
        message: `Workflow manager error: ${error}`,
        lastCheck: new Date(),
      };
      health.healthy = false;
    }

    // Check resource manager health
    try {
      if (this._resourceManager) {
        const resourceHealth = await this._resourceManager.getHealth();
        health.components.resources = {
          status: resourceHealth.healthy ? "healthy" : "degraded",
          lastCheck: new Date(),
        };
      } else {
        health.components.resources = {
          status: "healthy",
          message: "Not loaded",
          lastCheck: new Date(),
        };
      }
    } catch (error) {
      health.components.resources = {
        status: "unhealthy",
        message: `Resource manager error: ${error}`,
        lastCheck: new Date(),
      };
      health.healthy = false;
    }

    return health;
  }

  /**
   * Get SDK configuration (sanitized)
   */
  getConfig(): Partial<SDKConfig> {
    return {
      graphqlEndpoint: this.config.graphqlEndpoint,
      timeout: this.config.timeout,
      debug: this.config.debug,
      logging: this.config.logging,
      // Note: sensitive data like API keys are not included
    };
  }

  /**
   * Get SDK version
   */
  getVersion(): string {
    try {
      // In a real implementation, this would read from package.json
      return "0.1.0";
    } catch {
      return "unknown";
    }
  }

  /**
   * Get request statistics
   */
  getStats(): {
    requests: {
      total: number;
      successful: number;
      failed: number;
      averageResponseTime: number;
    };
    components: {
      workflows: any;
      resources: any;
      rules: any;
    };
  } {
    const requestLog = this.graphqlClient.getRequestLog();
    const successful = requestLog.filter((r) => r.status === "success").length;
    const failed = requestLog.filter((r) => r.status !== "success").length;
    const averageResponseTime =
      requestLog.length > 0
        ? requestLog.reduce((sum, r) => sum + r.duration, 0) / requestLog.length
        : 0;

    return {
      requests: {
        total: requestLog.length,
        successful,
        failed,
        averageResponseTime,
      },
      components: {
        workflows: this.workflowManager.getStats(),
        resources: this._resourceManager?.getStats() || null,
        rules: this._rulesEngine?.getStats() || null,
      },
    };
  }

  /**
   * Clear all caches and reset state
   */
  async reset(): Promise<void> {
    this.logger.info("Resetting SDK state");

    this.graphqlClient.clearRequestLog();
    await this.workflowManager.reset();

    if (this._resourceManager) {
      await this._resourceManager.reset();
    }

    if (this._rulesEngine) {
      await this._rulesEngine.reset();
    }

    if (this._functionManager) {
      await this._functionManager.reset();
    }

    if (this._llmRouter) {
      await this._llmRouter.reset();
    }
  }

  /**
   * Dispose of SDK resources
   */
  async dispose(): Promise<void> {
    this.logger.info("Disposing SDK resources");

    await this.workflowManager.dispose();

    if (this._resourceManager) {
      await this._resourceManager.dispose();
    }

    if (this._rulesEngine) {
      await this._rulesEngine.dispose();
    }

    if (this._functionManager) {
      await this._functionManager.dispose();
    }

    if (this._llmRouter) {
      await this._llmRouter.dispose();
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private validateAndNormalizeConfig(config: SDKConfig): SDKConfig {
    if (!config.graphqlEndpoint) {
      throw new CircuitBreakerError(
        "GraphQL endpoint is required",
        "INVALID_CONFIG",
        { config },
      );
    }

    // Validate URL format
    try {
      new URL(config.graphqlEndpoint);
    } catch {
      throw new CircuitBreakerError(
        "Invalid GraphQL endpoint URL",
        "INVALID_CONFIG",
        { endpoint: config.graphqlEndpoint },
      );
    }

    // Apply defaults
    return {
      timeout: 30000,
      debug: false,
      headers: {},
      ...config,
      logging: {
        level: "info",
        structured: false,
        ...config.logging,
      },
    };
  }
}

// ============================================================================
// Convenience Exports
// ============================================================================

/**
 * Create SDK instance with minimal configuration
 */
export function createSDK(
  graphqlEndpoint: string,
  config?: Partial<SDKConfig>,
): CircuitBreakerSDK {
  return new CircuitBreakerSDK({
    graphqlEndpoint,
    ...config,
  });
}

/**
 * Create SDK instance with full configuration and automatic initialization
 */
export async function createSDKAsync(
  config: SDKConfig,
): Promise<CircuitBreakerSDK> {
  return CircuitBreakerSDK.create(config);
}

// Export the main class as default
export default CircuitBreakerSDK;
