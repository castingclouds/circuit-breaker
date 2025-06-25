/**
 * Real-time Subscription Infrastructure
 *
 * This module provides comprehensive real-time subscription capabilities for the Circuit Breaker SDK,
 * enabling WebSocket-based GraphQL subscriptions with automatic reconnection, error recovery,
 * and type-safe event handling.
 */

import { Client } from './client.js';

// ============================================================================
// Types
// ============================================================================

/**
 * Unique identifier for subscriptions
 */
export class SubscriptionId {
  private id: string;

  constructor(id?: string) {
    this.id = id || crypto.randomUUID();
  }

  toString(): string {
    return this.id;
  }

  static fromString(id: string): SubscriptionId {
    return new SubscriptionId(id);
  }

  equals(other: SubscriptionId): boolean {
    return this.id === other.id;
  }
}

/**
 * GraphQL subscription definition
 */
export interface GraphQLSubscription {
  /** GraphQL subscription query */
  query: string;
  /** Query variables */
  variables?: Record<string, any>;
  /** Operation name */
  operationName?: string;
}

/**
 * GraphQL WebSocket message protocol
 */
export type GraphQLWSMessage =
  | { type: 'connection_init'; payload?: any }
  | { type: 'connection_ack' }
  | { type: 'start'; id: string; payload: GraphQLSubscription }
  | { type: 'data'; id: string; payload: any }
  | { type: 'error'; id: string; payload: any }
  | { type: 'complete'; id: string }
  | { type: 'stop'; id: string }
  | { type: 'ping'; payload?: any }
  | { type: 'pong'; payload?: any };

/**
 * Subscription handler interface
 */
export interface SubscriptionHandler<T> {
  /** Handle subscription data */
  onData(data: T): Promise<void>;
  /** Handle subscription error */
  onError(error: SubscriptionError): Promise<void>;
  /** Handle subscription completion */
  onComplete(): Promise<void>;
}

/**
 * Subscription configuration
 */
export interface SubscriptionConfig {
  /** Maximum reconnection attempts */
  reconnectAttempts: number;
  /** Delay between reconnection attempts (ms) */
  reconnectDelay: number;
  /** Heartbeat interval (ms) */
  heartbeatInterval: number;
  /** Message timeout (ms) */
  messageTimeout: number;
}

/**
 * Default subscription configuration
 */
export const defaultSubscriptionConfig: SubscriptionConfig = {
  reconnectAttempts: 5,
  reconnectDelay: 1000,
  heartbeatInterval: 30000,
  messageTimeout: 10000,
};

/**
 * Subscription error types
 */
export class SubscriptionError extends Error {
  constructor(
    message: string,
    public readonly subscriptionId?: SubscriptionId,
    public readonly payload?: any,
  ) {
    super(message);
    this.name = 'SubscriptionError';
  }

  static connectionFailed(message: string): SubscriptionError {
    return new SubscriptionError(`WebSocket connection failed: ${message}`);
  }

  static graphqlError(subscriptionId: SubscriptionId, payload: any): SubscriptionError {
    return new SubscriptionError(
      `Subscription ${subscriptionId.toString()} failed with GraphQL error`,
      subscriptionId,
      payload,
    );
  }

  static authenticationFailed(message: string): SubscriptionError {
    return new SubscriptionError(`Authentication failed: ${message}`);
  }

  static timeout(timeout: number): SubscriptionError {
    return new SubscriptionError(`Subscription timeout after ${timeout}ms`);
  }
}

/**
 * Subscription metrics for monitoring
 */
export class SubscriptionMetrics {
  private _activeSubscriptions = 0;
  private _messagesReceived = 0;
  private _connectionFailures = 0;
  private _reconnectionAttempts = 0;

  get activeSubscriptions(): number {
    return this._activeSubscriptions;
  }

  get messagesReceived(): number {
    return this._messagesReceived;
  }

  get connectionFailures(): number {
    return this._connectionFailures;
  }

  get reconnectionAttempts(): number {
    return this._reconnectionAttempts;
  }

  incrementActiveSubscriptions(): void {
    this._activeSubscriptions++;
  }

  decrementActiveSubscriptions(): void {
    this._activeSubscriptions--;
  }

  incrementMessagesReceived(): void {
    this._messagesReceived++;
  }

  incrementConnectionFailures(): void {
    this._connectionFailures++;
  }

  incrementReconnectionAttempts(): void {
    this._reconnectionAttempts++;
  }
}

// ============================================================================
// GraphQL Response Types
// ============================================================================

/**
 * Resource update event
 */
export interface ResourceUpdateEvent {
  id: string;
  workflowId: string;
  state: string;
  data: Record<string, any>;
  metadata: Record<string, any>;
  createdAt: string;
  updatedAt: string;
}

/**
 * Workflow event
 */
export interface WorkflowEvent {
  id: string;
  type: string;
  message: string;
  data: Record<string, any>;
  timestamp: string;
}

/**
 * Agent execution event
 */
export interface AgentExecutionEvent {
  id: string;
  agentId: string;
  status: string;
  output: any;
  timestamp: string;
}

/**
 * LLM stream chunk
 */
export interface LLMStreamChunk {
  id: string;
  content: string;
  finished: boolean;
  timestamp: string;
}

/**
 * Cost update event
 */
export interface CostUpdateEvent {
  userId?: string;
  projectId?: string;
  cost: number;
  timestamp: string;
  details: Record<string, any>;
}

/**
 * MCP server status update
 */
export interface MCPServerStatusUpdate {
  serverId: string;
  status: string;
  message?: string;
  timestamp: string;
}

/**
 * MCP session event
 */
export interface MCPSessionEvent {
  sessionId: string;
  event: string;
  data: Record<string, any>;
  timestamp: string;
}

// ============================================================================
// WebSocket Connection
// ============================================================================

/**
 * WebSocket connection wrapper with auto-reconnection
 */
export class WebSocketConnection {
  private websocket?: WebSocket;
  private config: SubscriptionConfig;
  private connected = false;
  private reconnectAttempts = 0;
  private messageQueue: GraphQLWSMessage[] = [];
  private messageHandlers = new Set<(message: GraphQLWSMessage) => void>();

  constructor(
    private url: string,
    config: Partial<SubscriptionConfig> = {},
  ) {
    this.config = { ...defaultSubscriptionConfig, ...config };
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.websocket = new WebSocket(this.url, 'graphql-ws');

        this.websocket.onopen = () => {
          this.connected = true;
          this.reconnectAttempts = 0;
          this.sendConnectionInit();
          this.flushMessageQueue();
          resolve();
        };

        this.websocket.onmessage = (event) => {
          try {
            const message: GraphQLWSMessage = JSON.parse(event.data);
            this.handleMessage(message);
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
          }
        };

        this.websocket.onclose = () => {
          this.connected = false;
          this.handleDisconnection();
        };

        this.websocket.onerror = (error) => {
          console.error('WebSocket error:', error);
          reject(SubscriptionError.connectionFailed('WebSocket connection error'));
        };

        // Connection timeout
        setTimeout(() => {
          if (!this.connected) {
            reject(SubscriptionError.timeout(this.config.messageTimeout));
          }
        }, this.config.messageTimeout);
      } catch (error) {
        reject(SubscriptionError.connectionFailed(String(error)));
      }
    });
  }

  isConnected(): boolean {
    return this.connected && this.websocket?.readyState === WebSocket.OPEN;
  }

  sendMessage(message: GraphQLWSMessage): void {
    if (this.isConnected()) {
      this.websocket!.send(JSON.stringify(message));
    } else {
      this.messageQueue.push(message);
    }
  }

  onMessage(handler: (message: GraphQLWSMessage) => void): void {
    this.messageHandlers.add(handler);
  }

  offMessage(handler: (message: GraphQLWSMessage) => void): void {
    this.messageHandlers.delete(handler);
  }

  close(): void {
    if (this.websocket) {
      this.websocket.close();
    }
  }

  private sendConnectionInit(): void {
    this.sendMessage({ type: 'connection_init' });
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift()!;
      this.sendMessage(message);
    }
  }

  private handleMessage(message: GraphQLWSMessage): void {
    this.messageHandlers.forEach((handler) => {
      try {
        handler(message);
      } catch (error) {
        console.error('Message handler error:', error);
      }
    });
  }

  private async handleDisconnection(): Promise<void> {
    if (this.reconnectAttempts < this.config.reconnectAttempts) {
      this.reconnectAttempts++;

      // Exponential backoff
      const delay = this.config.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

      setTimeout(() => {
        this.connect().catch((error) => {
          console.error('Reconnection failed:', error);
        });
      }, delay);
    }
  }
}

// ============================================================================
// Active Subscription
// ============================================================================

/**
 * Active subscription state
 */
export class ActiveSubscription<T = any> {
  constructor(
    public readonly id: SubscriptionId,
    public readonly subscription: GraphQLSubscription,
    private handler: SubscriptionHandler<T>,
  ) {}

  async handleData(payload: any): Promise<void> {
    try {
      await this.handler.onData(payload);
    } catch (error) {
      console.error('Subscription data handler error:', error);
    }
  }

  async handleError(error: SubscriptionError): Promise<void> {
    try {
      await this.handler.onError(error);
    } catch (handlerError) {
      console.error('Subscription error handler error:', handlerError);
    }
  }

  async handleComplete(): Promise<void> {
    try {
      await this.handler.onComplete();
    } catch (error) {
      console.error('Subscription complete handler error:', error);
    }
  }
}

// ============================================================================
// Subscription Manager
// ============================================================================

/**
 * Core subscription manager handling WebSocket connections and message routing
 */
export class SubscriptionManager {
  private websocket?: WebSocketConnection;
  private subscriptions = new Map<string, ActiveSubscription>();
  private metrics = new SubscriptionMetrics();

  constructor(
    private client: Client,
    private config: Partial<SubscriptionConfig> = {},
  ) {}

  async subscribe<T>(
    subscription: GraphQLSubscription,
    handler: SubscriptionHandler<T>,
  ): Promise<SubscriptionId> {
    // Ensure WebSocket connection is established
    await this.ensureConnection();

    const subscriptionId = new SubscriptionId();
    const activeSubscription = new ActiveSubscription(subscriptionId, subscription, handler);

    // Store the subscription
    this.subscriptions.set(subscriptionId.toString(), activeSubscription);

    // Send subscription start message
    this.sendSubscriptionStart(subscriptionId, subscription);

    this.metrics.incrementActiveSubscriptions();

    return subscriptionId;
  }

  async unsubscribe(subscriptionId: SubscriptionId): Promise<void> {
    // Send unsubscribe message
    this.sendSubscriptionStop(subscriptionId);

    // Remove from active subscriptions
    this.subscriptions.delete(subscriptionId.toString());

    this.metrics.decrementActiveSubscriptions();
  }

  getMetrics(): SubscriptionMetrics {
    return this.metrics;
  }

  async close(): Promise<void> {
    if (this.websocket) {
      this.websocket.close();
    }
    this.subscriptions.clear();
  }

  private async ensureConnection(): Promise<void> {
    if (!this.websocket || !this.websocket.isConnected()) {
      const wsUrl = this.buildWebSocketUrl();
      this.websocket = new WebSocketConnection(wsUrl, this.config);

      // Set up message handling
      this.websocket.onMessage((message) => this.handleMessage(message));

      await this.websocket.connect();
    }
  }

  private buildWebSocketUrl(): string {
    const url = new URL(this.client.getConfig().baseUrl);

    // Convert HTTP(S) to WS(S)
    if (url.protocol === 'http:') {
      url.protocol = 'ws:';
    } else if (url.protocol === 'https:') {
      url.protocol = 'wss:';
    }

    url.pathname = '/graphql';
    return url.toString();
  }

  private sendSubscriptionStart(id: SubscriptionId, subscription: GraphQLSubscription): void {
    if (this.websocket) {
      this.websocket.sendMessage({
        type: 'start',
        id: id.toString(),
        payload: subscription,
      });
    }
  }

  private sendSubscriptionStop(id: SubscriptionId): void {
    if (this.websocket) {
      this.websocket.sendMessage({
        type: 'stop',
        id: id.toString(),
      });
    }
  }

  private handleMessage(message: GraphQLWSMessage): void {
    this.metrics.incrementMessagesReceived();

    switch (message.type) {
      case 'data':
        this.handleSubscriptionData(message.id, message.payload);
        break;
      case 'error':
        this.handleSubscriptionError(message.id, message.payload);
        break;
      case 'complete':
        this.handleSubscriptionComplete(message.id);
        break;
      case 'connection_ack':
        // Connection acknowledged, ready to send subscriptions
        break;
      case 'ping':
        // Respond with pong
        if (this.websocket) {
          this.websocket.sendMessage({ type: 'pong', payload: message.payload });
        }
        break;
      default:
        // Handle other message types as needed
        break;
    }
  }

  private async handleSubscriptionData(id: string, payload: any): Promise<void> {
    const subscription = this.subscriptions.get(id);
    if (subscription) {
      await subscription.handleData(payload);
    }
  }

  private async handleSubscriptionError(id: string, payload: any): Promise<void> {
    const subscription = this.subscriptions.get(id);
    if (subscription) {
      const error = SubscriptionError.graphqlError(SubscriptionId.fromString(id), payload);
      await subscription.handleError(error);
    }
  }

  private async handleSubscriptionComplete(id: string): Promise<void> {
    const subscription = this.subscriptions.get(id);
    if (subscription) {
      await subscription.handleComplete();
      this.subscriptions.delete(id);
      this.metrics.decrementActiveSubscriptions();
    }
  }
}

// ============================================================================
// Subscription Client
// ============================================================================

/**
 * Subscription client for real-time GraphQL subscriptions
 */
export class SubscriptionClient {
  private manager: SubscriptionManager;

  constructor(private client: Client) {
    this.manager = new SubscriptionManager(client);
  }

  /**
   * Subscribe to resource state changes
   */
  resourceUpdates(): ResourceUpdateSubscriptionBuilder {
    return new ResourceUpdateSubscriptionBuilder(this.manager);
  }

  /**
   * Subscribe to workflow events
   */
  workflowEvents(): WorkflowEventSubscriptionBuilder {
    return new WorkflowEventSubscriptionBuilder(this.manager);
  }

  /**
   * Subscribe to agent execution events
   */
  agentExecutionStream(): AgentExecutionSubscriptionBuilder {
    return new AgentExecutionSubscriptionBuilder(this.manager);
  }

  /**
   * Subscribe to LLM response streaming
   */
  llmStream(requestId: string): LLMStreamSubscriptionBuilder {
    return new LLMStreamSubscriptionBuilder(this.manager, requestId);
  }

  /**
   * Subscribe to real-time cost updates
   */
  costUpdates(): CostUpdateSubscriptionBuilder {
    return new CostUpdateSubscriptionBuilder(this.manager);
  }

  /**
   * Subscribe to MCP server status updates
   */
  mcpServerStatusUpdates(): MCPServerStatusSubscriptionBuilder {
    return new MCPServerStatusSubscriptionBuilder(this.manager);
  }

  /**
   * Subscribe to MCP session events
   */
  mcpSessionEvents(): MCPSessionEventSubscriptionBuilder {
    return new MCPSessionEventSubscriptionBuilder(this.manager);
  }

  /**
   * Get subscription manager for advanced operations
   */
  getManager(): SubscriptionManager {
    return this.manager;
  }

  /**
   * Get current metrics
   */
  getMetrics(): SubscriptionMetrics {
    return this.manager.getMetrics();
  }

  /**
   * Close all subscriptions and connections
   */
  async close(): Promise<void> {
    await this.manager.close();
  }
}

// ============================================================================
// Subscription Builders
// ============================================================================

/**
 * Simple subscription handler implementation
 */
class SimpleHandler<T> implements SubscriptionHandler<T> {
  constructor(
    private dataHandler: (data: T) => Promise<void> | void,
    private errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    private completeHandler?: () => Promise<void> | void,
  ) {}

  async onData(data: T): Promise<void> {
    await this.dataHandler(data);
  }

  async onError(error: SubscriptionError): Promise<void> {
    if (this.errorHandler) {
      await this.errorHandler(error);
    }
  }

  async onComplete(): Promise<void> {
    if (this.completeHandler) {
      await this.completeHandler();
    }
  }
}

/**
 * Builder for resource update subscriptions
 */
export class ResourceUpdateSubscriptionBuilder {
  private _resourceId?: string;
  private _workflowId?: string;

  constructor(private manager: SubscriptionManager) {}

  resourceId(id: string): this {
    this._resourceId = id;
    return this;
  }

  workflowId(id: string): this {
    this._workflowId = id;
    return this;
  }

  async subscribe(
    handler: (data: ResourceUpdateEvent) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription ResourceUpdates($resourceId: String!) {
          resourceUpdates(resourceId: $resourceId) {
            id
            workflowId
            state
            data
            metadata
            createdAt
            updatedAt
          }
        }
      `,
      variables: { resourceId: this._resourceId },
      operationName: 'ResourceUpdates',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

/**
 * Builder for workflow event subscriptions
 */
export class WorkflowEventSubscriptionBuilder {
  private _workflowId?: string;

  constructor(private manager: SubscriptionManager) {}

  workflowId(id: string): this {
    this._workflowId = id;
    return this;
  }

  async subscribe(
    handler: (data: WorkflowEvent) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription WorkflowEvents($workflowId: String!) {
          workflowEvents(workflowId: $workflowId) {
            id
            type
            message
            data
            timestamp
          }
        }
      `,
      variables: { workflowId: this._workflowId },
      operationName: 'WorkflowEvents',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

/**
 * Builder for agent execution subscriptions
 */
export class AgentExecutionSubscriptionBuilder {
  private _executionId?: string;

  constructor(private manager: SubscriptionManager) {}

  executionId(id: string): this {
    this._executionId = id;
    return this;
  }

  async subscribe(
    handler: (data: AgentExecutionEvent) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription AgentExecutionStream($executionId: String!) {
          agentExecutionStream(executionId: $executionId) {
            id
            agentId
            status
            output
            timestamp
          }
        }
      `,
      variables: { executionId: this._executionId },
      operationName: 'AgentExecutionStream',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

/**
 * Builder for LLM stream subscriptions
 */
export class LLMStreamSubscriptionBuilder {
  constructor(
    private manager: SubscriptionManager,
    private requestId: string,
  ) {}

  async subscribe(
    handler: (data: LLMStreamChunk) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription LLMStream($requestId: String!) {
          llmStream(requestId: $requestId) {
            id
            content
            finished
            timestamp
          }
        }
      `,
      variables: { requestId: this.requestId },
      operationName: 'LLMStream',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

/**
 * Builder for cost update subscriptions
 */
export class CostUpdateSubscriptionBuilder {
  private _userId?: string;
  private _projectId?: string;

  constructor(private manager: SubscriptionManager) {}

  userId(id: string): this {
    this._userId = id;
    return this;
  }

  projectId(id: string): this {
    this._projectId = id;
    return this;
  }

  async subscribe(
    handler: (data: CostUpdateEvent) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription CostUpdates($userId: String) {
          costUpdates(userId: $userId) {
            userId
            projectId
            cost
            timestamp
            details
          }
        }
      `,
      variables: { userId: this._userId },
      operationName: 'CostUpdates',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

/**
 * Builder for MCP server status subscriptions
 */
export class MCPServerStatusSubscriptionBuilder {
  private _serverId?: string;

  constructor(private manager: SubscriptionManager) {}

  serverId(id: string): this {
    this._serverId = id;
    return this;
  }

  async subscribe(
    handler: (data: MCPServerStatusUpdate) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription MCPServerStatusUpdates($serverId: ID) {
          mcpServerStatusUpdates(serverId: $serverId) {
            serverId
            status
            message
            timestamp
          }
        }
      `,
      variables: { serverId: this._serverId },
      operationName: 'MCPServerStatusUpdates',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

/**
 * Builder for MCP session event subscriptions
 */
export class MCPSessionEventSubscriptionBuilder {
  private _userId?: string;
  private _serverId?: string;

  constructor(private manager: SubscriptionManager) {}

  userId(id: string): this {
    this._userId = id;
    return this;
  }

  serverId(id: string): this {
    this._serverId = id;
    return this;
  }

  async subscribe(
    handler: (data: MCPSessionEvent) => Promise<void> | void,
    errorHandler?: (error: SubscriptionError) => Promise<void> | void,
    completeHandler?: () => Promise<void> | void,
  ): Promise<SubscriptionId> {
    const subscription: GraphQLSubscription = {
      query: `
        subscription MCPSessionEvents($userId: String, $serverId: ID) {
          mcpSessionEvents(userId: $userId, serverId: $serverId) {
            sessionId
            event
            data
            timestamp
          }
        }
      `,
      variables: { userId: this._userId, serverId: this._serverId },
      operationName: 'MCPSessionEvents',
    };

    const subscriptionHandler = new SimpleHandler(handler, errorHandler, completeHandler);
    return this.manager.subscribe(subscription, subscriptionHandler);
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Subscribe to resource updates with a simple callback
 */
export async function subscribeResourceUpdates(
  client: Client,
  resourceId: string,
  handler: (resource: ResourceUpdateEvent) => void,
): Promise<SubscriptionId> {
  return client
    .subscriptions()
    .resourceUpdates()
    .resourceId(resourceId)
    .subscribe(handler);
}

/**
 * Subscribe to workflow events with a simple callback
 */
export async function subscribeWorkflowEvents(
  client: Client,
  workflowId: string,
  handler: (event: WorkflowEvent) => void,
): Promise<SubscriptionId> {
  return client
    .subscriptions()
    .workflowEvents()
    .workflowId(workflowId)
    .subscribe(handler);
}

/**
 * Subscribe to LLM streaming with a simple callback
 */
export async function subscribeLLMStream(
  client: Client,
  requestId: string,
  handler: (chunk: LLMStreamChunk) => void,
): Promise<SubscriptionId> {
  return client
    .subscriptions()
    .llmStream(requestId)
    .subscribe(handler);
}

/**
 * Subscribe to cost updates with a simple callback
 */
export async function subscribeCostUpdates(
  client: Client,
  userId: string,
  handler: (update: CostUpdateEvent) => void,
): Promise<SubscriptionId> {
  return client
    .subscriptions()
    .costUpdates()
    .userId(userId)
    .subscribe(handler);
}
