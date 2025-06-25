/**
 * Circuit Breaker TypeScript SDK
 *
 * A simple, clean TypeScript client for the Circuit Breaker workflow engine.
 * Mirrors the Rust SDK approach with minimal abstractions and direct API access.
 */

// ============================================================================
// Core Client
// ============================================================================

export { Client, ClientBuilder } from "./client.js";
export type { ClientBuilderConfig } from "./client.js";
export type { ClientConfig } from "./types.js";

// ============================================================================
// API Clients
// ============================================================================

export {
  WorkflowClient,
  WorkflowBuilder,
  createWorkflow,
} from "./workflows.js";
export { AgentClient, AgentBuilder, createAgent } from "./agents.js";
export {
  FunctionClient,
  FunctionBuilder,
  createFunction,
} from "./functions.js";
export {
  ResourceClient,
  ResourceBuilder,
  createResource,
} from "./resources.js";
export {
  RuleClient,
  RuleBuilder,
  LegacyRuleBuilder,
  ClientRuleEngine,
  createRule,
  createLegacyRule,
  createRuleEngine,
  evaluateRule,
  CommonRules,
} from "./rules.js";
export {
  LLMClient,
  ChatBuilder,
  Conversation,
  createChat,
  createConversation,
  quickChat,
  COMMON_MODELS,
} from "./llm.js";
export {
  AnalyticsClient,
  BudgetStatusBuilder,
  CostAnalyticsBuilder,
  SetBudgetBuilder,
  costAnalytics,
  budgetStatus,
  setBudget,
  getUserBudgetStatus,
  getProjectBudgetStatus,
  getUserMonthlyCostAnalytics,
  setUserMonthlyBudget,
} from "./analytics.js";
export {
  MCPClient,
  MCPServersBuilder,
  CreateMCPServerBuilder,
  UpdateMCPServerBuilder,
  ConfigureOAuthBuilder,
  ConfigureJWTBuilder,
  MCPServerType,
  MCPServerStatus,
  createMCPServer,
  listMCPServers,
  getMCPServerHealth,
  getCustomMCPServer,
} from "./mcp.js";
export {
  SubscriptionClient,
  SubscriptionManager,
  SubscriptionId,
  SubscriptionError,
  SubscriptionMetrics,
  WebSocketConnection,
  ResourceUpdateSubscriptionBuilder,
  WorkflowEventSubscriptionBuilder,
  AgentExecutionSubscriptionBuilder,
  LLMStreamSubscriptionBuilder,
  CostUpdateSubscriptionBuilder,
  MCPServerStatusSubscriptionBuilder,
  MCPSessionEventSubscriptionBuilder,
  subscribeResourceUpdates,
  subscribeWorkflowEvents,
  subscribeLLMStream,
  subscribeCostUpdates,
} from "./subscriptions.js";
export {
  NATSClient,
  CreateWorkflowInstanceBuilder,
  ExecuteActivityWithNATSBuilder,
  createWorkflowInstance,
  executeActivityWithNats,
  getNatsResource,
  getResourcesInState,
} from "./nats.js";

// ============================================================================
// Types
// ============================================================================

export type {
  // Common types
  PingResponse,
  ServerInfo,
  PaginationOptions,
  PaginatedResult,
  Result,
  ExecutionStatus,

  // Workflow types
  Workflow,
  WorkflowDefinition,
  WorkflowState,
  WorkflowTransition,
  WorkflowAction,
  WorkflowCondition,
  WorkflowExecution,
  WorkflowCreateInput,

  // Agent types
  Agent,
  AgentType,
  AgentConfig,
  MemoryConfig,
  Tool,
  AgentCreateInput,

  // Function types
  Function,
  FunctionConfig,
  DockerConfig,
  ContainerMount,
  ResourceLimits,
  FunctionExecution,
  FunctionCreateInput,

  // Resource types
  Resource,
  ResourceCreateInput,
  ResourceUpdateInput,

  // LLM types
  ChatMessage,
  ChatCompletionRequest,
  ChatCompletionResponse,
  Choice,
  Usage,
} from "./types.js";

// ============================================================================
// MCP Types (from mcp.js)
// ============================================================================

export type {
  MCPServer,
  MCPServerConnection,
  MCPOAuthProvider,
  MCPServerCapabilities,
  MCPServerHealth,
  MCPOAuthInitiation,
  MCPSession,
  MCPOAuthConfig,
  MCPJWTConfig,
  CreateMCPServerInput,
  UpdateMCPServerInput,
  ConfigureOAuthInput,
  ConfigureJWTInput,
  PaginationInput,
} from "./mcp.js";

// ============================================================================
// Subscription Types (from subscriptions.js)
// ============================================================================

export type {
  GraphQLSubscription,
  GraphQLWSMessage,
  SubscriptionHandler,
  SubscriptionConfig,
  ResourceUpdateEvent,
  WorkflowEvent,
  AgentExecutionEvent,
  LLMStreamChunk,
  CostUpdateEvent,
  MCPServerStatusUpdate,
  MCPSessionEvent,
} from "./subscriptions.js";

// ============================================================================
// NATS Types (from nats.js)
// ============================================================================

export type {
  NATSResource,
  HistoryEvent,
  CreateWorkflowInstanceInput,
  ExecuteActivityWithNATSInput,
} from "./nats.js";

// ============================================================================
// Rule Types (from rules.js)
// ============================================================================

export type {
  Rule,
  RuleCondition,
  RuleEvaluationResult,
  RuleCreateInput,
  LegacyRule,
} from "./rules.js";

// ============================================================================
// Error Classes
// ============================================================================

export {
  CircuitBreakerError,
  NetworkError,
  ValidationError,
  NotFoundError,
} from "./types.js";

// ============================================================================
// Re-export history event from resources
// ============================================================================

// export type { HistoryEvent } from "./resources.js"; // Not currently exported

// ============================================================================
// Re-export LLM specific types
// ============================================================================

export type {
  LLMModel,
  ChatCompletionChunk,
  ChoiceDelta,
  MessageDelta,
  TokenCount,
  ProviderHealth,
} from "./llm.js";

// ============================================================================
// Constants
// ============================================================================

export const SDK_VERSION = "0.1.0";
export const DEFAULT_BASE_URL = "http://localhost:3000";
export const DEFAULT_TIMEOUT = 30000;

// ============================================================================
// Main SDK Class (Convenience)
// ============================================================================

// ============================================================================
// Import required classes for SDK class
// ============================================================================

import { Client } from "./client.js";
import type { ClientConfig, PingResponse, ServerInfo } from "./types.js";
import type { WorkflowClient } from "./workflows.js";
import type { AgentClient } from "./agents.js";
import type { FunctionClient } from "./functions.js";
import type { ResourceClient } from "./resources.js";
import type { RuleClient } from "./rules.js";
import type { LLMClient } from "./llm.js";
import type { AnalyticsClient } from "./analytics.js";
import type { MCPClient } from "./mcp.js";
import type { SubscriptionClient } from "./subscriptions.js";
import type { NATSClient } from "./nats.js";

/**
 * Main SDK class that provides access to all API clients
 */
export class CircuitBreakerSDK {
  private client: Client;

  constructor(config: ClientConfig) {
    this.client = Client.builder()
      .baseUrl(config.baseUrl)
      .timeout(config.timeout || DEFAULT_TIMEOUT)
      .build();

    if (config.apiKey) {
      // Rebuild with API key
      this.client = Client.builder()
        .baseUrl(config.baseUrl)
        .apiKey(config.apiKey)
        .timeout(config.timeout || DEFAULT_TIMEOUT)
        .build();
    }

    // Add custom headers if provided
    if (config.headers) {
      let builder = Client.builder()
        .baseUrl(config.baseUrl)
        .timeout(config.timeout || DEFAULT_TIMEOUT);

      if (config.apiKey) {
        builder = builder.apiKey(config.apiKey);
      }

      Object.entries(config.headers).forEach(([key, value]) => {
        const headers = { [key]: value };
        builder = builder.headers(headers);
      });

      this.client = builder.build();
    }
  }

  /**
   * Test connection to the server
   */
  async ping(): Promise<PingResponse> {
    return this.client.ping();
  }

  /**
   * Get server information
   */
  async info(): Promise<ServerInfo> {
    return this.client.info();
  }

  /**
   * Access workflows API
   */
  workflows(): WorkflowClient {
    return this.client.workflows();
  }

  /**
   * Access agents API
   */
  agents(): AgentClient {
    return this.client.agents();
  }

  /**
   * Access functions API
   */
  functions(): FunctionClient {
    return this.client.functions();
  }

  /**
   * Access resources API
   */
  resources(): ResourceClient {
    return this.client.resources();
  }

  /**
   * Access rules API
   */
  rules(): RuleClient {
    return this.client.rules();
  }

  /**
   * Access LLM API
   */
  llm(): LLMClient {
    return this.client.llm();
  }

  /**
   * Access analytics and budget management API
   */
  analytics(): AnalyticsClient {
    return this.client.analytics();
  }

  /**
   * Access MCP (Model Context Protocol) API
   */
  mcp(): MCPClient {
    return this.client.mcp();
  }

  /**
   * Access real-time subscription API
   */
  subscriptions(): SubscriptionClient {
    return this.client.subscriptions();
  }

  /**
   * Access NATS-enhanced operations API
   */
  nats(): NATSClient {
    return this.client.nats();
  }

  /**
   * Get the underlying client
   */
  getClient(): Client {
    return this.client;
  }
}

// ============================================================================
// Convenience Factory Functions
// ============================================================================

/**
 * Create a new SDK instance
 */
export function createSDK(config: ClientConfig): CircuitBreakerSDK {
  return new CircuitBreakerSDK(config);
}

/**
 * Create a simple SDK instance with just a base URL
 */
export function createSimpleSDK(
  baseUrl: string,
  apiKey?: string,
): CircuitBreakerSDK {
  const config: ClientConfig = {
    baseUrl,
    ...(apiKey && { apiKey }),
  };
  return new CircuitBreakerSDK(config);
}

// ============================================================================
// Default Export
// ============================================================================

export default CircuitBreakerSDK;
