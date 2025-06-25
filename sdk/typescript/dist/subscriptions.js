/**
 * Real-time Subscription Infrastructure
 *
 * This module provides comprehensive real-time subscription capabilities for the Circuit Breaker SDK,
 * enabling WebSocket-based GraphQL subscriptions with automatic reconnection, error recovery,
 * and type-safe event handling.
 */
// ============================================================================
// Types
// ============================================================================
/**
 * Unique identifier for subscriptions
 */
export class SubscriptionId {
    constructor(id) {
        this.id = id || crypto.randomUUID();
    }
    toString() {
        return this.id;
    }
    static fromString(id) {
        return new SubscriptionId(id);
    }
    equals(other) {
        return this.id === other.id;
    }
}
/**
 * Default subscription configuration
 */
export const defaultSubscriptionConfig = {
    reconnectAttempts: 5,
    reconnectDelay: 1000,
    heartbeatInterval: 30000,
    messageTimeout: 10000,
};
/**
 * Subscription error types
 */
export class SubscriptionError extends Error {
    constructor(message, subscriptionId, payload) {
        super(message);
        this.subscriptionId = subscriptionId;
        this.payload = payload;
        this.name = 'SubscriptionError';
    }
    static connectionFailed(message) {
        return new SubscriptionError(`WebSocket connection failed: ${message}`);
    }
    static graphqlError(subscriptionId, payload) {
        return new SubscriptionError(`Subscription ${subscriptionId.toString()} failed with GraphQL error`, subscriptionId, payload);
    }
    static authenticationFailed(message) {
        return new SubscriptionError(`Authentication failed: ${message}`);
    }
    static timeout(timeout) {
        return new SubscriptionError(`Subscription timeout after ${timeout}ms`);
    }
}
/**
 * Subscription metrics for monitoring
 */
export class SubscriptionMetrics {
    constructor() {
        this._activeSubscriptions = 0;
        this._messagesReceived = 0;
        this._connectionFailures = 0;
        this._reconnectionAttempts = 0;
    }
    get activeSubscriptions() {
        return this._activeSubscriptions;
    }
    get messagesReceived() {
        return this._messagesReceived;
    }
    get connectionFailures() {
        return this._connectionFailures;
    }
    get reconnectionAttempts() {
        return this._reconnectionAttempts;
    }
    incrementActiveSubscriptions() {
        this._activeSubscriptions++;
    }
    decrementActiveSubscriptions() {
        this._activeSubscriptions--;
    }
    incrementMessagesReceived() {
        this._messagesReceived++;
    }
    incrementConnectionFailures() {
        this._connectionFailures++;
    }
    incrementReconnectionAttempts() {
        this._reconnectionAttempts++;
    }
}
// ============================================================================
// WebSocket Connection
// ============================================================================
/**
 * WebSocket connection wrapper with auto-reconnection
 */
export class WebSocketConnection {
    constructor(url, config = {}) {
        this.url = url;
        this.connected = false;
        this.reconnectAttempts = 0;
        this.messageQueue = [];
        this.messageHandlers = new Set();
        this.config = { ...defaultSubscriptionConfig, ...config };
    }
    async connect() {
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
                        const message = JSON.parse(event.data);
                        this.handleMessage(message);
                    }
                    catch (error) {
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
            }
            catch (error) {
                reject(SubscriptionError.connectionFailed(String(error)));
            }
        });
    }
    isConnected() {
        return this.connected && this.websocket?.readyState === WebSocket.OPEN;
    }
    sendMessage(message) {
        if (this.isConnected()) {
            this.websocket.send(JSON.stringify(message));
        }
        else {
            this.messageQueue.push(message);
        }
    }
    onMessage(handler) {
        this.messageHandlers.add(handler);
    }
    offMessage(handler) {
        this.messageHandlers.delete(handler);
    }
    close() {
        if (this.websocket) {
            this.websocket.close();
        }
    }
    sendConnectionInit() {
        this.sendMessage({ type: 'connection_init' });
    }
    flushMessageQueue() {
        while (this.messageQueue.length > 0) {
            const message = this.messageQueue.shift();
            this.sendMessage(message);
        }
    }
    handleMessage(message) {
        this.messageHandlers.forEach((handler) => {
            try {
                handler(message);
            }
            catch (error) {
                console.error('Message handler error:', error);
            }
        });
    }
    async handleDisconnection() {
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
export class ActiveSubscription {
    constructor(id, subscription, handler) {
        this.id = id;
        this.subscription = subscription;
        this.handler = handler;
    }
    async handleData(payload) {
        try {
            await this.handler.onData(payload);
        }
        catch (error) {
            console.error('Subscription data handler error:', error);
        }
    }
    async handleError(error) {
        try {
            await this.handler.onError(error);
        }
        catch (handlerError) {
            console.error('Subscription error handler error:', handlerError);
        }
    }
    async handleComplete() {
        try {
            await this.handler.onComplete();
        }
        catch (error) {
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
    constructor(client, config = {}) {
        this.client = client;
        this.config = config;
        this.subscriptions = new Map();
        this.metrics = new SubscriptionMetrics();
    }
    async subscribe(subscription, handler) {
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
    async unsubscribe(subscriptionId) {
        // Send unsubscribe message
        this.sendSubscriptionStop(subscriptionId);
        // Remove from active subscriptions
        this.subscriptions.delete(subscriptionId.toString());
        this.metrics.decrementActiveSubscriptions();
    }
    getMetrics() {
        return this.metrics;
    }
    async close() {
        if (this.websocket) {
            this.websocket.close();
        }
        this.subscriptions.clear();
    }
    async ensureConnection() {
        if (!this.websocket || !this.websocket.isConnected()) {
            const wsUrl = this.buildWebSocketUrl();
            this.websocket = new WebSocketConnection(wsUrl, this.config);
            // Set up message handling
            this.websocket.onMessage((message) => this.handleMessage(message));
            await this.websocket.connect();
        }
    }
    buildWebSocketUrl() {
        const url = new URL(this.client.getConfig().baseUrl);
        // Convert HTTP(S) to WS(S)
        if (url.protocol === 'http:') {
            url.protocol = 'ws:';
        }
        else if (url.protocol === 'https:') {
            url.protocol = 'wss:';
        }
        url.pathname = '/graphql';
        return url.toString();
    }
    sendSubscriptionStart(id, subscription) {
        if (this.websocket) {
            this.websocket.sendMessage({
                type: 'start',
                id: id.toString(),
                payload: subscription,
            });
        }
    }
    sendSubscriptionStop(id) {
        if (this.websocket) {
            this.websocket.sendMessage({
                type: 'stop',
                id: id.toString(),
            });
        }
    }
    handleMessage(message) {
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
    async handleSubscriptionData(id, payload) {
        const subscription = this.subscriptions.get(id);
        if (subscription) {
            await subscription.handleData(payload);
        }
    }
    async handleSubscriptionError(id, payload) {
        const subscription = this.subscriptions.get(id);
        if (subscription) {
            const error = SubscriptionError.graphqlError(SubscriptionId.fromString(id), payload);
            await subscription.handleError(error);
        }
    }
    async handleSubscriptionComplete(id) {
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
    constructor(client) {
        this.client = client;
        this.manager = new SubscriptionManager(client);
    }
    /**
     * Subscribe to resource state changes
     */
    resourceUpdates() {
        return new ResourceUpdateSubscriptionBuilder(this.manager);
    }
    /**
     * Subscribe to workflow events
     */
    workflowEvents() {
        return new WorkflowEventSubscriptionBuilder(this.manager);
    }
    /**
     * Subscribe to agent execution events
     */
    agentExecutionStream() {
        return new AgentExecutionSubscriptionBuilder(this.manager);
    }
    /**
     * Subscribe to LLM response streaming
     */
    llmStream(requestId) {
        return new LLMStreamSubscriptionBuilder(this.manager, requestId);
    }
    /**
     * Subscribe to real-time cost updates
     */
    costUpdates() {
        return new CostUpdateSubscriptionBuilder(this.manager);
    }
    /**
     * Subscribe to MCP server status updates
     */
    mcpServerStatusUpdates() {
        return new MCPServerStatusSubscriptionBuilder(this.manager);
    }
    /**
     * Subscribe to MCP session events
     */
    mcpSessionEvents() {
        return new MCPSessionEventSubscriptionBuilder(this.manager);
    }
    /**
     * Get subscription manager for advanced operations
     */
    getManager() {
        return this.manager;
    }
    /**
     * Get current metrics
     */
    getMetrics() {
        return this.manager.getMetrics();
    }
    /**
     * Close all subscriptions and connections
     */
    async close() {
        await this.manager.close();
    }
}
// ============================================================================
// Subscription Builders
// ============================================================================
/**
 * Simple subscription handler implementation
 */
class SimpleHandler {
    constructor(dataHandler, errorHandler, completeHandler) {
        this.dataHandler = dataHandler;
        this.errorHandler = errorHandler;
        this.completeHandler = completeHandler;
    }
    async onData(data) {
        await this.dataHandler(data);
    }
    async onError(error) {
        if (this.errorHandler) {
            await this.errorHandler(error);
        }
    }
    async onComplete() {
        if (this.completeHandler) {
            await this.completeHandler();
        }
    }
}
/**
 * Builder for resource update subscriptions
 */
export class ResourceUpdateSubscriptionBuilder {
    constructor(manager) {
        this.manager = manager;
    }
    resourceId(id) {
        this._resourceId = id;
        return this;
    }
    workflowId(id) {
        this._workflowId = id;
        return this;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
    constructor(manager) {
        this.manager = manager;
    }
    workflowId(id) {
        this._workflowId = id;
        return this;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
    constructor(manager) {
        this.manager = manager;
    }
    executionId(id) {
        this._executionId = id;
        return this;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
    constructor(manager, requestId) {
        this.manager = manager;
        this.requestId = requestId;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
    constructor(manager) {
        this.manager = manager;
    }
    userId(id) {
        this._userId = id;
        return this;
    }
    projectId(id) {
        this._projectId = id;
        return this;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
    constructor(manager) {
        this.manager = manager;
    }
    serverId(id) {
        this._serverId = id;
        return this;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
    constructor(manager) {
        this.manager = manager;
    }
    userId(id) {
        this._userId = id;
        return this;
    }
    serverId(id) {
        this._serverId = id;
        return this;
    }
    async subscribe(handler, errorHandler, completeHandler) {
        const subscription = {
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
export async function subscribeResourceUpdates(client, resourceId, handler) {
    return client
        .subscriptions()
        .resourceUpdates()
        .resourceId(resourceId)
        .subscribe(handler);
}
/**
 * Subscribe to workflow events with a simple callback
 */
export async function subscribeWorkflowEvents(client, workflowId, handler) {
    return client
        .subscriptions()
        .workflowEvents()
        .workflowId(workflowId)
        .subscribe(handler);
}
/**
 * Subscribe to LLM streaming with a simple callback
 */
export async function subscribeLLMStream(client, requestId, handler) {
    return client
        .subscriptions()
        .llmStream(requestId)
        .subscribe(handler);
}
/**
 * Subscribe to cost updates with a simple callback
 */
export async function subscribeCostUpdates(client, userId, handler) {
    return client
        .subscriptions()
        .costUpdates()
        .userId(userId)
        .subscribe(handler);
}
//# sourceMappingURL=subscriptions.js.map